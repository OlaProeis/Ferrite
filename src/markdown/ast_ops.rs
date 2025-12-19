//! AST Operations for WYSIWYG Editor
//!
//! This module provides structural operations on the markdown AST for word processor-like
//! editing behaviors in WYSIWYG mode.
//!
//! # Operations
//! - **Paragraph splitting**: Split a paragraph at cursor position into two
//! - **List item operations**: Split, merge, indent, outdent list items
//! - **Heading operations**: Convert heading to paragraph, split heading text
//!
//! # Design
//! Each operation takes the current markdown source and cursor context, then returns
//! a `StructuralEdit` describing the changes to apply.

// Allow dead code - this module contains complete API for WYSIWYG editing operations
// including enum variants and fields for future keyboard interaction features
// - needless_range_loop: Index loops are clearer for line-by-line source reconstruction
// - clone_on_copy: Sometimes clone is used for explicit intent
// - unnecessary_map_or: map_or can be clearer than is_some_and for some patterns
#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::unnecessary_map_or)]

use crate::markdown::parser::{HeadingLevel, ListType};

// ─────────────────────────────────────────────────────────────────────────────
// Structural Edit Result
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a structural edit to the markdown document.
/// Used to communicate changes from key handlers back to the editor.
#[derive(Debug, Clone)]
pub struct StructuralEdit {
    /// The new markdown source after the edit
    pub new_source: String,
    /// The cursor position after the edit (line, column within the widget)
    pub cursor_position: CursorPosition,
    /// Whether the edit was performed
    pub performed: bool,
}

impl StructuralEdit {
    /// Create a no-op edit (nothing changed)
    pub fn no_op() -> Self {
        Self {
            new_source: String::new(),
            performed: false,
            cursor_position: CursorPosition::default(),
        }
    }

    /// Create a successful edit with new source and cursor position
    pub fn success(new_source: String, cursor: CursorPosition) -> Self {
        Self {
            new_source,
            cursor_position: cursor,
            performed: true,
        }
    }
}

/// Cursor position within the document after an edit
#[derive(Debug, Clone, Default)]
pub struct CursorPosition {
    /// Line number (1-indexed, matching AST conventions)
    pub line: usize,
    /// Character offset within the editable content (0-indexed)
    pub offset: usize,
    /// Optional: node ID hint for focusing the right widget
    pub node_hint: Option<NodeHint>,
}

impl CursorPosition {
    pub fn new(line: usize, offset: usize) -> Self {
        Self {
            line,
            offset,
            node_hint: None,
        }
    }

    pub fn with_hint(mut self, hint: NodeHint) -> Self {
        self.node_hint = Some(hint);
        self
    }
}

/// Hint about which node type the cursor should be in after an edit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeHint {
    Paragraph,
    Heading(HeadingLevel),
    ListItem { list_type: ListType, index: usize },
}

// ─────────────────────────────────────────────────────────────────────────────
// Edit Context
// ─────────────────────────────────────────────────────────────────────────────

/// Context about where an edit is occurring in the document
#[derive(Debug, Clone)]
pub struct EditContext {
    /// Current node type being edited
    pub node_type: EditNodeType,
    /// Line number of the current node (1-indexed)
    pub start_line: usize,
    /// End line of the current node (1-indexed)
    pub end_line: usize,
    /// Cursor offset within the editable text (0-indexed)
    pub cursor_offset: usize,
    /// The text content being edited
    pub text: String,
    /// For list items: the list type
    pub list_type: Option<ListType>,
    /// For list items: the item index within the list (0-indexed)
    pub list_item_index: Option<usize>,
    /// For nested lists: the nesting depth (0 = top level)
    pub nesting_depth: usize,
}

/// Type of node being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditNodeType {
    Paragraph,
    Heading(HeadingLevel),
    ListItem,
    CodeBlock,
    BlockQuote,
    TableCell,
}

// ─────────────────────────────────────────────────────────────────────────────
// Paragraph Operations
// ─────────────────────────────────────────────────────────────────────────────

/// Split a paragraph at the cursor position into two paragraphs.
///
/// # Arguments
/// * `source` - The full markdown source
/// * `ctx` - Edit context with cursor position and paragraph info
///
/// # Returns
/// A `StructuralEdit` with the new source and cursor position in the new paragraph
pub fn split_paragraph(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::Paragraph {
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    // Get the current paragraph text
    let para_text = &ctx.text;
    let cursor = ctx.cursor_offset.min(para_text.len());

    // Split the text at cursor position
    let (before, after) = para_text.split_at(cursor);
    let before = before.trim_end();
    let after = after.trim_start();

    // Rebuild the source with two paragraphs
    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the paragraph
    for i in 0..line_idx {
        new_lines.push(lines[i].to_string());
    }

    // First paragraph (before cursor)
    if !before.is_empty() {
        new_lines.push(before.to_string());
    }

    // Blank line between paragraphs
    new_lines.push(String::new());

    // Second paragraph (after cursor)
    let new_para_line = new_lines.len() + 1; // 1-indexed
    if !after.is_empty() {
        new_lines.push(after.to_string());
    } else {
        new_lines.push(String::new());
    }

    // Lines after the original paragraph
    for i in ctx.end_line..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(new_para_line, 0).with_hint(NodeHint::Paragraph);

    StructuralEdit::success(new_source, cursor_pos)
}

/// Insert a new paragraph after the current node.
///
/// Used for Enter in headings - creates a paragraph below without splitting.
pub fn insert_paragraph_after(source: &str, ctx: &EditContext) -> StructuralEdit {
    let lines: Vec<&str> = source.lines().collect();
    let end_line_idx = ctx.end_line.min(lines.len());

    let mut new_lines: Vec<String> = Vec::new();

    // Lines up to and including the current node
    for i in 0..end_line_idx {
        new_lines.push(lines[i].to_string());
    }

    // Blank line for separation
    new_lines.push(String::new());

    // New empty paragraph line
    let new_para_line = new_lines.len() + 1;
    new_lines.push(String::new());

    // Rest of the document
    for i in end_line_idx..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(new_para_line, 0).with_hint(NodeHint::Paragraph);

    StructuralEdit::success(new_source, cursor_pos)
}

// ─────────────────────────────────────────────────────────────────────────────
// List Item Operations
// ─────────────────────────────────────────────────────────────────────────────

/// Split a list item at the cursor position into two list items.
///
/// # Arguments
/// * `source` - The full markdown source
/// * `ctx` - Edit context with cursor position and list item info
///
/// # Returns
/// A `StructuralEdit` with the new source and cursor in the new list item
pub fn split_list_item(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::ListItem {
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    let original_line = lines[line_idx];

    // Extract prefix (marker) and content from the line
    let (prefix, _content) = extract_list_prefix(original_line);
    if prefix.is_empty() {
        return StructuralEdit::no_op();
    }

    // Use the context text for splitting (more accurate than re-extracting)
    let item_text = &ctx.text;
    let cursor = ctx.cursor_offset.min(item_text.len());

    let (before, after) = item_text.split_at(cursor);
    let before = before.trim_end();
    let after = after.trim_start();

    // Determine the marker for the new item
    let new_marker = get_next_list_marker(prefix, ctx.list_type.as_ref());

    // Build new lines
    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the current item
    for i in 0..line_idx {
        new_lines.push(lines[i].to_string());
    }

    // Current item with text before cursor
    new_lines.push(format!("{}{}", prefix, before));

    // New item with text after cursor
    let new_item_line = new_lines.len() + 1;
    new_lines.push(format!("{}{}", new_marker, after));

    // Lines after the current item
    for i in (line_idx + 1)..lines.len() {
        // Update numbering for ordered lists if needed
        let line = lines[i];
        let updated = update_ordered_list_number(line, ctx.list_type.as_ref());
        new_lines.push(updated);
    }

    let new_source = new_lines.join("\n");
    let list_type = ctx.list_type.unwrap_or(ListType::Bullet);
    let cursor_pos = CursorPosition::new(new_item_line, 0).with_hint(NodeHint::ListItem {
        list_type,
        index: ctx.list_item_index.unwrap_or(0) + 1,
    });

    StructuralEdit::success(new_source, cursor_pos)
}

/// Exit a list by converting an empty list item to a paragraph.
///
/// Called when Enter is pressed on an empty list item.
pub fn exit_list_to_paragraph(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::ListItem {
        return StructuralEdit::no_op();
    }

    // Only exit if the item text is empty or whitespace
    if !ctx.text.trim().is_empty() {
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the empty item
    for i in 0..line_idx {
        new_lines.push(lines[i].to_string());
    }

    // Skip the empty list item line

    // Blank line to end the list
    new_lines.push(String::new());

    // New paragraph line
    let new_para_line = new_lines.len() + 1;
    new_lines.push(String::new());

    // Rest of the document
    for i in (line_idx + 1)..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(new_para_line, 0).with_hint(NodeHint::Paragraph);

    StructuralEdit::success(new_source, cursor_pos)
}

/// Merge a list item with the previous item (backspace at start of item).
pub fn merge_with_previous_list_item(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::ListItem || ctx.cursor_offset != 0 {
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    // If this is the first line, convert to paragraph (no previous item to merge with)
    if line_idx == 0 {
        return convert_list_item_to_paragraph(source, ctx);
    }

    // Check if previous line is also a list item
    let prev_line = lines[line_idx - 1];
    let (prev_prefix, prev_content) = extract_list_prefix(prev_line);

    if prev_prefix.is_empty() {
        // Previous line is not a list item - convert current to paragraph
        return convert_list_item_to_paragraph(source, ctx);
    }

    // Merge: append current item text to previous item
    let current_text = &ctx.text;

    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the previous item
    for i in 0..(line_idx - 1) {
        new_lines.push(lines[i].to_string());
    }

    // Merged item: previous content + space + current content
    let merged_text = if current_text.is_empty() {
        prev_content.to_string()
    } else {
        format!("{} {}", prev_content.trim_end(), current_text.trim_start())
    };
    let cursor_in_merged = prev_content.trim_end().len() + 1; // Position after the space
    new_lines.push(format!("{}{}", prev_prefix, merged_text));

    // Record the line number for cursor positioning
    let merged_line = new_lines.len();

    // Skip current item, continue with rest
    for i in (line_idx + 1)..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    let new_source = new_lines.join("\n");
    let list_type = ctx.list_type.unwrap_or(ListType::Bullet);
    let cursor_pos =
        CursorPosition::new(merged_line, cursor_in_merged).with_hint(NodeHint::ListItem {
            list_type,
            index: ctx.list_item_index.unwrap_or(1).saturating_sub(1),
        });

    StructuralEdit::success(new_source, cursor_pos)
}

/// Convert a list item to a paragraph (backspace when no previous item).
fn convert_list_item_to_paragraph(source: &str, ctx: &EditContext) -> StructuralEdit {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the item
    for i in 0..line_idx {
        new_lines.push(lines[i].to_string());
    }

    // Convert to paragraph (just the text, no marker)
    let para_line = new_lines.len() + 1;
    new_lines.push(ctx.text.clone());

    // Rest of the document
    for i in (line_idx + 1)..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(para_line, 0).with_hint(NodeHint::Paragraph);

    StructuralEdit::success(new_source, cursor_pos)
}

/// Indent a list item (Tab key) - create nested list.
pub fn indent_list_item(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::ListItem {
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    let original_line = lines[line_idx];
    let (prefix, content) = extract_list_prefix(original_line);

    if prefix.is_empty() {
        return StructuralEdit::no_op();
    }

    // Calculate current indentation
    let current_indent = prefix.chars().take_while(|c| c.is_whitespace()).count();
    let new_indent = current_indent + 2; // Standard markdown indent is 2 spaces

    // Build new indented line
    let indent_str = " ".repeat(new_indent);

    // Get the marker without leading whitespace
    let marker = prefix.trim_start();

    let mut new_lines: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i == line_idx {
            new_lines.push(format!("{}{}{}", indent_str, marker, content));
        } else {
            new_lines.push(line.to_string());
        }
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(ctx.start_line, ctx.cursor_offset);

    StructuralEdit::success(new_source, cursor_pos)
}

/// Outdent a list item (Shift+Tab) - promote to parent level.
pub fn outdent_list_item(source: &str, ctx: &EditContext) -> StructuralEdit {
    if ctx.node_type != EditNodeType::ListItem {
        return StructuralEdit::no_op();
    }

    if ctx.nesting_depth == 0 {
        // Already at top level, can't outdent further
        return StructuralEdit::no_op();
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_idx = ctx.start_line.saturating_sub(1);

    if line_idx >= lines.len() {
        return StructuralEdit::no_op();
    }

    let original_line = lines[line_idx];
    let (prefix, content) = extract_list_prefix(original_line);

    if prefix.is_empty() {
        return StructuralEdit::no_op();
    }

    // Calculate current indentation and reduce it
    let current_indent = prefix.chars().take_while(|c| c.is_whitespace()).count();
    let new_indent = current_indent.saturating_sub(2); // Remove 2 spaces of indent

    // Build new line with reduced indentation
    let indent_str = " ".repeat(new_indent);
    let marker = prefix.trim_start();

    let mut new_lines: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i == line_idx {
            new_lines.push(format!("{}{}{}", indent_str, marker, content));
        } else {
            new_lines.push(line.to_string());
        }
    }

    let new_source = new_lines.join("\n");
    let cursor_pos = CursorPosition::new(ctx.start_line, ctx.cursor_offset);

    StructuralEdit::success(new_source, cursor_pos)
}

// ─────────────────────────────────────────────────────────────────────────────
// Heading Operations
// ─────────────────────────────────────────────────────────────────────────────

/// Handle Enter in a heading - always creates a paragraph after the heading.
///
/// Unlike paragraph split, Enter in a heading doesn't split the heading.
/// It creates a new paragraph below the heading for continued typing.
pub fn heading_enter(source: &str, ctx: &EditContext) -> StructuralEdit {
    if !matches!(ctx.node_type, EditNodeType::Heading(_)) {
        return StructuralEdit::no_op();
    }

    insert_paragraph_after(source, ctx)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the list marker prefix and content from a line.
///
/// Returns (prefix, content) where prefix includes indentation and marker,
/// and content is the text after the marker.
fn extract_list_prefix(line: &str) -> (&str, &str) {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();

    // Check for task list markers
    if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        let prefix_len = indent_len + 6;
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        let prefix_len = indent_len + 6;
        return (&line[..prefix_len], rest);
    }

    // Check for bullet markers
    if let Some(rest) = trimmed.strip_prefix("- ") {
        let prefix_len = indent_len + 2;
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        let prefix_len = indent_len + 2;
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        let prefix_len = indent_len + 2;
        return (&line[..prefix_len], rest);
    }

    // Check for ordered list (digits followed by . or ) and space)
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0
        && i < chars.len()
        && (chars[i] == '.' || chars[i] == ')')
        && i + 1 < chars.len()
        && chars[i + 1] == ' '
    {
        let prefix_len = indent_len + i + 2;
        if prefix_len <= line.len() {
            return (&line[..prefix_len], &line[prefix_len..]);
        }
    }

    ("", line)
}

/// Get the marker for a new list item following the given prefix.
fn get_next_list_marker(prev_prefix: &str, list_type: Option<&ListType>) -> String {
    let indent = prev_prefix
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();
    let marker = prev_prefix.trim_start();

    match list_type {
        Some(ListType::Ordered { delimiter, .. }) => {
            // Extract number from previous marker and increment
            let num_str: String = marker.chars().take_while(|c| c.is_ascii_digit()).collect();
            let num: u32 = num_str.parse().unwrap_or(1);
            format!("{}{}{} ", indent, num + 1, delimiter)
        }
        Some(ListType::Bullet) | None => {
            // Use same bullet style
            let bullet = if marker.starts_with("- ") {
                "- "
            } else if marker.starts_with("* ") {
                "* "
            } else if marker.starts_with("+ ") {
                "+ "
            } else {
                "- "
            };
            format!("{}{}", indent, bullet)
        }
    }
}

/// Update ordered list numbering for lines after an insertion.
fn update_ordered_list_number(line: &str, list_type: Option<&ListType>) -> String {
    if !matches!(list_type, Some(ListType::Ordered { .. })) {
        return line.to_string();
    }

    let (prefix, _content) = extract_list_prefix(line);
    if prefix.is_empty() {
        return line.to_string();
    }

    // Check if this is an ordered list item
    let trimmed_prefix = prefix.trim_start();
    if !trimmed_prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
    {
        return line.to_string();
    }

    // For now, don't renumber - just return as-is
    // Full renumbering would require tracking all items in the list
    line.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Extract List Prefix Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_extract_bullet_list_prefix() {
        let (prefix, content) = extract_list_prefix("- Item text");
        assert_eq!(prefix, "- ");
        assert_eq!(content, "Item text");
    }

    #[test]
    fn test_extract_asterisk_list_prefix() {
        let (prefix, content) = extract_list_prefix("* Another item");
        assert_eq!(prefix, "* ");
        assert_eq!(content, "Another item");
    }

    #[test]
    fn test_extract_ordered_list_prefix() {
        let (prefix, content) = extract_list_prefix("1. First item");
        assert_eq!(prefix, "1. ");
        assert_eq!(content, "First item");
    }

    #[test]
    fn test_extract_indented_list_prefix() {
        let (prefix, content) = extract_list_prefix("  - Nested item");
        assert_eq!(prefix, "  - ");
        assert_eq!(content, "Nested item");
    }

    #[test]
    fn test_extract_task_list_prefix_unchecked() {
        let (prefix, content) = extract_list_prefix("- [ ] Todo item");
        assert_eq!(prefix, "- [ ] ");
        assert_eq!(content, "Todo item");
    }

    #[test]
    fn test_extract_task_list_prefix_checked() {
        let (prefix, content) = extract_list_prefix("- [x] Done item");
        assert_eq!(prefix, "- [x] ");
        assert_eq!(content, "Done item");
    }

    #[test]
    fn test_extract_no_list_prefix() {
        let (prefix, content) = extract_list_prefix("Regular text");
        assert_eq!(prefix, "");
        assert_eq!(content, "Regular text");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Paragraph Split Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_split_paragraph_middle() {
        let source = "Hello world";
        let ctx = EditContext {
            node_type: EditNodeType::Paragraph,
            start_line: 1,
            end_line: 1,
            cursor_offset: 5,
            text: "Hello world".to_string(),
            list_type: None,
            list_item_index: None,
            nesting_depth: 0,
        };

        let result = split_paragraph(source, &ctx);
        assert!(result.performed);
        assert!(result.new_source.contains("Hello"));
        assert!(result.new_source.contains("world"));
    }

    #[test]
    fn test_split_paragraph_start() {
        let source = "Hello world";
        let ctx = EditContext {
            node_type: EditNodeType::Paragraph,
            start_line: 1,
            end_line: 1,
            cursor_offset: 0,
            text: "Hello world".to_string(),
            list_type: None,
            list_item_index: None,
            nesting_depth: 0,
        };

        let result = split_paragraph(source, &ctx);
        assert!(result.performed);
    }

    #[test]
    fn test_split_paragraph_end() {
        let source = "Hello world";
        let ctx = EditContext {
            node_type: EditNodeType::Paragraph,
            start_line: 1,
            end_line: 1,
            cursor_offset: 11,
            text: "Hello world".to_string(),
            list_type: None,
            list_item_index: None,
            nesting_depth: 0,
        };

        let result = split_paragraph(source, &ctx);
        assert!(result.performed);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // List Item Split Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_split_list_item_bullet() {
        let source = "- First item\n- Second item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 1,
            end_line: 1,
            cursor_offset: 5,
            text: "First item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(0),
            nesting_depth: 0,
        };

        let result = split_list_item(source, &ctx);
        assert!(result.performed);
        // Should have three items now (First split into two + Second)
        let items: Vec<&str> = result
            .new_source
            .lines()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_split_list_item_ordered() {
        let source = "1. First item\n2. Second item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 1,
            end_line: 1,
            cursor_offset: 5,
            text: "First item".to_string(),
            list_type: Some(ListType::Ordered {
                start: 1,
                delimiter: '.',
            }),
            list_item_index: Some(0),
            nesting_depth: 0,
        };

        let result = split_list_item(source, &ctx);
        assert!(result.performed);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Exit List Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_exit_list_empty_item() {
        let source = "- First item\n- \n- Third item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0,
            text: "".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(1),
            nesting_depth: 0,
        };

        let result = exit_list_to_paragraph(source, &ctx);
        assert!(result.performed);
        assert!(matches!(
            result.cursor_position.node_hint,
            Some(NodeHint::Paragraph)
        ));
    }

    #[test]
    fn test_exit_list_non_empty_item() {
        let source = "- First item\n- Has content";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0,
            text: "Has content".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(1),
            nesting_depth: 0,
        };

        let result = exit_list_to_paragraph(source, &ctx);
        assert!(!result.performed); // Should not exit when item has content
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Indent/Outdent Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_indent_list_item() {
        let source = "- First item\n- Second item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0,
            text: "Second item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(1),
            nesting_depth: 0,
        };

        let result = indent_list_item(source, &ctx);
        assert!(result.performed);
        // Second line should now be indented
        let lines: Vec<&str> = result.new_source.lines().collect();
        assert!(lines[1].starts_with("  ")); // 2 spaces indent
    }

    #[test]
    fn test_outdent_list_item() {
        let source = "- First item\n  - Nested item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0,
            text: "Nested item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(0),
            nesting_depth: 1,
        };

        let result = outdent_list_item(source, &ctx);
        assert!(result.performed);
        // Second line should now be at top level
        let lines: Vec<&str> = result.new_source.lines().collect();
        assert!(lines[1].starts_with("- ")); // No indent
    }

    #[test]
    fn test_outdent_top_level_item() {
        let source = "- First item\n- Second item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0,
            text: "Second item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(1),
            nesting_depth: 0, // Already at top level
        };

        let result = outdent_list_item(source, &ctx);
        assert!(!result.performed); // Can't outdent top-level item
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Heading Enter Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_heading_enter() {
        let source = "# My Heading\n\nParagraph text";
        let ctx = EditContext {
            node_type: EditNodeType::Heading(HeadingLevel::H1),
            start_line: 1,
            end_line: 1,
            cursor_offset: 10,
            text: "My Heading".to_string(),
            list_type: None,
            list_item_index: None,
            nesting_depth: 0,
        };

        let result = heading_enter(source, &ctx);
        assert!(result.performed);
        assert!(matches!(
            result.cursor_position.node_hint,
            Some(NodeHint::Paragraph)
        ));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Merge List Item Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_merge_with_previous_list_item() {
        let source = "- First item\n- Second item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 2,
            end_line: 2,
            cursor_offset: 0, // At start of item
            text: "Second item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(1),
            nesting_depth: 0,
        };

        let result = merge_with_previous_list_item(source, &ctx);
        assert!(result.performed);
        // Should have one merged item
        let items: Vec<&str> = result
            .new_source
            .lines()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(items.len(), 1);
        assert!(items[0].contains("First item"));
        assert!(items[0].contains("Second item"));
    }

    #[test]
    fn test_merge_first_item_converts_to_paragraph() {
        let source = "- Only item";
        let ctx = EditContext {
            node_type: EditNodeType::ListItem,
            start_line: 1,
            end_line: 1,
            cursor_offset: 0,
            text: "Only item".to_string(),
            list_type: Some(ListType::Bullet),
            list_item_index: Some(0),
            nesting_depth: 0,
        };

        let result = merge_with_previous_list_item(source, &ctx);
        assert!(result.performed);
        // Should be converted to paragraph (no list marker)
        assert!(!result.new_source.starts_with("- "));
        assert!(result.new_source.contains("Only item"));
    }
}
