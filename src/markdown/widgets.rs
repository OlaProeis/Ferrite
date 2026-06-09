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

use crate::config::{EditorFont, Theme};
use crate::fonts::get_styled_font_family;
use crate::markdown::ansi_render;
use crate::markdown::code_execution::{
    self as code_exec_mod, CodeExecutionUi, RunHandle, RunStatus,
};
use crate::markdown::parser::{
    CalloutType, HeadingLevel, ListType, MarkdownNode, MarkdownNodeType,
};
use crate::terminal::TerminalTheme;
use crate::ui::phosphor_icons::{
    phosphor_rich_text, ARROWS_CLOCKWISE, ARROWS_LEFT_RIGHT, BUILDINGS, CALENDAR, CARET_DOWN,
    CARET_RIGHT, CHART_BAR, CHART_LINE_UP, CHART_PIE, CHECK, DIAMOND, FLOW_ARROW, GIT_BRANCH,
    HOURGLASS, LINK, LIST_CHECKS, PACKAGE, PLAY, SQUARES_FOUR, STOP, TEXT_ALIGN_CENTER,
    TEXT_ALIGN_LEFT, TEXT_ALIGN_RIGHT, TREE_STRUCTURE, USER, WARNING, X,
};
use eframe::egui::{self, Color32, FontFamily, FontId, Key, RichText, TextEdit, Ui};
use rust_i18n::t;
use std::time::Duration;

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
    /// For tables: which cell is currently being interacted with (focus or pending focus).
    /// Lets callers synchronize external state (e.g. [`crate::markdown::rendered_session::RenderedEditSession`])
    /// with the user's effective edit target without parsing widget internals.
    pub focused_cell: Option<(usize, usize)>,
}

impl WidgetOutput {
    /// Create an unchanged output with the given markdown.
    pub fn unchanged(markdown: String) -> Self {
        Self {
            changed: false,
            markdown,
            has_focus: false,
            focused_cell: None,
        }
    }

    /// Create a changed output with the new markdown.
    pub fn modified(markdown: String) -> Self {
        Self {
            changed: true,
            markdown,
            has_focus: false,
            focused_cell: None,
        }
    }

    /// Set the focus state.
    pub fn with_focus(mut self, has_focus: bool) -> Self {
        self.has_focus = has_focus;
        self
    }

    /// Set the focused cell coordinates (tables only).
    pub fn with_focused_cell(mut self, cell: Option<(usize, usize)>) -> Self {
        self.focused_cell = cell;
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
    /// Create colors for the given theme. `accent` is used for heading text in widgets.
    pub fn from_theme(theme: Theme, visuals: &egui::Visuals, accent: Color32) -> Self {
        let is_dark = match theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => visuals.dark_mode,
        };

        if is_dark {
            Self {
                text: Color32::from_rgb(220, 220, 220),
                heading: accent,
                code_bg: Color32::from_rgb(45, 45, 45),
                list_marker: Color32::from_rgb(150, 150, 150),
                muted: Color32::from_rgb(120, 120, 120),
            }
        } else {
            Self {
                text: Color32::from_rgb(30, 30, 30),
                heading: accent,
                code_bg: Color32::from_rgb(245, 245, 245),
                list_marker: Color32::from_rgb(100, 100, 100),
                muted: Color32::from_rgb(150, 150, 150),
            }
        }
    }

    /// Resolve colors using markdown frame accent when set (see [`crate::markdown::MarkdownEditor`]).
    pub fn resolved(ui: &Ui, theme: Theme) -> Self {
        let accent = ui
            .ctx()
            .data(|d| d.get_temp::<Color32>(crate::markdown::markdown_accent_temp_id()))
            .unwrap_or_else(|| crate::theme::accent::default_accent());
        Self::from_theme(theme, ui.visuals(), accent)
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
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

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
                    .on_hover_text(t!("widgets.list.decrease_level").to_string())
                    .clicked()
                {
                    *self.level = decrease_heading_level(*self.level);
                    changed = true;
                }
                if ui
                    .small_button("+")
                    .on_hover_text(t!("widgets.list.increase_level").to_string())
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
                    .frame(egui::Frame::NONE)
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
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

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
                    .frame(egui::Frame::NONE)
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
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

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
                        .frame(egui::Frame::NONE)
                        .desired_width(f32::INFINITY),
                );

                if response.changed() {
                    changed = true;
                }

                // Remove button (if controls enabled)
                if self.show_controls
                    && ui
                        .small_button(phosphor_rich_text(X, 12.0))
                        .on_hover_text(t!("widgets.list.remove_item").to_string())
                        .clicked()
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
                if ui.button(t!("widgets.list.add_item").to_string()).clicked() {
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

        MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } => {
            // Reconstruct the callout marker line
            let type_name = match callout_type {
                CalloutType::Note => "NOTE",
                CalloutType::Tip => "TIP",
                CalloutType::Warning => "WARNING",
                CalloutType::Caution => "CAUTION",
                CalloutType::Important => "IMPORTANT",
            };
            let collapse_marker = if *collapsed { "-" } else { "" };
            let title_part = match title {
                Some(t) => format!(" {}", t),
                None => String::new(),
            };
            let marker_line = format!("> [!{}]{}{}", type_name, collapse_marker, title_part);

            let inner = node
                .children
                .iter()
                .map(serialize_node)
                .collect::<Vec<_>>()
                .join("\n");

            if inner.is_empty() {
                marker_line
            } else {
                let content_lines = inner
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}", marker_line, content_lines)
            }
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

        MarkdownNodeType::VideoEmbed(info) => info.source_text.clone(),

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
                .map(|cell| serialize_inline_content(cell))
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
// Inline Markdown → LayoutJob (for table cell rich-text display)
// ─────────────────────────────────────────────────────────────────────────────

/// Build an egui `LayoutJob` that renders inline markdown formatting
/// (bold, italic, strikethrough, inline code) from raw markdown text.
pub(crate) fn build_inline_markdown_layout_job(
    text: &str,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
    link_color: Color32,
    code_bg: Color32,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    parse_inline_markdown(
        text,
        &mut job,
        false,
        false,
        false,
        font_size,
        editor_font,
        text_color,
        link_color,
        code_bg,
    );
    if job.sections.is_empty() {
        let family = get_styled_font_family(false, false, editor_font);
        job.append(
            text,
            0.0,
            egui::text::TextFormat {
                font_id: FontId::new(font_size, family),
                color: text_color,
                ..Default::default()
            },
        );
    }
    job
}

/// Build a LayoutJob for header cells (base bold, with inline formatting on top).
/// Map a display-mode click to a raw `cell.text` caret index (same galley as painted cell).
fn table_cell_raw_cursor_at_click(
    ui: &Ui,
    click_pos: egui::Pos2,
    cell_rect: egui::Rect,
    raw_text: &str,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
    code_bg: Color32,
    inner_w: f32,
    display_bold: bool,
) -> usize {
    if raw_text.is_empty() {
        return 0;
    }
    let job = if display_bold {
        build_cell_layout_job_with_base_bold(
            raw_text,
            font_size,
            editor_font,
            text_color,
            code_bg,
            inner_w,
        )
    } else {
        build_inline_markdown_layout_job(
            raw_text,
            font_size,
            editor_font,
            text_color,
            text_color,
            code_bg,
            inner_w,
        )
    };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let local_pos = egui::Vec2::new(
        click_pos.x - cell_rect.min.x,
        click_pos.y - cell_rect.min.y,
    );
    let displayed_idx = galley.cursor_from_pos(local_pos).index;
    map_displayed_to_raw(displayed_idx, raw_text).min(raw_text.chars().count())
}

/// Maps a cursor position in displayed text (without formatting markers) to the
/// corresponding position in raw markdown text (with formatting markers).
pub(crate) fn map_displayed_to_raw(displayed_idx: usize, raw_text: &str) -> usize {
    let chars: Vec<char> = raw_text.chars().collect();
    let mut raw_pos = 0;
    let mut displayed_pos = 0;

    while raw_pos < chars.len() {
        let remaining: String = chars[raw_pos..].iter().collect();

        if remaining.starts_with("**") || remaining.starts_with("__") || remaining.starts_with("~~")
        {
            raw_pos += 2;
            continue;
        }

        if chars[raw_pos] == '[' && raw_pos + 1 < chars.len() && chars[raw_pos + 1] == '[' {
            // Wikilink: [[target]] or [[target|display]] — count only visible text.
            raw_pos += 2;
            let content_start = raw_pos;
            while raw_pos + 1 < chars.len() {
                if chars[raw_pos] == ']' && chars[raw_pos + 1] == ']' {
                    let content: String = chars[content_start..raw_pos].iter().collect();
                    let visible = content
                        .split_once('|')
                        .map(|(_, display)| display)
                        .unwrap_or(content.as_str());
                    for (byte_off, _) in visible.char_indices() {
                        if displayed_pos >= displayed_idx {
                            return content_start
                                + content
                                    .find('|')
                                    .map(|pipe| pipe + 1 + byte_off)
                                    .unwrap_or(byte_off);
                        }
                        displayed_pos += 1;
                    }
                    raw_pos += 2;
                    break;
                }
                raw_pos += 1;
            }
            continue;
        }

        if chars[raw_pos] == '[' {
            raw_pos += 1;
            continue;
        }

        if remaining.starts_with("](") {
            raw_pos += 2;
            let mut paren_depth = 1;
            while raw_pos < chars.len() && paren_depth > 0 {
                if chars[raw_pos] == '(' {
                    paren_depth += 1;
                } else if chars[raw_pos] == ')' {
                    paren_depth -= 1;
                }
                raw_pos += 1;
            }
            continue;
        }

        if chars[raw_pos] == '`' {
            raw_pos += 1;
            continue;
        }

        if (chars[raw_pos] == '*' || chars[raw_pos] == '_')
            && !remaining.starts_with("**")
            && !remaining.starts_with("__")
        {
            let prev_is_space = raw_pos == 0 || chars[raw_pos - 1].is_whitespace();
            let next_is_space = raw_pos + 1 >= chars.len() || chars[raw_pos + 1].is_whitespace();
            let next_is_same = raw_pos + 1 < chars.len() && chars[raw_pos + 1] == chars[raw_pos];

            if prev_is_space || next_is_space || !next_is_same {
                let marker = chars[raw_pos];
                let has_closing = chars[raw_pos + 1..].iter().any(|&c| c == marker);
                if has_closing {
                    raw_pos += 1;
                    continue;
                }
            }
        }

        if displayed_pos >= displayed_idx {
            return raw_pos;
        }

        raw_pos += 1;
        displayed_pos += 1;
    }

    raw_pos
}

fn build_cell_layout_job_with_base_bold(
    text: &str,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
    code_bg: Color32,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    parse_inline_markdown(
        text,
        &mut job,
        true,
        false,
        false,
        font_size,
        editor_font,
        text_color,
        text_color,
        code_bg,
    );
    if job.sections.is_empty() {
        let family = get_styled_font_family(true, false, editor_font);
        job.append(
            text,
            0.0,
            egui::text::TextFormat {
                font_id: FontId::new(font_size, family),
                color: text_color,
                ..Default::default()
            },
        );
    }
    job
}

/// Parse `[text](url)` at the start of `s`; returns link text and bytes consumed.
fn parse_markdown_link_span(s: &str) -> Option<(&str, usize)> {
    if s.starts_with("[[") || !s.starts_with('[') {
        return None;
    }
    let rest = &s[1..];
    let close_bracket = rest.find(']')?;
    if !rest[close_bracket..].starts_with("](") {
        return None;
    }
    let link_text = &rest[..close_bracket];
    let url = &rest[close_bracket + 2..];
    let mut depth = 1usize;
    for (idx, ch) in url.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let consumed = 1 + close_bracket + 2 + idx + ch.len_utf8();
                    return Some((link_text, consumed));
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse `[[target]]` / `[[target|display]]`; returns visible text and bytes consumed.
fn parse_wikilink_span(s: &str) -> Option<(&str, usize)> {
    if !s.starts_with("[[") {
        return None;
    }
    let inner = &s[2..];
    let close = inner.find("]]")?;
    let content = &inner[..close];
    let visible = content
        .split_once('|')
        .map(|(_, display)| display)
        .unwrap_or(content);
    Some((visible, 2 + close + 2))
}

fn append_link_span(
    job: &mut egui::text::LayoutJob,
    text: &str,
    bold: bool,
    italic: bool,
    strike: bool,
    font_size: f32,
    editor_font: &EditorFont,
    link_color: Color32,
) {
    if text.is_empty() {
        return;
    }
    let family = get_styled_font_family(bold, italic, editor_font);
    let mut fmt = egui::text::TextFormat {
        font_id: FontId::new(font_size, family),
        color: link_color,
        underline: egui::Stroke::new(1.0, link_color),
        ..Default::default()
    };
    if italic {
        fmt.italics = true;
    }
    if strike {
        fmt.strikethrough = egui::Stroke::new(1.0, link_color);
    }
    job.append(text, 0.0, fmt);
}

/// Recursively parse inline markdown and append formatted sections to a LayoutJob.
fn parse_inline_markdown(
    text: &str,
    job: &mut egui::text::LayoutJob,
    bold: bool,
    italic: bool,
    strike: bool,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
    link_color: Color32,
    code_bg: Color32,
) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut plain_start = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some((visible, consumed)) = parse_wikilink_span(&text[i..]) {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                append_link_span(
                    job,
                    visible,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    link_color,
                );
                i += consumed;
                plain_start = i;
                continue;
            }
        } else if bytes[i] == b'[' {
            if let Some((link_text, consumed)) = parse_markdown_link_span(&text[i..]) {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                append_link_span(
                    job,
                    link_text,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    link_color,
                );
                i += consumed;
                plain_start = i;
                continue;
            }
        }

        if i + 2 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' && bytes[i + 2] == b'*' {
            // *** bold+italic delimiter (must be checked before **)
            if let Some(close) = find_closing_delimiter(&text[i + 3..], "***") {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                parse_inline_markdown(
                    &text[i + 3..i + 3 + close],
                    job,
                    !bold,
                    !italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                    link_color,
                    code_bg,
                );
                i = i + 3 + close + 3;
                plain_start = i;
            } else {
                i += 3;
            }
        } else if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            // ** bold delimiter
            if let Some(close) = find_closing_delimiter(&text[i + 2..], "**") {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                parse_inline_markdown(
                    &text[i + 2..i + 2 + close],
                    job,
                    !bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                    link_color,
                    code_bg,
                );
                i = i + 2 + close + 2;
                plain_start = i;
            } else {
                i += 2;
            }
        } else if i + 1 < len && bytes[i] == b'~' && bytes[i + 1] == b'~' {
            // ~~ strikethrough delimiter
            if let Some(close) = find_closing_delimiter(&text[i + 2..], "~~") {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                parse_inline_markdown(
                    &text[i + 2..i + 2 + close],
                    job,
                    bold,
                    italic,
                    !strike,
                    font_size,
                    editor_font,
                    text_color,
                    link_color,
                    code_bg,
                );
                i = i + 2 + close + 2;
                plain_start = i;
            } else {
                i += 2;
            }
        } else if bytes[i] == b'`' {
            // Inline code (no nesting)
            if let Some(close) = find_closing_delimiter(&text[i + 1..], "`") {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                let code_text = &text[i + 1..i + 1 + close];
                job.append(
                    code_text,
                    0.0,
                    egui::text::TextFormat {
                        font_id: FontId::new(font_size * 0.9, FontFamily::Monospace),
                        color: text_color,
                        background: code_bg,
                        ..Default::default()
                    },
                );
                i = i + 1 + close + 1;
                plain_start = i;
            } else {
                i += 1;
            }
        } else if bytes[i] == b'*' && (i + 1 >= len || bytes[i + 1] != b'*') {
            // * italic delimiter (but not **)
            if let Some(close) = find_closing_single_star(&text[i + 1..]) {
                flush_plain(
                    text,
                    plain_start,
                    i,
                    job,
                    bold,
                    italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                );
                parse_inline_markdown(
                    &text[i + 1..i + 1 + close],
                    job,
                    bold,
                    !italic,
                    strike,
                    font_size,
                    editor_font,
                    text_color,
                    link_color,
                    code_bg,
                );
                i = i + 1 + close + 1;
                plain_start = i;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    flush_plain(
        text,
        plain_start,
        len,
        job,
        bold,
        italic,
        strike,
        font_size,
        editor_font,
        text_color,
    );
}

/// Flush accumulated plain text as a formatted LayoutJob section.
fn flush_plain(
    text: &str,
    start: usize,
    end: usize,
    job: &mut egui::text::LayoutJob,
    bold: bool,
    italic: bool,
    strike: bool,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
) {
    if start >= end {
        return;
    }
    let slice = &text[start..end];
    if slice.is_empty() {
        return;
    }
    let family = get_styled_font_family(bold, italic, editor_font);
    let mut fmt = egui::text::TextFormat {
        font_id: FontId::new(font_size, family),
        color: text_color,
        ..Default::default()
    };
    if italic {
        fmt.italics = true;
    }
    if strike {
        fmt.strikethrough = egui::Stroke::new(1.0, text_color);
    }
    job.append(slice, 0.0, fmt);
}

/// Find the position of a closing delimiter in `text`, returning the byte offset
/// of the start of the delimiter (i.e., the length of content before it).
fn find_closing_delimiter(text: &str, delimiter: &str) -> Option<usize> {
    text.find(delimiter)
}

/// Find a closing single `*` that is not part of `**`.
fn find_closing_single_star(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                i += 2; // skip **
            } else {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Editable Table Widget
// ─────────────────────────────────────────────────────────────────────────────

/// Shared across all [`EditableTable`] instances in one frame / document view.
#[derive(Debug, Clone, Default)]
struct TableGlobalFocus {
    /// Table that had a focused cell on the previous frame.
    active_table: Option<egui::Id>,
    active_cell: Option<(usize, usize)>,
    /// Cell the user clicked (may belong to a different table; set during layout).
    pending_cell: Option<(egui::Id, usize, usize)>,
}

fn table_global_focus_id() -> egui::Id {
    egui::Id::new("ferrite_table_global_focus")
}

/// Force-commit signal egui id for the table starting at `table_line`.
///
/// Written by [`RenderedEditSession`] commit callbacks when the session switches away
/// from a [`BlockRef::TableCell`](crate::markdown::rendered_session::BlockRef::TableCell);
/// read by [`EditableTable::show`] on its next frame.
fn table_force_commit_id(table_line: usize) -> egui::Id {
    egui::Id::new("ferrite_table_force_commit").with(table_line)
}

/// Request that the next render of the table starting at `table_line` commit dirty edits
/// to source immediately (regardless of focus state).
///
/// Used by `RenderedEditSession` when switching from a table cell to a non-table block,
/// so the table writes back without waiting for the existing focus-loss defer cycle.
pub fn signal_table_force_commit(ctx: &egui::Context, table_line: usize) {
    ctx.data_mut(|d| d.insert_temp(table_force_commit_id(table_line), true));
}

/// Consume the force-commit signal for `table_line` (one-shot).
fn take_table_force_commit(ui: &mut Ui, table_line: usize) -> bool {
    let id = table_force_commit_id(table_line);
    let v = ui
        .memory(|m| m.data.get_temp::<bool>(id))
        .unwrap_or(false);
    if v {
        ui.memory_mut(|m| m.data.remove::<bool>(id));
    }
    v
}

fn load_table_global_focus(ui: &Ui) -> TableGlobalFocus {
    ui.memory(|mem| {
        mem.data
            .get_temp::<TableGlobalFocus>(table_global_focus_id())
            .unwrap_or_default()
    })
}

fn save_table_global_focus(ui: &mut Ui, global: TableGlobalFocus) {
    ui.memory_mut(|mem| {
        mem.data.insert_temp(table_global_focus_id(), global);
    });
}

fn request_table_cell_focus(
    edit_state: &mut TableEditState,
    global: &mut TableGlobalFocus,
    ui: &mut Ui,
    table_id: egui::Id,
    _table_line: usize,
    row: usize,
    col: usize,
    cursor_char: Option<usize>,
) {
    if let Some((fr, fc)) = edit_state.focused_cell {
        if fr != row || fc != col {
            let prev_cell_id = table_id.with("cell").with(fr).with(fc);
            ui.memory_mut(|m| m.surrender_focus(prev_cell_id));
        }
    }
    if let Some(active_table) = global.active_table {
        if active_table != table_id {
            if let Some((fr, fc)) = global.active_cell {
                let prev_cell_id = active_table.with("cell").with(fr).with(fc);
                ui.memory_mut(|m| m.surrender_focus(prev_cell_id));
            }
        }
    }
    let cell_id = table_id.with("cell").with(row).with(col);
    ui.memory_mut(|m| m.request_focus(cell_id));
    edit_state.pending_focus = Some((row, col));
    edit_state.pending_cursor_char = cursor_char;
    global.pending_cell = Some((table_id, row, col));
}

/// State for tracking table cell editing and navigation.
#[derive(Debug, Clone, Default)]
pub struct TableEditState {
    /// Currently focused cell (row, column). None if no cell is focused.
    pub focused_cell: Option<(usize, usize)>,
    /// Cell that should receive focus on the next frame.
    pub pending_focus: Option<(usize, usize)>,
    /// Caret index in raw cell text when entering edit mode from a display click.
    pub pending_cursor_char: Option<usize>,
    /// Whether any cell had focus in the previous frame.
    /// Used to detect when focus leaves the table entirely.
    pub had_focus_last_frame: bool,
    /// Whether any cell content was modified while editing.
    /// Reset when focus leaves the table.
    pub content_modified: bool,
    /// Buffer edits until focus has fully left (see `defer_commit_age`).
    defer_commit: bool,
    /// Frames since `defer_commit` began; commit only after >= 2 so tables lower in
    /// the document can record `TableGlobalFocus::pending_cell` first.
    defer_commit_age: u8,
    /// User-resized column widths. None = use auto-calculated widths.
    /// Stored as absolute pixels; normalized to table_width on each frame.
    pub custom_col_widths: Option<Vec<f32>>,
}

impl TableEditState {
    /// Create a new table edit state with no focused cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request focus on a specific cell.
    pub fn focus_cell(&mut self, row: usize, col: usize) {
        self.pending_focus = Some((row, col));
        self.pending_cursor_char = None;
    }

    /// Clear focus from all cells.
    pub fn clear_focus(&mut self) {
        self.focused_cell = None;
        self.pending_focus = None;
        self.pending_cursor_char = None;
    }

    /// Move to the next cell (right, then down to next row).
    pub fn move_next(&mut self, num_rows: usize, num_cols: usize) {
        if let Some((row, col)) = self.focused_cell {
            self.pending_cursor_char = None;
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
            self.pending_cursor_char = None;
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
            self.pending_cursor_char = None;
            if row + 1 < num_rows {
                self.pending_focus = Some((row + 1, col));
            }
            // If at last row, stay there
        }
    }

    /// Move to the cell in the previous row (same column).
    pub fn move_up(&mut self) {
        if let Some((row, col)) = self.focused_cell {
            self.pending_cursor_char = None;
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
                        .map(|cell| TableCellData::new(serialize_inline_content(cell)))
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
    /// Source line number (for stable cross-widget focus).
    source_line: Option<usize>,
    /// Hard maximum width for the table (overrides available_width)
    max_width: Option<f32>,
    /// Editor font for styled text rendering (bold/italic variants)
    editor_font: Option<EditorFont>,
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
            source_line: None,
            max_width: None,
            editor_font: None,
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

    /// Set the markdown source line for this table (used for focus switching).
    #[must_use]
    pub fn source_line(mut self, line: usize) -> Self {
        self.source_line = Some(line);
        self
    }

    /// Set a hard maximum width for the table.
    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set the editor font for styled text rendering in non-editing cells.
    #[must_use]
    pub fn editor_font(mut self, font: EditorFont) -> Self {
        self.editor_font = Some(font);
        self
    }

    /// Show the table widget and return the output.
    pub fn show(self, ui: &mut Ui) -> WidgetOutput {
        use crate::markdown::parser::TableAlignment;

        let colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

        let table_id = self.id.unwrap_or_else(|| ui.id().with("editable_table"));
        let table_line = self.source_line.unwrap_or(0);

        // Get or create the table edit state
        let mut edit_state: TableEditState = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(table_id.with("edit_state"), TableEditState::new)
                .clone()
        });

        let mut global = load_table_global_focus(ui);

        // Track if we should signal a change to the source
        let mut changed = false;

        // External (session-driven) force-commit: e.g. user switched from this cell to a
        // heading; the session's commit callback wrote a one-shot flag for this table.
        // Honor it by treating the existing buffered edits as committable on this frame.
        let force_commit_requested = take_table_force_commit(ui, table_line);

        // Cross-table pending focus from a table rendered earlier this frame.
        if let Some((tid, row, col)) = global.pending_cell {
            if tid == table_id && edit_state.pending_focus.is_none() {
                edit_state.pending_focus = Some((row, col));
                edit_state.pending_cursor_char = None;
                global.pending_cell = None;
            }
        }

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

        let num_cols = self.data.num_columns.max(1);
        let min_col_width = 40.0_f32;
        let _char_width = self.font_size * 0.6;
        let cell_h_pad = 8.0_f32;
        let cell_v_pad = 6.0_f32;
        let line_height = self.font_size * 1.4;

        // Use the explicit max_width if provided, otherwise fall back to available
        let frame_h_margin = 4.0 * 2.0;
        let hard_width = self
            .max_width
            .unwrap_or_else(|| ui.available_width())
            .min(ui.available_width());
        let table_width = (hard_width - frame_h_margin).max(min_col_width);

        // Column widths: short columns keep their natural width,
        // remaining space is distributed among long columns.
        let col_widths: Vec<f32> = {
            // Measure each column's single-line natural width
            let natural: Vec<f32> = (0..num_cols)
                .map(|ci| {
                    let max_text_w = self
                        .data
                        .rows
                        .iter()
                        .filter_map(|r| r.get(ci))
                        .map(|c| {
                            let galley = ui.fonts_mut(|f| {
                                f.layout_no_wrap(
                                    c.text.clone(),
                                    FontId::proportional(self.font_size),
                                    egui::Color32::PLACEHOLDER,
                                )
                            });
                            galley.size().x
                        })
                        .fold(0.0_f32, f32::max);
                    (max_text_w + cell_h_pad * 2.0).max(min_col_width)
                })
                .collect();

            let total_natural: f32 = natural.iter().sum();

            if total_natural <= table_width {
                // Everything fits on one line — scale up proportionally
                let scale = table_width / total_natural;
                natural.iter().map(|&w| w * scale).collect()
            } else {
                // Some columns must wrap. Give short columns their natural
                // width; distribute the remainder among long columns.
                let fair_share = table_width / num_cols as f32;

                // "Short" = fits in fair_share or less
                let mut short_total = 0.0_f32;
                let mut long_natural_total = 0.0_f32;
                let mut is_short = vec![false; num_cols];

                for (ci, &nw) in natural.iter().enumerate() {
                    if nw <= fair_share {
                        is_short[ci] = true;
                        short_total += nw;
                    } else {
                        long_natural_total += nw;
                    }
                }

                let remaining = (table_width - short_total).max(0.0);

                natural
                    .iter()
                    .enumerate()
                    .map(|(ci, &nw)| {
                        if is_short[ci] {
                            nw
                        } else if long_natural_total > 0.0 {
                            (nw / long_natural_total * remaining).max(min_col_width)
                        } else {
                            remaining / num_cols as f32
                        }
                    })
                    .collect()
            }
        };

        // Apply user-resized column widths if available, scaled to current table_width
        let col_widths = if let Some(ref custom) = edit_state.custom_col_widths {
            if custom.len() == num_cols {
                let sum: f32 = custom.iter().sum();
                if sum > 0.0 {
                    custom.iter().map(|&w| w * table_width / sum).collect()
                } else {
                    col_widths
                }
            } else {
                col_widths
            }
        } else {
            col_widths
        };

        // Pre-measure row heights at the exact column widths
        let row_heights: Vec<f32> = (0..self.data.rows.len())
            .map(|ri| {
                let mut max_h = line_height;
                if let Some(row) = self.data.rows.get(ri) {
                    for ci in 0..num_cols {
                        if let Some(cell) = row.get(ci) {
                            let cw = col_widths.get(ci).copied().unwrap_or(100.0);
                            let wrap_w = (cw - cell_h_pad * 2.0).max(20.0);
                            let galley = ui.fonts_mut(|f| {
                                f.layout(
                                    cell.text.clone(),
                                    FontId::proportional(self.font_size),
                                    egui::Color32::PLACEHOLDER,
                                    wrap_w,
                                )
                            });
                            max_h = max_h.max(galley.size().y);
                        }
                    }
                }
                max_h + cell_v_pad * 2.0
            })
            .collect();

        let stripe_color = if is_dark {
            egui::Color32::from_rgb(35, 38, 46)
        } else {
            egui::Color32::from_rgb(245, 247, 250)
        };

        // Display-cell hit targets (filled during layout; used after for focus switching).
        let mut cell_click_targets: Vec<((usize, usize), egui::Rect)> = Vec::new();

        // Main table frame
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(egui::Margin::symmetric(4, 0))
            .corner_radius(6)
            .shadow(if is_dark {
                egui::epaint::Shadow::NONE
            } else {
                egui::epaint::Shadow {
                    offset: [0, 1],
                    blur: 3,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(8),
                }
            })
            .show(ui, |ui| {
                ui.set_min_width(table_width);
                ui.set_max_width(table_width);
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                let frame_left = ui.cursor().min.x;
                let mut table_top_y = 0.0_f32;
                let mut table_bottom_y = 0.0_f32;

                for row_idx in 0..self.data.rows.len() {
                    let is_header = row_idx == 0;
                    let row_h = row_heights
                        .get(row_idx)
                        .copied()
                        .unwrap_or(line_height + cell_v_pad * 2.0);

                    // Reserve a paint slot for the background (filled after we know actual height)
                    let bg_idx = ui.painter().add(egui::Shape::Noop);

                    let row_response = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        // Force the row to be exactly table_width wide
                        ui.set_min_width(table_width);
                        ui.set_max_width(table_width);

                        for col_idx in 0..num_cols {
                            let cw = col_widths
                                .get(col_idx)
                                .copied()
                                .unwrap_or(table_width / num_cols as f32);

                            let text_color = if is_header {
                                colors.heading
                            } else {
                                colors.text
                            };

                            let cell_size = egui::vec2(cw, row_h);
                            ui.allocate_ui_with_layout(
                                cell_size,
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    ui.set_min_width(cw);
                                    ui.set_max_width(cw);

                                    ui.add_space(cell_v_pad);
                                    ui.horizontal(|ui| {
                                        ui.add_space(cell_h_pad);
                                        let inner_w = (cw - cell_h_pad * 2.0).max(20.0);

                                        if let Some(row) = self.data.rows.get_mut(row_idx) {
                                            if let Some(cell) = row.get_mut(col_idx) {
                                                let cell_id = table_id
                                                    .with("cell")
                                                    .with(row_idx)
                                                    .with(col_idx);
                                                let font = FontId::proportional(self.font_size);

                                                let cell_has_focus =
                                                    ui.memory(|mem| mem.has_focus(cell_id));
                                                let wants_focus =
                                                    pending_focus == Some((row_idx, col_idx));

                                                if cell_has_focus || wants_focus {
                                                    // EDITING MODE: show raw TextEdit
                                                    // Consume Shift+Tab before plain Tab — egui
                                                    // treats Modifiers::NONE as matching Shift+Tab too
                                                    // (`matches_logically`).
                                                    let shift_tab_pressed = cell_has_focus
                                                        && ui.input_mut(|i| {
                                                            i.consume_key(
                                                                egui::Modifiers::SHIFT,
                                                                Key::Tab,
                                                            )
                                                        });
                                                    let tab_pressed = cell_has_focus
                                                        && ui.input_mut(|i| {
                                                            i.consume_key(
                                                                egui::Modifiers::NONE,
                                                                Key::Tab,
                                                            )
                                                        });
                                                    let enter_pressed = cell_has_focus
                                                        && ui.input_mut(|i| {
                                                            i.consume_key(
                                                                egui::Modifiers::NONE,
                                                                Key::Enter,
                                                            )
                                                        });

                                                    let wrap_font = font.clone();
                                                    let wrap_color = text_color;
                                                    let mut layouter =
                                                        move |ui_inner: &egui::Ui,
                                                              buf: &dyn egui::TextBuffer,
                                                              _wrap_width: f32| {
                                                            let text = buf.as_str();
                                                            let job =
                                                                egui::text::LayoutJob::simple(
                                                                    text.to_string(),
                                                                    wrap_font.clone(),
                                                                    wrap_color,
                                                                    inner_w,
                                                                );
                                                            ui_inner
                                                                .fonts_mut(|f| f.layout_job(job))
                                                        };

                                                    let mut output =
                                                        TextEdit::multiline(&mut cell.text)
                                                            .id(cell_id)
                                                            .font(font)
                                                            .text_color(text_color)
                                                            .frame(egui::Frame::NONE)
                                                            .desired_width(inner_w)
                                                            .desired_rows(1)
                                                            // Default TextEdit steals Tab focus to egui tab order;
                                                            // stay in-tab and handle Tab→next cell ourselves.
                                                            .lock_focus(true)
                                                            .layouter(&mut layouter)
                                                            .show(ui);

                                                    if cell.text.contains('\n') {
                                                        cell.text = cell.text.replace('\n', " ");
                                                        edit_state.content_modified = true;
                                                    }

                                                    let response = output.response;
                                                    if wants_focus {
                                                        response.request_focus();
                                                        if let Some(pos) =
                                                            edit_state.pending_cursor_char.take()
                                                        {
                                                            let ccursor =
                                                                egui::text::CCursor::new(pos);
                                                            output.state.cursor.set_char_range(
                                                                Some(
                                                                    egui::text::CCursorRange::one(
                                                                        ccursor,
                                                                    ),
                                                                ),
                                                            );
                                                            output
                                                                .state
                                                                .store(ui.ctx(), cell_id);
                                                        }
                                                    }
                                                    if response.has_focus() {
                                                        edit_state.focused_cell =
                                                            Some((row_idx, col_idx));
                                                        any_cell_has_focus = true;
                                                        let nr = self.data.rows.len();
                                                        let nc = self.data.num_columns;
                                                        if shift_tab_pressed {
                                                            edit_state.move_prev(nc);
                                                        } else if tab_pressed {
                                                            edit_state.move_next(nr, nc);
                                                        } else if enter_pressed {
                                                            edit_state.move_down(nr);
                                                        } else if ui
                                                            .input(|i| i.key_pressed(Key::Escape))
                                                        {
                                                            edit_state.clear_focus();
                                                            ui.memory_mut(|m| {
                                                                m.surrender_focus(cell_id)
                                                            });
                                                        }
                                                    }
                                                    if response.changed() {
                                                        edit_state.content_modified = true;
                                                    }
                                                } else {
                                                    // DISPLAY MODE: show rich text with inline formatting
                                                    let ef = self
                                                        .editor_font
                                                        .as_ref()
                                                        .cloned()
                                                        .unwrap_or(EditorFont::Inter);
                                                    let display_bold = is_header;
                                                    let job = if display_bold {
                                                        build_cell_layout_job_with_base_bold(
                                                            &cell.text,
                                                            self.font_size,
                                                            &ef,
                                                            text_color,
                                                            colors.code_bg,
                                                            inner_w,
                                                        )
                                                    } else {
                                                        build_inline_markdown_layout_job(
                                                            &cell.text,
                                                            self.font_size,
                                                            &ef,
                                                            text_color,
                                                            text_color,
                                                            colors.code_bg,
                                                            inner_w,
                                                        )
                                                    };
                                                    let galley =
                                                        ui.fonts_mut(|f| f.layout_job(job));
                                                    // Full inner rect: empty cells need a non-zero hit
                                                    // target (Label + empty galley was zero-sized).
                                                    let inner_h =
                                                        (row_h - cell_v_pad * 2.0).max(line_height);
                                                    let (_, response) = ui.allocate_exact_size(
                                                        egui::vec2(inner_w, inner_h),
                                                        egui::Sense::click(),
                                                    );
                                                    cell_click_targets
                                                        .push(((row_idx, col_idx), response.rect));
                                                    ui.painter().galley(
                                                        response.rect.min,
                                                        galley,
                                                        text_color,
                                                    );

                                                    // When a TextEdit already has focus, egui often
                                                    // uses the first click only to defocus it;
                                                    // `clicked()` may not fire on the target cell.
                                                    // `primary_pressed()` on hover catches that case.
                                                    let switching_from_other = edit_state
                                                        .focused_cell
                                                        .is_some_and(|(fr, fc)| {
                                                            fr != row_idx || fc != col_idx
                                                        })
                                                        || global
                                                            .active_table
                                                            .is_some_and(|t| t != table_id);
                                                    let activate_cell = response.clicked()
                                                        || response.double_clicked()
                                                        || (switching_from_other
                                                            && response.hovered()
                                                            && ui.input(|i| {
                                                                i.pointer.primary_pressed()
                                                            }));
                                                    if activate_cell {
                                                        let cursor_char = ui
                                                            .ctx()
                                                            .input(|i| {
                                                                i.pointer.interact_pos()
                                                            })
                                                            .map(|click_pos| {
                                                                table_cell_raw_cursor_at_click(
                                                                    ui,
                                                                    click_pos,
                                                                    response.rect,
                                                                    &cell.text,
                                                                    self.font_size,
                                                                    &ef,
                                                                    text_color,
                                                                    colors.code_bg,
                                                                    inner_w,
                                                                    display_bold,
                                                                )
                                                            });
                                                        request_table_cell_focus(
                                                            &mut edit_state,
                                                            &mut global,
                                                            ui,
                                                            table_id,
                                                            table_line,
                                                            row_idx,
                                                            col_idx,
                                                            cursor_char,
                                                        );
                                                    }
                                                    if response.hovered() {
                                                        ui.ctx().set_cursor_icon(
                                                            egui::CursorIcon::Text,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    });
                                },
                            );
                        }
                    });

                    // Track table vertical extent for resize handles
                    let actual_rect = row_response.response.rect;
                    if row_idx == 0 {
                        table_top_y = actual_rect.min.y;
                    }
                    table_bottom_y = actual_rect.max.y;

                    // Paint row background into reserved slot using actual rendered dimensions
                    let bg_rect = egui::Rect::from_min_size(
                        egui::pos2(frame_left, actual_rect.min.y),
                        egui::vec2(table_width, actual_rect.height()),
                    );
                    let bg_color = if is_header {
                        header_bg
                    } else if row_idx % 2 == 0 {
                        stripe_color
                    } else {
                        cell_bg
                    };
                    ui.painter()
                        .set(bg_idx, egui::Shape::rect_filled(bg_rect, 0.0, bg_color));
                    let row_y_min = actual_rect.min.y;
                    let row_y_max = actual_rect.max.y;

                    // Vertical column separators
                    {
                        let mut x = frame_left;
                        for ci in 0..num_cols - 1 {
                            x += col_widths.get(ci).copied().unwrap_or(0.0);
                            ui.painter().vline(
                                x,
                                row_y_min..=row_y_max,
                                egui::Stroke::new(0.5, border_color),
                            );
                        }
                    }

                    // Horizontal separator after header
                    if is_header {
                        ui.painter().hline(
                            frame_left..=frame_left + table_width,
                            row_y_max,
                            egui::Stroke::new(1.5, border_color),
                        );
                    }
                }

                // Fallback when defocus-only click did not set pending_focus above.
                let cross_table_edit =
                    global.active_table.is_some_and(|t| t != table_id);
                if edit_state.pending_focus.is_none()
                    && (edit_state.had_focus_last_frame || cross_table_edit)
                {
                    let (pointer_down, pointer_pos) = ui.input(|i| {
                        (
                            i.pointer.primary_pressed() || i.pointer.primary_clicked(),
                            i.pointer.interact_pos(),
                        )
                    });
                    if pointer_down {
                        if let Some(pos) = pointer_pos {
                            if let Some(((row, col), cell_rect)) = cell_click_targets
                                .iter()
                                .rev()
                                .find(|(_, rect)| rect.contains(pos))
                            {
                                let row = *row;
                                let col = *col;
                                if edit_state.focused_cell != Some((row, col)) {
                                    let ef = self
                                        .editor_font
                                        .as_ref()
                                        .cloned()
                                        .unwrap_or(EditorFont::Inter);
                                    let cw = col_widths
                                        .get(col)
                                        .copied()
                                        .unwrap_or(table_width / num_cols as f32);
                                    let inner_w = (cw - cell_h_pad * 2.0).max(20.0);
                                    let text_color = if row == 0 {
                                        colors.heading
                                    } else {
                                        colors.text
                                    };
                                    let cursor_char = self
                                        .data
                                        .rows
                                        .get(row)
                                        .and_then(|r| r.get(col))
                                        .map(|cell| {
                                            table_cell_raw_cursor_at_click(
                                                ui,
                                                pos,
                                                *cell_rect,
                                                &cell.text,
                                                self.font_size,
                                                &ef,
                                                text_color,
                                                colors.code_bg,
                                                inner_w,
                                                row == 0,
                                            )
                                        });
                                    request_table_cell_focus(
                                        &mut edit_state,
                                        &mut global,
                                        ui,
                                        table_id,
                                        table_line,
                                        row,
                                        col,
                                        cursor_char,
                                    );
                                }
                            }
                        }
                    }
                }

                // ── Column resize handles ──
                if num_cols > 1 {
                    let table_h = (table_bottom_y - table_top_y).max(1.0);
                    let handle_half_w = 3.0_f32;
                    let mut x = frame_left;
                    for ci in 0..num_cols - 1 {
                        x += col_widths.get(ci).copied().unwrap_or(0.0);
                        let handle_rect = egui::Rect::from_min_size(
                            egui::pos2(x - handle_half_w, table_top_y),
                            egui::vec2(handle_half_w * 2.0, table_h),
                        );
                        let handle_id = table_id.with("col_resize").with(ci);
                        let response =
                            ui.interact(handle_rect, handle_id, egui::Sense::click_and_drag());

                        if response.hovered() || response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }

                        if response.dragged() {
                            let delta = response.drag_delta().x;
                            let mut widths = col_widths.clone();
                            let new_left = (widths[ci] + delta).max(min_col_width);
                            let new_right = (widths[ci + 1] - delta).max(min_col_width);
                            if new_left >= min_col_width && new_right >= min_col_width {
                                widths[ci] = new_left;
                                widths[ci + 1] = new_right;
                                edit_state.custom_col_widths = Some(widths);
                            }
                        }

                        if response.double_clicked() {
                            edit_state.custom_col_widths = None;
                        }

                        if response.dragged() {
                            ui.painter().vline(
                                x + response.drag_delta().x,
                                table_top_y..=table_bottom_y,
                                egui::Stroke::new(1.5, border_color),
                            );
                        }
                    }
                }

                // ── Toolbar (add/remove rows, columns, alignment) ──
                if self.show_controls {
                    ui.add_space(2.0);

                    egui::Frame::new()
                        .fill(hover_bg)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .corner_radius(egui::CornerRadius {
                            nw: 0,
                            ne: 0,
                            sw: 6,
                            se: 6,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;

                                let add_row_btn = ui.add(
                                    egui::Button::new(
                                        RichText::new(t!("widgets.table.add_row").to_string())
                                            .size(self.font_size * 0.85)
                                            .color(control_color),
                                    )
                                    .frame(false),
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
                                if add_row_btn
                                    .on_hover_text(t!("widgets.table.add_row_tooltip").to_string())
                                    .clicked()
                                {
                                    action = Some(TableAction::AddRow);
                                }

                                let add_col_btn = ui.add(
                                    egui::Button::new(
                                        RichText::new(t!("widgets.table.add_column").to_string())
                                            .size(self.font_size * 0.85)
                                            .color(control_color),
                                    )
                                    .frame(false),
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
                                if add_col_btn
                                    .on_hover_text(
                                        t!("widgets.table.add_column_tooltip").to_string(),
                                    )
                                    .clicked()
                                {
                                    action = Some(TableAction::AddColumn);
                                }

                                if self.data.num_columns > 1 {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    ui.label(
                                        RichText::new(
                                            t!("widgets.table.delete_column_label").to_string(),
                                        )
                                        .size(self.font_size * 0.8)
                                        .color(control_color),
                                    );

                                    for col in 0..self.data.num_columns {
                                        let col_label = format!("{}", col + 1);
                                        let del_col_btn = ui.add(
                                            egui::Button::new(
                                                RichText::new(&col_label)
                                                    .size(self.font_size * 0.8)
                                                    .color(control_color),
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(16.0, 16.0)),
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
                                            .on_hover_text(
                                                t!(
                                                    "widgets.table.delete_column",
                                                    index = (col + 1).to_string()
                                                )
                                                .to_string(),
                                            )
                                            .clicked()
                                        {
                                            action = Some(TableAction::RemoveColumn(col));
                                        }
                                    }
                                }

                                if self.show_alignment_controls && self.data.num_columns > 0 {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    ui.label(
                                        RichText::new(t!("widgets.table.align_label").to_string())
                                            .size(self.font_size * 0.8)
                                            .color(control_color),
                                    );

                                    for col in 0..self.data.num_columns {
                                        let align = self
                                            .data
                                            .alignments
                                            .get(col)
                                            .copied()
                                            .unwrap_or(TableAlignment::None);

                                        let (align_icon, tooltip, use_phosphor) = match align {
                                            TableAlignment::Left => (
                                                TEXT_ALIGN_LEFT,
                                                t!("widgets.table.align_left").to_string(),
                                                true,
                                            ),
                                            TableAlignment::Center => (
                                                TEXT_ALIGN_CENTER,
                                                t!("widgets.table.align_center").to_string(),
                                                true,
                                            ),
                                            TableAlignment::Right => (
                                                TEXT_ALIGN_RIGHT,
                                                t!("widgets.table.align_right").to_string(),
                                                true,
                                            ),
                                            TableAlignment::None => (
                                                "—",
                                                t!("widgets.table.align_none").to_string(),
                                                false,
                                            ),
                                        };

                                        let align_label = if use_phosphor {
                                            phosphor_rich_text(align_icon, self.font_size * 0.8)
                                                .color(control_color)
                                        } else {
                                            RichText::new(align_icon)
                                                .size(self.font_size * 0.8)
                                                .color(control_color)
                                        };

                                        let align_btn =
                                            ui.add(egui::Button::new(align_label).frame(false));
                                        if align_btn
                                            .on_hover_text(format!("{} (click to cycle)", tooltip))
                                            .clicked()
                                        {
                                            action = Some(TableAction::CycleAlignment(col));
                                        }
                                    }
                                }
                            });
                        });
                }
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

        // Detect focus loss: had focus last frame but not this frame.
        // A click on another cell defocuses the TextEdit first; treat pointer-over-cell
        // as in-table navigation (do not commit) even if pending_focus was not set yet.
        let pointer_on_cell = ui.ctx().input(|i| {
            i.pointer.interact_pos().is_some_and(|pos| {
                i.pointer.any_pressed()
                    && cell_click_targets
                        .iter()
                        .any(|(_, rect)| rect.contains(pos))
            })
        });
        let focus_lost = edit_state.had_focus_last_frame
            && !any_cell_has_focus
            && edit_state.pending_focus.is_none()
            && !pointer_on_cell;

        if focus_lost && edit_state.content_modified && edit_state.pending_focus.is_none() {
            // Always defer: egui reports focus loss on mouse *release* (any_pressed is false),
            // and tables below us in the document have not run yet to set pending_cell.
            edit_state.defer_commit = true;
            edit_state.defer_commit_age = 0;
        }

        // RenderedEditSession asked us to flush now (user moved to a non-table block).
        // Bypass the focus-loss defer cycle so source updates on this frame.
        if force_commit_requested && edit_state.content_modified {
            changed = true;
            edit_state.content_modified = false;
            edit_state.defer_commit = false;
            edit_state.defer_commit_age = 0;
            if global.active_table == Some(table_id) {
                global.active_table = None;
                global.active_cell = None;
            }
            crate::diag::event(
                "table_force_commit",
                format!("table {:?} committed via session signal", table_id),
            );
        } else if force_commit_requested {
            // Signal arrived but no dirty edits to flush; clear defer bookkeeping anyway
            // so a previously-deferred commit does not linger as a phantom write.
            edit_state.defer_commit = false;
            edit_state.defer_commit_age = 0;
        }

        // Update focus tracking for next frame
        edit_state.had_focus_last_frame = any_cell_has_focus;

        if edit_state.defer_commit {
            edit_state.defer_commit_age = edit_state.defer_commit_age.saturating_add(1);
            if edit_state.defer_commit_age >= 2 {
                let commit_now = match global.pending_cell {
                    None => true,
                    Some((tid, _, _)) if tid != table_id => global.active_table == Some(tid),
                    Some((tid, _, _)) if tid == table_id => false,
                    _ => false,
                };
                if commit_now {
                    changed = true;
                    edit_state.content_modified = false;
                    edit_state.defer_commit = false;
                    edit_state.defer_commit_age = 0;
                    if global.active_table == Some(table_id) {
                        global.active_table = None;
                        global.active_cell = None;
                    }
                } else if edit_state.defer_commit_age >= 8 {
                    // Safety valve: pending_cell never resolved (e.g. focus race).
                    edit_state.defer_commit = false;
                    edit_state.defer_commit_age = 0;
                    edit_state.content_modified = false;
                    crate::diag::event(
                        "table_defer_aborted",
                        format!("table {:?} gave up waiting for pending_cell", table_id),
                    );
                }
            }
        }

        if any_cell_has_focus {
            global.active_table = Some(table_id);
            global.active_cell = edit_state.focused_cell;
        } else if edit_state.pending_focus.is_some() {
            global.active_table = Some(table_id);
            global.active_cell = edit_state.pending_focus;
        }

        // Check if any cell has focus (for output)
        let has_focus = any_cell_has_focus;

        // Effective interaction target — prefer next-frame intent (clicks/Tab navigation)
        // over current focus so callers (RenderedEditSession) can update active block
        // without a one-frame lag.
        //
        // `edit_state.focused_cell` is sticky inside the widget (only assigned when a
        // cell reports `response.has_focus()`, never cleared on focus loss). Reporting
        // it unconditionally would falsely re-activate the cell to the session the
        // frame AFTER the user clicked a heading/paragraph, ping-ponging focus.
        // Gate on actual current focus or in-flight focus intent.
        let focused_cell_out =
            if any_cell_has_focus || edit_state.pending_focus.is_some() {
                edit_state.pending_focus.or(edit_state.focused_cell)
            } else {
                None
            };

        let pending_focus_dbg = edit_state.pending_focus;
        let pending_cell_dbg = global.pending_cell;
        let defer_commit_dbg = edit_state.defer_commit;
        let defer_commit_age_dbg = edit_state.defer_commit_age;

        save_table_global_focus(ui, global);

        // Save the edit state back to memory
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(table_id.with("edit_state"), edit_state);
        });

        // Generate markdown output
        let markdown = self.data.to_markdown();

        // Only report as modified when explicitly set (focus lost with edits, or action performed)
        // Don't use markdown comparison - edits are buffered until focus leaves the table
        if changed {
            crate::diag::event(
                "table_commit",
                format!(
                    "table {:?} rows={} defer={} pending_focus={:?}",
                    table_id,
                    self.data.rows.len(),
                    defer_commit_dbg,
                    pending_focus_dbg
                ),
            );
        } else if defer_commit_dbg {
            crate::diag::event(
                "table_defer_commit",
                format!(
                    "table {:?} age={} pending_cell={:?}",
                    table_id, defer_commit_age_dbg, pending_cell_dbg
                ),
            );
        }

        if changed {
            WidgetOutput::modified(markdown)
                .with_focus(has_focus)
                .with_focused_cell(focused_cell_out)
        } else {
            WidgetOutput::unchanged(markdown)
                .with_focus(has_focus)
                .with_focused_cell(focused_cell_out)
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

fn truncate_run_output(s: &str, max_chars: usize) -> String {
    let t = s.trim_end();
    let count = t.chars().count();
    if count <= max_chars {
        return t.to_string();
    }
    t.chars().take(max_chars).collect::<String>() + "…"
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
    /// User requested inserting the captured run output as a fenced block.
    /// Carries the plain-text body for the new ```output block.
    pub insert_output_below: Option<String>,
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
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

        // Use the provided ID (required for uniqueness)
        let block_id = self.id.expect("EditableCodeBlock requires an explicit ID");

        let exec_ctx = ui
            .memory(|mem| {
                mem.data
                    .get_temp::<CodeExecutionUi>(code_exec_mod::code_execution_ctx_id())
            })
            .unwrap_or_else(CodeExecutionUi::disabled);

        // Per-block run handle (live state) and the toast-fallback flag for
        // when inline output is disabled in settings.
        let run_key = block_id.with("run_handle");
        let toast_emitted_key = block_id.with("run_toast_emitted");

        let run_handle: Option<RunHandle> =
            ui.memory(|mem| mem.data.get_temp::<RunHandle>(run_key));

        // Snapshot the live state once per frame for rendering.
        let run_snapshot: Option<RunSnapshot> = run_handle.as_ref().and_then(|h| {
            h.lock().ok().map(|s| RunSnapshot {
                status: s.status.clone(),
                stdout: s.stdout.clone(),
                stderr: s.stderr.clone(),
                elapsed: s.elapsed(),
                timeout_secs: s.timeout_secs,
                cancel_requested: s.cancel_requested(),
            })
        });

        // Toast fallback when inline output is disabled in settings.
        if !exec_ctx.show_inline_output {
            if let Some(ref snap) = run_snapshot {
                if !snap.status.is_running() {
                    let already = ui
                        .memory(|mem| mem.data.get_temp::<bool>(toast_emitted_key))
                        .unwrap_or(false);
                    if !already {
                        let msg = format_completion_toast(snap);
                        code_exec_mod::push_code_execution_toast(ui.ctx(), msg);
                        ui.memory_mut(|mem| mem.data.insert_temp(toast_emitted_key, true));
                    }
                }
            }
        }

        // Keep repainting while a run is in progress so streaming output and
        // elapsed time stay current.
        if run_snapshot.as_ref().is_some_and(|s| s.status.is_running()) {
            ui.ctx().request_repaint_after(Duration::from_millis(80));
        }

        let mut insert_output_below: Option<String> = None;
        let mut dismiss_run = false;
        let mut cancel_run = false;

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

        egui::Frame::new()
            .fill(code_block_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .corner_radius(6)
            .show(ui, |ui| {
                // Header row with language selector/label and buttons
                ui.horizontal(|ui| {
                    if self.data.is_editing {
                        // Language dropdown in edit mode - use unique ID
                        let current_display = language_display_name(&self.data.language);
                        egui::ComboBox::from_id_salt(block_id.with("lang"))
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

                        if code_exec_mod::run_button_visible(&exec_ctx, &self.data.language) {
                            let is_running =
                                run_snapshot.as_ref().is_some_and(|s| s.status.is_running());
                            let run_resp = ui
                                .horizontal(|ui| {
                                    ui.label(phosphor_rich_text(PLAY, 11.0));
                                    ui.add_enabled(
                                        !is_running,
                                        egui::Button::new(
                                            RichText::new(
                                                t!("widgets.code_block.run_label").to_string(),
                                            )
                                            .small(),
                                        )
                                        .small(),
                                    )
                                })
                                .inner
                                .on_hover_text(format!(
                                    "{} — {}",
                                    t!("widgets.code_block.run_label"),
                                    t!("widgets.code_block.run_tooltip")
                                ));
                            if run_resp.clicked() {
                                let timeout = Duration::from_secs(exec_ctx.timeout_secs as u64);
                                let ready = exec_ctx.consent_acknowledged && exec_ctx.enable;
                                if ready {
                                    let handle = code_exec_mod::spawn_run(
                                        self.data.code.clone(),
                                        self.data.language.clone(),
                                        exec_ctx.working_directory.clone(),
                                        timeout,
                                        ui.ctx().clone(),
                                    );
                                    ui.memory_mut(|mem| {
                                        mem.data.insert_temp(run_key, handle);
                                        mem.data.remove::<bool>(toast_emitted_key);
                                    });
                                } else {
                                    code_exec_mod::push_pending_code_execution_consent(
                                        ui.ctx(),
                                        crate::state::PendingCodeRun {
                                            code: self.data.code.clone(),
                                            language: self.data.language.clone(),
                                            cwd: exec_ctx.working_directory.clone(),
                                            timeout_secs: exec_ctx.timeout_secs,
                                            block_id,
                                        },
                                    );
                                }
                                ui.ctx().request_repaint();
                            }
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
                //
                // auto_shrink: x=false (always fill width to allow horizontal scrolling),
                // y=true (size to content height). Setting y=false here would make the
                // perpendicular axis claim the full available height (egui's
                // `inner_size.max(content_size)` rule) and stretch a single code block
                // over the entire viewport, hiding subsequent fenced blocks (issue #129).
                egui::ScrollArea::horizontal()
                    .id_salt(block_id.with("scroll"))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if self.data.is_editing {
                            // Edit mode: show plain text editor with unique ID
                            ui.add(
                                TextEdit::multiline(&mut self.data.code)
                                    .id(block_id.with("editor"))
                                    .code_editor()
                                    .font(FontId::monospace(self.font_size))
                                    .text_color(code_text_color)
                                    .frame(egui::Frame::NONE)
                                    .desired_width(f32::INFINITY),
                            );
                            // No auto-exit - user must click "Done" button
                        } else {
                            // View mode: show syntax-highlighted code
                            let highlighted_lines = highlight_code(
                                &self.data.code,
                                &self.data.language,
                                self.dark_mode,
                            );

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
                                            ui.spacing_mut().item_spacing.x = 0.0;
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

                // Inline output panel — rendered inside the same frame so
                // visual grouping ties the run to its source block.
                if exec_ctx.show_inline_output {
                    if let Some(snap) = run_snapshot.as_ref() {
                        let mut response = OutputPanelResponse::default();
                        render_run_output_panel(
                            ui,
                            block_id,
                            snap,
                            self.font_size,
                            self.dark_mode,
                            &colors,
                            &mut response,
                        );
                        if response.dismiss {
                            dismiss_run = true;
                        }
                        if response.stop {
                            cancel_run = true;
                        }
                        if let Some(body) = response.insert_output_below {
                            insert_output_below = Some(body);
                        }
                    }
                }
            });

        if cancel_run {
            if let Some(handle) = run_handle.as_ref() {
                code_exec_mod::cancel(handle);
                ui.ctx().request_repaint();
            }
        }

        if dismiss_run {
            ui.memory_mut(|mem| {
                mem.data.remove::<RunHandle>(run_key);
                mem.data.remove::<bool>(toast_emitted_key);
            });
        }

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
            insert_output_below,
        }
    }
}

/// Snapshot of a [`code_exec_mod::RunState`] taken once per frame for
/// rendering. Cloning a frozen view keeps the lock contention minimal.
struct RunSnapshot {
    status: RunStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    elapsed: Duration,
    /// Configured timeout, surfaced from `RunState.timeout_secs` so the panel
    /// can render `Timed out after Ns` without re-reading settings.
    timeout_secs: u32,
    /// True once Stop has been clicked, even if the worker has not yet
    /// observed the flag and transitioned to `RunStatus::Cancelled`.
    cancel_requested: bool,
}

#[derive(Default)]
struct OutputPanelResponse {
    dismiss: bool,
    /// User clicked the **Stop** button while the run was live.
    stop: bool,
    insert_output_below: Option<String>,
}

fn render_run_output_panel(
    ui: &mut Ui,
    block_id: egui::Id,
    snap: &RunSnapshot,
    font_size: f32,
    dark_mode: bool,
    widget_colors: &WidgetColors,
    response: &mut OutputPanelResponse,
) {
    let theme = if dark_mode {
        TerminalTheme::ferrite_dark()
    } else {
        TerminalTheme::ferrite_light()
    };
    let panel_bg = if dark_mode {
        egui::Color32::from_rgb(24, 28, 34)
    } else {
        egui::Color32::from_rgb(245, 247, 250)
    };
    let panel_border = if dark_mode {
        egui::Color32::from_rgb(55, 60, 68)
    } else {
        egui::Color32::from_rgb(205, 212, 220)
    };
    let muted = widget_colors.muted;

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    let (status_glyph, status_text, status_color) = run_status_label(snap);
    let is_running = snap.status.is_running();
    // Subtle spinner rotation tied to elapsed time gives a visible cue that
    // the UI thread is still ticking even when the child produces no output.

    ui.horizontal(|ui| {
        if is_running {
            ui.label(
                RichText::new(running_spinner_frame(snap.elapsed))
                    .color(status_color)
                    .strong()
                    .small(),
            );
        } else {
            ui.label(
                phosphor_rich_text(status_glyph, 12.0)
                    .color(status_color)
                    .strong()
                    .small(),
            );
        }
        ui.label(
            RichText::new(status_text)
                .color(status_color)
                .strong()
                .small(),
        );
        ui.label(
            RichText::new(format!("· {}", format_duration(snap.elapsed)))
                .color(muted)
                .small(),
        );
        if matches!(&snap.status, RunStatus::Failed { .. }) {
            if let RunStatus::Failed { message } = &snap.status {
                ui.label(
                    RichText::new(format!("· {}", truncate_run_output(message, 200)))
                        .color(muted)
                        .small(),
                );
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Stop sits in the same slot as Dismiss while live; Dismiss is
            // hidden during a run so the user only ever sees the actionable
            // button for the current state.
            if is_running {
                let stop_enabled = !snap.cancel_requested;
                let stop_resp = ui
                    .horizontal(|ui| {
                        ui.label(phosphor_rich_text(STOP, 11.0));
                        ui.add_enabled(
                            stop_enabled,
                            egui::Button::new(
                                RichText::new(t!("widgets.code_block.run_stop").to_string())
                                    .small(),
                            )
                            .small(),
                        )
                    })
                    .inner
                    .on_hover_text(t!("widgets.code_block.run_stop_tooltip").to_string());
                if stop_resp.clicked() {
                    response.stop = true;
                }
            } else if ui
                .add(egui::Button::new(t!("widgets.code_block.run_dismiss").to_string()).small())
                .on_hover_text(t!("widgets.code_block.run_dismiss_tooltip").to_string())
                .clicked()
            {
                response.dismiss = true;
            }
            let no_output = snap.stdout.is_empty() && snap.stderr.is_empty();
            if !no_output {
                if ui
                    .add(
                        egui::Button::new(t!("widgets.code_block.run_copy_output").to_string())
                            .small(),
                    )
                    .on_hover_text(t!("widgets.code_block.run_copy_output_tooltip").to_string())
                    .clicked()
                {
                    let combined = combine_streams_plain(&snap.stdout, &snap.stderr);
                    ui.ctx().copy_text(combined);
                }
                if ui
                    .add(
                        egui::Button::new(t!("widgets.code_block.run_insert_output").to_string())
                            .small(),
                    )
                    .on_hover_text(t!("widgets.code_block.run_insert_output_tooltip").to_string())
                    .clicked()
                {
                    response.insert_output_below =
                        Some(combine_streams_plain(&snap.stdout, &snap.stderr));
                }
            }
        });
    });

    egui::Frame::new()
        .fill(panel_bg)
        .stroke(egui::Stroke::new(1.0, panel_border))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .corner_radius(4)
        .show(ui, |ui| {
            let stdout_lines = ansi_render::parse(&snap.stdout);
            let stderr_lines = ansi_render::parse(&snap.stderr);
            let no_output = stdout_lines.is_empty() && stderr_lines.is_empty();

            if no_output && !snap.status.is_running() {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t!("widgets.code_block.run_no_output").to_string())
                            .color(muted)
                            .italics()
                            .font(FontId::monospace(font_size)),
                    );
                    if matches!(&snap.status, RunStatus::Completed { exit_code: Some(0) }) {
                        ui.label(
                            RichText::new(t!("widgets.code_block.run_no_output_hint").to_string())
                                .color(muted)
                                .small()
                                .italics(),
                        );
                    }
                });
                return;
            }

            egui::ScrollArea::vertical()
                .id_salt(block_id.with("run_output_scroll"))
                .max_height(220.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(block_id.with("run_output_hscroll"))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ansi_render::render_lines(
                                ui,
                                &stdout_lines,
                                font_size,
                                theme.foreground,
                                theme.background,
                                &theme.ansi_colors,
                            );

                            if !stderr_lines.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!(
                                        "── {} ──",
                                        t!("widgets.code_block.run_stderr_heading")
                                    ))
                                    .color(muted)
                                    .small()
                                    .italics(),
                                );
                                ansi_render::render_lines(
                                    ui,
                                    &stderr_lines,
                                    font_size,
                                    theme.foreground,
                                    theme.background,
                                    &theme.ansi_colors,
                                );
                            }
                        });
                });
        });
}

fn run_status_label(snap: &RunSnapshot) -> (&'static str, String, egui::Color32) {
    match &snap.status {
        RunStatus::Running => (
            ARROWS_CLOCKWISE,
            t!("widgets.code_block.run_status_running").to_string(),
            egui::Color32::from_rgb(120, 170, 255),
        ),
        RunStatus::Completed { exit_code: Some(0) } => (
            CHECK,
            t!("widgets.code_block.run_status_success").to_string(),
            egui::Color32::from_rgb(120, 200, 130),
        ),
        RunStatus::Completed {
            exit_code: Some(code),
        } => (
            X,
            t!(
                "widgets.code_block.run_status_failure",
                code = code.to_string()
            )
            .to_string(),
            egui::Color32::from_rgb(230, 100, 100),
        ),
        RunStatus::Completed { exit_code: None } => (
            X,
            t!("widgets.code_block.run_status_failure_unknown").to_string(),
            egui::Color32::from_rgb(230, 100, 100),
        ),
        RunStatus::TimedOut => (
            X,
            t!(
                "widgets.code_block.run_status_timed_out",
                secs = snap.timeout_secs.to_string()
            )
            .to_string(),
            egui::Color32::from_rgb(230, 150, 70),
        ),
        RunStatus::Cancelled => (
            X,
            t!("widgets.code_block.run_status_cancelled").to_string(),
            egui::Color32::from_rgb(190, 190, 190),
        ),
        RunStatus::Failed { .. } => (
            X,
            t!("widgets.code_block.run_status_failed").to_string(),
            egui::Color32::from_rgb(230, 100, 100),
        ),
    }
}

/// Pick a Braille spinner frame from the run's elapsed time so the running
/// indicator visibly rotates without storing animation state in egui memory.
fn running_spinner_frame(elapsed: Duration) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let tick = (elapsed.as_millis() / 80) as usize;
    FRAMES[tick % FRAMES.len()]
}

fn format_duration(d: Duration) -> String {
    let total_ms = d.as_millis();
    if total_ms < 1000 {
        format!("{total_ms}ms")
    } else if total_ms < 60_000 {
        format!("{:.2}s", d.as_secs_f32())
    } else {
        let secs = d.as_secs();
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn combine_streams_plain(stdout: &[u8], stderr: &[u8]) -> String {
    let mut out = String::new();
    if !stdout.is_empty() {
        out.push_str(&strip_ansi(&String::from_utf8_lossy(stdout)));
    }
    if !stderr.is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&strip_ansi(&String::from_utf8_lossy(stderr)));
    }
    out
}

/// Best-effort ANSI escape stripping for clipboard / fence insertion.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next();
                    while let Some(&cc) = chars.peek() {
                        chars.next();
                        if cc.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    continue;
                } else if next == ']' {
                    chars.next();
                    while let Some(&cc) = chars.peek() {
                        chars.next();
                        if cc == '\x07' {
                            break;
                        }
                    }
                    continue;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn format_completion_toast(snap: &RunSnapshot) -> String {
    let plain = combine_streams_plain(&snap.stdout, &snap.stderr);
    match &snap.status {
        RunStatus::Completed { exit_code: Some(0) } => t!(
            "widgets.code_block.run_finished",
            output = truncate_run_output(&plain, 2000)
        )
        .to_string(),
        RunStatus::Completed { exit_code } => {
            let code = exit_code.map_or_else(|| "?".to_string(), |c| c.to_string());
            t!(
                "widgets.code_block.run_failed",
                error = format!("Exited with code {code}.\n{plain}")
            )
            .to_string()
        }
        RunStatus::TimedOut => t!(
            "widgets.code_block.run_failed",
            error = t!(
                "widgets.code_block.run_status_timed_out",
                secs = snap.timeout_secs.to_string()
            )
            .to_string()
        )
        .to_string(),
        RunStatus::Cancelled => t!(
            "widgets.code_block.run_failed",
            error = t!("widgets.code_block.run_status_cancelled").to_string()
        )
        .to_string(),
        RunStatus::Failed { message } => {
            t!("widgets.code_block.run_failed", error = message).to_string()
        }
        RunStatus::Running => String::new(),
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
    /// Whether this link consumed a click event (prevents parent from entering edit mode)
    pub click_consumed: bool,
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
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

        let link_id = self.id.expect("RenderedLinkWidget requires an explicit ID");

        // Track if we committed changes this frame
        let mut committed_changes = false;

        // Link color - use heading color as link color (blue-ish)
        let link_color = colors.heading;

        // Get dark mode for popup styling
        let is_dark = colors.text.r() > 128;

        // Render the link text with underline styling - clickable for interaction
        let link_response = ui.add(
            egui::Label::new(
                RichText::new(&self.state.edit_text)
                    .color(link_color)
                    .font(FontId::proportional(self.font_size))
                    .underline(),
            )
            .sense(egui::Sense::click()),
        );

        // Store rect before consuming response
        let link_rect = link_response.rect;

        // Show hand cursor on hover to indicate clickable link
        if link_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Get pointer state for detecting Ctrl+Click
        // We need to check for primary button release while hovering and with modifiers
        let (primary_released, modifiers, pointer_pos) = ui.input(|i| {
            (
                i.pointer.primary_released(),
                i.modifiers,
                i.pointer.interact_pos(),
            )
        });

        // Check if pointer is over this link when released
        let clicked_on_link =
            primary_released && pointer_pos.map_or(false, |pos| link_rect.contains(pos));

        // Track whether we consumed a click (to prevent parent from entering edit mode)
        let mut click_consumed = false;

        // Handle click interactions
        // Check for middle-click first (always opens in browser)
        if link_response.middle_clicked() {
            click_consumed = true;
            let can_open = self.state.edit_url.starts_with("http://")
                || self.state.edit_url.starts_with("https://");
            if can_open {
                if let Err(e) = open::that(&self.state.edit_url) {
                    log::error!("Failed to open URL: {}", e);
                } else {
                    log::debug!("Opened URL via middle-click: {}", self.state.edit_url);
                }
            }
        } else if clicked_on_link {
            click_consumed = true;
            // Check if Ctrl/Cmd was held during the click
            let open_in_browser = modifiers.ctrl || modifiers.command;

            if open_in_browser {
                // Ctrl+Click / Cmd+Click: Open URL in default browser
                let can_open = self.state.edit_url.starts_with("http://")
                    || self.state.edit_url.starts_with("https://");
                if can_open {
                    if let Err(e) = open::that(&self.state.edit_url) {
                        log::error!("Failed to open URL: {}", e);
                    } else {
                        log::debug!("Opened URL via Ctrl+Click: {}", self.state.edit_url);
                    }
                }
            } else {
                // Regular click: Open edit popup
                self.state.popup_open = !self.state.popup_open;
            }
        }

        // Show tooltip with URL and interaction hint when hovering (if popup not open)
        if link_response.hovered() && !self.state.popup_open {
            let can_open = self.state.edit_url.starts_with("http://")
                || self.state.edit_url.starts_with("https://");
            let tooltip = if can_open {
                format!(
                    "{}\n\nClick to edit • Ctrl+Click to open in browser",
                    self.state.edit_url
                )
            } else {
                format!("{}\n\nClick to edit", self.state.edit_url)
            };
            link_response.on_hover_text(tooltip);
        }

        // Show link edit popup (egui 0.34 Popup API; local bool avoids borrow conflict with edit fields).
        let mut popup_open = self.state.popup_open;
        let was_popup_open = popup_open;
        if popup_open {
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

            egui::Popup::new(popup_id, ui.ctx().clone(), link_rect, ui.layer_id())
                .open_bool(&mut popup_open)
                .align(egui::emath::RectAlign::TOP_START)
                .gap(4.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    egui::Frame::new()
                        .fill(popup_bg)
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .inner_margin(egui::Margin::same(12))
                        .corner_radius(6)
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 2],
                            blur: 8,
                            spread: 0,
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

                                let open_button = ui.add_enabled(
                                    can_open,
                                    egui::Button::new(t!("widgets.link.open").to_string()),
                                );

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

            self.state.popup_open = popup_open;
            // Commit edits when the popup closes (click outside or Escape).
            if was_popup_open && !popup_open && self.state.is_modified() {
                self.state.commit();
                committed_changes = true;
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
            click_consumed,
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

    /// Phosphor icon glyph for the diagram type.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Flowchart => CHART_BAR,
            Self::Sequence => ARROWS_LEFT_RIGHT,
            Self::Class => DIAMOND,
            Self::State => ARROWS_CLOCKWISE,
            Self::EntityRelationship => LINK,
            Self::UserJourney => USER,
            Self::Gantt => CALENDAR,
            Self::Pie => CHART_PIE,
            Self::Quadrant => SQUARES_FOUR,
            Self::Requirement => LIST_CHECKS,
            Self::GitGraph => GIT_BRANCH,
            Self::C4 => BUILDINGS,
            Self::Mindmap => TREE_STRUCTURE,
            Self::Timeline => HOURGLASS,
            Self::ZenUML => PACKAGE,
            Self::Sankey => FLOW_ARROW,
            Self::XYChart => CHART_LINE_UP,
            Self::Block => SQUARES_FOUR,
            Self::Unknown => CHART_BAR,
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
    } else if first_line_lower.starts_with("classdiagram") || first_line_lower.starts_with("class")
    {
        MermaidDiagramType::Class
    } else if first_line_lower.starts_with("statediagram") || first_line_lower.starts_with("state")
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
    /// Last source that successfully validated/rendered. Used to keep the
    /// previous diagram visible while the user fixes a small typo, instead of
    /// blanking the diagram on every transient parse failure.
    pub last_good_source: Option<String>,
    /// Most recent structured validation error (when current source fails to
    /// parse). Preserved across frames so the warning header keeps showing.
    pub last_error: Option<crate::markdown::mermaid::MermaidError>,
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
            last_good_source: None,
            last_error: None,
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
        use crate::markdown::mermaid::{
            render_mermaid_diagram, validate_mermaid_source, RenderResult,
        };

        let _colors = self
            .colors
            .unwrap_or_else(|| WidgetColors::resolved(ui, Theme::System));

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
        let frame = egui::Frame::new()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.5, border_color))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::same(0));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                // Header with diagram type indicator
                let header_frame = egui::Frame::new()
                    .fill(header_bg)
                    .corner_radius(egui::CornerRadius {
                        nw: 6,
                        ne: 6,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin::symmetric(12, 8));

                header_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Diagram type icon and name
                        ui.label(
                            phosphor_rich_text(self.data.diagram_type.icon(), self.font_size + 2.0),
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
                            let source_toggle = ui.horizontal(|ui| {
                                ui.label(
                                    phosphor_rich_text(
                                        if self.data.show_source {
                                            CARET_DOWN
                                        } else {
                                            CARET_RIGHT
                                        },
                                        self.font_size - 2.0,
                                    )
                                    .color(text_color),
                                );
                                ui.add(
                                    egui::Button::new(
                                        RichText::new("Source")
                                            .color(text_color)
                                            .size(self.font_size - 2.0),
                                    )
                                    .frame(false),
                                )
                            });
                            if source_toggle.inner.clicked() {
                                self.data.show_source = !self.data.show_source;
                            }
                        });
                    });
                });

                // Content area - show rendered diagram or source
                // Wrap in horizontal scroll area to handle wide diagrams without
                // breaking max_line_width for subsequent content.
                // See: ROADMAP.md "Blockquote/code block overflow"
                let content_frame = egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 8));

                content_frame.show(ui, |ui| {
                    // See issue #129: auto_shrink y must be true so the inner
                    // horizontal scroll area sizes to its content height instead
                    // of consuming all remaining vertical space and pushing
                    // subsequent fenced/mermaid blocks below the viewport.
                    egui::ScrollArea::horizontal()
                        .id_salt(block_id.with("scroll"))
                        .auto_shrink([false, true])
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
                                self.data.last_error = None;
                            } else {
                                // Parse-only validate first so we know whether
                                // to show the live source or fall back to the
                                // last successfully rendered source.
                                match validate_mermaid_source(&self.data.source) {
                                    Ok(()) => {
                                        let result = render_mermaid_diagram(
                                            ui,
                                            &self.data.source,
                                            self.dark_mode,
                                            self.font_size,
                                        );
                                        match result {
                                            RenderResult::Success => {
                                                self.data.last_good_source =
                                                    Some(self.data.source.clone());
                                                self.data.last_error = None;
                                            }
                                            RenderResult::ParseError(msg) => {
                                                // Render-time failure (e.g. layout panic).
                                                let err = crate::markdown::mermaid::MermaidError::from_message(
                                                    &self.data.source,
                                                    msg,
                                                );
                                                self.data.last_error = Some(err.clone());
                                                show_validation_warning(
                                                    ui,
                                                    &err,
                                                    self.font_size,
                                                    self.dark_mode,
                                                );
                                                ui.add_space(8.0);
                                                show_source_code(
                                                    ui,
                                                    block_id,
                                                    &self.data.source,
                                                    self.font_size,
                                                    self.dark_mode,
                                                    muted_color,
                                                );
                                            }
                                            RenderResult::Unsupported(msg) => {
                                                self.data.last_error = None;
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
                                                show_source_code(
                                                    ui,
                                                    block_id,
                                                    &self.data.source,
                                                    self.font_size,
                                                    self.dark_mode,
                                                    muted_color,
                                                );
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        self.data.last_error = Some(err.clone());
                                        show_validation_warning(
                                            ui,
                                            &err,
                                            self.font_size,
                                            self.dark_mode,
                                        );
                                        ui.add_space(6.0);

                                        if let Some(good) = self.data.last_good_source.clone() {
                                            // Fall back to the last successful
                                            // diagram so a transient typo
                                            // doesn't blank the preview.
                                            let _ = render_mermaid_diagram(
                                                ui,
                                                &good,
                                                self.dark_mode,
                                                self.font_size,
                                            );
                                        } else {
                                            // No good render to fall back to
                                            // — show the source so the user
                                            // can fix the issue.
                                            show_source_code(
                                                ui,
                                                block_id,
                                                &self.data.source,
                                                self.font_size,
                                                self.dark_mode,
                                                muted_color,
                                            );
                                        }
                                    }
                                }
                            }
                        });
                });

                // Render error display (if any stored in data)
                if let Some(error) = &self.data.render_error {
                    let error_frame = egui::Frame::new()
                        .fill(if self.dark_mode {
                            egui::Color32::from_rgb(60, 30, 30)
                        } else {
                            egui::Color32::from_rgb(255, 240, 240)
                        })
                        .inner_margin(egui::Margin::symmetric(12, 8));

                    error_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                phosphor_rich_text(WARNING, 14.0)
                                    .color(egui::Color32::from_rgb(220, 80, 80)),
                            );
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
        .id_salt(block_id.with("scroll"))
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

/// Render the warning header for a Mermaid validation failure. Shows the
/// offending line number, the parser message and an optional hint, all in a
/// soft amber/red banner. When `last_good_source` is preserved on the data,
/// this header is drawn *above* the previous good render so the user has
/// continuous visual context while they fix the typo.
fn show_validation_warning(
    ui: &mut Ui,
    error: &crate::markdown::mermaid::MermaidError,
    font_size: f32,
    dark_mode: bool,
) {
    let bg = if dark_mode {
        egui::Color32::from_rgb(60, 45, 25)
    } else {
        egui::Color32::from_rgb(255, 246, 224)
    };
    let border = if dark_mode {
        egui::Color32::from_rgb(180, 130, 40)
    } else {
        egui::Color32::from_rgb(220, 170, 70)
    };
    let text = if dark_mode {
        egui::Color32::from_rgb(255, 215, 140)
    } else {
        egui::Color32::from_rgb(120, 80, 10)
    };

    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(phosphor_rich_text(WARNING, font_size + 1.0).color(text));
                    ui.label(
                        RichText::new(t!("mermaid.warning_line", line = error.line).to_string())
                            .color(text)
                            .strong()
                            .size(font_size - 1.0),
                    );
                    ui.label(
                        RichText::new(&error.message)
                            .color(text)
                            .size(font_size - 1.0),
                    );
                });
                if let Some(hint) = &error.hint {
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(font_size + 6.0);
                        ui.label(
                            RichText::new(format!("💡 {}", hint))
                                .color(text)
                                .italics()
                                .size(font_size - 2.0),
                        );
                    });
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

    egui::Frame::new()
        .fill(error_bg)
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(WARNING, 14.0).color(error_text));
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

    #[test]
    fn test_widget_output_with_focused_cell() {
        let none = WidgetOutput::unchanged(String::new());
        assert_eq!(none.focused_cell, None);

        let some = WidgetOutput::modified(String::new()).with_focused_cell(Some((2, 3)));
        assert_eq!(some.focused_cell, Some((2, 3)));
    }

    #[test]
    fn test_table_force_commit_signal_roundtrip() {
        let ctx = egui::Context::default();
        signal_table_force_commit(&ctx, 5);
        // Stored via ctx.data_mut, readable via ctx.data
        let stored: bool = ctx
            .data(|d| d.get_temp::<bool>(table_force_commit_id(5)))
            .unwrap_or(false);
        assert!(stored, "signal should be persisted in egui temp data");

        // Different table_line — independent slot
        let other: bool = ctx
            .data(|d| d.get_temp::<bool>(table_force_commit_id(6)))
            .unwrap_or(false);
        assert!(!other);
    }

    #[test]
    fn test_table_force_commit_take_is_one_shot() {
        let ctx = egui::Context::default();
        signal_table_force_commit(&ctx, 9);

        ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(
                    take_table_force_commit(ui, 9),
                    "first take should observe the signal"
                );
                assert!(
                    !take_table_force_commit(ui, 9),
                    "second take should be cleared"
                );
            });
        });
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
        let dark = WidgetColors::from_theme(
            Theme::Dark,
            &egui::Visuals::dark(),
            crate::theme::accent::default_accent(),
        );
        let light = WidgetColors::from_theme(
            Theme::Light,
            &egui::Visuals::light(),
            crate::theme::accent::default_accent(),
        );

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
            insert_output_below: None,
        };
        assert!(output.changed);
        assert!(output.language_changed);
        assert_eq!(output.code, "code");
        assert_eq!(output.language, "rust");
        assert!(output.insert_output_below.is_none());
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
            click_consumed: false,
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
            click_consumed: false,
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
        assert_eq!(detect_mermaid_diagram_type(""), MermaidDiagramType::Unknown);
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
        assert_eq!(
            data.to_markdown(),
            "```mermaid\nflowchart TD\n  A --> B\n```"
        );
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
        assert_eq!(
            MermaidDiagramType::Sequence.display_name(),
            "Sequence Diagram"
        );
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
