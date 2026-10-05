use crate::app_theme::neutral;
use crate::blocks::{Block, Document, Kind};
use empire_ui::{
    menu::{Menu, MenuAlign, MenuEvent, MenuItem},
    Button, ButtonSize, ButtonVariant, Tooltip,
};
use gpui::{prelude::*, *};
use gpui_component::{
    input::{Input as NativeInput, InputEvent, InputState},
    text::{TextView, TextViewStyle},
    ActiveTheme,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

actions!(
    blocks,
    [
        FormatBold,
        FormatItalic,
        FormatUnderline,
        FormatCode,
        SelectDocument
    ]
);
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-a", SelectDocument, Some("BlockEditor > Input")),
        KeyBinding::new("ctrl-a", SelectDocument, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-a", SelectDocument, Some("BlockEditor")),
        KeyBinding::new("ctrl-a", SelectDocument, Some("BlockEditor")),
        KeyBinding::new("cmd-b", FormatBold, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-i", FormatItalic, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-u", FormatUnderline, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-e", FormatCode, Some("BlockEditor > Input")),
    ]);
}
pub enum EditorEvent {
    Change,
    Editing,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct TextSelection {
    anchor: (usize, usize),
    head: (usize, usize),
}
impl TextSelection {
    fn ordered(self) -> ((usize, usize), (usize, usize)) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
    fn range(self, i: usize, len: usize) -> std::ops::Range<usize> {
        let (a, z) = self.ordered();
        if i < a.0 || i > z.0 {
            return 0..0;
        }
        (if i == a.0 { a.1 } else { 0 })..(if i == z.0 { z.1 } else { len })
    }
}
#[derive(Clone)]
struct Snapshot {
    document: Document,
    active: Option<String>,
    cursor: usize,
    selected_block: Option<String>,
    text_selection: Option<TextSelection>,
}
#[derive(Clone)]
struct BlockDrag {
    id: String,
    owner: EntityId,
    label: String,
}
impl Render for BlockDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Keep the ghost below/right of the pointer so it cannot cover the insertion line.
        div().pt(px(24.)).pl(px(20.)).child(
            div()
                .w(px(240.))
                .px_2()
                .py_1()
                .rounded_md()
                .bg(neutral(0x2c2c2c))
                .text_size(px(16.))
                .truncate()
                .child(self.label.clone()),
        )
    }
}
struct BlockInput {
    state: Entity<InputState>,
    _subscription: Subscription,
}
pub struct BlockEditor {
    document: Document,
    inputs: HashMap<String, BlockInput>,
    active: Option<String>,
    selected_block: Option<String>,
    text_selection: Option<TextSelection>,
    mouse_anchor: Option<(usize, usize)>,
    mouse_unit: Option<(std::ops::Range<usize>, usize)>,
    selection_pointer: Option<Point<Pixels>>,
    document_scroll: Option<ScrollHandle>,
    scroll_tick_pending: bool,
    drop_target: Option<(String, bool)>,
    row_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    type_menu: Entity<Menu>,
    _type_menu_subscription: Subscription,
    focus_handle: FocusHandle,
    font_size: f32,
    font_family: SharedString,
    menu: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}
impl EventEmitter<EditorEvent> for BlockEditor {}
impl BlockEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let weak = cx.entity().downgrade();
        let type_menu = cx.new(|cx| {
            Menu::new(type_menu_items(&Kind::Paragraph), cx)
                .align(MenuAlign::Start)
                .width(220.)
                .trigger(move |_, _, cx| {
                    // The label follows the selected block, without rebuilding the menu entity.
                    let (label, icon) = weak
                        .upgrade()
                        .and_then(|editor| {
                            editor
                                .read(cx)
                                .active_index()
                                .map(|i| kind_display(&editor.read(cx).document.blocks[i].kind))
                        })
                        .unwrap_or(kind_display(&Kind::Paragraph));
                    Button::new("block-kind-trigger", label)
                        .icon_before(icon)
                        .icon_after("iconoir/regular/nav-arrow-down.svg")
                        .size(ButtonSize::Sm)
                        .variant(ButtonVariant::Ghost)
                        .into_any_element()
                })
        });
        let subscription = cx.subscribe_in(&type_menu, window, |this, _, event, window, cx| {
            match event {
                MenuEvent::Select(index) => {
                    if let Some((label, kind, _)) = block_types().get(*index) {
                        let current = this
                            .active_index()
                            .map(|i| kind_display(&this.document.blocks[i].kind).0);
                        if current != Some(*label) {
                            this.convert(kind.clone(), window, cx);
                            this.sync_type_menu(cx);
                        }
                        this.focus_handle.focus(window);
                    }
                }
                MenuEvent::OpenChange(true) => {
                    this.type_menu.read(cx).focus_handle(cx).focus(window)
                }
                MenuEvent::OpenChange(false) => {
                    if this.selected_block.is_some()
                        && this.type_menu.read(cx).focus_handle(cx).is_focused(window)
                    {
                        this.focus_handle.focus(window);
                    }
                }
                _ => {}
            }
            cx.notify();
        });
        let mut editor = Self {
            document: Document::parse(""),
            inputs: HashMap::new(),
            active: None,
            selected_block: None,
            text_selection: None,
            mouse_anchor: None,
            mouse_unit: None,
            selection_pointer: None,
            document_scroll: None,
            scroll_tick_pending: false,
            drop_target: None,
            row_bounds: Rc::new(RefCell::new(HashMap::new())),
            type_menu,
            _type_menu_subscription: subscription,
            focus_handle: cx.focus_handle(),
            font_size: 22.,
            font_family: crate::assets::FONT_FAMILY.into(),
            menu: false,
            undo: Vec::new(),
            redo: Vec::new(),
        };
        editor.sync_inputs(window, cx);
        editor
    }
    pub fn value(&self) -> SharedString {
        self.document.markdown().into()
    }
    pub fn set_scroll_handle(&mut self, scroll: ScrollHandle) {
        self.document_scroll = Some(scroll);
    }
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.clear_text_selections(None, cx);
        self.selected_block = None;
        cx.notify();
    }
    pub fn set_value(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_text_selections(None, cx);
        self.document = Document::parse(&value);
        self.row_bounds.borrow_mut().clear();
        self.type_menu
            .update(cx, |menu, cx| menu.set_open(false, cx));
        self.active = None;
        self.selected_block = None;
        self.drop_target = None;
        self.menu = false;
        self.undo.clear();
        self.redo.clear();
        self.sync_inputs(window, cx);
        cx.notify();
    }
    #[cfg(test)]
    pub fn content_font_family(&self) -> &str { self.font_family.as_ref() }
    pub fn set_font_family(&mut self, family: &str, cx: &mut Context<Self>) {
        if self.font_family.as_ref()!=family {
            self.font_family=family.to_owned().into();
            cx.notify();
        }
    }
    pub fn set_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        if self.font_size != size {
            self.font_size = size;
            cx.notify();
        }
    }
    pub fn read_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(window, cx);
        self.clear_text_selections(None, cx);
        self.type_menu
            .update(cx, |menu, cx| menu.set_open(false, cx));
        self.active = None;
        self.selected_block = None;
        self.drop_target = None;
        self.menu = false;
        cx.notify();
    }
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let i = self.active_index().unwrap_or(0);
        self.activate(i, self.document.blocks[i].text.len(), window, cx);
    }
    fn active_index(&self) -> Option<usize> {
        self.active
            .as_ref()
            .and_then(|id| self.document.blocks.iter().position(|b| &b.id == id))
    }
    fn state(&self, i: usize) -> Entity<InputState> {
        self.inputs[&self.document.blocks[i].id].state.clone()
    }
    fn snapshot(&self, cx: &App) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            active: self.active.clone(),
            selected_block: self.selected_block.clone(),
            text_selection: self.text_selection,
            cursor: self
                .active_index()
                .map(|i| self.state(i).read(cx).cursor())
                .unwrap_or(0),
        }
    }
    fn checkpoint(&mut self, cx: &App) {
        self.undo.push(self.snapshot(cx));
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_inputs(window, cx);
        cx.emit(EditorEvent::Change);
        cx.notify();
    }
    /// Inputs keep their identity and geometry while focus changes. There is no preview/editor swap.
    pub fn refresh_theme(&mut self, cx: &mut Context<Self>) {
        for block in &self.document.blocks {
            if let Some(input)=self.inputs.get(&block.id) {
                input.state.update(cx, |state,cx| state.set_inline_highlights(inline_styles(block),cx));
            }
        }
        cx.notify();
    }
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: HashSet<_> = self.document.blocks.iter().map(|b| b.id.clone()).collect();
        self.inputs.retain(|id, _| ids.contains(id));
        self.row_bounds
            .borrow_mut()
            .retain(|id, _| ids.contains(id));
        for block in &self.document.blocks {
            if !self.inputs.contains_key(&block.id) {
                let value = block.text.clone();
                let state = cx.new(|cx| {
                    let mut s = InputState::new(window, cx)
                        .auto_grow(1, 5000)
                        .block_mode(true)
                        .placeholder("Escreva ou digite /…");
                    s.set_value(value, window, cx);
                    s
                });
                let id = block.id.clone();
                let subscription = cx.subscribe_in(&state, window, move |this, _, event, w, cx| {
                    this.input_event(&id, event, w, cx)
                });
                self.inputs.insert(
                    block.id.clone(),
                    BlockInput {
                        state,
                        _subscription: subscription,
                    },
                );
            }
            let state = self.inputs[&block.id].state.clone();
            if state.read(cx).value().as_ref() != block.text {
                let value = block.text.clone();
                state.update(cx, |s, cx| s.set_value(value, window, cx));
            }
            let styles = inline_styles(block);
            state.update(cx, |s, cx| s.set_inline_highlights(styles, cx));
        }
    }
    fn point_in_document(&self, p: Point<Pixels>, cx: &App) -> Option<(usize, usize)> {
        let rows = self.row_bounds.borrow();
        let mut target = None;
        for (i, block) in self.document.blocks.iter().enumerate() {
            let Some(bounds) = rows.get(&block.id) else {
                continue;
            };
            target = Some(i);
            if p.y < bounds.bottom() {
                break;
            }
        }
        let i = target?;
        Some((i, self.state(i).read(cx).byte_offset_for_point(p)))
    }
    fn apply_text_selection(&mut self, selection: TextSelection, cx: &mut Context<Self>) {
        self.text_selection = Some(selection);
        self.selected_block = None;
        self.menu = false;
        self.active = Some(self.document.blocks[selection.anchor.0].id.clone());
        for (i, block) in self.document.blocks.iter().enumerate() {
            let range = selection.range(i, block.text.len());
            self.state(i).update(cx, |s, cx| {
                s.cancel_mouse_selection();
                s.set_byte_selection(range, cx);
            });
        }
        cx.notify();
    }
    fn select_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(window, cx);
        let last = self.document.blocks.len() - 1;
        self.clear_text_selections(None, cx);
        self.state(0).update(cx, |s, cx| s.focus(window, cx));
        self.apply_text_selection(
            TextSelection {
                anchor: (0, 0),
                head: (last, self.document.blocks[last].text.len()),
            },
            cx,
        );
        cx.emit(EditorEvent::Editing);
    }
    fn selected_text(&self) -> Option<String> {
        let selection = self.text_selection?;
        let (a, z) = selection.ordered();
        Some(
            (a.0..=z.0)
                .map(|i| {
                    let block = &self.document.blocks[i];
                    block.text[selection.range(i, block.text.len())].to_string()
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    }
    fn replace_text_selection(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.text_selection else {
            return;
        };
        self.checkpoint(cx);
        let (a, z) = selection.ordered();
        let (i, cursor) = self.document.replace_selection(a, z, text);
        self.clear_text_selections(None, cx);
        self.changed(window, cx);
        self.activate(i, cursor, window, cx);
    }
    fn collapse_document_selection(
        &mut self,
        end: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(selection) = self.text_selection else {
            cx.propagate();
            return;
        };
        let (a, z) = selection.ordered();
        let (i, cursor) = if end { z } else { a };
        self.activate(i, cursor, window, cx);
    }
    fn advance_selection_scroll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(pointer), Some(scroll)) = (self.selection_pointer, self.document_scroll.as_ref())
        else {
            return;
        };
        if self.mouse_anchor.is_none() || self.scroll_tick_pending {
            return;
        }
        let bounds = scroll.bounds();
        let delta = if pointer.y < bounds.top() + px(24.) {
            px(12.)
        } else if pointer.y > bounds.bottom() - px(24.) {
            px(-12.)
        } else {
            return;
        };
        let mut offset = scroll.offset();
        let next = (offset.y + delta).clamp(-scroll.max_offset().height, px(0.));
        if next == offset.y {
            return;
        }
        offset.y = next;
        scroll.set_offset(offset);
        self.scroll_tick_pending = true;
        let weak = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = weak.update(cx, |this, cx| {
                this.scroll_tick_pending = false;
                if let (Some(anchor), Some(pointer)) = (this.mouse_anchor, this.selection_pointer) {
                    if let Some(head) = this.point_in_document(pointer, cx) {
                        let selection = this.selection_for_pointer(anchor, head, cx);
                        this.apply_text_selection(selection, cx);
                        this.advance_selection_scroll(window, cx);
                    }
                }
            });
        });
        cx.notify();
    }
    fn selection_for_pointer(
        &self,
        anchor: (usize, usize),
        head: (usize, usize),
        cx: &App,
    ) -> TextSelection {
        let Some((unit, click_count)) = &self.mouse_unit else {
            return TextSelection { anchor, head };
        };
        if head.0 == anchor.0 && (unit.start..=unit.end).contains(&head.1) {
            return TextSelection {
                anchor: (anchor.0, unit.start),
                head: (anchor.0, unit.end),
            };
        }
        let forward = head >= (anchor.0, unit.start);
        let target = self
            .state(head.0)
            .read(cx)
            .selection_unit_for_offset(head.1, *click_count);
        TextSelection {
            anchor: (anchor.0, if forward { unit.start } else { unit.end }),
            head: (head.0, if forward { target.end } else { target.start }),
        }
    }
    fn extend_document_selection(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(anchor) = self.mouse_anchor else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) || cx.has_active_drag() {
            return;
        }
        let Some(head) = self.point_in_document(event.position, cx) else {
            return;
        };
        if head != anchor || self.text_selection.is_some() {
            let selection = self.selection_for_pointer(anchor, head, cx);
            self.apply_text_selection(selection, cx);
            self.selection_pointer = Some(event.position);
            self.advance_selection_scroll(window, cx);
            cx.stop_propagation();
        }
    }
    fn clear_text_selections(&mut self, except: Option<&str>, cx: &mut Context<Self>) {
        self.text_selection = None;
        self.mouse_anchor = None;
        self.mouse_unit = None;
        self.selection_pointer = None;
        for (id, input) in &self.inputs {
            if except == Some(id.as_str()) || input.state.read(cx).selection_range().is_empty() {
                continue;
            }
            input.state.update(cx, |state, cx| {
                let cursor = state.cursor();
                state.set_byte_selection(cursor..cursor, cx);
            });
        }
    }
    fn activate(&mut self, i: usize, cursor: usize, window: &mut Window, cx: &mut Context<Self>) {
        if i >= self.document.blocks.len() {
            return;
        }
        self.selected_block = None;
        self.active = Some(self.document.blocks[i].id.clone());
        self.clear_text_selections(None, cx);
        self.sync_inputs(window, cx);
        self.state(i).update(cx, |s, cx| {
            s.set_byte_selection(cursor..cursor, cx);
            s.focus(window, cx);
        });
        cx.emit(EditorEvent::Editing);
        cx.notify();
    }
    fn flush(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<_> = self.document.blocks.iter().map(|b| b.id.clone()).collect();
        for id in ids {
            self.input_event(&id, &InputEvent::Change, window, cx);
        }
    }
    fn input_event(
        &mut self,
        id: &str,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(i) = self.document.blocks.iter().position(|b| b.id == id) else {
            return;
        };
        if let InputEvent::BlockSelectionStart {
            offset,
            click_count,
        } = event
        {
            self.clear_text_selections(Some(id), cx);
            self.mouse_anchor = Some((i, *offset));
            if *click_count >= 2 {
                self.mouse_unit = Some((
                    self.state(i)
                        .read(cx)
                        .selection_unit_for_offset(*offset, *click_count),
                    *click_count,
                ));
            }
            self.selected_block = None;
            self.active = Some(id.into());
            self.menu = false;
            cx.emit(EditorEvent::Editing);
            cx.notify();
            return;
        }
        if matches!(event, InputEvent::Blur) {
            if self.text_selection.is_some() {
                return;
            }
            let state = self.state(i);
            if !state.read(cx).focus_handle(cx).is_focused(window)
                && !state.read(cx).selection_range().is_empty()
            {
                state.update(cx, |state, cx| {
                    let cursor = state.cursor();
                    state.set_byte_selection(cursor..cursor, cx);
                });
            }
            cx.notify();
            return;
        }
        if matches!(event, InputEvent::SelectionChanged) {
            cx.notify();
            return;
        }
        if matches!(event, InputEvent::Focus) {
            if !self.state(i).read(cx).focus_handle(cx).is_focused(window) {
                return;
            }
            if self.text_selection.is_some() {
                cx.notify();
                return;
            }
            let anchor = self.mouse_anchor;
            let unit = self.mouse_unit.clone();
            self.clear_text_selections(Some(id), cx);
            if anchor.is_some_and(|anchor| anchor.0 == i) {
                self.mouse_anchor = anchor;
                self.mouse_unit = unit;
            }
            self.selected_block = None;
            self.active = Some(id.into());
            self.menu = false;
            cx.emit(EditorEvent::Editing);
            cx.notify();
        }
        let state = self.state(i);
        if self.text_selection.is_some() && matches!(event, InputEvent::BlockEnter) {
            self.replace_text_selection("\n", window, cx);
            return;
        }
        let value = state.read(cx).value().to_string();
        if value != self.document.blocks[i].text {
            if let Some(selection) = self.text_selection {
                let old = &self.document.blocks[i].text;
                let local = selection.range(i, old.len());
                let suffix = old.len() - local.end;
                if value.len() >= local.start + suffix {
                    let replacement = value[local.start..value.len() - suffix].to_string();
                    self.replace_text_selection(&replacement, window, cx);
                    return;
                }
            }
        }
        if value != self.document.blocks[i].text {
            self.checkpoint(cx);
            self.document.blocks[i].edit(value);
            self.changed(window, cx);
            if self.active.as_deref() == Some(id) {
                let text = &self.document.blocks[i].text;
                self.menu = self.document.blocks[i].kind == Kind::Paragraph
                    && text.starts_with('/')
                    && !text.contains('\n');
                if self.document.blocks[i].kind == Kind::Paragraph {
                    let kind = match text.as_str() {
                        "# " => Some(Kind::Heading(1)),
                        "## " => Some(Kind::Heading(2)),
                        "### " => Some(Kind::Heading(3)),
                        "- " | "* " => Some(Kind::Bullet),
                        "1. " => Some(Kind::Number(1)),
                        "> " => Some(Kind::Quote),
                        "[] " | "- [ ] " => Some(Kind::Task(false)),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        self.document.blocks[i].edit(String::new());
                        self.convert(kind, window, cx);
                    }
                }
            }
        }
        match event {
            InputEvent::BlockEnter => {
                self.active = Some(id.into());
                if matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Source) {
                    state.update(cx, |s, cx| s.insert("\n", window, cx));
                    return;
                }
                if self.menu {
                    if let Some((_, kind)) = self.filtered_kinds().first().cloned() {
                        self.convert(kind, window, cx);
                    }
                    return;
                }
                self.checkpoint(cx);
                let next = self.document.split(i, state.read(cx).selection_range());
                self.changed(window, cx);
                self.activate(next, 0, window, cx);
            }
            InputEvent::BlockBackspace => {
                self.active = Some(id.into());
                if self.document.blocks[i].kind != Kind::Paragraph
                    && !matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Source)
                {
                    self.menu = false;
                    self.convert(Kind::Paragraph, window, cx);
                    return;
                }
                if i > 0 {
                    self.checkpoint(cx);
                    if let Some((next, cursor)) = self.document.merge_previous(i) {
                        self.changed(window, cx);
                        self.activate(next, cursor, window, cx);
                    }
                }
            }
            InputEvent::BlockDelete => {
                if i + 1 < self.document.blocks.len() {
                    self.checkpoint(cx);
                    if let Some((next, cursor)) = self.document.merge_previous(i + 1) {
                        self.changed(window, cx);
                        self.activate(next, cursor, window, cx);
                    }
                }
            }
            InputEvent::BlockPrevious => {
                if i > 0 {
                    self.activate(i - 1, self.document.blocks[i - 1].text.len(), window, cx);
                }
            }
            InputEvent::BlockNext => {
                if i + 1 < self.document.blocks.len() {
                    self.activate(i + 1, 0, window, cx);
                }
            }
            InputEvent::BlockUndo => self.history(false, window, cx),
            InputEvent::BlockRedo => self.history(true, window, cx),
            _ => {}
        }
    }
    fn history(&mut self, redo: bool, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        let Some(previous) = snapshot else { return };
        let current = self.snapshot(cx);
        if redo {
            self.undo.push(current)
        } else {
            self.redo.push(current)
        };
        self.document = previous.document;
        self.active = previous.active;
        self.menu = false;
        self.changed(window, cx);
        if let Some(selection) = previous.text_selection {
            self.state(selection.anchor.0)
                .update(cx, |s, cx| s.focus(window, cx));
            self.apply_text_selection(selection, cx);
        } else if let Some(id) = previous.selected_block {
            self.select_block(id, window, cx);
        } else if let Some(i) = self.active_index() {
            self.activate(i, previous.cursor, window, cx);
        }
    }
    fn convert(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.active_index() else { return };
        self.checkpoint(cx);
        let block = &mut self.document.blocks[i];
        if self.menu && self.selected_block.is_none() && block.text.starts_with('/') {
            block.edit(String::new());
        }
        block.kind = kind;
        block.invalidate();
        self.menu = false;
        self.changed(window, cx);
        if self.selected_block.is_none() {
            self.activate(i, self.document.blocks[i].text.len(), window, cx);
        }
    }
    fn format(&mut self, style: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selection) = self.text_selection {
            self.checkpoint(cx);
            let (a, z) = selection.ordered();
            let all = (a.0..=z.0)
                .filter(|i| {
                    !selection
                        .range(*i, self.document.blocks[*i].text.len())
                        .is_empty()
                })
                .all(|i| {
                    self.document.blocks[i].is_formatted(
                        selection.range(i, self.document.blocks[i].text.len()),
                        style,
                    )
                });
            for i in a.0..=z.0 {
                let range = selection.range(i, self.document.blocks[i].text.len());
                if self.document.blocks[i].is_formatted(range.clone(), style) == all {
                    self.document.blocks[i].format(range, style);
                }
            }
            self.changed(window, cx);
            self.apply_text_selection(selection, cx);
            return;
        }
        let Some(i) = self.active_index() else { return };
        let state = self.state(i);
        let range = state.read(cx).selection_range();
        if range.is_empty() {
            return;
        }
        self.checkpoint(cx);
        self.document.blocks[i].format(range.clone(), style);
        self.changed(window, cx);
        state.update(cx, |s, cx| {
            s.set_byte_selection(range, cx);
            s.focus(window, cx);
        });
    }
    fn focus_document_end(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.document.blocks.len().checked_sub(1) {
            if self.document.blocks[i].text.is_empty()
                && self.document.blocks[i].kind == Kind::Paragraph
            {
                self.activate(i, 0, window, cx);
                return;
            }
        }
        self.add_after(None, window, cx);
    }
    fn add_after(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.checkpoint(cx);
        let next = id
            .as_ref()
            .and_then(|id| self.document.blocks.iter().position(|b| &b.id == id))
            .map(|i| i + 1)
            .unwrap_or(self.document.blocks.len());
        self.document
            .blocks
            .insert(next, Block::new(Kind::Paragraph, String::new()));
        self.changed(window, cx);
        self.activate(next, 0, window, cx);
    }
    fn remove(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.active_index() else { return };
        self.checkpoint(cx);
        self.document.remove(i);
        self.changed(window, cx);
        self.activate(i.min(self.document.blocks.len() - 1), 0, window, cx);
    }
    fn select_block(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if !self.inputs.contains_key(&id) {
            return;
        }
        self.clear_text_selections(None, cx);
        self.active = Some(id.clone());
        self.selected_block = Some(id);
        self.drop_target = None;
        self.menu = false;
        self.type_menu
            .update(cx, |menu, cx| menu.set_open(false, cx));
        self.sync_type_menu(cx);
        self.focus_handle.focus(window);
        cx.emit(EditorEvent::Editing);
        cx.notify();
    }
    fn sync_type_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(i) = self.active_index() {
            let items = type_menu_items(&self.document.blocks[i].kind);
            self.type_menu
                .update(cx, |menu, cx| menu.set_items(items, cx));
        }
    }
    fn update_drop_target(&mut self, e: &DragMoveEvent<BlockDrag>, cx: &mut Context<Self>) {
        if e.drag(cx).owner != cx.entity_id() {
            return;
        }
        // GPUI calls on_drag_move globally, even outside each element's bounds.
        // One document-level listener must choose one insertion slot for the whole document.
        let bounds = self.row_bounds.borrow();
        let rows: Vec<_> = self
            .document
            .blocks
            .iter()
            .filter_map(|b| bounds.get(&b.id).map(|bounds| (b.id.clone(), *bounds)))
            .collect();
        let target =
            if e.event.position.x >= e.bounds.left() && e.event.position.x <= e.bounds.right() {
                insertion_target(&rows, e.event.position.y)
            } else {
                None
            };
        drop(bounds);
        if self.drop_target != target {
            self.drop_target = target;
            cx.notify();
        }
    }
    fn drop_block(&mut self, drag: &BlockDrag, window: &mut Window, cx: &mut Context<Self>) {
        if drag.owner != cx.entity_id() {
            return;
        }
        let Some((target, after)) = self.drop_target.take() else {
            return;
        };
        let snapshot = self.snapshot(cx);
        if self.document.move_to(&drag.id, &target, after) {
            self.undo.push(snapshot);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.changed(window, cx);
        }
        self.select_block(drag.id.clone(), window, cx);
    }
    fn filtered_kinds(&self) -> Vec<(&'static str, Kind)> {
        let query = self
            .active_index()
            .filter(|i| {
                self.selected_block.is_none()
                    && self.menu
                    && self.document.blocks[*i].text.starts_with('/')
            })
            .map(|i| {
                self.document.blocks[i]
                    .text
                    .trim_start_matches('/')
                    .to_lowercase()
            })
            .unwrap_or_default();
        block_types()
            .into_iter()
            .map(|(label, kind, _)| (label, kind))
            .filter(|(label, _)| label.to_lowercase().contains(&query))
            .collect()
    }
    fn toolbar(&self, text_selection: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let mut origin = point(px(0.), px(-38.));
        if text_selection {
            if let Some(i) = self
                .selection_toolbar_index(cx)
                .or_else(|| self.active_index())
            {
                let id = &self.document.blocks[i].id;
                if let (Some(selection), Some(row)) = (
                    self.state(i).read(cx).selection_first_line_bounds(),
                    self.row_bounds.borrow().get(id).copied(),
                ) {
                    // Five 28px icon buttons, four 4px gaps and 8px horizontal padding.
                    let width = px(164.);
                    origin.x = (selection.center().x - row.left() - width / 2.)
                        .max(px(0.))
                        .min((row.size.width - width).max(px(0.)));
                    origin.y = selection.top() - row.top() - px(42.);
                }
            }
        }
        let mut tools = div()
            .id("block-toolbar")
            .absolute()
            .top(origin.y)
            .left(origin.x)
            .h(px(34.))
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .rounded_md()
            .bg(neutral(0x2c2c2c))
            .shadow_sm()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default());
        {
            let geometry = self.row_bounds.clone();
            tools = tools.child(
                canvas(
                    move |bounds, _, _| {
                        geometry
                            .borrow_mut()
                            .insert("__toolbar".to_string(), bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        if text_selection {
            for (id, label, style, icon) in [
                ("bold", "Negrito (⌘B)", "bold", "iconoir/regular/bold.svg"),
                (
                    "italic",
                    "Itálico (⌘I)",
                    "italic",
                    "iconoir/regular/italic.svg",
                ),
                (
                    "underline",
                    "Sublinhado (⌘U)",
                    "underline",
                    "iconoir/regular/underline.svg",
                ),
                (
                    "strike",
                    "Riscado",
                    "strike",
                    "iconoir/regular/strikethrough.svg",
                ),
                (
                    "code",
                    "Código (⌘E)",
                    "code",
                    "iconoir/regular/code-brackets.svg",
                ),
            ] {
                tools = tools.child(
                    Tooltip::new(id, label).child(
                        Button::icon(id, icon)
                            .size(ButtonSize::IconSm)
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(move |this, _, w, cx| this.format(style, w, cx))),
                    ),
                );
            }
        }
        if text_selection {
            return tools;
        }
        tools
            .child(self.type_menu.clone())
            .child(
                Tooltip::new("delete-tip", "Excluir bloco").child(
                    Button::icon("block-delete", "iconoir/regular/trash.svg")
                        .size(ButtonSize::IconSm)
                        .variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(|this, _, w, cx| this.remove(w, cx))),
                ),
            )
            .child(
                Tooltip::new("undo-tip", "Desfazer").child(
                    Button::icon("block-undo", "iconoir/regular/undo.svg")
                        .size(ButtonSize::IconSm)
                        .variant(ButtonVariant::Ghost)
                        .disabled(self.undo.is_empty())
                        .on_click(cx.listener(|this, _, w, cx| this.history(false, w, cx))),
                ),
            )
            .child(
                Tooltip::new("redo-tip", "Refazer").child(
                    Button::icon("block-redo", "iconoir/regular/redo.svg")
                        .size(ButtonSize::IconSm)
                        .variant(ButtonVariant::Ghost)
                        .disabled(self.redo.is_empty())
                        .on_click(cx.listener(|this, _, w, cx| this.history(true, w, cx))),
                ),
            )
    }
    fn selection_toolbar_index(&self, cx: &App) -> Option<usize> {
        let selection = self.text_selection?;
        let (a, z) = selection.ordered();
        (a.0..=z.0).find(|i| !self.state(*i).read(cx).selection_range().is_empty())
    }
}
/// Bounds are window coordinates from the current layout, including scrolling and wrapping.
/// Half of a block selects its preceding slot; the other half selects the following slot.
/// Gaps remain covered, and the answer is independent of drag direction and listener order.
fn insertion_target(rows: &[(String, Bounds<Pixels>)], y: Pixels) -> Option<(String, bool)> {
    rows.iter()
        .find(|(_, bounds)| y < bounds.center().y)
        .map(|(id, _)| (id.clone(), false))
        .or_else(|| rows.last().map(|(id, _)| (id.clone(), true)))
}
fn block_types() -> Vec<(&'static str, Kind, &'static str)> {
    vec![
        ("Texto", Kind::Paragraph, "iconoir/regular/text.svg"),
        (
            "Título 1",
            Kind::Heading(1),
            "iconoir/regular/text-size.svg",
        ),
        (
            "Título 2",
            Kind::Heading(2),
            "iconoir/regular/text-size.svg",
        ),
        (
            "Título 3",
            Kind::Heading(3),
            "iconoir/regular/text-size.svg",
        ),
        ("Lista", Kind::Bullet, "iconoir/regular/list.svg"),
        (
            "Lista numerada",
            Kind::Number(1),
            "iconoir/regular/numbered-list-left.svg",
        ),
        (
            "Checklist",
            Kind::Task(false),
            "iconoir/regular/task-list.svg",
        ),
        ("Citação", Kind::Quote, "iconoir/regular/quote.svg"),
        (
            "Código",
            Kind::Code(String::new()),
            "iconoir/regular/code-brackets.svg",
        ),
        ("Divisor", Kind::Divider, "iconoir/regular/minus.svg"),
    ]
}
fn kind_display(kind: &Kind) -> (&'static str, &'static str) {
    let normalized = match kind {
        Kind::Task(_) => Kind::Task(false),
        Kind::Number(_) => Kind::Number(1),
        Kind::Code(_) => Kind::Code(String::new()),
        _ => kind.clone(),
    };
    block_types()
        .into_iter()
        .find(|(_, k, _)| *k == normalized)
        .map(|(label, _, icon)| (label, icon))
        .unwrap_or(("Markdown", "iconoir/regular/code-brackets.svg"))
}
fn type_menu_items(current: &Kind) -> Vec<MenuItem> {
    let selected_label = kind_display(current).0;
    block_types()
        .into_iter()
        .map(|(label, _, icon)| {
            MenuItem::new(label).icon(if label == selected_label {
                "iconoir/regular/check.svg"
            } else {
                icon
            })
        })
        .collect()
}
fn inline_styles(block: &Block) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    block
        .spans
        .iter()
        .map(|s| {
            let m = &s.marks;
            (
                s.range.clone(),
                HighlightStyle {
                    font_weight: Some(if m.bold || matches!(block.kind, Kind::Heading(_)) {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    }),
                    font_style: Some(if m.italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    }),
                    color: m.link.as_ref().map(|_| neutral(0xd4d4d4).into()),
                    background_color: m.code.then_some(neutral(0x303030).into()),
                    strikethrough: m.strike.then_some(StrikethroughStyle {
                        thickness: px(1.),
                        color: None,
                    }),
                    underline: (m.underline || m.link.is_some()).then_some(UnderlineStyle {
                        thickness: px(1.),
                        color: None,
                        wavy: false,
                    }),
                    ..Default::default()
                },
            )
        })
        .collect()
}
impl Render for BlockEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dragging = cx.has_active_drag();
        let mut root = div()
            .id("blocks")
            .key_context("BlockEditor")
            .track_focus(&self.focus_handle)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(px(self.font_size))
            .line_height(relative(1.6))
            .on_drag_move(cx.listener(|this, e: &DragMoveEvent<BlockDrag>, _, cx| {
                this.update_drop_target(e, cx)
            }))
            .on_drop(
                cx.listener(|this, drag: &BlockDrag, window, cx| this.drop_block(drag, window, cx)),
            )
            .on_action(cx.listener(|this, _: &FormatBold, w, cx| this.format("bold", w, cx)))
            .on_action(cx.listener(|this, _: &FormatItalic, w, cx| this.format("italic", w, cx)))
            .on_action(
                cx.listener(|this, _: &FormatUnderline, w, cx| this.format("underline", w, cx)),
            )
            .on_action(cx.listener(|this, _: &FormatCode, w, cx| this.format("code", w, cx)));
        root = root
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveLeft, w, cx| {
                    this.collapse_document_selection(false, w, cx)
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveUp, w, cx| {
                    this.collapse_document_selection(false, w, cx)
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveRight, w, cx| {
                    this.collapse_document_selection(true, w, cx)
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveDown, w, cx| {
                    this.collapse_document_selection(true, w, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &SelectDocument, w, cx| this.select_document(w, cx)))
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::SelectAll, w, cx| {
                    this.select_document(w, cx)
                }),
            )
            .capture_action(cx.listener(|this, _: &gpui_component::input::Copy, _, cx| {
                if let Some(text) = this.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                } else {
                    cx.propagate();
                }
            }))
            .capture_action(cx.listener(|this, _: &gpui_component::input::Cut, w, cx| {
                if let Some(text) = this.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    this.replace_text_selection("", w, cx);
                } else {
                    cx.propagate();
                }
            }))
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::Paste, w, cx| {
                    if this.text_selection.is_some() {
                        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                            this.replace_text_selection(&text, w, cx);
                        }
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::Backspace, w, cx| {
                    if this.text_selection.is_some() {
                        this.replace_text_selection("", w, cx);
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::Delete, w, cx| {
                    if this.text_selection.is_some() {
                        this.replace_text_selection("", w, cx);
                    } else {
                        cx.propagate();
                    }
                }),
            );
        let weak = cx.entity().downgrade();
        root = root.child(
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    let selection_owner = weak.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture {
                            let _ = selection_owner.update(cx, |this, cx| {
                                this.extend_document_selection(event, window, cx)
                            });
                        }
                    });
                    let selection_owner = weak.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            let _ = selection_owner.update(cx, |this, cx| {
                                this.selection_pointer = None;
                                if this.mouse_anchor.take().is_some()
                                    && this.text_selection.is_some()
                                {
                                    cx.stop_propagation();
                                }
                            });
                        }
                    });
                    let selection_owner = weak.clone();
                    window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            let _ = selection_owner.update(cx, |this, cx| {
                                if this.text_selection.is_some()
                                    && !this
                                        .row_bounds
                                        .borrow()
                                        .values()
                                        .any(|bounds| bounds.contains(&event.position))
                                {
                                    this.clear_selection(cx);
                                }
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_full(),
        );
        for i in 0..self.document.blocks.len() {
            let block = &self.document.blocks[i];
            let id = block.id.clone();
            let interaction_id = id.clone();
            let kind = block.kind.clone();
            let active = self.active.as_ref() == Some(&id);
            let selected = self.selected_block.as_ref() == Some(&id)
                && (self.focus_handle.is_focused(window)
                    || self
                        .type_menu
                        .read(cx)
                        .focus_handle(cx)
                        .within_focused(window, cx)
                    || self.type_menu.read(cx).is_open());
            let text_selection = self
                .selection_toolbar_index(cx)
                .map(|first| first == i)
                .unwrap_or_else(|| {
                    active
                        && self.selected_block.is_none()
                        && !self.state(i).read(cx).selection_range().is_empty()
                        && self.state(i).read(cx).focus_handle(cx).is_focused(window)
                });
            let size = match kind {
                Kind::Heading(n) => {
                    self.font_size
                        + match n {
                            1 => 8.,
                            2 => 5.,
                            _ => 2.,
                        }
                }
                Kind::Code(_) => self.font_size - 3.,
                _ => self.font_size,
            };
            let mut row = div()
                .id(SharedString::from(format!("block-{id}")))
                .capture_any_mouse_down(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if event.button != MouseButton::Left {
                        return;
                    }
                    if this.text_selection.is_some()
                        && this
                            .row_bounds
                            .borrow()
                            .get("__toolbar")
                            .is_some_and(|bounds| bounds.contains(&event.position))
                    {
                        return;
                    }
                    this.clear_text_selections(Some(&interaction_id), cx);
                    let in_content =
                        this.row_bounds
                            .borrow()
                            .get(&interaction_id)
                            .is_some_and(|bounds| {
                                bounds.contains(&event.position)
                                    && event.position.x >= bounds.left() + px(58.)
                            });
                    if in_content {
                        if event.click_count == 1 {
                            this.mouse_anchor = this.point_in_document(event.position, cx);
                        }
                        this.selected_block = None;
                        this.active = Some(interaction_id.clone());
                        this.menu = false;
                        cx.emit(EditorEvent::Editing);
                        cx.notify();
                    }
                }))
                .relative()
                .group("block-row")
                .rounded_md()
                .when(selected, |row| row.bg(neutral(0x383838)))
                .w_full()
                .min_w_0()
                .flex()
                .items_start()
                .gap_2()
                .py_1()
                .pl(px(58.))
                .text_size(px(size))
                .line_height(relative(1.6));
            let geometry = self.row_bounds.clone();
            let measured_id = id.clone();
            row = row.child(
                canvas(
                    move |bounds, _, _| {
                        geometry.borrow_mut().insert(measured_id.clone(), bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
            if let Some((target, after)) = &self.drop_target {
                if target == &id && dragging {
                    #[allow(unused_mut)]
                    let mut line = div()
                        .absolute()
                        .left(px(58.))
                        .right_0()
                        .h(px(2.))
                        .bg(neutral(0xd4d4d4))
                        .when(*after, |d| d.bottom(px(-5.)))
                        .when(!*after, |d| d.top(px(-5.)));
                    #[cfg(test)]
                    {
                        let geometry = self.row_bounds.clone();
                        line = line.child(
                            canvas(
                                move |bounds, _, _| {
                                    geometry
                                        .borrow_mut()
                                        .insert("__drop_line".to_string(), bounds);
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        );
                    }
                    row = row.child(deferred(line).with_priority(4));
                }
            }
            let add_id = id.clone();
            let select_id = id.clone();
            let drag = BlockDrag {
                id: id.clone(),
                owner: cx.entity_id(),
                label: block.text.chars().take(60).collect(),
            };
            let weak = cx.entity().downgrade();
            let controls = div()
                .absolute()
                .top(px(4.))
                .left(px(0.))
                .flex()
                .gap_1()
                .text_size(px(20.))
                .text_color(neutral(0x999999))
                .when(dragging, |d| d.opacity(0.))
                .when(!dragging, |d| {
                    d.opacity(0.).group_hover("block-row", |s| s.opacity(1.))
                })
                .child(
                    div()
                        .id(SharedString::from(format!("add-{id}")))
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .rounded_sm()
                        .hover(|s| s.bg(neutral(0x2c2c2c)))
                        .child(
                            svg()
                                .path("iconoir/regular/plus.svg")
                                .size(px(18.))
                                .text_color(neutral(0x999999)),
                        )
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.add_after(Some(add_id.clone()), w, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("drag-{id}")))
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::OpenHand)
                        .rounded_sm()
                        .hover(|s| s.bg(neutral(0x2c2c2c)))
                        .child(
                            svg()
                                .path(crate::assets::BLOCK_GRIP)
                                .size(px(18.))
                                .text_color(neutral(0x999999)),
                        )
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.select_block(select_id.clone(), w, cx);
                            cx.stop_propagation();
                        }))
                        .on_drag(drag, move |drag, _, w, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.select_block(drag.id.clone(), w, cx);
                                // on_drag begins after this frame's render; force a redraw
                                // with the drag installed to remove the toolbar immediately.
                                cx.notify();
                            });
                            cx.stop_propagation();
                            cx.new(|_| drag.clone())
                        }),
                );
            row = row.child(controls);
            let marker = match kind {
                Kind::Bullet => Some("•".into()),
                Kind::Number(n) => Some(format!("{n}.")),
                Kind::Quote => Some("▎".into()),
                _ => None,
            };
            if let Some(marker) = marker {
                row = row.child(div().flex_none().font_family(self.font_family.clone()).text_color(neutral(0xd4d4d4)).child(marker));
            }
            if let Kind::Task(done) = kind {
                row = row.child(
                    div()
                        .id(SharedString::from(format!("check-{id}")))
                        .flex_none()
                        .cursor_pointer()
                        .child(if done { "☑" } else { "☐" })
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.checkpoint(cx);
                            this.document.blocks[i].kind = Kind::Task(!done);
                            this.document.blocks[i].invalidate();
                            this.changed(w, cx);
                            cx.stop_propagation();
                        })),
                );
            }
            if kind == Kind::Divider {
                row = row.child(
                    div()
                        .id(SharedString::from(format!("divider-{id}")))
                        .w_full()
                        .h(px(2.))
                        .my_3()
                        .bg(neutral(0x414141))
                        .on_click(cx.listener(move |this, _, w, cx| this.activate(i, 0, w, cx))),
                );
            } else if kind == Kind::Source && !active && self.text_selection.is_none() {
                let raw = block.markdown();
                let cursor = block.text.len();
                row = row.child(
                    div()
                        .id(SharedString::from(format!("source-{id}")))
                        .font_family(self.font_family.clone())
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .cursor_text()
                        .child(
                            TextView::markdown(
                                SharedString::from(format!("md-{id}")),
                                raw,
                                window,
                                cx,
                            )
                            .w_full()
                            .h_auto()
                            .style(TextViewStyle {
                                heading_base_font_size: px(self.font_size),
                                is_dark: cx.theme().is_dark(),
                                highlight_theme: cx.theme().highlight_theme.clone(),
                                ..Default::default()
                            }),
                        )
                        .on_click(
                            cx.listener(move |this, _, w, cx| this.activate(i, cursor, w, cx)),
                        ),
                );
            } else {
                let mut input = NativeInput::new(&self.state(i))
                    .font_family(self.font_family.clone())
                    .bare_metrics()
                    .appearance(false)
                    .w_full()
                    .min_w_0()
                    .text_size(px(size))
                    .line_height(relative(1.6));
                if matches!(kind, Kind::Heading(_)) {
                    input = input.font_weight(FontWeight::BOLD);
                }
                let mut cell = div().flex_1().min_w_0();
                if matches!(kind, Kind::Code(_)) {
                    input = input.font_family("Menlo");
                    cell = cell.bg(neutral(0x202020)).rounded_md().p_2();
                }
                row = row.child(cell.child(input));
            }
            if !dragging && (selected || text_selection) {
                // The toolbar is already absolute and paints after this block/previous blocks.
                // Menu and Tooltip manage their own deferred popups; nesting deferred
                // elements would panic in GPUI when either popup opens.
                row = row.child(self.toolbar(text_selection, cx));
            }
            if !dragging && active && self.menu {
                let mut menu = div()
                    .id("slash-menu")
                    .absolute()
                    .top_full()
                    .left_0()
                    .w(px(330.))
                    .max_w_full()
                    .p_2()
                    .rounded_lg()
                    .bg(neutral(0x2c2c2c))
                    .shadow_md()
                    .occlude()
                    .flex()
                    .flex_wrap()
                    .gap_1();
                for (j, (label, kind)) in self.filtered_kinds().into_iter().enumerate() {
                    menu =
                        menu.child(
                            Button::new(SharedString::from(format!("kind-{j}")), label)
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(move |this, _, w, cx| {
                                    this.convert(kind.clone(), w, cx)
                                })),
                        );
                }
                row = row.child(deferred(menu).with_priority(3));
            }
            root = root.child(row);
        }
        root.child(
            div()
                .id("document-end")
                .w_full()
                .h(px(self.font_size * 1.6 + 24.))
                .cursor(CursorStyle::IBeam)
                .on_click(cx.listener(|this, _, w, cx| this.focus_document_end(w, cx))),
        )
    }
}

/// Real GPUI interaction tests, confined to an invisible, unfocused window.
#[cfg(test)]
#[allow(dead_code)]
pub fn verify_native_editor(cx: &mut App) {
    init(cx);
    struct Stage {
        editor: Entity<BlockEditor>,
    }
    impl Render for Stage {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let root = div().size_full().p(px(60.));
            if let Some(scroll) = self.editor.read(cx).document_scroll.clone() {
                root.child(
                    div()
                        .id("selection-test-scroll")
                        .w_full()
                        .h(px(150.))
                        .overflow_y_scroll()
                        .track_scroll(&scroll)
                        .child(self.editor.clone()),
                )
            } else {
                root.child(self.editor.clone())
            }
        }
    }
    fn frame(handle: AnyWindowHandle, cx: &mut App) {
        cx.update_window(handle, |_, w, cx| w.draw(cx).clear())
            .unwrap();
    }
    fn event(handle: AnyWindowHandle, event: PlatformInput, cx: &mut App) {
        cx.update_window(handle, |_, w, cx| {
            let _ = w.dispatch_event(event, cx);
        })
        .unwrap();
        frame(handle, cx);
    }
    fn movement(handle: AnyWindowHandle, p: Point<Pixels>, dragging: bool, cx: &mut App) {
        event(
            handle,
            PlatformInput::MouseMove(MouseMoveEvent {
                position: p,
                pressed_button: dragging.then_some(MouseButton::Left),
                ..Default::default()
            }),
            cx,
        );
    }
    fn down(handle: AnyWindowHandle, p: Point<Pixels>, cx: &mut App) {
        event(
            handle,
            PlatformInput::MouseDown(MouseDownEvent {
                position: p,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
    }
    fn up(handle: AnyWindowHandle, p: Point<Pixels>, cx: &mut App) {
        event(
            handle,
            PlatformInput::MouseUp(MouseUpEvent {
                position: p,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
    }
    let source = "Primeiro **forte**\n\n## Segundo\n\nTerceiro com várias linhas de conteúdo para variar a altura do bloco conforme a largura da janela. As linhas devem continuar alinhadas durante todo o arraste.\n\nQuarto";
    let mut editor = None;
    let window = cx
        .open_window(
            WindowOptions {
                show: false,
                focus: false,
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(640.), px(640.)),
                ))),
                ..Default::default()
            },
            |w, cx| {
                let entity = cx.new(|cx| {
                    let mut editor = BlockEditor::new(w, cx);
                    editor.set_value(source.to_string(), w, cx);
                    editor
                });
                editor = Some(entity.clone());
                let stage = cx.new(|_| Stage { editor: entity });
                cx.new(|cx| gpui_component::Root::new(stage, w, cx))
            },
        )
        .unwrap();
    let handle: AnyWindowHandle = window.into();
    let editor = editor.unwrap();
    frame(handle, cx);
    let ids: Vec<_> = editor
        .read(cx)
        .document
        .blocks
        .iter()
        .map(|b| b.id.clone())
        .collect();
    let bounds = |id: &str, cx: &App| editor.read(cx).row_bounds.borrow()[id];
    let grip = |id: &str, cx: &App| bounds(id, cx).origin + point(px(40.), px(16.));
    let first_geometry = bounds(&ids[0], cx);
    // Drag last to first, reverse direction over the gap, then continue upwards.
    let start = grip(&ids[3], cx);
    movement(handle, start, false, cx);
    down(handle, start, cx);
    movement(handle, start + point(px(0.), px(-8.)), true, cx);
    assert!(cx.has_active_drag(), "the native grip must start a drag");
    let top = bounds(&ids[0], cx).origin + point(px(90.), px(2.));
    movement(handle, top, true, cx);
    assert_eq!(editor.read(cx).drop_target, Some((ids[0].clone(), false)));
    assert_eq!(
        bounds("__drop_line", cx).center().y,
        bounds(&ids[0], cx).top() - px(4.)
    );
    assert_eq!(
        bounds("__drop_line", cx).left(),
        bounds(&ids[0], cx).left() + px(58.)
    );
    let gap = point(top.x, bounds(&ids[1], cx).bottom() + px(4.));
    movement(handle, gap, true, cx);
    assert_eq!(editor.read(cx).drop_target, Some((ids[2].clone(), false)));
    assert_eq!(bounds("__drop_line", cx).center().y, gap.y);
    movement(handle, top, true, cx);
    assert_eq!(editor.read(cx).drop_target, Some((ids[0].clone(), false)));
    up(handle, top, cx);
    assert_eq!(
        editor.read(cx).document.blocks[0].id,
        ids[3],
        "upward drop must match its preview"
    );
    // Drag the new first block below the last block.
    let start = grip(&ids[3], cx);
    movement(handle, start, false, cx);
    down(handle, start, cx);
    movement(handle, start + point(px(0.), px(8.)), true, cx);
    let last = bounds(&ids[2], cx);
    let bottom = point(last.left() + px(90.), last.bottom() - px(2.));
    movement(handle, bottom, true, cx);
    assert_eq!(editor.read(cx).drop_target, Some((ids[2].clone(), true)));
    assert_eq!(bounds("__drop_line", cx).center().y, last.bottom() + px(4.));
    up(handle, bottom, cx);
    assert_eq!(
        editor
            .read(cx)
            .document
            .blocks
            .iter()
            .map(|b| b.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        editor.read(cx).document.markdown(),
        source,
        "dragging retains Markdown and rich spans"
    );
    // A simple grip click selects the block, then the real dropdown opens and converts it.
    let p = grip(&ids[0], cx);
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert_eq!(editor.read(cx).selected_block.as_ref(), Some(&ids[0]));
    assert_eq!(
        bounds(&ids[0], cx),
        first_geometry,
        "block selection must not move the layout"
    );
    let toolbar = bounds("__toolbar", cx);
    let trigger = point(toolbar.left() + px(38.), toolbar.center().y);
    movement(handle, trigger, false, cx);
    down(handle, trigger, cx);
    up(handle, trigger, cx);
    let menu = editor.read(cx).type_menu.clone();
    assert!(
        menu.read(cx).is_open(),
        "type dropdown must stay mounted after it gets focus"
    );
    assert_eq!(
        bounds(&ids[0], cx),
        first_geometry,
        "dropdown must not move the layout"
    );
    let popup = menu
        .read(cx)
        .popup_bounds()
        .expect("dropdown must render its anchored popup");
    let heading = point(popup.left() + px(60.), popup.top() + px(5. + 28. + 14.));
    movement(handle, heading, false, cx);
    down(handle, heading, cx);
    up(handle, heading, cx);
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Heading(1));
    assert_eq!(editor.read(cx).document.blocks[0].text, "Primeiro forte");
    assert!(!menu.read(cx).is_open());
    assert_eq!(editor.read(cx).selected_block.as_ref(), Some(&ids[0]));
    // Plus belongs to its own row, even when a different block is selected.
    let b = bounds(&ids[1], cx);
    let plus = b.origin + point(px(12.), px(16.));
    movement(handle, plus, false, cx);
    down(handle, plus, cx);
    up(handle, plus, cx);
    assert_eq!(editor.read(cx).document.blocks.len(), 5);
    assert!(editor.read(cx).document.blocks[2].text.is_empty());
    assert_eq!(editor.read(cx).document.blocks[3].id, ids[2]);
    // A real double click in another input replaces the previous text selection.
    for id in [&ids[0], &ids[1]] {
        let b = bounds(id, cx);
        let p = b.origin + point(px(72.), px(18.));
        movement(handle, p, false, cx);
        event(
            handle,
            PlatformInput::MouseDown(MouseDownEvent {
                position: p,
                button: MouseButton::Left,
                click_count: 2,
                ..Default::default()
            }),
            cx,
        );
        up(handle, p, cx);
        assert!(!editor.read(cx).inputs[id]
            .state
            .read(cx)
            .selection_range()
            .is_empty());
        assert_eq!(
            editor
                .read(cx)
                .inputs
                .values()
                .filter(|input| { !input.state.read(cx).selection_range().is_empty() })
                .count(),
            1,
            "only one block may retain a text selection"
        );
    }
    // Anchor the toolbar to a word on a wrapped visual line, using actual shaped metrics.
    let b = bounds(&ids[2], cx);
    let p = b.origin + point(px(72.), px(18.));
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    let state = editor.read(cx).inputs[&ids[2]].state.clone();
    let word = state.read(cx).value().find("janela").unwrap();
    let range = word..word + "janela".len();
    state.update(cx, |s, cx| s.set_byte_selection(range.clone(), cx));
    frame(handle, cx);
    let selection = state.read(cx).selection_first_line_bounds().unwrap();
    let toolbar = bounds("__toolbar", cx);
    assert!(
        selection.top() > b.top() + px(35.),
        "the fixture must select a wrapped line"
    );
    assert_eq!(
        toolbar.bottom(),
        selection.top() - px(8.),
        "toolbar must sit above the selected visual line"
    );
    let expected_center = selection
        .center()
        .x
        .max(b.left() + px(82.))
        .min(b.right() - px(82.));
    assert!(
        (toolbar.center().x - expected_center).abs() < px(1.),
        "toolbar must center on the selected word within the document width"
    );
    let p = point(toolbar.left() + px(18.), toolbar.center().y);
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert_eq!(
        state.read(cx).selection_range(),
        range,
        "formatting must preserve the selection at the floating toolbar"
    );
    assert!(editor.read(cx).document.markdown().contains("**janela**"));
    // A simple click deselects text, and the blank document tail appends at the end.
    let p = bounds(&ids[0], cx).origin + point(px(72.), px(18.));
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert!(editor.read(cx).inputs.values().all(|input| input
        .state
        .read(cx)
        .selection_range()
        .is_empty()));
    let p = point(
        bounds(&ids[3], cx).left() + px(72.),
        bounds(&ids[3], cx).bottom() + px(22.),
    );
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert_eq!(editor.read(cx).document.blocks.len(), 6);
    let last = editor.read(cx).document.blocks.last().unwrap().id.clone();
    assert!(editor
        .read(cx)
        .document
        .blocks
        .last()
        .unwrap()
        .text
        .is_empty());
    assert_eq!(editor.read(cx).active.as_ref(), Some(&last));
    let p = point(
        bounds(&last, cx).left() + px(72.),
        bounds(&last, cx).bottom() + px(22.),
    );
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert_eq!(
        editor.read(cx).document.blocks.len(),
        6,
        "clicking below an empty final paragraph must reuse it"
    );
    // One document selection spans native inputs, in both pointer directions.
    let rich = "Olá **ação** 💚 com uma frase longa para verificar a seleção começando no meio de linhas quebradas. O texto deve continuar selecionável ao atravessar outros blocos, preservando palavras, acentos e a posição original do clique durante todo o gesto.\n\n## Meio forte\n\nÚltimo vídeo";
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.set_value(rich.into(), w, cx))
    })
    .unwrap();
    frame(handle, cx);
    frame(handle, cx);
    frame(handle, cx);
    let selection_ids: Vec<_> = editor
        .read(cx)
        .document
        .blocks
        .iter()
        .map(|b| b.id.clone())
        .collect();
    let first = bounds(&selection_ids[0], cx);
    let last = bounds(&selection_ids[2], cx);
    let first_start = point(first.left() + px(59.), first.top() + px(18.));
    let last_end = point(last.right() - px(10.), last.top() + px(18.));
    let plain = editor
        .read(cx)
        .document
        .blocks
        .iter()
        .map(|b| b.text.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    for (a, z) in [(first_start, last_end), (last_end, first_start)] {
        movement(handle, a, false, cx);
        down(handle, a, cx);
        movement(handle, z, true, cx);
        up(handle, z, cx);
        assert_eq!(
            editor.read(cx).selected_text().as_deref(),
            Some(plain.as_str()),
            "both drag directions select all intervening text"
        );
        for i in 0..3 {
            assert_eq!(
                editor.read(cx).state(i).read(cx).selection_range(),
                0..editor.read(cx).document.blocks[i].text.len()
            );
        }
    }
    // Start inside words, move within the starting block, then cross several rows.
    let first_mid = point(first.left() + px(110.), first.top() + px(18.));
    let last_mid = point(last.left() + px(120.), last.top() + px(18.));
    assert!(
        first.size.height > px(90.),
        "the fixture must have several wrapped visual lines"
    );
    let wrapped_mid = point(first.left() + px(160.), first.top() + px(80.));
    for (start, end, index) in [
        (first_mid, last_mid, 0),
        (last_mid, first_mid, 2),
        (wrapped_mid, last_mid, 0),
        (last_mid, wrapped_mid, 2),
    ] {
        let first_point = if index == 0 { start } else { end };
        let last_point = if index == 0 { end } else { start };
        let a = editor
            .read(cx)
            .state(0)
            .read(cx)
            .byte_offset_for_point(first_point);
        let z = editor
            .read(cx)
            .state(2)
            .read(cx)
            .byte_offset_for_point(last_point);
        assert!(a > 0 && a < editor.read(cx).document.blocks[0].text.len());
        assert!(z > 0 && z < editor.read(cx).document.blocks[2].text.len());
        let partial = format!(
            "{}\n\n{}\n\n{}",
            &editor.read(cx).document.blocks[0].text[a..],
            editor.read(cx).document.blocks[1].text,
            &editor.read(cx).document.blocks[2].text[..z]
        );
        movement(handle, start, false, cx);
        down(handle, start, cx);
        let initial = editor
            .read(cx)
            .mouse_anchor
            .expect("a middle-of-word press must record an anchor");
        let id = selection_ids[index].clone();
        cx.update_window(handle, |_, w, cx| {
            editor.update(cx, |this, cx| {
                // The native input owns the hit test, even when the row wrapper misses the press.
                this.mouse_anchor = None;
                this.input_event(
                    &id,
                    &InputEvent::BlockSelectionStart {
                        offset: initial.1,
                        click_count: 1,
                    },
                    w,
                    cx,
                );
                this.input_event(&id, &InputEvent::Focus, w, cx)
            })
        })
        .unwrap();
        let local = point(
            start.x + if index == 0 { px(10.) } else { px(-10.) },
            start.y,
        );
        movement(handle, local, true, cx);
        assert!(
            editor.read(cx).text_selection.is_some(),
            "the document must own the gesture while it is still inside the starting block"
        );
        assert_eq!(
            editor.read(cx).mouse_anchor,
            Some(initial),
            "moving inside the initial block must keep the original anchor"
        );
        let middle = bounds(&selection_ids[1], cx);
        movement(handle, point(start.x, middle.center().y), true, cx);
        movement(handle, end, true, cx);
        up(handle, end, cx);
        assert_eq!(
            editor.read(cx).selected_text().as_deref(),
            Some(partial.as_str()),
            "partial selection must preserve both middle-of-word endpoints in either direction"
        );
    }
    // A new click clears that one selection; shortcuts select the entire document.
    movement(handle, first_start, false, cx);
    down(handle, first_start, cx);
    up(handle, first_start, cx);
    assert!(editor.read(cx).text_selection.is_none());
    for key in ["cmd-a", "ctrl-a"] {
        event(
            handle,
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
            }),
            cx,
        );
        assert_eq!(
            editor.read(cx).selected_text().as_deref(),
            Some(plain.as_str()),
            "select-all shortcut must override the focused input"
        );
    }
    // Formatting is uniform across blocks and does not alter their kinds.
    let toolbar = bounds("__toolbar", cx);
    let p = point(toolbar.left() + px(18.), toolbar.center().y);
    movement(handle, p, false, cx);
    down(handle, p, cx);
    up(handle, p, cx);
    assert_eq!(
        editor.read(cx).selected_text().as_deref(),
        Some(plain.as_str()),
        "the floating toolbar must preserve the entire document selection"
    );
    for i in 0..3 {
        let block = &editor.read(cx).document.blocks[i];
        assert!(block.is_formatted(0..block.text.len(), "bold"));
    }
    assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Heading(2));
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.history(false, w, cx))
    })
    .unwrap();
    assert_eq!(editor.read(cx).document.markdown(), rich);
    // Native typing replaces the whole selection in a single undoable document edit.
    let state = editor.read(cx).state(0);
    cx.update_window(handle, |_, w, cx| {
        state.update(cx, |s, cx| s.replace_text_in_range(None, "Novo 💚", w, cx));
        editor.update(cx, |this, cx| {
            this.input_event(&selection_ids[0], &InputEvent::Change, w, cx)
        });
    })
    .unwrap();
    assert_eq!(editor.read(cx).document.blocks.len(), 1);
    assert_eq!(editor.read(cx).document.blocks[0].text, "Novo 💚");
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.history(false, w, cx))
    })
    .unwrap();
    assert_eq!(editor.read(cx).document.markdown(), rich);
    frame(handle, cx);
    event(
        handle,
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: Keystroke::parse("backspace").unwrap(),
            is_held: false,
        }),
        cx,
    );
    assert_eq!(editor.read(cx).document.blocks.len(), 1);
    assert!(editor.read(cx).document.blocks[0].text.is_empty());
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.history(false, w, cx))
    })
    .unwrap();
    assert_eq!(editor.read(cx).document.markdown(), rich);
    frame(handle, cx);
    event(
        handle,
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: Keystroke::parse("right").unwrap(),
            is_held: false,
        }),
        cx,
    );
    assert!(editor.read(cx).text_selection.is_none());
    assert_eq!(editor.read(cx).active.as_ref(), Some(&selection_ids[2]));
    assert!(editor.read(cx).inputs.values().all(|input| input
        .state
        .read(cx)
        .selection_range()
        .is_empty()));
    // Dragging toward a clipped viewport starts scrolling without changing content.
    let scroll = ScrollHandle::new();
    let long = (0..30)
        .map(|i| format!("Parágrafo {i}: ação 💚"))
        .collect::<Vec<_>>()
        .join("\n\n");
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| {
            this.set_value(long.clone(), w, cx);
            this.set_scroll_handle(scroll.clone());
        })
    })
    .unwrap();
    frame(handle, cx);
    let first_id = editor.read(cx).document.blocks[0].id.clone();
    let first = bounds(&first_id, cx);
    let p = point(first.left() + px(59.), first.top() + px(18.));
    movement(handle, p, false, cx);
    down(handle, p, cx);
    let edge = point(p.x, scroll.bounds().bottom() - px(2.));
    movement(handle, edge, true, cx);
    assert!(
        scroll.offset().y < px(0.),
        "selection at the viewport edge must start auto scrolling"
    );
    assert!(editor.read(cx).text_selection.is_some());
    up(handle, edge, cx);
    assert!(editor.read(cx).mouse_anchor.is_none());
    assert_eq!(editor.read(cx).document.markdown(), long);
    cx.update_window(handle, |_, w, _| w.remove_window())
        .unwrap();
    println!("Native editor verified: drag/drop, toolbar, implicit insertion, bidirectional document selection, Cmd/Ctrl+A, formatting, native replacement, deletion and undo.");
}

/// Yield to GPUI between every gesture step so real focus/selection subscriptions run.
#[cfg(test)]
#[allow(dead_code)]
pub fn verify_native_selection_async(cx: &mut App) {
    struct Stage {
        editor: Entity<BlockEditor>,
    }
    impl Render for Stage {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().p(px(60.)).child(self.editor.clone())
        }
    }
    cx.spawn(async move |cx| {
        let (handle, editor) = cx.update(|cx| {
            let mut editor = None;
            let window = cx.open_window(WindowOptions {
                show: false, focus: false,
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(0.), px(0.)), size(px(640.), px(640.))))),
                ..Default::default()
            }, |w, cx| {
                let entity = cx.new(|cx| {
                    let mut editor = BlockEditor::new(w, cx);
                    editor.set_value("Olá **ação** 💚 com uma frase longa para começar o arraste no meio das linhas quebradas. As palavras e os acentos precisam manter a posição durante todo o gesto.\n\n## Bloco intermediário\n\nÚltimo parágrafo com várias palavras e vídeo para testar o arraste no meio do texto.".into(), w, cx);
                    editor
                });
                editor = Some(entity.clone());
                let stage = cx.new(|_| Stage { editor: entity });
                cx.new(|cx| gpui_component::Root::new(stage, w, cx))
            }).unwrap();
            (window.into(), editor.unwrap())
        }).unwrap();
        for _ in 0..4 {
            cx.update(|cx| cx.update_window(handle, |_, w, cx| w.draw(cx).clear()).unwrap()).unwrap();
            cx.background_executor().timer(std::time::Duration::from_millis(15)).await;
        }
        for (direction, click_count) in [(false, 1), (true, 1), (false, 2), (true, 2), (false, 3), (true, 3)] {
            let (start, end, middle, local, expected) = cx.update(|cx| {
                let doc = &editor.read(cx).document;
                let rows = editor.read(cx).row_bounds.borrow();
                let first = rows[&doc.blocks[0].id];
                let last = rows[&doc.blocks[2].id];
                let mid = rows[&doc.blocks[1].id];
                let a = point(first.left() + px(140.), first.top() + px(65.));
                let z = point(last.left() + px(150.), last.top() + px(18.));
                let a_byte = editor.read(cx).state(0).read(cx).byte_offset_for_point(a);
                let z_byte = editor.read(cx).state(2).read(cx).byte_offset_for_point(z);
                assert!(a_byte > 0 && a_byte < doc.blocks[0].text.len());
                assert!(z_byte > 0 && z_byte < doc.blocks[2].text.len());
                let first_unit = editor.read(cx).state(0).read(cx).selection_unit_for_offset(a_byte, click_count);
                let last_unit = editor.read(cx).state(2).read(cx).selection_unit_for_offset(z_byte, click_count);
                let expected = format!("{}\n\n{}\n\n{}", &doc.blocks[0].text[first_unit.start..], doc.blocks[1].text, &doc.blocks[2].text[..last_unit.end]);
                let (start, end) = if direction { (z, a) } else { (a, z) };
                (start, end, point(start.x, mid.center().y), point(start.x + px(12.), start.y), expected)
            }).unwrap();
            let events = [
                PlatformInput::MouseMove(MouseMoveEvent { position: start, ..Default::default() }),
                PlatformInput::MouseDown(MouseDownEvent { position: start, button: MouseButton::Left, click_count, ..Default::default() }),
                PlatformInput::MouseMove(MouseMoveEvent { position: local, pressed_button: Some(MouseButton::Left), ..Default::default() }),
                PlatformInput::MouseMove(MouseMoveEvent { position: middle, pressed_button: Some(MouseButton::Left), ..Default::default() }),
                PlatformInput::MouseMove(MouseMoveEvent { position: end, pressed_button: Some(MouseButton::Left), ..Default::default() }),
                PlatformInput::MouseUp(MouseUpEvent { position: end, button: MouseButton::Left, click_count, ..Default::default() }),
            ];
            for (step, event) in events.into_iter().enumerate() {
                cx.update(|cx| {
                    cx.update_window(handle, |_, w, cx| { let _ = w.dispatch_event(event, cx); }).unwrap();
                }).unwrap();
                cx.background_executor().timer(std::time::Duration::from_millis(15)).await;
                cx.update(|cx| {
                    cx.update_window(handle, |_, w, cx| w.draw(cx).clear()).unwrap();
                    eprintln!("Gesture clicks={click_count} direction={direction} step={step}: anchor={:?} selection={:?}", editor.read(cx).mouse_anchor, editor.read(cx).text_selection);
                }).unwrap();
            }
            cx.update(|cx| {
                assert_eq!(editor.read(cx).selected_text().as_deref(), Some(expected.as_str()), "partial drag must survive real focus/selection event delivery between movements");
            }).unwrap();
        }
        cx.update(|cx| {
            cx.update_window(handle, |_, w, _| w.remove_window()).unwrap();
            println!("Async native selection verified: partial wrapped-line drag in both directions with real event delivery.");
            cx.quit();
        }).unwrap();
    }).detach();
}

#[cfg(test)]
mod tests {
    use super::{inline_styles, Document};
    use gpui::{FontStyle, FontWeight, KeyBindingContextPredicate, KeyContext};
    #[test]
    fn insertion_slots_cover_both_directions_and_gaps() {
        use super::insertion_target;
        use gpui::{point, px, size, Bounds};
        let rows: Vec<_> = [("a", 100., 40.), ("b", 148., 120.), ("c", 276., 40.)]
            .into_iter()
            .map(|(id, top, height)| {
                (
                    id.to_string(),
                    Bounds::new(point(px(20.), px(top)), size(px(400.), px(height))),
                )
            })
            .collect();
        let positions = [
            (90., "a", false),
            (110., "a", false),
            (120., "b", false),
            (144., "b", false),
            (200., "b", false),
            (208., "c", false),
            (272., "c", false),
            (290., "c", false),
            (296., "c", true),
            (340., "c", true),
        ];
        for (y, id, after) in positions.into_iter().chain(positions.into_iter().rev()) {
            assert_eq!(
                insertion_target(&rows, px(y)),
                Some((id.to_string(), after))
            );
        }
        let shifted: Vec<_> = rows
            .iter()
            .map(|(id, bounds)| {
                let mut b = *bounds;
                b.origin.y -= px(80.);
                (id.clone(), b)
            })
            .collect();
        assert_eq!(
            insertion_target(&shifted, px(64.)),
            Some(("b".into(), false))
        );
        assert_eq!(insertion_target(&[], px(0.)), None);
    }
    #[test]
    fn preview_slot_is_the_actual_upward_and_downward_drop() {
        use super::insertion_target;
        use gpui::{point, px, size, Bounds};
        let mut d = Document::parse("primeiro\n\nsegundo\n\nterceiro");
        let ids: Vec<_> = d.blocks.iter().map(|b| b.id.clone()).collect();
        let rows: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    Bounds::new(
                        point(px(0.), px(100. + i as f32 * 48.)),
                        size(px(400.), px(40.)),
                    ),
                )
            })
            .collect();
        let (target, after) = insertion_target(&rows, px(101.)).unwrap();
        assert!(d.move_to(&ids[2], &target, after));
        assert_eq!(d.blocks[0].id, ids[2]);
        let rows: Vec<_> = d
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                (
                    b.id.clone(),
                    Bounds::new(
                        point(px(0.), px(100. + i as f32 * 48.)),
                        size(px(400.), px(40.)),
                    ),
                )
            })
            .collect();
        let (target, after) = insertion_target(&rows, px(250.)).unwrap();
        assert!(d.move_to(&ids[2], &target, after));
        assert_eq!(
            d.blocks.iter().map(|b| b.id.clone()).collect::<Vec<_>>(),
            ids
        );
    }
    #[test]
    fn rich_runs_reset_normal_style_after_marks() {
        let d = Document::parse("normal **forte** *itálico* normal");
        let runs = inline_styles(&d.blocks[0]);
        assert_eq!(
            runs.iter().map(|(r, _)| r.len()).sum::<usize>(),
            d.blocks[0].text.len()
        );
        assert!(runs
            .iter()
            .any(|(_, s)| s.font_weight == Some(FontWeight::BOLD)));
        assert!(runs
            .iter()
            .any(|(_, s)| s.font_style == Some(FontStyle::Italic)));
        let last = &runs.last().unwrap().1;
        assert_eq!(last.font_weight, Some(FontWeight::NORMAL));
        assert_eq!(last.font_style, Some(FontStyle::Normal));
        let heading = Document::parse("# Título **forte**");
        assert!(inline_styles(&heading.blocks[0])
            .iter()
            .all(|(_, s)| s.font_weight == Some(FontWeight::BOLD)));
    }
    #[test]
    fn formatting_shortcuts_require_an_editor_input() {
        let predicate = KeyBindingContextPredicate::parse("BlockEditor > Input").unwrap();
        let contexts: Vec<KeyContext> = ["Sparkpad", "BlockEditor", "Input"]
            .into_iter()
            .map(|s| KeyContext::parse(s).unwrap())
            .collect();
        assert!(predicate.eval_inner(&contexts, &contexts));
        let title: Vec<KeyContext> = ["Sparkpad", "Input"]
            .into_iter()
            .map(|s| KeyContext::parse(s).unwrap())
            .collect();
        assert!(!predicate.eval_inner(&title, &title));
    }
}
