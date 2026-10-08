use crate::app_theme::neutral;
use crate::{
    block_editor::{BlockEditor, EditorEvent},
    macos,
    store::{Note, NoteSummary, Store},
    note_tree::NoteTree,
};
use empire_ui::{Button, ButtonSize, ButtonVariant, Input, ScrollArea, Slider, SliderEvent, Resizable, ResizablePanel, Popover, PopoverEvent};
use empire_ui::context_menu::ContextMenu;
use crate::sidebar_drag::{Destination,Edge};
use empire_ui::menu::{Menu, MenuItem, MenuAlign, MenuSide, MenuEvent};
use gpui::{prelude::*, *};
use gpui_component::{
    input::{InputEvent, InputState},
    Root,
};
use std::time::{Duration, Instant};

actions!(
    sparkpad,
    [ReadMode, NewNote, ToggleSidebar, HidePanel, QuitApp]
);

#[derive(Clone)]
struct NoteDrag { id: String, title: String }
impl Render for NoteDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().pt(px(24.)).pl(px(20.)).child(div().px_3().py_1().rounded_md().bg(neutral(0x303030)).text_color(neutral(0xeeeeee)).text_sm().child(self.title.clone()))
    }
}

const COVER_HEIGHT: f32 = 160.;
const PAGE_ICON_SIZE: f32 = 93.6;
const SIDEBAR_PAGE_ICON_SIZE: f32 = 18.2;

#[derive(Clone, Copy, PartialEq)]
enum PageAction { Icon, Cover }

fn cover_background(preset: &crate::covers::CoverPreset) -> Background {
    if preset.colors[0]==preset.colors[1] {rgb(preset.colors[0]).into()}
    else {linear_gradient(preset.angle, linear_color_stop(rgb(preset.colors[0]),0.), linear_color_stop(rgb(preset.colors[1]),1.))}
}

fn slider_popover(slider: Entity<Slider>, label: &'static str, icon: &'static str, cx: &mut App) -> Entity<Popover> {
    cx.new(|cx| {
        cx.subscribe(&slider, |_, _, _, cx| cx.notify()).detach();
        Popover::new(cx).motion(true).side(MenuSide::Top).align(MenuAlign::Start).width(224.).padding(12.)
            .trigger(move |open, _, _| {
                Button::icon(label, icon).size(ButtonSize::IconSm)
                    .variant(if open {ButtonVariant::Secondary} else {ButtonVariant::Ghost})
                    .into_any_element()
            })
            .content(move |_, cx| {
                div().w_full().flex().flex_col().gap_2()
                    .child(div().flex().justify_between().text_xs().text_color(neutral(0xa3a3a3))
                        .child(label).child(format!("{}%", slider.read(cx).value().round() as u32)))
                    .child(slider.clone()).into_any_element()
            })
    })
}

fn note_menu_items()->Vec<MenuItem> {vec![
    MenuItem::new("Renomear").icon("iconoir/regular/edit-pencil.svg"),
    MenuItem::new("Adicionar nota filha").icon("iconoir/regular/plus.svg"),
    MenuItem::new("Mover para raiz").icon("iconoir/regular/page.svg"),
    MenuItem::separator(),
    MenuItem::new("Excluir nota").icon("iconoir/regular/trash.svg").destructive(),
]}
struct PendingDelete {id:String,title:String,revision:i64,count:usize}
struct Panel {
    page_action: Option<PageAction>,
    title_hovered: bool,
    page_icon: Option<String>,
    icon_picker: Entity<crate::icon_picker::IconPicker>,
    page_cover: Option<String>,
    db: Store,
    sync_dialog: Option<Entity<crate::sync_settings::SyncSettings>>,
    notes: Vec<NoteSummary>,
    tree: NoteTree,
    #[cfg(test)]
    rendered_tree_rows: usize,
    #[cfg(test)]
    tree_row_bounds: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, Bounds<Pixels>>>>,
    tree_focus: FocusHandle,
    hovered_note: Option<String>,
    note_menu: Entity<Menu>,
    note_context_menu:Entity<Menu>,
    pending_delete:Option<PendingDelete>,
    delete_focus:FocusHandle,
    tree_drop:Option<Destination>,
    drag_pointer:Option<Point<Pixels>>,
    drag_source:Option<String>,
    drag_frame_pending:bool,
    drag_frame_at:Instant,
    hover_expand:Option<(String,Instant)>,
    note_menu_target: Option<String>,
    note_menu_open: bool,
    group_menu: Entity<Menu>,
    group_menu_target: Option<String>,
    group_menu_open: bool,
    group_title: Entity<InputState>,
    renaming_group: Option<String>,
    last_data_version: Option<i64>,
    last_preferences: Option<crate::preferences::Preferences>,
    selected: Option<Note>,
    title: Entity<InputState>,
    editor: Entity<BlockEditor>,
    opacity: Entity<Slider>,
    content_width: Entity<Slider>,
    opacity_popover: Entity<Popover>,
    width_popover: Entity<Popover>,
    font_menu: Entity<Menu>,
    content_font: usize,
    pending_opacity: Option<f32>,
    opacity_frame_pending: bool,
    subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    reader_scroll: ScrollHandle,
    editor_scroll: ScrollHandle,
    sidebar_scroll: ListState,
    tree_motion: crate::ui_motion::TreeMotion,
    sidebar_motion: empire_ui::motion::Tween,
    interactions: std::collections::HashMap<String,empire_ui::motion::Tween>,
    popup_motion: empire_ui::motion::Tween,
    painted_page_action: Option<PageAction>,
    editing: bool,
    sidebar: bool,
    sidebar_width: f32,
    pinned: bool,
    light_mode: bool,
    mcp_connected: Option<bool>,
    visible: bool,
    error: Option<String>,
    dirty: bool,
    last_edit: Instant,
    last_sync: Instant,
    font_size: f32,
    panel_request: Option<String>,
    last_bounds: Option<(f32, f32, f32, f32)>,
}
impl Panel {
    fn new(db: Store, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let light_mode=db.setting("theme").ok().flatten().as_deref()==Some("light");
        crate::app_theme::apply(light_mode,cx);
        let opacity_value = db
            .setting("opacity")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(94.)
            .clamp(25., 100.);
        let font_size = db
            .setting("font_size")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(22.)
            .clamp(14., 40.);
        let sidebar = db.setting("sidebar").ok().flatten().as_deref() != Some("false");
        let sidebar_width = db.setting("sidebar_width").ok().flatten()
            .and_then(|value| value.parse::<f32>().ok()).filter(|value| value.is_finite())
            .unwrap_or(206.).clamp(180., 420.);
        let content_width_value = db.setting("content_width").ok().flatten()
            .and_then(|value| value.parse::<f32>().ok()).filter(|value| value.is_finite())
            .unwrap_or(100.).clamp(40., 100.);
        let pinned = db.setting("always_on_top").ok().flatten().as_deref() != Some("false");
        let title =
            cx.new(|cx| empire_ui::input::single_line(window, cx).placeholder("Título da nota"));
        let editor = cx.new(|cx| {let mut editor=BlockEditor::new(window,cx);editor.set_asset_directory(db.mcp_sessions_dir().with_extension("assets"));editor});
        let opacity = cx.new(|cx| Slider::new(opacity_value, cx).bounds(25., 100.).step(0.));
        let content_width = cx.new(|cx| Slider::new(content_width_value, cx).bounds(40., 100.).step(0.));
        let opacity_popover = slider_popover(opacity.clone(), "Opacidade", "iconoir/regular/droplet-half.svg", cx);
        let width_popover = slider_popover(content_width.clone(), "Largura", "iconoir/regular/align-horizontal-spacing.svg", cx);
        let saved_font=db.setting("content_font").ok().flatten();
        let content_font=crate::assets::CONTENT_FONTS.iter().position(|font| Some(font.0)==saved_font.as_deref()).unwrap_or(0);
        let font_menu=cx.new(|cx| Menu::new(crate::assets::CONTENT_FONTS.iter().enumerate()
            .map(|(i,font)| MenuItem::radio(0,font.2,i==content_font).close_on_click(true)).collect(),cx).motion(true)
            .side(MenuSide::Top).align(MenuAlign::Start).width(272.)
            .trigger(|open,_,_| Button::icon("content-font","iconoir/regular/text.svg")
                .size(ButtonSize::IconSm).variant(if open {ButtonVariant::Secondary} else {ButtonVariant::Ghost}).into_any_element()));
        let notes = db.list_summaries().unwrap_or_default();
        let mut tree = NoteTree::default();
        tree.groups=db.list_groups().unwrap_or_default();
        tree.expanded = db.setting("expanded_notes").ok().flatten()
            .and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
        tree.set_notes(&notes);
        let selected_id = db.setting("selected_note").ok().flatten();
        let selected = notes
            .iter()
            .find(|n| Some(&n.id) == selected_id.as_ref())
            .or(notes.first())
            .and_then(|note| db.get(&note.id).ok());
        let panel_request = db.setting("panel_request").ok().flatten();
        let mcp_connected = crate::mcp_presence::connected(db.mcp_sessions_dir()).ok();
        let note_menu = cx.new(|cx| Menu::new(note_menu_items(), cx).motion(true).align(MenuAlign::End).width(200.).trigger(|_,_,_| {
            div().size(px(20.)).flex().items_center().justify_center().rounded(px(4.))
                .hover(|s| s.bg(neutral(0x414141)))
                .child(svg().path("iconoir/regular/more-horiz.svg").size(px(16.)).text_color(neutral(0xa3a3a3)))
                .into_any_element()
        }));
        let note_context_menu=cx.new(|cx|ContextMenu::menu(note_menu_items(),cx).motion(true).width(224.));
        let group_title=cx.new(|cx| empire_ui::input::single_line(window,cx).placeholder("Nome do grupo"));
        let group_menu=cx.new(|cx| Menu::new(vec![
            MenuItem::new("Renomear grupo").icon("iconoir/regular/edit-pencil.svg"),
            MenuItem::new("Adicionar nota").icon("iconoir/regular/plus.svg"),
            MenuItem::new("Excluir grupo (manter notas)").icon("iconoir/regular/trash.svg"),
        ],cx).motion(true).align(MenuAlign::End).width(248.).trigger(|_,_,_| {
            div().size(px(20.)).flex().items_center().justify_center().rounded(px(4.))
                .hover(|s| s.bg(neutral(0x414141)))
                .child(svg().path("iconoir/regular/more-horiz.svg").size(px(16.)).text_color(neutral(0xa3a3a3))).into_any_element()
        }));
        let recent=db.setting("recent_page_icons").ok().flatten().and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
        let icon_picker=cx.new(|cx| crate::icon_picker::IconPicker::new(recent,window,cx));
        let last_preferences=crate::preferences::Preferences::read(&db).ok();
        let mut panel = Self {
            page_action: None, title_hovered: false, page_icon: None, page_cover: None,
            icon_picker,
            db,
            sync_dialog: None,
            notes,
            tree,
            #[cfg(test)]
            rendered_tree_rows: 0,
            #[cfg(test)]
            tree_row_bounds: Default::default(),
            tree_focus: cx.focus_handle(),
            hovered_note: None,
            note_menu,
            note_context_menu,delete_focus:cx.focus_handle(),pending_delete:None,tree_drop:None,drag_pointer:None,drag_source:None,drag_frame_pending:false,drag_frame_at:Instant::now(),hover_expand:None,
            note_menu_target: None,
            note_menu_open: false,
            group_menu, group_menu_target:None, group_menu_open:false,
            group_title, renaming_group:None,
            last_data_version: None,
            last_preferences,
            selected: None,
            title,
            editor,
            opacity,
            content_width,
            opacity_popover,
            width_popover,
            font_menu,
            content_font,
            pending_opacity: None,
            opacity_frame_pending: false,
            subscriptions: Vec::new(),
            focus_handle: cx.focus_handle(),
            reader_scroll: ScrollHandle::new(),
            editor_scroll: ScrollHandle::new(),
            sidebar_scroll: ListState::new(0,ListAlignment::Top,px(32.)),
            tree_motion: Default::default(),
            sidebar_motion: empire_ui::motion::Tween::new(if sidebar {1.} else {0.}),
            interactions: Default::default(),
            popup_motion: empire_ui::motion::Tween::new(0.),painted_page_action:None,
            editing: false,
            sidebar,
            sidebar_width,
            pinned,
            light_mode,
            mcp_connected,
            visible: true,
            error: None,
            dirty: false,
            last_edit: Instant::now(),
            last_sync: Instant::now(),
            font_size,
            panel_request,
            last_bounds: None,
        };
        for menu in [panel.note_menu.clone(),panel.note_context_menu.clone()] {
            panel.subscriptions.push(cx.subscribe_in(&menu,window,|this,_,event,window,cx| {
                match event {
                    MenuEvent::OpenChange(open)=> {this.note_menu_open=*open;cx.notify();}
                    MenuEvent::Select(index)=> if let Some(id)=this.note_menu_target.clone() {this.note_action(*index,&id,window,cx);},
                    _=>{},
                }
            }));
        }
        panel.subscriptions.push(cx.subscribe(&panel.opacity_popover, |this, _, event, cx| {
            if matches!(event, PopoverEvent::OpenChange(true)) {
                this.dismiss_page_popup();
                this.width_popover.update(cx, |popover, cx| popover.dismiss(cx));
                this.font_menu.update(cx, |menu,cx| menu.dismiss(cx));
            }
        }));
        panel.subscriptions.push(cx.subscribe(&panel.width_popover, |this, _, event, cx| {
            if matches!(event, PopoverEvent::OpenChange(true)) {
                this.dismiss_page_popup();
                this.opacity_popover.update(cx, |popover, cx| popover.dismiss(cx));
                this.font_menu.update(cx, |menu,cx| menu.dismiss(cx));
            }
        }));
        panel.subscriptions.push(cx.subscribe(&panel.font_menu, |this, _, event, cx| {
            match event {
                MenuEvent::RadioChange {index,..} => {
                    if let Some(font)=crate::assets::CONTENT_FONTS.get(*index) {
                        this.content_font=*index;
                        if let Err(error)=this.db.set_setting("content_font",font.0) {
                            this.error=Some(error.to_string());
                        }
                        cx.notify();
                    }
                }
                MenuEvent::OpenChange(true) => {
                    this.dismiss_page_popup();
                    this.opacity_popover.update(cx, |popover,cx| popover.close(cx));
                    this.width_popover.update(cx, |popover,cx| popover.close(cx));
                }
                _ => {},
            }
        }));
        panel.subscriptions.push(cx.subscribe_in(&panel.group_title,window,|this,_,event,window,cx| {
            if matches!(event,InputEvent::Blur | InputEvent::PressEnter {..}) {
                this.finish_group_rename(cx);
                if matches!(event,InputEvent::PressEnter {..}) {window.focus(&this.tree_focus);}
            }
        }));
        panel.subscriptions.push(cx.subscribe_in(&panel.group_menu,window,|this,_,event,window,cx| {
            match event {
                MenuEvent::OpenChange(open) => {this.group_menu_open=*open;cx.notify();}
                MenuEvent::Select(index) => {
                    if let Some(id)=this.group_menu_target.clone() {
                        match index {
                            0 => this.rename_group(&id,window,cx),
                            1 => this.new_group_note(&id,window,cx),
                            2 => {
                                if this.renaming_group.as_deref()==Some(id.as_str()) {this.renaming_group=None;}
                                match this.db.delete_group(&id) {
                                    Ok(()) => this.refresh_tree(),
                                    Err(error) => this.error=Some(error.to_string()),
                                }
                                cx.notify();
                            }
                            _ => {},
                        }
                    }
                }
                _ => {},
            }
        }));
        panel.subscriptions.push(cx.subscribe(&panel.icon_picker,|this,_,event,cx| {
            match event {
                crate::icon_picker::PickerEvent::Select(value) => this.set_page_icon(value.as_deref(),cx),
                crate::icon_picker::PickerEvent::Upload => this.choose_icon(cx),
            }
        }));
        panel.sync_tree_motion(false);
        let scroll = panel.editor_scroll.clone();
        panel
            .editor
            .update(cx, |editor, _| editor.set_scroll_handle(scroll));
        panel.subscriptions.push(cx.subscribe_in(
            &panel.editor,
            window,
            |this, _, event, _, cx| {
                if matches!(event, EditorEvent::Change) {
                    this.changed(cx);
                } else if matches!(event, EditorEvent::Editing) {
                    this.editing = true;
                    cx.notify();
                }
            },
        ));
        panel.subscriptions.push(
            cx.subscribe_in(&panel.title, window, |this, _, event, _, cx| {
                if matches!(event, InputEvent::Focus) {
                    this.editor
                        .update(cx, |editor, cx| editor.clear_selection(cx));
                }
                if matches!(event, InputEvent::Change) {
                    this.changed(cx);
                }
            }),
        );
        panel.subscriptions.push(cx.subscribe_in(
            &panel.opacity,
            window,
            |this, _, event, window, cx| {
                if let SliderEvent::Change(values) = event {
                    // Mouse events can arrive faster than the display refresh rate.
                    // Apply only the latest alpha once per frame, rather than asking
                    // AppKit to recompose the transparent window for every event.
                    this.pending_opacity = Some(values[0] / 100.);
                    if !this.opacity_frame_pending {
                        this.opacity_frame_pending = true;
                        let panel = cx.entity().downgrade();
                        window.on_next_frame(move |window, cx| {
                            let _ = panel.update(cx, |this, cx| {
                                this.opacity_frame_pending = false;
                                if let Some(value) = this.pending_opacity.take() {
                                    macos::opacity(window, value);
                                    cx.notify();
                                }
                            });
                        });
                    }
                }
                if let SliderEvent::Commit(values) = event {
                    if let Err(e) = this.db.set_setting("opacity", &values[0].to_string()) {
                        this.error = Some(e.to_string());
                    }
                }
            },
        ));
        panel.subscriptions.push(cx.subscribe_in(&panel.content_width, window, |this, _, event, _, cx| {
            match event {
                SliderEvent::Change(_) => cx.notify(),
                SliderEvent::Commit(values) => {
                    if let Err(error) = this.db.set_setting("content_width", &values[0].to_string()) {
                        this.error = Some(error.to_string());
                        cx.notify();
                    }
                }
            }
        }));
        if let Some(note) = selected {
            panel.load(note, window, cx);
        }
        panel.subscriptions.push(cx.on_app_quit(|this, cx| {
            if !this.save(cx) && this.dirty {
                let title = format!("{} (recuperação)", this.title.read(cx).value());
                let text = this.editor.read(cx).value();
                if let Err(e) = this.db.create(&title, &text) {
                    eprintln!("Failed to preserve pending note: {e}");
                }
            }
            async {}
        }));
        #[cfg(not(test))]
        if panel.db.setting("onboarding_completed").ok().flatten().is_none() && panel.notes.len()<=1 {
            panel.open_sync_settings(true,window,cx);
        }
        panel.focus_handle.focus(window);
        macos::configure(window, opacity_value / 100., pinned);
        macos::appearance(window,light_mode);
        panel
    }
    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.light_mode=!self.light_mode;
        crate::app_theme::apply(self.light_mode,cx);
        self.editor.update(cx, |editor,cx| editor.refresh_theme(cx));
        macos::appearance(window,self.light_mode);
        if let Err(error)=self.db.set_setting("theme",if self.light_mode {"light"} else {"dark"}) {
            self.error=Some(error.to_string());
        }
        self.opacity_popover.update(cx, |_,cx| cx.notify());
        self.width_popover.update(cx, |_,cx| cx.notify());
        cx.notify();
    }
    fn open_sync_settings(&mut self,onboarding:bool,window:&mut Window,cx:&mut Context<Self>) {
        let path=self.db.path.clone();
        let dialog=cx.new(|cx|crate::sync_settings::SyncSettings::new(path,onboarding,window,cx));
        cx.subscribe_in(&dialog,window,|this,_,_:&crate::sync_settings::Closed,_,cx| {this.sync_dialog=None;cx.notify();}).detach();
        self.sync_dialog=Some(dialog);cx.notify();
    }
    fn apply_shared_note(&mut self,note:Note,window:&mut Window,cx:&mut Context<Self>) {
        if let Ok(Some(state))=crate::sync_storage::editor_state_at(&self.db,&note.id,note.revision) {
            self.editor.update(cx,|editor,cx| {let _=editor.apply_shared_state(&state,window,cx);});
        }
        if self.title.read(cx).value().as_ref()!=note.title {
            let old=self.title.read(cx).value().to_string();let range=self.title.read(cx).selection_range();
            let a=sparkpad_sync::transform_cursor(&old,&note.title,range.start);let b=sparkpad_sync::transform_cursor(&old,&note.title,range.end);
            self.title.update(cx,|title,cx| {title.set_value(note.title.clone(),window,cx);title.set_byte_selection(a.min(b)..a.max(b),cx);});
        }
        self.selected=Some(note);self.dirty=false;self.error=None;cx.notify();
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        // InputState emits deferred Change events even for set_value(). Compare content
        // to avoid treating note loads and MCP refreshes as local edits.
        self.dirty = self.selected.as_ref().is_some_and(|note| {
            self.title.read(cx).value().as_ref() != note.title
                || self.editor.read(cx).value().as_ref() != note.markdown
        });
        self.last_edit = Instant::now();
        cx.notify();
    }
    fn load(&mut self, note: Note, window: &mut Window, cx: &mut Context<Self>) {
        self.page_action = None;
        (self.page_icon,self.page_cover)=self.db.presentation(&note.id).unwrap_or_default();
        self.title
            .update(cx, |s, cx| s.set_value(note.title.clone(), window, cx));
        self.editor
            .update(cx, |s, cx| s.set_value(note.markdown.clone(), window, cx));
        if let Ok(Some(state))=crate::sync_storage::editor_state_at(&self.db,&note.id,note.revision) {
            self.editor.update(cx,|editor,_| {let _=editor.bind_shared_document(&state);});
        }
        self.dirty = false;
        self.error = None;
        self.reader_scroll.set_offset(point(px(0.), px(0.)));
        self.editor_scroll.set_offset(point(px(0.), px(0.)));
        let id = note.id.clone();
        self.selected = Some(note);
        self.reveal_note(&id);
        self.editing = false;
        cx.notify();
    }
    fn save(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.dirty {
            return self.error.is_none();
        }
        let Some(note) = &self.selected else {
            return true;
        };
        let title = self.title.read(cx).value();
        let text = self.editor.read(cx).value();
        let result=if crate::sync_storage::enabled(&self.db) && self.editor.read(cx).has_shared_document() {
            crate::sync_storage::vector(&self.db,&note.id).and_then(|vector| {
                self.editor.update(cx,|editor,_|editor.shared_update(&title,&vector)).and_then(|update| {
                    crate::sync_storage::save_editor_update(&self.db,&note.id,&update.unwrap_or_default())
                })
            })
        } else {
            self.db.update(&note.id,if title.trim().is_empty() {"Sem título"} else {&title},&text,note.revision)
        };
        match result {
            Ok(note) => {
                self.selected = Some(note);
                self.refresh_tree();
                self.dirty = false;
                self.error = None;
                cx.notify();
                true
            }
            Err(e) => {
                self.error = Some(format!("Não foi possível salvar: {e}"));
                cx.notify();
                false
            }
        }
    }
    fn select(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.save(cx) {
            return;
        }
        match self.db.select(id) {
            Ok(note) => self.load(note, window, cx),
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_child(None, window, cx);
    }
    fn new_child(&mut self, parent_id: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.save(cx) { return; }
        match self.db.create_child("Nova nota", "", parent_id) {
            Ok(note) => {
                if let Err(e) = self.db.select(&note.id) {
                    self.error = Some(e.to_string());
                    return;
                }
                self.refresh_tree();
                self.load(note, window, cx);
                self.editing = true;
                self.title.update(cx, |s, cx| s.focus(window, cx));
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn read_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.save(cx) {
            self.editing = false;
            self.editor.update(cx, |s, cx| s.read_mode(window, cx));
            self.focus_handle.focus(window);
            cx.notify();
        }
    }
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar = !self.sidebar;
        if let Err(e) = self.db.set_setting("sidebar", &self.sidebar.to_string()) {
            self.error = Some(e.to_string());
        }
        cx.notify();
    }
    fn visibility(&mut self, show: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !show {
            self.save(cx);
        }
        self.visible = show;
        macos::visible(window, show);
        if show {
            cx.activate(true);
            window.activate_window();
            if self.editing {
                self.editor.update(cx, |s, cx| s.focus(window, cx));
            } else {
                self.focus_handle.focus(window);
            }
        }
    }
    fn refresh_tree(&mut self) {
        if let Ok(notes) = self.db.list_summaries() {
            self.notes = notes;
            self.tree.groups=self.db.list_groups().unwrap_or_default();
            self.tree.set_notes(&self.notes);
        }
        self.sync_tree_motion(false);
        self.last_data_version = None;
    }
    fn create_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_group_rename(cx);
        match self.db.create_group("Novo grupo") {
            Ok(group) => {
                self.tree.expanded.insert(group.id.clone());
                self.refresh_tree();self.persist_expansion();
                if let Some(row)=self.tree.rows.iter().position(|r| r.group.is_some_and(|i| self.tree.groups[i].id==group.id)) {
                    self.sidebar_scroll.scroll_to_reveal_item(row);
                }
                self.rename_group(&group.id,window,cx);
            }
            Err(error) => {self.error=Some(error.to_string());cx.notify();}
        }
    }
    fn rename_group(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_group_rename(cx);
        if let Some(group)=self.tree.groups.iter().find(|g| g.id==id) {
            let title=group.title.clone();self.renaming_group=Some(id.to_owned());
            self.group_title.update(cx, |s,cx| {s.set_value(title,window,cx);s.focus(window,cx);});
            cx.notify();
        }
    }
    fn finish_group_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(id)=self.renaming_group.take() {
            let title=self.group_title.read(cx).value();
            if !title.trim().is_empty() {
                if let Err(error)=self.db.rename_group(&id,title.as_ref()) {self.error=Some(error.to_string());}
            }
            self.refresh_tree();cx.notify();
        }
    }
    fn new_group_note(&mut self, group: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.save(cx) {return;}
        match self.db.create_in_group("Nova nota","",group) {
            Ok(note) => {
                if let Err(error)=self.db.select(&note.id) {self.error=Some(error.to_string());cx.notify();return;}
                self.refresh_tree();self.load(note,window,cx);self.editing=true;
                self.title.update(cx,|s,cx| s.focus(window,cx));
            }
            Err(error) => {self.error=Some(error.to_string());cx.notify();}
        }
    }
    fn persist_expansion(&self) {
        if let Ok(value) = serde_json::to_string(&self.tree.expanded) {
            let _ = self.db.set_setting("expanded_notes", &value);
        }
    }
    fn reveal_note(&mut self, id: &str) {
        if let Some(row) = self.tree.reveal(id, &self.notes) {
            self.sync_tree_motion(false);
            self.sidebar_scroll.scroll_to_reveal_item(row);
            self.persist_expansion();
        }
    }
    fn toggle_note(&mut self, id: &str, cx: &mut Context<Self>) {
        self.tree.toggle(id, &self.notes);
        self.sync_tree_motion(true);
        self.persist_expansion();
        cx.notify();
    }
    fn sync_tree_motion(&mut self, animate:bool) {
        let mut offset=self.sidebar_scroll.logical_scroll_top();
        let anchor=self.tree_motion.rows.get(offset.item_ix).map(|r|r.key.clone());
        let old_count=self.sidebar_scroll.item_count();
        self.tree_motion.sync(&self.tree,&self.notes,animate);
        if let Some(index)=anchor.and_then(|key|self.tree_motion.rows.iter().position(|r|r.key==key)) {offset.item_ix=index;}
        self.sidebar_scroll.splice(0..old_count,self.tree_motion.rows.len());
        self.sidebar_scroll.scroll_to(offset);
    }
    fn interaction(&mut self,key:String,target:bool) -> f32 {
        let tween=self.interactions.entry(key).or_insert_with(||empire_ui::motion::Tween::new(0.));
        tween.set(if target {1.} else {0.},120);tween.value()
    }
    fn note_action(&mut self,index:usize,id:&str,window:&mut Window,cx:&mut Context<Self>) {
        match index {
            0=>{self.select(id,window,cx);self.title.update(cx,|s,cx|s.focus(window,cx));}
            1=>self.new_child(Some(id),window,cx),
            2=>self.move_note(id,None,window,cx),
            4=>self.request_delete(id,window,cx),
            _=>{},
        }
    }
    fn request_delete(&mut self,id:&str,window:&mut Window,cx:&mut Context<Self>) {
        if !self.save(cx) {return;}
        if let Ok(note)=self.db.get(id) {
            let count=self.tree.subtree_count(id);
            window.focus(&self.delete_focus);
            self.pending_delete=Some(PendingDelete {id:note.id,title:note.title,revision:note.revision,count});
            self.note_menu.update(cx,|m,cx|m.dismiss(cx));self.note_context_menu.update(cx,|m,cx|m.dismiss(cx));
            cx.notify();
        }
    }
    fn confirm_delete(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        let Some(pending)=self.pending_delete.take() else {return;};
        match self.db.delete_subtree(&pending.id,pending.revision) {
            Ok(())=> {
                self.refresh_tree();
                if self.selected.as_ref().is_some_and(|n|self.tree.note_index(&n.id).is_none()) {
                    self.dirty=false;
                    if let Some(next)=self.notes.first().and_then(|n|self.db.select(&n.id).ok()) {self.load(next,window,cx);}
                    else {self.selected=None;let _=self.db.set_setting("selected_note","");}
                }
                self.note_menu_target=None;window.focus(&self.tree_focus);
            }
            Err(e)=>self.error=Some(e.to_string()),
        }
        cx.notify();
    }
    fn update_tree_drop(&mut self,drag:&NoteDrag,row:crate::note_tree::TreeRow,event:&DragMoveEvent<NoteDrag>,window:&mut Window,cx:&mut Context<Self>) {
        let position=event.event.position;
        self.drag_pointer=Some(position);
        if !event.bounds.contains(&position) {return;}
        let depth=crate::sidebar_drag::desired_depth(row.depth,f32::from(position.x-event.bounds.left()));
        let next=crate::sidebar_drag::plan(&self.tree,&self.notes,&drag.id,row,f32::from(position.y-event.bounds.top())/f32::from(event.bounds.size.height).max(1.),depth);
        if next!=self.tree_drop {
            self.hover_expand=next.as_ref().filter(|d|d.edge==Edge::Inside).and_then(|d|d.parent.as_ref().or(d.group.as_ref())).map(|id|(id.clone(),Instant::now()));
            self.tree_drop=next;cx.notify();
        }
        self.schedule_drag_frame(window,cx);
    }
    fn schedule_drag_frame(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if self.drag_frame_pending {return;}
        self.drag_frame_pending=true;let panel=cx.entity().downgrade();
        window.on_next_frame(move |window,cx| {let _=panel.update(cx,|this,cx| {this.drag_frame_pending=false;this.step_tree_drag(window,cx);});});
    }
    fn step_tree_drag(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if !cx.has_active_drag() {self.tree_drop=None;self.hover_expand=None;self.drag_pointer=None;self.drag_source=None;cx.notify();return;}
        let mut keep_running=false;
        if let Some((id,since))=self.hover_expand.clone() {
            if since.elapsed()>=Duration::from_millis(500) {self.hover_expand=None;if !self.tree.expanded.contains(&id) {self.toggle_note(&id,cx);}}
            else {keep_running=true;}
        }
        let elapsed=self.drag_frame_at.elapsed().as_secs_f32().clamp(0.001,0.064);self.drag_frame_at=Instant::now();
        if let Some(pointer)=self.drag_pointer {
            let bounds=self.sidebar_scroll.viewport_bounds();
            if bounds.contains(&pointer) {
                let top=f32::from(pointer.y-bounds.top());let bottom=f32::from(bounds.bottom()-pointer.y);
                let speed=if top<36. {-(36.-top)/36.*280.} else if bottom<36. {(36.-bottom)/36.*280.} else {0.};
                if speed!=0. {
                    self.sidebar_scroll.scroll_by(px(speed*elapsed));keep_running=true;cx.notify();
                    // Re-evaluate the stationary pointer against measured visible rows only.
                    let start=self.sidebar_scroll.logical_scroll_top().item_ix;
                    for i in start..(start+128).min(self.tree_motion.rows.len()) {
                        let Some(row_bounds)=self.sidebar_scroll.bounds_for_item(i) else {continue;};
                        if row_bounds.top()>bounds.bottom() {break;}
                        if row_bounds.contains(&pointer) {
                            if let Some(source)=self.drag_source.as_deref() {
                                let depth=crate::sidebar_drag::desired_depth(self.tree_motion.rows[i].row.depth,f32::from(pointer.x-row_bounds.left()));
                                self.tree_drop=crate::sidebar_drag::plan(&self.tree,&self.notes,source,self.tree_motion.rows[i].row,f32::from(pointer.y-row_bounds.top())/f32::from(row_bounds.size.height).max(1.),depth);
                            }
                            break;
                        }
                    }
                }
            } else {self.tree_drop=None;self.hover_expand=None;cx.notify();}
        }
        if keep_running {self.schedule_drag_frame(window,cx);}
    }
    fn commit_tree_drop(&mut self,drag:&NoteDrag,window:&mut Window,cx:&mut Context<Self>) {
        let destination=self.tree_drop.take();self.hover_expand=None;self.drag_pointer=None;
        if !self.save(cx) {return;}
        let Some(d)=destination else {cx.notify();return;};
        let Some(revision)=self.db.get(&drag.id).ok().map(|n|n.revision) else {return;};
        match self.db.relocate_note(&drag.id,d.parent.as_deref(),d.group.as_deref(),d.anchor.as_deref(),d.edge==Edge::Before,revision) {
            Ok(note)=>{self.refresh_tree();self.reveal_note(&drag.id);if self.selected.as_ref().is_some_and(|n|n.id==drag.id) {self.load(note,window,cx);}}
            Err(e)=>self.error=Some(e.to_string()),
        }
        cx.notify();
    }
    fn move_note(&mut self, id: &str, parent: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.save(cx) { return; }
        let Some(revision) = self.notes.iter().find(|n| n.id == id).map(|n| n.revision) else { return; };
        match self.db.move_note(id, parent, revision) {
            Ok(note) => {
                self.refresh_tree();
                self.reveal_note(id);
                if self.selected.as_ref().is_some_and(|n| n.id == id) { self.load(note, window, cx); }
                cx.notify();
            }
            Err(error) => { self.error = Some(error.to_string()); cx.notify(); }
        }
    }
    fn tree_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tree_focus.is_focused(window) || self.tree.rows.is_empty()
            || event.keystroke.modifiers.platform || event.keystroke.modifiers.control || event.keystroke.modifiers.alt { return; }
        let positions:Vec<_>=self.tree.rows.iter().enumerate().filter_map(|(i,r)| r.is_page().then_some(i)).collect();
        if positions.is_empty() {return;}
        let current = self.tree.rows.iter().position(|r| r.is_page() && self.selected.as_ref().is_some_and(|n| n.id == self.notes[r.index].id)).unwrap_or(positions[0]);
        let note_position=positions.iter().position(|i| *i==current).unwrap_or(0);
        let row = self.tree.rows[current];
        let note = self.notes[row.index].clone();
        let target = match event.keystroke.key.as_str() {
            "up" => Some(positions[note_position.saturating_sub(1)]),
            "down" => Some(positions[(note_position+1).min(positions.len()-1)]),
            "right" if !self.tree.expanded.contains(&note.id) => {
                self.toggle_note(&note.id,cx); None
            }
            "right" if row.has_children => Some(positions[(note_position+1).min(positions.len()-1)]),
            "left" if self.tree.expanded.contains(&note.id) => { self.toggle_note(&note.id,cx); None }
            "left" => note.parent_id.as_ref().and_then(|id| self.tree.rows.iter().position(|r| r.is_page() && self.notes[r.index].id == *id)),
            "enter" => { self.editing=true; self.editor.update(cx, |e,cx| e.focus(window,cx)); None }
            _ => return,
        };
        if let Some(target) = target {
            let id = self.notes[self.tree.rows[target].index].id.clone();
            self.select(&id,window,cx);
        }
        cx.stop_propagation();
    }
    fn render_group_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let group=self.tree.groups[index].clone();let id=group.id.clone();
        let expanded=self.tree.expanded.contains(&id);
        let hovered=self.hovered_note.as_deref()==Some(id.as_str());
        let owns_menu=self.group_menu_target.as_deref()==Some(id.as_str());
        let hover_amount=self.interaction(format!("hover:{id}"),hovered);
        let expand_amount=self.interaction(format!("expand:{id}"),expanded);
        let hover_id=id.clone();let toggle_id=id.clone();let child_id=id.clone();let menu_id=id.clone();
        let title:AnyElement=if self.renaming_group.as_deref()==Some(id.as_str()) {
            Input::new(&self.group_title).unstyled().pad_x(0.).height(24.).text_size(12.).into_any_element()
        } else {div().truncate().text_xs().font_weight(FontWeight::MEDIUM).child(group.title).into_any_element()};
        let row=div().id(SharedString::from(format!("sidebar-group-{id}"))).relative()
            .h(px(30.)).w_full().min_w_0().px(px(8.)).rounded(px(6.)).flex().items_center().gap_1()
            .text_color(neutral(0xa3a3a3)).cursor_pointer().bg(Hsla::from(neutral(0x2a2a2a)).opacity(hover_amount))
            .on_hover(cx.listener(move |this,inside:&bool,_,cx| {
                if *inside {this.hovered_note=Some(hover_id.clone());}
                else if this.hovered_note.as_deref()==Some(hover_id.as_str()) {this.hovered_note=None;}
                cx.notify();
            }))
            .child(div().flex_1().min_w_0().flex().items_center().gap_1()
                .child(div().min_w_0().when(self.renaming_group.as_deref()==Some(id.as_str()),|d| d.flex_1()).child(title))
                .child(div().size(px(16.)).flex_none().flex().items_center().justify_center()
                    .opacity(hover_amount)
                    .child(svg().path("iconoir/regular/nav-arrow-right.svg").with_transformation(Transformation::rotate(radians(std::f32::consts::FRAC_PI_2*expand_amount)))
                        .size(px(12.)).text_color(neutral(0x858585)))))
            .when(hover_amount>0. || (self.group_menu_open && owns_menu), |d| {
                d.child(div().opacity(if self.group_menu_open && owns_menu {1.} else {hover_amount}).flex_none().flex().gap(px(2.))
                    .child(div().id(SharedString::from(format!("group-new-{child_id}"))).size(px(20.)).rounded(px(4.))
                        .flex().items_center().justify_center().hover(|s| s.bg(neutral(0x414141)))
                        .child(svg().path("iconoir/regular/plus.svg").size(px(16.)).text_color(neutral(0xa3a3a3)))
                        .on_click(cx.listener(move |this,_,window,cx| {this.new_group_note(&child_id,window,cx);cx.stop_propagation();})))
                    .when(hovered || (self.group_menu_open && owns_menu), |d| d.child(div()
                        .capture_any_mouse_down(cx.listener(move |this,_,_,_| this.group_menu_target=Some(menu_id.clone())))
                        .child(self.group_menu.clone()))))
            })
            .on_click(cx.listener(move |this,_,window,cx| {
                if this.renaming_group.as_deref()!=Some(toggle_id.as_str()) {
                    window.focus(&this.tree_focus);this.toggle_note(&toggle_id,cx);
                }
            }))
;
        #[cfg(test)]
        let row=row.child(self.measure_page(&format!("group-{id}")));
        div().h(px(31.)).w_full().min_w_0().child(row).into_any_element()
    }
    fn render_motion_row(&mut self,index:usize,cx:&mut Context<Self>)->AnyElement {
        let entry=&self.tree_motion.rows[index];let row=entry.row;let amount=entry.amount.value();
        let owner=if let Some(g)=row.group {format!("{}{}",if row.empty {"empty:"} else {""},self.tree.groups[g].id)} else if !row.spacer {format!("{}{}",if row.empty {"empty:"} else {""},self.notes[row.index].id)} else {String::new()};
        let context_id=owner.clone();
        let destination=self.tree_drop.as_ref().filter(|d|d.owner==owner).cloned();
        let content=self.render_tree_row(row,cx);
        let mut surface=div().relative().h(px(31.*amount)).w_full().min_w_0().overflow_hidden().opacity(amount)
            .child(content)
            .on_drag_move(cx.listener(move |this,event:&DragMoveEvent<NoteDrag>,window,cx| {let drag=event.drag(cx).clone();this.update_tree_drop(&drag,row,event,window,cx);}))
            .on_drop(cx.listener(|this,drag:&NoteDrag,window,cx|{this.commit_tree_drop(drag,window,cx);cx.stop_propagation();}));
        if let Some(d)=destination {
            surface=surface.child(div().absolute().left(px(8.+(d.depth as f32*12.).min(72.))).right(px(8.))
                .top(px(if d.edge==Edge::Before {0.} else {29.})).h(px(2.)).bg(neutral(0xc4c4c4)));
            if d.edge==Edge::Inside {surface=surface.bg(neutral(0x303030)).child(div().absolute().right(px(8.)).top(px(4.)).px_1().rounded_sm().text_size(px(10.)).bg(neutral(0x303030)).text_color(neutral(0xbcbcbc)).child("Dentro"));}
            #[cfg(test)] {surface=surface.child(self.measure_page("tree-drop-preview"));}
        }
        if row.is_page() {
            surface=surface.on_mouse_down(MouseButton::Right,cx.listener(move |this,_,_,cx| {
                this.note_menu_target=Some(context_id.clone());this.note_menu.update(cx,|m,cx|m.dismiss(cx));this.dismiss_page_popup();cx.notify();
            }));
            ContextMenu::new(&self.note_context_menu).detached().w_full().child(surface).into_any_element()
        } else {surface.into_any_element()}
    }
    fn render_tree_row(&mut self, row: crate::note_tree::TreeRow, cx: &mut Context<Self>) -> AnyElement {
        if row.spacer {return div().h(px(31.)).w_full().into_any_element();}
        if row.empty {
            let empty=div().h(px(31.)).w_full().min_w_0().relative()
                .flex().items_center().pl(px(38. + (row.depth as f32 * 12.).min(72.))).pr(px(8.))
                .text_size(px(14.)).text_color(neutral(0x858585)).child("Vazio");
            #[cfg(test)] let empty=empty.child(self.measure_page(&format!("empty-{}",row.group.map(|i|self.tree.groups[i].id.as_str()).unwrap_or_else(||self.notes[row.index].id.as_str()))));
            return empty.into_any_element();
        }
        if let Some(index)=row.group {return self.render_group_row(index,cx);}
        let note = &self.notes[row.index];
        let id = note.id.clone();
        let title = note.title.clone();
        let page_icon = note.icon.clone();
        let active = self.selected.as_ref().is_some_and(|n| n.id == id);
        let expanded = self.tree.expanded.contains(&id);
        let hovered = self.hovered_note.as_deref() == Some(id.as_str());
        let owns_menu = self.note_menu_target.as_deref() == Some(id.as_str());
        let hover_amount=self.interaction(format!("hover:{id}"),hovered);
        let expand_amount=self.interaction(format!("expand:{id}"),expanded);
        let select_id = id.clone();
        let toggle_id = id.clone();
        let child_id = id.clone();
        let hover_id = id.clone();
        let menu_id = id.clone();
        let drag = NoteDrag { id:id.clone(), title:title.clone() };
        let panel=cx.entity().downgrade();
        let item = div()
            .id(SharedString::from(format!("tree-note-{id}")))
            .relative().w_full().h(px(30.)).min_w_0()
            .flex().items_center()
            .pl(px(8. + (row.depth as f32 * 12.).min(72.))).pr(px(8.)).py(px(5.))
            .rounded(px(6.)).cursor_pointer()
            // Expansion is structure, not selection or hover: it never changes the background.
            .when(active, |s| s.bg(neutral(0x232323)))
            .bg(Hsla::from(neutral(if active {0x232323+((7.*hover_amount) as u32)*0x010101} else {0x2a2a2a})).opacity(if active {1.} else {hover_amount}))
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside { this.hovered_note = Some(hover_id.clone()); }
                else if this.hovered_note.as_ref() == Some(&hover_id) { this.hovered_note = None; }
                cx.notify();
            }))
            .child(
                // One 22px slot replaces the old separate chevron + page columns.
                div().w(px(22.)).h(px(20.)).mr(px(8.)).flex_none()
                    .flex().items_center().justify_center()
                    .child(
                        div().id(SharedString::from(format!("tree-toggle-{id}")))
                            .size(px(20.)).flex_none().flex().items_center().justify_center().rounded(px(4.))
                            .when(hovered, |d| d.hover(|s| s.bg(neutral(0x414141))))
                            .when(!hovered && page_icon.is_some(), |d| d.child(
                                crate::icon_picker::render_icon(page_icon.as_deref().unwrap_or_default(),SIDEBAR_PAGE_ICON_SIZE)))
                            .when(hovered || page_icon.is_none(), |d| d.child(svg().path(if hovered {
                                "iconoir/regular/nav-arrow-right.svg"
                            } else { "iconoir/regular/page.svg" })
                                .with_transformation(Transformation::rotate(radians(if hovered {std::f32::consts::FRAC_PI_2*expand_amount} else {0.})))
                                .size(px(if hovered {12.} else {SIDEBAR_PAGE_ICON_SIZE})).text_color(neutral(0xa3a3a3))))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                window.focus(&this.tree_focus);
                                this.toggle_note(&toggle_id,cx);
                                cx.stop_propagation();
                            })),
                    ),
            )
            .child(div().flex_1().min_w_0().text_size(px(14.)).font_weight(FontWeight::NORMAL)
                .text_color(if active {neutral(0xeeeeee)} else {neutral(0xbcbcbc)}).truncate().child(title))
            .when(hover_amount>0. || (self.note_menu_open && owns_menu), |d| {
                d.child(
                    div().opacity(if self.note_menu_open && owns_menu {1.} else {hover_amount}).flex_none().flex().items_center().gap(px(2.)).pl(px(3.))
                        .child(
                            div().id(SharedString::from(format!("tree-new-{id}")))
                                .size(px(20.)).flex().items_center().justify_center().rounded(px(4.))
                                .hover(|s| s.bg(neutral(0x414141)))
                                .child(svg().path("iconoir/regular/plus.svg").size(px(16.)).text_color(neutral(0xa3a3a3)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.new_child(Some(&child_id),window,cx);
                                    cx.stop_propagation();
                                })),
                        )
                        .when(hovered || (self.note_menu_open && owns_menu), |d| d.child(
                            div().id(SharedString::from(format!("tree-menu-{id}")))
                                .capture_any_mouse_down(cx.listener(move |this, _, _, _| {
                                    this.note_menu_target = Some(menu_id.clone());
                                }))
                                .child(self.note_menu.clone()),
                        )),
                )
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.tree_focus);
                this.select(&select_id,window,cx);
            }))
            .on_drag(drag, move |drag,_,_,cx| {let _=panel.update(cx,|this,cx| {this.tree_drop=None;this.hover_expand=None;this.drag_source=Some(drag.id.clone());this.drag_frame_at=Instant::now();this.sync_tree_motion(false);cx.notify();});cx.stop_propagation();cx.new(|_|drag.clone())})
;
        #[cfg(test)]
        let item = item.child({
            let geometry=self.tree_row_bounds.clone();
            let measured_id=id.clone();
            canvas(move |bounds,_,_| { geometry.borrow_mut().insert(measured_id.clone(),bounds); },|_,_,_,_| {})
                .absolute().size_full()
        });
        // Uniform pitch is 31px: 30px item + the reference's 1px inter-row gap.
        div().w_full().h(px(31.)).child(item).into_any_element()
    }
    fn set_page_icon(&mut self, icon: Option<&str>, cx: &mut Context<Self>) {
        if let Some(note)=&self.selected {
            match self.db.set_icon(&note.id,icon) {
                Ok(()) => { self.page_icon=icon.map(str::to_owned); self.remember_icon(icon,cx); self.refresh_tree(); self.page_action=None; }
                Err(e) => self.error=Some(e.to_string()),
            }
            cx.notify();
        }
    }
    fn open_icon_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value=self.page_icon.clone();
        self.icon_picker.update(cx,|picker,cx| picker.reset(value,window,cx));
        self.page_action=Some(PageAction::Icon);cx.notify();
    }
    fn remember_icon(&mut self,value:Option<&str>,cx:&mut Context<Self>) {
        if let Some(value)=value {
            let mut recent:Vec<String>=self.db.setting("recent_page_icons").ok().flatten()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            recent.retain(|item| item!=value);recent.insert(0,value.to_owned());recent.truncate(30);
            if let Ok(json)=serde_json::to_string(&recent) {let _=self.db.set_setting("recent_page_icons",&json);}
            self.icon_picker.update(cx,|picker,cx| picker.set_recent(recent,cx));
        }
    }
    fn choose_icon(&mut self,cx:&mut Context<Self>) {
        let Some(id)=self.selected.as_ref().map(|n| n.id.clone()) else {return;};
        let prompt=cx.prompt_for_paths(PathPromptOptions {files:true,directories:false,multiple:false,prompt:Some("Escolher ícone da página".into())});
        self.page_action=None;cx.notify();
        cx.spawn(async move |this,cx| {
            if let Ok(Ok(Some(paths)))=prompt.await {
                if let Some(path)=paths.first() {
                    let path=path.clone();let _=this.update(cx,|this,cx| {
                        match this.db.import_icon(&id,&path) {
                            Ok(value) => {
                                if this.selected.as_ref().is_some_and(|n| n.id==id) {this.page_icon=Some(value.clone());}
                                this.remember_icon(Some(&value),cx);this.refresh_tree();
                            }
                            Err(error) => this.error=Some(error.to_string()),
                        }
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
    fn choose_cover(&mut self, cx: &mut Context<Self>) {
        let Some(id)=self.selected.as_ref().map(|n| n.id.clone()) else { return; };
        let prompt=cx.prompt_for_paths(PathPromptOptions {
            files:true,directories:false,multiple:false,prompt:Some("Escolher imagem de capa".into()),
        });
        cx.spawn(async move |this,cx| {
            if let Ok(Ok(Some(paths)))=prompt.await {
                if let Some(path)=paths.first() {
                    let allowed=path.extension().and_then(|s| s.to_str()).is_some_and(|s|
                        ["png","jpg","jpeg","webp","gif"].contains(&s.to_ascii_lowercase().as_str()));
                    let _=this.update(cx, |this,cx| {
                        if !allowed { this.error=Some("Escolha uma imagem PNG, JPG, WebP ou GIF.".into()); }
                        else {
                            match this.db.import_cover(&id,path) {
                                Ok(value) => if this.selected.as_ref().is_some_and(|n| n.id==id) { this.page_cover=Some(value); },
                                Err(e) => this.error=Some(e.to_string()),
                            }
                        }
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
    fn dismiss_page_popup(&mut self) {
        self.page_action=None;self.painted_page_action=None;
        self.popup_motion=empire_ui::motion::Tween::new(0.);
    }
    fn render_page_actions(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let alpha=self.interaction("title-actions".into(),self.title_hovered || self.page_action.is_some());
        let mut bar=div().id("page-actions").absolute().left(px(58.)).right_0().top_0()
            .h(px(32.)).flex().items_center().gap(px(4.)).text_size(px(14.))
            .text_color(neutral(0x858585)).font_weight(FontWeight::NORMAL)
            .opacity(alpha);
        for (index,(icon,label)) in [
            ("emoji.svg",if self.page_icon.is_some() {"Alterar ícone"} else {"Adicionar ícone"}),
            ("media-image.svg",if self.page_cover.is_some() {"Alterar capa"} else {"Adicionar capa"}),
        ].into_iter().enumerate() {
            bar=bar.child(div().id(("page-action",index)).h(px(32.)).px(px(8.))
                .min_w_0().flex().items_center().gap(px(6.)).rounded(px(6.)).cursor_pointer()
                .hover(|s| s.bg(neutral(0x2a2a2a)).text_color(neutral(0xbcbcbc)))
                .child(svg().path(format!("iconoir/regular/{icon}")).size(px(16.)).flex_none()
                    .text_color(neutral(0x858585)))
                .child(div().min_w_0().truncate().child(label))
                .on_click(cx.listener(move |this,_,window,cx| {
                    match index {
                        0 => if this.page_action==Some(PageAction::Icon) {this.page_action=None;} else {this.open_icon_picker(window,cx);},
                        1 => this.page_action=if this.page_action==Some(PageAction::Cover) {None} else {Some(PageAction::Cover)},
                        _ => {},
                    }
                    cx.stop_propagation(); cx.notify();
                })));
        }
        #[cfg(test)] { bar=bar.child(self.measure_page("actions")); }
        bar.into_any_element()
    }
    #[cfg(test)]
    fn measure_page(&self, key: &str) -> AnyElement {
        let geometry=self.tree_row_bounds.clone(); let key=key.to_owned();
        canvas(move |bounds,_,_| {geometry.borrow_mut().insert(key.clone(),bounds);},|_,_,_,_| {})
            .absolute().top_0().left_0().size_full().into_any_element()
    }
    fn render_page_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let is_cover=self.painted_page_action==Some(PageAction::Cover);
        let popup_width: f32=if is_cover {520.} else {408.};
        let mut popup=div().id("page-action-popup").relative().opacity(self.popup_motion.value())
            .w(px(popup_width.min(f32::from(window.viewport_size().width)-24.))).p_3().flex().flex_col().gap_2().rounded(px(8.))
            .bg(neutral(0x202020)).border_1().border_color(neutral(0x414141)).shadow_lg()
            .text_size(px(14.)).font_weight(FontWeight::NORMAL).occlude()
            .on_mouse_down_out(cx.listener(|this,_,_,cx| { this.page_action=None; cx.notify(); }))
            .on_key_down(cx.listener(|this,event: &KeyDownEvent,_,cx| {
                if event.keystroke.key=="escape" { this.page_action=None; cx.stop_propagation(); cx.notify(); }
            }));
        if self.painted_page_action==Some(PageAction::Icon) {
            popup=popup.child(self.icon_picker.clone());
        } else if is_cover {
            popup=popup.child(div().flex().items_center().gap_2()
                .child(div().h(px(30.)).px_2().flex().items_center().border_b_2().border_color(neutral(0xeeeeee)).child("Galeria"))
                .child(Button::new("upload-cover","Carregar").variant(ButtonVariant::Ghost).size(ButtonSize::Sm)
                    .on_click(cx.listener(|this,_,_,cx| {this.page_action=None;this.choose_cover(cx);cx.notify();})))
                .child(div().flex_1())
                .when(self.page_cover.is_some(), |d| d.child(
                    Button::new("gallery-remove-cover","Remover").variant(ButtonVariant::Ghost).size(ButtonSize::Sm)
                        .on_click(cx.listener(|this,_,_,cx| this.set_page_cover(None,cx))))));
            popup=popup.child(div().text_color(neutral(0xa3a3a3)).child("Cor e gradiente"));
            let mut gallery=div().flex().flex_col().gap_2();
            for presets in crate::covers::PRESETS.chunks(4) {
                let mut row=div().flex().gap_2();
                for preset in presets {
                    let value=crate::covers::stored_value(preset);
                    let selected=self.page_cover.as_deref()==Some(value.as_str());
                    let tile=div().id(SharedString::from(format!("cover-preset-{}",preset.id)))
                        .group(preset.id).relative().flex_1().min_w_0().h(px(60.)).rounded(px(6.)).cursor_pointer()
                        .bg(cover_background(preset)).border_2()
                        .border_color(if selected {neutral(0xeeeeee).into()} else {transparent_black()})
                        .hover(|s| s.border_color(neutral(0xeeeeee)))
                        .child(div().absolute().left(px(4.)).top(px(4.)).px_1().rounded(px(3.))
                            .text_size(px(11.)).text_color(rgb(0xffffff)).bg(rgba(0x161616a0))
                            .opacity(0.).group_hover(preset.id, |d| d.opacity(1.)).child(preset.name))
                        .when(selected, |d| d.child(div().absolute().right(px(6.)).bottom(px(6.))
                            .size(px(20.)).flex().items_center().justify_center().rounded_full().bg(rgba(0x161616b0))
                            .child(svg().path("iconoir/regular/check.svg").size(px(14.)).text_color(rgb(0xffffff)))))
                        .on_click(cx.listener(move |this,_,_,cx| this.set_page_cover(Some(&value),cx)));
                    #[cfg(test)]
                    let tile=tile.child(self.measure_page(&format!("cover-{}",preset.id)));
                    row=row.child(tile);
                }
                gallery=gallery.child(row);
            }
            popup=popup.child(ScrollArea::new("cover-gallery",&self.reader_scroll)
                .h(px((f32::from(window.viewport_size().height)-160.).clamp(60.,220.)))
                .child(gallery));
        }
        #[cfg(test)] { popup=popup.child(self.measure_page("popup")); }
        div().absolute().left(px(58.)).top(px(38.))
            .child(anchored().snap_to_window_with_margin(px(12.)).child(popup)).into_any_element()
    }
    fn set_page_cover(&mut self, value: Option<&str>, cx: &mut Context<Self>) {
        if let Some(note)=&self.selected {
            match self.db.set_cover(&note.id,value) {
                Ok(()) => {self.page_cover=value.map(str::to_owned);self.page_action=None;},
                Err(e) => self.error=Some(e.to_string()),
            }
            cx.notify();
        }
    }
    fn render_cover(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let value=self.page_cover.clone().unwrap_or_default();
        let mut cover=div().id("page-cover").group("page-cover").relative().w_full()
            .absolute().top_0().left_0().right_0().h(px(COVER_HEIGHT)).overflow_hidden();
        if let Some(preset)=crate::covers::preset(&value) {
            cover=cover.bg(cover_background(preset));
        } else {
            cover=cover.child(img(std::path::PathBuf::from(value)).w_full().h_full().object_fit(ObjectFit::Cover));
        }
        cover=cover.child(div().absolute().right(px(8.)).bottom(px(8.))
            .opacity(0.).group_hover("page-cover", |d| d.opacity(1.))
            .child(Button::new("change-cover","Alterar capa").size(ButtonSize::Sm)
                .on_click(cx.listener(|this,_,_,cx| {this.page_action=Some(PageAction::Cover);cx.notify();}))));
        #[cfg(test)] {cover=cover.child(self.measure_page("cover-preview"));}
        cover.into_any_element()
    }
    fn render_page_icon(&mut self, overlap: bool, cx: &mut Context<Self>) -> AnyElement {
        let icon=div().id("page-icon").text_size(px(PAGE_ICON_SIZE)).font_family("Apple Color Emoji")
            .cursor_pointer().child(crate::icon_picker::render_icon(self.page_icon.as_deref().unwrap_or_default(),PAGE_ICON_SIZE))
            .when(overlap, |d| d.absolute().left(px(58.)).top(px(-78.-PAGE_ICON_SIZE/2.))
                .size(px(PAGE_ICON_SIZE)).line_height(relative(1.)).flex().items_center().justify_center())
            .when(!overlap, |d| d.mb_3())
            .on_click(cx.listener(|this,_,window,cx| this.open_icon_picker(window,cx)));
        #[cfg(test)]
        let icon=icon.relative().when(overlap, |d| d.absolute())
            .child(self.measure_page("page-icon"));
        icon.into_any_element()
    }
    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !cx.has_active_drag() {
            if self.tree_drop.take().is_some() {cx.notify();}
            self.hover_expand=None;self.drag_pointer=None;self.drag_source=None;
        }
        if macos::take_show() {self.visibility(true,window,cx);cx.activate(true);}
        if macos::take_toggle() {self.visibility(!self.visible || macos::is_minimized(window),window,cx);}
        if self.dirty
            && self.error.is_none()
            && self.last_edit.elapsed() > Duration::from_millis(if crate::sync_storage::enabled(&self.db) {80} else {350})
        {
            self.save(cx);
        }
        if crate::sync_storage::enabled(&self.db) {
            if let Some(note)=self.selected.clone() {
                if !self.editor.read(cx).has_shared_document() {
                    if let Ok(Some(state))=crate::sync_storage::editor_state_at(&self.db,&note.id,note.revision) {self.editor.update(cx,|editor,_| {let _=editor.bind_shared_document(&state);});}
                }
                if !self.dirty && self.editor.read(cx).value().as_ref()!=note.markdown {self.apply_shared_note(note,window,cx);}
            }
        }
        if self.last_sync.elapsed() < Duration::from_millis(if crate::sync_storage::enabled(&self.db) {100} else {500}) {
            return;
        }
        self.last_sync = Instant::now();
        let mcp_connected = crate::mcp_presence::connected(self.db.mcp_sessions_dir()).ok();
        if self.mcp_connected != mcp_connected {
            self.mcp_connected = mcp_connected;
            cx.notify();
        }
        if let Ok(request) = self.db.setting("panel_request") {
            if request != self.panel_request {
                self.panel_request = request;
                self.visibility(true, window, cx);
            }
        }
        let b = window.bounds();
        let bounds = (
            f32::from(b.origin.x),
            f32::from(b.origin.y),
            f32::from(b.size.width),
            f32::from(b.size.height),
        );
        if self.last_bounds != Some(bounds) {
            if let Err(e) = self
                .db
                .set_setting("window_bounds", &serde_json::to_string(&bounds).unwrap())
            {
                self.error = Some(e.to_string());
            }
            self.last_bounds = Some(bounds);
        }
        let data_version = self.db.data_version().ok();
        if data_version.is_some() && data_version == self.last_data_version && self.error.is_none() { return; }
        self.last_data_version = data_version;
        if let Ok(p)=crate::preferences::Preferences::read(&self.db) {
            if self.last_preferences.as_ref()!=Some(&p) {
                self.last_preferences=Some(p.clone());
                if self.opacity.read(cx).value()!=p.opacity {
                    self.opacity.update(cx,|slider,cx| slider.set_values(vec![p.opacity],cx));
                    self.pending_opacity=None;
                    macos::opacity(window,p.opacity/100.);
                }
                if self.content_width.read(cx).value()!=p.content_width {
                    self.content_width.update(cx,|slider,cx| slider.set_values(vec![p.content_width],cx));
                }
                self.font_size=p.font_size;self.sidebar=p.sidebar;self.sidebar_width=p.sidebar_width;
                if self.pinned!=p.always_on_top {self.pinned=p.always_on_top;macos::set_pinned(window,self.pinned);}
                let light=p.theme=="light";
                if self.light_mode!=light {
                    self.light_mode=light;crate::app_theme::apply(light,cx);
                    self.editor.update(cx,|editor,cx| editor.refresh_theme(cx));
                    macos::appearance(window,light);
                    self.opacity_popover.update(cx,|_,cx| cx.notify());
                    self.width_popover.update(cx,|_,cx| cx.notify());
                }
                let font=crate::assets::CONTENT_FONTS.iter().position(|f| f.0==p.content_font).unwrap_or(0);
                if self.content_font!=font {
                    self.content_font=font;
                    self.font_menu.update(cx,|menu,cx| menu.select_radio(font,cx));
                }
                cx.notify();
            }
        }
        match self.db.list_summaries() {
            Ok(notes) => {
                if let Some(note)=&self.selected {
                    (self.page_icon,self.page_cover)=self.db.presentation(&note.id).unwrap_or_default();
                                cx.notify();
                }
                let groups=self.db.list_groups().unwrap_or_default();
                let changed = notes != self.notes || groups!=self.tree.groups;
                if changed {
                    self.notes = notes;
                    self.tree.groups=groups;
                    self.tree.set_notes(&self.notes);
                    self.sync_tree_motion(false);
                    cx.notify();
                }
                if self.dirty && crate::sync_storage::enabled(&self.db) {self.save(cx);}
                if self.dirty || self.error.is_some() {return;}
                let selected_id = self.db.setting("selected_note").ok().flatten();
                let desired = self
                    .notes
                    .iter()
                    .find(|n| Some(&n.id) == selected_id.as_ref())
                    .or_else(|| {
                        self.notes
                            .iter()
                            .find(|n| self.selected.as_ref().is_some_and(|s| s.id == n.id))
                    })
                    .or(self.notes.first());
                let requires_load = match (desired, self.selected.as_ref()) {
                    (Some(a), Some(b)) => a.id != b.id || a.revision != b.revision,
                    (None, None) => false,
                    _ => true,
                };
                if requires_load {
                    if let Some(note) = desired.and_then(|n| self.db.get(&n.id).ok()) {
                        if crate::sync_storage::enabled(&self.db) && self.selected.as_ref().is_some_and(|selected|selected.id==note.id) {
                            self.apply_shared_note(note,window,cx);
                        } else {self.load(note,window,cx);}
                    } else {
                        self.selected = None;
                        cx.notify();
                    }
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }

    }
}
impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar_motion.set(if self.sidebar {1.} else {0.},220);
        let sidebar_amount=self.sidebar_motion.value();
        let tree_moving=self.tree_motion.moving();
        let removed=self.tree_motion.advance();
        if removed {
            let offset=self.sidebar_scroll.logical_scroll_top();
            self.sidebar_scroll.splice(0..self.sidebar_scroll.item_count(),self.tree_motion.rows.len());
            self.sidebar_scroll.scroll_to(offset);
        } else {
            for range in self.tree_motion.changed_ranges() {
                let count=range.len();self.sidebar_scroll.splice(range,count);
            }
        }
        if let Some(action)=self.page_action {self.painted_page_action=Some(action);}
        self.popup_motion.set(if self.page_action.is_some() {1.} else {0.},160);
        if self.page_action.is_none() && !self.popup_motion.moving() {self.painted_page_action=None;}
        #[cfg(test)] {self.rendered_tree_rows=0;}
        self.editor
            .update(cx, |s, cx| {
                // The 36px footer is outside the scroll viewport. Keep half a viewport
                // of clickable writing space after the last block, including on resize.
                s.set_end_space(((f32::from(window.viewport_size().height) - 36.).max(0.)) * 0.5, cx);
                s.set_font_size(self.font_size, cx);
                s.set_font_family(crate::assets::CONTENT_FONTS[self.content_font].1,cx);
            });
        let mut body = div()
            .id("panel-body")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col();
        if self.selected.is_none() {
            body = body
                .items_center()
                .justify_center()
                .gap_3()
                .child(div().text_2xl().child("Seu próximo pensamento começa aqui"))
                .child(
                    div()
                        .text_sm()
                        .text_color(neutral(0xa3a3a3))
                        .child("Crie uma nota ou peça ao seu agente pelo MCP."),
                )
                .child(
                    Button::new("empty-new", "Criar nota")
                        .on_click(cx.listener(|this, _, window, cx| this.new_note(window, cx))),
                );
        } else {
            let page_actions=self.render_page_actions(cx);
            let page_popup=self.painted_page_action.map(|_| self.render_page_popup(window,cx));
            let page_cover=self.page_cover.is_some().then(|| self.render_cover(cx));
            let overlapping_icon=(self.page_icon.is_some() && self.page_cover.is_some())
                .then(|| self.render_page_icon(true,cx));
            let inline_icon=(self.page_icon.is_some() && self.page_cover.is_none())
                .then(|| self.render_page_icon(false,cx));
            #[cfg(test)]
            let column_measure = Some(self.measure_page("document-column").into_any_element());
            #[cfg(not(test))]
            let column_measure: Option<AnyElement> = None;
            body = body.child(
                ScrollArea::new("document-scroll", &self.editor_scroll)
                    .flex_1()
                    .min_h_0()
                    .fade_color(neutral(0x161616).into())
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .flex()
                            .flex_col()
                            // Text retains its 78px inset (20px + 58px for block controls).
                            // The absolute cover spans the container; reserve its height above text.
                            .pl(px(20.))
                            .pr(px(78.))
                            .pt(px(78. + if self.page_cover.is_some() {COVER_HEIGHT} else {0.}))
                            .pb(px(20.))
                            .children(page_cover)
                            .child(
                                div().id("document-column").relative().min_w(px(100.))
                                    .w(relative(self.content_width.read(cx).value()/100.)).mx_auto()
                                    .flex().flex_col().gap_3()
                                    .children(overlapping_icon)
                                    .child(
                                div().id("page-title").group("page-title").relative().mt(px(-38.)).pt(px(38.))
                                    .on_hover(cx.listener(|this,inside: &bool,_,cx| {
                                        if this.title_hovered!=*inside { this.title_hovered=*inside; cx.notify(); }
                                    }))
                                    .w_full()
                                    .min_w_0()
                                    .pl(px(58.))
                                    .font_weight(FontWeight::BOLD)
                                    .children(inline_icon)
                                    .child(page_actions)
                                    .children(page_popup.map(|p| deferred(p).with_priority(5)))
                                    .child(
                                        div().font_family(crate::assets::CONTENT_FONTS[self.content_font].1).child(Input::new(&self.title)
                                            .unstyled()
                                            .pad_x(0.)
                                            .height((self.font_size + 10.) * 1.4 + 2.)
                                            .text_size(self.font_size + 10.)),
                                    ),
                            )
                            .child(self.editor.clone())
                            .children(column_measure)),
                    ),
            );
        }

        #[cfg(test)]
        let theme_measure=Some(self.measure_page("theme-button").into_any_element());
        #[cfg(not(test))]
        let theme_measure: Option<AnyElement>=None;
        #[cfg(test)]
        let font_measure=Some(self.measure_page("font-button").into_any_element());
        #[cfg(not(test))]
        let font_measure: Option<AnyElement>=None;
        let footer = div().id("footer-controls").overflow_x_scroll()
            .h(px(36.))
            .flex_none()
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .border_t_1()
            .border_color(neutral(0x303030))
            .child(self.opacity_popover.clone())
            .child(self.width_popover.clone())
            .child(Button::icon("sync-settings-button","iconoir/regular/cloud-sync.svg").size(ButtonSize::IconSm).variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|this,_,window,cx|this.open_sync_settings(false,window,cx))))
            .child(div().text_xs().text_color(neutral(0x999999)).child(match self.db.setting("sync_status").ok().flatten().as_deref() {
                Some("connected")=>"Synced",Some("connecting")=>"Connecting…",Some("offline")=>"Offline · saved locally",_=>"Local",
            }))
            .child(div().relative().flex_none().child(self.font_menu.clone()).children(font_measure))
            .child(div().relative().flex_none().child(Button::icon("toggle-theme", if self.light_mode {
                    "iconoir/regular/half-moon.svg"
                } else { "iconoir/regular/sun-light.svg" })
                .size(ButtonSize::IconSm).variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|this,_,window,cx| this.toggle_theme(window,cx)))).children(theme_measure))
            .child(div().flex_1())
            .child(
                Button::new("smaller", "A−")
                    .size(ButtonSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.font_size = (this.font_size - 2.).max(14.);
                        let _ = this
                            .db
                            .set_setting("font_size", &this.font_size.to_string());
                        cx.notify();
                    })),
            )
            .child(
                Button::new("larger", "A+")
                    .size(ButtonSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.font_size = (this.font_size + 2.).min(40.);
                        let _ = this
                            .db
                            .set_setting("font_size", &this.font_size.to_string());
                        cx.notify();
                    })),
            )
            .child(
                Button::icon("pin", if self.pinned {
                    "iconoir/solid/pin.svg"
                } else {
                    "iconoir/regular/pin-slash.svg"
                })
                    .size(ButtonSize::IconSm)
                    .variant(if self.pinned { ButtonVariant::Secondary } else { ButtonVariant::Ghost })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.pinned = !this.pinned;
                        macos::set_pinned(window, this.pinned);
                        if let Err(e) = this.db.set_setting("always_on_top", &this.pinned.to_string()) {
                            this.error = Some(e.to_string());
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::icon("quit", "iconoir/regular/log-out.svg")
                    .size(ButtonSize::IconSm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.save(cx) {
                            cx.quit();
                        }
                    })),
            );

        let sidebar_view = div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(neutral(0x1b1b1b))
            .child(
                div()
                    .id("sidebar-window-drag")
                    .relative()
                    .w_full()
                    .h(px(24.))
                    .flex_none()
                    .child(div().absolute().inset_0()
                        .cursor(CursorStyle::OpenHand)
                        .on_mouse_down(MouseButton::Left, |event, window, cx| {
                            macos::start_drag(event, window, cx)
                        }))
                    .child(
                        // The native traffic lights start at y=8 and are 14 px tall.
                        // With the sidebar padding, this centers both controls at y=15.
                        div().id("sidebar").absolute().right_0().top(px(-8.))
                            .size(px(20.)).occlude()
                            .flex().items_center().justify_center().rounded(px(4.))
                            .cursor_pointer().hover(|s| s.bg(neutral(0x414141)))
                            .child(svg().path("iconoir/regular/sidebar-collapse.svg")
                                .size(px(16.)).text_color(neutral(0xa3a3a3)))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx))),
                    ),
            )
            .child(self.note_context_menu.clone())
            .child(
                div()
                    .id("sidebar-heading")
                    .h(px(30.)).px(px(8.)).rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .id("sidebar-drag-handle")
                            .flex_1()
                            .min_w_0()
                            .cursor(CursorStyle::OpenHand)
                            .on_mouse_down(
                                MouseButton::Left,
                                |event, window, cx| macos::start_drag(event, window, cx),
                            )
                            .text_size(px(16.))
                            .text_color(neutral(0xeeeeee))
                            .font_weight(FontWeight::BOLD)
                            .child("SparkPad")
                            .on_drop(cx.listener(|this, drag: &NoteDrag, window, cx| {
                                this.move_note(&drag.id, None, window, cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(div().id("new-group").size(px(20.)).flex_none().rounded(px(4.))
                        .flex().items_center().justify_center().cursor_pointer()
                        .hover(|s| s.bg(neutral(0x414141)))
                        .child(svg().path("iconoir/regular/folder-plus.svg").size(px(16.)).text_color(neutral(0xa3a3a3)))
                        .on_click(cx.listener(|this,_,window,cx| this.create_group(window,cx))))
                    .child(
                        div().id("new-note").size(px(20.)).flex_none()
                            .flex().items_center().justify_center().rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(neutral(0x414141)))
                            .child(svg().path("iconoir/regular/plus.svg").size(px(16.)).text_color(neutral(0xa3a3a3)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.new_note(window, cx)
                            })),
                    ),
            )
            .child(
                div().id("notes-tree").relative().flex_1().min_h_0().min_w_0()
                    .when(self.tree_drop.as_ref().is_some_and(|d|d.owner=="__root-start"),|d|d.child(div().absolute().top_0().left(px(8.)).right(px(8.)).h(px(2.)).bg(neutral(0xc4c4c4))))
                    .on_drag_move(cx.listener(|this,event:&DragMoveEvent<NoteDrag>,window,cx| {
                        this.drag_pointer=Some(event.event.position);
                        if !event.bounds.contains(&event.event.position) {this.tree_drop=None;this.hover_expand=None;cx.notify();}
                        else {
                            let start=this.sidebar_scroll.logical_scroll_top().item_ix;
                            let on_row=(start..(start+128).min(this.tree_motion.rows.len())).any(|i|this.sidebar_scroll.bounds_for_item(i).is_some_and(|b|b.contains(&event.event.position)));
                            if !on_row {
                                let owner=this.tree.last_root().and_then(|i|this.tree.subtree_end(i)).map(|r|format!("{}{}",if r.empty {"empty:"} else {""},this.notes[r.index].id)).unwrap_or_else(||"__root-start".into());
                                this.tree_drop=Some(Destination {owner,parent:None,group:None,anchor:None,edge:Edge::After,depth:0});this.hover_expand=None;cx.notify();
                            }
                        }
                        this.schedule_drag_frame(window,cx);
                    }))
                    .on_drop(cx.listener(|this,drag:&NoteDrag,window,cx| {this.commit_tree_drop(drag,window,cx);cx.stop_propagation();}))
                    .track_focus(&self.tree_focus)
                    .on_key_down(cx.listener(|this,event:&KeyDownEvent,window,cx|this.tree_key(event,window,cx)))
                    .child(list(self.sidebar_scroll.clone(),cx.processor(|this,i:usize,_,cx| {
                        #[cfg(test)] {this.rendered_tree_rows+=1;}
                        this.render_motion_row(i,cx)
                    })).size_full()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(neutral(0xa3a3a3))
                    .child(div().size(px(6.)).rounded_full().bg(
                        if self.mcp_connected == Some(true) { neutral(0xeeeeee) } else { neutral(0x666666) }
                    ))
                    .child(match self.mcp_connected {
                        Some(true) => "MCP conectado",
                        Some(false) => "MCP desconectado",
                        None => "MCP indisponível",
                    }),
            );
        #[cfg(test)]
        let sidebar_view = sidebar_view.relative().child(self.measure_page("sidebar"));
        let content_view = div().size_full().min_w_0().min_h_0().flex().flex_col()
            .child(body).child(footer);
        #[cfg(test)]
        let content_view = content_view.relative().child(self.measure_page("content"));
        let layout: AnyElement = if self.sidebar_motion.moving() {
            div().flex_1().min_h_0().min_w_0().flex()
                .child(div().w(px(self.sidebar_width*sidebar_amount)).h_full().flex_none().overflow_hidden()
                    .child(div().w(px(self.sidebar_width)).h_full().child(sidebar_view)))
                .child(div().flex_1().min_w_0().h_full().child(content_view)).into_any_element()
        } else if self.sidebar {
            let panel = cx.entity().downgrade();
            div().flex_1().min_h_0().min_w_0().child(
                Resizable::horizontal("sidebar-content-split")
                    .child(ResizablePanel::new().size(self.sidebar_width).min_size(180.).max_size(420.)
                        .child(sidebar_view))
                    .child(ResizablePanel::new().min_size(240.).child(content_view))
                    .on_resize(move |sizes, _, cx| {
                        if let Some(width) = sizes.first().copied() {
                            let _ = panel.update(cx, |this, cx| {
                                this.sidebar_width = f32::from(width).clamp(180.,420.);
                                if let Err(error) = this.db.set_setting("sidebar_width", &this.sidebar_width.to_string()) {
                                    this.error = Some(error.to_string());
                                }
                                cx.notify();
                            });
                        }
                    })
            ).into_any_element()
        } else {
            div().flex_1().min_h_0().min_w_0().child(content_view).into_any_element()
        };

        if self.sidebar_motion.moving() || tree_moving || self.popup_motion.moving() || self.interactions.values().any(|t|t.moving()) {
            window.request_animation_frame();
        }

        div()
            .track_focus(&self.focus_handle)
            .key_context("Sparkpad")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded_xl()
            .bg(neutral(0x161616))
            .font_family(crate::assets::FONT_FAMILY)
            .text_color(neutral(0xeeeeee))
            .border_1()
            .border_color(neutral(0x414141))
            .children(self.sync_dialog.as_ref().map(|dialog|deferred(div().absolute().inset_0().flex().items_center().justify_center().p_4().occlude().bg(rgba(0x00000088)).child(dialog.clone())).with_priority(10)))
            .on_action(cx.listener(|this, _: &QuitApp, _, cx| {
                if this.save(cx) {
                    cx.quit();
                }
            }))
            .on_action(cx.listener(|this, _: &ReadMode, window, cx| this.read_mode(window, cx)))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(
                cx.listener(|this, _: &HidePanel, window, cx| {if this.pending_delete.take().is_some() {window.focus(&this.tree_focus);cx.notify();} else {this.visibility(false,window,cx);}}),
            )
            .when(self.error.is_some(), |s| {
                s.child(
                    div()
                        .p_3()
                        .bg(neutral(0x482a27))
                        .text_sm()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(self.error.clone().unwrap_or_default())
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    Button::new("retry-save", "Tentar salvar")
                                        .size(ButtonSize::Sm)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.error = None;
                                            this.save(cx);
                                        })),
                                )
                                .child(
                                    Button::new("keep-copy", "Preservar como nova nota")
                                        .size(ButtonSize::Sm)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let title =
                                                format!("{} (cópia)", this.title.read(cx).value());
                                            let text = this.editor.read(cx).value();
                                            match this.db.create(&title, &text) {
                                                Ok(note) => {
                                                    let _ = this.db.select(&note.id);
                                                    this.refresh_tree();
                                                    this.load(note, window, cx);
                                                }
                                                Err(e) => {
                                                    this.error = Some(e.to_string());
                                                    cx.notify();
                                                }
                                            }
                                        })),
                                ),
                        ),
                )
            })
            .child(layout)
            .child(
                div()
                    .id("window-drag-edge")
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(if self.sidebar { 4. } else { 10. }))
                    .cursor(CursorStyle::OpenHand)
                    .on_mouse_down(MouseButton::Left, |event, window, cx| {
                        macos::start_drag(event, window, cx)
                    }),
            )
            .when(!self.sidebar, |root| {
                root.child(
                    div().absolute().top(px(38.)).left(px(12.)).occlude().child(
                        Button::icon("sidebar", "iconoir/regular/sidebar-collapse.svg")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx))),
                    ),
                )
            })
            .when(self.pending_delete.is_some(),|root| {
                let confirm=div().relative().child(Button::new("confirm-delete-note","Excluir").variant(ButtonVariant::Destructive).on_click(cx.listener(|this,_,window,cx|this.confirm_delete(window,cx))));
                #[cfg(test)] let confirm=confirm.child(self.measure_page("confirm-delete-note"));
                let pending=self.pending_delete.as_ref().unwrap();
                let detail=if pending.count>1 {format!("A nota e suas {} subpáginas serão excluídas. Esta ação não pode ser desfeita.",pending.count-1)} else {"Esta ação não pode ser desfeita.".to_owned()};
                root.child(div().id("delete-note-dialog").track_focus(&self.delete_focus).absolute().size_full().flex().items_center().justify_center().occlude().bg(rgba(0x00000088))
                    .on_key_down(cx.listener(|this,e:&KeyDownEvent,_,cx|{if e.keystroke.key=="escape" {this.pending_delete=None;cx.stop_propagation();cx.notify();}}))
                    .child(div().w(px(380.)).max_w_full().p_5().rounded_lg().bg(neutral(0x202020)).border_1().border_color(neutral(0x414141)).flex().flex_col().gap_3()
                        .child(div().text_lg().font_weight(FontWeight::BOLD).child(format!("Excluir “{}”?",pending.title)))
                        .child(div().text_sm().text_color(neutral(0xa3a3a3)).child(detail))
                        .child(div().flex().justify_end().gap_2()
                            .child(Button::new("cancel-delete-note","Cancelar").variant(ButtonVariant::Ghost).on_click(cx.listener(|this,_,_,cx|{this.pending_delete=None;cx.notify();})))
                            .child(confirm))))
            })

    }
}

pub fn run(db: Store) {
    let app = Application::new().with_assets(crate::assets::Assets);
    app.on_reopen(|_| macos::request_show());
    app.run(move |cx| {
        crate::assets::register_fonts(cx.text_system());
        gpui_component::init(cx);
        crate::block_editor::init(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-enter",ReadMode,None),
            KeyBinding::new("cmd-enter",ReadMode,Some("Sparkpad > Input")),
            KeyBinding::new("cmd-shift-b",ToggleSidebar,None),
            KeyBinding::new("cmd-n",NewNote,None),
            KeyBinding::new("cmd-b",ToggleSidebar,None),
            KeyBinding::new("escape",HidePanel,None),
            KeyBinding::new("cmd-q",QuitApp,None),
        ]);
        if db.list_summaries().is_ok_and(|n| n.is_empty()) {
            let _ = db.create("Bem-vindo ao Sparkpad", "# Espaço para suas ideias\n\nUm bloco de notas mínimo, leve e local.\n\nEscreva aqui. Organize suas notas em páginas e subpáginas. Use **negrito**, *itálico*, listas e títulos sem sair do texto.\n\n## Seu agente também pode ajudar\nPelo MCP, agentes podem criar, organizar e atualizar suas notas.\n\n> Só o necessário para pensar e escrever.");
        }
        let saved_bounds = db.setting("window_bounds").ok().flatten().and_then(|v| serde_json::from_str::<(f32,f32,f32,f32)>(&v).ok());
        let default_bounds = Bounds::centered(None,size(px(780.),px(580.)),cx);
        let bounds = saved_bounds.filter(|(x,y,w,h)| x.is_finite() && y.is_finite() && *w >= 440. && *h >= 320.)
            .map(|(x,y,w,h)| Bounds::new(point(px(x),px(y)),size(px(w),px(h))))
            .filter(|b| cx.displays().iter().any(|d| d.bounds().intersects(b)))
            .unwrap_or(default_bounds);
        let window_handle = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Sparkpad".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(12.), px(8.))),
            }),
            kind: WindowKind::Normal,
            window_background: WindowBackgroundAppearance::Transparent,
            window_min_size: Some(size(px(440.),px(320.))),
            is_resizable: true,
            ..Default::default()
        },move |window,cx| {
            let panel = cx.new(|cx| Panel::new(db,window,cx));
            let weak = panel.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                let _ = weak.update(cx, |panel, cx| panel.visibility(false, window, cx));
                false
            });
            cx.new(|cx| Root::new(panel,window,cx))
        }).expect("Open Sparkpad window");
        let status = macos::StatusItem::new();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let _status = status; // Keep the status item alive for the application's lifetime.
            loop {
                Timer::after(Duration::from_millis(100)).await;
                if window_handle.update(cx,|root,window,cx| {
                    let view = root.view().clone().downcast::<Panel>().expect("Panel root");
                    view.update(cx,|panel,cx| panel.tick(window,cx));
                }).is_err() { break; }
            }
        }).detach();
    });
}


/// Native rendering regression: only visible rows are laid out, even with 10k pages.
#[cfg(test)]
#[allow(dead_code)]
pub fn verify_native_tree(cx: &mut App) {
    let db=Store::open(std::path::Path::new(":memory:")).unwrap();
    let first=db.create("00000 Root", "Root content").unwrap();
    for i in 1..10_000 { db.create(&format!("{i:05} Page"), "Body").unwrap(); }
    db.select(&first.id).unwrap();
    let mut panel=None;
    let handle=cx.open_window(WindowOptions {
        window_bounds:Some(WindowBounds::Windowed(Bounds::new(point(px(0.),px(0.)),size(px(1000.),px(640.))))),
        show:false,focus:false,..Default::default()
    }, |window,cx| {
        let view=cx.new(|cx| Panel::new(db,window,cx));
        panel=Some(view.clone());
        cx.new(|cx| Root::new(view,window,cx))
    }).unwrap();
    let panel=panel.unwrap();
    cx.update_window(handle.into(), |_,window,cx| { window.draw(cx).clear(); }).unwrap();
    assert_eq!(panel.read(cx).tree.rows.len(),10_000);
    assert!(panel.read(cx).rendered_tree_rows > 0 && panel.read(cx).rendered_tree_rows < 40,
        "10k roots must render only a viewport of rows");
    cx.update_window(handle.into(), |_,window,cx| {
        panel.update(cx, |panel,cx| {
            panel.new_child(Some(&first.id),window,cx);
            let child=panel.selected.as_ref().unwrap().id.clone();
            assert_eq!(panel.selected.as_ref().unwrap().parent_id.as_deref(),Some(first.id.as_str()));
            assert!(panel.tree.expanded.contains(&first.id));
            assert_eq!(panel.tree.rows.len(),10_001);
            panel.new_child(Some(&child),window,cx);
            let grandchild=panel.selected.as_ref().unwrap().id.clone();
            panel.toggle_note(&first.id,cx);
            assert_eq!(panel.tree.rows.len(),10_000);
            panel.select(&grandchild,window,cx);
            assert!(panel.tree.expanded.contains(&first.id));
            assert!(panel.tree.expanded.contains(&child));
            panel.move_note(&child,None,window,cx);
            assert_eq!(panel.db.get(&child).unwrap().parent_id,None);
            assert_eq!(panel.db.get(&grandchild).unwrap().parent_id,Some(child));
            assert!(panel.error.is_none());
        });
        window.draw(cx).clear();
    }).unwrap();
    assert!(panel.read(cx).rendered_tree_rows < 40);
    let (source,edge)=cx.update_window(handle.into(),|_,window,cx| {
        panel.update(cx,|state,cx| {state.reveal_note(&first.id);cx.notify();});draw_motion_frame(&panel,window,cx);
        let state=panel.read(cx);let note=state.tree_row_bounds.borrow()[&first.id];let bounds=state.sidebar_scroll.viewport_bounds();
        (point(note.left()+px(70.),note.center().y),point(bounds.left()+px(70.),bounds.bottom()-px(3.)))
    }).unwrap();
    for event in [
        PlatformInput::MouseMove(MouseMoveEvent {position:source,..Default::default()}),
        PlatformInput::MouseDown(MouseDownEvent {position:source,button:MouseButton::Left,click_count:1,..Default::default()}),
        PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(6.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
        PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(12.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
        PlatformInput::MouseMove(MouseMoveEvent {position:edge,pressed_button:Some(MouseButton::Left),..Default::default()}),
    ] {cx.update_window(handle.into(),|_,window,cx|{let _=window.dispatch_event(event,cx);draw_motion_frame(&panel,window,cx);}).unwrap();}
    let before=panel.read(cx).sidebar_scroll.logical_scroll_top();
    for _ in 0..3 {
        cx.update_window(handle.into(),|_,window,cx| {
            panel.update(cx,|state,cx| {state.drag_frame_at=Instant::now()-Duration::from_millis(32);state.step_tree_drag(window,cx);});
            draw_motion_frame(&panel,window,cx);
        }).unwrap();
    }
    let after=panel.read(cx).sidebar_scroll.logical_scroll_top();
    assert!(after.item_ix>before.item_ix || after.offset_in_item>before.offset_in_item,"stationary edge drag must keep scrolling");
    assert!(panel.read(cx).rendered_tree_rows<40,"auto-scroll must retain virtualization");
    cx.update_window(handle.into(),|_,window,cx| {
        cx.stop_active_drag(window);panel.update(cx,|state,cx|state.step_tree_drag(window,cx));
        assert!(panel.read(cx).tree_drop.is_none());window.remove_window();
    }).unwrap();
    println!("Native drag autoscroll verified: stationary edge scrolling, cleared preview on cancellation and fewer than 40 rendered rows with 10k notes.");
    println!("Native note tree verified: 10k notes render fewer than 40 rows; nested creation, collapse, reveal and subtree moves preserve state.");
}


#[cfg(test)]
fn draw_motion_frame(panel:&Entity<Panel>,window:&mut Window,cx:&mut App) {
    // Hidden windows do not receive compositor frames; notify the animated views
    // explicitly, using only this invisible window and no operating system input.
    panel.update(cx,|state,cx| {
        cx.notify();
        state.opacity_popover.update(cx,|_,cx|cx.notify());
        state.width_popover.update(cx,|_,cx|cx.notify());
        state.font_menu.update(cx,|_,cx|cx.notify());
        state.note_menu.update(cx,|_,cx|cx.notify());
        state.note_context_menu.update(cx,|_,cx|cx.notify());
        state.group_menu.update(cx,|_,cx|cx.notify());
    });
    window.refresh();
    window.draw(cx).clear();
}

#[cfg(test)]
#[allow(dead_code)]
pub fn verify_native_hover_async(cx: &mut App) {
    let db=Store::open(std::path::Path::new(":memory:")).unwrap();
    let root=db.create("A Parent","").unwrap();
    db.create_child("B Child","",Some(&root.id)).unwrap();
    let other=db.create("C Other","").unwrap();
    db.select(&root.id).unwrap();
    let mut panel=None;
    let handle=cx.open_window(WindowOptions {
        window_bounds:Some(WindowBounds::Windowed(Bounds::new(point(px(0.),px(0.)),size(px(1000.),px(640.))))),
        show:false,focus:false,..Default::default()
    },|w,cx| {
        let view=cx.new(|cx| Panel::new(db,w,cx)); panel=Some(view.clone());
        cx.new(|cx| Root::new(view,w,cx))
    }).unwrap();
    let panel=panel.unwrap();
    cx.update_window(handle.into(),|_,w,cx| w.draw(cx).clear()).unwrap();
    let bounds=panel.read(cx).tree_row_bounds.borrow()[&root.id];
    let other_bounds=panel.read(cx).tree_row_bounds.borrow()[&other.id];
    assert_eq!(bounds.size.height,px(30.));
    assert_eq!(other_bounds.top()-bounds.top(),px(31.));
    let icon=point(bounds.left()+px(19.),bounds.center().y);
    let menu=point(bounds.right()-px(18.),bounds.center().y);
    let outside=point(px(400.),px(400.));
    cx.spawn(async move |cx| {
        let events=[
            PlatformInput::MouseMove(MouseMoveEvent {position:icon,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:icon,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:icon,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:outside,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:menu,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:menu,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:menu,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:outside,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
        ];
        for (step,event) in events.into_iter().enumerate() {
            cx.update_window(handle.into(),|_,w,cx| { let _=w.dispatch_event(event,cx); draw_motion_frame(&panel,w,cx); }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);
                let state=panel.read(cx);
                if step==0 { assert_eq!(state.hovered_note.as_deref(),Some(root.id.as_str())); }
                if step==3 { assert_eq!(state.hovered_note,None); assert!(state.tree.expanded.contains(&root.id)); }
                if step==6 { assert!(state.note_menu_open,"the hover ellipsis must open its menu"); }
                if step==7 { assert!(state.note_menu_open,"the menu must survive leaving the row"); }
                if step==9 { assert!(!state.note_menu_open,"outside click must dismiss the menu"); }
            }).unwrap();
        }
        let bounds=cx.update(|cx| panel.read(cx).tree_row_bounds.borrow()["actions"]).unwrap();
        let title_position=point(bounds.left()+px(20.),bounds.bottom()+px(20.));
        let action_position=point(bounds.left()+px(20.),bounds.center().y);
        for (step,event) in [
            PlatformInput::MouseMove(MouseMoveEvent {position:title_position,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:action_position,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:action_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:action_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:outside,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
        ].into_iter().enumerate() {
            cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
            Timer::after(Duration::from_millis(250)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                if step==0 || step==1 {assert!(state.title_hovered,"title actions must stay visible when moving into their strip");}
                if step==3 {assert!(state.page_action==Some(PageAction::Icon),"title icon action must open picker");}
                if step==4 {assert!(!state.title_hovered);assert!(state.page_action.is_some());}
                if step==6 {assert!(state.page_action.is_none());}
                assert_eq!(state.tree_row_bounds.borrow()["actions"],bounds,"hover and popup must not move the title");
            }).unwrap();
        }
        for id in ["coral","lavender"] {
            cx.update_window(handle.into(),|_,w,cx| {
                panel.update(cx, |state,cx| {state.page_action=Some(PageAction::Cover);cx.notify();});
                draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            let tile=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);
                let state=panel.read(cx);let geometry=state.tree_row_bounds.borrow();
                let popup=geometry["popup"];
                assert!(popup.left()>=px(12.) && popup.right()<=w.viewport_size().width-px(12.),"gallery must fit inside the window");
                geometry[&format!("cover-{id}")].center()
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:tile,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:tile,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:tile,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                assert!(state.page_action.is_none(),"choosing a cover must close the gallery");
                assert_eq!(crate::covers::preset(state.page_cover.as_deref().unwrap()).unwrap().id,id);
                assert_eq!(state.db.presentation(&root.id).unwrap().1,state.page_cover);
                assert_eq!(state.db.get(&root.id).unwrap(),root,"choosing a cover must preserve Markdown, title and revision");
                assert_eq!(state.tree_row_bounds.borrow()["cover-preview"].size.height,px(160.));
                let geometry=state.tree_row_bounds.borrow();
                let cover=geometry["cover-preview"]; let content=geometry["content"];
                assert!((f32::from(cover.left()-content.left())).abs()<2.,"cover must ignore the text's left padding");
                assert!((f32::from(cover.right()-content.right())).abs()<2.,"cover must fill the content width without right padding");
                assert!((f32::from(cover.top()-content.top())).abs()<2.,"cover must start at the top without padding");
            }).unwrap();
        }
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx, |state,cx| state.set_page_icon(Some("💻"),cx));
            draw_motion_frame(&panel,w,cx);
        }).unwrap();
        Timer::after(Duration::from_millis(20)).await;
        let page_icon=cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let geometry=state.tree_row_bounds.borrow();
            let icon=geometry["page-icon"];let cover=geometry["cover-preview"];let content=geometry["content"];
            assert!((f32::from(icon.center().y-cover.bottom())).abs()<1.,"the icon must straddle the cover edge by exactly 50%");
            assert!((f32::from(icon.left()-content.left())-78.).abs()<1.,"the icon must align with the title's text inset");
            icon.center()
        }).unwrap();
        for event in [
            PlatformInput::MouseMove(MouseMoveEvent {position:page_icon,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:page_icon,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:page_icon,button:MouseButton::Left,click_count:1,..Default::default()}),
        ] {
            cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
            Timer::after(Duration::from_millis(20)).await;
        }
        cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx); assert!(panel.read(cx).page_action==Some(PageAction::Icon),"the overlapping icon must remain clickable");
            panel.update(cx, |state,cx| {state.page_action=None;cx.notify();});
        }).unwrap();
        println!("Native page icon verified: 50% cover overlap, title alignment and clickable picker.");
        // The new selector must receive real clicks inside the deferred popup.
        for (query,value,icons) in [("foguete","🚀",false),("heart","sparkpad:icon:svg:iconoir/regular/heart.svg",true)] {
            cx.update_window(handle.into(),|_,w,cx| {
                panel.update(cx,|state,cx| state.open_icon_picker(w,cx));draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                let picker=panel.read(cx).icon_picker.clone();
                picker.update(cx,|picker,cx| {picker.scroll_to_end();cx.notify();});
                draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);
                let state=panel.read(cx);let picker=state.icon_picker.read(cx);
                let geometry=picker.geometry.borrow();
                assert_eq!(geometry["scrollbar"],geometry["viewport"],"scrollbar must stay anchored to the emoji viewport at the end of scrolling");
                assert!(picker.scroll_offset()<px(0.),"the complete emoji catalog must actually scroll");
                let popup=state.tree_row_bounds.borrow()["popup"];
                assert!(geometry["scrollbar"].top()>=popup.top() && geometry["scrollbar"].bottom()<=popup.bottom(),"scrollbar must remain inside the popup");
            }).unwrap();
            if icons {
                let tab=cx.update_window(handle.into(),|_,w,cx| {
                    draw_motion_frame(&panel,w,cx);panel.read(cx).icon_picker.read(cx).geometry.borrow()["tab:Ícones"].center()
                }).unwrap();
                for event in [
                    PlatformInput::MouseDown(MouseDownEvent {position:tab,button:MouseButton::Left,click_count:1,..Default::default()}),
                    PlatformInput::MouseUp(MouseUpEvent {position:tab,button:MouseButton::Left,click_count:1,..Default::default()}),
                ] {
                    cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                    Timer::after(Duration::from_millis(20)).await;
                }
            }
            cx.update_window(handle.into(),|_,w,cx| {
                panel.read(cx).icon_picker.clone().update(cx,|picker,cx| picker.set_search(query,w,cx));draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            let choice=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let popup=state.tree_row_bounds.borrow()["popup"];
                assert!(popup.left()>=px(12.) && popup.right()<=w.viewport_size().width-px(12.));
                assert!(popup.top()>=px(12.) && popup.bottom()<=w.viewport_size().height-px(12.));
                state.icon_picker.read(cx).geometry.borrow()[&format!("choice:{value}")].center()
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:choice,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:choice,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:choice,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);
                assert!(state.page_action.is_none());assert_eq!(state.page_icon.as_deref(),Some(value));
                assert_eq!(state.db.presentation(&root.id).unwrap().0.as_deref(),Some(value));
                assert_eq!(state.notes.iter().find(|n| n.id==root.id).unwrap().icon.as_deref(),Some(value));
                let recent:Vec<String>=serde_json::from_str(&state.db.setting("recent_page_icons").unwrap().unwrap()).unwrap();
                assert_eq!(recent[0],value);assert_eq!(state.db.get(&root.id).unwrap(),root);
            }).unwrap();
        }
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| state.open_icon_picker(w,cx));draw_motion_frame(&panel,w,cx);
        }).unwrap();
        Timer::after(Duration::from_millis(20)).await;
        let upload_tab=cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx);panel.read(cx).icon_picker.read(cx).geometry.borrow()["tab:Fazer upload"].center()
        }).unwrap();
        for event in [
            PlatformInput::MouseDown(MouseDownEvent {position:upload_tab,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:upload_tab,button:MouseButton::Left,click_count:1,..Default::default()}),
        ] {
            cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
            Timer::after(Duration::from_millis(20)).await;
        }
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {state.set_page_icon(None,cx);assert_eq!(state.db.presentation(&root.id).unwrap().0,None);state.set_page_icon(Some("💻"),cx);});
            draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native Notion icon picker verified: Portuguese search, emoji/SVG selection by real clicks, upload tab, popup bounds, recent history, sidebar metadata, removal and unchanged document.");
        let original_column = cx.update_window(handle.into(), |_, w, cx| {
            draw_motion_frame(&panel,w,cx); panel.read(cx).tree_row_bounds.borrow()["document-column"]
        }).unwrap();
        for percent in [70., 40., 100.] {
            cx.update_window(handle.into(), |_, w, cx| {
                panel.read(cx).content_width.clone().update(cx, |slider, cx| {
                    slider.set_values(vec![percent], cx);
                    cx.emit(SliderEvent::Change(vec![percent]));
                    cx.emit(SliderEvent::Commit(vec![percent]));
                });
                draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            cx.update_window(handle.into(), |_, w, cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx); let geometry=state.tree_row_bounds.borrow();
                let column=geometry["document-column"]; let content=geometry["content"];
                let icon=geometry["page-icon"]; let cover=geometry["cover-preview"];
                assert!((f32::from(column.size.width-original_column.size.width*(percent/100.))).abs()<2.);
                let text_left=column.left()+px(58.);
                assert!((f32::from((text_left+column.right())/2.-content.center().x)).abs()<2., "text must remain centered");
                assert!((f32::from(icon.left()-text_left)).abs()<1., "icon must follow the centered title");
                assert!((f32::from(icon.center().y-cover.bottom())).abs()<1., "icon must keep its cover overlap");
                assert!((f32::from(cover.size.width-content.size.width)).abs()<2., "cover must remain full width");
                assert_eq!(state.db.setting("content_width").unwrap().unwrap().parse::<f32>().unwrap(),percent);
                assert_eq!(state.db.get(&root.id).unwrap(),root, "resizing must not edit the note");
            }).unwrap();
        }
        println!("Native content width verified: centered 40/70/100% columns, aligned icon, full-width cover and persisted preference.");
        for width in [false, true] {
            let trigger=cx.update_window(handle.into(), |_,w,cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                let popover=if width {&state.width_popover} else {&state.opacity_popover};
                assert!(!popover.read(cx).is_open());
                popover.read(cx).trigger_bounds().center()
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:trigger,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:trigger,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:trigger,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            let rail=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                let popover=if width {&state.width_popover} else {&state.opacity_popover};
                assert!(popover.read(cx).is_open());
                let popup=popover.read(cx).popup_bounds().unwrap();
                assert!(popup.bottom()<popover.read(cx).trigger_bounds().top(),"footer popovers must open upwards");
                assert!(popup.left()>=px(0.) && popup.right()<=w.viewport_size().width);
                point(popup.center().x,popup.bottom()-px(21.))
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:rail,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:rail,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:rail,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,_,cx| {
                let state=panel.read(cx);
                let slider=if width {&state.content_width} else {&state.opacity};
                let expected=if width {70.} else {62.5};
                assert!((slider.read(cx).value()-expected).abs()<2.,"slider inside the popup must receive clicks: width={width}, value={}, expected={expected}",slider.read(cx).value());
                let saved=state.db.setting(if width {"content_width"} else {"opacity"}).unwrap().unwrap().parse::<f32>().unwrap();
                assert!((saved-slider.read(cx).value()).abs()<0.1,"popup slider must persist on release");
            }).unwrap();
            for event in [
                PlatformInput::MouseDown(MouseDownEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:outside,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,_,cx| {
                let state=panel.read(cx);
                assert!(!state.opacity_popover.read(cx).is_open() && !state.width_popover.read(cx).is_open(),"outside click must close the popover");
            }).unwrap();
        }
        println!("Native footer popovers verified: icon clicks, upward positioning, live slider interaction, persistence and outside dismissal.");
        for light in [true, false] {
            let button=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx); panel.read(cx).tree_row_bounds.borrow()["theme-button"].center()
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:button,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:button,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:button,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);
                assert_eq!(state.light_mode,light,"footer button must switch the theme");
                assert_eq!(state.db.setting("theme").unwrap().as_deref(),Some(if light {"light"} else {"dark"}));
                let theme=gpui_component::Theme::global(cx);
                assert_eq!(theme.is_dark(),!light);
                assert_eq!(theme.font_family,SharedString::from(crate::assets::FONT_FAMILY));
                assert_eq!(theme.background,Hsla::from(rgb(if light {0xffffff} else {0x161616})));
                assert_eq!(theme.foreground,Hsla::from(rgb(if light {0x262626} else {0xeeeeee})));
                assert_eq!(state.db.get(&root.id).unwrap(),root,"theme switching must preserve the note");
                assert!(state.error.is_none());
            }).unwrap();
        }
        println!("Native light mode verified: real footer clicks switch both theme systems, neutral colors, preserved font, persistence and unchanged notes.");
        for index in [1,2,0] {
            let button=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);panel.read(cx).tree_row_bounds.borrow()["font-button"].center()
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:button,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:button,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:button,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            let item=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);assert!(state.font_menu.read(cx).is_open());
                let popup=state.font_menu.read(cx).popup_bounds().unwrap();
                point(popup.center().x,popup.top()+px(5.+14.+index as f32*28.))
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:item,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:item,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:item,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let font=crate::assets::CONTENT_FONTS[index];
                assert_eq!(state.content_font,index);
                assert_eq!(state.editor.read(cx).content_font_family(),font.1);
                assert_eq!(state.db.setting("content_font").unwrap().as_deref(),Some(font.0));
                assert_eq!(state.font_menu.read(cx).radio_selected(0),Some(index));
                assert!(!state.font_menu.read(cx).is_open());
                assert_eq!(gpui_component::Theme::global(cx).font_family,SharedString::from(crate::assets::FONT_FAMILY),"interface font must remain unchanged");
                assert_eq!(state.db.get(&root.id).unwrap(),root,"font switching must not edit Markdown or revision");
            }).unwrap();
        }
        println!("Native content fonts verified: real selector clicks, three font families, persisted preference, unchanged interface font and document.");
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {
                let previous=serde_json::to_value(crate::preferences::Preferences::read(&state.db).unwrap()).unwrap();
                crate::preferences::Preferences::update(&state.db,&serde_json::json!({"opacity":75,"content_width":60,"font_size":26,"sidebar_width":250,"sidebar":false,"always_on_top":false,"theme":"light","content_font":"serif"})).unwrap();
                state.last_sync=Instant::now()-Duration::from_secs(1);state.last_data_version=None;
                state.tick(w,cx);
                assert_eq!(state.opacity.read(cx).value(),75.);assert_eq!(state.content_width.read(cx).value(),60.);
                assert_eq!(state.font_size,26.);assert_eq!(state.sidebar_width,250.);
                assert!(!state.sidebar && !state.pinned && state.light_mode);
                assert_eq!(state.content_font,1);assert_eq!(state.font_menu.read(cx).radio_selected(0),Some(1));
                assert_eq!(state.db.get(&root.id).unwrap(),root,"MCP appearance settings must preserve notes");
                crate::preferences::Preferences::update(&state.db,&previous).unwrap();
                state.last_sync=Instant::now()-Duration::from_secs(1);state.last_data_version=None;
                state.tick(w,cx);
            });
            draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native MCP preferences verified: sync applies sliders, typography, sidebar, theme, font radio selection and pinning without editing documents.");
        let (group_id,grouped_note,grouped_child)=cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {
                let note=state.db.create("D Drag into group","Preserved Markdown").unwrap();
                let child=state.db.create_child("E Nested page","Preserved child",Some(&note.id)).unwrap();
                state.create_group(w,cx);
                let id=state.renaming_group.clone().expect("new group must start inline renaming");
                state.group_title.update(cx,|input,cx| input.set_value("Favoritos",w,cx));
                state.finish_group_rename(cx);
                assert_eq!(state.tree.groups.iter().find(|g| g.id==id).unwrap().title,"Favoritos");
                (id,note,child)
            })
        }).unwrap();
        Timer::after(Duration::from_millis(20)).await;
        let empty_group_position=cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let geometry=state.tree_row_bounds.borrow();
            let group=geometry[&format!("group-{group_id}")];let empty=geometry[&format!("empty-{group_id}")];
            assert_eq!(empty.size.height,px(31.));
            assert_eq!(empty.top(),group.bottom()+px(1.),"empty message must sit directly below its header");
            point(group.left()+px(50.),group.center().y)
        }).unwrap();
        for expanded in [false,true] {
            for event in [
                PlatformInput::MouseDown(MouseDownEvent {position:empty_group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:empty_group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,_,cx| {
                let state=panel.read(cx);
                assert_eq!(state.tree.rows.iter().any(|r|r.empty && r.group.is_some_and(|g|state.tree.groups[g].id==group_id)),expanded);
                assert_eq!(state.selected.as_ref().unwrap().id,root.id,"empty section toggles must not select a page");
            }).unwrap();
        }
        println!("Native empty group verified: real header clicks show/hide an indented nonselectable placeholder.");
        let (source,destination)=cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let geometry=state.tree_row_bounds.borrow();
            let source=geometry[&grouped_note.id];let group=geometry[&format!("group-{group_id}")];
            (point(source.left()+px(75.),source.center().y),group.center())
        }).unwrap();
        for event in [
            PlatformInput::MouseMove(MouseMoveEvent {position:source,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:source,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(6.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(12.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:destination,pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:destination,button:MouseButton::Left,click_count:1,..Default::default()}),
        ] {
            cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
            Timer::after(Duration::from_millis(20)).await;
        }
        let group_position=cx.update_window(handle.into(),|_,w,cx| {
            draw_motion_frame(&panel,w,cx);let state=panel.read(cx);
            assert_eq!(state.db.get(&grouped_note.id).unwrap(),grouped_note,"grouping a root page must not modify its document");
            assert_eq!(state.db.get(&grouped_child.id).unwrap(),grouped_child,"dragging must preserve nested pages");
            assert_eq!(state.notes.iter().find(|n| n.id==grouped_note.id).unwrap().group_id.as_deref(),Some(group_id.as_str()),"real drag must move page into the group");
            assert!(state.tree.expanded.contains(&group_id));
            let geometry=state.tree_row_bounds.borrow();let group=geometry[&format!("group-{group_id}")];
            point(group.left()+px(50.),group.center().y)
        }).unwrap();
        for (step,event) in [
            PlatformInput::MouseDown(MouseDownEvent {position:group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:group_position,button:MouseButton::Left,click_count:1,..Default::default()}),
        ].into_iter().enumerate() {
            cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            if step==1 {
                cx.update_window(handle.into(),|_,_,cx| {
                    let state=panel.read(cx);assert!(!state.tree.expanded.contains(&group_id));
                    assert!(!state.tree.rows.iter().any(|r| r.is_page() && state.notes[r.index].id==grouped_note.id));
                }).unwrap();
            }
        }
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {
                assert!(state.tree.expanded.contains(&group_id));
                state.new_group_note(&group_id,w,cx);
                let new_id=state.selected.as_ref().unwrap().id.clone();
                assert_eq!(state.notes.iter().find(|n| n.id==new_id).unwrap().group_id.as_deref(),Some(group_id.as_str()));
                state.select(&root.id,w,cx);
                state.db.delete_group(&group_id).unwrap();state.refresh_tree();
                assert!(state.notes.iter().all(|n| n.group_id.is_none()));
                assert_eq!(state.db.get(&grouped_note.id).unwrap(),grouped_note);
                assert_eq!(state.db.get(&root.id).unwrap(),root);
                state.db.delete(&grouped_child.id,grouped_child.revision).unwrap();
                state.db.delete(&grouped_note.id,grouped_note.revision).unwrap();
                let new_note=state.db.get(&new_id).unwrap();state.db.delete(&new_id,new_note.revision).unwrap();
                state.refresh_tree();cx.notify();
            });draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native sidebar groups verified: creation, inline rename, real drag, collapse/expand, grouped note creation and deletion preserving pages.");
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {
                let mut roots:Vec<_>=state.notes.iter().filter(|n| n.parent_id.is_none() && n.group_id.is_none()).map(|n|n.id.clone()).collect();
                roots.reverse();
                state.db.reorder_sidebar(&roots,false,None,None).unwrap();
                state.last_sync=Instant::now()-Duration::from_secs(1);state.last_data_version=None;
                state.tick(w,cx);
                let displayed:Vec<_>=state.tree.rows.iter().filter(|r|r.is_page() && r.depth==0)
                    .map(|r|state.notes[r.index].id.clone()).collect();
                assert_eq!(displayed,roots,"MCP sibling order must reach the virtualized sidebar");
                assert_eq!(state.db.get(&root.id).unwrap(),root);
            });
            draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native MCP ordering verified: persistent sibling positions update the virtualized sidebar without editing notes.");
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx, |state,cx| {
                state.select(&other.id,w,cx);assert!(state.page_cover.is_none());
                state.select(&root.id,w,cx);
                assert_eq!(crate::covers::preset(state.page_cover.as_deref().unwrap()).unwrap().id,"lavender");
                state.set_page_cover(None,cx);
                assert_eq!(state.db.presentation(&root.id).unwrap().1,None);
            });
            draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native cover gallery verified: real solid/gradient swatch clicks, popup bounds, persistence across note switches, removal and unchanged document.");
        let (drag_a,drag_b)=cx.update_window(handle.into(),|_,w,cx| {
            let notes=panel.update(cx,|state,cx| {
                let a=state.db.create("Drag A","Preserve A").unwrap();let b=state.db.create("Drag B","Preserve B").unwrap();
                state.refresh_tree();cx.notify();(a,b)
            });draw_motion_frame(&panel,w,cx);notes
        }).unwrap();
        for edge in [Edge::Before,Edge::Inside,Edge::After] {
            let (source,destination)=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let geometry=state.tree_row_bounds.borrow();
                let source=geometry[&drag_b.id];let target=geometry[&drag_a.id];
                (point(source.left()+px(65.),source.center().y),point(target.left()+px(50.),match edge {Edge::Before=>target.top()+px(2.),Edge::After=>target.bottom()-px(2.),Edge::Inside=>target.center().y}))
            }).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position:source,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:source,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(6.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
                PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(12.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
                PlatformInput::MouseMove(MouseMoveEvent {position:destination,pressed_button:Some(MouseButton::Left),..Default::default()}),
            ] {
                cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
            }
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let preview=state.tree_drop.as_ref().expect("drag must expose an insertion destination");
                assert_eq!(preview.edge,edge);assert!(state.tree_row_bounds.borrow().contains_key("tree-drop-preview"));
                if edge==Edge::Inside {assert_eq!(preview.parent.as_deref(),Some(drag_a.id.as_str()));}
                else {assert_eq!(preview.parent,None);assert_eq!(preview.depth,0);}
            }).unwrap();
            if edge==Edge::Inside {
                Timer::after(Duration::from_millis(520)).await;
                cx.update_window(handle.into(),|_,w,cx|{panel.update(cx,|state,cx|state.step_tree_drag(w,cx));draw_motion_frame(&panel,w,cx);assert!(panel.read(cx).tree.expanded.contains(&drag_a.id),"hover must open a closed destination after 500ms");}).unwrap();
            }
            cx.update_window(handle.into(),|_,w,cx| {
                let _=w.dispatch_event(PlatformInput::MouseUp(MouseUpEvent {position:destination,button:MouseButton::Left,click_count:1,..Default::default()}),cx);
                draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(30)).await;
            cx.update_window(handle.into(),|_,_,cx| {
                let state=panel.read(cx);let note=state.db.get(&drag_b.id).unwrap();assert_eq!(note.markdown,drag_b.markdown);
                assert_eq!(note.parent_id.as_deref(),if edge==Edge::Inside {Some(drag_a.id.as_str())} else {None});
                if edge!=Edge::Inside {
                    let rows:Vec<_>=state.tree.rows.iter().filter(|r|r.is_page()).map(|r|state.notes[r.index].id.as_str()).collect();
                    let a=rows.iter().position(|id|*id==drag_a.id).unwrap();let b=rows.iter().position(|id|*id==drag_b.id).unwrap();
                    assert_eq!(b<a,edge==Edge::Before,"line direction must match the persisted sibling order");
                }
                assert!(state.tree_drop.is_none(),"release must clear the insertion preview");
            }).unwrap();
        }
        let (source,destination)=cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {let note=state.db.get(&drag_b.id).unwrap();state.db.move_note(&note.id,Some(&drag_a.id),note.revision).unwrap();state.refresh_tree();cx.notify();});
            draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let source=state.tree_row_bounds.borrow()[&drag_b.id];let viewport=state.sidebar_scroll.viewport_bounds();
            (point(source.left()+px(65.),source.center().y),point(viewport.left()+px(65.),viewport.bottom()-px(48.)))
        }).unwrap();
        for event in [
            PlatformInput::MouseMove(MouseMoveEvent {position:source,..Default::default()}),
            PlatformInput::MouseDown(MouseDownEvent {position:source,button:MouseButton::Left,click_count:1,..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(6.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:source+point(px(12.),px(0.)),pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseMove(MouseMoveEvent {position:destination,pressed_button:Some(MouseButton::Left),..Default::default()}),
            PlatformInput::MouseUp(MouseUpEvent {position:destination,button:MouseButton::Left,click_count:1,..Default::default()}),
        ] {cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();Timer::after(Duration::from_millis(20)).await;}
        cx.update_window(handle.into(),|_,_,cx| {let state=panel.read(cx);assert_eq!(state.db.get(&drag_b.id).unwrap().parent_id,None,"dropping into empty tree space must promote a child to root");}).unwrap();
        println!("Native tree drag verified: before/inside/after previews, upward and downward reordering, nesting, outdent, delayed expansion and preserved Markdown.");
        let delete_child=cx.update_window(handle.into(),|_,w,cx| {
            let child=panel.update(cx,|state,cx| {let child=state.db.create_child("Delete child","Preserved until confirmation",Some(&drag_a.id)).unwrap();state.refresh_tree();state.select(&child.id,w,cx);child});
            draw_motion_frame(&panel,w,cx);child
        }).unwrap();
        for confirm in [false,true] {
            let position=cx.update_window(handle.into(),|_,w,cx|{draw_motion_frame(&panel,w,cx);let bounds=panel.read(cx).tree_row_bounds.borrow()[&drag_a.id];point(bounds.left()+px(55.),bounds.center().y)}).unwrap();
            for event in [
                PlatformInput::MouseMove(MouseMoveEvent {position,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position,button:MouseButton::Right,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position,button:MouseButton::Right,click_count:1,..Default::default()}),
            ] {cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();Timer::after(Duration::from_millis(20)).await;}
            let delete_position=cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);let menu=state.note_context_menu.read(cx);
                assert!(menu.is_open());assert_eq!(state.note_menu_target.as_deref(),Some(drag_a.id.as_str()));
                assert_eq!(state.selected.as_ref().unwrap().id,delete_child.id,"right click must target the clicked note without switching the document");
                let bounds=menu.popup_bounds().unwrap();assert!(bounds.left()>=px(0.)&&bounds.right()<=w.viewport_size().width);
                point(bounds.center().x,bounds.bottom()-px(19.))
            }).unwrap();
            for event in [
                PlatformInput::MouseDown(MouseDownEvent {position:delete_position,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:delete_position,button:MouseButton::Left,click_count:1,..Default::default()}),
            ] {cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();Timer::after(Duration::from_millis(20)).await;}
            cx.update_window(handle.into(),|_,_,cx|{let state=panel.read(cx);assert_eq!(state.pending_delete.as_ref().unwrap().count,2);assert!(state.db.get(&drag_a.id).is_ok());}).unwrap();
            if confirm {
                let position=cx.update_window(handle.into(),|_,w,cx|{draw_motion_frame(&panel,w,cx);panel.read(cx).tree_row_bounds.borrow()["confirm-delete-note"].center()}).unwrap();
                for event in [
                    PlatformInput::MouseDown(MouseDownEvent {position,button:MouseButton::Left,click_count:1,..Default::default()}),
                    PlatformInput::MouseUp(MouseUpEvent {position,button:MouseButton::Left,click_count:1,..Default::default()}),
                ] {cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();Timer::after(Duration::from_millis(20)).await;}
            } else {
                cx.update_window(handle.into(),|_,w,cx|{let _=w.dispatch_event(PlatformInput::KeyDown(KeyDownEvent {keystroke:Keystroke::parse("escape").unwrap(),is_held:false}),cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
                cx.update_window(handle.into(),|_,_,cx|{let state=panel.read(cx);assert!(state.pending_delete.is_none());assert!(state.db.get(&drag_a.id).is_ok());assert!(state.db.get(&delete_child.id).is_ok());}).unwrap();
            }
        }
        cx.update_window(handle.into(),|_,w,cx| {
            panel.update(cx,|state,cx| {
                assert!(state.pending_delete.is_none());assert!(state.db.get(&drag_a.id).is_err());assert!(state.db.get(&delete_child.id).is_err());
                assert!(state.selected.as_ref().is_some_and(|n|state.db.get(&n.id).is_ok()),"deleted selected child must switch to a remaining note");
                assert_eq!(state.db.get(&root.id).unwrap(),root);assert_eq!(state.db.get(&drag_b.id).unwrap().markdown,drag_b.markdown);
                let note=state.db.get(&drag_b.id).unwrap();state.db.delete(&note.id,note.revision).unwrap();state.refresh_tree();state.select(&root.id,w,cx);
            });draw_motion_frame(&panel,w,cx);
        }).unwrap();
        println!("Native context menu and deletion verified: right click, pointer anchoring, unchanged selection, destructive action, Escape cancellation, subtree confirmation and valid fallback selection.");
        for (target,expected) in [(286.,286.),(1200.,420.),(-200.,180.),(300.,300.)] {
            let before=cx.update(|cx| panel.read(cx).tree_row_bounds.borrow()["sidebar"]).unwrap();
            let divider=point(before.right(),before.center().y);
            let destination=point(before.left()+px(target),divider.y);
            let events=[
                PlatformInput::MouseMove(MouseMoveEvent {position:divider,..Default::default()}),
                PlatformInput::MouseDown(MouseDownEvent {position:divider,button:MouseButton::Left,click_count:1,..Default::default()}),
                PlatformInput::MouseMove(MouseMoveEvent {position:point(divider.x+px(6.),divider.y),pressed_button:Some(MouseButton::Left),..Default::default()}),
                PlatformInput::MouseMove(MouseMoveEvent {position:destination,pressed_button:Some(MouseButton::Left),..Default::default()}),
                PlatformInput::MouseUp(MouseUpEvent {position:destination,button:MouseButton::Left,click_count:1,..Default::default()}),
            ];
            for (step,event) in events.into_iter().enumerate() {
                cx.update_window(handle.into(),|_,w,cx| {let _=w.dispatch_event(event,cx);draw_motion_frame(&panel,w,cx);}).unwrap();
                Timer::after(Duration::from_millis(20)).await;
                cx.update_window(handle.into(),|_,w,cx| {
                    draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                    if step>=3 {
                        let geometry=state.tree_row_bounds.borrow();
                        let sidebar=geometry["sidebar"]; let content=geometry["content"];
                        assert!((f32::from(sidebar.size.width)-expected).abs()<2.,"sidebar must resize during drag and respect limits: step {step}, target {target}, expected {expected}, actual {:?}, before {:?}",sidebar.size.width,before.size.width);
                        assert!((f32::from(content.left()-sidebar.right())).abs()<2.,"content and footer must follow the sidebar edge");
                    }
                    if step==4 {
                        let saved=state.db.setting("sidebar_width").unwrap().unwrap().parse::<f32>().unwrap();
                        assert!((saved-expected).abs()<2.,"width must persist when the divider is released");
                    }
                }).unwrap();
            }
        }
        for visible in [false,true] {
            cx.update_window(handle.into(),|_,w,cx| {
                panel.update(cx, |state,cx| state.toggle_sidebar(cx));
                draw_motion_frame(&panel,w,cx);
            }).unwrap();
            Timer::after(Duration::from_millis(20)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx);let state=panel.read(cx);
                assert!(state.sidebar_motion.moving());
                let left=f32::from(state.tree_row_bounds.borrow()["content"].left());
                assert!(left>1. && left<301.,"sidebar must pass through intermediate widths, not jump: {left}");
            }).unwrap();
            Timer::after(Duration::from_millis(230)).await;
            cx.update_window(handle.into(),|_,w,cx| {
                draw_motion_frame(&panel,w,cx); let state=panel.read(cx);
                assert_eq!(state.sidebar,visible);
                let geometry=state.tree_row_bounds.borrow();
                if visible {assert!((f32::from(geometry["sidebar"].size.width)-300.).abs()<2.,"reopening sidebar must preserve its chosen width");}
                else {assert!((f32::from(geometry["content"].left())-1.).abs()<2.,"hiding sidebar must give content all available width");}
            }).unwrap();
        }
        cx.update_window(handle.into(),|_,w,cx| {
            w.remove_window();
            println!("Native resizable sidebar verified: live drag, min/max limits, persistence on release, content alignment and collapse/reopen.");
            println!("Native title actions verified: hover stays active over buttons, picker opens and dismisses outside, unchanged title geometry.");
            println!("Native Notion sidebar verified: 30px rows, 1px gaps, hover enter/leave, icon-only expansion, menu anchoring and outside dismissal.");
            crate::block_editor::verify_native_selection_async(cx);
        }).unwrap();
    }).detach();
}

/// Reproducible performance run through the production Panel, with synthetic data.
#[cfg(test)]
pub fn benchmark_native(output:String,startup:Instant,cx:&mut App){
    use crate::performance::{ms,stats};use serde_json::json;use std::{rc::Rc,cell::RefCell};
    let fixtures=crate::performance::fixtures();let path=std::env::temp_dir().join(format!("sparkpad-performance-{}.sqlite3",uuid::Uuid::new_v4()));
    let db=Store::open(&path).unwrap();db.set_setting("onboarding_completed","true").unwrap();
    let mut ids=Vec::new();for (name,body)in &fixtures{ids.push(db.create(name,body).unwrap().id);}
    db.set_setting("selected_note",&ids[0]).unwrap();let mut panel=None;
    let open=Instant::now();
    let handle=cx.open_window(WindowOptions{show:true,focus:true,window_bounds:Some(WindowBounds::Windowed(Bounds::new(point(px(50.),px(50.)),size(px(1100.),px(760.))))),titlebar:Some(TitlebarOptions{title:Some("Sparkpad performance — synthetic data".into()),..Default::default()}),..Default::default()},|w,cx|{
        let p=cx.new(|cx|Panel::new(db,w,cx));panel=Some(p.clone());cx.new(|cx|Root::new(p,w,cx))
    }).unwrap();let handle:AnyWindowHandle=handle.into();let panel=panel.unwrap();
    cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();
    let initialization=ms(open);let start_to_scene=ms(startup);let mut rows=Vec::new();
    let pick=std::env::var("SPARKPAD_PERF_CASES").ok();
    for ((name,source),id) in fixtures.iter().zip(&ids){
        if pick.as_ref().is_some_and(|s|!s.split(',').any(|p|p==name)){continue;}
        eprintln!("PERF starting {name}: {} bytes",source.len());
        let mut parse=Vec::new();for _ in 0..7{let t=Instant::now();std::hint::black_box(crate::blocks::Document::parse(source));parse.push(ms(t));}
        let mut read=Vec::new();for _ in 0..21{let t=Instant::now();std::hint::black_box(panel.read(cx).db.get(id).unwrap());read.push(ms(t));}
        let mut load=Vec::new();let mut transition=Vec::new();
        for _ in 0..7{
            cx.update_window(handle,|_,w,cx|panel.update(cx,|p,cx|p.select(&ids[0],w,cx))).unwrap();cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();
            let t=Instant::now();cx.update_window(handle,|_,w,cx|panel.update(cx,|p,cx|p.select(id,w,cx))).unwrap();load.push(ms(t));cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();transition.push(ms(t));
        }
        let mut scroll=Vec::new();let mut hover=Vec::new();let mut edit=Vec::new();
        for i in 0..50{let t=Instant::now();cx.update_window(handle,|_,w,cx|{let _=w.dispatch_event(PlatformInput::ScrollWheel(ScrollWheelEvent{position:point(px(800.),px(400.)),delta:ScrollDelta::Pixels(point(px(0.),px(if i<25{-120.}else{120.}))),touch_phase:TouchPhase::Moved,..Default::default()}),cx);}).unwrap();cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();scroll.push(ms(t));}
        cx.update_window(handle,|_,_,cx|panel.read(cx).editor_scroll.set_offset(point(px(0.),px(0.)))).unwrap();cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();
        for i in 0..50{let t=Instant::now();cx.update_window(handle,|_,w,cx|{let _=w.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent{position:point(px(420.+(i%2)as f32*70.),px(240.+(i%4)as f32*45.)),..Default::default()}),cx);}).unwrap();cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();hover.push(ms(t));}
        let editor=panel.read(cx).editor.clone();
        for _ in 0..20{let t=Instant::now();cx.update_window(handle,|_,w,cx|crate::block_editor::benchmark_edit(&editor,w,cx)).unwrap();cx.update_window(handle,|_,w,cx|w.draw(cx).clear()).unwrap();edit.push(ms(t));}
        let counts=crate::block_editor::benchmark_counts(&editor,cx);
        let row=json!({"case":name,"bytes":source.len(),"counts":counts,"parse":stats(parse),"sqlite_read":stats(read),"load":stats(load),"transition_to_scene":stats(transition),"scroll_cpu_scene":stats(scroll),"hover_cpu_scene":stats(hover),"edit_cpu_scene":if edit.is_empty(){serde_json::Value::Null}else{stats(edit)}});
        eprintln!("PERF result {row}");rows.push(row);
    }
    // Native display-link callbacks include the production render/present path. Warm up first.
    cx.update_window(handle,|_,w,cx|panel.update(cx,|p,cx|p.select(&ids[1],w,cx))).unwrap();
    let mut report=json!({"version":2,"window":[1100,760],"profile":"dev opt-level=3 dependencies=2 debug=0 incremental=0","initialization_to_scene_ms":initialization,"process_code_start_to_scene_ms":start_to_scene,"cases":rows});
    std::fs::write(&output,serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    cx.activate(true);
    let native_state=cx.update_window(handle,|_,w,_|{
        w.activate_window();macos::visible(w,true);
        // Start AppKit's layer display cycle before collecting display-link callbacks.
        use raw_window_handle::{HasWindowHandle,RawWindowHandle};
        use objc::{msg_send,sel,sel_impl};
        if let RawWindowHandle::AppKit(h)=w.window_handle().unwrap().as_raw(){unsafe{
            let view=h.ns_view.as_ptr() as cocoa::base::id;
            let _:()=msg_send![view,setNeedsDisplay:cocoa::base::YES];
            let layer:cocoa::base::id=msg_send![view,layer];let _:()=msg_send![layer,setNeedsDisplay];
            let win:cocoa::base::id=msg_send![view,window];let visible:cocoa::base::BOOL=msg_send![win,isVisible];let occlusion:u64=msg_send![win,occlusionState];
            eprintln!("PERF native window visible={visible} occlusion={occlusion}");
            return json!({"visible":visible,"occlusion_state":occlusion});
        }}
        json!(null)
    }).unwrap();
    report["native_window"]=native_state;
    std::fs::write(&output,serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let watchdog_output=output.clone();
    cx.spawn(async move|cx|{
        Timer::after(std::time::Duration::from_secs(60)).await;
        let _=cx.update(|cx|{
            if let Ok(bytes)=std::fs::read(&watchdog_output){
                if let Ok(mut report)=serde_json::from_slice::<serde_json::Value>(&bytes){
                    report["cadence_error"]=json!("Display-link run did not finish within 60 seconds; check desktop visibility and occlusion. CPU scene samples remain valid.");
                    let _=std::fs::write(&watchdog_output,serde_json::to_vec_pretty(&report).unwrap());
                }
            }
            eprintln!("PERF cadence timeout; CPU report saved");cx.quit();
        });
    }).detach();
    type CadenceState=(Vec<f64>,Instant,usize,usize,f64);
    let state=Rc::new(RefCell::new((Vec::<f64>::new(),Instant::now(),0usize,0usize,0.)));
    fn cadence(w:&mut Window,panel:Entity<Panel>,ids:Vec<String>,state:Rc<RefCell<CadenceState>>,mut report:serde_json::Value,output:String){
        w.refresh();
        w.on_next_frame(move|w,cx|{
            let done={let mut s=state.borrow_mut();let now=Instant::now();let elapsed=crate::performance::ms(s.1);s.1=now;s.2+=1;let warmup=30;let samples=180;if s.2==warmup{s.4=crate::performance::media_time();}if s.2>warmup{s.0.push(elapsed);}s.2>=warmup+samples};
            if done {
                let phase=state.borrow().3;eprintln!("PERF cadence phase {phase} complete");let samples=state.borrow().0.clone();let mean=samples.iter().sum::<f64>()/samples.len()as f64;
                if report["display_link"].is_null(){report["display_link"]=json!([]);}
                report["display_link"].as_array_mut().unwrap().push(json!({"case":if phase>=4{"code-5000-lines"}else if phase%2==0{"blocks-200"}else{"blocks-8000"},"mode":if phase==5{"typing"}else if phase<2||phase==4{"scroll"}else{"hover"},"presented":crate::performance::presented_stats(&format!("{output}.frames.jsonl"),state.borrow().4,crate::performance::media_time()),"frame_intervals":crate::performance::stats(samples),"effective_fps":1000./mean,"note":"GPUI display-link callback cadence before rendering; not a GPU timestamp"}));
                std::fs::write(&output,serde_json::to_vec_pretty(&report).unwrap()).unwrap();
                if phase==5{eprintln!("PERF report {output}");w.remove_window();cx.quit();return;}
                let next=phase+1;panel.update(cx,|p,cx|p.select(&ids[if next>=4{5}else if next%2==0{1}else{3}],w,cx));
                *state.borrow_mut()=(Vec::new(),Instant::now(),0,next,0.);
            }
            let i=state.borrow().2;let phase=state.borrow().3;
            if phase==5{crate::block_editor::benchmark_edit(&panel.read(cx).editor.clone(),w,cx);}else if phase<2||phase==4{let _=w.dispatch_event(PlatformInput::ScrollWheel(ScrollWheelEvent{position:point(px(800.),px(400.)),delta:ScrollDelta::Pixels(point(px(0.),px(if i%60<30{-30.}else{30.}))),touch_phase:TouchPhase::Moved,..Default::default()}),cx);}else{let _=w.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent{position:point(px(420.+(i%2)as f32*70.),px(240.+(i%4)as f32*45.)),..Default::default()}),cx);}
            cadence(w,panel,ids,state,report,output);
        });
    }
    cx.update_window(handle,|_,w,_|cadence(w,panel,ids,state,report,output)).unwrap();
}
