//! Mouse input handling for FerriteEditor.
//!
//! This module handles mouse events including wheel scrolling.

use super::super::buffer::TextBuffer;
use super::super::view::ViewState;
use super::InputResult;

/// Scroll speed: lines per wheel notch for vertical scrolling.
const SCROLL_LINES: f32 = 3.0;

/// Scroll speed: pixels per wheel notch for horizontal scrolling.
const HORIZONTAL_SCROLL_PIXELS: f32 = 40.0;

/// Handles mouse wheel scrolling.
///
/// When Shift is held, scrolls horizontally instead of vertically.
/// This allows navigation of long lines when word wrap is disabled.
pub(crate) fn handle_mouse_wheel(
    buffer: &TextBuffer,
    view: &mut ViewState,
    delta: egui::Vec2,
    modifiers: &egui::Modifiers,
) -> InputResult {
    // Ctrl+scroll = zoom (handled by the editor widget via egui::gui_zoom)
    if modifiers.command {
        return InputResult::NoChange;
    }

    // Shift+scroll = horizontal scroll
    if modifiers.shift {
        // Use delta.y for horizontal scrolling (since most mice have vertical wheel only)
        // Also support delta.x if the mouse has horizontal scroll capability
        let h_scroll_delta = if delta.x.abs() > delta.y.abs() {
            -delta.x * HORIZONTAL_SCROLL_PIXELS
        } else {
            -delta.y * HORIZONTAL_SCROLL_PIXELS
        };

        if h_scroll_delta.abs() > 0.01 {
            let current = view.horizontal_scroll();
            // Clamp to 0 (can't scroll past start, max is handled by rendering)
            view.set_horizontal_scroll((current + h_scroll_delta).max(0.0));
            return InputResult::ViewScrolled;
        }
        return InputResult::NoChange;
    }

    // Normal vertical scrolling
    // delta.y is positive when scrolling up (toward top of document)
    // We want to scroll in the opposite direction (scroll up = view moves down in document)
    let line_height = view.line_height();
    let total_lines = buffer.line_count();

    // delta.y > 0 means scroll wheel up = scroll content down (show earlier content)
    // delta.y < 0 means scroll wheel down = scroll content up (show later content)
    let scroll_amount = -delta.y * SCROLL_LINES * line_height;

    if scroll_amount.abs() > 0.01 {
        view.scroll_by(scroll_amount, total_lines);
        InputResult::ViewScrolled
    } else {
        InputResult::NoChange
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_modifiers() -> egui::Modifiers {
        egui::Modifiers::NONE
    }

    fn shift_modifier() -> egui::Modifiers {
        egui::Modifiers::SHIFT
    }

    #[test]
    fn test_mouse_wheel_scroll_down() {
        // Need enough lines to allow scrolling (more than viewport can show)
        let content: String = (0..20).map(|i| format!("Line {i}\n")).collect();
        let buffer = TextBuffer::from_string(&content);
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0); // 5 visible lines, 20 total = can scroll

        // Scroll down (delta.y < 0)
        let result = handle_mouse_wheel(
            &buffer,
            &mut view,
            egui::Vec2::new(0.0, -1.0),
            &no_modifiers(),
        );

        assert_eq!(result, InputResult::ViewScrolled);
        // View should have scrolled down
        assert!(view.first_visible_line() > 0 || view.scroll_offset_y() > 0.0);
    }

    #[test]
    fn test_mouse_wheel_scroll_up() {
        // Need enough lines to allow scrolling
        let content: String = (0..20).map(|i| format!("Line {i}\n")).collect();
        let buffer = TextBuffer::from_string(&content);
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0); // 5 visible lines, 20 total
        view.scroll_to_line(5); // Start scrolled down

        // Scroll up (delta.y > 0)
        let result = handle_mouse_wheel(
            &buffer,
            &mut view,
            egui::Vec2::new(0.0, 1.0),
            &no_modifiers(),
        );

        assert_eq!(result, InputResult::ViewScrolled);
        // View should have scrolled up
        assert!(view.first_visible_line() < 5);
    }

    #[test]
    fn test_mouse_wheel_no_change_on_zero_delta() {
        let buffer = TextBuffer::from_string("Line 1\nLine 2\nLine 3");
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0);

        // Zero delta should cause no change
        let result = handle_mouse_wheel(
            &buffer,
            &mut view,
            egui::Vec2::new(0.0, 0.0),
            &no_modifiers(),
        );

        assert_eq!(result, InputResult::NoChange);
    }

    #[test]
    fn test_shift_scroll_horizontal() {
        let buffer = TextBuffer::from_string("Line 1\nLine 2\nLine 3");
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0);

        // Shift+scroll should scroll horizontally
        let result = handle_mouse_wheel(
            &buffer,
            &mut view,
            egui::Vec2::new(0.0, -1.0),
            &shift_modifier(),
        );

        assert_eq!(result, InputResult::ViewScrolled);
        // Horizontal scroll should have increased
        assert!(view.horizontal_scroll() > 0.0);
    }

    #[test]
    fn test_shift_scroll_horizontal_clamp_left() {
        let buffer = TextBuffer::from_string("Line 1\nLine 2\nLine 3");
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0);
        view.set_horizontal_scroll(50.0);

        // Shift+scroll up (positive delta.y) should scroll left (reduce horizontal scroll)
        let result = handle_mouse_wheel(
            &buffer,
            &mut view,
            egui::Vec2::new(0.0, 5.0),
            &shift_modifier(),
        );

        assert_eq!(result, InputResult::ViewScrolled);
        // Horizontal scroll should be clamped at 0
        assert_eq!(view.horizontal_scroll(), 0.0);
    }
}
