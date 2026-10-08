use crate::app_theme::neutral;
use crate::blocks::{Block, Document, Kind};
use empire_ui::{
    menu::{Menu, MenuAlign, MenuEvent, MenuItem},
    Button, ButtonSize, ButtonVariant, Checkbox, CheckboxEvent, Tooltip,
};
use gpui::{prelude::*, *};
use gpui_component::{
    input::{Input as NativeInput, InputEvent, InputState},
    text::{TextView, TextViewStyle},
    ActiveTheme,
};
use std::{
    cell::{Cell,RefCell},
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
        SelectDocument,
        PastePlain
    ]
);
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-a", SelectDocument, Some("BlockEditor > Input")),
        KeyBinding::new("ctrl-a", SelectDocument, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-a", SelectDocument, Some("BlockEditor")),
        KeyBinding::new("ctrl-a", SelectDocument, Some("BlockEditor")),
        KeyBinding::new("cmd-c", gpui_component::input::Copy, Some("BlockEditor > Input")),
        KeyBinding::new("ctrl-c", gpui_component::input::Copy, Some("BlockEditor > Input")),
        KeyBinding::new("cmd-c", gpui_component::input::Copy, Some("BlockEditor")),
        KeyBinding::new("ctrl-c", gpui_component::input::Copy, Some("BlockEditor")),
        KeyBinding::new("cmd-x", gpui_component::input::Cut, Some("BlockEditor")),
        KeyBinding::new("ctrl-x", gpui_component::input::Cut, Some("BlockEditor")),
        KeyBinding::new("cmd-v", gpui_component::input::Paste, Some("BlockEditor")),
        KeyBinding::new("ctrl-v", gpui_component::input::Paste, Some("BlockEditor")),
        KeyBinding::new("cmd-shift-v", PastePlain, Some("BlockEditor")),
        KeyBinding::new("ctrl-shift-v", PastePlain, Some("BlockEditor")),
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
    highlight:Rc<RefCell<crate::code_highlight::CodeHighlight>>,
    highlight_language:Option<String>,
    language_menu: Option<Entity<Menu>>,
    _language_subscription: Option<Subscription>,
    checkbox: Option<Entity<Checkbox>>,
    _checkbox_subscription: Option<Subscription>,
}
struct CellInput {state:Entity<InputState>,_subscription:Subscription}
struct TableInputs {row_heights:Rc<RefCell<HashMap<usize,f32>>>,source:String,model:crate::markdown_table::MarkdownTable,cells:HashMap<(usize,usize),CellInput>}
#[derive(Clone,Copy)]
enum TableChange {AddRow,AddColumn,RemoveRow(usize),RemoveColumn(usize)}
pub struct BlockEditor {
    document: Document,
    shared: Option<sparkpad_sync::CollaborativeText>,
    asset_directory: Option<std::path::PathBuf>,
    resizing_image: Option<String>,
    media_error: Option<String>,
    cached_markdown:SharedString,
    row_heights:Rc<RefCell<HashMap<String,f32>>>,
    estimated_heights:HashMap<String,f32>,
    layout_origin:Rc<Cell<Option<Point<Pixels>>>>,
    layout_width:Rc<Cell<f32>>,
    inputs: HashMap<String, BlockInput>,
    tables:HashMap<String,TableInputs>,
    active: Option<String>,
    selected_block: Option<String>,
    text_selection: Option<TextSelection>,
    mouse_anchor: Option<(usize, usize)>,
    mouse_unit: Option<(std::ops::Range<usize>, usize)>,
    selection_pointer: Option<Point<Pixels>>,
    document_scroll: Option<ScrollHandle>,
    scroll_tick_pending: bool,
    drop_target: Option<(String, bool)>,
    copied_code: Option<(String, std::time::Instant)>,
    hovered_code: Option<String>,
    hovered_block:Option<String>,
    hover_motion:HashMap<String,empire_ui::motion::Tween>,
    row_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    slash_menu: Entity<Menu>,
    _slash_subscription: Subscription,
    type_menu: Entity<Menu>,
    _type_menu_subscription: Subscription,
    focus_handle: FocusHandle,
    font_size: f32,
    end_space: f32,
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
            Menu::new(type_menu_items(&Kind::Paragraph), cx).motion(true)
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
        let slash_menu = cx.new(|cx| Menu::new(Vec::new(), cx).motion(true).scroll_area(true)
            .align(MenuAlign::Start).width(320.)
            .trigger(|_, _, _| div().w(px(1.)).h(px(1.)).into_any_element()));
        let slash_subscription = cx.subscribe_in(&slash_menu, window, |this, _, event, window, cx| {
            match event {
                MenuEvent::Select(index) => {
                    // The first row is a nonselectable group label.
                    if let Some((_, kind)) = index.checked_sub(1).and_then(|i| this.filtered_kinds().get(i).cloned()) {
                        this.convert(kind, window, cx);
                    } else { this.menu = false; cx.notify(); }
                }
                MenuEvent::OpenChange(false) => { this.menu = false; cx.notify(); }
                _ => {}
            }
        });
        let mut editor = Self {
            document: Document::parse(""),
            cached_markdown:"".into(),row_heights:Default::default(),estimated_heights:Default::default(),layout_origin:Default::default(),layout_width:Rc::new(Cell::new(480.)),
            shared: None,
            asset_directory:None,resizing_image:None,media_error:None,
            inputs: HashMap::new(),
            tables:HashMap::new(),
            active: None,
            selected_block: None,
            text_selection: None,
            mouse_anchor: None,
            mouse_unit: None,
            selection_pointer: None,
            document_scroll: None,
            scroll_tick_pending: false,
            drop_target: None,
            copied_code:None,
            hovered_code:None,
            hovered_block:None,hover_motion:HashMap::new(),
            row_bounds: Rc::new(RefCell::new(HashMap::new())),
            slash_menu,
            _slash_subscription: slash_subscription,
            type_menu,
            _type_menu_subscription: subscription,
            focus_handle: cx.focus_handle(),
            font_size: 22.,
            end_space: 0.,
            font_family: crate::assets::FONT_FAMILY.into(),
            menu: false,
            undo: Vec::new(),
            redo: Vec::new(),
        };
        editor.sync_inputs(window, cx);
        editor
    }
    pub fn value(&self) -> SharedString {
        self.cached_markdown.clone()
    }
    pub fn set_asset_directory(&mut self,path:std::path::PathBuf){self.asset_directory=Some(path);}
    fn choose_image(&mut self,id:String,window:&mut Window,cx:&mut Context<Self>){
        let Some(root)=self.asset_directory.clone() else {self.media_error=Some("O armazenamento de imagens não está disponível.".into());cx.notify();return;};
        let prompt=cx.prompt_for_paths(PathPromptOptions{files:true,directories:false,multiple:false,prompt:Some("Inserir imagem".into())});
        cx.spawn_in(window,async move |this,cx|{
            if let Ok(Ok(Some(paths)))=prompt.await {if let Some(path)=paths.first(){
                let path=path.clone();let result=cx.background_executor().spawn(async move {crate::document_images::import(&root,&path)}).await;
                let _=this.update_in(cx,|this,w,cx|{
                    match result {Ok(url)=>if let Some(index)=this.document.blocks.iter().position(|b|b.id==id){
                        this.checkpoint(cx);this.document.blocks[index].kind=Kind::Image{url,title:None};this.document.blocks[index].invalidate();this.media_error=None;this.changed(w,cx);
                    },Err(error)=>{this.media_error=Some(error.to_string());cx.notify();}}
                });
            }}
        }).detach();
    }
    fn set_image_width(&mut self,id:&str,width:u16,window:&mut Window,cx:&mut Context<Self>){
        let Some(index)=self.document.blocks.iter().position(|b|b.id==id) else{return;};self.checkpoint(cx);
        if let Kind::Image{title,..}=&mut self.document.blocks[index].kind {*title=crate::blocks::resized_image_title(title.as_deref(),width);self.document.blocks[index].invalidate();self.changed(window,cx);}
    }
    fn resize_image(&mut self,event:&MouseMoveEvent,window:&mut Window,cx:&mut Context<Self>)->bool {
        let Some(id)=self.resizing_image.clone() else{return false;};
        let Some(bounds)=self.row_bounds.borrow().get(&id).copied() else{return false;};
        let available=(f32::from(bounds.size.width)-74.).max(100.);
        let width=(((f32::from(event.position.x-bounds.origin.x)-58.)/available)*100.).round().clamp(10.,100.) as u16;
        if let Some(index)=self.document.blocks.iter().position(|b|b.id==id){
            if let Kind::Image{title,..}=&mut self.document.blocks[index].kind {
                if crate::blocks::image_width(title.as_deref())!=width {*title=crate::blocks::resized_image_title(title.as_deref(),width);self.document.blocks[index].invalidate();self.changed(window,cx);}
            }
        }
        cx.stop_propagation();true
    }
    pub fn has_shared_document(&self)->bool {self.shared.is_some()}
    pub fn bind_shared_document(&mut self,state:&[u8])->anyhow::Result<()> {
        let actor=u64::from_le_bytes(uuid::Uuid::new_v4().as_bytes()[..8].try_into().unwrap()) & ((1u64<<53)-1);
        let mut shared=sparkpad_sync::CollaborativeText::load(actor,state)?;
        // Keep any keystrokes entered while the initial sync was being prepared.
        shared.edit(&self.cached_markdown);
        self.shared=Some(shared);self.undo.clear();self.redo.clear();Ok(())
    }
    pub fn shared_update(&mut self,title:&str,vector:&[u8])->anyhow::Result<Option<Vec<u8>>> {
        let Some(shared)=&mut self.shared else {return Ok(None)};
        shared.document.set_local_text("title",title);
        Ok(Some(shared.document.delta(vector)?))
    }
    pub fn apply_shared_state(&mut self,state:&[u8],window:&mut Window,cx:&mut Context<Self>)->anyhow::Result<()> {
        if self.shared.is_none() {self.bind_shared_document(state)?;}
        let shared=self.shared.as_mut().unwrap();shared.document.apply(state)?;
        let value=shared.document.text("body");
        self.reconcile_value(&value,window,cx);Ok(())
    }
    fn reconcile_value(&mut self,value:&str,window:&mut Window,cx:&mut Context<Self>) {
        if self.cached_markdown.as_ref()==value {return;}
        let old=self.document.clone();let mut next=Document::parse(value);
        let mut used=HashSet::new();let mut matches=vec![None;next.blocks.len()];
        // Reuse native inputs for unchanged and moved blocks before matching edited blocks.
        for (i,block) in next.blocks.iter().enumerate() {
            if let Some(j)=old.blocks.iter().enumerate().find_map(|(j,b)|(!used.contains(&j) && b.kind==block.kind && b.text==block.text).then_some(j)) {matches[i]=Some(j);used.insert(j);}
        }
        for (i,block) in next.blocks.iter().enumerate() {
            if matches[i].is_some() {continue;}
            let candidate=old.blocks.iter().enumerate().filter(|(j,b)|!used.contains(j) && std::mem::discriminant(&b.kind)==std::mem::discriminant(&block.kind))
                .min_by_key(|(j,_)|j.abs_diff(i)).map(|(j,_)|j);
            if let Some(j)=candidate {matches[i]=Some(j);used.insert(j);}
        }
        let mut selections=Vec::new();
        for (i,source) in matches.iter().enumerate() {
            if let Some(j)=source {
                let previous=&old.blocks[*j];next.blocks[i].id=previous.id.clone();
                if let Some(input)=self.inputs.get(&previous.id) {
                    let range=input.state.read(cx).selection_range();
                    let a=sparkpad_sync::transform_cursor(&previous.text,&next.blocks[i].text,range.start);
                    let b=sparkpad_sync::transform_cursor(&previous.text,&next.blocks[i].text,range.end);
                    selections.push((previous.id.clone(),a.min(b)..a.max(b)));
                }
            }
        }
        let selection=self.text_selection.and_then(|selection| {
            let map=|p:(usize,usize)| {matches.iter().position(|j|*j==Some(p.0)).map(|i|(i,sparkpad_sync::transform_cursor(&old.blocks[p.0].text,&next.blocks[i].text,p.1)))};
            Some(TextSelection {anchor:map(selection.anchor)?,head:map(selection.head)?})
        });
        self.document=next;self.cached_markdown=value.to_owned().into();self.mouse_anchor=None;self.mouse_unit=None;
        if self.active.as_ref().is_some_and(|id|!self.document.blocks.iter().any(|b|&b.id==id)) {self.active=None;}
        if self.selected_block.as_ref().is_some_and(|id|!self.document.blocks.iter().any(|b|&b.id==id)) {self.selected_block=None;}
        self.sync_inputs(window,cx);
        for (id,range) in selections {if let Some(input)=self.inputs.get(&id) {input.state.update(cx,|s,cx|s.set_byte_selection(range,cx));}}
        if let Some(selection)=selection {self.apply_text_selection(selection,cx);} else {self.text_selection=None;}
        cx.notify();
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
        self.shared = None;
        self.document = Document::parse(&value);
        self.cached_markdown=value.into();self.row_heights.borrow_mut().clear();self.estimated_heights.clear();self.hover_motion.clear();
        self.hovered_code = None;
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
            self.font_family=family.to_owned().into();self.row_heights.borrow_mut().clear();self.estimated_heights.clear();
            for table in self.tables.values(){table.row_heights.borrow_mut().clear();}
            cx.notify();
        }
    }
    pub fn set_end_space(&mut self, height: f32, cx: &mut Context<Self>) {
        if (self.end_space - height).abs() > 0.5 {
            self.end_space = height;
            cx.notify();
        }
    }
    pub fn set_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        if self.font_size != size {
            self.font_size = size;self.row_heights.borrow_mut().clear();self.estimated_heights.clear();
            for table in self.tables.values(){table.row_heights.borrow_mut().clear();}
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
        if let Some(shared)=&mut self.shared {shared.boundary();return;}
        self.undo.push(self.snapshot(cx));
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cached_markdown=self.document.markdown().into();
        if let Some(shared)=&mut self.shared {shared.edit(&self.cached_markdown);}
        self.sync_inputs(window, cx);
        cx.emit(EditorEvent::Change);
        cx.notify();
    }
    /// Inputs keep their identity and geometry while focus changes. There is no preview/editor swap.
    pub fn refresh_theme(&mut self, cx: &mut Context<Self>) {
        for block in &self.document.blocks {
            if let Some(input)=self.inputs.get_mut(&block.id) {
                let styles = match &block.kind {
                    Kind::Code(_) => Vec::new(),
                    _ => inline_styles(block),
                };
                input.state.update(cx, |state,cx| {
                    state.set_inline_highlights(styles,cx);
                    state.set_inline_font_ranges(inline_fonts(block),cx);
                });
            }
        }
        for table in self.tables.values_mut(){for (r,row) in table.model.rows.iter().enumerate(){for (c,block) in row.iter().enumerate(){if let Some(cell)=table.cells.get(&(r,c)){cell.state.update(cx,|s,cx|{s.set_inline_highlights(inline_styles(block),cx);s.set_inline_font_ranges(inline_fonts(block),cx);});}}}}
        cx.notify();
    }
    // Keep native fields for the viewport and the active/selection endpoints only.
    // The document model remains complete; virtualization never truncates Markdown.
    fn virtual_rows(&mut self,window:&Window)->Vec<(bool,f32,f32)> {
        let scroll=self.document_scroll.as_ref().map(|s|s.offset().y).unwrap_or(px(0.));
        let bounds=self.document_scroll.as_ref().map(|s|s.bounds()).filter(|b|b.size.height>px(0.));
        let viewport=bounds.unwrap_or(Bounds::new(point(px(0.),px(0.)),window.viewport_size()));
        let origin=self.layout_origin.get().map(|p|p.y+scroll).unwrap_or(viewport.top()+px(150.));
        let width=self.layout_width.get().max(120.);
        let start=f32::from(viewport.top())-500.;let end=f32::from(viewport.bottom())+500.;
        let virtualize=self.document.blocks.len()>80;
        let heights=self.row_heights.borrow();let mut top=f32::from(origin);let mut rows=Vec::with_capacity(self.document.blocks.len());
        for (i,block) in self.document.blocks.iter().enumerate(){
            let height=heights.get(&block.id).copied().unwrap_or_else(||{
                let estimate=self.estimated_heights.entry(block.id.clone()).or_insert_with(||{
                    let size=self.font_size+if matches!(block.kind,Kind::Heading(1)){8.}else if matches!(block.kind,Kind::Heading(_)){3.}else{0.};
                    let chars=((width-74.)/(size*0.53)).max(8.) as usize;
                    let lines=block.text.lines().map(|line|(line.chars().count().max(1)+chars-1)/chars).sum::<usize>().max(1);
                    match &block.kind{Kind::Divider=>26.,Kind::Image{..}=>320.,Kind::Table=>block.text.lines().count().saturating_sub(2)as f32*48.+118.,Kind::Code(_)=>lines.min(5000)as f32*(self.font_size-3.)*1.6+60.,_=>lines.min(5000)as f32*size*1.6+8.}
                });*estimate
            });
            let pinned=self.active.as_ref()==Some(&block.id)||self.selected_block.as_ref()==Some(&block.id)||self.text_selection.is_some_and(|s|s.anchor.0==i||s.head.0==i);
            rows.push((!virtualize||pinned||(top+height>=start&&top<=end),height,top));top+=height+8.;
        }
        rows
    }
    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted=self.virtual_rows(window);
        let ids: HashSet<_> = self.document.blocks.iter().map(|b| b.id.clone()).collect();
        self.inputs.retain(|id, _| ids.contains(id));
        self.row_bounds
            .borrow_mut()
            .retain(|id, _| ids.contains(id));
        for (index, block) in self.document.blocks.iter().enumerate() {
            if !wanted[index].0 && !self.inputs.contains_key(&block.id){continue;}
            let placeholder = if index + 1 == self.document.blocks.len() {
                "Escreva ou digite /…"
            } else {
                ""
            };
            if !self.inputs.contains_key(&block.id) {
                let value = block.text.clone();
                let state = cx.new(|cx| {
                    let mut s = InputState::new(window, cx)
                        .auto_grow(1, 5000)
                        .block_mode(true)
                        .placeholder(placeholder);
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
                        highlight: Default::default(),highlight_language:None,
                        language_menu: None,
                        _language_subscription: None,
                        checkbox: None,
                        _checkbox_subscription: None,
                    },
                );
            }
            let state = self.inputs[&block.id].state.clone();
            if let Kind::Task(done) = block.kind {
                if self.inputs[&block.id].checkbox.is_none() {
                    let checkbox = cx.new(|cx| Checkbox::new("", done, cx));
                    let block_id = block.id.clone();
                    let subscription = cx.subscribe_in(&checkbox, window, move |this, _, event, w, cx| {
                        let CheckboxEvent::Toggle(done) = event;
                        let Some(index) = this.document.blocks.iter().position(|block| block.id == block_id) else {
                            return;
                        };
                        if !matches!(this.document.blocks[index].kind, Kind::Task(current) if current != *done) {
                            return;
                        }
                        this.checkpoint(cx);
                        this.document.blocks[index].kind = Kind::Task(*done);
                        this.document.blocks[index].invalidate();
                        this.changed(w, cx);
                    });
                    let input = self.inputs.get_mut(&block.id).unwrap();
                    input.checkbox = Some(checkbox);
                    input._checkbox_subscription = Some(subscription);
                }
                self.inputs[&block.id].checkbox.as_ref().unwrap().update(cx, |checkbox, cx| {
                    checkbox.set_checked(done, cx);
                });
            } else {
                let input = self.inputs.get_mut(&block.id).unwrap();
                input.checkbox = None;
                input._checkbox_subscription = None;
            }
            if state.read(cx).value().as_ref() != block.text {
                let value = block.text.clone();
                state.update(cx, |s, cx| s.set_value(value, window, cx));
            }
            if let Kind::Code(language) = &block.kind {
                if self.inputs[&block.id].language_menu.is_none() {
                    let weak = cx.entity().downgrade();
                    let block_id = block.id.clone();
                    let trigger_id = block_id.clone();
                    let menu = cx.new(|cx| Menu::new(code_language_items(language), cx)
                        .motion(true).align(MenuAlign::End).width(230.)
                        .trigger(move |_, _, cx| {
                            let label = weak.upgrade().and_then(|editor| {
                                editor.read(cx).document.blocks.iter().find(|b| b.id == trigger_id)
                                    .and_then(|b| match &b.kind { Kind::Code(lang) => Some(crate::code_highlight::label(lang)), _ => None })
                            }).unwrap_or_else(|| "Texto simples".into());
                            Button::new("code-language-trigger", label)
                                .icon_after("iconoir/regular/nav-arrow-down.svg")
                                .size(ButtonSize::Xs).variant(ButtonVariant::Ghost).into_any_element()
                        }));
                    let subscription = cx.subscribe_in(&menu, window, move |this, menu, event, w, cx| {
                        match event {
                            MenuEvent::Select(index) => {
                                if let Some(&(language, _)) = crate::code_highlight::LANGUAGES.get(*index) {
                                    this.set_code_language(&block_id, language, w, cx);
                                }
                            }
                            MenuEvent::OpenChange(true) => menu.read(cx).focus_handle(cx).focus(w),
                            MenuEvent::OpenChange(false) => {
                                if menu.read(cx).focus_handle(cx).is_focused(w) {
                                    if let Some(i) = this.document.blocks.iter().position(|b| b.id == block_id) {
                                        this.state(i).update(cx, |state, cx| state.focus(w, cx));
                                    }
                                }
                            }
                            _ => {}
                        }
                        cx.notify();
                    });
                    let input = self.inputs.get_mut(&block.id).unwrap();
                    input.language_menu = Some(menu);
                    input._language_subscription = Some(subscription);
                }
                self.inputs[&block.id].language_menu.as_ref().unwrap().update(cx, |menu, cx| {
                    menu.set_items(code_language_items(language), cx);
                });
            } else if let Some(menu) = &self.inputs[&block.id].language_menu {
                menu.update(cx, |menu, cx| menu.set_open(false, cx));
            }
            let input = self.inputs.get_mut(&block.id).unwrap();
            if let Kind::Code(language)=&block.kind {
                if input.highlight_language.as_ref()!=Some(language){
                    let cache=input.highlight.clone();let language=language.clone();
                    input.highlight_language=Some(language.clone());
                    state.update(cx,|s,cx|s.set_inline_highlight_provider(Some(Box::new(move|text,range,theme|cache.borrow_mut().styles_in_range(&language,&text.to_string(),range,theme))),cx));
                }
            }else if input.highlight_language.take().is_some(){
                state.update(cx,|s,cx|s.set_inline_highlight_provider(None,cx));
                *input.highlight.borrow_mut()=Default::default();
            }
            let styles = match &block.kind {
                Kind::Code(_) => Vec::new(),
                _ => inline_styles(block),
            };
            state.update(cx, |s, cx| {
                s.set_placeholder(placeholder, window, cx);
                s.set_inline_highlights(styles, cx);
                s.set_inline_font_ranges(inline_fonts(block), cx);
            });
        }
        self.sync_tables(window,cx);
    }
    fn sync_tables(&mut self, window:&mut Window, cx:&mut Context<Self>) {
        let tables:Vec<_>=self.document.blocks.iter().filter(|b|b.kind==Kind::Table).map(|b|(b.id.clone(),b.text.clone())).collect();
        let ids:HashSet<_>=tables.iter().map(|(id,_)|id.clone()).collect();self.tables.retain(|id,_|ids.contains(id));
        for (id,source) in tables {
            if self.tables.get(&id).is_some_and(|table|table.source==source){continue;}
            let Some(model)=crate::markdown_table::MarkdownTable::parse(&source) else{self.tables.remove(&id);continue;};
            let table=self.tables.entry(id.clone()).or_insert_with(||TableInputs{row_heights:Default::default(),source:source.clone(),model:model.clone(),cells:HashMap::new()});
            table.cells.retain(|&(r,c),_|r<model.rows.len()&&c<model.align.len());
            for (r,row) in model.rows.iter().enumerate(){for (c,block) in row.iter().enumerate(){
                if !table.cells.contains_key(&(r,c)){
                    let state=cx.new(|cx|{let mut s=InputState::new(window,cx).auto_grow(1,8).block_mode(true);s.set_value(block.text.clone(),window,cx);s});
                    let cell_id=id.clone();let subscription=cx.subscribe_in(&state,window,move|this,_,event,w,cx|this.table_event(&cell_id,r,c,event,w,cx));
                    table.cells.insert((r,c),CellInput{state,_subscription:subscription});
                }
                table.cells[&(r,c)].state.update(cx,|s,cx|{
                    if s.value().as_ref()!=block.text {let old=s.value().to_string();let range=s.selection_range();let start=sparkpad_sync::transform_cursor(&old,&block.text,range.start);let end=sparkpad_sync::transform_cursor(&old,&block.text,range.end);s.set_value(block.text.clone(),window,cx);s.set_byte_selection(start.min(end)..start.max(end),cx);}
                    s.set_inline_highlights(inline_styles(block),cx);s.set_inline_font_ranges(inline_fonts(block),cx);
                });
            }}
            table.row_heights.borrow_mut().clear();table.model=model;table.source=source;
        }
    }
    fn focused_table_cell(&self,window:&Window,cx:&App)->Option<(String,usize,usize)>{
        self.tables.iter().find_map(|(id,t)|t.cells.iter().find_map(|(&(r,c),cell)|cell.state.read(cx).focus_handle(cx).is_focused(window).then(||(id.clone(),r,c))))
    }
    fn focus_table_cell(&mut self,id:&str,r:usize,c:usize,window:&mut Window,cx:&mut Context<Self>){
        self.clear_text_selections(None,cx);self.mouse_anchor=None;self.selected_block=None;self.active=Some(id.into());self.menu=false;
        if let Some(cell)=self.tables.get(id).and_then(|t|t.cells.get(&(r,c))){cell.state.update(cx,|s,cx|s.focus(window,cx));}
        cx.emit(EditorEvent::Editing);cx.notify();
    }
    fn save_table(&mut self,id:&str,source:String,window:&mut Window,cx:&mut Context<Self>){
        let Some(i)=self.document.blocks.iter().position(|b|b.id==id&&b.kind==Kind::Table)else{return};
        if self.document.blocks[i].text==source{return;}
        self.checkpoint(cx);self.document.blocks[i].edit(source);self.changed(window,cx);
    }
    fn table_event(&mut self,id:&str,r:usize,c:usize,event:&InputEvent,window:&mut Window,cx:&mut Context<Self>){
        match event {
            InputEvent::Change=>{
                let Some(table)=self.tables.get(id)else{return};let Some(cell)=table.cells.get(&(r,c))else{return};
                let state=cell.state.clone();let value=state.read(cx).value().to_string();let mut block=table.model.rows[r][c].clone();if block.text==value{return;}block.edit(value);
                let Some(source)=self.document.blocks.iter().find(|b|b.id==id).map(|b|b.text.clone())else{return};
                let table=self.tables.get_mut(id).unwrap();let next=table.model.edit_cell(&source,r,c,&block);
                // Preserve the live cell's identity/focus, refreshing only its styles.
                if !block.text.trim().is_empty()&&block.text.trim()==block.text&&!block.text.contains(['\n','\r']) {
                    table.source=next.clone();table.row_heights.borrow_mut().remove(&r);
                    state.update(cx,|s,cx|{s.set_inline_highlights(inline_styles(&block),cx);s.set_inline_font_ranges(inline_fonts(&block),cx);});
                }
                self.save_table(id,next,window,cx);
            }
            InputEvent::Focus|InputEvent::BlockSelectionStart{..}=>{self.mouse_anchor=None;self.text_selection=None;self.selected_block=None;self.active=Some(id.into());self.menu=false;cx.emit(EditorEvent::Editing);cx.notify();}
            InputEvent::BlockEnter=>{
                let rows=self.tables.get(id).map(|t|t.model.rows.len()).unwrap_or(0);if r+1>=rows{self.table_structure(id,TableChange::AddRow,window,cx);}self.focus_table_cell(id,r+1,c,window,cx);
            }
            InputEvent::BlockUndo=>self.table_history(false,id,r,c,window,cx),
            InputEvent::BlockRedo=>self.table_history(true,id,r,c,window,cx),
            _=>{}
        }
    }
    fn table_history(&mut self,redo:bool,id:&str,r:usize,c:usize,window:&mut Window,cx:&mut Context<Self>){
        self.history(redo,window,cx);
        if let Some(table)=self.tables.get(id){let row=r.min(table.model.rows.len()-1);let col=c.min(table.model.align.len()-1);self.focus_table_cell(id,row,col,window,cx);}
    }
    fn table_structure(&mut self,id:&str,change:TableChange,window:&mut Window,cx:&mut Context<Self>){
        let Some(table)=self.tables.get(id)else{return};let mut model=table.model.clone();
        match change {TableChange::AddRow=>model.add_row(),TableChange::AddColumn=>model.add_column(),TableChange::RemoveRow(r)=>model.remove_row(r),TableChange::RemoveColumn(c)=>model.remove_column(c)}
        self.save_table(id,model.markdown(),window,cx);
    }
    fn table_tab(&mut self,back:bool,window:&mut Window,cx:&mut Context<Self>)->bool{
        let Some((id,r,c))=self.focused_table_cell(window,cx)else{return false};let cols=self.tables[&id].model.align.len();let rows=self.tables[&id].model.rows.len();let index=r*cols+c;
        if back&&index==0{return true;}
        let next=if back{index-1}else{index+1};if next>=rows*cols{self.table_structure(&id,TableChange::AddRow,window,cx);}
        self.focus_table_cell(&id,next/cols,next%cols,window,cx);true
    }
    fn render_table(&self,id:&str,window:&mut Window,top:f32,cx:&mut Context<Self>)->AnyElement{
        let Some(table)=self.tables.get(id)else{return div().child("Tabela inválida").into_any_element()};
        let mut grid=div().flex().flex_col().min_w(px(180.*table.model.align.len() as f32+32.)).border_1().border_color(neutral(0x303030)).rounded_md().overflow_hidden();
        let viewport=self.document_scroll.as_ref().map(|s|s.bounds()).filter(|b|b.size.height>px(0.)).unwrap_or(Bounds::new(point(px(0.),px(0.)),window.viewport_size()));
        let focused=self.focused_table_cell(window,cx).filter(|(table,_,_)|table==id).map(|(_,row,_)|row);
        let mut y=top+5.;let mut skipped=0.;
        for (r,row) in table.model.rows.iter().enumerate(){
            let height=table.row_heights.borrow().get(&r).copied().unwrap_or_else(||{
                let lines=row.iter().map(|cell|cell.text.lines().map(|line|(line.chars().count().max(1)+12)/13).sum::<usize>().max(1)).max().unwrap_or(1).min(8);
                (lines as f32*(self.font_size-2.)*1.6+16.+if r==0{24.}else{0.}).max(48.)
            });
            let visible=table.model.rows.len()<=80||focused==Some(r)||(y+height>=f32::from(viewport.top())-300.&&y<=f32::from(viewport.bottom())+300.);y+=height;
            if !visible{skipped+=height;continue;}
            if skipped>0.{grid=grid.child(div().w_full().h(px(skipped)).flex_none());skipped=0.;}
            let heights=table.row_heights.clone();let owner=cx.entity().downgrade();
            let mut line=div().relative().flex().child(canvas(move|bounds,_,cx|{let old=heights.borrow_mut().insert(r,f32::from(bounds.size.height));if old.is_none_or(|h|(h-f32::from(bounds.size.height)).abs()>0.5){let owner=owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|_,cx|cx.notify());});}},|_,_,_,_|{}).absolute().inset_0()).when(r==0,|d|d.bg(neutral(0x222222)));
            for (c,_) in row.iter().enumerate(){
                let remove_id=id.to_owned();
                let mut cell=div().w(px(180.)).flex_none().min_h(px(48.)).px_3().py_2().border_r_1().border_b_1().border_color(neutral(0x303030));
                if r==0 {cell=cell.child(div().flex().justify_end().child(Button::new(SharedString::from(format!("remove-col-{id}-{c}")),"−").size(ButtonSize::Xs).variant(ButtonVariant::Ghost).disabled(table.model.align.len()==1).on_click(cx.listener(move|this,_,w,cx|this.table_structure(&remove_id,TableChange::RemoveColumn(c),w,cx)))));}
                cell=cell.child(NativeInput::new(&table.cells[&(r,c)].state).appearance(false).bare_metrics().w_full().font_family(self.font_family.clone()).font_weight(if r==0{FontWeight::SEMIBOLD}else{FontWeight::NORMAL}).text_align(match table.model.align[c]{markdown::mdast::AlignKind::Right=>TextAlign::Right,markdown::mdast::AlignKind::Center=>TextAlign::Center,_=>TextAlign::Left}).text_size(px(self.font_size-2.)));
                line=line.child(cell);
            }
            let remove_id=id.to_owned();
            line=line.child(div().w(px(32.)).flex_none().flex().items_center().justify_center().when(r>0,|d|d.child(Button::new(SharedString::from(format!("remove-row-{id}-{r}")),"−").size(ButtonSize::Xs).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.table_structure(&remove_id,TableChange::RemoveRow(r),w,cx))))));
            grid=grid.child(line);
        }
        if skipped>0.{grid=grid.child(div().w_full().h(px(skipped)).flex_none());}
        let row_id=id.to_owned();let col_id=id.to_owned();
        div().flex_1().min_w_0().flex().flex_col().gap_2()
            .child(div().id(SharedString::from(format!("table-scroll-{id}"))).w_full().min_w_0().overflow_x_scroll().child(grid))
            .child(div().flex().gap_2()
                .child(Button::new(SharedString::from(format!("add-row-{id}")),"+ Linha").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.table_structure(&row_id,TableChange::AddRow,w,cx))))
                .child(Button::new(SharedString::from(format!("add-column-{id}")),"+ Coluna").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.table_structure(&col_id,TableChange::AddColumn,w,cx)))))
            .into_any_element()
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
            if let Some(input)=self.inputs.get(&block.id){input.state.update(cx, |s, cx| {
                s.cancel_mouse_selection();
                s.set_byte_selection(range, cx);
            });}
        }
        cx.notify();
    }
    fn focused_code_index(&self, window: &Window, cx: &App) -> Option<usize> {
        self.document.blocks.iter().enumerate().find_map(|(i, block)| {
            (matches!(block.kind, Kind::Code(_)) && self.inputs.get(&block.id).is_some_and(|input|input.state.read(cx).focus_handle(cx).is_focused(window))).then_some(i)
        })
    }
    fn select_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id,r,c))=self.focused_table_cell(window,cx){let state=self.tables[&id].cells[&(r,c)].state.clone();let len=state.read(cx).value().len();state.update(cx,|s,cx|s.set_byte_selection(0..len,cx));return;}
        self.flush(window, cx);
        if let Some(i) = self.focused_code_index(window, cx) {
            self.clear_text_selections(None, cx);
            self.selected_block = None;
            self.active = Some(self.document.blocks[i].id.clone());
            self.menu = false;
            let len = self.document.blocks[i].text.len();
            self.state(i).update(cx, |state, cx| state.set_byte_selection(0..len, cx));
            cx.emit(EditorEvent::Editing);
            cx.notify();
            return;
        }
        let last = self.document.blocks.len() - 1;
        self.clear_text_selections(None, cx);
        self.activate(0,0,window,cx);
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
    #[cfg(test)]
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
    fn clipboard_range(&self, cx: &App) -> Option<((usize, usize), (usize, usize))> {
        if let Some(selection) = self.text_selection { return Some(selection.ordered()); }
        if let Some(id) = &self.selected_block {
            let i = self.document.blocks.iter().position(|block| &block.id == id)?;
            return Some(((i, 0), (i, self.document.blocks[i].text.len())));
        }
        let i = self.active_index()?;
        let range = self.state(i).read(cx).selection_range();
        Some(((i, range.start), (i, range.end)))
    }
    fn copy_rich(&mut self, cut: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.focused_table_cell(window,cx).is_some(){return false;}
        let Some((a, z)) = self.clipboard_range(cx) else { return false; };
        if a == z && self.selected_block.is_none() { return false; }
        let fragment = self.document.fragment(a, z);
        let plain = fragment.iter().map(|block| block.text.as_str()).collect::<Vec<_>>().join("\n\n");
        if a.0 == z.0 && self.selected_block.is_none() && matches!(self.document.blocks[a.0].kind, Kind::Code(_)) {
            crate::macos::write_plain_clipboard(&plain, cx);
        } else {
            let html = crate::rich_clipboard::to_html(&fragment);
            let markdown = Document::from_blocks(fragment).markdown();
            crate::macos::write_rich_clipboard(plain, &html, Some(&markdown), cx);
        }
        if cut {
            self.checkpoint(cx);
            let (i, cursor) = self.document.replace_selection(a, z, "");
            self.clear_text_selections(None, cx); self.changed(window, cx); self.activate(i, cursor, window, cx);
        }
        true
    }
    fn paste_clipboard(&mut self, plain_only: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.focused_table_cell(window,cx).is_some(){return false;}
        let Some((a, z)) = self.clipboard_range(cx) else { return false; };
        if !plain_only && !matches!(self.document.blocks[a.0].kind,Kind::Code(_)|Kind::Source|Kind::Table){
            if let Some(image)=cx.read_from_clipboard().and_then(|item|item.into_entries().find_map(|e|if let ClipboardEntry::Image(image)=e{Some(image)}else{None})) {
                if let Some(root)=&self.asset_directory {
                    let ext=match image.format {ImageFormat::Png=>"png",ImageFormat::Jpeg=>"jpg",ImageFormat::Webp=>"webp",ImageFormat::Gif=>"gif",ImageFormat::Svg=>"svg",_=>""};
                    match crate::document_images::store(root,ext,image.bytes()) {
                        Ok(url)=>{self.checkpoint(cx);let block=Block::new(Kind::Image{url,title:None},String::new());self.document.replace_fragment(a,z,vec![block]);self.clear_text_selections(None,cx);self.changed(window,cx);return true;},
                        Err(error)=>{self.media_error=Some(error.to_string());cx.notify();return true;}
                    }
                }
            }
        }
        let plain = crate::macos::clipboard_string("public.utf8-plain-text")
            .or_else(|| cx.read_from_clipboard().and_then(|item| item.text()));
        if matches!(self.document.blocks[a.0].kind, Kind::Code(_) | Kind::Source | Kind::Table) || plain_only {
            let Some(text) = plain else { return false; };
            self.checkpoint(cx);
            let (i, cursor) = self.document.replace_selection(a, z, &text);
            self.clear_text_selections(None, cx); self.changed(window, cx); self.activate(i, cursor, window, cx); return true;
        }
        let mut fragment = if let Some(markdown) = crate::macos::clipboard_string("dev.mpires.sparkpad.markdown") {
            Document::parse(&markdown).blocks
        } else { crate::macos::clipboard_string("public.html").map(|html| crate::rich_clipboard::from_html(&html)).unwrap_or_default() };
        if fragment.is_empty() {
            let Some(text) = plain else { return false; };
            if !text.contains('\n') {
                self.checkpoint(cx);
                let (i, cursor) = self.document.replace_selection(a, z, &text);
                self.clear_text_selections(None, cx); self.changed(window, cx); self.activate(i, cursor, window, cx); return true;
            }
            fragment = text.replace("\r\n", "\n").split('\n').map(|line| Block::new(Kind::Paragraph, line.into())).collect();
        }
        self.checkpoint(cx);
        let (i, cursor) = self.document.replace_fragment(a, z, fragment);
        self.clear_text_selections(None, cx); self.changed(window, cx); self.activate(i, cursor, window, cx); true
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
        let id=self.document.blocks[i].id.clone();
        if self.tables.contains_key(&id){self.focus_table_cell(&id,0,0,window,cx);return;}
        self.state(i).update(cx, |s, cx| {
            s.set_byte_selection(cursor..cursor, cx);
            s.focus(window, cx);
        });
        cx.emit(EditorEvent::Editing);
        cx.notify();
    }
    fn flush(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<_> = self.inputs.keys().cloned().collect();
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
                let text = self.document.blocks[i].text.clone();
                self.menu = self.document.blocks[i].kind == Kind::Paragraph
                    && text.starts_with('/')
                    && !text.contains('\n');
                self.sync_slash_menu(cx);
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
                if matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Source | Kind::Table) {
                    state.update(cx, |s, cx| {
                        if matches!(self.document.blocks[i].kind, Kind::Code(_)) { s.replace("\n", window, cx); }
                        else { s.insert("\n", window, cx); }
                    });
                    return;
                }
                if self.menu {
                    self.slash_menu.update(cx, |menu, cx| menu.choose_highlighted(cx));
                    return;
                }
                self.checkpoint(cx);
                let next = self.document.split(i, state.read(cx).selection_range());
                self.changed(window, cx);
                self.activate(next, 0, window, cx);
            }
            InputEvent::BlockBackspace => {
                if matches!(self.document.blocks[i].kind, Kind::Code(_)) { return; }
                self.active = Some(id.into());
                if self.document.blocks[i].kind != Kind::Paragraph
                    && !matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Source | Kind::Table)
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
                if matches!(self.document.blocks[i].kind, Kind::Code(_)) { return; }
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
        if let Some(shared)=&mut self.shared {
            if shared.undo(redo) {
                let value=shared.document.text("body");
                self.reconcile_value(&value,window,cx);
                cx.emit(EditorEvent::Change);
            }
            return;
        }
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
    fn indent_code(&mut self, outdent: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(i) = self.document.blocks.iter().enumerate().find_map(|(i, b)| {
            (matches!(b.kind, Kind::Code(_)) && self.inputs.get(&b.id).is_some_and(|input|input.state.read(cx).focus_handle(cx).is_focused(window))).then_some(i)
        }) else { return false; };
        if self.text_selection.is_some_and(|s| s.anchor.0 != s.head.0) { return false; }
        let state = self.state(i);
        let text = state.read(cx).value().to_string();
        let (value, selection) = code_indent(&text, state.read(cx).selection_range(), outdent);
        if value != text {
            self.checkpoint(cx);
            self.document.blocks[i].edit(value);
            self.active = Some(self.document.blocks[i].id.clone());
            self.text_selection = None;
            self.changed(window, cx);
            state.update(cx, |s, cx| s.set_byte_selection(selection, cx));
        }
        true
    }
    fn copy_code(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(block) = self.document.blocks.iter().find(|b| b.id == id && matches!(b.kind, Kind::Code(_))) else { return; };
        let code = self.inputs[&block.id].state.read(cx).value().to_string();
        crate::macos::write_plain_clipboard(&code, cx);
        let stamp = std::time::Instant::now();
        self.copied_code = Some((id.to_owned(), stamp));
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            let _ = this.update(cx, |this, cx| {
                if this.copied_code.as_ref().is_some_and(|(_, time)| *time == stamp) {
                    this.copied_code = None;
                    cx.notify();
                }
            });
        }).detach();
    }
    fn set_code_language(&mut self, id: &str, language: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.document.blocks.iter().position(|b| b.id == id) else { return };
        if !matches!(self.document.blocks[i].kind, Kind::Code(_)) { return; }
        let cursor = self.state(i).read(cx).cursor();
        if self.document.blocks[i].kind != Kind::Code(language.to_owned()) {
            self.checkpoint(cx);
            self.document.blocks[i].kind = Kind::Code(language.to_owned());
            self.document.blocks[i].invalidate();
            self.changed(window, cx);
        }
        self.activate(i, cursor, window, cx);
    }
    fn convert(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.active_index() else { return };
        self.checkpoint(cx);
        let block = &mut self.document.blocks[i];
        if self.menu && self.selected_block.is_none() && block.text.starts_with('/') {
            block.edit(String::new());
        }
        block.kind = kind.clone();
        if kind==Kind::Table && block.text.is_empty(){block.edit("| Coluna 1 | Coluna 2 |\n| --- | --- |\n| Valor | Valor |".into());}
        block.invalidate();
        self.menu = false;
        self.slash_menu.update(cx, |menu, cx| menu.dismiss(cx));
        self.changed(window, cx);
        if matches!(kind,Kind::Image{..}) {self.choose_image(self.document.blocks[i].id.clone(),window,cx);}
        else if self.selected_block.is_none() {
            self.activate(i, self.document.blocks[i].text.len(), window, cx);
        }
    }
    fn format(&mut self, style: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id,r,c))=self.focused_table_cell(window,cx){
            let table=&self.tables[&id];let state=table.cells[&(r,c)].state.clone();let range=state.read(cx).selection_range();if range.is_empty(){return;}
            let mut cell=table.model.rows[r][c].clone();cell.format(range.clone(),style);
            let source=&self.document.blocks.iter().find(|b|b.id==id).unwrap().text;let next=table.model.replace_cell(source,r,c,&cell);self.save_table(&id,next,window,cx);state.update(cx,|s,cx|s.set_byte_selection(range,cx));return;
        }
        if let Some(selection) = self.text_selection {
            let (a, z) = selection.ordered();
            if !(a.0..=z.0).any(|i| !matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) && !selection.range(i, self.document.blocks[i].text.len()).is_empty()) { return; }
            self.checkpoint(cx);
            let all = (a.0..=z.0)
                .filter(|i| {
                    !matches!(self.document.blocks[*i].kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) && !selection
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
                if matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) { continue; }
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
        if matches!(self.document.blocks[i].kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) { return; }
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
        if let Some(shared)=&mut self.shared {shared.boundary();}
        if self.document.move_to(&drag.id, &target, after) {
            if self.shared.is_none() {self.undo.push(snapshot);}
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.changed(window, cx);
        }
        self.select_block(drag.id.clone(), window, cx);
    }
    fn sync_slash_menu(&mut self, cx: &mut Context<Self>) {
        let mut items = vec![MenuItem::group_label("Blocos básicos")];
        for (label, kind) in self.filtered_kinds() {
            let shortcut = match kind {
                Kind::Heading(1) => "#", Kind::Heading(2) => "##", Kind::Heading(3) => "###",
                Kind::Bullet => "-", Kind::Number(_) => "1.", Kind::Task(_) => "[]",
                Kind::Quote => ">", Kind::Code(_) => "```", Kind::Divider => "---", _ => "",
            };
            items.push(MenuItem::new(label).icon(kind_display(&kind).1).shortcut(shortcut));
        }
        if items.len() == 1 { items.push(MenuItem::group_label("Nenhum bloco encontrado")); }
        items.push(MenuItem::separator());
        items.push(MenuItem::new("Fechar menu").shortcut("esc"));
        let open = self.menu;
        self.slash_menu.update(cx, |menu, cx| {
            menu.set_items(items, cx);
            menu.set_open(open, cx);
            if open { menu.navigate(1, cx); }
        });
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
    fn toolbar(&self, text_selection: bool, cx: &mut Context<Self>) -> Stateful<Div> {
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
                        .disabled(self.shared.as_ref().map(|s|!s.can_undo()).unwrap_or(self.undo.is_empty()))
                        .on_click(cx.listener(|this, _, w, cx| this.history(false, w, cx))),
                ),
            )
            .child(
                Tooltip::new("redo-tip", "Refazer").child(
                    Button::icon("block-redo", "iconoir/regular/redo.svg")
                        .size(ButtonSize::IconSm)
                        .variant(ButtonVariant::Ghost)
                        .disabled(self.shared.as_ref().map(|s|!s.can_redo()).unwrap_or(self.redo.is_empty()))
                        .on_click(cx.listener(|this, _, w, cx| this.history(true, w, cx))),
                ),
            )
    }
    fn selection_toolbar_index(&self, cx: &App) -> Option<usize> {
        let selection = self.text_selection?;
        let (a, z) = selection.ordered();
        (a.0..=z.0).find(|i| !matches!(self.document.blocks[*i].kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) && self.inputs.get(&self.document.blocks[*i].id).is_some_and(|input|!input.state.read(cx).selection_range().is_empty()))
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
        ("Imagem",Kind::Image{url:String::new(),title:None},"iconoir/regular/media-image.svg"),
        ("Tabela",Kind::Table,"iconoir/regular/table.svg"),
        ("Título 4",Kind::Heading(4),"iconoir/regular/text-size.svg"),
        ("Título 5",Kind::Heading(5),"iconoir/regular/text-size.svg"),
        ("Título 6",Kind::Heading(6),"iconoir/regular/text-size.svg"),
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
/// Apply code indentation as one edit while preserving byte-based selections.
fn code_indent(text: &str, selection: std::ops::Range<usize>, outdent: bool) -> (String, std::ops::Range<usize>) {
    if selection.is_empty() && !outdent {
        let mut value = text.to_owned();
        value.insert_str(selection.start, "  ");
        return (value, selection.start + 2..selection.start + 2);
    }
    let start = text[..selection.start].rfind('\n').map_or(0, |p| p + 1);
    // A selection ending at the start of a line does not include that line.
    let last = if !selection.is_empty() { selection.end - 1 } else { selection.end };
    let mut edits = Vec::new();
    let mut offset = start;
    loop {
        if outdent {
            let line = &text[offset..];
            let n = if line.starts_with('\t') { 1 } else { line.bytes().take(2).take_while(|&b| b == b' ').count() };
            if n > 0 { edits.push((offset, offset + n, "")); }
        } else { edits.push((offset, offset, "  ")); }
        let Some(next) = text[offset..].find('\n').map(|n| offset + n + 1) else { break; };
        if next > last { break; }
        offset = next;
    }
    let map = |p: usize| {
        let mut delta = 0isize;
        for &(a, b, replacement) in &edits {
            if p < a { break; }
            if p < b { return (a as isize + delta + replacement.len() as isize) as usize; }
            delta += replacement.len() as isize - (b - a) as isize;
        }
        (p as isize + delta) as usize
    };
    let range = map(selection.start)..map(selection.end);
    let mut value = text.to_owned();
    for &(a, b, replacement) in edits.iter().rev() { value.replace_range(a..b, replacement); }
    (value, range)
}

fn code_language_items(current: &str) -> Vec<MenuItem> {
    crate::code_highlight::LANGUAGES.iter().map(|(id, label)| {
        let item = MenuItem::new(*label);
        if *id == crate::code_highlight::canonical(current) {
            item.icon("iconoir/regular/check.svg")
        } else { item }
    }).collect()
}
fn inline_fonts(block: &Block) -> Vec<(std::ops::Range<usize>, SharedString)> {
    if matches!(block.kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) { return vec![]; }
    let mut ranges: Vec<(std::ops::Range<usize>, SharedString)> = Vec::new();
    for span in block.spans.iter().filter(|span| span.marks.code) {
        if let Some((range, _)) = ranges.last_mut().filter(|(range, _)| range.end == span.range.start) {
            range.end = span.range.end;
        } else { ranges.push((span.range.clone(), "JetBrains Mono".into())); }
    }
    ranges
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
                    color: if m.code { Some(rgb(if empire_ui::theme_mode() == empire_ui::ThemeMode::Light {0x35764a} else {0x8fd7a3}).into()) }
                        else { m.link.as_ref().map(|_| neutral(0xd4d4d4).into()) },
                    background_color: m.code.then_some(rgb(if empire_ui::theme_mode() == empire_ui::ThemeMode::Light {0xedf5ee} else {0x203128}).into()),
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
        let planned=self.virtual_rows(window);
        if planned.iter().enumerate().any(|(i,(visible,_,_))|*visible&&!self.inputs.contains_key(&self.document.blocks[i].id)){self.sync_inputs(window,cx);}
        let dragging = cx.has_active_drag();
        self.row_bounds.borrow_mut().retain(|id, _| id != "__toolbar" && !id.starts_with("__code-tools-") && !id.starts_with("__code-copy-"));
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
            .capture_action(cx.listener(|this, _: &gpui_component::input::IndentInline, w, cx| {
                if this.table_tab(false,w,cx) || this.indent_code(false, w, cx) { cx.stop_propagation(); } else { cx.propagate(); }
            }))
            .capture_action(cx.listener(|this, _: &gpui_component::input::OutdentInline, w, cx| {
                if this.table_tab(true,w,cx) || this.indent_code(true, w, cx) { cx.stop_propagation(); } else { cx.propagate(); }
            }))
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
                    if this.menu {
                        this.slash_menu.update(cx, |menu, cx| menu.navigate(-1, cx));
                        cx.stop_propagation();
                    } else { this.collapse_document_selection(false, w, cx); }
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveRight, w, cx| {
                    this.collapse_document_selection(true, w, cx)
                }),
            )
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::MoveDown, w, cx| {
                    if this.menu {
                        this.slash_menu.update(cx, |menu, cx| menu.navigate(1, cx));
                        cx.stop_propagation();
                    } else { this.collapse_document_selection(true, w, cx); }
                }),
            )
            .capture_action(cx.listener(|this, _: &gpui_component::input::Escape, _, cx| {
                if this.menu {
                    this.menu = false;
                    this.slash_menu.update(cx, |menu, cx| menu.dismiss(cx));
                    cx.stop_propagation(); cx.notify();
                } else { cx.propagate(); }
            }))
            .on_action(cx.listener(|this, _: &SelectDocument, w, cx| this.select_document(w, cx)))
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::SelectAll, w, cx| {
                    this.select_document(w, cx)
                }),
            )
            .capture_action(cx.listener(|this, _: &gpui_component::input::Copy, w, cx| {
                if this.copy_rich(false, w, cx) {
                    cx.stop_propagation();
                } else {
                    cx.propagate();
                }
            }))
            .capture_action(cx.listener(|this, _: &gpui_component::input::Cut, w, cx| {
                if this.copy_rich(true, w, cx) {
                    cx.stop_propagation();
                } else {
                    cx.propagate();
                }
            }))
            .capture_action(
                cx.listener(|this, _: &gpui_component::input::Paste, w, cx| {
                    if this.paste_clipboard(false, w, cx) {
                        cx.stop_propagation();
                    } else {
                        cx.propagate();
                    }
                }),
            )
            .on_action(cx.listener(|this, _: &PastePlain, w, cx| {
                if this.paste_clipboard(true, w, cx) { cx.stop_propagation(); } else { cx.propagate(); }
            }))
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
                                if !this.resize_image(event,window,cx) {this.extend_document_selection(event, window, cx)}
                            });
                        }
                    });
                    let selection_owner = weak.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            let _ = selection_owner.update(cx, |this, cx| {
                                if this.resizing_image.take().is_some(){cx.stop_propagation();cx.notify();}
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
        let origin=self.layout_origin.clone();let layout_width=self.layout_width.clone();let owner=cx.entity().downgrade();let scroll=self.document_scroll.clone();
        root=root.child(canvas(move|bounds,_,cx|{
            let offset=scroll.as_ref().map(|s|s.offset()).unwrap_or_default();let next=bounds.origin-offset;
            let resized=(layout_width.get()-f32::from(bounds.size.width)).abs()>0.5;
            let moved=origin.get().is_none_or(|old|old.y!=next.y);
            origin.set(Some(next));layout_width.set(f32::from(bounds.size.width));
            if resized||moved {let owner=owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|this,cx|{if resized{this.row_heights.borrow_mut().clear();this.estimated_heights.clear();}cx.notify();});});}
        },|_,_,_,_|{}).absolute().inset_0());
        if let Some(error)=&self.media_error {root=root.child(div().text_sm().text_color(rgb(0xf08b8b)).child(error.clone()));}
        let mut skipped=0.;let mut rendered=HashSet::new();
        for i in 0..self.document.blocks.len() {
            if !planned[i].0{skipped+=planned[i].1+8.;continue;}
            if skipped>0.{root=root.child(div().w_full().h(px((skipped-8.).max(0.))).flex_none());skipped=0.;}
            rendered.insert(self.document.blocks[i].id.clone());
            let key=self.document.blocks[i].id.clone();
            let hover=self.hover_motion.entry(key.clone()).or_insert_with(||empire_ui::motion::Tween::new(0.));
            hover.set(if !dragging && self.hovered_block.as_ref()==Some(&key) {1.} else {0.},120);
            let hover_alpha=hover.value();
            let hover_id=key;
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
            let text_selection = !matches!(kind, Kind::Code(_) | Kind::Table | Kind::Image { .. }) && self
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
                        if event.click_count == 1 && !this.tables.contains_key(&interaction_id) {
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
                .on_hover(cx.listener(move |this,inside:&bool,_,cx| {
                    if *inside {this.hovered_block=Some(hover_id.clone());}
                    else if this.hovered_block.as_ref()==Some(&hover_id) {this.hovered_block=None;}
                    cx.notify();
                }))
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
            let is_image=matches!(kind,Kind::Image{..});let image_owner=cx.entity().downgrade();let heights=self.row_heights.clone();let height_owner=cx.entity().downgrade();let height_id=id.clone();
            row = row.child(
                canvas(
                    move |bounds, _, cx| {
                        let before=geometry.borrow_mut().insert(measured_id.clone(), bounds);
                        let prior=heights.borrow_mut().insert(height_id.clone(),f32::from(bounds.size.height));
                        if prior.is_none_or(|old|(old-f32::from(bounds.size.height)).abs()>0.5){let owner=height_owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|_,cx|cx.notify());});}
                        if is_image && before.is_none_or(|old|old.size.width!=bounds.size.width){let owner=image_owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|_,cx|cx.notify());});}
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
                .opacity(hover_alpha)
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
            if let Kind::Task(_) = kind {
                row = row.child(
                    div()
                        .id(SharedString::from(format!("check-{id}")))
                        .flex_none()
                        .h(px(size * 1.6))
                        .flex()
                        .items_center()
                        .child(self.inputs[&id].checkbox.as_ref().unwrap().clone())
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.mouse_anchor = None;
                            cx.stop_propagation();
                        }))
                        .on_click(cx.listener(|_, _, _, cx| {
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
            } else if let Kind::Image{url,title}=&kind {
                let image_id=id.clone();let resize_id=id.clone();let alt_id=id.clone();let smaller_id=id.clone();let larger_id=id.clone();
                let available=self.row_bounds.borrow().get(&id).map(|b|f32::from(b.size.width)-74.).unwrap_or(480.).max(100.);
                let width=crate::blocks::image_width(title.as_deref());
                let source=self.asset_directory.as_ref().and_then(|root|crate::document_images::local(root,url)).map(ImageSource::from)
                    .or_else(||(url.starts_with("https://")||url.starts_with("http://")).then(||ImageSource::from(SharedString::from(url.clone()))));
                let mut content=div().flex_1().min_w_0().flex().flex_col().gap_2();
                if let Some(source)=source {
                    content=content.child(div().relative().w(px(available*f32::from(width)/100.)).max_w_full()
                        .child(img(source).w(px(available*f32::from(width)/100.)).max_w_full().h_auto().object_fit(ObjectFit::Contain).rounded_md())
                        .child(div().id(SharedString::from(format!("image-resize-{id}"))).absolute().right_0().top(relative(0.5)).w(px(7.)).h(px(36.)).rounded_sm().bg(neutral(0x999999)).cursor(CursorStyle::ResizeLeftRight)
                            .on_mouse_down(MouseButton::Left,cx.listener(move|this,_,w,cx|{this.checkpoint(cx);this.resizing_image=Some(resize_id.clone());this.select_block(resize_id.clone(),w,cx);w.prevent_default();cx.stop_propagation();}))))
                        .child(div().flex().items_center().gap_2()
                            .child(Button::new(SharedString::from(format!("replace-image-{id}")),"Trocar imagem").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.choose_image(image_id.clone(),w,cx))))
                            .child(Button::new(SharedString::from(format!("caption-image-{id}")),"Legenda").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|{if let Some(i)=this.document.blocks.iter().position(|b|b.id==alt_id){this.activate(i,this.document.blocks[i].text.len(),w,cx);}})))
                            .child(Button::new(SharedString::from(format!("image-smaller-{id}")),"−").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.set_image_width(&smaller_id,width.saturating_sub(10),w,cx))))
                            .child(div().text_xs().text_color(neutral(0x999999)).child(format!("{width}%")))
                            .child(Button::new(SharedString::from(format!("image-larger-{id}")),"+").size(ButtonSize::Sm).variant(ButtonVariant::Ghost).on_click(cx.listener(move|this,_,w,cx|this.set_image_width(&larger_id,width.saturating_add(10),w,cx)))));
                } else {content=content.child(Button::new(SharedString::from(format!("upload-image-{id}")),"Escolher imagem").variant(ButtonVariant::Secondary).on_click(cx.listener(move|this,_,w,cx|this.choose_image(image_id.clone(),w,cx))));}
                if active && self.selected_block.is_none(){content=content.child(NativeInput::new(&self.state(i)).appearance(false).w_full());}
                else if !block.text.is_empty(){content=content.child(div().text_sm().text_color(neutral(0x999999)).child(block.text.clone()));}
                row=row.child(content);
            } else if kind==Kind::Table {
                row=row.child(self.render_table(&id,window,planned[i].2,cx));
            } else if kind==Kind::Source && !active && self.text_selection.is_none() {
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
                let mut cell = div().id(SharedString::from(format!("block-cell-{id}"))).flex_1().min_w_0();
                if matches!(kind, Kind::Code(_)) {
                    input = input.font_family("JetBrains Mono");
                    let hover_code_id = id.clone();
                    let geometry = self.row_bounds.clone();
                    let measured = format!("__code-{id}");
                    cell = cell.relative().bg(neutral(0x202020)).rounded_md().p(px(30.))
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside { this.hovered_code = Some(hover_code_id.clone()); }
                            else if this.hovered_code.as_ref() == Some(&hover_code_id) { this.hovered_code = None; }
                            cx.notify();
                        }))
                        .child(canvas(move |bounds, _, _| { geometry.borrow_mut().insert(measured.clone(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                        .child(input);
                    if let Some(menu) = &self.inputs[&id].language_menu {
                        if self.hovered_code.as_ref() == Some(&id) || menu.read(cx).is_open() {
                            let copied = self.copied_code.as_ref().is_some_and(|(copied, _)| copied == &id);
                            let copy_id = id.clone();
                            let geometry = self.row_bounds.clone();
                            let measured = format!("__code-tools-{id}");
                            let copy_geometry = self.row_bounds.clone();
                            let copy_measured = format!("__code-copy-{id}");
                            let controls = div().id(SharedString::from(format!("code-tools-{id}")))
                                .absolute().top(px(8.)).right(px(8.))
                                .flex().items_center().gap_1().p_1().rounded_md()
                                .bg(neutral(0x303030)).border_1().border_color(neutral(0x414141))
                                .text_size(px(12.)).font_family(crate::assets::FONT_FAMILY).text_color(neutral(0xd4d4d4))
                                .child(canvas(move |bounds, _, _| { geometry.borrow_mut().insert(measured.clone(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                                .child(menu.clone())
                                .child(div().relative()
                                    .child(canvas(move |bounds, _, _| { copy_geometry.borrow_mut().insert(copy_measured.clone(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                                    .child(Tooltip::new(SharedString::from(format!("copy-code-tip-{id}")), if copied { "Código copiado" } else { "Copiar código" })
                                        .child(Button::icon(SharedString::from(format!("copy-code-{id}")), if copied { "iconoir/regular/check.svg" } else { "iconoir/regular/copy.svg" })
                                            .size(ButtonSize::IconXs).variant(ButtonVariant::Ghost)
                                            .on_click(cx.listener(move |this, _, w, cx| {
                                                this.copy_code(&copy_id, cx);
                                                if let Some(i) = this.document.blocks.iter().position(|b| b.id == copy_id) {
                                                    this.state(i).update(cx, |state, cx| state.focus(w, cx));
                                                }
                                                cx.stop_propagation();
                                            })) )));
                            cell = cell.child(controls.with_animation(
                                SharedString::from(format!("code-tools-enter-{id}")),
                                Animation::new(std::time::Duration::from_millis(120)).with_easing(|t| 1. - (1. - t).powi(3)),
                                |element, progress| element.opacity(progress),
                            ));
                        }
                    }
                    row = row.child(cell);
                } else {
                    row = row.child(cell.child(input));
                }
            }
            if !dragging && (selected || text_selection) {
                // The toolbar is already absolute and paints after this block/previous blocks.
                // Menu and Tooltip manage their own deferred popups; nesting deferred
                // elements would panic in GPUI when either popup opens.
                row = row.child(self.toolbar(text_selection, cx).with_animation(
                    SharedString::from(format!("toolbar-enter-{id}")),
                    Animation::new(std::time::Duration::from_millis(120)).with_easing(|t|1.-(1.-t).powi(3)),
                    |element,progress|element.opacity(progress),
                ));
            }
            if !dragging && active && self.menu {
                row = row.child(div().id("slash-menu-anchor").absolute().top_full().left(px(36.))
                    .font_family(crate::assets::FONT_FAMILY).child(self.slash_menu.clone()));
            }
            root = root.child(row);
        }
        if skipped>0.{root=root.child(div().w_full().h(px((skipped-8.).max(0.))).flex_none());}
        self.row_bounds.borrow_mut().retain(|id,_|rendered.contains(id)||id.starts_with("__"));
        self.hover_motion.retain(|id,t|rendered.contains(id)||t.moving());
        if self.hover_motion.values().any(|t|t.moving()) {window.request_animation_frame();}
        root.child(
            div()
                .id("document-end")
                .w_full()
                .h(px(self.end_space.max(self.font_size * 1.6 + 24.)))
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
    let original_clipboard = crate::macos::snapshot_clipboard();
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
        for copy_key in ["cmd-c", "ctrl-c"] {
            cx.write_to_clipboard(ClipboardItem::new_string("clipboard sentinel".into()));
            event(
                handle,
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: Keystroke::parse(copy_key).unwrap(),
                    is_held: false,
                }),
                cx,
            );
            assert_eq!(
                cx.read_from_clipboard().and_then(|item| item.text()).as_deref(),
                Some(plain.as_str()),
                "copy must preserve every selected block instead of being overwritten by the focused input"
            );
        }
    }
    crate::macos::restore_clipboard(original_clipboard);
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
    // Inline code toggles on all selected blocks and survives Markdown reload.
    for enabled in [true, false] {
        frame(handle, cx);
        event(handle, PlatformInput::KeyDown(KeyDownEvent {
            keystroke: Keystroke::parse("cmd-e").unwrap(), is_held: false,
        }), cx);
        let current = editor.read(cx);
        let restored = Document::parse(&current.document.markdown());
        for (index, block) in current.document.blocks.iter().enumerate() {
            assert_eq!(block.is_formatted(0..block.text.len(), "code"), enabled);
            assert_eq!(restored.blocks[index].is_formatted(0..restored.blocks[index].text.len(), "code"), enabled);
            assert_eq!(!inline_fonts(block).is_empty(), enabled);
            assert_eq!(inline_styles(block).iter().any(|(_, style)| style.background_color.is_some()), enabled);
        }
    }
    for _ in 0..2 {
        cx.update_window(handle, |_, w, cx| {
            editor.update(cx, |this, cx| this.history(false, w, cx));
        }).unwrap();
    }
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
    let _isolated_clipboard = crate::macos::TestClipboard::new();
    let rich_paste = "# Career\n\nHello **bold** *itálico* <u>ação</u> `code` [link](https://example.com)\n\n- one\n- two\n\n```rust\nlet x = 1;\n```";
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| {
            this.document_scroll = None;
            this.set_value(rich_paste.into(), w, cx);
            this.activate(1, 0, w, cx);
        });
    }).unwrap();
    frame(handle, cx);
    for key in ["cmd-a", "cmd-c"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    let html = crate::macos::clipboard_string("public.html").expect("standard HTML representation");
    assert!(html.contains("<h1>Career</h1>") && html.contains("<strong>bold</strong>") && html.contains("<code>code</code>") && html.contains("<ul>") && html.contains("<pre>"));
    let exported = crate::rich_clipboard::from_html(&html);
    for (original, exported) in editor.read(cx).document.blocks.iter().zip(exported.iter()) {
        let original = original.slice(0..original.text.len());
        let exported = exported.slice(0..exported.text.len());
        assert_eq!((&original.kind, &original.text, &original.spans), (&exported.kind, &exported.text, &exported.spans));
    }
    // An external editor offers HTML + plain text, with no Sparkpad metadata.
    crate::macos::write_rich_clipboard("External\nRich ação\nitem".into(), "<h2>External</h2><p>Rich <strong><em>ação</em></strong> <u>underlined</u> <code>x &lt; 3</code></p><ul><li>item</li></ul>", None, cx);
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("ctrl-v").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks.len(), 3);
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Heading(2));
    let pasted = &editor.read(cx).document.blocks[1];
    let action = pasted.text.find("ação").unwrap();
    assert!(pasted.is_formatted(action..action + "ação".len(), "bold"));
    assert!(pasted.is_formatted(action..action + "ação".len(), "italic"));
    cx.update_window(handle, |_, w, cx| { editor.update(cx, |this, cx| this.history(false, w, cx)); }).unwrap();
    assert_eq!(editor.read(cx).document.markdown(), rich_paste, "rich paste must be a single undoable edit");
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| {
            let start = this.document.blocks[1].text.find("bold").unwrap();
            this.activate(1, start, w, cx);
            this.state(1).update(cx, |s, cx| s.set_byte_selection(start..start + 4, cx));
        });
    }).unwrap();
    frame(handle, cx);
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("cmd-c").unwrap(), is_held: false }), cx);
    let partial_html = crate::macos::clipboard_string("public.html").unwrap();
    assert!(partial_html.contains("<strong>bold</strong>") && !partial_html.contains("Career"));
    crate::macos::write_rich_clipboard("novo".into(), "<p><em>novo</em></p>", None, cx);
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("cmd-v").unwrap(), is_held: false }), cx);
    let pasted = &editor.read(cx).document.blocks[1];
    let start = pasted.text.find("novo").unwrap();
    assert!(pasted.is_formatted(start..start + 4, "italic"));
    assert!(!pasted.is_formatted(start..start + 4, "bold"));
    assert_eq!(editor.read(cx).document.blocks.len(), 5, "inline rich paste must preserve surrounding blocks");
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.set_value("```js\nconst value = 42;\n```".into(), w, cx));
    }).unwrap();
    frame(handle, cx);
    let code_id = editor.read(cx).document.blocks[0].id.clone();
    let code_bounds = editor.read(cx).row_bounds.borrow()[&format!("__code-{code_id}")];
    let resting = editor.read(cx).state(0).read(cx).byte_offset_for_point(point(code_bounds.left() + px(30.), code_bounds.top() + px(30.)));
    assert!(!editor.read(cx).row_bounds.borrow().contains_key(&format!("__code-tools-{code_id}")), "code controls must be hidden without hover");
    movement(handle, point(code_bounds.left() + px(4.), code_bounds.top() + px(4.)), false, cx);
    let hovered = editor.read(cx).row_bounds.borrow()[&format!("__code-{code_id}")];
    assert_eq!(code_bounds, hovered, "hover controls must not change code geometry");
    let controls = editor.read(cx).row_bounds.borrow()[&format!("__code-tools-{code_id}")];
    assert!((controls.right() - code_bounds.right() + px(8.)).abs() <= px(1.5));
    assert!((controls.top() - code_bounds.top() - px(8.)).abs() <= px(1.5));
    assert_eq!(resting, editor.read(cx).state(0).read(cx).byte_offset_for_point(point(code_bounds.left() + px(30.), code_bounds.top() + px(30.))));
    let copy_bounds = editor.read(cx).row_bounds.borrow()[&format!("__code-copy-{code_id}")];
    down(handle, copy_bounds.center(), cx);
    up(handle, copy_bounds.center(), cx);
    assert_eq!(crate::macos::clipboard_string("public.utf8-plain-text").as_deref(), Some("const value = 42;"));
    cx.update_window(handle, |_, w, cx| assert!(editor.read(cx).state(0).read(cx).focus_handle(cx).is_focused(w), "copy control keeps code editing focus")).unwrap();
    movement(handle, point(px(0.), px(0.)), false, cx);
    assert!(!editor.read(cx).row_bounds.borrow().contains_key(&format!("__code-tools-{code_id}")));
    movement(handle, point(code_bounds.left() + px(4.), code_bounds.top() + px(4.)), false, cx);
    let language_menu = editor.read(cx).inputs[&code_id].language_menu.clone().unwrap();
    cx.update_window(handle, |_, w, cx| language_menu.read(cx).focus_handle(cx).focus(w)).unwrap();
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("enter").unwrap(), is_held: false }), cx);
    movement(handle, point(px(0.), px(0.)), false, cx);
    assert!(language_menu.read(cx).is_open(), "dropdown must remain usable outside code hover");
    assert!(editor.read(cx).row_bounds.borrow().contains_key(&format!("__code-tools-{code_id}")));
    for key in std::iter::repeat("down").take(crate::code_highlight::LANGUAGES.iter().position(|(id, _)| *id == "rust").unwrap()).chain(std::iter::once("enter")) {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Code("rust".into()), "language menu must update the targeted code block");
    assert!(editor.read(cx).value().starts_with("```rust\n"));
    assert!(matches!(&Document::parse(editor.read(cx).value().as_ref()).blocks[0].kind, Kind::Code(lang) if lang == "rust"));
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.history(false, w, cx));
    }).unwrap();
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Code("js".into()), "one undo restores the previous language");
    cx.update_window(handle, |_, w, cx| {
        editor.update(cx, |this, cx| this.activate(0, 0, w, cx));
    }).unwrap();
    frame(handle, cx);
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("tab").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[0].text, "  const value = 42;");
    cx.update_window(handle, |_, w, cx| assert!(editor.read(cx).state(0).read(cx).focus_handle(cx).is_focused(w), "Tab must keep focus in the code input")).unwrap();
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("shift-tab").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[0].text, "const value = 42;");
    movement(handle, point(code_bounds.left() + px(4.), code_bounds.top() + px(4.)), false, cx);
    cx.update_window(handle, |_, w, cx| language_menu.read(cx).focus_handle(cx).focus(w)).unwrap();
    for key in std::iter::once("enter").chain(std::iter::repeat("down").take(crate::code_highlight::LANGUAGES.iter().position(|(id, _)| *id == "tsx").unwrap())).chain(std::iter::once("enter")) {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Code("tsx".into()));
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
        this.set_value("```tsx\nconst ação = () => (\n  <div>Hello</div>\n);\n```".into(), w, cx);
        this.activate(0, 0, w, cx);
        this.state(0).update(cx, |state, cx| state.set_byte_selection(0..this.document.blocks[0].text.find("\n);").unwrap(), cx));
    })).unwrap();
    frame(handle, cx);
    let before = editor.read(cx).document.blocks[0].text.clone();
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("tab").unwrap(), is_held: false }), cx);
    assert!(editor.read(cx).document.blocks[0].text.starts_with("  const ação = () => (\n    <div>"));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("shift-tab").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[0].text, before);
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.history(false, w, cx))).unwrap();
    assert!(editor.read(cx).document.blocks[0].text.starts_with("  const"));
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.history(false, w, cx))).unwrap();
    assert_eq!(editor.read(cx).document.blocks[0].text, before);
    editor.update(cx, |this, cx| {
        let id = this.document.blocks[0].id.clone();
        this.copy_code(&id, cx);
    });
    assert_eq!(crate::macos::clipboard_string("public.utf8-plain-text").as_deref(), Some(before.as_str()));
    assert!(crate::macos::clipboard_string("public.html").is_none(), "code copy must not retain stale HTML");
    assert!(crate::macos::clipboard_string("dev.mpires.sparkpad.markdown").is_none(), "code copy must not include fences");
    let code = "const Card = () => <div>ação</div>;\n// second line";
    let fixture = format!("before\n\n```tsx\n{code}\n```\n\nafter");
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
        this.set_value(fixture.clone(), w, cx);
        this.activate(1, 0, w, cx);
    })).unwrap();
    frame(handle, cx);
    for select_all in ["cmd-a", "ctrl-a"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(select_all).unwrap(), is_held: false }), cx);
        assert!(editor.read(cx).text_selection.is_none(), "select all in code must stay local");
        assert_eq!(editor.read(cx).state(1).read(cx).selection_range(), 0..code.len());
        assert!(editor.read(cx).state(0).read(cx).selection_range().is_empty());
        assert!(editor.read(cx).state(2).read(cx).selection_range().is_empty());
        assert!(!editor.read(cx).row_bounds.borrow().contains_key("__toolbar"), "code text selection must not open formatting toolbar");
    }
    let history = editor.read(cx).undo.len();
    for key in ["cmd-b", "cmd-i", "cmd-u", "cmd-e"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert_eq!(editor.read(cx).document.markdown(), fixture);
    assert_eq!(editor.read(cx).undo.len(), history, "formatting code must not create edits");
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("cmd-c").unwrap(), is_held: false }), cx);
    assert_eq!(crate::macos::clipboard_string("public.utf8-plain-text").as_deref(), Some(code));
    assert!(crate::macos::clipboard_string("public.html").is_none());
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
        this.apply_text_selection(TextSelection { anchor: (1, 2), head: (1, 8) }, cx);
        this.format("strike", w, cx);
    })).unwrap();
    frame(handle, cx);
    assert!(editor.read(cx).selection_toolbar_index(cx).is_none());
    assert!(!editor.read(cx).row_bounds.borrow().contains_key("__toolbar"));
    assert_eq!(editor.read(cx).document.markdown(), fixture);
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.activate(0, 0, w, cx))).unwrap();
    frame(handle, cx);
    for key in ["cmd-a", "cmd-b"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert!(editor.read(cx).document.blocks[0].is_formatted(0..6, "bold"));
    assert!(editor.read(cx).document.blocks[2].is_formatted(0..5, "bold"));
    assert!(!editor.read(cx).document.blocks[1].is_formatted(0..code.len(), "bold"), "document formatting must skip code");
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.activate(1, 0, w, cx))).unwrap();
    frame(handle, cx);
    for key in ["cmd-a", "cmd-x"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert_eq!(editor.read(cx).document.blocks.len(), 3);
    assert_eq!(editor.read(cx).document.blocks[1].text, "");
    assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Code("tsx".into()));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("backspace").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks.len(), 3, "backspace in empty code must not merge notes");
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("cmd-v").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[1].text, code);
    assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Code("tsx".into()));
    for key in ["cmd-a", "enter"] {
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse(key).unwrap(), is_held: false }), cx);
    }
    assert_eq!(editor.read(cx).document.blocks[1].text, "\n", "Enter replaces the code selection in place");
    assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Code("tsx".into()));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("cmd-z").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[1].text, code);
    println!("Native code editing isolation verified: Cmd/Ctrl+A, no formatting toolbar or marks, literal copy/cut/paste, mixed document formatting and empty code preservation.");
    println!("Native code hover verified: absolute inset controls, unchanged geometry, hover enter/leave, real copy button and interactive dropdown outside hover.");
    println!("Native code controls verified: Tab/Shift+Tab, multiline indentation, focus, undo, TSX dropdown and literal code clipboard.");
    println!("Native code language selector verified: keyboard dropdown, Markdown persistence and undo.");
    // Slash commands keep the native input focused while the menu owns arrow navigation.
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
        this.set_value(String::new(), w, cx);
        this.activate(0, 0, w, cx);
        this.state(0).update(cx, |state, cx| state.replace("/", w, cx));
    })).unwrap();
    frame(handle, cx);
    let slash = editor.read(cx).slash_menu.clone();
    assert!(editor.read(cx).menu && slash.read(cx).is_open(), "slash opens the command dropdown");
    assert_eq!(slash.read(cx).highlighted(), Some(1));
    cx.update_window(handle, |_, w, cx| assert!(editor.read(cx).state(0).read(cx).focus_handle(cx).is_focused(w))).unwrap();
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("down").unwrap(), is_held: false }), cx);
    assert_eq!(slash.read(cx).highlighted(), Some(2));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("up").unwrap(), is_held: false }), cx);
    assert_eq!(slash.read(cx).highlighted(), Some(1));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("down").unwrap(), is_held: false }), cx);
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("enter").unwrap(), is_held: false }), cx);
    assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Heading(1));
    assert_eq!(editor.read(cx).document.blocks[0].text, "");
    assert!(!editor.read(cx).menu && !slash.read(cx).is_open());
    cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
        this.set_value(String::new(), w, cx); this.activate(0, 0, w, cx);
        this.state(0).update(cx, |state, cx| state.replace("/", w, cx));
    })).unwrap();
    frame(handle, cx);
    cx.update_window(handle, |_, w, cx| editor.read(cx).state(0).update(cx, |state, cx| state.replace("código", w, cx))).unwrap();
    frame(handle, cx);
    assert_eq!(editor.read(cx).filtered_kinds().len(), 1);
    assert_eq!(slash.read(cx).highlighted(), Some(1));
    event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("escape").unwrap(), is_held: false }), cx);
    assert!(!editor.read(cx).menu && !slash.read(cx).is_open());
    assert_eq!(editor.read(cx).document.blocks[0].text, "/código", "Escape only dismisses the menu");
    println!("Native slash dropdown verified: input focus, filtering, arrows, Enter conversion and Escape dismissal.");
    for light in [false, true] {
        cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
            crate::app_theme::apply(light, cx);
            this.set_value("- [ ] First task\n- [x] Second task".into(), w, cx);
        })).unwrap();
        frame(handle, cx);
        let first_id = editor.read(cx).document.blocks[0].id.clone();
        let checkbox = editor.read(cx).inputs[&first_id].checkbox.clone().unwrap();
        let task_point = |cx: &App| bounds(&first_id, cx).origin + point(px(66.), px(4. + editor.read(cx).font_size * 0.8));
        let point = task_point(cx);
        down(handle, point, cx);
        up(handle, point, cx);
        frame(handle, cx);
        assert!(checkbox.read(cx).checked());
        assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Task(true));
        assert!(editor.read(cx).value().starts_with("- [x] First task"));
        assert!(editor.read(cx).text_selection.is_none(), "checkbox clicks must not select document text");
        cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.history(false, w, cx))).unwrap();
        frame(handle, cx);
        assert!(!checkbox.read(cx).checked(), "undo must restore the component state");
        cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| this.history(true, w, cx))).unwrap();
        frame(handle, cx);
        assert!(checkbox.read(cx).checked(), "redo must restore the component state");
        // Keep the same component while the block moves to a different index.
        cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
            this.document.blocks.swap(0, 1);
            this.changed(w, cx);
        })).unwrap();
        frame(handle, cx);
        let point = task_point(cx);
        down(handle, point, cx);
        up(handle, point, cx);
        frame(handle, cx);
        assert_eq!(editor.read(cx).document.blocks[0].kind, Kind::Task(true));
        assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Task(false), "toggle must follow block identity after reordering");
        event(handle, PlatformInput::KeyDown(KeyDownEvent { keystroke: Keystroke::parse("space").unwrap(), is_held: false }), cx);
        frame(handle, cx);
        assert_eq!(editor.read(cx).document.blocks[1].kind, Kind::Task(true), "Space must toggle the focused checkbox");
        assert_eq!(editor.read(cx).document.blocks[1].text, "First task", "checkbox keyboard activation must not edit the task text");
        cx.update_window(handle, |_, w, cx| editor.update(cx, |this, cx| {
            this.document.blocks[1].kind = Kind::Paragraph;
            this.changed(w, cx);
        })).unwrap();
        assert!(editor.read(cx).inputs[&first_id].checkbox.is_none(), "conversion must remove checkbox state and subscription");
    }
    // A remote insertion must preserve the native input, focus and local undo origin.
    let seed=sparkpad_sync::Document::new(101);
    seed.set_text("body","Hello 🌱 world\n\n- [ ] Task");
    let remote=sparkpad_sync::Document::load(102,&seed.state()).unwrap();
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{
        this.set_value("Hello 🌱 world\n\n- [ ] Task".into(),w,cx);
        this.bind_shared_document(&seed.state()).unwrap();
        this.checkpoint(cx);this.document.blocks[0].edit("Hello 🌱 brave world".into());this.changed(w,cx);
        this.select_block(this.document.blocks[0].id.clone(),w,cx);
    })).unwrap();
    let block_id=editor.read(cx).document.blocks[0].id.clone();
    let input=editor.read(cx).inputs[&block_id].state.clone();
    cx.update_window(handle,|_,w,cx|input.update(cx,|input,cx|{input.focus(w,cx);input.set_byte_selection(12..12,cx);})).unwrap();
    remote.set_text("body","Remote introduction\n\nHello 🌱 world\n\n- [x] Task");
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.apply_shared_state(&remote.state(),w,cx).unwrap())).unwrap();
    assert_eq!(editor.read(cx).document.blocks.iter().find(|b|b.text=="Hello 🌱 brave world").unwrap().id,block_id);
    assert_eq!(editor.read(cx).inputs[&block_id].state.entity_id(),input.entity_id());
    assert!(editor.read(cx).value().contains("Hello 🌱 brave world"));
    assert!(editor.read(cx).value().contains("- [x] Task"));
    cx.update_window(handle,|_,w,cx|{
        assert!(input.read(cx).focus_handle(cx).is_focused(w));
        editor.update(cx,|this,cx|this.history(false,w,cx));
    }).unwrap();
    assert!(editor.read(cx).value().contains("Remote introduction"));
    assert!(editor.read(cx).value().contains("Hello 🌱 world"));
    assert!(!editor.read(cx).value().contains("brave"));
    assert!(editor.read(cx).value().contains("- [x] Task"));
    // Image geometry and Markdown width survive resize/undo; tables preserve source when edited.
    let root=std::env::temp_dir().join(format!("sparkpad-image-native-{}",uuid::Uuid::new_v4()));
    let url=crate::document_images::store(&root,"svg",b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"450\"><rect width=\"800\" height=\"450\" fill=\"#a58adf\"/></svg>").unwrap();
    let source=format!("Before\n\n![Caption]({url})\n\n| Name | Value |\n| :--- | ---: |\n| **Example** | 10 |\n\nAfter");
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.set_asset_directory(root.clone());this.set_value(source.clone(),w,cx);})).unwrap();
    frame(handle,cx);
    let image_id=editor.read(cx).document.blocks.iter().find(|b|matches!(b.kind,Kind::Image{..})).unwrap().id.clone();
    let table_id=editor.read(cx).document.blocks.iter().find(|b|b.kind==Kind::Table).unwrap().id.clone();
    let geometry=editor.read(cx).row_bounds.borrow()[&image_id];
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{
        this.checkpoint(cx);this.resizing_image=Some(image_id.clone());
        let event=MouseMoveEvent{position:point(geometry.left()+px(58.)+(geometry.size.width-px(74.))*0.5,geometry.top()),..Default::default()};
        assert!(this.resize_image(&event,w,cx));this.resizing_image=None;
    })).unwrap();
    let image=editor.read(cx).document.blocks.iter().find(|b|b.id==image_id).unwrap();
    assert!(matches!(&image.kind,Kind::Image{title,..}if crate::blocks::image_width(title.as_deref())==50));
    assert!(image.markdown().contains("sparkpad-width=50"));
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.history(false,w,cx))).unwrap();
    assert_eq!(editor.read(cx).value().as_ref(),source);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{let index=this.document.blocks.iter().position(|b|b.id==table_id).unwrap();this.activate(index,0,w,cx);})).unwrap();frame(handle,cx);
    assert_eq!(editor.read(cx).value().as_ref(),source,"viewing and editing tables must preserve all Markdown");
    // Edit actual native table cells; keyboard navigation and structural operations share document undo.
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.focus_table_cell(&table_id,1,0,w,cx))).unwrap();frame(handle,cx);
    event(handle,PlatformInput::KeyDown(KeyDownEvent{keystroke:Keystroke::parse("cmd-a").unwrap(),is_held:false}),cx);
    assert!(editor.read(cx).text_selection.is_none(),"select-all within a table must stay within the focused cell");
    let cell=editor.read(cx).tables[&table_id].cells[&(1,0)].state.clone();
    assert_eq!(cell.read(cx).selection_range(),0.."Example".len());
    cx.update_window(handle,|_,w,cx|cell.update(cx,|s,cx|{s.set_byte_selection(0..s.value().len(),cx);s.replace_text_in_range(None,"Café 🌱",w,cx);})).unwrap();frame(handle,cx);
    assert!(editor.read(cx).value().contains("**Café 🌱**"));assert!(editor.read(cx).value().contains("| :--- | ---: |"));assert!(editor.read(cx).value().contains("| 10 |"));
    event(handle,PlatformInput::KeyDown(KeyDownEvent{keystroke:Keystroke::parse("tab").unwrap(),is_held:false}),cx);
    cx.update_window(handle,|_,w,cx|assert!(editor.read(cx).tables[&table_id].cells[&(1,1)].state.read(cx).focus_handle(cx).is_focused(w))).unwrap();
    event(handle,PlatformInput::KeyDown(KeyDownEvent{keystroke:Keystroke::parse("enter").unwrap(),is_held:false}),cx);
    assert_eq!(editor.read(cx).tables[&table_id].model.rows.len(),3);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.table_structure(&table_id,TableChange::AddColumn,w,cx);})).unwrap();frame(handle,cx);
    assert_eq!(editor.read(cx).tables[&table_id].model.align.len(),3);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.table_history(false,&table_id,2,1,w,cx))).unwrap();frame(handle,cx);
    assert_eq!(editor.read(cx).tables[&table_id].model.align.len(),2);
    event(handle,PlatformInput::KeyDown(KeyDownEvent{keystroke:Keystroke::parse("cmd-shift-z").unwrap(),is_held:false}),cx);
    assert_eq!(editor.read(cx).tables[&table_id].model.align.len(),3);
    event(handle,PlatformInput::KeyDown(KeyDownEvent{keystroke:Keystroke::parse("cmd-z").unwrap(),is_held:false}),cx);
    assert_eq!(editor.read(cx).tables[&table_id].model.align.len(),2);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.table_structure(&table_id,TableChange::RemoveRow(2),w,cx))).unwrap();
    assert_eq!(editor.read(cx).tables[&table_id].model.rows.len(),2);
    println!("Native visual tables verified: formatted Unicode cell edits, preserved neighbors/alignment, Tab, Enter adds row, add column, structural undo and remove row.");
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.active=None;this.focus_handle.focus(w);cx.notify();})).unwrap();frame(handle,cx);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{
        this.set_value("Before After".into(),w,cx);this.activate(0,7,w,cx);
        let image=Image::from_bytes(ImageFormat::Png,include_bytes!("../assets/app/icon.png").to_vec());
        cx.write_to_clipboard(ClipboardItem::new_image(&image));assert!(this.paste_clipboard(false,w,cx));
        assert!(this.document.blocks.iter().any(|b|matches!(b.kind,Kind::Image{..})),"clipboard image must become its own block");
        assert!(this.value().starts_with("Before "));assert!(this.value().ends_with("After"));
        this.history(false,w,cx);assert_eq!(this.value().as_ref(),"Before After");
    })).unwrap();
    // Virtualization must preserve the whole model, keyboard focus and history.
    let large=(0..1000).map(|i|format!("Paragraph {i}: **café** and ideas 🌱.")).collect::<Vec<_>>().join("\n\n");
    let virtual_scroll=ScrollHandle::new();
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.set_scroll_handle(virtual_scroll.clone());this.set_value(large.clone(),w,cx);})).unwrap();
    for _ in 0..3{frame(handle,cx);}
    assert_eq!(editor.read(cx).document.blocks.len(),1000);
    assert!(editor.read(cx).inputs.len()<100,"a large page must not create every native field");
    assert!(editor.read(cx).row_bounds.borrow().len()<100,"only viewport rows should render");
    assert_eq!(editor.read(cx).value().as_ref(),large);
    virtual_scroll.set_offset(point(px(0.),px(-12000.)));for _ in 0..3{frame(handle,cx);}
    assert!(editor.read(cx).row_bounds.borrow().len()<100);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.activate(999,0,w,cx))).unwrap();for _ in 0..3{frame(handle,cx);}
    let last=editor.read(cx).state(999);
    cx.update_window(handle,|_,w,cx|last.update(cx,|s,cx|s.replace_text_in_range(Some(0..0),"Last 🌱 ",w,cx))).unwrap();frame(handle,cx);
    assert!(editor.read(cx).value().contains("Last 🌱 Paragraph 999"));
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.history(false,w,cx))).unwrap();frame(handle,cx);
    assert_eq!(editor.read(cx).value().as_ref(),large);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.select_document(w,cx))).unwrap();frame(handle,cx);
    assert_eq!(editor.read(cx).text_selection.unwrap().ordered(),((0,0),(999,editor.read(cx).document.blocks[999].text.len())));
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.clear_selection(cx);this.set_font_size(28.,cx);this.read_mode(w,cx);})).unwrap();for _ in 0..3{frame(handle,cx);}
    assert_eq!(editor.read(cx).value().as_ref(),large);
    let big_table="| Name | Value |\n| --- | --- |\n".to_owned()+(0..400).map(|i|format!("| Item {i} | {i} |\n")).collect::<String>().as_str();
    virtual_scroll.set_offset(point(px(0.),px(0.)));
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|{this.set_font_size(22.,cx);this.set_value(big_table.clone(),w,cx);})).unwrap();for _ in 0..3{frame(handle,cx);}
    let id=editor.read(cx).document.blocks[0].id.clone();assert!(editor.read(cx).tables[&id].row_heights.borrow().len()<80);
    cx.update_window(handle,|_,w,cx|editor.update(cx,|this,cx|this.focus_table_cell(&id,400,1,w,cx))).unwrap();for _ in 0..3{frame(handle,cx);}
    let cell=editor.read(cx).tables[&id].cells[&(400,1)].state.clone();
    cx.update_window(handle,|_,w,cx|cell.update(cx,|s,cx|{s.set_byte_selection(0..s.value().len(),cx);s.replace_text_in_range(None,"Last cell",w,cx);})).unwrap();frame(handle,cx);
    assert!(editor.read(cx).value().contains("| Item 399 | Last cell |"));
    println!("Native virtualization verified: full Markdown, bounded fields/rows, scrolling, distant focus, Unicode editing, undo, document selection, font reflow and offscreen table-cell editing.");
    std::fs::remove_dir_all(root).unwrap();
    println!("Native images/tables verified: image import, proportional width, resize undo, table preview/edit and lossless Markdown.");
    println!("Native collaborative editing verified: remote insertion, Unicode, checkbox, input identity, focus, and own undo preserving remote work.");
    println!("Native empire-ui task checkboxes verified: click, keyboard, Markdown, undo/redo, reordered identity, conversion, and both themes.");
    cx.update_window(handle, |_, w, _| w.remove_window()).unwrap();
    println!("Native rich clipboard verified: standard HTML export, external formatted HTML paste, partial selections, Unicode, surrounding styles and undo.");
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
    fn code_indentation_preserves_unicode_ranges_and_line_boundaries() {
        use super::code_indent;
        let text = "ação\ntexto\nlast";
        let end = text.find("last").unwrap();
        let (indented, range) = code_indent(text, 0..end, false);
        assert_eq!(indented, "  ação\n  texto\nlast");
        let (restored, restored_range) = code_indent(&indented, range, true);
        assert_eq!(restored, text);
        assert_eq!(restored_range, 0..end);
        assert_eq!(code_indent("\tx\n x", 0..5, true).0, "x\nx");
        assert_eq!(code_indent(" x", 0..0, true), ("x".into(), 0..0));
        assert_eq!(code_indent("x", 1..1, true), ("x".into(), 1..1));
        assert_eq!(code_indent("", 0..0, false), ("  ".into(), 2..2));
        assert_eq!(code_indent("coração", 2..2, false).1, 4..4);
    }
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

#[cfg(test)]
pub fn benchmark_edit(editor:&Entity<BlockEditor>,window:&mut Window,cx:&mut App){
    editor.update(cx,|this,cx|{
        if let Some(id)=this.document.blocks.iter().find(|b|b.kind==Kind::Table).map(|b|b.id.clone()){
            this.focus_table_cell(&id,1,0,window,cx);let state=this.tables[&id].cells[&(1,0)].state.clone();
            state.update(cx,|s,cx|s.replace_text_in_range(Some(0..0),"x",window,cx));
            this.table_event(&id,1,0,&InputEvent::Change,window,cx);return;
        }
        let Some(index)=this.document.blocks.iter().position(|b|b.kind==Kind::Paragraph||matches!(b.kind,Kind::Code(_)))else{return};
        if this.active.as_ref()!=Some(&this.document.blocks[index].id){this.activate(index,0,window,cx);}let state=this.state(index);
        state.update(cx,|s,cx|s.replace_text_in_range(Some(0..0),"x",window,cx));
        this.input_event(&this.document.blocks[index].id.clone(),&InputEvent::Change,window,cx);
    });
}
#[cfg(test)]
pub fn benchmark_counts(editor:&Entity<BlockEditor>,cx:&App)->serde_json::Value{
    let s=editor.read(cx);serde_json::json!({"blocks":s.document.blocks.len(),"inputs":s.inputs.len(),"table_cells":s.tables.values().map(|t|t.cells.len()).sum::<usize>(),"measured_rows":s.row_bounds.borrow().len()})
}
