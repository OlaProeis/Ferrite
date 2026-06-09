//! Markdown parser implementation using comrak
//!
//! This module wraps comrak's parsing functions to provide a clean API
//! for parsing markdown text and rendering it to HTML.

use comrak::{
    nodes::{
        AstNode, ListDelimType, ListType as ComrakListType, NodeValue,
        TableAlignment as ComrakTableAlignment,
    },
    parse_document, Arena, Options,
};

use crate::error::Result;

// ─────────────────────────────────────────────────────────────────────────────
// Public Types
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration options for markdown parsing and rendering.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Enable GitHub Flavored Markdown tables
    pub tables: bool,
    /// Enable strikethrough syntax (~~text~~)
    pub strikethrough: bool,
    /// Enable autolink URLs and emails
    pub autolink: bool,
    /// Enable task lists (- [ ] and - [x])
    pub tasklist: bool,
    /// Enable superscript (^text^)
    pub superscript: bool,
    /// Enable footnotes
    pub footnotes: bool,
    /// Enable description lists
    pub description_lists: bool,
    /// Enable front matter (YAML/TOML)
    pub front_matter_delimiter: Option<String>,
    /// Make URLs safe by removing potentially dangerous protocols
    pub safe_urls: bool,
    /// Generate GitHub-style heading IDs
    pub header_ids: Option<String>,
    /// Treat soft breaks as hard line breaks (renders newlines as `<br>`)
    pub hardbreaks: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            tables: true,
            strikethrough: true,
            autolink: true,
            tasklist: true,
            superscript: false,
            footnotes: true,
            description_lists: false,
            front_matter_delimiter: Some("---".to_string()),
            safe_urls: true,
            header_ids: Some(String::new()),
            hardbreaks: false,
        }
    }
}

impl MarkdownOptions {
    /// Convert to comrak Options.
    fn to_comrak_options(&self) -> Options {
        let mut options = Options::default();

        // Extension options
        options.extension.strikethrough = self.strikethrough;
        options.extension.table = self.tables;
        options.extension.autolink = self.autolink;
        options.extension.tasklist = self.tasklist;
        options.extension.superscript = self.superscript;
        options.extension.footnotes = self.footnotes;
        options.extension.description_lists = self.description_lists;
        options.extension.front_matter_delimiter = self.front_matter_delimiter.clone();
        options.extension.header_ids = self.header_ids.clone();

        // Render options
        options.render.unsafe_ = !self.safe_urls;
        options.render.hardbreaks = self.hardbreaks;

        options
    }
}

/// Heading level (H1-H6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    H1 = 1,
    H2 = 2,
    H3 = 3,
    H4 = 4,
    H5 = 5,
    H6 = 6,
}

impl From<u8> for HeadingLevel {
    fn from(level: u8) -> Self {
        match level {
            1 => HeadingLevel::H1,
            2 => HeadingLevel::H2,
            3 => HeadingLevel::H3,
            4 => HeadingLevel::H4,
            5 => HeadingLevel::H5,
            _ => HeadingLevel::H6,
        }
    }
}

/// List type (ordered or unordered)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    Bullet,
    Ordered { start: u32, delimiter: char },
}

/// Table cell alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableAlignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

impl From<ComrakTableAlignment> for TableAlignment {
    fn from(align: ComrakTableAlignment) -> Self {
        match align {
            ComrakTableAlignment::None => TableAlignment::None,
            ComrakTableAlignment::Left => TableAlignment::Left,
            ComrakTableAlignment::Center => TableAlignment::Center,
            ComrakTableAlignment::Right => TableAlignment::Right,
        }
    }
}

/// Type of GitHub-style callout/admonition block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutType {
    Note,
    Tip,
    Warning,
    Caution,
    Important,
}

impl CalloutType {
    /// Parse a callout type string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NOTE" => Some(CalloutType::Note),
            "TIP" => Some(CalloutType::Tip),
            "WARNING" => Some(CalloutType::Warning),
            "CAUTION" => Some(CalloutType::Caution),
            "IMPORTANT" => Some(CalloutType::Important),
            _ => None,
        }
    }

    /// Get the display name for this callout type.
    pub fn display_name(&self) -> &'static str {
        match self {
            CalloutType::Note => "Note",
            CalloutType::Tip => "Tip",
            CalloutType::Warning => "Warning",
            CalloutType::Caution => "Caution",
            CalloutType::Important => "Important",
        }
    }

    /// Get the icon character for this callout type.
    pub fn icon(&self) -> &'static str {
        match self {
            CalloutType::Note => "ℹ",
            CalloutType::Tip => "💡",
            CalloutType::Warning => "⚠",
            CalloutType::Caution => "🔶",
            CalloutType::Important => "❗",
        }
    }
}

/// Represents the type of a markdown node.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownNodeType {
    /// Root document node
    Document,
    /// Block quote (>)
    BlockQuote,
    /// GitHub-style callout/admonition (> [!TYPE])
    Callout {
        callout_type: CalloutType,
        /// Custom title (if provided), otherwise uses default type name
        title: Option<String>,
        /// Whether the callout is collapsed by default (> [!TYPE]-)
        collapsed: bool,
    },
    /// List container
    List { list_type: ListType, tight: bool },
    /// List item
    Item,
    /// Code block with optional language
    CodeBlock {
        language: String,
        info: String,
        literal: String,
    },
    /// HTML block
    HtmlBlock(String),
    /// Paragraph
    Paragraph,
    /// Heading (H1-H6)
    Heading { level: HeadingLevel, setext: bool },
    /// Thematic break (horizontal rule)
    ThematicBreak,
    /// Table
    Table {
        alignments: Vec<TableAlignment>,
        num_columns: usize,
    },
    /// Table row
    TableRow { header: bool },
    /// Table cell
    TableCell,
    /// Inline text content
    Text(String),
    /// Task list marker
    TaskItem { checked: bool },
    /// Soft line break
    SoftBreak,
    /// Hard line break
    LineBreak,
    /// Inline code
    Code(String),
    /// Inline HTML
    HtmlInline(String),
    /// Emphasis (italic)
    Emphasis,
    /// Strong emphasis (bold)
    Strong,
    /// Strikethrough
    Strikethrough,
    /// Superscript
    Superscript,
    /// Link
    Link { url: String, title: String },
    /// Image
    Image { url: String, title: String },
    /// Footnote reference
    FootnoteReference(String),
    /// Footnote definition
    FootnoteDefinition(String),
    /// Description list
    DescriptionList,
    /// Description item
    DescriptionItem,
    /// Description term
    DescriptionTerm,
    /// Description details
    DescriptionDetails,
    /// Front matter (YAML/TOML)
    FrontMatter(String),
    /// Wikilink ([[target]] or [[target|display text]])
    Wikilink {
        /// The link target (file name or path, without .md extension)
        target: String,
        /// Optional display text (if [[target|display]] syntax is used)
        display: Option<String>,
    },
    /// Embedded video ({{video URL}} or bare YouTube URL paragraph)
    VideoEmbed(VideoEmbedInfo),
}

/// Supported video hosting provider for embeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoProvider {
    YouTube,
    Unknown,
}

/// Parsed video embed metadata extracted from markdown source.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoEmbedInfo {
    pub provider: VideoProvider,
    /// YouTube video ID when `provider` is `YouTube`.
    pub video_id: Option<String>,
    /// Normalized URL (http/https only).
    pub url: String,
    /// Whether the URL domain is allowlisted for interactive WebView embedding.
    pub trusted: bool,
    /// Original markdown source text preserved for round-trip editing.
    pub source_text: String,
}

/// A node in the markdown AST with position information.
#[derive(Debug, Clone)]
pub struct MarkdownNode {
    /// The type of this node
    pub node_type: MarkdownNodeType,
    /// Child nodes
    pub children: Vec<MarkdownNode>,
    /// Start line in source (1-indexed)
    pub start_line: usize,
    /// End line in source (1-indexed)
    pub end_line: usize,
}

impl MarkdownNode {
    /// Create a new markdown node.
    fn new(
        node_type: MarkdownNodeType,
        start_line: usize,
        _start_column: usize,
        end_line: usize,
        _end_column: usize,
    ) -> Self {
        Self {
            node_type,
            children: Vec::new(),
            start_line,
            end_line,
        }
    }

    /// Get all text content from this node and its descendants.
    pub fn text_content(&self) -> String {
        let mut text = String::new();
        self.collect_text(&mut text);
        text
    }

    fn collect_text(&self, output: &mut String) {
        match &self.node_type {
            MarkdownNodeType::Text(t) => output.push_str(t),
            MarkdownNodeType::Code(t) => output.push_str(t),
            MarkdownNodeType::SoftBreak => output.push(' '),
            MarkdownNodeType::LineBreak => output.push('\n'),
            MarkdownNodeType::Wikilink { target, display } => {
                output.push_str(display.as_deref().unwrap_or(target));
            }
            MarkdownNodeType::VideoEmbed(info) => output.push_str(&info.source_text),
            _ => {}
        }
        for child in &self.children {
            child.collect_text(output);
        }
    }
}

/// A parsed markdown document containing the AST and metadata.
#[derive(Debug, Clone)]
pub struct MarkdownDocument {
    /// Root node of the AST
    pub root: MarkdownNode,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Parse markdown text into an AST document.
///
/// # Arguments
/// * `markdown` - The markdown text to parse
///
/// # Returns
/// A `MarkdownDocument` containing the parsed AST, or an error if parsing fails.
///
/// # Example
/// ```ignore
/// let doc = parse_markdown("# Hello\n\nWorld")?;
/// assert_eq!(doc.headings().len(), 1);
/// ```
pub fn parse_markdown(markdown: &str) -> Result<MarkdownDocument> {
    parse_markdown_with_options(markdown, &MarkdownOptions::default())
}

/// Parse markdown text with custom options.
///
/// # Arguments
/// * `markdown` - The markdown text to parse
/// * `options` - Parsing options
///
/// # Returns
/// A `MarkdownDocument` containing the parsed AST.
pub fn parse_markdown_with_options(
    markdown: &str,
    options: &MarkdownOptions,
) -> Result<MarkdownDocument> {
    let arena = Arena::new();
    let comrak_options = options.to_comrak_options();

    let root = parse_document(&arena, markdown, &comrak_options);

    // Convert comrak AST to our own structure
    let mut front_matter = None;
    let mut converted_root = convert_node(root, &mut front_matter)?;

    // Detect GitHub-style callouts (> [!TYPE]) within blockquotes and convert
    // them to dedicated Callout nodes for styled rendering.
    // IMPORTANT: This must run BEFORE merge_consecutive_blockquotes, otherwise
    // consecutive callouts separated by blank lines get merged into a single
    // blockquote and only the first [!TYPE] marker is detected.
    convert_callout_blockquotes(&mut converted_root);

    // Merge consecutive blockquote siblings into a single blockquote node.
    // This handles the case where the user separates blockquote paragraphs with
    // blank lines, which comrak parses as separate BlockQuote nodes. Merging
    // them produces a single continuous blockquote with a single border.
    // Note: Callout nodes are NOT BlockQuote nodes, so they won't be merged.
    merge_consecutive_blockquotes(&mut converted_root);

    // FIX: Comrak treats "- " (single dash + optional whitespace) as a setext
    // heading underline, but in a markdown editor the user is almost always
    // starting a list item. Detect these false setext headings and convert
    // them back to Paragraph + List(Item).
    fix_false_setext_headings(&mut converted_root, markdown);

    // Extract wikilinks from Text nodes: [[target]] and [[target|display text]]
    // This must run after all block-level transformations are complete.
    extract_wikilinks(&mut converted_root);
    crate::markdown::video_embed::extract_video_embeds(&mut converted_root);

    // FIX: Comrak returns line numbers as if frontmatter doesn't exist.
    // When frontmatter is present, we need to calculate the offset and adjust all line numbers.
    let line_offset = calculate_frontmatter_offset(&converted_root);
    let adjusted_root = if line_offset > 0 {
        adjust_line_numbers(converted_root, line_offset)
    } else {
        converted_root
    };

    Ok(MarkdownDocument {
        root: adjusted_root,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Post-Processing Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Merge consecutive `BlockQuote` siblings into a single blockquote node.
///
/// When the user writes blockquote paragraphs separated by blank lines:
/// ```markdown
/// > Line 1
///
/// > Line 2
/// ```
/// Comrak parses these as two separate `BlockQuote` nodes. This function
/// merges them into a single continuous blockquote so the renderer draws
/// one border instead of two.
fn merge_consecutive_blockquotes(node: &mut MarkdownNode) {
    // First, recursively process all children (depth-first)
    for child in &mut node.children {
        merge_consecutive_blockquotes(child);
    }

    // Then merge consecutive blockquote siblings at this level
    if node.children.len() < 2 {
        return;
    }

    let mut i = 0;
    while i < node.children.len().saturating_sub(1) {
        let is_current_bq = matches!(node.children[i].node_type, MarkdownNodeType::BlockQuote);
        let is_next_bq = matches!(node.children[i + 1].node_type, MarkdownNodeType::BlockQuote);

        if is_current_bq && is_next_bq {
            // Merge: move children from the next blockquote into the current one
            let next = node.children.remove(i + 1);
            let current = &mut node.children[i];
            current.end_line = next.end_line;
            current.children.extend(next.children);
            // Don't increment i — check if the following sibling is also a blockquote
        } else {
            i += 1;
        }
    }
}

/// Fix false setext headings that are actually empty list items.
///
/// Comrak interprets `"Some text\n- "` as a setext heading (level 2) because
/// a single `-` followed by optional whitespace is a valid setext underline.
/// However, in a markdown editor, the user is almost always starting a list
/// item when they type `- `. This function detects such cases and converts
/// the heading back to a Paragraph followed by a List containing an empty Item.
///
/// Note: `--` and `---` under text are legitimate setext H2 headings per the
/// CommonMark spec and are NOT corrected here.
fn fix_false_setext_headings(node: &mut MarkdownNode, source: &str) {
    // Recursively process children first
    for child in &mut node.children {
        fix_false_setext_headings(child, source);
    }

    let source_lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    let mut replacements: Vec<(usize, Vec<MarkdownNode>)> = Vec::new();

    while i < node.children.len() {
        let child = &node.children[i];
        if let MarkdownNodeType::Heading {
            level: HeadingLevel::H2,
            setext: true,
        } = &child.node_type
        {
            // Find the setext underline by scanning backwards from end_line.
            // Comrak's end_line can extend past the underline (including blank
            // lines or even the next paragraph), so we locate the last
            // all-dashes line in the heading's source range.
            let start_idx = child.start_line.saturating_sub(1); // 1-indexed to 0-indexed
            let end_idx = child.end_line; // 0-indexed exclusive

            let mut underline_info: Option<(&str, usize)> = None; // (trimmed, 1-indexed line)
            for idx in (start_idx..end_idx).rev() {
                if let Some(line) = source_lines.get(idx) {
                    let t = line.trim();
                    if !t.is_empty() && t.chars().all(|c| c == '-') {
                        underline_info = Some((t, idx + 1));
                        break;
                    }
                }
            }

            if let Some((trimmed, underline_line)) = underline_info {
                // Only a single `-` is treated as false setext (list item start).
                // `--`, `---`, etc. are legitimate setext H2 underlines per CommonMark.
                if trimmed == "-" {
                    let heading_start = child.start_line;
                    let children = child.children.clone();

                    let paragraph = MarkdownNode {
                        node_type: MarkdownNodeType::Paragraph,
                        children,
                        start_line: heading_start,
                        end_line: underline_line.saturating_sub(1).max(heading_start),
                    };

                    let list_item = MarkdownNode {
                        node_type: MarkdownNodeType::Item,
                        children: Vec::new(),
                        start_line: underline_line,
                        end_line: underline_line,
                    };

                    let list = MarkdownNode {
                        node_type: MarkdownNodeType::List {
                            list_type: ListType::Bullet,
                            tight: true,
                        },
                        children: vec![list_item],
                        start_line: underline_line,
                        end_line: underline_line,
                    };

                    replacements.push((i, vec![paragraph, list]));
                }
            }
        }
        i += 1;
    }

    // Apply replacements in reverse order so indices remain valid
    for (idx, replacement) in replacements.into_iter().rev() {
        node.children.splice(idx..=idx, replacement);
    }
}

/// Convert `BlockQuote` nodes containing `[!TYPE]` markers into `Callout` nodes.
///
/// GitHub-style callouts use the syntax:
/// ```markdown
/// > [!NOTE]
/// > Content here
/// ```
/// With optional custom title: `> [!WARNING] Custom Title`
/// And optional collapsed state: `> [!NOTE]-`
///
/// This function handles three cases:
/// 1. A single blockquote whose first paragraph starts with `[!TYPE]` → convert to Callout
/// 2. A single blockquote with multiple paragraphs, some starting with `[!TYPE]` → split & convert
/// 3. Consecutive separate blockquotes (handled naturally since this runs before merge)
fn convert_callout_blockquotes(node: &mut MarkdownNode) {
    // Recursively process all children depth-first
    for child in &mut node.children {
        convert_callout_blockquotes(child);
    }

    // Replace children, potentially splitting blockquotes that contain multiple callouts
    let old_children = std::mem::take(&mut node.children);
    let mut new_children = Vec::with_capacity(old_children.len());

    for child in old_children {
        if matches!(child.node_type, MarkdownNodeType::BlockQuote) {
            new_children.extend(split_and_convert_blockquote(child));
        } else {
            new_children.push(child);
        }
    }

    node.children = new_children;
}

/// Check if a paragraph node's first text child starts with a valid `[!TYPE]` marker.
fn paragraph_starts_with_callout(node: &MarkdownNode) -> bool {
    if !matches!(node.node_type, MarkdownNodeType::Paragraph) {
        return false;
    }
    if let Some(first_child) = node.children.first() {
        if let MarkdownNodeType::Text(t) = &first_child.node_type {
            let trimmed = t.trim_start();
            if trimmed.starts_with("[!") {
                if let Some(close_pos) = trimmed.find(']') {
                    let type_str = &trimmed[2..close_pos];
                    return CalloutType::from_str(type_str).is_some();
                }
            }
        }
    }
    false
}

/// Process a single blockquote: either convert it to a Callout, split it into
/// multiple Callout/BlockQuote nodes, or leave it as a plain BlockQuote.
fn split_and_convert_blockquote(mut blockquote: MarkdownNode) -> Vec<MarkdownNode> {
    // Find which paragraph children start with [!TYPE]
    let callout_starts: Vec<usize> = blockquote
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if paragraph_starts_with_callout(c) {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    match callout_starts.len() {
        0 => {
            // Regular blockquote, no callout markers
            vec![blockquote]
        }
        1 if callout_starts[0] == 0 => {
            // Single callout at the start — convert the whole blockquote
            if let Some((ct, title, collapsed)) = extract_callout_info(&mut blockquote) {
                blockquote.node_type = MarkdownNodeType::Callout {
                    callout_type: ct,
                    title,
                    collapsed,
                };
            }
            vec![blockquote]
        }
        _ => {
            // Multiple callout markers (or callout not at start) — split
            split_blockquote_at_callouts(blockquote, &callout_starts)
        }
    }
}

/// Split a single blockquote into multiple nodes at each `[!TYPE]` paragraph boundary.
///
/// This handles the case where the user writes multiple callouts inside a single
/// blockquote (separated by `>` blank lines but no un-quoted blank lines):
/// ```markdown
/// > [!NOTE]
/// > Note content
/// >
/// > [!TIP]
/// > Tip content
/// ```
fn split_blockquote_at_callouts(
    blockquote: MarkdownNode,
    callout_starts: &[usize],
) -> Vec<MarkdownNode> {
    let base_start = blockquote.start_line;
    let base_end = blockquote.end_line;
    let children = blockquote.children;
    let len = children.len();

    // Build section boundaries: (start_idx, end_idx, is_callout)
    let mut sections: Vec<(usize, usize, bool)> = Vec::new();

    // Any content before the first callout marker is a regular blockquote
    if callout_starts[0] > 0 {
        sections.push((0, callout_starts[0], false));
    }

    // Each callout section extends from its marker to the next marker (or end)
    for (i, &start) in callout_starts.iter().enumerate() {
        let end = callout_starts.get(i + 1).copied().unwrap_or(len);
        sections.push((start, end, true));
    }

    sections
        .iter()
        .map(|&(start_idx, end_idx, is_callout)| {
            let section: Vec<MarkdownNode> = children[start_idx..end_idx].to_vec();
            let s_line = section.first().map_or(base_start, |c| c.start_line);
            let e_line = section.last().map_or(base_end, |c| c.end_line);

            let mut node = MarkdownNode {
                node_type: MarkdownNodeType::BlockQuote,
                children: section,
                start_line: s_line,
                end_line: e_line,
            };

            if is_callout {
                if let Some((ct, title, collapsed)) = extract_callout_info(&mut node) {
                    node.node_type = MarkdownNodeType::Callout {
                        callout_type: ct,
                        title,
                        collapsed,
                    };
                }
            }

            node
        })
        .collect()
}

/// Try to extract callout info from a blockquote node.
/// Returns `Some((type, optional_title, collapsed))` if the blockquote
/// starts with a `[!TYPE]` marker in its first paragraph's first text node.
/// Also removes the marker text from the AST so only content remains.
fn extract_callout_info(
    blockquote: &mut MarkdownNode,
) -> Option<(CalloutType, Option<String>, bool)> {
    // The blockquote must have at least one child (a Paragraph)
    let first_child = blockquote.children.first_mut()?;
    if !matches!(first_child.node_type, MarkdownNodeType::Paragraph) {
        return None;
    }

    // The paragraph's first child should be a Text node
    let first_text = first_child.children.first_mut()?;
    let text = match &first_text.node_type {
        MarkdownNodeType::Text(t) => t.clone(),
        _ => return None,
    };

    // Try to match the pattern: [!TYPE] or [!TYPE]- with optional title
    let trimmed = text.trim_start();
    if !trimmed.starts_with("[!") {
        return None;
    }

    // Find the closing bracket
    let close_bracket = trimmed.find(']')?;
    let type_str = &trimmed[2..close_bracket];

    let callout_type = CalloutType::from_str(type_str)?;

    // Check for collapsed marker right after the closing bracket
    let after_bracket = &trimmed[close_bracket + 1..];
    let (collapsed, rest) = if after_bracket.starts_with('-') {
        (true, after_bracket[1..].trim_start())
    } else {
        (false, after_bracket.trim_start())
    };

    // Everything after the marker on the same line is the custom title
    let title = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };

    // Remove the [!TYPE] marker text from the first text node
    let remaining_text = text[trimmed.len()..].to_string();
    if remaining_text.is_empty() {
        // Remove the first text node entirely since it only contained the marker
        first_child.children.remove(0);
        // If the paragraph is now empty, remove it too
        if first_child.children.is_empty() {
            blockquote.children.remove(0);
        } else {
            // If there's a SoftBreak right after the removed text, remove it too
            if !first_child.children.is_empty()
                && matches!(
                    first_child.children[0].node_type,
                    MarkdownNodeType::SoftBreak
                )
            {
                first_child.children.remove(0);
            }
        }
    } else {
        first_text.node_type = MarkdownNodeType::Text(remaining_text);
    }

    Some((callout_type, title, collapsed))
}

// ─────────────────────────────────────────────────────────────────────────────
// Wikilink Extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Recursively walk the AST and split `Text` nodes that contain `[[...]]`
/// wikilink syntax into a sequence of `Text` and `Wikilink` nodes.
fn extract_wikilinks(node: &mut MarkdownNode) {
    // Recurse into children first (depth-first)
    for child in &mut node.children {
        extract_wikilinks(child);
    }

    // Now process this node's direct children: look for Text nodes containing [[...]]
    let old_children = std::mem::take(&mut node.children);
    let mut new_children = Vec::with_capacity(old_children.len());

    for child in old_children {
        if let MarkdownNodeType::Text(ref text) = child.node_type {
            if text.contains("[[") {
                let split = split_text_with_wikilinks(text, child.start_line, child.end_line);
                new_children.extend(split);
            } else {
                new_children.push(child);
            }
        } else {
            new_children.push(child);
        }
    }

    node.children = new_children;
}

/// Split a text string containing `[[...]]` patterns into a sequence of
/// `Text` and `Wikilink` nodes.
///
/// Handles:
/// - `[[target]]` → Wikilink { target, display: None }
/// - `[[target|display text]]` → Wikilink { target, display: Some("display text") }
/// - Unclosed `[[` is left as plain text
fn split_text_with_wikilinks(text: &str, start_line: usize, end_line: usize) -> Vec<MarkdownNode> {
    let mut result = Vec::new();
    let mut remaining = text;

    while let Some(open_pos) = remaining.find("[[") {
        // Push any text before the opening [[
        if open_pos > 0 {
            let before = &remaining[..open_pos];
            result.push(MarkdownNode {
                node_type: MarkdownNodeType::Text(before.to_string()),
                children: Vec::new(),
                start_line,
                end_line,
            });
        }

        let after_open = &remaining[open_pos + 2..];

        // Find the closing ]] — but don't cross newlines
        if let Some(close_pos) = after_open.find("]]") {
            let inner = &after_open[..close_pos];

            // Don't allow newlines inside wikilinks
            if inner.contains('\n') {
                // Malformed — push the [[ as text and continue
                result.push(MarkdownNode {
                    node_type: MarkdownNodeType::Text("[[".to_string()),
                    children: Vec::new(),
                    start_line,
                    end_line,
                });
                remaining = after_open;
                continue;
            }

            // Parse target and optional display text (split on first |)
            let (target, display) = if let Some(pipe_pos) = inner.find('|') {
                let t = inner[..pipe_pos].trim().to_string();
                let d = inner[pipe_pos + 1..].trim().to_string();
                (t, if d.is_empty() { None } else { Some(d) })
            } else {
                (inner.trim().to_string(), None)
            };

            // Only create a wikilink if the target is non-empty
            if !target.is_empty() {
                result.push(MarkdownNode {
                    node_type: MarkdownNodeType::Wikilink { target, display },
                    children: Vec::new(),
                    start_line,
                    end_line,
                });
            } else {
                // Empty target like [[]] — push as plain text
                result.push(MarkdownNode {
                    node_type: MarkdownNodeType::Text(format!("[[{}]]", inner)),
                    children: Vec::new(),
                    start_line,
                    end_line,
                });
            }

            remaining = &after_open[close_pos + 2..];
        } else {
            // No closing ]] found — push [[ as text and continue
            result.push(MarkdownNode {
                node_type: MarkdownNodeType::Text("[[".to_string()),
                children: Vec::new(),
                start_line,
                end_line,
            });
            remaining = after_open;
        }
    }

    // Push any remaining text
    if !remaining.is_empty() {
        result.push(MarkdownNode {
            node_type: MarkdownNodeType::Text(remaining.to_string()),
            children: Vec::new(),
            start_line,
            end_line,
        });
    }

    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal Conversion Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Calculate the line offset caused by frontmatter.
/// Comrak returns line numbers as if frontmatter doesn't exist, so we need to
/// find the frontmatter node and use its actual source line count as offset.
fn calculate_frontmatter_offset(root: &MarkdownNode) -> usize {
    // Look for a FrontMatter node at the start
    if let Some(first_child) = root.children.first() {
        if let MarkdownNodeType::FrontMatter(content) = &first_child.node_type {
            // Count lines in frontmatter content, plus 2 for the --- delimiters
            let content_lines = content.lines().count();
            // Frontmatter format: ---\ncontent\n---\n
            // The delimiters add 2 lines, but comrak might include them in content
            // We check if content starts/ends with --- to avoid double-counting
            let has_start_delimiter = content.starts_with("---");
            let has_end_delimiter = content.trim_end().ends_with("---");

            let delimiter_lines = match (has_start_delimiter, has_end_delimiter) {
                (true, true) => 0,   // Both included in content
                (true, false) => 1,  // Only start included
                (false, true) => 1,  // Only end included
                (false, false) => 2, // Neither included
            };

            return content_lines + delimiter_lines;
        }
    }
    0
}

/// Recursively adjust all line numbers in the AST by the given offset.
fn adjust_line_numbers(mut node: MarkdownNode, offset: usize) -> MarkdownNode {
    // Don't adjust the FrontMatter node itself (it should stay at line 0 or 1)
    if !matches!(node.node_type, MarkdownNodeType::FrontMatter(_)) {
        // Only adjust if the line numbers are non-zero (line 0 is special for document root)
        if node.start_line > 0 {
            node.start_line += offset;
        }
        if node.end_line > 0 {
            node.end_line += offset;
        }
    }

    // Recursively adjust children
    node.children = node
        .children
        .into_iter()
        .map(|child| adjust_line_numbers(child, offset))
        .collect();

    node
}

/// Convert a comrak AST node to our MarkdownNode structure.
fn convert_node<'a>(
    node: &'a AstNode<'a>,
    front_matter: &mut Option<String>,
) -> Result<MarkdownNode> {
    let ast = node.data.borrow();
    let sourcepos = ast.sourcepos;

    let node_type = convert_node_value(&ast.value, front_matter)?;

    let mut markdown_node = MarkdownNode::new(
        node_type,
        sourcepos.start.line,
        sourcepos.start.column,
        sourcepos.end.line,
        sourcepos.end.column,
    );

    // Convert children
    for child in node.children() {
        let child_node = convert_node(child, front_matter)?;
        markdown_node.children.push(child_node);
    }

    Ok(markdown_node)
}

/// Convert a comrak NodeValue to our MarkdownNodeType.
fn convert_node_value(
    value: &NodeValue,
    front_matter: &mut Option<String>,
) -> Result<MarkdownNodeType> {
    let node_type = match value {
        NodeValue::Document => MarkdownNodeType::Document,
        NodeValue::BlockQuote => MarkdownNodeType::BlockQuote,
        NodeValue::List(list) => {
            let list_type = match list.list_type {
                ComrakListType::Bullet => ListType::Bullet,
                ComrakListType::Ordered => ListType::Ordered {
                    start: list.start as u32,
                    delimiter: if list.delimiter == ListDelimType::Period {
                        '.'
                    } else {
                        ')'
                    },
                },
            };
            MarkdownNodeType::List {
                list_type,
                tight: list.tight,
            }
        }
        NodeValue::Item(_) => MarkdownNodeType::Item,
        NodeValue::CodeBlock(code) => MarkdownNodeType::CodeBlock {
            language: code.info.clone(),
            info: code.info.clone(),
            literal: code.literal.clone(),
        },
        NodeValue::HtmlBlock(html) => MarkdownNodeType::HtmlBlock(html.literal.clone()),
        NodeValue::Paragraph => MarkdownNodeType::Paragraph,
        NodeValue::Heading(heading) => MarkdownNodeType::Heading {
            level: HeadingLevel::from(heading.level),
            setext: heading.setext,
        },
        NodeValue::ThematicBreak => MarkdownNodeType::ThematicBreak,
        NodeValue::Table(table) => MarkdownNodeType::Table {
            alignments: table
                .alignments
                .iter()
                .map(|a| TableAlignment::from(*a))
                .collect(),
            num_columns: table.num_columns,
        },
        NodeValue::TableRow(header) => MarkdownNodeType::TableRow { header: *header },
        NodeValue::TableCell => MarkdownNodeType::TableCell,
        NodeValue::Text(text) => MarkdownNodeType::Text(text.clone()),
        NodeValue::TaskItem(checked) => MarkdownNodeType::TaskItem {
            checked: checked.map(|c| c == 'x' || c == 'X').unwrap_or(false),
        },
        NodeValue::SoftBreak => MarkdownNodeType::SoftBreak,
        NodeValue::LineBreak => MarkdownNodeType::LineBreak,
        NodeValue::Code(code) => MarkdownNodeType::Code(code.literal.clone()),
        NodeValue::HtmlInline(html) => MarkdownNodeType::HtmlInline(html.clone()),
        NodeValue::Emph => MarkdownNodeType::Emphasis,
        NodeValue::Strong => MarkdownNodeType::Strong,
        NodeValue::Strikethrough => MarkdownNodeType::Strikethrough,
        NodeValue::Superscript => MarkdownNodeType::Superscript,
        NodeValue::Link(link) => MarkdownNodeType::Link {
            url: link.url.clone(),
            title: link.title.clone(),
        },
        NodeValue::Image(image) => MarkdownNodeType::Image {
            url: image.url.clone(),
            title: image.title.clone(),
        },
        NodeValue::FootnoteReference(ref_data) => {
            MarkdownNodeType::FootnoteReference(ref_data.name.clone())
        }
        NodeValue::FootnoteDefinition(def) => {
            MarkdownNodeType::FootnoteDefinition(def.name.clone())
        }
        NodeValue::DescriptionList => MarkdownNodeType::DescriptionList,
        NodeValue::DescriptionItem(_) => MarkdownNodeType::DescriptionItem,
        NodeValue::DescriptionTerm => MarkdownNodeType::DescriptionTerm,
        NodeValue::DescriptionDetails => MarkdownNodeType::DescriptionDetails,
        NodeValue::FrontMatter(fm) => {
            *front_matter = Some(fm.clone());
            MarkdownNodeType::FrontMatter(fm.clone())
        }
        // Handle other node types that might be added in future versions
        _ => MarkdownNodeType::Text(String::new()),
    };

    Ok(node_type)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // Basic Parsing Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_empty_document() {
        let doc = parse_markdown("").unwrap();
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_parse_simple_paragraph() {
        let doc = parse_markdown("Hello, world!").unwrap();
        assert!(!doc.root.children.is_empty());
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::Paragraph
        ));
    }

    #[test]
    fn test_parse_heading_h1() {
        let doc = parse_markdown("# Heading 1").unwrap();
        assert!(!doc.root.children.is_empty());
        if let MarkdownNodeType::Heading { level, .. } = &doc.root.children[0].node_type {
            assert_eq!(*level, HeadingLevel::H1);
        } else {
            panic!("Expected heading node");
        }
    }

    #[test]
    fn test_parse_heading_h2() {
        let doc = parse_markdown("## Heading 2").unwrap();
        if let MarkdownNodeType::Heading { level, .. } = &doc.root.children[0].node_type {
            assert_eq!(*level, HeadingLevel::H2);
        } else {
            panic!("Expected heading node");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // List Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_unordered_list() {
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let doc = parse_markdown(markdown).unwrap();
        assert!(!doc.root.children.is_empty());

        let list = &doc.root.children[0];
        if let MarkdownNodeType::List { list_type, .. } = &list.node_type {
            assert!(matches!(list_type, ListType::Bullet));
        } else {
            panic!("Expected list node");
        }
        assert_eq!(list.children.len(), 3);
    }

    #[test]
    fn test_parse_ordered_list() {
        let markdown = "1. First\n2. Second\n3. Third";
        let doc = parse_markdown(markdown).unwrap();

        let list = &doc.root.children[0];
        if let MarkdownNodeType::List { list_type, .. } = &list.node_type {
            if let ListType::Ordered { start, .. } = list_type {
                assert_eq!(*start, 1);
            } else {
                panic!("Expected ordered list");
            }
        } else {
            panic!("Expected list node");
        }
    }

    #[test]
    fn test_parse_task_list() {
        let markdown = "- [ ] Unchecked\n- [x] Checked";
        let doc = parse_markdown(markdown).unwrap();

        let list = &doc.root.children[0];
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_task_list_with_formatting() {
        // Test task list with inline formatting (bold, links, code)
        // This is the pattern that was failing to render in the preview
        let markdown = "- [ ] **Bold text** ([link](https://example.com)) - description `code`";
        let doc = parse_markdown(markdown).unwrap();

        // Should have one list
        assert_eq!(
            doc.root.children.len(),
            1,
            "Expected 1 root child (the list)"
        );

        let list = &doc.root.children[0];
        assert!(
            matches!(list.node_type, MarkdownNodeType::List { .. }),
            "Expected List, got {:?}",
            list.node_type
        );

        // Should have one child (could be Item or TaskItem depending on AST structure)
        assert_eq!(list.children.len(), 1, "Expected 1 list child");

        let list_child = &list.children[0];
        // Note: In comrak's AST for task lists, the list child can be either:
        // - Item (with TaskItem as a child) in some versions
        // - TaskItem directly (in current version)
        // Our rendering code handles both cases
        let is_valid_list_item = matches!(
            list_child.node_type,
            MarkdownNodeType::Item | MarkdownNodeType::TaskItem { .. }
        );
        assert!(
            is_valid_list_item,
            "Expected Item or TaskItem, got {:?}",
            list_child.node_type
        );

        // Check for task item marker (either the node itself is TaskItem, or it has TaskItem child)
        let is_task_marked = matches!(list_child.node_type, MarkdownNodeType::TaskItem { .. })
            || list_child
                .children
                .iter()
                .any(|c| matches!(c.node_type, MarkdownNodeType::TaskItem { .. }));
        assert!(
            is_task_marked,
            "Task list should have TaskItem marker. Node type: {:?}, Children: {:?}",
            list_child.node_type,
            list_child
                .children
                .iter()
                .map(|c| format!("{:?}", c.node_type))
                .collect::<Vec<_>>()
        );

        // Should have a Paragraph child containing the text
        let para_node = list_child
            .children
            .iter()
            .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));
        assert!(
            para_node.is_some(),
            "Task list item should have Paragraph child. Children types: {:?}",
            list_child
                .children
                .iter()
                .map(|c| format!("{:?}", c.node_type))
                .collect::<Vec<_>>()
        );

        let para = para_node.unwrap();

        // Paragraph should have children (not empty)
        assert!(
            !para.children.is_empty(),
            "Paragraph should have children. Para: {:?}",
            para
        );

        // Paragraph should contain Strong (bold) element
        let has_strong = para
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Strong));
        assert!(
            has_strong,
            "Paragraph should contain Strong node. Children: {:?}",
            para.children
                .iter()
                .map(|c| format!("{:?}", c.node_type))
                .collect::<Vec<_>>()
        );

        // Paragraph should contain Link element
        let has_link = para
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Link { .. }));
        assert!(
            has_link,
            "Paragraph should contain Link node. Children: {:?}",
            para.children
                .iter()
                .map(|c| format!("{:?}", c.node_type))
                .collect::<Vec<_>>()
        );

        // Text content should be preserved
        let text = para.text_content();
        assert!(
            text.contains("Bold text"),
            "Should contain 'Bold text', got: '{}'",
            text
        );
    }

    #[test]
    fn test_parse_tight_task_list_structure() {
        // Test a tight task list (no blank lines between items) - similar to ROADMAP.md
        let markdown = "#### Bug Fixes\n- [ ] **First issue** - description\n- [ ] **Second issue** - more text";
        let doc = parse_markdown(markdown).unwrap();

        // First child should be a heading
        assert!(matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::Heading { .. }
        ));

        // Second child should be a list
        let list = &doc.root.children[1];
        assert!(
            matches!(list.node_type, MarkdownNodeType::List { .. }),
            "Expected List, got {:?}",
            list.node_type
        );

        // List should have 2 items
        assert_eq!(list.children.len(), 2, "List should have 2 items");

        // Each item should be Item or TaskItem with Paragraph children
        for (i, list_child) in list.children.iter().enumerate() {
            // Can be either Item (with TaskItem child) or TaskItem directly
            let is_valid_list_item = matches!(
                list_child.node_type,
                MarkdownNodeType::Item | MarkdownNodeType::TaskItem { .. }
            );
            assert!(
                is_valid_list_item,
                "List child {} should be Item or TaskItem, got {:?}",
                i, list_child.node_type
            );

            // Check for task marker (either the node itself or as child)
            let is_task_marked = matches!(list_child.node_type, MarkdownNodeType::TaskItem { .. })
                || list_child
                    .children
                    .iter()
                    .any(|c| matches!(c.node_type, MarkdownNodeType::TaskItem { .. }));
            let has_para = list_child
                .children
                .iter()
                .any(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

            assert!(
                is_task_marked,
                "Item {} should be marked as task. Children: {:?}",
                i,
                list_child
                    .children
                    .iter()
                    .map(|c| format!("{:?}", c.node_type))
                    .collect::<Vec<_>>()
            );
            assert!(
                has_para,
                "Item {} should have Paragraph. Children: {:?}",
                i,
                list_child
                    .children
                    .iter()
                    .map(|c| format!("{:?}", c.node_type))
                    .collect::<Vec<_>>()
            );

            // Check paragraph has content
            if let Some(para) = list_child
                .children
                .iter()
                .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph))
            {
                assert!(
                    !para.children.is_empty(),
                    "Item {} paragraph should have children",
                    i
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Inline Element Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_bold_text() {
        let markdown = "This is **bold** text";
        let doc = parse_markdown(markdown).unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("bold"));
    }

    #[test]
    fn test_parse_bold_text_ast_structure() {
        // Verify the AST structure for **bold** includes a Strong node
        let markdown = "This is **bold** text";
        let doc = parse_markdown(markdown).unwrap();

        // Should have one paragraph
        assert_eq!(doc.root.children.len(), 1);
        let para = &doc.root.children[0];
        assert!(
            matches!(para.node_type, MarkdownNodeType::Paragraph),
            "Expected Paragraph, got {:?}",
            para.node_type
        );

        // Paragraph should have children including a Strong node
        let has_strong = para
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Strong));
        assert!(
            has_strong,
            "Paragraph should contain Strong node. Children: {:?}",
            para.children
                .iter()
                .map(|c| &c.node_type)
                .collect::<Vec<_>>()
        );

        // Find the Strong node and verify it has text content
        let strong_node = para
            .children
            .iter()
            .find(|c| matches!(c.node_type, MarkdownNodeType::Strong))
            .unwrap();
        assert_eq!(strong_node.text_content(), "bold");
    }

    #[test]
    fn test_parse_italic_text() {
        let markdown = "This is *italic* text";
        let doc = parse_markdown(markdown).unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("italic"));
    }

    #[test]
    fn test_parse_inline_code() {
        let markdown = "Use `code` inline";
        let doc = parse_markdown(markdown).unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("code"));
    }

    #[test]
    fn test_parse_strikethrough() {
        let markdown = "This is ~~deleted~~ text";
        let doc = parse_markdown(markdown).unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("deleted"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Nested Emphasis Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_bold_italic_triple_asterisk() {
        // ***bold italic*** should produce nested Strong > Emphasis > Text
        let markdown = "***bold italic***";
        let doc = parse_markdown(markdown).unwrap();

        // Verify text content is preserved
        let text = doc.root.text_content();
        assert!(text.contains("bold italic"));

        // Verify AST structure: Paragraph > Strong > Emphasis > Text
        let para = &doc.root.children[0];
        assert!(matches!(para.node_type, MarkdownNodeType::Paragraph));

        // First child of paragraph should be Strong or Emphasis
        let first_inline = &para.children[0];
        let is_strong_or_emph = matches!(
            first_inline.node_type,
            MarkdownNodeType::Strong | MarkdownNodeType::Emphasis
        );
        assert!(is_strong_or_emph, "Expected Strong or Emphasis node");

        // Verify nested structure exists
        assert!(
            !first_inline.children.is_empty(),
            "Nested emphasis should have children"
        );
    }

    #[test]
    fn test_parse_bold_inside_italic() {
        // *__bold inside italic__* or _**bold inside italic**_
        let markdown = "_**bold inside italic**_";
        let doc = parse_markdown(markdown).unwrap();

        let text = doc.root.text_content();
        assert!(text.contains("bold inside italic"));

        // Verify we have nested structure
        let para = &doc.root.children[0];
        let first_inline = &para.children[0];
        assert!(
            !first_inline.children.is_empty(),
            "Should have nested children"
        );
    }

    #[test]
    fn test_parse_italic_inside_bold() {
        // **_italic inside bold_** or __*italic inside bold*__
        let markdown = "**_italic inside bold_**";
        let doc = parse_markdown(markdown).unwrap();

        let text = doc.root.text_content();
        assert!(text.contains("italic inside bold"));

        // Verify AST has nested structure
        let para = &doc.root.children[0];
        let first_inline = &para.children[0];
        assert!(
            !first_inline.children.is_empty(),
            "Should have nested children"
        );
    }

    #[test]
    fn test_parse_mixed_emphasis_in_sentence() {
        let markdown = "This has **bold**, *italic*, and ***both***.";
        let doc = parse_markdown(markdown).unwrap();

        let text = doc.root.text_content();
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
        assert!(text.contains("both"));
    }

    #[test]
    fn test_parse_underscore_emphasis() {
        // Test underscore variants work the same as asterisks
        let markdown = "__bold__ and _italic_ and ___both___";
        let doc = parse_markdown(markdown).unwrap();

        let text = doc.root.text_content();
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
        assert!(text.contains("both"));
    }

    #[test]
    fn test_parse_strikethrough_with_bold() {
        let markdown = "~~**bold strikethrough**~~";
        let doc = parse_markdown(markdown).unwrap();

        let text = doc.root.text_content();
        assert!(text.contains("bold strikethrough"));

        // Verify nested structure
        let para = &doc.root.children[0];
        assert!(!para.children.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Table Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_table() {
        let markdown = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let doc = parse_markdown(markdown).unwrap();

        // Find the table node
        let table = doc
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, MarkdownNodeType::Table { .. }));
        assert!(table.is_some());

        if let MarkdownNodeType::Table { num_columns, .. } = &table.unwrap().node_type {
            assert_eq!(*num_columns, 2);
        }
    }

    #[test]
    fn test_parse_table_with_alignment() {
        let markdown =
            "| Left | Center | Right |\n|:-----|:------:|------:|\n| L    | C      | R     |";
        let doc = parse_markdown(markdown).unwrap();

        let table = doc
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, MarkdownNodeType::Table { .. }));
        assert!(table.is_some());

        if let MarkdownNodeType::Table { alignments, .. } = &table.unwrap().node_type {
            assert_eq!(alignments.len(), 3);
            assert_eq!(alignments[0], TableAlignment::Left);
            assert_eq!(alignments[1], TableAlignment::Center);
            assert_eq!(alignments[2], TableAlignment::Right);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Block Quote Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_blockquote() {
        let markdown = "> This is a quote";
        let doc = parse_markdown(markdown).unwrap();

        assert!(!doc.root.children.is_empty());
        assert!(matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::BlockQuote
        ));
    }

    #[test]
    fn test_parse_nested_blockquote() {
        let markdown = "> Level 1\n>> Level 2";
        let doc = parse_markdown(markdown).unwrap();
        assert!(matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::BlockQuote
        ));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Thematic Break Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_horizontal_rule() {
        let markdown = "Before\n\n---\n\nAfter";
        let doc = parse_markdown(markdown).unwrap();

        let hr = doc
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, MarkdownNodeType::ThematicBreak));
        assert!(hr.is_some());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Node Helper Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_text_content() {
        let doc = parse_markdown("Hello **world**!").unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Error Handling Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_malformed_markdown() {
        // Comrak is very permissive - even "malformed" markdown parses
        // This test ensures we don't crash on unusual input
        let inputs = [
            "# Unclosed heading",
            "```\nunclosed code block",
            "| broken | table",
            "[unclosed link(",
            "![broken image",
            "***nested emphasis**",
        ];

        for input in inputs {
            let result = parse_markdown(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Position Information Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_source_position() {
        let doc = parse_markdown("# Heading\n\nParagraph").unwrap();

        // First child (heading) should start at line 1
        let heading = &doc.root.children[0];
        assert_eq!(heading.start_line, 1);
    }

    #[test]
    fn test_list_item_structure() {
        // Test tight list (no blank lines between items)
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let doc = parse_markdown(markdown).unwrap();

        let list = &doc.root.children[0];
        assert!(matches!(list.node_type, MarkdownNodeType::List { .. }));

        // Check the first list item
        let first_item = &list.children[0];
        assert!(matches!(first_item.node_type, MarkdownNodeType::Item));

        // List item should have exactly one child (Paragraph)
        assert_eq!(first_item.children.len(), 1);

        // The list item should have a Paragraph child (even for tight lists in comrak)
        let has_paragraph = first_item
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));
        assert!(has_paragraph, "List item should have Paragraph child");

        // Get the paragraph and check it has the text
        let para = first_item
            .children
            .iter()
            .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph))
            .unwrap();
        assert_eq!(para.text_content(), "Item 1");

        // Check text content is accessible from the item node
        let text_content = first_item.text_content();
        assert_eq!(text_content, "Item 1");
    }

    #[test]
    fn test_loose_list_item_structure() {
        // Test loose list (blank lines between items)
        let markdown = "- Item 1\n\n- Item 2\n\n- Item 3";
        let doc = parse_markdown(markdown).unwrap();

        let list = &doc.root.children[0];
        assert!(matches!(list.node_type, MarkdownNodeType::List { .. }));

        // Check the first list item
        let first_item = &list.children[0];
        assert!(matches!(first_item.node_type, MarkdownNodeType::Item));

        // The list item should have a Paragraph child
        let has_paragraph = first_item
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));
        assert!(
            has_paragraph,
            "Loose list items should have Paragraph children"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Callout / Admonition Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_note_callout() {
        let markdown = "> [!NOTE]\n> This is a note";
        let doc = parse_markdown(markdown).unwrap();
        assert!(!doc.root.children.is_empty());

        let node = &doc.root.children[0];
        if let MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } = &node.node_type
        {
            assert_eq!(*callout_type, CalloutType::Note);
            assert!(title.is_none());
            assert!(!collapsed);
        } else {
            panic!("Expected Callout node, got {:?}", node.node_type);
        }

        // Should have content
        let text = node.text_content();
        assert!(
            text.contains("This is a note"),
            "Callout should contain content text, got: '{}'",
            text
        );
    }

    #[test]
    fn test_parse_warning_callout_with_custom_title() {
        let markdown = "> [!WARNING] Custom Title\n> Be careful here";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        if let MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } = &node.node_type
        {
            assert_eq!(*callout_type, CalloutType::Warning);
            assert_eq!(title.as_deref(), Some("Custom Title"));
            assert!(!collapsed);
        } else {
            panic!("Expected Callout node, got {:?}", node.node_type);
        }
    }

    #[test]
    fn test_parse_collapsed_callout() {
        let markdown = "> [!NOTE]-\n> This is collapsed by default";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        if let MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } = &node.node_type
        {
            assert_eq!(*callout_type, CalloutType::Note);
            assert!(title.is_none());
            assert!(collapsed, "Should be collapsed by default");
        } else {
            panic!("Expected Callout node, got {:?}", node.node_type);
        }
    }

    #[test]
    fn test_parse_collapsed_callout_with_title() {
        let markdown = "> [!TIP]- Click to expand\n> Hidden content";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        if let MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } = &node.node_type
        {
            assert_eq!(*callout_type, CalloutType::Tip);
            assert_eq!(title.as_deref(), Some("Click to expand"));
            assert!(collapsed);
        } else {
            panic!("Expected Callout node, got {:?}", node.node_type);
        }
    }

    #[test]
    fn test_parse_all_callout_types() {
        let types = vec![
            ("NOTE", CalloutType::Note),
            ("TIP", CalloutType::Tip),
            ("WARNING", CalloutType::Warning),
            ("CAUTION", CalloutType::Caution),
            ("IMPORTANT", CalloutType::Important),
        ];

        for (type_str, expected_type) in types {
            let markdown = format!("> [!{}]\n> Content", type_str);
            let doc = parse_markdown(&markdown).unwrap();

            let node = &doc.root.children[0];
            if let MarkdownNodeType::Callout { callout_type, .. } = &node.node_type {
                assert_eq!(
                    *callout_type, expected_type,
                    "Failed for type: {}",
                    type_str
                );
            } else {
                panic!(
                    "Expected Callout node for {}, got {:?}",
                    type_str, node.node_type
                );
            }
        }
    }

    #[test]
    fn test_parse_callout_case_insensitive() {
        // GitHub callout types should be case-insensitive
        let markdown = "> [!note]\n> lowercase note";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        assert!(
            matches!(
                &node.node_type,
                MarkdownNodeType::Callout {
                    callout_type: CalloutType::Note,
                    ..
                }
            ),
            "Callout types should be case-insensitive, got {:?}",
            node.node_type
        );
    }

    #[test]
    fn test_parse_regular_blockquote_not_callout() {
        // A regular blockquote should NOT become a callout
        let markdown = "> Just a regular quote";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        assert!(
            matches!(node.node_type, MarkdownNodeType::BlockQuote),
            "Regular blockquote should remain BlockQuote, got {:?}",
            node.node_type
        );
    }

    #[test]
    fn test_parse_unknown_callout_type_stays_blockquote() {
        // An unknown type like [!UNKNOWN] should stay as a regular blockquote
        let markdown = "> [!UNKNOWN]\n> Content";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        assert!(
            matches!(node.node_type, MarkdownNodeType::BlockQuote),
            "Unknown callout type should remain BlockQuote, got {:?}",
            node.node_type
        );
    }

    #[test]
    fn test_callout_with_multiline_content() {
        let markdown = "> [!NOTE]\n> Line 1\n> Line 2\n> Line 3";
        let doc = parse_markdown(markdown).unwrap();

        let node = &doc.root.children[0];
        assert!(
            matches!(
                &node.node_type,
                MarkdownNodeType::Callout {
                    callout_type: CalloutType::Note,
                    ..
                }
            ),
            "Expected Callout, got {:?}",
            node.node_type
        );

        let text = node.text_content();
        assert!(text.contains("Line 1"), "Should have Line 1");
        assert!(text.contains("Line 3"), "Should have Line 3");
    }

    #[test]
    fn test_adjacent_callouts_separated_by_paragraph() {
        let markdown = "> [!WARNING]-\n> HEI\n\nas\n\n> [!TIP] Test\n> Hvor er jeg?\n\n- list item";
        let doc = parse_markdown(markdown).unwrap();

        assert_eq!(
            doc.root.children.len(),
            4,
            "Expected heading-less doc with warning callout, paragraph, tip callout, list; got {} nodes",
            doc.root.children.len()
        );

        assert!(
            matches!(
                doc.root.children[0].node_type,
                MarkdownNodeType::Callout {
                    callout_type: CalloutType::Warning,
                    ..
                }
            ),
            "First node should be WARNING callout"
        );
        assert!(
            matches!(doc.root.children[1].node_type, MarkdownNodeType::Paragraph),
            "Second node should be plain paragraph 'as'"
        );
        assert!(
            matches!(
                doc.root.children[2].node_type,
                MarkdownNodeType::Callout {
                    callout_type: CalloutType::Tip,
                    ..
                }
            ),
            "Third node should be TIP callout"
        );
        assert!(
            matches!(doc.root.children[3].node_type, MarkdownNodeType::List { .. }),
            "Fourth node should be a list"
        );

        let warning = &doc.root.children[0];
        assert!(
            !warning.text_content().contains("Hvor er jeg"),
            "Warning callout must not swallow the tip callout body"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Wikilink Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_wikilink() {
        let doc = parse_markdown("Check [[note-b]] for details").unwrap();
        let para = &doc.root.children[0];
        assert!(matches!(para.node_type, MarkdownNodeType::Paragraph));

        let has_wikilink = para.children.iter().any(|c| {
            matches!(
                &c.node_type,
                MarkdownNodeType::Wikilink { target, display } if target == "note-b" && display.is_none()
            )
        });
        assert!(
            has_wikilink,
            "Should contain a Wikilink node. Children: {:?}",
            para.children
                .iter()
                .map(|c| format!("{:?}", c.node_type))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_wikilink_with_display_text() {
        let doc = parse_markdown("See [[note-b|Custom Text]] here").unwrap();
        let para = &doc.root.children[0];

        let wikilink = para
            .children
            .iter()
            .find(|c| matches!(&c.node_type, MarkdownNodeType::Wikilink { .. }));
        assert!(wikilink.is_some(), "Should have a Wikilink node");

        if let MarkdownNodeType::Wikilink { target, display } = &wikilink.unwrap().node_type {
            assert_eq!(target, "note-b");
            assert_eq!(display.as_deref(), Some("Custom Text"));
        }
    }

    #[test]
    fn test_parse_wikilink_with_spaces() {
        let doc = parse_markdown("Open [[My Document]] now").unwrap();
        let para = &doc.root.children[0];

        let wikilink = para
            .children
            .iter()
            .find(|c| matches!(&c.node_type, MarkdownNodeType::Wikilink { .. }));
        assert!(wikilink.is_some());

        if let MarkdownNodeType::Wikilink { target, display } = &wikilink.unwrap().node_type {
            assert_eq!(target, "My Document");
            assert!(display.is_none());
        }
    }

    #[test]
    fn test_parse_multiple_wikilinks() {
        let doc = parse_markdown("Link [[a]] and [[b|B text]] here").unwrap();
        let para = &doc.root.children[0];

        let wikilinks: Vec<_> = para
            .children
            .iter()
            .filter(|c| matches!(&c.node_type, MarkdownNodeType::Wikilink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 2, "Should have 2 wikilinks");
    }

    #[test]
    fn test_parse_wikilink_text_content() {
        let doc = parse_markdown("[[note-b|Display]]").unwrap();
        let text = doc.root.text_content();
        assert!(
            text.contains("Display"),
            "text_content should use display text"
        );

        let doc2 = parse_markdown("[[note-b]]").unwrap();
        let text2 = doc2.root.text_content();
        assert!(
            text2.contains("note-b"),
            "text_content should fall back to target"
        );
    }

    #[test]
    fn test_parse_unclosed_wikilink() {
        let doc = parse_markdown("This [[unclosed stays as text").unwrap();
        let text = doc.root.text_content();
        assert!(text.contains("[["), "Unclosed [[ should remain as text");

        let has_wikilink = doc.root.children.iter().any(|c| {
            c.children
                .iter()
                .any(|cc| matches!(&cc.node_type, MarkdownNodeType::Wikilink { .. }))
        });
        assert!(
            !has_wikilink,
            "Unclosed [[ should NOT produce a Wikilink node"
        );
    }

    #[test]
    fn test_parse_empty_wikilink() {
        let doc = parse_markdown("This [[]] is empty").unwrap();
        let has_wikilink = doc.root.children.iter().any(|c| {
            c.children
                .iter()
                .any(|cc| matches!(&cc.node_type, MarkdownNodeType::Wikilink { .. }))
        });
        assert!(
            !has_wikilink,
            "Empty [[]] should NOT produce a Wikilink node"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Setext Heading Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_setext_h2_double_dash_is_valid() {
        // "Text\n--" is a legitimate setext H2 per CommonMark spec
        let doc = parse_markdown("Heading\n--").unwrap();

        let first = &doc.root.children[0];
        assert!(
            matches!(
                first.node_type,
                MarkdownNodeType::Heading {
                    level: HeadingLevel::H2,
                    setext: true
                }
            ),
            "\"Text\\n--\" should produce a setext H2, got {:?}",
            first.node_type
        );
        assert_eq!(first.text_content(), "Heading");
    }

    #[test]
    fn test_setext_h2_triple_dash_is_valid() {
        // "Text\n---" directly under text is a setext H2 (not a thematic break)
        let doc = parse_markdown("Heading\n---").unwrap();

        let first = &doc.root.children[0];
        assert!(
            matches!(
                first.node_type,
                MarkdownNodeType::Heading {
                    level: HeadingLevel::H2,
                    setext: true
                }
            ),
            "\"Text\\n---\" should produce a setext H2, got {:?}",
            first.node_type
        );
    }

    #[test]
    fn test_triple_dash_horizontal_rule_with_blank_lines() {
        // "---" between blank lines is a thematic break, not a heading
        let doc = parse_markdown("Before\n\n---\n\nAfter").unwrap();

        let hr = doc
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, MarkdownNodeType::ThematicBreak));
        assert!(
            hr.is_some(),
            "--- between blank lines should produce ThematicBreak"
        );
    }

    #[test]
    fn test_yaml_frontmatter_preserved() {
        let markdown = "---\ntitle: Test\ndate: 2026-01-01\n---\n\n# Hello";
        let doc = parse_markdown(markdown).unwrap();

        let has_frontmatter = doc
            .root
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::FrontMatter(_)));
        assert!(
            has_frontmatter,
            "YAML frontmatter delimiters should still parse correctly"
        );

        let has_heading = doc.root.children.iter().any(|c| {
            matches!(
                c.node_type,
                MarkdownNodeType::Heading {
                    level: HeadingLevel::H1,
                    ..
                }
            )
        });
        assert!(has_heading, "Heading after frontmatter should still parse");
    }

    #[test]
    fn test_single_dash_false_setext_still_works() {
        // A single `-` under text is ambiguous with list item start;
        // the editor treats it as Paragraph + List, not as a heading.
        let doc = parse_markdown("Text\n-").unwrap();

        let has_heading = doc
            .root
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Heading { .. }));
        assert!(!has_heading, "Single dash should not produce a heading");

        assert!(
            matches!(doc.root.children[0].node_type, MarkdownNodeType::Paragraph),
            "First child should be Paragraph"
        );
    }

    #[test]
    fn test_single_dash_false_setext_in_longer_doc() {
        // The single-dash fix must work even when comrak's end_line extends
        // past the underline (e.g. includes trailing blank lines).
        let doc = parse_markdown("Text\n-\n\nMore text").unwrap();

        let has_heading = doc
            .root
            .children
            .iter()
            .any(|c| matches!(c.node_type, MarkdownNodeType::Heading { .. }));
        assert!(
            !has_heading,
            "Single dash should not produce a heading even with trailing content"
        );

        let text = doc.root.text_content();
        assert!(text.contains("Text"), "Should preserve 'Text'");
        assert!(text.contains("More text"), "Should preserve 'More text'");
    }

    #[test]
    fn test_multiline_setext_h2() {
        // Multi-line text followed by -- is a valid setext heading
        let doc = parse_markdown("Line 1\nLine 2\nLine 3\n--").unwrap();

        let first = &doc.root.children[0];
        assert!(
            matches!(
                first.node_type,
                MarkdownNodeType::Heading {
                    level: HeadingLevel::H2,
                    setext: true
                }
            ),
            "Multi-line text with -- underline should be setext H2, got {:?}",
            first.node_type
        );
        let text = first.text_content();
        assert!(text.contains("Line 1"), "Should contain 'Line 1'");
        assert!(text.contains("Line 3"), "Should contain 'Line 3'");
    }
}
