# Documentation Index

> **Index rules:** This file is a documentation map only. Do not add project history, task lists, architecture overviews, or session notes. When adding docs, append a single bullet with a one-line description under the appropriate section.

## Core Context
- `ai-context.md` - Ferrite v0.3.1 — Leftover Bug Fix & Editor Closeout (PRD) agent rules, architecture, and where things live.

## Technical Docs
- `technical/platform/v0.3.1-test-checklist.md` - Pre-tag manual QA checklist for all v0.3.1 features/fixes; references the `test_md/` fixtures and logs defects for the next PRD.
- `technical/platform/v0.3.1-leftover-closeout-qa.md` — Manual QA checklist for leftover closeout (#171/#174/#173/#175/#170/#149/#177/#168/#164/#154).
- `technical/platform/v0.3.1-mermaid-manual-qa.md` — Mermaid fidelity QA rows for #165/#166 (fit-to-pane, dense layout, CJK subgraph titles, autonumber).
- `technical/platform/v0.3.1-total-test.md` — Consolidated manual test checklist for the whole v0.3.1 release (merges all prior checklists).
- `../031-closeout-prd.md` (repo root) — final 0.3.1 orchestrator queue: 2026-09-08 review blockers (Tier A) + GitHub issues #175–#186 (Tier B/C) with fix instructions per task.
- `technical/platform/v0.3.1-minimal-release-test.md` — ~60-minute prioritized 0.3.0→0.3.1 manual pass (Tier 1 crash/data-loss, Tier 2 headline features, Tier 3 smoke); the realistic pre-tag gate.
- `technical/platform/v0.3.1-assessment.md` — Final pre-release AI review: release blockers, bug findings by severity, docs gaps, verification matrix.
- `technical/platform/v0.3.1-grok-bot-qa-brief.md` — Hand-off brief for an autonomous Linux agent (Grok Bot): build `0.3.1-experimental`, run the automated gate + minimal/total checklists with Linux translations, extra rows for #176/#178/#183–#186, and the report format.
- `technical/platform/macos-open-with.md` — macOS Finder Open With (#154); winit 0.30 openURLs inject + AE queue; cold/warm tab routing via `handle_macos_open_paths`; CLI escape hatch.
- `technical/platform/single-instance.md` — Single-instance OS lock + `OK` handshake (#186); `--new-instance` / `allow_multiple_instances` skip; Windows `instance.port` sidecar.
- `technical/files/session-persistence.md` - Session save/restore, crash recovery, identity-gated recovery, and workspace file-watcher external reload (`Tab::apply_external_disk_reload`).
- `technical/files/line-ending-preservation.md` — Per-tab `LineEnding` detect/store/rejoin/save (#174); post-decode detection for UTF-16; dominant-ending policy for mixed files; CRLF/LF round-trip on preview edit and save.
- `technical/files/reload-from-disk.md` — Explicit Reload from Disk (#149); undo step, `pending_toast`, large-file rope resync via `content_version`, terminal/loading guards; `reload_tab_by_id` → `apply_external_disk_reload`.
- `technical/editor/document-stats-list-detection.md` — Character-aware list-marker detection in `DocumentStats::is_list_item` (fixes #171 multi-byte panic).
- `technical/editor/document-outline.md` — Outline panel: heading tree build, collapse state, click-to-jump.
- `technical/editor/outline-heading-unescape.md` — CommonMark backslash-escape unescape for outline heading titles (#166); PUA placeholders encode escape index for order-independent restore.
- `technical/editor/rope-line-index.md` — `(line, col)` → char index via ropey line breaks (`rope_line_col_to_char_index`); go-to-line, paste insert, line ops; not `split('\n')` alone.
- `technical/editor/find-in-rendered.md` — Find next/prev scrolls Rendered/Split preview to the match (#175); Ctrl+F select-all query; Raw unchanged.
- `technical/editor/search-in-files-offsets.md` — Search-in-Files char vs byte offset contract (#178); regex literal search on original line; CRLF-safe document offsets; whole-word UTF-8 advance in find/replace.
- `technical/editor/indent-selection.md` — Raw Tab/Shift+Tab block indent/outdent (#177); soft/hard tabs from settings; multi-cursor; rendered list Tab unchanged.
- `technical/editor/primary-selection-paste.md` — Linux X11/Wayland primary selection (#183); 150 ms debounced publish; raw-editor middle-click paste; Linux-only setting.
- `technical/editor/spellcheck.md` — Spellcheck (#184): bundled en_US Hunspell, Markdown tokenizer, worker; Settings toggle (off by default); raw-only 300-squiggle cap, context-menu suggestions, status-bar loading.
- `technical/document-statistics.md` — Stats panel metrics, `DocumentStats` struct, and caching.
- `technical/markdown/rendered-edit-container-prefix.md` — Canonical `split_container_prefix` for Rendered/Split click-to-edit: seed without `>` / list markers, re-attach on commit; AST heading level and culled `end_line`.
- `technical/markdown/rendered-edit-source-range.md` - Span math for rendered paragraph/list commits (`block_replace_end_line`, `update_source_range`); fixes multi-line buffer duplication without bumping `source_epoch`.
- `technical/markdown/rendered-edit-flush.md` - `flush_rendered_edit_session` and app flush helpers; wires `commit_active` on view/tab/save/close/focus-loss so lone focused blocks reach `tab.content`.
- `technical/markdown/rendered-edit-session-paragraphs-lists.md` - Plain paragraph/list session model; Enter commit+exit, Shift+Enter soft break, buffer resync, and commit-on-switch behaviour.
- `technical/markdown/gfm-table-column-alignment.md` - GFM table per-column alignment in rendered view; `table_cell_galley_paint_pos` compensates galley offset and block-shifts short text within cell width.
- `technical/markdown/video-embed-parsing.md` - Video embed AST parsing; explicit `{{video URL}}` with optional `width`/`height` params, allowlist, and `source_text` round-trip.
- `technical/markdown/video-embed-rendering.md` - Video embed rendered view; WebView relay path, thumbnail fallback, `video_display_size()` sizing, and drag-resize handle with source write-back.
- `technical/markdown/html-entities.md` — Decode named/numeric HTML entities in Rendered/Split (#173); `html_entities.rs`; entity-only `HtmlInline` → `Text` at parse time.
- `technical/markdown/local-image-assets.md` — Local drag-drop + Rendered preview (#164/#182); fail/mtime texture cache; `assets_dir_for_write` untitled abort; unique drop names; table-cell icon fallback; beside-doc `image.png`.
- `technical/markdown/clipboard-image-paste.md` — Paste OS clipboard image bytes to `./assets/` + `![](assets/…)` (#164); shared helpers with drag-drop; preview-lock / Raw-Split policy.
- `technical/markdown/smart-paste.md` — URL link/image smart paste; char-index selection replace (UTF-8 safe); paste/bracket event gates; clipboard image bytes; no remote fetch.
- `technical/markdown/rendered-arrow-navigation.md` — Arrow Up/Down between rendered blocks (#170); boundary leave from TextEdit; preview-lock nav; Split-safe focus.
- `technical/mermaid/parser-safe-slicing.md` — Panic-free mermaid delimiter slices (`slice_between`); inverted brackets fall through; corpus over `test_md/test_mermaid_*.md` + `test_flowcharts.md`.
- `technical/mermaid/dense-flowchart-layout.md` — Sugiyama tuning for heavy `&` fan-out; forward lanes grouped per `(from, to)` and clamped to the node box; sibling coords `.max(margin)`.
- `technical/mermaid/flowchart-subgraph-header-parsing.md` — Bare multi-word/CJK subgraph titles match mermaid.js; warning when an edge still uses the former first-token id.
- `technical/mermaid/inline-fit-to-pane.md` — Inline Mermaid fit-to-pane: `set_transform_layer` (not Scene), parent-clip, one `EguiTextMeasurer` measure, popup `layout_width`.
- `technical/mermaid/inline-fit-native-toggle.md` — Per-block session-only Fit width / Native size toggle on inline Mermaid headers.
- `technical/mermaid/sequence-autonumber-parsing.md` — Sequence `autonumber [start [step]]` / `off`; first whitespace token only (`autonumbering` rejected).
- `technical/mermaid/sequence-autonumber-rendering.md` — Autonumber badges at arrow start (`from_x + dir*12`); `saturating_add` step; `EguiTextMeasurer` for fit width.
- `technical/mermaid/issue-165-fixtures.md` — Manual QA and regression repros for #165 (fit-to-pane, dense layout, CJK titles, autonumber).
- `technical/viewers/csv-viewer.md` - CSV/TSV rendered table viewer; inline cell editing, Tab/Shift+Tab + arrow keyboard navigation, and lazy parsing for large files.
- `technical/ui/settings-search-redesign.md` — Search-first Settings: stable `id_salt` per entry, frozen overview recents, Ctrl+F focus gate, sanitized `MAX_RECENTLY_CHANGED` (8).
- `technical/ui/emoji-font-fallback.md` — OS emoji font fallback on Proportional/Monospace for document text (#168); Phosphor UI icons unchanged.
- `technical/ui/rendered-raw-fonts.md` — Separate Rendered-view font (`rendered_font_family`, None = same as editor) and keyed `custom:<name>` registry via `FontSelection` (#176).
- `technical/terminal/terminal-nerd-fonts.md` — Terminal font picker (`terminal_font_family`) and lazy Nerd Font PUA fallback with cell clipping (#185).
- `technical/ui/ribbon-window-control.md` - New Window icon in the ribbon right cluster (beside Export/Terminal); title-bar Window menu removed; `RibbonAction::NewWindow` wiring unchanged.
- `technical/ui/raw-editor-context-menu.md` - Raw FerriteEditor right-click menu (Copy/Cut/Paste/Select All/Undo); `EditorWidget` + app-level undo via `EditorOutput.request_undo`.
- `technical/ui/tab-context-menu.md` - Tab strip right-click menu; i18n labels (`tab.new_tab`/`tab.close`), hover rows via background-layer fill, content-sized popup in `action_registry.rs`.
- `technical/ui/command-palette.md` - Searchable command launcher (Alt+Space); deferred dispatch, fuzzy search, palette-only commands via unbound defaults.
- `technical/ui/preview-lock.md` - Per-tab preview read-only flag; padlock overlay, command-palette **Lock editing** toggle, session persistence, markdown/CSV/tree gating.

## i18n
- `i18n/mermaid-toggle-locale-keys.md` — English base strings for the hover-revealed Fit/Native header toggle on inline Mermaid blocks.
