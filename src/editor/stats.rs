//! Text statistics for the editor
//!
//! This module provides efficient counting of words, characters, lines,
//! and paragraphs for display in the status bar.

use super::count_lines;

// ─────────────────────────────────────────────────────────────────────────────
// TextStats
// ─────────────────────────────────────────────────────────────────────────────

/// Text statistics for a document.
///
/// Contains counts of words, characters (with and without spaces),
/// lines, and paragraphs.
///
/// # Example
///
/// ```ignore
/// let stats = TextStats::from_text("Hello, World!\n\nNew paragraph.");
/// assert_eq!(stats.words, 4);
/// assert_eq!(stats.paragraphs, 2);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextStats {
    /// Number of words (sequences of non-whitespace characters)
    pub words: usize,
    /// Number of characters including whitespace
    pub characters: usize,
    /// Number of characters excluding whitespace
    pub characters_no_spaces: usize,
    /// Number of lines (including empty lines)
    pub lines: usize,
    /// Number of paragraphs (non-empty text blocks separated by blank lines)
    pub paragraphs: usize,
}

impl TextStats {
    /// Create a new empty TextStats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate statistics from the given text.
    ///
    /// This is an efficient single-pass algorithm that calculates all
    /// statistics simultaneously.
    pub fn from_text(text: &str) -> Self {
        if text.is_empty() {
            return Self {
                words: 0,
                characters: 0,
                characters_no_spaces: 0,
                lines: 1, // Empty document has 1 line
                paragraphs: 0,
            };
        }

        let mut stats = Self::new();

        // Count lines using the existing function
        stats.lines = count_lines(text);

        // Single pass for words, characters, and paragraphs
        let mut in_word = false;
        let mut in_paragraph = false;
        let mut consecutive_newlines = 0;
        let mut line_has_content = false;

        for ch in text.chars() {
            // Count all characters
            stats.characters += 1;

            if ch.is_whitespace() {
                // End of word if we were in one
                if in_word {
                    in_word = false;
                }

                if ch == '\n' {
                    consecutive_newlines += 1;

                    // If we had content on this line, we're in a paragraph
                    if line_has_content && !in_paragraph {
                        in_paragraph = true;
                        stats.paragraphs += 1;
                    }

                    // Two or more consecutive newlines end a paragraph
                    if consecutive_newlines >= 2 {
                        in_paragraph = false;
                    }

                    line_has_content = false;
                } else {
                    consecutive_newlines = 0;
                }
            } else {
                // Non-whitespace character
                stats.characters_no_spaces += 1;
                consecutive_newlines = 0;
                line_has_content = true;

                // Start of word if we weren't in one
                if !in_word {
                    in_word = true;
                    stats.words += 1;
                }
            }
        }

        // Handle final paragraph if document doesn't end with newlines
        if line_has_content && !in_paragraph {
            stats.paragraphs += 1;
        }

        stats
    }

    /// Format the statistics for display in the status bar.
    ///
    /// Returns a compact string like "150 words | 892 chars | 25 lines"
    pub fn format_compact(&self) -> String {
        format!(
            "{} words | {} chars | {} lines",
            self.words, self.characters, self.lines
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // TextStats Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_stats_empty_text() {
        let stats = TextStats::from_text("");
        assert_eq!(stats.words, 0);
        assert_eq!(stats.characters, 0);
        assert_eq!(stats.characters_no_spaces, 0);
        assert_eq!(stats.lines, 1);
        assert_eq!(stats.paragraphs, 0);
    }

    #[test]
    fn test_stats_single_word() {
        let stats = TextStats::from_text("Hello");
        assert_eq!(stats.words, 1);
        assert_eq!(stats.characters, 5);
        assert_eq!(stats.characters_no_spaces, 5);
        assert_eq!(stats.lines, 1);
        assert_eq!(stats.paragraphs, 1);
    }

    #[test]
    fn test_stats_simple_sentence() {
        let stats = TextStats::from_text("Hello, World!");
        assert_eq!(stats.words, 2);
        assert_eq!(stats.characters, 13);
        assert_eq!(stats.characters_no_spaces, 12);
        assert_eq!(stats.lines, 1);
        assert_eq!(stats.paragraphs, 1);
    }

    #[test]
    fn test_stats_multiple_lines() {
        let stats = TextStats::from_text("Line one\nLine two\nLine three");
        assert_eq!(stats.words, 6);
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.paragraphs, 1); // Single paragraph (no blank lines)
    }

    #[test]
    fn test_stats_multiple_paragraphs() {
        let stats = TextStats::from_text("First paragraph.\n\nSecond paragraph.");
        assert_eq!(stats.words, 4);
        assert_eq!(stats.paragraphs, 2);
    }

    #[test]
    fn test_stats_multiple_paragraphs_complex() {
        let text =
            "Paragraph one here.\n\nParagraph two.\nStill paragraph two.\n\nParagraph three.";
        let stats = TextStats::from_text(text);
        assert_eq!(stats.paragraphs, 3);
    }

    #[test]
    fn test_stats_trailing_newline() {
        let stats = TextStats::from_text("Hello\n");
        assert_eq!(stats.lines, 2);
        assert_eq!(stats.words, 1);
        assert_eq!(stats.paragraphs, 1);
    }

    #[test]
    fn test_stats_only_whitespace() {
        let stats = TextStats::from_text("   \n\n   ");
        assert_eq!(stats.words, 0);
        assert_eq!(stats.characters, 8); // 3 spaces + 2 newlines + 3 spaces = 8
        assert_eq!(stats.characters_no_spaces, 0);
        assert_eq!(stats.paragraphs, 0);
    }

    #[test]
    fn test_stats_unicode() {
        // "Привет мир! 你好世界" = "Hello world! 你好世界"
        // Words: "Привет", "мир!", "你好世界" = 3 words (Chinese has no spaces)
        let stats = TextStats::from_text("Привет мир! 你好世界");
        assert_eq!(stats.words, 3);
        assert_eq!(stats.characters, 16);
        assert_eq!(stats.characters_no_spaces, 14);
    }

    #[test]
    fn test_stats_mixed_whitespace() {
        let stats = TextStats::from_text("word1  word2\t\tword3");
        assert_eq!(stats.words, 3);
    }

    #[test]
    fn test_stats_format_compact() {
        let stats = TextStats {
            words: 150,
            characters: 892,
            characters_no_spaces: 743,
            lines: 25,
            paragraphs: 5,
        };
        assert_eq!(stats.format_compact(), "150 words | 892 chars | 25 lines");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Default and Clone Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_stats_default() {
        let stats = TextStats::default();
        assert_eq!(stats.words, 0);
        assert_eq!(stats.characters, 0);
        assert_eq!(stats.characters_no_spaces, 0);
        assert_eq!(stats.lines, 0);
        assert_eq!(stats.paragraphs, 0);
    }

    #[test]
    fn test_stats_clone() {
        let stats = TextStats::from_text("Hello World");
        let cloned = stats.clone();
        assert_eq!(stats, cloned);
    }

    #[test]
    fn test_stats_copy() {
        let stats = TextStats::from_text("Hello World");
        let copied = stats;
        assert_eq!(stats.words, copied.words);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge Case Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_stats_only_newlines() {
        let stats = TextStats::from_text("\n\n\n");
        assert_eq!(stats.lines, 4);
        assert_eq!(stats.words, 0);
        assert_eq!(stats.paragraphs, 0);
    }

    #[test]
    fn test_stats_single_character() {
        let stats = TextStats::from_text("a");
        assert_eq!(stats.words, 1);
        assert_eq!(stats.characters, 1);
        assert_eq!(stats.lines, 1);
        assert_eq!(stats.paragraphs, 1);
    }

    #[test]
    fn test_stats_markdown_document() {
        let markdown = "# Heading\n\nThis is a paragraph with **bold** text.\n\n- Item 1\n- Item 2\n\nAnother paragraph.";
        let stats = TextStats::from_text(markdown);
        assert!(stats.words > 0);
        assert!(stats.paragraphs > 0);
        assert!(stats.lines > 0);
    }

    #[test]
    fn test_stats_real_world_text() {
        let text = r#"# My Document

This is the first paragraph. It contains multiple sentences.
Each sentence adds to the word count.

## Section Two

Here's another paragraph with some code: `let x = 42;`

And a final thought."#;

        let stats = TextStats::from_text(text);
        assert!(stats.words > 20);
        // Paragraphs: "# My Document", "This is the first...", "## Section Two",
        // "Here's another...", "And a final thought." = 5
        assert_eq!(stats.paragraphs, 5);
    }
}
