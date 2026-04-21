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

/// Height of the title bar area to exclude from north edge resize detection.
/// This prevents cursor flicker conflicts with window control buttons.
const TITLE_BAR_EXCLUSION_HEIGHT: f32 = 35.0;

/// Width of the button area on the right side of the title bar.
/// This area contains window control buttons (close, maximize, minimize, fullscreen)
/// and other title bar buttons (settings, zen mode, view mode).
/// North edge resize is only disabled in this area to allow resize from the
/// left and center portions of the window's top edge.
const TITLE_BAR_BUTTON_AREA_WIDTH: f32 = 280.0;

/// Right margin gap (in logical pixels) between window control buttons and the
/// window's right edge.  Must be larger than `CORNER_GRAB_SIZE` so the NE
/// corner grab zone stays button-free.  Kept in sync with the
/// `ui.add_space(12.0)` call in `title_bar.rs`.
const TITLE_BAR_BUTTON_RIGHT_MARGIN: f32 = 12.0;

/// State for tracking window resize operations.
#[derive(Debug, Clone, Default)]
pub struct WindowResizeState {
    /// Currently detected resize direction (None if not hovering edge).
    current_direction: Option<ResizeDirection>,
    /// Whether we're actively in a resize operation.
    is_resizing: bool,
    /// Cursor to apply at end of frame (to override UI element cursors).
    pending_cursor: Option<CursorIcon>,
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

    /// Apply the pending resize cursor if one was set.
    /// Call this at the END of your update() function to ensure resize cursor
    /// takes priority over UI element cursors.
    pub fn apply_cursor(&mut self, ctx: &egui::Context) {
        if let Some(cursor) = self.pending_cursor.take() {
            ctx.set_cursor_icon(cursor);
        }
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
/// The function excludes the title bar area from north-edge resize detection
/// to prevent cursor flicker conflicts with window control buttons (close,
/// maximize, minimize).
///
/// Returns `true` if a resize operation was initiated (the calling code may
/// want to skip drag-to-move in this case).
pub fn handle_window_resize(ctx: &egui::Context, state: &mut WindowResizeState) -> bool {
    // Don't handle resize if window is maximized
    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_maximized {
        state.current_direction = None;
        state.is_resizing = false;
        state.pending_cursor = None;
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
            state.pending_cursor = None;
        }
        return false;
    };

    // If we're in a resize operation, continue until mouse is released
    if state.is_resizing {
        if !primary_down {
            state.is_resizing = false;
            state.current_direction = None;
            state.pending_cursor = None;
        }
        return true;
    }

    // Detect resize direction based on pointer position
    // Pass title bar exclusion height to avoid conflicts with window control buttons
    let direction = detect_resize_direction_with_exclusion(
        window_rect,
        pointer_pos,
        TITLE_BAR_EXCLUSION_HEIGHT,
    );

    // Update state
    state.current_direction = direction;

    // Set cursor based on direction
    // We store it as pending and apply at the end of the frame to override UI cursors
    if let Some(dir) = direction {
        let desired_cursor = direction_to_cursor(dir);
        state.pending_cursor = Some(desired_cursor);

        // Initiate resize on mouse press
        if primary_pressed {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            state.is_resizing = true;
            return true;
        }
    } else {
        state.pending_cursor = None;
    }

    false
}

/// Detect which resize direction (if any) the pointer is in.
fn detect_resize_direction(window_rect: Rect, pointer_pos: Pos2) -> Option<ResizeDirection> {
    detect_resize_direction_with_exclusion(window_rect, pointer_pos, 0.0)
}

/// Detect which resize direction (if any) the pointer is in, with title bar exclusion.
///
/// The `title_bar_height` parameter specifies the height of the title bar area
/// where north-edge resize detection should be disabled to avoid conflicts
/// with window control buttons (close, maximize, minimize, fullscreen).
///
/// North edge resize is only disabled in the RIGHT portion of the title bar
/// where the window control buttons are located. The left and center portions
/// of the top edge can still be used for resizing.
fn detect_resize_direction_with_exclusion(
    window_rect: Rect,
    pointer_pos: Pos2,
    title_bar_height: f32,
) -> Option<ResizeDirection> {
    let min = window_rect.min;
    let max = window_rect.max;

    // Check if pointer is in the title bar exclusion zone (top area)
    let in_title_bar = pointer_pos.y < min.y + title_bar_height;

    // Check if pointer is in the button area (right side of title bar)
    // Window control buttons (close, max, min, fullscreen) and title bar buttons
    // (settings, zen mode, view mode) are all on the right side.
    let in_button_area = pointer_pos.x > max.x - TITLE_BAR_BUTTON_AREA_WIDTH;

    // North edge/corner resize is only disabled when BOTH in title bar AND in button area
    let disable_north_resize = in_title_bar && in_button_area;

    // Check if pointer is near each edge
    let near_left = pointer_pos.x < min.x + RESIZE_BORDER_WIDTH;
    let near_right = pointer_pos.x > max.x - RESIZE_BORDER_WIDTH;
    let near_top = pointer_pos.y < min.y + RESIZE_BORDER_WIDTH;
    let near_bottom = pointer_pos.y > max.y - RESIZE_BORDER_WIDTH;

    // Check if pointer is in corner zones (larger area for easier grabbing)
    // These are used to exclude corners from edge detection
    let in_left_zone = pointer_pos.x < min.x + CORNER_GRAB_SIZE;
    let in_right_zone = pointer_pos.x > max.x - CORNER_GRAB_SIZE;
    let in_top_zone = pointer_pos.y < min.y + CORNER_GRAB_SIZE;
    let in_bottom_zone = pointer_pos.y > max.y - CORNER_GRAB_SIZE;

    // Corners take priority (check corner combinations first)
    // NorthWest corner: always enabled (no buttons on left side)
    // NorthEast corner: disabled when in button area (buttons are there)
    if near_top || in_top_zone {
        // NorthWest corner - enabled unless we're in the button area AND title bar
        if (near_left || in_left_zone)
            && pointer_pos.x < min.x + CORNER_GRAB_SIZE
            && pointer_pos.y < min.y + CORNER_GRAB_SIZE
            && !disable_north_resize
        {
            return Some(ResizeDirection::NorthWest);
        }
        // NorthEast corner – enabled because the button right margin
        // (TITLE_BAR_BUTTON_RIGHT_MARGIN = 12 px) is larger than the corner
        // grab zone (CORNER_GRAB_SIZE = 10 px).  Any cursor position that
        // satisfies `x > max.x - CORNER_GRAB_SIZE` is therefore always
        // outside all buttons, so resize is safe everywhere in this zone.
        if (near_right || in_right_zone)
            && pointer_pos.x > max.x - CORNER_GRAB_SIZE
            && pointer_pos.y < min.y + CORNER_GRAB_SIZE
            && (!in_title_bar || pointer_pos.x > max.x - TITLE_BAR_BUTTON_RIGHT_MARGIN)
        {
            return Some(ResizeDirection::NorthEast);
        }
    }

    // South corners always work
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

    // Edge detection - exclude corner zones
    // For East/West edges:
    // - Normally exclude top and bottom corner zones so corners take priority
    // - In title bar area, still exclude corner zones (NorthEast corner is disabled,
    //   but we don't want to show East edge in that corner either as it conflicts with buttons)
    // - West edge can work in top zone when NorthWest corner is enabled (but the corner will take priority)

    // Check if we're in the actual corner zones (both dimensions)
    let in_northwest_corner = in_left_zone && in_top_zone;
    let in_northeast_corner = in_right_zone && in_top_zone;
    let in_southwest_corner = in_left_zone && in_bottom_zone;
    let in_southeast_corner = in_right_zone && in_bottom_zone;

    // West edge: exclude corner zones
    // In title bar outside button area, NorthWest corner works, so exclude that zone
    // In title bar in button area, corners are disabled, but still exclude corner zones to avoid confusion
    if near_left && !in_northwest_corner && !in_southwest_corner {
        return Some(ResizeDirection::West);
    }
    // East edge: exclude corner zones
    // In title bar, NorthEast corner is disabled, and we also want to block East edge there
    // (since that's where the window control buttons are)
    if near_right && !in_northeast_corner && !in_southeast_corner {
        return Some(ResizeDirection::East);
    }
    // North edge - only disabled when in button area of title bar
    if near_top && !in_left_zone && !in_right_zone && !disable_north_resize {
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

// ═══════════════════════════════════════════════════════════════════════════════
// Viewport Constraint Utilities
// ═══════════════════════════════════════════════════════════════════════════════

/// Constraints for a floating panel or window.
#[derive(Debug, Clone)]
pub struct PanelConstraints {
    /// Minimum width of the panel.
    pub min_width: f32,
    /// Maximum width of the panel.
    pub max_width: f32,
    /// Minimum height of the panel.
    pub min_height: f32,
    /// Maximum height of the panel.
    pub max_height: f32,
    /// Margin from viewport edges.
    pub margin: f32,
}

impl Default for PanelConstraints {
    fn default() -> Self {
        Self {
            min_width: 200.0,
            max_width: 800.0,
            min_height: 150.0,
            max_height: 600.0,
            margin: 8.0,
        }
    }
}

/// Result of constraining a panel to viewport bounds.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are informational for debugging
pub struct ConstrainedPanel {
    /// The constrained position (top-left corner).
    pub pos: Pos2,
    /// The constrained size.
    pub size: egui::Vec2,
    /// Whether the panel was resized to fit.
    pub was_resized: bool,
    /// Whether the panel was repositioned to fit.
    pub was_repositioned: bool,
}

/// Constrain a rectangle to fit within viewport bounds.
///
/// This function ensures a panel or floating window fits entirely within
/// the visible viewport, respecting minimum/maximum size constraints and
/// maintaining a margin from edges.
///
/// # Arguments
///
/// * `desired_rect` - The desired position and size of the panel
/// * `viewport` - The available viewport bounds (typically `ctx.screen_rect()`)
/// * `constraints` - Size constraints and margin settings
///
/// # Returns
///
/// A `ConstrainedPanel` with the adjusted position and size.
///
/// # Example
///
/// ```ignore
/// let viewport = ctx.screen_rect();
/// let desired = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(500.0, 400.0));
/// let result = constrain_rect_to_viewport(desired, viewport, &PanelConstraints::default());
/// // Use result.pos and result.size to position the window
/// ```
pub fn constrain_rect_to_viewport(
    desired_rect: Rect,
    viewport: Rect,
    constraints: &PanelConstraints,
) -> ConstrainedPanel {
    let margin = constraints.margin;
    let mut was_resized = false;
    let mut was_repositioned = false;

    // Calculate the available area (viewport minus margins)
    let available = Rect::from_min_max(
        Pos2::new(viewport.min.x + margin, viewport.min.y + margin),
        Pos2::new(viewport.max.x - margin, viewport.max.y - margin),
    );

    // Calculate maximum allowed dimensions based on available space
    let max_available_width = available.width().max(constraints.min_width);
    let max_available_height = available.height().max(constraints.min_height);

    // Constrain size to both configured limits and available space
    let mut width = desired_rect.width();
    let mut height = desired_rect.height();

    // Apply size constraints
    if width < constraints.min_width {
        width = constraints.min_width;
        was_resized = true;
    }
    if width > constraints.max_width {
        width = constraints.max_width;
        was_resized = true;
    }
    if width > max_available_width {
        width = max_available_width;
        was_resized = true;
    }

    if height < constraints.min_height {
        height = constraints.min_height;
        was_resized = true;
    }
    if height > constraints.max_height {
        height = constraints.max_height;
        was_resized = true;
    }
    if height > max_available_height {
        height = max_available_height;
        was_resized = true;
    }

    let size = egui::Vec2::new(width, height);

    // Calculate initial position (using center of desired rect as anchor)
    let desired_center = desired_rect.center();
    let mut pos = Pos2::new(
        desired_center.x - width / 2.0,
        desired_center.y - height / 2.0,
    );

    // Clamp position to keep panel within available area
    // Check right edge
    if pos.x + width > available.max.x {
        pos.x = available.max.x - width;
        was_repositioned = true;
    }
    // Check left edge
    if pos.x < available.min.x {
        pos.x = available.min.x;
        was_repositioned = true;
    }
    // Check bottom edge
    if pos.y + height > available.max.y {
        pos.y = available.max.y - height;
        was_repositioned = true;
    }
    // Check top edge
    if pos.y < available.min.y {
        pos.y = available.min.y;
        was_repositioned = true;
    }

    // Log constraint adjustments in debug builds
    #[cfg(debug_assertions)]
    if was_resized || was_repositioned {
        log::debug!(
            "Panel constrained: desired {:?} -> pos {:?}, size {:?} (resized: {}, repositioned: {})",
            desired_rect,
            pos,
            size,
            was_resized,
            was_repositioned
        );
    }

    ConstrainedPanel {
        pos,
        size,
        was_resized,
        was_repositioned,
    }
}

/// Calculate a centered position for a panel within the viewport.
///
/// This is a convenience function for panels that should be centered
/// in the visible area.
///
/// # Arguments
///
/// * `viewport` - The available viewport bounds
/// * `panel_size` - The desired size of the panel
/// * `constraints` - Size constraints and margin settings
///
/// # Returns
///
/// A `ConstrainedPanel` centered in the viewport with proper constraints.
pub fn center_panel_in_viewport(
    viewport: Rect,
    panel_size: egui::Vec2,
    constraints: &PanelConstraints,
) -> ConstrainedPanel {
    let center = viewport.center();
    let desired_rect = Rect::from_center_size(center, panel_size);
    constrain_rect_to_viewport(desired_rect, viewport, constraints)
}

/// Get constraints suitable for the Search in Files panel.
pub fn search_panel_constraints() -> PanelConstraints {
    PanelConstraints {
        min_width: 350.0,  // Minimum to show search field + buttons
        max_width: 700.0,  // Don't get too wide
        min_height: 200.0, // Show at least search field + a few results
        max_height: 600.0, // Don't take up entire screen
        margin: 16.0,      // Keep some padding from edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Viewport Constraint Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_constrain_rect_centered() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let constraints = PanelConstraints::default();

        // Panel that fits easily - should stay centered
        let desired = Rect::from_center_size(viewport.center(), egui::vec2(400.0, 300.0));
        let result = constrain_rect_to_viewport(desired, viewport, &constraints);

        assert!(!result.was_resized);
        assert!(!result.was_repositioned);
        assert_eq!(result.size, egui::vec2(400.0, 300.0));
    }

    #[test]
    fn test_constrain_rect_too_large() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));
        let constraints = PanelConstraints {
            min_width: 200.0,
            max_width: 800.0,
            min_height: 150.0,
            max_height: 600.0,
            margin: 10.0,
        };

        // Panel larger than viewport - should be shrunk
        let desired = Rect::from_center_size(viewport.center(), egui::vec2(600.0, 500.0));
        let result = constrain_rect_to_viewport(desired, viewport, &constraints);

        assert!(result.was_resized);
        // Should fit within viewport minus margins
        assert!(result.size.x <= 380.0); // 400 - 2*10
        assert!(result.size.y <= 280.0); // 300 - 2*10
    }

    #[test]
    fn test_constrain_rect_off_screen_right() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let constraints = PanelConstraints {
            margin: 10.0,
            ..Default::default()
        };

        // Panel positioned off the right edge
        let desired = Rect::from_min_size(Pos2::new(700.0, 200.0), egui::vec2(200.0, 200.0));
        let result = constrain_rect_to_viewport(desired, viewport, &constraints);

        assert!(result.was_repositioned);
        // Should be moved left to fit
        assert!(result.pos.x + result.size.x <= 790.0); // 800 - 10
    }

    #[test]
    fn test_constrain_rect_off_screen_bottom() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let constraints = PanelConstraints {
            margin: 10.0,
            ..Default::default()
        };

        // Panel positioned off the bottom edge
        let desired = Rect::from_min_size(Pos2::new(200.0, 500.0), egui::vec2(200.0, 200.0));
        let result = constrain_rect_to_viewport(desired, viewport, &constraints);

        assert!(result.was_repositioned);
        // Should be moved up to fit
        assert!(result.pos.y + result.size.y <= 590.0); // 600 - 10
    }

    #[test]
    fn test_constrain_rect_respects_min_size() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let constraints = PanelConstraints {
            min_width: 300.0,
            min_height: 200.0,
            ..Default::default()
        };

        // Panel smaller than minimum - should be enlarged
        let desired = Rect::from_center_size(viewport.center(), egui::vec2(100.0, 100.0));
        let result = constrain_rect_to_viewport(desired, viewport, &constraints);

        assert!(result.was_resized);
        assert!(result.size.x >= 300.0);
        assert!(result.size.y >= 200.0);
    }

    #[test]
    fn test_center_panel_in_viewport() {
        let viewport = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let constraints = PanelConstraints::default();
        let panel_size = egui::vec2(400.0, 300.0);

        let result = center_panel_in_viewport(viewport, panel_size, &constraints);

        // Should be centered
        let center = Pos2::new(
            result.pos.x + result.size.x / 2.0,
            result.pos.y + result.size.y / 2.0,
        );
        let viewport_center = viewport.center();
        assert!((center.x - viewport_center.x).abs() < 1.0);
        assert!((center.y - viewport_center.y).abs() < 1.0);
    }

    #[test]
    fn test_search_panel_constraints() {
        let constraints = search_panel_constraints();
        assert!(constraints.min_width > 0.0);
        assert!(constraints.max_width > constraints.min_width);
        assert!(constraints.min_height > 0.0);
        assert!(constraints.max_height > constraints.min_height);
        assert!(constraints.margin > 0.0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Window Resize Tests
    // ─────────────────────────────────────────────────────────────────────────

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

    // ─────────────────────────────────────────────────────────────────────────
    // Title Bar Exclusion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_title_bar_north_edge_left_side() {
        // Large window to test title bar exclusion
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // North edge on the LEFT side (outside button area) - should work
        // Position: x=100 (well outside the button area which is at x > 800 - 280 = 520)
        let direction =
            detect_resize_direction_with_exclusion(rect, Pos2::new(100.0, 2.0), title_bar_height);
        assert_eq!(direction, Some(ResizeDirection::North));
    }

    #[test]
    fn test_title_bar_north_edge_button_area_blocked() {
        // Large window to test title bar exclusion
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // North edge in the BUTTON AREA (right side) - should be blocked
        // Button area starts at x = 800 - 280 = 520
        let direction =
            detect_resize_direction_with_exclusion(rect, Pos2::new(700.0, 2.0), title_bar_height);
        assert_eq!(direction, None);
    }

    #[test]
    fn test_title_bar_northwest_corner() {
        // Large window to test title bar exclusion
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // NorthWest corner (left side, no buttons) - should work
        let direction =
            detect_resize_direction_with_exclusion(rect, Pos2::new(2.0, 2.0), title_bar_height);
        assert_eq!(direction, Some(ResizeDirection::NorthWest));
    }

    #[test]
    fn test_title_bar_northeast_corner_enabled() {
        // Large window to test title bar exclusion
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // NorthEast corner zone (x > 790, y < 10).
        // With TITLE_BAR_BUTTON_RIGHT_MARGIN (12 px) > CORNER_GRAB_SIZE (10 px),
        // the rightmost 10 px strip is always button-free, so NE resize works.
        let direction =
            detect_resize_direction_with_exclusion(rect, Pos2::new(798.0, 2.0), title_bar_height);
        assert_eq!(direction, Some(ResizeDirection::NorthEast));
    }

    #[test]
    fn test_title_bar_south_corners_always_work() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // SouthWest corner - should always work
        assert_eq!(
            detect_resize_direction_with_exclusion(rect, Pos2::new(2.0, 598.0), title_bar_height),
            Some(ResizeDirection::SouthWest)
        );

        // SouthEast corner - should always work
        assert_eq!(
            detect_resize_direction_with_exclusion(rect, Pos2::new(798.0, 598.0), title_bar_height),
            Some(ResizeDirection::SouthEast)
        );
    }

    #[test]
    fn test_title_bar_east_west_edges_work_in_title_bar() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
        let title_bar_height = TITLE_BAR_EXCLUSION_HEIGHT;

        // West edge in title bar area (but not in corner zone) - should work
        // y=20 is outside the corner zone (CORNER_GRAB_SIZE = 10) but inside title bar (35)
        assert_eq!(
            detect_resize_direction_with_exclusion(rect, Pos2::new(2.0, 20.0), title_bar_height),
            Some(ResizeDirection::West)
        );

        // East edge in title bar area (but not in corner zone) - should work
        // y=25 is outside the corner zone (10px) but inside title bar (35px)
        let direction =
            detect_resize_direction_with_exclusion(rect, Pos2::new(798.0, 25.0), title_bar_height);
        assert_eq!(direction, Some(ResizeDirection::East));
    }
}
