# Custom Title Bar

## Overview

Ferrite uses a custom title bar instead of native OS window decorations by default. This provides a consistent look across Windows, macOS, and Linux, and allows for integrated menu placement.

**Optional native title bar:** Linux and macOS users can enable **Use system title bar** in Settings → Appearance (restart required). See [System Title Bar Setting](./system-title-bar-setting.md). Windows always uses custom chrome.

## Key Files

- `src/app/title_bar.rs` — custom title bar panel (`render_title_bar`)
- `src/app/title_bar_drag.rs` — Linux ISS-54 restore/drag state machine (pure transitions + tests)
- `src/main.rs` - Window configuration via `apply_window_chrome()` (decorations off by default)
- `src/ui/window.rs` - `apply_window_chrome()` — decorations + borderless transparency workaround

## Implementation Details

### Window Configuration

The window is created without native decorations:

```rust
let viewport = eframe::egui::ViewportBuilder::default()
    .with_title(APP_NAME)
    .with_decorations(false)  // No native title bar
    .with_inner_size([width, height])
    .with_min_inner_size([400.0, 300.0]);
```

### Title Bar Structure

The title bar and menu bar are combined into a single `TopBottomPanel`:

```
┌────────────────────────────────────────────────────────────────────┐
│ 📝 Filename - Ferrite                                  [_][□][×]  │
│ File  View  Help                                                   │
└────────────────────────────────────────────────────────────────────┘
```

### Components

| Element | Description |
|---------|-------------|
| App Icon | 📝 emoji placeholder |
| Window Title | Dynamic: "Filename - Ferrite" |
| Drag Area | Empty space allows window dragging |
| Minimize | `−` line button, triggers `ViewportCommand::Minimized` |
| Maximize/Restore | `□`/`❐` button, toggles `ViewportCommand::Maximized` |
| Close | `×` button with red hover, triggers exit flow |
| Menu Bar | File, View, Help menus below title |

### Theme Integration

Colors adapt to light/dark mode:

```rust
let title_bar_color = if is_dark {
    Color32::from_rgb(32, 32, 32)
} else {
    Color32::from_rgb(240, 240, 240)
};

let close_hover_color = Color32::from_rgb(232, 17, 35);  // Windows-style red
```

### Window Controls

#### Drag to Move

```rust
let drag_response = ui.allocate_rect(drag_rect, Sense::click_and_drag());
if drag_response.dragged() {
    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
}
```

#### Drag out of maximized (non-Windows fallback)

On Windows, `begin_maximized_drag_out()` restores under the cursor in one atomic Win32 call before `StartDrag`.

On Linux (or Win32 failure on Windows), the fallback sends `Maximized(false)` plus size/position, then **defers** `StartDrag` one frame. Linux uses `TitleBarDragState` in temp data; other non-Windows platforms use `pending_drag_id`. Same-frame restore + drag caused stuck WM state ([#153](https://github.com/OlaProeis/Ferrite/issues/153)).

On **Linux X11**, double-click restore also re-applies stored pre-maximise `OuterPosition` / `InnerSize` after `Maximized(false)` — see [linux-custom-title-bar-iss-54.md](./linux-custom-title-bar-iss-54.md).

#### Double-Click to Maximize

Non-Linux: toggles `ViewportCommand::Maximized(!is_maximized)`.

Linux: maximise sends `Maximized(true)`; restore from maximised sends `Maximized(false)` plus stored geometry (ISS-54).

### Window State Tracking

The app tracks window position and size for persistence:

```rust
fn update_window_state(&mut self, ctx: &Context) -> bool {
    ctx.input(|i| {
        if let Some(rect) = i.viewport().outer_rect {
            // Update settings.window_size with current position/size
        }
    });
}
```

## Dynamic Window Title

The title updates based on the active tab:

```rust
fn window_title(&self) -> String {
    if let Some(tab) = self.state.active_tab() {
        format!("{} - {}", tab.title(), APP_NAME)
    } else {
        APP_NAME.to_string()
    }
}
```

Updated via viewport command each frame:

```rust
ctx.send_viewport_cmd(ViewportCommand::Title(title));
```

## Close Request Handling

Close button triggers unsaved changes check:

```rust
if close_btn.clicked() {
    if self.state.request_exit() {
        self.should_exit = true;
    }
    // Otherwise confirmation dialog is shown
}
```

## Related Documentation

- [Linux X11 title bar restore (ISS-54)](./linux-custom-title-bar-iss-54.md)
- [eframe Window](./eframe-window.md) - Window lifecycle and configuration
- [Keyboard Shortcuts](./keyboard-shortcuts.md) - Window shortcuts
- [Theme System](./theme-system.md) - Title bar theming
