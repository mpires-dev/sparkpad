# TSX typing optimization and presentation instrumentation

Follow-up measured October 8, 2026 on the same M1 machine and optimized dev build as the [initial report](README.md). Synthetic 5,000-line TSX note, 252,790 bytes, warm SQLite/filesystem caches, no concurrent compilation or sync worker.

## Results

| CPU operation | Previous median | New median | New p95 |
| --- | ---: | ---: | ---: |
| Typing and drawing | 110.43 ms | 10.92 ms | 11.58 ms |
| Open/switch to first scene | 175.79 ms | 76.04 ms | 82.31 ms |
| Scroll and drawing | 3.47 ms | 2.70 ms | 3.31 ms |
| Hover and drawing | 2.71 ms | 1.24 ms | 1.90 ms |

Typing uses 20 inserted characters at the beginning of the large code block, with document updates, undo checkpoint and native CPU layout/scene generation. All 20 samples were below 16.67 ms and 13.33 ms; maximum 11.66 ms. The median improvement is 10.1×. These CPU measurements exclude GPU presentation and cannot establish displayed FPS.

## Implementation

The editor keeps the full incremental Tree-sitter syntax tree, but requests highlight captures and normalized style spans only for the visible byte range. The range is obtained after the input's viewport/layout calculation, in the same frame. Scrolling does not temporarily display stale colors, and multiline syntax retains context from the full document. Source, viewport, language and theme changes invalidate the cache. Returning a block to ordinary text removes its syntax provider.

## Actual FPS validation

The harness now supports opt-in Metal `MTLDrawable.addPresentedHandler` timestamps from `presentedTime`, recorded in a JSONL trace. It separates displayed-frame intervals from CPU timings and GPUI callback cadence. Six visible-window phases cover scrolling and hover on 200/8,000-block notes, followed by scrolling and typing in the 5,000-line TSX note. Each phase warms up for 30 callbacks and records 180 callbacks. Fewer than ten valid presentation intervals rejects an FPS result.

The capture could not run because the host desktop was **locked**, confirmed by `IOConsoleUsers.CGSSessionScreenIsLocked = Yes`. AppKit reported window visibility but occlusionState 8192 without the public Visible bit (2), so GPUI correctly suspended its display link. There were zero presentation timestamps. No FPS number is claimed. No production display-link behavior was changed to bypass occlusion.

To complete validation, unlock the Mac and leave the benchmark window visible, then run:

```sh
SPARKPAD_PERF_CASES=code-5000-lines bash scripts/benchmark-native.sh "$PWD/docs/performance/tsx-fps.json"
```

Read each `display_link[].presented` entry. `valid: true` is required; `effective_fps` there uses actual presented frames. `display_link[].effective_fps` separately describes callback cadence. The external monitor is configured for 75 Hz (13.33 ms budget); the active benchmark display must be checked when interpreting the capture.

Raw results: [tsx-fps.json](tsx-fps.json). The previous [after.json](after.json) is retained as the first optimization pass.

## Regression checks

`cargo test --bin sparkpad`: **59 passed**, including viewport colors compared character by character against a complete fresh TSX parse, multiline comments crossing viewport boundaries, Unicode edits and scrolling between distant ranges. Existing language/theme and UTF-8 incremental edit tests also passed.

`cargo test --test rich_text`: native regression executable completed successfully, including language selection, TSX indentation, code clipboard isolation, undo, themes, visual tables, virtualization and asynchronous selection gestures. `git diff --check` and benchmark script syntax checks passed.
