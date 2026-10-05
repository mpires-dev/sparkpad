use gpui::{rgb, App, Rgba};
use gpui_component::{Theme, ThemeMode};

// Shared neutral colors for the page, editor and overlays. Cover colors are independent.
pub fn neutral(dark: u32) -> Rgba {
    let color = if empire_ui::theme_mode() == empire_ui::ThemeMode::Light {
        match dark {
            0x161616 => 0xffffff, 0x1b1b1b => 0xf7f7f7, 0x202020 => 0xffffff,
            0x232323 => 0xefefef, 0x2a2a2a | 0x2c2c2c => 0xe8e8e8,
            0x303030 => 0xe5e5e5, 0x383838 => 0xdedede, 0x414141 => 0xd4d4d4,
            0x484848 => 0xd8d8d8, 0xeeeeee => 0x262626, 0xd4d4d4 => 0x404040,
            0xbcbcbc => 0x525252, 0xa3a3a3 | 0x999999 => 0x737373,
            0x858585 => 0x808080, 0x666666 => 0xa3a3a3, 0x482a27 => 0xffe5e0,
            other => other,
        }
    } else { dark };
    rgb(color)
}

pub fn apply(light: bool, cx: &mut App) {
    empire_ui::set_theme(if light {empire_ui::ThemeMode::Light} else {empire_ui::ThemeMode::Dark});
    Theme::change(if light {ThemeMode::Light} else {ThemeMode::Dark}, None, cx);
    let theme=Theme::global_mut(cx);
    theme.font_family=crate::assets::FONT_FAMILY.into();
    theme.background=neutral(0x161616).into();
    theme.foreground=neutral(0xeeeeee).into();
    theme.muted_foreground=neutral(0xa3a3a3).into();
    theme.caret=neutral(0xeeeeee).into();
    theme.selection=neutral(0x484848).into();
    theme.accent=neutral(0x303030).into();
    theme.accent_foreground=neutral(0xeeeeee).into();
    theme.primary=neutral(0xd4d4d4).into();
    theme.primary_foreground=neutral(0x161616).into();
    theme.primary_hover=neutral(0xeeeeee).into();
    theme.primary_active=neutral(0xbcbcbc).into();
    theme.ring=neutral(0xa3a3a3).into();
    theme.border=neutral(0x414141).into();
    theme.input=neutral(0x414141).into();
    theme.popover=neutral(0x202020).into();
    theme.popover_foreground=neutral(0xeeeeee).into();
    theme.secondary=neutral(0x303030).into();
    theme.secondary_hover=neutral(0x383838).into();
    theme.secondary_foreground=neutral(0xeeeeee).into();
    theme.link=neutral(0xd4d4d4).into();
    theme.drag_border=neutral(0xd4d4d4).into();
}
