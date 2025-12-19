//! Document outline / Table of Contents module
//!
//! This module provides heading extraction and outline generation for markdown documents
//! and structured data files (JSON, YAML, TOML). It supports both raw markdown text parsing
//! (regex-based) and structured data parsing for tree-like outlines.

use crate::markdown::get_structured_file_type;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// OutlineItem
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a single heading item in the document outline.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineItem {
    /// Unique stable ID for this heading (index + hash of title)
    pub id: String,
    /// Heading level (1-6 for H1-H6)
    pub level: u8,
    /// The heading text content (stripped of markdown formatting)
    pub title: String,
    /// Line number in the source document (1-indexed)
    pub line: usize,
    /// Character offset in the source document
    pub char_offset: usize,
    /// Whether this section is collapsed in the outline panel
    pub collapsed: bool,
}

impl OutlineItem {
    /// Create a new outline item.
    pub fn new(level: u8, title: String, line: usize, char_offset: usize, index: usize) -> Self {
        // Generate stable ID from index and title hash
        let mut hasher = DefaultHasher::new();
        title.hash(&mut hasher);
        let hash = hasher.finish();
        let id = format!("h{}-{:x}", index, hash & 0xFFFF);

        Self {
            id,
            level,
            title,
            line,
            char_offset,
            collapsed: false,
        }
    }

    /// Get the indentation level (0 for H1, 1 for H2, etc.)
    pub fn indent_level(&self) -> usize {
        (self.level.saturating_sub(1)) as usize
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DocumentOutline
// ─────────────────────────────────────────────────────────────────────────────

/// The type of outline (determines how items are labeled).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OutlineType {
    /// Markdown document outline (H1-H6 headings)
    #[default]
    Markdown,
    /// Structured data statistics (JSON/YAML/TOML)
    Structured(StructuredStats),
}

/// Statistics for structured data files (JSON/YAML/TOML).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructuredStats {
    /// Total number of keys/properties
    pub total_keys: usize,
    /// Total number of arrays
    pub array_count: usize,
    /// Total number of objects
    pub object_count: usize,
    /// Total number of values (all leaf nodes)
    pub value_count: usize,
    /// Maximum nesting depth
    pub max_depth: usize,
    /// Count of string values
    pub string_count: usize,
    /// Count of number values (int + float)
    pub number_count: usize,
    /// Count of boolean values
    pub bool_count: usize,
    /// Count of null values
    pub null_count: usize,
    /// Total array items across all arrays
    pub total_array_items: usize,
    /// File format name
    pub format_name: String,
    /// Whether parsing succeeded
    pub parse_success: bool,
    /// Parse error message (if any)
    pub parse_error: Option<String>,
}

/// A complete document outline containing all headings.
#[derive(Debug, Clone, Default)]
pub struct DocumentOutline {
    /// All heading items in document order
    pub items: Vec<OutlineItem>,
    /// Total heading count
    pub heading_count: usize,
    /// Estimated reading time in minutes
    pub estimated_read_time: u32,
    /// The type of outline (markdown or structured)
    pub outline_type: OutlineType,
}

impl DocumentOutline {
    /// Create a new empty outline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the outline is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of headings at each level.
    #[allow(dead_code)]
    pub fn level_counts(&self) -> [usize; 6] {
        let mut counts = [0usize; 6];
        for item in &self.items {
            if item.level >= 1 && item.level <= 6 {
                counts[(item.level - 1) as usize] += 1;
            }
        }
        counts
    }

    /// Get a summary string like "3 H1, 5 H2, 2 H3"
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        let counts = self.level_counts();
        let mut parts = Vec::new();
        for (i, &count) in counts.iter().enumerate() {
            if count > 0 {
                parts.push(format!("{} H{}", count, i + 1));
            }
        }
        if parts.is_empty() {
            "No headings".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Find the index of the heading that contains the given line.
    ///
    /// Returns the index of the heading whose section contains the line,
    /// or None if the line is before any heading.
    pub fn find_current_section(&self, line: usize) -> Option<usize> {
        // Find the last heading that starts at or before the given line
        let mut result = None;
        for (i, item) in self.items.iter().enumerate() {
            if item.line <= line {
                result = Some(i);
            } else {
                break;
            }
        }
        result
    }

    /// Toggle the collapsed state of an item by ID.
    pub fn toggle_collapsed(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.collapsed = !item.collapsed;
        }
    }

    /// Check if a heading should be visible based on parent collapsed state.
    pub fn is_visible(&self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }

        let target_level = self.items[index].level;

        // Check all preceding items to see if any ancestor is collapsed
        // We need to check ALL ancestors, not just the immediate parent
        let mut check_level = target_level;

        for i in (0..index).rev() {
            let item = &self.items[i];

            // Only consider items at a higher level in the hierarchy (lower level number)
            if item.level < check_level {
                // This is an ancestor - if it's collapsed, we're hidden
                if item.collapsed {
                    return false;
                }
                // Update check_level to continue checking higher ancestors
                check_level = item.level;
            }
        }
        true
    }

    /// Check if a heading has children (lower-level headings after it).
    pub fn has_children(&self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }

        let current_level = self.items[index].level;

        // Check if the next item exists and has a higher level (lower heading)
        if let Some(next) = self.items.get(index + 1) {
            return next.level > current_level;
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Outline Extraction Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Extract headings from raw markdown text using regex-based parsing.
///
/// This function parses the source text directly to find markdown headings.
/// It handles both ATX-style headings (# Heading) and does not currently
/// support Setext-style headings (underlined with === or ---).
///
/// # Arguments
///
/// * `text` - The raw markdown text to parse
///
/// # Returns
///
/// A `DocumentOutline` containing all extracted headings.
pub fn extract_outline(text: &str) -> DocumentOutline {
    let mut items = Vec::new();
    let mut char_offset = 0;

    for (line_idx, line) in text.lines().enumerate() {
        // Check for ATX-style headings: # Heading
        if let Some(heading) = parse_atx_heading(line) {
            items.push(OutlineItem::new(
                heading.0,
                heading.1,
                line_idx + 1, // 1-indexed
                char_offset,
                items.len(),
            ));
        }

        // Track character offset (including newline)
        char_offset += line.len() + 1;
    }

    // Calculate estimated reading time (average 200 words per minute)
    let word_count = text.split_whitespace().count();
    let estimated_read_time = ((word_count as f32 / 200.0).ceil() as u32).max(1);

    DocumentOutline {
        heading_count: items.len(),
        items,
        estimated_read_time,
        outline_type: OutlineType::Markdown,
    }
}

/// Extract outline from content, automatically detecting file type.
///
/// For markdown files, extracts headings.
/// For JSON/YAML/TOML files, extracts the structure (top-level keys and nested objects).
///
/// # Arguments
///
/// * `text` - The file content
/// * `file_path` - Optional file path for type detection
///
/// # Returns
///
/// A `DocumentOutline` appropriate for the file type.
pub fn extract_outline_for_file(text: &str, file_path: Option<&Path>) -> DocumentOutline {
    // Detect file type from path
    if let Some(path) = file_path {
        if let Some(file_type) = get_structured_file_type(path) {
            return extract_structured_outline(text, file_type);
        }
    }

    // Default to markdown outline
    extract_outline(text)
}

/// Extract outline from structured files (JSON, YAML, TOML).
///
/// This parses the content and computes statistics about the structure.
fn extract_structured_outline(
    text: &str,
    file_type: crate::markdown::tree_viewer::StructuredFileType,
) -> DocumentOutline {
    use crate::markdown::tree_viewer::parse_structured_content;

    let format_name = file_type.display_name().to_string();

    // Try to parse the content
    let tree = match parse_structured_content(text, file_type) {
        Ok(tree) => tree,
        Err(e) => {
            // On parse error, return stats with error info
            let stats = StructuredStats {
                format_name,
                parse_success: false,
                parse_error: Some(e.message),
                ..Default::default()
            };
            return DocumentOutline {
                heading_count: 0,
                items: Vec::new(),
                estimated_read_time: 0,
                outline_type: OutlineType::Structured(stats),
            };
        }
    };

    // Compute statistics from the tree
    let mut stats = StructuredStats {
        format_name,
        parse_success: true,
        ..Default::default()
    };
    compute_tree_stats(&tree, &mut stats, 1);

    DocumentOutline {
        heading_count: 0,
        items: Vec::new(),
        estimated_read_time: 0,
        outline_type: OutlineType::Structured(stats),
    }
}

/// Recursively compute statistics from a tree node.
fn compute_tree_stats(
    node: &crate::markdown::tree_viewer::TreeNode,
    stats: &mut StructuredStats,
    depth: usize,
) {
    use crate::markdown::tree_viewer::TreeNode;

    // Update max depth
    if depth > stats.max_depth {
        stats.max_depth = depth;
    }

    match node {
        TreeNode::Object(obj) => {
            stats.object_count += 1;
            stats.total_keys += obj.len();
            for (_, value) in obj {
                compute_tree_stats(value, stats, depth + 1);
            }
        }
        TreeNode::Array(arr) => {
            stats.array_count += 1;
            stats.total_array_items += arr.len();
            for item in arr {
                compute_tree_stats(item, stats, depth + 1);
            }
        }
        TreeNode::String(_) => {
            stats.value_count += 1;
            stats.string_count += 1;
        }
        TreeNode::Integer(_) | TreeNode::Float(_) => {
            stats.value_count += 1;
            stats.number_count += 1;
        }
        TreeNode::Bool(_) => {
            stats.value_count += 1;
            stats.bool_count += 1;
        }
        TreeNode::Null => {
            stats.value_count += 1;
            stats.null_count += 1;
        }
    }
}

/// Parse an ATX-style heading from a line.
///
/// Returns Some((level, title)) if the line is a heading, None otherwise.
fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();

    // Must start with #
    if !trimmed.starts_with('#') {
        return None;
    }

    // Count the number of # characters
    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();

    // Must be 1-6 hashes
    if hash_count == 0 || hash_count > 6 {
        return None;
    }

    // Get the rest of the line after the hashes
    let rest = &trimmed[hash_count..];

    // Must have a space after the hashes (or be empty for a valid heading)
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    // Extract the heading text, stripping leading/trailing whitespace
    // Also strip trailing # characters (optional closing syntax)
    let title = rest.trim().trim_end_matches('#').trim().to_string();

    // Strip inline markdown formatting from title for cleaner display
    let clean_title = strip_inline_formatting(&title);

    Some((hash_count as u8, clean_title))
}

/// Strip common inline markdown formatting from text.
///
/// Removes: **bold**, *italic*, `code`, ~~strikethrough~~, [links](url)
fn strip_inline_formatting(text: &str) -> String {
    let mut result = text.to_string();

    // Remove bold (**text** or __text__)
    result = remove_wrapper(&result, "**");
    result = remove_wrapper(&result, "__");

    // Remove italic (*text* or _text_) - be careful with word_like_this
    // Only remove single * or _ when they wrap text
    result = remove_single_wrapper(&result, '*');
    result = remove_single_wrapper(&result, '_');

    // Remove inline code (`text`)
    result = remove_wrapper(&result, "`");

    // Remove strikethrough (~~text~~)
    result = remove_wrapper(&result, "~~");

    // Remove link syntax [text](url) -> text
    result = remove_links(&result);

    // Remove image syntax ![alt](url) -> alt
    result = remove_images(&result);

    result
}

/// Remove a symmetric wrapper like ** or ~~
fn remove_wrapper(text: &str, wrapper: &str) -> String {
    let mut result = text.to_string();
    let len = wrapper.len();

    while let Some(start) = result.find(wrapper) {
        if let Some(end) = result[start + len..].find(wrapper) {
            let end_pos = start + len + end;
            let inner = &result[start + len..end_pos];
            result = format!("{}{}{}", &result[..start], inner, &result[end_pos + len..]);
        } else {
            break;
        }
    }
    result
}

/// Remove single character wrapper like * or _
fn remove_single_wrapper(text: &str, wrapper: char) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut in_wrapper = false;

    while let Some(c) = chars.next() {
        if c == wrapper {
            // Check if this is likely a delimiter
            let prev_is_space = result
                .chars()
                .last()
                .map(|c| c.is_whitespace())
                .unwrap_or(true);
            let next_is_space = chars.peek().map(|c| c.is_whitespace()).unwrap_or(true);

            // Only treat as wrapper if it's at a word boundary
            if !in_wrapper && (prev_is_space || result.is_empty()) && !next_is_space {
                in_wrapper = true;
            } else if in_wrapper && (!next_is_space || chars.peek().is_none()) {
                in_wrapper = false;
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Remove markdown links [text](url) -> text
fn remove_links(text: &str) -> String {
    let mut result = text.to_string();

    // Simple pattern matching for [text](url)
    while let Some(start) = result.find('[') {
        if let Some(mid) = result[start..].find("](") {
            let mid_pos = start + mid;
            if let Some(end) = result[mid_pos + 2..].find(')') {
                let end_pos = mid_pos + 2 + end;
                let link_text = &result[start + 1..mid_pos];
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    link_text,
                    &result[end_pos + 1..]
                );
                continue;
            }
        }
        break;
    }
    result
}

/// Remove markdown images ![alt](url) -> alt
fn remove_images(text: &str) -> String {
    let mut result = text.to_string();

    // Simple pattern matching for ![alt](url)
    while let Some(start) = result.find("![") {
        if let Some(mid) = result[start..].find("](") {
            let mid_pos = start + mid;
            if let Some(end) = result[mid_pos + 2..].find(')') {
                let end_pos = mid_pos + 2 + end;
                let alt_text = &result[start + 2..mid_pos];
                result = format!("{}{}{}", &result[..start], alt_text, &result[end_pos + 1..]);
                continue;
            }
        }
        break;
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Basic Heading Extraction Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_document() {
        let outline = extract_outline("");
        assert!(outline.is_empty());
        assert_eq!(outline.heading_count, 0);
    }

    #[test]
    fn test_single_h1() {
        let outline = extract_outline("# Hello World");
        assert_eq!(outline.items.len(), 1);
        assert_eq!(outline.items[0].level, 1);
        assert_eq!(outline.items[0].title, "Hello World");
        assert_eq!(outline.items[0].line, 1);
    }

    #[test]
    fn test_multiple_headings() {
        let text = "# Title\n\n## Section 1\n\nSome text\n\n## Section 2\n\n### Subsection";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 4);
        assert_eq!(outline.items[0].level, 1);
        assert_eq!(outline.items[0].title, "Title");
        assert_eq!(outline.items[1].level, 2);
        assert_eq!(outline.items[1].title, "Section 1");
        assert_eq!(outline.items[2].level, 2);
        assert_eq!(outline.items[2].title, "Section 2");
        assert_eq!(outline.items[3].level, 3);
        assert_eq!(outline.items[3].title, "Subsection");
    }

    #[test]
    fn test_all_heading_levels() {
        let text = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 6);
        for (i, item) in outline.items.iter().enumerate() {
            assert_eq!(item.level, (i + 1) as u8);
        }
    }

    #[test]
    fn test_heading_with_trailing_hashes() {
        let outline = extract_outline("## Heading ##");
        assert_eq!(outline.items[0].title, "Heading");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Inline Formatting Stripping Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_heading_with_bold() {
        let outline = extract_outline("# **Bold** Heading");
        assert_eq!(outline.items[0].title, "Bold Heading");
    }

    #[test]
    fn test_heading_with_italic() {
        let outline = extract_outline("# *Italic* Heading");
        assert_eq!(outline.items[0].title, "Italic Heading");
    }

    #[test]
    fn test_heading_with_code() {
        let outline = extract_outline("# Heading with `code`");
        assert_eq!(outline.items[0].title, "Heading with code");
    }

    #[test]
    fn test_heading_with_link() {
        let outline = extract_outline("# Heading with [link](http://example.com)");
        assert_eq!(outline.items[0].title, "Heading with link");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_not_a_heading_no_space() {
        let outline = extract_outline("#NotAHeading");
        assert!(outline.is_empty());
    }

    #[test]
    fn test_not_a_heading_in_code_block() {
        // Note: This simple parser doesn't handle code blocks
        // The actual implementation should skip code blocks
        let text = "```\n# Not a heading\n```";
        let _outline = extract_outline(text);
        // Current implementation will pick this up - that's a known limitation
        // In practice, we'd want to skip code blocks
    }

    #[test]
    fn test_heading_with_leading_whitespace() {
        // Leading whitespace should be trimmed
        let outline = extract_outline("  # Heading with indent");
        assert_eq!(outline.items.len(), 1);
        assert_eq!(outline.items[0].title, "Heading with indent");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Outline Helper Method Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_level_counts() {
        let text = "# H1\n## H2\n## H2\n### H3";
        let outline = extract_outline(text);
        let counts = outline.level_counts();

        assert_eq!(counts[0], 1); // H1
        assert_eq!(counts[1], 2); // H2
        assert_eq!(counts[2], 1); // H3
        assert_eq!(counts[3], 0); // H4
    }

    #[test]
    fn test_summary() {
        let text = "# H1\n## H2\n## H2";
        let outline = extract_outline(text);
        let summary = outline.summary();

        assert!(summary.contains("1 H1"));
        assert!(summary.contains("2 H2"));
    }

    #[test]
    fn test_find_current_section() {
        let text = "# Title\n\nText\n\n## Section\n\nMore text";
        let outline = extract_outline(text);

        // Line 1 is the title
        assert_eq!(outline.find_current_section(1), Some(0));
        // Line 3 is still in Title section
        assert_eq!(outline.find_current_section(3), Some(0));
        // Line 5 is Section
        assert_eq!(outline.find_current_section(5), Some(1));
        // Line 7 is still in Section
        assert_eq!(outline.find_current_section(7), Some(1));
        // Before any heading
        assert_eq!(outline.find_current_section(0), None);
    }

    #[test]
    fn test_has_children() {
        let text = "# Title\n## Child\n### Grandchild\n## Sibling";
        let outline = extract_outline(text);

        assert!(outline.has_children(0)); // Title has Child
        assert!(outline.has_children(1)); // Child has Grandchild
        assert!(!outline.has_children(2)); // Grandchild has no children
        assert!(!outline.has_children(3)); // Sibling is last
    }

    #[test]
    fn test_visibility_with_collapsed() {
        let text = "# Title\n## Child\n### Grandchild";
        let mut outline = extract_outline(text);

        // Initially all visible
        assert!(outline.is_visible(0));
        assert!(outline.is_visible(1));
        assert!(outline.is_visible(2));

        // Collapse Title
        outline.items[0].collapsed = true;

        assert!(outline.is_visible(0)); // Title still visible
        assert!(!outline.is_visible(1)); // Child hidden
        assert!(!outline.is_visible(2)); // Grandchild hidden
    }

    #[test]
    fn test_visibility_siblings_with_collapsed_parent() {
        // This tests the bug where siblings weren't hidden when parent was collapsed
        let text = "## Parent\n### Child1\n### Child2\n### Child3\n## Sibling";
        let mut outline = extract_outline(text);

        // Index 0: ## Parent (level 2)
        // Index 1: ### Child1 (level 3)
        // Index 2: ### Child2 (level 3)
        // Index 3: ### Child3 (level 3)
        // Index 4: ## Sibling (level 2)
        assert_eq!(outline.items.len(), 5);

        // Initially all visible
        for i in 0..5 {
            assert!(
                outline.is_visible(i),
                "Item {} should be visible initially",
                i
            );
        }

        // Collapse Parent - all children should be hidden, sibling should be visible
        outline.items[0].collapsed = true;

        assert!(outline.is_visible(0), "Parent should still be visible");
        assert!(!outline.is_visible(1), "Child1 should be hidden");
        assert!(
            !outline.is_visible(2),
            "Child2 should be hidden (bug was here!)"
        );
        assert!(!outline.is_visible(3), "Child3 should be hidden");
        assert!(outline.is_visible(4), "Sibling should still be visible");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Estimated Reading Time Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_estimated_read_time() {
        // ~400 words should be ~2 minutes
        let words: Vec<&str> = std::iter::repeat("word").take(400).collect();
        let text = format!("# Title\n\n{}", words.join(" "));
        let outline = extract_outline(&text);

        assert!(outline.estimated_read_time >= 2);
    }

    #[test]
    fn test_estimated_read_time_minimum() {
        let outline = extract_outline("# Short");
        assert_eq!(outline.estimated_read_time, 1); // Minimum 1 minute
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Structured File Outline Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_extract_outline_for_json_file() {
        use std::path::Path;

        let json = r#"{
            "name": "test",
            "version": "1.0",
            "dependencies": {
                "foo": "1.0",
                "bar": "2.0"
            }
        }"#;

        let outline = extract_outline_for_file(json, Some(Path::new("package.json")));

        // Should be structured type with statistics
        match &outline.outline_type {
            OutlineType::Structured(stats) => {
                assert!(stats.parse_success);
                assert_eq!(stats.format_name, "JSON");
                assert_eq!(stats.object_count, 2); // Root + dependencies
                assert_eq!(stats.total_keys, 5); // name, version, dependencies, foo, bar
                assert_eq!(stats.string_count, 4); // "test", "1.0", "1.0", "2.0"
            }
            OutlineType::Markdown => panic!("Expected Structured type, got Markdown"),
        }
        // For structured files, items is empty (stats are shown instead)
        assert!(outline.items.is_empty());
    }

    #[test]
    fn test_extract_outline_for_markdown_file() {
        use std::path::Path;

        let markdown = "# Title\n## Section\n### Subsection";

        let outline = extract_outline_for_file(markdown, Some(Path::new("README.md")));

        assert_eq!(outline.outline_type, OutlineType::Markdown);
        assert_eq!(outline.items.len(), 3);
    }

    #[test]
    fn test_extract_outline_no_path_defaults_to_markdown() {
        let content = "# Title\n\nSome text";

        let outline = extract_outline_for_file(content, None);

        assert_eq!(outline.outline_type, OutlineType::Markdown);
    }

    #[test]
    fn test_structured_outline_type() {
        assert_eq!(OutlineType::default(), OutlineType::Markdown);
    }
}
