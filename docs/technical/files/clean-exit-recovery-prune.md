# Clean exit recovery prune (ISS-23)

Periodic crash snapshots can write `recovery/<tab_id>.json` while a tab is dirty. If the user restores that buffer and later undoes back to a clean baseline (`!Tab::is_modified()`), the blob can outlive the edit until the next launch — offering stale text on restart even though the tab matches disk.

Clean shutdown must drop those stale blobs while still preserving recovery for tabs that remain dirty (including pathless quick-note buffers).

## Exit path

`FerriteApp::on_exit` (`src/app/mod.rs`), after window close / File › Exit / Alt+F4 when no blocking unsaved prompt remains:

1. `save_recovery_content_excluding(discarded_ids)` — writes or refreshes blobs only for modified tabs (see session-persistence “Don’t Save” flow).
2. Deletes recovery and autosave artifacts for tabs the user declined to save.
3. **`AppState::prune_non_dirty_recovery_blobs`** — for each open, non-special tab where `!is_modified()`, calls `delete_recovery_content(tab.id)`.
4. Saves `session.json` with `clean_shutdown` set.
5. **`clear_crash_recovery_snapshot_file`** — removes `session.recovery.json` only; does not wipe the whole `recovery/` directory.

Dirty tabs keep the blobs written in step 1. Clean tabs (including restore → undo to match disk) have no `recovery/<id>.json` left after step 3.

## Relation to tab close (ISS-21)

Tab close uses `discard_closed_tab_recovery` to delete the closed tab’s blob and prune ids not in the live set. Clean exit does **not** replace that path; it complements crash-snapshot cleanup by scrubbing blobs for **still-open** tabs that are no longer modified.

## Regression test

`test_clean_exit_prunes_non_dirty_session_recovery_blobs` in `src/state.rs` seeds stale recovery files, builds clean vs dirty tabs, runs `prune_non_dirty_recovery_blobs`, and asserts the clean id is gone while the dirty blob remains.
