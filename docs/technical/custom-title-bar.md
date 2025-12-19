# Custom Title Bar

## Overview

Ferrite uses a custom title bar instead of native OS window decorations. This provides a consistent look across Windows, macOS, and Linux, and allows for integrated menu placement.

## Key Files

- `src/app.rs` - Title bar rendering in `render_ui()` method
- `src/main.rs` - Window configuration with `with_decorations(false)`

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

#### Double-Click to Maximize

```rust
if drag_response.double_clicked() {
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
}
```

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

- [eframe Window](./eframe-window.md) - Window lifecycle and configuration
- [Keyboard Shortcuts](./keyboard-shortcuts.md) - Window shortcuts
- [Theme System](./theme-system.md) - Title bar theming
