# Multi-Window Session Persistence (v2)

**Status:** Implemented (v0.3.1-experimental).  
**Design reference:** [multi-window.md](./multi-window.md) (session schema v2 section)  
**Related:** [session-persistence.md](../files/session-persistence.md), [multi-window-implementation.md](./multi-window-implementation.md)

## Overview

Session files now persist **all document windows**: per-window tab strips, active tab index, window geometry, and last-focused window. Loading remains **backward compatible** with v1 `session.json` files that only store a flat `tabs` array.

## Schema (`SESSION_VERSION = 2`)

Defined in `src/config/session.rs`:

| Field | Purpose |
|-------|---------|
| `windows: Vec<SessionWindowState>` | Per-window tabs, `active_tab_index`, `geometry` |
| `focused_window_index` | Index into `windows` for last-focused window |
| `tabs` / `active_tab_index` | v1 legacy fields; empty/zero when v2 `windows` is populated |

`SessionWindowState` carries `window_id`, tab list (`SessionTabState` — unchanged from v1), `active_tab_index`, and `SessionWindowGeometry` (`width`, `height`, optional `x`/`y`, `maximized`).

**Restore path selection** (`SessionState::uses_multi_window_restore()`):

- `version >= 2` **and** `windows` non-empty → multi-window restore
- Otherwise → legacy single-window: flat `tabs` → one `DocumentWindowState` with `window_id: 0`, geometry from `Settings.window_size`

Unknown `version > 2` is rejected at load time (existing pattern).

## Capture (`AppState::capture_session_state`)

`src/state.rs` iterates `AppState::windows` and:

1. Maps each window's `tab_ids` to `SessionTabState` (skips special tabs and ephemeral PDF viewers)
2. Remaps `active_tab_index` to the **persisted** tab list (so filtered special tabs do not skew the index)
3. Writes `focused_window_index` from `focused_window_id`
4. Leaves v1 `tabs` empty (v2 prefers `windows[]`)

Called on clean shutdown (`FerriteApp::on_exit`) and for periodic crash snapshots (`update_session_recovery`).

## Restore (`AppState::restore_from_session_result`)

1. Builds `window_specs` from v2 `windows` or v1 flat `tabs`
2. Rebuilds `DocumentWindowState` entries with `first_frame: true` for v2 (geometry applied on first viewport frame)
3. Restores tabs into the global `tabs` store and appends each to the correct window strip
4. Sets `focused_window_id` from `focused_window_index`
5. Seeds an empty tab in any window whose strip ended up empty

Recovery content (`recovery/<tab_id>.json`) and tab-id identity rules are unchanged — tab ids remain process-global.

## Geometry application on startup

| Window | Where geometry is applied |
|--------|---------------------------|
| Primary (`window_id == 0`) | `FerriteApp::update()` — `ViewportCommand::OuterPosition`, `InnerSize`, `Maximized` when `first_frame` |
| Secondary | `render_secondary_document_windows()` — `ViewportBuilder` position/size on `first_frame` |

Per-frame capture continues via `update_window_geometry_for()`; primary geometry is also mirrored into `Settings.window_size` on shutdown.

## Tests

| Test | Location |
|------|----------|
| v2 serde round-trip | `config::session::tests::test_session_v2_multi_window_roundtrip` |
| Legacy v1 JSON deserialize | `config::session::tests::test_legacy_v1_session_deserializes_with_defaults` |
| v2 restore (two windows, focus, geometry) | `state::tests::test_restore_multi_window_session_v2` |
| v1 restore (single window) | `state::tests::test_restore_legacy_v1_single_window_session` |
| Capture → restore round-trip | `state::tests::test_capture_session_state_multi_window_roundtrip` |

## Manual QA checklist

1. **New Window** → open different files in each window → edit both
2. Quit cleanly → relaunch → both windows, tabs, active tab per window, and positions restore
3. Repeat with crash recovery (`session.recovery.json`) if desired
4. Place a v1 `session.json` (flat `tabs` only) → restores as a single primary window

| Platform | Status (2026-06) |
|----------|------------------|
| Windows | Unit/integration tests pass; full GUI restart QA recommended locally |
| macOS | Not run in CI — manual checklist above |
| Linux X11 | Not run in CI — manual checklist above |
| Linux Wayland | Not run in CI — manual checklist above; verify window position restore |

Platform-specific bugs found during QA should be filed as follow-ups unless trivial one-line fixes.
