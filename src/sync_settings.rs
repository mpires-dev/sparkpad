use crate::{
    app_theme::neutral,
    store::Store,
    sync_client::{self, Config},
};
use empire_ui::{Button, ButtonVariant, Input};
use gpui::{prelude::*, *};
use gpui_component::input::InputState;
use std::path::PathBuf;

pub struct SyncSettings {
    path: PathBuf,
    url: Entity<InputState>,
    token: Entity<InputState>,
    busy: bool,
    error: Option<String>,
    onboarding: bool,
}
pub struct Closed;
impl EventEmitter<Closed> for SyncSettings {}
impl SyncSettings {
    pub fn new(
        path: PathBuf,
        onboarding: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let config = sync_client::read_config(&path);
        let url = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("https://your-server");
            if let Some(config) = &config {
                state.set_value(config.url.clone(), window, cx);
            }
            state
        });
        let token = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .masked(true)
                .placeholder("Server connection key");
            if let Some(config) = &config {
                state.set_value(config.token.clone(), window, cx);
            }
            state
        });
        Self {
            path,
            url,
            token,
            busy: false,
            error: None,
            onboarding,
        }
    }
    fn finish(&mut self, cx: &mut Context<Self>) {
        if let Ok(store) = Store::open(&self.path) {
            let _ = store.set_setting("onboarding_completed", "true");
        }
        cx.emit(Closed);
        cx.notify();
    }
    fn local(&mut self, cx: &mut Context<Self>) {
        match sync_client::write_config(&self.path, None)
            .and_then(|_| crate::sync_storage::disable(&Store::open(&self.path)?))
        {
            Ok(()) => self.finish(cx),
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn connect(&mut self, cx: &mut Context<Self>) {
        let config = Config {
            url: self
                .url
                .read(cx)
                .value()
                .trim()
                .trim_end_matches('/')
                .into(),
            token: self.token.read(cx).value().trim().into(),
        };
        self.busy = true;
        self.error = None;
        cx.notify();
        let path = self.path.clone();
        let task = cx.background_executor().spawn(async move {
            sync_client::validate_connection(&config)?;
            sync_client::write_config(&path, Some(&config))?;
            let store = Store::open(&path)?;
            crate::sync_storage::enable(&store)?;
            crate::sync_storage::drain(&store)?;
            anyhow::Ok(())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => this.finish(cx),
                    Err(e) => {
                        this.error = Some(e.to_string());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }
}
impl Render for SyncSettings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("sync-settings").w(px(460.)).max_w_full().max_h(px(f32::from(window.bounds().size.height)-32.))
            .overflow_y_scroll().p_6().rounded_xl().bg(neutral(0x202020)).border_1().border_color(neutral(0x414141))
            .flex().flex_col().gap_4()
            .child(div().flex().items_center().gap_3().child(img("sparkpad/app-icon.png").size(px(48.)))
                .child(div().flex().flex_col().child(div().text_size(px(24.)).font_weight(FontWeight::BOLD).child(if self.onboarding {"Welcome to Sparkpad"} else {"Storage & sync"}))
                .child(div().text_sm().text_color(neutral(0xa3a3a3)).child("Your notes always stay available on this device."))))
            .child(div().p_4().rounded_lg().border_1().border_color(neutral(0x414141)).flex().flex_col().gap_2()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Keep everything on this Mac"))
                .child(div().text_sm().text_color(neutral(0xa3a3a3)).child("No account or server. Write and organize your notes completely offline."))
                .child(Button::new("sync-local","Use local mode").variant(ButtonVariant::Secondary).disabled(self.busy).on_click(cx.listener(|this,_,_,cx|this.local(cx)))))
            .child(div().flex().flex_col().gap_2().child(div().font_weight(FontWeight::SEMIBOLD).child("Connect to your server"))
                .child(div().text_sm().text_color(neutral(0xa3a3a3)).child("Sync across devices and edit together. Local notes will be added to the server; downloaded notes remain available offline."))
                .child(Input::new(&self.url)).child(Input::new(&self.token))
                .child(Button::new("sync-connect",if self.busy {"Connecting…"} else {"Connect & sync"}).disabled(self.busy).on_click(cx.listener(|this,_,_,cx|this.connect(cx)))))
            .children(self.error.as_ref().map(|error|div().text_sm().text_color(rgb(0xf08b8b)).child(error.clone())))
            .when(!self.onboarding,|view|view.child(Button::new("sync-cancel","Cancel").variant(ButtonVariant::Ghost).disabled(self.busy).on_click(cx.listener(|_,_,_,cx|cx.emit(Closed)))))
            .child(div().text_xs().text_color(neutral(0xa3a3a3)).child("You can change this choice later in Storage & sync."))
    }
}
