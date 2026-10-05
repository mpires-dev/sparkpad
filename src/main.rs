#[cfg(all(feature = "gui", target_os = "macos"))]
mod icon_picker;
#[cfg(all(feature = "gui", target_os = "macos"))]
mod app_theme;
#[cfg(all(feature = "gui", target_os = "macos"))]
mod assets;
#[cfg(all(feature = "gui", target_os = "macos"))]
mod block_editor;
mod blocks;
mod covers;
mod icon_catalog;
mod preferences;
#[cfg(all(feature = "gui", target_os = "macos"))]
mod macos;
mod mcp;
mod mcp_presence;
mod store;
mod note_tree;
#[cfg(all(feature = "gui", target_os = "macos"))]
mod ui;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next();
    if mode.as_deref() == Some("--help") {
        println!("Sparkpad\n  sparkpad           Minimal native macOS notes\n  sparkpad mcp       MCP server over stdio\n  sparkpad db-path   Print SQLite location\n\nOverride storage with SPARKPAD_DB (INTERVIEW_COMPANION_DB is also supported).");
        return Ok(());
    }
    let path = store::default_path()?;
    if mode.as_deref() == Some("db-path") {
        println!("{}", path.display());
        return Ok(());
    }
    let db = store::Store::open(&path)?;
    match mode.as_deref() {
        Some("mcp") => mcp::serve(db),
        None => {
            #[cfg(all(feature = "gui", target_os = "macos"))]
            {
                ui::run(db);
                Ok(())
            }
            #[cfg(not(all(feature = "gui", target_os = "macos")))]
            {
                anyhow::bail!("Build with the gui feature on macOS to open the panel")
            }
        }
        Some(other) => anyhow::bail!("Unknown command: {other}"),
    }
}
