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
    CoverPreset { id: "green", name: "Verde", colors: [0x58a56b, 0x58a56b], angle: 0. },
    CoverPreset { id: "forest", name: "Verde floresta", colors: [0x24634b, 0x24634b], angle: 0. },
    CoverPreset { id: "sage", name: "Sálvia", colors: [0xa6b99a, 0xa6b99a], angle: 0. },
    CoverPreset { id: "lime", name: "Lima", colors: [0xb8cf5a, 0xb8cf5a], angle: 0. },
    CoverPreset { id: "purple", name: "Roxo", colors: [0x8055c9, 0x8055c9], angle: 0. },
    CoverPreset { id: "lilac", name: "Lilás", colors: [0xc4a7e7, 0xc4a7e7], angle: 0. },
    CoverPreset { id: "plum", name: "Ameixa", colors: [0x78466d, 0x78466d], angle: 0. },
    CoverPreset { id: "indigo", name: "Índigo", colors: [0x4c5ba6, 0x4c5ba6], angle: 0. },
    CoverPreset { id: "sky", name: "Azul céu", colors: [0x8fc9e8, 0x8fc9e8], angle: 0. },
    CoverPreset { id: "turquoise", name: "Turquesa", colors: [0x49b8a8, 0x49b8a8], angle: 0. },
    CoverPreset { id: "orange", name: "Laranja", colors: [0xed934c, 0xed934c], angle: 0. },
    CoverPreset { id: "terracotta", name: "Terracota", colors: [0xbd7056, 0xbd7056], angle: 0. },
    CoverPreset { id: "rose", name: "Rosa suave", colors: [0xe8b2c0, 0xe8b2c0], angle: 0. },
    CoverPreset { id: "sand", name: "Areia", colors: [0xd7c4a5, 0xd7c4a5], angle: 0. },
    CoverPreset { id: "meadow", name: "Prado", colors: [0x2f806b, 0xc5dfa0], angle: 135. },
    CoverPreset { id: "matcha", name: "Matcha", colors: [0x7b9b68, 0xe3e9be], angle: 180. },
    CoverPreset { id: "aurora", name: "Aurora", colors: [0x48bda4, 0x8b68cc], angle: 135. },
    CoverPreset { id: "violet-mist", name: "Névoa violeta", colors: [0x8c69c8, 0xe1c7ed], angle: 160. },
    CoverPreset { id: "amethyst", name: "Ametista", colors: [0x523886, 0xb885c8], angle: 135. },
    CoverPreset { id: "berry", name: "Frutas vermelhas", colors: [0x76478b, 0xf49cba], angle: 145. },
    CoverPreset { id: "twilight", name: "Crepúsculo", colors: [0x3c5293, 0xc39bd9], angle: 135. },
    CoverPreset { id: "glacier", name: "Geleira", colors: [0x5ba4c7, 0xd2eced], angle: 180. },
    CoverPreset { id: "lagoon", name: "Lagoa", colors: [0x247d93, 0x98ded0], angle: 135. },
    CoverPreset { id: "peach", name: "Pêssego", colors: [0xf3a27b, 0xffe1ba], angle: 160. },
    CoverPreset { id: "honey", name: "Mel", colors: [0xd99d40, 0xf9e7a2], angle: 135. },
    CoverPreset { id: "rosewater", name: "Água de rosas", colors: [0xd798ad, 0xf6ddce], angle: 180. },
    CoverPreset { id: "copper", name: "Cobre", colors: [0x965b51, 0xe4b68e], angle: 145. },
    CoverPreset { id: "moonlight", name: "Luar", colors: [0x5b6174, 0xd4d2e3], angle: 135. },
];

const PREFIX: &str = "sparkpad:cover:";

pub fn stored_value(preset: &CoverPreset) -> String {
    format!("{PREFIX}{}", preset.id)
}

pub fn preset(value: &str) -> Option<&'static CoverPreset> {
    let id = value.strip_prefix(PREFIX)?;
    PRESETS.iter().find(|preset| preset.id == id)
}
