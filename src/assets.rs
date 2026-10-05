use gpui::{AssetSource, SharedString, TextSystem};
use std::borrow::Cow;

pub const BLOCK_GRIP: &str = "sparkpad/icons/grip-vertical.svg";

pub const FONT_FAMILY: &str = "NV Legible Next";

pub const CONTENT_FONTS: [(&str, &str, &str); 3] = [
    ("sans", FONT_FAMILY, "Sans serif — NV Legible Next"),
    ("serif", "Libron", "Serifada — Libron"),
    ("mono", "JetBrains Mono", "Mono — JetBrains Mono"),
];

/// Embed all faces so typography does not depend on fonts installed on the Mac.
pub fn register_fonts(text_system: &TextSystem) {
    text_system.add_fonts(vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/NV_Legible_Next-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/NV_Legible_Next-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/NV_Legible_Next-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/NV_Legible_Next-BoldItalic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Libron-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Libron-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Libron-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Libron-BoldItalic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf")),
    ]).expect("Register embedded Sparkpad fonts");
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == BLOCK_GRIP {
            Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/grip-vertical.svg"
            ))))
        } else {
            empire_ui::assets::Assets.load(path)
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut assets = empire_ui::assets::Assets.list(path)?;
        if BLOCK_GRIP.starts_with(path) {
            assets.push(BLOCK_GRIP.into());
        }
        Ok(assets)
    }
}
