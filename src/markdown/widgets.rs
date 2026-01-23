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
use eframe::egui::{self, Color32, FontId, Key, RichText, TextEdit, Ui};
use rust_i18n::t;

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
    /// Whether any cell currently has focus (for tables)
    pub has_focus: bool,
}

impl WidgetOutput {
    /// Create an unchanged output with the given markdown.
    pub fn unchanged(markdown: String) -> Self {
        Self {
            changed: false,
            markdown,
            has_focus: false,
        }
    }

    /// Create a changed output with the new markdown.
    pub fn modified(markdown: String) -> Self {
        Self {
            changed: true,
            markdown,
            has_focus: false,
        }
    }

    /// Set the focus state.
    pub fn with_focus(mut self, has_focus: bool) -> Self {
        self.has_focus = has_focus;
        self
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

/// State for tracking table cell editing and navigation.
#[derive(Debug, Clone, Default)]
pub struct TableEditState {
    /// Currently focused cell (row, column). None if no cell is focused.
    pub focused_cell: Option<(usize, usize)>,
    /// Cell that should receive focus on the next frame.
    pub pending_focus: Option<(usize, usize)>,
    /// Whether any cell had focus in the previous frame.
    /// Used to detect when focus leaves the table entirely.
    pub had_focus_last_frame: bool,
    /// Whether any cell content was modified while editing.
    /// Reset when focus leaves the table.
    pub content_modified: bool,
}

impl TableEditState {
    /// Create a new table edit state with no focused cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request focus on a specific cell.
    pub fn focus_cell(&mut self, row: usize, col: usize) {
        self.pending_focus = Some((row, col));
    }

    /// Clear focus from all cells.
    pub fn clear_focus(&mut self) {
        self.focused_cell = None;
        self.pending_focus = None;
    }

    /// Move to the next cell (right, then down to next row).
    pub fn move_next(&mut self, num_rows: usize, num_cols: usize) {
        if let Some((row, col)) = self.focused_cell {
            if col + 1 < num_cols {
                // Move right
                self.pending_focus = Some((row, col + 1));
            } else if row + 1 < num_rows {
                // Move to first cell of next row
                self.pending_focus = Some((row + 1, 0));
            }
            // If at last cell, stay there
        }
    }

    /// Move to the previous cell (left, then up to previous row).
    pub fn move_prev(&mut self, num_cols: usize) {
        if let Some((row, col)) = self.focused_cell {
            if col > 0 {
                // Move left
                self.pending_focus = Some((row, col - 1));
            } else if row > 0 {
                // Move to last cell of previous row
                self.pending_focus = Some((row - 1, num_cols - 1));
            }
            // If at first cell, stay there
        }
    }

    /// Move to the cell in the next row (same column).
    pub fn move_down(&mut self, num_rows: usize) {
        if let Some((row, col)) = self.focused_cell {
            if row + 1 < num_rows {
                self.pending_focus = Some((row + 1, col));
            }
            // If at last row, stay there
        }
    }

    /// Move to the cell in the previous row (same column).
    pub fn move_up(&mut self) {
        if let Some((row, col)) = self.focused_cell {
            if row > 0 {
                self.pending_focus = Some((row - 1, col));
            }
            // If at first row, stay there
        }
    }
}

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

        // Get or create the table edit state
        let mut edit_state: TableEditState = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(table_id.with("edit_state"), TableEditState::new)
                .clone()
        });

        // Track if we should signal a change to the source
        let mut changed = false;
        
        // Track if any cell has focus this frame
        let mut any_cell_has_focus = false;

        // Track actions to perform after iteration (to avoid borrow issues)
        let mut action: Option<TableAction> = None;

        // Track which cell to request focus on
        let pending_focus = edit_state.pending_focus.take();

        // Determine dark mode for styling
        let is_dark = colors.text.r() > 128;

        // Table styling colors - modern, subtle palette
        let header_bg = if is_dark {
            egui::Color32::from_rgb(40, 44, 52)
        } else {
            egui::Color32::from_rgb(248, 249, 250)
        };

        let cell_bg = if is_dark {
            egui::Color32::from_rgb(30, 33, 40)
        } else {
            egui::Color32::from_rgb(255, 255, 255)
        };

        let border_color = if is_dark {
            egui::Color32::from_rgb(55, 60, 70)
        } else {
            egui::Color32::from_rgb(222, 226, 230)
        };

        let hover_bg = if is_dark {
            egui::Color32::from_rgb(50, 55, 65)
        } else {
            egui::Color32::from_rgb(240, 242, 245)
        };

        let control_color = if is_dark {
            egui::Color32::from_rgb(140, 145, 155)
        } else {
            egui::Color32::from_rgb(130, 135, 145)
        };

        let control_hover_color = if is_dark {
            egui::Color32::from_rgb(200, 205, 215)
        } else {
            egui::Color32::from_rgb(80, 85, 95)
        };

        ui.add_space(4.0);

        // Main table frame with modern styling
        egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(0.0)
            .rounding(6.0)
            .shadow(if is_dark {
                egui::epaint::Shadow::NONE
            } else {
                egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 1.0),
                    blur: 3.0,
                    spread: 0.0,
                    color: egui::Color32::from_black_alpha(8),
                }
            })
            .show(ui, |ui| {
                // Calculate column widths based on content
                let min_col_width = 80.0_f32;
                let char_width = self.font_size * 0.6;
                let cell_padding = 28.0_f32;

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

                        let text_width = (max_text_len as f32 * char_width) + cell_padding;
                        text_width.max(min_col_width).min(400.0)
                    })
                    .collect();

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;

                    // Render each row
                    for row_idx in 0..self.data.rows.len() {
                        let is_header = row_idx == 0;
                        let row_bg = if is_header { header_bg } else { cell_bg };

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;

                            // Render cells for this row
                            for col_idx in 0..self.data.num_columns {
                                let col_width = col_widths.get(col_idx).copied().unwrap_or(min_col_width);
                                let is_last_col = col_idx == self.data.num_columns - 1;

                                // Cell styling with subtle borders
                                let cell_stroke = if is_last_col {
                                    egui::Stroke::NONE
                                } else {
                                    egui::Stroke::new(1.0, border_color)
                                };

                                let cell_response = egui::Frame::none()
                                    .fill(row_bg)
                                    .stroke(egui::Stroke::NONE)
                                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
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

                                                let font = if is_header {
                                                    FontId::proportional(self.font_size)
                                                } else {
                                                    FontId::proportional(self.font_size)
                                                };

                                                ui.set_min_width(col_width);

                                                let text_edit = TextEdit::singleline(&mut cell.text)
                                                    .id(cell_id)
                                                    .font(font)
                                                    .text_color(text_color)
                                                    .frame(false)
                                                    .desired_width(col_width);

                                                let output = text_edit.show(ui);
                                                let response = output.response;

                                                if pending_focus == Some((row_idx, col_idx)) {
                                                    response.request_focus();
                                                }

                                                if response.has_focus() {
                                                    edit_state.focused_cell = Some((row_idx, col_idx));
                                                    any_cell_has_focus = true;

                                                    let num_rows = self.data.rows.len();
                                                    let num_cols = self.data.num_columns;

                                                    if ui.input(|i| i.key_pressed(Key::Tab) && !i.modifiers.shift) {
                                                        edit_state.move_next(num_rows, num_cols);
                                                    } else if ui.input(|i| i.key_pressed(Key::Tab) && i.modifiers.shift) {
                                                        edit_state.move_prev(num_cols);
                                                    } else if ui.input(|i| i.key_pressed(Key::Enter)) {
                                                        edit_state.move_down(num_rows);
                                                    } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                                                        edit_state.clear_focus();
                                                        ui.memory_mut(|mem| mem.surrender_focus(cell_id));
                                                    }
                                                }

                                                if response.changed() {
                                                    edit_state.content_modified = true;
                                                }
                                            }
                                        }
                                    });

                                // Draw right border AFTER cell content is rendered,
                                // using the actual cell bounds from the Frame response
                                if !is_last_col {
                                    let cell_rect = cell_response.response.rect;
                                    ui.painter().vline(
                                        cell_rect.right(),
                                        cell_rect.y_range(),
                                        cell_stroke,
                                    );
                                }
                            }

                            // Row controls (always visible when controls enabled)
                            if self.show_controls {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    
                                    // Delete row button (if more than one row)
                                    if self.data.rows.len() > 1 {
                                        let del_btn = ui.add(
                                            egui::Button::new(
                                                RichText::new("×")
                                                    .size(self.font_size * 0.9)
                                                    .color(control_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(18.0, 18.0))
                                        );
                                        if del_btn.hovered() {
                                            ui.painter().text(
                                                del_btn.rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "×",
                                                FontId::proportional(self.font_size * 0.9),
                                                control_hover_color,
                                            );
                                        }
                                        if del_btn.on_hover_text("Delete row").clicked() {
                                            action = Some(TableAction::RemoveRow(row_idx));
                                        }
                                    }
                                });
                            }
                        });
                    }

                    // Modern toolbar at bottom
                    if self.show_controls {
                        ui.add_space(2.0);
                        
                        // Subtle toolbar background
                        egui::Frame::none()
                            .fill(hover_bg)
                            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                            .rounding(egui::Rounding {
                                nw: 0.0,
                                ne: 0.0,
                                sw: 6.0,
                                se: 6.0,
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;

                                    // Add row button
                                    let add_row_btn = ui.add(
                                        egui::Button::new(
                                            RichText::new(t!("widgets.table.add_row").to_string())
                                                .size(self.font_size * 0.85)
                                                .color(control_color)
                                        )
                                        .frame(false)
                                    );
                                    if add_row_btn.hovered() {
                                        ui.painter().text(
                                            add_row_btn.rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            t!("widgets.table.add_row").to_string(),
                                            FontId::proportional(self.font_size * 0.85),
                                            control_hover_color,
                                        );
                                    }
                                    if add_row_btn.on_hover_text("Add a new row").clicked() {
                                        action = Some(TableAction::AddRow);
                                    }

                                    // Add column button
                                    let add_col_btn = ui.add(
                                        egui::Button::new(
                                            RichText::new(t!("widgets.table.add_column").to_string())
                                                .size(self.font_size * 0.85)
                                                .color(control_color)
                                        )
                                        .frame(false)
                                    );
                                    if add_col_btn.hovered() {
                                        ui.painter().text(
                                            add_col_btn.rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            t!("widgets.table.add_column").to_string(),
                                            FontId::proportional(self.font_size * 0.85),
                                            control_hover_color,
                                        );
                                    }
                                    if add_col_btn.on_hover_text("Add a new column").clicked() {
                                        action = Some(TableAction::AddColumn);
                                    }

                                    // Separator
                                    if self.data.num_columns > 1 {
                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(4.0);

                                        // Column delete buttons (compact)
                                        ui.label(
                                            RichText::new(t!("widgets.table.delete_column_label").to_string())
                                                .size(self.font_size * 0.8)
                                                .color(control_color)
                                        );
                                        
                                        for col in 0..self.data.num_columns {
                                            let col_label = format!("{}", col + 1);
                                            let del_col_btn = ui.add(
                                                egui::Button::new(
                                                    RichText::new(&col_label)
                                                        .size(self.font_size * 0.8)
                                                        .color(control_color)
                                                )
                                                .frame(false)
                                                .min_size(egui::vec2(16.0, 16.0))
                                            );
                                            if del_col_btn.hovered() {
                                                ui.painter().text(
                                                    del_col_btn.rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    &col_label,
                                                    FontId::proportional(self.font_size * 0.8),
                                                    control_hover_color,
                                                );
                                            }
                                            if del_col_btn
                                                .on_hover_text(t!("widgets.table.delete_column", index = (col + 1).to_string()).to_string())
                                                .clicked()
                                            {
                                                action = Some(TableAction::RemoveColumn(col));
                                            }
                                        }
                                    }

                                    // Alignment controls (if enabled)
                                    if self.show_alignment_controls && self.data.num_columns > 0 {
                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(4.0);

                                        ui.label(
                                            RichText::new(t!("widgets.table.align_label").to_string())
                                                .size(self.font_size * 0.8)
                                                .color(control_color)
                                        );

                                        for col in 0..self.data.num_columns {
                                            let align = self
                                                .data
                                                .alignments
                                                .get(col)
                                                .copied()
                                                .unwrap_or(TableAlignment::None);

                                            let (align_icon, tooltip) = match align {
                                                TableAlignment::Left => ("⬅", t!("widgets.table.align_left").to_string()),
                                                TableAlignment::Center => ("⬌", t!("widgets.table.align_center").to_string()),
                                                TableAlignment::Right => ("➡", t!("widgets.table.align_right").to_string()),
                                                TableAlignment::None => ("—", t!("widgets.table.align_none").to_string()),
                                            };

                                            let align_btn = ui.add(
                                                egui::Button::new(
                                                    RichText::new(align_icon)
                                                        .size(self.font_size * 0.8)
                                                        .color(control_color)
                                                )
                                                .frame(false)
                                            );
                                            if align_btn.on_hover_text(format!("{} (click to cycle)", tooltip)).clicked() {
                                                action = Some(TableAction::CycleAlignment(col));
                                            }
                                        }
                                    }
                                });
                            });
                    }
                });
            });

        ui.add_space(4.0);

        // Apply the action (after the UI iteration is complete)
        // Actions like add/remove row/column should trigger immediate change
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
            // Clear content_modified since we're committing via action
            edit_state.content_modified = false;
        }

        // Detect focus loss: had focus last frame but not this frame
        // This is when we commit cell edits to the source
        let focus_lost = edit_state.had_focus_last_frame && !any_cell_has_focus;
        
        if focus_lost && edit_state.content_modified {
            // Focus left the table and content was modified - signal change
            changed = true;
            edit_state.content_modified = false;
        }
        
        // Update focus tracking for next frame
        edit_state.had_focus_last_frame = any_cell_has_focus;

        // Check if any cell has focus (for output)
        let has_focus = any_cell_has_focus;

        // Save the edit state back to memory
        ui.memory_mut(|mem| {
            mem.data.insert_temp(table_id.with("edit_state"), edit_state);
        });

        // Generate markdown output
        let markdown = self.data.to_markdown();

        // Only report as modified when explicitly set (focus lost with edits, or action performed)
        // Don't use markdown comparison - edits are buffered until focus leaves the table
        if changed {
            WidgetOutput::modified(markdown).with_focus(has_focus)
        } else {
            WidgetOutput::unchanged(markdown).with_focus(has_focus)
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
    "jsx",
    "tsx",
    "go",
    "java",
    "c",
    "cpp",
    "csharp",
    "html",
    "css",
    "scss",
    "json",
    "yaml",
    "toml",
    "markdown",
    "bash",
    "powershell",
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
    "graphql",
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
        "jsx" => "JSX",
        "tsx" => "TSX",
        "go" => "Go",
        "java" => "Java",
        "c" => "C",
        "cpp" | "c++" => "C++",
        "csharp" | "cs" | "c#" => "C#",
        "html" => "HTML",
        "css" => "CSS",
        "scss" => "SCSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "markdown" | "md" => "Markdown",
        "bash" | "sh" | "shell" => "Bash",
        "powershell" | "ps1" => "PowerShell",
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
        "graphql" | "gql" => "GraphQL",
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
        "jsx" => "jsx",
        "tsx" => "tsx",
        "go" | "golang" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" | "cxx" => "cpp",
        "csharp" | "cs" | "c#" => "csharp",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "markdown" | "md" => "markdown",
        "bash" | "sh" | "shell" | "zsh" => "bash",
        "powershell" | "ps1" => "powershell",
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
        "graphql" | "gql" => "graphql",
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
                            .add(egui::Button::new(t!("common.copy").to_string()).small())
                            .on_hover_text(t!("widgets.code_block.copy_tooltip").to_string())
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
                                t!("widgets.code_block.finish_tooltip").to_string()
                            } else {
                                t!("widgets.code_block.edit_tooltip").to_string()
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

                // Wrap code content in horizontal scroll area to prevent width overflow.
                // This ensures long code lines scroll horizontally instead of expanding
                // the parent layout and breaking max_line_width for subsequent content.
                // See: ROADMAP.md "Blockquote/code block overflow"
                egui::ScrollArea::horizontal()
                    .id_source(block_id.with("scroll"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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

            settings_response.on_hover_text(t!("widgets.link.edit").to_string());
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
                                        RichText::new(t!("widgets.link.text_label").to_string())
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
                                    RichText::new(t!("widgets.link.url_label").to_string())
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
                                    ui.add_enabled(can_open, egui::Button::new(t!("widgets.link.open").to_string()));

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
                                    .button(t!("widgets.link.copy").to_string())
                                    .on_hover_text(t!("widgets.link.copy_tooltip").to_string())
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
// Mermaid Diagram Widget
// ─────────────────────────────────────────────────────────────────────────────

/// The type of Mermaid diagram detected from source.
///
/// MermaidJS supports various diagram types, each with its own syntax.
/// This enum helps identify the diagram type for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MermaidDiagramType {
    /// Flowchart diagrams (flowchart, graph)
    Flowchart,
    /// Sequence diagrams
    Sequence,
    /// Class diagrams
    Class,
    /// State diagrams
    State,
    /// Entity-Relationship diagrams
    EntityRelationship,
    /// User Journey diagrams
    UserJourney,
    /// Gantt charts
    Gantt,
    /// Pie charts
    Pie,
    /// Quadrant charts
    Quadrant,
    /// Requirement diagrams
    Requirement,
    /// Git graph diagrams
    GitGraph,
    /// C4 diagrams
    C4,
    /// Mindmap diagrams
    Mindmap,
    /// Timeline diagrams
    Timeline,
    /// ZenUML diagrams
    ZenUML,
    /// Sankey diagrams
    Sankey,
    /// XY charts
    XYChart,
    /// Block diagrams
    Block,
    /// Unknown or unrecognized diagram type
    Unknown,
}

impl MermaidDiagramType {
    /// Get a human-readable display name for the diagram type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Flowchart => "Flowchart",
            Self::Sequence => "Sequence Diagram",
            Self::Class => "Class Diagram",
            Self::State => "State Diagram",
            Self::EntityRelationship => "Entity-Relationship Diagram",
            Self::UserJourney => "User Journey",
            Self::Gantt => "Gantt Chart",
            Self::Pie => "Pie Chart",
            Self::Quadrant => "Quadrant Chart",
            Self::Requirement => "Requirement Diagram",
            Self::GitGraph => "Git Graph",
            Self::C4 => "C4 Diagram",
            Self::Mindmap => "Mindmap",
            Self::Timeline => "Timeline",
            Self::ZenUML => "ZenUML Diagram",
            Self::Sankey => "Sankey Diagram",
            Self::XYChart => "XY Chart",
            Self::Block => "Block Diagram",
            Self::Unknown => "Diagram",
        }
    }

    /// Get an icon/emoji representing the diagram type.
    /// Uses simple single-codepoint characters to avoid rendering issues.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Flowchart => "📊",
            Self::Sequence => "⇆",      // Simple bidirectional arrow (no variation selector)
            Self::Class => "◇",         // Diamond shape for class diagrams
            Self::State => "⟳",         // Circular arrow for state
            Self::EntityRelationship => "🔗",
            Self::UserJourney => "👤",   // Person silhouette
            Self::Gantt => "📅",
            Self::Pie => "◔",           // Circle with quarter fill for pie charts
            Self::Quadrant => "📐",
            Self::Requirement => "📋",
            Self::GitGraph => "🌳",
            Self::C4 => "🏢",
            Self::Mindmap => "💭",       // Thought bubble for mindmap
            Self::Timeline => "⏳",
            Self::ZenUML => "📦",
            Self::Sankey => "≋",        // Triple tilde for flow/sankey
            Self::XYChart => "📈",
            Self::Block => "▦",         // Grid pattern for blocks
            Self::Unknown => "📊",
        }
    }
}

/// Detect the diagram type from mermaid source code.
///
/// Parses the first non-empty, non-comment line to identify the diagram type.
/// MermaidJS diagram definitions start with a keyword indicating the type.
///
/// # Examples
/// ```ignore
/// let diagram_type = detect_mermaid_diagram_type("flowchart TD\n  A --> B");
/// assert_eq!(diagram_type, MermaidDiagramType::Flowchart);
/// ```
pub fn detect_mermaid_diagram_type(source: &str) -> MermaidDiagramType {
    // Find the first non-empty, non-comment line
    let first_line = source
        .lines()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !line.starts_with("%%"))
        .unwrap_or("");

    let first_line_lower = first_line.to_lowercase();

    // Check for diagram type keywords
    if first_line_lower.starts_with("flowchart")
        || first_line_lower.starts_with("graph")
        || first_line_lower.starts_with("flowchart-v2")
    {
        MermaidDiagramType::Flowchart
    } else if first_line_lower.starts_with("sequencediagram")
        || first_line_lower.starts_with("sequence")
    {
        MermaidDiagramType::Sequence
    } else if first_line_lower.starts_with("classdiagram")
        || first_line_lower.starts_with("class")
    {
        MermaidDiagramType::Class
    } else if first_line_lower.starts_with("statediagram")
        || first_line_lower.starts_with("state")
    {
        MermaidDiagramType::State
    } else if first_line_lower.starts_with("erdiagram") || first_line_lower.starts_with("er") {
        MermaidDiagramType::EntityRelationship
    } else if first_line_lower.starts_with("journey") {
        MermaidDiagramType::UserJourney
    } else if first_line_lower.starts_with("gantt") {
        MermaidDiagramType::Gantt
    } else if first_line_lower.starts_with("pie") {
        MermaidDiagramType::Pie
    } else if first_line_lower.starts_with("quadrantchart") {
        MermaidDiagramType::Quadrant
    } else if first_line_lower.starts_with("requirementdiagram")
        || first_line_lower.starts_with("requirement")
    {
        MermaidDiagramType::Requirement
    } else if first_line_lower.starts_with("gitgraph") {
        MermaidDiagramType::GitGraph
    } else if first_line_lower.starts_with("c4") {
        MermaidDiagramType::C4
    } else if first_line_lower.starts_with("mindmap") {
        MermaidDiagramType::Mindmap
    } else if first_line_lower.starts_with("timeline") {
        MermaidDiagramType::Timeline
    } else if first_line_lower.starts_with("zenuml") {
        MermaidDiagramType::ZenUML
    } else if first_line_lower.starts_with("sankey") {
        MermaidDiagramType::Sankey
    } else if first_line_lower.starts_with("xychart") {
        MermaidDiagramType::XYChart
    } else if first_line_lower.starts_with("block") {
        MermaidDiagramType::Block
    } else {
        MermaidDiagramType::Unknown
    }
}

/// Data for a mermaid diagram block.
#[derive(Debug, Clone)]
pub struct MermaidBlockData {
    /// The mermaid source code
    pub source: String,
    /// Detected diagram type
    pub diagram_type: MermaidDiagramType,
    /// Whether the block is expanded to show source
    pub show_source: bool,
    /// Cached SVG output from rendering (if available)
    pub rendered_svg: Option<String>,
    /// Error message if rendering failed
    pub render_error: Option<String>,
    /// Whether we're currently rendering
    pub is_rendering: bool,
    /// Original source (to detect changes)
    original_source: String,
}

impl MermaidBlockData {
    /// Create new mermaid block data from source.
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let diagram_type = detect_mermaid_diagram_type(&source);
        Self {
            original_source: source.clone(),
            source,
            diagram_type,
            show_source: false, // Default to rendered diagram view
            rendered_svg: None,
            render_error: None,
            is_rendering: false,
        }
    }

    /// Check if the source has been modified.
    pub fn is_modified(&self) -> bool {
        self.source != self.original_source
    }

    /// Mark the current state as saved.
    pub fn mark_saved(&mut self) {
        self.original_source = self.source.clone();
    }

    /// Convert to markdown (code block format).
    pub fn to_markdown(&self) -> String {
        format!("```mermaid\n{}\n```", self.source)
    }

    /// Update the diagram type based on current source.
    pub fn update_diagram_type(&mut self) {
        self.diagram_type = detect_mermaid_diagram_type(&self.source);
    }
}

/// Output from the mermaid block widget.
#[derive(Debug, Clone)]
pub struct MermaidBlockOutput {
    /// Whether the content was modified
    pub changed: bool,
    /// The mermaid source code
    pub source: String,
    /// The markdown representation
    pub markdown: String,
    /// Detected diagram type
    pub diagram_type: MermaidDiagramType,
}

/// A widget for displaying and editing mermaid diagrams.
///
/// This widget renders mermaid source code with:
/// - Diagram type detection and display
/// - Syntax-highlighted source view
/// - Visual distinction from regular code blocks
/// - Toggle between source and rendered views (when rendering available)
///
/// # Example
///
/// ```ignore
/// let mut data = MermaidBlockData::new("flowchart TD\n  A --> B");
///
/// let output = MermaidBlock::new(&mut data)
///     .font_size(14.0)
///     .dark_mode(true)
///     .show(ui);
///
/// if output.changed {
///     // Handle changes
/// }
/// ```
pub struct MermaidBlock<'a> {
    /// The mermaid block data
    data: &'a mut MermaidBlockData,
    /// Font size for the source code
    font_size: f32,
    /// Whether dark mode is active
    dark_mode: bool,
    /// Colors for styling
    colors: Option<WidgetColors>,
    /// Unique ID for this block
    id: Option<egui::Id>,
}

impl<'a> MermaidBlock<'a> {
    /// Create a new mermaid block widget.
    pub fn new(data: &'a mut MermaidBlockData) -> Self {
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

    /// Set a custom ID for the block.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Show the mermaid block widget and return the output.
    pub fn show(self, ui: &mut Ui) -> MermaidBlockOutput {
        use crate::markdown::mermaid::{render_mermaid_diagram, RenderResult};

        let _colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::from_theme(Theme::Light, ui.visuals()));

        // Use the provided ID or generate one
        let block_id = self.id.unwrap_or_else(|| egui::Id::new("mermaid_block"));

        // Track original source for change detection
        let original_source = self.data.source.clone();

        // Update diagram type if source changed
        if self.data.is_modified() {
            self.data.update_diagram_type();
        }

        // Styling based on dark mode
        let bg_color = if self.dark_mode {
            egui::Color32::from_rgb(35, 45, 55)
        } else {
            egui::Color32::from_rgb(240, 245, 250)
        };

        let border_color = if self.dark_mode {
            egui::Color32::from_rgb(60, 100, 140)
        } else {
            egui::Color32::from_rgb(150, 180, 210)
        };

        let header_bg = if self.dark_mode {
            egui::Color32::from_rgb(45, 60, 75)
        } else {
            egui::Color32::from_rgb(220, 235, 250)
        };

        let text_color = if self.dark_mode {
            egui::Color32::from_rgb(200, 210, 220)
        } else {
            egui::Color32::from_rgb(40, 50, 60)
        };

        let muted_color = if self.dark_mode {
            egui::Color32::from_rgb(140, 150, 160)
        } else {
            egui::Color32::from_rgb(100, 110, 120)
        };

        let accent_color = if self.dark_mode {
            egui::Color32::from_rgb(100, 160, 220)
        } else {
            egui::Color32::from_rgb(30, 100, 170)
        };

        // Main frame
        let frame = egui::Frame::none()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.5, border_color))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(0.0));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Header with diagram type indicator
                let header_frame = egui::Frame::none()
                    .fill(header_bg)
                    .rounding(egui::Rounding {
                        nw: 6.0,
                        ne: 6.0,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0));

                header_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Diagram type icon and name
                        ui.label(
                            RichText::new(self.data.diagram_type.icon())
                                .size(self.font_size + 2.0),
                        );
                        ui.label(
                            RichText::new(self.data.diagram_type.display_name())
                                .color(accent_color)
                                .strong()
                                .size(self.font_size),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Mermaid badge
                            ui.label(
                                RichText::new(t!("mermaid.badge").to_string())
                                    .color(muted_color)
                                    .italics()
                                    .size(self.font_size - 2.0),
                            );

                            // Toggle source view button
                            let toggle_text = if self.data.show_source {
                                "▼ Source"
                            } else {
                                "▶ Source"
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(toggle_text)
                                            .color(text_color)
                                            .size(self.font_size - 2.0),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.data.show_source = !self.data.show_source;
                            }
                        });
                    });
                });

                // Content area - show rendered diagram or source
                // Wrap in horizontal scroll area to handle wide diagrams without
                // breaking max_line_width for subsequent content.
                // See: ROADMAP.md "Blockquote/code block overflow"
                let content_frame = egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0));

                content_frame.show(ui, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_source(block_id.with("scroll"))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if self.data.show_source {
                                // Show source code
                                show_source_code(ui, block_id, &self.data.source, self.font_size, self.dark_mode, muted_color);
                            } else if self.data.source.trim().is_empty() {
                                // Empty diagram
                                ui.label(
                                    RichText::new(t!("mermaid.empty").to_string())
                                        .color(muted_color)
                                        .italics()
                                        .font(FontId::monospace(self.font_size)),
                                );
                            } else {
                                // Render diagram natively
                                let result = render_mermaid_diagram(ui, &self.data.source, self.dark_mode, self.font_size);
                                
                                match result {
                                    RenderResult::Success => {
                                        // Diagram rendered successfully
                                    }
                                    RenderResult::ParseError(msg) => {
                                        // Show parse error
                                        show_render_error(ui, &msg, muted_color, self.font_size, self.dark_mode);
                                        ui.add_space(8.0);
                                        show_source_code(ui, block_id, &self.data.source, self.font_size, self.dark_mode, muted_color);
                                    }
                                    RenderResult::Unsupported(msg) => {
                                        // Show unsupported message with source
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new("🚧")
                                                    .size(self.font_size * 2.0),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(&msg)
                                                    .color(accent_color)
                                                    .size(self.font_size),
                                            );
                                        });
                                        ui.add_space(8.0);
                                        show_source_code(ui, block_id, &self.data.source, self.font_size, self.dark_mode, muted_color);
                                    }
                                }
                            }
                        });
                });

                // Render error display (if any stored in data)
                if let Some(error) = &self.data.render_error {
                    let error_frame = egui::Frame::none()
                        .fill(if self.dark_mode {
                            egui::Color32::from_rgb(60, 30, 30)
                        } else {
                            egui::Color32::from_rgb(255, 240, 240)
                        })
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0));

                    error_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚠").color(egui::Color32::from_rgb(220, 80, 80)));
                            ui.label(
                                RichText::new(error)
                                    .color(if self.dark_mode {
                                        egui::Color32::from_rgb(255, 180, 180)
                                    } else {
                                        egui::Color32::from_rgb(180, 50, 50)
                                    })
                                    .size(self.font_size - 1.0),
                            );
                        });
                    });
                }
            });
        });

        // Check for changes
        let changed = self.data.source != original_source;

        MermaidBlockOutput {
            changed,
            source: self.data.source.clone(),
            markdown: self.data.to_markdown(),
            diagram_type: self.data.diagram_type,
        }
    }
}

/// Show source code with syntax highlighting.
fn show_source_code(
    ui: &mut Ui,
    block_id: egui::Id,
    source: &str,
    font_size: f32,
    dark_mode: bool,
    muted_color: egui::Color32,
) {
    use crate::markdown::syntax::highlight_code;
    
    let lines = highlight_code(source, "mermaid", dark_mode);

    egui::ScrollArea::vertical()
        .id_source(block_id.with("scroll"))
        .max_height(300.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                if lines.is_empty() {
                    ui.label(
                        RichText::new(t!("mermaid.empty").to_string())
                            .color(muted_color)
                            .italics()
                            .font(FontId::monospace(font_size)),
                    );
                } else {
                    for line in &lines {
                        ui.horizontal(|ui| {
                            for segment in &line.segments {
                                ui.label(segment.to_rich_text(font_size));
                            }
                        });
                    }
                }
            });
        });
}

/// Show render error message.
fn show_render_error(
    ui: &mut Ui,
    error: &str,
    _muted_color: egui::Color32,
    font_size: f32,
    dark_mode: bool,
) {
    let error_bg = if dark_mode {
        egui::Color32::from_rgb(60, 40, 40)
    } else {
        egui::Color32::from_rgb(255, 245, 245)
    };
    
    let error_text = if dark_mode {
        egui::Color32::from_rgb(255, 180, 180)
    } else {
        egui::Color32::from_rgb(180, 50, 50)
    };
    
    egui::Frame::none()
        .fill(error_bg)
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").color(error_text));
                ui.label(
                    RichText::new(t!("mermaid.rendering_error", error = error).to_string())
                        .color(error_text)
                        .size(font_size - 1.0),
                );
            });
        });
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

    // ─────────────────────────────────────────────────────────────────────────
    // Mermaid Diagram Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_detect_mermaid_flowchart() {
        assert_eq!(
            detect_mermaid_diagram_type("flowchart TD\n  A --> B"),
            MermaidDiagramType::Flowchart
        );
        assert_eq!(
            detect_mermaid_diagram_type("graph LR\n  A --> B"),
            MermaidDiagramType::Flowchart
        );
        assert_eq!(
            detect_mermaid_diagram_type("FLOWCHART TB\n  Start --> End"),
            MermaidDiagramType::Flowchart
        );
    }

    #[test]
    fn test_detect_mermaid_sequence() {
        assert_eq!(
            detect_mermaid_diagram_type("sequenceDiagram\n  Alice->>Bob: Hello"),
            MermaidDiagramType::Sequence
        );
    }

    #[test]
    fn test_detect_mermaid_class() {
        assert_eq!(
            detect_mermaid_diagram_type("classDiagram\n  Animal <|-- Duck"),
            MermaidDiagramType::Class
        );
    }

    #[test]
    fn test_detect_mermaid_state() {
        assert_eq!(
            detect_mermaid_diagram_type("stateDiagram-v2\n  [*] --> Still"),
            MermaidDiagramType::State
        );
    }

    #[test]
    fn test_detect_mermaid_er() {
        assert_eq!(
            detect_mermaid_diagram_type("erDiagram\n  CUSTOMER ||--o{ ORDER : places"),
            MermaidDiagramType::EntityRelationship
        );
    }

    #[test]
    fn test_detect_mermaid_journey() {
        assert_eq!(
            detect_mermaid_diagram_type("journey\n  title My working day"),
            MermaidDiagramType::UserJourney
        );
    }

    #[test]
    fn test_detect_mermaid_gantt() {
        assert_eq!(
            detect_mermaid_diagram_type("gantt\n  title A Gantt Diagram"),
            MermaidDiagramType::Gantt
        );
    }

    #[test]
    fn test_detect_mermaid_pie() {
        assert_eq!(
            detect_mermaid_diagram_type("pie title Pets\n  \"Dogs\" : 386"),
            MermaidDiagramType::Pie
        );
    }

    #[test]
    fn test_detect_mermaid_gitgraph() {
        assert_eq!(
            detect_mermaid_diagram_type("gitGraph\n  commit"),
            MermaidDiagramType::GitGraph
        );
    }

    #[test]
    fn test_detect_mermaid_mindmap() {
        assert_eq!(
            detect_mermaid_diagram_type("mindmap\n  root((mindmap))"),
            MermaidDiagramType::Mindmap
        );
    }

    #[test]
    fn test_detect_mermaid_timeline() {
        assert_eq!(
            detect_mermaid_diagram_type("timeline\n  title History of Events"),
            MermaidDiagramType::Timeline
        );
    }

    #[test]
    fn test_detect_mermaid_unknown() {
        assert_eq!(
            detect_mermaid_diagram_type("unknown diagram type"),
            MermaidDiagramType::Unknown
        );
        assert_eq!(
            detect_mermaid_diagram_type(""),
            MermaidDiagramType::Unknown
        );
    }

    #[test]
    fn test_detect_mermaid_with_comments() {
        // Should skip %% comment lines
        assert_eq!(
            detect_mermaid_diagram_type("%% This is a comment\nflowchart TD\n  A --> B"),
            MermaidDiagramType::Flowchart
        );
    }

    #[test]
    fn test_mermaid_block_data_new() {
        let data = MermaidBlockData::new("flowchart TD\n  A --> B");
        assert_eq!(data.diagram_type, MermaidDiagramType::Flowchart);
        assert!(!data.is_modified());
        assert!(!data.show_source); // Default to rendered diagram view
        assert!(data.rendered_svg.is_none());
        assert!(data.render_error.is_none());
    }

    #[test]
    fn test_mermaid_block_data_modification_detection() {
        let mut data = MermaidBlockData::new("flowchart TD\n  A --> B");
        assert!(!data.is_modified());

        data.source = "flowchart TD\n  A --> C".to_string();
        assert!(data.is_modified());

        data.mark_saved();
        assert!(!data.is_modified());
    }

    #[test]
    fn test_mermaid_block_data_to_markdown() {
        let data = MermaidBlockData::new("flowchart TD\n  A --> B");
        assert_eq!(data.to_markdown(), "```mermaid\nflowchart TD\n  A --> B\n```");
    }

    #[test]
    fn test_mermaid_block_data_update_diagram_type() {
        let mut data = MermaidBlockData::new("flowchart TD\n  A --> B");
        assert_eq!(data.diagram_type, MermaidDiagramType::Flowchart);

        data.source = "sequenceDiagram\n  Alice->>Bob: Hello".to_string();
        data.update_diagram_type();
        assert_eq!(data.diagram_type, MermaidDiagramType::Sequence);
    }

    #[test]
    fn test_mermaid_diagram_type_display_name() {
        assert_eq!(MermaidDiagramType::Flowchart.display_name(), "Flowchart");
        assert_eq!(MermaidDiagramType::Sequence.display_name(), "Sequence Diagram");
        assert_eq!(MermaidDiagramType::Class.display_name(), "Class Diagram");
        assert_eq!(MermaidDiagramType::Unknown.display_name(), "Diagram");
    }

    #[test]
    fn test_mermaid_diagram_type_icon() {
        assert!(!MermaidDiagramType::Flowchart.icon().is_empty());
        assert!(!MermaidDiagramType::Sequence.icon().is_empty());
        assert!(!MermaidDiagramType::Unknown.icon().is_empty());
    }

    #[test]
    fn test_mermaid_block_output_fields() {
        let output = MermaidBlockOutput {
            changed: true,
            source: "flowchart TD\n  A --> B".to_string(),
            markdown: "```mermaid\nflowchart TD\n  A --> B\n```".to_string(),
            diagram_type: MermaidDiagramType::Flowchart,
        };
        assert!(output.changed);
        assert_eq!(output.diagram_type, MermaidDiagramType::Flowchart);
    }
}
