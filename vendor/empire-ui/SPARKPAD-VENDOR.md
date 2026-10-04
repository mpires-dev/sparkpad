# Vendored empire-ui

This library was written for Fennel Motion and copied into Sparkpad so a checkout builds without a separate video-editor workspace.

The component sources and assets are unchanged. `Cargo.toml` replaces inherited workspace package fields and dependencies with their original explicit values: version 0.1.0, Rust 2021, AGPL-3.0-or-later, GPUI 0.2.2, gpui-component 0.5.1 and serde_json 1. The Sparkpad root applies its documented GPUI patches.

Iconoir assets retain their MIT license in `assets/iconoir/LICENSE`. The library retains AGPL-3.0-or-later, as specified in its original workspace.
