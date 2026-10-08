# Ferrite v0.3.1 — Leftover Bug Fix & Editor Closeout (PRD) - AI Context

## Rules (DO NOT UPDATE)
- **Implementation sessions:** follow **Implementation Phase Rules** below only.
- **Update sessions:** follow **Update Phase Rules** below only when you receive the update handover prompt.
- Only do the task specified; do not start the next task or go over scope.
- Run `cargo test` after code changes to verify tests pass.
- Follow existing code patterns and conventions.
- Use Context7 MCP to fetch library documentation when needed (resolve library ID first, then fetch docs). Task operations use **`cyclopsctl tasks` CLI only**.

## Implementation Phase Rules
When working from **`current-handover-prompt.md`** (the normal case for every cyclopsctl task cycle):

- **DO:** Implement and test only the current parent task described in the handover.
- **DO:** Run `cargo test` before finishing; meet the task test strategy.
- **DO:** Use Context7 MCP for up-to-date library documentation when implementing unfamiliar APIs or frameworks.
- **DO NOT:** Read `prd.md` during cyclopsctl cycles — task scope, details, and test strategy are already in this handover.
- **DO NOT:** Mark tasks done or change task status.
- **DO NOT:** Run `cyclopsctl tasks next`, rewrite `current-handover-prompt.md`, or edit `ai-context.md`.
- **DO NOT:** Create or update docs in `docs/`, or edit `docs/index.md`.
- **DO NOT:** Edit `update-handover-prompt.md`.

Task completion and all documentation updates happen only in the **update phase** (`update-handover-prompt.md`).

## Update Phase Rules
When `update-handover-prompt.md` is provided (after implementation in the same agent session):

- **DO:** Follow every step in `update-handover-prompt.md`.
- **DO:** Use `cyclopsctl tasks list pending --project-root G:\DEV\markDownNotepad` and pick the **lowest numeric parent id** for the next handover — not `cyclopsctl tasks next` (priority can skip ahead).
- **DO:** Rewrite `current-handover-prompt.md` for the **next** task (this is the only time that file may change).
- **DO:** Update `ai-context.md` project memory per update handover step 2 (key facts only, not a changelog).
- **DO:** Use `cyclopsctl tasks` with `--project-root G:\DEV\markDownNotepad` for all task commands (see Environment in the handover).
- **DO:** Document by feature (e.g., `auth-layer.md`), not by task number; update `docs/index.md` when adding documentation.
- **DO NOT:** Re-implement or extend the task you just finished unless tests are broken.

## Conventions
- **Documentation:** Feature-based names in `docs/` (e.g., `auth-layer.md`), not `task-1.md`. Update `docs/index.md` in the update phase only.
- **Tasks:** `cyclopsctl tasks` CLI only from agents.

## Handover Files
| File | Who may edit | When |
|------|----------------|------|
| `current-handover-prompt.md` | Update-phase agent only | After implementation |
| `update-handover-prompt.md` | Human / template only | Never edited by agents |
| `ai-context.md` | Update-phase agent only | Every update phase — project memory bullets (see update handover step 2) |

## Tech Stack
cyclopsctl tasks CLI

## Architecture & Data Model
See `prd.md` for product architecture. This file captures agent workflow rules and where project artifacts live.

## Project Memory
- `DocumentStats::is_list_item` (`src/editor/stats.rs`) must use `chars()` iteration and `Option` matching — never byte-length gates or `.unwrap()` on `chars().nth(n)`; multi-byte first characters caused #171.
- List detection regression tests live in the `stats` module: `test_is_list_item_multibyte_no_panic`, `test_is_list_item_ascii_markers`, `test_doc_stats_multibyte_line_start_no_panic`.
- Detail: [`docs/technical/editor/document-stats-list-detection.md`](docs/technical/editor/document-stats-list-detection.md).
- `Tab.line_ending` (`LineEnding::{Lf,Crlf}` in `src/state.rs`) is set on every load path via `detect_from_content` **after decode** — never `detect_from_bytes` on raw UTF-16 bytes (CRLF misdetects as LF); new tabs use `LineEnding::platform_default()`. Open uses raw `fs::read` bytes — never `lines().join("\n")` on load.
- Buffer rewrites must use `LineEnding::split_lines` + `join_lines` (or `tab.line_ending.join_lines` when a `Tab` is available) — never `str::lines()` + `join("\n")`. Line ops normalize mixed-EOL files to the tab’s dominant ending. Save writes `encode_content()` bytes with no EOL force.
- Detail: [`docs/technical/files/line-ending-preservation.md`](docs/technical/files/line-ending-preservation.md).
- Entity-only `HtmlInline` (no `<` tags) decodes via `html_entities::decode_html_entities` in `transform_inline_html_siblings` → `Text`; unknown named entities stay literal. Tag-bearing fragments remain `HtmlInline` / `«HTML»`.
- Detail: [`docs/technical/markdown/html-entities.md`](docs/technical/markdown/html-entities.md).
- Find in Rendered/Split (#175): `UiState.scroll_to_match` → `MarkdownEditor::scroll_to_search_match` (prefer `tab.rendered_line_mappings` Y, else `search_match_source_line` → `scroll_to_line`); do not let sync `pending_ratio` overwrite a find jump. Ctrl+F uses `find_query_select_all_char_range`. Raw still uses `EditorWidget` `SearchHighlights.scroll_to_match`.
- Detail: [`docs/technical/editor/find-in-rendered.md`](docs/technical/editor/find-in-rendered.md).
- Rendered Arrow Up/Down (#170): `rendered_arrow_nav::collect_navigable_blocks` + `adjacent_navigable`; `editor.rs` queues boundary leave from TextEdit row cache, applies block jumps at end of frame; preview lock still navigates; do not steal keys when Raw/find focused or pointer is over another Split pane. `FormattedListItem.item` is 0-based child index (not display number). Last block memory: `arrow_nav_sel_key` → `rendered_editor_id(tab.id).with("ferrite_rendered_arrow_nav_sel")`; cleared in `cleanup_rendered_editor_memory`.
- Detail: [`docs/technical/markdown/rendered-arrow-navigation.md`](docs/technical/markdown/rendered-arrow-navigation.md).
- Reload from Disk (#149): `apply_external_disk_reload` → `record_edit` + `increment_content_version` + `mark_saved` (one Ctrl+Z step); errors/untitled use `AppState::pending_toast` drained in `FerriteApp` with `get_app_time()` — never `show_toast(..., 0.0)`. Skip when terminal focused, `is_loading()`, or special/viewer tabs. Raw rope: `FerriteEditorStorage.content_versions` checked before large-file hash skip.
- Detail: [`docs/technical/files/reload-from-disk.md`](docs/technical/files/reload-from-disk.md).
- Raw Tab/Shift+Tab (#177): `FerriteEditor::handle_tab_key` — multi-line selection → `block_indent_or_outdent`; empty caret → insert `indent_string` or line outdent; `EditorWidget::tab_settings` wires `use_spaces`/`tab_size`. Helpers in `src/editor/ferrite/indent.rs`. Do not change rendered list Tab in `markdown/editor.rs`.
- Detail: [`docs/technical/editor/indent-selection.md`](docs/technical/editor/indent-selection.md).
- Emoji fallback (#168): `cached_emoji_font()` (`OnceLock`) + `validate_emoji_font_bytes` (TTC index 0) in `src/fonts.rs`; `register_emoji_font_fallback` appends `FONT_EMOJI` to Proportional/Monospace after JetBrains, before CJK; skip silently if missing. Document text only — Phosphor stays UI icons.
- Detail: [`docs/technical/ui/emoji-font-fallback.md`](docs/technical/ui/emoji-font-fallback.md).
- `(line, col)` → char index for paste/line ops/go-to-line: `rope_line_col_to_char_index` in `src/string_utils.rs` (ropey breaks: `\n`, `\r`, U+2028, form-feed, etc.) — never `split('\n')` alone; `cursor_byte_from_line_col` / `insert_markdown_at_line_col` use it.
- Detail: [`docs/technical/editor/rope-line-index.md`](docs/technical/editor/rope-line-index.md).
- Clipboard image paste (#164): `try_consume_clipboard_image_paste` after URL smart-paste; `arboard` RGBA → PNG via `encode_rgba_to_png` / `save_clipboard_rgba_to_assets`; non-empty text `Event::Paste` wins; skip Rendered+preview-lock; shared `insert_asset_image_markdown` with drag-drop. No remote fetch.
- Detail: [`docs/technical/markdown/clipboard-image-paste.md`](docs/technical/markdown/clipboard-image-paste.md).
- Local images (#164/#182): `resolve_local_image_path` (beside-doc `image.png`/`./image.png`, MarkText `./assets/`, bare filename → `assets/`); `render_image` caches `Failed { msg, mtime, at }` (retry on mtime change or 2 s backoff, one `warn`) and textures by path+mtime+len (meta ≤500 ms). Writes use `assets_dir_for_write` (`None` → toast `image_paste.save_document_first`, no CWD `assets/`); drop/paste go through `unique_asset_dest_path` after `flush_active_rendered_session`. Table cells: icon+alt+URL tooltip only.
- Detail: [`docs/technical/markdown/local-image-assets.md`](docs/technical/markdown/local-image-assets.md).
- macOS Open With (#154): reception in `src/platform/macos.rs` (no custom NSApplicationDelegate on winit 0.30); cold → `initial_paths` / `open_initial_paths`; warm → update loop `handle_macos_open_paths` → shared `open_os_paths_in_focused_window` with single-instance IPC. Escape hatch: CLI / `open -a Ferrite`.
- Detail: [`docs/technical/platform/macos-open-with.md`](docs/technical/platform/macos-open-with.md).
- Search-in-Files (#178): literal mode uses one escaped `RegexBuilder` on the original line (no `to_lowercase()` copy); `SearchMatch.char_offset`/`match_len` are **char** counts, `match_start`/`match_end` are **byte** offsets into `line_content`; document offsets via `split_inclusive('\n')` + `line_segment_without_eol` (CRLF = 2). `FindState::advance_past_char_at` for whole-word rejection — never `match_start + 1` on UTF-8.
- Detail: [`docs/technical/editor/search-in-files-offsets.md`](docs/technical/editor/search-in-files-offsets.md).
- Mermaid delimiter slices must use `parse_util::slice_between` (`find` open + `rfind` close; `None` when `start + open.len() > end`) — never raw `find`/`rfind` + slice. On `None`, fall through to the next shape / plain-id path. `validate_mermaid_source` is not `catch_unwind`-wrapped; release `panic = abort` relies on parser panic-freedom.
- Detail: [`docs/technical/mermaid/parser-safe-slicing.md`](docs/technical/mermaid/parser-safe-slicing.md).
- Rendered click-to-edit in quotes/lists: `split_container_prefix` is the only prefix split — seed strips it (`extract_paragraph_content` / `seed_session_block_text`); commit re-attaches it. Heading level from `heading_level_for_commit` (AST Heading node, else `#` after prefix). Culled commits use `ast_end_line_for_session_block` (edit_state, else AST `end_line`); do not bump `source_epoch`.
- Detail: [`docs/technical/markdown/rendered-edit-container-prefix.md`](docs/technical/markdown/rendered-edit-container-prefix.md).
- Smart-paste URL-over-selection: FerriteEditor `primary_selection().ordered()` → `cursor_to_char_pos` char range → `char_index_to_byte_index` → `replace_selection_with_link`; never byte-window `find`. `consume_smart_paste` returns early via `events_indicate_paste` (no per-frame `tab.content.clone()`).
- Detail: [`docs/technical/markdown/smart-paste.md`](docs/technical/markdown/smart-paste.md).
- Auto-close pre-render: `handle_auto_close_pre_render` clones content only when `events_indicate_auto_close` sees a bracket `Event::Text` this frame.
- Settings search-first: wrap every entry in `ui.scope_builder(UiBuilder::new().id_salt(("setting", entry.id)))`; freeze `overview_recent_ids` for the visit (still persist immediately). Exclude `KEYBOARD_SHORTCUTS_ID` from recents. Ctrl+F: skip while `key_capture` is set; consume only if search is focused or nothing is. `sanitize()` prunes unknown ids then truncates to `MAX_RECENTLY_CHANGED` (8).
- Detail: [`docs/technical/ui/settings-search-redesign.md`](docs/technical/ui/settings-search-redesign.md).
- Single-instance (#186): `load_config()` before `try_acquire_instance`; `allow_multiple_instances` or `--new-instance` → `InstanceAcquire::Independent` (no lock/pid/port, `instance_listener = None`). Primary: `File::try_lock()` on `instance.lock`, write port, hold the handle. Windows exclusive lock blocks reads — also write `instance.port`. Secondary waits for `OK\n` (1 s); missing OK → stale, become primary. Forward via `canonicalize_forward_path` + `normalize_path` (secondary CWD).
- Detail: [`docs/technical/platform/single-instance.md`](docs/technical/platform/single-instance.md).
- Rendered vs Raw fonts (#176): `Settings.rendered_font_family: Option<EditorFont>` (`None` = same as `font_family`). Custom faces are keyed `"custom:<name>"` (not `FONT_CUSTOM`); only the **editor** custom is prepended to `Proportional`. Rebuild paths take `FontSelection { editor, rendered, terminal }` from `FontSelection::from_settings`.
- Detail: [`docs/technical/ui/rendered-raw-fonts.md`](docs/technical/ui/rendered-raw-fonts.md).
- Terminal fonts (#185): `Settings.terminal_font_family: Option<String>` (`None` = JetBrains Mono) drives `FontSelection.terminal` and named family `FONT_TERMINAL` (`ferrite-terminal`). Widget uses `font_family` + `painter.with_clip_rect(cell_rect.expand2(vec2(0.0, 1.0)))` — do not revert to `FontId::monospace`. Settings picker reuses `custom_font_picker`.
- Lazy Nerd fallback: `Terminal::poll` sets `NERD_FONT_REQUESTED` when `needs_nerd_font` sees PUA (U+E000–U+F8FF, U+F0000–U+FFFFD, Powerline U+E0A0–U+E0D4); after `poll_all` call `ensure_nerd_font_loaded` (rebuild + `bump_font_generation` + `schedule_prewarm`, keep CJK/complex prefs). Missing font: one debug log, no toast. `register_nerd_font_fallback` pushes `FONT_NERD` onto Monospace and `ferrite-terminal` after emoji.
- Detail: [`docs/technical/terminal/terminal-nerd-fonts.md`](docs/technical/terminal/terminal-nerd-fonts.md).
- Linux primary selection (#183): `platform::set_primary` / `get_primary` via arboard `LinuxClipboardKind::Primary` (one `thread_local` `Clipboard`; never `.wait()`); stubs elsewhere. Publish only when `middle_click_paste` and `primary_publish_decision` says so (150 ms, pointer up). Middle-click: `get_primary` → `set_cursor` + `insert_text_at_all_cursors`; skip fold gutter and Vim non-insert. Drag must use `PointerButton::Primary` only — do not change tab-strip/terminal/rendered middle-click.
- Detail: [`docs/technical/editor/primary-selection-paste.md`](docs/technical/editor/primary-selection-paste.md).
- Inline Mermaid fit (#165): never `Scene::show` for static fit — `set_transform_layer` + clip `to_global.inverse() * (parent_clip ∩ outer_rect)`; no `register_pan_and_zoom`. `measure_mermaid_diagram(..., &dyn TextMeasurer)` once in `render_validated_mermaid` with `EguiTextMeasurer`; pass `natural_size` down; popup gets `layout_width`. Forward lanes group by `(from, to)` and clamp via `forward_lane_pixel_offset`; autonumber badge at `from_x + dir*12` with `saturating_add`; match `autonumber` on the first whitespace token only.
- Detail: [`docs/technical/mermaid/inline-fit-to-pane.md`](docs/technical/mermaid/inline-fit-to-pane.md), [`sequence-autonumber-rendering.md`](docs/technical/mermaid/sequence-autonumber-rendering.md), [`dense-flowchart-layout.md`](docs/technical/mermaid/dense-flowchart-layout.md).
- Spellcheck (#184): feature `spellcheck` (default on); `Dictionary::load` only on the worker thread; `words_to_check` returns **char** offsets; `diagnostics_for(tab, version)` is `None` when stale; personal words at `<config_dir>/spellcheck/user-words.txt`. `AppState.spellcheck` stays `None` until `settings.spellcheck_enabled` is on — create/drop in `sync_spellcheck_service`; **take** the `Option` out of `AppState` before the raw `&mut Tab` borrow. Visible window ±60, 250 ms debounce, cap 300 nearest viewport; raw only (no Split preview squiggles). Context menu: `replace_word_range` is one undo step; `AddWord`/`IgnoreWord` drop matching cached diags immediately. Do not call `Dictionary::new` on the UI thread.
- Detail: [`docs/technical/editor/spellcheck.md`](docs/technical/editor/spellcheck.md).

## Where Things Live
| Want to... | Look in... |
|------------|------------|
| Product requirements | `prd.md` |
| Current implementation handover | `current-handover-prompt.md` |
| Post-task update rules | `update-handover-prompt.md` |
| Documentation map | `docs/index.md` |
| Tasks and complexity | `.cyclopsctl/tasks/tasks.json`, `.cyclopsctl/reports/complexity-report.json` |
| Cyclopsctl config | `cyclopsctl.toml` |
| Rendered arrow nav helpers | `src/markdown/rendered_arrow_nav.rs` |
| Raw Tab indent/outdent helpers | `src/editor/ferrite/indent.rs` |
| Local image assets helpers (drop/paste) | `src/app/file_ops.rs` (`insert_asset_image_markdown`, `unique_asset_dest_path`, `save_clipboard_rgba_to_assets`) |
| Local image path resolution / write gate | `src/path_utils.rs` (`assets_dir_for_paths`, `assets_dir_for_write`, `resolve_local_image_path`) |
| Image fail + texture cache | `src/markdown/editor.rs` (`ImageLoadResult`, `load_image_with_fail_cache`, `render_image`) |
| macOS Open With path reception | `src/platform/macos.rs` (`init_app_delegate`, `take_opened_files`, `bind_egui_context`) |
| macOS warm Open With → tabs | `src/app/file_ops.rs` (`handle_macos_open_paths`, `open_os_paths_in_focused_window`) |
| Search-in-Files offset helpers | `src/ui/search.rs` (`line_segment_without_eol`, `SearchPanel::search`) |
| Find whole-word UTF-8 advance | `src/editor/find_replace.rs` (`FindState::advance_past_char_at`) |
| Safe Mermaid delimiter slicing | `src/markdown/mermaid/parse_util.rs` (`slice_between`) |
| Rendered edit container prefix | `src/markdown/editor.rs` (`split_container_prefix`, `heading_level_for_commit`, `ast_end_line_for_session_block`, `continuation_prefix`) |
| Smart-paste UTF-8 link replace + paste gates | `src/app/input_handling.rs` (`replace_selection_with_link`, `events_indicate_paste`, `events_indicate_auto_close`, `cursor_byte_from_line_col`) |
| Settings registry / recents | `src/ui/settings/registry.rs` (`note_recently_changed`, `sanitize_recently_changed`, `MAX_RECENTLY_CHANGED`) |
| Settings panel shell (search, chips, overview snapshot, Ctrl+F) | `src/ui/settings/mod.rs` (`render_entry`, `show_overview`, `end_frame`) |
| Reload from disk + deferred toasts | `src/state.rs` (`request_reload_from_disk`, `reload_tab_by_id`, `apply_external_disk_reload`, `pending_toast`); drain in `src/app/mod.rs` |
| Raw editor content-version resync | `src/editor/widget.rs` (`FerriteEditorStorage.content_versions`, `editor_needs_content_sync`) |
| Single-instance lock / handshake / skip | `src/single_instance.rs` (`try_acquire_instance`, `InstanceAcquire`, `canonicalize_forward_path`); `--new-instance` + early `load_config()` in `src/main.rs` |
| FontSelection + custom-font registry | `src/fonts.rs` (`FontSelection::from_settings`, `custom_font_id`, `register_custom_fonts`, `revert_unloaded_custom_fonts`, `create_font_definitions_inner`) |
| Rendered vs editor font wiring | `src/app/central_panel.rs` (`rendered_font_family`); `MarkdownEditor::with_settings`; Settings `appearance.rendered_font_family` |
| Shared custom-font ComboBox | `src/ui/settings/appearance.rs` (`custom_font_picker`) |
| Terminal family + lazy Nerd Font | `src/fonts.rs` (`FONT_TERMINAL`, `needs_nerd_font`, `ensure_nerd_font_loaded`, `NERD_FONT_LOADED` / `NERD_FONT_REQUESTED`) |
| Terminal PUA request + cell clip | `src/terminal/mod.rs` (`Terminal::poll`); `src/terminal/widget.rs` (`font_family`, clip rect); `src/ui/terminal_panel.rs` (after `poll_all`) |
| Terminal font setting | `src/config/settings.rs` (`terminal_font_family`); `src/ui/settings/terminal.rs` (`render_font_family`) |
| Linux primary selection | `src/platform/primary_selection.rs` (`set_primary`, `get_primary`, `primary_publish_decision`) |
| Raw middle-click paste / publish | `src/editor/ferrite/editor.rs` (`maybe_publish_primary_selection`, `middle_clicked` path); `EditorWidget::middle_click_paste` |
| Middle-click paste setting | `src/config/settings.rs` (`middle_click_paste`); `src/ui/settings/editor.rs` + `registry.rs` (`#[cfg(target_os = "linux")]`) |
| Inline Mermaid fit transform layer | `src/markdown/widgets.rs` (`show_inline_mermaid_diagram`, `render_validated_mermaid`) |
| Mermaid measure + real-font metrics | `src/markdown/mermaid/mod.rs` (`measure_mermaid_diagram`); `src/markdown/mermaid/text.rs` (`EguiTextMeasurer`, `is_estimated`) |
| Forward-edge lane clamp | `src/markdown/mermaid/flowchart/render/edges.rs` (`compute_forward_edge_lanes`, `forward_lane_pixel_offset`) |
| Sequence autonumber badge / token | `src/markdown/mermaid/sequence.rs` (`draw_autonumber_badge`, `is_autonumber_directive`) |
| Mermaid block memory keys | `src/markdown/editor.rs` (`mermaid_block_id` / `mermaid_state_id`); popup `layout_width` in `src/ui/mermaid_popup.rs` |
| Rope line/col → char index | `src/string_utils.rs` (`rope_line_col_to_char_index`, `rope_char_index_at_line_start`) |
| Title bar maximized drag defer | `src/app/title_bar.rs` (`pending_drag_id`, `drag_armed_id`) |
| Entity caret mapping (code/link) | `src/markdown/widgets.rs` (`map_displayed_to_raw`, skip entity branch in code span / link text) |
| Outline escape mask/restore | `src/editor/outline.rs` (`mask_backslash_escapes`, `restore_backslash_escapes`, PUA `U+E000 + idx`) |
| Spellcheck service / worker | `src/spellcheck/mod.rs` (`SpellcheckService`, `should_request_check`, `cap_diagnostics_nearest_viewport`); `src/spellcheck/worker.rs` (`Request` / `Response`, coalesce) |
| Spellcheck tokenizer | `src/spellcheck/tokenize.rs` (`words_to_check`, `words_to_check_opts`, `LineContext`, `TokenizeOptions`) |
| Spellcheck dictionary + user words | `src/spellcheck/dictionary.rs` (`Dictionary::load`, `discover_dictionary_langs`, `user_words_path`); assets `assets/dictionaries/en_US.{aff,dic,LICENSE}` |
| Spellcheck AppState / lifecycle | `src/state.rs` (`spellcheck: Option<SpellcheckService>`); `FerriteApp::sync_spellcheck_service` in `src/app/mod.rs` |
| Spellcheck settings UI | `src/config/settings.rs` (`spellcheck_*`); `src/ui/settings/editor.rs` (`render_spellcheck`); registry `editor.spellcheck` |
| Spellcheck raw squiggles | `src/app/central_panel.rs` (`append_spellcheck_diagnostics`); take service before `EditorWidget::show_with_spellcheck` |
| Spellcheck context menu | `src/editor/widget.rs` (`prepend_spellcheck_menu_items`); `FerriteEditor::diagnostic_at` / `replace_word_range` |
| Spellcheck status bar | `src/app/status_bar.rs` (`spellcheck.loading` until `Loaded`) |
