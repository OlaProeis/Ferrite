# Ferrite — Editor UX Polish & Correctness Wave (PRD) - AI Context

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
Rust, cyclopsctl tasks CLI

## Architecture & Data Model
See `prd.md` for product architecture. This file captures agent workflow rules and where project artifacts live.

## Project Memory
- Rendered paragraph/list commits use `block_replace_end_line` + `update_source_range` (`src/markdown/editor.rs`) to merge AST `end_line` with committed buffer line count — avoids grow/shrink duplication without bumping `source_epoch`. See `docs/technical/markdown/rendered-edit-source-range.md`.
- Lone focused rendered edits flush via `flush_rendered_edit_session` → `commit_active` before view/tab/save/close/focus loss; app helpers in `src/app/mod.rs` (`set_active_tab_flushing`, etc.). See `docs/technical/markdown/rendered-edit-flush.md`.
- Plain paragraph Enter: commit+exit (same as formatted); Shift+Enter = soft break. `consume_plain_block_enter` must run before `TextEdit::show` on the active block so plain Enter never inserts a structural newline. See `docs/technical/markdown/rendered-edit-session-paragraphs-lists.md`.
- Line indices for source replacement are 1-based inclusive; use saturating arithmetic in span math.
- Workspace watcher reload of clean tabs uses `Tab::apply_external_disk_reload` (`notify_external_content_change` + `mark_saved`, no undo); dirty tabs skip reload. UTF-8/lossy read path only. See `docs/technical/files/session-persistence.md`.
- GFM table display alignment: paint via `table_cell_galley_paint_pos` (`cell_rect.min - galley.rect.min` + `table_cell_block_align_shift`); `table_cell_raw_cursor_at_click` uses the same paint math. See `docs/technical/markdown/gfm-table-column-alignment.md`.
- CSV rendered cell keyboard nav: Tab/Shift+Tab wrap and arrow clamp via `src/markdown/table_cell_nav.rs` (shared with GFM `TableEditState`); `TextEdit::lock_focus(true)` + commit-before-navigate through `queue_cell_commit`. Click requests table focus for immediate arrows. See `docs/technical/viewers/csv-viewer.md`.
- Video embeds require explicit `{{video URL}}` syntax — `try_parse_video_paragraph` no longer auto-embeds bare YouTube autolinks; they stay `Link` nodes. See `docs/technical/markdown/video-embed-parsing.md`.
- Optional `width`/`height` on `{{video …}}` parse into `VideoEmbedInfo` via `parse_braced_video_content` (clamp `1..=8192`; unknown keys ignored; `source_text` verbatim). `video_display_size` in `video_render.rs` sizes the player/WebView rect (width-only → 16:9; clamp to pane width). See `docs/technical/markdown/video-embed-rendering.md`.
- Video drag-resize: bottom-right handle in `render_video_embed`; pending size in egui temp data; on release `rewrite_video_embed_dimensions` + `mark_line_modified` in `editor.rs`. WebView sync skipped while handle hovered/dragging (HWND blocks egui). See `docs/technical/markdown/video-embed-rendering.md`.
- New Window: single `APP_WINDOW` `icon_button` in ribbon RTL cluster (Export | New Window | Terminal); title-bar Window menu removed. `RibbonAction::NewWindow` → `handle_new_window`; tooltip `menu.window.new_window` + Ctrl+Shift+N. See `docs/technical/ui/ribbon-window-control.md`.
- Raw editor context menu: `show_raw_editor_context_menu` on `EditorWidget` response (`src/editor/widget.rs`); Copy/Cut/Paste/Select All + Undo; paste via `arboard`, undo via `EditorOutput.request_undo` → `handle_undo()` in `central_panel.rs`. See `docs/technical/ui/raw-editor-context-menu.md`.
- Tab context menu: `ActionRegistry` + `render_action_menu_with_shortcuts` (`src/ui/action_registry.rs`); popup in `central_panel.rs`; i18n `tab.new_tab`/`tab.close`; hover via background-layer fill (command-palette pattern); no fixed min-width. See `docs/technical/ui/tab-context-menu.md`.
- Preview lock palette command: `ShortcutCommand::TogglePreviewLock` ("Lock editing") → `handle_toggle_preview_lock()` in `navigation.rs` (`Tab::toggle_preview_locked()` + toast); unbound default (`M::none()` — no palette shortcut badge); padlock overlay unchanged. See `docs/technical/ui/preview-lock.md`.

## Where Things Live
| Want to... | Look in... |
|------------|------------|
| Product requirements | `prd.md` |
| Current implementation handover | `current-handover-prompt.md` |
| Post-task update rules | `update-handover-prompt.md` |
| Documentation map | `docs/index.md` |
| Tasks and complexity | `.cyclopsctl/tasks/tasks.json`, `.cyclopsctl/reports/complexity-report.json` |
| Cyclopsctl config | `cyclopsctl.toml` |
| Ribbon New Window control | `src/ui/ribbon.rs`, `docs/technical/ui/ribbon-window-control.md` |
| Raw editor context menu | `src/editor/widget.rs`, `src/editor/ferrite/editor.rs`, `docs/technical/ui/raw-editor-context-menu.md` |
| Tab context menu | `src/ui/action_registry.rs`, `src/app/central_panel.rs`, `docs/technical/ui/tab-context-menu.md` |
| Preview lock / Lock editing palette | `src/app/navigation.rs`, `src/config/settings.rs`, `docs/technical/ui/preview-lock.md` |
| Command palette | `src/ui/command_palette.rs`, `src/app/commands.rs`, `docs/technical/ui/command-palette.md` |
