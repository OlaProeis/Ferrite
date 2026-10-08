//! Table of Contents Generation
//!
//! This module provides functionality to generate and manage a Table of Contents
//! (TOC) for markdown documents. It parses headings and generates navigable
//! markdown links with proper slugification.
//!
//! # TOC Block Format
//!
//! ```markdown
//! <!-- TOC -->
//! - [Introduction](#introduction)
//!   - [Getting Started](#getting-started)
//! - [Features](#features)
//! <!-- /TOC -->
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use crate::markdown::toc::{generate_toc, TocOptions};
//!
//! let markdown = "# Hello\n\n## World\n\nSome text";
//! let toc = generate_toc(markdown, TocOptions::default());
//! ```

use crate::editor::{extract_outline, OutlineItem};
use slug::slugify;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// TOC block start marker
pub const TOC_START: &str = "<!-- TOC -->";

/// TOC block end marker
pub const TOC_END: &str = "<!-- /TOC -->";

// ─────────────────────────────────────────────────────────────────────────────
// TOC Options
// ─────────────────────────────────────────────────────────────────────────────

/// Options for TOC generation.
#[derive(Debug, Clone)]
pub struct TocOptions {
    /// Minimum heading level to include (1-6)
    pub min_level: u8,
    /// Maximum heading level to include (1-6)
    pub max_level: u8,
    /// Whether to use bullet points (-) or numbers (1.)
    pub use_bullets: bool,
    /// Indentation string for nested items (typically "  ")
    pub indent: String,
}

impl Default for TocOptions {
    fn default() -> Self {
        Self {
            min_level: 1,
            max_level: 3,
            use_bullets: true,
            indent: "  ".to_string(),
        }
    }
}

impl TocOptions {
    /// Create options that include all heading levels (H1-H6).
    #[allow(dead_code)] // Public API for TOC generation options
    pub fn all_levels() -> Self {
        Self {
            max_level: 6,
            ..Default::default()
        }
    }

    /// Set the maximum heading depth to include.
    #[allow(dead_code)] // Builder pattern for TOC options
    pub fn with_max_level(mut self, level: u8) -> Self {
        self.max_level = level.clamp(1, 6);
        self
    }

    /// Set the minimum heading level to include.
    #[allow(dead_code)] // Builder pattern for TOC options
    pub fn with_min_level(mut self, level: u8) -> Self {
        self.min_level = level.clamp(1, 6);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Result
// ─────────────────────────────────────────────────────────────────────────────

/// Result of TOC generation/insertion.
#[derive(Debug, Clone)]
pub struct TocResult {
    /// The modified text with TOC inserted/updated
    pub text: String,
    /// New cursor position after TOC operation
    pub cursor: usize,
    /// Whether a TOC block was found and updated (vs inserted new)
    pub was_update: bool,
    /// Number of headings included in the TOC
    pub heading_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Heading
// ─────────────────────────────────────────────────────────────────────────────

/// A heading extracted for TOC generation.
#[derive(Debug, Clone)]
pub struct TocHeading {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text (with inline formatting stripped)
    pub text: String,
    /// Slugified anchor (e.g., "getting-started")
    pub anchor: String,
}

impl TocHeading {
    /// Create a new TOC heading from an outline item.
    pub fn from_outline_item(item: &OutlineItem) -> Option<Self> {
        if item.level == 0 {
            return None; // Not a heading (code block, table, etc.)
        }

        let anchor = slugify(&item.title);
        Some(Self {
            level: item.level,
            text: item.title.clone(),
            anchor,
        })
    }

    /// Generate the markdown line for this heading in the TOC.
    ///
    /// # Arguments
    /// * `base_level` - The minimum level in the document (for calculating indent)
    /// * `options` - TOC generation options
    pub fn to_toc_line(&self, base_level: u8, options: &TocOptions) -> String {
        let indent_level = self.level.saturating_sub(base_level);
        let indent = options.indent.repeat(indent_level as usize);
        let bullet = if options.use_bullets { "-" } else { "1." };
        format!("{}{} [{}](#{})", indent, bullet, self.text, self.anchor)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Generation Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Extract headings from markdown text for TOC generation.
///
/// This reuses the outline extraction logic but filters and converts
/// to TOC-specific heading structures.
pub fn extract_toc_headings(text: &str, options: &TocOptions) -> Vec<TocHeading> {
    let outline = extract_outline(text);

    outline
        .items
        .iter()
        .filter_map(|item| {
            // Only include actual headings within the level range
            if item.level >= options.min_level && item.level <= options.max_level {
                TocHeading::from_outline_item(item)
            } else {
                None
            }
        })
        .collect()
}

/// Generate a Table of Contents markdown string from the document.
///
/// # Arguments
/// * `text` - The markdown document text
/// * `options` - TOC generation options
///
/// # Returns
/// The TOC as a markdown string (without the <!-- TOC --> markers)
pub fn generate_toc_content(text: &str, options: &TocOptions) -> String {
    let headings = extract_toc_headings(text, options);

    if headings.is_empty() {
        return String::new();
    }

    // Find the base (minimum) level for calculating indentation
    let base_level = headings.iter().map(|h| h.level).min().unwrap_or(1);

    // Join with the document's own line ending so inserting a TOC into a
    // CRLF file doesn't introduce mixed endings.
    let ending = crate::state::LineEnding::detect_from_content(text);
    ending.join_lines(headings.iter().map(|h| h.to_toc_line(base_level, options)))
}

/// Generate a complete TOC block with markers.
///
/// # Arguments
/// * `text` - The markdown document text
/// * `options` - TOC generation options
///
/// # Returns
/// The complete TOC block including <!-- TOC --> markers
pub fn generate_toc_block(text: &str, options: &TocOptions) -> String {
    let content = generate_toc_content(text, options);
    let nl = crate::state::LineEnding::detect_from_content(text).as_str();

    if content.is_empty() {
        format!("{}{}{}", TOC_START, nl, TOC_END)
    } else {
        format!("{}{}{}{}{}", TOC_START, nl, content, nl, TOC_END)
    }
}

/// Find the existing TOC block in the document.
///
/// # Returns
/// `Some((start_byte, end_byte))` if a TOC block was found, `None` otherwise.
/// The range includes both markers and all content between them.
pub fn find_toc_block(text: &str) -> Option<(usize, usize)> {
    let start_pos = text.find(TOC_START)?;
    let end_pos = text[start_pos..].find(TOC_END)?;

    // end_pos is relative to start_pos, so adjust
    let absolute_end = start_pos + end_pos + TOC_END.len();

    Some((start_pos, absolute_end))
}

/// Insert or update the Table of Contents in a document.
///
/// If an existing TOC block is found (<!-- TOC -->...<!-- /TOC -->), it will be
/// replaced with the newly generated TOC. Otherwise, the TOC will be inserted
/// at the cursor position.
///
/// # Arguments
/// * `text` - The markdown document text
/// * `cursor` - Current cursor position (used for insertion if no existing TOC)
/// * `options` - TOC generation options
///
/// # Returns
/// A `TocResult` with the modified text and metadata
pub fn insert_or_update_toc(text: &str, cursor: usize, options: &TocOptions) -> TocResult {
    let toc_block = generate_toc_block(text, options);
    let heading_count = extract_toc_headings(text, options).len();

    // Check for existing TOC block
    if let Some((start, end)) = find_toc_block(text) {
        // Replace existing TOC
        let new_text = format!("{}{}{}", &text[..start], toc_block, &text[end..]);
        let new_cursor = start + toc_block.len();

        return TocResult {
            text: new_text,
            cursor: new_cursor,
            was_update: true,
            heading_count,
        };
    }

    // Insert at cursor position
    // Ensure we're on a valid char boundary
    let cursor = cursor.min(text.len());
    let cursor = crate::string_utils::floor_char_boundary(text, cursor);

    // Check if we need to add newlines for proper formatting
    let nl = crate::state::LineEnding::detect_from_content(text).as_str();
    let needs_newline_before = cursor > 0 && !text[..cursor].ends_with('\n');
    let needs_newline_after = cursor < text.len()
        && !(text[cursor..].starts_with('\n') || text[cursor..].starts_with("\r\n"));

    let prefix = if needs_newline_before {
        format!("{nl}{nl}")
    } else {
        String::new()
    };
    let suffix = if needs_newline_after {
        format!("{nl}{nl}")
    } else {
        String::new()
    };

    let new_text = format!(
        "{}{}{}{}{}",
        &text[..cursor],
        prefix,
        toc_block,
        suffix,
        &text[cursor..]
    );

    let new_cursor = cursor + prefix.len() + toc_block.len();

    TocResult {
        text: new_text,
        cursor: new_cursor,
        was_update: false,
        heading_count,
    }
}

/// Remove the TOC block from a document if present.
///
/// # Returns
/// The modified text with TOC removed, or the original text if no TOC found.
#[allow(dead_code)] // Public API for TOC removal
pub fn remove_toc(text: &str) -> String {
    if let Some((start, end)) = find_toc_block(text) {
        // Also remove surrounding empty lines if present
        let before = &text[..start];
        let after = &text[end..];

        // Trim trailing whitespace from before, and leading whitespace from after
        let before_trimmed = before.trim_end_matches(&[' ', '\t'][..]);
        let after_trimmed = after.trim_start_matches(&[' ', '\t', '\n', '\r'][..]);

        // Ensure proper spacing between sections
        if !before_trimmed.is_empty() && !after_trimmed.is_empty() {
            format!("{}\n\n{}", before_trimmed, after_trimmed)
        } else {
            format!("{}{}", before_trimmed, after_trimmed)
        }
    } else {
        text.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_heading() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Getting Started"), "getting-started");
        assert_eq!(slugify("Drag & Drop"), "drag-drop");
        assert_eq!(slugify("What's New?"), "what-s-new");
    }

    #[test]
    fn test_extract_toc_headings() {
        let text = "# Title\n\n## Section 1\n\nText\n\n## Section 2\n\n### Subsection";
        let headings = extract_toc_headings(text, &TocOptions::default());

        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].text, "Section 1");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[3].text, "Subsection");
        assert_eq!(headings[3].level, 3);
    }

    #[test]
    fn test_extract_toc_headings_with_level_filter() {
        let text = "# Title\n\n## Section\n\n### Subsection\n\n#### Deep";
        let options = TocOptions::default().with_max_level(2);
        let headings = extract_toc_headings(text, &options);

        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].text, "Section");
    }

    #[test]
    fn test_generate_toc_content() {
        let text = "# Title\n\n## Section 1\n\n## Section 2";
        let toc = generate_toc_content(text, &TocOptions::default());

        assert!(toc.contains("- [Title](#title)"));
        assert!(toc.contains("  - [Section 1](#section-1)"));
        assert!(toc.contains("  - [Section 2](#section-2)"));
    }

    #[test]
    fn test_generate_toc_block() {
        let text = "# Hello\n\n## World";
        let block = generate_toc_block(text, &TocOptions::default());

        assert!(block.starts_with(TOC_START));
        assert!(block.ends_with(TOC_END));
        assert!(block.contains("- [Hello](#hello)"));
    }

    #[test]
    fn test_find_toc_block() {
        let text = "Some text\n\n<!-- TOC -->\n- [A](#a)\n<!-- /TOC -->\n\nMore text";
        let result = find_toc_block(text);

        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!(text[start..end].starts_with(TOC_START));
        assert!(text[start..end].ends_with(TOC_END));
    }

    #[test]
    fn test_find_toc_block_not_found() {
        let text = "# Title\n\n## Section\n\nNo TOC here";
        assert!(find_toc_block(text).is_none());
    }

    #[test]
    fn test_insert_toc_new() {
        let text = "# Title\n\n## Section\n\nSome content";
        let result = insert_or_update_toc(text, 0, &TocOptions::default());

        assert!(!result.was_update);
        assert!(result.text.contains(TOC_START));
        assert!(result.text.contains("- [Title](#title)"));
        assert_eq!(result.heading_count, 2);
    }

    #[test]
    fn test_update_existing_toc() {
        let text = "# Title\n\n<!-- TOC -->\n- old content\n<!-- /TOC -->\n\n## Section";
        let result = insert_or_update_toc(text, 0, &TocOptions::default());

        assert!(result.was_update);
        assert!(result.text.contains("- [Title](#title)"));
        assert!(result.text.contains("- [Section](#section)"));
        assert!(!result.text.contains("old content"));
    }

    #[test]
    fn test_remove_toc() {
        let text = "# Title\n\n<!-- TOC -->\n- [Title](#title)\n<!-- /TOC -->\n\n## Section";
        let result = remove_toc(text);

        assert!(!result.contains(TOC_START));
        assert!(!result.contains(TOC_END));
        assert!(result.contains("# Title"));
        assert!(result.contains("## Section"));
    }

    #[test]
    fn test_toc_with_empty_document() {
        let text = "";
        let result = insert_or_update_toc(text, 0, &TocOptions::default());

        assert_eq!(result.heading_count, 0);
        assert!(result.text.contains(TOC_START));
        assert!(result.text.contains(TOC_END));
    }

    #[test]
    fn test_toc_with_no_headings() {
        let text = "Just some plain text\n\nwith no headings at all.";
        let toc = generate_toc_content(text, &TocOptions::default());

        assert!(toc.is_empty());
    }

    #[test]
    fn test_insert_toc_preserves_crlf() {
        // Inserting into a CRLF document must not introduce bare-LF lines
        // (the old `join("\n")` produced a mixed-EOL file).
        let text = "# Title\r\n\r\n## Section\r\n\r\nSome content\r\n";
        let result = insert_or_update_toc(text, text.len(), &TocOptions::default());

        assert!(result.text.contains(TOC_START));
        let without_crlf = result.text.replace("\r\n", "");
        assert!(
            !without_crlf.contains('\n') && !without_crlf.contains('\r'),
            "mixed line endings in: {:?}",
            result.text
        );
    }

    #[test]
    fn test_update_existing_toc_preserves_crlf() {
        let text =
            "# Title\r\n\r\n<!-- TOC -->\r\n- old content\r\n<!-- /TOC -->\r\n\r\n## Section\r\n";
        let result = insert_or_update_toc(text, 0, &TocOptions::default());

        assert!(result.was_update);
        let without_crlf = result.text.replace("\r\n", "");
        assert!(
            !without_crlf.contains('\n') && !without_crlf.contains('\r'),
            "mixed line endings in: {:?}",
            result.text
        );
    }

    #[test]
    fn test_toc_heading_with_special_chars() {
        let text = "# Hello & Goodbye!\n\n## What's New?";
        let headings = extract_toc_headings(text, &TocOptions::default());

        assert_eq!(headings[0].anchor, "hello-goodbye");
        assert_eq!(headings[1].anchor, "what-s-new");
    }

    #[test]
    fn test_toc_preserves_inline_formatting_text() {
        // The outline extractor strips inline formatting, so TOC gets clean text
        let text = "# **Bold** Title\n\n## `Code` Section";
        let headings = extract_toc_headings(text, &TocOptions::default());

        assert_eq!(headings[0].text, "Bold Title");
        assert_eq!(headings[1].text, "Code Section");
    }

    #[test]
    fn test_toc_options_with_min_level() {
        let text = "# Title\n\n## Section\n\n### Subsection";
        let options = TocOptions::default().with_min_level(2);
        let headings = extract_toc_headings(text, &options);

        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "Section");
        assert_eq!(headings[1].text, "Subsection");
    }

    #[test]
    fn test_toc_all_levels() {
        let text = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
        let options = TocOptions::all_levels();
        let headings = extract_toc_headings(text, &options);

        assert_eq!(headings.len(), 6);
    }
}
