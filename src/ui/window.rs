//! Custom window resize handling for borderless windows.
//!
//! This module provides custom resize functionality for windows with native
//! decorations disabled (`with_decorations(false)`). It detects mouse proximity
//! to window edges, changes cursor icons appropriately, and initiates resize
//! operations via egui's ViewportCommand system.

// Allow clippy lints:
// - collapsible_if: Corner detection logic is clearer with nested conditions
#![allow(clippy::collapsible_if)]

//! ## Usage
//!
//! Call `handle_window_resize` at the start of each frame, before rendering UI:
//!
//! ```ignore
//! fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!     handle_window_resize(ctx, &mut self.resize_state);
//!     // ... rest of UI
//! }
//! ```

use eframe::egui::{self, CursorIcon, Pos2, Rect, ResizeDirection, ViewportCommand};

/// Width of the resize border in logical pixels.
const RESIZE_BORDER_WIDTH: f32 = 5.0;

/// Corner grab area size (slightly larger than edge for easier corner detection).
const CORNER_GRAB_SIZE: f32 = 10.0;

/// State for tracking window resize operations.
#[derive(Debug, Clone, Default)]
pub struct WindowResizeState {
    /// Currently detected resize direction (None if not hovering edge).
    current_direction: Option<ResizeDirection>,
    /// Whether we're actively in a resize operation.
    is_resizing: bool,
}

impl WindowResizeState {
    /// Create a new resize state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if currently resizing.
    pub fn is_resizing(&self) -> bool {
        self.is_resizing
    }

    /// Get current resize direction.
    pub fn current_direction(&self) -> Option<ResizeDirection> {
        self.current_direction
    }
}

/// Handle window resize for borderless windows.
///
/// This function should be called at the start of each frame, before rendering
/// any UI elements. It:
///
/// 1. Detects if the mouse is hovering over a resize edge/corner
/// 2. Changes the cursor icon to indicate resize capability
/// 3. Initiates resize operation when mouse is pressed
///
/// Returns `true` if a resize operation was initiated (the calling code may
/// want to skip drag-to-move in this case).
pub fn handle_window_resize(ctx: &egui::Context, state: &mut WindowResizeState) -> bool {
    // Don't handle resize if window is maximized
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_maximized {
        state.current_direction = None;
        state.is_resizing = false;
        return false;
    }

    // Get pointer position and mouse state
    // Note: hover_pos() is in window-local coordinates (0,0 is top-left of window)
    let (pointer_pos, primary_pressed, primary_down) = ctx.input(|i| {
        let pos = i.pointer.hover_pos();
        let pressed = i.pointer.primary_pressed();
        let down = i.pointer.primary_down();
        (pos, pressed, down)
    });

    // Use screen_rect() which gives us the local coordinate rect of the window
    // This is (0,0) to (width, height) in window-local coordinates
    let window_rect = ctx.screen_rect();

    let Some(pointer_pos) = pointer_pos else {
        if !primary_down {
            state.current_direction = None;
            state.is_resizing = false;
        }
        return false;
    };

    // If we're in a resize operation, continue until mouse is released
    if state.is_resizing {
        if !primary_down {
            state.is_resizing = false;
            state.current_direction = None;
        }
        return true;
    }

    // Detect resize direction based on pointer position
    let direction = detect_resize_direction(window_rect, pointer_pos);

    // Update state
    state.current_direction = direction;

    // Set cursor based on direction
    if let Some(dir) = direction {
        ctx.set_cursor_icon(direction_to_cursor(dir));

        // Initiate resize on mouse press
        if primary_pressed {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            state.is_resizing = true;
            return true;
        }
    }

    false
}

/// Detect which resize direction (if any) the pointer is in.
fn detect_resize_direction(window_rect: Rect, pointer_pos: Pos2) -> Option<ResizeDirection> {
    let min = window_rect.min;
    let max = window_rect.max;

    // Check if pointer is near each edge
    let near_left = pointer_pos.x < min.x + RESIZE_BORDER_WIDTH;
    let near_right = pointer_pos.x > max.x - RESIZE_BORDER_WIDTH;
    let near_top = pointer_pos.y < min.y + RESIZE_BORDER_WIDTH;
    let near_bottom = pointer_pos.y > max.y - RESIZE_BORDER_WIDTH;

    // Check if pointer is in corner zones (larger area for easier grabbing)
    let in_left_zone = pointer_pos.x < min.x + CORNER_GRAB_SIZE;
    let in_right_zone = pointer_pos.x > max.x - CORNER_GRAB_SIZE;
    let in_top_zone = pointer_pos.y < min.y + CORNER_GRAB_SIZE;
    let in_bottom_zone = pointer_pos.y > max.y - CORNER_GRAB_SIZE;

    // Corners take priority (check corner combinations first)
    if near_top || in_top_zone {
        if (near_left || in_left_zone)
            && pointer_pos.x < min.x + CORNER_GRAB_SIZE
            && pointer_pos.y < min.y + CORNER_GRAB_SIZE
        {
            return Some(ResizeDirection::NorthWest);
        }
        if (near_right || in_right_zone)
            && pointer_pos.x > max.x - CORNER_GRAB_SIZE
            && pointer_pos.y < min.y + CORNER_GRAB_SIZE
        {
            return Some(ResizeDirection::NorthEast);
        }
    }

    if near_bottom || in_bottom_zone {
        if (near_left || in_left_zone)
            && pointer_pos.x < min.x + CORNER_GRAB_SIZE
            && pointer_pos.y > max.y - CORNER_GRAB_SIZE
        {
            return Some(ResizeDirection::SouthWest);
        }
        if (near_right || in_right_zone)
            && pointer_pos.x > max.x - CORNER_GRAB_SIZE
            && pointer_pos.y > max.y - CORNER_GRAB_SIZE
        {
            return Some(ResizeDirection::SouthEast);
        }
    }

    // Then check edges (only if clearly on an edge, not in a corner zone)
    if near_left && !in_top_zone && !in_bottom_zone {
        return Some(ResizeDirection::West);
    }
    if near_right && !in_top_zone && !in_bottom_zone {
        return Some(ResizeDirection::East);
    }
    if near_top && !in_left_zone && !in_right_zone {
        return Some(ResizeDirection::North);
    }
    if near_bottom && !in_left_zone && !in_right_zone {
        return Some(ResizeDirection::South);
    }

    None
}

/// Convert a resize direction to the appropriate cursor icon.
fn direction_to_cursor(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::North => CursorIcon::ResizeNorth,
        ResizeDirection::South => CursorIcon::ResizeSouth,
        ResizeDirection::East => CursorIcon::ResizeEast,
        ResizeDirection::West => CursorIcon::ResizeWest,
        ResizeDirection::NorthEast => CursorIcon::ResizeNorthEast,
        ResizeDirection::NorthWest => CursorIcon::ResizeNorthWest,
        ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
        ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
    }
}

/// Check if a pointer position is within the resize border zone.
///
/// This can be used by other UI elements (like the title bar) to determine
/// if they should defer to resize handling.
#[allow(dead_code)]
pub fn is_in_resize_zone(window_rect: Rect, pointer_pos: Pos2) -> bool {
    detect_resize_direction(window_rect, pointer_pos).is_some()
}

/// Get the resize zone rectangle for a given edge.
///
/// Useful for debugging or custom hit testing.
#[allow(dead_code)]
pub fn get_resize_zone_rect(window_rect: Rect, edge: ResizeDirection) -> Rect {
    let min = window_rect.min;
    let max = window_rect.max;
    let b = RESIZE_BORDER_WIDTH;
    let c = CORNER_GRAB_SIZE;

    match edge {
        ResizeDirection::North => {
            Rect::from_min_max(Pos2::new(min.x + c, min.y), Pos2::new(max.x - c, min.y + b))
        }
        ResizeDirection::South => {
            Rect::from_min_max(Pos2::new(min.x + c, max.y - b), Pos2::new(max.x - c, max.y))
        }
        ResizeDirection::West => {
            Rect::from_min_max(Pos2::new(min.x, min.y + c), Pos2::new(min.x + b, max.y - c))
        }
        ResizeDirection::East => {
            Rect::from_min_max(Pos2::new(max.x - b, min.y + c), Pos2::new(max.x, max.y - c))
        }
        ResizeDirection::NorthWest => {
            Rect::from_min_max(Pos2::new(min.x, min.y), Pos2::new(min.x + c, min.y + c))
        }
        ResizeDirection::NorthEast => {
            Rect::from_min_max(Pos2::new(max.x - c, min.y), Pos2::new(max.x, min.y + c))
        }
        ResizeDirection::SouthWest => {
            Rect::from_min_max(Pos2::new(min.x, max.y - c), Pos2::new(min.x + c, max.y))
        }
        ResizeDirection::SouthEast => {
            Rect::from_min_max(Pos2::new(max.x - c, max.y - c), Pos2::new(max.x, max.y))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_corners() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));

        // Northwest corner
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(2.0, 2.0)),
            Some(ResizeDirection::NorthWest)
        );

        // Northeast corner
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(98.0, 2.0)),
            Some(ResizeDirection::NorthEast)
        );

        // Southwest corner
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(2.0, 98.0)),
            Some(ResizeDirection::SouthWest)
        );

        // Southeast corner
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(98.0, 98.0)),
            Some(ResizeDirection::SouthEast)
        );
    }

    #[test]
    fn test_detect_edges() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));

        // North edge (middle)
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(50.0, 2.0)),
            Some(ResizeDirection::North)
        );

        // South edge (middle)
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(50.0, 98.0)),
            Some(ResizeDirection::South)
        );

        // West edge (middle)
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(2.0, 50.0)),
            Some(ResizeDirection::West)
        );

        // East edge (middle)
        assert_eq!(
            detect_resize_direction(rect, Pos2::new(98.0, 50.0)),
            Some(ResizeDirection::East)
        );
    }

    #[test]
    fn test_detect_center() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));

        // Center of window - no resize
        assert_eq!(detect_resize_direction(rect, Pos2::new(50.0, 50.0)), None);

        // Just inside the border
        assert_eq!(detect_resize_direction(rect, Pos2::new(20.0, 20.0)), None);
    }

    #[test]
    fn test_cursor_mapping() {
        assert_eq!(
            direction_to_cursor(ResizeDirection::North),
            CursorIcon::ResizeNorth
        );
        assert_eq!(
            direction_to_cursor(ResizeDirection::SouthEast),
            CursorIcon::ResizeSouthEast
        );
    }
}
