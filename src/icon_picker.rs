use crate::app_theme::neutral;
use empire_ui::{Button, ButtonSize, ButtonVariant, Input};
use gpui::{prelude::*, *};
use gpui_component::{input::{InputEvent, InputState},scroll::Scrollbar};
use std::collections::HashSet;
use crate::icon_catalog::{emojis, fold, icon_paths};
pub use crate::icon_catalog::{SVG_PREFIX, IMAGE_PREFIX};

const CATEGORIES: [(&str,&str);9] = [
    ("Pessoas e emoções","emoji"),("Pessoas e corpo","hand-brake"),
    ("Animais e natureza","leaf"),("Comidas e bebidas","apple"),
    ("Viagens e lugares","airplane"),("Atividades","soccer-ball"),
    ("Objetos","light-bulb"),("Símbolos","check-circle"),("Bandeiras","white-flag"),
];
fn base(value:&str) -> String {value.chars().filter(|c| !(0x1f3fb..=0x1f3ff).contains(&(*c as u32))).collect()}
pub fn render_icon(value:&str,size:f32) -> AnyElement {
    if let Some(path)=value.strip_prefix(SVG_PREFIX) {
        svg().path(path.to_owned()).size(px(size)).text_color(neutral(0xbcbcbc)).into_any_element()
    } else if let Some(path)=value.strip_prefix(IMAGE_PREFIX) {
        img(std::path::PathBuf::from(path)).size(px(size)).object_fit(ObjectFit::Contain).rounded(px(size*0.08)).into_any_element()
    } else {
        div().size(px(size)).flex().items_center().justify_center().line_height(relative(1.))
            .text_size(px(size)).font_weight(FontWeight::NORMAL).font_family("Apple Color Emoji").child(value.to_owned()).into_any_element()
    }
}
#[derive(Clone,Copy,PartialEq)]
enum Tab {Emoji,Icons,Upload}
#[derive(Clone)]
struct Choice {value:String,name:String}
#[derive(Clone)]
enum Row {Heading(String),Choices(Vec<Choice>)}
pub enum PickerEvent {Select(Option<String>),Upload}
pub struct IconPicker {
    search:Entity<InputState>,_search_subscription:Subscription,
    tab:Tab,category:Option<usize>,tone:usize,show_tones:bool,hovered_name:String,
    recent:Vec<String>,current:Option<String>,rows:Vec<Row>,
    scroll:UniformListScrollHandle,columns:usize,last_query:String,
    #[cfg(test)] pub geometry:std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String,Bounds<Pixels>>>>,
}
impl IconPicker {
    pub fn new(recent:Vec<String>,window:&mut Window,cx:&mut Context<Self>) -> Self {
        let search=cx.new(|cx| empire_ui::input::single_line(window,cx).placeholder("Filtrar…"));
        let subscription=cx.subscribe(&search,|this,_,event,cx| {
            if matches!(event,InputEvent::Change) {this.rebuild(cx);cx.notify();}
        });
        let mut picker=Self {search,_search_subscription:subscription,tab:Tab::Emoji,category:None,tone:0,show_tones:false,hovered_name:String::new(),
            recent,current:None,rows:Vec::new(),scroll:UniformListScrollHandle::default(),columns:10,last_query:String::new(),
            #[cfg(test)] geometry:Default::default(),
        };
        picker.rebuild(cx);picker
    }
    pub fn reset(&mut self,current:Option<String>,window:&mut Window,cx:&mut Context<Self>) {
        self.tab=if current.as_deref().is_some_and(|v| v.starts_with(SVG_PREFIX)) {Tab::Icons}
            else if current.as_deref().is_some_and(|v| v.starts_with(IMAGE_PREFIX)) {Tab::Upload} else {Tab::Emoji};
        self.current=current;self.category=None;self.show_tones=false;self.hovered_name.clear();
        self.search.update(cx,|s,cx| s.set_value("",window,cx));self.rebuild(cx);cx.notify();
    }
    pub fn set_recent(&mut self,recent:Vec<String>,cx:&mut Context<Self>) {self.recent=recent;self.rebuild(cx);cx.notify();}
    #[cfg(test)]
    pub fn set_search(&mut self,value:&str,window:&mut Window,cx:&mut Context<Self>) {
        self.search.update(cx,|s,cx| s.set_value(value.to_owned(),window,cx));self.rebuild(cx);cx.notify();
    }
    fn choose(&mut self,value:String,cx:&mut Context<Self>) {cx.emit(PickerEvent::Select(Some(value)));}
    fn section(&mut self,name:&str,choices:Vec<Choice>) {
        if choices.is_empty() {return;}
        self.rows.push(Row::Heading(name.to_owned()));
        self.rows.extend(choices.chunks(self.columns).map(|c| Row::Choices(c.to_vec())));
    }
    fn rebuild(&mut self,cx:&App) {
        #[cfg(test)] self.geometry.borrow_mut().clear();
        self.rows.clear();self.last_query=fold(self.search.read(cx).value().as_ref());
        let query=self.last_query.clone();
        if self.tab==Tab::Emoji {
            let chosen_bases:HashSet<_>=emojis().iter().filter(|e| e.tone==self.tone && self.tone>0 && self.tone<6).map(|e| base(&e.value)).collect();
            if query.is_empty() && self.category.is_none() {
                self.section("Recentes",self.recent.iter().filter(|v| !v.starts_with("sparkpad:icon:")).map(|v| Choice {value:v.clone(),name:v.clone()}).collect());
            }
            for category in 0..CATEGORIES.len() {
                if self.category.is_some_and(|c| c!=category) {continue;}
                let choices=emojis().iter().filter(|e| e.category==category)
                    .filter(|e| self.tone==6 || e.tone==self.tone || (e.tone==0 && !chosen_bases.contains(&base(&e.value))))
                    .filter(|e| query.is_empty() || e.search.contains(&query) || e.value.contains(&query))
                    .map(|e| Choice {value:e.value.clone(),name:e.name.clone()}).collect();
                self.section(CATEGORIES[category].0,choices);
            }
        } else if self.tab==Tab::Icons {
            if query.is_empty() {
                self.section("Recentes",self.recent.iter().filter(|v| v.starts_with(SVG_PREFIX)).map(|v| Choice {value:v.clone(),name:v.rsplit('/').next().unwrap_or(v).trim_end_matches(".svg").replace('-'," ")}).collect());
            }
            let search=match query.as_str() {"coracao"=>"heart","pasta"=>"folder","estrela"=>"star","casa"=>"home","pessoa"=>"user","livro"=>"book","nota"=>"page",other=>other};
            let choices=icon_paths().iter().filter(|path| search.is_empty() || path.contains(&search.replace(' ',"-")))
                .map(|path| Choice {value:format!("{SVG_PREFIX}{path}"),name:path.rsplit('/').next().unwrap_or(path).trim_end_matches(".svg").replace('-'," ")}).collect();
            self.section("Ícones",choices);
        }
        self.scroll.scroll_to_item(0,ScrollStrategy::Top);
    }
    #[cfg(test)]
    pub fn scroll_to_end(&self) {
        self.scroll.scroll_to_item(self.rows.len().saturating_sub(1),ScrollStrategy::Bottom);
    }
    #[cfg(test)]
    pub fn scroll_offset(&self) -> Pixels { self.scroll.0.borrow().base_handle.offset().y }
    #[cfg(test)]
    fn measure(&self,key:String) -> AnyElement {
        let geometry=self.geometry.clone();canvas(move |bounds,_,_| {geometry.borrow_mut().insert(key.clone(),bounds);},|_,_,_,_| {})
            .absolute().top_0().left_0().size_full().into_any_element()
    }
    fn row(&mut self,index:usize,cx:&mut Context<Self>) -> AnyElement {
        let row=self.rows[index].clone();
        let content=match row {
            Row::Heading(name)=>div().w_full().h(px(40.)).flex().items_center().text_xs().font_weight(FontWeight::MEDIUM).text_color(neutral(0xa3a3a3)).child(name),
            Row::Choices(choices)=> {
                let count=choices.len();let mut row=div().w_full().h(px(40.)).flex().items_center();
                for (col,choice) in choices.into_iter().enumerate() {
                    let value=choice.value.clone();let name=choice.name.clone();let selected=self.current.as_deref()==Some(value.as_str());
                    let cell=div().id(("icon-choice",index*self.columns+col)).group(SharedString::from(format!("choice-{index}-{col}")))
                        .relative().flex_1().min_w_0().h(px(36.)).rounded(px(5.)).cursor_pointer().flex().items_center().justify_center()
                        .when(selected,|d| d.bg(neutral(0x303030)))
                        .hover(|s| s.bg(neutral(0x383838)))
                        .child(render_icon(&choice.value,if self.tab==Tab::Emoji {25.} else {21.}))
                        .on_hover(cx.listener(move |this,inside:&bool,_,cx| {
                            if *inside {this.hovered_name=name.clone();}
                            else if this.hovered_name==name {this.hovered_name.clear();}
                            cx.notify();
                        }))
                        .on_click(cx.listener(move |this,_,_,cx| this.choose(value.clone(),cx)));
                    #[cfg(test)] let cell=cell.child(self.measure(format!("choice:{}",choice.value)));
                    row=row.child(cell);
                }
                for _ in count..self.columns {row=row.child(div().flex_1().min_w_0().h(px(36.)));}
                row
            }
        };
        content.into_any_element()
    }
}
impl EventEmitter<PickerEvent> for IconPicker {}
impl Render for IconPicker {
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>) -> impl IntoElement {
        let width=(f32::from(window.viewport_size().width)-50.).min(382.);
        let columns=(width/36.).floor().max(4.) as usize;
        if columns!=self.columns {self.columns=columns;self.rebuild(cx);}
        let height=(f32::from(window.viewport_size().height)-198.-if self.show_tones {34.} else {0.}).clamp(72.,280.);
        let mut tabs=div().w_full().flex().items_center().gap_2().h(px(34.)).border_b_1().border_color(neutral(0x414141));
        for (tab,label) in [(Tab::Emoji,"Emoji"),(Tab::Icons,"Ícones"),(Tab::Upload,"Fazer upload")] {
            let tab_button=div().id(label).relative().h(px(34.)).px_1().flex().items_center().cursor_pointer()
                .text_sm().text_color(if self.tab==tab {neutral(0xeeeeee)} else {neutral(0xa3a3a3)})
                .when(self.tab==tab,|d| d.border_b_2().border_color(neutral(0xeeeeee)))
                .on_click(cx.listener(move |this,_,_,cx| {this.tab=tab;this.category=None;this.show_tones=false;this.rebuild(cx);cx.notify();})).child(label);
            #[cfg(test)] let tab_button=tab_button.child(self.measure(format!("tab:{label}")));
            tabs=tabs.child(tab_button);
        }
        tabs=tabs.child(div().flex_1()).child(Button::new("picker-remove","Remover").size(ButtonSize::Sm).variant(ButtonVariant::Ghost)
            .on_click(cx.listener(|_,_,_,cx| cx.emit(PickerEvent::Select(None)))));
        let mut panel=div().w_full().min_w_0().font_family(crate::assets::FONT_FAMILY).flex().flex_col().gap_2().child(tabs);
        if self.tab==Tab::Upload {
            let mut upload=div().h(px(height+40.)).w_full().flex().flex_col().items_center().justify_center().gap_3();
            if let Some(value)=self.current.as_ref().filter(|v| v.starts_with(IMAGE_PREFIX)) {upload=upload.child(render_icon(value,72.));}
            upload=upload.child(Button::new("picker-upload","Escolher imagem").size(ButtonSize::Sm)
                .on_click(cx.listener(|_,_,_,cx| cx.emit(PickerEvent::Upload))))
                .child(div().text_xs().text_color(neutral(0xa3a3a3)).child("PNG, JPG, WebP, GIF ou SVG · até 10 MB"));
            panel=panel.child(upload);
        } else {
            let search=div().flex_1().min_w_0().child(Input::new(&self.search).clearable()
                .prefix(svg().path("iconoir/regular/search.svg").size(px(16.)).text_color(neutral(0xa3a3a3))));
            let random=Button::icon("picker-random","iconoir/regular/shuffle.svg").size(ButtonSize::IconSm).variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|this,_,_,cx| {
                    let choices:Vec<_>=this.rows.iter().filter_map(|r| if let Row::Choices(c)=r {Some(c)} else {None}).flatten().collect();
                    if !choices.is_empty() {let choice=choices[(uuid::Uuid::new_v4().as_u128()%choices.len() as u128) as usize].value.clone();this.choose(choice,cx);}
                }));
            let mut bar=div().w_full().flex().items_center().gap_2().child(search).child(random);
            if self.tab==Tab::Emoji {
                bar=bar.child(Button::new("picker-tone",["✋","✋🏻","✋🏼","✋🏽","✋🏾","✋🏿","✋"][self.tone])
                    .size(ButtonSize::Sm).variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this,_,_,cx| {this.show_tones=!this.show_tones;cx.notify();})));
            }
            panel=panel.child(bar);
            if self.show_tones {
                let mut tones=div().flex().items_center().gap_1();
                for (tone,label) in ["✋","✋🏻","✋🏼","✋🏽","✋🏾","✋🏿","Todos"].into_iter().enumerate() {
                    tones=tones.child(Button::new(("skin-tone",tone),label).size(ButtonSize::Sm)
                        .variant(if self.tone==tone {ButtonVariant::Secondary} else {ButtonVariant::Ghost})
                        .on_click(cx.listener(move |this,_,_,cx| {this.tone=tone;this.show_tones=false;this.rebuild(cx);cx.notify();})));
                }
                panel=panel.child(tones);
            }
            if self.rows.is_empty() {panel=panel.child(div().h(px(height)).flex().items_center().justify_center().text_sm().text_color(neutral(0xa3a3a3)).child("Nenhum resultado"));}
            else {
                // Scrollbar is an absolute element without insets of its own. Give it
                // an anchored overlay so its static position cannot follow the list.
                let scrollbar=div().absolute().inset_0()
                    .child(Scrollbar::vertical(&self.scroll).id("icon-grid-scrollbar"));
                #[cfg(test)] let scrollbar=scrollbar.child(self.measure("scrollbar".into()));
                let viewport=div().relative().h(px(height)).w_full().flex_none().overflow_hidden()
                .child(uniform_list("picker-grid",self.rows.len(),cx.processor(|this,range:std::ops::Range<usize>,_,cx| range.map(|i| this.row(i,cx)).collect::<Vec<_>>()))
                    .h_full().w_full().track_scroll(self.scroll.clone()))
                .child(scrollbar);
                #[cfg(test)] let viewport=viewport.child(self.measure("viewport".into()));
                panel=panel.child(viewport);
            }
            panel=panel.child(div().h(px(16.)).text_xs().text_color(neutral(0xa3a3a3)).truncate().child(self.hovered_name.clone()));
            if self.tab==Tab::Emoji {
                let mut categories=div().w_full().h(px(32.)).flex().items_center().gap(px(2.)).border_t_1().border_color(neutral(0x414141));
                categories=categories.child(Button::icon("picker-all","iconoir/regular/clock.svg").size(ButtonSize::IconSm)
                    .variant(if self.category.is_none() {ButtonVariant::Secondary} else {ButtonVariant::Ghost})
                    .on_click(cx.listener(|this,_,_,cx| {this.category=None;this.rebuild(cx);cx.notify();})));
                for (category,(_,icon)) in CATEGORIES.iter().enumerate() {
                    categories=categories.child(Button::icon(("picker-category",category),format!("iconoir/regular/{icon}.svg"))
                        .size(ButtonSize::IconSm).variant(if self.category==Some(category) {ButtonVariant::Secondary} else {ButtonVariant::Ghost})
                        .on_click(cx.listener(move |this,_,_,cx| {this.category=Some(category);this.rebuild(cx);cx.notify();})));
                }
                panel=panel.child(categories);
            }
        }
        panel
    }
}
#[cfg(test)]
mod tests {
    use super::{emojis,base,fold,icon_paths,CATEGORIES};
    use std::collections::HashSet;
    #[test]
    fn catalog_is_complete_searchable_and_tones_are_valid() {
        assert_eq!(emojis().len(),3944);
        let values:HashSet<_>=emojis().iter().map(|e| e.value.as_str()).collect();assert_eq!(values.len(),3944);
        for value in ["💻","🚀","👍🏿","👩‍💻","🇧🇷"] {assert!(values.contains(value));}
        assert!(emojis().iter().any(|e| e.value=="💻" && fold(&e.search).contains("computador")));
        assert_eq!(base("👍🏿"),"👍");assert_eq!(fold("Coração"),"coracao");
        assert!(icon_paths().len()>1600);assert!(icon_paths().iter().any(|p| p=="iconoir/regular/heart.svg"));
        for path in icon_paths() {assert!(empire_ui::assets::lookup(path).is_some(),"catalog references missing asset: {path}");}
        for (_,icon) in CATEGORIES {assert!(empire_ui::assets::lookup(&format!("iconoir/regular/{icon}.svg")).is_some(),"missing category icon: {icon}");}
    }
}
