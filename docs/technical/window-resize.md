# Custom Window Resize for Borderless Windows

## Overview

This document describes the custom window resize functionality implemented for Ferrite's borderless window mode. Since the application uses `with_decorations(false)` to provide a custom title bar, native OS resize handles are not available. This module provides custom edge/corner detection and resize initiation.

## Architecture

### Module Location

- **Source**: `src/ui/window.rs`
- **Integration**: `src/app.rs`

### Key Components

#### `WindowResizeState`

Tracks the current resize operation state:

```rust
pub struct WindowResizeState {
    current_direction: Option<ResizeDirection>,
    is_resizing: bool,
}
```

- `current_direction`: The detected resize direction based on mouse position (None if not near an edge)
- `is_resizing`: Whether a resize operation is currently active (mouse held down)

#### `handle_window_resize()`

The main entry point called at the start of each frame:

1. **Early exit if maximized**: Resize is disabled when window is maximized
2. **Get window rect and pointer position**: Required for edge detection
3. **Continue active resize**: If a resize is in progress, maintain state until mouse is released
4. **Detect resize direction**: Check if pointer is near any edge/corner
5. **Set cursor icon**: Change cursor to indicate resize capability
6. **Initiate resize**: On mouse press, send `ViewportCommand::BeginResize(direction)` to egui

### Resize Zones

The implementation defines two key constants:

- **`RESIZE_BORDER_WIDTH`** (5px): The width of edge resize zones
- **`CORNER_GRAB_SIZE`** (10px): The size of corner grab areas (larger for easier targeting)

### Edge Detection Logic

The `detect_resize_direction()` function:

1. Checks proximity to each edge (left, right, top, bottom)
2. Prioritizes corners over edges (corners are checked first)
3. Returns the appropriate `ResizeDirection` variant

Supported directions:
- `North`, `South`, `East`, `West` (edges)
- `NorthEast`, `NorthWest`, `SouthEast`, `SouthWest` (corners)

### Cursor Icons

Each direction maps to a corresponding cursor icon:

| Direction | Cursor Icon |
|-----------|-------------|
| North | `ResizeNorth` (↑) |
| South | `ResizeSouth` (↓) |
| East | `ResizeEast` (→) |
| West | `ResizeWest` (←) |
| NorthEast | `ResizeNorthEast` (↗) |
| NorthWest | `ResizeNorthWest` (↖) |
| SouthEast | `ResizeSouthEast` (↘) |
| SouthWest | `ResizeSouthWest` (↙) |

## Integration with Title Bar

The title bar drag-to-move functionality is updated to defer to resize handling:

```rust
// Handle drag to move window (but not if we're in a resize zone)
let is_in_resize = self.window_resize_state.current_direction().is_some()
    || self.window_resize_state.is_resizing();
if drag_response.dragged() && !is_in_resize {
    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
}
```

This prevents conflicts between window move and resize operations when the mouse is near edges.

## Frame Order

The resize handling must be called **early** in the frame, before any UI rendering:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Handle window resize for borderless window (must be early, before UI)
    handle_window_resize(ctx, &mut self.window_resize_state);
    
    // ... rest of UI rendering
}
```

This ensures:
1. Resize state is updated before UI checks it
2. Cursor icon changes are applied immediately
3. Resize commands are sent before any UI interaction

## Platform Considerations

- **Windows**: Fully supported via winit/eframe integration
- **macOS**: Should work similarly (winit handles platform specifics)
- **Linux**: Supported on X11 and Wayland (winit handles platform specifics)

Note: Platform-specific behavior may vary slightly depending on the window manager.

## Testing Checklist

- [ ] Cursor changes to resize icons when hovering edges
- [ ] Cursor changes to diagonal icons when hovering corners
- [ ] All 8 resize directions work correctly
- [ ] Minimum window size is enforced (400x300)
- [ ] Resize is disabled when window is maximized
- [ ] Title bar drag-to-move works in center area
- [ ] Title bar does not initiate move when dragging from edges
- [ ] Double-click on title bar still toggles maximize
- [ ] Smooth 60fps performance during resize

## API Reference

### Public Functions

```rust
/// Handle window resize for borderless windows.
/// Returns `true` if a resize operation was initiated.
pub fn handle_window_resize(ctx: &egui::Context, state: &mut WindowResizeState) -> bool

/// Check if a pointer position is within the resize border zone.
pub fn is_in_resize_zone(window_rect: Rect, pointer_pos: Pos2) -> bool

/// Get the resize zone rectangle for a given edge.
pub fn get_resize_zone_rect(window_rect: Rect, edge: ResizeDirection) -> Rect
```

### WindowResizeState Methods

```rust
/// Create a new resize state.
pub fn new() -> Self

/// Check if currently resizing.
pub fn is_resizing(&self) -> bool

/// Get current resize direction.
pub fn current_direction(&self) -> Option<ResizeDirection>
```

## Dependencies

- `eframe::egui` - UI framework with viewport commands
- `egui::ResizeDirection` - Direction enum for resize operations
- `egui::ViewportCommand::BeginResize` - Command to initiate native resize
