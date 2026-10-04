//! Public preference names and defaults shared by MCP and the UI synchronizer.
use crate::store::Store;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Preferences {
    pub opacity: f32, pub content_width: f32, pub font_size: f32,
    pub sidebar_width: f32, pub sidebar: bool, pub always_on_top: bool,
    pub theme: String, pub content_font: String,
}
impl Preferences {
    pub fn read(db: &Store) -> Result<Self> {
        let number=|key,default,min,max| -> Result<f32> {
            Ok(db.setting(key)?.and_then(|v| v.parse::<f32>().ok()).filter(|v| v.is_finite()).unwrap_or(default).clamp(min,max))
        };
        let theme=db.setting("theme")?.filter(|s| s=="light").unwrap_or("dark".into());
        let content_font=db.setting("content_font")?.filter(|s| ["sans","serif","mono"].contains(&s.as_str())).unwrap_or("sans".into());
        Ok(Self {opacity:number("opacity",94.,25.,100.)?,content_width:number("content_width",100.,40.,100.)?,
            font_size:number("font_size",22.,14.,40.)?,sidebar_width:number("sidebar_width",206.,180.,420.)?,
            sidebar:db.setting("sidebar")?.as_deref()!=Some("false"),
            always_on_top:db.setting("always_on_top")?.as_deref()!=Some("false"),theme,content_font})
    }
    pub fn update(db: &Store, value: &Value) -> Result<Self> {
        let map=value.as_object().ok_or_else(|| anyhow::anyhow!("preferences must be an object"))?;
        if map.is_empty() {bail!("Provide at least one preference")}
        let mut values=Vec::new();
        for (key,value) in map {
            let bounds=match key.as_str() {
                "opacity"=>Some((25.,100.)),"content_width"=>Some((40.,100.)),
                "font_size"=>Some((14.,40.)),"sidebar_width"=>Some((180.,420.)),_=>None,
            };
            let stored=if let Some((min,max))=bounds {
                let n=value.as_f64().filter(|n| n.is_finite() && *n>=min && *n<=max)
                    .ok_or_else(|| anyhow::anyhow!("{key} must be a number between {min} and {max}"))?;
                n.to_string()
            } else {match key.as_str() {
                "sidebar"|"always_on_top"=>value.as_bool().ok_or_else(|| anyhow::anyhow!("{key} must be a boolean"))?.to_string(),
                "theme"|"content_font"=>{
                    let allowed:&[&str]=if key=="theme" {&["dark","light"]} else {&["sans","serif","mono"]};
                    value.as_str().filter(|v| allowed.contains(v)).ok_or_else(|| anyhow::anyhow!("Invalid {key}; choose from {allowed:?}"))?.to_owned()
                },
                _=>bail!("Unknown preference: {key}"),
            }};
            values.push((key.as_str(),stored));
        }
        db.set_settings(&values)?;
        Self::read(db)
    }
}
