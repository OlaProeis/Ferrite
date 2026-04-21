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
// ContentType
// ─────────────────────────────────────────────────────────────────────────────

/// The type of content represented by an outline item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// A markdown heading with level 1-6
    Heading(u8),
    /// A generic fenced code block (```)
    CodeBlock,
    /// A Mermaid diagram block (```mermaid)
    MermaidDiagram,
    /// A markdown table (lines with | separators)
    Table,
    /// An image (![alt](url))
    Image,
    /// A blockquote section (> text)
    Blockquote,
}

impl ContentType {
    /// Get a short label for display (e.g., "H1", "CODE", "TABLE")
    #[allow(dead_code)] // Public API for outline display customization
    pub fn label(&self) -> &'static str {
        match self {
            ContentType::Heading(1) => "H1",
            ContentType::Heading(2) => "H2",
            ContentType::Heading(3) => "H3",
            ContentType::Heading(4) => "H4",
            ContentType::Heading(5) => "H5",
            ContentType::Heading(_) => "H6",
            ContentType::CodeBlock => "</>",
            ContentType::MermaidDiagram => "◇",
            ContentType::Table => "⊞",
            ContentType::Image => "▣",
            ContentType::Blockquote => "❝",
        }
    }

    /// Get the heading level if this is a heading, otherwise None
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            ContentType::Heading(level) => Some(*level),
            _ => None,
        }
    }

    /// Check if this is a heading type
    pub fn is_heading(&self) -> bool {
        matches!(self, ContentType::Heading(_))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OutlineItem
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a single item in the document outline.
///
/// Can be a heading (H1-H6) or a content block (code, Mermaid, table, image, blockquote).
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineItem {
    /// Unique stable ID for this item (index + hash of title)
    pub id: String,
    /// The type of content this item represents
    pub content_type: ContentType,
    /// Heading level (1-6 for H1-H6, 0 for non-headings)
    /// Kept for backwards compatibility - use content_type for new code
    pub level: u8,
    /// The display text (heading text or content description)
    pub title: String,
    /// Line number in the source document (1-indexed)
    pub line: usize,
    /// Character offset in the source document
    pub char_offset: usize,
    /// Whether this section is collapsed in the outline panel
    pub collapsed: bool,
}

impl OutlineItem {
    /// Create a new heading outline item.
    pub fn new(level: u8, title: String, line: usize, char_offset: usize, index: usize) -> Self {
        // Generate stable ID from index and title hash
        let mut hasher = DefaultHasher::new();
        title.hash(&mut hasher);
        let hash = hasher.finish();
        let id = format!("h{}-{:x}", index, hash & 0xFFFF);

        Self {
            id,
            content_type: ContentType::Heading(level),
            level,
            title,
            line,
            char_offset,
            collapsed: false,
        }
    }

    /// Create a new content block outline item.
    pub fn new_content(
        content_type: ContentType,
        title: String,
        line: usize,
        char_offset: usize,
        index: usize,
    ) -> Self {
        // Generate stable ID from index, content type, and title hash
        let mut hasher = DefaultHasher::new();
        title.hash(&mut hasher);
        let type_prefix = match content_type {
            ContentType::Heading(l) => format!("h{}", l),
            ContentType::CodeBlock => "code".to_string(),
            ContentType::MermaidDiagram => "mermaid".to_string(),
            ContentType::Table => "table".to_string(),
            ContentType::Image => "img".to_string(),
            ContentType::Blockquote => "quote".to_string(),
        };
        let hash = hasher.finish();
        let id = format!("{}{}-{:x}", type_prefix, index, hash & 0xFFFF);

        // For backwards compatibility, set level based on content type
        let level = match content_type {
            ContentType::Heading(l) => l,
            _ => 0, // Non-headings have level 0
        };

        Self {
            id,
            content_type,
            level,
            title,
            line,
            char_offset,
            collapsed: false,
        }
    }

    /// Get the indentation level (0 for H1, 1 for H2, etc.)
    /// Content blocks always return 0 (no indentation).
    pub fn indent_level(&self) -> usize {
        match self.content_type {
            ContentType::Heading(level) => (level.saturating_sub(1)) as usize,
            _ => 0,
        }
    }

    /// Check if this item is a heading
    pub fn is_heading(&self) -> bool {
        self.content_type.is_heading()
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

/// Extract headings and content blocks from raw markdown text.
///
/// This function parses the source text directly to find:
/// - ATX-style headings (# Heading)
/// - Fenced code blocks (```) and Mermaid diagrams (```mermaid)
/// - Markdown tables (lines with | separators)
/// - Images (![alt](url))
/// - Blockquotes (> text)
///
/// # Arguments
///
/// * `text` - The raw markdown text to parse
///
/// # Returns
///
/// A `DocumentOutline` containing all extracted items.
pub fn extract_outline(text: &str) -> DocumentOutline {
    let mut items = Vec::new();

    // Build a map of line number (0-indexed) to character offset
    // This is more accurate than reconstructing offsets from lines() iteration
    let line_offsets: Vec<usize> = {
        let mut offsets = vec![0]; // Line 0 starts at char 0
        for (i, ch) in text.chars().enumerate() {
            if ch == '\n' {
                offsets.push(i + 1); // Next line starts after the \n
            }
        }
        offsets
    };

    // State tracking
    let mut in_code_block = false;
    let mut code_block_start_line: Option<usize> = None;
    let mut code_block_start_offset: Option<usize> = None;
    let mut code_block_is_mermaid = false;
    let mut in_blockquote = false;
    let mut blockquote_start_line: Option<usize> = None;
    let mut blockquote_start_offset: Option<usize> = None;
    let mut in_table = false;
    let mut table_start_line: Option<usize> = None;
    let mut table_start_offset: Option<usize> = None;

    for (line_idx, line) in text.lines().enumerate() {
        let line_num = line_idx + 1; // 1-indexed
        let trimmed = line.trim();

        // Get the character offset for this line from our pre-built map
        let char_offset = line_offsets.get(line_idx).copied().unwrap_or(0);

        // Handle code block boundaries
        if trimmed.starts_with("```") {
            if !in_code_block {
                // Starting a code block
                in_code_block = true;
                code_block_start_line = Some(line_num);
                code_block_start_offset = Some(char_offset);

                // Check if it's a Mermaid diagram
                let lang = trimmed.trim_start_matches('`').trim();
                code_block_is_mermaid = lang.eq_ignore_ascii_case("mermaid");
            } else {
                // Ending a code block - add the item
                if let (Some(start_line), Some(start_offset)) =
                    (code_block_start_line, code_block_start_offset)
                {
                    let content_type = if code_block_is_mermaid {
                        ContentType::MermaidDiagram
                    } else {
                        ContentType::CodeBlock
                    };

                    // Generate a title from the code block
                    let title = if code_block_is_mermaid {
                        "Mermaid diagram".to_string()
                    } else {
                        format!("Code block (line {})", start_line)
                    };

                    items.push(OutlineItem::new_content(
                        content_type,
                        title,
                        start_line,
                        start_offset,
                        items.len(),
                    ));
                }

                in_code_block = false;
                code_block_start_line = None;
                code_block_start_offset = None;
                code_block_is_mermaid = false;
            }
        } else if !in_code_block {
            // Only process other content when not in a code block

            // Check for ATX-style headings: # Heading
            if let Some(heading) = parse_atx_heading(line) {
                // End any active blockquote or table
                finalize_blockquote(
                    &mut items,
                    &mut in_blockquote,
                    &mut blockquote_start_line,
                    &mut blockquote_start_offset,
                );
                finalize_table(
                    &mut items,
                    &mut in_table,
                    &mut table_start_line,
                    &mut table_start_offset,
                );

                items.push(OutlineItem::new(
                    heading.0,
                    heading.1,
                    line_num,
                    char_offset,
                    items.len(),
                ));
            }
            // Check for images: ![alt](url)
            else if let Some(image_title) = parse_image(trimmed) {
                // End any active blockquote or table
                finalize_blockquote(
                    &mut items,
                    &mut in_blockquote,
                    &mut blockquote_start_line,
                    &mut blockquote_start_offset,
                );
                finalize_table(
                    &mut items,
                    &mut in_table,
                    &mut table_start_line,
                    &mut table_start_offset,
                );

                items.push(OutlineItem::new_content(
                    ContentType::Image,
                    image_title,
                    line_num,
                    char_offset,
                    items.len(),
                ));
            }
            // Check for tables: lines starting with |
            else if is_table_line(trimmed) {
                if !in_table {
                    // Start of a new table
                    in_table = true;
                    table_start_line = Some(line_num);
                    table_start_offset = Some(char_offset);
                }
                // If already in table, continue
            }
            // Check for blockquotes: > text
            else if trimmed.starts_with('>') {
                // End any active table
                finalize_table(
                    &mut items,
                    &mut in_table,
                    &mut table_start_line,
                    &mut table_start_offset,
                );

                if !in_blockquote {
                    // Start of a new blockquote
                    in_blockquote = true;
                    blockquote_start_line = Some(line_num);
                    blockquote_start_offset = Some(char_offset);
                }
                // If already in blockquote, continue
            }
            // Handle end of continuous content blocks
            else {
                // End active blockquote if we're not in one
                if in_blockquote && !trimmed.is_empty() {
                    finalize_blockquote(
                        &mut items,
                        &mut in_blockquote,
                        &mut blockquote_start_line,
                        &mut blockquote_start_offset,
                    );
                }
                // End active table if we're not in one
                if in_table {
                    finalize_table(
                        &mut items,
                        &mut in_table,
                        &mut table_start_line,
                        &mut table_start_offset,
                    );
                }
            }
        }

        // Note: char_offset is now looked up from line_offsets map at loop start,
        // so no manual tracking needed here.
    }

    // Finalize any remaining content blocks at end of document
    finalize_blockquote(
        &mut items,
        &mut in_blockquote,
        &mut blockquote_start_line,
        &mut blockquote_start_offset,
    );
    finalize_table(
        &mut items,
        &mut in_table,
        &mut table_start_line,
        &mut table_start_offset,
    );

    // Count only headings for heading_count
    let heading_count = items.iter().filter(|i| i.is_heading()).count();

    // Calculate estimated reading time (average 200 words per minute)
    let word_count = text.split_whitespace().count();
    let estimated_read_time = ((word_count as f32 / 200.0).ceil() as u32).max(1);

    DocumentOutline {
        heading_count,
        items,
        estimated_read_time,
        outline_type: OutlineType::Markdown,
    }
}

/// Helper to finalize a blockquote and add it to items
fn finalize_blockquote(
    items: &mut Vec<OutlineItem>,
    in_blockquote: &mut bool,
    start_line: &mut Option<usize>,
    start_offset: &mut Option<usize>,
) {
    if *in_blockquote {
        if let (Some(line), Some(offset)) = (*start_line, *start_offset) {
            items.push(OutlineItem::new_content(
                ContentType::Blockquote,
                format!("Blockquote (line {})", line),
                line,
                offset,
                items.len(),
            ));
        }
        *in_blockquote = false;
        *start_line = None;
        *start_offset = None;
    }
}

/// Helper to finalize a table and add it to items
fn finalize_table(
    items: &mut Vec<OutlineItem>,
    in_table: &mut bool,
    start_line: &mut Option<usize>,
    start_offset: &mut Option<usize>,
) {
    if *in_table {
        if let (Some(line), Some(offset)) = (*start_line, *start_offset) {
            items.push(OutlineItem::new_content(
                ContentType::Table,
                format!("Table (line {})", line),
                line,
                offset,
                items.len(),
            ));
        }
        *in_table = false;
        *start_line = None;
        *start_offset = None;
    }
}

/// Check if a line is part of a markdown table
fn is_table_line(line: &str) -> bool {
    // A table line starts with | and contains at least one more |
    if line.starts_with('|') {
        return line.matches('|').count() >= 2;
    }
    // Also check for tables without leading |, like: col1 | col2
    if line.contains('|') && !line.starts_with('>') {
        let parts: Vec<&str> = line.split('|').collect();
        return parts.len() >= 2 && parts.iter().all(|p| !p.trim().is_empty() || p.is_empty());
    }
    false
}

/// Parse an image from a line, returning the alt text or URL as title
fn parse_image(line: &str) -> Option<String> {
    // Look for ![alt](url) pattern
    if let Some(start) = line.find("![") {
        if let Some(mid) = line[start..].find("](") {
            let mid_pos = start + mid;
            if let Some(end) = line[mid_pos + 2..].find(')') {
                let alt_text = &line[start + 2..mid_pos];
                let url = &line[mid_pos + 2..mid_pos + 2 + end];

                // Use alt text if available, otherwise use filename from URL
                let title = if !alt_text.is_empty() {
                    alt_text.to_string()
                } else {
                    // Extract filename from URL
                    url.rsplit('/').next().unwrap_or("Image").to_string()
                };

                return Some(title);
            }
        }
    }
    None
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
            } else if in_wrapper && (next_is_space || chars.peek().is_none()) {
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
        // Headings inside code blocks should NOT be extracted
        let text = "```\n# Not a heading\n```";
        let outline = extract_outline(text);

        // Should only have the code block, not the fake heading inside
        assert_eq!(outline.heading_count, 0, "Should have no headings");
        assert_eq!(outline.items.len(), 1, "Should have 1 code block");
        assert!(matches!(
            outline.items[0].content_type,
            ContentType::CodeBlock
        ));
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

    // ─────────────────────────────────────────────────────────────────────────
    // Content Type Extraction Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_code_block_extraction() {
        let text = "# Title\n\n```rust\nfn main() {}\n```\n\nSome text";
        let outline = extract_outline(text);

        // Should have 2 items: heading and code block
        assert_eq!(outline.items.len(), 2);
        assert_eq!(outline.heading_count, 1);

        // Check heading
        assert!(matches!(
            outline.items[0].content_type,
            ContentType::Heading(1)
        ));

        // Check code block
        assert!(matches!(
            outline.items[1].content_type,
            ContentType::CodeBlock
        ));
        assert_eq!(outline.items[1].line, 3); // Line where code block starts
    }

    #[test]
    fn test_mermaid_diagram_extraction() {
        let text = "# Flowchart\n\n```mermaid\nflowchart TD\n    A --> B\n```";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 2);

        // Check mermaid diagram
        assert!(matches!(
            outline.items[1].content_type,
            ContentType::MermaidDiagram
        ));
        assert!(outline.items[1].title.contains("Mermaid"));
    }

    #[test]
    fn test_table_extraction() {
        let text = "# Data\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |\n\nText after";
        let outline = extract_outline(text);

        // Should have heading and table
        assert_eq!(outline.items.len(), 2);
        assert!(matches!(outline.items[1].content_type, ContentType::Table));
    }

    #[test]
    fn test_image_extraction() {
        let text = "# Images\n\n![My Image](path/to/image.png)\n\nMore text";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 2);
        assert!(matches!(outline.items[1].content_type, ContentType::Image));
        assert_eq!(outline.items[1].title, "My Image");
    }

    #[test]
    fn test_image_extraction_no_alt_text() {
        let text = "![](path/to/logo.png)";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 1);
        assert!(matches!(outline.items[0].content_type, ContentType::Image));
        assert_eq!(outline.items[0].title, "logo.png"); // Uses filename
    }

    #[test]
    fn test_blockquote_extraction() {
        let text = "# Quote Section\n\n> This is a quote\n> Spanning multiple lines\n\nNormal text";
        let outline = extract_outline(text);

        assert_eq!(outline.items.len(), 2);
        assert!(matches!(
            outline.items[1].content_type,
            ContentType::Blockquote
        ));
    }

    #[test]
    fn test_mixed_content_types() {
        let text = r#"# Document

This is some intro text.

## Code Example

```python
print("Hello")
```

## Diagram

```mermaid
graph TD
    A --> B
```

## Data Table

| Name | Value |
|------|-------|
| foo  | 123   |

## Important Note

> This is a blockquote
> with multiple lines

## Image

![Screenshot](./screenshot.png)

The end."#;

        let outline = extract_outline(text);

        // Count by type
        let headings: Vec<_> = outline
            .items
            .iter()
            .filter(|i| i.content_type.is_heading())
            .collect();
        let code_blocks: Vec<_> = outline
            .items
            .iter()
            .filter(|i| matches!(i.content_type, ContentType::CodeBlock))
            .collect();
        let mermaid: Vec<_> = outline
            .items
            .iter()
            .filter(|i| matches!(i.content_type, ContentType::MermaidDiagram))
            .collect();
        let tables: Vec<_> = outline
            .items
            .iter()
            .filter(|i| matches!(i.content_type, ContentType::Table))
            .collect();
        let blockquotes: Vec<_> = outline
            .items
            .iter()
            .filter(|i| matches!(i.content_type, ContentType::Blockquote))
            .collect();
        let images: Vec<_> = outline
            .items
            .iter()
            .filter(|i| matches!(i.content_type, ContentType::Image))
            .collect();

        assert_eq!(headings.len(), 6, "Should have 6 headings");
        assert_eq!(code_blocks.len(), 1, "Should have 1 code block");
        assert_eq!(mermaid.len(), 1, "Should have 1 mermaid diagram");
        assert_eq!(tables.len(), 1, "Should have 1 table");
        assert_eq!(blockquotes.len(), 1, "Should have 1 blockquote");
        assert_eq!(images.len(), 1, "Should have 1 image");

        // Verify heading_count only counts headings
        assert_eq!(outline.heading_count, 6);
    }

    #[test]
    fn test_content_inside_code_block_ignored() {
        // Content inside code blocks should NOT be extracted
        let text = r#"```markdown
# This is not a heading
| not | a | table |
> not a quote
![not an image](url)
```"#;

        let outline = extract_outline(text);

        // Should only have the code block itself, nothing inside it
        assert_eq!(outline.items.len(), 1);
        assert!(matches!(
            outline.items[0].content_type,
            ContentType::CodeBlock
        ));
    }

    #[test]
    fn test_content_type_label() {
        assert_eq!(ContentType::Heading(1).label(), "H1");
        assert_eq!(ContentType::Heading(2).label(), "H2");
        assert_eq!(ContentType::CodeBlock.label(), "</>");
        assert_eq!(ContentType::MermaidDiagram.label(), "◇");
        assert_eq!(ContentType::Table.label(), "⊞");
        assert_eq!(ContentType::Image.label(), "▣");
        assert_eq!(ContentType::Blockquote.label(), "❝");
    }

    #[test]
    fn test_content_type_heading_level() {
        assert_eq!(ContentType::Heading(1).heading_level(), Some(1));
        assert_eq!(ContentType::Heading(3).heading_level(), Some(3));
        assert_eq!(ContentType::CodeBlock.heading_level(), None);
        assert_eq!(ContentType::Table.heading_level(), None);
    }

    #[test]
    fn test_content_type_is_heading() {
        assert!(ContentType::Heading(1).is_heading());
        assert!(ContentType::Heading(6).is_heading());
        assert!(!ContentType::CodeBlock.is_heading());
        assert!(!ContentType::MermaidDiagram.is_heading());
        assert!(!ContentType::Table.is_heading());
        assert!(!ContentType::Image.is_heading());
        assert!(!ContentType::Blockquote.is_heading());
    }

    #[test]
    fn test_is_table_line() {
        // Valid table lines
        assert!(is_table_line("| A | B |"));
        assert!(is_table_line("|---|---|"));
        assert!(is_table_line("| cell |  |"));

        // Not table lines
        assert!(!is_table_line("regular text"));
        assert!(!is_table_line("> quote | with pipe")); // Blockquote takes precedence
        assert!(!is_table_line("# Heading"));
    }

    #[test]
    fn test_parse_image() {
        // With alt text
        assert_eq!(
            parse_image("![alt text](url.png)"),
            Some("alt text".to_string())
        );

        // Without alt text - uses filename
        assert_eq!(
            parse_image("![](path/to/image.png)"),
            Some("image.png".to_string())
        );

        // Not an image
        assert_eq!(parse_image("regular text"), None);
        assert_eq!(parse_image("[link](url)"), None); // Link, not image
    }
}
