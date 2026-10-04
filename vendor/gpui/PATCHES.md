# Interview Companion — GPUI 0.2.2

This is the crates.io GPUI 0.2.2 source (Apache-2.0), copied locally to isolate the fix.
The Fennel checkout and Cargo registry are unchanged.

`src/text_system.rs`: resolve each TextRun's font before merging adjacent font runs.
The original condition compared the previous run to the previous resolved font,
so equal foreground/decoration merged different weights/styles. This discarded
bold and italic in native rich text, and could carry italic into normal text.
The change only merges runs whose actual resolved font IDs match.

`tests/rich_text.rs` in the application exercises the real macOS text system
with normal, bold, italic, then normal text of the same color, and checks font IDs.

`WindowTextSystem::new` is public so the regression test can shape text without opening any window or simulating user input.

`src/window.rs`: expose `DispatchEventResult`, matching the existing public `Window::dispatch_event` API. Native editor regressions dispatch events only inside an invisible, unfocused window; they never post system input or move the user’s pointer.
