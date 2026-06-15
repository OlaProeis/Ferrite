# Single-Instance Protocol

## Overview

Ensures only one Ferrite **process** runs at a time. When a second instance is launched (e.g., double-clicking a file in Windows Explorer), it forwards file paths to the already-running process via local TCP, then exits. The running process opens received paths in the **last-focused document window** (multi-window aware) and raises that viewport. On Windows, the secondary process also grants the primary process foreground permission before exiting so the existing window can be raised more reliably.

Multi-window file routing details: [multi-window-file-routing.md](./multi-window-file-routing.md).

## Key Files

| File | Purpose |
|------|---------|
| `src/single_instance.rs` | Protocol implementation: lock file, pid file, TCP client, background accept thread, channel-based path delivery |
| `src/main.rs` | Instance check **early** in startup (before config/icon loading) via `try_acquire_instance` |
| `src/app/mod.rs` | Stores `SingleInstanceListener`, provides egui context for repaint wakeup |
| `src/app/file_ops.rs` | `handle_instance_paths()` — drains `InstanceIncoming` channel, opens paths in focused window |
| `src/app/windows.rs` | `focus_document_window()` — raises any document viewport by id |
| `src/platform/mod.rs` | Windows foreground-permission helper for Explorer-launched secondary processes |

## Architecture

```
Secondary instance                 Primary instance
─────────────────                  ────────────────

 main() starts                      Background thread
   ↓                                (blocking accept loop)
 Parse CLI args                          ↓
   ↓                              listener.accept() ← blocks
 Read lock file → port                  ↓
   ↓                              Connection arrives!
 TCP connect(port)  ───────────→  Read paths from stream
   ↓                                     ↓
 AllowSetForegroundWindow(pid)    Send paths via mpsc channel
 Write paths + shutdown(Write)             ↓
   ↓                                     ↓
 Exit process                     ctx.request_repaint() ← wakes UI
                                         ↓
                                  UI thread: poll() drains channel
                                         ↓
                                  Focus + attention request
                                         ↓
                                  Open file in focused window (instant)
```

## Protocol

1. **Lock file**: `{config_dir}/instance.lock` contains the TCP port of the running instance as plain text
   - Windows: `%APPDATA%\ferrite\instance.lock`
   - Linux: `~/.config/ferrite/instance.lock`
   - macOS: `~/Library/Application Support/ferrite/instance.lock`

2. **PID file**: `{config_dir}/instance.pid` stores the primary process ID
   - Used on Windows so the Explorer-launched secondary process can call `AllowSetForegroundWindow(primary_pid)` before exiting

3. **Startup flow** (runs early, before config/logging/icon loading):
   ```text
   Parse CLI args → Read lock file → port exists?
     YES → connect to port (500ms timeout)
       SUCCESS → (Windows: allow foreground for primary PID) → send paths, shutdown(Write), exit
       FAIL → stale lock, delete, become primary
     NO → become primary
   ```

4. **Primary instance**:
   - Binds `TcpListener` on `127.0.0.1:0` (OS picks port)
   - Writes port to lock file
   - Writes `std::process::id()` to `instance.pid`
   - Spawns background thread (`single-instance-accept`) that blocks on `accept()`
   - Accepted connections are read with 100ms timeout (localhost data arrives in <1ms)
   - Paths sent to UI via `mpsc::channel`; UI woken via `ctx.request_repaint()`
   - Accepted messages are `InstanceIncoming { paths, focus_only }` (paths plus optional `__FOCUS__` flag)
   - UI thread drains channel with `try_recv()` (non-blocking, nanoseconds), then:
     - Resolves target via `AppState::file_open_target_window(None)` (last-focused window)
     - Opens paths with `open_file_smart_in_window(..., Some(target_window))`
     - Calls `focus_document_window(ctx, target_window)` (`ViewportCommand::Focus` + `RequestUserAttention`)

5. **Secondary instance** (exits in <100ms):
   - Connects to `127.0.0.1:{port}` with 500ms timeout
   - On Windows, reads `instance.pid` and calls `AllowSetForegroundWindow(primary_pid)` before exit
   - Sends file paths as UTF-8 lines (one per line)
   - Sends `__FOCUS__` if no paths (just bring window forward)
   - Calls `stream.shutdown(Write)` to send FIN immediately
   - Exits cleanly via `return Ok(())`

6. **Cleanup**: Lock file and pid file removed on `Drop` of `SingleInstanceListener`

## Performance Design

The protocol is designed for instant response (<50ms end-to-end):

| Component | Technique | Latency |
|-----------|-----------|---------|
| Secondary startup | Single-instance check runs before config/logging/icon loading | ~50ms |
| Windows foreground handoff | Secondary process grants foreground permission to primary | ~0ms |
| TCP delivery | Explicit `shutdown(Write)` sends EOF immediately | <1ms |
| Primary accept | Dedicated blocking thread, no polling delay | <1ms |
| UI wakeup | `ctx.request_repaint()` from background thread bypasses idle intervals | <1ms |
| Channel drain | `mpsc::try_recv()` is non-blocking | nanoseconds |

Previously, the protocol used per-frame polling of a non-blocking listener on the UI thread. In deep idle (500ms repaint interval), this caused 1-2 second delays. The background thread + repaint wakeup architecture eliminates this entirely.

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Stale lock (crashed instance) | TCP connect fails → lock deleted → new primary |
| Stale pid file | Ignored if unreadable; focus falls back to viewport commands only |
| No paths (bare launch) | `__FOCUS__` signal sent → last-focused document window raised |
| Multiple Ferrite windows open | Paths open in last-focused window; not always primary ROOT |
| Config dir unavailable | Warning logged, app runs without single-instance |
| Listener bind failure | App runs normally, just no IPC |
| Directory path received | Opened as workspace (same as drag-and-drop) |
| Multiple paths | All opened as tabs; first directory becomes workspace |
| App shutting down | Channel sender dropped → accept thread exits cleanly |

## Integration Points

- Background thread provides repaint context via `Arc<Mutex<Option<egui::Context>>>`
- `handle_instance_paths()` calls `set_repaint_ctx()` each frame (cheap when already set)
- Uses `focus_document_window()` so the correct child viewport is raised when multiple windows exist
- Uses `instance.pid` + `AllowSetForegroundWindow` on Windows so Explorer-launched secondary processes can transfer foreground rights to the primary process
- Reuses `open_file_smart_in_window()` and `state.open_workspace()` for consistent behavior
- Lock and pid files are stored in the same config directory as other Ferrite config (`get_config_dir()`)

## No New Dependencies

Uses only `std::net` (TcpListener/TcpStream), `std::sync::mpsc`, `std::thread`, and `egui::Context` — no external crates added.
