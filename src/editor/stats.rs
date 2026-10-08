//! Text statistics for the editor
//!
//! This module provides efficient counting of words, characters, lines,
//! and paragraphs for display in the status bar and statistics panel.

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
// DocumentStats
// ─────────────────────────────────────────────────────────────────────────────

/// Comprehensive document statistics including text and markdown elements.
///
/// Combines basic text statistics (from `TextStats`) with markdown-specific
/// metrics like heading counts, link counts, code blocks, etc.
///
/// # Example
///
/// ```ignore
/// let stats = DocumentStats::from_text("# Hello\n\nThis is [a link](url).");
/// assert_eq!(stats.text.words, 5);
/// assert_eq!(stats.headings_by_level[0], 1); // 1 H1
/// assert_eq!(stats.link_count, 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentStats {
    /// Basic text statistics (words, chars, lines, paragraphs)
    pub text: TextStats,
    /// Count of headings at each level (index 0 = H1, index 5 = H6)
    pub headings_by_level: [usize; 6],
    /// Total number of headings
    pub heading_count: usize,
    /// Number of markdown links [text](url)
    pub link_count: usize,
    /// Number of images ![alt](url)
    pub image_count: usize,
    /// Number of fenced code blocks (excluding mermaid)
    pub code_block_count: usize,
    /// Number of Mermaid diagrams
    pub mermaid_count: usize,
    /// Number of tables
    pub table_count: usize,
    /// Number of blockquotes
    pub blockquote_count: usize,
    /// Estimated reading time in minutes (250 WPM)
    pub reading_time_minutes: u32,
    /// Number of list items (both ordered and unordered)
    pub list_item_count: usize,
    /// Number of horizontal rules
    pub horizontal_rule_count: usize,
}

impl DocumentStats {
    /// Create empty document statistics.
    #[allow(dead_code)] // Public API for stats initialization
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate comprehensive statistics from markdown text.
    ///
    /// This parses the text to extract both basic text statistics
    /// and markdown-specific element counts.
    pub fn from_text(text: &str) -> Self {
        let text_stats = TextStats::from_text(text);
        let mut stats = Self {
            text: text_stats,
            ..Default::default()
        };

        // Calculate reading time (250 WPM average)
        stats.reading_time_minutes = ((stats.text.words as f32 / 250.0).ceil() as u32).max(1);

        // Parse markdown elements
        stats.parse_markdown_elements(text);

        stats
    }

    /// Parse markdown elements from text.
    fn parse_markdown_elements(&mut self, text: &str) {
        let mut in_code_block = false;
        let mut code_block_is_mermaid = false;

        for line in text.lines() {
            let trimmed = line.trim();

            // Handle code block boundaries
            if trimmed.starts_with("```") {
                if !in_code_block {
                    // Starting a code block
                    in_code_block = true;
                    let lang = trimmed.trim_start_matches('`').trim();
                    code_block_is_mermaid = lang.eq_ignore_ascii_case("mermaid");
                } else {
                    // Ending a code block - count it
                    if code_block_is_mermaid {
                        self.mermaid_count += 1;
                    } else {
                        self.code_block_count += 1;
                    }
                    in_code_block = false;
                    code_block_is_mermaid = false;
                }
                continue;
            }

            // Skip content inside code blocks
            if in_code_block {
                continue;
            }

            // Check for headings
            if let Some(level) = Self::parse_heading_level(trimmed) {
                if level >= 1 && level <= 6 {
                    self.headings_by_level[(level - 1) as usize] += 1;
                    self.heading_count += 1;
                }
            }
            // Check for horizontal rules
            else if Self::is_horizontal_rule(trimmed) {
                self.horizontal_rule_count += 1;
            }
            // Check for list items
            else if Self::is_list_item(trimmed) {
                self.list_item_count += 1;
            }
            // Check for blockquotes (only first line of multi-line quote)
            else if trimmed.starts_with('>') {
                // Count blockquote starts (simplified - counts each > line)
                // A more accurate count would track contiguous blockquote sections
            }

            // Check for tables (simplified - count lines with | that aren't in code)
            if Self::is_table_line(trimmed) && !trimmed.starts_with('>') {
                // Tables are counted by the outline extractor, we'll sync with that
            }

            // Count links and images (can appear anywhere in a line)
            self.link_count += Self::count_links(line);
            self.image_count += Self::count_images(line);
        }
    }

    /// Parse heading level from a line (1-6 for H1-H6, None if not a heading).
    fn parse_heading_level(line: &str) -> Option<u8> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            return None;
        }

        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
        if hash_count == 0 || hash_count > 6 {
            return None;
        }

        let rest = &trimmed[hash_count..];
        if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
            return None;
        }

        Some(hash_count as u8)
    }

    /// Check if a line is a horizontal rule.
    fn is_horizontal_rule(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }

        // Must be 3+ of same char: ---, ***, ___
        let chars: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.len() < 3 {
            return false;
        }

        let first = chars[0];
        if first != '-' && first != '*' && first != '_' {
            return false;
        }

        chars.iter().all(|&c| c == first)
    }

    /// Check if a line is a list item.
    pub(crate) fn is_list_item(line: &str) -> bool {
        let trimmed = line.trim_start();
        let mut chars = trimmed.chars();

        // Unordered: starts with -, *, + followed by space
        if let (Some(first), Some(second)) = (chars.next(), chars.next()) {
            if (first == '-' || first == '*' || first == '+') && second == ' ' {
                return true;
            }
        }

        // Ordered: starts with digit(s) followed by . or ) and space
        let mut chars = trimmed.chars();
        let mut digit_count = 0usize;
        while let Some(c) = chars.next() {
            if c.is_ascii_digit() {
                digit_count += 1;
                if digit_count > 9 {
                    return false;
                }
            } else if (c == '.' || c == ')') && digit_count > 0 {
                return matches!(chars.next(), Some(' '));
            } else {
                return false;
            }
        }

        false
    }

    /// Check if a line is part of a table.
    fn is_table_line(line: &str) -> bool {
        if line.starts_with('|') {
            return line.matches('|').count() >= 2;
        }
        if line.contains('|') && !line.starts_with('>') {
            let parts: Vec<&str> = line.split('|').collect();
            return parts.len() >= 2;
        }
        false
    }

    /// Count markdown links in a line (not images).
    fn count_links(line: &str) -> usize {
        let mut count = 0;
        let mut chars = line.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            // Skip images - they start with ![
            if c == '!' {
                if let Some(&(_, '[')) = chars.peek() {
                    // Skip the image
                    continue;
                }
            }

            // Look for [ that starts a link
            if c == '[' {
                // Make sure it's not preceded by ! (image)
                let is_image = i > 0 && line.chars().nth(i - 1) == Some('!');
                if is_image {
                    continue;
                }

                // Find the closing ]( pattern
                let rest = &line[i..];
                if let Some(bracket_end) = rest.find(']') {
                    // Use safe slicing with get() to avoid panicking on invalid UTF-8 boundaries
                    let after_bracket = rest.get(bracket_end + 1..);
                    if let Some(after) = after_bracket {
                        if after.starts_with('(') {
                            // Check if there's a closing )
                            let after_paren = rest.get(bracket_end + 2..);
                            if let Some(remainder) = after_paren {
                                if remainder.contains(')') {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        count
    }

    /// Count images in a line.
    fn count_images(line: &str) -> usize {
        let mut count = 0;
        let mut search_start = 0;

        while let Some(pos) = line[search_start..].find("![") {
            let abs_pos = search_start + pos;
            let rest = &line[abs_pos..];

            // Find ]( pattern
            if let Some(bracket_end) = rest.find("](") {
                // Use safe slicing with get() to avoid panicking on invalid UTF-8 boundaries
                let after_pattern = rest.get(bracket_end + 2..);
                if let Some(remainder) = after_pattern {
                    if remainder.contains(')') {
                        count += 1;
                    }
                }
            }

            // Move past this "![" to find next - use char_indices to be safe
            search_start = abs_pos + 2;
        }

        count
    }

    /// Get the total heading count.
    #[allow(dead_code)] // Public API for document statistics
    pub fn total_headings(&self) -> usize {
        self.heading_count
    }

    /// Format reading time for display.
    pub fn format_reading_time(&self) -> String {
        if self.reading_time_minutes == 1 {
            "1 min read".to_string()
        } else {
            format!("{} min read", self.reading_time_minutes)
        }
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

    // ─────────────────────────────────────────────────────────────────────────
    // DocumentStats Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_doc_stats_empty() {
        let stats = DocumentStats::from_text("");
        assert_eq!(stats.text.words, 0);
        assert_eq!(stats.heading_count, 0);
        assert_eq!(stats.link_count, 0);
        assert_eq!(stats.image_count, 0);
        assert_eq!(stats.reading_time_minutes, 1); // Minimum 1 minute
    }

    #[test]
    fn test_doc_stats_headings() {
        let text = "# H1\n## H2\n## Another H2\n### H3\n#### H4\n##### H5\n###### H6";
        let stats = DocumentStats::from_text(text);

        assert_eq!(stats.headings_by_level[0], 1); // H1
        assert_eq!(stats.headings_by_level[1], 2); // H2
        assert_eq!(stats.headings_by_level[2], 1); // H3
        assert_eq!(stats.headings_by_level[3], 1); // H4
        assert_eq!(stats.headings_by_level[4], 1); // H5
        assert_eq!(stats.headings_by_level[5], 1); // H6
        assert_eq!(stats.heading_count, 7);
    }

    #[test]
    fn test_doc_stats_links() {
        let text = "Here is [a link](http://example.com) and [another](url).";
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.link_count, 2);
        assert_eq!(stats.image_count, 0);
    }

    #[test]
    fn test_doc_stats_images() {
        let text = "![Image 1](path/to/img.png)\n\nSome text\n\n![Image 2](url)";
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.image_count, 2);
        assert_eq!(stats.link_count, 0); // Images are not counted as links
    }

    #[test]
    fn test_doc_stats_mixed_links_and_images() {
        let text = "A [link](url) and an ![image](img.png) on same line.";
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.link_count, 1);
        assert_eq!(stats.image_count, 1);
    }

    #[test]
    fn test_doc_stats_code_blocks() {
        let text = r#"# Code Example

```rust
fn main() {
    println!("Hello");
}
```

Some text.

```python
print("World")
```
"#;
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.code_block_count, 2);
        assert_eq!(stats.heading_count, 1);
    }

    #[test]
    fn test_doc_stats_mermaid() {
        let text = r#"# Diagram

```mermaid
flowchart TD
    A --> B
```

```MERMAID
graph LR
    X --> Y
```
"#;
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.mermaid_count, 2);
        assert_eq!(stats.code_block_count, 0);
    }

    #[test]
    fn test_is_list_item_multibyte_no_panic() {
        // Multi-byte line starts must not panic and must not match unless a real marker is present
        for line in ["§ foo", "ß", "中文", "é item"] {
            assert!(
                !DocumentStats::is_list_item(line),
                "expected false for {line:?}"
            );
        }
    }

    #[test]
    fn test_is_list_item_ascii_markers() {
        assert!(DocumentStats::is_list_item("- item"));
        assert!(DocumentStats::is_list_item("* item"));
        assert!(DocumentStats::is_list_item("+ item"));
        assert!(DocumentStats::is_list_item("1. item"));
        assert!(DocumentStats::is_list_item("2) item"));

        assert!(!DocumentStats::is_list_item("-item"));
        assert!(!DocumentStats::is_list_item("1.item"));
    }

    #[test]
    fn test_doc_stats_multibyte_line_start_no_panic() {
        let text = "§ section\nß\n中文\né item\n- real list item";
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.list_item_count, 1);
    }

    #[test]
    fn test_doc_stats_list_items() {
        let text = r#"# List

- Item 1
- Item 2
* Item 3
+ Item 4

1. First
2. Second
3) Third
"#;
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.list_item_count, 7);
    }

    #[test]
    fn test_doc_stats_horizontal_rules() {
        let text = "Text\n\n---\n\nMore text\n\n***\n\n___\n\nEnd";
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.horizontal_rule_count, 3);
    }

    #[test]
    fn test_doc_stats_reading_time() {
        // 500 words at 250 WPM = 2 minutes
        let words: Vec<&str> = std::iter::repeat("word").take(500).collect();
        let text = words.join(" ");
        let stats = DocumentStats::from_text(&text);
        assert_eq!(stats.reading_time_minutes, 2);

        // 100 words at 250 WPM = 0.4 minutes, rounded up to 1
        let words: Vec<&str> = std::iter::repeat("word").take(100).collect();
        let text = words.join(" ");
        let stats = DocumentStats::from_text(&text);
        assert_eq!(stats.reading_time_minutes, 1);
    }

    #[test]
    fn test_doc_stats_code_block_ignores_content() {
        // Headings and links inside code blocks should be ignored
        let text = r#"# Real heading

```markdown
# Not a heading
[not a link](url)
![not an image](img)
- not a list item
---
```
"#;
        let stats = DocumentStats::from_text(text);
        assert_eq!(stats.heading_count, 1); // Only "Real heading"
        assert_eq!(stats.link_count, 0);
        assert_eq!(stats.image_count, 0);
        assert_eq!(stats.list_item_count, 0);
        assert_eq!(stats.horizontal_rule_count, 0);
    }

    #[test]
    fn test_doc_stats_format_reading_time() {
        let mut stats = DocumentStats::new();
        stats.reading_time_minutes = 1;
        assert_eq!(stats.format_reading_time(), "1 min read");

        stats.reading_time_minutes = 5;
        assert_eq!(stats.format_reading_time(), "5 min read");
    }

    #[test]
    fn test_doc_stats_comprehensive() {
        let text = r#"# My Document

This is a paragraph with a [link](http://example.com) and some text.

## Section 1

![Screenshot](./image.png)

Here's a list:
- First item
- Second item
- Third item

---

## Section 2

```rust
fn example() {}
```

```mermaid
graph TD
    A --> B
```

### Subsection

More [links](url1) and [more](url2) here.

The end."#;

        let stats = DocumentStats::from_text(text);

        assert_eq!(stats.heading_count, 4); // H1, 2x H2, H3
        assert_eq!(stats.headings_by_level[0], 1); // H1
        assert_eq!(stats.headings_by_level[1], 2); // H2
        assert_eq!(stats.headings_by_level[2], 1); // H3
        assert_eq!(stats.link_count, 3);
        assert_eq!(stats.image_count, 1);
        assert_eq!(stats.code_block_count, 1);
        assert_eq!(stats.mermaid_count, 1);
        assert_eq!(stats.list_item_count, 3);
        assert_eq!(stats.horizontal_rule_count, 1);
        assert!(stats.text.words > 20);
    }
}
