//! Keyboard input handling for FerriteEditor.
//!
//! This module handles keyboard events including cursor movement,
//! text insertion, deletion operations, and text selection.

use egui::{Key, Modifiers};

use super::super::buffer::TextBuffer;
use super::super::cursor::{Cursor, Selection};
use super::super::grapheme;
use super::super::view::ViewState;
use super::{InputHandler, InputResult};

/// Handles a key press event.
pub(crate) fn handle_key_press(
    key: Key,
    modifiers: &Modifiers,
    buffer: &mut TextBuffer,
    cursor: &mut Cursor,
    view: &mut ViewState,
) -> InputResult {
    match key {
        // Enter key - insert newline
        Key::Enter => {
            insert_text(buffer, cursor, "\n");
            InputResult::TextChanged
        }

        // Backspace - delete character before cursor
        Key::Backspace => {
            if delete_backward(buffer, cursor) {
                InputResult::TextChanged
            } else {
                InputResult::NoChange
            }
        }

        // Delete - delete character after cursor
        Key::Delete => {
            if delete_forward(buffer, cursor) {
                InputResult::TextChanged
            } else {
                InputResult::NoChange
            }
        }

        // Arrow keys - cursor movement
        Key::ArrowLeft => {
            move_cursor_left(buffer, cursor, modifiers.ctrl || modifiers.command);
            InputResult::CursorMoved
        }
        Key::ArrowRight => {
            move_cursor_right(buffer, cursor, modifiers.ctrl || modifiers.command);
            InputResult::CursorMoved
        }
        Key::ArrowUp => {
            // Use visual row movement when wrap is enabled
            if view.is_wrap_enabled() {
                move_cursor_up_visual(buffer, cursor, view);
            } else {
                move_cursor_up(buffer, cursor);
            }
            InputResult::CursorMoved
        }
        Key::ArrowDown => {
            // Use visual row movement when wrap is enabled
            if view.is_wrap_enabled() {
                move_cursor_down_visual(buffer, cursor, view);
            } else {
                move_cursor_down(buffer, cursor);
            }
            InputResult::CursorMoved
        }

        // Home/End - line start/end or document start/end with Ctrl
        Key::Home => {
            if modifiers.ctrl || modifiers.command {
                // Ctrl+Home: go to document start
                cursor.line = 0;
                cursor.column = 0;
            } else {
                // Home: go to line start
                cursor.column = 0;
            }
            InputResult::CursorMoved
        }
        Key::End => {
            if modifiers.ctrl || modifiers.command {
                // Ctrl+End: go to document end
                let last_line = buffer.line_count().saturating_sub(1);
                cursor.line = last_line;
                cursor.column = InputHandler::line_length(buffer, last_line);
            } else {
                // End: go to line end
                cursor.column = InputHandler::line_length(buffer, cursor.line);
            }
            InputResult::CursorMoved
        }

        // Page Up/Down - scroll by viewport
        Key::PageUp => {
            page_up(buffer, cursor, view);
            InputResult::CursorMoved
        }
        Key::PageDown => {
            page_down(buffer, cursor, view);
            InputResult::CursorMoved
        }

        _ => InputResult::NoChange,
    }
}

/// Handles a key press event with selection support.
pub(crate) fn handle_key_press_with_selection(
    key: Key,
    modifiers: &Modifiers,
    buffer: &mut TextBuffer,
    selection: &mut Selection,
    view: &mut ViewState,
) -> InputResult {
    let shift = modifiers.shift;
    let ctrl_or_cmd = modifiers.ctrl || modifiers.command;

    match key {
        // Enter key - insert newline (delete selection first)
        Key::Enter => {
            // Delete selection first if any
            if selection.is_range() {
                let (start, end) = selection.ordered();
                let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
                let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
                buffer.remove(start_pos, end_pos - start_pos);
                *selection = Selection::collapsed(start);
            }

            let mut cursor = selection.head;
            insert_text(buffer, &mut cursor, "\n");
            *selection = Selection::collapsed(cursor);
            InputResult::TextChanged
        }

        // Backspace - delete selection or character before cursor
        Key::Backspace => {
            if selection.is_range() {
                // Delete selection
                let (start, end) = selection.ordered();
                let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
                let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
                buffer.remove(start_pos, end_pos - start_pos);
                *selection = Selection::collapsed(start);
                InputResult::TextChanged
            } else {
                let mut cursor = selection.head;
                if delete_backward(buffer, &mut cursor) {
                    *selection = Selection::collapsed(cursor);
                    InputResult::TextChanged
                } else {
                    InputResult::NoChange
                }
            }
        }

        // Delete - delete selection or character after cursor
        Key::Delete => {
            if selection.is_range() {
                // Delete selection
                let (start, end) = selection.ordered();
                let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
                let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
                buffer.remove(start_pos, end_pos - start_pos);
                *selection = Selection::collapsed(start);
                InputResult::TextChanged
            } else {
                let mut cursor = selection.head;
                if delete_forward(buffer, &mut cursor) {
                    *selection = Selection::collapsed(cursor);
                    InputResult::TextChanged
                } else {
                    InputResult::NoChange
                }
            }
        }

        // Arrow keys - cursor movement with optional selection extension
        Key::ArrowLeft => {
            let mut new_cursor = selection.head;
            move_cursor_left(buffer, &mut new_cursor, ctrl_or_cmd);

            if shift {
                // Extend selection
                *selection = selection.with_head(new_cursor);
            } else if selection.is_range() {
                // Collapse to start of selection
                *selection = Selection::collapsed(selection.start_pos());
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }
        Key::ArrowRight => {
            let mut new_cursor = selection.head;
            move_cursor_right(buffer, &mut new_cursor, ctrl_or_cmd);

            if shift {
                // Extend selection
                *selection = selection.with_head(new_cursor);
            } else if selection.is_range() {
                // Collapse to end of selection
                *selection = Selection::collapsed(selection.end_pos());
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }
        Key::ArrowUp => {
            let mut new_cursor = selection.head;
            if view.is_wrap_enabled() {
                move_cursor_up_visual(buffer, &mut new_cursor, view);
            } else {
                move_cursor_up(buffer, &mut new_cursor);
            }

            if shift {
                *selection = selection.with_head(new_cursor);
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }
        Key::ArrowDown => {
            let mut new_cursor = selection.head;
            if view.is_wrap_enabled() {
                move_cursor_down_visual(buffer, &mut new_cursor, view);
            } else {
                move_cursor_down(buffer, &mut new_cursor);
            }

            if shift {
                *selection = selection.with_head(new_cursor);
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }

        // Home/End - line start/end or document start/end
        Key::Home => {
            let new_cursor = if ctrl_or_cmd {
                // Ctrl+Home: go to document start
                Cursor::new(0, 0)
            } else {
                // Home: go to line start
                Cursor::new(selection.head.line, 0)
            };

            if shift {
                *selection = selection.with_head(new_cursor);
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }
        Key::End => {
            let new_cursor = if ctrl_or_cmd {
                // Ctrl+End: go to document end
                let last_line = buffer.line_count().saturating_sub(1);
                let last_col = InputHandler::line_length(buffer, last_line);
                Cursor::new(last_line, last_col)
            } else {
                // End: go to line end
                let line_len = InputHandler::line_length(buffer, selection.head.line);
                Cursor::new(selection.head.line, line_len)
            };

            if shift {
                *selection = selection.with_head(new_cursor);
            } else {
                *selection = Selection::collapsed(new_cursor);
            }
            InputResult::CursorMoved
        }

        // Page Up/Down - scroll by viewport
        Key::PageUp => {
            let mut cursor = selection.head;
            page_up(buffer, &mut cursor, view);

            if shift {
                *selection = selection.with_head(cursor);
            } else {
                *selection = Selection::collapsed(cursor);
            }
            InputResult::CursorMoved
        }
        Key::PageDown => {
            let mut cursor = selection.head;
            page_down(buffer, &mut cursor, view);

            if shift {
                *selection = selection.with_head(cursor);
            } else {
                *selection = Selection::collapsed(cursor);
            }
            InputResult::CursorMoved
        }

        _ => InputResult::NoChange,
    }
}

/// Inserts text at the current cursor position.
pub(crate) fn insert_text(buffer: &mut TextBuffer, cursor: &mut Cursor, text: &str) {
    let char_pos = InputHandler::cursor_to_char_pos(buffer, cursor);
    buffer.insert(char_pos, text);

    // Update cursor position after insert
    // Count newlines and characters to determine new position
    let mut new_line = cursor.line;
    let mut new_col = cursor.column;

    for ch in text.chars() {
        if ch == '\n' {
            new_line += 1;
            new_col = 0;
        } else {
            new_col += 1;
        }
    }

    cursor.line = new_line;
    cursor.column = new_col;
}

/// Deletes the grapheme cluster before the cursor (backspace).
/// Returns true if anything was deleted.
fn delete_backward(buffer: &mut TextBuffer, cursor: &mut Cursor) -> bool {
    if cursor.column > 0 {
        let prev_col = if let Some(line) = buffer.get_line(cursor.line) {
            let stripped = grapheme::line_text_stripped(&line);
            grapheme::prev_grapheme_boundary(stripped, cursor.column)
        } else {
            cursor.column.saturating_sub(1)
        };
        let chars_to_delete = cursor.column - prev_col;
        let char_pos = InputHandler::cursor_to_char_pos(buffer, cursor);
        buffer.remove(char_pos - chars_to_delete, chars_to_delete);
        cursor.column = prev_col;
        true
    } else if cursor.line > 0 {
        // At start of line — join with previous line (remove newline)
        let prev_line_len = InputHandler::line_length(buffer, cursor.line - 1);
        let char_pos = InputHandler::cursor_to_char_pos(buffer, cursor);
        buffer.remove(char_pos - 1, 1);
        cursor.line -= 1;
        cursor.column = prev_line_len;
        true
    } else {
        false
    }
}

/// Deletes the grapheme cluster after the cursor (delete key).
/// Returns true if anything was deleted.
fn delete_forward(buffer: &mut TextBuffer, cursor: &mut Cursor) -> bool {
    let char_pos = InputHandler::cursor_to_char_pos(buffer, cursor);
    if char_pos >= buffer.len() {
        return false;
    }

    let line_len = InputHandler::line_length(buffer, cursor.line);
    if cursor.column < line_len {
        let chars_to_delete = if let Some(line) = buffer.get_line(cursor.line) {
            let stripped = grapheme::line_text_stripped(&line);
            grapheme::next_grapheme_boundary(stripped, cursor.column) - cursor.column
        } else {
            1
        };
        buffer.remove(char_pos, chars_to_delete);
    } else {
        // At end of line — remove newline to join lines
        buffer.remove(char_pos, 1);
    }
    true
}

/// Moves cursor left by one grapheme cluster (or word with Ctrl).
fn move_cursor_left(buffer: &TextBuffer, cursor: &mut Cursor, word_mode: bool) {
    if word_mode {
        move_cursor_word_left(buffer, cursor);
    } else if cursor.column > 0 {
        if let Some(line) = buffer.get_line(cursor.line) {
            let stripped = grapheme::line_text_stripped(&line);
            cursor.column = grapheme::prev_grapheme_boundary(stripped, cursor.column);
        } else {
            cursor.column = cursor.column.saturating_sub(1);
        }
    } else if cursor.line > 0 {
        cursor.line -= 1;
        cursor.column = InputHandler::line_length(buffer, cursor.line);
    }
}

/// Moves cursor right by one grapheme cluster (or word with Ctrl).
fn move_cursor_right(buffer: &TextBuffer, cursor: &mut Cursor, word_mode: bool) {
    if word_mode {
        move_cursor_word_right(buffer, cursor);
    } else {
        let line_len = InputHandler::line_length(buffer, cursor.line);
        if cursor.column < line_len {
            if let Some(line) = buffer.get_line(cursor.line) {
                let stripped = grapheme::line_text_stripped(&line);
                cursor.column = grapheme::next_grapheme_boundary(stripped, cursor.column);
            } else {
                cursor.column += 1;
            }
        } else if cursor.line < buffer.line_count().saturating_sub(1) {
            cursor.line += 1;
            cursor.column = 0;
        }
    }
}

/// Moves cursor up by one line (or visual row if wrap is enabled).
fn move_cursor_up(buffer: &TextBuffer, cursor: &mut Cursor) {
    if cursor.line > 0 {
        cursor.line -= 1;
        // Clamp column to line length
        let line_len = InputHandler::line_length(buffer, cursor.line);
        cursor.column = cursor.column.min(line_len);
    }
}

/// Moves cursor down by one line (or visual row if wrap is enabled).
fn move_cursor_down(buffer: &TextBuffer, cursor: &mut Cursor) {
    let last_line = buffer.line_count().saturating_sub(1);
    if cursor.line < last_line {
        cursor.line += 1;
        // Clamp column to line length
        let line_len = InputHandler::line_length(buffer, cursor.line);
        cursor.column = cursor.column.min(line_len);
    }
}

/// Moves cursor up by one visual row, respecting word wrap.
///
/// When word wrap is enabled, up/down should move by visual row rather than
/// logical line. This function estimates the visual row position and moves
/// accordingly.
///
/// # Arguments
/// * `buffer` - The text buffer
/// * `cursor` - The cursor to move
/// * `view` - The view state containing wrap information
pub fn move_cursor_up_visual(buffer: &TextBuffer, cursor: &mut Cursor, view: &ViewState) {
    if !view.is_wrap_enabled() {
        // Fall back to logical line movement
        move_cursor_up(buffer, cursor);
        return;
    }

    let visual_rows = view.get_visual_rows(cursor.line);
    if visual_rows <= 1 {
        // Only one visual row in this line, move to previous logical line
        move_cursor_up(buffer, cursor);
    } else {
        // Estimate which visual row we're on based on column position
        // This is a simplified approach - for more accuracy, we'd need the actual galley
        let line_len = InputHandler::line_length(buffer, cursor.line);
        if line_len == 0 {
            move_cursor_up(buffer, cursor);
            return;
        }

        // Estimate characters per visual row
        let chars_per_row = (line_len + visual_rows - 1) / visual_rows;
        let current_visual_row = cursor.column / chars_per_row.max(1);

        if current_visual_row > 0 {
            // Move up within the same logical line
            let new_col =
                (current_visual_row - 1) * chars_per_row + (cursor.column % chars_per_row.max(1));
            cursor.column = new_col.min(line_len);
        } else {
            // At top visual row, move to previous logical line
            if cursor.line > 0 {
                cursor.line -= 1;
                let prev_line_len = InputHandler::line_length(buffer, cursor.line);
                let prev_visual_rows = view.get_visual_rows(cursor.line);

                if prev_visual_rows > 1 {
                    // Move to the last visual row of previous line
                    let chars_per_row = (prev_line_len + prev_visual_rows - 1) / prev_visual_rows;
                    let last_row_start = (prev_visual_rows - 1) * chars_per_row;
                    cursor.column = (last_row_start + (cursor.column % chars_per_row.max(1)))
                        .min(prev_line_len);
                } else {
                    cursor.column = cursor.column.min(prev_line_len);
                }
            }
        }
    }
}

/// Moves cursor down by one visual row, respecting word wrap.
///
/// When word wrap is enabled, up/down should move by visual row rather than
/// logical line.
///
/// # Arguments
/// * `buffer` - The text buffer
/// * `cursor` - The cursor to move
/// * `view` - The view state containing wrap information
pub fn move_cursor_down_visual(buffer: &TextBuffer, cursor: &mut Cursor, view: &ViewState) {
    if !view.is_wrap_enabled() {
        // Fall back to logical line movement
        move_cursor_down(buffer, cursor);
        return;
    }

    let visual_rows = view.get_visual_rows(cursor.line);
    let line_len = InputHandler::line_length(buffer, cursor.line);

    if visual_rows <= 1 || line_len == 0 {
        // Only one visual row in this line, move to next logical line
        move_cursor_down(buffer, cursor);
    } else {
        // Estimate which visual row we're on
        let chars_per_row = (line_len + visual_rows - 1) / visual_rows;
        let current_visual_row = cursor.column / chars_per_row.max(1);

        if current_visual_row < visual_rows - 1 {
            // Move down within the same logical line
            let new_col =
                (current_visual_row + 1) * chars_per_row + (cursor.column % chars_per_row.max(1));
            cursor.column = new_col.min(line_len);
        } else {
            // At bottom visual row, move to next logical line
            let last_line = buffer.line_count().saturating_sub(1);
            if cursor.line < last_line {
                cursor.line += 1;
                let next_line_len = InputHandler::line_length(buffer, cursor.line);
                // Try to maintain column position within the first row
                cursor.column = (cursor.column % chars_per_row.max(1)).min(next_line_len);
            }
        }
    }
}

/// Moves cursor left by one word.
fn move_cursor_word_left(buffer: &TextBuffer, cursor: &mut Cursor) {
    // If at start of line, move to end of previous line
    if cursor.column == 0 {
        if cursor.line > 0 {
            cursor.line -= 1;
            cursor.column = InputHandler::line_length(buffer, cursor.line);
        }
        return;
    }

    // Get line content
    if let Some(line) = buffer.get_line(cursor.line) {
        let line = line.trim_end_matches(['\r', '\n']);
        let chars: Vec<char> = line.chars().collect();

        // Skip whitespace going backward
        let mut col = cursor.column.min(chars.len());
        while col > 0 && chars.get(col - 1).map_or(false, |c| c.is_whitespace()) {
            col -= 1;
        }

        // Skip non-whitespace going backward (the word)
        while col > 0 && chars.get(col - 1).map_or(false, |c| !c.is_whitespace()) {
            col -= 1;
        }

        cursor.column = col;
    }
}

/// Moves cursor right by one word.
fn move_cursor_word_right(buffer: &TextBuffer, cursor: &mut Cursor) {
    let line_len = InputHandler::line_length(buffer, cursor.line);

    // If at end of line, move to start of next line
    if cursor.column >= line_len {
        let last_line = buffer.line_count().saturating_sub(1);
        if cursor.line < last_line {
            cursor.line += 1;
            cursor.column = 0;
        }
        return;
    }

    // Get line content
    if let Some(line) = buffer.get_line(cursor.line) {
        let line = line.trim_end_matches(['\r', '\n']);
        let chars: Vec<char> = line.chars().collect();

        // Skip non-whitespace going forward (the word)
        let mut col = cursor.column;
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }

        // Skip whitespace going forward
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        cursor.column = col;
    }
}

/// Moves cursor up by one page.
fn page_up(buffer: &TextBuffer, cursor: &mut Cursor, view: &mut ViewState) {
    let page_lines = InputHandler::visible_lines(view);
    let new_line = cursor.line.saturating_sub(page_lines);
    cursor.line = new_line;

    // Clamp column to line length
    let line_len = InputHandler::line_length(buffer, cursor.line);
    cursor.column = cursor.column.min(line_len);

    // Scroll view
    let total_lines = buffer.line_count();
    view.scroll_by(-(page_lines as f32 * view.line_height()), total_lines);
}

/// Moves cursor down by one page.
fn page_down(buffer: &TextBuffer, cursor: &mut Cursor, view: &mut ViewState) {
    let page_lines = InputHandler::visible_lines(view);
    let last_line = buffer.line_count().saturating_sub(1);
    let new_line = (cursor.line + page_lines).min(last_line);
    cursor.line = new_line;

    // Clamp column to line length
    let line_len = InputHandler::line_length(buffer, cursor.line);
    cursor.column = cursor.column.min(line_len);

    // Scroll view
    let total_lines = buffer.line_count();
    view.scroll_by(page_lines as f32 * view.line_height(), total_lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_buffer(content: &str) -> TextBuffer {
        TextBuffer::from_string(content)
    }

    #[test]
    fn test_insert_newline() {
        let mut buffer = create_test_buffer("HelloWorld");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        let result = handle_key_press(
            Key::Enter,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::TextChanged);
        assert_eq!(buffer.to_string(), "Hello\nWorld");
        assert_eq!(cursor.line, 1);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_backspace_within_line() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        let result = handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::TextChanged);
        assert_eq!(buffer.to_string(), "Hell");
        assert_eq!(cursor.column, 4);
    }

    #[test]
    fn test_backspace_joins_lines() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(1, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(buffer.to_string(), "HelloWorld");
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 5);
    }

    #[test]
    fn test_backspace_at_document_start() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        let result = handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::NoChange);
        assert_eq!(buffer.to_string(), "Hello");
    }

    #[test]
    fn test_delete_forward() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        let result = handle_key_press(
            Key::Delete,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::TextChanged);
        assert_eq!(buffer.to_string(), "ello");
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_delete_joins_lines() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Delete,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(buffer.to_string(), "HelloWorld");
    }

    #[test]
    fn test_arrow_left() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 3);
        let mut view = ViewState::new();

        let result = handle_key_press(
            Key::ArrowLeft,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(result, InputResult::CursorMoved);
        assert_eq!(cursor.column, 2);
    }

    #[test]
    fn test_arrow_left_wraps_to_previous_line() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(1, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowLeft,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 5);
    }

    #[test]
    fn test_arrow_right() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 2);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.column, 3);
    }

    #[test]
    fn test_arrow_right_wraps_to_next_line() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 1);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_arrow_up() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(1, 3);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowUp,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 3);
    }

    #[test]
    fn test_arrow_up_clamps_column() {
        let mut buffer = create_test_buffer("Hi\nWorld");
        let mut cursor = Cursor::new(1, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowUp,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 2); // "Hi" only has 2 chars
    }

    #[test]
    fn test_arrow_down() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(0, 3);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowDown,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 1);
        assert_eq!(cursor.column, 3);
    }

    #[test]
    fn test_home() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 3);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Home,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_ctrl_home() {
        let mut buffer = create_test_buffer("Hello\nWorld\nTest");
        let mut cursor = Cursor::new(2, 3);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Home,
            &Modifiers::CTRL,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_end() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 2);
        let mut view = ViewState::new();

        handle_key_press(
            Key::End,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.column, 5);
    }

    #[test]
    fn test_ctrl_end() {
        let mut buffer = create_test_buffer("Hello\nWorld\nTest");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::End,
            &Modifiers::CTRL,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        assert_eq!(cursor.line, 2);
        assert_eq!(cursor.column, 4); // "Test" has 4 chars
    }

    #[test]
    fn test_ctrl_arrow_left_word() {
        let mut buffer = create_test_buffer("Hello World Test");
        let mut cursor = Cursor::new(0, 11); // At "T" of "Test"
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowLeft,
            &Modifiers::CTRL,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        // Should move to start of "World"
        assert_eq!(cursor.column, 6);
    }

    #[test]
    fn test_ctrl_arrow_right_word() {
        let mut buffer = create_test_buffer("Hello World Test");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::CTRL,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        // Should move past "Hello " to start of "World"
        assert_eq!(cursor.column, 6);
    }

    #[test]
    fn test_page_down() {
        let content: String = (0..50).map(|i| format!("Line {i}\n")).collect();
        let mut buffer = create_test_buffer(&content);
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        handle_key_press(
            Key::PageDown,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        // Should move down by ~10 lines
        assert_eq!(cursor.line, 10);
    }

    #[test]
    fn test_page_up() {
        let content: String = (0..50).map(|i| format!("Line {i}\n")).collect();
        let mut buffer = create_test_buffer(&content);
        let mut cursor = Cursor::new(30, 0);
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines
        view.scroll_to_line(30);

        handle_key_press(
            Key::PageUp,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );

        // Should move up by ~10 lines
        assert_eq!(cursor.line, 20);
    }

    // ── Grapheme-cluster-aware tests ───────────────────────────────────

    #[test]
    fn test_arrow_right_emoji_zwj_family() {
        // 👨‍👩‍👧 = 5 chars, 1 grapheme cluster
        let mut buffer = create_test_buffer("👨\u{200D}👩\u{200D}👧");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(cursor.column, 5, "should skip entire ZWJ family emoji");
    }

    #[test]
    fn test_arrow_left_emoji_zwj_family() {
        let mut buffer = create_test_buffer("👨\u{200D}👩\u{200D}👧");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowLeft,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(
            cursor.column, 0,
            "should jump back over entire ZWJ family emoji"
        );
    }

    #[test]
    fn test_backspace_deletes_entire_emoji_cluster() {
        let mut buffer = create_test_buffer("a👨\u{200D}👩\u{200D}👧b");
        let mut cursor = Cursor::new(0, 6); // right after the emoji
        let mut view = ViewState::new();

        handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(buffer.to_string(), "ab");
        assert_eq!(cursor.column, 1);
    }

    #[test]
    fn test_delete_removes_entire_emoji_cluster() {
        let mut buffer = create_test_buffer("a👨\u{200D}👩\u{200D}👧b");
        let mut cursor = Cursor::new(0, 1); // on the emoji
        let mut view = ViewState::new();

        handle_key_press(
            Key::Delete,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(buffer.to_string(), "ab");
        assert_eq!(cursor.column, 1);
    }

    #[test]
    fn test_bengali_conjunct_single_arrow_press() {
        // ক্ষ = ক + ্ + ষ  (3 chars, 1 grapheme cluster)
        let mut buffer = create_test_buffer("ক্ষ");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(
            cursor.column, 3,
            "single right-arrow should skip entire Bengali conjunct"
        );
    }

    #[test]
    fn test_bengali_conjunct_backspace() {
        let mut buffer = create_test_buffer("ক্ষ");
        let mut cursor = Cursor::new(0, 3);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(buffer.to_string(), "");
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_korean_jamo_single_arrow_press() {
        // Conjoining Hangul jamo: ᄒ + ᅡ + ᆫ = 3 chars, 1 grapheme
        let mut buffer = create_test_buffer("\u{1112}\u{1161}\u{11AB}");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(cursor.column, 3, "should skip entire jamo syllable block");
    }

    #[test]
    fn test_combining_diacritic_arrow_right() {
        // e + combining acute = 2 chars, 1 grapheme
        let mut buffer = create_test_buffer("e\u{0301}x");
        let mut cursor = Cursor::new(0, 0);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(
            cursor.column, 2,
            "should skip base + combining mark as one grapheme"
        );
    }

    #[test]
    fn test_latin_unchanged_regression() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 2);
        let mut view = ViewState::new();

        handle_key_press(
            Key::ArrowRight,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(cursor.column, 3, "Latin should still move by 1");

        handle_key_press(
            Key::ArrowLeft,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(cursor.column, 2, "Latin left should still move by 1");
    }

    #[test]
    fn test_backspace_latin_unchanged_regression() {
        let mut buffer = create_test_buffer("Hello");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Backspace,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(buffer.to_string(), "Hell");
        assert_eq!(cursor.column, 4);
    }

    #[test]
    fn test_delete_at_end_of_line_joins() {
        let mut buffer = create_test_buffer("Hello\nWorld");
        let mut cursor = Cursor::new(0, 5);
        let mut view = ViewState::new();

        handle_key_press(
            Key::Delete,
            &Modifiers::NONE,
            &mut buffer,
            &mut cursor,
            &mut view,
        );
        assert_eq!(buffer.to_string(), "HelloWorld");
    }
}
