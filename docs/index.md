# Documentation Index

> **Index rules:** This file is a documentation map only. Do not add project history, task lists, architecture overviews, or session notes. When adding docs, append a single bullet with a one-line description under the appropriate section.

## Core Context
- `ai-context.md` - Ferrite v0.3.1 — Leftover Bug Fix & Editor Closeout (PRD) agent rules, architecture, and where things live.

## Technical Docs
- `technical/config/atomic-config-save.md` — Atomic `config.json.tmp` save; `config.json.bak` is the previous file; failed full load does not overwrite `.bak` until `note_user_settings_edit`.
- `technical/config/settings-debounced-save.md` — SI-4: Settings toggles flush to `config.json` after 500 ms debounce (`poll_settings_save_debounce`) or immediately when the Settings tab closes.
- `technical/config/corrupt-config-rename.md` — Syntax-invalid `config.json` renamed to `config.json.corrupt-YYYYMMDD-HHMMSS` (keep 3); defaults loaded; `ConfigLoadOutcome::CorruptFileRenamed` and `AppState.corrupt_config_renamed_to`.
- `technical/config/tolerant-settings-load.md` — Per-field tolerant `config.json` load (ISS-9): `from_json_tolerant`, `TolerantSettingsLoad.dropped_field_count`, wired in `load_config_internal` after the file parses as JSON.
- `technical/config/config-load-toasts.md` — One-shot startup toasts for corrupt config rename and invalid-field recovery; `notification.config_*` locale keys; `show_pending_config_load_toasts` in `FerriteApp`.
- `technical/config/temp-data-dir-test-helper.md` — `with_temp_data_dir` isolates config/session tests; audit fails if a test calls `get_config_dir()` outside that helper.
- `technical/config/shipped-language-locales.md` — ISS-56: all 10 `locales/*.yaml` in `Language::all()`; ja/zh_Hans title-bar and keyboard command keys; `language_all_matches_locale_files` + extended `locale_keys_exist_in_all_yaml`.
- `technical/platform/macos-open-with.md` — macOS Finder Open With (#154); winit 0.30 openURLs inject + AE queue; cold/warm tab routing via `handle_macos_open_paths`; CLI escape hatch.
- `technical/platform/single-instance.md` — Single-instance OS lock + `OK` handshake (#186); `--new-instance` / `allow_multiple_instances` skip; Windows `instance.port` sidecar.
- `technical/platform/qa-round2-clippy-hygiene.md` — Round 2 clippy: explicit `OpenOptions::truncate` on `instance.lock`; egui unit tests use `Context::run_ui` instead of deprecated `Context::run`.
- `technical/platform/forwarded-open-owning-viewport.md` — ISS-34: single-instance forwarded open raises `viewport_for_tab` (owning window), not last-focused; deferred X11 Focus after IPC.
- `technical/platform/linux-custom-title-bar-iss-54.md` — ISS-54: Linux X11 custom title bar double-click restore + drag-from-maximised; `PreMaximizeGeometry` temp storage; `title_bar_drag` state machine and deferred `StartDrag`.
- `technical/platform/emoji-rendering-limitation.md` — ISS-30: epaint outline-only emoji (no CBDT/COLR colour); U+FE0F zero-width via patched `vendor/epaint` + pre-warm filters in `src/fonts.rs`.
- `technical/files/session-persistence.md` - Session save/restore, crash recovery, identity-gated recovery, live-PID `session.lock` staleness, `--new-instance` session/config isolation, Quick note exit-dialog line, and workspace file-watcher external reload.
- `technical/files/session-restore-caret-only.md` — ISS-36: restore applies caret only (drops persisted selection); `Tab::apply_restored_caret_only`, `pending_restore_editor_reset`, `FerriteEditor::reset_transient_input_state` on first raw frame.
- `technical/files/iss-36-session-restore-typing-investigation.md` — ISS-36 SES-6 investigation: automated restore-then-type did not reproduce the buffer replacement; hardening from the caret, checkout, and anomalous-undo work; QA row needs a second sighting.
- `technical/files/recovery-conflict-banner.md` — Recovery-vs-disk banner hides when the live buffer matches the restore-time disk snapshot and shows again on a later edit (`recovery_conflict_banner_visible`).
- `technical/files/closed-tab-recovery.md` — Tab close deletes `recovery/<id>.json`, rewrites `session.recovery.json` to the live tab set, and prunes other recovery blobs (`discard_closed_tab_recovery`).
- `technical/files/clean-exit-recovery-prune.md` — Clean exit deletes `recovery/<id>.json` for open tabs that are no longer modified; dirty tabs keep blobs (`prune_non_dirty_recovery_blobs` in `on_exit`, ISS-23).
- `technical/files/line-ending-preservation.md` — Per-tab `LineEnding` detect/store/rejoin/save (#174); post-decode detection for UTF-16; dominant ending for new breaks/rejoins; mixed bytes preserved verbatim on save (ISS-50/EOL-9 closed by design).
- `technical/files/reload-from-disk.md` — Explicit Reload from Disk (#149); undo step, `pending_toast`, large-file rope resync via `content_version`, terminal/loading guards; `reload_tab_by_id` → `apply_external_disk_reload`.
- `technical/files/file-watcher-own-save.md` — GIT-3: workspace watcher ignores own saves via recorded `(path, mtime, len)` (`OwnWriteRegistry`); 5 s TTL; filter in `handle_file_watcher_events` before external reload.
- `technical/editor/document-stats-list-detection.md` — Character-aware list-marker detection in `DocumentStats::is_list_item` (fixes #171 multi-byte panic).
- `technical/editor/document-outline.md` — Outline panel: heading tree build, collapse state, click-to-jump.
- `technical/editor/outline-heading-unescape.md` — CommonMark backslash-escape unescape for outline heading titles (#166); PUA placeholders encode escape index for order-independent restore.
- `technical/editor/rope-line-index.md` — `(line, col)` → char index via ropey line breaks (`rope_line_col_to_char_index`); go-to-line, paste insert, line ops; not `split('\n')` alone.
- `technical/editor/find-in-rendered.md` — Find next/prev scrolls Rendered/Split preview to the match (#175); Ctrl+F select-all query; Raw unchanged.
- `technical/editor/search-in-files-offsets.md` — Search-in-Files char vs byte offset contract (#178); regex literal search on original line; CRLF-safe document offsets; whole-word UTF-8 advance in find/replace.
- `technical/editor/unicode-casefold-literal-search.md` — ISS-44: per-char `fold_char` / `find_all_case_insensitive` in `util/casefold.rs` for literal Find/Replace and Search-in-Files (İ↔i); ß≠ss; regex mode unchanged.
- `technical/editor/indent-selection.md` — Raw Tab/Shift+Tab block indent/outdent (#177, ISS-51); outdent strips one leading `\t` before spaces; soft/hard insert from settings; multi-cursor; rendered list Tab unchanged.
- `technical/editor/undo-coalescing.md` — Typing-run undo merge via `record_with_meta` (700 ms); raw editor via `record_external_edit_from_snapshot_coalesced`; focus/mode/caret-jump breaks; Split/Rendered fenced code blocks are one group on Done or focus-out.
- `technical/editor/auto-close-brackets.md` — Auto-pair, wrap, and skip-over; type-through and backtick/quote-run guards via `should_insert_auto_closer`; mermaid-fence and parenthesis caret regression tests.
- `technical/editor/ferrite-editor.md` — FerriteEditor widget architecture and input; Raw Enter is plain newline only—no list-marker continuation (ISS-49/STAT-3; Rendered uses `split_list_item`).
- `technical/editor/ferrite-editor-storage-checkout.md` — ISS-36: per-tab `Option<FerriteEditor>` slot (no `mem::replace` placeholder); `get_ferrite_editor_mut` returns `None` while checked out for `EditorWidget::show`.
- `technical/editor/anomalous-buffer-shrink-undo.md` — ISS-36: classify large one-frame buffer shrinks; error log + dedicated undo groups (`record_anomalous_buffer_shrink_groups`) so one Ctrl+Z restores pre-frame text; never blocks the edit.
- `technical/editor/primary-selection-paste.md` — Linux X11/Wayland primary selection (#183); 150 ms debounced publish; raw-editor middle-click paste; Linux-only setting.
- `technical/editor/spellcheck.md` — Spellcheck (#184): bundled en_US Hunspell, Markdown tokenizer (`MIN_SPELLCHECK_WORD_CHARS` 3, link text vs URL skips), worker; Settings toggle (off by default); raw-only 300-squiggle cap, context-menu suggestions, status-bar loading.
- `technical/editor/diagnostic-squiggle-wrap.md` — ISS-48: spell/LSP squiggles under wrapped lines via `rendering/squiggle.rs` (galley row `x_offset`, per-row segments); tests `cargo test --bin ferrite squiggle`.
- `technical/editor/diagnostic-hover-popup-guard.md` — SPL-3: suppress raw diagnostic hover tooltips (spell “Unknown word”, LSP) while a context menu/popup is open; `Context::any_popup_open` via `src/app/mod.rs`.
- `technical/editor/raw-table-column-guides.md` — ISS-61: Raw GFM table column guides per row from each line’s galley (`pos_from_cursor`); monospace keeps header continuous segments; wrap-off only with `TABLE_GUIDES_DISABLED_WRAP_REASON`; tests `cargo test --bin ferrite table_guides`.
- `technical/document-statistics.md` — Stats panel metrics, `DocumentStats` struct, and caching.
- `technical/markdown/rendered-edit-container-prefix.md` — Canonical `split_container_prefix` for Rendered/Split click-to-edit: seed without `>` / list markers, re-attach on commit; AST heading level and culled `end_line`.
- `technical/markdown/rendered-edit-source-range.md` - Span math for rendered paragraph/list commits (`block_replace_end_line`, `update_source_range`); fixes multi-line buffer duplication without bumping `source_epoch`.
- `technical/markdown/rendered-edit-flush.md` — `commit_pending_rendered_edits` before save, view switch, tab switch, session/recovery snapshot, and external-reload checks; status bar uses `rendered_session_pending_text` instead of committing every frame.
- `technical/markdown/rendered-session-write-through.md` — Per-frame write of dirty paragraph, list, heading, and table-cell sessions into `Tab::content`; code blocks mark unsaved without writing until commit.
- `technical/markdown/rendered-edit-session-paragraphs-lists.md` - Plain paragraph/list session model; Enter commit+exit, Shift+Enter soft break, buffer resync, and commit-on-switch behaviour.
- `technical/markdown/preview-code-block-undo.md` — Rendered/Split fenced code blocks (#167 / ISS-40): buffer in `CodeBlockData`; tab `break_undo_group` at Edit and after commit; one undo step per session; debug log only on commit.
- `technical/markdown/preview-code-block-textedit-identity.md` — ISS-40 part 2: scoped `BlockRef::CodeBlock` TextEdit id, `sync_codeblock_data_from_parsed` keeps draft while focused; one-shot `request_focus`; harness `code_block_ten_keystroke_frames_keep_focus_and_chars`.
- `technical/markdown/code-block-selection-copy.md` — Rendered code block mouse selection copy (CPY-2 / ISS-39): one `LayoutJob` + selectable label; `selected_code_text` maps galley char range to raw bytes; header Copy unchanged.
- `technical/markdown/gfm-table-column-alignment.md` - GFM table per-column alignment in rendered view; `table_cell_galley_paint_pos` compensates galley offset and block-shifts short text within cell width.
- `technical/markdown/video-embed-parsing.md` - Video embed AST parsing; explicit `{{video URL}}` with optional `width`/`height` params, allowlist, and `source_text` round-trip.
- `technical/markdown/video-embed-rendering.md` - Video embed rendered view; WebView relay path, thumbnail fallback, `video_display_size()` sizing, and drag-resize handle with source write-back.
- `technical/markdown/html-entities.md` — Decode named/numeric HTML entities in Rendered/Split (#173); `html_entities.rs`; entity-only `HtmlInline` → `Text` at parse time.
- `technical/markdown/github-html-block-subset.md` — Phase 1 block HTML (`<div align>`, `<details>`, `<br>`): parser coalescing, adjacent single-line div line remap/split (ISS-33), `extract_paragraph_content` inner slice, summary markup.
- `technical/markdown/github-html-subset.md` — GitHub HTML Phases 1–2 overview: sanitization, inline `<kbd>`/`<sup>`/`<sub>`/`<strong>`/`<em>`, sized `<img>`, renderer wiring.
- `technical/markdown/heading-inline-html-subset.md` — HTML-4: `<br>`/`<kbd>` (and related inline HTML) in ATX headings via `render_inline_content`; `heading_plain_text` for session seed; outline/folding strip.
- `technical/markdown/local-image-assets.md` — Local drag-drop + Rendered preview (#164/#182); fail/mtime texture cache; `assets_dir_for_write` untitled abort; unique drop names; table-cell icon fallback; beside-doc `image.png`.
- `technical/markdown/clipboard-image-paste.md` — Paste OS clipboard image bytes to `./assets/` + `![](assets/…)` (#164); shared helpers with drag-drop; preview-lock / Raw-Split policy.
- `technical/markdown/smart-paste.md` — URL link/image smart paste; char-index selection replace (UTF-8 safe); paste/bracket event gates; clipboard image bytes; no remote fetch.
- `technical/markdown/autolink-rendered-styling.md` — Bare and `<https://…>` autolinks use the same link color/underline as `[text](url)` in formatted-block display (#ISS-26); `MarkdownLinkKind` + `parse_inline_markdown` autolink spans.
- `technical/markdown/inline-display-to-raw.md` — `InlineDisplaySpan` + `parse_inline_display_spans`; `map_displayed_to_raw` / table-cell and `FormattedBlockLayout::raw_cursor_at` click-to-edit caret mapping (ISS-52).
- `technical/markdown/formatted-link-open.md` — Ctrl/Cmd+click and middle-click on links in paragraphs, list items, and table cells open the target before click-to-edit (ISS-42); plain click still edits.
- `technical/markdown/rendered-arrow-navigation.md` — Arrow Up/Down between rendered blocks (#170); boundary leave from TextEdit; preview-lock nav; Split-safe focus.
- `technical/mermaid/parser-safe-slicing.md` — Panic-free mermaid delimiter slices (`slice_between`); inverted brackets fall through; corpus over `test_md/test_mermaid_*.md` + `test_flowcharts.md`.
- `technical/mermaid/dense-flowchart-layout.md` — Sugiyama tuning for heavy `&` fan-out; forward lanes grouped per `(from, to)` and clamped to the node box; sibling coords `.max(margin)`.
- `technical/mermaid/flowchart-subgraph-header-parsing.md` — Bare multi-word/CJK subgraph titles match mermaid.js; warning when an edge still uses the former first-token id.
- `technical/mermaid/inline-fit-to-pane.md` — Inline Mermaid fit-to-pane: `set_transform_layer` (not Scene), parent-clip, one `EguiTextMeasurer` measure, popup `layout_width`.
- `technical/mermaid/inline-fit-native-toggle.md` — Per-block session-only Fit width / Native size toggle on inline Mermaid headers.
- `technical/mermaid/sequence-autonumber-parsing.md` — Sequence `autonumber [start [step]]` / `off`; first whitespace token only (`autonumbering` rejected).
- `technical/mermaid/sequence-autonumber-rendering.md` — Per-message `sequence_number` from parse-time autonumber state; mid-diagram `autonumber off`; badges at arrow start with label offset; `EguiTextMeasurer` for fit width.
- `technical/mermaid/cjk-label-measure-paint-parity.md` — ISS-59: `EguiTextMeasurer` via `painter().layout_no_wrap`; measurer/paint ≤1.1×; box vs text gap from padding + `text_width_factor`; `mermaid::measure` tests.
- `technical/mermaid/issue-165-fixtures.md` — Manual QA and regression repros for #165 (fit-to-pane, dense layout, CJK titles, autonumber).
- `technical/mermaid/grokbot-round2-fidelity-decisions.md` — GrokBot ISS-32/59/60 go/no-go: defer subgraph hierarchy to 0.3.2; ISS-59/60 → tasks 17/16; maps to mermaid PRD §12 and round 2 close-out §8.
- `technical/viewers/csv-viewer.md` - CSV/TSV rendered table viewer; inline cell editing, Tab/Shift+Tab + arrow keyboard navigation, lazy parsing for large files, and CSV-4 large-file banner predicate (`csv_large_file_raw_content_banner_visible`).
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

## Release QA (v0.3.1)

- `../031-closeout-prd.md` (repo root) — final 0.3.1 orchestrator queue: 2026-09-08 review blockers (Tier A) + GitHub issues #175–#186 (Tier B/C) with fix instructions per task.
- `ai-workflow/prds/prd-v0.3.1-qa-closeout.md` — GrokBot Linux QA close-out (round 1, ISS-1…29): config robustness (ISS-9), undo coalescing (ISS-6), auto-close/keyboard trio, session/recovery hygiene, Settings & rendered polish; buckets A/B fixed in `5325e9b`/`97347d3`, the rest by the orchestrator in `51f5a3a`.
- `ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md` — GrokBot QA round 2 (ISS-30…63): ISS-36 restore/typing data-loss investigation, rendered-edit commit timing & undo (ISS-39/40/41/52), links in blocks + adjacent `<div align>` (ISS-42/33), mermaid fidelity decisions (ISS-32/59/60), ISS-49/50 closed by design (task 18), search/spellcheck/locale minors, platform verification and GrokBot re-run gate; isolated rows fixed directly in `5eec844`. All 34 orchestrator tasks done; §9 GrokBot re-run still open.

### Manual test lists (shortest first)
- `technical/platform/v0.3.1-quick-manual-test.md` — **Start here.** ~30-minute Windows pass with a real keyboard: Part A = GrokBot blockers (ISS-1/3/7/9/36/41, CRLF/UTF-16), Part B = headline 0.3.1 features, Part C = glance; gate + hand-off to the GrokBot Linux re-run.
- `technical/platform/v0.3.1-minimal-release-test.md` — ~60-minute prioritized 0.3.0→0.3.1 manual pass (Tier 1 crash/data-loss, Tier 2 headline features, Tier 3 smoke); stale "KNOWN BUG until PRD lands" notes removed.
- `technical/platform/v0.3.1-total-test.md` — Exhaustive manual checklist for the whole v0.3.1 release (merges the three archived lists); the reference GrokBot ran.
- `technical/platform/v0.3.1-assessment.md` — 2026-08-08 pre-release AI review: release blockers (all fixed), bug findings by severity, docs gaps, verification matrix.
- `technical/platform/iss-13-terminal-shell-settings-triage.md` — ISS-13: no Settings shell picker in `prd-v0.3.0`; Windows shell choice is terminal panel + menu; minimal-release row 29 wording corrected.
- `technical/platform/archive/` — superseded lists kept for provenance only: `v0.3.1-test-checklist.md` (feature wave), `v0.3.1-leftover-closeout-qa.md` (#171/#174/#173/#175/#170/#149/#177/#168/#164/#154), `v0.3.1-mermaid-manual-qa.md` (#165/#166 rows). Do not run separately.

### GrokBot (autonomous Linux QA)
- `../grokbot/README.md` (repo root) — Layout of the GrokBot material (canonical report, re-run hand-off, unpacked batch notes, evidence zips, superseded snapshots), assessment of the run, fix-commit map, next steps.
- `../grokbot/New/qa-report-v0.3.1-linux.md` — Canonical QA report at `8e6ef50` (198 rows, ISS-1…63) with the **Status** column filled for round 1, `5eec844` and round 2; every fix is "GrokBot re-run pending".
- `../grokbot/New/rerun-v0.3.1-round2.md` — Hand this to GrokBot: rows to re-run per ISS, fix commit, expected result, run order.
- `technical/platform/v0.3.1-grok-bot-qa-brief.md` — Original hand-off brief for an autonomous Linux agent: build `0.3.1-experimental`, run the automated gate + minimal/total checklists with Linux translations, extra rows for #176/#178/#183–#186, and the report format.
- `technical/platform/grokbot-qa-tier-a-rerun.md` — Ship gate: Windows agent preflight (`cargo test --bin ferrite`, clippy); Linux Xvfb re-run checklist for ISS-2/5/6/7/9 rows and QA report updates.
- `technical/platform/grokbot-qa-tier-b-status.md` — Tier B closeout (round 1 tasks 22–27) for ISS-12/13/14/18/24/29/54 with links to feature docs; statuses now also in the canonical report.
- `technical/platform/grokbot-qa-round2-rerun-gate.md` — Round 2 close-out GrokBot handoff, QA report Status semantics, and §9 ship gate; FAILs become new issues.

## i18n
- `i18n/mermaid-toggle-locale-keys.md` — English base strings for the hover-revealed Fit/Native header toggle on inline Mermaid blocks.
