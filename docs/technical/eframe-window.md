# eframe Window System

## Overview

The main application window implementation using eframe/egui. Handles window lifecycle, dynamic title updates, responsive layout, and window state persistence.

## Key Files

- `src/app.rs` - Main `FerriteApp` struct implementing `eframe::App`
- `src/main.rs` - Application entry point with `eframe::run_native`

## Implementation Details

### FerriteApp Structure

```rust
pub struct FerriteApp {
    state: AppState,              // Central application state
    should_exit: bool,            // Exit flag after confirmation
    last_window_size: Option<egui::Vec2>,  // For detecting changes
    last_window_pos: Option<egui::Pos2>,   // For detecting changes
}
```

### Entry Point Configuration

The application is launched via `eframe::run_native()` with `NativeOptions`:

```rust
let viewport = egui::ViewportBuilder::default()
    .with_title(APP_NAME)
    .with_inner_size([window_size.width, window_size.height])
    .with_min_inner_size([400.0, 300.0])
    .with_position([x, y])  // If saved
    .with_maximized(maximized);
```

### Dynamic Window Title

The window title updates reactively based on the active tab:

| State | Title Format |
|-------|-------------|
| New file | `Untitled - Ferrite` |
| Saved file | `filename.md - Ferrite` |
| Modified | `filename.md* - Ferrite` |

Implementation uses `ViewportCommand::Title`:
```rust
ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
```

### Window State Persistence

Window size and position are tracked and saved to settings:

1. `update_window_state()` monitors viewport changes
2. Changes detected via `outer_rect` from viewport info
3. Settings updated with new `WindowSize` values
4. Saved on application shutdown via `AppState::shutdown()`

### Close Request Handling

The app intercepts close requests to check for unsaved changes:

```rust
if ctx.input(|i| i.viewport().close_requested()) {
    if !self.handle_close_request() {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
}
```

### UI Layout

| Panel | Purpose |
|-------|---------|
| `TopBottomPanel::top` | Menu bar (File, Help) |
| `TopBottomPanel::bottom` | Status bar (message, cursor position) |
| `CentralPanel` | Main content area (tabs, editor placeholder) |

### Dialog Windows

- **Confirmation Dialog**: Unsaved changes on exit/close
- **About Dialog**: Application version and info

## Dependencies Used

- `eframe` - Native application framework
- `egui` - Immediate mode GUI library
- `log` - Logging for debug/info messages

## Usage

Run the application:
```bash
cargo run
```

The window will:
1. Load size/position from settings (or use defaults: 1200x800)
2. Display with menu bar, status bar, and tab bar
3. Track and save window state changes
4. Show confirmation dialog on exit if unsaved changes exist

## Tests

This module uses manual testing:

1. **Window sizing**: Resize window → close → reopen → verify size restored
2. **Position persistence**: Move window → close → reopen → verify position
3. **Title updates**: Create tabs, modify content → verify title changes
4. **Exit handling**: Modify tab → try to close → verify confirmation dialog
5. **Menu actions**: File → New, Exit; Help → About

## Related Documentation

- [App State](./app-state.md) - AppState integration
- [Settings & Config](./settings-config.md) - WindowSize struct
