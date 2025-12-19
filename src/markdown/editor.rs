//! WYSIWYG Markdown Editor Widget
//!
//! This module provides a WYSIWYG (What You See Is What You Get) markdown editor
//! that renders markdown as editable egui widgets, allowing users to edit content
//! directly in rendered view.
//!
//! # Features
//! - Parses markdown into AST using the parser from Task 19
//! - Renders each AST node as an editable egui widget
//! - Propagates edits back to markdown source
//! - Supports toggling between raw and rendered modes
//! - Theme-aware styling
//! - Word processor-like keyboard interactions (Enter, Backspace, Tab, Shift+Tab)
//!
//! # Keyboard Interactions (WYSIWYG Mode)
//! - **Enter in Paragraph**: Splits the paragraph at cursor into two paragraphs
//! - **Enter in List Item**: Splits the list item, inserting a new item after
//! - **Enter on Empty List Item**: Exits the list, creates a paragraph after
//! - **Enter in Heading**: Creates a new paragraph below the heading
//! - **Backspace at List Item Start**: Merges with previous item or converts to paragraph
//! - **Tab in List Item**: Indents to create nested list
//! - **Shift+Tab in Nested List**: Outdents to parent level
//!
//! # Example
//! ```ignore
//! let output = MarkdownEditor::new(&mut content)
//!     .with_settings(&settings)
//!     .show(ui);
//!
//! if output.changed {
//!     // Content was modified
//! }
//! ```

// Allow dead code and unused imports - this module has builder pattern methods and output fields for future extensibility
// The ast_ops imports are for planned WYSIWYG keyboard interactions (Enter, Backspace, Tab behavior)
// - too_many_arguments: Rendering functions need many parameters for proper configuration
// - only_used_in_recursion: Recursive rendering functions pass context through
// - ptr_arg: Using &mut String for direct source modification
// - needless_range_loop: Index loops are clearer for line-by-line source manipulation
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_range_loop)]

use crate::config::{EditorFont, Settings, Theme};
use crate::fonts;
use crate::markdown::ast_ops::{
    exit_list_to_paragraph, heading_enter, indent_list_item, merge_with_previous_list_item,
    outdent_list_item, split_list_item, split_paragraph, EditContext, EditNodeType, StructuralEdit,
};
use crate::markdown::parser::{
    parse_markdown, HeadingLevel, ListType, MarkdownNode, MarkdownNodeType,
};
use crate::markdown::widgets::{
    CodeBlockData, EditableCodeBlock, EditableTable, RenderedLinkState, RenderedLinkWidget,
    TableData, WidgetColors,
};
use eframe::egui::{
    self, Color32, FontId, Key, Response, RichText, ScrollArea, TextEdit, Ui, Vec2,
};
use log::debug;

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Editor Mode
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The editing mode for the markdown editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Raw markdown text editing mode
    #[default]
    Raw,
    /// WYSIWYG rendered editing mode
    Rendered,
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Editor Output
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Result of showing the markdown editor widget.
pub struct MarkdownEditorOutput {
    /// The egui response from the editor container.
    pub response: Response,
    /// Whether the content was modified.
    pub changed: bool,
    /// Current cursor position (line, column) - 0-indexed.
    pub cursor_position: (usize, usize),
    /// Current editing mode.
    pub mode: EditorMode,
    /// Focused element info for rendered mode (character range in source)
    pub focused_element: Option<FocusedElement>,
    /// Current scroll offset (for sync scrolling)
    pub scroll_offset: f32,
}

/// Information about the currently focused element in rendered mode.
#[derive(Debug, Clone)]
pub struct FocusedElement {
    /// Start character index in source markdown
    pub start_char: usize,
    /// End character index in source markdown
    pub end_char: usize,
    /// Selection within the element (relative to element start)
    pub selection: Option<(usize, usize)>,
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Theme Colors
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Theme-aware colors for the WYSIWYG editor.
#[derive(Debug, Clone)]
pub struct EditorColors {
    /// Background color
    pub background: Color32,
    /// Primary text color
    pub text: Color32,
    /// Heading text color
    pub heading: Color32,
    /// Code background color
    pub code_bg: Color32,
    /// Code text color
    pub code_text: Color32,
    /// Block quote border color
    pub quote_border: Color32,
    /// Block quote text color
    pub quote_text: Color32,
    /// Link color
    pub link: Color32,
    /// Horizontal rule color
    pub hr: Color32,
    /// List bullet/number color
    pub list_marker: Color32,
    /// Task list checkbox color
    pub checkbox: Color32,
}

impl EditorColors {
    /// Create colors for the given theme.
    pub fn from_theme(theme: Theme, visuals: &egui::Visuals) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::System => {
                if visuals.dark_mode {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }

    /// Dark theme colors.
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(30, 30, 30),
            text: Color32::from_rgb(220, 220, 220),
            heading: Color32::from_rgb(100, 180, 255),
            code_bg: Color32::from_rgb(45, 45, 45),
            code_text: Color32::from_rgb(200, 200, 150),
            quote_border: Color32::from_rgb(80, 80, 80),
            quote_text: Color32::from_rgb(180, 180, 180),
            link: Color32::from_rgb(100, 180, 255),
            hr: Color32::from_rgb(80, 80, 80),
            list_marker: Color32::from_rgb(150, 150, 150),
            checkbox: Color32::from_rgb(100, 180, 255),
        }
    }

    /// Light theme colors.
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(255, 255, 255),
            text: Color32::from_rgb(30, 30, 30),
            heading: Color32::from_rgb(0, 100, 180),
            code_bg: Color32::from_rgb(245, 245, 245),
            code_text: Color32::from_rgb(80, 80, 80),
            quote_border: Color32::from_rgb(200, 200, 200),
            quote_text: Color32::from_rgb(100, 100, 100),
            link: Color32::from_rgb(0, 100, 180),
            hr: Color32::from_rgb(200, 200, 200),
            list_marker: Color32::from_rgb(100, 100, 100),
            checkbox: Color32::from_rgb(0, 100, 180),
        }
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Editable Node State
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// State for an editable node in the WYSIWYG editor.
/// This tracks the text content and modification status of each editable element.
#[derive(Debug, Clone)]
struct EditableNode {
    /// Unique ID for this node
    id: usize,
    /// The text content being edited
    text: String,
    /// Start line in source (for mapping back)
    start_line: usize,
    /// End line in source (for mapping back)
    end_line: usize,
    /// Whether this node was modified
    modified: bool,
}

/// Tracks all editable nodes and their states.
#[derive(Debug, Clone, Default)]
struct EditState {
    /// All editable nodes indexed by their ID
    nodes: Vec<EditableNode>,
    /// Counter for generating unique node IDs
    next_id: usize,
    /// Currently focused node ID
    focused_node: Option<usize>,
    /// Selection within the focused node (start, end) - relative to node text
    focused_selection: Option<(usize, usize)>,
}

impl EditState {
    fn new() -> Self {
        Self::default()
    }

    fn add_node(&mut self, text: String, start_line: usize, end_line: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(EditableNode {
            id,
            text,
            start_line,
            end_line,
            modified: false,
        });
        id
    }

    fn get_node_mut(&mut self, id: usize) -> Option<&mut EditableNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    fn any_modified(&self) -> bool {
        self.nodes.iter().any(|n| n.modified)
    }

    fn clear(&mut self) {
        self.nodes.clear();
        self.next_id = 0;
        self.focused_node = None;
        self.focused_selection = None;
    }

    /// Set the currently focused node and selection within it
    fn set_focus(&mut self, node_id: usize, selection: Option<(usize, usize)>) {
        self.focused_node = Some(node_id);
        self.focused_selection = selection;
    }

    /// Get focused element info for the output
    fn get_focused_element(&self, source: &str) -> Option<FocusedElement> {
        let node_id = self.focused_node?;
        let node = self.nodes.iter().find(|n| n.id == node_id)?;

        // Convert line numbers to character indices
        let start_char = line_to_char_index(source, node.start_line);
        let end_char = line_to_char_index(source, node.end_line + 1).min(source.len());

        Some(FocusedElement {
            start_char,
            end_char,
            selection: self.focused_selection,
        })
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Structural Edit State
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Tracks the context of the currently focused editable widget for structural operations.
/// This enables word processor-like keyboard behavior (Enter, Backspace, Tab).
#[derive(Debug, Clone, Default)]
struct StructuralEditState {
    /// Pending structural edit to apply at end of frame
    pending_edit: Option<StructuralEdit>,
    /// Current edit context (populated when a widget is focused)
    current_context: Option<EditContext>,
}

impl StructuralEditState {
    fn new() -> Self {
        Self::default()
    }

    /// Set the current edit context (called when a widget gains focus or is edited)
    fn set_context(&mut self, ctx: EditContext) {
        self.current_context = Some(ctx);
    }

    /// Clear the current context
    fn clear_context(&mut self) {
        self.current_context = None;
    }

    /// Set a pending structural edit to apply
    fn set_pending_edit(&mut self, edit: StructuralEdit) {
        if edit.performed {
            self.pending_edit = Some(edit);
        }
    }

    /// Take the pending edit (returns and clears it)
    fn take_pending_edit(&mut self) -> Option<StructuralEdit> {
        self.pending_edit.take()
    }
}

/// Result of checking for structural key presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralKeyAction {
    /// No structural key was pressed
    None,
    /// Enter key pressed
    Enter,
    /// Backspace at position 0
    BackspaceAtStart,
    /// Tab key pressed
    Tab,
    /// Shift+Tab pressed
    ShiftTab,
}

/// Check if a structural key was pressed given the input state.
fn check_structural_keys(ui: &Ui, cursor_at_start: bool) -> StructuralKeyAction {
    ui.input(|i| {
        // Check Enter (without modifiers to avoid conflicts with Shift+Enter for line break)
        if i.key_pressed(Key::Enter) && !i.modifiers.shift && !i.modifiers.ctrl && !i.modifiers.alt
        {
            return StructuralKeyAction::Enter;
        }

        // Check Backspace at start of text
        if i.key_pressed(Key::Backspace) && cursor_at_start {
            return StructuralKeyAction::BackspaceAtStart;
        }

        // Check Tab (without Shift)
        if i.key_pressed(Key::Tab) && !i.modifiers.shift {
            return StructuralKeyAction::Tab;
        }

        // Check Shift+Tab
        if i.key_pressed(Key::Tab) && i.modifiers.shift {
            return StructuralKeyAction::ShiftTab;
        }

        StructuralKeyAction::None
    })
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Markdown Editor Widget
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// A WYSIWYG markdown editor widget.
///
/// This widget provides two editing modes:
/// - **Raw mode**: Plain text editing of markdown source
/// - **Rendered mode**: Edit content through styled, semantic egui widgets
///
/// In rendered mode, each markdown element (headings, paragraphs, lists, etc.)
/// is rendered as an editable widget. Edits are synchronized back to the
/// underlying markdown source.
///
/// # Example
///
/// ```ignore
/// let output = MarkdownEditor::new(&mut content)
///     .mode(EditorMode::Rendered)
///     .font_size(14.0)
///     .show(ui);
/// ```
pub struct MarkdownEditor<'a> {
    /// The markdown content being edited
    content: &'a mut String,
    /// Current editing mode
    mode: EditorMode,
    /// Font size for the editor
    font_size: f32,
    /// Font family for the editor
    font_family: EditorFont,
    /// Whether word wrap is enabled
    word_wrap: bool,
    /// Theme for styling
    theme: Theme,
    /// Custom ID for the editor
    id: Option<egui::Id>,
    /// Line number to scroll to (1-indexed, from outline navigation)
    scroll_to_line: Option<usize>,
}

impl<'a> MarkdownEditor<'a> {
    /// Create a new markdown editor for the given content.
    pub fn new(content: &'a mut String) -> Self {
        Self {
            content,
            mode: EditorMode::Raw,
            font_size: 14.0,
            font_family: EditorFont::default(),
            word_wrap: true,
            theme: Theme::Light,
            id: None,
            scroll_to_line: None,
        }
    }

    /// Set the editing mode.
    #[must_use]
    pub fn mode(mut self, mode: EditorMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the font size.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set whether word wrap is enabled.
    #[must_use]
    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    /// Set the theme.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the font family.
    #[must_use]
    pub fn font_family(mut self, font_family: EditorFont) -> Self {
        self.font_family = font_family;
        self
    }

    /// Set a custom ID for the editor.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set a line to scroll to (1-indexed, for outline navigation).
    #[must_use]
    pub fn scroll_to_line(mut self, line: Option<usize>) -> Self {
        self.scroll_to_line = line;
        self
    }

    /// Apply settings to the editor widget.
    #[must_use]
    pub fn with_settings(mut self, settings: &Settings) -> Self {
        self.font_size = settings.font_size;
        self.font_family = settings.font_family;
        self.word_wrap = settings.word_wrap;
        self.theme = settings.theme;
        self
    }

    /// Show the editor widget and return the output.
    pub fn show(self, ui: &mut Ui) -> MarkdownEditorOutput {
        let id = self.id.unwrap_or_else(|| ui.id().with("markdown_editor"));
        let colors = EditorColors::from_theme(self.theme, ui.visuals());

        match self.mode {
            EditorMode::Raw => self.show_raw_editor(ui, id),
            EditorMode::Rendered => self.show_rendered_editor(ui, id, &colors),
        }
    }

    /// Show the raw text editor (plain markdown editing).
    fn show_raw_editor(self, ui: &mut Ui, id: egui::Id) -> MarkdownEditorOutput {
        let original_content = self.content.clone();
        let font_size = self.font_size;
        let word_wrap = self.word_wrap;
        let editor_font = self.font_family;

        // Get font family for regular text
        let font_family = fonts::get_styled_font_family(false, false, editor_font);

        let scroll_output = ScrollArea::vertical()
            .id_source(id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let font_family_clone = font_family.clone();
                let mut layouter = move |ui: &Ui, text: &str, wrap_width: f32| {
                    let font_id = FontId::new(font_size, font_family_clone.clone());
                    let layout_job = if word_wrap {
                        egui::text::LayoutJob::simple(
                            text.to_owned(),
                            font_id,
                            ui.visuals().text_color(),
                            wrap_width,
                        )
                    } else {
                        egui::text::LayoutJob::simple_singleline(
                            text.to_owned(),
                            font_id,
                            ui.visuals().text_color(),
                        )
                    };
                    ui.fonts(|f| f.layout_job(layout_job))
                };

                TextEdit::multiline(self.content)
                    .id(id)
                    .frame(false)
                    .font(FontId::new(font_size, font_family.clone()))
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .show(ui)
            });

        let text_output = scroll_output.inner;
        let changed = *self.content != original_content;

        let cursor_position = if let Some(cursor_range) = text_output.cursor_range {
            let cursor = cursor_range.primary;
            char_index_to_line_col(self.content, cursor.ccursor.index)
        } else {
            (0, 0)
        };

        if changed {
            debug!("Raw editor content changed");
        }

        MarkdownEditorOutput {
            response: text_output.response,
            changed,
            cursor_position,
            mode: EditorMode::Raw,
            focused_element: None, // Raw mode doesn't use element tracking
            scroll_offset: scroll_output.state.offset.y,
        }
    }

    /// Show the WYSIWYG rendered editor.
    fn show_rendered_editor(
        self,
        ui: &mut Ui,
        id: egui::Id,
        colors: &EditorColors,
    ) -> MarkdownEditorOutput {
        let original_content = self.content.clone();
        let mut edit_state = EditState::new();
        let mut structural_state = StructuralEditState::new();

        // Parse the markdown content
        let doc = match parse_markdown(self.content) {
            Ok(doc) => doc,
            Err(e) => {
                // On parse error, show error and fall back to raw editing
                ui.colored_label(Color32::RED, format!("Parse error: {}", e));
                return self.show_raw_editor(ui, id);
            }
        };

        // Calculate scroll offset for outline navigation if needed
        let target_scroll_offset: Option<f32> = if let Some(target_line) = self.scroll_to_line {
            let font_id = FontId::new(
                self.font_size,
                fonts::get_styled_font_family(false, false, self.font_family),
            );
            let line_height = ui.fonts(|f| f.row_height(&font_id));
            // Target scroll position: put the line roughly 1/3 from top of viewport
            let viewport_height = ui.available_height();
            // target_line is 1-indexed, convert to 0-indexed for calculation
            let target_y = (target_line.saturating_sub(1)) as f32 * line_height;
            Some((target_y - viewport_height / 3.0).max(0.0))
        } else {
            None
        };

        // Render the document in a scroll area
        let mut scroll_area = ScrollArea::vertical()
            .id_source(id.with("rendered_scroll"))
            .auto_shrink([false, false]);

        // Apply scroll offset if we need to jump to a line
        if let Some(offset) = target_scroll_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        // Compute content hash to use as a unique ID scope.
        // This ensures that when content changes (e.g., edited in raw mode),
        // all inner TextEdit widgets get new IDs and re-read their content
        // instead of using cached internal state.
        let content_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.content.hash(&mut hasher);
            hasher.finish()
        };

        let scroll_output = scroll_area.show(ui, |ui| {
            // Push the content hash as an ID scope so all inner widgets
            // get unique IDs when content changes
            ui.push_id(content_hash, |ui| {
                // Minimal spacing - let individual elements control their margins
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 1.0);

                // Render all children of the document root
                // Note: Using original render_node (not the structural_keys version) since
                // structural key handling is currently disabled due to compatibility issues
                for node in &doc.root.children {
                    render_node(
                        ui,
                        node,
                        self.content,
                        &mut edit_state,
                        colors,
                        self.font_size,
                        self.font_family,
                        0,
                    );
                }

                // Keep structural_state alive to avoid unused variable warning
                let _ = &structural_state;

                // Return a response from the scroll area content
                ui.allocate_response(Vec2::ZERO, egui::Sense::hover())
            })
            .inner
        });

        // Apply any pending structural edits
        let mut structural_changed = false;
        if let Some(pending_edit) = structural_state.take_pending_edit() {
            if pending_edit.performed {
                *self.content = pending_edit.new_source;
                structural_changed = true;
                debug!(
                    "Applied structural edit, cursor at line {}",
                    pending_edit.cursor_position.line
                );
            }
        }

        // Check if any nodes were modified and rebuild markdown if needed
        let content_changed = edit_state.any_modified();
        if content_changed {
            rebuild_markdown(self.content, &edit_state, &original_content);
            debug!("WYSIWYG editor content changed, rebuilt markdown");
        }

        let changed = content_changed || structural_changed;

        // Get focused element info for formatting commands
        let focused_element = edit_state.get_focused_element(&original_content);

        MarkdownEditorOutput {
            response: scroll_output.inner,
            changed,
            cursor_position: (0, 0), // Position tracking is simplified in WYSIWYG mode
            mode: EditorMode::Rendered,
            focused_element,
            scroll_offset: scroll_output.state.offset.y,
        }
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Node Rendering
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Render a markdown node with structural key handling.
/// This wraps the standard rendering and adds detection of Enter, Backspace, Tab, Shift+Tab
/// to enable word processor-like editing behavior.
fn render_node_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
    parent_list_type: Option<&ListType>,
    list_item_index: Option<usize>,
) {
    match &node.node_type {
        MarkdownNodeType::Heading { level, .. } => {
            render_heading_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                *level,
            );
        }
        MarkdownNodeType::Paragraph => {
            render_paragraph_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        }
        MarkdownNodeType::CodeBlock {
            language, literal, ..
        } => {
            render_code_block(
                ui, source, edit_state, colors, font_size, language, literal, node,
            );
        }
        MarkdownNodeType::BlockQuote => {
            render_blockquote_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        }
        MarkdownNodeType::List { list_type, .. } => {
            render_list_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                list_type,
            );
        }
        MarkdownNodeType::ThematicBreak => {
            render_thematic_break(ui, colors);
        }
        MarkdownNodeType::Table { .. } => {
            render_table(ui, node, source, edit_state, colors, font_size);
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("HTML:").color(colors.quote_text).italics());
            });
            ui.add(
                TextEdit::multiline(&mut html.clone())
                    .code_editor()
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        }
        MarkdownNodeType::Link { url, title } => {
            render_link(ui, node, source, edit_state, colors, font_size, url, title);
        }
        MarkdownNodeType::Strong => {
            render_styled_inline(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                TextStyle::new().with_bold(),
            );
        }
        MarkdownNodeType::Emphasis => {
            render_styled_inline(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                TextStyle::new().with_italic(),
            );
        }
        MarkdownNodeType::Document => {
            for child in &node.children {
                render_node_with_structural_keys(
                    ui,
                    child,
                    source,
                    edit_state,
                    structural_state,
                    colors,
                    font_size,
                    editor_font,
                    indent_level,
                    parent_list_type,
                    list_item_index,
                );
            }
        }
        MarkdownNodeType::Item => {
            // List items are handled by render_list_with_structural_keys
        }
        MarkdownNodeType::TableRow { .. } | MarkdownNodeType::TableCell => {
            // Tables handled by render_table
        }
        _ => {
            let text = node.text_content();
            if !text.is_empty() {
                ui.label(&text);
            }
        }
    }
}

/// Render a markdown node as an editable egui widget.
/// (Legacy function for backward compatibility - without structural key handling)
fn render_node(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    match &node.node_type {
        MarkdownNodeType::Heading { level, .. } => {
            render_heading(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                *level,
            );
        }
        MarkdownNodeType::Paragraph => {
            render_paragraph(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        }
        MarkdownNodeType::CodeBlock {
            language, literal, ..
        } => {
            render_code_block(
                ui, source, edit_state, colors, font_size, language, literal, node,
            );
        }
        MarkdownNodeType::BlockQuote => {
            render_blockquote(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        }
        MarkdownNodeType::List { list_type, .. } => {
            render_list(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                list_type,
            );
        }
        MarkdownNodeType::ThematicBreak => {
            render_thematic_break(ui, colors);
        }
        MarkdownNodeType::Table { .. } => {
            render_table(ui, node, source, edit_state, colors, font_size);
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            // Show HTML blocks as code
            ui.horizontal(|ui| {
                ui.label(RichText::new("HTML:").color(colors.quote_text).italics());
            });
            ui.add(
                TextEdit::multiline(&mut html.clone())
                    .code_editor()
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        }
        MarkdownNodeType::Link { url, title } => {
            render_link(ui, node, source, edit_state, colors, font_size, url, title);
        }
        MarkdownNodeType::Strong => {
            // Render strong (bold) with proper style accumulation for nested formatting
            render_styled_inline(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                TextStyle::new().with_bold(),
            );
        }
        MarkdownNodeType::Emphasis => {
            // Render emphasis (italic) with proper style accumulation for nested formatting
            render_styled_inline(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                TextStyle::new().with_italic(),
            );
        }
        // Skip container nodes that are handled by their parents
        MarkdownNodeType::Document => {
            for child in &node.children {
                render_node(
                    ui,
                    child,
                    source,
                    edit_state,
                    colors,
                    font_size,
                    editor_font,
                    indent_level,
                );
            }
        }
        MarkdownNodeType::Item => {
            // Handled by render_list
        }
        MarkdownNodeType::TableRow { .. } | MarkdownNodeType::TableCell => {
            // Handled by render_table
        }
        _ => {
            // For other inline nodes, render as text if they have content
            let text = node.text_content();
            if !text.is_empty() {
                ui.label(&text);
            }
        }
    }
}

/// Render a heading as an editable widget.
fn render_heading(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    base_font_size: f32,
    editor_font: EditorFont,
    level: HeadingLevel,
) {
    let text = node.text_content();
    let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

    // Font sizes for different heading levels
    let font_size = match level {
        HeadingLevel::H1 => base_font_size * 1.8,
        HeadingLevel::H2 => base_font_size * 1.5,
        HeadingLevel::H3 => base_font_size * 1.3,
        HeadingLevel::H4 => base_font_size * 1.15,
        HeadingLevel::H5 => base_font_size * 1.05,
        HeadingLevel::H6 => base_font_size,
    };

    // Headings use bold font
    let font_family = fonts::get_styled_font_family(true, false, editor_font);

    // Add small top margin for headings (separation from previous content)
    let top_margin = match level {
        HeadingLevel::H1 => 8.0,
        HeadingLevel::H2 => 6.0,
        _ => 4.0,
    };
    ui.add_space(top_margin);

    // Editable heading text with left indent
    let (has_focus, selection, changed, new_text) = ui
        .horizontal(|ui| {
            ui.add_space(4.0); // Small left indent for headings

            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let text_edit = TextEdit::singleline(&mut editable.text)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.heading)
                    .frame(false)
                    .desired_width(f32::INFINITY);

                let output = text_edit.show(ui);

                let changed = output.response.changed();
                let has_focus = output.response.has_focus();
                let selection = if has_focus {
                    output.cursor_range.map(|range| {
                        let primary = range.primary.ccursor.index;
                        let secondary = range.secondary.ccursor.index;
                        if primary < secondary {
                            (primary, secondary)
                        } else {
                            (secondary, primary)
                        }
                    })
                } else {
                    None
                };

                let new_text = if changed {
                    editable.modified = true;
                    Some(editable.text.clone())
                } else {
                    None
                };

                (has_focus, selection, changed, new_text)
            } else {
                (false, None, false, None)
            }
        })
        .inner;

    // Update source if changed
    if changed {
        if let Some(text) = new_text {
            update_source_line(source, node.start_line, &format_heading(&text, level));
        }
    }

    // Track focus
    if has_focus {
        edit_state.set_focus(node_id, selection);
    }
    // No bottom margin - heading should be close to following content
}

/// Render a heading with structural key handling (Enter creates paragraph after).
fn render_heading_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    base_font_size: f32,
    editor_font: EditorFont,
    level: HeadingLevel,
) {
    let text = node.text_content();
    let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

    // Font sizes for different heading levels
    let font_size = match level {
        HeadingLevel::H1 => base_font_size * 1.8,
        HeadingLevel::H2 => base_font_size * 1.5,
        HeadingLevel::H3 => base_font_size * 1.3,
        HeadingLevel::H4 => base_font_size * 1.15,
        HeadingLevel::H5 => base_font_size * 1.05,
        HeadingLevel::H6 => base_font_size,
    };

    // Headings use bold font
    let font_family = fonts::get_styled_font_family(true, false, editor_font);

    // Add small top margin for headings
    let top_margin = match level {
        HeadingLevel::H1 => 8.0,
        HeadingLevel::H2 => 6.0,
        _ => 4.0,
    };
    ui.add_space(top_margin);

    // Editable heading text with left indent
    ui.horizontal(|ui| {
        ui.add_space(4.0);

        if let Some(editable) = edit_state.get_node_mut(node_id) {
            let response = ui.add(
                TextEdit::singleline(&mut editable.text)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.heading)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );

            // Note: Structural key handling disabled for now to fix editing bugs
            // TODO(deferred): Re-enable with proper key event handling
            // The ast_ops module provides Enter/Tab/Backspace handling for WYSIWYG mode
            let _ = structural_state;

            if response.changed() {
                editable.modified = true;
                update_source_line(
                    source,
                    node.start_line,
                    &format_heading(&editable.text, level),
                );
            }
        }
    });
}

/// Render a paragraph with structural key handling (Enter splits paragraph).
fn render_paragraph_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    // Check if paragraph contains special inline elements
    let has_inline_elements = node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Link { .. }
                | MarkdownNodeType::Strong
                | MarkdownNodeType::Emphasis
                | MarkdownNodeType::Strikethrough
                | MarkdownNodeType::Code(_)
        )
    });

    // Check if paragraph starts with bold text (section header pattern)
    let starts_with_bold = node
        .children
        .first()
        .map(|c| matches!(c.node_type, MarkdownNodeType::Strong))
        .unwrap_or(false);

    if starts_with_bold && indent_level == 0 {
        ui.add_space(6.0);
    }

    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    if has_inline_elements {
        // For formatted paragraphs, use hybrid click-to-edit approach
        let formatted_para_id = ui.id().with("formatted_paragraph_sk").with(node.start_line);

        let mut para_edit_state = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(
                    formatted_para_id.with("edit_state"),
                    FormattedItemEditState::default,
                )
                .clone()
        });

        let widget_id = formatted_para_id.with("text_edit");

        ui.horizontal(|ui| {
            ui.add_space(4.0 + indent_level as f32 * 20.0);

            if para_edit_state.editing {
                // EDIT MODE: Show TextEdit with raw markdown
                let text_edit = TextEdit::multiline(&mut para_edit_state.edit_text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family.clone()))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(ui.available_width())
                    .desired_rows(1);

                let response = ui.add(text_edit);

                if para_edit_state.needs_focus {
                    response.request_focus();
                    para_edit_state.needs_focus = false;
                }

                let enter_pressed = response.has_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                let escape_pressed =
                    response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                if enter_pressed || focus_lost {
                    update_source_range(
                        source,
                        node.start_line,
                        node.end_line,
                        &para_edit_state.edit_text,
                    );

                    let node_id = edit_state.add_node(
                        para_edit_state.edit_text.clone(),
                        node.start_line,
                        node.end_line,
                    );
                    if let Some(editable) = edit_state.get_node_mut(node_id) {
                        editable.modified = true;
                    }

                    para_edit_state.editing = false;
                    debug!(
                        "Saved and exiting edit mode for formatted paragraph at line {}",
                        node.start_line
                    );
                } else if escape_pressed {
                    para_edit_state.editing = false;
                    debug!(
                        "Cancelled edit mode for formatted paragraph at line {}",
                        node.start_line
                    );
                }

                // Note: Structural key handling disabled for now
                let _ = structural_state;

                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                });
            } else {
                // DISPLAY MODE: Show formatted text, click to edit
                let display_response = ui
                    .horizontal_wrapped(|ui| {
                        let style = TextStyle::new();
                        for child in &node.children {
                            render_inline_node(
                                ui,
                                child,
                                source,
                                edit_state,
                                colors,
                                font_size,
                                editor_font,
                                style,
                            );
                        }
                    })
                    .response;

                let sense_response = ui.interact(
                    display_response.rect,
                    formatted_para_id.with("click_sense"),
                    egui::Sense::click(),
                );

                if sense_response.clicked() {
                    para_edit_state.editing = true;
                    para_edit_state.needs_focus = true;
                    para_edit_state.edit_text =
                        extract_paragraph_content(source, node.start_line, node.end_line);
                    debug!(
                        "Entering edit mode for formatted paragraph at line {}",
                        node.start_line
                    );

                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                    });
                }

                if sense_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
            }
        });
    } else {
        // Simple text-only paragraph with structural key support
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

        ui.horizontal(|ui| {
            ui.add_space(4.0 + indent_level as f32 * 20.0);

            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let response = ui.add(
                    TextEdit::multiline(&mut editable.text)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .desired_rows(1),
                );

                // Note: Structural key handling disabled for now
                let _ = structural_state;

                if response.changed() {
                    editable.modified = true;
                    update_source_range(source, node.start_line, node.end_line, &editable.text);
                }
            }
        });
    }
}

/// Render a blockquote with structural key support for children.
fn render_blockquote_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    ui.horizontal(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(4.0, ui.available_height()), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.quote_border);

        ui.add_space(8.0);

        ui.vertical(|ui| {
            for child in &node.children {
                render_node_with_structural_keys(
                    ui,
                    child,
                    source,
                    edit_state,
                    structural_state,
                    colors,
                    font_size,
                    editor_font,
                    indent_level + 1,
                    None,
                    None,
                );
            }
        });
    });
}

/// Render a list with structural key support for items.
fn render_list_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
    list_type: &ListType,
) {
    let mut item_number = match list_type {
        ListType::Ordered { start, .. } => *start,
        ListType::Bullet => 0,
    };

    for (idx, child) in node.children.iter().enumerate() {
        if let MarkdownNodeType::Item = &child.node_type {
            render_list_item_with_structural_keys(
                ui,
                child,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                list_type,
                item_number,
                idx,
            );
            item_number += 1;
        }
    }

    if indent_level == 0 {
        ui.add_space(4.0);
    }
}

/// Render a single list item with structural key support.
fn render_list_item_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
    list_type: &ListType,
    item_number: u32,
    _item_index: usize,
) {
    let is_task = node
        .children
        .iter()
        .any(|c| matches!(c.node_type, MarkdownNodeType::TaskItem { .. }));

    let para_node = node
        .children
        .iter()
        .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

    let nested_lists: Vec<&MarkdownNode> = node
        .children
        .iter()
        .filter(|c| matches!(c.node_type, MarkdownNodeType::List { .. }))
        .collect();

    // Check if paragraph has inline formatting (bold, italic, etc.)
    let has_inline_formatting = para_node
        .map(|p| {
            p.children.iter().any(|c| {
                matches!(
                    c.node_type,
                    MarkdownNodeType::Strong
                        | MarkdownNodeType::Emphasis
                        | MarkdownNodeType::Strikethrough
                        | MarkdownNodeType::Link { .. }
                        | MarkdownNodeType::Code(_)
                )
            })
        })
        .unwrap_or(false);

    // For simple text (no inline formatting), register editable node BEFORE the layout
    let simple_text_node_id = if !has_inline_formatting {
        if let Some(para) = para_node {
            let text = para.text_content();
            if !text.is_empty() {
                Some((
                    edit_state.add_node(text.clone(), para.start_line, para.end_line),
                    para.start_line,
                    para.end_line,
                ))
            } else {
                None
            }
        } else {
            let text: String = node
                .children
                .iter()
                .filter(|c| {
                    !matches!(
                        c.node_type,
                        MarkdownNodeType::List { .. } | MarkdownNodeType::TaskItem { .. }
                    )
                })
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                Some((
                    edit_state.add_node(text.clone(), node.start_line, node.end_line),
                    node.start_line,
                    node.end_line,
                ))
            } else {
                None
            }
        }
    } else {
        None
    };

    let indent_width = indent_level as f32 * 20.0;
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    // For formatted items, create a unique ID for tracking edit state
    let formatted_item_id = ui.id().with("formatted_list_item_sk").with(node.start_line);

    ui.horizontal(|ui| {
        ui.add_space(indent_width);

        if is_task {
            let checked = node
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

            let mut task_checked = checked;
            if ui.checkbox(&mut task_checked, "").changed() {
                debug!("Task item toggled: {}", task_checked);
            }
        } else {
            let marker = match list_type {
                ListType::Bullet => if indent_level == 0 {
                    "\u{2022}" // bullet •
                } else {
                    "\u{25E6}" // white bullet ◦
                }
                .to_string(),
                ListType::Ordered { delimiter, .. } => format!("{}{}", item_number, delimiter),
            };
            ui.label(
                RichText::new(&marker)
                    .color(colors.list_marker)
                    .font(FontId::new(font_size, font_family.clone())),
            );
            ui.add_space(4.0);
        }

        // Render item content
        if has_inline_formatting {
            if let Some(para) = para_node {
                // Get or create edit state for this formatted item
                let mut item_edit_state = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(
                            formatted_item_id.with("edit_state"),
                            FormattedItemEditState::default,
                        )
                        .clone()
                });

                let widget_id = formatted_item_id.with("text_edit");

                if item_edit_state.editing {
                    // EDIT MODE: Show TextEdit with raw markdown
                    let text_edit = TextEdit::singleline(&mut item_edit_state.edit_text)
                        .id(widget_id)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .margin(egui::vec2(0.0, 2.0));

                    let response = ui.add(text_edit);

                    // Request focus if needed (first frame after entering edit mode)
                    if item_edit_state.needs_focus {
                        response.request_focus();
                        item_edit_state.needs_focus = false;
                    }

                    // Check for exit conditions
                    let enter_pressed =
                        response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let escape_pressed =
                        response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                    let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                    if enter_pressed || focus_lost {
                        // Save changes and exit edit mode
                        update_source_range(
                            source,
                            para.start_line,
                            para.end_line,
                            &item_edit_state.edit_text,
                        );

                        // Mark as modified
                        let node_id = edit_state.add_node(
                            item_edit_state.edit_text.clone(),
                            para.start_line,
                            para.end_line,
                        );
                        if let Some(editable) = edit_state.get_node_mut(node_id) {
                            editable.modified = true;
                        }

                        item_edit_state.editing = false;
                        debug!(
                            "Saved and exiting edit mode for formatted list item at line {}",
                            para.start_line
                        );
                    } else if escape_pressed {
                        // Cancel without saving
                        item_edit_state.editing = false;
                        debug!(
                            "Cancelled edit mode for formatted list item at line {}",
                            para.start_line
                        );
                    }

                    // Note: Structural key handling disabled for now
                    let _ = structural_state;

                    // Always save the current state (including text edits)
                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                    });
                } else {
                    // DISPLAY MODE: Show formatted text, click to edit
                    let display_response = ui
                        .horizontal_wrapped(|ui| {
                            let style = TextStyle::new();
                            for child in &para.children {
                                render_inline_node(
                                    ui,
                                    child,
                                    source,
                                    edit_state,
                                    colors,
                                    font_size,
                                    editor_font,
                                    style,
                                );
                            }
                        })
                        .response;

                    // Make the display area interactive - enter edit mode on click
                    let sense_response = ui.interact(
                        display_response.rect,
                        formatted_item_id.with("click_sense"),
                        egui::Sense::click(),
                    );

                    if sense_response.clicked() {
                        // Enter edit mode
                        item_edit_state.editing = true;
                        item_edit_state.needs_focus = true;
                        // Get raw markdown content from source
                        item_edit_state.edit_text =
                            extract_list_item_content(source, para.start_line);
                        debug!(
                            "Entering edit mode for formatted list item at line {}, content: '{}'",
                            para.start_line, item_edit_state.edit_text
                        );

                        // Store the new state
                        ui.memory_mut(|mem| {
                            mem.data
                                .insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                        });
                    }

                    // Show hover cursor to indicate clickability
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text - editable
            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let widget_id = ui.id().with("list_item_text_sk").with(start_line);

                let response = ui.add(
                    TextEdit::singleline(&mut editable.text)
                        .id(widget_id)
                        .font(FontId::new(font_size, font_family))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .clip_text(false),
                );

                // Note: Structural key handling disabled for now
                let _ = structural_state;

                if response.changed() {
                    editable.modified = true;
                    update_source_range(source, start_line, end_line, &editable.text);
                }
            }
        }
    });

    // Render nested lists
    for nested_list in nested_lists {
        if let MarkdownNodeType::List {
            list_type: nested_type,
            ..
        } = &nested_list.node_type
        {
            render_list_with_structural_keys(
                ui,
                nested_list,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level + 1,
                nested_type,
            );
        }
    }
}

/// Render a paragraph as an editable widget.
fn render_paragraph(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    // Check if paragraph contains any special inline elements (links, formatting)
    let has_inline_elements = node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Link { .. }
                | MarkdownNodeType::Strong
                | MarkdownNodeType::Emphasis
                | MarkdownNodeType::Strikethrough
                | MarkdownNodeType::Code(_)
        )
    });

    // Check if paragraph starts with bold text (section header pattern like "**Description:**")
    let starts_with_bold = node
        .children
        .first()
        .map(|c| matches!(c.node_type, MarkdownNodeType::Strong))
        .unwrap_or(false);

    // Add top margin for section-header-like paragraphs
    if starts_with_bold && indent_level == 0 {
        ui.add_space(6.0);
    }

    // Get font family for regular (non-styled) text
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    if has_inline_elements {
        // For formatted paragraphs, use hybrid click-to-edit approach
        let formatted_para_id = ui.id().with("formatted_paragraph").with(node.start_line);

        // Get or create edit state for this formatted paragraph
        let mut para_edit_state = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(
                    formatted_para_id.with("edit_state"),
                    FormattedItemEditState::default,
                )
                .clone()
        });

        let widget_id = formatted_para_id.with("text_edit");

        ui.horizontal(|ui| {
            ui.add_space(4.0 + indent_level as f32 * 20.0);

            if para_edit_state.editing {
                // EDIT MODE: Show TextEdit with raw markdown
                let text_edit = TextEdit::multiline(&mut para_edit_state.edit_text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family.clone()))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(ui.available_width())
                    .desired_rows(1);

                let response = ui.add(text_edit);

                // Request focus if needed
                if para_edit_state.needs_focus {
                    response.request_focus();
                    para_edit_state.needs_focus = false;
                }

                // Check for exit conditions
                let enter_pressed = response.has_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                let escape_pressed =
                    response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                if enter_pressed || focus_lost {
                    // Save changes and exit edit mode
                    update_source_range(
                        source,
                        node.start_line,
                        node.end_line,
                        &para_edit_state.edit_text,
                    );

                    // Mark as modified
                    let node_id = edit_state.add_node(
                        para_edit_state.edit_text.clone(),
                        node.start_line,
                        node.end_line,
                    );
                    if let Some(editable) = edit_state.get_node_mut(node_id) {
                        editable.modified = true;
                    }

                    para_edit_state.editing = false;
                    debug!(
                        "Saved and exiting edit mode for formatted paragraph at line {}",
                        node.start_line
                    );
                } else if escape_pressed {
                    // Cancel without saving
                    para_edit_state.editing = false;
                    debug!(
                        "Cancelled edit mode for formatted paragraph at line {}",
                        node.start_line
                    );
                }

                // Always save the current state
                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                });
            } else {
                // DISPLAY MODE: Show formatted text, click to edit
                let display_response = ui
                    .horizontal_wrapped(|ui| {
                        let style = TextStyle::new();
                        for child in &node.children {
                            render_inline_node(
                                ui,
                                child,
                                source,
                                edit_state,
                                colors,
                                font_size,
                                editor_font,
                                style,
                            );
                        }
                    })
                    .response;

                // Make the display area interactive
                let sense_response = ui.interact(
                    display_response.rect,
                    formatted_para_id.with("click_sense"),
                    egui::Sense::click(),
                );

                if sense_response.clicked() {
                    // Enter edit mode
                    para_edit_state.editing = true;
                    para_edit_state.needs_focus = true;
                    para_edit_state.edit_text =
                        extract_paragraph_content(source, node.start_line, node.end_line);
                    debug!(
                        "Entering edit mode for formatted paragraph at line {}",
                        node.start_line
                    );

                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                    });
                }

                if sense_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
            }
        });
    } else {
        // Simple text-only paragraph - use editable text directly
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

        let (has_focus, selection, changed, new_text) = ui
            .horizontal(|ui| {
                // Add base left indent + any extra indentation
                ui.add_space(4.0 + indent_level as f32 * 20.0);

                if let Some(editable) = edit_state.get_node_mut(node_id) {
                    let text_edit = TextEdit::multiline(&mut editable.text)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .desired_rows(1);

                    let output = text_edit.show(ui);

                    let changed = output.response.changed();
                    let has_focus = output.response.has_focus();
                    let selection = if has_focus {
                        output.cursor_range.map(|range| {
                            let primary = range.primary.ccursor.index;
                            let secondary = range.secondary.ccursor.index;
                            if primary < secondary {
                                (primary, secondary)
                            } else {
                                (secondary, primary)
                            }
                        })
                    } else {
                        None
                    };

                    let new_text = if changed {
                        editable.modified = true;
                        Some(editable.text.clone())
                    } else {
                        None
                    };

                    (has_focus, selection, changed, new_text)
                } else {
                    (false, None, false, None)
                }
            })
            .inner;

        // Update source if changed
        if changed {
            if let Some(text) = new_text {
                update_source_range(source, node.start_line, node.end_line, &text);
            }
        }

        // Track focus
        if has_focus {
            edit_state.set_focus(node_id, selection);
        }
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Text Style Accumulator
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Accumulated text styles for nested formatting.
/// Tracks bold, italic, and strikethrough states that can be combined.
#[derive(Debug, Clone, Copy, Default)]
struct TextStyle {
    /// Whether text should be bold
    bold: bool,
    /// Whether text should be italic
    italic: bool,
    /// Whether text should be strikethrough
    strikethrough: bool,
}

impl TextStyle {
    /// Create a new default (unstyled) text style.
    fn new() -> Self {
        Self::default()
    }

    /// Create a new style with bold enabled.
    fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Create a new style with italic enabled.
    fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Create a new style with strikethrough enabled.
    fn with_strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Apply this style to a RichText with proper font family.
    ///
    /// This uses explicit font families for bold/italic instead of relying
    /// on egui's `.strong()` method which may not work with all fonts.
    fn apply(&self, text: RichText, font_size: f32, editor_font: EditorFont) -> RichText {
        // Get the appropriate font family for the style combination
        let family = fonts::get_styled_font_family(self.bold, self.italic, editor_font);
        let mut styled = text.font(FontId::new(font_size, family));

        // Strikethrough is a separate decoration, not a font variant
        if self.strikethrough {
            styled = styled.strikethrough();
        }
        styled
    }
}

/// Render inline content (text, links, bold, italic, etc.) with proper formatting.
fn render_inline_content(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    ui.horizontal_wrapped(|ui| {
        // Add base left indent + any extra indentation
        ui.add_space(4.0 + indent_level as f32 * 20.0);

        let style = TextStyle::new();
        for child in &node.children {
            render_inline_node(
                ui,
                child,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                style,
            );
        }
    });
}

/// Render a single inline node (text, link, bold, italic, etc.).
/// The `style` parameter accumulates formatting from parent nodes to handle nested emphasis.
fn render_inline_node(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    style: TextStyle,
) {
    match &node.node_type {
        MarkdownNodeType::Text(text) => {
            // Apply accumulated styles to the text
            // Apply color, then use styled font with bold/italic variant
            let rich_text = RichText::new(text).color(colors.text);
            let styled = style.apply(rich_text, font_size, editor_font);
            ui.label(styled);
        }

        MarkdownNodeType::Link { url, title } => {
            // Render link as editable text with link styling
            // Note: links don't inherit text styles to maintain their distinct appearance
            render_link(ui, node, source, edit_state, colors, font_size, url, title);
        }

        MarkdownNodeType::Strong => {
            // Add bold to the style and render children with accumulated styles
            let new_style = style.with_bold();
            for child in &node.children {
                render_inline_node(
                    ui,
                    child,
                    source,
                    edit_state,
                    colors,
                    font_size,
                    editor_font,
                    new_style,
                );
            }
        }

        MarkdownNodeType::Emphasis => {
            // Add italic to the style and render children with accumulated styles
            let new_style = style.with_italic();
            for child in &node.children {
                render_inline_node(
                    ui,
                    child,
                    source,
                    edit_state,
                    colors,
                    font_size,
                    editor_font,
                    new_style,
                );
            }
        }

        MarkdownNodeType::Strikethrough => {
            // Add strikethrough to the style and render children with accumulated styles
            let new_style = style.with_strikethrough();
            for child in &node.children {
                render_inline_node(
                    ui,
                    child,
                    source,
                    edit_state,
                    colors,
                    font_size,
                    editor_font,
                    new_style,
                );
            }
        }

        MarkdownNodeType::Code(code) => {
            // Inline code has its own styling - doesn't inherit text styles
            ui.label(
                RichText::new(code)
                    .color(colors.code_text)
                    .font(FontId::monospace(font_size * 0.9))
                    .background_color(colors.code_bg),
            );
        }

        MarkdownNodeType::SoftBreak => {
            ui.label(" ");
        }

        MarkdownNodeType::LineBreak => {
            ui.end_row();
        }

        _ => {
            // For other nodes with children, render them with current style
            if !node.children.is_empty() {
                for child in &node.children {
                    render_inline_node(
                        ui,
                        child,
                        source,
                        edit_state,
                        colors,
                        font_size,
                        editor_font,
                        style,
                    );
                }
            } else {
                // For leaf nodes, just render text content with current style
                let text = node.text_content();
                if !text.is_empty() {
                    let rich_text = RichText::new(&text).color(colors.text);
                    let styled = style.apply(rich_text, font_size, editor_font);
                    ui.label(styled);
                }
            }
        }
    }
}

/// Render a code block as an editable widget with syntax highlighting and language selection.
fn render_code_block(
    ui: &mut Ui,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    language: &str,
    literal: &str,
    node: &MarkdownNode,
) {
    // Determine if we're in dark mode based on the background color
    let dark_mode = colors.background.r() < 128;

    // Create a stable ID for this code block using only position info
    // We use start_line as the primary identifier - it's stable during editing
    // Note: We don't include content hash because that changes during editing!
    let code_block_id = egui::Id::new(("codeblock", node.start_line));

    // Convert EditorColors to WidgetColors for the code block widget
    let widget_colors = WidgetColors {
        text: colors.text,
        heading: colors.heading,
        code_bg: colors.code_bg,
        list_marker: colors.list_marker,
        muted: colors.quote_text,
    };

    // Store the code block data in egui's memory so it persists across frames
    let mut code_data = ui.memory_mut(|mem| {
        mem.data
            .get_temp_mut_or_insert_with(code_block_id.with("state"), || {
                CodeBlockData::new(literal, language)
            })
            .clone()
    });

    // CRITICAL: Check if the source content has changed (e.g., edited in raw mode)
    // If so, update the cached data to match the current parsed content.
    // This fixes the bug where editing a code block in raw mode wouldn't update
    // the rendered view because the cached CodeBlockData was stale.
    if code_data.code != literal || code_data.language != language {
        code_data = CodeBlockData::new(literal, language);
    }

    // Create and show the editable code block widget
    let output = EditableCodeBlock::new(&mut code_data)
        .font_size(font_size)
        .dark_mode(dark_mode)
        .colors(widget_colors)
        .id(code_block_id)
        .show(ui);

    // Update stored data
    ui.memory_mut(|mem| {
        mem.data.insert_temp(code_block_id.with("state"), code_data);
    });

    // Handle changes
    if output.changed {
        // Update the source with the new code and/or language
        update_code_block(
            source,
            node.start_line,
            node.end_line,
            &output.language,
            &output.code,
        );

        // Mark that something was modified in edit state
        let node_id = edit_state.add_node(output.code.clone(), node.start_line, node.end_line);
        if let Some(editable) = edit_state.get_node_mut(node_id) {
            editable.modified = true;
        }

        debug!(
            "Code block at line {} modified (language: {})",
            node.start_line, output.language
        );
    }
}

/// Render a block quote.
fn render_blockquote(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
) {
    ui.horizontal(|ui| {
        // Quote border
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(4.0, ui.available_height()), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.quote_border);

        ui.add_space(8.0);

        ui.vertical(|ui| {
            for child in &node.children {
                render_node(
                    ui,
                    child,
                    source,
                    edit_state,
                    colors,
                    font_size,
                    editor_font,
                    indent_level + 1,
                );
            }
        });
    });
}

/// Render a list (ordered or unordered).
fn render_list(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
    list_type: &ListType,
) {
    let mut item_number = match list_type {
        ListType::Ordered { start, .. } => *start,
        ListType::Bullet => 0,
    };

    for child in &node.children {
        if let MarkdownNodeType::Item = &child.node_type {
            render_list_item(
                ui,
                child,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                list_type,
                item_number,
            );
            item_number += 1;
        }
    }

    // Add small spacing after top-level lists
    if indent_level == 0 {
        ui.add_space(4.0);
    }
}

/// State for tracking click-to-edit mode on formatted list items.
#[derive(Debug, Clone, Default)]
struct FormattedItemEditState {
    /// Whether the item is currently being edited
    editing: bool,
    /// The raw markdown content being edited
    edit_text: String,
    /// Flag to request focus on the next frame
    needs_focus: bool,
}

/// Extract the raw content text from a source line (removes list marker prefix).
fn extract_list_item_content(source: &str, start_line: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > 0 && start_line <= lines.len() {
        let line = lines[start_line - 1];
        let (_, content) = extract_line_prefix(line);
        content.to_string()
    } else {
        String::new()
    }
}

/// Extract raw paragraph content from source lines.
fn extract_paragraph_content(source: &str, start_line: usize, end_line: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > 0 && start_line <= lines.len() {
        let end = end_line.min(lines.len());
        lines[(start_line - 1)..end]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    }
}

/// Render a single list item.
fn render_list_item(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    indent_level: usize,
    list_type: &ListType,
    item_number: u32,
) {
    // Check for task item
    let is_task = node
        .children
        .iter()
        .any(|c| matches!(c.node_type, MarkdownNodeType::TaskItem { .. }));

    // Find the paragraph node (contains the list item content)
    let para_node = node
        .children
        .iter()
        .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

    // Collect nested lists to render separately
    let nested_lists: Vec<&MarkdownNode> = node
        .children
        .iter()
        .filter(|c| matches!(c.node_type, MarkdownNodeType::List { .. }))
        .collect();

    // Check if paragraph has inline formatting (bold, italic, etc.)
    let has_inline_formatting = para_node
        .map(|p| {
            p.children.iter().any(|c| {
                matches!(
                    c.node_type,
                    MarkdownNodeType::Strong
                        | MarkdownNodeType::Emphasis
                        | MarkdownNodeType::Strikethrough
                        | MarkdownNodeType::Link { .. }
                        | MarkdownNodeType::Code(_)
                )
            })
        })
        .unwrap_or(false);

    // For simple text (no inline formatting), register editable node BEFORE the layout
    let simple_text_node_id = if !has_inline_formatting {
        if let Some(para) = para_node {
            let text = para.text_content();
            if !text.is_empty() {
                Some((
                    edit_state.add_node(text.clone(), para.start_line, para.end_line),
                    para.start_line,
                    para.end_line,
                ))
            } else {
                None
            }
        } else {
            let text: String = node
                .children
                .iter()
                .filter(|c| {
                    !matches!(
                        c.node_type,
                        MarkdownNodeType::List { .. } | MarkdownNodeType::TaskItem { .. }
                    )
                })
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                Some((
                    edit_state.add_node(text.clone(), node.start_line, node.end_line),
                    node.start_line,
                    node.end_line,
                ))
            } else {
                None
            }
        }
    } else {
        None
    };

    let indent_width = indent_level as f32 * 20.0;
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    // For formatted items, create a unique ID for tracking edit state
    let formatted_item_id = ui.id().with("formatted_list_item").with(node.start_line);

    let focus_info: (bool, Option<(usize, usize)>, Option<usize>) = ui.horizontal(|ui| {
        // Indentation
        ui.add_space(indent_width);

        if is_task {
            // Find the task item state
            let checked = node
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

            let mut task_checked = checked;
            if ui.checkbox(&mut task_checked, "").changed() {
                debug!("Task item toggled: {}", task_checked);
            }
        } else {
            // List marker - use different markers for nested levels
            let marker = match list_type {
                ListType::Bullet => {
                    if indent_level == 0 {
                        "\u{2022}" // bullet •
                    } else {
                        "\u{25E6}" // white bullet ◦
                    }
                }
                .to_string(),
                ListType::Ordered { delimiter, .. } => format!("{}{}", item_number, delimiter),
            };
            ui.label(
                RichText::new(&marker)
                    .color(colors.list_marker)
                    .font(FontId::new(font_size, font_family.clone())),
            );
            ui.add_space(4.0);
        }

        // Render item content
        if has_inline_formatting {
            if let Some(para) = para_node {
                // Get or create edit state for this formatted item
                let mut item_edit_state = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(formatted_item_id.with("edit_state"), FormattedItemEditState::default)
                        .clone()
                });

                let widget_id = formatted_item_id.with("text_edit");

                if item_edit_state.editing {
                    // EDIT MODE: Show TextEdit with raw markdown
                    let text_edit = TextEdit::singleline(&mut item_edit_state.edit_text)
                        .id(widget_id)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .margin(egui::vec2(0.0, 2.0));

                    let response = ui.add(text_edit);

                    // Request focus if needed (first frame after entering edit mode)
                    if item_edit_state.needs_focus {
                        response.request_focus();
                        item_edit_state.needs_focus = false;
                    }

                    // Check for exit conditions:
                    // 1. Focus lost (clicked elsewhere)
                    // 2. Enter key pressed
                    // 3. Escape key pressed (cancel without saving)
                    let enter_pressed = response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let escape_pressed = response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                    let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                    if enter_pressed || focus_lost {
                        // Save changes and exit edit mode
                        update_source_range(source, para.start_line, para.end_line, &item_edit_state.edit_text);

                        // Mark as modified
                        let node_id = edit_state.add_node(item_edit_state.edit_text.clone(), para.start_line, para.end_line);
                        if let Some(editable) = edit_state.get_node_mut(node_id) {
                            editable.modified = true;
                        }

                        item_edit_state.editing = false;
                        debug!("Saved and exiting edit mode for formatted list item at line {}", para.start_line);
                    } else if escape_pressed {
                        // Cancel without saving
                        item_edit_state.editing = false;
                        debug!("Cancelled edit mode for formatted list item at line {}", para.start_line);
                    }

                    // Always save the current state (including text edits)
                    ui.memory_mut(|mem| {
                        mem.data.insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                    });
                } else {
                    // DISPLAY MODE: Show formatted text, click to edit
                    let display_response = ui.horizontal_wrapped(|ui| {
                        let style = TextStyle::new();
                        for child in &para.children {
                            render_inline_node(ui, child, source, edit_state, colors, font_size, editor_font, style);
                        }
                    }).response;

                    // Make the display area interactive - enter edit mode on click
                    let sense_response = ui.interact(
                        display_response.rect,
                        formatted_item_id.with("click_sense"),
                        egui::Sense::click(),
                    );

                    if sense_response.clicked() {
                        // Enter edit mode
                        item_edit_state.editing = true;
                        item_edit_state.needs_focus = true;
                        // Get raw markdown content from source
                        item_edit_state.edit_text = extract_list_item_content(source, para.start_line);
                        debug!("Entering edit mode for formatted list item at line {}, content: '{}'",
                               para.start_line, item_edit_state.edit_text);

                        // Store the new state
                        ui.memory_mut(|mem| {
                            mem.data.insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                        });
                    }

                    // Show hover cursor to indicate clickability
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text - editable
            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let widget_id = ui.id().with("list_item_text").with(start_line);

                let text_edit = TextEdit::singleline(&mut editable.text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(f32::INFINITY)
                    .clip_text(false);

                let output = text_edit.show(ui);

                let changed = output.response.changed();
                let has_focus = output.response.has_focus();
                let selection = if has_focus {
                    output.cursor_range.map(|range| {
                        let primary = range.primary.ccursor.index;
                        let secondary = range.secondary.ccursor.index;
                        if primary < secondary {
                            (primary, secondary)
                        } else {
                            (secondary, primary)
                        }
                    })
                } else {
                    None
                };

                if changed {
                    editable.modified = true;
                    update_source_range(source, start_line, end_line, &editable.text);
                }

                // Return focus info for tracking
                return (has_focus, selection, Some(node_id));
            }
        }
        (false, None, None)
    }).inner;

    // Track focus for list item
    if focus_info.0 {
        if let Some(node_id) = focus_info.2 {
            edit_state.set_focus(node_id, focus_info.1);
        }
    }

    // Render any nested lists with increased indentation
    for nested_list in nested_lists {
        if let MarkdownNodeType::List {
            list_type: nested_type,
            ..
        } = &nested_list.node_type
        {
            render_list(
                ui,
                nested_list,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level + 1,
                nested_type,
            );
        }
    }
}

/// Render a thematic break (horizontal rule).
fn render_thematic_break(ui: &mut Ui, colors: &EditorColors) {
    ui.add_space(4.0);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, colors.hr);
    ui.add_space(4.0);
}

/// Render a table as an editable widget.
fn render_table(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
) {
    // Create a unique ID for this table based on its position
    let table_id = ui.id().with("table").with(node.start_line);

    // Convert EditorColors to WidgetColors for the table widget
    let widget_colors = WidgetColors {
        text: colors.text,
        heading: colors.heading,
        code_bg: colors.code_bg,
        list_marker: colors.list_marker,
        muted: colors.quote_text,
    };

    // Store the table data in egui's memory so it persists across frames
    let mut table_data = ui.memory_mut(|mem| {
        mem.data
            .get_temp_mut_or_insert_with(table_id.with("data"), || TableData::from_node(node))
            .clone()
    });

    // Create and show the editable table widget
    let output = EditableTable::new(&mut table_data)
        .font_size(font_size)
        .colors(widget_colors)
        .with_controls(true)
        .with_alignment_controls(true)
        .id(table_id)
        .show(ui);

    // Update stored data if changed
    if output.changed {
        // Update the source with the new markdown
        update_table_in_source(source, node.start_line, node.end_line, &output.markdown);

        // Update the stored table data
        ui.memory_mut(|mem| {
            mem.data.insert_temp(table_id.with("data"), table_data);
        });

        // Mark that something was modified
        let node_id = edit_state.add_node(output.markdown.clone(), node.start_line, node.end_line);
        if let Some(editable) = edit_state.get_node_mut(node_id) {
            editable.modified = true;
        }

        debug!("Table at line {} modified", node.start_line);
    } else {
        // Still update stored data to keep cell edits
        ui.memory_mut(|mem| {
            mem.data.insert_temp(table_id.with("data"), table_data);
        });
    }
}

/// Update a table in the source markdown.
fn update_table_in_source(
    source: &mut String,
    start_line: usize,
    end_line: usize,
    new_table: &str,
) {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > 0 && start_line <= lines.len() {
        let mut new_lines: Vec<String> = Vec::new();

        // Lines before the table
        for i in 0..(start_line - 1) {
            new_lines.push(lines[i].to_string());
        }

        // The new table content
        for line in new_table.lines() {
            new_lines.push(line.to_string());
        }

        // Lines after the table
        for i in end_line..lines.len() {
            new_lines.push(lines[i].to_string());
        }

        *source = new_lines.join("\n");
    }
}

/// Render front matter (YAML/TOML header).
fn render_front_matter(ui: &mut Ui, colors: &EditorColors, font_size: f32, content: &str) {
    egui::Frame::none()
        .fill(colors.code_bg)
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Front Matter")
                    .color(colors.quote_text)
                    .font(FontId::monospace(font_size * 0.8))
                    .italics(),
            );
            ui.add(
                TextEdit::multiline(&mut content.to_string())
                    .code_editor()
                    .font(FontId::monospace(font_size * 0.9))
                    .text_color(colors.code_text)
                    .frame(false)
                    .desired_width(f32::INFINITY)
                    .interactive(false), // Front matter editing disabled for now
            );
        });
}

/// Render a link as an editable widget with hover menu.
/// Shows a settings icon on hover that opens a popup for editing text/URL.
fn render_link(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    url: &str,
    title: &str,
) {
    let text = node.text_content();

    // Create a stable ID for this link using position info
    let link_id = egui::Id::new(("link", node.start_line, url));

    // Convert EditorColors to WidgetColors for the link widget
    let widget_colors = WidgetColors {
        text: colors.text,
        heading: colors.link, // Use link color as "heading" for the link widget
        code_bg: colors.code_bg,
        list_marker: colors.list_marker,
        muted: colors.quote_text,
    };

    // Get or create link state from egui's memory
    let mut link_state = ui.memory_mut(|mem| {
        mem.data
            .get_temp_mut_or_insert_with(link_id.with("state"), || {
                RenderedLinkState::new(&text, url)
            })
            .clone()
    });

    // Create and show the rendered link widget
    let output = RenderedLinkWidget::new(&mut link_state, title)
        .font_size(font_size)
        .colors(widget_colors)
        .id(link_id)
        .show(ui);

    // Update stored state
    ui.memory_mut(|mem| {
        mem.data.insert_temp(link_id.with("state"), link_state);
    });

    // Handle changes - update the markdown source
    if output.changed {
        // Update the link in the source
        update_link_in_source(
            source,
            node.start_line,
            node.end_line,
            &text,
            url,
            &output.text,
            &output.url,
            title,
            output.is_autolink,
        );

        // Mark that something was modified in edit state
        let node_id = edit_state.add_node(output.markdown.clone(), node.start_line, node.end_line);
        if let Some(editable) = edit_state.get_node_mut(node_id) {
            editable.modified = true;
        }

        debug!(
            "Link at line {} modified: [{}]({}) -> [{}]({}), is_autolink={}",
            node.start_line, text, url, output.text, output.url, output.is_autolink
        );
    }
}

/// Render inline content with accumulated text styles.
/// This handles nested emphasis like ***bold italic*** by propagating styles through children.
fn render_styled_inline(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: EditorFont,
    style: TextStyle,
) {
    // Render all children with the given style
    for child in &node.children {
        render_inline_node(
            ui,
            child,
            source,
            edit_state,
            colors,
            font_size,
            editor_font,
            style,
        );
    }
}
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Source Synchronization
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Format a heading back to markdown.
fn format_heading(text: &str, level: HeadingLevel) -> String {
    let prefix = "#".repeat(level as usize);
    format!("{} {}", prefix, text.trim())
}

/// Update a single line in the source.
fn update_source_line(source: &mut String, line: usize, new_content: &str) {
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let line_count = lines.len();
    if line > 0 && line <= line_count {
        let mut new_lines = lines;
        new_lines[line - 1] = new_content.to_string();
        *source = new_lines.join("\n");
    }
}

/// Extract the prefix from a markdown line (list marker, indentation, etc.)
/// Returns the prefix and the content separately.
fn extract_line_prefix(line: &str) -> (&str, &str) {
    // Match patterns like:
    // - "  - " (indented bullet)
    // - "- " (bullet)
    // - "* " (bullet)
    // - "1. " (ordered)
    // - "  1. " (indented ordered)
    // - "- [ ] " (task unchecked)
    // - "- [x] " (task checked)
    // - "> " (blockquote)

    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();

    // Check for list markers
    if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        let prefix_len = indent_len + 6; // "- [x] "
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        let prefix_len = indent_len + 6; // "- [ ] "
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("- ") {
        let prefix_len = indent_len + 2; // "- "
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        let prefix_len = indent_len + 2; // "* "
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        let prefix_len = indent_len + 2; // "+ "
        return (&line[..prefix_len], rest);
    }
    if let Some(rest) = trimmed.strip_prefix("> ") {
        let prefix_len = indent_len + 2; // "> "
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
        let prefix_len = indent_len + i + 2; // digits + delimiter + space
        if prefix_len <= line.len() {
            return (&line[..prefix_len], &line[prefix_len..]);
        }
    }

    // No special prefix found
    ("", line)
}

/// Update a range of lines in the source, preserving list markers and prefixes.
fn update_source_range(source: &mut String, start_line: usize, end_line: usize, new_content: &str) {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > 0 && start_line <= lines.len() {
        let mut new_lines: Vec<String> = Vec::new();

        // Lines before the range
        for i in 0..(start_line - 1) {
            if i < lines.len() {
                new_lines.push(lines[i].to_string());
            }
        }

        // Get the prefix from the original first line (to preserve list markers)
        let original_first_line = lines.get(start_line - 1).unwrap_or(&"");
        let (prefix, _) = extract_line_prefix(original_first_line);

        // The new content - first line gets the original prefix
        let content_lines: Vec<&str> = new_content.lines().collect();
        for (idx, content_line) in content_lines.iter().enumerate() {
            if idx == 0 && !prefix.is_empty() {
                // First line: preserve the original prefix
                new_lines.push(format!("{}{}", prefix, content_line));
            } else if idx > 0 && !prefix.is_empty() {
                // Continuation lines: preserve indentation but no marker
                let indent = prefix
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();
                let marker_indent = "  "; // Standard continuation indent
                new_lines.push(format!("{}{}{}", indent, marker_indent, content_line));
            } else {
                new_lines.push(content_line.to_string());
            }
        }

        // Handle empty content case
        if content_lines.is_empty() && !prefix.is_empty() {
            new_lines.push(prefix.to_string());
        }

        // Lines after the range
        for i in end_line..lines.len() {
            new_lines.push(lines[i].to_string());
        }

        *source = new_lines.join("\n");
    }
}

/// Update a code block in the source.
fn update_code_block(
    source: &mut String,
    start_line: usize,
    end_line: usize,
    language: &str,
    new_content: &str,
) {
    let lines: Vec<&str> = source.lines().collect();
    if start_line > 0 && end_line <= lines.len() {
        let mut new_lines: Vec<String> = Vec::new();

        // Lines before the code block
        for i in 0..(start_line - 1) {
            new_lines.push(lines[i].to_string());
        }

        // The code block
        new_lines.push(format!("```{}", language));
        for content_line in new_content.lines() {
            new_lines.push(content_line.to_string());
        }
        new_lines.push("```".to_string());

        // Lines after the code block
        for i in end_line..lines.len() {
            new_lines.push(lines[i].to_string());
        }

        *source = new_lines.join("\n");
    }
}

/// Update a link in the source markdown.
/// Finds and replaces the old link syntax with the new text and URL.
fn update_link_in_source(
    source: &mut String,
    start_line: usize,
    end_line: usize,
    old_text: &str,
    old_url: &str,
    new_text: &str,
    new_url: &str,
    title: &str,
    is_autolink: bool,
) {
    let lines: Vec<&str> = source.lines().collect();

    // Handle both 0-indexed and 1-indexed line numbers from the parser
    // If start_line is 0, treat it as line 1 (first line)
    let effective_start = if start_line == 0 { 1 } else { start_line };
    let effective_end = if end_line == 0 { 1 } else { end_line };

    if effective_start > 0 && effective_start <= lines.len() {
        let mut new_lines: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1; // 1-indexed

            if line_num >= effective_start && line_num <= effective_end {
                let modified_line = if is_autolink {
                    // Autolink: just replace the bare URL with new URL (no markdown injection)
                    // This keeps the source clean and doesn't add [text](url) syntax
                    if line.contains(old_url) {
                        line.replace(old_url, new_url)
                    } else {
                        line.to_string()
                    }
                } else {
                    // Regular markdown link syntax
                    // Build the new link
                    let new_link = if title.is_empty() {
                        format!("[{}]({})", new_text, new_url)
                    } else {
                        format!("[{}]({} \"{}\")", new_text, new_url, title)
                    };

                    // Build the old link pattern (could have title or not)
                    let old_link_with_title = format!("[{}]({} \"", old_text, old_url);
                    let old_link_simple = format!("[{}]({})", old_text, old_url);

                    // Try to replace the link
                    if line.contains(&old_link_with_title) {
                        // Has title - need to match the full pattern
                        // Find the end of the title
                        if let Some(start_idx) = line.find(&old_link_with_title) {
                            let after_title_start = start_idx + old_link_with_title.len();
                            if let Some(end_quote_idx) = line[after_title_start..].find("\"") {
                                let end_paren_idx = after_title_start + end_quote_idx + 1;
                                if end_paren_idx < line.len()
                                    && line.chars().nth(end_paren_idx + 1) == Some(')')
                                {
                                    // Found complete link with title
                                    let old_full = &line[start_idx..=end_paren_idx + 1];
                                    line.replace(old_full, &new_link)
                                } else {
                                    line.replace(&old_link_simple, &new_link)
                                }
                            } else {
                                line.replace(&old_link_simple, &new_link)
                            }
                        } else {
                            line.replace(&old_link_simple, &new_link)
                        }
                    } else if line.contains(&old_link_simple) {
                        line.replace(&old_link_simple, &new_link)
                    } else {
                        // Fallback: try partial match on just the URL (for edge cases)
                        let url_pattern = format!("]({})", old_url);
                        let new_url_pattern = format!("]({})", new_url);
                        if line.contains(&url_pattern) && old_text == new_text {
                            // Only URL changed
                            line.replace(&url_pattern, &new_url_pattern)
                        } else if line.contains(old_text) && line.contains(old_url) {
                            // Both present but different format - try more aggressive replacement
                            let text_pattern = format!("[{}]", old_text);
                            let new_text_pattern = format!("[{}]", new_text);
                            line.replace(&text_pattern, &new_text_pattern)
                                .replace(old_url, new_url)
                        } else {
                            line.to_string()
                        }
                    }
                };

                new_lines.push(modified_line);
            } else {
                new_lines.push(line.to_string());
            }
        }

        *source = new_lines.join("\n");
    }
}

/// Rebuild the markdown source from modified nodes.
fn rebuild_markdown(_source: &mut String, edit_state: &EditState, _original: &str) {
    // For now, rely on individual node updates.
    // More sophisticated rebuilding would track all modifications
    // and rebuild the entire document if needed.

    // This function is called after individual updates have been applied,
    // so we just log that a rebuild was triggered.
    debug!(
        "Markdown rebuild completed with {} nodes",
        edit_state.nodes.len()
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Utility Functions
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Convert a character index to (line, column) position.
fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in text.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Convert a line number (1-indexed) to character index.
fn line_to_char_index(text: &str, target_line: usize) -> usize {
    if target_line <= 1 {
        return 0;
    }

    let mut current_line = 1;
    for (i, ch) in text.chars().enumerate() {
        if ch == '\n' {
            current_line += 1;
            if current_line >= target_line {
                return i + 1;
            }
        }
    }

    text.len()
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Tests
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // EditorMode Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_editor_mode_default() {
        let mode = EditorMode::default();
        assert_eq!(mode, EditorMode::Raw);
    }

    #[test]
    fn test_editor_mode_equality() {
        assert_eq!(EditorMode::Raw, EditorMode::Raw);
        assert_eq!(EditorMode::Rendered, EditorMode::Rendered);
        assert_ne!(EditorMode::Raw, EditorMode::Rendered);
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // EditorColors Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_dark_theme_colors() {
        let colors = EditorColors::dark();
        assert!(colors.background.r() < 50); // Dark background
        assert!(colors.text.r() > 200); // Light text
    }

    #[test]
    fn test_light_theme_colors() {
        let colors = EditorColors::light();
        assert!(colors.background.r() > 200); // Light background
        assert!(colors.text.r() < 50); // Dark text
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // EditState Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_edit_state_new() {
        let state = EditState::new();
        assert!(state.nodes.is_empty());
        assert_eq!(state.next_id, 0);
    }

    #[test]
    fn test_edit_state_add_node() {
        let mut state = EditState::new();
        let id = state.add_node("test".to_string(), 1, 1);
        assert_eq!(id, 0);
        assert_eq!(state.nodes.len(), 1);
        assert_eq!(state.next_id, 1);
    }

    #[test]
    fn test_edit_state_get_node_mut() {
        let mut state = EditState::new();
        let id = state.add_node("test".to_string(), 1, 1);

        let node = state.get_node_mut(id);
        assert!(node.is_some());
        assert_eq!(node.unwrap().text, "test");
    }

    #[test]
    fn test_edit_state_any_modified() {
        let mut state = EditState::new();
        state.add_node("test".to_string(), 1, 1);
        assert!(!state.any_modified());

        if let Some(node) = state.get_node_mut(0) {
            node.modified = true;
        }
        assert!(state.any_modified());
    }

    #[test]
    fn test_edit_state_clear() {
        let mut state = EditState::new();
        state.add_node("test".to_string(), 1, 1);
        state.clear();

        assert!(state.nodes.is_empty());
        assert_eq!(state.next_id, 0);
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // TextStyle Tests (for nested emphasis support)
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_text_style_default() {
        let style = TextStyle::new();
        assert!(!style.bold);
        assert!(!style.italic);
        assert!(!style.strikethrough);
    }

    #[test]
    fn test_text_style_with_bold() {
        let style = TextStyle::new().with_bold();
        assert!(style.bold);
        assert!(!style.italic);
        assert!(!style.strikethrough);
    }

    #[test]
    fn test_text_style_with_italic() {
        let style = TextStyle::new().with_italic();
        assert!(!style.bold);
        assert!(style.italic);
        assert!(!style.strikethrough);
    }

    #[test]
    fn test_text_style_with_strikethrough() {
        let style = TextStyle::new().with_strikethrough();
        assert!(!style.bold);
        assert!(!style.italic);
        assert!(style.strikethrough);
    }

    #[test]
    fn test_text_style_bold_and_italic() {
        // Simulates ***bold and italic*** or **_text_**
        let style = TextStyle::new().with_bold().with_italic();
        assert!(style.bold);
        assert!(style.italic);
        assert!(!style.strikethrough);
    }

    #[test]
    fn test_text_style_all_combined() {
        // All three styles combined
        let style = TextStyle::new()
            .with_bold()
            .with_italic()
            .with_strikethrough();
        assert!(style.bold);
        assert!(style.italic);
        assert!(style.strikethrough);
    }

    #[test]
    fn test_text_style_chaining_order_independent() {
        // Order shouldn't matter
        let style1 = TextStyle::new().with_bold().with_italic();
        let style2 = TextStyle::new().with_italic().with_bold();

        assert_eq!(style1.bold, style2.bold);
        assert_eq!(style1.italic, style2.italic);
    }

    #[test]
    fn test_text_style_apply_no_style() {
        let style = TextStyle::new();
        let text = RichText::new("test");
        let _styled = style.apply(text, 14.0, EditorFont::Inter);
        // Just verify it doesn't panic; visual styling tested via egui
    }

    #[test]
    fn test_text_style_apply_with_styles() {
        let style = TextStyle::new().with_bold().with_italic();
        let text = RichText::new("test");
        let _styled = style.apply(text, 14.0, EditorFont::Inter);
        // Just verify it doesn't panic; visual styling tested via egui
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Format Heading Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Source Update Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_update_source_line() {
        let mut source = "Line 1\nLine 2\nLine 3".to_string();
        update_source_line(&mut source, 2, "Modified Line 2");
        assert_eq!(source, "Line 1\nModified Line 2\nLine 3");
    }

    #[test]
    fn test_update_source_line_first() {
        let mut source = "First\nSecond".to_string();
        update_source_line(&mut source, 1, "New First");
        assert_eq!(source, "New First\nSecond");
    }

    #[test]
    fn test_update_source_range() {
        let mut source = "Line 1\nLine 2\nLine 3\nLine 4".to_string();
        update_source_range(&mut source, 2, 3, "New Content");
        assert_eq!(source, "Line 1\nNew Content\nLine 4");
    }

    #[test]
    fn test_update_source_range_preserves_bullet_list() {
        let mut source = "# Header\n- Item 1\n- Item 2".to_string();
        update_source_range(&mut source, 2, 2, "Modified Item");
        assert_eq!(source, "# Header\n- Modified Item\n- Item 2");
    }

    #[test]
    fn test_update_source_range_preserves_ordered_list() {
        let mut source = "# Header\n1. First\n2. Second".to_string();
        update_source_range(&mut source, 2, 2, "Modified First");
        assert_eq!(source, "# Header\n1. Modified First\n2. Second");
    }

    #[test]
    fn test_extract_line_prefix_bullet() {
        let (prefix, content) = extract_line_prefix("- Item text");
        assert_eq!(prefix, "- ");
        assert_eq!(content, "Item text");
    }

    #[test]
    fn test_extract_line_prefix_ordered() {
        let (prefix, content) = extract_line_prefix("1. First item");
        assert_eq!(prefix, "1. ");
        assert_eq!(content, "First item");
    }

    #[test]
    fn test_extract_line_prefix_indented_bullet() {
        let (prefix, content) = extract_line_prefix("  - Nested item");
        assert_eq!(prefix, "  - ");
        assert_eq!(content, "Nested item");
    }

    #[test]
    fn test_extract_line_prefix_task_unchecked() {
        let (prefix, content) = extract_line_prefix("- [ ] Todo item");
        assert_eq!(prefix, "- [ ] ");
        assert_eq!(content, "Todo item");
    }

    #[test]
    fn test_extract_line_prefix_task_checked() {
        let (prefix, content) = extract_line_prefix("- [x] Done item");
        assert_eq!(prefix, "- [x] ");
        assert_eq!(content, "Done item");
    }

    #[test]
    fn test_extract_line_prefix_no_prefix() {
        let (prefix, content) = extract_line_prefix("Regular paragraph");
        assert_eq!(prefix, "");
        assert_eq!(content, "Regular paragraph");
    }

    #[test]
    fn test_extract_line_prefix_blockquote() {
        let (prefix, content) = extract_line_prefix("> Quoted text");
        assert_eq!(prefix, "> ");
        assert_eq!(content, "Quoted text");
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Char Index Conversion Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_char_index_to_line_col_empty() {
        assert_eq!(char_index_to_line_col("", 0), (0, 0));
    }

    #[test]
    fn test_char_index_to_line_col_single_line() {
        let text = "Hello";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0));
        assert_eq!(char_index_to_line_col(text, 3), (0, 3));
    }

    #[test]
    fn test_char_index_to_line_col_multiline() {
        let text = "Hello\nWorld";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0));
        assert_eq!(char_index_to_line_col(text, 5), (0, 5));
        assert_eq!(char_index_to_line_col(text, 6), (1, 0));
        assert_eq!(char_index_to_line_col(text, 8), (1, 2));
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // MarkdownEditor Builder Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_markdown_editor_builder() {
        let mut content = "# Test".to_string();
        let editor = MarkdownEditor::new(&mut content)
            .mode(EditorMode::Rendered)
            .font_size(16.0)
            .word_wrap(false)
            .theme(Theme::Dark);

        assert_eq!(editor.mode, EditorMode::Rendered);
        assert_eq!(editor.font_size, 16.0);
        assert!(!editor.word_wrap);
        assert_eq!(editor.theme, Theme::Dark);
    }

    #[test]
    fn test_markdown_editor_default_values() {
        let mut content = String::new();
        let editor = MarkdownEditor::new(&mut content);

        assert_eq!(editor.mode, EditorMode::Raw);
        assert_eq!(editor.font_size, 14.0);
        assert!(editor.word_wrap);
        assert_eq!(editor.theme, Theme::Light);
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Link Update Tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_update_link_in_source_simple() {
        let mut source = "Check out [Example](https://example.com) for more.".to_string();
        update_link_in_source(
            &mut source,
            1,
            1,
            "Example",
            "https://example.com",
            "New Text",
            "https://new-url.com",
            "",
            false, // not an autolink
        );
        assert_eq!(
            source,
            "Check out [New Text](https://new-url.com) for more."
        );
    }

    #[test]
    fn test_update_link_in_source_text_only() {
        let mut source = "Click [here](https://example.com) now.".to_string();
        update_link_in_source(
            &mut source,
            1,
            1,
            "here",
            "https://example.com",
            "this link",
            "https://example.com",
            "",
            false,
        );
        assert_eq!(source, "Click [this link](https://example.com) now.");
    }

    #[test]
    fn test_update_link_in_source_url_only() {
        let mut source = "Visit [Google](https://google.com) today.".to_string();
        update_link_in_source(
            &mut source,
            1,
            1,
            "Google",
            "https://google.com",
            "Google",
            "https://www.google.com",
            "",
            false,
        );
        assert_eq!(source, "Visit [Google](https://www.google.com) today.");
    }

    #[test]
    fn test_update_link_in_source_multiline() {
        let mut source = "Line 1\n[Link](https://url.com)\nLine 3".to_string();
        update_link_in_source(
            &mut source,
            2,
            2,
            "Link",
            "https://url.com",
            "Updated",
            "https://new.com",
            "",
            false,
        );
        assert_eq!(source, "Line 1\n[Updated](https://new.com)\nLine 3");
    }

    #[test]
    fn test_update_link_in_source_preserves_other_lines() {
        let mut source = "# Header\n\n[Old Link](https://old.com)\n\nParagraph text.".to_string();
        update_link_in_source(
            &mut source,
            3,
            3,
            "Old Link",
            "https://old.com",
            "New Link",
            "https://new.com",
            "",
            false,
        );
        assert_eq!(
            source,
            "# Header\n\n[New Link](https://new.com)\n\nParagraph text."
        );
    }

    #[test]
    fn test_update_link_in_source_multiple_links_same_line() {
        let mut source = "See [A](https://a.com) and [B](https://b.com) here.".to_string();
        // Update only the first link
        update_link_in_source(
            &mut source,
            1,
            1,
            "A",
            "https://a.com",
            "Alpha",
            "https://alpha.com",
            "",
            false,
        );
        assert!(source.contains("[Alpha](https://alpha.com)"));
        assert!(source.contains("[B](https://b.com)")); // B unchanged
    }

    #[test]
    fn test_update_link_in_source_autolink_url_change() {
        // Autolink: bare URL in source - only URL can be edited
        // This should just replace the URL, not inject markdown syntax
        let mut source = "Check out https://example.com for more info.".to_string();
        update_link_in_source(
            &mut source,
            1,
            1,
            "https://example.com",
            "https://example.com",
            "https://new-example.com", // text is ignored for autolinks
            "https://new-example.com",
            "",
            true, // IS an autolink
        );
        // Should just replace the URL, not inject [text](url) syntax
        assert_eq!(source, "Check out https://new-example.com for more info.");
    }

    #[test]
    fn test_update_link_in_source_autolink_preserves_format() {
        // Autolink should never inject markdown syntax
        let mut source = "Visit https://old-url.com today.".to_string();
        update_link_in_source(
            &mut source,
            1,
            1,
            "https://old-url.com",
            "https://old-url.com",
            "https://new-url.com",
            "https://new-url.com",
            "",
            true,
        );
        // Just URL replaced, no markdown syntax added
        assert_eq!(source, "Visit https://new-url.com today.");
    }
}
