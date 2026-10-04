//! The same searchable, embedded catalog for the desktop picker and headless MCP.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const SVG_PREFIX: &str = "sparkpad:icon:svg:";
pub const IMAGE_PREFIX: &str = "sparkpad:icon:image:";
#[derive(Deserialize, Serialize)]
pub struct Emoji {
    pub value: String, pub name: String, pub search: String,
    pub category: usize, pub tone: usize,
}
pub fn emojis() -> &'static [Emoji] {
    static DATA: OnceLock<Vec<Emoji>> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str::<Vec<Emoji>>(include_str!("../assets/emoji/catalog.json"))
        .expect("embedded emoji catalog").into_iter().map(|mut e| { e.search=fold(&e.search); e }).collect())
}
pub fn fold(value: &str) -> String {
    value.to_lowercase().chars().map(|c| match c {
        'á'|'à'|'â'|'ã'=>'a', 'é'|'ê'=>'e', 'í'=>'i', 'ó'|'ô'|'õ'=>'o', 'ú'|'ü'=>'u', 'ç'=>'c', other=>other,
    }).collect()
}
pub fn icon_paths() -> &'static [String] {
    static DATA: OnceLock<Vec<String>> = OnceLock::new();
    DATA.get_or_init(|| include_str!("../assets/iconoir-catalog.txt").lines().map(str::to_owned).collect())
}
