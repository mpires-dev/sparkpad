//! Portable images shared by the native editor, Markdown and the sync worker.
use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub fn name(url: &str) -> Option<&str> {
    let name = url.strip_prefix("asset:")?;
    let (h, e) = name.split_once('.')?;
    (h.len() == 64
        && h.bytes().all(|b| b.is_ascii_hexdigit())
        && ["png", "jpg", "jpeg", "gif", "webp", "svg"].contains(&e))
    .then_some(name)
}
pub fn references(markdown: &str) -> Vec<String> {
    markdown
        .split("asset:")
        .skip(1)
        .filter_map(|s| {
            let end = s
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '.')
                .unwrap_or(s.len());
            let candidate = &s[..end];
            name(&format!("asset:{candidate}")).map(|_| candidate.to_owned())
        })
        .collect()
}
pub fn import(root: &Path, source: &Path) -> Result<String> {
    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(source)?
        .take(10_000_001)
        .read_to_end(&mut bytes)?;
    store(root, &ext, &bytes)
}
pub fn store(root: &Path, ext: &str, bytes: &[u8]) -> Result<String> {
    if bytes.len() > 10_000_000 {
        bail!("A imagem deve ter no máximo 10 MB.");
    }
    let valid = match ext {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => bytes.starts_with(&[255, 216, 255]),
        "gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        "svg" => std::str::from_utf8(bytes).is_ok_and(|s| s.contains("<svg")),
        _ => false,
    };
    if !valid {
        bail!("Escolha uma imagem PNG, JPG, GIF, WebP ou SVG válida.");
    }
    std::fs::create_dir_all(root)?;
    let name = format!("{:x}.{ext}", Sha256::digest(bytes));
    let path = root.join(&name);
    if !path.exists() {
        let staging = root.join(format!(".image-{}", uuid::Uuid::new_v4()));
        std::fs::write(&staging, bytes)?;
        std::fs::rename(staging, path)?;
    }
    Ok(format!("asset:{name}"))
}
pub fn local(root: &Path, url: &str) -> Option<PathBuf> {
    Some(root.join(name(url)?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_images_deduplicate_and_reject_bad_paths() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let url = store(&root, "svg", svg).unwrap();
        assert_eq!(url, store(&root, "svg", svg).unwrap());
        assert_eq!(
            references(&format!("![Logo]({url})")),
            vec![name(&url).unwrap()]
        );
        assert!(name("asset:../../private.png").is_none());
        assert!(store(&root, "png", b"bad").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
