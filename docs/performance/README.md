# Native frontend performance

Measured on October 7–8, 2026. All times below are milliseconds unless stated otherwise.

**Follow-up:** [TSX typing optimization and FPS instrumentation](tsx-fps.md) reduces the 5,000-line TSX typing median from 110.43 ms to 10.92 ms. Actual FPS capture requires unlocking the host desktop. The tables below retain the initial optimization pass.

## Environment and scope

- Apple M1 MacBook Pro (MacBookPro17,1), 8 CPU cores, 16 GB RAM.
- Native Rust/GPUI production `Panel`, 1100 × 760 logical-pixel window, default 22px document font.
- Build matches the current installer: dev profile, application optimization level 3, dependencies level 2, debug symbols disabled, incremental compilation disabled. This is not a release/LTO build.
- Seven deterministic Markdown fixtures, including Unicode, headings, bold text, links, a visual GFM table and highlighted TSX. The largest fixture is 1,747,544 bytes, below the current 2,000,000-byte note limit.
- Temporary SQLite database, no user notes and no sync worker. All optimized samples were collected without a concurrent compiler.
- Baseline samples for the largest documents/code overlapped some compilation. They are local diagnostic measurements rather than controlled laboratory results. Smaller baseline cases completed before compilation. The differences in large-document rendering are substantial, but small differences should not be treated as statistically significant.

## Opening and switching documents

`transition_to_scene` measures selecting a saved note, reading SQLite, parsing, constructing native editor fields and drawing its first CPU scene. Each case has seven samples; between loads, the harness selects the small control note. The 20-block case therefore reloads the same control note. OS filesystem caches are warm. These are not cold-disk or GPU presentation timings.

| Document | Before median | After median | After p95 | Median speedup |
| --- | ---: | ---: | ---: | ---: |
| 20 blocks · 4.2 KB | 6.06 | 5.74 | 6.10 | 1.1× |
| 200 blocks · 43 KB | 62.79 | 8.66 | 9.12 | 7.3× |
| 2,000 blocks · 436 KB | 1,245.21 | 60.86 | 61.20 | 20.5× |
| 8,000 blocks · 1.75 MB | 16,127.75 | 309.20 | 315.12 | 52.2× |
| 400-row table · 1,203 cells | 3,710.68 | 237.07 | 374.35 | 15.7× |
| TSX · 5,000 lines · 253 KB | 34,719.02 | 175.79 | 183.63 | 197.5× |
| Single paragraph · 530 KB | 81.61 | 59.33 | 65.87 | 1.4× |

The synthetic startup run reached its first CPU scene in 313.33 ms from the harness entry point; window/Panel creation and its first draw accounted for 151.68 ms. The first number includes font registration, fixture generation and database setup. It is not an OS process-launch or cold-start measurement of the installed app.

The large note now instantiates 36 native block fields instead of 8000, while retaining all 8,000 blocks and their Markdown. Rendered row bounds in this sampling scenario dropped from 8000 to 10.

## Scrolling and hover

Each operation measures actual native event dispatch followed by `Window::draw`, including layout and CPU scene generation. Scrolling uses 50 wheel events of ±120px. Hover uses 50 mouse moves across document rows. These measurements exclude waiting for GPU completion and physical display presentation.

| Document | Scroll before median | Scroll after median / p95 | Hover before median | Hover after median / p95 |
| --- | ---: | ---: | ---: | ---: |
| 20 blocks · 4.2 KB | 4.96 | 4.83 / 5.89 | 4.74 | 4.72 / 6.00 |
| 200 blocks · 43 KB | 39.66 | 4.35 / 5.76 | 39.62 | 3.04 / 3.86 |
| 2,000 blocks · 436 KB | 428.13 | 4.50 / 5.80 | 428.83 | 2.93 / 3.57 |
| 8,000 blocks · 1.75 MB | 2,134.60 | 5.64 / 7.69 | 2,154.13 | 3.55 / 4.28 |
| 400-row table · 1,203 cells | 120.14 | 9.44 / 10.71 | 120.93 | 6.25 / 7.39 |
| TSX · 5,000 lines · 253 KB | 450.03 | 3.47 / 4.46 | 739.18 | 2.71 / 3.37 |
| Single paragraph · 530 KB | 17.68 | 13.40 / 14.62 | 16.99 | 13.25 / 14.49 |

A 60Hz frame has a total budget of 16.67ms; the configured external display is 75Hz, with a 13.33ms budget. CPU work alone being below either threshold does not establish delivered FPS.

**Actual FPS is unverified.** The native display-link experiment failed to complete within its 20-second watchdog, despite explicit AppKit activation and window display. The initial baseline cadence experiment also had a harness error. Both failures are recorded in the raw JSON. No reciprocal CPU timing is presented as measured FPS. A foreground, visible-window capture with GPU presentation instrumentation remains necessary to validate compositor/GPU smoothness.

## Typing

Twenty inserted characters per fixture, including native input updates, document changes, undo checkpoint, syntax/style maintenance and CPU draw. The table edits the first body cell; text/code edit the beginning of the block. The first sample includes acquiring focus.

| Document | Before median | After median | After p95 | Samples over 16.67ms |
| --- | ---: | ---: | ---: | ---: |
| 20 blocks · 4.2 KB | 5.12 | 5.39 | 5.91 | 0/20 |
| 200 blocks · 43 KB | 41.66 | 3.70 | 4.34 | 0/20 |
| 2,000 blocks · 436 KB | 506.95 | 4.57 | 5.28 | 0/20 |
| 8,000 blocks · 1.75 MB | 4285.28 | 8.26 | 8.92 | 0/20 |
| 400-row table · 1,203 cells | Not sampled | 7.30 | 8.12 | 0/20 |
| TSX · 5,000 lines · 253 KB | Not sampled | 110.43 | 116.14 | 20/20 |
| Single paragraph · 530 KB | Not sampled | 34.39 | 38.20 | 20/20 |

The additional typing probes exposed remaining costs that opening/scroll tests do not reveal: full-document syntax capture/style generation still costs roughly 110ms when editing a 5,000-line TSX block, and editing one 530KB paragraph takes roughly 34ms. These cases are not yet within a 60Hz interaction budget.

## Parsing, storage and memory

| Document | Parse median | Warm SQLite read median |
| --- | ---: | ---: |
| 20 blocks · 4.2 KB | 0.455 | 0.005 |
| 200 blocks · 43 KB | 5.284 | 0.012 |
| 2,000 blocks · 436 KB | 56.373 | 0.067 |
| 8,000 blocks · 1.75 MB | 302.436 | 0.246 |
| 400-row table · 1,203 cells | 8.759 | 0.007 |
| TSX · 5,000 lines · 253 KB | 10.058 | 0.050 |
| Single paragraph · 530 KB | 34.943 | 0.097 |

The 8,000-block opening time is now dominated by Markdown parsing rather than SQLite. Each parse has seven samples; each warm SQLite read has 21. With seven samples, nearest-rank p95 is the maximum, not a stable tail-latency estimate. Raw JSON includes means, maxima and threshold counts.

| Whole stress process | Before | After |
| --- | ---: | ---: |
| Maximum resident memory (`time -l`) | 2179 MiB | 929 MiB |
| Peak memory footprint (`time -l`) | 3518 MiB | 963 MiB |

A separate sanity snapshot of the rebuilt installed app, about 48 seconds after opening the existing workspace, showed roughly **150 MiB RSS and 1% CPU** (`ps`). This is one observation, not a controlled long-duration idle benchmark.

These are process peaks across repeated fixture loads, parsing, text layout caches and undo history, not idle application memory. The after run also includes typing in code, tables and the giant paragraph; those extra probes were absent from the baseline.

## Implemented optimizations

1. Virtualize documents above 80 blocks: render nearby rows with overscan, reuse measured/estimated heights and instantiate unvisited native fields only when needed. Focus and selection endpoints remain pinned. The complete document stays in the model.
2. Virtualize large visual tables, cache their parsed model and row heights, and update one edited cell and subsequent byte ranges without reparsing all cells. Short rows, empty/whitespace cells and pasted line breaks retain parser fallbacks.
3. Cache serialized Markdown so ordinary render comparisons do not repeatedly serialize the entire document.
4. Notify inputs and invalidate rich wrapping only when highlight/font spans change. Reuse rich wrapping between frames and skip the second shaping pass for color-only spans.
5. Clip tall auto-growing input fields to the ancestor viewport and search sorted highlight spans from the visible range.
6. Replace quadratic highlight normalization with an endpoint sweep that preserves capture precedence.
7. Describe the smallest UTF-8-safe edit to Tree-sitter so unchanged subtrees can be reused. Whole-range style capture is still synchronous.

## Remaining optimization opportunities

- Move large-code highlight queries/style generation off the input path, coalesce updates and apply only the latest generation. Incremental parsing alone is insufficient for the 5,000-line edit probe.
- Avoid shaping/copying the complete logical paragraph when only a few wrapped lines are visible, and incrementally update giant-paragraph wrapping after edits.
- Replace whole-document undo snapshots with block-level edits where possible.
- Bound retained native field caches during a full traversal of very large notes. The current cache retains fields already visited; this run scrolls a limited distance rather than traversing every row.
- Lazily create table-cell entities: the 400-row table still has 1,203 input states even though only nearby rows are rendered.
- Parse large documents outside the UI thread or cache parsed documents for fast repeated switches, with revision-based invalidation.
- Complete GPU/compositor cadence capture. Also test image-heavy notes, live server sync and long-duration scrolling separately; those are outside this synthetic frontend benchmark.

## Reproduce

Run from an unlocked, logged-in macOS desktop, with other heavy processes idle:

```bash
scripts/benchmark-native.sh "$PWD/docs/performance/latest.json"
```

The script compiles the native test executable before measuring. `SPARKPAD_PERF_CASES=blocks-200,blocks-8000` restricts CPU fixtures; display-link probes still use the normal and giant documents. The harness saves CPU results before starting cadence probes and marks cadence failures explicitly.

Raw measurements: [before.json](before.json), [after.json](after.json). The baseline represents the native implementation immediately before this optimization pass; the working tree already contained earlier image/table/sync changes, so it is not an identified Git release baseline.

## Regression validation

Native editor regressions cover selection, formatting, rich clipboard, code controls, Unicode, undo/redo, visual tables, collaboration, image resizing and the new virtualized document path. Added probes assert complete Markdown preservation, bounded initial fields/rows, scrolling, distant focus/editing, font reflow and editing an offscreen table cell. Unit regressions compare highlight sweep precedence with the prior algorithm and check UTF-8 edit boundaries and incremental table byte ranges. Validation completed successfully:

- `cargo test --bin sparkpad`: **58 passed**.
- `cargo test --test highlight_ranges`: **2 passed**, including 256 deterministic overlap/precedence cases checked against the previous normalization algorithm.
- `cargo test --test rich_text`: native GPUI regression executable completed successfully, including virtualization, rich clipboard, visual tables, Unicode editing, selection, undo/redo, code controls, collaboration and image resizing.
- `git diff --check` and `bash -n scripts/benchmark-native.sh`: passed.

The native test target uses a custom executable (`harness = false`), so its successful native assertions are reported by scenario rather than by a Rust test-case counter.
