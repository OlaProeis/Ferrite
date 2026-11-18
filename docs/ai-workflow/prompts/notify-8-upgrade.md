# Bump `notify` 6.1.1 → 8.2

Paste this whole file into a new chat. Do this crate only. Do not commit unless asked.

## Why

Ferrite is on `notify` 6.1.1. The latest stable is **8.2.2** (2026-11-15 crates.io `max_stable_version` was 8.2.2; 8.2.0 shipped 2025-08-03). **Do not take notify 9.** 9.0.0 is still a release candidate (rc.9 on 2026-11-15) and changes `Config`, `Watcher::watched_paths`, and error types.

notify 7 moved event types into `notify-types` and re-exports them. notify 8 raises MSRV to 1.77. Ferrite's MSRV is 1.92, so that is fine. The `crossbeam` feature was renamed to `crossbeam-channel` and is off by default. Ferrite uses `std::sync::mpsc`, so leave that feature off.

## Repo

- Path: `G:\DEV\markDownNotepad`
- One branch for this change. Do not bump `serde_yaml` / `serde_norway`, `comrak`, or `egui` in the same branch.

## What to change

In `Cargo.toml`, set `notify = "8"`. Keep default features.

The only production use is `src/workspaces/watcher.rs`:

- `RecommendedWatcher::new(callback, Config::default().with_poll_interval(Duration::from_millis(500)))`
- `watch(path, RecursiveMode::Recursive)`
- `EventKind::Create` → `FileCreated`, `Modify` → `FileModified`, `Remove` → `FileDeleted`
- `Access` and `Other` stay ignored

`with_poll_interval` only affects the poll backend. Keep the call if it still exists on `Config`. If notify 8 removed it, delete the call. Do not switch to `PollWatcher` to preserve it.

The comment in `Cargo.toml` above `notify` is load-bearing: `default-features = false` previously forced the poll backend and froze folder opens for 10+ seconds on Linux. Native backends must stay on (Linux inotify, macOS FSEvents, Windows `ReadDirectoryChangesW`).

Path contract: `handle_file_watcher_events` in `src/app/file_ops.rs` compares event paths to `tab.path` with `==`, then calls `consume_own_write_event`. Own-save suppression records a canonical path (`OwnWriteRegistry` in `src/workspaces/own_writes.rs`). If notify 8 emits a different form (a `\\?\` prefix, a non-canonical path, or a relative path), normalize at the watcher boundary with `crate::path_utils::normalize_path` before the event is queued. Do not change the own-save signature unless that normalization cannot be done at the boundary.

Do not add `notify-debouncer-mini` or `notify-debouncer-full`. The app already polls `try_recv` once per frame.

## Tests

Existing unit tests do not start a real watcher. They must still pass:

- `cargo test --bin ferrite test_filter_events`
- `cargo test --bin ferrite file_watcher_tests`
- `cargo test --bin ferrite own_writes`

`cargo clippy --bin ferrite` on `src/workspaces/watcher.rs`.

The real check is manual, on Windows, with a folder opened as a workspace:

1. Create a file in that folder from Explorer. The tree updates without a multi-second freeze.
2. Edit an open, unmodified file in another editor. Ferrite reloads it.
3. Edit that file in Ferrite and save. The log shows `Ignoring file watcher event for own save`. The tab stays clean and does not flicker back to the pre-save buffer. This is GIT-3 (`docs/technical/files/file-watcher-own-save.md`).
4. Delete a file from Explorer. The tree updates.

If a step fails, fix the path normalization or the event mapping. Do not paper over it by ignoring `Modify` events.

## Docs

Add a short note to `docs/technical/files/file-watcher-own-save.md`: watcher is `notify` 8, `RecommendedWatcher` with default features, poll interval only if the poll backend is selected. One bullet in `docs/index.md` only if you add a new doc; otherwise the existing `file-watcher-own-save.md` bullet is enough.

## Leave alone

- `serde_yaml` / `serde_norway`, `comrak`, `egui`.
- Reload-from-disk (#149) and the own-save TTL (5 seconds).
- Do not mark tasks done.
