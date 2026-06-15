# Multi-Window File Routing & Single-Instance Integration

**Status:** Implemented (v0.3.1-experimental).  
**Design reference:** [multi-window.md](./multi-window.md) § Single-instance routing, § File open routing  
**Depends on:** [Multi-Window Implementation (MVP)](./multi-window-implementation.md) (task 15)

## Overview

File opens are **window-aware**: each action targets either the last OS-focused document window or the viewport currently rendering UI. The single-instance TCP protocol (Explorer double-click, CLI relaunch) forwards paths to the running process and opens them in the **last-focused** window, raising that viewport.

## Routing rules

| Action source | Target window | Helper |
|---------------|---------------|--------|
| File menu, ribbon Open, keyboard Ctrl+O | Last-focused (`focused_window_id`) | `FerriteApp::focused_file_open_window()` |
| Single-instance IPC (`handle_instance_paths`) | Last-focused | `focused_file_open_window()` |
| File tree click, drag-drop, quick switcher (Ctrl+P), search-in-files, wikilinks, backlinks | Viewport being rendered (`working_window_id`) | `FerriteApp::viewport_file_open_window()` |

Resolution lives in `AppState::file_open_target_window()` (`src/state.rs`):

1. Explicit `target_window` when valid → use it  
2. Else `focused_window_id` when window still exists  
3. Else fall back to primary (`windows[0]`)

Each viewport sets `working_window_id` before `render_ui()` and calls `set_focused_window(id)` when egui reports viewport focus (primary in `App::ui()`, secondaries in `windows.rs`).

## API surface

| Function | File | Role |
|----------|------|------|
| `open_file_smart_in_window(path, focus, app_time, target_window)` | `src/app/file_ops.rs` | Smart open (background load for ≥5 MB); `None` target → focused window |
| `open_file_with_focus(..., target_window)` | `src/state.rs` | Sync open; appends tab to target window strip |
| `open_file_loading(..., target_window)` | `src/state.rs` | Large-file placeholder tab in target window |
| `find_tab_by_path(path)` | `src/state.rs` | Returns `(window_id, strip_index)` across all windows |
| `handle_instance_paths(ctx)` | `src/app/file_ops.rs` | Drains single-instance channel; opens/focuses |
| `focus_document_window(ctx, window_id)` | `src/app/windows.rs` | `ViewportCommand::Focus` + `RequestUserAttention` on any window |

## Single-instance integration

Wire format unchanged (UTF-8 paths, one per line; `__FOCUS__` for bare launch). Message type is now `InstanceIncoming { paths, focus_only }` in `src/single_instance.rs`.

| Incoming | Behaviour |
|----------|-----------|
| File path(s) | Open each in `focused_file_open_window()`; raise that viewport |
| Directory | Open workspace (process-wide); focus target window |
| `__FOCUS__` only | Raise last-focused window, no new tabs |
| Paths + `__FOCUS__` | Open paths, then ensure window is raised |

Secondary process still exits after TCP forward; Windows uses `instance.pid` + `AllowSetForegroundWindow` before exit. See [single-instance.md](./single-instance.md).

## Entry points updated (task 16)

- `handle_open_file()` — focused window  
- File tree handler (`src/app/mod.rs`) — viewport  
- `handle_dropped_files(ctx, target_window)` — per-viewport in `render_ui()`  
- Quick switcher (`central_panel.rs`) — viewport + focus after open  
- Search navigation, wikilinks, recent-files popup, new-file-from-tree — viewport  
- CLI startup paths — primary only (single window at launch)

## Tests

- `state::tests::test_file_open_target_window_uses_focused_when_none`
- `state::tests::test_open_file_routes_to_target_window`
- `single_instance::tests::test_read_message_parses_paths_and_focus_only`

## Follow-up

| Task | Scope |
|------|-------|
| **17** | Session schema v2: restore all windows, tab lists, geometries |
