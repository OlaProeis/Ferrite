# Changelog

All notable changes to Ferrite will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — v0.3.1

Work in progress on `master`; not yet tagged.

### Added

- **CSV rendered cell editing (MVP)** — Double-click to edit cells in the CSV/TSV Rendered view for small files; RFC 4180 serialization, undo integration, keyboard commit/cancel. Large lazy-parsed files still prompt to edit in Raw view. See [`docs/technical/viewers/csv-viewer.md`](docs/technical/viewers/csv-viewer.md).
- **Video embed parsing** — Detect `{{video URL}}` and bare YouTube URLs in their own paragraph; `VideoEmbed` AST node with allowlist and round-trip `source_text`. Playback rendering (wry / thumbnail fallback) is follow-up work. See [`docs/technical/markdown/video-embed-parsing.md`](docs/technical/markdown/video-embed-parsing.md).

### Fixed

- **Windows single-instance foreground** ([#147](https://github.com/OlaProeis/Ferrite/issues/147)) — Opening a file from Explorer into an already-running Ferrite window now raises the existing window more reliably. The secondary process reads `instance.pid` and calls `AllowSetForegroundWindow(primary_pid)` before forwarding paths; the primary also sends `ViewportCommand::RequestUserAttention(Informational)` when focus alone is insufficient. Thanks [@Star-sumi](https://github.com/Star-sumi) ([PR #148](https://github.com/OlaProeis/Ferrite/pull/148)). See [`docs/technical/platform/single-instance.md`](docs/technical/platform/single-instance.md).

## [0.3.0] - 2026-05-22

Platform refresh: export, code run, Mermaid first wave, **rendered edit session** (WYSIWYG block switching), accent, quick-note workflow, Phosphor icons — on **eframe / egui 0.34.2** with **Rust 1.92** MSRV. See [`docs/technical/platform/eframe-egui-034-upgrade.md`](docs/technical/platform/eframe-egui-034-upgrade.md) and the [v0.3.0 regression matrix](docs/technical/platform/v0.3.0-regression-matrix.md) (includes 0.34 delta, Task 89; rendered editing RS-1…RS-7, Tasks 94–105).

### Added

#### Platform (egui 0.34 / eframe 0.34 — Task 89)
- **eframe / egui stack upgraded to 0.34.2** — Platform bump (0.28 → 0.31 → 0.34 for v0.3.0). **skrifa + vello_cpu** default text backend; **Phosphor 0.12**; Windows **glow** renderer retained. Migration: viewport rects (`screen_rect` → `viewport_rect` / `content_rect`), **Popup** API for menus/dropdowns, **Tooltip** API, ScrollArea edge-fade disabled globally for visual parity. See `docs/technical/platform/eframe-egui-034-upgrade.md`.
- **HarfRust validation under egui 0.34** — Complex-script cursor/selection via cluster shaping; `shape_line_clusters`, `validate_cluster_byte_ranges`, 32 shaping unit tests. Word wrap + complex script still uses egui galley only (documented limit).
- **Mutex / deadlock audit** — Cross-thread paths verified (single-instance, terminal, code-run workers, global caches); no `egui::Mutex` in Ferrite code.
- **MSRV raised to Rust 1.92** — `rust-toolchain.toml`, `package.rust-version`, CI workflows read the pinned toolchain.
- **v0.3.0 regression matrix updated** — 0.34-specific delta checks added to `docs/technical/platform/v0.3.0-regression-matrix.md`; Windows manual pass recorded (89.3–89.7).

#### Platform (egui 0.31 / eframe 0.31)
- **eframe / egui stack upgraded to 0.31.x** (Task 57) — Dependency bump from 0.28 with API migrations across themes, fonts, editor, markdown, terminal, and UI. See `docs/technical/platform/eframe-egui-031-upgrade.md`. Intended to address platform input/windowing issues tied to older winit (e.g. [#106](https://github.com/OlaProeis/Ferrite/issues/106), [#111](https://github.com/OlaProeis/Ferrite/issues/111)); confirm on target OS builds before closing.
- **Cross-platform regression matrix for the egui 0.31 upgrade** (Task 58) — `docs/technical/platform/v0.3.0-regression-matrix.md` defines stable test IDs (LAU/FILE/KBD/IME/TRM/DLG/WIN/MMD/FNT/THM) covering every egui 0.31 risk surface across Win11 / macOS-AS / macOS-Intel / Linux-X11 / Linux-Wayland with explicit release-gate criteria. Executed on Win10 as a Win11 proxy; macOS and Linux rows deferred to CI / external contributors until filled in.

#### Export — PDF & HTML
- **Native PDF export** (Tasks 60–61) — File → Export → PDF… with options for page size, margins, optional page break before H1, and link annotations. Implemented with **krilla** + **krilla-svg** (see `docs/technical/planning/pdf-export-pipeline.md`, `docs/technical/viewers/pdf-export.md`).
- **Print preview** (Task 62) — Reuses the same PDF render path as export; writes a temp PDF and opens it in the in-app **PdfViewer** tab (`docs/technical/viewers/print-preview.md`).
- **Themed HTML export** (Task 63) — Stronger parity with in-app rendering: theme-aware CSS, Mermaid as inline SVG, syntect-highlighted code blocks, export options dialog (`docs/technical/viewers/themed-html-export.md`, `docs/technical/viewers/document-export.md`).

#### Executable fenced code blocks
- **Code execution settings** (Task 64) — Editor → Code execution: master enable, shell/Python toggles, timeout (default 30s, max 300s). See `docs/technical/config/code-execution-settings.md`.
- **Run on fenced blocks** (Tasks 65–66) — **Run** control in rendered/split preview for supported languages; background worker; inline ANSI-colored output (`ansi_render` / vte-style SGR); exit status; insert output as fenced block. See `docs/technical/markdown/code-block-run.md`.
- **Timeout and Stop** (Task 67) — Hard timeout and user **Stop**; clear labels for timeout vs cancel (`docs/technical/markdown/code-block-cancellation.md`).
- **First-run consent** (Task 68) — Modal before first **Run**; queue payload until user accepts; can be skipped via Settings (`docs/technical/markdown/code-execution-consent-dialog.md`).

#### Mermaid — first wave ([#4](https://github.com/OlaProeis/Ferrite/issues/4))
- **Insert → Mermaid…** (Tasks 69–70) — Toolbar templates per diagram type; **About/Help** syntax section and snippets aligned with insert (`docs/technical/markdown/mermaid-insert-toolbar.md`, `docs/technical/mermaid/mermaid-syntax-help.md`).
- **Inline validation** (Task 71) — Parse-time errors: warning header in preview, last-good diagram fallback, squiggles in raw editor (`docs/technical/mermaid/mermaid-inline-validation.md`).
- **Flowchart shapes & style** (Task 72) — Extra shapes and `style` / classDef plumbing (`docs/technical/mermaid/flowchart-shapes-and-style.md`).
- **State diagrams: fork/join & history** (Task 73) — `<<fork>>` / `<<join>>` bars and `[H]` / `[H*]` glyphs (`docs/technical/mermaid/state-pseudostates-fork-join-history.md`).

#### Mermaid — flowchart edge routing & layout polish ([#83](https://github.com/OlaProeis/Ferrite/issues/83), FC-83a)
The native flowchart renderer (no mmdr / SVG fallback) gained several rendering passes that close the bulk of the FC-83a regression versus Mermaid Live. See `docs/technical/mermaid/flowchart-edge-obstacle-routing.md` and `docs/technical/mermaid/flowchart-layout-algorithm.md`.
- **Edge–node obstacle avoidance** — Forward edges that would otherwise pass through an unrelated node now detour around the obstacle. New `flowchart/utils.rs` helpers (`segment_intersects_rect`, `path_intersects_any`, `bezier_intersects_any`, `collect_node_obstacles`, `union_rect_bounds`) feed a multi-strategy router in `flowchart/render/edges.rs` (`route_forward_edge` → `try_orthogonal_route` → `route_via_side_corridor`). Unit-tested in `obstacle_tests`.
- **Painter sizing from actual content** — `render_flowchart` now allocates from real node/subgraph bounds via `layout_content_size` and adds horizontal padding on the side(s) where back-edges actually loop (`back_edge_horizontal_padding`; `BACK_EDGE_LOOP_MARGIN + NODE_OBSTACLE_PADDING + (lanes-1)·BACK_EDGE_LANE_SPACING`), so feedback loops are not clipped without leaving a spurious left gutter when all loops run on the right (FC-83a).
- **TD/BT layer centering** — Sugiyama cross-axis placement now centers each layer within the widest layer (`max_cross_size`), matching LR/RL behaviour, instead of centering in the full container `available_width` (which left a large empty strip on the left when the painter only allocated actual node bounds). Post-layout bounds are normalized so content starts at the layout margin.
- **Back-edge side channels** — Replaced the fixed ±40 px cubic-bezier loops with stable orthogonal routes that hug the graph bounding box at `BACK_EDGE_LOOP_MARGIN = 24 px`. Loop side is picked from source position; horizontal exits detour above/below same-row nodes when needed.
- **Parallel back-edge lanes** — `compute_back_edge_lanes` assigns distinct `BackEdgeLane { side_sign, lane_index }` slots so multiple loops targeting the same node on the same side stay visually separate (FC-83a: `E → B` and `F → B` no longer merge onto one bus). Lane spacing `BACK_EDGE_LANE_SPACING = 36 px`.
- **Inner back-edge direct path** — Lane-0 back-edges (`try_inner_back_edge_direct_path` + `inner_back_edge_path_candidates` + `try_inner_back_edge_tight_loop`) exit the source at the top-outer corner, rise vertically along the source's outer edge, and enter the target at its side-centre — instead of going horizontal-first across the trunk. Stepped-outward variants clear sibling branches; tight-loop fallback stays just outside the source. Pinned by `fc_83a_inner_e_to_b_goes_up_first`.
- **Branch parent snap (alone-on-layer)** — `align_branch_nodes_to_children` in `flowchart/layout/sugiyama.rs` shifts decision nodes with 2+ forward children onto the cross-axis barycenter of those children, matching dagre/Mermaid.js behaviour. The shift is gated to nodes that are alone on their layer so it cannot pull a branch parent onto a sibling (regression-tested via FC-83a `decide`).
- **Same-layer sibling spacing safety net** — New `resolve_layer_overlaps` pass walks every layer in cross-axis order and enforces `node_spacing.x` (or `.y` for LR/RL) between adjacent siblings via iterative half-and-half push. Fixes the coffee-machine repro in `test_md/test_flowcharts.md` (previously `C` overlapped `H` by ~68 px and `D` overlapped `G` by ~79 px at 800 px width) and protects against future regressions from subgraph clustering or custom node sizing. Pinned by `test_layout_coffee_machine_all_nodes` (per-layer no-overlap assertion).

#### Appearance & hub
- **User-configurable Ferrite accent** — Color picker in **Settings → Appearance** and **Welcome**; drives headings, selection, tabs, view mode segment, productivity hub chrome, status bar (LSP line, git branch); markdown links keep the standard link color. See `docs/technical/ui/theme-system.md`.
- **Productivity Hub polish** — Card layout, Pomodoro emphasis, floating **×** re-docks to sidebar (does not hide the hub), stable docked widths via clipped child UI, detached window respects user resize without content-driven growth or a fixed 560px width floor, scrollbar margin vs resize handle (`docs/technical/productivity/productivity-panel.md`).

#### Distribution & fonts
- **macOS Gatekeeper troubleshooting** ([#130](https://github.com/OlaProeis/Ferrite/issues/130)) — Unsigned CI `.app` on macOS 15.x: docs at `docs/install/macos.md`, index + release checklist links; quarantine / Open Anyway workarounds documented.
- **Custom font picker stability (Intel macOS)** ([#133](https://github.com/OlaProeis/Ferrite/issues/133)) — Deferred font load until explicit combo selection to avoid spurious error toast (`docs/technical/fonts/custom-font-picker-deferred-load.md`).

#### Files & session
- **Quick note workflow** (on by default, **Settings → Files**) — Pathless “scratch” tabs no longer block quit when modified; closing a modified untitled tab still prompts (Save / Don't save / Cancel); empty untitled tabs close silently; double-click an untitled tab to set a display name. Turn off in settings to restore classic save prompts for pathless tabs. Unsaved text persists via session recovery when **Restore previous session on startup** is enabled. Clean exit keeps `recovery/` content (only the crash snapshot file is cleared). See `docs/technical/config/quick-note-workflow.md`.
- **Workspace file index** — **Quick File Switcher** (Ctrl+P) and **Search in Files** (Ctrl+Shift+F) now search the **entire workspace** on a background thread, not only folders expanded in the sidebar. Incremental file batches and an animated progress bar (“Indexing… N files found”) on large trees; index rebuilds when files are created, deleted, or renamed. Same walk rules as the file tree (`node_modules`, `.git`, etc.). See [`docs/technical/files/workspace-file-index.md`](docs/technical/files/workspace-file-index.md).

#### Editor — scroll sync
- **Split-view live sync** — In markdown **Split**, optional alignment between the Ferrite raw editor and rendered preview while scrolling. Uses **source line + fraction** anchors (not scroll percentage) so code blocks and Mermaid stay aligned across different preview heights. After ~120 ms idle, one snap updates the other pane; **top/bottom** (within 5 px) snap to document edges. Detects wheel, **scrollbar drag**, and keyboard via offset delta. Controls on the semantic minimap footer: **Sync** (master, persists to settings) and **2-way** (default on: preview scroll also moves raw; off = raw → preview only). Settings: `sync_scroll_enabled` (default off), `sync_scroll_bidirectional`. See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md).
- **Mode-toggle sync (Ctrl+E)** — With sync enabled, switching **Raw ↔ Rendered** preserves position via the same hybrid top/bottom/line-mapping strategy (interpolation within large blocks). Split mode continues to use real-time sync instead of toggle-time conversion.
- **Rendered height fixup** — When viewport culling remeasures block heights (e.g. code blocks), scroll ratio is preserved on the next frame after scrolling stops to avoid layout “nudges” unrelated to sync.

#### Rendered edit session (Tasks 94–105)
Consolidated WYSIWYG editing architecture replacing per-widget focus/defer hacks. One block (heading, paragraph, list item, formatted block, or table cell) is active at a time; switching commits the previous buffer and opens the target in **one click**. Hub: [`docs/technical/markdown/rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md); PRD: [`docs/ai-workflow/prds/prd-rendered-edit-session.md`](docs/ai-workflow/prds/prd-rendered-edit-session.md).
- **`source_epoch` widget identity** (Tasks 95, 97) — Per-tab counter bumped on external invalidation (raw edits, file reload, undo/redo, find/replace, split raw pane) but **not** on rendered block commits. Rendered TextEdit ids scoped by `editor_id` + `source_epoch` instead of `content_hash`; `content_hash` retained for viewport culling only. See [`rendered-widget-identity.md`](docs/technical/markdown/rendered-widget-identity.md), `Tab::source_epoch()` in `src/state.rs`.
- **`RenderedEditSession` coordinator** (Tasks 96–98) — `BlockRef`, tab-scoped session state machine (`src/markdown/rendered_session.rs`), `switch_to_ui` closes the previous block (`SaveIfDirty`) and queues `PendingActivation` for focus and cursor placement. Headings wired first; legacy heading-specific `rendered_focus` paths removed.
- **Paragraphs & list items** (Task 99) — Plain blocks share session buffers and the unified commit path; one-click cross-block switching among headings, paragraphs, and simple list items. Session buffers invalidate on `source_epoch` bump.
- **Formatted click-to-edit** (Task 100) — Formatted paragraphs and list items use `BlockEditState.formatted_editing` (styled display ↔ raw TextEdit); click enters edit with galley-based cursor mapping; Escape discards. Removed `FormattedItemEditState` and broken deferred blur logic. See [`rendered-edit-session-formatted.md`](docs/technical/markdown/rendered-edit-session-formatted.md).
- **Tables on session model** (Task 101) — `BlockRef::TableCell` participates in block switching; `signal_table_force_commit` commits the full table when leaving for a non-table block; Tab / Shift+Tab within a table preserves deferred table commit. See [`rendered-edit-session-tables.md`](docs/technical/markdown/rendered-edit-session-tables.md).
- **Split-view parity** (Task 102) — Rendered pane in split shares the same session and `source_epoch` as rendered-only mode (`rendered_editor_id(tab.id)`); raw-pane edits bump epoch and invalidate session buffers (RS-6). See [`rendered-edit-session-split-view.md`](docs/technical/markdown/rendered-edit-session-split-view.md).
- **Block-commit undo** (Task 103) — Keystrokes inside an active rendered block stay off the undo stack until close/switch; one logical Ctrl+Z step per block commit via `src/markdown/rendered_commit_undo.rs`. See [`rendered-edit-session-undo.md`](docs/technical/markdown/rendered-edit-session-undo.md).
- **Legacy cleanup & docs** (Tasks 104–105) — Removed `rendered_focus.rs` and remaining per-widget focus/commit dead paths. RS-1…RS-7 and TBLE-1…TBLE-3 acceptance tests documented in [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12.

#### Localization
- **Spanish UI language** — **Español** added to the Settings / Welcome language selector (`locales/es.yaml`); system locale `es` / `es-*` detection wired through `Language::Spanish`.

#### UI iconography — Phosphor Icons
- **Phosphor icon font** — Integrated [`egui-phosphor`](https://github.com/alexbenis/egui-phosphor) **0.12.0** (egui 0.34). Regular-weight glyphs registered at startup; helpers `phosphor_font()` / `phosphor_rich_text()` and a central mapping module (`src/ui/phosphor_icons.rs`).
- **App chrome** — Ribbon, format toolbar, outline & productivity panels, terminal, settings & about, command palette, quick switcher, file tree, status bar, dialogs, title bar, tab close, theme toggle, and recovery UI now use Phosphor instead of emoji or mixed Unicode symbols.
- **Preview & viewers** — Markdown preview widgets (table alignment, list remove, code **Run** / **Stop** / status, mermaid diagram-type icons & warnings), JSON/YAML/TOML tree viewer, CSV row-count banner, Gantt done checkmarks, ER diagram PK/FK markers, and editor gutter fold carets.
- **Locale cleanup** — Removed duplicate emoji from UI strings in `en`, `de`, `es`, `ja`, and `zh_Hans` where icons are drawn separately (tree viewer toolbar, outline stats heading, CSV header labels).
- *Unchanged by design:* Git file-status badges, markdown callout body icons (Note/Tip/Warning content), and **R/S/V** view-mode segment letters.

### Changed

- **Build warnings — four-phase cleanup (Tasks 90–93)** — After the egui 0.34 upgrade, `cargo build` emitted ~268 warnings (unused imports, unused locals, deprecated egui 0.34 APIs, and `dead_code`). Four hygiene passes brought the count to **zero** with no intended behavior changes: (1) **Task 90** — `cargo fix` and manual unused-import cleanup in `app/`, `editor/ferrite/mod.rs`, `ui/mod.rs` (~55); (2) **Task 91** — unused variables, parameters, and unnecessary `mut` in central panel, title bar, Vim view, markdown widgets, terminal/productivity panels (~9); (3) **Task 92** — egui 0.34 deprecated API migration (`Ui::close_menu` → `close`, `child_ui` → `new_child`, `ComboBox::from_id_salt`, `Button::selectable`, panel `show_inside`, `Frame::corner_radius`, etc.) across terminal panel, central panel, settings, and app shell (~152); (4) **Task 93** — `dead_code` audit: surgical removals, wiring fixes, and item-level `#[allow(dead_code)]` for intentional public API (~49). Triage policy documented in [`CONTRIBUTING.md`](CONTRIBUTING.md) § `dead_code` warnings.
- **Minimum Rust version: 1.92** — Required by the egui 0.34 stack; use `rustup toolchain install 1.92.0` or rely on `rust-toolchain.toml`.
- **Undo granularity in raw mode** — Removed 500 ms time-based merging in `EditHistory`. Each `record_operations` call (typically one per dirty editor frame while typing) is now its own undo step, so Ctrl+Z steps back incrementally instead of reverting an entire fast-typing burst in one action. Replace edits (delete + insert from the same diff) still undo atomically. See `docs/technical/editor/undo-redo.md` and `docs/technical/editor/edit-history.md`.
- **Rendered mode block-commit undo** (Task 103) — Rendered WYSIWYG edits batch in session buffers until block close/switch; each commit produces **one** undo step (not one per keystroke). See [`rendered-edit-session-undo.md`](docs/technical/markdown/rendered-edit-session-undo.md).
- **Main ribbon toolbar always icon-only** — Removed the left collapse/expand control and section labels ("File", "Edit", "Tools", structured-data type name, Export text). The toolbar is now a fixed compact icon bar (28px tall); Save and Export dropdowns still show full labels inside their menus. Tooltips and keyboard shortcuts are unchanged.
- **UI icons: emoji → Phosphor** — Replaced inconsistent emoji and one-off Unicode symbols across Ferrite's chrome and rendered-view controls with the Phosphor regular set for sharper, theme-consistent glyphs at all DPI scales. Mixed icon+text labels use separate Phosphor and proportional labels (or painter dual-font rendering) so glyphs render reliably; window titles stay text-only.

### Fixed

- **CRITICAL: Smart-paste of mixed-script text crashes Ferrite (`STATUS_STACK_BUFFER_OVERRUN` / `0xc0000409`)** — Pasting any text whose first colon is followed by a multi-byte UTF-8 codepoint (e.g. `Hebrew: שלום עולם`, `Bengali: আমি বাংলায়`, `Hindi: नमस्ते`, `note: 你好`, `emoji: 👨‍👩‍👧`) aborted the process in release builds (`panic = "abort"`). Root cause was `FerriteApp::is_url` in `src/app/input_handling.rs` doing `&s[colon_pos..colon_pos + 3]` to look for `://`, which panicked when `colon_pos + 3` landed on a non-char-boundary inside a 2-/3-/4-byte UTF-8 codepoint. The smart-paste pipeline (`consume_smart_paste`) calls `is_url` / `is_image_url` for *every* paste event to decide whether to wrap the URL in a markdown link or image, so any paste containing a mixed-script line tripped the panic. Fixed by replacing the direct slice with `s.get(colon_pos..colon_pos + 3) == Some("://")`, which returns `None` instead of panicking on a non-char-boundary range. Pinned by 5 regression tests (`is_url_does_not_panic_on_mixed_script_text`, `is_image_url_does_not_panic_on_mixed_script_text`, plus URL/image-URL positive coverage). Tracked as Issue **I-3** in the v0.3.0 regression matrix; release-gate-S1 → resolved.
- **Split / rendered view: only the first of consecutive fenced code blocks visible** ([#129](https://github.com/OlaProeis/Ferrite/issues/129)) — Viewport/layout interaction hid later blocks until edits; fixed per `docs/technical/markdown/consecutive-fenced-blocks-fix.md` (Task 83).
- **Empty markdown table cells hard to focus / edit** ([#131](https://github.com/OlaProeis/Ferrite/issues/131)) — Hit targets and Tab / Shift+Tab navigation for empty cells (Task 84); see `docs/technical/markdown/table-cell-focus-navigation.md`.
- **Rendered / split WYSIWYG: double-click required to switch blocks** (Tasks 94–101) — After editing a heading, paragraph, list item, formatted block, or table cell, the first click on another block often only defocused the active `TextEdit`; a second click was needed to focus the target. Root cause: scattered per-widget defer/commit logic racing with egui focus and `content_hash`-scoped widget id remaps. Fixed by `RenderedEditSession::switch_to_ui`: one click closes the active block (commit if dirty) and opens the target with correct cursor (RS-1, RS-4, RS-5). See [`rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md).
- **Cursor flash or disappear while typing in rendered headings** (Tasks 95–97) — `ui.push_id(content_hash, …)` remapped all TextEdit ids whenever the source changed during a focus transition, destroying caret state mid-session. Stable ids under `source_epoch` keep the caret visible while typing (RS-2). See [`rendered-widget-identity.md`](docs/technical/markdown/rendered-widget-identity.md).
- **Formatted list/paragraph stuck in raw edit mode after clicking away** (Task 94) — Click-to-edit blocks with inline formatting (`**bold**`, etc.) could remain in raw TextEdit after blur because `formatted_exit_should_save` deferred on the blur frame but was never re-checked. Phase 0 hotfix plus full session migration (Task 100): immediate save/exit on dismiss; `formatted_editing` cleared on switch (RS-3). See [`rendered-edit-session-formatted.md`](docs/technical/markdown/rendered-edit-session-formatted.md).
- **Rendered / split tables: two clicks to edit another cell after typing** — After editing a cell, the first click on another cell (same table or a different table in the file) often did nothing; a second click was required. Fixed initially with `TableGlobalFocus` and pointer-down activation (Task 84); **superseded in v0.3.0** by session-integrated table cells (`BlockRef::TableCell`, `signal_table_force_commit` on cross-block exit). See [`table-cell-focus-navigation.md`](docs/technical/markdown/table-cell-focus-navigation.md), [`rendered-edit-session-tables.md`](docs/technical/markdown/rendered-edit-session-tables.md).
- **Quick file switcher (Ctrl+P) search quality** — Workspace quick open now tokenizes paths on `-`, `_`, `.`, and path separators so queries like `tables` match `test_tables.md` and `box` matches `test_box_drawing.md`. Search includes both indexed tree files and recent files (so unexpanded folders still match files you have opened recently). Replaced loose full-path fuzzy matching and the +100 recent-file score boost that ranked unrelated recent files above real matches. See `src/ui/quick_switcher.rs`.
- **Frontmatter panel stale after tab switch** — The FM tab could show the previous file’s fields, report “No frontmatter detected” on files that had YAML, or splice the wrong body when **Add frontmatter** was clicked. Caused by caching on `content_version` alone (each tab’s counter starts at `0`). Fixed by keying the cache on `(tab_id, content_version)` with regression tests. See `docs/technical/ui/frontmatter-panel.md` (*Caching*).
- **Rendered / split scroll “ghost snap” with sync off** — Stale `pending_scroll_*` from prior split sync or mode-toggle could still move the preview after scrolling stopped when **Sync scroll** was disabled. Turning sync off now clears tab pending targets and resets per-tab `SyncScrollState`; rendered mode only applies programmatic scroll when sync is on. See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md).
- **Split 2-way sync: preview jumped up when scrolling to bottom** — Rendered→raw top/bottom idle snap incorrectly wrote `tab.pending_scroll_offset`, which the **preview** pane consumes on the next frame (raw max scroll is much smaller than preview max on code-block-heavy docs). Top/bottom snaps from the rendered pane now use `SyncScrollState::set_raw_target()` → `EditorWidget::pending_sync_scroll_offset`; middle-of-document sync still uses `pending_scroll_anchor`. Raw→preview path unchanged (`pending_scroll_offset` for preview). See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md) § Split-view scroll delivery.
- **Crash recovery dropped file opened on cold start** — After a crash or forced exit, double-clicking a file while Ferrite was closed showed the session recovery dialog, but choosing **Restore session** only restored the previous session and did not open the file the user had just launched. Startup paths from CLI / file association are now deferred until the user answers the dialog, then opened afterward (existing tabs with the same path are focused, not duplicated). See `docs/technical/files/session-persistence.md`.
- **Recovery / autosave files could bleed across tabs (cross-tab data loss)** (Task 106) — Recovery JSON and `untitled_*.autosave` files persist across launches, but `tab_id` is reset on every launch, so a leftover recovery file (e.g. an untitled `asdasd` buffer with `tab_id=10` from a previous session) could silently apply to an unrelated path-backed tab assigned the same id this session — and a save would overwrite the real file. Recovery and autosave now require **path-and-disk-hash identity** (`RecoveryContent.path` / `original_content_hash`, `AutoSaveMetadata.disk_content_hash`) before they are applied. Mismatches are rejected and emit `session_recovery_identity_mismatch` diag events; legacy pre-task-106 files (no identity) keep the historical "tab id only" matching for upgraders. When the recovery file's identity matches but the buffer differs from current disk, the central panel now renders a non-blocking **Recovered content differs from this file on disk** banner above the editor with **Keep Recovered** / **Reload from Disk** actions (editing remains unblocked). `prune_stale_recovery_files` extends to `untitled_<id>.md.autosave`. See `docs/technical/files/session-persistence.md` § Identity-Gated Recovery.
- **CRITICAL: Restored recovery snapshots dropped all edits on the second crash cycle (Task 106.6 — disk-hash anchoring)** — After clicking **Restore** on the session-recovery dialog, the new tab was built via `Tab::with_file(path, recovered_content)`, which sets **both** `tab.content` and `tab.original_content` to the recovered buffer. That broke the tab's identity link to disk: `is_modified()` returned `false`, `disk_content_hash()` hashed the recovered buffer, and the next crash snapshot wrote that wrong hash into `recovery/<id>.json::original_content_hash`. On the *next* launch, Layer-3 identity gating in `AppState::try_apply_recovery` correctly rejected the poisoned recovery (disk hash didn't match the stored `original_content_hash`), falling back to disk content — silently discarding every edit made since the previous Restore. Reproduces as: edit raw → kill → recover (ok) → edit rendered → kill → recover (jumps back to pre-first-edit, both edits gone). `restore_from_session_result` now splits the recovery branch: pure `ResolvedContent::Recovered` keeps existing behaviour (disk == content or disk unreadable, both safe), while `ResolvedContent::RecoveredWithDiskDivergence` constructs the tab from `on_disk_content` and swaps in the recovered buffer via `set_content` — so `original_content` stays anchored to disk, `disk_content_hash()` matches the live file, and the next snapshot's `original_content_hash` is correct. Bonus: Ctrl+Z after Restore now walks back to disk content as one logical step. Regression tests: `test_restore_with_divergence_anchors_original_to_disk`, `test_restore_then_edit_keeps_disk_hash_anchor` in `src/state.rs`. See `docs/technical/files/session-persistence.md` § Disk-hash anchoring across recovery cycles.
- **Export menu double icons (PDF, Print preview)** — Locale strings still contained emoji while the ribbon already prefixed Phosphor icons; stripped duplicate glyphs from export menu labels in all locale files.
- **Outline panel tab selector layout** — Phosphor icons in tab labels broke hit-testing when allocated via nested `allocate_ui_at_rect`; fixed with painter-based dual-font tab labels (`paint_tab_label`) so icons and text stay centered and clickable.
- **Mermaid flowcharts shifted right with a large left gap** ([#83](https://github.com/OlaProeis/Ferrite/issues/83), FC-83a) — TD/BT diagrams with feedback loops (e.g. `test_md/test_mermaid_issue_83.md`) appeared right-aligned inside the diagram frame. Root cause: layers were centered in `available_width` while the renderer sized to actual node bounds, plus symmetric back-edge padding offset content left. Fixed by centering TD/BT layers on `max_cross_size`, normalizing layout to the margin, and applying back-edge padding only on sides that need loop clearance. Pinned by `test_fc_83a_layout_has_all_nodes` (min_x at margin) and `fc_83a_back_edge_padding_is_right_only`.
- **Multi-cursor copy / cut only copied the primary selection** — With multiple cursors, each with a range, Ctrl+C (and cut) put only the primary line on the clipboard. `selected_text()` now joins every non-empty selection (top-to-bottom, newline-separated, VS Code-style); copy/cut handlers use `has_any_selection()`; cut marks all affected lines dirty for syntax highlighting. Regression test: `test_selected_text_multi_cursor` in `src/editor/ferrite/editor.rs`.
- **Search in Files panel grew to full window height** — Opening **Search in Files** (Ctrl+Shift+F) started small then expanded frame-by-frame as results loaded (egui `Window` / `Resize` grows to `last_content_size` each frame, which reads as a slow animation even with `animation_time = 0`). Results now scroll inside a fixed-height region; max height capped at 480px (default 320px); window fade in/out disabled. See `src/ui/search.rs`, `src/ui/window.rs` (`search_panel_constraints`).
- **Detached Productivity Hub fought resize and snapped width** — Shrinking the floating hub felt animated because wide content re-expanded the window every frame and `max_size` enforced a 560px width floor from dock width. Removed the floor; clipped scrollable inner content (same pattern as the docked sidebar); viewport-based max size only; fade in/out disabled. A separate OS window on another monitor (true pop-out) is not in v0.3.0; terminal pop-out (`show_viewport_immediate`) is the precedent for a follow-up. See `src/ui/productivity_panel.rs`.
- **CSV rendered view: long cell text overflowed column bounds** — The 50-character cap did not guarantee fit within the fixed column pixel width (common with long numeric strings), so text spilled into neighbouring columns or past the table edge before ellipsis appeared. Rendered cells now use `truncate_cell_to_pixel_width()` (font-metrics binary search + `...`) and a painter clip rect while keeping the existing per-cell `allocate_exact_size` column layout; hover tooltips still show the full value. See `src/markdown/csv_viewer.rs` (`render_row_cells`), [`docs/technical/viewers/csv-viewer.md`](docs/technical/viewers/csv-viewer.md).
- **Per-document view mode (Raw / Split / Rendered) not restored on reopen** — Closing a tab and opening the same file again always opened in the global **Default view mode** (e.g. Raw), even after switching to Split. Root cause: `open_file` ignored saved `TabInfo` in `last_open_tabs`, closing a tab dropped that file’s entry on the next settings save, and title-bar view changes were not persisted. Reopen now restores `view_mode` and `split_ratio` per path; closed tabs upsert into `last_open_tabs`; settings merge open tabs without erasing closed-file state. See [`docs/technical/view-mode-persistence.md`](docs/technical/view-mode-persistence.md).
- **Quick note workflow: closing untitled tabs with content did not prompt to save** — With **Quick note workflow** enabled (default), save prompts were suppressed for all pathless tabs, so closing a modified untitled tab discarded content silently even though quitting the app correctly stayed frictionless. `Tab::should_prompt_to_save` now takes a `SavePromptContext`: **tab close** (×, Ctrl+W) still shows the unsaved-changes dialog (Save / Don't save / Cancel) for modified untitled buffers; **app exit** continues to skip the dialog and rely on session recovery. Saved files on disk are unchanged. See [`docs/technical/config/quick-note-workflow.md`](docs/technical/config/quick-note-workflow.md).
- **Document nav buttons visible above modal overlays** — The scroll-to top/middle/bottom controls (shown on hover in raw and rendered editors) painted on top of the Quick File Switcher (Ctrl+P), command palette, and Search in Files panel because both used `egui::Order::Foreground`. Nav buttons now use `Order::Middle` (below modal overlays) and are suppressed while any of those panels is open. See `src/ui/nav_buttons.rs`, `src/app/central_panel.rs`.
- **Rendered / split: Ctrl+Home and Ctrl+End jumped within the active block** — Inline `TextEdit` widgets in rendered preview consumed those keys, moving the caret to the start/end of the current block instead of scrolling the document. Intercepted at the `ScrollArea` level in `MarkdownEditor::show_rendered_editor` (Raw mode was already correct via FerriteEditor). Contributed by [@moabtools](https://github.com/moabtools) in [PR #137](https://github.com/OlaProeis/Ferrite/pull/137).
- **Status-bar Help (`?`) vs bottom-right resize** (I-1, WIN-5) — Dragging from the bottom-right corner to resize no longer opened About/Help on release. Foreground resize click guards (`consume_clicks_in_resize_zones`) and `WindowResizeState::blocks_widget_clicks()` prevent edge widgets from stealing resize clicks. See [`docs/technical/platform/window-resize.md`](docs/technical/platform/window-resize.md).
- **Integrated terminal: CJK paste / input local-echo** (I-2, TRM-3) — Pasting or typing CJK text (e.g. Chinese) into the integrated terminal showed `????` on the prompt line while the shell still received correct UTF-8 and error output displayed the real characters (Windows PSReadLine/cmd echoed with the legacy console code page). Fixed by initializing UTF-8 when spawning shells in `src/terminal/pty.rs` (PowerShell: `[Console]::InputEncoding`, `[Console]::OutputEncoding`, `$OutputEncoding`, `chcp 65001`; cmd: `chcp 65001`), and lazy-loading CJK fonts when the terminal sends CJK input so echoed glyphs render in the monospace grid. **Open a new terminal tab** to pick up the startup script — existing sessions are not retrofitted. See [`docs/technical/terminal/terminal-cjk-wide-chars.md`](docs/technical/terminal/terminal-cjk-wide-chars.md).
- **Rendered view: scroll jump when toggling task list checkboxes** — Clicking `- [ ]` / `- [x]` checkboxes mid-document could nudge the preview up or down even though the user had not scrolled. Root cause: the one-character source change invalidated viewport culling (`content_hash` mismatch → bootstrap remeasure with different `total_height`), while `pointer.any_down()` during the click started the scroll-cooldown window that suppressed height fixup. Fixed by reusing culling layout when block `(start_line, end_line)` structure is unchanged, refreshing `content_hash` without remeasuring, and treating only wheel input or scrollbar drag as active scroll (not checkbox clicks). See [`docs/technical/markdown/task-list-checkbox.md`](docs/technical/markdown/task-list-checkbox.md), [`docs/technical/markdown/rendered-view-viewport-culling.md`](docs/technical/markdown/rendered-view-viewport-culling.md), [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md).

## [0.2.9] - 2026-04-23

Hotfix release for four critical v0.2.8 regressions. No new features — upgrade
strongly recommended.

### Fixed

- **Crash in Split / Rendered view on empty documents** ([#127](https://github.com/OlaProeis/Ferrite/issues/127)) — The WYSIWYG renderer's viewport-culling bootstrap used an inclusive range `first_vis..=last_vis` that, when the document had zero blocks, iterated once and indexed `doc.root.children[0]`, aborting with "index out of bounds". Switched to a half-open range so empty documents render as a no-op. This was reproducible by launching Ferrite with Split set as the default view mode, or by clicking the Split/Rendered button on any freshly-created untitled tab.
- **CRITICAL: No unsaved-changes indicator and no save prompt on close, leading to silent data loss** — Edits made through the raw FerriteEditor wrote the new content into `tab.content` but never bumped `content_version`, so the cached `is_modified()` kept returning its initial `false`. Result: the title bar never showed the `*` dirty mark, Ctrl+W / window close skipped the "Save changes?" dialog, and auto-save never fired. Fixed by routing raw-mode edits through `prepare_undo_snapshot_hashed` + `record_edit_from_snapshot`, and by centralizing `content_version` bumps inside `record_edit_from_snapshot` / `set_content` so every edit path (raw, rendered, tree viewer) invalidates the cache.
- **CRITICAL: Undo / redo not working — "Nothing to undo" after typing** — The FerriteEditor maintains its own internal edit state, but `handle_undo` / `handle_redo` (the Ctrl+Z / Ctrl+Y keyboard handlers) read from `tab.edit_history`, which received no operations from raw-mode edits. Fixed by diffing the pre-edit snapshot against the post-edit content on every frame where FerriteEditor reports `is_content_dirty()` and recording the resulting ops onto `tab.edit_history`.
- **Selection invisible in Light mode** ([#121](https://github.com/OlaProeis/Ferrite/issues/121)) — FerriteEditor draws the selection rectangle *on top of* the text galley, so the fill has to stay partly transparent (otherwise selected text would disappear behind a solid rectangle). In Light mode the theme's selection colour `(215, 230, 250)` composited at ~40% alpha over white collapsed to essentially white, making the selection invisible. Bumped `BaseColors::light().selected` to a more saturated blue `(100, 170, 245)` that stays visible at reduced alpha while still looking at home as a light-theme widget highlight.
- **Document side panel tab labels overlapping at default width** — The Outline/Stats/Links/FM/Hub tabs split `available_width / 5` per tab, which clipped every label at the previous 200 px default. Raised the default panel width to 300 px and the minimum to 260 px so all five tab labels render in full. Existing users with a persisted `outline_width` below 260 are auto-migrated to 260 on next launch by the settings validator.

## [0.2.8] - 2026-04-14

### Added

#### Command Palette ([#59](https://github.com/OlaProeis/Ferrite/issues/59))
- **Alt+Space command launcher** - Searchable command palette with fuzzy search across all available actions. Shows recently used commands first when empty, category-grouped browsing, keyboard shortcut hints per command. Configurable shortcut (default Alt+Space, alternative Ctrl+Shift+P selectable on Welcome page). Replaces the need for traditional menus — all ribbon and keyboard actions are discoverable through the palette.
  - **Platform-specific Alt+Space suppression** - On Windows, a thread-level keyboard hook (`WH_KEYBOARD`) intercepts Alt+Space before it reaches the OS window manager, preventing the system menu (Restore/Move/Size/Close) from appearing. No-op on macOS/Linux.
  - **Deferred command dispatch** - Palette commands are dispatched after render (same phase as keyboard shortcuts) to prevent mid-render state mutations that caused crashes with commands like Toggle View Mode.
  - **Open/Close Workspace** - Added to command palette as palette-only commands (no default keyboard shortcut).

#### Tab Interactions ([#118](https://github.com/OlaProeis/Ferrite/issues/118))
- **Middle-click to close tabs** - Middle-clicking a tab closes it, matching standard behavior in browsers and other tabbed applications.

#### Rendered View Performance ([#105](https://github.com/OlaProeis/Ferrite/issues/105))
*Addresses slow/unusable rendered view on large documents (6k+ lines, 50k+ words).*

- **Markdown AST caching** - Cache parsed `MarkdownDocument` using blake3 content hash. Only re-parses when content actually changes instead of 60×/second.
- **Rendered view viewport culling** - `.show_viewport()` with 500px overscan buffer. Off-screen blocks get `ui.allocate_space()` instead of full rendering. Reduces per-frame work from O(N blocks) to O(visible blocks).
- **Block-level height cache** - LRU cache of measured heights per rendered block keyed on content hash. Enables accurate scrollbar positioning with viewport culling.
- **Lazy block height estimation** - Heuristic heights for unmeasured blocks, render budget cap (max 20 blocks/frame), progressive refinement. Initial lag under 100ms for 10K+ block files.

#### Editor Performance
- **Uniform height mode for large files** - Auto-enabled for 100K+ line files: O(1) line positioning, no O(N) `cumulative_heights` vector, force-disabled word wrap above threshold.
- **Smarter LineCache invalidation and scaling** - Targeted `invalidate_range(start, end)` evicts only affected lines instead of full cache clear. Dynamic `max_entries(visible_lines)` scales cache with viewport. 80%+ hit rate for unchanged regions after edits.
- **Per-frame O(N) elimination** - 7 per-frame O(N) operations on `tab.content` cached via `content_version` counter: `TextStats::from_text()`, `tab.title()/is_modified()` (blake3 hash), save button, CJK/complex script font detection, auto-save tab loop, frontmatter panel content clone, MarkdownEditor content clone.
- **Background thread file loading** - Files ≥5MB load on a background thread with progress bar, spinner, MB loaded/total, and cancellation on tab close. UI remains responsive at 60fps during load.

#### Viewer Performance
- **CSV raw view per-frame allocation fix** - `show_raw_view()` called `self.content.to_string()` every frame. Fixed with blake3 hash-guarded `raw_view_text` cache.
- **TreeViewer parse and raw view caching** - Two blake3-guarded caches: raw view text cache (skip per-frame `to_string()`), parsed tree cache (skip per-frame `parse_structured_content()`). Supports JSON/YAML/TOML.
- **Central panel undo content clone elimination** - Removed per-frame `tab.content.clone()` for undo recording; all modes use `Tab::edit_history` with blake3 hash-based snapshot elision (`prepare_undo_snapshot_hashed`). *(v0.3.0: time-based undo grouping removed — see [0.3.0] Changed.)*

#### Unicode & Complex Script Support (Phase 2: Text Shaping Engine)
*Depends on: Phase 1 font loading from v0.2.7*

- **HarfRust integration for FerriteEditor** - Integrated [HarfRust](https://github.com/harfbuzz/harfrust) (pure-Rust HarfBuzz port, v0.5.2+) for correctly positioned, contextually-formed glyphs for Arabic, Bengali, Devanagari, Tamil, and other complex scripts.
- **Shaped galley cache** - Extended `LineCache` to store shaped text runs (glyph IDs + positions) keyed on content+font+width. LRU eviction with invalidation on content/font/viewport changes.
- **Grapheme-cluster-aware cursor** - Replaced character-based cursor movement with grapheme-cluster-aware navigation using `unicode-segmentation`. Correct stepping over Bengali conjuncts, Korean jamo, emoji ZWJ sequences.
- **Shaped text measurement** - Word wrap, line width, scroll offset, cursor/mouse positioning, and selection rendering all use shaped advance widths for complex-script lines.

#### Image & PDF Viewer Tabs ([#108](https://github.com/OlaProeis/Ferrite/issues/108))
- **Image viewer tabs** - Open PNG, JPEG, GIF, WebP, BMP files in a dedicated viewer tab with zoom (Ctrl+scroll), fit-to-window, metadata display (dimensions, format, file size).
- **PDF viewer tab** - Open PDF files using [hayro](https://github.com/LaurenzV/hayro) (pure Rust PDF renderer). Page navigation, zoom, texture caching per page.

#### Markdown Rendering Settings
- **Strict line breaks setting** - "Strict line breaks" toggle in Settings → Editor (default: off). When enabled, single newlines render as hard `<br>` breaks (Obsidian model). Also available on the Welcome page.

### Fixed

- **macOS .md file association** ([#102](https://github.com/OlaProeis/Ferrite/issues/102)) - Added `UTImportedTypeDeclarations` block to `info_plist_ext.xml` to properly declare the `net.daringfireball.markdown` UTI. This enables opening markdown files from Finder via "Open With Ferrite" or double-clicking when Ferrite is the default application.
- **Windows IME candidate box positioning** ([#103](https://github.com/OlaProeis/Ferrite/issues/103), [#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Applied `layer_transform_to_global()` to `IMEOutput` coordinates so the OS receives correct screen coordinates for the candidate popup.
- **Double-dash setext headings and line collapsing** - Extended `fix_false_setext_headings()` to handle `--` (not just `-`). Preprocessing for `\n-- ` line breaks while preserving `---` horizontal rules and YAML frontmatter.
- **No space between paragraphs in rendered view** ([#109](https://github.com/OlaProeis/Ferrite/issues/109)) - Added proper paragraph spacing after paragraph blocks. Cumulative heights updated for accurate scrollbar.
- **Trailing spaces on plain text paragraphs** - Plain paragraph WYSIWYG editing dropped trailing spaces due to per-frame re-initialization from AST. Fixed with persistent egui edit buffer (keyed by `node.start_line`).
- **Search highlight positioning in markdown tables + z-order** - Fixed find/render coordinate mapping for table cells. Resolved z-order stacking where Jump to menu rendered above Find and Search panels.
- **Search highlight misalignment after document edits** - Recompute match positions on buffer mutations. Version/hash-based auto-refresh for highlight state.
- **Terminal CJK double-width character rendering** ([#110](https://github.com/OlaProeis/Ferrite/issues/110)) - Added `unicode-width` crate. `put_char()` advances cursor by 2 for wide chars with continuation markers. Renderer draws wide chars spanning 2 cell widths. Selection snaps to wide char boundaries.
- **Windows 11 borderless window UI offset** ([#112](https://github.com/OlaProeis/Ferrite/issues/112)) - Added `.with_transparent(true)` to `ViewportBuilder` as DWM compositing workaround for Intel HD 4600 GPU rendering offset in borderless mode.
- **Scrollbar not resetting when switching documents** ([#113](https://github.com/OlaProeis/Ferrite/issues/113)) - Scoped central-panel editor/preview widget IDs with `tab.id` to prevent egui `ScrollArea` state leaking across tab switches.
- **Strikethrough and inline formatting lost in tables** ([#117](https://github.com/OlaProeis/Ferrite/issues/117)) - Table cells containing `~~strikethrough~~`, `**bold**`, `*italic*`, or `***bold italic***` markdown lost their formatting during parsing/serialization because `text_content()` stripped inline markup. Replaced with `serialize_inline_content()` in both `serialize_table` and `TableData::from_node` to preserve formatting through round-trip. Additionally, table cells now render inline markdown as formatted rich text (bold, italic, strikethrough, inline code) using a custom `LayoutJob`-based parser. Click a cell to switch to raw markdown editing; click away to see rendered formatting. Supports full nesting (e.g., `**~~bold struck~~**`, `***~~triple~~***`).
- **Linux Cinnamon file dialog detection** ([#116](https://github.com/OlaProeis/Ferrite/issues/116)) - Linux Mint Cinnamon (`XDG_CURRENT_DESKTOP=X-Cinnamon`) was not recognized as a native desktop, causing unnecessary portal fallback. Added `x-cinnamon` to native desktop detection list, updated portal install instructions to recommend `xdg-desktop-portal-xapp`/`gtk` for Cinnamon, and fixed dialog cancellation being misclassified as a portal failure.
- **Custom font crash on Linux** ([#114](https://github.com/OlaProeis/Ferrite/issues/114)) - Selecting certain system fonts (font collections, corrupt files, unsupported formats) crashed the app via epaint panic. Added TTF/OTF magic-byte validation rejecting `.ttc` collections, WOFF, Type 1, and corrupt data. Wrapped font loading in `catch_unwind` as safety net. On failure, gracefully falls back to Inter font with toast notification. Also fixed HarfRust text shaping for custom fonts (`FONT_CUSTOM` case was missing from `ttf_bytes_for_font_id_shaping`).

## [0.2.7] - 2026-03-11

### Added

#### Markdown Linking
- **Wikilinks support** ([#1](https://github.com/OlaProeis/Ferrite/issues/1)) - `[[target]]` and `[[target|display]]` syntax with relative path resolution, spaces in filenames, click-to-navigate, same-folder-first tie-breaker, ambiguity prompting
- **Backlinks panel** - Panel showing files linking to the current document; graph-based indexing for workspaces >50 files; click-to-navigate; detects both `[[wikilinks]]` and `[markdown](links)`

#### Content Blocks
- **GitHub-style callouts** - `> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, `> [!CAUTION]`, `> [!IMPORTANT]` with color-coded rendering, custom titles, and collapsible state (`> [!NOTE]-`)

#### Zoom
- **Ctrl+Scroll Wheel zoom** ([#85](https://github.com/OlaProeis/Ferrite/issues/85)) - Ctrl+Mouse Wheel mapped to `egui::gui_zoom` (same as Ctrl++/Ctrl+-). Added `ZoomIn`/`ZoomOut`/`ResetZoom` as shortcut commands with keyboard bindings.

#### Check for Updates
- **Manual Check for Updates** - Settings → About section with button to check GitHub Releases API; shows up-to-date, update available with download link, or error; user-initiated only (offline-first)
- **Security hardening** - Response URL validated against GitHub releases prefix; TLS via rustls (pure Rust)

#### Unicode & Complex Script Support
- **Lazy font loading for complex scripts** - Extends the CJK lazy-loading system to cover 22 Unicode ranges across 11 script families: Arabic (5 sub-ranges), Bengali, Devanagari, Thai, Hebrew, Tamil, Georgian, Armenian, Ethiopic, other Indic (Gujarati, Gurmukhi, Kannada, Malayalam, Telugu), and Southeast Asian (Myanmar, Khmer, Sinhala). System fonts are loaded on demand when script characters are detected in file content or IME input (~1-5MB per script vs ~15-20MB for CJK). No new dependencies.
- **Script detection utility** - `detect_complex_scripts()` and `needs_complex_script_fonts()` with 17 unit tests covering all script families and boundary cases; used to trigger lazy font loading.
- **Complex script font preferences** - Settings → Appearance → Additional Scripts section for pre-selecting fonts per script (Arabic, Bengali, Devanagari, Thai, Hebrew, Tamil, Georgian, Armenian, Ethiopic, Other Indic, Southeast Asian). User preferences are tried first before platform defaults; persisted in config; fonts load on-demand when files with those scripts are opened.

#### Large File & Performance
- **Large file detection** - Files >10MB on open show non-blocking performance warning toast
- **Lazy CSV row parsing** - Large CSV/TSV files (≥1MB) now use byte-offset row indexing instead of parsing all rows into memory. Only visible rows (~200) are parsed on demand with viewport caching. For a 1M-row CSV, reduces additional memory from ~100-200MB to ~8MB. Small files (<1MB) now use cached full parse (previously re-parsed every frame)

#### Window & File Handling
- **Single-instance protocol** - Double-clicking files (file tree or OS/Explorer) opens them as tabs in the existing Ferrite window instead of spawning new processes; lock file + TCP IPC with background accept thread for instant response

#### PortableApps.com Support
- **PortableApps.com Format packaging** - Full PAF-compliant portable distribution (`FerriteMDPortable_x.y.z_English.paf.exe`). Stores all settings in `Data\settings\` via the PAF launcher — no registry writes, no AppData modifications. Compatible with the PortableApps.com Platform menu.
- **`FERRITE_DATA_DIR` environment variable** - New config directory override via env var, enabling external launchers (PortableApps.com, custom wrappers) to redirect settings storage. Checked before `portable/` folder detection; fully backward-compatible with existing portable and standard installs.
- **Automated PAF build in CI** - Release workflow now builds and signs the `.paf.exe` installer alongside the portable zip and MSI. Version, icons, and metadata are derived automatically from the git tag.

#### Nix/NixOS Support
- **Official Nix flake** - First-class `flake.nix` for reproducible builds and development shells. Supports `nix run github:OlaProeis/Ferrite`, `nix build .#ferrite`, and `nix develop` with Rust toolchain + platform dependencies. Targets x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin. Includes declarative NixOS/Home Manager usage example. Contributed by [@liuxiaopai-ai](https://github.com/liuxiaopai-ai) ([PR #92](https://github.com/OlaProeis/Ferrite/pull/92)).
- **Nix CI workflow** - GitHub Actions workflow (`.github/workflows/nix.yml`) evaluates package outputs for all supported systems and runs `nix flake check` on Linux.

#### Installer (Windows MSI)
- **File associations** - Optional per-extension file type registration (.md, .markdown, .txt, .json, .yaml, .yml, .toml, .csv, .tsv) via OpenWithProgids; adds Ferrite to "Open With" menu and Windows Default Apps settings without overriding existing defaults
- **Explorer context menu** - Optional "Open with Ferrite" right-click entry for files and "Open Folder with Ferrite" for directories (including folder background)
- **Add to System PATH** - Optional PATH entry so `ferrite` can be run from any terminal; cleanly removed on uninstall
- **Desktop shortcut** - Optional desktop shortcut alongside the existing Start Menu shortcut
- **Feature selection UI** - Installer now uses WixUI_FeatureTree with a customization page where users can toggle each feature group independently
- **Launch after install** - "Launch Ferrite" checkbox on the installer exit dialog (checked by default)

#### Table Editing
- **Table background & layout fix** - Replaced pre-painted row backgrounds (which used pre-measured heights causing misalignment) with the Shape::Noop placeholder technique (same approach as egui's Frame). Backgrounds are now painted after content using actual rendered row dimensions, eliminating gaps, overlap, and text rendering outside row bounds. Fixed cell layout direction from inherited `left_to_right` to explicit `top_down` so vertical padding works correctly.
- **Column resizing** - Drag vertical column separators to resize adjacent columns. Cursor changes to resize-horizontal on hover; visual guide line during drag. Custom widths persist in egui memory per table and scale proportionally on window resize. Double-click a separator to reset all columns to auto-calculated widths.

#### Editing Modes
- **Vim mode** - Optional Vim-style modal editing with Normal/Insert/Visual modes. Essential Vim commands: hjkl movement, dd (delete line), yy (yank), p (paste), /search, v/V (visual selection). Mode indicator in status bar ([NORMAL]/[INSERT]/[VISUAL]). Toggle in Settings → Editor. Ctrl+ shortcuts still work globally when Vim mode is active.

#### Welcome View
- **Welcome view on first run** - Welcome tab on first launch with configuration for theme, language, editor settings (word wrap, line numbers, minimap, bracket matching, syntax highlighting), max line width, CJK font preference, and auto-save. Only shown when no CLI paths and no session-restored tabs. Contributed by [@blizzard007dev](https://github.com/blizzard007dev) ([PR #80](https://github.com/OlaProeis/Ferrite/pull/80)).

#### Frontmatter Editor ([#94](https://github.com/OlaProeis/Ferrite/issues/94))
- **Visual frontmatter panel** - FM tab in outline panel for editing YAML frontmatter as key-value pairs with form-based interface, bidirectional sync with source
- **Basic field type support** - String, date, and list (tags with chip UI) field types with appropriate input widgets

#### UI Declutter & Edge Toggles
- **Format toolbar moved to editor bottom** - Markdown formatting buttons (bold, italic, code, headings, lists, etc.) moved from ribbon to a collapsible toolbar at the bottom of the raw editor area. Visible in Raw and Split modes for markdown files. Collapse/expand via chevron toggle.
- **Side panel toggle strip** - Replaced separate Outline and Productivity Hub ribbon buttons with a thin toggle strip on the right edge of the editor. Click to open/close the side panel (Outline, Statistics, Backlinks, Productivity Hub tabs).

#### Localization
- **German and Japanese in Settings** - Deutsch and 日本語 now available in Settings → Appearance → Language (locale files already existed)

### Changed

#### Editing
- **Keep text selected after formatting** ([#72](https://github.com/OlaProeis/Ferrite/issues/72)) - Bold, Italic, and other formatting operations now preserve both selection and editor focus. Users can chain formatting operations (e.g. Bold → Italic) without reselecting text.

#### Settings
- **Default maximum line width changed to 100 characters** - The default `max_line_width` setting is now 100 characters (was "Off"). This provides a comfortable reading width out of the box, especially for zen mode, while still allowing users to change it to Off, 80, 120, or a custom width.

#### Refactoring
- **Flowchart modular refactor** - Split monolithic `flowchart.rs` (3600 lines) into 12 focused modules under `flowchart/` directory: `types.rs`, `parser.rs`, `layout/` (config, graph, subgraph, sugiyama), `render/` (colors, nodes, edges, subgraphs), `utils.rs`. Zero behavior changes, all 83 tests pass.

#### Single-Instance Performance
- **Instant file opening from Explorer** - Redesigned single-instance IPC for near-zero latency. Moved instance check to before config/icon loading so the secondary process exits in <100ms (was 3-5s). Replaced per-frame non-blocking poll with a dedicated background accept thread that wakes the UI immediately via `ctx.request_repaint()`, eliminating idle repaint delays (was up to 500ms). Added explicit `shutdown(Write)` for immediate EOF delivery. End-to-end: <50ms (was 3-10s).

#### View Mode
- **View mode bar always visible** - The view mode segmented control (Raw/Split/Rendered) now appears for all editor tabs, not just markdown/structured/tabular files. File types that don't support split view (e.g. `.rs`, `.py`, `.txt`) show a compact two-mode segment (Raw | Rendered). When default view mode is Split and a non-split-capable file is opened, the tab automatically falls back to Raw mode.

#### Window Controls
- **Window control button redesign** - Close (×), Minimize (–), Maximize/Restore, and Fullscreen buttons are now drawn with crisp manually-painted icons (line segments, no font glyphs), rounded hover backgrounds (4 px radius), and a more compact footprint (36 × 22 px, down from 46 × 28 px). All four buttons now have consistent rounded-rect hover styling.
- **Close button** - Icon drawn as two diagonal line segments for pixel-accurate rendering; switches to white on red hover.
- **Maximize button** - Rectangle icon with a thicker top edge (2 px) to suggest a window title bar; restore icon unchanged.
- **Fullscreen button** - Replaced broken arrow icon (was rendering as ×) with proper corner-bracket icons: expand ⌜⌝⌞⌟ when windowed, compress when in fullscreen.
- **NE corner resize re-enabled** - Top-right corner (`NorthEast` direction) can now be used to resize the window. The 12 px right margin on the button group keeps the 10 px corner grab zone button-free, so resize and close never conflict. `TITLE_BAR_BUTTON_RIGHT_MARGIN` constant documents the invariant.

### Fixed

- **Linux file dialog error handling** ([#97](https://github.com/OlaProeis/Ferrite/issues/97)) - Detect xdg-desktop-portal failures on Linux (Hyprland, Sway, i3, etc.) and show helpful error modal with distro-specific install instructions (pacman, apt, dnf) and "Copy Install Command" button, instead of silent failure
- **macOS Release .app Bundle** ([#93](https://github.com/OlaProeis/Ferrite/issues/93)) - Release CI now packages proper `.app` bundle via `cargo-bundle` instead of raw binary, fixing Gatekeeper blocking on macOS
- **Task List Checkbox Rendering** ([#95](https://github.com/OlaProeis/Ferrite/issues/95)) - Task list checkboxes now render as proper UI elements instead of ASCII `[ ]`/`[x]` in rendered view; bullet point markers suppressed for task list items
- **Light mode text invisible** - All `RichText::strong()` labels (section headers in Settings, Terminal, Files, About, and other panels) were invisible in light mode. Root cause: egui's `strong_text_color()` returns `widgets.active.fg_stroke.color`, which was set to `Color32::WHITE` for the pressed-button state. This bypasses `override_text_color`, rendering white-on-white text. Fixed by setting `active.fg_stroke` to `colors.text.primary` in the light theme.
- **Images not displaying in rendered mode** - Markdown `![](path)` images now render in Rendered/Split view; path resolution relative to document and workspace root; PNG, JPEG, GIF, WebP support; graceful placeholders for missing/unsupported files
- **CJK rendering after restart with explicit preference** ([#76](https://github.com/OlaProeis/Ferrite/issues/76)) - Preload user's preferred CJK font at startup when preference is explicit (non-Auto), so restored tabs render correctly without tofu. Verified: `preload_explicit_cjk_font()` loads the appropriate font, `bump_font_generation()` increments counter, `editor.rs` invalidates line cache (Task 45)
- **CJK fonts load on language switch** - Switching to Chinese or Japanese in the Welcome panel (or Settings) now lazily loads the required CJK font immediately, so translated UI labels render correctly instead of showing squares
- **Latin-only names in Language and CJK selectors** - Language and CJK Regional Preference dropdowns now use Latin-only display names (e.g. "Chinese (Simplified)", "Japanese") so they render correctly before CJK fonts load
- **Syntax highlighting per-frame re-parsing** - `highlight_line()` was called every frame before cache check, causing lag on long lines; now checks cache first and only parses on cache miss
- **Scrollbar position with word wrap** - Scrollbar thumb position now uses cumulative y-offsets from height cache instead of uniform line height; scrollbar accurately reflects position
- **Scrollbar drag reverse mapping** - Dragging scrollbar now uses `y_offset_to_line()` binary search instead of uniform division for accurate jumps
- **Scrollbar jumping** - Replaced per-frame `rebuild_height_cache` with dirty-flag approach; smoothed scrollbar height to avoid abrupt changes as wrap info is discovered
- **Crash on large selection delete with word wrap** - Fixed capacity overflow panic when deleting large selections; added `saturating_sub`, viewport clamping, and `truncate_wrap_info()` for stale entries
- **List item text not wrapping in preview** ([#82](https://github.com/OlaProeis/Ferrite/issues/82)) - Long list item text in rendered/split view extended as a single line beyond the pane edge instead of wrapping. Root cause: all 4 list item `TextEdit` widgets used `singleline` mode which fundamentally cannot wrap. Changed to `multiline` with custom `LayoutJob` layouter (matching the paragraph wrapping pattern), `desired_rows(1)`, and newline stripping to prevent Enter from inserting line breaks within a list item.
- **Empty list item causes heading mis-render** ([#82](https://github.com/OlaProeis/Ferrite/issues/82)) - Typing `- ` (dash-space, no content yet) after a paragraph caused the paragraph above to render as a large heading. Root cause: comrak interprets a single `-` followed by optional whitespace as a setext heading underline (valid per CommonMark spec), but in a markdown editor this is almost always the start of a list item. Added `fix_false_setext_headings()` post-processing in `parser.rs` that detects single-dash setext headings and converts them back to Paragraph + List(Item).
- **Windows IME backspace deleting editor text** ([#91](https://github.com/OlaProeis/Ferrite/issues/91)) - When using Chinese (or Japanese/Korean) input methods on Windows, pressing Backspace during IME composition deleted already-committed characters in the editor instead of only modifying the pinyin/romaji in the composition window. Root cause: egui forwards raw `Key::Backspace` events alongside IME preedit updates, and the editor processed both. Fixed by suppressing all `Key` and `Text` events while `ime_enabled` is true (active composition session). Affects Microsoft Pinyin, Xiaolanghao/Rime, and all other IME input methods on all platforms.
- **Crash when opening binary files as text** - Opening image files (PNG, JPEG, etc.) or other binary data as text documents caused a panic: "byte index is not a char boundary". Root cause: (1) Binary files were not detected before opening, (2) The `count_links()` and `count_images()` functions in `stats.rs` used unsafe byte string slicing that panicked on invalid UTF-8 boundaries. Fixed with: (1) New `is_binary_content()` function in `state.rs` that detects binary data using null byte detection and non-printable character ratio heuristics, (2) Safe string slicing using `get()` instead of direct byte indexing in `editor/stats.rs`. Binary files now show a user-friendly error message instead of crashing.
- **Extra spaces when copying from rendered mode** - Copying text from syntax-highlighted code blocks or inline-formatted paragraphs in rendered/split view inserted phantom spaces at every token boundary (e.g. `json. loads( f. read())` instead of `json.loads(f.read())`). Root cause: each syntax-highlighted segment and inline node was rendered as a separate `ui.label()`, and egui's default `item_spacing.x` (~8px) added gaps between them. Fixed by setting `item_spacing.x = 0.0` in code block horizontal layouts (`widgets.rs`) and inline content `horizontal_wrapped` layouts (`editor.rs`).
- **Session recovery dialog after "Don't Save"** - Closing the app with unsaved changes and choosing "Don't Save" caused the session recovery dialog to appear on next launch, also closing any active workspace. Root cause: `create_lock_file()` was called before `load_session_state()` at startup, so `check_and_clear_lock_file()` always found the just-created lock file and set `is_crash = true` on every launch. Combined with the session file recording `has_unsaved_content = true` for discarded tabs, this falsely triggered the recovery prompt. Also meant crash detection was completely broken (lock file never persisted through runtime). Fixed by reordering: load session state first (checks previous run's lock file), then create lock file for the current session.

---

## [0.2.6.1] - 2026-02-06

> **Patch release:** First code-signed release. Integrated terminal workspace, productivity hub, major app.rs refactoring into ~15 modules, and numerous bug fixes.

### Added

#### Integrated Terminal Workspace 🎉
> Contributed by [@wolverin0](https://github.com/wolverin0) in [PR #74](https://github.com/OlaProeis/Ferrite/pull/74) — first major community contribution!

- **Multiple terminal instances** - Create and manage multiple terminal sessions with tabs and shell selection (PowerShell, CMD, WSL, bash)
- **Tiling & splitting** - Create complex 2D grids with horizontal and vertical splits
- **Smart maximize** - Temporarily maximize any pane to focus on work (Ctrl+Shift+M)
- **Layout persistence** - Save and load your favorite terminal arrangements to JSON files
- **Theming & transparency** - Custom color schemes (Dracula, etc.) and background opacity
- **Drag-and-drop tabs** - Reorder terminals with visual feedback
- **AI-ready indicator** - Visual "breathing" animation when terminal is waiting for input (perfect for AI agents)

#### Productivity Hub
> Also contributed by [@wolverin0](https://github.com/wolverin0) in [PR #74](https://github.com/OlaProeis/Ferrite/pull/74)

- **Productivity panel** - Quick-access panel for common editing, navigation, and workflow tasks

#### Editor Improvements
- **Tab drag reorder** - Tabs can now be reordered by dragging with visual drop target indicator and `swap_tabs()` state method
- **File watcher auto-reload** - Externally modified files are now automatically reloaded when the tab has no unsaved changes; shows toast notification. If tab has unsaved changes, shows a warning instead
- **Undo after text formatting** - Bold, italic, and other formatting operations now create discrete undo entries via `break_group()` calls; Ctrl+Z reliably reverses only the format
- **Multiline blockquote rendering** - Consecutive blockquotes separated by blank lines are now merged into a single continuous block with one border
- **CJK first-line paragraph indentation** ([#20](https://github.com/OlaProeis/Ferrite/issues/20), [#26](https://github.com/OlaProeis/Ferrite/issues/26)) - Fixed first-line-only indentation for Chinese (2em) and Japanese (1em) paragraphs in rendered mode using egui `LayoutJob` with `leading_space`

#### Security
- **Code signing** - Windows artifacts (exe, MSI, portable zip) are now digitally signed via [SignPath.io](https://signpath.io/) with a production certificate from [SignPath Foundation](https://signpath.org). No more "Unknown publisher" warnings from Windows SmartScreen.

#### Memory Optimization
- **Memory diagnostics** - Added `[MEM]` log messages at startup showing memory usage at key initialization points (visible with `--log-level info`)

### Changed

#### App.rs Refactoring
> **Major restructure:** Split the monolithic 7,600+ line `app.rs` into ~15 focused modules under `src/app/`.

- New modules: `mod.rs`, `title_bar.rs`, `central_panel.rs`, `keyboard.rs`, `input_handling.rs`, `line_ops.rs`, `file_ops.rs`, `formatting.rs`, `navigation.rs`, `find_replace.rs`, `export.rs`, `dialogs.rs`, `status_bar.rs`, `helpers.rs`, `types.rs`
- See [refactoring plan](docs/technical/planning/app-rs-refactoring-plan.md)

### Fixed

#### Bug Fixes
- **Duplicate Line (Ctrl+Shift+D) wrong position** - Rewrote `handle_duplicate_line()` to use `cursor_position` (line, col) synced from FerriteEditor instead of stale `tab.cursors` char index
- **Keyboard shortcut conflict: Ctrl+Shift+E** ([#46](https://github.com/OlaProeis/Ferrite/issues/46)) - `ToggleFileTree` and `ExportHtml` were both bound to `Ctrl+Shift+E`. Changed `ExportHtml` to `Ctrl+Shift+X`
- **Maximize/restore button icon** - Button icon disappeared on hover because text was painted under the hover background. Rewrote to use custom painter drawing
- **Drag-drop image inserts at wrong position** - Image markdown link was inserted at stale `tab.cursors` position instead of actual editor cursor. Now uses `cursor_position` (line, col)
- **Smart paste not working** - Selection state was read from stale `tab.cursors` instead of FerriteEditor. Now queries FerriteEditor directly via `get_ferrite_editor_mut()`
- **Auto-save toggle inconsistency** - Title bar toggle directly flipped `auto_save_enabled` field instead of calling `toggle_auto_save()` which also clears `last_edit_time`
- **Rendered mode raw editor stuttering** - Switching from Rendered to Raw mode caused full FerriteEditor recreation, losing viewport/syntax state. Added `set_content()` method for in-place buffer replacement
- **Keyboard shortcut conflict: Ctrl+Backtick** - `FormatInlineCode` and `ToggleTerminal` both bound to same key. Changed `FormatInlineCode` to `Ctrl+Shift+Backtick`
- **CJK font crash on startup** ([#63](https://github.com/OlaProeis/Ferrite/issues/63)) - Fixed crash when a non-Auto CJK preference is persisted but the font cannot be loaded. Fonts now return `None` gracefully. Minor: tofu (□) may appear in settings labels when no CJK documents are open (fonts load lazily)
- **Portable Windows startup crash** ([#57](https://github.com/OlaProeis/Ferrite/issues/57)) - Validate persisted window position values on load. Corrupted values (NaN, infinity, out-of-bounds) are reset so the OS selects a safe default. Portable ZIP now always includes the `portable/` folder

#### Memory Optimization - CJK Font Loading
> **Reduced startup memory by ~80MB** for users with CJK font preferences set.

- **Lazy CJK font loading** - CJK fonts now load on-demand when text containing those scripts is detected, instead of loading all 4 fonts (~80MB) at startup
- **System locale detection** - Automatically detects system language and preloads only the ONE CJK font the user likely needs (~20MB):
  - Japanese locale (ja-JP) → Japanese font only
  - Korean locale (ko-KR) → Korean font only
  - Chinese Simplified (zh-CN) → SC font only
  - Chinese Traditional (zh-TW) → TC font only
  - Other locales → no preload (fully lazy)
- **Settings change optimization** - Changing CJK preference in settings no longer loads all fonts; only already-loaded fonts are preserved

**Memory impact:**
| Scenario | Before | After |
|----------|--------|-------|
| English user, no CJK | ~130 MB | ~50 MB |
| Japanese user | ~130 MB | ~70 MB |
| User with explicit CJK pref | ~130 MB | ~50 MB (loads on-demand) |

---

## [0.2.6] - 2026-01-26

> **Major Release:** Complete custom text editor (FerriteEditor) replacing egui's TextEdit. Enables editing of 100MB+ files with ~80MB RAM usage (previously 1.8GB+ for 4MB files).

### Added

#### Custom Text Editor (FerriteEditor) 🎉
> **Milestone achieved:** 80MB file now uses ~80MB RAM (was 460MB+). Editing is smooth and responsive.

Complete ground-up reimplementation of the text editor:

- **FerriteEditor widget** - Custom text editor built with egui drawing primitives
- **Virtual scrolling** - Only renders visible lines + buffer, enabling 100MB+ file editing
- **Rope-based buffer** - O(log n) text operations via `ropey` crate for instant edits
- **Full selection support** - Click-drag, Shift+Arrow, Shift+Home/End, double-click (word), triple-click (line), Ctrl+A
- **Clipboard operations** - Ctrl+A/C/X/V with proper selection handling
- **Syntax highlighting** - Viewport-aware per-line highlighting using syntect
- **Search highlights** - Find/replace integration with capped highlight count (1000 max visible)
- **Bracket matching** - Windowed O(window) algorithm (~200 lines around cursor), works at any file size
- **Word wrap** - Dynamic line heights with proper visual cursor navigation
- **Undo/redo** - Operation-based EditHistory on each tab, Ctrl+Z/Ctrl+Y *(v0.3.0+: per-record grouping; v0.2.x used 500 ms merge)*
- **IME support** - Chinese Pinyin, Japanese Romaji, Korean Hangul input
- **Code folding** - Fold regions with gutter indicators, navigation skips folds
- **Multi-cursor** - Ctrl+Click to add cursors, simultaneous editing

#### UI/UX Improvements
- **Document navigation buttons** - Top/Middle/Bottom jump buttons in editor corner
- **Semi-transparent selection** - Selected text remains readable through highlight
- **Cursor blink** - Standard ~500ms blink interval with theme-aware color
- **Auto-focus new documents** - Cursor ready to type immediately without clicking
- **.txt files in Open dialog** - Text files now visible in default filter

### Fixed

#### Memory & Performance ([#45](https://github.com/OlaProeis/Ferrite/issues/45))
> **Critical fix:** Opening a 4MB text file caused 1.8GB RAM usage and laggy editor

- **Editor per-frame content clone** - Fixed 240MB/second allocation from cloning document every frame. Now uses lazy undo snapshot pattern.
- **Case-insensitive search allocation** - Fixed full document copy for search. Now uses regex `(?i)` flag for streaming search.
- **Search debouncing** - Added 150ms debounce preventing search on every keystroke.
- **Large file memory optimization** - Files >1MB get hash-based modification detection, cleared original bytes, reduced undo stack (10 vs 100).
- **Bracket matching O(N) fix** - Was allocating entire buffer every frame (4.8GB/sec for 80MB file). Now uses windowed ~20KB extraction.
- **Memory release on tab close** - Memory properly freed when closing large file tabs.

#### Editor Bugs
- **Text jumping to next line** - Fixed cursor unexpectedly jumping when typing at end of line
- **Cannot scroll to bottom** - Fixed missing lines at bottom of large files with/without word wrap
- **Outline/Minimap cursor placement** - Fixed cursor landing several lines below clicked heading
- **Search highlight alignment** - Fixed highlight drift on wrapped lines in large files
- **Box drawing characters** - Fixed U+2500-U+257F rendering as squares (added JetBrains Mono fallback)

#### UI Fixes
- **File browser context menu icons** - Fixed doubled/square icons in right-click menu
- **Link hover gear icon removed** - Click now edits, Ctrl+Click opens in browser
- **Initial cursor visibility** - New documents show blinking cursor immediately
- **Cursor appearance** - Theme-aware color, proper height matching line height
- **Windows Start Menu icon** - Fixed pixelated/low-res icon (proper multi-size .ico)

### Changed

#### FerriteEditor Modular Architecture
> Improved maintainability by splitting 2735-line monolith into focused modules

- **editor.rs refactored** - Reduced from 2735 to 1551 lines (43% reduction)
- **New modules extracted:**
  - `buffer.rs` - Rope-based TextBuffer with efficient text operations
  - `cursor.rs` - Cursor and Selection types with multi-cursor support
  - `history.rs` - EditHistory with operation-based undo/redo
  - `view.rs` - ViewState for virtual scrolling and viewport tracking
  - `line_cache.rs` - LRU galley cache for efficient rendering
  - `selection.rs` - Selection rendering, word boundaries, select_all
  - `highlights.rs` - Search/bracket highlight rendering
  - `find_replace.rs` - Replace operations with undo support
  - `mouse.rs` - Click position to cursor conversion
  - `search.rs` - Search match management API
  - `input/` - Keyboard and IME input handling
  - `rendering/` - Cursor, gutter, and text rendering
- **Pattern:** Rust's `impl FerriteEditor` distributed across modules

#### Integration Updates
- Format toolbar connected to FerriteEditor buffer operations
- Outline panel and minimap integrated with new scroll system
- Font settings dynamically update editor rendering
- Line numbers toggle works without restart
- File save preserves encoding through FerriteEditor

### Deferred to v0.2.7
- **Editor-Preview scroll synchronization** — **Addressed in v0.3.0** (split live sync + mode-toggle hybrid). See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md). Optional follow-ups: soft follow during scroll, mapping warmup for heavy Mermaid.
- **Large file preview disablement** - Preview disabled message for >5MB files
- **SignPath code signing** - Awaiting organization approval

### Technical
- Added `ropey` crate for rope-based text buffer
- New `src/editor/ferrite/` module structure with 15+ submodules
- `LARGE_FILE_THRESHOLD` (1MB) and `LARGE_FILE_MAX_UNDO` (10) constants
- Hash-based modification detection for large files
- Lazy undo snapshot pattern with `pending_undo_state`
- Search debounce with `find_search_pending` and `find_search_requested_at`
- ViewState tracks wrapped line heights for proper scrolling
- LineCache with LRU eviction (200 entries max)

## [0.2.5.3] - 2026-01-24

### Added

#### Flathub Distribution
- **Flathub submission files** - Added `.desktop` and `.metainfo.xml` files for Flathub packaging at `assets/linux/`

#### Code Signing (Pending)
- **SignPath integration** - Windows artifacts (exe, MSI, portable zip) will be code signed via [SignPath.io](https://signpath.io/) free tier for open source once organization approval is complete. This helps prevent Windows Defender false positives and establishes trust with users.
- **CI/CD signing workflow** - Signing is integrated into GitHub Actions release workflow and will run automatically on tagged releases once approved.

#### UI Improvements
- **View Mode Segmented Control** - Replaced single-letter toggle button (R/S/V) with a polished pill-shaped segmented control showing all three view modes at once. Users can now click directly on the mode they want (Raw, Split, Rendered) with clear visual feedback for the active mode. The control adapts to file type: 3 modes for markdown/CSV, 2 modes for JSON/YAML/TOML. Visible in both normal and Zen mode.
- **App logo in title bar** - Added Ferrite logo with transparent background to the title bar for better brand visibility.

#### Syntax Highlighting
- **Extended syntax support** - Added 100+ additional language syntaxes via `two-face` crate, including PowerShell (.ps1/.psm1/.psd1), TypeScript/TSX, Zig, Svelte, Vue, Terraform, Nix, and many more. Previously unsupported languages now get proper syntax highlighting instead of plain text.
- **Syntax theme selector** - New dropdown in Appearance settings to choose from 25+ syntax highlighting color themes including Dracula, Nord, Catppuccin (Mocha/Latte/Frappe/Macchiato), Gruvbox (light/dark), Solarized (light/dark), One Half, GitHub, VS Code Dark+, and more. Set to "Auto" to match the app theme.

### Fixed

#### Linux Desktop Integration
- **Alt-tab/taskbar visibility on Wayland** - Fixed Ferrite window not appearing in alt-tab switcher or taskbar on Linux desktop environments (KDE Plasma, GNOME) running Wayland. Added `app_id` to ViewportBuilder for proper window identification.

#### Icon Rendering
- **Find/Replace replace icon** - Fixed the replace icon (↳) showing as a square box in the Find and Replace panel. Changed to a universally-supported arrow character (→).
- **Tree viewer context menu icon** - Fixed the context menu button (⋯) in JSON/YAML/TOML tree viewer showing as a square. Changed to simple dots (...) for reliable rendering.
- **Font atlas pre-warming** - Added additional symbols (⇄⇅↳↵…⋯) to the font atlas pre-warm list to ensure they render correctly from startup.

#### UI Positioning
- **Recent files menu position** - Fixed the recent files/folders popup menu appearing below and covering the filename button in the status bar. Menu now appears above the button using proper anchor positioning.

#### Performance
- **Linux folder opening freeze** - Fixed critical 10+ second UI freeze when opening workspace folders on Linux (especially Fedora/KDE Plasma). Root causes:
  - **notify crate misconfiguration** - Was configured with `default-features = false, features = ["macos_kqueue"]` which disabled the inotify backend on Linux, forcing fallback to slow polling-based file watching that had to walk and stat entire directory trees.
  - **Synchronous recursive directory scanning** - `Workspace::new()` scanned the entire directory tree recursively on the main UI thread before showing anything. Now uses lazy loading: only the root directory is scanned initially, subdirectories are scanned on-demand when expanded.

#### Bug Fixes
- **Line breaks in list items** ([#41](https://github.com/OlaProeis/Ferrite/issues/41)) - Fixed hard line breaks (`\` at end of line) within list items showing as a square box instead of rendering as a proper line break.
- **Git deleted file icon rendering** - Fixed git "deleted" status icon showing as a square box in the file tree. The previous icon character (✕) was not supported by the embedded Inter font. Changed to standard ASCII minus character (-) for reliable cross-platform rendering.
- **Blockquote/table overflow** - Added horizontal scrolling for tables and blockquotes when content exceeds container width. Previously, wide content would expand the layout and break max line width for all subsequent content. Now wide tables scroll horizontally while the rest of the document respects the configured line width setting. Code blocks and mermaid diagrams already have internal horizontal scroll handling.
- **PowerShell file rendering collapse** - Fixed critical bug where PowerShell and other files without syntax definitions would collapse all content to a single line after initial render. Root cause: the fallback path for unsupported languages used `code.lines()` which strips newline characters. Fix uses `LinesWithEndings` to preserve newlines in plain text rendering.
- **View mode segment not clickable** - Fixed issue on Linux where clicking the view mode segment (R/S/V buttons) in the title bar would initiate window drag instead of switching modes. Increased the drag exclusion zone width to fully cover all title bar controls.
- **Inter font missing box-drawing characters** - Fixed box-drawing characters (─│┌┐└┘ etc.) rendering as squares when using Inter font. The embedded Inter font doesn't include Unicode box-drawing block (U+2500-U+257F). Added JetBrains Mono as fallback font for Inter to provide these characters.

## [0.2.5.2] - 2026-01-20

### Added

#### New Features
- **Delete Line shortcut** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Cmd/Ctrl+D deletes current line (configurable in settings) - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **Move Line Up/Down** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Alt+Up/Down swaps current line with adjacent line - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **macOS file type associations** ([#30](https://github.com/OlaProeis/Ferrite/pull/30)) - Ferrite appears in Finder's "Open With" menu for .md, .json, .yaml, .toml, .txt files - thanks [@abcd-ca](https://github.com/abcd-ca)!

#### Installation & Distribution
- **Windows portable build** - True portable mode (`ferrite-portable-windows-x64.zip`) with `portable` folder for self-contained operation. All settings stored next to executable - perfect for USB drives.
- **Windows MSI installer** - Proper Windows installer (`ferrite-windows-x64.msi`) with Start Menu shortcut, application icon, and clean uninstall support via Windows Settings. Built with WiX Toolset.
- **Linux RPM package** - Native package (`ferrite-editor.x86_64.rpm`) for Fedora, RHEL, CentOS, Rocky Linux, and other RPM-based distributions. Includes desktop entry and icon integration.

#### Internationalization
- **I18n audit & cleanup** - Comprehensive audit of hardcoded strings, replacement with translation keys
- **Orphaned key removal** - Removed ~200 unused translation keys from locale files
- **Locale file sync** - All locale files now have consistent structure matching en.yaml
- **New language support** - Added Estonian and Norwegian Bokmål via Weblate community translations

### Fixed

#### Bug Fixes
- **Ctrl+X cutting entire document** - Fixed egui bug where Ctrl+X with no text selection would cut the entire document. Now correctly does nothing when nothing is selected.
- **Linux window drag stuck mouse** - Fixed critical bug where dragging the custom title bar on Linux caused the mouse to get "stuck" in drag mode. Root cause: egui's widget-level drag tracking desynchronized with the window manager after `ViewportCommand::StartDrag`. Fix bypasses egui's drag state machine entirely, using raw input detection (`primary_pressed()`) for immediate, reliable window drag initiation.
- **Split mode cursor position** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Line operations now work correctly in Split view; rendered pane no longer overwrites cursor position - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **macOS modifier tooltips** ([#28](https://github.com/OlaProeis/Ferrite/pull/28), [#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Tooltips now show "Cmd+E" on macOS instead of hardcoded "Ctrl+E" - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **Semantic minimap highlight accuracy** - Use byte offsets matching search behavior for correct highlight positioning

> **Note:** Opening files via "Open With" or dragging onto app icon not yet supported on macOS due to [winit#1751](https://github.com/rust-windowing/winit/issues/1751). Workaround: use `open -a Ferrite file.md` or File > Open.

## [0.2.5.1] - 2026-01-17

### Added

#### Idle Mode CPU Optimization
- **Tiered idle repaint system** - Implements intelligent repaint scheduling based on user interaction time:
  - Active (animations/dialogs): Continuous repaint at 60 FPS
  - Light idle (0-2 seconds): 100ms interval (~10 FPS)
  - Deep idle (2+ seconds): 500ms interval (~2 FPS)
- **User interaction tracking** - Detects keyboard, mouse, and scroll activity to determine idle state
- **Scroll animation awareness** - Continuous repaint during sync scroll animations

#### Multi-Encoding File Support
- **Automatic encoding detection** - Files are now automatically detected for encoding on open using `encoding_rs` and `chardetng` crates. No more garbled text when opening legacy files.
- **Common encoding support** - Full support for Latin-1, Windows-1252, ISO-8859-x, Shift-JIS, EUC-KR, GBK, and other common encodings beyond UTF-8.
- **Status bar indicator** - Current file encoding displayed in status bar with click-to-change dropdown menu for manual encoding selection.
- **Preserve encoding on save** - Files are saved back in their original encoding by default, not forced to UTF-8.

#### Memory Optimization
- **CJK font lazy loading** - CJK fonts (Korean, Japanese, Chinese) are now loaded on-demand when CJK characters are detected in documents, rather than at startup. This reduces idle memory usage from ~250MB to ~72MB for non-CJK users.
- **Granular per-language CJK loading** - Each CJK language font is loaded independently based on detected script (Hangul → Korean font, Hiragana/Katakana → Japanese font, Han characters → user's preferred Chinese variant). Opening a Korean document only loads Korean fonts (~30MB), not all CJK fonts (~180MB).
- **Custom memory allocators** - Platform-specific high-performance allocators: `mimalloc` on Windows, `jemalloc` on Linux/macOS. Reduces heap fragmentation and memory usage, especially for long-running sessions.
- **Viewer state cleanup** - Fixed memory leak by cleaning up `tree_viewer_states`, `csv_viewer_states`, and `sync_scroll_states` HashMap entries when tabs are closed.
- **egui temp data cleanup** - Stale `SyntaxHighlightCache` and per-tab temporary data cleared from egui memory on tab close.

#### Cursor Positioning Improvements
- **Galley-based click mapping** - Use egui's Galley for accurate click-to-character index conversion in rendered/split view click-to-edit.
- **Formatting marker mapping** - Map displayed text positions to raw markdown positions accounting for `**`, `*`, `` ` ``, `~~`, and `[links](url)` syntax.
- **Text wrapping support** - Handle wrapped lines correctly by using actual text rect width for measurement.
- **Bold font measurement** - Use bold font for measurement when content starts with bold markers for better accuracy.

#### Scroll Navigation Accuracy
- **Unified scroll calculation** - Single function for all scroll-to-line operations (find, search-in-files, outline, minimap) ensuring consistent positioning.
- **Fixed off-by-one errors** - Consistent 0-indexed vs 1-indexed line number handling across all navigation functions.
- **Fresh line height** - Ensure actual rendered line height is used instead of stale/default values when calculating scroll positions.
- **Large file accuracy** - Scroll navigation now works correctly in files with 3000+ lines; previously target lines could be hundreds of pixels off or completely out of view.
- **Semantic minimap highlight fix** - Fixed highlight offset when clicking items in semantic minimap/outline panel. The highlight now correctly marks the target line by using byte offsets (matching search behavior) instead of character offsets.

#### Settings & UX
- **Session restore option** - New setting to disable tab restoration on startup. When disabled, app starts with a single empty tab instead of restoring previous session.

### Fixed

#### CPU Usage Optimization
- **Fixed 10% idle CPU usage** - Application now uses <1% CPU when truly idle (previously ~10% even with no user interaction)
- **Window title optimization** - Only send viewport title command when title actually changes, avoiding per-frame overhead
- **Disabled repaint on widget change** - Set `options.repaint_on_widget_change = false` to prevent unnecessary repaints
- **Animation time optimization** - Removed conflicting animation_time override in ThemeManager

#### Intel Mac CPU Usage ([#24](https://github.com/OlaProeis/Ferrite/issues/24))
- **Removed verbose debug logging** - Eliminated `[LIST_ITEM_DEBUG]` statements in rendered mode that generated ~50,000 log lines per 22 seconds.
- **Fixed continuous repaint in Rendered mode** - Root cause: `Sense::hover()` on scroll area content caused ~60fps repaints bypassing idle throttling. Changed to `Sense::focusable_noninteractive()` for proper ~10fps idle throttling.

#### Bug Fixes
- **New file dirty flag** - New untitled files no longer prompt to save when closed if they haven't been modified.
- **CJK first-line indentation** - Paragraph indentation now correctly applies only to the first line, not the entire paragraph.
- **Workspace close button alignment** - X button to close workspace panel shifted left to prevent overlap with resize handles.
- **Linux close button** - Fixed issue where window close button couldn't be clicked due to hit-testing/overlay interference.

### Technical
- New `last_interaction_time` field tracks user activity for idle detection
- New `get_idle_repaint_interval()` returns appropriate interval based on idle duration
- New `had_user_input()` detects keyboard, mouse, and scroll events
- Enhanced `needs_continuous_repaint()` to check for scroll animations
- Explicit vsync and run_and_return settings in NativeOptions
- Added FPS diagnostic logging (debug builds only) for idle CPU optimization verification
- New `CjkScriptDetection` struct and `detect_cjk_scripts()` function for granular script identification
- Per-language `AtomicBool` flags track which CJK fonts have been loaded
- `CjkLoadSpec` determines fonts to load based on detected scripts and user preferences
- Platform-conditional allocator setup in `main.rs` with feature flags
- Documentation: [Idle Mode Optimization](docs/technical/platform/idle-mode-optimization.md)

### Changed

#### Antivirus False Positive Mitigation
- **Adjusted release build profile** - Changed `lto` from "fat" to "thin", `opt-level` from "z" to "3", disabled symbol stripping to reduce Windows Defender false positives.
- **Documentation** - Added "Antivirus False Positives" section to README explaining the issue and workarounds.

## [0.2.5] - 2026-01-16

### Added

#### Mermaid Improvements
- **Modular refactor** - Split 7000+ line `mermaid.rs` into `src/markdown/mermaid/` directory with separate files per diagram type
- **Edge parsing fixes** - Fix chained edge parsing (`A --> B --> C`), arrow pattern matching, label extraction
- **Flowchart direction fix** - Respect LR/TB/RL/BT direction keywords in layout algorithm
- **Node detection fixes** - Fix missing nodes and improve branching layout in complex flowcharts
- **YAML frontmatter support** - Parse `---` metadata blocks with `title:`, `config:` etc. (MermaidJS v8.13+ syntax)
- **Parallel edge operator** - Support `A --> B & C & D` syntax for multiple edges from one source
- **Rendering performance** - AST and layout caching with blake3 hashing for complex diagrams
- **classDef/class styling** - Node styling via `classDef` and `class` directives
- **linkStyle edge styling** - Edge customization via `linkStyle` directive
- **Subgraph improvements** - Layer clustering, internal layout, edge routing, title expansion, nested margins
- **Asymmetric shape rendering** - Flag/asymmetric node shape with proper text centering
- **Viewport clipping fix** - Prevent diagram clipping with negative coordinate shifting
- **Crash prevention** - Infinite loop safety, panic handling for malformed input

#### Split View Enhancements
- **Dual editable panes** - Split view rendered pane is now fully editable, matching full Rendered mode behavior
- Both panes edit the same content with changes syncing instantly
- Full undo/redo support for edits in either pane

#### Git Integration
- **Git status auto-refresh** - Automatic refresh of file tree git badges on file save, window focus, periodic interval (10 seconds), and file system events
- **Debounced refresh** - 500ms debounce prevents excessive git2 calls during rapid operations

#### CSV Support ([#19](https://github.com/OlaProeis/Ferrite/issues/19))
- **CSV/TSV viewer** - Native table view for CSV and TSV files with fixed-width column alignment
- **Rainbow column coloring** - Alternating column colors for improved readability
- **Delimiter detection** - Auto-detect comma, tab, semicolon, pipe separators
- **Header row detection** - Intelligent detection and highlighting of header rows

#### Internationalization ([#18](https://github.com/OlaProeis/Ferrite/issues/18))
- **i18n infrastructure** - YAML translation files in `locales/` directory with rust-i18n integration
- **Weblate integration** - Community translations via [hosted.weblate.org/projects/ferrite](https://hosted.weblate.org/projects/ferrite/)
- **String extraction** - UI strings moved to translation keys

#### CJK Writing Conventions ([#20](https://github.com/OlaProeis/Ferrite/issues/20))
- **Paragraph indentation** - First-line indentation setting for Chinese (2 chars), Japanese (1 char), or custom
- **Rendered view support** - Apply `text-indent` styling to paragraphs in preview mode

#### New Features
- **Keyboard shortcut customization** - Users can rebind shortcuts via settings panel; stored in config.json
- **Custom font selection** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Select preferred font for editor and UI; important for CJK regional glyph preferences
- **Main menu UI redesign** - Modernized main menu with improved layout and visual design
- **Windows fullscreen toggle** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Dedicated fullscreen button (F10) separate from Zen mode (F11)

#### Semantic Minimap
- **Header labels** - Display actual H1/H2/H3 text in minimap instead of unreadable scaled pixels
- **Content type indicators** - Visual markers for code blocks, mermaid diagrams, tables, images
- **Density visualization** - Text density shown as subtle horizontal bars between headers
- **Mode toggle** - Settings option to choose "Visual" (pixel) or "Semantic" (labels) mode

#### Editor Productivity
- **Drag & drop images** - Drop images into editor → auto-save to `./assets/` → insert markdown link
- **Table of Contents generation** - Insert/update `<!-- TOC -->` block with auto-generated heading links (Ctrl+Shift+U)
- **Document statistics panel** - Tabbed info panel with word count, reading time, heading/link/image counts
- **Snippets/abbreviations** - User-defined text expansions (`;date` → current date, `;time` → current time)
- **Recent folders** - Recent files menu now includes workspace folders

#### Branding
- **New Ferrite logo** - Orange geometric crystal icon
- **Platform icons** - Windows `.ico`, macOS `.iconset`, Linux PNGs (16-512px)
- **Window icon** - Embedded 256px icon replaces default eframe "E" logo

### Fixed

#### Bug Fixes
- **Search highlight drift** - Fixed find/search highlight boxes drifting progressively further from matched text; caused by byte vs character position mismatch in UTF-8 text
- **Config.json persistence** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Fixed window state dirty flag; settings now persist correctly across restarts
- **Session restore reliability** - Workspace folders and recent files now persist correctly across restarts with atomic file writes
- **Recent files persistence** - Recent files list now saves immediately on file open, pruning stale paths
- **Line width in rendered/split view** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Line width setting now respects pane boundaries with proper centering behavior
- **Quick switcher mouse support** - Fixed mouse hover/click not working (item flickering but not selecting)
- **Table editing cursor loss** - Table cells no longer lose focus after each keystroke in Rendered/Split modes; edits are buffered and committed when focus leaves (deferred update model)
- **Zen mode rendered centering** - Content now centers properly in rendered/split view when Zen mode (F11) is active
- **Windows top edge resize** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Window can now be resized from all edges including top
- **macOS Intel CPU optimization** ([#24](https://github.com/OlaProeis/Ferrite/issues/24)) - Idle repaint scheduling reduces CPU usage on Intel Macs

### Technical
- Split `mermaid.rs` into modular structure: `src/markdown/mermaid/` with `mod.rs`, `flowchart.rs`, `sequence.rs`, etc.
- Added `GitAutoRefresh` struct for managing refresh timing and focus tracking
- Added `had_focus_last_frame` and `content_modified` fields to `TableEditState` for focus tracking
- Added blake3 hashing for Mermaid diagram caching
- Added 11 unit tests for git auto-refresh logic
- Added comprehensive technical documentation in `docs/technical/`

### Deferred
- **Mermaid diagram toolbar** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Toolbar button to insert mermaid code blocks (deferred to v0.3.0)
- **Mermaid syntax hints** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Help panel with diagram type syntax examples (deferred to v0.3.0)
- **Simplified Chinese translation** - Waiting for community contributor (deferred)
- **Mermaid code cleanup** - Flowchart.rs modular refactor and documentation (deferred to v0.2.6)
- **Executable code blocks** - Run code snippets in preview (deferred to v0.2.6)
- **Content blocks/callouts** - GitHub-style `[!NOTE]` admonitions (deferred to v0.2.6)

## [0.2.3] - 2025-01-12

### Added

#### Editor Productivity
- **Go to Line (Ctrl+G)** - Quick navigation to specific line number with modal dialog and viewport centering
- **Duplicate Line (Ctrl+Shift+D)** - Duplicate current line or selection with proper char-to-byte index handling
- **Move Line Up/Down (Alt+↑/↓)** - Rearrange lines without cut/paste, cursor follows moved line
- **Auto-close Brackets & Quotes** - Type `(`, `[`, `{`, `"`, or `'` to get matching pair with cursor in middle; selection wrapping and skip-over behavior
- **Smart Paste for Links** - Select text then paste URL to create `[text](url)` markdown link; image URLs create `![](url)` syntax

#### UX Improvements
- **Configurable line width** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Limit text width for improved readability with presets (Off/80/100/120) or custom value; text centered in viewport

#### Platform & Distribution
- **macOS Intel cross-compilation** - CI now cross-compiles for Intel Macs from ARM64 runner

### Fixed

#### Bug Fixes
- **Task list rendering** - Task list items with inline formatting now render correctly; fixed checkbox alignment and replaced interactive checkboxes with non-interactive ASCII-style `[ ]`/`[x]` markers (interactive editing planned for v0.3.0)
- **macOS Intel support** ([#16](https://github.com/OlaProeis/Ferrite/issues/16)) - Fixed artifact naming for Intel Mac builds; separate x86_64 build via `macos-13` runner
- **Linux close button cursor flicker** - Fixed cursor rapidly switching between pointer/resize near window close button by adding title bar exclusion zone (35px) for north-edge resize detection and cursor caching

### Technical
- Added 7 new technical documentation files in `docs/technical/`
- Extended keyboard shortcut system with pre-render key consumption for move line operations

## [0.2.2] - 2025-01-11

### Added

#### CLI Features
- **Command-line file opening** ([#9](https://github.com/OlaProeis/Ferrite/issues/9)) - Open files directly: `ferrite file.md`, `ferrite file1.md file2.md`, or `ferrite ./folder/`
- **Version and help flags** ([#10](https://github.com/OlaProeis/Ferrite/issues/10)) - Support for `-V/--version` and `-h/--help` CLI arguments
- **Configurable log level** ([#11](https://github.com/OlaProeis/Ferrite/issues/11)) - New `log_level` setting in config.json with CLI override (`--log-level debug|info|warn|error|off`)

#### UX Improvements
- **Default view mode setting** ([#3](https://github.com/OlaProeis/Ferrite/issues/3)) - Choose default view mode (Raw/Rendered/Split) for new tabs in Settings > Appearance

### Fixed

#### Bug Fixes
- **CJK character rendering** ([#7](https://github.com/OlaProeis/Ferrite/issues/7)) - Multi-region CJK support (Korean, Chinese, Japanese) via system font fallback (PR [#8](https://github.com/OlaProeis/Ferrite/pull/8) by [@SteelCrab](https://github.com/SteelCrab) 🙏)
- **Undo/redo behavior** ([#5](https://github.com/OlaProeis/Ferrite/issues/5)) - Fixed scroll position reset, focus loss, double-press requirement, and cursor restoration
- **UTF-8 tree viewer crash** - Fixed string slicing panic when displaying JSON/YAML with multi-byte characters (Norwegian øæå, Chinese, emoji)
- **Misleading code folding UI** ([#12](https://github.com/OlaProeis/Ferrite/issues/12)) - Fold indicators now hidden by default (setting available for power users); removed confusing "Raw View" button from tree viewer toolbar

#### Performance
- **Large file editing** - Deferred syntax highlighting keeps typing responsive in 5000+ line files
- **Scroll performance** - Galley caching for instant syntax colors when scrolling via minimap

### Changed
- **Ubuntu 22.04 compatibility** ([#6](https://github.com/OlaProeis/Ferrite/issues/6)) - Release builds now target Ubuntu 22.04 for glibc 2.35 compatibility

### Documentation
- Added CLI reference documentation (`docs/cli.md`)
- Added technical docs for log level config, default view mode, and code folding UI changes

## [0.2.1] - 2025-01-10

### Added

#### Mermaid Diagram Enhancements
- **Sequence Diagram Control Blocks** - Full support for `loop`, `alt`, `opt`, `par`, `critical`, `break` blocks with proper nesting and colored labels
- **Sequence Activation Boxes** - `activate`/`deactivate` commands and `+`/`-` shorthand on messages for lifeline activation tracking
- **Sequence Notes** - `Note left/right/over` syntax with dog-ear corner rendering
- **Flowchart Subgraphs** - Nested `subgraph`/`end` blocks with semi-transparent backgrounds and direction overrides
- **Composite/Nested States** - State diagrams now support `state Parent { ... }` syntax with recursive nesting
- **Advanced State Transitions** - Color-coded transitions, smart anchor points, and cross-nesting-level edge routing

#### Layout Improvements
- **Flowchart Branching** - Sugiyama-style layered graph layout with proper side-by-side branch placement
- **Cycle Detection** - Back-edges rendered with smooth bezier curves instead of crossing lines
- **Smart Edge Routing** - Decision node edges exit from different points to prevent crossing
- **Edge Declaration Order** - Branch ordering now matches Mermaid's convention (later-declared edges go left)

### Fixed
- **Text Measurement** - Replaced character-count estimation with egui font metrics for accurate node sizing
- **Node Overflow** - Nodes dynamically resize to fit their labels without clipping
- **Edge Labels** - Long labels truncate with ellipsis instead of overflowing
- **User Journey Icons** - Fixed unsupported emoji rendering with text fallbacks

### Technical
- Extended `mermaid.rs` from ~4000 to ~6000+ lines
- Added technical documentation for all new features in `docs/technical/`

## [0.2.0] - 2025-01-09

### Added

#### Major Features
- **Split View** - Side-by-side raw editor and rendered preview with resizable divider and per-tab split ratio persistence
- **MermaidJS Native Rendering** - 11 diagram types rendered natively in Rust/egui (flowchart, sequence, pie, state, mindmap, class, ER, git graph, gantt, timeline, user journey)
- **Editor Minimap** - VS Code-style scaled preview with click-to-navigate, viewport indicator, and search highlights visible in minimap
- **Code Folding** - Fold detection for headings, code blocks, and lists with gutter indicators (▶/▼) and indentation-based folding for JSON/YAML
- **Live Pipeline Panel** - Pipe JSON/YAML content through shell commands with real-time output preview and command history
- **Zen Mode** - Distraction-free writing with centered text column and configurable column width
- **Git Integration** - Visual status indicators in file tree showing modified, added, untracked, and ignored files (using git2 library)
- **Auto-Save** - Configurable delay (default 15s), per-tab toggle, temp-file based for safety
- **Session Persistence** - Restore open tabs on restart with cursor position, scroll offset, view mode, and per-tab split ratio
- **Bracket Matching** - Highlight matching brackets `()[]{}<>` and markdown emphasis pairs `**` and `__` with theme-aware colors

### Fixed
- **Rendered Mode List Editing** - Fixed item index mapping issues, proper structural key hashing, and edit state consistency (Tasks 64-69)
- **Light Mode Contrast** - Improved text and border visibility with WCAG AA compliant contrast ratios, added separator between tabs and editor
- **Scroll Synchronization** - Bidirectional sync between Raw and Rendered modes with hybrid line-based/percentage approach and mode switch scroll preservation
- **Search-in-Files Navigation** - Click result now scrolls to match with transient highlight that auto-clears on scroll or edit
- **Search Panel Viewport** - Fixed top and bottom clipping issues with proper bounds calculation

### Changed
- **Tab Context Menu** - Reorganized icons with logical grouping for better visual clarity

### Technical
- Added ~4000 lines of Mermaid rendering code in `src/markdown/mermaid.rs`
- New modules: `src/vcs/` for git integration, `src/editor/minimap.rs`, `src/editor/folding.rs`, `src/editor/matching.rs`, `src/ui/pipeline.rs`, `src/config/session.rs`
- Comprehensive technical documentation for all major features in `docs/technical/`

### Deferred
- **Multi-cursor editing** (Task 72) - Deferred to v0.3.0, requires custom text editor implementation

## [0.1.0] - 2025-01-XX

### Added

#### Core Editor
- Multi-tab file editing with unsaved changes tracking
- Three view modes: Raw, Rendered, and Split (Both)
- Full undo/redo support per tab (Ctrl+Z, Ctrl+Y)
- Line numbers with scroll synchronization
- Text statistics (words, characters, lines) in status bar

#### Markdown Support
- WYSIWYG markdown editing with live preview
- Click-to-edit formatting for lists, headings, and paragraphs
- Formatting toolbar (bold, italic, headings, lists, links, code)
- Sync scrolling between raw and rendered views
- Syntax highlighting for code blocks (syntect)
- GFM (GitHub Flavored Markdown) support via comrak

#### Multi-Format Support
- JSON file editing with tree viewer
- YAML file editing with tree viewer
- TOML file editing with tree viewer
- Tree viewer features: expand/collapse, inline editing, path copying
- File-type aware adaptive toolbar

#### Workspace Features
- Open folders as workspaces
- File tree sidebar with expand/collapse
- Quick file switcher (Ctrl+P) with fuzzy matching
- Search in files (Ctrl+Shift+F) with results panel
- File system watching for external changes
- Workspace settings persistence (.ferrite/ folder)

#### User Interface
- Modern ribbon-style toolbar
- Custom borderless window with title bar
- Custom resize handles for all edges and corners
- Light and dark themes with runtime switching
- Document outline panel for navigation
- Settings panel with appearance, editor, and file options
- About dialog with version info
- Help panel with keyboard shortcuts reference
- Native file dialogs (open, save, save as)
- Recent files menu in status bar
- Toast notifications for user feedback

#### Export Features
- Export document to HTML file with themed CSS
- Copy as HTML to clipboard

#### Platform Support
- Windows executable with embedded icon
- Linux .desktop file for application integration
- macOS support (untested)

#### Developer Experience
- Comprehensive technical documentation
- Optimized release profile (LTO, symbol stripping)
- Makefile for common build tasks
- Clean codebase with zero clippy warnings

### Technical Details
- Built with Rust 1.70+ and egui 0.28
- Immediate mode GUI architecture
- Per-tab state management
- Platform-specific configuration storage
- Graceful error handling with fallbacks

---

## Version History

- **0.2.8** - Command palette, LSP integration (Phases 1-2), HarfRust text shaping, image/PDF viewer tabs, rendered view performance (AST caching, viewport culling, lazy estimation), per-frame O(N) elimination, background file loading, strict line breaks, middle-click close tabs, custom font crash prevention, 13 bug fixes
- **0.2.7** - Wikilinks & backlinks, Vim mode, welcome view, GitHub-style callouts, check for updates, Ctrl+Scroll zoom, keep text selected after formatting, frontmatter editor, format toolbar & side panel toggles, lazy CSV parsing, large file detection, single-instance protocol, MSI installer overhaul, Nix/NixOS flake support, Unicode complex script font loading (Phase 1), Linux portal dialog error handling, macOS .app bundle CI, task list checkbox rendering, flowchart refactoring, window control redesign, word-wrap scroll fixes, 20+ bug fixes
- **0.2.6.1** - First signed release, integrated terminal workspace, productivity hub, app.rs refactoring (~15 modules), CJK memory optimization, 8+ bug fixes
- **0.2.6** - Custom text editor with virtual scrolling (critical for large files), memory optimization fixes
- **0.2.5.3** - Windows code signing (SignPath), View Mode Segmented Control, app logo in title bar, extended syntax highlighting (100+ languages), syntax theme selector (25+ themes), list line break fix, table overflow fix, PowerShell rendering fix
- **0.2.5.2** - Delete Line shortcut, Move Line Up/Down, macOS file associations, Windows portable build, MSI installer, Linux RPM package, Linux window drag fix, I18n cleanup, new language support
- **0.2.5.1** - Multi-encoding support, memory optimization (250MB → 60-80MB), CPU optimization (10% → <1% idle), cursor positioning improvements, Intel Mac CPU fix, bug fixes
- **0.2.5** - Mermaid refactor, CSV viewer, semantic minimap, i18n, CJK indentation, custom fonts, snippets, TOC generation, drag-drop images, document statistics, main menu redesign, split view editing, bug fixes
- **0.2.3** - Editor productivity release (Go to Line, Duplicate Line, Move Line, Auto-close, Smart Paste, Line Width)
- **0.2.2** - Stability & CLI release (CJK fonts, undo/redo fixes, CLI arguments, default view mode)
- **0.2.1** - Mermaid diagram improvements (control blocks, subgraphs, nested states, improved layout)
- **0.2.0** - Major feature release (Split View, Mermaid, Minimap, Git integration, and more)
- **0.1.0** - Initial public release

[0.3.0]: https://github.com/OlaProeis/Ferrite/compare/v0.2.9...v0.3.0
[0.2.9]: https://github.com/OlaProeis/Ferrite/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/OlaProeis/Ferrite/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/OlaProeis/Ferrite/compare/v0.2.6.1...v0.2.7
[0.2.6.1]: https://github.com/OlaProeis/Ferrite/compare/v0.2.6...v0.2.6.1
[0.2.6]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5-hotfix.3...v0.2.6
[0.2.5.3]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5-hotfix.2...v0.2.5-hotfix.3
[0.2.5.2]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5-hotfix.1...v0.2.5-hotfix.2
[0.2.5.1]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5...v0.2.5-hotfix.1
[0.2.5]: https://github.com/OlaProeis/Ferrite/compare/v0.2.3...v0.2.5
[0.2.3]: https://github.com/OlaProeis/Ferrite/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/OlaProeis/Ferrite/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/OlaProeis/Ferrite/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/OlaProeis/Ferrite/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/OlaProeis/Ferrite/releases/tag/v0.1.0
