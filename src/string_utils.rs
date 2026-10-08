//! UTF-8 Safe String Utilities
//!
//! This module provides safe string slicing operations that handle UTF-8
//! character boundaries correctly. Rust strings are UTF-8 encoded, so indices
//! must fall on character boundaries. These utilities ensure safe slicing
//! even when given arbitrary byte positions (e.g., from egui cursor positions).

// Allow dead code - this is a utility module with functions for future use
#![allow(dead_code)]
//!
//! # Problem
//! Characters like `ø`, `æ`, `å`, `中`, `🎉` are multi-byte in UTF-8.
//! If you try `text[5..10]` and index 5 or 10 falls inside a multi-byte
//! character, Rust panics.
//!
//! # Solution
//! Use `floor_char_boundary()` and `ceil_char_boundary()` to adjust indices
//! to valid UTF-8 character boundaries before slicing.
//!
//! # Example
//! ```ignore
//! use crate::string_utils::{safe_slice, safe_slice_from, safe_slice_to};
//!
//! let text = "Hei på deg"; // Norwegian text with ø
//! let slice = safe_slice(text, 4, 7); // Safe even if indices are mid-char
//! ```

// ─────────────────────────────────────────────────────────────────────────────
// Character Boundary Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the largest index that is less than or equal to `index`
/// and is on a UTF-8 character boundary.
///
/// If `index` is greater than the string length, returns the string length.
/// If `index` is already on a character boundary, returns `index`.
///
/// # Example
/// ```ignore
/// let s = "Hei på deg"; // 'å' is 2 bytes
/// let idx = floor_char_boundary(s, 5); // Returns valid boundary
/// ```
#[inline]
pub fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    if index == 0 {
        return 0;
    }

    // Walk backwards to find the start of the character
    let bytes = s.as_bytes();
    let mut i = index;
    while i > 0 && !is_utf8_char_start(bytes[i]) {
        i -= 1;
    }
    i
}

/// Returns the smallest index that is greater than or equal to `index`
/// and is on a UTF-8 character boundary.
///
/// If `index` is greater than or equal to the string length, returns the string length.
/// If `index` is already on a character boundary, returns `index`.
///
/// # Example
/// ```ignore
/// let s = "Hei på deg"; // 'å' is 2 bytes
/// let idx = ceil_char_boundary(s, 5); // Returns valid boundary
/// ```
#[inline]
pub fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    if index == 0 {
        return 0;
    }

    // Walk forwards to find the start of the next character
    let bytes = s.as_bytes();
    let mut i = index;
    while i < bytes.len() && !is_utf8_char_start(bytes[i]) {
        i += 1;
    }
    i
}

/// Check if a byte is the start of a UTF-8 character.
///
/// In UTF-8:
/// - Single-byte chars (ASCII): 0xxxxxxx (0x00-0x7F)
/// - Multi-byte char start: 11xxxxxx (0xC0-0xFF)
/// - Continuation bytes: 10xxxxxx (0x80-0xBF)
///
/// This returns true for single-byte chars and multi-byte start bytes.
#[inline]
fn is_utf8_char_start(byte: u8) -> bool {
    // A byte is a char start if it's NOT a continuation byte (10xxxxxx)
    (byte & 0b11000000) != 0b10000000
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe Slicing Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Safely slice a string from `start` to `end`, adjusting indices to
/// valid UTF-8 character boundaries.
///
/// - `start` is adjusted down to the nearest character boundary (floor)
/// - `end` is adjusted up to the nearest character boundary (ceil)
///
/// If `start >= end` after adjustment, returns an empty string.
///
/// # Example
/// ```ignore
/// let text = "Hello 世界!";
/// let slice = safe_slice(text, 6, 12); // "世界"
/// ```
#[inline]
pub fn safe_slice(s: &str, start: usize, end: usize) -> &str {
    let start = floor_char_boundary(s, start);
    let end = ceil_char_boundary(s, end);

    if start >= end {
        return "";
    }

    &s[start..end]
}

/// Safely slice from the beginning of a string to `end`.
///
/// `end` is adjusted to a valid character boundary.
///
/// # Example
/// ```ignore
/// let text = "Hei på deg";
/// let slice = safe_slice_to(text, 5); // "Hei p"
/// ```
#[inline]
pub fn safe_slice_to(s: &str, end: usize) -> &str {
    let end = floor_char_boundary(s, end);
    &s[..end]
}

/// Safely slice from `start` to the end of a string.
///
/// `start` is adjusted to a valid character boundary.
///
/// # Example
/// ```ignore
/// let text = "Hei på deg";
/// let slice = safe_slice_from(text, 5); // "å deg"
/// ```
#[inline]
pub fn safe_slice_from(s: &str, start: usize) -> &str {
    let start = ceil_char_boundary(s, start);
    &s[start..]
}

/// Check if an index is on a valid UTF-8 character boundary.
///
/// Returns true if slicing at this index would be safe.
#[inline]
pub fn is_char_boundary(s: &str, index: usize) -> bool {
    if index == 0 || index >= s.len() {
        return true;
    }
    is_utf8_char_start(s.as_bytes()[index])
}

/// Clamp an index to a valid character boundary, preferring floor.
///
/// This is useful when you have a cursor position that might be
/// in the middle of a character.
#[inline]
pub fn clamp_to_char_boundary(s: &str, index: usize) -> usize {
    floor_char_boundary(s, index)
}

/// Get the byte length of the character at the given byte index.
///
/// Returns 0 if the index is out of bounds or in the middle of a character.
#[inline]
pub fn char_len_at(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return 0;
    }

    let bytes = s.as_bytes();
    let b = bytes[index];

    // Check if this is a continuation byte
    if !is_utf8_char_start(b) {
        return 0;
    }

    // Determine character length from first byte
    if b < 0x80 {
        1 // ASCII
    } else if b < 0xE0 {
        2 // 2-byte char
    } else if b < 0xF0 {
        3 // 3-byte char
    } else {
        4 // 4-byte char
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Index Conversion Utilities
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a character index to a byte index.
///
/// This is useful when you have a character count (e.g., from user input)
/// and need to convert it to a byte offset for slicing.
///
/// Returns the string length if `char_index` is beyond the string.
pub fn char_index_to_byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// True for Unicode line breaks ropey treats as newlines (matches `ropey::Rope`).
fn is_ropey_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{2028}' | '\u{2029}' | '\x0b' | '\x0c' | '\u{0085}'
    )
}

/// Character length of `line` content excluding ropey trailing line breaks.
fn rope_line_content_char_len(text: &str, line: usize) -> usize {
    ropey::Rope::from_str(text)
        .get_line(line)
        .map(|l| l.chars().take_while(|c| !is_ropey_line_break(*c)).count())
        .unwrap_or(0)
}

/// Convert (line, column) to a character index using ropey's line-break rules.
pub fn rope_line_col_to_char_index(text: &str, line: usize, col: usize) -> usize {
    let rope = ropey::Rope::from_str(text);
    let line_start = rope.try_line_to_char(line).unwrap_or(0);
    let line_len = rope_line_content_char_len(text, line);
    line_start + col.min(line_len)
}

/// Character index at the start of `line` (0-indexed), per ropey line breaks.
pub fn rope_char_index_at_line_start(text: &str, line: usize) -> usize {
    ropey::Rope::from_str(text)
        .try_line_to_char(line)
        .unwrap_or_else(|_| text.chars().count())
}

/// Line count per ropey (includes `\r`, U+2028, form-feed, etc.).
pub fn rope_line_count(text: &str) -> usize {
    ropey::Rope::from_str(text).len_lines()
}

/// Convert a byte index to a character index.
///
/// Returns the number of characters before the given byte index.
/// If the byte index is in the middle of a character, it counts
/// up to (but not including) that character.
pub fn byte_index_to_char_index(s: &str, byte_index: usize) -> usize {
    let byte_index = floor_char_boundary(s, byte_index);
    s[..byte_index].chars().count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // floor_char_boundary Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_floor_ascii() {
        let s = "Hello";
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 2), 2);
        assert_eq!(floor_char_boundary(s, 5), 5);
        assert_eq!(floor_char_boundary(s, 10), 5); // Beyond end
    }

    #[test]
    fn test_floor_norwegian() {
        let s = "Hei på deg"; // 'å' at byte 5-6 (2 bytes)
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 4), 4); // 'p'
        assert_eq!(floor_char_boundary(s, 5), 5); // Start of 'å'
        assert_eq!(floor_char_boundary(s, 6), 5); // Middle of 'å', floors to 5
        assert_eq!(floor_char_boundary(s, 7), 7); // ' '
    }

    #[test]
    fn test_floor_chinese() {
        let s = "你好世界"; // Each char is 3 bytes
        assert_eq!(floor_char_boundary(s, 0), 0); // Start of '你'
        assert_eq!(floor_char_boundary(s, 1), 0); // Middle of '你'
        assert_eq!(floor_char_boundary(s, 2), 0); // Middle of '你'
        assert_eq!(floor_char_boundary(s, 3), 3); // Start of '好'
        assert_eq!(floor_char_boundary(s, 4), 3); // Middle of '好'
    }

    #[test]
    fn test_floor_emoji() {
        let s = "Hi🎉!"; // 🎉 is 4 bytes
        assert_eq!(floor_char_boundary(s, 2), 2); // Start of 🎉
        assert_eq!(floor_char_boundary(s, 3), 2); // Middle of 🎉
        assert_eq!(floor_char_boundary(s, 4), 2); // Middle of 🎉
        assert_eq!(floor_char_boundary(s, 5), 2); // Middle of 🎉
        assert_eq!(floor_char_boundary(s, 6), 6); // '!'
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ceil_char_boundary Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ceil_ascii() {
        let s = "Hello";
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 2), 2);
        assert_eq!(ceil_char_boundary(s, 5), 5);
        assert_eq!(ceil_char_boundary(s, 10), 5);
    }

    #[test]
    fn test_ceil_norwegian() {
        let s = "Hei på deg"; // 'å' at byte 5-6 (2 bytes)
        assert_eq!(ceil_char_boundary(s, 5), 5); // Start of 'å'
        assert_eq!(ceil_char_boundary(s, 6), 7); // Middle of 'å', ceils to next char
    }

    #[test]
    fn test_ceil_chinese() {
        let s = "你好"; // Each char is 3 bytes
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 1), 3); // Middle of '你', ceils to '好'
        assert_eq!(ceil_char_boundary(s, 2), 3);
        assert_eq!(ceil_char_boundary(s, 3), 3);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // safe_slice Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_safe_slice_ascii() {
        let s = "Hello World";
        assert_eq!(safe_slice(s, 0, 5), "Hello");
        assert_eq!(safe_slice(s, 6, 11), "World");
        assert_eq!(safe_slice(s, 0, 100), "Hello World");
    }

    #[test]
    fn test_safe_slice_norwegian() {
        let s = "Hei på deg";
        // Even if we give mid-character indices, it should work
        assert_eq!(safe_slice(s, 0, 3), "Hei");
        assert_eq!(safe_slice(s, 4, 7), "på"); // 'å' is at byte 5-6
    }

    #[test]
    fn test_safe_slice_chinese() {
        let s = "你好世界";
        assert_eq!(safe_slice(s, 0, 3), "你");
        assert_eq!(safe_slice(s, 3, 6), "好");
        assert_eq!(safe_slice(s, 0, 12), "你好世界");
    }

    #[test]
    fn test_safe_slice_emoji() {
        let s = "Hi🎉Bye";
        assert_eq!(safe_slice(s, 0, 2), "Hi");
        assert_eq!(safe_slice(s, 2, 6), "🎉"); // 🎉 is 4 bytes
        assert_eq!(safe_slice(s, 6, 9), "Bye");
    }

    #[test]
    fn test_safe_slice_empty() {
        let s = "Hello";
        assert_eq!(safe_slice(s, 5, 5), "");
        assert_eq!(safe_slice(s, 3, 2), ""); // start > end
    }

    // ─────────────────────────────────────────────────────────────────────────
    // safe_slice_to / safe_slice_from Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_safe_slice_to() {
        let s = "Hei på deg";
        assert_eq!(safe_slice_to(s, 3), "Hei");
        assert_eq!(safe_slice_to(s, 6), "Hei p"); // Floors mid-char
    }

    #[test]
    fn test_safe_slice_from() {
        let s = "Hei på deg";
        assert_eq!(safe_slice_from(s, 4), "på deg");
        assert_eq!(safe_slice_from(s, 6), " deg"); // Ceils mid-char
    }

    // ─────────────────────────────────────────────────────────────────────────
    // is_char_boundary Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_is_char_boundary() {
        let s = "Hei på";
        assert!(is_char_boundary(s, 0));
        assert!(is_char_boundary(s, 4));
        assert!(is_char_boundary(s, 5)); // Start of 'å'
        assert!(!is_char_boundary(s, 6)); // Middle of 'å'
        assert!(is_char_boundary(s, 7)); // End
    }

    // ─────────────────────────────────────────────────────────────────────────
    // char_len_at Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_char_len_at() {
        let s = "Aå中🎉";
        assert_eq!(char_len_at(s, 0), 1); // 'A' - ASCII
        assert_eq!(char_len_at(s, 1), 2); // 'å' - 2 bytes
        assert_eq!(char_len_at(s, 2), 0); // Middle of 'å'
        assert_eq!(char_len_at(s, 3), 3); // '中' - 3 bytes
        assert_eq!(char_len_at(s, 6), 4); // '🎉' - 4 bytes
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Index Conversion Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_char_to_byte_index() {
        let s = "Hei på"; // H(1) e(1) i(1) ' '(1) p(1) å(2) = 7 bytes, 6 chars
        assert_eq!(char_index_to_byte_index(s, 0), 0); // 'H'
        assert_eq!(char_index_to_byte_index(s, 4), 4); // 'p'
        assert_eq!(char_index_to_byte_index(s, 5), 5); // 'å' starts at byte 5
        assert_eq!(char_index_to_byte_index(s, 6), 7); // End
        assert_eq!(char_index_to_byte_index(s, 100), 7); // Beyond end
    }

    #[test]
    fn test_byte_to_char_index() {
        let s = "Hei på";
        assert_eq!(byte_index_to_char_index(s, 0), 0);
        assert_eq!(byte_index_to_char_index(s, 4), 4);
        assert_eq!(byte_index_to_char_index(s, 5), 5);
        assert_eq!(byte_index_to_char_index(s, 6), 5); // Middle of 'å', counts up to start
        assert_eq!(byte_index_to_char_index(s, 7), 6);
    }

    #[test]
    fn rope_line_col_clamps_past_line_end() {
        let text = "éé\nx";
        assert_eq!(rope_line_col_to_char_index(text, 0, 99), 2);
    }

    #[test]
    fn rope_line_index_matches_ropey_unicode_breaks() {
        use super::{rope_char_index_at_line_start, rope_line_col_to_char_index, rope_line_count};
        let text = format!("a{LS}b{FF}c", LS = '\u{2028}', FF = '\u{000c}');
        assert_eq!(rope_line_count(&text), 3);
        assert_eq!(rope_char_index_at_line_start(&text, 0), 0);
        assert_eq!(rope_char_index_at_line_start(&text, 1), 2);
        assert_eq!(rope_char_index_at_line_start(&text, 2), 4);
        assert_eq!(rope_line_col_to_char_index(&text, 1, 1), 3);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_string() {
        let s = "";
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(ceil_char_boundary(s, 0), 0);
        assert_eq!(safe_slice(s, 0, 0), "");
        assert_eq!(safe_slice_to(s, 0), "");
        assert_eq!(safe_slice_from(s, 0), "");
    }

    #[test]
    fn test_mixed_content() {
        let s = "Hello 世界! 🎉 Café naïve";
        // Should not panic on any index
        for i in 0..=s.len() + 5 {
            let _ = floor_char_boundary(s, i);
            let _ = ceil_char_boundary(s, i);
            let _ = safe_slice(s, 0, i);
            let _ = safe_slice_to(s, i);
            let _ = safe_slice_from(s, i);
        }
    }

    #[test]
    fn test_all_norwegian_chars() {
        // Test all Norwegian special characters
        let s = "æøåÆØÅ";
        for i in 0..=s.len() {
            let floor = floor_char_boundary(s, i);
            let ceil = ceil_char_boundary(s, i);
            // These should never panic
            let _ = &s[..floor];
            let _ = &s[ceil..];
        }
    }
}
