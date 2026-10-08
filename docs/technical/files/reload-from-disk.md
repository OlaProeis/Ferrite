# Reload from Disk (#149)

Explicit user action to replace the active tab's in-memory buffer with the current file on disk, reusing the same reload path as the external file watcher.

## Entry points

| Surface | Binding / location |
|---------|-------------------|
| Keyboard | **Ctrl+Shift+R** (default; rebindable via Settings → Keyboard) |
| Command palette | **Reload from Disk** (`ShortcutCommand::Reload`, File category) |
| Ribbon | Save dropdown → **Reload from Disk** |
| F1 help | Listed under File shortcuts |

Wiring: `src/config/settings.rs` (`ShortcutCommand::Reload`) → `src/app/keyboard.rs` / `dispatch_palette_command` → `handle_reload_from_disk` in `src/app/file_ops.rs`.

**Keyboard guards:** `handle_keyboard_shortcuts` returns immediately when the terminal has focus (Reload and all other shortcuts are blocked). Reload is also registered only inside the explicit `!terminal_has_focus` block. `handle_reload_from_disk` additionally skips special tabs, image/PDF viewers, and tabs where `Tab::is_loading()`.

## Behaviour

`AppState::request_reload_from_disk(strip_index)` centralizes policy:

| Tab state | Result |
|-----------|--------|
| **No path** (untitled) | `pending_toast` → `notification.reload_no_path` — no dialog |
| **Loading** | No-op — no dialog, no toast |
| **Dirty** (unsaved edits) | Unsaved-changes dialog (`dialog.reload_from_disk.confirm`); `PendingAction::ReloadFromDisk` |
| **Clean + path** | Immediate reload via `reload_tab_by_id` |

On success, `handle_reload_from_disk` shows `notification.reloaded_single` (with real app time) and flushes the rendered session for the tab.

Special tabs (`Tab::is_special()`), image/PDF viewers, and loading tabs are ignored — no reload, no toast.

### Dirty confirmation dialog

Reuses the standard unsaved-changes modal in `src/app/dialogs.rs`:

- **Don't Save** — reload via `reload_tab_by_id`; deletes autosave backup for that tab; success toast
- **Save** — saves first; reloads only if save clears the modified flag
- **Cancel** — `cancel_pending_action`

Reload resolves by **tab id** (`PendingAction::ReloadFromDisk(tab_id)`), not strip index, so tabs closed or reordered while the dialog is open never redirect the reload to another tab.

## Disk reload implementation

`reload_tab_by_id` reads raw bytes with `std::fs::read` and calls `Tab::apply_external_disk_reload(bytes)` — the same helper used by the external-change watcher. That path:

- Re-detects encoding and `LineEnding` from decoded content (not raw bytes)
- Replaces buffer content
- Records **one undo step** via `record_edit(old_content, old_cursor)` before `mark_saved()` — Ctrl+Z restores the pre-reload buffer as a single step (edit history is not cleared)
- Bumps `content_version` so the raw editor rope resyncs even when byte length is unchanged (critical for >5 MB files)
- Sets `pending_cursor_restore` (clamped to new content length)

Errors (missing file, I/O) are queued in `AppState::pending_toast` and shown on the next frame with real app time.

### Toasts and timing

`AppState` has no access to wall-clock time. Reload paths that run inside state (`request_reload_from_disk`, `handle_confirmed_action` for `ReloadFromDisk`) set `pending_toast: Option<String>`. `FerriteApp` drains it each frame in `mod.rs` via `get_app_time()` — never `show_toast(..., 0.0, ...)`.

Success toasts in `file_ops.rs` and `dialogs.rs` call `get_app_time()` directly.

### Large-file raw editor resync

`FerriteEditorStorage` in `src/editor/widget.rs` tracks `content_versions: HashMap<tab_id, u64>` alongside content length/hash. `editor_needs_content_sync` checks `Tab::content_version()` **before** the large-file hash skip: when the version changes, the FerriteEditor rope is resynced from `tab.content` even if byte length is identical (>5 MB same-length reload bug).

## Out of scope

- Changing auto-reload behaviour for clean tabs when an external edit is detected (watcher path unchanged)
- macOS Finder **Open With** (separate task)

## Tests

State module (`src/state.rs`):

- `test_external_disk_reload_records_single_undo_step` — one undo restores pre-reload content; `undo_count` grows by exactly 1
- `test_reload_tab_by_id_skips_loading_tab` / `test_request_reload_skips_loading_tab`
- `test_request_reload_untitled_tab_toasts_no_dialog`
- `test_request_reload_dirty_tab_shows_confirm_dialog`
- `test_reload_tab_from_disk_reads_file_and_marks_saved`
- Existing `apply_external_disk_reload` / recovery-conflict reload tests

Editor widget (`src/editor/widget.rs`):

- `test_editor_needs_content_sync_large_file_version_bump` — bumped `content_version` forces resync when length unchanged
