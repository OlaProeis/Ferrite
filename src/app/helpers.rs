//! Helper utility functions for the Ferrite application module.
//!
//! This module contains standalone functions for text position conversion
//! and other utilities used across the app module.

use crate::markdown::FormattingState;

/// Get the display name for the primary modifier key.
/// Returns "Cmd" on macOS, "Ctrl" on Windows/Linux.
///
/// This is used for displaying keyboard shortcuts in the UI.
/// The actual keyboard handling uses `egui::Modifiers::command` which
/// automatically maps to the correct key per platform.
pub fn modifier_symbol() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// Convert a character index to line and column (0-indexed).
pub(crate) fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut current_index = 0;

    for ch in text.chars() {
        if current_index >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_index += 1;
    }

    (line, col)
}

/// Convert line and column (0-indexed) to a character index.
pub(crate) fn line_col_to_char_index(text: &str, target_line: usize, target_col: usize) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;
    let mut char_index = 0;

    for ch in text.chars() {
        if current_line == target_line && current_col == target_col {
            return char_index;
        }
        if ch == '\n' {
            if current_line == target_line {
                // Target column is beyond line end, return end of line
                return char_index;
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
        char_index += 1;
    }

    // Return end of text if target position is beyond text
    char_index
}

/// Find the byte range of a line in the content (1-indexed line number).
///
/// Returns `Some((start_byte, end_byte))` for the line content,
/// or `None` if the line number is invalid.
pub(crate) fn find_line_byte_range(content: &str, line_num: usize) -> Option<(usize, usize)> {
    if line_num == 0 {
        return None;
    }

    let target_idx = line_num - 1; // Convert to 0-indexed

    // Simple approach: find the byte position by scanning the actual bytes
    let bytes = content.as_bytes();
    let mut line_start = 0;
    let mut current_line = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if current_line == target_idx {
            // Found the start of our target line, now find its end
            let mut line_end = i;
            for j in i..bytes.len() {
                if bytes[j] == b'\n' {
                    // Don't include \r if present
                    line_end = if j > 0 && bytes[j - 1] == b'\r' {
                        j - 1
                    } else {
                        j
                    };
                    break;
                }
                line_end = j + 1;
            }
            return Some((i, line_end));
        }

        if byte == b'\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // Handle last line (no trailing newline)
    if current_line == target_idx {
        return Some((line_start, bytes.len()));
    }

    None
}

/// Convert a byte offset to character offset.
///
/// This is needed because `String::find()` returns byte offsets, but egui's
/// text system (CCursor) uses character offsets. For ASCII text they're the same,
/// but for UTF-8 content with multi-byte characters they differ.
pub(crate) fn byte_to_char_offset(content: &str, byte_offset: usize) -> usize {
    // Count characters up to the byte offset
    content[..byte_offset.min(content.len())].chars().count()
}

/// Convert a character offset to (line, column) - 0-indexed.
///
/// NOTE: This expects a CHARACTER offset, not a byte offset.
/// Use `byte_to_char_offset()` first if you have a byte offset from `String::find()`.
pub(crate) fn offset_to_line_col(content: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in content.chars().enumerate() {
        if i >= char_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Get the current formatting state for the active editor.
///
/// Returns None if no editor is active.
pub(crate) fn get_formatting_state_for(
    content: &str,
    cursor_line: usize,
    cursor_col: usize,
) -> FormattingState {
    use crate::markdown::detect_raw_formatting_state;
    let char_index = line_col_to_char_index(content, cursor_line, cursor_col);
    detect_raw_formatting_state(content, char_index)
}
