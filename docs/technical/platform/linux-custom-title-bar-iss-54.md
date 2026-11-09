# Linux X11 custom title bar — maximize restore (ISS-54)

## Symptom (GrokBot QA, xfwm4 / X11)

With the default **custom** title bar on Linux X11:

1. **Double-click restore** after title-bar maximize: the maximize/restore icon toggled, but the client area stayed at the maximised size and position (e.g. full monitor width with a negative x offset).
2. **Drag-from-maximised**: dragging the title bar did not un-maximise or follow the pointer.

The maximise **button** restore path was already acceptable; the bug was specific to double-click toggle and deferred drag-out on the non-Win32 viewport path.

## Fix (Linux only)

Scope is `#[cfg(target_os = "linux")]` in `src/app/title_bar.rs`. Windows keeps `begin_maximized_drag_out()`; macOS keeps the legacy deferred `pending_drag_id` bool.

### Pre-maximise geometry

While the window is not maximised, each frame stores `PreMaximizeGeometry` in egui temp data (`("titlebar_pre_maximize", viewport_id)`):

- `outer_position` — `outer_rect.min`
- `inner_size` — `outer_rect.size()`

### Double-click restore

When maximised and the user double-clicks the drag area:

- Send `ViewportCommand::Maximized(false)`.
- Re-apply stored `OuterPosition` and `InnerSize` (X11 often leaves the client at maximised dimensions after `Maximized(false)` alone).

When not maximised, double-click still sends `Maximized(true)` only (geometry was captured on prior frames).

### Drag-from-maximised

Uses a small pure state machine in `src/app/title_bar_drag.rs` (unit-tested):

| State | Meaning |
|-------|---------|
| `Idle` | Normal |
| `PendingUnmaximise` | `Maximized(false)` (+ `InnerSize` and cursor-relative `OuterPosition` for drag-out) sent; wait one frame |
| `Dragging` | WM owns the drag after `StartDrag` |

Transitions:

- Pointer moves past the drag threshold while maximised → `Idle` → `PendingUnmaximise`, emit `Maximized(false)` (plus size/position for drag-out).
- Next title-bar frame while `PendingUnmaximise` → `Dragging`, emit `StartDrag` (GH #153 / ISS-54 one-frame delay).
- Primary release while `Dragging` → `Idle`.

Temp key: `("titlebar_drag_state", viewport_id)`.

## Key files

| File | Role |
|------|------|
| `src/app/title_bar.rs` | Linux wiring: geometry capture, double-click, drag-out, deferred `StartDrag` |
| `src/app/title_bar_drag.rs` | `PreMaximizeGeometry`, `title_bar_transition`, `apply_viewport_cmds`, tests |
| `src/app/platform.rs` | Win32-only `begin_maximized_drag_out()` |

## Tests

```bash
cargo test --bin ferrite title_bar_drag
```

Covers: drag start while maximised → `PendingUnmaximise` + `Maximized(false)`; next frame → `StartDrag`; double-click restore → geometry commands.

## Manual verification

On a real X11 session with a compositor (not Xvfb):

1. Double-click title bar to maximise, double-click again — window returns to prior size and position.
2. From maximised, drag the title bar — window un-maximises and follows the pointer.

## Related

- [Custom title bar](./custom-title-bar.md) — cross-platform overview
- GrokBot row ISS-54 / TTL-6a in `grokbot/New/qa-report-v0.3.1-linux.md`
- PRD: `docs/ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md` §5.6 ISS-54
