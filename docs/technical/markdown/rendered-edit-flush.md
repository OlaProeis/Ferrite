# Rendered Edit — Session Flush Points

In-progress rendered edits live in `RenderedEditSession` block buffers until an **edge-triggered commit** runs. Before task 2, only block switch, click-outside dismiss, and TextEdit `lost_focus` committed a lone focused block — toggling view mode, switching tabs, saving, or closing could leave edits in the session buffer but not in `tab.content`.

**Related:** [Rendered edit session overview](./rendered-edit-session.md), [Source range replacement](./rendered-edit-source-range.md)

## Entry point

| Symbol | Module | Role |
|--------|--------|------|
| `flush_rendered_edit_session` | `src/markdown/editor.rs` | Load session, call `commit_active`, persist session, apply undo |
| `flush_tab_rendered_session` | `src/app/mod.rs` | Resolve tab by id, delegate to editor flush |
| `flush_active_rendered_session` | `src/app/mod.rs` | Flush the active tab in the working window |
| `flush_window_rendered_sessions` | `src/app/mod.rs` | Flush every tab in one document window |
| `flush_all_rendered_sessions` | `src/app/mod.rs` | Flush all open tabs |
| `set_active_tab_flushing` | `src/app/mod.rs` | Flush active tab, then `AppState::set_active_tab` |

`flush_rendered_edit_session` only runs when:

- `tab.view_mode` is `Rendered` or `Split`
- Tab is not special or large-file
- Session has an **active** block whose buffer is **dirty**

It mirrors the commit callback shape used by `session_dismiss_if_clicked_outside` and `render_session_plain_text_block` (`lost_focus`): seed `EditState` from parsed AST, `commit_session_block` → `write_session_block_to_source`, then `save_for_epoch_ctx` (no `source_epoch` bump on commit).

## Wired flush points

| Trigger | Call site |
|---------|-----------|
| View-mode toggle (Ctrl+E, ribbon, title bar segment) | `handle_toggle_view_mode`, title bar `ViewSegmentAction` |
| Tab switch (click, Ctrl+Tab, next/prev, save-dialog tab activate) | `set_active_tab_flushing` |
| Save / Save As / Save All | `handle_save_file`, `handle_save_as_file`, `handle_save_all_modified_tabs` (per tab) |
| Close tab | `handle_close_current_tab`, central panel tab close, file-delete tab close |
| App / window exit | `handle_close_request`, title bar close (all tabs or window scope) |
| Primary viewport focus loss | `FerriteApp::ui()` edge on `viewport().focused` |

Save and close paths flush **before** `is_modified()` / save-prompt checks so dirty session text is reflected in `tab.content`.

## Manual smoke

1. One-line doc, single paragraph in Rendered mode — edit text, press **Ctrl+E** (Raw). Raw must show the edit without clicking another block first.
2. Edit mid-session — switch tabs — return; content must persist.
3. Edit mid-session — **Save** — reopen file; edit must be on disk.

## Tests

- `test_flush_rendered_edit_session_commits_dirty_paragraph` in `src/markdown/editor.rs`
- Full suite: `cargo test` (RS-1…RS-7 in `rendered_session::tests`)
