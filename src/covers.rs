//! Lightweight cover presets: drawn by GPUI and stored as stable IDs in SQLite.
pub struct CoverPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub colors: [u32; 2],
    pub angle: f32,
}

pub const PRESETS: &[CoverPreset] = &[
    CoverPreset { id: "coral", name: "Coral", colors: [0xe84c50, 0xe84c50], angle: 0. },
    CoverPreset { id: "gold", name: "Amarelo", colors: [0xf8b750, 0xf8b750], angle: 0. },
    CoverPreset { id: "blue", name: "Azul", colors: [0x2091bb, 0x2091bb], angle: 0. },
    CoverPreset { id: "cream", name: "Creme", colors: [0xffeddf, 0xffeddf], angle: 0. },
    CoverPreset { id: "mint", name: "Menta", colors: [0x36b5bc, 0xc6b1a9], angle: 180. },
    CoverPreset { id: "pink", name: "Rosa", colors: [0xff2e86, 0xff2e86], angle: 0. },
    CoverPreset { id: "sunset", name: "Pôr do sol", colors: [0xc97956, 0xf21d25], angle: 135. },
    CoverPreset { id: "dawn", name: "Amanhecer", colors: [0xdc9c89, 0x9fd3e0], angle: 135. },
    CoverPreset { id: "ocean", name: "Oceano", colors: [0x23769e, 0xeab3d1], angle: 160. },
    CoverPreset { id: "lavender", name: "Lavanda", colors: [0x654cb9, 0xd44261], angle: 145. },
    CoverPreset { id: "slate", name: "Ardósia", colors: [0x394055, 0x9ec3c8], angle: 135. },
    CoverPreset { id: "neutral", name: "Cinza", colors: [0x454545, 0xa7a7a7], angle: 135. },
];

const PREFIX: &str = "sparkpad:cover:";

pub fn stored_value(preset: &CoverPreset) -> String {
    format!("{PREFIX}{}", preset.id)
}

pub fn preset(value: &str) -> Option<&'static CoverPreset> {
    let id = value.strip_prefix(PREFIX)?;
    PRESETS.iter().find(|preset| preset.id == id)
}
