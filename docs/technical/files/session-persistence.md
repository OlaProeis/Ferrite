# Session Persistence & Crash Recovery

## Overview

Ferrite implements crash-safe session persistence that saves and restores the full editor session state (open tabs, active tab, scroll positions, cursor positions, and unsaved content) across restarts and after crashes.

## Architecture

### Session State Model

The session persistence system consists of three main components:

1. **SessionState** - Top-level session state containing:
   - Schema version (for migration support; **v2** adds multi-window — see [multi-window-session.md](../platform/multi-window-session.md))
   - Timestamp of last save
   - Clean shutdown flag
   - **v2:** `windows: Vec<SessionWindowState>` (per-window tabs, active tab, geometry) and `focused_window_index`
   - **v1:** flat `tabs: Vec<SessionTabState>` and `active_tab_index` (still deserializes; restores as one primary window)
   - Application mode (single file or workspace)

2. **SessionTabState** - Per-tab state including:
   - Tab ID
   - File path (if saved)
   - Display title
   - View mode (Raw/Rendered)
   - Preview lock flag (`preview_locked`, defaults unlocked for legacy sessions — see [preview-lock.md](../ui/preview-lock.md))
   - Cursor position (character index and line/column)
   - Selection range
   - Scroll offset
   - Has unsaved content flag
   - File modification time (for conflict detection)
   - Original content hash

3. **RecoveryContent** - Stored separately for tabs with unsaved changes:
   - Tab ID
   - Full document content
   - Save timestamp
   - **`path`** (added in task 106) — file path the recovery is anchored to (`None` for untitled tabs)
   - **`original_content_hash`** (task 106) — hash of the on-disk content the tab was loaded from when the recovery was written
   - **`schema_version`** (task 106) — defaults to `1` for legacy files via serde defaults; future schema bumps are rejected at the loader boundary

4. **AutoSaveMetadata** - JSON header for `<file>.autosave` temp files:
   - Tab ID, original path, save timestamp, content hash
   - **`disk_content_hash`** (task 106) — hash of the on-disk content the tab was loaded from at the moment this autosave was written; defaults to `None` for legacy files via serde defaults

5. **RecoveryConflict** (task 106.5) - Lives on `AppState.recovery_conflicts` keyed by `tab_id` while the central panel banner is visible:
   - `recovered_content` — buffer applied during restore
   - `on_disk_content` — current disk content captured at restore time

### File Locations

All session files are stored in the user's config directory:

- **Windows**: `%APPDATA%\ferrite\`
- **macOS**: `~/Library/Application Support/ferrite/`
- **Linux**: `~/.config/ferrite/`

Files:
- `session.json` - Clean session state (saved on normal shutdown)
- `session.recovery.json` - Crash recovery state (saved periodically)
- `session.lock` - Lock file (indicates app is running)
- `recovery/` - Directory containing per-tab recovery content files

### Persistence Flow

#### On Startup
1. Create lock file to detect future crashes
2. Check for crash recovery file and lock file presence
3. If crash detected (lock file existed from previous session):
   - Load crash recovery state
   - If unsaved changes exist, show recovery dialog
   - User can choose to restore or start fresh
4. If clean shutdown (no lock file):
   - Silently restore session from session.json

#### While Running
- Session save throttle (5 second debounce)
- On content changes, mark session as dirty
- Periodically save crash recovery snapshot
- Save recovery content for tabs with unsaved changes

#### On Clean Shutdown
1. Capture session state
2. Mark as clean shutdown
3. Save to session.json
4. Clear crash recovery data
5. Remove lock file

#### On Exit with "Don't Save"

When the exit confirmation dialog is answered with **Don't Save**, the user
has explicitly discarded the changes in the tabs that triggered the prompt.
`FerriteApp.discard_unsaved_on_exit` is set and `on_exit` then:

1. Computes the **discarded set** — exactly the tabs for which
   `should_prompt_to_save(settings, AppExit)` is true. Quick-note scratch
   buffers (pathless tabs when `quick_note_workflow` is enabled) never
   prompt, are *not* in the set, and keep their recovery content.
2. Calls `save_recovery_content_excluding(discarded_ids)` instead of the
   unconditional `save_recovery_content()` — discarded buffers are **not**
   re-persisted.
3. Deletes `recovery/<id>.json` and the autosave temp file for every
   discarded tab (both are written periodically while the app runs).
4. Forces `has_unsaved_content = false` for the discarded tabs in the
   captured `session.json`, so the next launch reloads them cleanly from
   disk.

**Regression this prevents:** previously the dialog cleared the recovery
directory, but `on_exit` ran afterwards and unconditionally re-saved
recovery content for every still-modified tab. On the next launch the
identity gate passed (same path, unchanged disk) and the discarded buffer
resurfaced via the "Recovered content differs from this file on disk"
banner; the leftover autosave file additionally re-offered the discarded
changes when the file was reopened.

The same autosave cleanup applies when a single tab close or a window close
is confirmed with **Don't Save** (see `render_dialogs` in
`src/app/dialogs.rs`): the tab's recovery file *and* autosave temp file are
deleted along with the per-tab UI state.

#### "Save" in the exit / window-close dialog

The dialog's **Save** button saves **all** tabs that triggered the prompt
via `FerriteApp::handle_save_all_modified_tabs` (optionally scoped to the
closing window). Path-backed tabs are saved directly through
`AppState::save_tab_by_id`; pathless tabs are activated one at a time and
routed through the blocking Save As dialog. If any save fails or is
cancelled, the confirmation dialog stays open. (Previously only the active
tab was saved, so the dialog could never complete with two or more modified
tabs.)

## Data Flow

```
                    ┌─────────────────┐
                    │   AppState      │
                    │  (Runtime)      │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              │                             │
              ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │ capture_session │           │restore_from_    │
    │ _state()        │           │session_result() │
    └────────┬────────┘           └────────▲────────┘
             │                             │
             ▼                             │
    ┌─────────────────┐           ┌────────┴────────┐
    │ SessionState    │◄─────────►│ load_session_   │
    │ (Serializable)  │           │ state()         │
    └────────┬────────┘           └─────────────────┘
             │
             ▼
    ┌─────────────────┐
    │ session.json /  │
    │ .recovery.json  │
    └─────────────────┘
```

## Recovery UI

When crash recovery is detected with unsaved changes, a modal dialog appears offering:

- **Restore Session**: Restores all tabs from the previous session, including recovered unsaved content
- **Start Fresh**: Discards the recovery data and starts with an empty editor

If Ferrite was launched with file paths (e.g. double-click while the app was closed), those paths are **deferred** until the user answers the dialog, then opened afterward (existing tabs with the same path are focused, not duplicated).

### Recovery vs. Disk Conflict Banner (task 106.5)

When an identity-validated recovery file is applied but its buffer differs from
the current disk content, the central panel renders a non-blocking banner above
the editor for the active tab. Editing is allowed while the banner is visible —
only the two action buttons clear it:

- **Keep Recovered** (`AppState::keep_recovered_buffer`) — drops the conflict
  entry; the buffer stays modified so the user can save manually.
- **Reload from Disk** (`AppState::apply_reload_from_disk_for_conflict`) —
  replaces the tab buffer with the on-disk content captured at restore time
  and marks the tab saved.

Closing the tab also clears any pending conflict so the banner cannot resurrect
on a future tab whose runtime id collides with the cleared one.

## Identity-Gated Recovery (task 106)

Recovery files in `recovery/<tab_id>.json` and autosave files under `autosave/`
persist across launches, but the per-session `tab_id` namespace is reset to 0
on every launch and re-issued monotonically. Without identity checks, a
leftover recovery file from a previous session could silently apply to an
unrelated tab in the current session that happened to be assigned the same id
— a data-loss hazard ("cross-tab bleed"; the original repro was an untitled
buffer named `asdasd` for `tab_id=10` overwriting
`task_50_table_inline_formatting.md` on save).

`AppState::try_apply_recovery` (`src/state.rs`) gates application of recovery
content in three layers, in order:

1. **Legacy bypass.** Pre-task-106 files have neither `path` nor
   `original_content_hash` set. They fall back to historical "tab id only"
   matching so users upgrading from older Ferrite versions don't lose
   recovered text. No conflict banner can be raised in this branch.
2. **Path equality.** `recovered.path` must equal `session_tab.path` (covers
   untitled tabs via `None == None`). A mismatch indicates a reused tab id and
   is rejected with the `session_recovery_identity_mismatch` diag event.
3. **Disk hash.** When the tab is path-backed and the file exists on disk,
   the current disk content is hashed (same algorithm as
   `crate::config::hash_content`) and compared against
   `recovered.original_content_hash`. A mismatch means the file was edited
   externally between sessions; the recovery is rejected and the caller falls
   through to a fresh disk load. Encoding-failure on `read_to_string` is
   tolerated (path identity is trusted).

When all three layers pass and the recovered buffer differs from current disk
content, `try_apply_recovery` returns
`ResolvedContent::RecoveredWithDiskDivergence { content, on_disk_content }`
and `restore_from_session_result` populates `AppState.recovery_conflicts` so
the banner described above is rendered.

### Disk-hash anchoring across recovery cycles

**Invariant:** after `restore_from_session_result` applies a divergent
recovery, the new `Tab` must keep `original_content` (and therefore
`disk_content_hash()`) anchored to the **actual on-disk text**, not to the
recovered buffer.

If the anchor is set to the recovered buffer instead, the bug manifests on
the *second* crash/restore cycle:

1. Edit → kill → Restore. `Tab::with_file(path, recovered)` was used, so
   `original_content == content`, `is_modified() == false`, and
   `disk_content_hash() == hash(recovered)`.
2. New edits happen → `is_modified()` flips to `true` → the next crash
   snapshot writes `recovery/<id>.json` with
   `original_content_hash = hash(recovered)`.
3. Kill → restart. Disk hash check in `try_apply_recovery` compares
   `hash(disk) != hash(recovered)` and rejects the recovery file. The tab
   silently falls back to disk content — every edit since the previous
   recovery is gone.

`restore_from_session_result` handles this by using `on_disk_content` for
the constructor call when the resolver returned
`ResolvedContent::RecoveredWithDiskDivergence`, then calling `set_content`
to swap in the recovered buffer (which bumps `content_version`, records one
undo step, and leaves `original_content` pointing at disk). Regression
tests: `test_restore_with_divergence_anchors_original_to_disk` and
`test_restore_then_edit_keeps_disk_hash_anchor` in `state.rs`.

The autosave path enforces equivalent identity in
`config::session::check_auto_save_identity`:
`metadata.original_path != tab_path` is rejected outright; when
`metadata.disk_content_hash` is `Some(want)` the disk is re-hashed and a
mismatch is rejected. Both sides emit the same
`session_recovery_identity_mismatch` diag key (with `source=autosave` in the
message).

### Pruning Stale Files

`AppState::prune_stale_recovery_files` runs once after session restore and
deletes:

- `recovery/<tab_id>.json` whose id is not in the live tab set
  (`config::prune_recovery_dir`)
- `autosave/untitled_<tab_id>.md.autosave` whose id is not in the live tab
  set (`config::prune_auto_save_dir`)

Path-backed autosaves (`<stem>_<pathhash>.md.autosave`) are not pruned by id
because they are keyed by file path; identity for those is enforced at
recovery time by `check_auto_save_identity`.

## Workspace File Watcher — External Reload

When a file open in a workspace tab is modified on disk by another application,
the workspace file watcher reloads **unmodified** tabs automatically
(`FerriteApp::handle_file_watcher_events` in `src/app/file_ops.rs`). Tabs with
local unsaved edits are **not** overwritten; a warning is logged instead.

Reload path (`Tab::apply_external_disk_reload` in `src/state.rs`):

1. Read raw bytes from disk (UTF-8 decode with lossy fallback — no chardetng on
   this path; full encoding detection still happens on initial open).
2. Replace `content`, call `notify_external_content_change()` (bumps
   `content_version` / `source_epoch`; **does not** record undo).
3. Refresh encoding metadata: `original_bytes` (small files only),
   `detected_encoding` / `current_encoding` → `"utf-8"`, `had_bom` from BOM scan.
4. Call `mark_saved()` so `original_content` / `original_content_hash` match
   the new buffer — tab stays clean (no `*` indicator, no save prompt on close).

This mirrors the recovery banner's **Reload from Disk** action
(`AppState::apply_reload_from_disk_for_conflict`), which also ends with
`mark_saved()`. Scope is **workspace mode only**; single-file mode has no file
watcher reload.

## Conflict Detection

The system tracks file modification time to detect conflicts:

- **NoConflict**: File hasn't changed since last read
- **ModifiedOnDisk**: File was modified externally
- **FileDeleted**: File no longer exists
- **NoFile**: Tab has no associated file

This is independent of the identity-gated recovery banner above: mtime-based
conflict detection runs at restore time for every tab, while the recovery
banner only appears for tabs whose recovered buffer differs from current disk
after passing the identity gate.

## Implementation Files

- `src/config/session.rs` - Session state model, persistence functions, and identity helpers (`check_auto_save_identity`, `prune_recovery_dir`, `prune_auto_save_dir`)
- `src/config/mod.rs` - Module exports
- `src/state.rs` - `capture_session_state()`, `restore_from_session_result()`, `try_apply_recovery()`, conflict-banner action methods (`keep_recovered_buffer`, `apply_reload_from_disk_for_conflict`), `save_recovery_content_excluding()`, `save_tab_by_id()`
- `src/app/mod.rs` - Lifecycle integration (startup, periodic saves, shutdown incl. the `discard_unsaved_on_exit` flow), autosave loop with `disk_content_hash` propagation
- `src/app/dialogs.rs` - Unsaved-changes confirmation dialog (Save-all / Don't Save / Cancel, recovery + autosave cleanup on discard)
- `src/app/file_ops.rs` - `handle_save_all_modified_tabs()` (Save button in exit / window-close dialog); `handle_file_watcher_events()` (workspace external reload)
- `src/state.rs` - `Tab::apply_external_disk_reload()` (watcher reload baseline + encoding refresh)
- `src/app/central_panel.rs` - `render_recovery_conflict_banner` (task 106.5)

## Workspace Session Persistence

### Workspace Path Handling

The session system stores the workspace root path when in workspace mode:

1. **On Save**: The workspace path is canonicalized before saving to ensure consistent storage across restarts and to resolve any relative paths or symlinks.

2. **On Restore**: The system:
   - Validates the saved path is non-empty
   - Attempts to canonicalize the path for consistency
   - Checks if the path exists on disk
   - Verifies it's actually a directory
   - Falls back to single-file mode if any validation fails

### Debug Logging

Comprehensive debug logging is available for troubleshooting session persistence issues. Enable debug logging with `--log-level debug` to see:

- Session file selection (recovery vs. session.json)
- File modification time comparisons
- Workspace path during save/load operations
- Path canonicalization results
- Validation failures and reasons

### Error Handling for Invalid Paths

The system handles various error conditions gracefully:

- **Empty path**: Session starts in single-file mode
- **Non-existent path**: Warning logged, starts in single-file mode
- **Path not a directory**: Warning logged, starts in single-file mode
- **Workspace open failure**: Warning logged with error details, starts in single-file mode

## Testing Strategy

### Unit Tests
- Session state serialization/deserialization roundtrip
- Content hashing consistency
- Save throttle timing logic

### Integration Testing
1. Open multiple documents, make edits, close cleanly
2. Reopen - verify all tabs restored with correct positions
3. Open documents, make unsaved changes, force-kill process
4. Reopen - verify recovery dialog appears
5. Test both "Restore" and "Start Fresh" options

### Manual Testing
- Verify session files are created in correct location
- Verify lock file is removed on clean exit
- Verify periodic saves occur while editing
- Test with large files and many tabs

## Future Enhancements

1. **Diff View in Conflict Banner**: Show a side-by-side diff of recovered vs. on-disk inside the banner before the user picks `Keep Recovered` / `Reload from Disk` (task 106.5 just exposes the strings via `RecoveryConflict`).
2. **Rendered Scroll Sync**: Track rendered mode scroll offset separately
3. **Multi-window Support**: Track multiple window states
4. **Session History**: Keep history of previous sessions
5. **Selective Restore**: Allow restoring individual tabs from recovery
