# Documentation Index

> **Index rules:** This file is a documentation map only. Do not add project history, task lists, architecture overviews, or session notes. When adding docs, append a single bullet with a one-line description under the appropriate section.

## Core Context
- `ai-context.md` - Ferrite v0.3.1 — Leftover Bug Fix & Editor Closeout (PRD) agent rules, architecture, and where things live.

## Technical Docs
- `technical/config/atomic-config-save.md` — Atomic `config.json.tmp` save; `config.json.bak` is the previous file; failed full load does not overwrite `.bak` until `note_user_settings_edit`.
- `technical/config/corrupt-config-rename.md` — Syntax-invalid `config.json` renamed to `config.json.corrupt-YYYYMMDD-HHMMSS` (keep 3); defaults loaded; `ConfigLoadOutcome::CorruptFileRenamed` and `AppState.corrupt_config_renamed_to`.
- `technical/config/tolerant-settings-load.md` — Per-field tolerant `config.json` load (ISS-9): `from_json_tolerant`, `TolerantSettingsLoad.dropped_field_count`, wired in `load_config_internal` after the file parses as JSON.
- `technical/config/config-load-toasts.md` — One-shot startup toasts for corrupt config rename and invalid-field recovery; `notification.config_*` locale keys; `show_pending_config_load_toasts` in `FerriteApp`.
- `technical/config/temp-data-dir-test-helper.md` — `with_temp_data_dir` isolates config/session tests; audit fails if a test calls `get_config_dir()` outside that helper.
- `technical/platform/v0.3.1-test-checklist.md` - Pre-tag manual QA checklist for all v0.3.1 features/fixes; references the `test_md/` fixtures and logs defects for the next PRD.
- `technical/platform/v0.3.1-leftover-closeout-qa.md` — Manual QA checklist for leftover closeout (#171/#174/#173/#175/#170/#149/#177/#168/#164/#154).
- `technical/platform/v0.3.1-mermaid-manual-qa.md` — Mermaid fidelity QA rows for #165/#166 (fit-to-pane, dense layout, CJK subgraph titles, autonumber).
- `technical/platform/v0.3.1-total-test.md` — Consolidated manual test checklist for the whole v0.3.1 release (merges all prior checklists).
- `../031-closeout-prd.md` (repo root) — final 0.3.1 orchestrator queue: 2026-09-08 review blockers (Tier A) + GitHub issues #175–#186 (Tier B/C) with fix instructions per task.
- `ai-workflow/prds/prd-v0.3.1-qa-closeout.md` — GrokBot Linux QA close-out: config robustness (ISS-9), undo coalescing (ISS-6), auto-close/keyboard trio, session/recovery hygiene, Settings & rendered polish; buckets A/B already fixed in `5325e9b`/`97347d3`. Not yet parsed into the orchestrator.
- `technical/platform/v0.3.1-minimal-release-test.md` — ~60-minute prioritized 0.3.0→0.3.1 manual pass (Tier 1 crash/data-loss, Tier 2 headline features, Tier 3 smoke); the realistic pre-tag gate.
- `technical/platform/iss-13-terminal-shell-settings-triage.md` — ISS-13: no Settings shell picker in `prd-v0.3.0`; Windows shell choice is terminal panel + menu; minimal-release row 29 wording corrected.
- `technical/platform/v0.3.1-assessment.md` — Final pre-release AI review: release blockers, bug findings by severity, docs gaps, verification matrix.
- `technical/platform/v0.3.1-grok-bot-qa-brief.md` — Hand-off brief for an autonomous Linux agent (Grok Bot): build `0.3.1-experimental`, run the automated gate + minimal/total checklists with Linux translations, extra rows for #176/#178/#183–#186, and the report format.
- `technical/platform/grokbot-qa-tier-b-status.md` — `grokbot/qa-report-v0.3.1-linux.md` Issues **Status** column; Tier B closeout (tasks 22–27) for ISS-12/13/14/18/24/29 with links to feature docs.
- `technical/platform/grokbot-qa-tier-a-rerun.md` — Ship gate: Windows agent preflight (`cargo test --bin ferrite`, clippy); Linux Xvfb re-run checklist for ISS-2/5/6/7/9 rows and QA report updates.
- `technical/platform/macos-open-with.md` — macOS Finder Open With (#154); winit 0.30 openURLs inject + AE queue; cold/warm tab routing via `handle_macos_open_paths`; CLI escape hatch.
- `technical/platform/single-instance.md` — Single-instance OS lock + `OK` handshake (#186); `--new-instance` / `allow_multiple_instances` skip; Windows `instance.port` sidecar.
- `technical/files/session-persistence.md` - Session save/restore, crash recovery, identity-gated recovery, and workspace file-watcher external reload (`Tab::apply_external_disk_reload`).
- `technical/files/recovery-conflict-banner.md` — Recovery-vs-disk banner hides when the live buffer matches the restore-time disk snapshot and shows again on a later edit (`recovery_conflict_banner_visible`).
- `technical/files/closed-tab-recovery.md` — Tab close deletes `recovery/<id>.json`, rewrites `session.recovery.json` to the live tab set, and prunes other recovery blobs (`discard_closed_tab_recovery`).
- `technical/files/clean-exit-recovery-prune.md` — Clean exit deletes `recovery/<id>.json` for open tabs that are no longer modified; dirty tabs keep blobs (`prune_non_dirty_recovery_blobs` in `on_exit`, ISS-23).
- `technical/files/line-ending-preservation.md` — Per-tab `LineEnding` detect/store/rejoin/save (#174); post-decode detection for UTF-16; dominant-ending policy for mixed files; CRLF/LF round-trip on preview edit and save.
- `technical/files/reload-from-disk.md` — Explicit Reload from Disk (#149); undo step, `pending_toast`, large-file rope resync via `content_version`, terminal/loading guards; `reload_tab_by_id` → `apply_external_disk_reload`.
- `technical/editor/document-stats-list-detection.md` — Character-aware list-marker detection in `DocumentStats::is_list_item` (fixes #171 multi-byte panic).
- `technical/editor/document-outline.md` — Outline panel: heading tree build, collapse state, click-to-jump.
- `technical/editor/outline-heading-unescape.md` — CommonMark backslash-escape unescape for outline heading titles (#166); PUA placeholders encode escape index for order-independent restore.
- `technical/editor/rope-line-index.md` — `(line, col)` → char index via ropey line breaks (`rope_line_col_to_char_index`); go-to-line, paste insert, line ops; not `split('\n')` alone.
- `technical/editor/find-in-rendered.md` — Find next/prev scrolls Rendered/Split preview to the match (#175); Ctrl+F select-all query; Raw unchanged.
- `technical/editor/search-in-files-offsets.md` — Search-in-Files char vs byte offset contract (#178); regex literal search on original line; CRLF-safe document offsets; whole-word UTF-8 advance in find/replace.
- `technical/editor/indent-selection.md` — Raw Tab/Shift+Tab block indent/outdent (#177); soft/hard tabs from settings; multi-cursor; rendered list Tab unchanged.
- `technical/editor/undo-coalescing.md` — Typing-run undo merge via `record_with_meta` (700 ms); raw editor via `record_external_edit_from_snapshot_coalesced`; focus/mode/caret-jump breaks; Split/Rendered fenced code blocks are one group on Done or focus-out.
- `technical/editor/auto-close-brackets.md` — Auto-pair, wrap, and skip-over; type-through and backtick/quote-run guards via `should_insert_auto_closer`; mermaid-fence and parenthesis caret regression tests.
- `technical/editor/primary-selection-paste.md` — Linux X11/Wayland primary selection (#183); 150 ms debounced publish; raw-editor middle-click paste; Linux-only setting.
- `technical/editor/spellcheck.md` — Spellcheck (#184): bundled en_US Hunspell, Markdown tokenizer, worker; Settings toggle (off by default); raw-only 300-squiggle cap, context-menu suggestions, status-bar loading.
- `technical/document-statistics.md` — Stats panel metrics, `DocumentStats` struct, and caching.
- `technical/markdown/rendered-edit-container-prefix.md` — Canonical `split_container_prefix` for Rendered/Split click-to-edit: seed without `>` / list markers, re-attach on commit; AST heading level and culled `end_line`.
- `technical/markdown/rendered-edit-source-range.md` - Span math for rendered paragraph/list commits (`block_replace_end_line`, `update_source_range`); fixes multi-line buffer duplication without bumping `source_epoch`.
- `technical/markdown/rendered-edit-flush.md` - `flush_rendered_edit_session` and app flush helpers; wires `commit_active` on view/tab/save/close/focus-loss so lone focused blocks reach `tab.content`.
- `technical/markdown/rendered-edit-session-paragraphs-lists.md` - Plain paragraph/list session model; Enter commit+exit, Shift+Enter soft break, buffer resync, and commit-on-switch behaviour.
- `technical/markdown/preview-code-block-undo.md` — Rendered/Split fenced code blocks (#167): buffer in `CodeBlockData` until Done/focus-out; one `record_source_commit` undo step per session (not per keystroke).
- `technical/markdown/gfm-table-column-alignment.md` - GFM table per-column alignment in rendered view; `table_cell_galley_paint_pos` compensates galley offset and block-shifts short text within cell width.
- `technical/markdown/video-embed-parsing.md` - Video embed AST parsing; explicit `{{video URL}}` with optional `width`/`height` params, allowlist, and `source_text` round-trip.
- `technical/markdown/video-embed-rendering.md` - Video embed rendered view; WebView relay path, thumbnail fallback, `video_display_size()` sizing, and drag-resize handle with source write-back.
- `technical/markdown/html-entities.md` — Decode named/numeric HTML entities in Rendered/Split (#173); `html_entities.rs`; entity-only `HtmlInline` → `Text` at parse time.
- `technical/markdown/local-image-assets.md` — Local drag-drop + Rendered preview (#164/#182); fail/mtime texture cache; `assets_dir_for_write` untitled abort; unique drop names; table-cell icon fallback; beside-doc `image.png`.
- `technical/markdown/clipboard-image-paste.md` — Paste OS clipboard image bytes to `./assets/` + `![](assets/…)` (#164); shared helpers with drag-drop; preview-lock / Raw-Split policy.
- `technical/markdown/smart-paste.md` — URL link/image smart paste; char-index selection replace (UTF-8 safe); paste/bracket event gates; clipboard image bytes; no remote fetch.
- `technical/markdown/autolink-rendered-styling.md` — Bare and `<https://…>` autolinks use the same link color/underline as `[text](url)` in formatted-block display (#ISS-26); `MarkdownLinkKind` + `parse_inline_markdown` autolink spans.
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
- `technical/ui/settings-search-initial-focus.md` — ISS-18: defer search focus on tab open, merge same-frame `Event::Text` after `request_focus`, skip global shortcuts during chord capture.
- `technical/ui/settings-recently-changed-list.md` — Recently changed cap (8), keyboard rebind exclusion, and minimal-release T2-25 manual QA (ISS-16).
- `technical/ui/settings-keyboard-chip-search-count.md` — Live-search Keyboard chip counts matching shortcut rows (command, category, binding), not the single registry entry (ISS-15).
- `technical/ui/repaint-after-deferred-ui-updates.md` — ISS-24: explicit `request_repaint` after debounced Find, Settings edits, and search/chips when `repaint_on_widget_change` is off; Xvfb/llvmpipe was primary repro.
- `technical/ui/settings-esc-open-shortcut.md` — Settings special tab: Ctrl+, via `open_settings_tab` (not `show_settings`); Esc closes tab when no text field/capture/dialog focus (ISS-12).
- `technical/ui/settings-all-view-scroll.md` — ISS-14: All chip shows recents + Essentials only; content `ScrollArea` uses `auto_shrink([false,false])` + `max_height(available_height())` so long lists scroll in the tab.
- `technical/ui/status-bar.md` — Bottom bar: cached `Tab::text_stats()` via `content_version` (ISS-25), line ops/`record_edit` bumps, Vim label gated on `settings.vim_mode`.
- `technical/ui/emoji-font-fallback.md` — OS emoji font fallback on Proportional/Monospace for document text (#168); Phosphor UI icons unchanged.
- `technical/ui/settings-search-emoji-font.md` — ISS-29: Settings search `ferrite-settings-input` font (bundled Noto Emoji before OS color); Phosphor family inherits Proportional fallbacks for epaint replacement glyphs.
- `technical/ui/rendered-raw-fonts.md` — Separate Rendered-view font (`rendered_font_family`, None = same as editor) and keyed `custom:<name>` registry via `FontSelection` (#176).
- `technical/terminal/terminal-nerd-fonts.md` — Terminal font picker (`terminal_font_family`) and lazy Nerd Font PUA fallback with cell clipping (#185).
- `technical/terminal/terminal-settings-validation.md` — Non-blocking red hints for terminal startup command, prompt regex lines, and custom sound path (ISS-19); validators in `terminal_validation.rs`.
- `technical/ui/ribbon-window-control.md` - New Window icon in the ribbon right cluster (beside Export/Terminal); title-bar Window menu removed; `RibbonAction::NewWindow` wiring unchanged.
- `technical/ui/raw-editor-context-menu.md` - Raw FerriteEditor right-click menu (Copy/Cut/Paste/Select All/Undo); `EditorWidget` + app-level undo via `EditorOutput.request_undo`.
- `technical/ui/tab-context-menu.md` - Tab strip right-click menu; i18n labels (`tab.new_tab`/`tab.close`), hover rows via background-layer fill, content-sized popup in `action_registry.rs`.
- `technical/ui/command-palette.md` - Searchable command launcher (Alt+Space); deferred dispatch, fuzzy search, palette-only commands via unbound defaults.
- `technical/ui/preview-lock.md` - Per-tab preview read-only flag; padlock overlay, command-palette **Lock editing** toggle, session persistence, markdown/CSV/tree gating.

## i18n
- `i18n/mermaid-toggle-locale-keys.md` — English base strings for the hover-revealed Fit/Native header toggle on inline Mermaid blocks.
