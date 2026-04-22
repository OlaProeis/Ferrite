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

use crate::config::{EditorFont, HeaderSpacing, MaxLineWidth, ParagraphIndent, Settings, Theme};
use crate::fonts;
use crate::markdown::ast_ops::{
    exit_list_to_paragraph, heading_enter, indent_list_item, merge_with_previous_list_item,
    outdent_list_item, split_list_item, split_paragraph, EditContext, EditNodeType, StructuralEdit,
};
use crate::markdown::cache;
use crate::markdown::parser::{
    CalloutType, HeadingLevel, ListType, MarkdownNode, MarkdownNodeType,
};
use crate::markdown::widgets::{
    CodeBlockData, EditableCodeBlock, EditableTable, MermaidBlock, MermaidBlockData,
    RenderedLinkState, RenderedLinkWidget, TableData, TableEditState, WidgetColors,
};
use crate::ui::{render_nav_buttons, NavAction};
use eframe::egui::{
    self, Color32, ColorImage, FontId, Key, Response, RichText, ScrollArea, TextEdit,
    TextureHandle, TextureOptions, Ui, Vec2,
};
use log::{debug, warn};
use std::io::Read;
use std::path::{Path, PathBuf};

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
    /// Total content height inside the scroll area (for sync scrolling)
    pub content_height: f32,
    /// Viewport height of the scroll area (for sync scrolling)
    pub viewport_height: f32,
    /// Line-to-Y mappings for rendered mode (source_line -> rendered_y)
    /// Used for accurate scroll sync between Raw and Rendered modes
    pub line_mappings: Vec<LineMapping>,
    /// Wikilink target that was clicked (for navigation).
    /// When set, the caller should resolve this target to a file path and open it.
    pub wikilink_clicked: Option<String>,
}

/// Maps a source line range to a rendered Y position range.
/// Used for scroll synchronization between Raw and Rendered views.
#[derive(Debug, Clone, Default)]
pub struct LineMapping {
    /// Start line in source (1-indexed)
    pub start_line: usize,
    /// End line in source (1-indexed)  
    pub end_line: usize,
    /// Y position where this element starts in rendered view
    pub rendered_y: f32,
    /// Height of this element in rendered view (pixels)
    pub rendered_height: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Viewport Culling
// ─────────────────────────────────────────────────────────────────────────────

/// Extra pixels above and below the viewport to pre-render, avoiding pop-in
/// during fast scrolling.
const VIEWPORT_OVERSCAN_PX: f32 = 500.0;

/// Spacing between rendered blocks (must match the `item_spacing.y` set during layout).
const BLOCK_ITEM_SPACING_Y: f32 = 1.0;

/// Extra vertical space after block-level paragraphs (and code blocks) so consecutive
/// paragraphs are visibly separated; ~0.5em at 32px line height. Included in viewport
/// block height measurements via `render_node` layout (not added to `BLOCK_ITEM_SPACING_Y`).
const PARAGRAPH_TRAILING_SPACE_Y: f32 = 16.0;

/// Max blocks to newly measure (via full egui render) per frame during the
/// progressive measurement pass.  Keeps first-frame cost bounded for large
/// documents (10K+ blocks) while the scroll position self-corrects.
const MAX_NEW_MEASUREMENTS_PER_FRAME: usize = 20;

/// Baseline pixels-per-line used for heuristic height estimates when a block
/// has never been rendered.  Roughly matches a 14 px body font with default
/// line spacing.
const ESTIMATED_LINE_HEIGHT_PX: f32 = 20.0;

/// Cached block positions for the rendered view, stored in egui temp memory.
/// Invalidated when content or available width changes.
#[derive(Clone)]
struct ViewportCullingState {
    content_hash: u64,
    available_width: f32,
    /// Y offset where each block starts (includes inter-block spacing).
    block_start_y: Vec<f32>,
    /// Rendered height of each top-level block (excludes `BLOCK_ITEM_SPACING_Y` between
    /// blocks; includes in-flow spacing such as [`PARAGRAPH_TRAILING_SPACE_Y`] after
    /// paragraphs and code blocks).
    block_heights: Vec<f32>,
    /// Total content height (blocks + spacing), measured from the layout.
    total_height: f32,
    /// Per-block flag: `true` = height was obtained from a real egui render or
    /// the block-height cache; `false` = heuristic estimate only.
    block_measured: Vec<bool>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Source Extraction (for per-block height caching)
// ─────────────────────────────────────────────────────────────────────────────

/// Byte offset of the start of each line.
/// `offsets[0]` = byte start of line 1, `offsets[1]` = byte start of line 2, etc.
fn line_start_byte_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, &b) in content.as_bytes().iter().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Extract the source text for a block spanning `start_line..=end_line` (1-indexed).
fn block_source_slice<'a>(
    content: &'a str,
    offsets: &[usize],
    start_line: usize,
    end_line: usize,
) -> &'a str {
    if start_line == 0 || end_line == 0 || offsets.is_empty() {
        return "";
    }
    let start = offsets
        .get(start_line.saturating_sub(1))
        .copied()
        .unwrap_or(0);
    let end = offsets.get(end_line).copied().unwrap_or(content.len());
    &content[start..end.min(content.len())]
}

/// Heuristic height for a block that has never been rendered.
/// Uses `(end_line - start_line + 1) * ESTIMATED_LINE_HEIGHT_PX` as baseline,
/// with a small per-block minimum to avoid zero-height placeholders.
fn estimate_block_height(start_line: usize, end_line: usize) -> f32 {
    let lines = if end_line >= start_line {
        (end_line - start_line + 1) as f32
    } else {
        1.0
    };
    (lines * ESTIMATED_LINE_HEIGHT_PX).max(ESTIMATED_LINE_HEIGHT_PX)
}

// ─────────────────────────────────────────────────────────────────────────────
// Rendered View Search Highlight Overlay
// ─────────────────────────────────────────────────────────────────────────────

/// Converts a byte position in the source to a 1-indexed line number.
fn byte_pos_to_line_1indexed(line_offsets: &[usize], byte_pos: usize) -> usize {
    match line_offsets.binary_search(&byte_pos) {
        Ok(idx) => idx + 1,
        Err(idx) => idx, // idx is the line whose start is > byte_pos, so line = idx
    }
}

/// Paints search highlight overlays for matches that fall within a rendered block.
///
/// For table blocks, highlights are subdivided into per-row strips.
/// For other blocks (paragraphs, headings), a full-width strip is painted.
#[allow(clippy::too_many_arguments)]
fn paint_rendered_search_highlights(
    ui: &Ui,
    search_highlights: &[(usize, usize)],
    current_match: usize,
    content: &str,
    line_offsets: &[usize],
    node_start_line: usize,
    node_end_line: usize,
    block_y_top: f32,
    block_y_bottom: f32,
    block_left: f32,
    block_right: f32,
    is_table: bool,
    is_dark: bool,
) {
    if search_highlights.is_empty() || node_start_line == 0 {
        return;
    }

    let block_start_byte = line_offsets
        .get(node_start_line.saturating_sub(1))
        .copied()
        .unwrap_or(0);
    let block_end_byte = line_offsets
        .get(node_end_line)
        .copied()
        .unwrap_or(content.len());

    let current_match_color = if is_dark {
        Color32::from_rgba_unmultiplied(255, 200, 0, 100)
    } else {
        Color32::from_rgba_unmultiplied(255, 220, 0, 120)
    };
    let other_match_color = if is_dark {
        Color32::from_rgba_unmultiplied(180, 150, 50, 60)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 100, 80)
    };

    let block_height = block_y_bottom - block_y_top;
    let total_source_lines = node_end_line.saturating_sub(node_start_line) + 1;
    if block_height <= 0.0 || total_source_lines == 0 {
        return;
    }

    let painter = ui.painter();

    for (idx, &(match_start, match_end)) in search_highlights.iter().enumerate() {
        if match_end <= block_start_byte || match_start >= block_end_byte {
            continue;
        }

        let color = if idx == current_match {
            current_match_color
        } else {
            other_match_color
        };

        let match_line = byte_pos_to_line_1indexed(line_offsets, match_start);

        if is_table {
            // Table: subdivide into rows. Each source line = one visual row,
            // except the separator line (start_line + 1) is collapsed.
            let table_lines = total_source_lines;
            let visual_rows = if table_lines > 2 {
                table_lines - 1 // header + data rows (separator merged)
            } else {
                table_lines.max(1)
            };
            let row_height = block_height / visual_rows as f32;

            let offset_from_start = match_line.saturating_sub(node_start_line);
            let visual_row = if offset_from_start == 0 {
                0 // header
            } else if offset_from_start == 1 {
                continue; // separator line — skip
            } else {
                offset_from_start - 1 // data rows shifted by 1 (separator removed)
            };

            let row_y = block_y_top + visual_row as f32 * row_height;
            let highlight_rect = egui::Rect::from_min_max(
                egui::Pos2::new(block_left, row_y),
                egui::Pos2::new(block_right, row_y + row_height),
            );
            painter.rect_filled(highlight_rect, 2.0, color);
        } else {
            // Non-table block: paint a proportional strip
            let line_frac =
                (match_line.saturating_sub(node_start_line)) as f32 / total_source_lines as f32;
            let approx_line_height = block_height / total_source_lines as f32;
            let y = block_y_top + line_frac * block_height;
            let highlight_rect = egui::Rect::from_min_max(
                egui::Pos2::new(block_left, y),
                egui::Pos2::new(block_right, (y + approx_line_height).min(block_y_bottom)),
            );
            painter.rect_filled(highlight_rect, 2.0, color);
        }
    }
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
    /// Pending scroll offset to apply (for sync scrolling on mode switch)
    pending_scroll_offset: Option<f32>,
    /// Maximum line width setting for centering text column
    max_line_width: MaxLineWidth,
    /// Whether Zen Mode is enabled (centered text column)
    zen_mode: bool,
    /// Maximum column width in characters for Zen Mode centering
    zen_max_column_width: f32,
    /// CJK paragraph first-line indentation
    paragraph_indent: ParagraphIndent,
    /// Vertical spacing between headers in rendered view
    header_spacing: HeaderSpacing,
    /// File context for wikilink resolution (current file dir + workspace root)
    wikilink_context: Option<WikilinkContext>,
    /// Treat soft breaks as hard line breaks in rendered view
    strict_line_breaks: bool,
    /// Search match byte ranges for overlay highlighting in rendered view
    search_highlights: Option<Vec<(usize, usize)>>,
    /// Index of the currently focused search match
    current_search_match: usize,
    /// Whether to render external web images
    render_web_images: bool,
}

/// Context for resolving wikilinks to actual files during rendering.
/// Stored in egui memory per-frame so `render_wikilink` can check file existence.
#[derive(Debug, Clone)]
pub struct WikilinkContext {
    /// Directory of the currently open file (for relative resolution)
    pub current_dir: Option<PathBuf>,
    /// Workspace root (for workspace-wide resolution)
    pub workspace_root: Option<PathBuf>,
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
            pending_scroll_offset: None,
            max_line_width: MaxLineWidth::Off,
            zen_mode: false,
            zen_max_column_width: 80.0,
            paragraph_indent: ParagraphIndent::Off,
            header_spacing: HeaderSpacing::default(),
            wikilink_context: None,
            strict_line_breaks: false,
            search_highlights: None,
            current_search_match: 0,
            render_web_images: false,
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

    /// Set a pending scroll offset to apply (for sync scrolling on mode switch).
    #[must_use]
    pub fn pending_scroll_offset(mut self, offset: Option<f32>) -> Self {
        self.pending_scroll_offset = offset;
        self
    }

    /// Set the maximum line width for text centering.
    ///
    /// When enabled and the viewport is wider than the specified width,
    /// text is constrained to that width and centered horizontally.
    #[must_use]
    pub fn max_line_width(mut self, width: MaxLineWidth) -> Self {
        self.max_line_width = width;
        self
    }

    /// Enable Zen Mode with centered text column.
    ///
    /// When enabled, the text content is centered horizontally with a maximum
    /// column width (in characters), while the editor background fills the available space.
    /// Zen Mode takes priority over max_line_width setting.
    #[must_use]
    pub fn zen_mode(mut self, enabled: bool, max_column_width: f32) -> Self {
        self.zen_mode = enabled;
        self.zen_max_column_width = max_column_width;
        self
    }

    /// Set the CJK paragraph first-line indentation.
    ///
    /// When enabled, paragraphs in rendered view will have first-line indentation
    /// following Chinese (2em) or Japanese (1em) typography conventions.
    #[must_use]
    pub fn paragraph_indent(mut self, indent: ParagraphIndent) -> Self {
        self.paragraph_indent = indent;
        self
    }

    /// Set the vertical spacing between headers in rendered view.
    #[must_use]
    pub fn header_spacing(mut self, spacing: HeaderSpacing) -> Self {
        self.header_spacing = spacing;
        self
    }

    /// Set the wikilink resolution context (current file directory and workspace root).
    ///
    /// When provided, wikilinks that cannot be resolved to existing files are
    /// rendered with a distinct "broken link" visual style.
    #[must_use]
    pub fn wikilink_context(mut self, ctx: WikilinkContext) -> Self {
        self.wikilink_context = Some(ctx);
        self
    }

    /// Set whether to render external web images.
    #[must_use]
    pub fn render_web_images(mut self, enabled: bool) -> Self {
        self.render_web_images = enabled;
        self
    }

    /// Set strict line breaks mode.
    ///
    /// When enabled, soft breaks (single newlines) in markdown source are
    /// rendered as hard line breaks instead of being collapsed to spaces.
    #[must_use]
    pub fn strict_line_breaks(mut self, enabled: bool) -> Self {
        self.strict_line_breaks = enabled;
        self
    }

    /// Set search highlights to render as overlays in the rendered view.
    #[must_use]
    pub fn search_highlights(mut self, matches: Vec<(usize, usize)>, current: usize) -> Self {
        self.search_highlights = Some(matches);
        self.current_search_match = current;
        self
    }

    /// Apply settings to the editor widget.
    #[must_use]
    pub fn with_settings(mut self, settings: &Settings) -> Self {
        self.font_size = settings.font_size;
        self.font_family = settings.font_family.clone();
        self.word_wrap = settings.word_wrap;
        self.theme = settings.theme;
        self.max_line_width = settings.max_line_width;
        self.paragraph_indent = settings.paragraph_indent;
        self.strict_line_breaks = settings.strict_line_breaks;
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
        let font_size = self.font_size;
        let word_wrap = self.word_wrap;
        let editor_font = self.font_family.clone();

        let font_family = fonts::get_styled_font_family(false, false, &editor_font);

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
        let changed = text_output.response.changed();

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
            content_height: scroll_output.content_size.y,
            viewport_height: scroll_output.inner_rect.height(),
            line_mappings: Vec::new(), // Raw mode doesn't need line mappings
            wikilink_clicked: None,    // Raw mode doesn't have clickable wikilinks
        }
    }

    /// Show the WYSIWYG rendered editor.
    fn show_rendered_editor(
        self,
        ui: &mut Ui,
        id: egui::Id,
        colors: &EditorColors,
    ) -> MarkdownEditorOutput {
        let mut edit_state = EditState::new();
        let mut structural_state = StructuralEditState::new();

        // Clear the link click consumed flag at start of each frame
        // This prevents stale flags from previous frames affecting edit mode entry
        ui.memory_mut(|mem| {
            mem.data
                .remove::<bool>(egui::Id::new("link_click_consumed_this_frame"));
        });

        // Store wikilink resolution context in egui memory so render_wikilink can access it
        if let Some(ctx) = &self.wikilink_context {
            ui.memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new("wikilink_resolution_context"), ctx.clone());
            });
        }

        // Store strict line breaks flag in egui memory for render_inline_node
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(egui::Id::new("strict_line_breaks"), self.strict_line_breaks);
        });

        // Store render web images flag in egui memory for render_image
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(egui::Id::new("render_web_images"), self.render_web_images);
        });

        // Parse the markdown content (cached by blake3 hash — skips re-parse when unchanged)
        let doc = match cache::get_or_parse(self.content) {
            Ok(doc) => doc,
            Err(e) => {
                ui.colored_label(Color32::RED, format!("Parse error: {}", e));
                return self.show_raw_editor(ui, id);
            }
        };

        // DEBUG: Document structure logging removed - was too verbose (every frame)
        // Enable manually if needed for debugging:
        // debug!("[LIST_DEBUG] Document has {} top-level nodes", doc.root.children.len());

        // Calculate scroll offset for outline navigation if needed
        // Uses same calculation as Raw mode for consistency:
        // - 1-indexed line input, converted to 0-indexed
        // - Position at 1/4 from top (better visibility than 1/3)
        let target_scroll_offset: Option<f32> = if let Some(target_line) = self.scroll_to_line {
            let font_id = FontId::new(
                self.font_size,
                fonts::get_styled_font_family(false, false, &self.font_family),
            );
            let line_height = ui.fonts(|f| f.row_height(&font_id));
            let viewport_height = ui.available_height();
            // Convert 1-indexed to 0-indexed for calculation
            let line_index = target_line.saturating_sub(1);
            let target_y = line_index as f32 * line_height;
            // Position at 1/4 from top for better visibility tolerance
            Some((target_y - viewport_height * 0.25).max(0.0))
        } else {
            None
        };

        // Check for pending navigation scroll from nav buttons (stored in previous frame)
        let nav_scroll_id = id.with("nav_scroll_target");
        let pending_nav_scroll: Option<f32> = ui.memory(|mem| mem.data.get_temp(nav_scroll_id));
        if pending_nav_scroll.is_some() {
            // Clear the pending scroll after reading it
            ui.memory_mut(|mem| {
                mem.data.remove::<f32>(nav_scroll_id);
            });
        }

        // Ctrl+Scroll Zoom: detect before ScrollArea consumes the scroll events
        let ctrl_scroll_zoom: Option<bool> = ui.input(|i| {
            if !i.modifiers.command {
                return None;
            }
            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    if delta.y.abs() > 0.01 {
                        return Some(delta.y > 0.0);
                    }
                }
            }
            None
        });

        if let Some(is_zoom_in) = ctrl_scroll_zoom {
            if is_zoom_in {
                egui::gui_zoom::zoom_in(ui.ctx());
            } else {
                egui::gui_zoom::zoom_out(ui.ctx());
            }
            ui.input_mut(|i| {
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                i.events.retain(|e| {
                    !matches!(e, egui::Event::MouseWheel { modifiers, .. } if modifiers.command)
                });
            });
        }

        // Render the document in a scroll area
        let mut scroll_area = ScrollArea::vertical()
            .id_source(id.with("rendered_scroll"))
            .auto_shrink([false, false]);

        // Priority order for scroll offset:
        // 1. Nav button scroll (from previous frame)
        // 2. Pending scroll offset from mode switch
        // 3. Target scroll offset from outline navigation
        if let Some(offset) = pending_nav_scroll {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
            log::debug!(
                "Applied nav button scroll offset in rendered mode: {}",
                offset
            );
        } else if let Some(offset) = self.pending_scroll_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
            log::debug!("Applied pending scroll offset in rendered mode: {}", offset);
        } else if let Some(offset) = target_scroll_offset {
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

        // Collect line mappings during render for scroll sync
        let mut line_mappings: Vec<LineMapping> = Vec::new();

        // Calculate content width and centering margin
        // Both Zen mode and non-zen mode use max_line_width setting
        // Zen mode: centers content; Non-zen mode: left-aligned
        let char_width = self.font_size * 0.6; // Approximate average character width
        let outer_available_width = ui.available_width();

        let (content_margin, effective_content_width) =
            if let Some(max_width_px) = self.max_line_width.to_pixels(char_width) {
                // max_line_width is set - constrain width
                // Cap to available width to prevent overflow
                let effective_width = max_width_px.min(outer_available_width);

                if self.zen_mode {
                    // Zen mode: center the content
                    let margin = if outer_available_width > effective_width {
                        (outer_available_width - effective_width) / 2.0
                    } else {
                        0.0
                    };
                    (margin, Some(effective_width))
                } else {
                    // Non-zen mode: left-aligned (no margin)
                    (0.0, Some(effective_width))
                }
            } else {
                // No max_line_width set - use full available width, no centering
                (0.0, None)
            };

        // ── Viewport culling state ──────────────────────────────────────────
        let culling_id = id.with("viewport_culling");
        let culling_state: Option<ViewportCullingState> =
            ui.memory(|mem| mem.data.get_temp(culling_id));

        let block_count = doc.root.children.len();
        let has_valid_heights = culling_state.as_ref().map_or(false, |s| {
            s.content_hash == content_hash
                && s.block_heights.len() == block_count
                && (s.available_width - outer_available_width).abs() < 1.0
        });

        // Accumulator for the updated culling state (populated inside the closure).
        let mut new_culling: Option<ViewportCullingState> = None;

        let scroll_output = scroll_area.show_viewport(ui, |ui, viewport| {
            // Tell the scroll area the total content height so the scrollbar
            // range is correct even when most blocks are culled.
            if let Some(ref cs) = culling_state {
                if has_valid_heights {
                    ui.set_min_height(cs.total_height);
                }
            }

            ui.push_id(content_hash, |ui| {
                ui.horizontal(|ui| {
                    if content_margin > 0.0 {
                        ui.add_space(content_margin);
                    }

                    let content_width = effective_content_width.unwrap_or(ui.available_width());
                    ui.vertical(|ui| {
                        ui.set_max_width(content_width);
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, BLOCK_ITEM_SPACING_Y);

                        if has_valid_heights && block_count > 0 {
                            // ── Fast path: cull off-screen blocks ────────────────
                            let cs = culling_state.as_ref().unwrap();

                            let vis_top = (viewport.min.y - VIEWPORT_OVERSCAN_PX).max(0.0);
                            let vis_bottom = viewport.max.y + VIEWPORT_OVERSCAN_PX;

                            let first_vis = cs
                                .block_start_y
                                .partition_point(|&y| y <= vis_top)
                                .saturating_sub(1);
                            let last_vis = cs
                                .block_start_y
                                .partition_point(|&y| y < vis_bottom)
                                .min(block_count)
                                .saturating_sub(1);

                            if first_vis > 0 {
                                let pre =
                                    (cs.block_start_y[first_vis] - BLOCK_ITEM_SPACING_Y).max(0.0);
                                ui.allocate_space(Vec2::new(content_width, pre));
                            }

                            // Track which visible blocks got newly measured so we
                            // can refine the culling state afterwards.
                            let mut updated_heights = cs.block_heights.clone();
                            let mut updated_measured = cs.block_measured.clone();
                            let mut new_measures_this_frame: usize = 0;
                            let line_offsets = line_start_byte_offsets(self.content);
                            let rp_hash = cache::render_params_hash(content_width, self.font_size);

                            let is_dark_mode = ui.visuals().dark_mode;

                            for i in first_vis..=last_vis.min(block_count.saturating_sub(1)) {
                                let node = &doc.root.children[i];
                                let y_before = ui.cursor().top();
                                let block_left = ui.cursor().left();

                                render_node(
                                    ui,
                                    node,
                                    self.content,
                                    &mut edit_state,
                                    colors,
                                    self.font_size,
                                    &self.font_family,
                                    0,
                                    self.paragraph_indent,
                                    self.header_spacing,
                                );

                                let y_after = ui.cursor().top();
                                let height = (y_after - y_before).max(1.0);

                                // Paint search highlight overlays on this block
                                if let Some(ref highlights) = self.search_highlights {
                                    let is_table =
                                        matches!(node.node_type, MarkdownNodeType::Table { .. });
                                    paint_rendered_search_highlights(
                                        ui,
                                        highlights,
                                        self.current_search_match,
                                        self.content,
                                        &line_offsets,
                                        node.start_line,
                                        node.end_line,
                                        y_before,
                                        y_after,
                                        block_left,
                                        block_left + content_width,
                                        is_table,
                                        is_dark_mode,
                                    );
                                }

                                if !updated_measured[i] {
                                    let s = block_source_slice(
                                        self.content,
                                        &line_offsets,
                                        node.start_line,
                                        node.end_line,
                                    );
                                    cache::insert_block_height(s, rp_hash, height);
                                    updated_heights[i] = height;
                                    updated_measured[i] = true;
                                    new_measures_this_frame += 1;
                                } else if (height - updated_heights[i]).abs() > 0.5 {
                                    updated_heights[i] = height;
                                }

                                line_mappings.push(LineMapping {
                                    start_line: node.start_line,
                                    end_line: node.end_line,
                                    rendered_y: cs.block_start_y[i],
                                    rendered_height: updated_heights[i],
                                });
                            }

                            let after_idx = last_vis + 1;
                            if after_idx < block_count {
                                let rendered_end =
                                    cs.block_start_y[last_vis] + updated_heights[last_vis];
                                let post = (cs.total_height - rendered_end - BLOCK_ITEM_SPACING_Y)
                                    .max(0.0);
                                ui.allocate_space(Vec2::new(content_width, post));
                            }

                            for i in 0..first_vis {
                                line_mappings.push(LineMapping {
                                    start_line: doc.root.children[i].start_line,
                                    end_line: doc.root.children[i].end_line,
                                    rendered_y: cs.block_start_y[i],
                                    rendered_height: updated_heights[i],
                                });
                            }
                            for i in (last_vis + 1)..block_count {
                                line_mappings.push(LineMapping {
                                    start_line: doc.root.children[i].start_line,
                                    end_line: doc.root.children[i].end_line,
                                    rendered_y: cs.block_start_y[i],
                                    rendered_height: updated_heights[i],
                                });
                            }

                            // If any heights changed, rebuild start_y and total_height.
                            if new_measures_this_frame > 0 {
                                let mut start_y = Vec::with_capacity(block_count);
                                let mut y = 0.0f32;
                                for (i, &h) in updated_heights.iter().enumerate() {
                                    start_y.push(y);
                                    y += h;
                                    if i + 1 < block_count {
                                        y += BLOCK_ITEM_SPACING_Y;
                                    }
                                }
                                new_culling = Some(ViewportCullingState {
                                    content_hash,
                                    available_width: outer_available_width,
                                    block_start_y: start_y,
                                    block_heights: updated_heights,
                                    total_height: y,
                                    block_measured: updated_measured,
                                });
                                // Still have unmeasured blocks — request another frame
                                // so the progressive pass continues.
                                if new_culling
                                    .as_ref()
                                    .unwrap()
                                    .block_measured
                                    .iter()
                                    .any(|&m| !m)
                                {
                                    ui.ctx().request_repaint();
                                }
                            }
                        } else {
                            // ── Bootstrap / lazy measurement pass ─────────────────
                            // Build a ViewportCullingState immediately using a mix
                            // of block-height cache hits, heuristic estimates, and a
                            // limited number of real renders (budget-capped).
                            let line_offsets = line_start_byte_offsets(self.content);
                            let rp_hash = cache::render_params_hash(content_width, self.font_size);

                            let mut boot_heights: Vec<f32> = Vec::with_capacity(block_count);
                            let mut boot_measured: Vec<bool> = Vec::with_capacity(block_count);

                            // Phase 1: Determine height for every block from cache
                            // or heuristic, without rendering anything.
                            for node in &doc.root.children {
                                let cached_h = {
                                    let s = block_source_slice(
                                        self.content,
                                        &line_offsets,
                                        node.start_line,
                                        node.end_line,
                                    );
                                    cache::get_block_height(s, rp_hash)
                                };
                                match cached_h {
                                    Some(h) => {
                                        boot_heights.push(h);
                                        boot_measured.push(true);
                                    }
                                    None => {
                                        let est =
                                            estimate_block_height(node.start_line, node.end_line);
                                        boot_heights.push(est);
                                        boot_measured.push(false);
                                    }
                                }
                            }

                            // Phase 2: Build start_y from the heights.
                            let mut boot_start_y: Vec<f32> = Vec::with_capacity(block_count);
                            {
                                let mut y = 0.0f32;
                                for (i, &h) in boot_heights.iter().enumerate() {
                                    boot_start_y.push(y);
                                    y += h;
                                    if i + 1 < block_count {
                                        y += BLOCK_ITEM_SPACING_Y;
                                    }
                                }
                            }
                            let boot_total: f32 = if block_count > 0 {
                                boot_start_y[block_count - 1] + boot_heights[block_count - 1]
                            } else {
                                0.0
                            };

                            // Set the min height so the scrollbar approximates the
                            // full document even on the very first frame.
                            ui.set_min_height(boot_total);

                            // Phase 3: Render only the viewport-visible blocks,
                            // capped by the render budget.
                            let vis_top = (viewport.min.y - VIEWPORT_OVERSCAN_PX).max(0.0);
                            let vis_bottom = viewport.max.y + VIEWPORT_OVERSCAN_PX;

                            let first_vis = boot_start_y
                                .partition_point(|&y| y <= vis_top)
                                .saturating_sub(1);
                            let last_vis = boot_start_y
                                .partition_point(|&y| y < vis_bottom)
                                .min(block_count)
                                .saturating_sub(1);

                            if first_vis > 0 {
                                let pre = (boot_start_y[first_vis] - BLOCK_ITEM_SPACING_Y).max(0.0);
                                ui.allocate_space(Vec2::new(content_width, pre));
                            }

                            let mut new_measures: usize = 0;
                            let boot_is_dark = ui.visuals().dark_mode;
                            for i in first_vis..=last_vis.min(block_count.saturating_sub(1)) {
                                let node = &doc.root.children[i];
                                let y_before = ui.cursor().top();
                                let block_left = ui.cursor().left();

                                let within_budget = new_measures < MAX_NEW_MEASUREMENTS_PER_FRAME;

                                if boot_measured[i] || within_budget {
                                    render_node(
                                        ui,
                                        node,
                                        self.content,
                                        &mut edit_state,
                                        colors,
                                        self.font_size,
                                        &self.font_family,
                                        0,
                                        self.paragraph_indent,
                                        self.header_spacing,
                                    );
                                    let y_after = ui.cursor().top();
                                    let h = (y_after - y_before).max(1.0);

                                    // Paint search highlight overlays on this block
                                    if let Some(ref highlights) = self.search_highlights {
                                        let is_table = matches!(
                                            node.node_type,
                                            MarkdownNodeType::Table { .. }
                                        );
                                        paint_rendered_search_highlights(
                                            ui,
                                            highlights,
                                            self.current_search_match,
                                            self.content,
                                            &line_offsets,
                                            node.start_line,
                                            node.end_line,
                                            y_before,
                                            y_after,
                                            block_left,
                                            block_left + content_width,
                                            is_table,
                                            boot_is_dark,
                                        );
                                    }

                                    if !boot_measured[i] {
                                        new_measures += 1;
                                    }
                                    let s = block_source_slice(
                                        self.content,
                                        &line_offsets,
                                        node.start_line,
                                        node.end_line,
                                    );
                                    cache::insert_block_height(s, rp_hash, h);
                                    boot_heights[i] = h;
                                    boot_measured[i] = true;
                                } else {
                                    ui.allocate_space(Vec2::new(content_width, boot_heights[i]));
                                }

                                line_mappings.push(LineMapping {
                                    start_line: node.start_line,
                                    end_line: node.end_line,
                                    rendered_y: boot_start_y[i],
                                    rendered_height: boot_heights[i],
                                });
                            }

                            let after_idx = last_vis + 1;
                            if after_idx < block_count {
                                let rendered_end = boot_start_y[last_vis] + boot_heights[last_vis];
                                let post =
                                    (boot_total - rendered_end - BLOCK_ITEM_SPACING_Y).max(0.0);
                                ui.allocate_space(Vec2::new(content_width, post));
                            }

                            // Off-screen line mappings for scroll sync.
                            for i in 0..first_vis {
                                line_mappings.push(LineMapping {
                                    start_line: doc.root.children[i].start_line,
                                    end_line: doc.root.children[i].end_line,
                                    rendered_y: boot_start_y[i],
                                    rendered_height: boot_heights[i],
                                });
                            }
                            for i in (last_vis + 1)..block_count {
                                line_mappings.push(LineMapping {
                                    start_line: doc.root.children[i].start_line,
                                    end_line: doc.root.children[i].end_line,
                                    rendered_y: boot_start_y[i],
                                    rendered_height: boot_heights[i],
                                });
                            }

                            // Rebuild start_y in case visible-block heights changed.
                            let mut final_start_y: Vec<f32> = Vec::with_capacity(block_count);
                            {
                                let mut y = 0.0f32;
                                for (i, &h) in boot_heights.iter().enumerate() {
                                    final_start_y.push(y);
                                    y += h;
                                    if i + 1 < block_count {
                                        y += BLOCK_ITEM_SPACING_Y;
                                    }
                                }
                            }
                            let final_total: f32 = if block_count > 0 {
                                final_start_y[block_count - 1] + boot_heights[block_count - 1]
                            } else {
                                0.0
                            };

                            new_culling = Some(ViewportCullingState {
                                content_hash,
                                available_width: outer_available_width,
                                block_start_y: final_start_y,
                                block_heights: boot_heights,
                                total_height: final_total,
                                block_measured: boot_measured,
                            });

                            if new_culling
                                .as_ref()
                                .unwrap()
                                .block_measured
                                .iter()
                                .any(|&m| !m)
                            {
                                ui.ctx().request_repaint();
                            }
                        }

                        let _ = &structural_state;
                    });

                    if content_margin > 0.0 {
                        ui.add_space(content_margin);
                    }
                });

                ui.allocate_response(Vec2::ZERO, egui::Sense::focusable_noninteractive())
            })
            .inner
        });

        // ── Persist culling state ────────────────────────────────────────────
        if let Some(cs) = new_culling {
            ui.memory_mut(|mem| {
                mem.data.insert_temp(culling_id, cs);
            });
        }

        // Render navigation buttons overlay (top-left corner of scroll area)
        // These buttons allow quick jumping to top, middle, or bottom of the document
        let is_dark_mode = ui.visuals().dark_mode;
        let nav_action = render_nav_buttons(ui, scroll_output.inner_rect, is_dark_mode);

        // Handle navigation button actions by storing target scroll offset in memory
        // This will be applied on the next frame
        if nav_action != NavAction::None {
            let content_height = scroll_output.content_size.y;
            let viewport_height = scroll_output.inner_rect.height();

            let target_offset = match nav_action {
                NavAction::Top => 0.0,
                NavAction::Middle => {
                    // Center the middle of the document in the viewport
                    let middle = content_height / 2.0;
                    (middle - viewport_height / 2.0).max(0.0)
                }
                NavAction::Bottom => {
                    // Scroll to show the bottom of the document
                    (content_height - viewport_height).max(0.0)
                }
                NavAction::None => 0.0, // unreachable
            };

            // Store the target offset in egui memory for the next frame
            ui.memory_mut(|mem| {
                mem.data.insert_temp(nav_scroll_id, target_offset);
            });

            // Request repaint to apply the scroll on the next frame
            ui.ctx().request_repaint();
        }

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
            rebuild_markdown(self.content, &edit_state, "");
            debug!("WYSIWYG editor content changed, rebuilt markdown");
        }

        let changed = content_changed || structural_changed;

        // Get focused element info for formatting commands
        let focused_element = edit_state.get_focused_element(self.content);

        // Check if a wikilink was clicked this frame
        let wikilink_id = egui::Id::new("wikilink_clicked_target");
        let wikilink_clicked = ui.memory(|mem| mem.data.get_temp::<String>(wikilink_id));
        if wikilink_clicked.is_some() {
            ui.memory_mut(|mem| {
                mem.data.remove::<String>(wikilink_id);
            });
        }

        MarkdownEditorOutput {
            response: scroll_output.inner,
            changed,
            cursor_position: (0, 0), // Position tracking is simplified in WYSIWYG mode
            mode: EditorMode::Rendered,
            focused_element,
            scroll_offset: scroll_output.state.offset.y,
            content_height: scroll_output.content_size.y,
            viewport_height: scroll_output.inner_rect.height(),
            line_mappings,
            wikilink_clicked,
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
    editor_font: &EditorFont,
    indent_level: usize,
    parent_list_type: Option<&ListType>,
    list_item_index: Option<usize>,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
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
                header_spacing,
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
                paragraph_indent,
            );
            ui.add_space(PARAGRAPH_TRAILING_SPACE_Y);
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
                paragraph_indent,
                header_spacing,
            );
        }
        MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } => {
            render_callout_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                structural_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                *callout_type,
                title.as_deref(),
                *collapsed,
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
            render_table(ui, node, source, edit_state, colors, font_size, editor_font);
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            // Hide HTML comments completely (standard markdown behavior)
            // HTML comments start with <!-- and end with -->
            let trimmed = html.trim();
            if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                // HTML comment - don't render anything
            } else {
                // Other HTML blocks - show with subtle indicator
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("«HTML»")
                            .color(colors.quote_text)
                            .small()
                            .italics(),
                    );
                });
            }
        }
        MarkdownNodeType::Link { url, title } => {
            render_link(ui, node, source, edit_state, colors, font_size, url, title);
        }
        MarkdownNodeType::Wikilink { target, display } => {
            render_wikilink(ui, colors, font_size, target, display.as_deref());
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
                    paragraph_indent,
                    header_spacing,
                );
            }
        }
        MarkdownNodeType::Image { url, title } => {
            render_image(ui, node, colors, font_size, url, title);
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
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
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
                header_spacing,
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
                paragraph_indent,
            );
            ui.add_space(PARAGRAPH_TRAILING_SPACE_Y);
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
                paragraph_indent,
                header_spacing,
            );
        }
        MarkdownNodeType::Callout {
            callout_type,
            title,
            collapsed,
        } => {
            render_callout(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                *callout_type,
                title.as_deref(),
                *collapsed,
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
            render_table(ui, node, source, edit_state, colors, font_size, editor_font);
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            // Hide HTML comments completely (standard markdown behavior)
            // HTML comments start with <!-- and end with -->
            let trimmed = html.trim();
            if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                // HTML comment - don't render anything
            } else {
                // Other HTML blocks - show with subtle indicator
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("«HTML»")
                            .color(colors.quote_text)
                            .small()
                            .italics(),
                    );
                });
            }
        }
        MarkdownNodeType::Link { url, title } => {
            render_link(ui, node, source, edit_state, colors, font_size, url, title);
        }
        MarkdownNodeType::Wikilink { target, display } => {
            render_wikilink(ui, colors, font_size, target, display.as_deref());
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
                    paragraph_indent,
                    header_spacing,
                );
            }
        }
        MarkdownNodeType::Image { url, title } => {
            render_image(ui, node, colors, font_size, url, title);
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

/// Get (top_margin, bottom_margin) in pixels for a heading level and spacing preset.
fn header_margins(spacing: HeaderSpacing, level: HeadingLevel) -> (f32, f32) {
    let (base_top, base_bottom) = match level {
        HeadingLevel::H1 => (8.0, 0.0),
        HeadingLevel::H2 => (6.0, 0.0),
        _ => (4.0, 0.0),
    };
    match spacing {
        HeaderSpacing::Compact => (base_top * 0.5, 0.0),
        HeaderSpacing::Normal => (base_top, base_bottom),
        HeaderSpacing::Relaxed => (
            base_top * 1.5,
            match level {
                HeadingLevel::H1 => 6.0,
                HeadingLevel::H2 => 4.0,
                _ => 2.0,
            },
        ),
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
    editor_font: &EditorFont,
    level: HeadingLevel,
    header_spacing: HeaderSpacing,
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

    // Add top margin for headings (separation from previous content)
    let (top_margin, bottom_margin) = header_margins(header_spacing, level);
    ui.add_space(top_margin);

    // Editable heading text with left indent
    // Create explicit ID for heading TextEdit to prevent any potential conflicts
    let heading_widget_id = ui.id().with("heading_text").with(node.start_line);
    let heading_edit_buffer_id = ui.id().with("heading_edit_buffer").with(node.start_line);
    let heading_edit_tracking_id = ui.id().with("heading_edit_tracking").with(node.start_line);

    // Track whether this heading was previously focused (to detect focus loss)
    let was_editing = ui.memory(|mem| {
        mem.data
            .get_temp::<bool>(heading_edit_tracking_id)
            .unwrap_or(false)
    });

    let available_width = ui.available_width();
    let (has_focus, selection) = ui
        .horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(4.0);

            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let mut edit_buffer = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(heading_edit_buffer_id, || {
                            editable.text.clone()
                        })
                        .clone()
                });

                let text_edit = TextEdit::singleline(&mut edit_buffer)
                    .id(heading_widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.heading)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width());

                let output = text_edit.show(ui);

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

                // Update edit buffer and tracking in memory
                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(heading_edit_buffer_id, edit_buffer.clone());
                    mem.data.insert_temp(heading_edit_tracking_id, has_focus);
                });

                if output.response.changed() {
                    editable.modified = true;
                    update_source_line(
                        source,
                        node.start_line,
                        &format_heading(&edit_buffer, level),
                    );
                }

                // Only commit changes when focus is LOST (was editing, now not)
                // This prevents rebuild during active editing
                if was_editing && !has_focus {
                    // Clear the edit buffer
                    ui.memory_mut(|mem| {
                        mem.data.remove::<String>(heading_edit_buffer_id);
                    });
                }

                (has_focus, selection)
            } else {
                (false, None)
            }
        })
        .inner;

    // Track focus
    if has_focus {
        edit_state.set_focus(node_id, selection);
    }
    // Add bottom margin based on header spacing setting
    if bottom_margin > 0.0 {
        ui.add_space(bottom_margin);
    }
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
    editor_font: &EditorFont,
    level: HeadingLevel,
    header_spacing: HeaderSpacing,
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

    // Add top margin for headings
    let (top_margin, bottom_margin) = header_margins(header_spacing, level);
    ui.add_space(top_margin);

    // Editable heading text with left indent
    // Create explicit ID for heading TextEdit to prevent any potential conflicts
    let heading_widget_id = ui.id().with("heading_text_sk").with(node.start_line);
    let heading_edit_buffer_id = ui.id().with("heading_sk_edit_buffer").with(node.start_line);
    let heading_edit_tracking_id = ui
        .id()
        .with("heading_sk_edit_tracking")
        .with(node.start_line);

    // Track whether this heading was previously focused
    let was_editing = ui.memory(|mem| {
        mem.data
            .get_temp::<bool>(heading_edit_tracking_id)
            .unwrap_or(false)
    });

    let available_width = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(4.0);

        if let Some(editable) = edit_state.get_node_mut(node_id) {
            let mut edit_buffer = ui.memory_mut(|mem| {
                mem.data
                    .get_temp_mut_or_insert_with(heading_edit_buffer_id, || editable.text.clone())
                    .clone()
            });

            let response = ui.add(
                TextEdit::singleline(&mut edit_buffer)
                    .id(heading_widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.heading)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width()),
            );

            let _ = structural_state;

            let has_focus = response.has_focus();

            // Update edit buffer and tracking in memory
            ui.memory_mut(|mem| {
                mem.data
                    .insert_temp(heading_edit_buffer_id, edit_buffer.clone());
                mem.data.insert_temp(heading_edit_tracking_id, has_focus);
            });

            if response.changed() {
                editable.modified = true;
                update_source_line(
                    source,
                    node.start_line,
                    &format_heading(&edit_buffer, level),
                );
            }

            // Only commit when focus is lost
            if was_editing && !has_focus {
                // Clear the edit buffer
                ui.memory_mut(|mem| {
                    mem.data.remove::<String>(heading_edit_buffer_id);
                });
            }
        }
    });
    // Add bottom margin based on header spacing setting
    if bottom_margin > 0.0 {
        ui.add_space(bottom_margin);
    }
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
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
) {
    // Check if paragraph contains special inline elements (including images)
    let has_inline_elements = node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Link { .. }
                | MarkdownNodeType::Wikilink { .. }
                | MarkdownNodeType::Strong
                | MarkdownNodeType::Emphasis
                | MarkdownNodeType::Strikethrough
                | MarkdownNodeType::Code(_)
                | MarkdownNodeType::Image { .. }
        )
    });

    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    // Calculate CJK paragraph indentation (only for top-level paragraphs)
    let cjk_indent = if indent_level == 0 {
        paragraph_indent.to_pixels(font_size).unwrap_or(0.0)
    } else {
        0.0
    };

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

        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(4.0 + indent_level as f32 * 20.0);

            if para_edit_state.editing {
                let font_family_clone = font_family.clone();
                let text_color = colors.text;
                let cjk_leading = cjk_indent;
                let mut layouter =
                    move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap_width;
                        job.append(
                            text,
                            cjk_leading,
                            egui::text::TextFormat {
                                font_id: FontId::new(font_size, font_family_clone.clone()),
                                color: text_color,
                                ..Default::default()
                            },
                        );
                        ui.fonts(|f| f.layout_job(job))
                    };
                let text_edit = TextEdit::multiline(&mut para_edit_state.edit_text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family.clone()))
                    .text_color(colors.text)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter);

                let mut output = text_edit.show(ui);
                let response = output.response.clone();

                if para_edit_state.needs_focus {
                    response.request_focus();
                    para_edit_state.needs_focus = false;

                    // Apply pending cursor position if set
                    if let Some(cursor_pos) = para_edit_state.pending_cursor_pos.take() {
                        let ccursor = egui::text::CCursor::new(cursor_pos);
                        let cursor_range = egui::text::CCursorRange::one(ccursor);
                        output.state.cursor.set_char_range(Some(cursor_range));
                        output.state.store(ui.ctx(), widget_id);
                        debug!("[PARA_DEBUG] Set cursor position to {} for paragraph", cursor_pos);
                    }
                }

                let enter_pressed = response.has_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                let escape_pressed =
                    response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                if response.changed() {
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
                }

                if enter_pressed || focus_lost {
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
                        // CJK first-line indent: Add spacer at start of horizontal_wrapped
                        // This only affects the first line - wrapped content starts flush left
                        if cjk_indent > 0.0 {
                            ui.add_space(cjk_indent);
                        }
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
                    // Check if a link widget consumed this click
                    let link_consumed = ui.memory(|mem| {
                        mem.data
                            .get_temp::<bool>(egui::Id::new("link_click_consumed_this_frame"))
                            .unwrap_or(false)
                    });

                    if !link_consumed {
                        para_edit_state.editing = true;
                        para_edit_state.needs_focus = true;
                        para_edit_state.edit_text =
                            extract_paragraph_content(source, node.start_line, node.end_line);

                        // Calculate cursor position from click location using Galley for accuracy
                        // This maps screen position to character index in displayed text
                        let cursor_pos = if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            let displayed_text = node.text_content();
                            let displayed_idx = compute_displayed_cursor_index(
                                ui,
                                &displayed_text,
                                click_pos,
                                display_response.rect,
                                font_size,
                                editor_font,
                                &para_edit_state.edit_text,
                            );
                            // Map displayed position to raw position (accounting for formatting markers)
                        let raw_idx = map_displayed_to_raw(displayed_idx, &para_edit_state.edit_text);
                        Some(raw_idx.min(para_edit_state.edit_text.chars().count()))
                    } else {
                        None
                    };
                    para_edit_state.pending_cursor_pos = cursor_pos;

                        debug!(
                            "Entering edit mode for formatted paragraph at line {}, cursor_pos={:?}",
                            node.start_line, cursor_pos
                        );

                        ui.memory_mut(|mem| {
                            mem.data
                                .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                        });
                    }
                }

                if sense_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
            }
        });
    } else {
        // Simple text-only paragraph — persist edit buffer in egui memory
        // to prevent trailing spaces being stripped by AST round-trip each frame.
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

        let para_edit_buffer_id = ui.id().with("para_edit_buffer").with(node.start_line);
        let para_edit_tracking_id = ui.id().with("para_edit_tracking").with(node.start_line);

        let was_editing = ui.memory(|mem| {
            mem.data
                .get_temp::<bool>(para_edit_tracking_id)
                .unwrap_or(false)
        });

        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(4.0 + indent_level as f32 * 20.0);

            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let source_text = extract_paragraph_content(source, node.start_line, node.end_line);
                let mut edit_buffer = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(para_edit_buffer_id, || source_text)
                        .clone()
                });

                let font_family_clone = font_family.clone();
                let text_color = colors.text;
                let cjk_leading = cjk_indent;
                let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = wrap_width;
                    job.append(
                        text,
                        cjk_leading,
                        egui::text::TextFormat {
                            font_id: FontId::new(font_size, font_family_clone.clone()),
                            color: text_color,
                            ..Default::default()
                        },
                    );
                    ui.fonts(|f| f.layout_job(job))
                };

                let widget_id = ui.id().with("para_text").with(node.start_line);
                let text_edit = TextEdit::multiline(&mut edit_buffer)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family.clone()))
                    .text_color(colors.text)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter);

                let output = text_edit.show(ui);
                let has_focus = output.response.has_focus();

                let _ = structural_state;

                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(para_edit_buffer_id, edit_buffer.clone());
                    mem.data.insert_temp(para_edit_tracking_id, has_focus);
                });

                if output.response.changed() {
                    editable.modified = true;
                    update_source_range(source, node.start_line, node.end_line, &edit_buffer);
                }

                if was_editing && !has_focus {
                    ui.memory_mut(|mem| {
                        mem.data.remove::<String>(para_edit_buffer_id);
                    });
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
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
) {
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;
    const BORDER_WIDTH: f32 = 4.0;
    const BORDER_GAP: f32 = 8.0;

    let available_width = ui.available_width();
    let group_response = ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

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
                    paragraph_indent,
                    header_spacing,
                );
            }
        });
    });

    // Paint the quote border using the actual rendered content height
    let rect = group_response.response.rect;
    let border_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(BORDER_WIDTH, rect.height()),
    );
    ui.painter()
        .rect_filled(border_rect, 0.0, colors.quote_border);
}

/// Get the color scheme for a callout type.
/// Returns (border_color, background_color, icon_color) for both dark and light themes.
fn callout_colors(callout_type: CalloutType, is_dark: bool) -> (Color32, Color32, Color32) {
    // Background uses from_rgba_unmultiplied for correct subtle tinting.
    // Alpha ~25 out of 255 gives a gentle wash behind the content.
    match callout_type {
        CalloutType::Note => {
            if is_dark {
                (
                    Color32::from_rgb(56, 132, 244),                   // border
                    Color32::from_rgba_unmultiplied(56, 132, 244, 25), // bg
                    Color32::from_rgb(88, 166, 255),                   // icon/title
                )
            } else {
                (
                    Color32::from_rgb(9, 105, 218),
                    Color32::from_rgba_unmultiplied(9, 105, 218, 20),
                    Color32::from_rgb(9, 105, 218),
                )
            }
        }
        CalloutType::Tip => {
            if is_dark {
                (
                    Color32::from_rgb(63, 185, 80),
                    Color32::from_rgba_unmultiplied(63, 185, 80, 25),
                    Color32::from_rgb(63, 185, 80),
                )
            } else {
                (
                    Color32::from_rgb(26, 127, 55),
                    Color32::from_rgba_unmultiplied(26, 127, 55, 20),
                    Color32::from_rgb(26, 127, 55),
                )
            }
        }
        CalloutType::Warning => {
            if is_dark {
                (
                    Color32::from_rgb(210, 153, 34),
                    Color32::from_rgba_unmultiplied(210, 153, 34, 25),
                    Color32::from_rgb(210, 153, 34),
                )
            } else {
                (
                    Color32::from_rgb(154, 103, 0),
                    Color32::from_rgba_unmultiplied(154, 103, 0, 20),
                    Color32::from_rgb(154, 103, 0),
                )
            }
        }
        CalloutType::Caution => {
            if is_dark {
                (
                    Color32::from_rgb(218, 190, 36),
                    Color32::from_rgba_unmultiplied(218, 190, 36, 25),
                    Color32::from_rgb(218, 190, 36),
                )
            } else {
                (
                    Color32::from_rgb(155, 130, 10),
                    Color32::from_rgba_unmultiplied(155, 130, 10, 20),
                    Color32::from_rgb(155, 130, 10),
                )
            }
        }
        CalloutType::Important => {
            if is_dark {
                (
                    Color32::from_rgb(219, 97, 109),
                    Color32::from_rgba_unmultiplied(219, 97, 109, 25),
                    Color32::from_rgb(219, 97, 109),
                )
            } else {
                (
                    Color32::from_rgb(191, 57, 67),
                    Color32::from_rgba_unmultiplied(191, 57, 67, 20),
                    Color32::from_rgb(191, 57, 67),
                )
            }
        }
    }
}

/// Render a callout (GitHub-style admonition) with structural key support.
fn render_callout_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
    callout_type: CalloutType,
    custom_title: Option<&str>,
    default_collapsed: bool,
) {
    const BASE_INDENT: f32 = 4.0;
    const BORDER_WIDTH: f32 = 4.0;
    const BORDER_GAP: f32 = 8.0;

    let is_dark = colors.background.r() < 128;
    let (border_color, bg_color, title_color) = callout_colors(callout_type, is_dark);

    // Scope all child widget IDs under a unique ID to prevent collisions
    // between multiple callouts. Using (start_line, end_line) for uniqueness.
    let scope_id = ("callout_struct", node.start_line, node.end_line);

    let group_response = ui.push_id(scope_id, |ui| {
        let callout_id = ui.make_persistent_id("collapsed");
        let is_collapsed = ui.data_mut(|d| *d.get_persisted_mut_or(callout_id, default_collapsed));

        let title_text = custom_title.unwrap_or(callout_type.display_name());
        let icon = callout_type.icon();

        let available_width = ui.available_width();
        let inner = ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

            ui.vertical(|ui| {
                let title_row = ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(icon)
                            .color(title_color)
                            .font(FontId::proportional(font_size)),
                    );

                    let arrow = if is_collapsed { "▶" } else { "▼" };
                    ui.label(
                        RichText::new(arrow)
                            .color(title_color)
                            .font(FontId::proportional(font_size * 0.7)),
                    );

                    ui.label(
                        RichText::new(title_text)
                            .color(title_color)
                            .font(FontId::proportional(font_size))
                            .strong(),
                    );
                });

                // Place a clickable rect over the title row for collapse toggle
                let title_rect = title_row.response.rect;
                let click_response = ui.allocate_rect(title_rect, egui::Sense::click());
                if click_response.clicked() {
                    ui.data_mut(|d| {
                        let val = d.get_persisted_mut_or(callout_id, default_collapsed);
                        *val = !*val;
                    });
                }
                // Show pointer cursor on hover
                if click_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                if !is_collapsed {
                    ui.add_space(2.0);
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
                            paragraph_indent,
                            header_spacing,
                        );
                    }
                }
            });
        });
        inner.response
    });

    // Paint styled background and left border
    let rect = group_response.response.rect;
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(rect.width() - BASE_INDENT, rect.height()),
    );
    ui.painter().rect_filled(content_rect, 4.0, bg_color);

    let border_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(BORDER_WIDTH, rect.height()),
    );
    ui.painter().rect_filled(border_rect, 2.0, border_color);
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
    editor_font: &EditorFont,
    indent_level: usize,
    list_type: &ListType,
) {
    // Add small top margin for top-level lists
    if indent_level == 0 {
        ui.add_space(4.0);
    }

    let mut item_number = match list_type {
        ListType::Ordered { start, .. } => *start,
        ListType::Bullet => 0,
    };

    for (idx, child) in node.children.iter().enumerate() {
        // Handle both regular list items (Item) and task list items (TaskItem)
        // Note: In some markdown AST structures, task lists have TaskItem as direct
        // children of List, not wrapped in an Item node
        let should_render = matches!(
            &child.node_type,
            MarkdownNodeType::Item | MarkdownNodeType::TaskItem { .. }
        );

        if should_render {
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
    editor_font: &EditorFont,
    indent_level: usize,
    list_type: &ListType,
    item_number: u32,
    item_index: usize,
) {
    // Check if this node IS a TaskItem (direct child of List) or CONTAINS a TaskItem child
    let (is_task, task_checked) = if let MarkdownNodeType::TaskItem { checked } = &node.node_type {
        // The node itself is a TaskItem (task list structure)
        (true, *checked)
    } else {
        // Regular Item - check if it has a TaskItem child
        let task_child = node.children.iter().find_map(|c| {
            if let MarkdownNodeType::TaskItem { checked } = &c.node_type {
                Some(*checked)
            } else {
                None
            }
        });
        (task_child.is_some(), task_child.unwrap_or(false))
    };

    let para_node = node
        .children
        .iter()
        .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

    let nested_lists: Vec<&MarkdownNode> = node
        .children
        .iter()
        .filter(|c| matches!(c.node_type, MarkdownNodeType::List { .. }))
        .collect();

    // Check if paragraph has inline formatting (bold, italic, images, line breaks, etc.)
    // LineBreak must be included here because single-line TextEdit cannot render newlines,
    // and would display them as replacement characters (□). See GitHub issue #41.
    // Also check for task items which need checkbox rendering
    let has_inline_formatting = para_node
        .map(|p| {
            p.children.iter().any(|c| {
                matches!(
                    c.node_type,
                    MarkdownNodeType::Strong
                        | MarkdownNodeType::Emphasis
                        | MarkdownNodeType::Strikethrough
                        | MarkdownNodeType::Link { .. }
                        | MarkdownNodeType::Wikilink { .. }
                        | MarkdownNodeType::Code(_)
                        | MarkdownNodeType::Image { .. }
                        | MarkdownNodeType::LineBreak
                        | MarkdownNodeType::TaskItem { .. }
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

    // Base indentation to align with content area + nested indent
    // Use 4.0 to match BASE_INDENT used by headings, paragraphs, code blocks, etc.
    let base_indent = 4.0;
    let nested_indent = indent_level as f32 * 20.0;
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    let available_width = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_max_width(available_width);

        // Total indentation: base + nested
        ui.add_space(base_indent + nested_indent);

        // Render list marker (bullet, number, or checkbox for tasks)
        if is_task {
            // Use egui Checkbox for task list items - now clickable!
            let mut checked = task_checked;
            let checkbox_response = ui.checkbox(&mut checked, "");

            // Handle checkbox click - toggle the source
            if checkbox_response.changed() {
                // Toggle the task marker in the source
                if let Some(source_line) = source.lines().nth(node.start_line.saturating_sub(1)) {
                    let new_line = if task_checked {
                        // Was checked, now unchecked: [x] -> [ ]
                        source_line.replace("[x]", "[ ]").replace("[X]", "[ ]")
                    } else {
                        // Was unchecked, now checked: [ ] -> [x]
                        source_line.replace("[ ]", "[x]")
                    };
                    update_source_line(source, node.start_line, &new_line);

                    // Mark as modified
                    let node_id = edit_state.add_node(
                        para_node.map(|p| p.text_content()).unwrap_or_default(),
                        node.start_line,
                        node.end_line,
                    );
                    if let Some(editable) = edit_state.get_node_mut(node_id) {
                        editable.modified = true;
                    }
                }
            }
            ui.add_space(2.0);
        } else {
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
                let formatted_item_id = ui
                    .id()
                    .with("formatted_list_item_sk")
                    .with(para.start_line)
                    .with(item_index);

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
                    // EDIT MODE: Show multiline TextEdit with wrapping for raw markdown
                    let font_family_for_layout = font_family.clone();
                    let text_color = colors.text;
                    let mut layouter =
                        move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap.max_width = wrap_width;
                            job.append(
                                text,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: FontId::new(font_size, font_family_for_layout.clone()),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                            ui.fonts(|f| f.layout_job(job))
                        };
                    let text_edit = TextEdit::multiline(&mut item_edit_state.edit_text)
                        .id(widget_id)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .desired_rows(1)
                        .margin(egui::vec2(0.0, 2.0))
                        .layouter(&mut layouter);

                    // Use show() to get TextEditOutput for cursor manipulation
                    let mut output = text_edit.show(ui);
                    let response = output.response.clone();

                    // Strip newlines that multiline TextEdit inserts on Enter
                    if item_edit_state.edit_text.contains('\n') {
                        item_edit_state.edit_text = item_edit_state.edit_text.replace('\n', "");
                    }

                    // Request focus if needed (first frame after entering edit mode)
                    if item_edit_state.needs_focus {
                        response.request_focus();
                        item_edit_state.needs_focus = false;

                        // Apply pending cursor position if set
                        if let Some(cursor_pos) = item_edit_state.pending_cursor_pos.take() {
                            let ccursor = egui::text::CCursor::new(cursor_pos);
                            let cursor_range = egui::text::CCursorRange::one(ccursor);
                            output.state.cursor.set_char_range(Some(cursor_range));
                            output.state.store(ui.ctx(), widget_id);
                            debug!("[LIST_DEBUG] Set cursor position to {} for list item (sk)", cursor_pos);
                        }
                    }

                    // Check for exit conditions
                    let enter_pressed =
                        response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let escape_pressed =
                        response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                    let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                    if response.changed() {
                        // Save changes immediately
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
                    }

                    if enter_pressed || focus_lost {
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
                        // Check if a link widget consumed this click
                        let link_consumed = ui.memory(|mem| {
                            mem.data
                                .get_temp::<bool>(egui::Id::new("link_click_consumed_this_frame"))
                                .unwrap_or(false)
                        });

                        if !link_consumed {
                            // DEBUG: Log click on structural key list item
                            debug!(
                                "[LIST_DEBUG] CLICK DETECTED (sk): para.start_line={}, item_index={}, \
                                 display_rect={:?}",
                                para.start_line, item_index, display_response.rect
                            );

                            // Enter edit mode
                            item_edit_state.editing = true;
                            item_edit_state.needs_focus = true;
                            // Get raw markdown content from source
                            item_edit_state.edit_text =
                                extract_list_item_content(source, para.start_line);

                            // Calculate cursor position from click location
                            // Use the DISPLAYED text to calculate position, then use directly in raw text
                            // (don't scale - scaling makes position drift worse)
                            let cursor_pos = if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                                let rect = display_response.rect;
                                let raw_len = item_edit_state.edit_text.len();
                                // Get the displayed text (without formatting markers like **)
                                let displayed_text = para.text_content();
                                let displayed_len = displayed_text.len();
                                if displayed_len > 0 && rect.width() > 0.0 {
                                    let relative_x = ((click_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                    // Map to displayed character position and use directly
                                    // Don't scale to raw - that makes the drift worse
                                    let char_pos = (relative_x * displayed_len as f32).round() as usize;
                                    Some(char_pos.min(raw_len))
                                } else {
                                    Some(0)
                                }
                            } else {
                                None
                            };
                            item_edit_state.pending_cursor_pos = cursor_pos;

                            // DEBUG: Log edit mode entry
                            debug!(
                                "[LIST_DEBUG] EDIT MODE ENTERED (sk): para.start_line={}, item_index={}, content='{}', cursor_pos={:?}",
                                para.start_line, item_index, item_edit_state.edit_text, cursor_pos
                            );

                            // Store the new state
                            ui.memory_mut(|mem| {
                                mem.data
                                    .insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                            });
                        }
                    }

                    // Show hover cursor to indicate clickability
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text - editable with wrapping
            if let Some(editable) = edit_state.get_node_mut(node_id) {
                let widget_id = ui.id().with("list_item_text_sk").with(start_line);

                let font_family_for_layout = font_family.clone();
                let text_color = colors.text;
                let mut layouter =
                    move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap_width;
                        job.append(
                            text,
                            0.0,
                            egui::text::TextFormat {
                                font_id: FontId::new(font_size, font_family_for_layout.clone()),
                                color: text_color,
                                ..Default::default()
                            },
                        );
                        ui.fonts(|f| f.layout_job(job))
                    };
                let text_edit = TextEdit::multiline(&mut editable.text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter);

                let response = ui.add(text_edit);

                // Strip newlines — list items should not contain literal newlines
                if editable.text.contains('\n') {
                    editable.text = editable.text.replace('\n', "");
                }

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
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
) {
    // Check if paragraph contains any special inline elements (links, formatting, images)
    let has_inline_elements = node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Link { .. }
                | MarkdownNodeType::Wikilink { .. }
                | MarkdownNodeType::Strong
                | MarkdownNodeType::Emphasis
                | MarkdownNodeType::Strikethrough
                | MarkdownNodeType::Code(_)
                | MarkdownNodeType::Image { .. }
        )
    });

    // Get font family for regular (non-styled) text
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    // Calculate CJK paragraph indentation (only for top-level paragraphs)
    let cjk_indent = if indent_level == 0 {
        paragraph_indent.to_pixels(font_size).unwrap_or(0.0)
    } else {
        0.0
    };

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

        if para_edit_state.editing {
            let available_width = ui.available_width();
            ui.horizontal(|ui| {
                ui.set_max_width(available_width);
                ui.add_space(4.0 + indent_level as f32 * 20.0);

                let font_family_clone = font_family.clone();
                let text_color = colors.text;
                let cjk_leading = cjk_indent;
                let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = wrap_width;
                    job.append(
                        text,
                        cjk_leading,
                        egui::text::TextFormat {
                            font_id: FontId::new(font_size, font_family_clone.clone()),
                            color: text_color,
                            ..Default::default()
                        },
                    );
                    ui.fonts(|f| f.layout_job(job))
                };
                let text_edit = TextEdit::multiline(&mut para_edit_state.edit_text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family.clone()))
                    .text_color(colors.text)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter);

                let mut output = text_edit.show(ui);
                let response = output.response.clone();

                // Request focus if needed
                if para_edit_state.needs_focus {
                    response.request_focus();
                    para_edit_state.needs_focus = false;

                    // Apply pending cursor position if set
                    if let Some(cursor_pos) = para_edit_state.pending_cursor_pos.take() {
                        let ccursor = egui::text::CCursor::new(cursor_pos);
                        let cursor_range = egui::text::CCursorRange::one(ccursor);
                        output.state.cursor.set_char_range(Some(cursor_range));
                        output.state.store(ui.ctx(), widget_id);
                        debug!(
                            "[PARA_DEBUG] Set cursor position to {} for paragraph (2)",
                            cursor_pos
                        );
                    }
                }

                // Check for exit conditions
                let enter_pressed = response.has_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                let escape_pressed =
                    response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                if response.changed() {
                    // Save changes immediately
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
                }

                if enter_pressed || focus_lost {
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
            });
        } else {
            let base_indent = 4.0 + indent_level as f32 * 20.0;

            let available_width_display = ui.available_width();
            let display_response = ui
                .horizontal(|ui| {
                    ui.set_max_width(available_width_display);
                    ui.add_space(base_indent);

                    ui.scope(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            // CJK first-line indent: spacer at start (first line only)
                            if cjk_indent > 0.0 {
                                ui.add_space(cjk_indent);
                            }
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
                    })
                    .response
                })
                .inner;

            // Make the display area interactive
            let sense_response = ui.interact(
                display_response.rect,
                formatted_para_id.with("click_sense"),
                egui::Sense::click(),
            );

            if sense_response.clicked() {
                // Check if a link widget consumed this click
                let link_consumed = ui.memory(|mem| {
                    mem.data
                        .get_temp::<bool>(egui::Id::new("link_click_consumed_this_frame"))
                        .unwrap_or(false)
                });

                if !link_consumed {
                    // Enter edit mode
                    para_edit_state.editing = true;
                    para_edit_state.needs_focus = true;
                    para_edit_state.edit_text =
                        extract_paragraph_content(source, node.start_line, node.end_line);

                    // Calculate cursor position from click location using Galley for accuracy
                    // This maps screen position to character index in displayed text
                    let cursor_pos =
                        if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            let displayed_text = node.text_content();
                            let displayed_idx = compute_displayed_cursor_index(
                                ui,
                                &displayed_text,
                                click_pos,
                                display_response.rect,
                                font_size,
                                editor_font,
                                &para_edit_state.edit_text,
                            );
                            // Map displayed position to raw position (accounting for formatting markers)
                            let raw_idx =
                                map_displayed_to_raw(displayed_idx, &para_edit_state.edit_text);
                            Some(raw_idx.min(para_edit_state.edit_text.chars().count()))
                        } else {
                            None
                        };
                    para_edit_state.pending_cursor_pos = cursor_pos;

                    debug!(
                        "Entering edit mode for formatted paragraph at line {}, cursor_pos={:?}",
                        node.start_line, cursor_pos
                    );

                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(formatted_para_id.with("edit_state"), para_edit_state);
                    });
                }
            }

            if sense_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        }
    } else {
        // Simple text-only paragraph - use editable text directly
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);

        let available_width = ui.available_width();
        let (has_focus, selection, changed, new_text) = ui
            .horizontal(|ui| {
                ui.set_max_width(available_width);
                ui.add_space(4.0 + indent_level as f32 * 20.0);

                if let Some(editable) = edit_state.get_node_mut(node_id) {
                    let font_family_clone = font_family.clone();
                    let text_color = colors.text;
                    let cjk_leading = cjk_indent;
                    let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap_width;
                        job.append(
                            text,
                            cjk_leading,
                            egui::text::TextFormat {
                                font_id: FontId::new(font_size, font_family_clone.clone()),
                                color: text_color,
                                ..Default::default()
                            },
                        );
                        ui.fonts(|f| f.layout_job(job))
                    };
                    let text_edit = TextEdit::multiline(&mut editable.text)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0))
                        .desired_width(ui.available_width())
                        .desired_rows(1)
                        .layouter(&mut layouter);

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
    fn apply(&self, text: RichText, font_size: f32, editor_font: &EditorFont) -> RichText {
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

/// Compute the character index in displayed text from a click position using egui's Galley.
///
/// This function uses proper font metrics via Galley layout to accurately map a screen
/// click position to a character index in the displayed text (text without formatting markers).
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `displayed_text` - The text as shown to the user (without `**`, `*`, etc. markers)
/// * `click_pos` - The screen position of the click
/// * `text_rect` - The rectangle containing the rendered text
/// * `font_size` - The font size used for rendering
/// * `editor_font` - The font family used for rendering
///
/// # Returns
/// The character index in `displayed_text` where the click occurred (0 to displayed_text.len())
fn compute_displayed_cursor_index(
    ui: &Ui,
    displayed_text: &str,
    click_pos: egui::Pos2,
    text_rect: egui::Rect,
    font_size: f32,
    editor_font: &EditorFont,
    raw_text: &str,
) -> usize {
    if displayed_text.is_empty() {
        return 0;
    }

    // Check if the raw text starts with bold markers - if so, use bold font for measurement
    // This improves accuracy since formatted content (especially list items) often starts bold
    let starts_with_bold = raw_text.starts_with("**") || raw_text.starts_with("__");
    let font_family = fonts::get_styled_font_family(starts_with_bold, false, editor_font);
    let font_id = FontId::new(font_size, font_family);

    // Create a Galley for measuring the displayed text
    // IMPORTANT: Use layout() with wrap_width to handle text wrapping correctly
    // If we use layout_no_wrap(), clicks on wrapped lines will map to wrong positions
    let galley = ui.fonts(|f| {
        f.layout(
            displayed_text.to_owned(),
            font_id,
            Color32::PLACEHOLDER, // Color doesn't affect measurement
            text_rect.width(),    // Use actual rendered width for proper wrapping
        )
    });

    // Compute local click position relative to text_rect's top-left as Vec2
    let local_pos = egui::Vec2::new(click_pos.x - text_rect.min.x, click_pos.y - text_rect.min.y);

    // Use cursor_from_pos to get the exact character index
    let cursor = galley.cursor_from_pos(local_pos);
    let displayed_idx = cursor.ccursor.index;

    // Clamp to valid range (cursor_from_pos should already do this, but be safe)
    displayed_idx.min(displayed_text.chars().count())
}

/// Maps a cursor position in displayed text (without formatting markers) to the
/// corresponding position in raw markdown text (with formatting markers).
///
/// # Arguments
/// * `displayed_idx` - The cursor position in displayed text (character index)
/// * `raw_text` - The raw markdown text containing formatting markers like `**`, `*`, etc.
///
/// # Returns
/// The corresponding cursor position in raw text (character index)
///
/// # Algorithm
/// Walks through raw text, skipping formatting markers while counting displayed characters.
/// When the displayed character count reaches the target, returns the raw position.
///
/// Handles these markdown formatting markers:
/// - Bold: `**` or `__`
/// - Italic: `*` or `_` (single, not part of bold)
/// - Code: backticks
/// - Strikethrough: `~~`
/// - Links: `[text](url)` - skips `[`, `](url)` but includes `text`
fn map_displayed_to_raw(displayed_idx: usize, raw_text: &str) -> usize {
    let chars: Vec<char> = raw_text.chars().collect();
    let mut raw_pos = 0;
    let mut displayed_pos = 0;

    while raw_pos < chars.len() {
        // Look at remaining characters from current position
        let remaining: String = chars[raw_pos..].iter().collect();

        // Check for double-character markers first (order matters)
        // Skip these BEFORE checking if we've reached target position
        if remaining.starts_with("**") || remaining.starts_with("__") || remaining.starts_with("~~")
        {
            raw_pos += 2;
            continue;
        }

        // Check for link structure: [text](url) or [text](url "title")
        if chars[raw_pos] == '[' {
            // Skip opening bracket, the text inside will be counted as displayed
            raw_pos += 1;
            continue;
        }

        // Check for link URL part: ](url) or ](url "title")
        if remaining.starts_with("](") {
            // Skip ]( and everything until closing )
            raw_pos += 2; // skip "]("
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

        // Check for single-character markers
        // Note: Must check after ** and __ to avoid false positives
        if chars[raw_pos] == '`' {
            raw_pos += 1;
            continue;
        }

        // Check for italic markers (* or _) that are NOT part of bold
        // Only skip if it looks like a formatting marker (not standalone punctuation)
        if (chars[raw_pos] == '*' || chars[raw_pos] == '_')
            && !remaining.starts_with("**")
            && !remaining.starts_with("__")
        {
            // Check context: is this likely a formatting marker?
            // A marker is usually at word boundaries or paired
            let prev_is_space = raw_pos == 0 || chars[raw_pos - 1].is_whitespace();
            let next_is_space = raw_pos + 1 >= chars.len() || chars[raw_pos + 1].is_whitespace();
            let next_is_same = raw_pos + 1 < chars.len() && chars[raw_pos + 1] == chars[raw_pos];

            // Skip if it looks like a formatting marker (at boundary or paired)
            if prev_is_space || next_is_space || !next_is_same {
                // Check if there's a matching closing marker ahead
                let marker = chars[raw_pos];
                let has_closing = chars[raw_pos + 1..].iter().any(|&c| c == marker);
                if has_closing {
                    raw_pos += 1;
                    continue;
                }
            }
        }

        // NOW check if we've reached the target displayed position
        // This must be AFTER skipping all formatting markers
        if displayed_pos >= displayed_idx {
            return raw_pos;
        }

        // Regular content character - advance both positions
        raw_pos += 1;
        displayed_pos += 1;
    }

    // Return final position (may be at end of raw text)
    raw_pos
}

/// Render inline content (text, links, bold, italic, etc.) with proper formatting.
fn render_inline_content(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
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
    editor_font: &EditorFont,
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

        MarkdownNodeType::Wikilink { target, display } => {
            render_wikilink(ui, colors, font_size, target, display.as_deref());
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

        MarkdownNodeType::Image { url, title } => {
            // Images break out of the inline flow - end the current row and render as block
            ui.end_row();
            render_image(ui, node, colors, font_size, url, title);
        }

        MarkdownNodeType::SoftBreak => {
            let hardbreaks = ui.memory(|mem| {
                mem.data
                    .get_temp::<bool>(egui::Id::new("strict_line_breaks"))
                    .unwrap_or(false)
            });
            if hardbreaks {
                ui.end_row();
            } else {
                ui.label(" ");
            }
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
///
/// This function detects mermaid code blocks and routes them to the specialized
/// mermaid rendering widget for diagram visualization.
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
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;

    // Check if this is a mermaid diagram block
    // Mermaid blocks get special rendering with diagram type detection
    if language.eq_ignore_ascii_case("mermaid") {
        render_mermaid_block(ui, source, edit_state, colors, font_size, literal, node);
        ui.add_space(PARAGRAPH_TRAILING_SPACE_Y);
        return;
    }

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

    // Add left indent and show code block widget.
    // Note: The EditableCodeBlock widget has its own internal horizontal scroll area
    // for the code content, so we don't need an outer scroll wrapper here.
    // We use ui.indent() to add the base indent while preserving proper layout.
    let output = ui
        .indent(code_block_id.with("indent"), |ui| {
            // Override indent amount (default is 18.0 which is too much)
            let saved_indent = ui.spacing().indent;
            ui.spacing_mut().indent = BASE_INDENT;

            let result = EditableCodeBlock::new(&mut code_data)
                .font_size(font_size)
                .dark_mode(dark_mode)
                .colors(widget_colors)
                .id(code_block_id)
                .show(ui);

            ui.spacing_mut().indent = saved_indent;
            result
        })
        .inner;

    // Update stored data
    ui.memory_mut(|mem| {
        mem.data.insert_temp(code_block_id.with("state"), code_data);
    });

    ui.add_space(PARAGRAPH_TRAILING_SPACE_Y);

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

/// Render a mermaid diagram block with specialized visualization.
///
/// Mermaid blocks are detected by the `mermaid` language tag and rendered
/// with diagram type indicators and styled source view. This provides better
/// UX than treating them as regular code blocks.
///
/// # Features
/// - Automatic diagram type detection (flowchart, sequence, class, etc.)
/// - Visual indicator showing the diagram type
/// - Syntax-highlighted source code view
/// - Distinct styling to differentiate from regular code blocks
///
/// # Future Enhancements
/// - SVG rendering via kroki.io API integration
/// - Caching of rendered diagrams
/// - Real-time preview updates
fn render_mermaid_block(
    ui: &mut Ui,
    _source: &mut String,
    _edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    literal: &str,
    node: &MarkdownNode,
) {
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;

    // Determine if we're in dark mode based on the background color
    let dark_mode = colors.background.r() < 128;

    // Create a stable ID for this mermaid block using position info
    let mermaid_block_id = egui::Id::new(("mermaid_block", node.start_line));

    // Convert EditorColors to WidgetColors for the mermaid widget
    let widget_colors = WidgetColors {
        text: colors.text,
        heading: colors.heading,
        code_bg: colors.code_bg,
        list_marker: colors.list_marker,
        muted: colors.quote_text,
    };

    // Store the mermaid block data in egui's memory so it persists across frames
    let mut mermaid_data = ui.memory_mut(|mem| {
        mem.data
            .get_temp_mut_or_insert_with(mermaid_block_id.with("state"), || {
                MermaidBlockData::new(literal)
            })
            .clone()
    });

    // Check if the source content has changed (e.g., edited in raw mode)
    // If so, update the cached data to match the current parsed content.
    if mermaid_data.source != literal {
        mermaid_data = MermaidBlockData::new(literal);
    }

    // Add left indent and show mermaid block widget.
    // Note: The MermaidBlock widget has its own internal horizontal scroll area
    // for the diagram content, so we don't need an outer scroll wrapper here.
    let output = ui
        .indent(mermaid_block_id.with("indent"), |ui| {
            // Override indent amount (default is 18.0 which is too much)
            let saved_indent = ui.spacing().indent;
            ui.spacing_mut().indent = BASE_INDENT;

            let result = MermaidBlock::new(&mut mermaid_data)
                .font_size(font_size)
                .dark_mode(dark_mode)
                .colors(widget_colors)
                .id(mermaid_block_id)
                .show(ui);

            ui.spacing_mut().indent = saved_indent;
            result
        })
        .inner;

    // Update stored data
    ui.memory_mut(|mem| {
        mem.data
            .insert_temp(mermaid_block_id.with("state"), mermaid_data);
    });

    // Log if changes were detected (for debugging)
    if output.changed {
        debug!(
            "Mermaid block at line {} detected change (type: {:?})",
            node.start_line, output.diagram_type
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
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
) {
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;
    const BORDER_WIDTH: f32 = 4.0;
    const BORDER_GAP: f32 = 8.0;

    // Create a stable ID for this blockquote's scroll area
    let blockquote_id = egui::Id::new(("blockquote", node.start_line));

    let available_width = ui.available_width();
    let group_response = ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

        egui::ScrollArea::horizontal()
            .id_source(blockquote_id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
                            paragraph_indent,
                            header_spacing,
                        );
                    }
                });
            });
    });

    // Paint the quote border using the actual rendered content height
    let rect = group_response.response.rect;
    let border_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(BORDER_WIDTH, rect.height()),
    );
    ui.painter()
        .rect_filled(border_rect, 0.0, colors.quote_border);
}

/// Render a callout (GitHub-style admonition) in non-structural mode.
fn render_callout(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
    callout_type: CalloutType,
    custom_title: Option<&str>,
    default_collapsed: bool,
) {
    const BASE_INDENT: f32 = 4.0;
    const BORDER_WIDTH: f32 = 4.0;
    const BORDER_GAP: f32 = 8.0;

    let is_dark = colors.background.r() < 128;
    let (border_color, bg_color, title_color) = callout_colors(callout_type, is_dark);

    // Scope all child widget IDs under a unique ID to prevent collisions
    let scope_id = ("callout_render", node.start_line, node.end_line);

    let group_response = ui.push_id(scope_id, |ui| {
        let callout_id = ui.make_persistent_id("collapsed");
        let is_collapsed = ui.data_mut(|d| *d.get_persisted_mut_or(callout_id, default_collapsed));

        let title_text = custom_title.unwrap_or(callout_type.display_name());
        let icon = callout_type.icon();

        let available_width = ui.available_width();
        let inner = ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

            egui::ScrollArea::horizontal()
                .id_source("scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Title row with icon and collapse toggle
                        let title_row = ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(icon)
                                    .color(title_color)
                                    .font(FontId::proportional(font_size)),
                            );

                            let arrow = if is_collapsed { "▶" } else { "▼" };
                            ui.label(
                                RichText::new(arrow)
                                    .color(title_color)
                                    .font(FontId::proportional(font_size * 0.7)),
                            );

                            ui.label(
                                RichText::new(title_text)
                                    .color(title_color)
                                    .font(FontId::proportional(font_size))
                                    .strong(),
                            );
                        });

                        // Place a clickable rect over the title row for collapse toggle
                        let title_rect = title_row.response.rect;
                        let click_response = ui.allocate_rect(title_rect, egui::Sense::click());
                        if click_response.clicked() {
                            ui.data_mut(|d| {
                                let val = d.get_persisted_mut_or(callout_id, default_collapsed);
                                *val = !*val;
                            });
                        }
                        // Show pointer cursor on hover
                        if click_response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if !is_collapsed {
                            ui.add_space(2.0);
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
                                    paragraph_indent,
                                    header_spacing,
                                );
                            }
                        }
                    });
                });
        });
        inner.response
    });

    // Paint styled background and left border
    let rect = group_response.response.rect;
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(rect.width() - BASE_INDENT, rect.height()),
    );
    ui.painter().rect_filled(content_rect, 4.0, bg_color);

    let border_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + BASE_INDENT, rect.min.y),
        Vec2::new(BORDER_WIDTH, rect.height()),
    );
    ui.painter().rect_filled(border_rect, 2.0, border_color);
}

/// Render a list (ordered or unordered).
fn render_list(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    list_type: &ListType,
) {
    // Add small top margin for top-level lists to separate from preceding content
    // This helps ensure clicks on the first list item don't accidentally hit the element above
    if indent_level == 0 {
        ui.add_space(4.0);
    }

    let mut item_number = match list_type {
        ListType::Ordered { start, .. } => *start,
        ListType::Bullet => 0,
    };

    for (child_idx, child) in node.children.iter().enumerate() {
        // Handle both regular list items (Item) and task list items (TaskItem)
        // Note: In some markdown AST structures, task lists have TaskItem as direct
        // children of List, not wrapped in an Item node
        let should_render = matches!(
            &child.node_type,
            MarkdownNodeType::Item | MarkdownNodeType::TaskItem { .. }
        );

        if should_render {
            let _ = child_idx; // Suppress unused warning
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
    /// Pending cursor position to set after entering edit mode (character index)
    pending_cursor_pos: Option<usize>,
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
    editor_font: &EditorFont,
    indent_level: usize,
    list_type: &ListType,
    item_number: u32,
) {
    // Check if this node IS a TaskItem (direct child of List) or CONTAINS a TaskItem child
    let (is_task, task_checked) = if let MarkdownNodeType::TaskItem { checked } = &node.node_type {
        // The node itself is a TaskItem (task list structure)
        (true, *checked)
    } else {
        // Regular Item - check if it has a TaskItem child
        let task_child = node.children.iter().find_map(|c| {
            if let MarkdownNodeType::TaskItem { checked } = &c.node_type {
                Some(*checked)
            } else {
                None
            }
        });
        (task_child.is_some(), task_child.unwrap_or(false))
    };

    // Find the paragraph node (contains the list item content)
    // For TaskItem nodes, the Paragraph is a direct child
    // For Item nodes, the Paragraph is also a direct child (sibling of TaskItem marker)
    let para_node = node
        .children
        .iter()
        .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

    // Note: Verbose per-frame debug logging removed to fix CPU usage issues on Intel Macs.
    // The original [LIST_ITEM_DEBUG] statements were causing ~50,000 log lines per 22 seconds.
    // See docs/technical/intel-mac-cpu-issue-analysis.md for details.

    // Collect nested lists to render separately
    let nested_lists: Vec<&MarkdownNode> = node
        .children
        .iter()
        .filter(|c| matches!(c.node_type, MarkdownNodeType::List { .. }))
        .collect();

    // Check if paragraph has inline formatting (bold, italic, images, line breaks, etc.)
    // LineBreak must be included here because single-line TextEdit cannot render newlines,
    // and would display them as replacement characters (□). See GitHub issue #41.
    let has_inline_formatting = para_node
        .map(|p| {
            p.children.iter().any(|c| {
                matches!(
                    c.node_type,
                    MarkdownNodeType::Strong
                        | MarkdownNodeType::Emphasis
                        | MarkdownNodeType::Strikethrough
                        | MarkdownNodeType::Link { .. }
                        | MarkdownNodeType::Wikilink { .. }
                        | MarkdownNodeType::Code(_)
                        | MarkdownNodeType::Image { .. }
                        | MarkdownNodeType::LineBreak
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

    // Base indentation to align with content area + nested indent
    // Use 4.0 to match BASE_INDENT used by headings, paragraphs, code blocks, etc.
    let base_indent = 4.0;
    let nested_indent = indent_level as f32 * 20.0;
    let font_family = fonts::get_styled_font_family(false, false, editor_font);

    let available_width = ui.available_width();
    let focus_info: (bool, Option<(usize, usize)>, Option<usize>) =     ui.horizontal(|ui| {
        ui.set_max_width(available_width);

        // Total indentation: base + nested
        ui.add_space(base_indent + nested_indent);

        // Render list marker (bullet, number, or checkbox for tasks)
        if is_task {
            // Use egui Checkbox for task list items - now clickable!
            let mut checked = task_checked;
            let checkbox_response = ui.checkbox(&mut checked, "");

            // Handle checkbox click - toggle the source
            if checkbox_response.changed() {
                // Toggle the task marker in the source
                if let Some(source_line) = source.lines().nth(node.start_line.saturating_sub(1)) {
                    let new_line = if task_checked {
                        // Was checked, now unchecked: [x] -> [ ]
                        source_line.replace("[x]", "[ ]").replace("[X]", "[ ]")
                    } else {
                        // Was unchecked, now checked: [ ] -> [x]
                        source_line.replace("[ ]", "[x]")
                    };
                    update_source_line(source, node.start_line, &new_line);

                    // Mark as modified
                    let node_id = edit_state.add_node(
                        para_node.map(|p| p.text_content()).unwrap_or_default(),
                        node.start_line,
                        node.end_line,
                    );
                    if let Some(editable) = edit_state.get_node_mut(node_id) {
                        editable.modified = true;
                    }
                }
            }
            ui.add_space(2.0);
        } else {
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
                // Create unique ID using para.start_line (matches content extraction)
                // AND item_number for additional uniqueness guarantee
                // FIX: Previously used node.start_line which could differ from para.start_line
                let formatted_item_id = ui
                    .id()
                    .with("formatted_list_item")
                    .with(para.start_line)
                    .with(item_number);

                // Get or create edit state for this formatted item
                let mut item_edit_state = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(formatted_item_id.with("edit_state"), FormattedItemEditState::default)
                        .clone()
                });

                let widget_id = formatted_item_id.with("text_edit");

                if item_edit_state.editing {
                    // EDIT MODE: Show multiline TextEdit with wrapping for raw markdown
                    let font_family_for_layout = font_family.clone();
                    let text_color = colors.text;
                    let mut layouter =
                        move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap.max_width = wrap_width;
                            job.append(
                                text,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: FontId::new(font_size, font_family_for_layout.clone()),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                            ui.fonts(|f| f.layout_job(job))
                        };
                    let text_edit = TextEdit::multiline(&mut item_edit_state.edit_text)
                        .id(widget_id)
                        .font(FontId::new(font_size, font_family.clone()))
                        .text_color(colors.text)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .desired_rows(1)
                        .margin(egui::vec2(0.0, 2.0))
                        .layouter(&mut layouter);

                    // Use show() to get TextEditOutput for cursor manipulation
                    let mut output = text_edit.show(ui);
                    let response = output.response.clone();

                    // Strip newlines that multiline TextEdit inserts on Enter
                    if item_edit_state.edit_text.contains('\n') {
                        item_edit_state.edit_text = item_edit_state.edit_text.replace('\n', "");
                    }

                    // Request focus if needed (first frame after entering edit mode)
                    if item_edit_state.needs_focus {
                        response.request_focus();
                        item_edit_state.needs_focus = false;

                        // Apply pending cursor position if set
                        if let Some(cursor_pos) = item_edit_state.pending_cursor_pos.take() {
                            let ccursor = egui::text::CCursor::new(cursor_pos);
                            let cursor_range = egui::text::CCursorRange::one(ccursor);
                            output.state.cursor.set_char_range(Some(cursor_range));
                            output.state.store(ui.ctx(), widget_id);
                            debug!("[LIST_DEBUG] Set cursor position to {} for list item", cursor_pos);
                        }
                    }

                    // Check for exit conditions:
                    // 1. Focus lost (clicked elsewhere)
                    // 2. Enter key pressed
                    // 3. Escape key pressed (cancel without saving)
                    let enter_pressed = response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    let escape_pressed = response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
                    let focus_lost = response.lost_focus() && !enter_pressed && !escape_pressed;

                    if response.changed() {
                        // Save changes immediately
                        update_source_range(source, para.start_line, para.end_line, &item_edit_state.edit_text);

                        // Mark as modified
                        let node_id = edit_state.add_node(item_edit_state.edit_text.clone(), para.start_line, para.end_line);
                        if let Some(editable) = edit_state.get_node_mut(node_id) {
                            editable.modified = true;
                        }
                    }

                    if enter_pressed || focus_lost {
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
                        // Check if a link widget consumed this click
                        let link_consumed = ui.memory(|mem| {
                            mem.data
                                .get_temp::<bool>(egui::Id::new("link_click_consumed_this_frame"))
                                .unwrap_or(false)
                        });

                        if !link_consumed {
                            // DEBUG: Log detailed click information
                            debug!(
                                "[LIST_DEBUG] CLICK DETECTED on list item: node.start_line={}, para.start_line={}, \
                                 display_rect={:?}, item_number={}",
                                node.start_line, para.start_line, display_response.rect, item_number
                            );

                            // Enter edit mode
                            item_edit_state.editing = true;
                            item_edit_state.needs_focus = true;
                            // Get raw markdown content from source
                            item_edit_state.edit_text = extract_list_item_content(source, para.start_line);

                            // Calculate cursor position from click location using Galley for accuracy
                            // This maps screen position to character index in displayed text
                            let cursor_pos = if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                                let displayed_text = para.text_content();
                                let displayed_idx = compute_displayed_cursor_index(
                                    ui,
                                    &displayed_text,
                                    click_pos,
                                    display_response.rect,
                                    font_size,
                                    editor_font,
                                    &item_edit_state.edit_text,
                                );
                                // Map displayed position to raw position (accounting for formatting markers)
                                let raw_idx = map_displayed_to_raw(displayed_idx, &item_edit_state.edit_text);
                                Some(raw_idx.min(item_edit_state.edit_text.chars().count()))
                            } else {
                                None
                            };
                            item_edit_state.pending_cursor_pos = cursor_pos;

                            // DEBUG: Log the edit state being set
                            debug!(
                                "[LIST_DEBUG] EDIT MODE ENTERED: formatted_item_id uses node.start_line={}, \
                                 extracting content from para.start_line={}, content='{}', cursor_pos={:?}",
                                node.start_line, para.start_line, item_edit_state.edit_text, cursor_pos
                            );

                            // Store the new state
                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(formatted_item_id.with("edit_state"), item_edit_state);
                            });
                        }
                    }

                    // Show hover cursor to indicate clickability
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text - editable
            // Use egui memory to store the edit buffer so it persists across frames
            let edit_buffer_id = ui.id().with("list_item_edit_buffer").with(start_line);
            let edit_tracking_id = ui.id().with("list_item_edit_tracking").with(start_line);

            // Track whether this item was previously focused (to detect focus loss)
            let was_editing = ui.memory(|mem| {
                mem.data.get_temp::<bool>(edit_tracking_id).unwrap_or(false)
            });

            if let Some(editable) = edit_state.get_node_mut(node_id) {
                // Get or initialize the edit buffer from egui memory
                // If not editing yet, initialize from current text
                let mut edit_buffer = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_insert_with(edit_buffer_id, || editable.text.clone())
                        .clone()
                });

                let widget_id = ui.id().with("list_item_text").with(start_line);

                let font_family_for_layout = font_family.clone();
                let text_color = colors.text;
                let mut layouter =
                    move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap_width;
                        job.append(
                            text,
                            0.0,
                            egui::text::TextFormat {
                                font_id: FontId::new(font_size, font_family_for_layout.clone()),
                                color: text_color,
                                ..Default::default()
                            },
                        );
                        ui.fonts(|f| f.layout_job(job))
                    };
                let text_edit = TextEdit::multiline(&mut edit_buffer)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.text)
                    .frame(false)
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter);

                let output = text_edit.show(ui);

                // Strip newlines — list items should not contain literal newlines
                if edit_buffer.contains('\n') {
                    edit_buffer = edit_buffer.replace('\n', "");
                }

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

                ui.memory_mut(|mem| {
                    mem.data.insert_temp(edit_buffer_id, edit_buffer.clone());
                    mem.data.insert_temp(edit_tracking_id, has_focus);
                });

                if output.response.changed() {
                    // Commit the edit buffer to source immediately
                    editable.modified = true;
                    update_source_range(source, start_line, end_line, &edit_buffer);
                }

                // Only commit changes when focus is LOST (was editing, now not)
                // This prevents rebuild during active editing
                if was_editing && !has_focus {
                    // Clear the edit buffer so next edit starts fresh
                    ui.memory_mut(|mem| {
                        mem.data.remove::<String>(edit_buffer_id);
                    });
                }

                // Return focus info for tracking
                return (has_focus, selection, Some(node_id));
            }
        } else {
            // Neither inline formatting path nor simple text path was taken.
            // This can happen with unusual list structures (e.g., list items containing
            // only nested lists, or empty list items). Use debug level since this fires
            // every frame and the fallback handles it gracefully.
            debug!(
                "List item at line {} has no paragraph: has_inline_formatting={}, simple_text_node_id={}, para_node={}, is_task={}",
                node.start_line,
                has_inline_formatting,
                simple_text_node_id.is_some(),
                para_node.is_some(),
                is_task
            );
            // Fallback: try to render any text content we can find
            let fallback_text = node.text_content();
            if !fallback_text.is_empty() {
                debug!(
                    "Fallback render for list item at line {} with text: '{}'",
                    node.start_line,
                    fallback_text.chars().take(50).collect::<String>()
                );
                ui.label(
                    RichText::new(&fallback_text)
                        .color(colors.text)
                        .font(FontId::new(font_size, font_family)),
                );
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
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;

    ui.add_space(4.0); // Vertical spacing above
    ui.horizontal(|ui| {
        ui.add_space(BASE_INDENT); // Horizontal indent
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, colors.hr);
    });
    ui.add_space(4.0); // Vertical spacing below
}

/// Render a table as an editable widget.
fn render_table(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
) {
    // Base left indent to align with paragraphs and headers
    const BASE_INDENT: f32 = 4.0;

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

    // Capture available width BEFORE any layout changes
    let table_avail_width = (ui.available_width() - BASE_INDENT).max(100.0);

    let output = EditableTable::new(&mut table_data)
        .font_size(font_size)
        .colors(widget_colors)
        .with_controls(true)
        .with_alignment_controls(true)
        .id(table_id)
        .max_width(table_avail_width)
        .editor_font(editor_font.clone())
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
    const BASE_INDENT: f32 = 4.0;

    let available_width = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(BASE_INDENT);

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
                        .desired_width(ui.available_width())
                        .interactive(false),
                );
            });
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

    // If the link consumed a click, store a flag so parent handlers can skip edit mode
    if output.click_consumed {
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(egui::Id::new("link_click_consumed_this_frame"), true);
        });
    }

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

/// Render a wikilink as a clickable label.
///
/// Wikilinks are rendered as colored, underlined text (like internal links).
/// If a `WikilinkContext` is available in egui memory, the target is checked
/// for existence — broken links are styled with a dimmed red color.
/// Clicking navigates to the target file. The target is stored in egui memory
/// and picked up by `MarkdownEditorOutput::wikilink_clicked`.
fn render_wikilink(
    ui: &mut Ui,
    colors: &EditorColors,
    font_size: f32,
    target: &str,
    display: Option<&str>,
) {
    let label_text = display.unwrap_or(target);

    // Check if target file exists (using context stored in egui memory)
    let target_exists = ui
        .memory(|mem| {
            mem.data
                .get_temp::<WikilinkContext>(egui::Id::new("wikilink_resolution_context"))
        })
        .map(|ctx| {
            wikilink_target_exists(
                target,
                ctx.current_dir.as_deref(),
                ctx.workspace_root.as_deref(),
            )
        })
        .unwrap_or(true); // Default to "exists" if no context is available

    // Color: green-ish blue for valid links, dimmed red for broken links
    let wikilink_color = if target_exists {
        Color32::from_rgb(
            colors.link.r().saturating_sub(30),
            colors.link.g(),
            colors.link.b().saturating_add(20).min(255),
        )
    } else {
        // Broken link: dimmed red/orange
        Color32::from_rgb(200, 100, 100)
    };

    let mut rich = RichText::new(label_text)
        .color(wikilink_color)
        .font(FontId::proportional(font_size))
        .underline();

    if !target_exists {
        rich = rich.strikethrough();
    }

    let link_response = ui.add(egui::Label::new(rich).sense(egui::Sense::click()));

    let link_rect = link_response.rect;

    // Hand cursor on hover
    if link_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Use manual pointer check (same pattern as RenderedLinkWidget) because
    // the parent paragraph's ui.interact() call swallows Label::clicked().
    let (primary_released, pointer_pos) =
        ui.input(|i| (i.pointer.primary_released(), i.pointer.interact_pos()));
    let was_clicked = primary_released && pointer_pos.map_or(false, |pos| link_rect.contains(pos));

    // Tooltip showing the target and status
    let tooltip = if !target_exists {
        if display.is_some() {
            format!("[[{}]]\nFile not found", target)
        } else {
            "File not found".to_string()
        }
    } else if display.is_some() {
        format!("[[{}]]\nClick to open", target)
    } else {
        "Click to open".to_string()
    };
    link_response.on_hover_text(tooltip);

    // On click: store target in egui memory for the output to pick up,
    // and also mark click as consumed so parent doesn't enter edit mode
    if was_clicked {
        let target_owned = target.to_string();
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(egui::Id::new("wikilink_clicked_target"), target_owned);
            mem.data
                .insert_temp(egui::Id::new("link_click_consumed_this_frame"), true);
        });
    }
}

/// Quick check whether a wikilink target can be resolved to an existing file.
/// Used during rendering to style broken links differently.
fn wikilink_target_exists(
    target: &str,
    current_dir: Option<&std::path::Path>,
    workspace_root: Option<&std::path::Path>,
) -> bool {
    let target = target.trim();
    if target.is_empty() {
        return false;
    }

    let check = |dir: &std::path::Path| -> bool {
        let exact = dir.join(target);
        if exact.is_file() {
            return true;
        }
        if !target.to_lowercase().ends_with(".md") {
            let with_md = dir.join(format!("{}.md", target));
            if with_md.is_file() {
                return true;
            }
        }
        false
    };

    if let Some(dir) = current_dir {
        if check(dir) {
            return true;
        }
    }
    if let Some(root) = workspace_root {
        if check(root) {
            return true;
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Image Rendering
// ─────────────────────────────────────────────────────────────────────────────

/// Cached image data stored in egui memory to avoid reloading every frame.
#[derive(Clone)]
struct CachedImageTexture {
    texture: TextureHandle,
    original_width: u32,
    original_height: u32,
}

/// Result of attempting to load an image — either success or a description of the failure.
#[derive(Clone)]
enum ImageLoadResult {
    Loading,
    Loaded(CachedImageTexture),
    Failed(String),
}

/// Resolve an image URL to an absolute path on disk.
///
/// Resolution order:
/// 1. If URL is a web URL (http/https), returns None (not supported).
/// 2. If URL is an absolute path, uses it directly.
/// 3. Resolves relative to the current document's directory.
/// 4. Falls back to workspace root.
fn resolve_image_path(
    url: &str,
    current_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> Option<PathBuf> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    // Skip web URLs — we only support local images
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        return None;
    }

    // Strip leading file:// protocol if present
    let path_str = url.strip_prefix("file://").unwrap_or(url);

    let path = Path::new(path_str);

    // If absolute path, use directly
    if path.is_absolute() {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        return None;
    }

    // Resolve relative to current document directory
    if let Some(dir) = current_dir {
        let resolved = dir.join(path_str);
        if resolved.is_file() {
            return Some(resolved);
        }
    }

    // Fall back to workspace root
    if let Some(root) = workspace_root {
        let resolved = root.join(path_str);
        if resolved.is_file() {
            return Some(resolved);
        }
    }

    None
}

/// Load an image from disk, decode it, and create an egui texture.
fn load_image_texture(ctx: &egui::Context, path: &Path) -> Result<CachedImageTexture, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {}", e))?;

    let img = image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode: {}", e))?;

    use image::GenericImageView;
    let (width, height) = img.dimensions();
    const MAX_PIXELS: u32 = 4096 * 4096;
    if width.saturating_mul(height) > MAX_PIXELS {
        return Err(format!("Image dimensions too large: {}x{}", width, height));
    }

    let rgba = img.to_rgba8();

    let pixels: Vec<Color32> = rgba
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_image = ColorImage {
        size: [width as usize, height as usize],
        pixels,
    };

    let texture_name = format!("md_img_{}", path.display());
    let texture = ctx.load_texture(&texture_name, color_image, TextureOptions::LINEAR);

    Ok(CachedImageTexture {
        texture,
        original_width: width,
        original_height: height,
    })
}

/// Render a markdown image node.
///
/// Resolves the image path relative to the current document, loads and caches
/// the texture in egui memory, and renders it scaled to fit the available width.
/// Falls back to showing alt text with a placeholder icon on failure.
fn render_image(
    ui: &mut Ui,
    node: &MarkdownNode,
    colors: &EditorColors,
    font_size: f32,
    url: &str,
    title: &str,
) {
    let alt_text = node.text_content();

    // Get file context from egui memory (same context used for wikilinks)
    let wl_ctx: Option<WikilinkContext> = ui.memory(|mem| {
        mem.data
            .get_temp::<WikilinkContext>(egui::Id::new("wikilink_resolution_context"))
    });

    let resolved_path = resolve_image_path(
        url,
        wl_ctx.as_ref().and_then(|c| c.current_dir.as_deref()),
        wl_ctx.as_ref().and_then(|c| c.workspace_root.as_deref()),
    );

    // Web URLs: show placeholder with link text
    if url.starts_with("http://") || url.starts_with("https://") {
        let render_web_images = ui.memory(|mem| {
            mem.data
                .get_temp::<bool>(egui::Id::new("render_web_images"))
                .unwrap_or(false)
        });

        if !render_web_images {
            render_image_placeholder(ui, colors, font_size, &alt_text, "Web images not supported");
            return;
        }

        // Async web image loading
        let cache_id = egui::Id::new("md_web_image_cache").with(url);
        let cached: Option<ImageLoadResult> = ui.data(|d| d.get_temp(cache_id));

        let load_result = match cached {
            Some(res) => res,
            None => {
                // Limit concurrent downloads to prevent thread/network exhaustion
                static ACTIVE_DOWNLOADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                const MAX_CONCURRENT_DOWNLOADS: usize = 4;

                if ACTIVE_DOWNLOADS.load(std::sync::atomic::Ordering::Relaxed) >= MAX_CONCURRENT_DOWNLOADS {
                    render_image_placeholder(ui, colors, font_size, &alt_text, "Queued for download...");
                    ui.ctx().request_repaint(); // keep trying next frame
                    return;
                }

                // Start loading
                ui.data_mut(|d| d.insert_temp(cache_id, ImageLoadResult::Loading));
                ACTIVE_DOWNLOADS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                let ctx = ui.ctx().clone();
                let url = url.to_string();
                let alt_text = alt_text.clone();

                std::thread::spawn(move || {
                    struct DownloadGuard;
                    impl Drop for DownloadGuard {
                        fn drop(&mut self) {
                            ACTIVE_DOWNLOADS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    let _guard = DownloadGuard;

                    let agent = ureq::AgentBuilder::new()
                        .timeout(std::time::Duration::from_secs(10))
                        .build();

                    let result = match agent
                        .get(&url)
                        .set("User-Agent", "Ferrite Markdown Editor")
                        .call()
                    {
                        Ok(resp) => {
                            const MAX_BYTES: u64 = 10 * 1024 * 1024; // 10MB limit
                            let content_len = resp.header("Content-Length").and_then(|l| l.parse::<u64>().ok());
                            if content_len.is_some() && content_len.unwrap() > MAX_BYTES {
                                ImageLoadResult::Failed(format!("Image too large: {} bytes", content_len.unwrap()))
                            } else {
                                use std::io::Read;
                                let mut reader = resp.into_reader().take(MAX_BYTES);
                                let mut bytes = Vec::new();
                                if let Err(e) = reader.read_to_end(&mut bytes) {
                                    ImageLoadResult::Failed(format!("Download error: {}", e))
                                } else if bytes.len() as u64 == MAX_BYTES {
                                    ImageLoadResult::Failed("Image exceeds 10MB limit".to_string())
                                } else {
                                    match image::load_from_memory(&bytes) {
                                        Ok(img) => {
                                            use image::GenericImageView;
                                            let (width, height) = img.dimensions();
                                            const MAX_PIXELS: u32 = 4096 * 4096;
                                            
                                            if width.saturating_mul(height) > MAX_PIXELS {
                                                ImageLoadResult::Failed(format!("Image dimensions too large: {}x{}", width, height))
                                            } else {
                                                let rgba = img.to_rgba8();
                                                let pixels: Vec<egui::Color32> = rgba
                                                    .pixels()
                                                    .map(|p| {
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            p[0], p[1], p[2], p[3],
                                                        )
                                                    })
                                                    .collect();
                                                let color_image = egui::ColorImage {
                                                    size: [width as usize, height as usize],
                                                    pixels,
                                                };
                                                let texture = ctx.load_texture(
                                                    format!("web_img_{}", url),
                                                    color_image,
                                                    egui::TextureOptions::LINEAR,
                                                );
                                                ImageLoadResult::Loaded(CachedImageTexture {
                                                    texture,
                                                    original_width: width,
                                                    original_height: height,
                                                })
                                            }
                                        }
                                        Err(e) => ImageLoadResult::Failed(format!("Decode error: {}", e)),
                                    }
                                }
                            }
                        }
                        Err(e) => ImageLoadResult::Failed(format!("Network error: {}", e)),
                    };

                    ctx.data_mut(|d| d.insert_temp(cache_id, result));
                    ctx.request_repaint();
                });

                ImageLoadResult::Loading
            }
        };

        match load_result {
            ImageLoadResult::Loading => {
                render_image_placeholder(ui, colors, font_size, &alt_text, "Loading...");
                return;
            }
            ImageLoadResult::Loaded(tex) => {
                render_cached_image(ui, &tex, &alt_text, title, url);
                return;
            }
            ImageLoadResult::Failed(msg) => {
                render_image_placeholder(ui, colors, font_size, &alt_text, &msg);
                return;
            }
        }
    }

    let Some(resolved) = resolved_path else {
        let hint = if url.is_empty() {
            "No image path"
        } else {
            "Image not found"
        };
        render_image_placeholder(ui, colors, font_size, &alt_text, hint);
        return;
    };

    // Use the resolved path as a stable cache key
    let cache_id = egui::Id::new("md_image_cache").with(&resolved);

    // Check cache first
    let cached: Option<ImageLoadResult> = ui.data(|d| d.get_temp(cache_id));

    let load_result = cached.unwrap_or_else(|| {
        // Load and cache
        let result = match load_image_texture(ui.ctx(), &resolved) {
            Ok(tex) => ImageLoadResult::Loaded(tex),
            Err(msg) => {
                log::warn!("Failed to load image '{}': {}", url, msg);
                ImageLoadResult::Failed(msg)
            }
        };
        ui.data_mut(|d| d.insert_temp(cache_id, result.clone()));
        result
    });

    match load_result {
        ImageLoadResult::Loading => {
            render_image_placeholder(ui, colors, font_size, &alt_text, "Loading...");
        }
        ImageLoadResult::Loaded(cached_tex) => {
            render_cached_image(ui, &cached_tex, &alt_text, title, url);
        }
        ImageLoadResult::Failed(msg) => {
            render_image_placeholder(ui, colors, font_size, &alt_text, &msg);
        }
    }
}

/// Render an image that has already been loaded into a texture.
fn render_cached_image(
    ui: &mut Ui,
    cached_tex: &CachedImageTexture,
    alt_text: &str,
    title: &str,
    url: &str,
) {
    let available_width = ui.available_width();
    let orig_w = cached_tex.original_width as f32;
    let orig_h = cached_tex.original_height as f32;

    // Scale to fit available width, maintaining aspect ratio
    let (display_w, display_h) = if orig_w > available_width {
        let scale = available_width / orig_w;
        (available_width, orig_h * scale)
    } else {
        (orig_w, orig_h)
    };

    let sized =
        egui::load::SizedTexture::new(cached_tex.texture.id(), Vec2::new(display_w, display_h));
    let image_widget = egui::Image::from_texture(sized);
    let response = ui.add(image_widget);

    // Show tooltip with alt text and/or title on hover
    let tooltip = build_image_tooltip(alt_text, title, url);
    if !tooltip.is_empty() {
        response.on_hover_text(tooltip);
    }
}

/// Build a tooltip string from alt text, title, and URL.
fn build_image_tooltip(alt_text: &str, title: &str, url: &str) -> String {
    let mut parts = Vec::new();
    if !alt_text.is_empty() {
        parts.push(alt_text.to_string());
    }
    if !title.is_empty() && title != alt_text {
        parts.push(title.to_string());
    }
    if !url.is_empty() {
        parts.push(url.to_string());
    }
    parts.join("\n")
}

/// Render a placeholder for images that couldn't be loaded.
/// Shows an icon and the alt text (or an error hint).
fn render_image_placeholder(
    ui: &mut Ui,
    colors: &EditorColors,
    font_size: f32,
    alt_text: &str,
    hint: &str,
) {
    let frame_color = colors.quote_border;
    let bg_color = colors.code_bg;

    egui::Frame::none()
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, frame_color))
        .rounding(4.0)
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Image icon
                ui.label(
                    RichText::new("\u{1F5BC}") // 🖼 framed picture emoji
                        .color(frame_color)
                        .size(font_size * 1.2),
                );

                ui.vertical(|ui| {
                    if !alt_text.is_empty() {
                        ui.label(
                            RichText::new(alt_text)
                                .color(colors.text)
                                .size(font_size)
                                .italics(),
                        );
                    }
                    ui.label(
                        RichText::new(hint)
                            .color(frame_color)
                            .size(font_size * 0.85),
                    );
                });
            });
        });
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
    editor_font: &EditorFont,
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

// ─────────────────────────────────────────────────────────────────────────────
// Memory Cleanup
// ─────────────────────────────────────────────────────────────────────────────

/// Clean up temporary data stored in egui's memory for the rendered markdown editor.
///
/// This function removes all temp data entries for types used by the rendered editor's
/// interactive widgets (headings, paragraphs, lists, code blocks, tables, etc.). These
/// entries are keyed by UI hierarchy IDs combined with line numbers, and can accumulate
/// when switching between tabs or editing documents with varying numbers of elements.
///
/// Call this function when a tab is closed to free memory that would otherwise persist
/// until the application exits.
///
/// # Types Cleaned
/// - `FormattedItemEditState` - Click-to-edit state for paragraphs and list items
/// - `CodeBlockData` - Code block content and edit state
/// - `MermaidBlockData` - Mermaid diagram source and render state
/// - `TableData` - Table cell contents and structure
/// - `TableEditState` - Table cell focus and navigation state
/// - `RenderedLinkState` - Link edit popup state
///
/// # Note
/// This performs a blanket cleanup of ALL entries for these types. When multiple tabs
/// are open, this will also clear temp data for the remaining tabs. This is acceptable
/// because:
/// 1. These are temporary edit buffers - content is preserved in the document source
/// 2. The data is lazily recreated when widgets are rendered
/// 3. At most one tab is typically being actively edited
///
/// # Example
/// ```ignore
/// // In tab close handler:
/// self.state.close_tab(index);
/// cleanup_rendered_editor_memory(ctx);
/// ```
pub fn cleanup_rendered_editor_memory(ctx: &egui::Context) {
    ctx.memory_mut(|mem| {
        // Clean up rendered editor widget temp data
        mem.data.remove_by_type::<FormattedItemEditState>();
        mem.data.remove_by_type::<CodeBlockData>();
        mem.data.remove_by_type::<MermaidBlockData>();
        mem.data.remove_by_type::<TableData>();
        mem.data.remove_by_type::<TableEditState>();
        mem.data.remove_by_type::<RenderedLinkState>();
        mem.data.remove_by_type::<ImageLoadResult>();
    });

    log::debug!("Cleaned up rendered editor temporary memory");
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
        let _styled = style.apply(text, 14.0, &EditorFont::Inter);
        // Just verify it doesn't panic; visual styling tested via egui
    }

    #[test]
    fn test_text_style_apply_with_styles() {
        let style = TextStyle::new().with_bold().with_italic();
        let text = RichText::new("test");
        let _styled = style.apply(text, 14.0, &EditorFont::Inter);
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
