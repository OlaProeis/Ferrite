# Rendered Edit — Session Flush and Save Commit

In-progress rendered edits live in `RenderedEditSession` block buffers (and in table/code widget temp data) until a **commit** runs. Edge-triggered commits still happen on block switch, click-outside dismiss, and TextEdit `lost_focus`. **Save** and related paths use a single app entry point so buffered edits reach `tab.content` even when the tab is in Raw view or a table cell still has focus (ISS-41 / TBL-2).

**Related:** [Rendered edit session overview](./rendered-edit-session.md), [Source range replacement](./rendered-edit-source-range.md), [Preview code block undo](./preview-code-block-undo.md)

## Primary entry points

| Symbol | Module | Role |
|--------|--------|------|
| `commit_pending_rendered_edits_for_tab` | `src/markdown/editor.rs` | Session `commit_active` + persisted table/code widget flush; applies undo queue; returns whether content changed |
| `FerriteApp::commit_pending_rendered_edits` | `src/app/mod.rs` | Runs the above for **every open tab**; returns `HashSet<usize>` of changed tab ids |
| `flush_rendered_edit_session` | `src/markdown/editor.rs` | Alias for `commit_pending_rendered_edits_for_tab` |
| `flush_tab_rendered_session` | `src/app/mod.rs` | Resolve tab by id, delegate to commit |
| `flush_active_rendered_session` | `src/app/mod.rs` | Commit the active tab in the working window |
| `flush_window_rendered_sessions` | `src/app/mod.rs` | Commit every tab in one document window |
| `flush_all_rendered_sessions` | `src/app/mod.rs` | Commit all open tabs |
| `set_active_tab_flushing` | `src/app/mod.rs` | Commit active tab, then `AppState::set_active_tab` |
| `rendered_session_pending_text` | `src/markdown/editor.rs` | Read-only overlay of dirty session / table / code buffers for status-bar counts; does not write `tab.content` |

`commit_pending_rendered_edits_for_tab` skips special and large-file tabs. Unlike the pre–task-5 flush, it does **not** require `ViewMode::Rendered` or `Split` — pending session buffers commit from Raw as well.

### Session block commit

When the session has an **active** block with a **dirty** buffer: seed `EditState` from the parsed AST, `commit_active` → `commit_session_block` → `write_session_block_to_source`, then `save_for_epoch_ctx` and `apply_rendered_commit_undo_entries` (no `source_epoch` bump on commit).

### Widget-backed commit (table / code)

Table cell text and in-edit fenced code blocks live in egui temp data, not always in `BlockEditState`. Each rendered frame mirrors buffers to **tab-stable** keys:

| Key helper | Module | Stored data |
|------------|--------|-------------|
| `persist_table_key(editor_id, source_epoch, table_line)` | `src/markdown/rendered_session.rs` | `TableData` + `TableEditState` (`content_modified`) |
| `persist_codeblock_key(editor_id, source_epoch, start_line)` | `src/markdown/rendered_session.rs` | `CodeBlockData` while `is_editing` |

On commit (save path or flush), the editor walks the document AST for tables and code blocks, reads those keys from `egui::Context`, and writes through `update_table_in_source` / `update_code_block` with `rendered_commit_undo::record_source_commit`.

## Wired commit points

| Trigger | Call site |
|---------|-----------|
| **Save / Save As / Save All** | `handle_save_file`, `handle_save_as_file`, `handle_save_all_modified_tabs` — **`commit_pending_rendered_edits`** at the top (all tabs) |
| View-mode toggle (Ctrl+E, ribbon, title bar segment) | `handle_toggle_view_mode` — `flush_active_rendered_session` **before** `toggle_view_mode` |
| Tab switch (click, Ctrl+Tab, next/prev, ribbon back, save-dialog tab activate) | `set_active_tab_flushing` — commit **before** the active index changes |
| Crash recovery / forced session save | `update_session_recovery` and `force_session_save` — `commit_pending_rendered_edits` **before** `capture_session_state` and `save_recovery_content`. CLI startup calls `force_session_save(None)` and skips the commit (no egui context yet). `on_exit` has no context; `handle_close_request` commits first |
| External file change | `handle_file_watcher_events` — commit all tabs **before** `is_modified()` decides reload vs “changed externally” |
| Close tab | `handle_close_current_tab`, central panel tab close, file-delete tab close |
| App / window exit | `handle_close_request` — **`commit_pending_rendered_edits`**; title bar close (window scope) |
| Primary viewport focus loss | `FerriteApp::ui()` edge on `viewport().focused` |
| Reload from disk (active tab) | `handle_reload_from_disk` — flush active tab before modified check |
| Status bar word/line counts | `render_status_bar` uses `rendered_session_pending_text` when a session or persisted table/code buffer is dirty. Paragraph and table-cell text also write through each frame; see [rendered session write-through](./rendered-session-write-through.md) |

Save, snapshot, view-switch, tab-switch, and external-reload paths commit **before** reading `tab.content` for disk write, session metadata, or `is_modified()` checks. The status bar is the exception: it overlays pending text and leaves the buffer alone until one of those commit points.

## Manual smoke

1. Rendered table — edit a cell, switch to Raw **without** clicking another cell, **Ctrl+S** — saved file must contain the edit.
2. One-line doc, paragraph in Rendered — edit, switch to Raw, **Save** — disk and Raw buffer match the edit.
3. Edit mid-session — switch tabs — return; content must persist.
4. Edit mid-session — **Save** — reopen file; edit must be on disk.

## Tests

- `test_flush_rendered_edit_session_commits_dirty_paragraph` — session paragraph flush
- `save_includes_paragraph_session_edit_without_blur` — Raw view + session + `save_active_tab`
- `save_includes_table_cell_edit_without_blur` — persisted table buffer + save
- `rendered_session::tests` (RS-1…RS-7 and table-cell session matrix)
- `app::rendered_commit_point_tests` — cell edit visible to `capture_session_state`; Rendered→Raw paragraph commit; tab switch updates the previous tab; status-bar overlay does not write `tab.content`
