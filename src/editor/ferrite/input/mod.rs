//! Input handling module for FerriteEditor.
//!
//! This module provides keyboard and mouse input handling for the Ferrite editor,
//! including character insertion, cursor movement, text deletion, selection, and scrolling.

pub mod keyboard;
mod mouse;

use egui::Event;

use super::buffer::TextBuffer;
use super::cursor::{Cursor, Selection};
use super::view::ViewState;

/// Result of processing an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputResult {
    /// No change was made.
    NoChange,
    /// The cursor moved but text was not modified.
    CursorMoved,
    /// Text was modified (insert/delete).
    TextChanged,
    /// The view scrolled (mouse wheel, etc.) without cursor movement.
    ViewScrolled,
}

/// Handles keyboard and mouse input for the editor.
///
/// `InputHandler` provides methods for processing input events and
/// translating them into cursor movements, text modifications, or view scrolling.
pub struct InputHandler;

impl InputHandler {
    /// Processes an egui event and updates the buffer/cursor accordingly.
    /// This is the legacy API that works with Cursor directly.
    ///
    /// # Arguments
    /// * `event` - The egui event to process
    /// * `buffer` - The text buffer to modify
    /// * `cursor` - The current cursor position (will be updated)
    /// * `view` - The view state for scrolling operations
    ///
    /// # Returns
    /// An `InputResult` indicating what kind of change occurred.
    #[allow(dead_code)]
    pub fn handle_event(
        event: &Event,
        buffer: &mut TextBuffer,
        cursor: &mut Cursor,
        view: &mut ViewState,
    ) -> InputResult {
        match event {
            // Text input (character insertion)
            Event::Text(text) => {
                if !text.is_empty() && text != "\n" && text != "\r" {
                    keyboard::insert_text(buffer, cursor, text);
                    return InputResult::TextChanged;
                }
                InputResult::NoChange
            }

            // Key press events
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => keyboard::handle_key_press(*key, modifiers, buffer, cursor, view),

            // Mouse wheel scrolling (Shift+scroll = horizontal scroll)
            Event::MouseWheel {
                delta, modifiers, ..
            } => mouse::handle_mouse_wheel(buffer, view, *delta, modifiers),

            _ => InputResult::NoChange,
        }
    }

    /// Processes an egui event and updates the buffer/selection accordingly.
    /// This is the new API that supports text selection.
    ///
    /// # Arguments
    /// * `event` - The egui event to process
    /// * `buffer` - The text buffer to modify
    /// * `selection` - The current selection (will be updated)
    /// * `view` - The view state for scrolling operations
    ///
    /// # Returns
    /// An `InputResult` indicating what kind of change occurred.
    pub fn handle_event_with_selection(
        event: &Event,
        buffer: &mut TextBuffer,
        selection: &mut Selection,
        view: &mut ViewState,
    ) -> InputResult {
        match event {
            // Text input (character insertion)
            Event::Text(text) => {
                if !text.is_empty() && text != "\n" && text != "\r" {
                    // Delete selection first if any
                    if selection.is_range() {
                        let (start, end) = selection.ordered();
                        let start_pos = Self::cursor_to_char_pos(buffer, &start);
                        let end_pos = Self::cursor_to_char_pos(buffer, &end);
                        buffer.remove(start_pos, end_pos - start_pos);
                        *selection = Selection::collapsed(start);
                    }

                    let mut cursor = selection.head;
                    keyboard::insert_text(buffer, &mut cursor, text);
                    *selection = Selection::collapsed(cursor);
                    return InputResult::TextChanged;
                }
                InputResult::NoChange
            }

            // Key press events
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                keyboard::handle_key_press_with_selection(*key, modifiers, buffer, selection, view)
            }

            // Mouse wheel scrolling (Shift+scroll = horizontal scroll)
            Event::MouseWheel {
                delta, modifiers, ..
            } => mouse::handle_mouse_wheel(buffer, view, *delta, modifiers),

            _ => InputResult::NoChange,
        }
    }

    /// Converts cursor (line, column) to character position in buffer.
    ///
    /// Handles out-of-bounds cursors gracefully by clamping to valid ranges.
    /// The result is always clamped to `[0, buffer.len()]` to prevent panics
    /// in downstream rope operations (slice, remove).
    pub fn cursor_to_char_pos(buffer: &TextBuffer, cursor: &Cursor) -> usize {
        // Clamp line to valid range to prevent panics
        let line_count = buffer.line_count();
        let clamped_line = cursor.line.min(line_count.saturating_sub(1));

        // Use try_line_to_char for safety, fallback to buffer end
        let line_start = buffer
            .try_line_to_char(clamped_line)
            .unwrap_or(buffer.len());

        // Clamp final position to buffer length to prevent rope slice/remove panics.
        // cursor.column could exceed line length after deletions or with stale cursors.
        (line_start + cursor.column).min(buffer.len())
    }

    /// Returns the length of a line (excluding newline characters).
    /// Handles both LF (\n) and CRLF (\r\n) line endings.
    pub(crate) fn line_length(buffer: &TextBuffer, line: usize) -> usize {
        buffer
            .get_line(line)
            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
            .unwrap_or(0)
    }

    /// Returns the number of visible lines in the viewport.
    pub(crate) fn visible_lines(view: &ViewState) -> usize {
        if view.line_height() > 0.0 {
            (view.viewport_height() / view.line_height()).floor() as usize
        } else {
            10 // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;

    fn create_test_buffer(content: &str) -> TextBuffer {
        TextBuffer::from_string(content)
    }

    #[test]
    fn test_insert_character() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        let result = InputHandler::handle_event(
            &Event::Text("!".to_string()),
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::TextChanged);
        assert_eq!(buffer.to_string(), "Hello!");
        assert_eq!(cursor.column, 6);
    }

    #[test]
    fn test_insert_at_middle() {
        let mut buffer = create_test_buffer("Helo");
        let mut cursor = Cursor::new(0, 2);
        let mut view = ViewState::new();

        InputHandler::handle_event(
            &Event::Text("l".to_string()),
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(buffer.to_string(), "Hello");
        assert_eq!(cursor.column, 3);
    }

    #[test]
    fn test_no_change_on_key_release() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        let result = InputHandler::handle_event(
            &Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: false, // Key release
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::NoChange);
    }

    #[test]
    fn test_unicode_input() {
        let mut buffer = create_test_buffer("");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        // Insert Japanese text
        InputHandler::handle_event(
            &Event::Text("こんにちは".to_string()),
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(buffer.to_string(), "こんにちは");
        assert_eq!(cursor.column, 5); // 5 Japanese characters
    }

    #[test]
    fn test_emoji_input() {
        let mut buffer = create_test_buffer("Hello ");
        let mut cursor = Cursor::new(0, 6);
        let mut view = ViewState::new();

        InputHandler::handle_event(
            &Event::Text("🌍".to_string()),
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(buffer.to_string(), "Hello 🌍");
        assert_eq!(cursor.column, 7);
    }
}
