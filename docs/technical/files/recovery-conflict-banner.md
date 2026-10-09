# Recovery conflict banner

After session restore, a path-backed tab can keep a recovered buffer that differs from the file on disk. `AppState::recovery_conflicts` stores that pair (`recovered_content`, `on_disk_content`) keyed by the tab's runtime id. The central panel draws the banner from `AppState::has_recovery_conflict`.

The entry is created once, when restore applies `ResolvedContent::RecoveredWithDiskDivergence`. It is removed only by **Keep Recovered** (`keep_recovered_buffer`), **Reload from Disk** (`apply_reload_from_disk_for_conflict`), or closing the tab.

## When the banner shows

`has_recovery_conflict` re-reads the live `tab.content` on every call (`recovery_conflict_banner_visible` in `src/state.rs`):

- Large files pass the tab's cached `original_content_hash` (hash of the on-disk text). The live buffer is hashed with the same `hash_content` helper and compared to that cached hash. The disk snapshot is not hashed again.
- Other files compare the buffer to `RecoveryConflict.on_disk_content` directly.

A match hides the banner and leaves the conflict entry in place, so a later edit that diverges shows the banner again. **Keep Recovered** and **Reload from Disk** still drop the entry, which hides the banner for the rest of the session.

The editor syncs `tab.content` before the next frame's banner check, so raw and rendered edits (including undo back to the disk snapshot) are picked up without a separate dirty flag.

## Related

- [Session persistence](session-persistence.md) — identity gate, restore, and the two banner actions.
