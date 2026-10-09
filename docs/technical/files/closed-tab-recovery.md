# Closed tab recovery cleanup

Closing a tab must not leave a crash-recovery artifact that the next launch can restore.

## Close path

Every tab close ends in `FerriteApp::cleanup_tab_state` (`src/app/mod.rs`). That includes the tab strip, the close-current-tab command, confirm-dialog closes, startup removal of the empty default tab, and file delete in `src/app/file_ops.rs`.

`cleanup_tab_state` builds the set of tab ids still open on `AppState` and calls `discard_closed_tab_recovery`.

## What the helper does

`discard_closed_tab_recovery` (`src/config/session.rs`):

1. Deletes `recovery/<closed_tab_id>.json` (`delete_recovery_content`).
2. Rewrites `session.recovery.json` with `save_crash_recovery_snapshot_file`. The helper loads the existing snapshot, keeps only `live_tab_ids` via `SessionState::retain_tab_ids` (flat `tabs` and per-window lists; empty windows are dropped; active and focused indexes are clamped), marks it a crash snapshot, and writes it back. If no snapshot file exists, the rewrite is a no-op. A snapshot that fails to parse is left unchanged.
3. Calls `prune_recovery_dir(live_tab_ids)` so any other `recovery/<id>.json` whose id is not still open is removed.

Periodic saves still use `save_crash_recovery_state` with a freshly captured session. The close path is the one that rewrites an already-written snapshot after a tab is gone.

Clean exit drops the crash snapshot (`clear_crash_recovery_snapshot_file`) and still keeps `recovery/` blobs for **dirty** open tabs (including pathless quick-note buffers). Stale blobs for **clean** open tabs are removed in `on_exit` via `AppState::prune_non_dirty_recovery_blobs` — see [`clean-exit-recovery-prune.md`](clean-exit-recovery-prune.md).
