# File watcher — own-save suppression (GIT-3)

Workspace mode watches the tree with `notify` and reloads clean open tabs when a watched file changes on disk. A save from Ferrite itself also triggers `FileModified`, which previously caused an immediate external reload right after “Saved file”.

## Mechanism

After a successful in-app write, the app **stats the file** and stores a signature keyed by **canonical normalized path**:

| Field | Source |
|-------|--------|
| `modified` | `std::fs::metadata(path).modified()` |
| `len` | `metadata.len()` |

When a `WorkspaceEvent::FileModified` arrives for a path that is open in a tab:

1. `AppState::consume_own_write_event(path)` reads the current stat.
2. If it matches the pending entry, the entry is **removed** and the event is **ignored** (no reload, no “reloaded externally” toast).
3. If stat differs (real external edit), reload proceeds as before via `handle_external_file_modifications`.

Entries **expire after 5 seconds** (`OWN_WRITE_TTL`) if no matching event is seen, so the map cannot grow without bound.

## Code locations

| Piece | Location |
|-------|----------|
| Registry + unit tests | `src/workspaces/own_writes.rs` (`OwnWriteRegistry`, `FileStatSignature`) |
| Record on save | `AppState::save_tab_by_id`, `save_active_tab_as` → `record_own_write` |
| Watcher filter | `FerriteApp::handle_file_watcher_events` in `src/app/file_ops.rs` |
| Reload loop (shared with watcher) | `handle_external_file_modifications` in `src/app/file_ops.rs` → `Tab::apply_external_disk_reload` |

Explicit **Reload from Disk** (#149) is unchanged; this only suppresses **automatic** watcher reloads for our own writes.

## Tests

- Unit: `workspaces::own_writes::tests` — match/consume, length change, TTL, mtime change.
- Integration: `app::file_ops::file_watcher_tests` — save then simulated watcher path skips reload; external write after own-write consumed still reloads.
