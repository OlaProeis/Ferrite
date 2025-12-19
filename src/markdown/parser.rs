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

/// Represents the type of a markdown node.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownNodeType {
    /// Root document node
    Document,
    /// Block quote (>)
    BlockQuote,
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
    #[allow(dead_code)]
    /// Original source text
    source: String,
    #[allow(dead_code)]
    /// Front matter content if present
    front_matter: Option<String>,
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
    let converted_root = convert_node(root, &mut front_matter)?;

    Ok(MarkdownDocument {
        root: converted_root,
        source: markdown.to_string(),
        front_matter,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal Conversion Functions
// ─────────────────────────────────────────────────────────────────────────────

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
}
