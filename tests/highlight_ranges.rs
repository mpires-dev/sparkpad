// Exercise the exact vendored normalization code without enabling the component
// crate's unrelated GUI dev-dependencies or duplicating the implementation.
use gpui_component::Colorize;
#[path = "../vendor/gpui-component/src/highlighter/highlight_ranges.rs"]
mod highlight_ranges;
