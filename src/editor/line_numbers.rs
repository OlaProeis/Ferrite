//! Line counting utilities for the text editor

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Count the number of lines in the given text.
///
/// Returns at least 1 for empty text (representing a single empty line).
pub fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.chars().filter(|&c| c == '\n').count() + 1
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(count_lines(""), 1);
    }

    #[test]
    fn test_count_lines_single_line() {
        assert_eq!(count_lines("Hello, World!"), 1);
    }

    #[test]
    fn test_count_lines_multiple_lines() {
        assert_eq!(count_lines("Line 1\nLine 2\nLine 3"), 3);
    }

    #[test]
    fn test_count_lines_trailing_newline() {
        assert_eq!(count_lines("Line 1\n"), 2);
    }

    #[test]
    fn test_count_lines_only_newlines() {
        assert_eq!(count_lines("\n\n\n"), 4);
    }
}
