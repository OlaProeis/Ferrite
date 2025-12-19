//! Markdown Formatting Operations
//!
//! This module provides formatting commands for the markdown editor,
//! supporting both raw text and WYSIWYG modes.
//!
//! # Supported Formatting Commands
//! - **Inline**: Bold, Italic, Inline Code, Strikethrough
//! - **Links**: Links, Images
//! - **Blocks**: Code Block, Headings (1-6), Lists, Blockquote

// Allow dead code - this module contains complete formatting API with variants
// and methods for UI buttons that may not all be wired up yet
// - needless_return: Explicit returns can be clearer for early exit patterns
// - manual_range_contains: Explicit comparisons can be clearer
#![allow(dead_code)]
#![allow(clippy::needless_return)]
#![allow(clippy::manual_range_contains)]

//! # Usage
//! ```ignore
//! use crate::markdown::formatting::{MarkdownFormatCommand, apply_raw_format};
//!
//! let result = apply_raw_format(
//!     "Hello world",
//!     Some((0, 5)),  // Selection: "Hello"
//!     MarkdownFormatCommand::Bold,
//! );
//! assert_eq!(result.text, "**Hello** world");
//! ```

use crate::markdown::parser::HeadingLevel;
use crate::string_utils::{ceil_char_boundary, floor_char_boundary};

// ─────────────────────────────────────────────────────────────────────────────
// Format Command Enum
// ─────────────────────────────────────────────────────────────────────────────

/// Markdown formatting commands that can be applied to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownFormatCommand {
    /// Bold text (**text**)
    Bold,
    /// Italic text (*text*)
    Italic,
    /// Inline code (`code`)
    InlineCode,
    /// Strikethrough (~~text~~)
    Strikethrough,
    /// Link ([text](url))
    Link,
    /// Image (![alt](url))
    Image,
    /// Fenced code block (```code```)
    CodeBlock,
    /// Heading level 1-6
    Heading(u8),
    /// Bullet list
    BulletList,
    /// Numbered list
    NumberedList,
    /// Blockquote
    Blockquote,
}

impl MarkdownFormatCommand {
    /// Get the keyboard shortcut label for this command.
    pub fn shortcut_label(&self) -> &'static str {
        match self {
            Self::Bold => "Ctrl+B",
            Self::Italic => "Ctrl+I",
            Self::InlineCode => "Ctrl+`",
            Self::Strikethrough => "Ctrl+Shift+S",
            Self::Link => "Ctrl+K",
            Self::Image => "Ctrl+Shift+K",
            Self::CodeBlock => "Ctrl+Shift+C",
            Self::Heading(1) => "Ctrl+1",
            Self::Heading(2) => "Ctrl+2",
            Self::Heading(3) => "Ctrl+3",
            Self::Heading(4) => "Ctrl+4",
            Self::Heading(5) => "Ctrl+5",
            Self::Heading(6) => "Ctrl+6",
            Self::Heading(_) => "Ctrl+1-6",
            Self::BulletList => "Ctrl+Shift+B",
            Self::NumberedList => "Ctrl+Shift+N",
            Self::Blockquote => "Ctrl+Q",
        }
    }

    /// Get the icon for this command (for toolbar).
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Bold => "𝐁",
            Self::Italic => "𝐼",
            Self::InlineCode => "</>",
            Self::Strikethrough => "S̶",
            Self::Link => "🔗",
            Self::Image => "🖼",
            Self::CodeBlock => "{ }",
            Self::Heading(1) => "H1",
            Self::Heading(2) => "H2",
            Self::Heading(3) => "H3",
            Self::Heading(4) => "H4",
            Self::Heading(5) => "H5",
            Self::Heading(6) => "H6",
            Self::Heading(_) => "H",
            Self::BulletList => "\u{2022}", // bullet •
            Self::NumberedList => "1.",
            Self::Blockquote => "\u{275D}", // heavy double turned comma quotation mark ❝
        }
    }

    /// Get the tooltip text for this command.
    pub fn tooltip(&self) -> String {
        let name = match self {
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::InlineCode => "Inline Code",
            Self::Strikethrough => "Strikethrough",
            Self::Link => "Insert Link",
            Self::Image => "Insert Image",
            Self::CodeBlock => "Code Block",
            Self::Heading(n) => return format!("Heading {} ({})", n, self.shortcut_label()),
            Self::BulletList => "Bullet List",
            Self::NumberedList => "Numbered List",
            Self::Blockquote => "Blockquote",
        };
        format!("{} ({})", name, self.shortcut_label())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Format Result
// ─────────────────────────────────────────────────────────────────────────────

/// Result of applying a formatting command.
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// The new text after formatting
    pub text: String,
    /// New cursor position (character index)
    pub cursor: usize,
    /// New selection range (start, end) if applicable
    pub selection: Option<(usize, usize)>,
    /// Whether the formatting was applied (vs removed/toggled off)
    pub applied: bool,
}

impl FormatResult {
    /// Create a result with just cursor position.
    pub fn with_cursor(text: String, cursor: usize) -> Self {
        Self {
            text,
            cursor,
            selection: None,
            applied: true,
        }
    }

    /// Create a result with a selection range.
    pub fn with_selection(text: String, start: usize, end: usize) -> Self {
        Self {
            text,
            cursor: end,
            selection: Some((start, end)),
            applied: true,
        }
    }

    /// Mark that formatting was removed rather than applied.
    pub fn toggled_off(mut self) -> Self {
        self.applied = false;
        self
    }

    /// Mark that formatting was not applied (e.g., no selection).
    pub fn not_applied(mut self) -> Self {
        self.applied = false;
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatting State
// ─────────────────────────────────────────────────────────────────────────────

/// Current formatting state at a cursor position.
/// Used for toolbar state reflection.
#[derive(Debug, Clone, Default)]
pub struct FormattingState {
    /// Is the cursor in bold text?
    pub is_bold: bool,
    /// Is the cursor in italic text?
    pub is_italic: bool,
    /// Is the cursor in inline code?
    pub is_inline_code: bool,
    /// Is the cursor in strikethrough text?
    pub is_strikethrough: bool,
    /// Is the cursor in a link?
    pub is_link: bool,
    /// Is the cursor in an image?
    pub is_image: bool,
    /// Is the cursor in a code block?
    pub is_code_block: bool,
    /// Current heading level (None if not in heading)
    pub heading_level: Option<HeadingLevel>,
    /// Is the cursor in a bullet list?
    pub is_bullet_list: bool,
    /// Is the cursor in a numbered list?
    pub is_numbered_list: bool,
    /// Is the cursor in a blockquote?
    pub is_blockquote: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw Mode Formatting
// ─────────────────────────────────────────────────────────────────────────────

/// Apply a formatting command in raw mode.
///
/// # Arguments
/// * `text` - The full text content
/// * `selection` - Optional selection range (start, end) in character indices
/// * `command` - The formatting command to apply
///
/// # Returns
/// A `FormatResult` with the new text and cursor/selection position.
pub fn apply_raw_format(
    text: &str,
    selection: Option<(usize, usize)>,
    command: MarkdownFormatCommand,
) -> FormatResult {
    match command {
        MarkdownFormatCommand::Bold => apply_inline_format(text, selection, "**", "**"),
        MarkdownFormatCommand::Italic => apply_inline_format(text, selection, "*", "*"),
        MarkdownFormatCommand::InlineCode => apply_inline_format(text, selection, "`", "`"),
        MarkdownFormatCommand::Strikethrough => apply_inline_format(text, selection, "~~", "~~"),
        MarkdownFormatCommand::Link => apply_link_format(text, selection, false),
        MarkdownFormatCommand::Image => apply_link_format(text, selection, true),
        MarkdownFormatCommand::CodeBlock => apply_code_block_format(text, selection),
        MarkdownFormatCommand::Heading(level) => apply_heading_format(text, selection, level),
        MarkdownFormatCommand::BulletList => apply_list_format(text, selection, false),
        MarkdownFormatCommand::NumberedList => apply_list_format(text, selection, true),
        MarkdownFormatCommand::Blockquote => apply_blockquote_format(text, selection),
    }
}

/// Apply inline formatting with delimiters (bold, italic, code, strikethrough).
fn apply_inline_format(
    text: &str,
    selection: Option<(usize, usize)>,
    prefix: &str,
    suffix: &str,
) -> FormatResult {
    let (start, end) = selection.unwrap_or({
        // No selection - find word at cursor or use cursor position
        (text.len(), text.len())
    });

    // Ensure valid range and adjust to UTF-8 char boundaries
    let start = floor_char_boundary(text, start.min(text.len()));
    let end = ceil_char_boundary(text, end.min(text.len()));
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    let selected_text = &text[start..end];

    // Check if already formatted - toggle off
    if selected_text.starts_with(prefix) && selected_text.ends_with(suffix) {
        // Remove formatting
        let inner_start = prefix.len();
        let inner_end = selected_text.len().saturating_sub(suffix.len());
        let inner = &selected_text[inner_start..inner_end];
        let new_text = format!("{}{}{}", &text[..start], inner, &text[end..]);
        return FormatResult::with_selection(new_text, start, start + inner.len()).toggled_off();
    }

    // Check if surrounding text has the formatting
    let before_start = floor_char_boundary(text, start.saturating_sub(prefix.len()));
    let after_end = ceil_char_boundary(text, (end + suffix.len()).min(text.len()));

    if before_start + prefix.len() <= start
        && text[before_start..start].ends_with(prefix)
        && text[end..after_end].starts_with(suffix)
    {
        // Remove surrounding formatting
        let new_text = format!(
            "{}{}{}",
            &text[..before_start],
            selected_text,
            &text[after_end..]
        );
        return FormatResult::with_selection(
            new_text,
            before_start,
            before_start + selected_text.len(),
        )
        .toggled_off();
    }

    // Apply formatting
    if start == end {
        // No selection - do nothing for inline formatting
        // User must select text first
        FormatResult::with_cursor(text.to_string(), start).not_applied()
    } else {
        // Wrap selection
        let new_text = format!(
            "{}{}{}{}{}",
            &text[..start],
            prefix,
            selected_text,
            suffix,
            &text[end..]
        );
        let new_end = start + prefix.len() + selected_text.len() + suffix.len();
        FormatResult::with_cursor(new_text, new_end)
    }
}

/// Apply link or image formatting.
fn apply_link_format(
    text: &str,
    selection: Option<(usize, usize)>,
    is_image: bool,
) -> FormatResult {
    let (start, end) = selection.unwrap_or((text.len(), text.len()));
    // Adjust to UTF-8 char boundaries
    let start = floor_char_boundary(text, start.min(text.len()));
    let end = ceil_char_boundary(text, end.min(text.len()));
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    let selected_text = &text[start..end];
    let prefix = if is_image { "![" } else { "[" };

    // Check if already a link/image - detect and toggle
    if is_markdown_link(text, start, end, is_image) {
        // For simplicity, we won't remove links - just create a new one
        // Full toggle implementation would require more complex parsing
    }

    if selected_text.is_empty() {
        // No selection - do nothing for link/image formatting
        // User must select text first
        FormatResult::with_cursor(text.to_string(), start).not_applied()
    } else {
        // Use selection as link text
        let new_text = format!(
            "{}{}{}](url){}",
            &text[..start],
            prefix,
            selected_text,
            &text[end..]
        );
        // Select "url" for easy replacement
        let url_start = start + prefix.len() + selected_text.len() + 2;
        let url_end = url_start + 3;
        FormatResult::with_selection(new_text, url_start, url_end)
    }
}

/// Check if the selection is inside a markdown link.
fn is_markdown_link(text: &str, _start: usize, _end: usize, _is_image: bool) -> bool {
    // Simple check - look for [...](...) pattern around selection
    // For now, return false - full implementation would parse the link structure
    let _ = text;
    false
}

/// Apply code block formatting.
fn apply_code_block_format(text: &str, selection: Option<(usize, usize)>) -> FormatResult {
    let (start, end) = selection.unwrap_or((text.len(), text.len()));
    // Adjust to UTF-8 char boundaries
    let start = floor_char_boundary(text, start.min(text.len()));
    let end = ceil_char_boundary(text, end.min(text.len()));
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    // Find line boundaries (rfind returns byte position which is safe for '\n')
    let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[end..]
        .find('\n')
        .map(|i| end + i)
        .unwrap_or(text.len());

    let lines_text = &text[line_start..line_end];

    // Check if already in a code block (line starts with ```)
    if lines_text.trim_start().starts_with("```") {
        // Already a code block - remove it
        let lines: Vec<&str> = lines_text.lines().collect();
        if lines.len() >= 2 {
            // Remove first and last lines if they're fence markers
            let first_is_fence = lines
                .first()
                .map(|l| l.trim_start().starts_with("```"))
                .unwrap_or(false);
            let last_is_fence = lines.last().map(|l| l.trim() == "```").unwrap_or(false);

            if first_is_fence && last_is_fence {
                let inner: String = lines[1..lines.len() - 1].join("\n");
                let new_text = format!("{}{}{}", &text[..line_start], inner, &text[line_end..]);
                return FormatResult::with_cursor(new_text, line_start).toggled_off();
            }
        }
    }

    // Add code block fences
    let new_text = format!(
        "{}```\n{}\n```{}",
        &text[..line_start],
        lines_text,
        &text[line_end..]
    );

    // Position cursor after opening fence (for language tag)
    let cursor = line_start + 3;
    FormatResult::with_cursor(new_text, cursor)
}

/// Apply heading formatting.
fn apply_heading_format(text: &str, selection: Option<(usize, usize)>, level: u8) -> FormatResult {
    let level = level.clamp(1, 6);
    let (start, _end) = selection.unwrap_or((text.len(), text.len()));
    // Adjust to UTF-8 char boundary
    let start = floor_char_boundary(text, start.min(text.len()));

    // Find line boundaries (rfind returns byte position which is safe for '\n')
    let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[start..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(text.len());

    let line = &text[line_start..line_end];

    // Check if line already has a heading marker
    let trimmed = line.trim_start();
    let existing_level = trimmed.chars().take_while(|&c| c == '#').count();

    // Calculate byte offset for existing_level (# is ASCII, so safe)
    let content = if existing_level > 0 {
        // Remove existing heading marker - existing_level is count of '#' chars which are ASCII
        trimmed[existing_level..].trim_start()
    } else {
        trimmed
    };

    // If same level, toggle off (remove heading)
    if existing_level == level as usize {
        let new_text = format!("{}{}{}", &text[..line_start], content, &text[line_end..]);
        return FormatResult::with_cursor(new_text, line_start).toggled_off();
    }

    // Apply new heading level
    let hashes = "#".repeat(level as usize);
    let new_line = format!("{} {}", hashes, content);
    let new_text = format!("{}{}{}", &text[..line_start], new_line, &text[line_end..]);

    // Position cursor at end of heading text
    let cursor = line_start + new_line.len();
    FormatResult::with_cursor(new_text, cursor)
}

/// Apply list formatting (bullet or numbered).
fn apply_list_format(
    text: &str,
    selection: Option<(usize, usize)>,
    numbered: bool,
) -> FormatResult {
    let (start, end) = selection.unwrap_or((text.len(), text.len()));
    // Adjust to UTF-8 char boundaries
    let start = floor_char_boundary(text, start.min(text.len()));
    let end = ceil_char_boundary(text, end.min(text.len()));
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    // Find line boundaries for the selection (rfind returns byte position which is safe for '\n')
    let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[end..]
        .find('\n')
        .map(|i| end + i)
        .unwrap_or(text.len());

    let lines_text = &text[line_start..line_end];
    let lines: Vec<&str> = lines_text.lines().collect();

    // Check if all lines are already list items
    let all_list = lines.iter().all(|line| {
        let trimmed = line.trim_start();
        if numbered {
            is_numbered_list_item(trimmed)
        } else {
            is_bullet_list_item(trimmed)
        }
    });

    let new_lines: Vec<String> = if all_list {
        // Remove list markers
        lines
            .iter()
            .map(|line| {
                let trimmed = line.trim_start();
                remove_list_marker(trimmed).to_string()
            })
            .collect()
    } else {
        // Add list markers
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let trimmed = line.trim_start();
                // Remove any existing list marker first
                let content = remove_list_marker(trimmed);
                if numbered {
                    format!("{}. {}", i + 1, content)
                } else {
                    format!("- {}", content)
                }
            })
            .collect()
    };

    let new_lines_text = new_lines.join("\n");
    let new_text = format!(
        "{}{}{}",
        &text[..line_start],
        new_lines_text,
        &text[line_end..]
    );

    let toggled_off = all_list;
    let cursor = line_start + new_lines_text.len();

    let result = FormatResult::with_cursor(new_text, cursor);
    if toggled_off {
        result.toggled_off()
    } else {
        result
    }
}

/// Check if a line is a bullet list item.
fn is_bullet_list_item(trimmed: &str) -> bool {
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || trimmed.starts_with("- [ ] ")
        || trimmed.starts_with("- [x] ")
        || trimmed.starts_with("- [X] ")
}

/// Check if a line is a numbered list item.
fn is_numbered_list_item(trimmed: &str) -> bool {
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    i > 0
        && i < chars.len()
        && (chars[i] == '.' || chars[i] == ')')
        && i + 1 < chars.len()
        && chars[i + 1] == ' '
}

/// Remove list marker from a line.
fn remove_list_marker(trimmed: &str) -> &str {
    // Task list markers
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("- [X] ") {
        return rest;
    }

    // Bullet markers
    if let Some(rest) = trimmed.strip_prefix("- ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        return rest;
    }

    // Numbered markers
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
        return &trimmed[i + 2..];
    }

    trimmed
}

/// Apply blockquote formatting.
fn apply_blockquote_format(text: &str, selection: Option<(usize, usize)>) -> FormatResult {
    let (start, end) = selection.unwrap_or((text.len(), text.len()));
    // Adjust to UTF-8 char boundaries
    let start = floor_char_boundary(text, start.min(text.len()));
    let end = ceil_char_boundary(text, end.min(text.len()));
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    // Find line boundaries for the selection (rfind returns byte position which is safe for '\n')
    let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[end..]
        .find('\n')
        .map(|i| end + i)
        .unwrap_or(text.len());

    let lines_text = &text[line_start..line_end];
    let lines: Vec<&str> = lines_text.lines().collect();

    // Check if all lines are already blockquotes
    let all_quotes = lines.iter().all(|line| line.trim_start().starts_with("> "));

    let new_lines: Vec<String> = if all_quotes {
        // Remove blockquote markers
        lines
            .iter()
            .map(|line| {
                let trimmed = line.trim_start();
                trimmed.strip_prefix("> ").unwrap_or(trimmed).to_string()
            })
            .collect()
    } else {
        // Add blockquote markers
        lines.iter().map(|line| format!("> {}", line)).collect()
    };

    let new_lines_text = new_lines.join("\n");
    let new_text = format!(
        "{}{}{}",
        &text[..line_start],
        new_lines_text,
        &text[line_end..]
    );

    let toggled_off = all_quotes;
    let cursor = line_start + new_lines_text.len();

    let result = FormatResult::with_cursor(new_text, cursor);
    if toggled_off {
        result.toggled_off()
    } else {
        result
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatting State Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Detect the formatting state at a cursor position in raw text.
pub fn detect_raw_formatting_state(text: &str, cursor: usize) -> FormattingState {
    // Adjust cursor to UTF-8 char boundary
    let cursor = floor_char_boundary(text, cursor.min(text.len()));
    let mut state = FormattingState::default();

    // Find line boundaries (rfind returns byte position which is safe for '\n')
    let line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[cursor..]
        .find('\n')
        .map(|i| cursor + i)
        .unwrap_or(text.len());
    let line = &text[line_start..line_end];
    let trimmed = line.trim_start();

    // Check line-level formatting
    // Heading
    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hash_count) && trimmed.chars().nth(hash_count) == Some(' ') {
        state.heading_level = Some(HeadingLevel::from(hash_count as u8));
    }

    // Blockquote
    state.is_blockquote = trimmed.starts_with("> ");

    // List items
    state.is_bullet_list = is_bullet_list_item(trimmed);
    state.is_numbered_list = is_numbered_list_item(trimmed);

    // Code block - check if we're inside fenced code
    state.is_code_block = is_inside_code_block(text, cursor);

    // Inline formatting detection (simplified - checks surrounding delimiters)
    // This is a basic implementation; full parsing would use the AST
    let before = &text[..cursor];
    let after = &text[cursor..];

    // Bold: count ** before and after cursor
    state.is_bold = has_balanced_inline_marker(before, after, "**");

    // Italic: count single * (but not **)
    state.is_italic = has_balanced_inline_marker(before, after, "*") && !state.is_bold;

    // Inline code: count `
    state.is_inline_code = has_balanced_inline_marker(before, after, "`") && !state.is_code_block;

    // Strikethrough: count ~~
    state.is_strikethrough = has_balanced_inline_marker(before, after, "~~");

    state
}

/// Check if cursor is inside a fenced code block.
fn is_inside_code_block(text: &str, cursor: usize) -> bool {
    // Adjust cursor to UTF-8 char boundary
    let cursor = floor_char_boundary(text, cursor);
    let before = &text[..cursor];

    // Count fence openers and closers before cursor
    let mut in_block = false;
    for line in before.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_block = !in_block;
        }
    }

    in_block
}

/// Check if there's a balanced inline marker around the cursor.
fn has_balanced_inline_marker(before: &str, after: &str, marker: &str) -> bool {
    // Find the last line (inline formatting doesn't span lines typically)
    let line_before = before.rsplit('\n').next().unwrap_or(before);
    let line_after = after.split('\n').next().unwrap_or(after);

    // Count markers in the line portion before cursor
    let count_before = count_non_overlapping(line_before, marker);

    // For proper toggle detection, we need odd count before AND marker after
    if count_before % 2 == 1 {
        // Check if there's a matching marker after
        line_after.contains(marker)
    } else {
        false
    }
}

/// Count non-overlapping occurrences of a pattern.
fn count_non_overlapping(text: &str, pattern: &str) -> usize {
    if pattern.is_empty() {
        return 0;
    }
    text.match_indices(pattern).count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Bold Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bold_with_selection() {
        let result = apply_raw_format("Hello world", Some((0, 5)), MarkdownFormatCommand::Bold);
        assert_eq!(result.text, "**Hello** world");
        assert!(result.applied);
    }

    #[test]
    fn test_bold_without_selection() {
        // When no text is selected, bold should do nothing
        let result = apply_raw_format("Hello", Some((5, 5)), MarkdownFormatCommand::Bold);
        assert_eq!(result.text, "Hello");
        assert!(!result.applied);
        assert!(result.selection.is_none());
    }

    #[test]
    fn test_bold_toggle_off() {
        let result = apply_raw_format("**Hello** world", Some((0, 9)), MarkdownFormatCommand::Bold);
        assert_eq!(result.text, "Hello world");
        assert!(!result.applied);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Italic Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_italic_with_selection() {
        let result = apply_raw_format("Hello world", Some((6, 11)), MarkdownFormatCommand::Italic);
        assert_eq!(result.text, "Hello *world*");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Link Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_link_with_selection() {
        let result = apply_raw_format("Click here", Some((6, 10)), MarkdownFormatCommand::Link);
        assert_eq!(result.text, "Click [here](url)");
        assert_eq!(result.selection, Some((13, 16))); // "url" selected
    }

    #[test]
    fn test_link_without_selection() {
        // When no text is selected, link should do nothing
        let result = apply_raw_format("Hello", Some((5, 5)), MarkdownFormatCommand::Link);
        assert_eq!(result.text, "Hello");
        assert!(!result.applied);
    }

    #[test]
    fn test_image_without_selection() {
        // When no text is selected, image should do nothing
        let result = apply_raw_format("Hello", Some((5, 5)), MarkdownFormatCommand::Image);
        assert_eq!(result.text, "Hello");
        assert!(!result.applied);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Heading Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_heading_h1() {
        let result = apply_raw_format(
            "Hello world",
            Some((0, 0)),
            MarkdownFormatCommand::Heading(1),
        );
        assert_eq!(result.text, "# Hello world");
    }

    #[test]
    fn test_heading_h2() {
        let result = apply_raw_format(
            "Hello world",
            Some((0, 0)),
            MarkdownFormatCommand::Heading(2),
        );
        assert_eq!(result.text, "## Hello world");
    }

    #[test]
    fn test_heading_change_level() {
        let result = apply_raw_format(
            "# Hello world",
            Some((0, 0)),
            MarkdownFormatCommand::Heading(2),
        );
        assert_eq!(result.text, "## Hello world");
    }

    #[test]
    fn test_heading_toggle_off() {
        let result = apply_raw_format(
            "# Hello world",
            Some((0, 0)),
            MarkdownFormatCommand::Heading(1),
        );
        assert_eq!(result.text, "Hello world");
        assert!(!result.applied);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // List Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bullet_list_single_line() {
        let result = apply_raw_format("Item one", Some((0, 0)), MarkdownFormatCommand::BulletList);
        assert_eq!(result.text, "- Item one");
    }

    #[test]
    fn test_bullet_list_toggle_off() {
        let result = apply_raw_format(
            "- Item one",
            Some((0, 0)),
            MarkdownFormatCommand::BulletList,
        );
        assert_eq!(result.text, "Item one");
        assert!(!result.applied);
    }

    #[test]
    fn test_numbered_list_single_line() {
        let result = apply_raw_format(
            "Item one",
            Some((0, 0)),
            MarkdownFormatCommand::NumberedList,
        );
        assert_eq!(result.text, "1. Item one");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Blockquote Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_blockquote_single_line() {
        let result = apply_raw_format("A quote", Some((0, 0)), MarkdownFormatCommand::Blockquote);
        assert_eq!(result.text, "> A quote");
    }

    #[test]
    fn test_blockquote_toggle_off() {
        let result = apply_raw_format("> A quote", Some((0, 0)), MarkdownFormatCommand::Blockquote);
        assert_eq!(result.text, "A quote");
        assert!(!result.applied);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Code Block Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_code_block_single_line() {
        let result = apply_raw_format("let x = 1;", Some((0, 0)), MarkdownFormatCommand::CodeBlock);
        assert!(result.text.starts_with("```\n"));
        assert!(result.text.contains("let x = 1;"));
        assert!(result.text.ends_with("\n```"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Formatting State Detection Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_detect_heading_state() {
        let state = detect_raw_formatting_state("# Heading text", 5);
        assert_eq!(state.heading_level, Some(HeadingLevel::H1));
    }

    #[test]
    fn test_detect_blockquote_state() {
        let state = detect_raw_formatting_state("> Quote text", 5);
        assert!(state.is_blockquote);
    }

    #[test]
    fn test_detect_bullet_list_state() {
        let state = detect_raw_formatting_state("- List item", 5);
        assert!(state.is_bullet_list);
    }

    #[test]
    fn test_detect_numbered_list_state() {
        let state = detect_raw_formatting_state("1. List item", 5);
        assert!(state.is_numbered_list);
    }

    #[test]
    fn test_detect_code_block_state() {
        let text = "```\ncode here\n```";
        let state = detect_raw_formatting_state(text, 8); // Inside "code here"
        assert!(state.is_code_block);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Command Metadata Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_shortcut_labels() {
        assert_eq!(MarkdownFormatCommand::Bold.shortcut_label(), "Ctrl+B");
        assert_eq!(MarkdownFormatCommand::Italic.shortcut_label(), "Ctrl+I");
        assert_eq!(MarkdownFormatCommand::Link.shortcut_label(), "Ctrl+K");
    }

    #[test]
    fn test_icons() {
        assert!(!MarkdownFormatCommand::Bold.icon().is_empty());
        assert!(!MarkdownFormatCommand::BulletList.icon().is_empty());
    }

    #[test]
    fn test_tooltips() {
        let tooltip = MarkdownFormatCommand::Bold.tooltip();
        assert!(tooltip.contains("Bold"));
        assert!(tooltip.contains("Ctrl+B"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UTF-8 Safe Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bold_norwegian_chars() {
        // Norwegian characters: ø (2 bytes), æ (2 bytes), å (2 bytes)
        let result = apply_raw_format("Hei på deg", Some((4, 6)), MarkdownFormatCommand::Bold);
        assert!(result.text.contains("**på**"));
    }

    #[test]
    fn test_bold_chinese_chars() {
        // Chinese characters: 你好 (each char is 3 bytes)
        let result = apply_raw_format("Hello 你好 World", Some((6, 12)), MarkdownFormatCommand::Bold);
        assert!(result.text.contains("**你好**"));
    }

    #[test]
    fn test_bold_emoji() {
        // Emoji: 🎉 (4 bytes)
        let result = apply_raw_format("Party 🎉 time", Some((6, 10)), MarkdownFormatCommand::Bold);
        assert!(result.text.contains("**🎉**"));
    }

    #[test]
    fn test_formatting_state_norwegian() {
        // Test cursor position in text with Norwegian chars
        let state = detect_raw_formatting_state("# Hei på deg", 5);
        assert_eq!(state.heading_level, Some(HeadingLevel::H1));
    }

    #[test]
    fn test_heading_norwegian() {
        // Test heading formatting with Norwegian text
        let result = apply_raw_format("Østersjøen", Some((0, 0)), MarkdownFormatCommand::Heading(1));
        assert_eq!(result.text, "# Østersjøen");
    }

    #[test]
    fn test_list_format_with_unicode() {
        // Test list formatting with unicode content
        let result = apply_raw_format("日本語テスト", Some((0, 0)), MarkdownFormatCommand::BulletList);
        assert_eq!(result.text, "- 日本語テスト");
    }

    #[test]
    fn test_blockquote_with_accented_chars() {
        // Test blockquote with accented characters: café, naïve
        let result = apply_raw_format("Café naïve", Some((0, 0)), MarkdownFormatCommand::Blockquote);
        assert_eq!(result.text, "> Café naïve");
    }

    #[test]
    fn test_no_panic_on_any_byte_index() {
        // Ensure no panic when given arbitrary byte indices that may fall mid-character
        let text = "Hei på deg 你好 🎉";
        // Test various byte positions, some may fall inside multi-byte chars
        for i in 0..=text.len() + 5 {
            for j in i..=text.len() + 5 {
                // Should not panic
                let _ = apply_raw_format(text, Some((i, j)), MarkdownFormatCommand::Bold);
                let _ = apply_raw_format(text, Some((i, j)), MarkdownFormatCommand::Italic);
                let _ = detect_raw_formatting_state(text, i);
            }
        }
    }
}
