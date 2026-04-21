//! Cursor rendering for FerriteEditor.
//!
//! This module handles cursor positioning and rendering, with full support for:
//! - Single-line text (non-wrapped mode with horizontal scrolling)
//! - Word-wrapped text (cursor correctly positions on visual rows)
//! - Cursor blinking (configurable via `cursor_visible` parameter)
//!
//! # Architecture
//!
//! The cursor position is calculated in two stages:
//! 1. **Line Y position**: Calculated in `editor.rs` based on wrap_info
//! 2. **Row-within-line offset**: Calculated here using egui's galley positioning
//!
//! For wrapped text, we use `galley.pos_from_cursor()` which returns the cursor's
//! position relative to the galley origin, automatically accounting for which
//! visual row the cursor is on.
//!
//! # Key Functions
//!
//! - [`render_cursor`] - Main entry point, renders cursor at correct position
//! - [`calculate_wrapped_cursor_position`] - Handles wrapped text cursor positioning
//! - [`get_cursor_position`] - Public API for getting cursor coordinates without rendering

use egui::{Color32, FontId, Pos2, Rect, Vec2};

use super::super::buffer::TextBuffer;
use super::super::cursor::Cursor;
use super::super::view::ViewState;

/// Duration of each cursor blink phase (visible or hidden).
/// Standard is ~500ms, giving a full on/off cycle of ~1 second.
pub const CURSOR_BLINK_INTERVAL_MS: u64 = 500;

/// Renders the cursor at its current position.
///
/// Handles both wrapped and non-wrapped text modes. For wrapped text, correctly
/// positions the cursor on the appropriate visual row within a logical line.
///
/// # Arguments
/// * `painter` - The egui Painter for drawing
/// * `buffer` - The text buffer containing line content
/// * `cursor` - Current cursor position (line, column)
/// * `view` - View state containing wrap settings and scroll offset
/// * `font_id` - Font used for text measurement
/// * `text_start_x` - X coordinate where text area begins (after gutter)
/// * `line_top_y` - Y coordinate of the cursor's logical line top
/// * `wrap_width` - Width at which text wraps (ignored if wrap disabled)
/// * `cursor_color` - Color for the cursor (should match theme)
/// * `cursor_visible` - Whether the cursor should be drawn (for blink effect)
pub fn render_cursor(
    painter: &egui::Painter,
    buffer: &TextBuffer,
    cursor: &Cursor,
    view: &ViewState,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    wrap_width: f32,
    cursor_color: Color32,
    cursor_visible: bool,
) {
    // Skip rendering if cursor is in hidden phase of blink cycle
    if !cursor_visible {
        return;
    }

    let (cursor_x, cursor_y, cursor_height) = if view.is_wrap_enabled() {
        calculate_wrapped_cursor_position(
            painter,
            buffer,
            cursor,
            view,
            font_id,
            text_start_x,
            line_top_y,
            wrap_width,
        )
    } else {
        calculate_unwrapped_cursor_position(
            painter,
            buffer,
            cursor,
            view,
            font_id,
            text_start_x,
            line_top_y,
        )
    };

    // Draw cursor as a thin vertical line
    let cursor_rect =
        Rect::from_min_size(Pos2::new(cursor_x, cursor_y), Vec2::new(2.0, cursor_height));

    painter.rect_filled(cursor_rect, 0.0, cursor_color);
}

/// Calculates cursor position for non-wrapped text.
///
/// In non-wrapped mode, all text stays on a single visual row per logical line.
/// The X position is calculated by measuring text width up to the cursor column,
/// then adjusting for horizontal scroll offset.
///
/// For complex-script lines (Arabic, Bengali, etc.) the HarfRust shaping
/// pipeline is used for accurate advance-width measurement.
fn calculate_unwrapped_cursor_position(
    painter: &egui::Painter,
    buffer: &TextBuffer,
    cursor: &Cursor,
    view: &ViewState,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
) -> (f32, f32, f32) {
    let cursor_x = if cursor.column == 0 {
        text_start_x - view.horizontal_scroll()
    } else if let Some(line_content) = buffer.get_line(cursor.line) {
        let display = line_content.trim_end_matches(['\r', '\n']);

        if crate::fonts::needs_complex_script_fonts(display) {
            let font_bytes = crate::fonts::ttf_bytes_for_font_id_shaping(font_id);
            if let Some(x) = super::super::shaping::shaped_column_to_x(
                display,
                font_bytes,
                font_id.size,
                cursor.column,
            ) {
                text_start_x + x - view.horizontal_scroll()
            } else {
                let chars_before: String = display.chars().take(cursor.column).collect();
                let galley = painter.layout_no_wrap(chars_before, font_id.clone(), Color32::WHITE);
                text_start_x + galley.size().x - view.horizontal_scroll()
            }
        } else {
            let chars_before: String = display.chars().take(cursor.column).collect();
            let galley = painter.layout_no_wrap(chars_before, font_id.clone(), Color32::WHITE);
            text_start_x + galley.size().x - view.horizontal_scroll()
        }
    } else {
        text_start_x - view.horizontal_scroll()
    };

    (cursor_x, line_top_y, view.line_height())
}

/// Calculates cursor position for wrapped text.
///
/// When text wraps, a single logical line spans multiple visual rows. This function
/// uses egui's galley cursor positioning to find exactly which visual row the cursor
/// is on and its X position within that row.
///
/// # How it works
///
/// 1. Create a wrapped galley for the cursor's line content
/// 2. Convert the cursor column to a `CCursor` (character cursor)
/// 3. Use `galley.pos_from_cursor()` to get the cursor's rect relative to galley origin
/// 4. Add line_top_y to the rect's Y to get absolute screen position
///
/// # Returns
/// Tuple of (x, y, height) for cursor rendering:
/// - `x`: Horizontal position in screen coordinates
/// - `y`: Vertical position (accounts for which visual row within the wrapped line)
/// - `height`: Height of the cursor (matches the visual row height)
fn calculate_wrapped_cursor_position(
    painter: &egui::Painter,
    buffer: &TextBuffer,
    cursor: &Cursor,
    view: &ViewState,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    wrap_width: f32,
) -> (f32, f32, f32) {
    let effective_wrap_width = if wrap_width > 0.0 {
        wrap_width
    } else {
        f32::INFINITY
    };
    let base_line_height = view.line_height();

    if let Some(line_content) = buffer.get_line(cursor.line) {
        let display_content = line_content.trim_end_matches(['\r', '\n']);

        // Create a wrapped galley matching how text is rendered
        let galley = painter.layout(
            display_content.to_string(),
            font_id.clone(),
            Color32::WHITE,
            effective_wrap_width,
        );

        // Clamp cursor column to valid range
        let char_count = display_content.chars().count();
        let cursor_col = cursor.column.min(char_count);

        // Use egui's built-in cursor positioning - this is the key to correct
        // wrapped text cursor placement. The galley tracks which visual row
        // each character is on, and pos_from_cursor returns a rect whose
        // min.y accounts for the row offset within the galley.
        let ccursor = egui::text::CCursor::new(cursor_col);
        let galley_cursor = galley.from_ccursor(ccursor);
        let cursor_rect = galley.pos_from_cursor(&galley_cursor);

        // cursor_rect.min is relative to galley origin:
        // - min.x: X offset within the current visual row
        // - min.y: Y offset from galley top (0 for row 0, ~16 for row 1, etc.)
        let cursor_x = text_start_x + cursor_rect.min.x;
        let cursor_y = line_top_y + cursor_rect.min.y;
        let row_height = cursor_rect.height().max(base_line_height);

        (cursor_x, cursor_y, row_height)
    } else {
        (text_start_x, line_top_y, base_line_height)
    }
}

/// Gets cursor position coordinates without rendering.
///
/// This is useful for other components that need to know cursor position,
/// such as selection rendering or IME positioning.
///
/// # Returns
/// Tuple of (x, y, height) in screen coordinates.
#[allow(dead_code)]
pub fn get_cursor_position(
    painter: &egui::Painter,
    buffer: &TextBuffer,
    cursor: &Cursor,
    view: &ViewState,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    wrap_width: f32,
) -> (f32, f32, f32) {
    if view.is_wrap_enabled() {
        calculate_wrapped_cursor_position(
            painter,
            buffer,
            cursor,
            view,
            font_id,
            text_start_x,
            line_top_y,
            wrap_width,
        )
    } else {
        calculate_unwrapped_cursor_position(
            painter,
            buffer,
            cursor,
            view,
            font_id,
            text_start_x,
            line_top_y,
        )
    }
}
