//! Editable Markdown Widgets
//!
//! This module provides standalone editable widgets for markdown elements
//! that synchronize changes back to the markdown source through the AST.
//!
//! # Widgets
//! - `EditableHeading` - H1-H6 headings with level controls
//! - `EditableParagraph` - Multi-line paragraph editing
//! - `EditableList` - Ordered and unordered lists with item management
//!
//! Each widget operates on markdown AST nodes and returns the modified
//! markdown text when changes are made.

// Allow dead code for WYSIWYG widgets that are designed but not yet fully integrated
#![allow(dead_code)]

use crate::config::Theme;
use crate::markdown::parser::{HeadingLevel, ListType, MarkdownNode, MarkdownNodeType};
use eframe::egui::{self, Color32, FontId, RichText, TextEdit, Ui};

// ─────────────────────────────────────────────────────────────────────────────
// Widget Output
// ─────────────────────────────────────────────────────────────────────────────

/// Output from an editable markdown widget.
#[derive(Debug, Clone)]
pub struct WidgetOutput {
    /// Whether the content was modified
    pub changed: bool,
    /// The new markdown text for this element
    pub markdown: String,
}

impl WidgetOutput {
    /// Create an unchanged output with the given markdown.
    pub fn unchanged(markdown: String) -> Self {
        Self {
            changed: false,
            markdown,
        }
    }

    /// Create a changed output with the new markdown.
    pub fn modified(markdown: String) -> Self {
        Self {
            changed: true,
            markdown,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme-aware Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for markdown widgets based on theme.
#[derive(Debug, Clone)]
pub struct WidgetColors {
    pub text: Color32,
    pub heading: Color32,
    pub code_bg: Color32,
    pub list_marker: Color32,
    pub muted: Color32,
}

impl WidgetColors {
    /// Create colors for the given theme.
    pub fn from_theme(theme: Theme, visuals: &egui::Visuals) -> Self {
        let is_dark = match theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => visuals.dark_mode,
        };

        if is_dark {
            Self {
                text: Color32::from_rgb(220, 220, 220),
                heading: Color32::from_rgb(100, 180, 255),
                code_bg: Color32::from_rgb(45, 45, 45),
                list_marker: Color32::from_rgb(150, 150, 150),
                muted: Color32::from_rgb(120, 120, 120),
            }
        } else {
            Self {
                text: Color32::from_rgb(30, 30, 30),
                heading: Color32::from_rgb(0, 100, 180),
                code_bg: Color32::from_rgb(245, 245, 245),
                list_marker: Color32::from_rgb(100, 100, 100),
                muted: Color32::from_rgb(150, 150, 150),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable Heading Widget
// ─────────────────────────────────────────────────────────────────────────────

/// An editable heading widget (H1-H6) that syncs to markdown.
///
/// This widget renders a heading with:
/// - Visual level indicator (# symbols)
/// - Scaled font size based on level
/// - Inline text editing
/// - Outputs markdown string on change
///
/// # Example
///
/// ```ignore
/// let mut text = "My Heading".to_string();
/// let mut level = HeadingLevel::H1;
///
/// let output = EditableHeading::new(&mut text, &mut level)
///     .font_size(14.0)
///     .show(ui);
///
/// if output.changed {
///     // output.markdown contains "# My Heading"
/// }
/// ```
pub struct EditableHeading<'a> {
    /// The heading text (without # prefix)
    text: &'a mut String,
    /// The heading level
    level: &'a mut HeadingLevel,
    /// Base font size
    font_size: f32,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Whether to show level controls
    show_level_controls: bool,
}

impl<'a> EditableHeading<'a> {
    /// Create a new editable heading widget.
    pub fn new(text: &'a mut String, level: &'a mut HeadingLevel) -> Self {
        Self {
            text,
            level,
            font_size: 14.0,
            colors: None,
            show_level_controls: false,
        }
    }

    /// Set the base font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Enable level controls (buttons to change H1-H6).
    #[must_use]
    pub fn with_level_controls(mut self) -> Self {
        self.show_level_controls = true;
        self
    }

    /// Show the heading widget and return the output.
    pub fn show(self, ui: &mut Ui) -> WidgetOutput {
        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        let original_text = self.text.clone();
        let original_level = *self.level;

        // Calculate font size based on heading level
        let heading_font_size = match *self.level {
            HeadingLevel::H1 => self.font_size * 2.0,
            HeadingLevel::H2 => self.font_size * 1.75,
            HeadingLevel::H3 => self.font_size * 1.5,
            HeadingLevel::H4 => self.font_size * 1.25,
            HeadingLevel::H5 => self.font_size * 1.1,
            HeadingLevel::H6 => self.font_size,
        };

        let mut changed = false;

        ui.horizontal(|ui| {
            // Level indicator (non-editable)
            let prefix = "#".repeat(*self.level as usize);
            ui.label(
                RichText::new(&prefix)
                    .color(colors.muted)
                    .font(FontId::monospace(heading_font_size * 0.7)),
            );

            ui.add_space(8.0);

            // Level controls (if enabled)
            if self.show_level_controls {
                if ui
                    .small_button("−")
                    .on_hover_text("Decrease level")
                    .clicked()
                {
                    *self.level = decrease_heading_level(*self.level);
                    changed = true;
                }
                if ui
                    .small_button("+")
                    .on_hover_text("Increase level")
                    .clicked()
                {
                    *self.level = increase_heading_level(*self.level);
                    changed = true;
                }
                ui.add_space(4.0);
            }

            // Editable heading text
            let response = ui.add(
                TextEdit::singleline(self.text)
                    .font(FontId::proportional(heading_font_size))
                    .text_color(colors.heading)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );

            if response.changed() {
                changed = true;
            }
        });

        // Generate markdown output
        let markdown = format_heading(self.text, *self.level);

        if changed || *self.text != original_text || *self.level != original_level {
            WidgetOutput::modified(markdown)
        } else {
            WidgetOutput::unchanged(markdown)
        }
    }
}

/// Decrease heading level (H1 stays H1).
fn decrease_heading_level(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H1,
        HeadingLevel::H2 => HeadingLevel::H1,
        HeadingLevel::H3 => HeadingLevel::H2,
        HeadingLevel::H4 => HeadingLevel::H3,
        HeadingLevel::H5 => HeadingLevel::H4,
        HeadingLevel::H6 => HeadingLevel::H5,
    }
}

/// Increase heading level (H6 stays H6).
fn increase_heading_level(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H2,
        HeadingLevel::H2 => HeadingLevel::H3,
        HeadingLevel::H3 => HeadingLevel::H4,
        HeadingLevel::H4 => HeadingLevel::H5,
        HeadingLevel::H5 => HeadingLevel::H6,
        HeadingLevel::H6 => HeadingLevel::H6,
    }
}

/// Format a heading as markdown.
pub fn format_heading(text: &str, level: HeadingLevel) -> String {
    let prefix = "#".repeat(level as usize);
    format!("{} {}", prefix, text.trim())
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable Paragraph Widget
// ─────────────────────────────────────────────────────────────────────────────

/// An editable paragraph widget that syncs to markdown.
///
/// This widget renders a paragraph with:
/// - Multi-line text editing
/// - Word wrap support
/// - Outputs markdown string on change
///
/// # Example
///
/// ```ignore
/// let mut text = "This is a paragraph.\nWith multiple lines.".to_string();
///
/// let output = EditableParagraph::new(&mut text)
///     .font_size(14.0)
///     .show(ui);
///
/// if output.changed {
///     // output.markdown contains the paragraph text
/// }
/// ```
pub struct EditableParagraph<'a> {
    /// The paragraph text
    text: &'a mut String,
    /// Font size
    font_size: f32,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Indentation level (for nested paragraphs)
    indent_level: usize,
}

impl<'a> EditableParagraph<'a> {
    /// Create a new editable paragraph widget.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            font_size: 14.0,
            colors: None,
            indent_level: 0,
        }
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set the indentation level.
    #[must_use]
    pub fn indent(mut self, level: usize) -> Self {
        self.indent_level = level;
        self
    }

    /// Show the paragraph widget and return the output.
    pub fn show(self, ui: &mut Ui) -> WidgetOutput {
        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        let original_text = self.text.clone();

        ui.horizontal(|ui| {
            // Indentation
            if self.indent_level > 0 {
                ui.add_space(self.indent_level as f32 * 20.0);
            }

            // Editable paragraph text
            ui.add(
                TextEdit::multiline(self.text)
                    .font(FontId::proportional(self.font_size))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );
        });

        // Generate markdown output (paragraph is just the text with blank lines around it)
        let markdown = format_paragraph(self.text);

        if *self.text != original_text {
            WidgetOutput::modified(markdown)
        } else {
            WidgetOutput::unchanged(markdown)
        }
    }
}

/// Format a paragraph as markdown.
pub fn format_paragraph(text: &str) -> String {
    text.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable List Widget
// ─────────────────────────────────────────────────────────────────────────────

/// An individual list item.
#[derive(Debug, Clone)]
pub struct ListItem {
    /// The text content of the item
    pub text: String,
    /// Whether this is a task item
    pub is_task: bool,
    /// Whether the task is checked (only relevant if is_task is true)
    pub checked: bool,
}

impl ListItem {
    /// Create a new regular list item.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_task: false,
            checked: false,
        }
    }

    /// Create a new task list item.
    pub fn task(text: impl Into<String>, checked: bool) -> Self {
        Self {
            text: text.into(),
            is_task: true,
            checked,
        }
    }
}

/// An editable list widget (ordered or unordered) that syncs to markdown.
///
/// This widget renders a list with:
/// - Ordered (1. 2. 3.) or unordered (• • •) markers
/// - Inline editing of items
/// - Add/remove item controls
/// - Task list checkbox support
/// - Outputs markdown string on change
///
/// # Example
///
/// ```ignore
/// let mut items = vec![
///     ListItem::new("First item"),
///     ListItem::new("Second item"),
/// ];
/// let mut list_type = ListType::Bullet;
///
/// let output = EditableList::new(&mut items, &mut list_type)
///     .font_size(14.0)
///     .show(ui);
///
/// if output.changed {
///     // output.markdown contains "- First item\n- Second item"
/// }
/// ```
pub struct EditableList<'a> {
    /// The list items
    items: &'a mut Vec<ListItem>,
    /// The list type (bullet or ordered)
    list_type: &'a mut ListType,
    /// Font size
    font_size: f32,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Whether to show add/remove controls
    show_controls: bool,
    /// Indentation level (for nested lists)
    indent_level: usize,
}

impl<'a> EditableList<'a> {
    /// Create a new editable list widget.
    pub fn new(items: &'a mut Vec<ListItem>, list_type: &'a mut ListType) -> Self {
        Self {
            items,
            list_type,
            font_size: 14.0,
            colors: None,
            show_controls: false,
            indent_level: 0,
        }
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Enable add/remove controls.
    #[must_use]
    pub fn with_controls(mut self) -> Self {
        self.show_controls = true;
        self
    }

    /// Set the indentation level.
    #[must_use]
    pub fn indent(mut self, level: usize) -> Self {
        self.indent_level = level;
        self
    }

    /// Show the list widget and return the output.
    pub fn show(self, ui: &mut Ui) -> WidgetOutput {
        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        let original_items: Vec<ListItem> = self.items.clone();
        let original_type = *self.list_type;
        let mut changed = false;
        let mut item_to_remove: Option<usize> = None;

        // List type toggle (if controls enabled)
        if self.show_controls {
            ui.horizontal(|ui| {
                ui.add_space(self.indent_level as f32 * 20.0);

                let is_bullet = matches!(self.list_type, ListType::Bullet);
                if ui.selectable_label(is_bullet, "\u{2022}").clicked() && !is_bullet {
                    *self.list_type = ListType::Bullet;
                    changed = true;
                }
                if ui.selectable_label(!is_bullet, "1.").clicked() && is_bullet {
                    *self.list_type = ListType::Ordered {
                        start: 1,
                        delimiter: '.',
                    };
                    changed = true;
                }
            });
        }

        // Render each list item
        let start_number = match self.list_type {
            ListType::Ordered { start, .. } => *start,
            ListType::Bullet => 0,
        };

        for (i, item) in self.items.iter_mut().enumerate() {
            let item_number = start_number + i as u32;

            ui.horizontal(|ui| {
                // Indentation
                ui.add_space(self.indent_level as f32 * 20.0);

                // Task checkbox or list marker
                if item.is_task {
                    if ui.checkbox(&mut item.checked, "").changed() {
                        changed = true;
                    }
                } else {
                    // List marker
                    let marker = match self.list_type {
                        ListType::Bullet => "\u{2022}".to_string(), // bullet •
                        ListType::Ordered { delimiter, .. } => {
                            format!("{}{}", item_number, delimiter)
                        }
                    };
                    ui.label(
                        RichText::new(&marker)
                            .color(colors.list_marker)
                            .font(FontId::proportional(self.font_size)),
                    );
                }

                ui.add_space(8.0);

                // Editable item text
                let response = ui.add(
                    TextEdit::singleline(&mut item.text)
                        .font(FontId::proportional(self.font_size))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(f32::INFINITY),
                );

                if response.changed() {
                    changed = true;
                }

                // Remove button (if controls enabled)
                if self.show_controls && ui.small_button("×").on_hover_text("Remove item").clicked()
                {
                    item_to_remove = Some(i);
                }
            });
        }

        // Handle item removal
        if let Some(index) = item_to_remove {
            self.items.remove(index);
            changed = true;
        }

        // Add new item button (if controls enabled)
        if self.show_controls {
            ui.horizontal(|ui| {
                ui.add_space(self.indent_level as f32 * 20.0);
                if ui.button("+ Add item").clicked() {
                    self.items.push(ListItem::new(""));
                    changed = true;
                }
            });
        }

        // Generate markdown output
        let markdown = format_list(self.items, self.list_type);

        // Check for any changes
        let items_changed =
            self.items.len() != original_items.len()
                || self.items.iter().zip(original_items.iter()).any(|(a, b)| {
                    a.text != b.text || a.is_task != b.is_task || a.checked != b.checked
                });

        if changed || items_changed || *self.list_type != original_type {
            WidgetOutput::modified(markdown)
        } else {
            WidgetOutput::unchanged(markdown)
        }
    }
}

/// Format a list as markdown.
pub fn format_list(items: &[ListItem], list_type: &ListType) -> String {
    let mut output = String::new();
    let start_number = match list_type {
        ListType::Ordered { start, .. } => *start,
        ListType::Bullet => 0,
    };

    for (i, item) in items.iter().enumerate() {
        let marker = if item.is_task {
            let checkbox = if item.checked { "[x]" } else { "[ ]" };
            format!("- {}", checkbox)
        } else {
            match list_type {
                ListType::Bullet => "-".to_string(),
                ListType::Ordered { delimiter, .. } => {
                    format!("{}{}", start_number + i as u32, delimiter)
                }
            }
        };

        output.push_str(&marker);
        output.push(' ');
        output.push_str(&item.text);
        output.push('\n');
    }

    // Remove trailing newline
    if output.ends_with('\n') {
        output.pop();
    }

    output
}

// ─────────────────────────────────────────────────────────────────────────────
// AST to Markdown Serialization
// ─────────────────────────────────────────────────────────────────────────────

/// Serialize a markdown node back to markdown text.
pub fn serialize_node(node: &MarkdownNode) -> String {
    match &node.node_type {
        MarkdownNodeType::Document => {
            let mut output = String::new();
            for child in &node.children {
                if !output.is_empty() {
                    output.push_str("\n\n");
                }
                output.push_str(&serialize_node(child));
            }
            output
        }

        MarkdownNodeType::Heading { level, .. } => {
            let text = node.text_content();
            format_heading(&text, *level)
        }

        MarkdownNodeType::Paragraph => serialize_inline_content(node),

        MarkdownNodeType::CodeBlock {
            language, literal, ..
        } => {
            if language.is_empty() {
                format!("```\n{}\n```", literal)
            } else {
                format!("```{}\n{}\n```", language, literal)
            }
        }

        MarkdownNodeType::BlockQuote => {
            let inner = node
                .children
                .iter()
                .map(serialize_node)
                .collect::<Vec<_>>()
                .join("\n");
            inner
                .lines()
                .map(|line| format!("> {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        }

        MarkdownNodeType::List { list_type, .. } => {
            let items: Vec<ListItem> = node
                .children
                .iter()
                .filter_map(|child| {
                    if let MarkdownNodeType::Item = &child.node_type {
                        // Check for task item
                        let is_task = child
                            .children
                            .iter()
                            .any(|c| matches!(c.node_type, MarkdownNodeType::TaskItem { .. }));
                        let checked = child
                            .children
                            .iter()
                            .find_map(|c| {
                                if let MarkdownNodeType::TaskItem { checked } = &c.node_type {
                                    Some(*checked)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(false);

                        let text = child.text_content();

                        if is_task {
                            Some(ListItem::task(text, checked))
                        } else {
                            Some(ListItem::new(text))
                        }
                    } else {
                        None
                    }
                })
                .collect();

            format_list(&items, list_type)
        }

        MarkdownNodeType::ThematicBreak => "---".to_string(),

        MarkdownNodeType::Table {
            num_columns,
            alignments,
        } => serialize_table(node, *num_columns, alignments),

        MarkdownNodeType::FrontMatter(content) => {
            format!("---\n{}\n---", content)
        }

        MarkdownNodeType::HtmlBlock(html) => html.clone(),

        // Inline elements
        MarkdownNodeType::Text(text) => text.clone(),
        MarkdownNodeType::Code(code) => format!("`{}`", code),
        MarkdownNodeType::Emphasis => format!("*{}*", node.text_content()),
        MarkdownNodeType::Strong => format!("**{}**", node.text_content()),
        MarkdownNodeType::Strikethrough => format!("~~{}~~", node.text_content()),
        MarkdownNodeType::Link { url, title } => {
            let text = node.text_content();
            if title.is_empty() {
                format!("[{}]({})", text, url)
            } else {
                format!("[{}]({} \"{}\")", text, url, title)
            }
        }
        MarkdownNodeType::Image { url, title } => {
            let alt = node.text_content();
            if title.is_empty() {
                format!("![{}]({})", alt, url)
            } else {
                format!("![{}]({} \"{}\")", alt, url, title)
            }
        }
        MarkdownNodeType::SoftBreak => " ".to_string(),
        MarkdownNodeType::LineBreak => "  \n".to_string(),

        // Container nodes that shouldn't be serialized directly
        _ => node.text_content(),
    }
}

/// Serialize inline content from a node's children.
fn serialize_inline_content(node: &MarkdownNode) -> String {
    let mut output = String::new();
    for child in &node.children {
        output.push_str(&serialize_inline_node(child));
    }
    output
}

/// Serialize an inline node.
fn serialize_inline_node(node: &MarkdownNode) -> String {
    match &node.node_type {
        MarkdownNodeType::Text(text) => text.clone(),
        MarkdownNodeType::Code(code) => format!("`{}`", code),
        MarkdownNodeType::Emphasis => {
            let inner = serialize_inline_content(node);
            format!("*{}*", inner)
        }
        MarkdownNodeType::Strong => {
            let inner = serialize_inline_content(node);
            format!("**{}**", inner)
        }
        MarkdownNodeType::Strikethrough => {
            let inner = serialize_inline_content(node);
            format!("~~{}~~", inner)
        }
        MarkdownNodeType::Link { url, title } => {
            let inner = serialize_inline_content(node);
            if title.is_empty() {
                format!("[{}]({})", inner, url)
            } else {
                format!("[{}]({} \"{}\")", inner, url, title)
            }
        }
        MarkdownNodeType::Image { url, title } => {
            let alt = serialize_inline_content(node);
            if title.is_empty() {
                format!("![{}]({})", alt, url)
            } else {
                format!("![{}]({} \"{}\")", alt, url, title)
            }
        }
        MarkdownNodeType::SoftBreak => " ".to_string(),
        MarkdownNodeType::LineBreak => "  \n".to_string(),
        MarkdownNodeType::HtmlInline(html) => html.clone(),
        _ => node.text_content(),
    }
}

/// Serialize a table node.
fn serialize_table(
    node: &MarkdownNode,
    num_columns: usize,
    alignments: &[crate::markdown::parser::TableAlignment],
) -> String {
    use crate::markdown::parser::TableAlignment;

    let mut rows: Vec<Vec<String>> = Vec::new();

    for row_node in &node.children {
        if let MarkdownNodeType::TableRow { .. } = &row_node.node_type {
            let cells: Vec<String> = row_node
                .children
                .iter()
                .map(|cell| cell.text_content())
                .collect();
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    // Header row
    if !rows.is_empty() {
        output.push('|');
        for cell in &rows[0] {
            output.push(' ');
            output.push_str(cell);
            output.push_str(" |");
        }
        output.push('\n');
    }

    // Separator row with alignment
    output.push('|');
    for i in 0..num_columns {
        let align = alignments.get(i).copied().unwrap_or(TableAlignment::None);
        let sep = match align {
            TableAlignment::Left => ":---",
            TableAlignment::Center => ":---:",
            TableAlignment::Right => "---:",
            TableAlignment::None => "---",
        };
        output.push_str(sep);
        output.push('|');
    }
    output.push('\n');

    // Data rows
    for row in rows.iter().skip(1) {
        output.push('|');
        for cell in row {
            output.push(' ');
            output.push_str(cell);
            output.push_str(" |");
        }
        output.push('\n');
    }

    // Remove trailing newline
    if output.ends_with('\n') {
        output.pop();
    }

    output
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable Table Widget
// ─────────────────────────────────────────────────────────────────────────────

/// State for an editable table cell.
#[derive(Debug, Clone)]
pub struct TableCellData {
    /// The text content of the cell
    pub text: String,
}

impl TableCellData {
    /// Create a new table cell with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// State for an editable table.
#[derive(Debug, Clone)]
pub struct TableData {
    /// Table rows (first row is the header)
    pub rows: Vec<Vec<TableCellData>>,
    /// Column alignments
    pub alignments: Vec<crate::markdown::parser::TableAlignment>,
    /// Number of columns
    pub num_columns: usize,
}

impl TableData {
    /// Create a new empty table with the given dimensions.
    pub fn new(num_columns: usize, num_rows: usize) -> Self {
        let alignments = vec![crate::markdown::parser::TableAlignment::None; num_columns];
        let rows = (0..num_rows)
            .map(|_| (0..num_columns).map(|_| TableCellData::new("")).collect())
            .collect();

        Self {
            rows,
            alignments,
            num_columns,
        }
    }

    /// Create table data from a markdown table node.
    pub fn from_node(node: &MarkdownNode) -> Self {
        use crate::markdown::parser::TableAlignment;

        // Extract alignments and num_columns from the table node
        let (alignments, num_columns) = match &node.node_type {
            MarkdownNodeType::Table {
                alignments,
                num_columns,
            } => (alignments.clone(), *num_columns),
            _ => (Vec::new(), 0),
        };

        // Extract rows from children
        let rows: Vec<Vec<TableCellData>> = node
            .children
            .iter()
            .filter_map(|row_node| {
                if let MarkdownNodeType::TableRow { .. } = &row_node.node_type {
                    let cells: Vec<TableCellData> = row_node
                        .children
                        .iter()
                        .map(|cell| TableCellData::new(cell.text_content()))
                        .collect();
                    Some(cells)
                } else {
                    None
                }
            })
            .collect();

        // Ensure alignments match column count
        let alignments = if alignments.len() < num_columns {
            let mut a = alignments;
            a.resize(num_columns, TableAlignment::None);
            a
        } else {
            alignments
        };

        Self {
            rows,
            alignments,
            num_columns,
        }
    }

    /// Add a new row at the end of the table.
    pub fn add_row(&mut self) {
        let new_row = (0..self.num_columns)
            .map(|_| TableCellData::new(""))
            .collect();
        self.rows.push(new_row);
    }

    /// Insert a new row at the specified index.
    pub fn insert_row(&mut self, index: usize) {
        let new_row = (0..self.num_columns)
            .map(|_| TableCellData::new(""))
            .collect();
        if index <= self.rows.len() {
            self.rows.insert(index, new_row);
        }
    }

    /// Remove a row at the specified index.
    /// Cannot remove the header row (index 0) if it's the only row.
    pub fn remove_row(&mut self, index: usize) {
        if self.rows.len() > 1 && index < self.rows.len() {
            self.rows.remove(index);
        }
    }

    /// Add a new column at the end of the table.
    pub fn add_column(&mut self) {
        use crate::markdown::parser::TableAlignment;

        self.num_columns += 1;
        self.alignments.push(TableAlignment::None);
        for row in &mut self.rows {
            row.push(TableCellData::new(""));
        }
    }

    /// Insert a new column at the specified index.
    pub fn insert_column(&mut self, index: usize) {
        use crate::markdown::parser::TableAlignment;

        if index <= self.num_columns {
            self.num_columns += 1;
            self.alignments.insert(index, TableAlignment::None);
            for row in &mut self.rows {
                row.insert(index, TableCellData::new(""));
            }
        }
    }

    /// Remove a column at the specified index.
    /// Cannot remove if it's the only column.
    pub fn remove_column(&mut self, index: usize) {
        if self.num_columns > 1 && index < self.num_columns {
            self.num_columns -= 1;
            if index < self.alignments.len() {
                self.alignments.remove(index);
            }
            for row in &mut self.rows {
                if index < row.len() {
                    row.remove(index);
                }
            }
        }
    }

    /// Set the alignment for a column.
    pub fn set_column_alignment(
        &mut self,
        column: usize,
        alignment: crate::markdown::parser::TableAlignment,
    ) {
        if column < self.alignments.len() {
            self.alignments[column] = alignment;
        }
    }

    /// Cycle to the next alignment for a column.
    pub fn cycle_column_alignment(&mut self, column: usize) {
        use crate::markdown::parser::TableAlignment;

        if column < self.alignments.len() {
            self.alignments[column] = match self.alignments[column] {
                TableAlignment::None => TableAlignment::Left,
                TableAlignment::Left => TableAlignment::Center,
                TableAlignment::Center => TableAlignment::Right,
                TableAlignment::Right => TableAlignment::None,
            };
        }
    }

    /// Generate the markdown table syntax.
    pub fn to_markdown(&self) -> String {
        use crate::markdown::parser::TableAlignment;

        if self.rows.is_empty() || self.num_columns == 0 {
            return String::new();
        }

        let mut output = String::new();

        // Calculate column widths for better formatting
        let mut col_widths: Vec<usize> = vec![3; self.num_columns];
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.text.len());
                }
            }
        }

        // Header row
        if !self.rows.is_empty() {
            output.push('|');
            for (i, cell) in self.rows[0].iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                output.push(' ');
                output.push_str(&format!("{:width$}", cell.text, width = width));
                output.push_str(" |");
            }
            output.push('\n');
        }

        // Separator row with alignment
        output.push('|');
        for i in 0..self.num_columns {
            let align = self
                .alignments
                .get(i)
                .copied()
                .unwrap_or(TableAlignment::None);
            let width = col_widths.get(i).copied().unwrap_or(3);
            let sep = match align {
                TableAlignment::Left => format!(":{}", "-".repeat(width.max(3) - 1)),
                TableAlignment::Center => {
                    format!(":{}:", "-".repeat(width.max(3).saturating_sub(2)))
                }
                TableAlignment::Right => format!("{}:", "-".repeat(width.max(3) - 1)),
                TableAlignment::None => "-".repeat(width.max(3)),
            };
            output.push_str(&sep);
            output.push('|');
        }
        output.push('\n');

        // Data rows
        for row in self.rows.iter().skip(1) {
            output.push('|');
            for (i, cell) in row.iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                output.push(' ');
                output.push_str(&format!("{:width$}", cell.text, width = width));
                output.push_str(" |");
            }
            output.push('\n');
        }

        // Remove trailing newline
        if output.ends_with('\n') {
            output.pop();
        }

        output
    }

    /// Check if the table has a header row.
    pub fn has_header(&self) -> bool {
        !self.rows.is_empty()
    }

    /// Get the number of rows (including header).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

/// An editable table widget that syncs to markdown.
///
/// This widget renders a markdown table with:
/// - Editable cells using `TextEdit`
/// - Add/remove row and column buttons
/// - Column alignment controls
/// - Automatic markdown regeneration
///
/// # Example
///
/// ```ignore
/// let mut table_data = TableData::from_node(&table_node);
///
/// let output = EditableTable::new(&mut table_data)
///     .font_size(14.0)
///     .show(ui);
///
/// if output.changed {
///     // output.markdown contains the regenerated table
/// }
/// ```
pub struct EditableTable<'a> {
    /// The table data
    data: &'a mut TableData,
    /// Font size for cells
    font_size: f32,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Whether to show add/remove controls
    show_controls: bool,
    /// Whether to show alignment controls
    show_alignment_controls: bool,
    /// Unique ID for the table
    id: Option<egui::Id>,
}

impl<'a> EditableTable<'a> {
    /// Create a new editable table widget.
    pub fn new(data: &'a mut TableData) -> Self {
        Self {
            data,
            font_size: 14.0,
            colors: None,
            show_controls: true,
            show_alignment_controls: true,
            id: None,
        }
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Enable or disable add/remove controls.
    #[must_use]
    pub fn with_controls(mut self, enabled: bool) -> Self {
        self.show_controls = enabled;
        self
    }

    /// Enable or disable alignment controls (currently disabled/not implemented).
    #[must_use]
    #[allow(dead_code)]
    pub fn with_alignment_controls(mut self, _enabled: bool) -> Self {
        // Alignment controls are disabled for now
        self.show_alignment_controls = false;
        self
    }

    /// Set a custom ID for the table.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Show the table widget and return the output.
    pub fn show(self, ui: &mut Ui) -> WidgetOutput {
        use crate::markdown::parser::TableAlignment;

        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        let table_id = self.id.unwrap_or_else(|| ui.id().with("editable_table"));

        // Track original state for change detection
        let original_markdown = self.data.to_markdown();
        let mut changed = false;

        // Track actions to perform after iteration (to avoid borrow issues)
        let mut action: Option<TableAction> = None;

        // Determine dark mode for styling
        let is_dark = colors.text.r() > 128;

        // Table styling colors
        let header_bg = if is_dark {
            egui::Color32::from_rgb(45, 50, 60)
        } else {
            egui::Color32::from_rgb(240, 242, 245)
        };

        let cell_bg = if is_dark {
            egui::Color32::from_rgb(35, 38, 45)
        } else {
            egui::Color32::from_rgb(255, 255, 255)
        };

        let border_color = if is_dark {
            egui::Color32::from_rgb(60, 65, 75)
        } else {
            egui::Color32::from_rgb(200, 205, 210)
        };

        ui.add_space(4.0);

        // Main table frame
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(0.0)
            .rounding(4.0)
            .show(ui, |ui| {
                // Alignment controls row (if enabled and there are columns)
                if self.show_alignment_controls && self.data.num_columns > 0 {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new("Align:").small().color(colors.muted));

                        for col in 0..self.data.num_columns {
                            let align = self
                                .data
                                .alignments
                                .get(col)
                                .copied()
                                .unwrap_or(TableAlignment::None);

                            let align_icon = match align {
                                TableAlignment::Left => "⬅",
                                TableAlignment::Center => "⬌",
                                TableAlignment::Right => "➡",
                                TableAlignment::None => "—",
                            };

                            let tooltip = match align {
                                TableAlignment::Left => "Left aligned (click to cycle)",
                                TableAlignment::Center => "Center aligned (click to cycle)",
                                TableAlignment::Right => "Right aligned (click to cycle)",
                                TableAlignment::None => "No alignment (click to cycle)",
                            };

                            if ui.small_button(align_icon).on_hover_text(tooltip).clicked() {
                                action = Some(TableAction::CycleAlignment(col));
                            }
                        }
                    });
                    ui.separator();
                }

                // Calculate column widths based on content
                let min_col_width = 60.0_f32; // Minimum column width
                let char_width = self.font_size * 0.65; // Approximate character width for proportional font
                let cell_padding = 24.0_f32; // Cell padding (left + right)

                let col_widths: Vec<f32> = (0..self.data.num_columns)
                    .map(|col_idx| {
                        let max_text_len = self
                            .data
                            .rows
                            .iter()
                            .filter_map(|row| row.get(col_idx))
                            .map(|cell| cell.text.len())
                            .max()
                            .unwrap_or(0);

                        // Calculate width: text length * char width + padding, with min/max bounds
                        let text_width = (max_text_len as f32 * char_width) + cell_padding;
                        text_width.max(min_col_width).min(400.0) // Clamp between 60 and 400
                    })
                    .collect();

                // Render table using horizontal layouts (not Grid) for better width control
                ui.vertical(|ui| {
                    // Render each row
                    for row_idx in 0..self.data.rows.len() {
                        let is_header = row_idx == 0;
                        let row_bg = if is_header { header_bg } else { cell_bg };

                        ui.horizontal(|ui| {
                            // Row delete button (if controls enabled and not the only row)
                            if self.show_controls && self.data.rows.len() > 1 {
                                egui::Frame::none()
                                    .fill(row_bg)
                                    .inner_margin(egui::Margin::symmetric(2.0, 4.0))
                                    .show(ui, |ui| {
                                        if ui
                                            .small_button("🗑")
                                            .on_hover_text("Delete row")
                                            .clicked()
                                        {
                                            action = Some(TableAction::RemoveRow(row_idx));
                                        }
                                    });
                            } else if self.show_controls {
                                // Placeholder for alignment - same width as delete button
                                ui.allocate_space(egui::vec2(24.0, 20.0));
                            }

                            // Render cells for this row
                            for col_idx in 0..self.data.num_columns {
                                let col_width =
                                    col_widths.get(col_idx).copied().unwrap_or(min_col_width);

                                egui::Frame::none()
                                    .fill(row_bg)
                                    .stroke(egui::Stroke::new(0.5, border_color))
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                    .show(ui, |ui| {
                                        if let Some(row) = self.data.rows.get_mut(row_idx) {
                                            if let Some(cell) = row.get_mut(col_idx) {
                                                let cell_id = table_id
                                                    .with("cell")
                                                    .with(row_idx)
                                                    .with(col_idx);

                                                let text_color = if is_header {
                                                    colors.heading
                                                } else {
                                                    colors.text
                                                };

                                                let font = FontId::proportional(self.font_size);

                                                // Allocate exact width for the cell
                                                ui.set_min_width(col_width);

                                                let response = ui.add(
                                                    TextEdit::singleline(&mut cell.text)
                                                        .id(cell_id)
                                                        .font(font)
                                                        .text_color(text_color)
                                                        .frame(false)
                                                        .min_size(egui::vec2(col_width, 0.0)),
                                                );

                                                if response.changed() {
                                                    changed = true;
                                                }
                                            }
                                        }
                                    });
                            }

                            // Column add button on the right (only on first row)
                            if self.show_controls && row_idx == 0 {
                                egui::Frame::none()
                                    .fill(row_bg)
                                    .inner_margin(egui::Margin::symmetric(2.0, 4.0))
                                    .show(ui, |ui| {
                                        if ui
                                            .small_button("➕")
                                            .on_hover_text("Add column")
                                            .clicked()
                                        {
                                            action = Some(TableAction::AddColumn);
                                        }
                                    });
                            }
                        }); // end horizontal row
                    } // end for each row

                    // Add row button row
                    if self.show_controls {
                        ui.horizontal(|ui| {
                            // Offset to align with cells (skip delete button space)
                            if self.data.rows.len() > 1 {
                                ui.allocate_space(egui::vec2(24.0, 20.0));
                            }

                            if ui
                                .button("➕ Add row")
                                .on_hover_text("Add a new row")
                                .clicked()
                            {
                                action = Some(TableAction::AddRow);
                            }
                        });
                    }
                }); // end vertical

                // Column delete buttons row (if controls enabled and more than one column)
                if self.show_controls && self.data.num_columns > 1 {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new("Del col:").small().color(colors.muted));

                        for col in 0..self.data.num_columns {
                            if ui
                                .small_button("×")
                                .on_hover_text(format!("Delete column {}", col + 1))
                                .clicked()
                            {
                                action = Some(TableAction::RemoveColumn(col));
                            }
                        }
                    });
                }
            });

        ui.add_space(4.0);

        // Apply the action (after the UI iteration is complete)
        if let Some(action) = action {
            changed = true;
            match action {
                TableAction::AddRow => self.data.add_row(),
                TableAction::InsertRow(idx) => self.data.insert_row(idx),
                TableAction::RemoveRow(idx) => self.data.remove_row(idx),
                TableAction::AddColumn => self.data.add_column(),
                TableAction::InsertColumn(idx) => self.data.insert_column(idx),
                TableAction::RemoveColumn(idx) => self.data.remove_column(idx),
                TableAction::CycleAlignment(col) => self.data.cycle_column_alignment(col),
            }
        }

        // Generate markdown output
        let markdown = self.data.to_markdown();

        if changed || markdown != original_markdown {
            WidgetOutput::modified(markdown)
        } else {
            WidgetOutput::unchanged(markdown)
        }
    }
}

/// Internal enum for table modification actions.
#[derive(Debug, Clone)]
enum TableAction {
    AddRow,
    InsertRow(usize),
    RemoveRow(usize),
    AddColumn,
    InsertColumn(usize),
    RemoveColumn(usize),
    CycleAlignment(usize),
}

// ─────────────────────────────────────────────────────────────────────────────
// Link Data (Simple)
// ─────────────────────────────────────────────────────────────────────────────

/// Data for a link - just stores the URL and title for markdown generation.
#[derive(Debug, Clone)]
pub struct LinkData {
    /// The display text of the link
    pub text: String,
    /// The URL destination
    pub url: String,
    /// Optional title attribute
    pub title: String,
}

impl LinkData {
    /// Create a new link with the given text and URL.
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
            title: String::new(),
        }
    }

    /// Create a new link with a title.
    pub fn with_title(
        text: impl Into<String>,
        url: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
            title: title.into(),
        }
    }

    /// Generate the markdown syntax for this link.
    pub fn to_markdown(&self) -> String {
        if self.title.is_empty() {
            format!("[{}]({})", self.text, self.url)
        } else {
            format!("[{}]({} \"{}\")", self.text, self.url, self.title)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inline Formatting Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Format text as bold markdown.
pub fn format_bold(text: &str) -> String {
    format!("**{}**", text)
}

/// Format text as italic markdown.
pub fn format_italic(text: &str) -> String {
    format!("*{}*", text)
}

/// Format text as strikethrough markdown.
pub fn format_strikethrough(text: &str) -> String {
    format!("~~{}~~", text)
}

/// Format inline code markdown.
pub fn format_inline_code(text: &str) -> String {
    format!("`{}`", text)
}

/// Check if text is wrapped in bold delimiters.
pub fn is_bold(text: &str) -> bool {
    text.starts_with("**") && text.ends_with("**") && text.len() > 4
}

/// Check if text is wrapped in italic delimiters.
pub fn is_italic(text: &str) -> bool {
    (text.starts_with('*') && text.ends_with('*') && !text.starts_with("**") && text.len() > 2)
        || (text.starts_with('_')
            && text.ends_with('_')
            && !text.starts_with("__")
            && text.len() > 2)
}

/// Remove bold delimiters from text.
pub fn unwrap_bold(text: &str) -> &str {
    if is_bold(text) {
        &text[2..text.len() - 2]
    } else {
        text
    }
}

/// Remove italic delimiters from text.
pub fn unwrap_italic(text: &str) -> &str {
    if is_italic(text) {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// Toggle bold formatting on text (add if not bold, remove if bold).
pub fn toggle_bold(text: &str) -> String {
    if is_bold(text) {
        unwrap_bold(text).to_string()
    } else {
        format_bold(text)
    }
}

/// Toggle italic formatting on text (add if not italic, remove if italic).
pub fn toggle_italic(text: &str) -> String {
    if is_italic(text) {
        unwrap_italic(text).to_string()
    } else {
        format_italic(text)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable Code Block Widget
// ─────────────────────────────────────────────────────────────────────────────

/// Supported programming languages for code block syntax highlighting.
/// These match syntect's supported languages and common markdown code fence identifiers.
pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "", // Plain text (no highlighting)
    "rust",
    "python",
    "javascript",
    "typescript",
    "go",
    "java",
    "c",
    "cpp",
    "csharp",
    "html",
    "css",
    "json",
    "yaml",
    "toml",
    "markdown",
    "bash",
    "sql",
    "ruby",
    "php",
    "swift",
    "kotlin",
    "scala",
    "lua",
    "perl",
    "r",
    "haskell",
    "elixir",
    "clojure",
    "xml",
    "dockerfile",
    "makefile",
    "diff",
];

/// Get the display name for a language code.
pub fn language_display_name(lang: &str) -> &str {
    match lang {
        "" => "Plain Text",
        "rust" => "Rust",
        "python" => "Python",
        "javascript" | "js" => "JavaScript",
        "typescript" | "ts" => "TypeScript",
        "go" => "Go",
        "java" => "Java",
        "c" => "C",
        "cpp" | "c++" => "C++",
        "csharp" | "cs" | "c#" => "C#",
        "html" => "HTML",
        "css" => "CSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "markdown" | "md" => "Markdown",
        "bash" | "sh" | "shell" => "Bash",
        "sql" => "SQL",
        "ruby" | "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "kotlin" | "kt" => "Kotlin",
        "scala" => "Scala",
        "lua" => "Lua",
        "perl" | "pl" => "Perl",
        "r" => "R",
        "haskell" | "hs" => "Haskell",
        "elixir" | "ex" => "Elixir",
        "clojure" | "clj" => "Clojure",
        "xml" => "XML",
        "dockerfile" | "docker" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        "diff" | "patch" => "Diff",
        other => other,
    }
}

/// Normalize a language string to a canonical form.
pub fn normalize_language(lang: &str) -> &'static str {
    let lower = lang.to_lowercase();
    match lower.as_str() {
        "" => "",
        "rust" | "rs" => "rust",
        "python" | "py" => "python",
        "javascript" | "js" => "javascript",
        "typescript" | "ts" => "typescript",
        "go" | "golang" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" | "cxx" => "cpp",
        "csharp" | "cs" | "c#" => "csharp",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "markdown" | "md" => "markdown",
        "bash" | "sh" | "shell" | "zsh" => "bash",
        "sql" => "sql",
        "ruby" | "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kotlin" | "kt" => "kotlin",
        "scala" => "scala",
        "lua" => "lua",
        "perl" | "pl" => "perl",
        "r" => "r",
        "haskell" | "hs" => "haskell",
        "elixir" | "ex" => "elixir",
        "clojure" | "clj" => "clojure",
        "xml" => "xml",
        "dockerfile" | "docker" => "dockerfile",
        "makefile" | "make" => "makefile",
        "diff" | "patch" => "diff",
        _ => "", // Unknown language falls back to plain text
    }
}

/// Data for an editable code block.
#[derive(Debug, Clone)]
pub struct CodeBlockData {
    /// The code content
    pub code: String,
    /// The programming language identifier
    pub language: String,
    /// Whether the code block is currently in edit mode
    pub is_editing: bool,
    /// Original language (to detect changes)
    original_language: String,
    /// Original code (to detect changes)
    original_code: String,
}

impl CodeBlockData {
    /// Create a new code block with the given content and language.
    pub fn new(code: impl Into<String>, language: impl Into<String>) -> Self {
        let code = code.into();
        let language = language.into();
        Self {
            original_code: code.clone(),
            original_language: language.clone(),
            code,
            language,
            is_editing: false,
        }
    }

    /// Check if the code block has been modified.
    pub fn is_modified(&self) -> bool {
        self.code != self.original_code || self.language != self.original_language
    }

    /// Reset the original values to match current values (after saving).
    pub fn mark_saved(&mut self) {
        self.original_code = self.code.clone();
        self.original_language = self.language.clone();
    }

    /// Generate the markdown for this code block.
    pub fn to_markdown(&self) -> String {
        if self.language.is_empty() {
            format!("```\n{}\n```", self.code)
        } else {
            format!("```{}\n{}\n```", self.language, self.code)
        }
    }
}

/// Output from the EditableCodeBlock widget.
#[derive(Debug, Clone)]
pub struct CodeBlockOutput {
    /// Whether the content or language was modified
    pub changed: bool,
    /// Whether the language was specifically changed
    pub language_changed: bool,
    /// The new markdown representation
    pub markdown: String,
    /// The current code content
    pub code: String,
    /// The current language
    pub language: String,
}

/// An editable code block widget with syntax highlighting and language selection.
///
/// This widget provides:
/// - View mode: Syntax-highlighted code with a Copy button
/// - Edit mode: Language dropdown + TextEdit for code editing
/// - Click to enter edit mode, blur to exit
/// - Automatic markdown synchronization
///
/// # Example
///
/// ```ignore
/// let mut data = CodeBlockData::new("fn main() {}", "rust");
///
/// let output = EditableCodeBlock::new(&mut data)
///     .font_size(14.0)
///     .dark_mode(true)
///     .show(ui);
///
/// if output.changed {
///     // output.markdown contains the updated code block
/// }
/// ```
pub struct EditableCodeBlock<'a> {
    /// The code block data
    data: &'a mut CodeBlockData,
    /// Font size for the code
    font_size: f32,
    /// Whether dark mode is active
    dark_mode: bool,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Unique ID for this code block
    id: Option<egui::Id>,
}

impl<'a> EditableCodeBlock<'a> {
    /// Create a new editable code block widget.
    pub fn new(data: &'a mut CodeBlockData) -> Self {
        Self {
            data,
            font_size: 14.0,
            dark_mode: false,
            colors: None,
            id: None,
        }
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set dark mode.
    #[must_use]
    pub fn dark_mode(mut self, dark: bool) -> Self {
        self.dark_mode = dark;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set a custom ID for the code block.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Show the code block widget and return the output.
    pub fn show(self, ui: &mut Ui) -> CodeBlockOutput {
        use crate::markdown::syntax::highlight_code;

        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        // Use the provided ID (required for uniqueness)
        let block_id = self.id.expect("EditableCodeBlock requires an explicit ID");

        // Track changes
        let original_code = self.data.code.clone();
        let mut language_changed = false;

        // Styling based on dark mode
        let code_block_bg = if self.dark_mode {
            egui::Color32::from_rgb(35, 39, 46)
        } else {
            egui::Color32::from_rgb(233, 236, 239)
        };

        let border_color = if self.dark_mode {
            egui::Color32::from_rgb(55, 60, 68)
        } else {
            egui::Color32::from_rgb(195, 202, 210)
        };

        let code_text_color = if self.dark_mode {
            egui::Color32::from_rgb(200, 200, 150)
        } else {
            egui::Color32::from_rgb(80, 80, 80)
        };

        // Add some vertical spacing before code block
        ui.add_space(4.0);

        egui::Frame::none()
            .fill(code_block_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .rounding(6.0)
            .show(ui, |ui| {
                // Header row with language selector/label and buttons
                ui.horizontal(|ui| {
                    if self.data.is_editing {
                        // Language dropdown in edit mode - use unique ID
                        let current_display = language_display_name(&self.data.language);
                        egui::ComboBox::from_id_source(block_id.with("lang"))
                            .selected_text(current_display)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for &lang in SUPPORTED_LANGUAGES {
                                    let display = language_display_name(lang);
                                    if ui
                                        .selectable_label(self.data.language == lang, display)
                                        .clicked()
                                    {
                                        self.data.language = lang.to_string();
                                        language_changed = true;
                                    }
                                }
                            });
                    } else {
                        // Language label in view mode
                        let display = if self.data.language.is_empty() {
                            "Code"
                        } else {
                            language_display_name(&self.data.language)
                        };
                        ui.label(
                            RichText::new(display)
                                .color(colors.muted)
                                .font(FontId::monospace(self.font_size * 0.8))
                                .italics(),
                        );
                    }

                    // Push buttons to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Copy button
                        if ui
                            .add(egui::Button::new("Copy").small())
                            .on_hover_text("Copy to clipboard")
                            .clicked()
                        {
                            ui.ctx().copy_text(self.data.code.clone());
                            log::debug!("Code block copied to clipboard");
                        }

                        // Edit/Done button - ONLY way to toggle edit mode
                        let edit_text = if self.data.is_editing { "Done" } else { "Edit" };
                        if ui
                            .add(egui::Button::new(edit_text).small())
                            .on_hover_text(if self.data.is_editing {
                                "Finish editing"
                            } else {
                                "Edit code"
                            })
                            .clicked()
                        {
                            self.data.is_editing = !self.data.is_editing;
                        }
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                if self.data.is_editing {
                    // Edit mode: show plain text editor with unique ID
                    ui.add(
                        TextEdit::multiline(&mut self.data.code)
                            .id(block_id.with("editor"))
                            .code_editor()
                            .font(FontId::monospace(self.font_size))
                            .text_color(code_text_color)
                            .frame(false)
                            .desired_width(f32::INFINITY),
                    );
                    // No auto-exit - user must click "Done" button
                } else {
                    // View mode: show syntax-highlighted code
                    let highlighted_lines =
                        highlight_code(&self.data.code, &self.data.language, self.dark_mode);

                    ui.vertical(|ui| {
                        if highlighted_lines.is_empty() {
                            ui.label(
                                RichText::new(" ")
                                    .font(FontId::monospace(self.font_size))
                                    .color(code_text_color),
                            );
                        } else {
                            for line in &highlighted_lines {
                                ui.horizontal(|ui| {
                                    if line.segments.is_empty() {
                                        ui.label(
                                            RichText::new(" ")
                                                .font(FontId::monospace(self.font_size)),
                                        );
                                    } else {
                                        for segment in &line.segments {
                                            ui.label(segment.to_rich_text(self.font_size));
                                        }
                                    }
                                });
                            }
                        }
                    });
                    // No click-to-edit - user must click "Edit" button
                }
            });

        // Add some vertical spacing after code block
        ui.add_space(4.0);

        // Determine if changed
        let code_changed = self.data.code != original_code;
        let changed = code_changed || language_changed;

        CodeBlockOutput {
            changed,
            language_changed,
            markdown: self.data.to_markdown(),
            code: self.data.code.clone(),
            language: self.data.language.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rendered Link Widget
// ─────────────────────────────────────────────────────────────────────────────

/// State for a rendered link widget.
/// Tracks whether the popup is open and temporary edit values.
#[derive(Debug, Clone)]
pub struct RenderedLinkState {
    /// Whether the edit popup is currently open
    pub popup_open: bool,
    /// Temporary display text while editing (before committing)
    pub edit_text: String,
    /// Temporary URL while editing (before committing)
    pub edit_url: String,
    /// Original text (for change detection)
    original_text: String,
    /// Original URL (for change detection)
    original_url: String,
    /// Whether this is an autolink (bare URL where text == url)
    is_autolink: bool,
}

impl RenderedLinkState {
    /// Create a new link state with the given text and URL.
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        let text = text.into();
        let url = url.into();
        let is_autolink = text == url;
        Self {
            popup_open: false,
            edit_text: text.clone(),
            edit_url: url.clone(),
            original_text: text,
            original_url: url,
            is_autolink,
        }
    }

    /// Check if this is an autolink (bare URL in source).
    /// For autolinks, only the URL can be edited - there's no separate text.
    pub fn is_autolink(&self) -> bool {
        self.is_autolink
    }

    /// Check if the link has been modified.
    pub fn is_modified(&self) -> bool {
        if self.is_autolink {
            // For autolinks, only URL changes matter
            self.edit_url != self.original_url
        } else {
            self.edit_text != self.original_text || self.edit_url != self.original_url
        }
    }

    /// Commit changes - update original values to match edits.
    pub fn commit(&mut self) {
        if self.is_autolink {
            // For autolinks, keep text in sync with URL
            self.edit_text = self.edit_url.clone();
            self.original_text = self.edit_url.clone();
        } else {
            self.original_text = self.edit_text.clone();
        }
        self.original_url = self.edit_url.clone();
    }

    /// Reset edits to original values (cancel).
    pub fn reset(&mut self) {
        self.edit_text = self.original_text.clone();
        self.edit_url = self.original_url.clone();
    }
}

/// Output from the RenderedLinkWidget.
#[derive(Debug, Clone)]
pub struct RenderedLinkOutput {
    /// Whether the content was modified and committed
    pub changed: bool,
    /// The new display text
    pub text: String,
    /// The new URL
    pub url: String,
    /// The markdown representation (or just URL for autolinks)
    pub markdown: String,
    /// Whether this is an autolink (bare URL, no separate text)
    pub is_autolink: bool,
}

/// A rendered link widget with hover menu for editing.
///
/// This widget provides:
/// - View mode: Styled link text (non-clickable) with hover settings icon
/// - Edit popup: Fields for display text and URL, plus Open/Copy/Done buttons
/// - Automatic markdown synchronization
///
/// # Example
///
/// ```ignore
/// let mut state = RenderedLinkState::new("Example", "https://example.com");
///
/// let output = RenderedLinkWidget::new(&mut state, "Example Link")
///     .font_size(14.0)
///     .show(ui);
///
/// if output.changed {
///     // Update markdown source with output.text and output.url
/// }
/// ```
pub struct RenderedLinkWidget<'a> {
    /// The link state
    state: &'a mut RenderedLinkState,
    /// The title attribute (for tooltip)
    title: String,
    /// Font size for the link text
    font_size: f32,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Unique ID for this link
    id: Option<egui::Id>,
}

impl<'a> RenderedLinkWidget<'a> {
    /// Create a new rendered link widget.
    pub fn new(state: &'a mut RenderedLinkState, title: impl Into<String>) -> Self {
        Self {
            state,
            title: title.into(),
            font_size: 14.0,
            colors: None,
            id: None,
        }
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the widget colors.
    #[must_use]
    pub fn colors(mut self, colors: WidgetColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set a custom ID for the link.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Show the link widget and return the output.
    pub fn show(self, ui: &mut Ui) -> RenderedLinkOutput {
        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        let link_id = self.id.expect("RenderedLinkWidget requires an explicit ID");

        // Track if we committed changes this frame
        let mut committed_changes = false;

        // Link color - use heading color as link color (blue-ish)
        let link_color = colors.heading;

        // Get dark mode for popup styling
        let is_dark = colors.text.r() > 128;

        // Render the link text with underline styling
        let link_response = ui.add(
            egui::Label::new(
                RichText::new(&self.state.edit_text)
                    .color(link_color)
                    .font(FontId::proportional(self.font_size))
                    .underline(),
            )
            .sense(egui::Sense::hover()),
        );

        // Store rect before consuming response
        let link_rect = link_response.rect;

        // Create a unified hover zone that includes both link and button area
        // This prevents flickering when moving between them
        let hover_zone = egui::Rect::from_min_max(
            link_rect.min,
            link_rect.max + egui::vec2(26.0, 0.0), // Extend to include button
        );

        // Check if mouse is anywhere in the combined hover zone
        let mouse_in_hover_zone = ui.rect_contains_pointer(hover_zone);
        let show_settings = mouse_in_hover_zone || self.state.popup_open;

        // Show tooltip with URL when hovering over link (if popup not open)
        if mouse_in_hover_zone && !self.state.popup_open {
            link_response.on_hover_text(format!("URL: {}", self.state.edit_url));
        }

        if show_settings {
            // Position the settings button immediately after the link (no gap)
            let button_rect = egui::Rect::from_min_size(
                link_rect.right_top(),
                egui::vec2(24.0, link_rect.height().max(20.0)),
            );

            // Draw settings button
            let settings_response =
                ui.put(button_rect, egui::Button::new("⚙").small().frame(false));

            if settings_response.clicked() {
                self.state.popup_open = !self.state.popup_open;
            }

            settings_response.on_hover_text("Edit link");
        }

        // Show popup if open
        if self.state.popup_open {
            let popup_id = link_id.with("popup");

            // Popup styling
            let popup_bg = if is_dark {
                egui::Color32::from_rgb(45, 50, 60)
            } else {
                egui::Color32::from_rgb(250, 250, 252)
            };

            let border_color = if is_dark {
                egui::Color32::from_rgb(70, 75, 85)
            } else {
                egui::Color32::from_rgb(180, 185, 195)
            };

            // Position popup below the link
            let popup_pos = link_rect.left_bottom() + egui::vec2(0.0, 4.0);

            // Track if we should close
            let mut should_close = false;

            let area_response = egui::Area::new(popup_id)
                .fixed_pos(popup_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::none()
                        .fill(popup_bg)
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .inner_margin(egui::Margin::same(12.0))
                        .rounding(6.0)
                        .shadow(egui::epaint::Shadow {
                            offset: egui::vec2(0.0, 2.0),
                            blur: 8.0,
                            spread: 0.0,
                            color: egui::Color32::from_black_alpha(40),
                        })
                        .show(ui, |ui| {
                            ui.set_min_width(280.0);

                            // Only show text field for markdown links (not autolinks)
                            if !self.state.is_autolink() {
                                // Display text field
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Text:")
                                            .color(colors.muted)
                                            .font(FontId::proportional(self.font_size * 0.9)),
                                    );
                                    ui.add_space(16.0);
                                    ui.add(
                                        TextEdit::singleline(&mut self.state.edit_text)
                                            .id(link_id.with("text_field"))
                                            .font(FontId::proportional(self.font_size))
                                            .text_color(colors.text)
                                            .desired_width(200.0),
                                    );
                                });

                                ui.add_space(8.0);
                            }

                            // URL field
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("URL:")
                                        .color(colors.muted)
                                        .font(FontId::proportional(self.font_size * 0.9)),
                                );
                                ui.add_space(20.0);
                                ui.add(
                                    TextEdit::singleline(&mut self.state.edit_url)
                                        .id(link_id.with("url_field"))
                                        .font(FontId::monospace(self.font_size * 0.9))
                                        .text_color(colors.text)
                                        .desired_width(200.0),
                                );
                            });

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Action buttons
                            ui.horizontal(|ui| {
                                // Open Link button
                                let can_open = self.state.edit_url.starts_with("http://")
                                    || self.state.edit_url.starts_with("https://");

                                let open_button =
                                    ui.add_enabled(can_open, egui::Button::new("🔗 Open"));

                                // Store clicked state before consuming response
                                let open_clicked = open_button.clicked();

                                // Show appropriate hover text
                                let hover_text = if can_open {
                                    "Open URL in browser"
                                } else {
                                    "Only http/https URLs can be opened"
                                };
                                open_button.on_hover_text(hover_text);

                                if open_clicked && can_open {
                                    if let Err(e) = open::that(&self.state.edit_url) {
                                        log::error!("Failed to open URL: {}", e);
                                    } else {
                                        log::debug!("Opened URL: {}", self.state.edit_url);
                                    }
                                }

                                ui.add_space(4.0);

                                // Copy URL button
                                if ui
                                    .button("📋 Copy")
                                    .on_hover_text("Copy URL to clipboard")
                                    .clicked()
                                {
                                    ui.ctx().copy_text(self.state.edit_url.clone());
                                    log::debug!("Copied URL to clipboard: {}", self.state.edit_url);
                                }
                            });
                        })
                });

            // Get the popup's actual rect for click-outside detection
            let popup_rect = area_response.response.rect;

            // Check for click outside the popup to close it
            let ctx = ui.ctx();
            if ctx.input(|i| i.pointer.any_pressed()) {
                if let Some(mouse_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    // Check if click is outside both popup and the link/button hover zone
                    if !popup_rect.contains(mouse_pos) && !hover_zone.contains(mouse_pos) {
                        should_close = true;
                        // Commit any changes made before closing
                        if self.state.is_modified() {
                            self.state.commit();
                            committed_changes = true;
                        }
                    }
                }
            }

            if should_close {
                self.state.popup_open = false;
            }
        }

        // Determine if we need to report changes
        let changed = committed_changes;
        let is_autolink = self.state.is_autolink();

        // Generate markdown - for autolinks, just return the URL (no markdown syntax)
        let markdown = if is_autolink {
            self.state.edit_url.clone()
        } else if self.title.is_empty() {
            format!("[{}]({})", self.state.edit_text, self.state.edit_url)
        } else {
            format!(
                "[{}]({} \"{}\")",
                self.state.edit_text, self.state.edit_url, self.title
            )
        };

        RenderedLinkOutput {
            changed,
            text: self.state.edit_text.clone(),
            url: self.state.edit_url.clone(),
            markdown,
            is_autolink,
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
    // Heading Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_heading_h1() {
        let result = format_heading("Hello World", HeadingLevel::H1);
        assert_eq!(result, "# Hello World");
    }

    #[test]
    fn test_format_heading_h3() {
        let result = format_heading("Test", HeadingLevel::H3);
        assert_eq!(result, "### Test");
    }

    #[test]
    fn test_format_heading_trims_whitespace() {
        let result = format_heading("  Spaced  ", HeadingLevel::H2);
        assert_eq!(result, "## Spaced");
    }

    #[test]
    fn test_decrease_heading_level() {
        assert_eq!(decrease_heading_level(HeadingLevel::H1), HeadingLevel::H1);
        assert_eq!(decrease_heading_level(HeadingLevel::H2), HeadingLevel::H1);
        assert_eq!(decrease_heading_level(HeadingLevel::H6), HeadingLevel::H5);
    }

    #[test]
    fn test_increase_heading_level() {
        assert_eq!(increase_heading_level(HeadingLevel::H1), HeadingLevel::H2);
        assert_eq!(increase_heading_level(HeadingLevel::H5), HeadingLevel::H6);
        assert_eq!(increase_heading_level(HeadingLevel::H6), HeadingLevel::H6);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // List Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_bullet_list() {
        let items = vec![ListItem::new("First"), ListItem::new("Second")];
        let list_type = ListType::Bullet;
        let result = format_list(&items, &list_type);
        assert_eq!(result, "- First\n- Second");
    }

    #[test]
    fn test_format_ordered_list() {
        let items = vec![ListItem::new("First"), ListItem::new("Second")];
        let list_type = ListType::Ordered {
            start: 1,
            delimiter: '.',
        };
        let result = format_list(&items, &list_type);
        assert_eq!(result, "1. First\n2. Second");
    }

    #[test]
    fn test_format_task_list() {
        let items = vec![
            ListItem::task("Unchecked", false),
            ListItem::task("Checked", true),
        ];
        let list_type = ListType::Bullet;
        let result = format_list(&items, &list_type);
        assert_eq!(result, "- [ ] Unchecked\n- [x] Checked");
    }

    #[test]
    fn test_format_ordered_list_custom_start() {
        let items = vec![ListItem::new("Third"), ListItem::new("Fourth")];
        let list_type = ListType::Ordered {
            start: 3,
            delimiter: ')',
        };
        let result = format_list(&items, &list_type);
        assert_eq!(result, "3) Third\n4) Fourth");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Output Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_widget_output_unchanged() {
        let output = WidgetOutput::unchanged("test".to_string());
        assert!(!output.changed);
        assert_eq!(output.markdown, "test");
    }

    #[test]
    fn test_widget_output_modified() {
        let output = WidgetOutput::modified("test".to_string());
        assert!(output.changed);
        assert_eq!(output.markdown, "test");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // List Item Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_list_item_new() {
        let item = ListItem::new("Test");
        assert_eq!(item.text, "Test");
        assert!(!item.is_task);
        assert!(!item.checked);
    }

    #[test]
    fn test_list_item_task() {
        let item = ListItem::task("Task", true);
        assert_eq!(item.text, "Task");
        assert!(item.is_task);
        assert!(item.checked);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Colors Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_widget_colors_from_theme() {
        // Just verify colors are created without panic
        let dark = WidgetColors::from_theme(Theme::Dark, &egui::Visuals::dark());
        let light = WidgetColors::from_theme(Theme::Light, &egui::Visuals::light());

        assert!(dark.text.r() > 200); // Light text on dark
        assert!(light.text.r() < 50); // Dark text on light
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Table Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_table_cell_data_new() {
        let cell = TableCellData::new("Test content");
        assert_eq!(cell.text, "Test content");
    }

    #[test]
    fn test_table_data_new() {
        let table = TableData::new(3, 2);
        assert_eq!(table.num_columns, 3);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.alignments.len(), 3);
        assert!(table.rows[0].iter().all(|c| c.text.is_empty()));
    }

    #[test]
    fn test_table_data_add_row() {
        let mut table = TableData::new(2, 1);
        assert_eq!(table.rows.len(), 1);
        table.add_row();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1].len(), 2);
    }

    #[test]
    fn test_table_data_insert_row() {
        let mut table = TableData::new(2, 2);
        table.rows[0][0].text = "Header".to_string();
        table.rows[1][0].text = "Data".to_string();

        table.insert_row(1);
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[0][0].text, "Header");
        assert_eq!(table.rows[1][0].text, ""); // New row
        assert_eq!(table.rows[2][0].text, "Data");
    }

    #[test]
    fn test_table_data_remove_row() {
        let mut table = TableData::new(2, 3);
        table.rows[0][0].text = "Header".to_string();
        table.rows[1][0].text = "Row 1".to_string();
        table.rows[2][0].text = "Row 2".to_string();

        table.remove_row(1);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1][0].text, "Row 2");
    }

    #[test]
    fn test_table_data_remove_row_protects_last() {
        let mut table = TableData::new(2, 1);
        table.remove_row(0);
        assert_eq!(table.rows.len(), 1); // Should not remove last row
    }

    #[test]
    fn test_table_data_add_column() {
        let mut table = TableData::new(2, 2);
        table.add_column();
        assert_eq!(table.num_columns, 3);
        assert_eq!(table.alignments.len(), 3);
        assert_eq!(table.rows[0].len(), 3);
        assert_eq!(table.rows[1].len(), 3);
    }

    #[test]
    fn test_table_data_insert_column() {
        let mut table = TableData::new(2, 2);
        table.rows[0][0].text = "Col1".to_string();
        table.rows[0][1].text = "Col2".to_string();

        table.insert_column(1);
        assert_eq!(table.num_columns, 3);
        assert_eq!(table.rows[0][0].text, "Col1");
        assert_eq!(table.rows[0][1].text, ""); // New column
        assert_eq!(table.rows[0][2].text, "Col2");
    }

    #[test]
    fn test_table_data_remove_column() {
        let mut table = TableData::new(3, 2);
        table.rows[0][0].text = "A".to_string();
        table.rows[0][1].text = "B".to_string();
        table.rows[0][2].text = "C".to_string();

        table.remove_column(1);
        assert_eq!(table.num_columns, 2);
        assert_eq!(table.rows[0].len(), 2);
        assert_eq!(table.rows[0][0].text, "A");
        assert_eq!(table.rows[0][1].text, "C");
    }

    #[test]
    fn test_table_data_remove_column_protects_last() {
        let mut table = TableData::new(1, 2);
        table.remove_column(0);
        assert_eq!(table.num_columns, 1); // Should not remove last column
    }

    #[test]
    fn test_table_data_set_alignment() {
        use crate::markdown::parser::TableAlignment;

        let mut table = TableData::new(3, 2);
        table.set_column_alignment(0, TableAlignment::Left);
        table.set_column_alignment(1, TableAlignment::Center);
        table.set_column_alignment(2, TableAlignment::Right);

        assert_eq!(table.alignments[0], TableAlignment::Left);
        assert_eq!(table.alignments[1], TableAlignment::Center);
        assert_eq!(table.alignments[2], TableAlignment::Right);
    }

    #[test]
    fn test_table_data_cycle_alignment() {
        use crate::markdown::parser::TableAlignment;

        let mut table = TableData::new(1, 1);
        assert_eq!(table.alignments[0], TableAlignment::None);

        table.cycle_column_alignment(0);
        assert_eq!(table.alignments[0], TableAlignment::Left);

        table.cycle_column_alignment(0);
        assert_eq!(table.alignments[0], TableAlignment::Center);

        table.cycle_column_alignment(0);
        assert_eq!(table.alignments[0], TableAlignment::Right);

        table.cycle_column_alignment(0);
        assert_eq!(table.alignments[0], TableAlignment::None);
    }

    #[test]
    fn test_table_data_to_markdown_basic() {
        let mut table = TableData::new(2, 2);
        table.rows[0][0].text = "Header 1".to_string();
        table.rows[0][1].text = "Header 2".to_string();
        table.rows[1][0].text = "Cell 1".to_string();
        table.rows[1][1].text = "Cell 2".to_string();

        let markdown = table.to_markdown();
        assert!(markdown.contains("| Header 1"));
        assert!(markdown.contains("| Header 2"));
        assert!(markdown.contains("| Cell 1"));
        assert!(markdown.contains("| Cell 2"));
        assert!(markdown.contains("---")); // Separator
    }

    #[test]
    fn test_table_data_to_markdown_with_alignment() {
        use crate::markdown::parser::TableAlignment;

        let mut table = TableData::new(3, 2);
        table.rows[0][0].text = "Left".to_string();
        table.rows[0][1].text = "Center".to_string();
        table.rows[0][2].text = "Right".to_string();
        table.rows[1][0].text = "A".to_string();
        table.rows[1][1].text = "B".to_string();
        table.rows[1][2].text = "C".to_string();

        table.set_column_alignment(0, TableAlignment::Left);
        table.set_column_alignment(1, TableAlignment::Center);
        table.set_column_alignment(2, TableAlignment::Right);

        let markdown = table.to_markdown();
        assert!(markdown.contains(":--")); // Left align
        assert!(markdown.contains(":-")); // Center starts with :
        assert!(markdown.contains("-:")); // Right align ends with :
    }

    #[test]
    fn test_table_data_to_markdown_empty() {
        let table = TableData::new(0, 0);
        assert_eq!(table.to_markdown(), "");
    }

    #[test]
    fn test_table_row_count() {
        let table = TableData::new(2, 5);
        assert_eq!(table.row_count(), 5);
    }

    #[test]
    fn test_table_has_header() {
        let table = TableData::new(2, 2);
        assert!(table.has_header());

        let empty_table = TableData {
            rows: vec![],
            alignments: vec![],
            num_columns: 0,
        };
        assert!(!empty_table.has_header());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Link Data Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_link_data_new() {
        let link = LinkData::new("Click here", "https://example.com");
        assert_eq!(link.text, "Click here");
        assert_eq!(link.url, "https://example.com");
        assert!(link.title.is_empty());
    }

    #[test]
    fn test_link_data_with_title() {
        let link = LinkData::with_title("Click here", "https://example.com", "Example Site");
        assert_eq!(link.text, "Click here");
        assert_eq!(link.url, "https://example.com");
        assert_eq!(link.title, "Example Site");
    }

    #[test]
    fn test_link_data_to_markdown_simple() {
        let link = LinkData::new("Click here", "https://example.com");
        assert_eq!(link.to_markdown(), "[Click here](https://example.com)");
    }

    #[test]
    fn test_link_data_to_markdown_with_title() {
        let link = LinkData::with_title("Click here", "https://example.com", "Example Site");
        assert_eq!(
            link.to_markdown(),
            "[Click here](https://example.com \"Example Site\")"
        );
    }

    #[test]
    fn test_link_data_to_markdown_empty_text() {
        let link = LinkData::new("", "https://example.com");
        assert_eq!(link.to_markdown(), "[](https://example.com)");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Inline Formatting Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_bold() {
        assert_eq!(format_bold("text"), "**text**");
    }

    #[test]
    fn test_format_italic() {
        assert_eq!(format_italic("text"), "*text*");
    }

    #[test]
    fn test_format_strikethrough() {
        assert_eq!(format_strikethrough("text"), "~~text~~");
    }

    #[test]
    fn test_format_inline_code() {
        assert_eq!(format_inline_code("code"), "`code`");
    }

    #[test]
    fn test_is_bold() {
        assert!(is_bold("**bold**"));
        assert!(is_bold("**bold text**"));
        assert!(!is_bold("*italic*"));
        assert!(!is_bold("plain text"));
        assert!(!is_bold("****")); // Too short
        assert!(!is_bold("**")); // Too short
    }

    #[test]
    fn test_is_italic() {
        assert!(is_italic("*italic*"));
        assert!(is_italic("_italic_"));
        assert!(!is_italic("**bold**"));
        assert!(!is_italic("plain text"));
        assert!(!is_italic("*")); // Too short
        assert!(!is_italic("__bold__")); // Double underscore is bold, not italic
    }

    #[test]
    fn test_unwrap_bold() {
        assert_eq!(unwrap_bold("**bold**"), "bold");
        assert_eq!(unwrap_bold("**bold text**"), "bold text");
        assert_eq!(unwrap_bold("plain text"), "plain text"); // No change if not bold
    }

    #[test]
    fn test_unwrap_italic() {
        assert_eq!(unwrap_italic("*italic*"), "italic");
        assert_eq!(unwrap_italic("_italic_"), "italic");
        assert_eq!(unwrap_italic("plain text"), "plain text"); // No change if not italic
    }

    #[test]
    fn test_toggle_bold() {
        assert_eq!(toggle_bold("text"), "**text**");
        assert_eq!(toggle_bold("**text**"), "text");
    }

    #[test]
    fn test_toggle_italic() {
        assert_eq!(toggle_italic("text"), "*text*");
        assert_eq!(toggle_italic("*text*"), "text");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Code Block Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_code_block_data_new() {
        let data = CodeBlockData::new("let x = 5;", "rust");
        assert_eq!(data.code, "let x = 5;");
        assert_eq!(data.language, "rust");
        assert!(!data.is_editing);
        assert!(!data.is_modified());
    }

    #[test]
    fn test_code_block_data_modification_detection() {
        let mut data = CodeBlockData::new("code", "rust");
        assert!(!data.is_modified());

        data.code = "modified code".to_string();
        assert!(data.is_modified());

        data.mark_saved();
        assert!(!data.is_modified());
    }

    #[test]
    fn test_code_block_data_language_change() {
        let mut data = CodeBlockData::new("code", "rust");
        assert!(!data.is_modified());

        data.language = "python".to_string();
        assert!(data.is_modified());
    }

    #[test]
    fn test_code_block_to_markdown_with_language() {
        let data = CodeBlockData::new("fn main() {}", "rust");
        assert_eq!(data.to_markdown(), "```rust\nfn main() {}\n```");
    }

    #[test]
    fn test_code_block_to_markdown_no_language() {
        let data = CodeBlockData::new("plain text", "");
        assert_eq!(data.to_markdown(), "```\nplain text\n```");
    }

    #[test]
    fn test_code_block_to_markdown_multiline() {
        let data = CodeBlockData::new("line1\nline2\nline3", "python");
        assert_eq!(data.to_markdown(), "```python\nline1\nline2\nline3\n```");
    }

    #[test]
    fn test_language_display_name() {
        assert_eq!(language_display_name("rust"), "Rust");
        assert_eq!(language_display_name("python"), "Python");
        assert_eq!(language_display_name("javascript"), "JavaScript");
        assert_eq!(language_display_name(""), "Plain Text");
        assert_eq!(language_display_name("cpp"), "C++");
        assert_eq!(language_display_name("csharp"), "C#");
    }

    #[test]
    fn test_normalize_language() {
        assert_eq!(normalize_language("rs"), "rust");
        assert_eq!(normalize_language("Rust"), "rust");
        assert_eq!(normalize_language("RUST"), "rust");
        assert_eq!(normalize_language("py"), "python");
        assert_eq!(normalize_language("js"), "javascript");
        assert_eq!(normalize_language("ts"), "typescript");
        assert_eq!(normalize_language("c++"), "cpp");
        assert_eq!(normalize_language("sh"), "bash");
        assert_eq!(normalize_language(""), "");
        assert_eq!(normalize_language("unknown_lang"), "");
    }

    #[test]
    fn test_supported_languages_contains_common() {
        assert!(SUPPORTED_LANGUAGES.contains(&"rust"));
        assert!(SUPPORTED_LANGUAGES.contains(&"python"));
        assert!(SUPPORTED_LANGUAGES.contains(&"javascript"));
        assert!(SUPPORTED_LANGUAGES.contains(&""));
    }

    #[test]
    fn test_code_block_output_fields() {
        let output = CodeBlockOutput {
            changed: true,
            language_changed: true,
            markdown: "```rust\ncode\n```".to_string(),
            code: "code".to_string(),
            language: "rust".to_string(),
        };
        assert!(output.changed);
        assert!(output.language_changed);
        assert_eq!(output.code, "code");
        assert_eq!(output.language, "rust");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Rendered Link State Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_rendered_link_state_new() {
        let state = RenderedLinkState::new("Click here", "https://example.com");
        assert_eq!(state.edit_text, "Click here");
        assert_eq!(state.edit_url, "https://example.com");
        assert!(!state.popup_open);
        assert!(!state.is_modified());
    }

    #[test]
    fn test_rendered_link_state_modification_detection() {
        let mut state = RenderedLinkState::new("Text", "https://example.com");
        assert!(!state.is_modified());

        state.edit_text = "New Text".to_string();
        assert!(state.is_modified());

        state.commit();
        assert!(!state.is_modified());
    }

    #[test]
    fn test_rendered_link_state_url_modification() {
        let mut state = RenderedLinkState::new("Text", "https://example.com");
        assert!(!state.is_modified());

        state.edit_url = "https://new-url.com".to_string();
        assert!(state.is_modified());
    }

    #[test]
    fn test_rendered_link_state_commit() {
        let mut state = RenderedLinkState::new("Original", "https://original.com");
        state.edit_text = "Modified".to_string();
        state.edit_url = "https://modified.com".to_string();

        assert!(state.is_modified());

        state.commit();

        assert!(!state.is_modified());
        assert_eq!(state.edit_text, "Modified");
        assert_eq!(state.edit_url, "https://modified.com");
    }

    #[test]
    fn test_rendered_link_state_reset() {
        let mut state = RenderedLinkState::new("Original", "https://original.com");
        state.edit_text = "Modified".to_string();
        state.edit_url = "https://modified.com".to_string();

        assert!(state.is_modified());

        state.reset();

        assert!(!state.is_modified());
        assert_eq!(state.edit_text, "Original");
        assert_eq!(state.edit_url, "https://original.com");
    }

    #[test]
    fn test_rendered_link_output_fields() {
        let output = RenderedLinkOutput {
            changed: true,
            text: "Link Text".to_string(),
            url: "https://example.com".to_string(),
            markdown: "[Link Text](https://example.com)".to_string(),
            is_autolink: false,
        };
        assert!(output.changed);
        assert_eq!(output.text, "Link Text");
        assert_eq!(output.url, "https://example.com");
        assert_eq!(output.markdown, "[Link Text](https://example.com)");
        assert!(!output.is_autolink);
    }

    #[test]
    fn test_rendered_link_output_autolink() {
        let output = RenderedLinkOutput {
            changed: true,
            text: "https://example.com".to_string(),
            url: "https://example.com".to_string(),
            markdown: "https://example.com".to_string(), // Just the URL for autolinks
            is_autolink: true,
        };
        assert!(output.is_autolink);
        // For autolinks, markdown is just the URL (no [text](url) syntax)
        assert_eq!(output.markdown, "https://example.com");
    }
}
