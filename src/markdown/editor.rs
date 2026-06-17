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
use crate::markdown::code_execution::CodeExecutionUi;
use crate::markdown::rendered_session::{
    self, BlockRef, CommitPolicy, PendingActivation, RenderedEditSession,
};
use crate::markdown::rendered_commit_undo;
use crate::markdown::video_embed::rewrite_video_embed_dimensions;
use crate::markdown::video_render::{self, VideoEmbedResizeContext, VideoRenderColors};
use crate::markdown::parser::{
    parse_markdown, CalloutType, HeadingLevel, ListType, MarkdownDocument, MarkdownNode,
    MarkdownNodeType,
    TableAlignment,
};
use crate::markdown::widgets::{
    CodeBlockData, EditableCodeBlock, EditableTable, MermaidBlock, MermaidBlockData,
    RenderedLinkState, RenderedLinkWidget, TableData, TableEditState, WidgetColors,
};
use crate::path_utils::resolve_local_link_path;
use crate::ui::{
    parse_frontmatter_fields, render_nav_buttons, FrontmatterField, FrontmatterValue,
    MermaidPopupState, NavAction,
};
use eframe::egui::{
    self, Color32, ColorImage, FontId, Key, Margin, Response, RichText, ScrollArea, Stroke,
    TextEdit, TextureHandle, TextureOptions, Ui, Vec2,
};
use log::{debug, warn};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Editor Mode
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// The editing mode for the markdown editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Raw markdown text editing mode
    #[default]
    Raw,
    /// WYSIWYG rendered editing mode
    Rendered,
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Editor Output
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Result of showing the markdown editor widget.
pub struct MarkdownEditorOutput {
    /// The egui response from the editor container.
    pub response: Response,
    /// Whether the content was modified.
    pub changed: bool,
    /// True when block-commit undo entries were queued this frame (see `rendered_commit_undo`).
    pub undo_recorded: bool,
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Viewport Culling
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
    /// `(start_line, end_line)` per top-level block — used to reuse layout when only
    /// inline content changes (e.g. task checkbox toggles) without remeasuring.
    block_line_ranges: Vec<(usize, usize)>,
}

fn block_line_ranges_from_nodes(children: &[MarkdownNode]) -> Vec<(usize, usize)> {
    children
        .iter()
        .map(|n| (n.start_line, n.end_line))
        .collect()
}

fn block_structure_matches(ranges: &[(usize, usize)], children: &[MarkdownNode]) -> bool {
    !ranges.is_empty()
        && ranges.len() == children.len()
        && ranges
            .iter()
            .zip(children.iter())
            .all(|(&(s, e), n)| n.start_line == s && n.end_line == e)
}

/// True when the user is actively scrolling (wheel or scrollbar drag), not merely clicking.
fn is_active_scroll_input(ui: &Ui) -> bool {
    ui.input(|i| {
        i.smooth_scroll_delta.y.abs() > 0.5
            || (i.pointer.any_down() && i.pointer.is_decidedly_dragging())
    })
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Block Source Extraction (for per-block height caching)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Rendered View Search Highlight Overlay
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
                continue; // separator line â€” skip
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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Theme Colors
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
    /// Create colors for the given theme and user accent. Hyperlink color stays standard blue.
    pub fn from_theme(theme: Theme, visuals: &egui::Visuals, accent: Color32) -> Self {
        let mut c = match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::System => {
                if visuals.dark_mode {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        };
        let dark = match theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => visuals.dark_mode,
        };
        c.heading = accent;
        c.checkbox = accent;
        c.link = crate::theme::accent::standard_link_color(dark);
        c
    }

    /// Dark theme colors (default accent/link before `from_theme` overrides).
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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Editable Node State
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Structural Edit State
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Markdown Editor Widget
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
    /// User accent (matches Settings.accent_color)
    accent_rgb: [u8; 3],
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
    /// Gating and cwd for running fenced code from the preview (optional).
    code_execution: Option<CodeExecutionUi>,
    /// External-invalidation epoch from tab state; scopes rendered widget ids (see PRD).
    source_epoch: u64,
    /// When true, rendered-view mutations are blocked (preview lock #144).
    preview_locked: bool,
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
            accent_rgb: crate::theme::accent::DEFAULT_ACCENT_RGB,
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
            code_execution: None,
            source_epoch: 0,
            preview_locked: false,
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

    #[must_use]
    pub fn accent_rgb(mut self, rgb: [u8; 3]) -> Self {
        self.accent_rgb = rgb;
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
        self.accent_rgb = settings.accent_color;
        self.code_execution = Some(CodeExecutionUi::from_settings(settings));
        self
    }

    /// Snapshot for code-block **Run** (preview cwd, timeouts, and permission flags).
    #[must_use]
    pub fn code_execution(mut self, ctx: CodeExecutionUi) -> Self {
        self.code_execution = Some(ctx);
        self
    }

    /// Per-tab external invalidation epoch for stable rendered widget ids.
    ///
    /// Bumps only on raw edits, undo/redo, reload, etc. — not on rendered WYSIWYG commits.
    #[must_use]
    pub fn source_epoch(mut self, epoch: u64) -> Self {
        self.source_epoch = epoch;
        self
    }

    /// Block rendered-view WYSIWYG mutations while keeping read-only interactions.
    #[must_use]
    pub fn preview_locked(mut self, locked: bool) -> Self {
        self.preview_locked = locked;
        self
    }

    /// Show the editor widget and return the output.
    pub fn show(self, ui: &mut Ui) -> MarkdownEditorOutput {
        let id = self.id.unwrap_or_else(|| ui.id().with("markdown_editor"));
        let accent = Color32::from_rgb(self.accent_rgb[0], self.accent_rgb[1], self.accent_rgb[2]);
        ui.ctx().data_mut(|d| {
            d.insert_temp(crate::markdown::markdown_accent_temp_id(), accent);
        });
        let colors = EditorColors::from_theme(self.theme, ui.visuals(), accent);

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
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let font_family_clone = font_family.clone();
                let mut layouter = move |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                    let text = buf.as_str();
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
                    ui.fonts_mut(|f| f.layout_job(layout_job))
                };

                TextEdit::multiline(self.content)
                    .id(id)
                    .frame(egui::Frame::NONE)
                    .font(FontId::new(font_size, font_family.clone()))
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .show(ui)
            });

        let text_output = scroll_output.inner;
        let changed = text_output.response.changed();

        let cursor_position = if let Some(cursor_range) = text_output.cursor_range {
            let cursor = cursor_range.primary;
            char_index_to_line_col(self.content, cursor.index)
        } else {
            (0, 0)
        };

        if changed {
            debug!("Raw editor content changed");
        }

        MarkdownEditorOutput {
            response: text_output.response.response,
            changed,
            undo_recorded: false,
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
    ///
    /// Rendered block editing is coordinated by [`RenderedEditSession`](crate::markdown::rendered_session::RenderedEditSession).
    /// See `docs/technical/markdown/rendered-edit-session.md`.
    fn show_rendered_editor(
        self,
        ui: &mut Ui,
        id: egui::Id,
        colors: &EditorColors,
    ) -> MarkdownEditorOutput {
        rendered_commit_undo::begin_frame(ui.ctx());

        ui.ctx().data_mut(|d| {
            d.insert_temp(
                crate::markdown::preview_locked_temp_id(),
                self.preview_locked,
            );
        });

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

        let code_exec_ctx = self
            .code_execution
            .clone()
            .unwrap_or_else(CodeExecutionUi::disabled);
        ui.memory_mut(|mem| {
            mem.data.insert_temp(
                crate::markdown::code_execution::code_execution_ctx_id(),
                code_exec_ctx,
            );
        });

        // Parse the markdown content (cached by blake3 hash â€” skips re-parse when unchanged)
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
            let line_height = ui.fonts_mut(|f| f.row_height(&font_id));
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

        // Ctrl+Scroll Zoom: detect before ScrollArea consumes the scroll events.
        // While a Mermaid popup is open, the popup owns wheel input locally.
        let popup_open = MermaidPopupState::is_open(ui.ctx());
        let ctrl_scroll_zoom: Option<bool> = if popup_open {
            None
        } else {
            ui.input(|i| {
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
            })
        };

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

        // Ctrl+Home / Ctrl+End: jump to document start/end in Rendered view.
        // Inline TextEdits would otherwise consume these keys and only navigate
        // within the focused block, which is not what users expect.
        let (doc_jump_home, doc_jump_end) = ui.input(|i| {
            if !i.modifiers.command {
                return (false, false);
            }
            (i.key_pressed(Key::Home), i.key_pressed(Key::End))
        });
        let doc_edge_scroll: Option<f32> = if doc_jump_home {
            Some(0.0)
        } else if doc_jump_end {
            // Use cached content height from the prior frame's culling state so we
            // land precisely at the bottom. f32::INFINITY breaks egui's scrollbar
            // math; a finite fallback gets clamped to the real max by egui.
            let total_h: Option<f32> = ui.memory(|mem| {
                mem.data
                    .get_temp::<ViewportCullingState>(id.with("viewport_culling"))
                    .map(|s| s.total_height)
            });
            Some(total_h.unwrap_or(1.0e9))
        } else {
            None
        };
        if doc_edge_scroll.is_some() {
            ui.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key {
                            key: Key::Home | Key::End,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.command
                    )
                });
            });
        }

        // Render the document in a scroll area
        let mut scroll_area = ScrollArea::vertical()
            .id_salt(id.with("rendered_scroll"))
            .auto_shrink([false, false]);

        let height_fixup_id = id.with("rendered_height_fixup");
        let scroll_cooldown_id = id.with("rendered_scroll_cooldown");
        let user_scrolling = is_active_scroll_input(ui);
        if user_scrolling {
            ui.memory_mut(|mem| {
                mem.data.insert_temp(scroll_cooldown_id, Instant::now());
            });
            ui.memory_mut(|mem| mem.data.remove::<f32>(height_fixup_id));
        } else if let Some(offset) = ui.memory(|mem| mem.data.get_temp::<f32>(height_fixup_id)) {
            ui.memory_mut(|mem| mem.data.remove::<f32>(height_fixup_id));
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }
        let within_scroll_cooldown = ui
            .memory(|mem| mem.data.get_temp::<Instant>(scroll_cooldown_id))
            .is_some_and(|t| t.elapsed() < Duration::from_millis(200));

        // Priority order for scroll offset:
        // 0. Ctrl+Home / Ctrl+End (highest — explicit user navigation)
        // 1. Nav button scroll (from previous frame)
        // 2. Pending scroll offset from mode switch
        // 3. Target scroll offset from outline navigation
        if let Some(offset) = doc_edge_scroll {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        } else if let Some(offset) = pending_nav_scroll {
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

        // Hash of current content — used only for viewport culling / height-cache invalidation.
        // Widget identity is scoped by (editor_id, source_epoch) so rendered commits do not
        // reset egui focus. See `docs/technical/markdown/rendered-widget-identity.md`.
        let content_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.content.hash(&mut hasher);
            hasher.finish()
        };
        let source_epoch = self.source_epoch;
        let mut rendered_session = rendered_session::load_for_epoch(ui, id, source_epoch);

        if self.preview_locked && rendered_session.active.is_some() {
            let mut noop = |_: BlockRef, _: &rendered_session::BlockEditState| {};
            rendered_session.close_active(CommitPolicy::Discard, &mut noop);
        }

        // Collect line mappings during render for scroll sync
        let mut line_mappings: Vec<LineMapping> = Vec::new();

        // Calculate content width and centering margin. A configured max line
        // width should behave as a responsive reading column in rendered view,
        // not as a left-pinned block outside Zen mode.
        let char_width = self.font_size * 0.6; // Approximate average character width
        let outer_available_width = ui.available_width();

        let (content_margin, effective_content_width) =
            if let Some(max_width_px) = self.max_line_width.to_pixels(char_width) {
                // max_line_width is set - constrain width
                // Cap to available width to prevent overflow
                let effective_width = max_width_px.min(outer_available_width);

                let margin = if outer_available_width > effective_width {
                    (outer_available_width - effective_width) / 2.0
                } else {
                    0.0
                };
                (margin, Some(effective_width))
            } else {
                // No max_line_width set - use full available width, no centering
                (0.0, None)
            };

        // â”€â”€ Viewport culling state â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        let culling_id = id.with("viewport_culling");
        let culling_state: Option<ViewportCullingState> =
            ui.memory(|mem| mem.data.get_temp(culling_id));
        let previous_total_height = culling_state.as_ref().map(|cs| cs.total_height);

        let block_count = doc.root.children.len();
        let block_line_ranges = block_line_ranges_from_nodes(&doc.root.children);
        let block_structure_valid = culling_state.as_ref().map_or(false, |s| {
            s.block_heights.len() == block_count
                && (s.available_width - outer_available_width).abs() < 1.0
                && block_structure_matches(&s.block_line_ranges, &doc.root.children)
        });
        let has_valid_heights = culling_state.as_ref().map_or(false, |s| {
            s.content_hash == content_hash
                && s.block_heights.len() == block_count
                && (s.available_width - outer_available_width).abs() < 1.0
        }) || block_structure_valid;

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

            ui.push_id(id, |ui| {
                ui.push_id(source_epoch, |ui| {
                ui.memory_mut(|mem| {
                    mem.data
                        .insert_temp(session_active_clicked_key(ui), false);
                });
                ui.horizontal(|ui| {
                    if content_margin > 0.0 {
                        ui.add_space(content_margin);
                    }

                    let content_width = effective_content_width.unwrap_or(ui.available_width());
                    ui.vertical(|ui| {
                        ui.set_max_width(content_width);
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, BLOCK_ITEM_SPACING_Y);

                        if has_valid_heights && block_count > 0 {
                            // â”€â”€ Fast path: cull off-screen blocks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
                                    &mut rendered_session,
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
                                    block_line_ranges: block_line_ranges.clone(),
                                });
                                // Still have unmeasured blocks â€” request another frame
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
                            // â”€â”€ Bootstrap / lazy measurement pass â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
                            // NOTE: When block_count == 0 (empty document) the inclusive
                            // range `first_vis..=last_vis` below would otherwise iterate
                            // once and panic on `doc.root.children[0]`. See issue #127.
                            let vis_top = (viewport.min.y - VIEWPORT_OVERSCAN_PX).max(0.0);
                            let vis_bottom = viewport.max.y + VIEWPORT_OVERSCAN_PX;

                            let first_vis = boot_start_y
                                .partition_point(|&y| y <= vis_top)
                                .saturating_sub(1);
                            let last_vis = boot_start_y
                                .partition_point(|&y| y < vis_bottom)
                                .min(block_count)
                                .saturating_sub(1);

                            if block_count > 0 && first_vis > 0 {
                                let pre = (boot_start_y[first_vis] - BLOCK_ITEM_SPACING_Y).max(0.0);
                                ui.allocate_space(Vec2::new(content_width, pre));
                            }

                            let mut new_measures: usize = 0;
                            let boot_is_dark = ui.visuals().dark_mode;
                            // Use a half-open range so an empty document
                            // (block_count == 0) yields an empty iterator
                            // rather than accessing children[0].
                            let render_end = (last_vis + 1).min(block_count);
                            for i in first_vis..render_end {
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
                                        &mut rendered_session,
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
                                block_line_ranges: block_line_ranges.clone(),
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

                session_dismiss_if_clicked_outside(
                    ui,
                    &mut rendered_session,
                    self.content,
                    &mut edit_state,
                );

                ui.allocate_response(Vec2::ZERO, egui::Sense::focusable_noninteractive())
                })
                .inner
            })
            .inner
        });

        rendered_session::save_for_epoch(ui, id, source_epoch, rendered_session);

        // â”€â”€ Persist culling state â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        if let Some(cs) = new_culling {
            if let Some(old_total) = previous_total_height {
                if (old_total - cs.total_height).abs() > 1.0 {
                    let viewport_h = scroll_output.inner_rect.height();
                    let max_old = (old_total - viewport_h).max(0.0);
                    let max_new = (cs.total_height - viewport_h).max(0.0);
                    let cur = scroll_output.state.offset.y;
                    let still_scrolling = is_active_scroll_input(ui);
                    if !still_scrolling
                        && !within_scroll_cooldown
                        && max_old > 1.0
                        && max_new > 0.0
                    {
                        let ratio = (cur / max_old).clamp(0.0, 1.0);
                        let corrected = ratio * max_new;
                        if (corrected - cur).abs() > 1.5 {
                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(height_fixup_id, corrected);
                            });
                            ui.ctx().request_repaint();
                        }
                    }
                }
            }
            ui.memory_mut(|mem| {
                mem.data.insert_temp(culling_id, cs);
            });
        } else if block_structure_valid {
            if let Some(ref cs) = culling_state {
                if cs.content_hash != content_hash {
                    let mut updated = cs.clone();
                    updated.content_hash = content_hash;
                    ui.memory_mut(|mem| {
                        mem.data.insert_temp(culling_id, updated);
                    });
                }
            }
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
            if !self.preview_locked && pending_edit.performed {
                *self.content = pending_edit.new_source;
                structural_changed = true;
                debug!(
                    "Applied structural edit, cursor at line {}",
                    pending_edit.cursor_position.line
                );
            }
        }

        // Check if any nodes were modified and rebuild markdown if needed
        let content_changed = edit_state.any_modified() && !self.preview_locked;
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
            undo_recorded: rendered_commit_undo::had_commits(ui.ctx()),
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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Node Rendering
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Render a markdown node with structural key handling.
/// This wraps the standard rendering and adds detection of Enter, Backspace, Tab, Shift+Tab
/// to enable word processor-like editing behavior.
fn render_node_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
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
            render_heading(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                *level,
                header_spacing,
                true,
            );
        }
        MarkdownNodeType::Paragraph => {
            render_paragraph_with_structural_keys(
                ui,
                node,
                source,
                edit_state,
                session,
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
                session,
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
                session,
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
                session,
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
            render_table(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
            );
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::AlignedDiv { alignment } => {
            render_aligned_div(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                *alignment,
            );
        }
        MarkdownNodeType::Details { summary, open } => {
            render_details(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                summary,
                *open,
            );
        }
        MarkdownNodeType::LineBreak => {
            ui.add_space(font_size * 0.4);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            render_unrecognized_html_block(ui, colors, html);
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
                    session,
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
        MarkdownNodeType::Image {
            url,
            title,
            width,
            height,
        } => {
            render_image(ui, node, colors, font_size, url, title, *width, *height);
        }
        MarkdownNodeType::VideoEmbed(info) => {
            render_video_embed(ui, node, info, source, edit_state, colors, font_size);
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
    session: &mut RenderedEditSession,
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
                session,
                colors,
                font_size,
                editor_font,
                *level,
                header_spacing,
                false,
            );
        }
        MarkdownNodeType::Paragraph => {
            render_paragraph(
                ui,
                node,
                source,
                edit_state,
                session,
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
                session,
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
                session,
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
                session,
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
            render_table(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
            );
        }
        MarkdownNodeType::FrontMatter(content) => {
            render_front_matter(ui, colors, font_size, content);
        }
        MarkdownNodeType::AlignedDiv { alignment } => {
            render_aligned_div(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                *alignment,
            );
        }
        MarkdownNodeType::Details { summary, open } => {
            render_details(
                ui,
                node,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                indent_level,
                paragraph_indent,
                header_spacing,
                summary,
                *open,
            );
        }
        MarkdownNodeType::LineBreak => {
            ui.add_space(font_size * 0.4);
        }
        MarkdownNodeType::HtmlBlock(html) => {
            render_unrecognized_html_block(ui, colors, html);
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
                    session,
                    colors,
                    font_size,
                    editor_font,
                    indent_level,
                    paragraph_indent,
                    header_spacing,
                );
            }
        }
        MarkdownNodeType::Image {
            url,
            title,
            width,
            height,
        } => {
            render_image(ui, node, colors, font_size, url, title, *width, *height);
        }
        MarkdownNodeType::VideoEmbed(info) => {
            render_video_embed(ui, node, info, source, edit_state, colors, font_size);
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

/// Derive heading level from the `#` prefix on a source line (for session commits).
fn heading_level_from_source(source: &str, line: usize) -> HeadingLevel {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .map(|l| {
            let count = l
                .chars()
                .take_while(|&c| c == '#')
                .count()
                .clamp(1, 6) as u8;
            HeadingLevel::from(count)
        })
        .unwrap_or(HeadingLevel::H1)
}

fn mark_line_modified(edit_state: &mut EditState, line: usize) {
    if let Some(node) = edit_state.nodes.iter_mut().find(|n| n.start_line == line) {
        node.modified = true;
    }
}

/// Line count for committed block text (`lines()` ignores a trailing newline).
fn committed_block_line_count(content: &str) -> usize {
    if content.is_empty() {
        0
    } else {
        content.lines().count()
    }
}

/// 1-indexed inclusive end line for replacing a block in source.
///
/// Merges the AST span (`ast_end_line`) with the committed buffer span so growing
/// replaces trailing lines that would otherwise be kept, and shrinking still clears
/// lines that were part of the old multi-line block.
fn block_replace_end_line(start_line: usize, ast_end_line: usize, new_content: &str) -> usize {
    let committed_line_count = committed_block_line_count(new_content);
    let committed_end = if committed_line_count == 0 {
        start_line.saturating_sub(1)
    } else {
        start_line.saturating_add(committed_line_count.saturating_sub(1))
    };
    ast_end_line.max(committed_end)
}

fn mark_block_modified(edit_state: &mut EditState, line: usize, end_line: usize) {
    if let Some(node) = edit_state.nodes.iter_mut().find(|n| n.start_line == line) {
        node.modified = true;
        node.end_line = end_line;
    }
}

/// Write a session block buffer to source (headings, paragraphs, list items, formatted variants).
///
/// For [`BlockRef::TableCell`] the buffer is not committed directly — the cell text lives in the
/// `EditableTable` widget, not in `BlockEditState`. Instead this records a one-shot force-commit
/// signal so the table flushes its buffered edits on its next render.
fn write_session_block_to_source(
    ctx: &egui::Context,
    block: BlockRef,
    state: &rendered_session::BlockEditState,
    source: &mut String,
    edit_state: &mut EditState,
) {
    match block {
        BlockRef::Heading { line, .. } => {
            let commit_level = heading_level_from_source(source, line);
            update_source_line(
                source,
                line,
                &format_heading(&state.text, commit_level),
            );
            mark_line_modified(edit_state, line);
        }
        BlockRef::Paragraph { line } | BlockRef::FormattedParagraph { line, .. } => {
            let ast_end_line = edit_state
                .nodes
                .iter()
                .find(|n| n.start_line == line)
                .map(|n| n.end_line)
                .unwrap_or(line);
            let end_line = block_replace_end_line(line, ast_end_line, &state.text);
            update_source_range(source, line, end_line, &state.text);
            mark_block_modified(edit_state, line, end_line);
        }
        BlockRef::ListItem { line, .. } | BlockRef::FormattedListItem { line, .. } => {
            let text = state.text.replace('\n', "");
            let ast_end_line = edit_state
                .nodes
                .iter()
                .find(|n| n.start_line == line)
                .map(|n| n.end_line)
                .unwrap_or(line);
            let end_line = block_replace_end_line(line, ast_end_line, &text);
            update_source_range(source, line, end_line, &text);
            mark_block_modified(edit_state, line, end_line);
        }
        BlockRef::TableCell { table_line, .. } => {
            // Table cell text is owned by EditableTable, not BlockEditState. Signal the
            // table to flush dirty edits on its next render so leaving a cell for a
            // heading/paragraph/list item commits buffered cell changes synchronously.
            crate::markdown::widgets::signal_table_force_commit(ctx, table_line);
        }
    }
}

fn block_start_line(block: BlockRef) -> usize {
    match block {
        BlockRef::Heading { line, .. }
        | BlockRef::Paragraph { line }
        | BlockRef::ListItem { line, .. }
        | BlockRef::FormattedParagraph { line, .. }
        | BlockRef::FormattedListItem { line, .. } => line,
        BlockRef::TableCell { table_line, .. } => table_line,
    }
}

fn find_block_node_at_line<'a>(node: &'a MarkdownNode, line: usize) -> Option<&'a MarkdownNode> {
    if node.start_line == line {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_block_node_at_line(child, line) {
            return Some(found);
        }
    }
    None
}

/// Seed one AST node into `edit_state` so flush commits can resolve block spans.
fn seed_edit_state_for_active_block(
    edit_state: &mut EditState,
    doc: &MarkdownDocument,
    block: BlockRef,
    source: &str,
) {
    let line = block_start_line(block);
    let Some(node) = find_block_node_at_line(&doc.root, line) else {
        return;
    };
    let text = match block {
        BlockRef::Heading { .. } => node.text_content(),
        BlockRef::ListItem { .. } | BlockRef::FormattedListItem { .. } => {
            extract_list_item_content(source, line)
        }
        _ => extract_paragraph_content(source, node.start_line, node.end_line),
    };
    edit_state.add_node(text, node.start_line, node.end_line);
}

/// Flush any dirty active rendered-edit block into `tab.content`.
///
/// Call before view-mode toggle, tab switch, save, close, or window focus loss so
/// in-progress WYSIWYG edits reach source without requiring focus to leave the block.
pub fn flush_rendered_edit_session(ctx: &egui::Context, tab: &mut crate::state::Tab) -> bool {
    use crate::config::ViewMode;

    if !matches!(tab.view_mode, ViewMode::Rendered | ViewMode::Split) {
        return false;
    }
    if tab.is_special() || tab.is_large_file() {
        return false;
    }

    let editor_id = rendered_session::rendered_editor_id(tab.id);
    let source_epoch = tab.source_epoch();
    let mut session = rendered_session::load_for_epoch_ctx(ctx, editor_id, source_epoch);

    let active = match session.active {
        Some(block) => block,
        None => return false,
    };
    let is_dirty = session
        .blocks
        .get(&active)
        .map(|s| s.dirty)
        .unwrap_or(false);
    if !is_dirty {
        return false;
    }

    ctx.data_mut(|d| {
        d.insert_temp(
            crate::markdown::preview_locked_temp_id(),
            tab.is_preview_locked(),
        );
    });

    rendered_commit_undo::begin_frame(ctx);
    tab.prepare_undo_snapshot_hashed();

    let mut edit_state = EditState::new();
    if let Ok(doc) = cache::get_or_parse(&tab.content) {
        seed_edit_state_for_active_block(&mut edit_state, &doc, active, &tab.content);
    }

    let mut commit =
        |block: BlockRef, state: &rendered_session::BlockEditState| {
            commit_session_block(ctx, block, state, &mut tab.content, &mut edit_state);
        };
    session.commit_active(&mut commit);

    rendered_session::save_for_epoch_ctx(ctx, editor_id, source_epoch, session);
    tab.apply_rendered_commit_undo_entries(rendered_commit_undo::take_pending_commits(ctx));

    true
}

/// Commit a session block to source and enqueue one logical undo step (see `rendered_commit_undo`).
fn commit_session_block(
    ctx: &egui::Context,
    block: BlockRef,
    state: &rendered_session::BlockEditState,
    source: &mut String,
    edit_state: &mut EditState,
) {
    if crate::markdown::preview_locked_from_ctx(ctx) {
        return;
    }
    log::trace!("commit_block: {:?}", block);
    if matches!(block, BlockRef::TableCell { .. }) {
        // Table flush may not mutate source on this frame; undo is recorded when the table
        // writes markdown in `render_table`.
        write_session_block_to_source(ctx, block, state, source, edit_state);
        return;
    }
    rendered_commit_undo::record_source_commit(ctx, source, |source| {
        write_session_block_to_source(ctx, block, state, source, edit_state);
    });
}

/// Switch active block and break the undo group before committing the previous block.
fn session_switch_to_ui(
    ui: &mut Ui,
    session: &mut RenderedEditSession,
    block: BlockRef,
    activation: PendingActivation,
    source: &mut String,
    edit_state: &mut EditState,
) {
    if crate::markdown::preview_locked_from_ui(ui) {
        return;
    }
    rendered_commit_undo::mark_break_before_next_commit(ui.ctx());
    let ctx = ui.ctx().clone();
    let mut commit = |block: BlockRef, state: &rendered_session::BlockEditState| {
        commit_session_block(&ctx, block, state, source, edit_state);
    };
    session.switch_to_ui(ui, block, activation, &mut commit);
}

/// Reload a formatted block's session buffer from source (Escape / discard path).
fn reload_formatted_block_from_source(
    block: BlockRef,
    state: &mut rendered_session::BlockEditState,
    source: &str,
    edit_state: &EditState,
) {
    match block {
        BlockRef::FormattedParagraph { line, .. } => {
            let end_line = edit_state
                .nodes
                .iter()
                .find(|n| n.start_line == line)
                .map(|n| n.end_line)
                .unwrap_or(line);
            state.text = extract_paragraph_content(source, line, end_line);
        }
        BlockRef::FormattedListItem { line, .. } => {
            state.text = extract_list_item_content(source, line);
        }
        _ => {}
    }
}

/// Reload a plain paragraph or list item session buffer from source (Escape / discard path).
fn reload_plain_block_from_source(
    block: BlockRef,
    state: &mut rendered_session::BlockEditState,
    source: &str,
    edit_state: &EditState,
) {
    match block {
        BlockRef::Paragraph { line } => {
            let end_line = edit_state
                .nodes
                .iter()
                .find(|n| n.start_line == line)
                .map(|n| n.end_line)
                .unwrap_or(line);
            state.text = extract_paragraph_content(source, line, end_line);
        }
        BlockRef::ListItem { line, .. } => {
            state.text = extract_list_item_content(source, line);
        }
        _ => {}
    }
}

/// Consume Enter before TextEdit so plain Enter does not insert a structural newline.
///
/// Lists: any Enter commits+exits. Paragraphs: plain Enter commits+exits; Shift+Enter
/// inserts a soft line break preserved on commit (same model as formatted blocks).
fn consume_plain_block_enter(ui: &mut Ui, session: &RenderedEditSession, block_ref: BlockRef, strip_newlines: bool) {
    if crate::markdown::preview_locked_from_ui(ui) || session.active != Some(block_ref) {
        return;
    }
    ui.input_mut(|i| {
        if strip_newlines {
            let _ = i.consume_key(egui::Modifiers::NONE, Key::Enter);
            let _ = i.consume_key(egui::Modifiers::SHIFT, Key::Enter);
        } else if !i.modifiers.shift {
            let _ = i.consume_key(egui::Modifiers::NONE, Key::Enter);
        }
    });
}

/// Stable Id for the per-frame "active block was clicked" flag.
///
/// Must be stable across all `ui` scopes — `ui.id()` differs depending on
/// `ui.horizontal/vertical/scope` nesting depth, so deriving the key from `ui.id()`
/// caused writes (from deeply nested blocks) and reads (from the outer
/// `session_dismiss_if_clicked_outside` scope) to land on different keys. Only one
/// rendered editor renders per frame, so a process-global Id is safe.
fn session_active_clicked_key(_ui: &Ui) -> egui::Id {
    egui::Id::new("ferrite_rendered_session_active_clicked")
}

fn note_session_active_clicked(ui: &mut Ui, block_ref: BlockRef, session: &RenderedEditSession, response: &Response) {
    if session.active == Some(block_ref) && response.clicked() {
        ui.memory_mut(|mem| {
            mem.data.insert_temp(session_active_clicked_key(ui), true);
        });
    }
}

/// Commit and close the active session block when the user clicks outside it.
///
/// Rect-based hit tests are unreliable — multiline TextEdit `response.rect` can extend
/// below the visible text and swallow clicks meant for blocks underneath. Instead we
/// track whether the active block received `response.clicked()` this frame.
fn session_dismiss_if_clicked_outside(
    ui: &mut Ui,
    session: &mut RenderedEditSession,
    source: &mut String,
    edit_state: &mut EditState,
) {
    if session.active.is_none() {
        return;
    }

    if !ui.input(|i| i.pointer.any_click()) {
        return;
    }

    if ui
        .memory(|mem| mem.data.get_temp::<bool>(session_active_clicked_key(ui)))
        .unwrap_or(false)
    {
        return;
    }

    let ctx = ui.ctx().clone();
    let mut commit =
        |block: BlockRef, state: &rendered_session::BlockEditState| {
            commit_session_block(&ctx, block, state, source, edit_state);
        };
    session.close_active_ui(ui, CommitPolicy::SaveIfDirty, &mut commit);
}

/// Ensure a formatted block has a session buffer seeded from raw source.
///
/// Called once per render before display/edit dispatch so the buffer is always ready
/// (e.g. on first paint, after `invalidate_buffers` from a source-epoch bump).
fn ensure_formatted_block_initialized(
    session: &mut RenderedEditSession,
    block_ref: BlockRef,
    cold_text: String,
) {
    if !session.blocks.contains_key(&block_ref) {
        session.on_text_changed(block_ref, cold_text);
        // `on_text_changed` marks dirty; this is a cold seed, not a user edit.
        if let Some(state) = session.blocks.get_mut(&block_ref) {
            state.dirty = false;
        }
    }
}

/// Session-backed raw-markdown TextEdit for formatted blocks (paragraphs / list items).
///
/// Renders the multiline TextEdit bound to `session.blocks[block_ref].text`, applies any
/// `PendingActivation` (focus + cursor), and handles Enter (commit + return to display),
/// Escape (discard + return to display), and lost_focus (commit). Caller is responsible
/// for parent layout (horizontal row, indent space).
fn render_session_formatted_edit_text(
    ui: &mut Ui,
    block_ref: BlockRef,
    session: &mut RenderedEditSession,
    source: &mut String,
    edit_state: &mut EditState,
    font_size: f32,
    font_family: egui::FontFamily,
    text_color: egui::Color32,
    leading: f32,
    editor_font: &EditorFont,
    strip_newlines: bool,
) -> (bool, Option<(usize, usize)>) {
    let preview_locked = crate::markdown::preview_locked_from_ui(ui);
    let widget_id = block_ref.widget_id(ui);

    let ctx = ui.ctx().clone();
    let mut commit_block = |block: BlockRef, state: &rendered_session::BlockEditState| {
        commit_session_block(&ctx, block, state, source, edit_state);
    };

    let font_family_clone = font_family.clone();
    let leading_for_layout = leading;
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let text = buf.as_str();
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;
        job.append(
            text,
            leading_for_layout,
            egui::text::TextFormat {
                font_id: FontId::new(font_size, font_family_clone.clone()),
                color: text_color,
                ..Default::default()
            },
        );
        ui.fonts_mut(|f| f.layout_job(job))
    };

    let mut output = {
        let block_state = session
            .blocks
            .get_mut(&block_ref)
            .expect("formatted block initialized above");
        if strip_newlines && block_state.text.contains('\n') {
            block_state.text = block_state.text.replace('\n', "");
        }
        TextEdit::multiline(&mut block_state.text)
            .id(widget_id)
            .font(FontId::new(font_size, font_family))
            .text_color(text_color)
            .frame(egui::Frame::NONE)
            .margin(egui::vec2(0.0, 0.0))
            .desired_width(ui.available_width())
            .desired_rows(1)
            .interactive(!preview_locked)
            .layouter(&mut layouter)
            .show(ui)
    };

    let response = output.response.clone();

    if preview_locked {
        if let Some(state) = session.blocks.get_mut(&block_ref) {
            state.formatted_editing = false;
        }
    }

    if let Some(activation) = session
        .blocks
        .get_mut(&block_ref)
        .and_then(|s| s.pending_activation.take())
    {
        if activation.request_focus {
            response.request_focus();
        }
        if let Some(pos) = activation.cursor_char_index {
            let ccursor = egui::text::CCursor::new(pos);
            output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
            output.state.store(ui.ctx(), widget_id);
        }
    }

    if response.changed() && !preview_locked {
        let mut text = session
            .blocks
            .get(&block_ref)
            .map(|s| s.text.clone())
            .unwrap_or_default();
        if strip_newlines && text.contains('\n') {
            text = text.replace('\n', "");
        }
        session.on_text_changed(block_ref, text);
    }

    let has_focus_now = response.has_focus();
    // Lists strip newlines anyway, so any Enter exits. Paragraphs let Shift+Enter
    // insert a newline (preserved on commit) and plain Enter commits + exits.
    let enter_pressed = has_focus_now
        && ui.input(|i| i.key_pressed(Key::Enter) && (strip_newlines || !i.modifiers.shift));
    let escape_pressed = has_focus_now && ui.input(|i| i.key_pressed(Key::Escape));

    if enter_pressed {
        log::trace!(
            "session formatted: enter -> commit + display {:?}",
            block_ref
        );
        session.close_active_ui(ui, CommitPolicy::SaveIfDirty, &mut commit_block);
    } else if escape_pressed {
        log::trace!(
            "session formatted: escape -> discard + display {:?}",
            block_ref
        );
        let mut reload = |blk: BlockRef, state: &mut rendered_session::BlockEditState| {
            reload_formatted_block_from_source(blk, state, source, edit_state);
        };
        session.discard_active(&mut reload);
        // `discard_active` does not clear `active` or surrender focus.
        session.active = None;
        block_ref.surrender_focus(ui);
    } else if session.active == Some(block_ref) && response.lost_focus() {
        session.close_active_ui(ui, CommitPolicy::SaveIfDirty, &mut commit_block);
    } else if session.active != Some(block_ref) && response.has_focus() {
        // Focus arrived by Tab cycling or focus_id replay — record as active.
        session_switch_to_ui(
            ui,
            session,
            block_ref,
            PendingActivation {
                cursor_char_index: None,
                request_focus: false,
            },
            source,
            edit_state,
        );
    }

    note_session_active_clicked(ui, block_ref, session, &response);

    let _ = editor_font;

    let has_focus = response.has_focus();
    let selection = if has_focus {
        output.cursor_range.map(|range| {
            let primary = range.primary.index;
            let secondary = range.secondary.index;
            if primary < secondary {
                (primary, secondary)
            } else {
                (secondary, primary)
            }
        })
    } else {
        None
    };
    (has_focus, selection)
}

/// Activate a formatted block from a display-area click: switch session, queue cursor.
///
/// Uses the [`FormattedBlockLayout`](crate::markdown::rendered_session::FormattedBlockLayout)
/// stored during paint so hit-testing runs on the same galley as the painted display.
fn reset_session_block_if_locked(
    ui: &Ui,
    session: &mut RenderedEditSession,
    block_ref: BlockRef,
    source_text: &str,
) {
    if !crate::markdown::preview_locked_from_ui(ui) {
        return;
    }
    if let Some(state) = session.blocks.get_mut(&block_ref) {
        state.text = source_text.to_string();
        state.dirty = false;
        state.formatted_editing = false;
    }
}

fn enter_formatted_edit_on_display_click(
    ui: &mut Ui,
    block_ref: BlockRef,
    session: &mut RenderedEditSession,
    source: &mut String,
    edit_state: &mut EditState,
    display_rect: egui::Rect,
    leading_indent: f32,
    raw_text: &str,
    font_size: f32,
    editor_font: &EditorFont,
    text_color: Color32,
    link_color: Color32,
    code_bg: Color32,
) {
    if crate::markdown::preview_locked_from_ui(ui) {
        return;
    }

    // Bail if a link widget consumed this click first.
    let link_consumed = ui.memory(|mem| {
        mem.data
            .get_temp::<bool>(egui::Id::new("link_click_consumed_this_frame"))
            .unwrap_or(false)
    });
    if link_consumed {
        return;
    }

    let Some(layout) = rendered_session::layout_for_formatted_click(
        ui,
        session,
        block_ref,
        rendered_session::FormattedBlockLayoutParams {
            raw_text,
            font_size,
            editor_font,
            text_color,
            link_color,
            code_bg,
        },
    ) else {
        log::warn!(
            "formatted display click without layout_wrap_width for {:?}",
            block_ref
        );
        return;
    };

    let cursor_pos = ui.ctx().input(|i| i.pointer.interact_pos()).map(|click_pos| {
        layout.raw_cursor_at(click_pos, display_rect, leading_indent)
    });

    session_switch_to_ui(
        ui,
        session,
        block_ref,
        PendingActivation {
            cursor_char_index: cursor_pos,
            request_focus: true,
        },
        source,
        edit_state,
    );
    if let Some(state) = session.blocks.get_mut(&block_ref) {
        state.formatted_editing = true;
    }
    // The display widget that received the click is not the active block's TextEdit
    // (the TextEdit only renders next frame), so `note_session_active_clicked` is
    // never reached for it. Mark the click here so the end-of-frame
    // `session_dismiss_if_clicked_outside` does not immediately close the block we
    // just switched into edit mode.
    mark_session_active_clicked_if_clicked(ui);
    log::trace!(
        "session formatted: display click -> edit {:?} cursor={:?}",
        block_ref,
        cursor_pos
    );
}

/// Session-backed multiline TextEdit for plain paragraphs and simple list items.
fn render_session_plain_text_block(
    ui: &mut Ui,
    block_ref: BlockRef,
    session: &mut RenderedEditSession,
    source: &mut String,
    edit_state: &mut EditState,
    _end_line: usize,
    cold_text: String,
    font_size: f32,
    font_family: egui::FontFamily,
    text_color: egui::Color32,
    leading: f32,
    editor_font: &EditorFont,
    strip_newlines: bool,
) -> (bool, Option<(usize, usize)>) {
    if !session.blocks.contains_key(&block_ref) {
        session.on_text_changed(block_ref, cold_text.clone());
        if let Some(state) = session.blocks.get_mut(&block_ref) {
            state.dirty = false;
        }
    } else if session.active != Some(block_ref) {
        if let Some(state) = session.blocks.get_mut(&block_ref) {
            if !state.dirty && state.text != cold_text {
                state.text = cold_text.clone();
            }
        }
    }
    reset_session_block_if_locked(ui, session, block_ref, &cold_text);

    let preview_locked = crate::markdown::preview_locked_from_ui(ui);
    let widget_id = block_ref.widget_id(ui);

    consume_plain_block_enter(ui, session, block_ref, strip_newlines);

    let font_family_clone = font_family.clone();
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let text = buf.as_str();
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;
        job.append(
            text,
            leading,
            egui::text::TextFormat {
                font_id: FontId::new(font_size, font_family_clone.clone()),
                color: text_color,
                ..Default::default()
            },
        );
        ui.fonts_mut(|f| f.layout_job(job))
    };

    let mut output = {
        let block_state = session
            .blocks
            .get_mut(&block_ref)
            .expect("session block initialized above");
        if strip_newlines && block_state.text.contains('\n') {
            block_state.text = block_state.text.replace('\n', "");
        }
        TextEdit::multiline(&mut block_state.text)
            .id(widget_id)
            .font(FontId::new(font_size, font_family))
            .text_color(text_color)
            .frame(egui::Frame::NONE)
            .margin(egui::vec2(0.0, 0.0))
            .desired_width(ui.available_width())
            .desired_rows(1)
            .interactive(!preview_locked)
            .layouter(&mut layouter)
            .show(ui)
    };

    let response = &output.response;
    let buffer_snapshot = session
        .blocks
        .get(&block_ref)
        .map(|s| s.text.clone())
        .unwrap_or_default();

    if response.has_focus() {
        if session.active != Some(block_ref) {
            session_switch_to_ui(
                ui,
                session,
                block_ref,
                PendingActivation {
                    cursor_char_index: None,
                    request_focus: false,
                },
                source,
                edit_state,
            );
        }
    } else if session.active.is_some_and(|a| a != block_ref)
        && (response.clicked()
            || (response.hovered() && ui.input(|i| i.pointer.primary_pressed())))
    {
        let cursor_idx =
            heading_click_cursor(ui, response, &buffer_snapshot, font_size, editor_font);
        session_switch_to_ui(
            ui,
            session,
            block_ref,
            PendingActivation {
                cursor_char_index: cursor_idx,
                request_focus: true,
            },
            source,
            edit_state,
        );
        response.request_focus();
    }

    if let Some(activation) = session
        .blocks
        .get_mut(&block_ref)
        .and_then(|s| s.pending_activation.take())
    {
        if activation.request_focus {
            response.request_focus();
        }
        if let Some(pos) = activation.cursor_char_index {
            let ccursor = egui::text::CCursor::new(pos);
            output.state.cursor.set_char_range(Some(
                egui::text::CCursorRange::one(ccursor),
            ));
            output.state.store(ui.ctx(), widget_id);
        }
    }

    if response.changed() && !preview_locked {
        let mut text = session
            .blocks
            .get(&block_ref)
            .map(|s| s.text.clone())
            .unwrap_or_default();
        if strip_newlines && text.contains('\n') {
            text = text.replace('\n', "");
        }
        session.on_text_changed(block_ref, text);
    }

    let ctx = ui.ctx().clone();
    let mut commit_block =
        |block: BlockRef, state: &rendered_session::BlockEditState| {
            commit_session_block(&ctx, block, state, source, edit_state);
        };

    let has_focus_now = response.has_focus();
    // Lists strip newlines anyway, so any Enter exits. Paragraphs let Shift+Enter
    // insert a newline (preserved on commit) and plain Enter commits + exits.
    let enter_pressed = has_focus_now
        && ui.input(|i| i.key_pressed(Key::Enter) && (strip_newlines || !i.modifiers.shift));
    let escape_pressed = has_focus_now && ui.input(|i| i.key_pressed(Key::Escape));

    if enter_pressed {
        log::trace!(
            "session plain: enter -> commit + display {:?}",
            block_ref
        );
        if strip_newlines {
            if let Some(state) = session.blocks.get_mut(&block_ref) {
                state.text = state.text.replace('\n', "");
            }
        }
        session.close_active_ui(ui, CommitPolicy::SaveIfDirty, &mut commit_block);
    } else if escape_pressed {
        log::trace!(
            "session plain: escape -> discard + display {:?}",
            block_ref
        );
        let mut reload = |blk: BlockRef, state: &mut rendered_session::BlockEditState| {
            reload_plain_block_from_source(blk, state, source, edit_state);
        };
        session.discard_active(&mut reload);
        session.active = None;
        block_ref.surrender_focus(ui);
    } else if session.active == Some(block_ref) && response.lost_focus() {
        session.close_active_ui(ui, CommitPolicy::SaveIfDirty, &mut commit_block);
    } else if session.active != Some(block_ref) && response.has_focus() {
        session_switch_to_ui(
            ui,
            session,
            block_ref,
            PendingActivation {
                cursor_char_index: None,
                request_focus: false,
            },
            source,
            edit_state,
        );
    }

    note_session_active_clicked(ui, block_ref, session, response);

    let has_focus = response.has_focus();
    let selection = if has_focus {
        output.cursor_range.map(|range| {
            let primary = range.primary.index;
            let secondary = range.secondary.index;
            if primary < secondary {
                (primary, secondary)
            } else {
                (secondary, primary)
            }
        })
    } else {
        None
    };

    (has_focus, selection)
}

fn heading_click_cursor(
    ui: &Ui,
    response: &Response,
    text: &str,
    font_size: f32,
    editor_font: &EditorFont,
) -> Option<usize> {
    let click_pos = ui.ctx().input(|i| i.pointer.interact_pos())?;
    Some(
        compute_displayed_cursor_index(
            ui,
            text,
            click_pos,
            response.rect,
            font_size,
            editor_font,
            0.0,
        )
        .min(text.chars().count()),
    )
}

/// Render a heading as an editable widget (session-backed; `structural` selects widget id path).
fn render_heading(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
    colors: &EditorColors,
    base_font_size: f32,
    editor_font: &EditorFont,
    level: HeadingLevel,
    header_spacing: HeaderSpacing,
    structural: bool,
) {
    let source_text = node.text_content();
    let node_id = edit_state.add_node(source_text.clone(), node.start_line, node.end_line);
    let block_ref = BlockRef::Heading {
        line: node.start_line,
        structural,
    };

    let font_size = match level {
        HeadingLevel::H1 => base_font_size * 1.8,
        HeadingLevel::H2 => base_font_size * 1.5,
        HeadingLevel::H3 => base_font_size * 1.3,
        HeadingLevel::H4 => base_font_size * 1.15,
        HeadingLevel::H5 => base_font_size * 1.05,
        HeadingLevel::H6 => base_font_size,
    };

    let font_family = fonts::get_styled_font_family(true, false, editor_font);
    let (top_margin, bottom_margin) = header_margins(header_spacing, level);
    ui.add_space(top_margin);

    let widget_id = block_ref.widget_id(ui);
    if !session.blocks.contains_key(&block_ref) {
        session.on_text_changed(block_ref, source_text.clone());
    }
    reset_session_block_if_locked(ui, session, block_ref, &source_text);

    let preview_locked = crate::markdown::preview_locked_from_ui(ui);
    let available_width = ui.available_width();
    let (has_focus, selection) = ui
        .horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(4.0);

            let mut output = {
                let block_state = session
                    .blocks
                    .get_mut(&block_ref)
                    .expect("heading block initialized above");
                TextEdit::singleline(&mut block_state.text)
                    .id(widget_id)
                    .font(FontId::new(font_size, font_family))
                    .text_color(colors.heading)
                    .frame(egui::Frame::NONE)
                    .margin(egui::vec2(0.0, 0.0))
                    .desired_width(ui.available_width())
                    .interactive(!preview_locked)
                    .show(ui)
            };

            let response = &output.response;
            let buffer_snapshot = session
                .blocks
                .get(&block_ref)
                .map(|s| s.text.clone())
                .unwrap_or_default();

            if response.has_focus() {
                if session.active != Some(block_ref) {
                    session_switch_to_ui(
                        ui,
                        session,
                        block_ref,
                        PendingActivation {
                            cursor_char_index: None,
                            request_focus: false,
                        },
                        source,
                        edit_state,
                    );
                }
            } else if session.active.is_some_and(|a| a != block_ref)
                && (response.clicked()
                    || (response.hovered() && ui.input(|i| i.pointer.primary_pressed())))
            {
                let cursor_idx = heading_click_cursor(
                    ui,
                    response,
                    &buffer_snapshot,
                    font_size,
                    editor_font,
                );
                session_switch_to_ui(
                    ui,
                    session,
                    block_ref,
                    PendingActivation {
                        cursor_char_index: cursor_idx,
                        request_focus: true,
                    },
                    source,
                    edit_state,
                );
                response.request_focus();
            }

            if let Some(activation) = session
                .blocks
                .get_mut(&block_ref)
                .and_then(|s| s.pending_activation.take())
            {
                if activation.request_focus {
                    response.request_focus();
                }
                if let Some(pos) = activation.cursor_char_index {
                    let ccursor = egui::text::CCursor::new(pos);
                    output.state.cursor.set_char_range(Some(
                        egui::text::CCursorRange::one(ccursor),
                    ));
                    output.state.store(ui.ctx(), widget_id);
                }
            }

            if response.changed() && !preview_locked {
                if let Some(text) = session.blocks.get(&block_ref).map(|s| s.text.clone()) {
                    session.on_text_changed(block_ref, text);
                }
            }

            let ctx = ui.ctx().clone();
            let mut commit_heading =
                |block: BlockRef, state: &rendered_session::BlockEditState| {
                    commit_session_block(&ctx, block, state, source, edit_state);
                };

            if session.active == Some(block_ref) && response.lost_focus() {
                session.close_active_ui(
                    ui,
                    CommitPolicy::SaveIfDirty,
                    &mut commit_heading,
                );
            }

            note_session_active_clicked(ui, block_ref, session, response);

            let has_focus = response.has_focus();
            let selection = if has_focus {
                output.cursor_range.map(|range| {
                    let primary = range.primary.index;
                    let secondary = range.secondary.index;
                    if primary < secondary {
                        (primary, secondary)
                    } else {
                        (secondary, primary)
                    }
                })
            } else {
                None
            };

            (has_focus, selection)
        })
        .inner;

    if has_focus {
        edit_state.set_focus(node_id, selection);
    }
    if bottom_margin > 0.0 {
        ui.add_space(bottom_margin);
    }
}

/// Render a paragraph with structural key handling (Enter splits paragraph).
/// Whether a paragraph contains GitHub HTML inline nodes that need AST rendering.
fn paragraph_has_html_inline(node: &MarkdownNode) -> bool {
    node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Kbd
                | MarkdownNodeType::Superscript
                | MarkdownNodeType::Subscript
                | MarkdownNodeType::LineBreak
                | MarkdownNodeType::HtmlInline(_)
        )
    })
}

fn render_paragraph_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
    structural_state: &mut StructuralEditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
) {
    if paragraph_has_html_inline(node) {
        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            render_inline_content(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        });
        return;
    }

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
        // Formatted paragraphs (structural): session-backed click-to-edit.
        // Structural Enter (paragraph split) remains disabled; mirrored from prior code.
        let _ = structural_state;
        let block_ref = BlockRef::FormattedParagraph {
            line: node.start_line,
            structural: true,
        };
        let cold_text = extract_paragraph_content(source, node.start_line, node.end_line);
        let node_id = edit_state.add_node(cold_text.clone(), node.start_line, node.end_line);
        ensure_formatted_block_initialized(session, block_ref, cold_text);

        let editing = session
            .blocks
            .get(&block_ref)
            .map(|s| s.formatted_editing)
            .unwrap_or(false);
        let base_indent = 4.0 + indent_level as f32 * 20.0;
        let available_width = ui.available_width();

        if editing {
            let (has_focus, selection) = ui
                .horizontal(|ui| {
                    ui.set_max_width(available_width);
                    ui.add_space(base_indent);
                    render_session_formatted_edit_text(
                        ui,
                        block_ref,
                        session,
                        source,
                        edit_state,
                        font_size,
                        font_family.clone(),
                        colors.text,
                        cjk_indent,
                        editor_font,
                        false,
                    )
                })
                .inner;
            if has_focus {
                edit_state.set_focus(node_id, selection);
            }
        } else {
            let raw_display = session
                .blocks
                .get(&block_ref)
                .map(|s| s.text.clone())
                .unwrap_or_default();
            let display_response = ui
                .horizontal(|ui| {
                    ui.set_max_width(available_width);
                    ui.add_space(base_indent);

                    if cjk_indent > 0.0 {
                        ui.add_space(cjk_indent);
                    }
                    let wrap_w = ui.available_width();
                    with_block_align_widget(ui, |ui| {
                        rendered_session::paint_formatted_block_display(
                            ui,
                            session,
                            block_ref,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                            wrap_w,
                        )
                    })
                })
                .inner;

            let sense_id = block_ref.widget_id(ui).with("display_sense");
            let sense_response =
                ui.interact(display_response.rect, sense_id, egui::Sense::click());

            if sense_response.clicked() {
                enter_formatted_edit_on_display_click(
                    ui,
                    block_ref,
                    session,
                    source,
                    edit_state,
                    display_response.rect,
                    0.0,
                    &raw_display,
                    font_size,
                    editor_font,
                    colors.text,
                    colors.link,
                    colors.code_bg,
                );
            }
            if sense_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        }
    } else {
        // Simple text-only paragraph — session-backed buffer (preserves trailing spaces via extract).
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);
        let block_ref = BlockRef::Paragraph {
            line: node.start_line,
        };
        let source_text = extract_paragraph_content(source, node.start_line, node.end_line);

        let available_width = ui.available_width();
        let (has_focus, selection) = ui
            .horizontal(|ui| {
                ui.set_max_width(available_width);
                ui.add_space(4.0 + indent_level as f32 * 20.0);

                let _ = structural_state;
                render_session_plain_text_block(
                    ui,
                    block_ref,
                    session,
                    source,
                    edit_state,
                    node.end_line,
                    source_text,
                    font_size,
                    font_family.clone(),
                    colors.text,
                    cjk_indent,
                    editor_font,
                    false,
                )
            })
            .inner;

        if has_focus {
            edit_state.set_focus(node_id, selection);
        }
    }
}

/// Render a blockquote with structural key support for children.
fn render_blockquote_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
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
                    session,
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

/// Collapse/expand chevrons for callout title rows (Unicode escapes avoid source encoding issues).
const CALLOUT_ARROW_COLLAPSED: &str = "\u{25B6}"; // ▶
const CALLOUT_ARROW_EXPANDED: &str = "\u{25BC}"; // ▼

/// Paint the callout background wash and left accent bar.
///
/// Uses the scroll/content rect for height so the fill does not bleed over
/// subsequent blocks when the outer horizontal layout claims extra vertical space.
fn paint_callout_chrome(
    group_rect: egui::Rect,
    content_rect: egui::Rect,
    base_indent: f32,
    border_width: f32,
    border_color: Color32,
    bg_color: Color32,
    painter: &egui::Painter,
) {
    let paint_rect = if content_rect.is_positive() {
        egui::Rect::from_min_max(
            egui::pos2(group_rect.min.x + base_indent, content_rect.min.y),
            egui::pos2(group_rect.max.x, content_rect.max.y),
        )
    } else {
        egui::Rect::from_min_size(
            egui::pos2(group_rect.min.x + base_indent, group_rect.min.y),
            Vec2::new(group_rect.width() - base_indent, group_rect.height()),
        )
    };

    painter.rect_filled(paint_rect, 4.0, bg_color);

    let border_rect = egui::Rect::from_min_size(
        egui::pos2(group_rect.min.x + base_indent, paint_rect.min.y),
        Vec2::new(border_width, paint_rect.height()),
    );
    painter.rect_filled(border_rect, 2.0, border_color);
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
    session: &mut RenderedEditSession,
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

    let mut content_paint_rect = egui::Rect::NOTHING;
    let group_response = ui.push_id(scope_id, |ui| {
        let callout_id = ui.make_persistent_id("collapsed");
        let is_collapsed = ui.data_mut(|d| *d.get_persisted_mut_or(callout_id, default_collapsed));

        let title_text = custom_title.unwrap_or(callout_type.display_name());
        let icon = callout_type.icon();

        let available_width = ui.available_width();
        let inner = ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

            // See issue #129: auto_shrink y must be true so this horizontal
            // scroll area shrinks to fit its content vertically.
            let scroll_out = egui::ScrollArea::horizontal()
                .id_salt("scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let title_row = ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(icon)
                                    .color(title_color)
                                    .font(FontId::proportional(font_size)),
                            );

                            let arrow = if is_collapsed {
                                CALLOUT_ARROW_COLLAPSED
                            } else {
                                CALLOUT_ARROW_EXPANDED
                            };
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
                                    session,
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
                    ui.min_rect()
                });
            content_paint_rect = scroll_out.inner;
        });
        inner.response
    });

    paint_callout_chrome(
        group_response.response.rect,
        content_paint_rect,
        BASE_INDENT,
        BORDER_WIDTH,
        border_color,
        bg_color,
        ui.painter(),
    );
}

/// Render a list with structural key support for items.
fn render_list_with_structural_keys(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
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
                session,
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
    session: &mut RenderedEditSession,
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
    // and would display them as replacement characters (â–¡). See GitHub issue #41.
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
            let mut checked = task_checked;
            if crate::markdown::preview_locked_from_ui(ui) {
                ui.add_enabled(false, egui::Checkbox::new(&mut checked, ""));
            } else {
                let checkbox_response = ui.checkbox(&mut checked, "");

                if checkbox_response.changed() {
                    if let Some(source_line) = source.lines().nth(node.start_line.saturating_sub(1)) {
                        let new_line = if task_checked {
                            source_line.replace("[x]", "[ ]").replace("[X]", "[ ]")
                        } else {
                            source_line.replace("[ ]", "[x]")
                        };
                        update_source_line(source, node.start_line, &new_line);

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
            }
            ui.add_space(2.0);
        } else {
            let marker = match list_type {
                ListType::Bullet => {
                    if indent_level == 0 {
                        "\u{2022}" // bullet â€¢
                    } else {
                        "\u{25E6}" // white bullet â—¦
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
                // Formatted list items (structural): session-backed click-to-edit.
                // Structural Enter handling stays disabled (mirrors prior code).
                let _ = structural_state;
                let block_ref = BlockRef::FormattedListItem {
                    line: para.start_line,
                    item: item_index as u32,
                    structural: true,
                };
                let cold_text = extract_list_item_content(source, para.start_line);
                let node_id = edit_state.add_node(cold_text.clone(), para.start_line, para.end_line);
                ensure_formatted_block_initialized(session, block_ref, cold_text);

                let editing = session
                    .blocks
                    .get(&block_ref)
                    .map(|s| s.formatted_editing)
                    .unwrap_or(false);

                if editing {
                    let (has_focus, selection) = render_session_formatted_edit_text(
                        ui,
                        block_ref,
                        session,
                        source,
                        edit_state,
                        font_size,
                        font_family.clone(),
                        colors.text,
                        0.0,
                        editor_font,
                        true,
                    );
                    if has_focus {
                        edit_state.set_focus(node_id, selection);
                    }
                } else {
                    let raw_display = session
                        .blocks
                        .get(&block_ref)
                        .map(|s| s.text.clone())
                        .unwrap_or_default();
                    let wrap_w = ui.available_width();
                    let display_response = with_block_align_widget(ui, |ui| {
                        rendered_session::paint_formatted_block_display(
                            ui,
                            session,
                            block_ref,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                            wrap_w,
                        )
                    });

                    let sense_id = block_ref.widget_id(ui).with("display_sense");
                    let sense_response =
                        ui.interact(display_response.rect, sense_id, egui::Sense::click());

                    if sense_response.clicked() {
                        enter_formatted_edit_on_display_click(
                            ui,
                            block_ref,
                            session,
                            source,
                            edit_state,
                            display_response.rect,
                            0.0,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                        );
                    }
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text — session-backed buffer
            let block_ref = BlockRef::ListItem {
                line: start_line,
                item: item_index as u32,
            };
            let cold_text = extract_list_item_content(source, start_line);
            let (has_focus, selection) = render_session_plain_text_block(
                ui,
                block_ref,
                session,
                source,
                edit_state,
                end_line,
                cold_text,
                font_size,
                font_family.clone(),
                colors.text,
                0.0,
                editor_font,
                true,
            );
            let _ = structural_state;
            if has_focus {
                edit_state.set_focus(node_id, selection);
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
                session,
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
    session: &mut RenderedEditSession,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
) {
    if paragraph_has_html_inline(node) {
        let available_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            render_inline_content(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                indent_level,
            );
        });
        return;
    }

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
        // Formatted paragraphs (non-structural): session-backed click-to-edit.
        let block_ref = BlockRef::FormattedParagraph {
            line: node.start_line,
            structural: false,
        };
        let cold_text = extract_paragraph_content(source, node.start_line, node.end_line);
        let node_id = edit_state.add_node(cold_text.clone(), node.start_line, node.end_line);
        ensure_formatted_block_initialized(session, block_ref, cold_text);

        let editing = session
            .blocks
            .get(&block_ref)
            .map(|s| s.formatted_editing)
            .unwrap_or(false);
        let base_indent = 4.0 + indent_level as f32 * 20.0;
        let available_width = ui.available_width();

        if editing {
            let (has_focus, selection) = ui
                .horizontal(|ui| {
                    ui.set_max_width(available_width);
                    ui.add_space(base_indent);
                    render_session_formatted_edit_text(
                        ui,
                        block_ref,
                        session,
                        source,
                        edit_state,
                        font_size,
                        font_family.clone(),
                        colors.text,
                        cjk_indent,
                        editor_font,
                        false,
                    )
                })
                .inner;
            if has_focus {
                edit_state.set_focus(node_id, selection);
            }
        } else {
            let raw_display = session
                .blocks
                .get(&block_ref)
                .map(|s| s.text.clone())
                .unwrap_or_default();
            let display_response = ui
                .horizontal(|ui| {
                    ui.set_max_width(available_width);
                    ui.add_space(base_indent);

                    if cjk_indent > 0.0 {
                        ui.add_space(cjk_indent);
                    }
                    let wrap_w = ui.available_width();
                    with_block_align_widget(ui, |ui| {
                        rendered_session::paint_formatted_block_display(
                            ui,
                            session,
                            block_ref,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                            wrap_w,
                        )
                    })
                })
                .inner;

            let sense_id = block_ref.widget_id(ui).with("display_sense");
            let sense_response =
                ui.interact(display_response.rect, sense_id, egui::Sense::click());

            if sense_response.clicked() {
                enter_formatted_edit_on_display_click(
                    ui,
                    block_ref,
                    session,
                    source,
                    edit_state,
                    display_response.rect,
                    0.0,
                    &raw_display,
                    font_size,
                    editor_font,
                    colors.text,
                    colors.link,
                    colors.code_bg,
                );
            }
            if sense_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        }
    } else {
        // Simple text-only paragraph — session-backed buffer.
        let text = node.text_content();
        let node_id = edit_state.add_node(text.clone(), node.start_line, node.end_line);
        let block_ref = BlockRef::Paragraph {
            line: node.start_line,
        };
        let source_text = extract_paragraph_content(source, node.start_line, node.end_line);

        let available_width = ui.available_width();
        let (has_focus, selection) = ui
            .horizontal(|ui| {
                ui.set_max_width(available_width);
                ui.add_space(4.0 + indent_level as f32 * 20.0);

                render_session_plain_text_block(
                    ui,
                    block_ref,
                    session,
                    source,
                    edit_state,
                    node.end_line,
                    source_text,
                    font_size,
                    font_family.clone(),
                    colors.text,
                    cjk_indent,
                    editor_font,
                    false,
                )
            })
            .inner;

        if has_focus {
            edit_state.set_focus(node_id, selection);
        }
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Text Style Accumulator
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

/// Compute the character index in plain displayed text from a click using egui's Galley.
///
/// Used for headings and other single-font blocks. Formatted inline blocks use
/// [`FormattedBlockLayout`](crate::markdown::rendered_session::FormattedBlockLayout) instead.
fn compute_displayed_cursor_index(
    ui: &Ui,
    displayed_text: &str,
    click_pos: egui::Pos2,
    text_rect: egui::Rect,
    font_size: f32,
    editor_font: &EditorFont,
    leading_indent: f32,
) -> usize {
    if displayed_text.is_empty() {
        return 0;
    }

    let wrap_width = text_rect.width().max(1.0);
    let local_pos = egui::Vec2::new(
        (click_pos.x - text_rect.min.x - leading_indent).max(0.0),
        click_pos.y - text_rect.min.y,
    );

    let starts_with_bold = displayed_text.starts_with("**") || displayed_text.starts_with("__");
    let font_family = fonts::get_styled_font_family(starts_with_bold, false, editor_font);
    let font_id = FontId::new(font_size, font_family);
    let galley = ui.fonts_mut(|f| {
        f.layout(
            displayed_text.to_owned(),
            font_id,
            Color32::PLACEHOLDER,
            wrap_width,
        )
    });

    galley
        .cursor_from_pos(local_pos)
        .index
        .min(displayed_text.chars().count())
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
    with_block_align_widget(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
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

        MarkdownNodeType::Kbd => {
            render_kbd_span(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                style,
            );
        }

        MarkdownNodeType::Superscript => {
            render_script_span(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                style,
                true,
            );
        }

        MarkdownNodeType::Subscript => {
            render_script_span(
                ui,
                node,
                source,
                edit_state,
                colors,
                font_size,
                editor_font,
                style,
                false,
            );
        }

        MarkdownNodeType::Image {
            url,
            title,
            width,
            height,
        } => {
            // Images break out of the inline flow - end the current row and render as block
            ui.end_row();
            render_image(ui, node, colors, font_size, url, title, *width, *height);
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

        MarkdownNodeType::HtmlInline(html) => {
            ui.label(
                RichText::new("«HTML»")
                    .color(colors.quote_text)
                    .small()
                    .italics(),
            );
            let _ = html;
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

    let preview_locked = crate::markdown::preview_locked_from_ui(ui);
    if preview_locked {
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
    if output.changed && !preview_locked {
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

    // Handle "Insert as block" requests from the run output panel — append a
    // fenced ```output block right after the current code block.
    if !preview_locked {
        if let Some(body) = output.insert_output_below {
            insert_output_block_after(source, node.end_line, &body);
            let node_id = edit_state.add_node(output.code.clone(), node.start_line, node.end_line);
            if let Some(editable) = edit_state.get_node_mut(node_id) {
                editable.modified = true;
            }
        }
    }
}

/// Append a fenced ```output block to `source` immediately after `end_line`
/// (1-indexed). Used by the inline run-output panel's "Insert as block" action.
fn insert_output_block_after(source: &mut String, end_line: usize, body: &str) {
    let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let insert_idx = end_line.min(lines.len());
    let block_body = body.trim_end_matches('\n').to_string();
    let mut block: Vec<String> = vec![String::new(), "```output".to_string()];
    if block_body.is_empty() {
        block.push(String::new());
    } else {
        for line in block_body.lines() {
            block.push(line.to_string());
        }
    }
    block.push("```".to_string());
    let tail = lines.split_off(insert_idx);
    lines.extend(block);
    lines.extend(tail);
    *source = lines.join("\n");
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
    // If so, update the cached data to match the current parsed content while
    // preserving the last successfully rendered source (so a transient parse
    // failure during typing keeps the previous diagram visible).
    if mermaid_data.source != literal {
        let preserved_good = mermaid_data.last_good_source.clone();
        let preserved_err = mermaid_data.last_error.clone();
        mermaid_data = MermaidBlockData::new(literal);
        mermaid_data.last_good_source = preserved_good;
        mermaid_data.last_error = preserved_err;
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

    if let Some(anchor) = output.open_anchor {
        MermaidPopupState::request_open(
            ui.ctx(),
            output.source,
            anchor,
            dark_mode,
            font_size,
            mermaid_block_id.value(),
        );
    }

    // Log if changes were detected (for debugging)
    if output.changed {
        debug!(
            "Mermaid block at line {} detected change (type: {:?})",
            node.start_line, output.diagram_type
        );
    }
}

/// Map table/cell alignment to egui block layout alignment.
fn table_alignment_to_egui(alignment: TableAlignment) -> egui::Align {
    match alignment {
        TableAlignment::Center => egui::Align::Center,
        TableAlignment::Right => egui::Align::RIGHT,
        TableAlignment::Left | TableAlignment::None => egui::Align::LEFT,
    }
}

fn block_text_align_memory_id() -> egui::Id {
    egui::Id::new("markdown_block_text_align")
}

fn with_block_text_align<R>(
    ui: &mut Ui,
    alignment: TableAlignment,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let prev = ui.ctx().memory(|mem| {
        mem.data
            .get_temp::<TableAlignment>(block_text_align_memory_id())
    });
    ui.ctx().memory_mut(|mem| {
        mem.data
            .insert_temp(block_text_align_memory_id(), alignment);
    });
    let result = add_contents(ui);
    ui.ctx().memory_mut(|mem| {
        if let Some(prev) = prev {
            mem.data
                .insert_temp(block_text_align_memory_id(), prev);
        } else {
            mem.data
                .remove::<TableAlignment>(block_text_align_memory_id());
        }
    });
    result
}

fn block_text_align_from_ui(ui: &Ui) -> TableAlignment {
    ui.ctx().memory(|mem| {
        mem.data
            .get_temp::<TableAlignment>(block_text_align_memory_id())
            .unwrap_or(TableAlignment::None)
    })
}

/// Position a shrink-wrapped child (e.g. formatted galley) inside an aligned `<div>`.
fn with_block_align_widget<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    match block_text_align_from_ui(ui) {
        TableAlignment::Center => ui
            .horizontal(|ui| {
                ui.set_width(ui.available_width().max(0.0));
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center).with_main_justify(true),
                    add_contents,
                )
                .inner
            })
            .inner,
        TableAlignment::Right => ui
            .horizontal(|ui| {
                ui.set_width(ui.available_width().max(0.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), add_contents).inner
            })
            .inner,
        TableAlignment::Left | TableAlignment::None => add_contents(ui),
    }
}

/// Render a `<summary>` string that may contain inline HTML or markdown.
fn render_details_summary(
    ui: &mut Ui,
    summary: &str,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
) {
    let doc = match parse_markdown(summary) {
        Ok(doc) => doc,
        Err(_) => {
            ui.label(
                RichText::new(summary)
                    .color(colors.text)
                    .font(FontId::proportional(font_size)),
            );
            return;
        }
    };

    let para = doc.root.children.iter().find(|child| {
        matches!(child.node_type, MarkdownNodeType::Paragraph)
    });

    if let Some(para) = para {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
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
        });
    } else {
        ui.label(
            RichText::new(summary)
                .color(colors.text)
                .font(FontId::proportional(font_size)),
        );
    }
}

/// Passthrough indicator for HTML blocks outside the safe GitHub subset.
fn render_unrecognized_html_block(ui: &mut Ui, colors: &EditorColors, html: &str) {
    let trimmed = html.trim();
    if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
        return;
    }
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("«HTML»")
                .color(colors.quote_text)
                .small()
                .italics(),
        );
    });
}

/// Render a GitHub-style aligned `<div>` block.
fn render_aligned_div(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
    alignment: TableAlignment,
) {
    const BASE_INDENT: f32 = 4.0;
    // Center text alignment is applied per-widget; block layout stays left-to-right.
    let layout_align = match alignment {
        TableAlignment::Right => egui::Align::RIGHT,
        _ => egui::Align::LEFT,
    };
    let available_width = ui.available_width();

    with_block_text_align(ui, alignment, |ui| {
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(BASE_INDENT);
            ui.with_layout(egui::Layout::top_down(layout_align), |ui| {
                ui.set_width((available_width - BASE_INDENT).max(0.0));
                for child in &node.children {
                    render_node(
                        ui,
                        child,
                        source,
                        edit_state,
                        session,
                        colors,
                        font_size,
                        editor_font,
                        indent_level,
                        paragraph_indent,
                        header_spacing,
                    );
                }
            });
        });
    });
}

/// Render a GitHub-style `<details>/<summary>` collapsible block.
///
/// Default state is **closed** unless the source tag includes the `open` attribute.
fn render_details(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    indent_level: usize,
    paragraph_indent: ParagraphIndent,
    header_spacing: HeaderSpacing,
    summary: &str,
    open: bool,
) {
    const BASE_INDENT: f32 = 4.0;
    let scope_id = ("details_render", node.start_line, node.end_line);
    let default_collapsed = !open;

    ui.push_id(scope_id, |ui| {
        let details_id = ui.make_persistent_id("collapsed");
        let is_collapsed =
            ui.data_mut(|d| *d.get_persisted_mut_or(details_id, default_collapsed));

        ui.horizontal(|ui| {
            ui.add_space(BASE_INDENT);
            ui.vertical(|ui| {
                let title_row = ui.horizontal(|ui| {
                    let arrow = if is_collapsed {
                        CALLOUT_ARROW_COLLAPSED
                    } else {
                        CALLOUT_ARROW_EXPANDED
                    };
                    ui.label(
                        RichText::new(arrow)
                            .color(colors.text)
                            .font(FontId::proportional(font_size * 0.7)),
                    );
                    if !summary.is_empty() {
                        render_details_summary(
                            ui,
                            summary,
                            source,
                            edit_state,
                            colors,
                            font_size,
                            editor_font,
                        );
                    }
                });

                let title_rect = title_row.response.rect;
                let click_response = ui.allocate_rect(title_rect, egui::Sense::click());
                if click_response.clicked() {
                    ui.data_mut(|d| {
                        let val = d.get_persisted_mut_or(details_id, default_collapsed);
                        *val = !*val;
                    });
                }
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
                            session,
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
}

/// Render a block quote.
fn render_blockquote(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
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

        // See issue #129: auto_shrink y must be true on horizontal-only scroll
        // areas so the perpendicular axis sizes to content height instead of
        // claiming all remaining viewport height (egui's `max(inner, content)`
        // rule for scroll_enabled=false / auto_shrink=false).
        egui::ScrollArea::horizontal()
            .id_salt(blockquote_id)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for child in &node.children {
                        render_node(
                            ui,
                            child,
                            source,
                            edit_state,
                            session,
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
    session: &mut RenderedEditSession,
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

    let mut content_paint_rect = egui::Rect::NOTHING;
    let group_response = ui.push_id(scope_id, |ui| {
        let callout_id = ui.make_persistent_id("collapsed");
        let is_collapsed = ui.data_mut(|d| *d.get_persisted_mut_or(callout_id, default_collapsed));

        let title_text = custom_title.unwrap_or(callout_type.display_name());
        let icon = callout_type.icon();

        let available_width = ui.available_width();
        let inner = ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            ui.add_space(BASE_INDENT + BORDER_WIDTH + BORDER_GAP);

            // See issue #129: auto_shrink y must be true so this horizontal
            // scroll area shrinks to fit its content vertically and does not
            // monopolise the viewport height (which would push the next block
            // off-screen).
            let scroll_out = egui::ScrollArea::horizontal()
                .id_salt("scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Title row with icon and collapse toggle
                        let title_row = ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(icon)
                                    .color(title_color)
                                    .font(FontId::proportional(font_size)),
                            );

                            let arrow = if is_collapsed {
                                CALLOUT_ARROW_COLLAPSED
                            } else {
                                CALLOUT_ARROW_EXPANDED
                            };
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
                                    session,
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
                    ui.min_rect()
                });
            content_paint_rect = scroll_out.inner;
        });
        inner.response
    });

    paint_callout_chrome(
        group_response.response.rect,
        content_paint_rect,
        BASE_INDENT,
        BORDER_WIDTH,
        border_color,
        bg_color,
        ui.painter(),
    );
}

/// Render a list (ordered or unordered).
fn render_list(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    session: &mut RenderedEditSession,
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
            render_list_item(
                ui,
                child,
                source,
                edit_state,
                session,
                colors,
                font_size,
                editor_font,
                indent_level,
                list_type,
                item_number,
                child_idx,
            );
            item_number += 1;
        }
    }

    // Add small spacing after top-level lists
    if indent_level == 0 {
        ui.add_space(4.0);
    }
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
    session: &mut RenderedEditSession,
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
    // and would display them as replacement characters (â–¡). See GitHub issue #41.
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
            let mut checked = task_checked;
            if crate::markdown::preview_locked_from_ui(ui) {
                ui.add_enabled(false, egui::Checkbox::new(&mut checked, ""));
            } else {
                let checkbox_response = ui.checkbox(&mut checked, "");

                if checkbox_response.changed() {
                    if let Some(source_line) = source.lines().nth(node.start_line.saturating_sub(1)) {
                        let new_line = if task_checked {
                            source_line.replace("[x]", "[ ]").replace("[X]", "[ ]")
                        } else {
                            source_line.replace("[ ]", "[x]")
                        };
                        update_source_line(source, node.start_line, &new_line);

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
            }
            ui.add_space(2.0);
        } else {
            let marker = match list_type {
                ListType::Bullet => {
                    if indent_level == 0 {
                        "\u{2022}" // bullet â€¢
                    } else {
                        "\u{25E6}" // white bullet â—¦
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
                // Formatted list items (non-structural): session-backed click-to-edit.
                let block_ref = BlockRef::FormattedListItem {
                    line: para.start_line,
                    item: item_number,
                    structural: false,
                };
                let cold_text = extract_list_item_content(source, para.start_line);
                let node_id = edit_state.add_node(cold_text.clone(), para.start_line, para.end_line);
                ensure_formatted_block_initialized(session, block_ref, cold_text);

                let editing = session
                    .blocks
                    .get(&block_ref)
                    .map(|s| s.formatted_editing)
                    .unwrap_or(false);

                if editing {
                    let (has_focus, selection) = render_session_formatted_edit_text(
                        ui,
                        block_ref,
                        session,
                        source,
                        edit_state,
                        font_size,
                        font_family.clone(),
                        colors.text,
                        0.0,
                        editor_font,
                        true,
                    );
                    if has_focus {
                        edit_state.set_focus(node_id, selection);
                    }
                } else {
                    let raw_display = session
                        .blocks
                        .get(&block_ref)
                        .map(|s| s.text.clone())
                        .unwrap_or_default();
                    let wrap_w = ui.available_width();
                    let display_response = with_block_align_widget(ui, |ui| {
                        rendered_session::paint_formatted_block_display(
                            ui,
                            session,
                            block_ref,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                            wrap_w,
                        )
                    });

                    let sense_id = block_ref.widget_id(ui).with("display_sense");
                    let sense_response =
                        ui.interact(display_response.rect, sense_id, egui::Sense::click());

                    if sense_response.clicked() {
                        enter_formatted_edit_on_display_click(
                            ui,
                            block_ref,
                            session,
                            source,
                            edit_state,
                            display_response.rect,
                            0.0,
                            &raw_display,
                            font_size,
                            editor_font,
                            colors.text,
                            colors.link,
                            colors.code_bg,
                        );
                    }
                    if sense_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }
            }
        } else if let Some((node_id, start_line, end_line)) = simple_text_node_id {
            // Simple text — session-backed buffer
            let block_ref = BlockRef::ListItem {
                line: start_line,
                item: item_index as u32,
            };
            let cold_text = extract_list_item_content(source, start_line);
            let (has_focus, selection) = render_session_plain_text_block(
                ui,
                block_ref,
                session,
                source,
                edit_state,
                end_line,
                cold_text,
                font_size,
                font_family.clone(),
                colors.text,
                0.0,
                editor_font,
                true,
            );
            return (has_focus, selection, Some(node_id));
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
                session,
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
    session: &mut RenderedEditSession,
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
    let preview_locked = crate::markdown::preview_locked_from_ui(ui);
    let mut table_data = if preview_locked {
        TableData::from_node(node)
    } else {
        ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_insert_with(table_id.with("data"), || TableData::from_node(node))
                .clone()
        })
    };

    if preview_locked {
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(table_id.with("edit_state"), TableEditState::new());
        });
    }

    // Capture available width BEFORE any layout changes
    let table_avail_width = (ui.available_width() - BASE_INDENT).max(100.0);

    let output = EditableTable::new(&mut table_data)
        .font_size(font_size)
        .colors(widget_colors)
        .with_controls(true)
        .with_alignment_controls(true)
        .id(table_id)
        .source_line(node.start_line)
        .max_width(table_avail_width)
        .editor_font(editor_font.clone())
        .show(ui);

    // Sync RenderedEditSession.active with cell focus so cross-block switches commit
    // the table (via the force-commit signal written by commit_session_block) and
    // intra-table movement stays in the table without firing per-cell commits.
    sync_table_cell_session_active(
        ui,
        node.start_line,
        session,
        source,
        edit_state,
        &output,
    );

    // Update stored data if changed
    if output.changed && !preview_locked {
        let ctx = ui.ctx().clone();
        let markdown = output.markdown.clone();
        rendered_commit_undo::record_source_commit(&ctx, source, |source| {
            update_table_in_source(source, node.start_line, node.end_line, &markdown);
        });

        // Mark that something was modified
        let node_id = edit_state.add_node(output.markdown.clone(), node.start_line, node.end_line);
        if let Some(editable) = edit_state.get_node_mut(node_id) {
            editable.modified = true;
        }

        debug!("Table at line {} modified", node.start_line);
    }

    ui.memory_mut(|mem| {
        mem.data.insert_temp(table_id.with("data"), table_data);
    });
}

/// Reconcile `RenderedEditSession.active` with the cell the user is interacting with.
///
/// Behaviour matrix (target cell = `BlockRef::TableCell { table_line, row, col }`):
///
/// | Previous `session.active`                      | New focused cell | Action                                              |
/// |------------------------------------------------|------------------|-----------------------------------------------------|
/// | `Some(target)` (same cell)                     | `Some(target)`   | No-op                                               |
/// | `Some(TableCell{table_line, …})` (intra-table) | `Some(other)`    | Direct assign (no commit; preserves deferred table) |
/// | `Some(other block)` or `None`                  | `Some(cell)`     | `switch_to_ui` (commits previous block via callback)|
/// | any                                            | `None`           | Leave session.active untouched; existing widget-level focus-loss + dismiss path handles commit + clear |
fn sync_table_cell_session_active(
    ui: &mut Ui,
    table_line: usize,
    session: &mut RenderedEditSession,
    source: &mut String,
    edit_state: &mut EditState,
    output: &crate::markdown::widgets::WidgetOutput,
) {
    let Some((row, col)) = output.focused_cell else {
        return;
    };

    let target = BlockRef::TableCell {
        table_line,
        row,
        col,
    };

    if session.active == Some(target) {
        // No transition; keep dismiss flag honest so the table's own click doesn't
        // get treated as an outside click.
        mark_session_active_clicked_if_clicked(ui);
        return;
    }

    let intra_table = matches!(
        session.active,
        Some(BlockRef::TableCell {
            table_line: t,
            ..
        }) if t == table_line
    );

    if intra_table {
        // Movement between cells of the same table — do not run commit_fn for the
        // previous cell (which would set the force-commit signal and falsely flush
        // the whole table mid-navigation). Just update the active pointer.
        log::trace!(
            "session table: intra-table move {:?} -> {:?}",
            session.active,
            target
        );
        session.active = Some(target);
        mark_session_active_clicked_if_clicked(ui);
        return;
    }

    // Cross-block (or cross-table) entry into a table cell. Use switch_to_ui so the
    // previous block (heading, paragraph, …, or a cell of a different table) commits
    // through `commit_session_block`. PendingActivation::default skips focus/cursor
    // requests because EditableTable already manages cell focus.
    log::trace!(
        "session table: cross-block enter {:?} -> {:?}",
        session.active,
        target
    );
    session_switch_to_ui(
        ui,
        session,
        target,
        PendingActivation::default(),
        source,
        edit_state,
    );
    mark_session_active_clicked_if_clicked(ui);
}

/// Record that the active session block was "clicked" this frame so
/// `session_dismiss_if_clicked_outside` does not falsely dismiss while the user is
/// still interacting with the table.
fn mark_session_active_clicked_if_clicked(ui: &mut Ui) {
    if ui.input(|i| i.pointer.any_click()) {
        let key = session_active_clicked_key(ui);
        ui.memory_mut(|mem| {
            mem.data.insert_temp(key, true);
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

/// Render front matter as a read-only document properties panel.
fn render_front_matter(ui: &mut Ui, colors: &EditorColors, font_size: f32, content: &str) {
    const BASE_INDENT: f32 = 4.0;

    let normalized_content = normalize_front_matter_content(content);
    let fields = parse_frontmatter_fields(normalized_content);
    if fields.is_empty() && !normalized_content.trim().is_empty() {
        render_front_matter_fallback(ui, colors, font_size, normalized_content);
        return;
    }

    let available_width = ui.available_width();
    let card_width = (available_width - BASE_INDENT - 8.0).max(180.0);
    let is_dark = ui.visuals().dark_mode;
    let card_fill = if is_dark {
        Color32::from_rgba_unmultiplied(28, 31, 38, 138)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 253, 180)
    };
    let card_stroke = if is_dark {
        Color32::from_rgba_unmultiplied(105, 114, 132, 94)
    } else {
        Color32::from_rgba_unmultiplied(190, 200, 216, 125)
    };

    ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(BASE_INDENT);

        ui.vertical(|ui| {
            ui.set_width(card_width);
            egui::Frame::new()
                .fill(card_fill)
                .stroke(egui::Stroke::new(1.0, card_stroke))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .corner_radius(10)
                .show(ui, |ui| {
                    let inner_width = (card_width - 32.0).max(140.0);
                    ui.set_width(inner_width);
                    ui.horizontal(|ui| {
                        ui.set_width(inner_width);
                        ui.label(
                            RichText::new("Note Properties")
                                .color(colors.text)
                                .size(font_size * 0.96)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let count_text = if fields.len() == 1 {
                                "1 field".to_string()
                            } else {
                                format!("{} fields", fields.len())
                            };
                            ui.label(
                                RichText::new(count_text)
                                    .color(colors.quote_text)
                                    .size(font_size * 0.78),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if fields.is_empty() {
                        ui.label(
                            RichText::new("No properties")
                                .color(colors.quote_text)
                                .size(font_size * 0.9)
                                .italics(),
                        );
                    } else {
                        ui.spacing_mut().item_spacing.y = 7.0;
                        for field in &fields {
                            render_front_matter_property_row(
                                ui,
                                colors,
                                font_size,
                                field,
                                inner_width,
                            );
                        }
                    }
                });
        });
    });

    ui.add_space(PARAGRAPH_TRAILING_SPACE_Y);
}

fn render_front_matter_property_row(
    ui: &mut Ui,
    colors: &EditorColors,
    font_size: f32,
    field: &FrontmatterField,
    row_width: f32,
) {
    let key_width = (row_width * 0.32).clamp(72.0, 150.0);
    let value_width = (row_width - key_width - 14.0).max(72.0);

    ui.horizontal(|ui| {
        ui.set_width(row_width);
        ui.add_sized(
            Vec2::new(key_width, 22.0),
            egui::Label::new(
                RichText::new(&field.key)
                    .color(colors.quote_text)
                    .size(font_size * 0.88),
            ),
        );

        ui.vertical(|ui| {
            ui.set_width(value_width);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(value_width);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                render_front_matter_property_value(ui, colors, font_size, &field.value);
            });
        });
    });
}

fn render_front_matter_property_value(
    ui: &mut Ui,
    colors: &EditorColors,
    font_size: f32,
    value: &FrontmatterValue,
) {
    match value {
        FrontmatterValue::String(value) | FrontmatterValue::Number(value) => {
            render_front_matter_text_value(ui, colors, font_size, value);
        }
        FrontmatterValue::Bool(value) => {
            let text = if *value { "true" } else { "false" };
            render_front_matter_pill(ui, colors, font_size, text);
        }
        FrontmatterValue::List(items) => {
            if items.is_empty() {
                render_front_matter_empty_value(ui, colors, font_size);
            } else {
                for item in items {
                    render_front_matter_pill(ui, colors, font_size, item);
                }
            }
        }
        FrontmatterValue::Mapping(raw) => {
            render_front_matter_text_value(ui, colors, font_size, raw);
        }
        FrontmatterValue::Null => {
            render_front_matter_empty_value(ui, colors, font_size);
        }
    }
}

fn render_front_matter_text_value(ui: &mut Ui, colors: &EditorColors, font_size: f32, value: &str) {
    if value.trim().is_empty() {
        render_front_matter_empty_value(ui, colors, font_size);
    } else {
        ui.add(egui::Label::new(
            RichText::new(value)
                .color(colors.text)
                .size(font_size * 0.96),
        ));
    }
}

fn render_front_matter_empty_value(ui: &mut Ui, colors: &EditorColors, font_size: f32) {
    ui.label(
        RichText::new("No value")
            .color(colors.quote_text)
            .size(font_size * 0.9)
            .italics(),
    );
}

fn render_front_matter_pill(ui: &mut Ui, colors: &EditorColors, font_size: f32, text: &str) {
    let is_dark = ui.visuals().dark_mode;
    let fill = if is_dark {
        Color32::from_rgba_unmultiplied(95, 105, 125, 150)
    } else {
        Color32::from_rgba_unmultiplied(220, 230, 245, 210)
    };
    let stroke = if is_dark {
        Color32::from_rgba_unmultiplied(130, 145, 170, 120)
    } else {
        Color32::from_rgba_unmultiplied(175, 190, 220, 170)
    };

    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .color(colors.text)
                    .size(font_size * 0.88),
            );
        });
}

fn normalize_front_matter_content(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(after_open) = trimmed.strip_prefix("---") else {
        return content;
    };

    let after_open = after_open.trim_start_matches(['\r', '\n']);
    if let Some((yaml, _rest)) = after_open.split_once("\n---") {
        yaml.trim()
    } else if let Some((yaml, _rest)) = after_open.split_once("\r\n---") {
        yaml.trim()
    } else {
        content
    }
}

fn render_front_matter_fallback(ui: &mut Ui, colors: &EditorColors, font_size: f32, content: &str) {
    const BASE_INDENT: f32 = 4.0;

    let available_width = ui.available_width();
    ui.horizontal(|ui| {
        ui.set_max_width(available_width);
        ui.add_space(BASE_INDENT);

        egui::Frame::new()
            .fill(colors.code_bg)
            .inner_margin(8)
            .corner_radius(4)
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
                        .frame(egui::Frame::NONE)
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
    let link_ctx = ui.memory(|mem| {
        mem.data
            .get_temp::<WikilinkContext>(egui::Id::new("wikilink_resolution_context"))
    });

    // Create and show the rendered link widget
    let output = RenderedLinkWidget::new(&mut link_state, title)
        .font_size(font_size)
        .colors(widget_colors)
        .id(link_id)
        .resolution_context(
            link_ctx.as_ref().and_then(|ctx| ctx.current_dir.clone()),
            link_ctx.as_ref().and_then(|ctx| ctx.workspace_root.clone()),
        )
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
/// for existence â€” broken links are styled with a dimmed red color.
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

fn render_video_embed(
    ui: &mut Ui,
    node: &MarkdownNode,
    info: &crate::markdown::parser::VideoEmbedInfo,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
) {
    let video_colors = VideoRenderColors {
        text: colors.text,
        link: colors.link,
        frame_border: colors.quote_border,
        frame_bg: colors.code_bg,
    };
    let resize = VideoEmbedResizeContext::new(
        node.start_line,
        !crate::markdown::preview_locked_from_ui(ui),
    );
    if let Some(commit) =
        video_render::render_video_embed(ui, info, &video_colors, font_size, Some(resize))
    {
        if rewrite_video_embed_dimensions(source, node.start_line, info, commit.width, commit.height)
        {
            if !edit_state.nodes.iter().any(|n| n.start_line == node.start_line) {
                edit_state.add_node(info.source_text.clone(), node.start_line, node.end_line);
            }
            mark_line_modified(edit_state, node.start_line);
            ui.ctx().request_repaint();
        }
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Image Rendering
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Cached image data stored in egui memory to avoid reloading every frame.
#[derive(Clone)]
struct CachedImageTexture {
    texture: TextureHandle,
    original_width: u32,
    original_height: u32,
}

/// Result of attempting to load an image â€” either success or a description of the failure.
#[derive(Clone)]
enum ImageLoadResult {
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

    // Skip web URLs â€” we only support local images
    let resolved = resolve_local_link_path(url, current_dir, workspace_root)?;
    resolved.is_file().then_some(resolved)
}

/// Load an image from disk, decode it, and create an egui texture.
fn load_image_texture(ctx: &egui::Context, path: &Path) -> Result<CachedImageTexture, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {}", e))?;

    let img = image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<Color32> = rgba
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_image = ColorImage {
        size: [width as usize, height as usize],
        source_size: egui::vec2(width as f32, height as f32),
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
    html_width: Option<u32>,
    html_height: Option<u32>,
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
        render_image_placeholder(ui, colors, font_size, &alt_text, "Web images not supported");
        return;
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
        ImageLoadResult::Loaded(cached_tex) => {
            let available_width = ui.available_width();
            let orig_w = cached_tex.original_width as f32;
            let orig_h = cached_tex.original_height as f32;

            let (target_w, target_h) = match (html_width, html_height) {
                (Some(w), Some(h)) => (w as f32, h as f32),
                (Some(w), None) if orig_w > 0.0 => {
                    let scale = w as f32 / orig_w;
                    (w as f32, orig_h * scale)
                }
                (None, Some(h)) if orig_h > 0.0 => {
                    let scale = h as f32 / orig_h;
                    (orig_w * scale, h as f32)
                }
                _ => (orig_w, orig_h),
            };

            // Scale to fit available width, maintaining aspect ratio
            let (display_w, display_h) = if target_w > available_width && available_width > 0.0 {
                let scale = available_width / target_w;
                (available_width, target_h * scale)
            } else {
                (target_w, target_h)
            };

            let sized = egui::load::SizedTexture::new(
                cached_tex.texture.id(),
                Vec2::new(display_w, display_h),
            );
            let image_widget = egui::Image::from_texture(sized);
            let response = ui.add(image_widget);

            // Show tooltip with alt text and/or title on hover
            let tooltip = build_image_tooltip(&alt_text, title, url);
            if !tooltip.is_empty() {
                response.on_hover_text(tooltip);
            }
        }
        ImageLoadResult::Failed(msg) => {
            render_image_placeholder(ui, colors, font_size, &alt_text, &msg);
        }
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

    egui::Frame::new()
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, frame_color))
        .corner_radius(4)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Image icon
                ui.label(
                    RichText::new("\u{1F5BC}") // ðŸ–¼ framed picture emoji
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

/// Render a GitHub-style `<kbd>` span with boxed monospace styling.
fn render_kbd_span(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    style: TextStyle,
) {
    let border = colors.text.gamma_multiply(0.35);
    egui::Frame::new()
        .fill(colors.code_bg)
        .stroke(Stroke::new(1.0, border))
        .inner_margin(Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for child in &node.children {
                    render_inline_node(
                        ui,
                        child,
                        source,
                        edit_state,
                        colors,
                        font_size * 0.9,
                        editor_font,
                        style,
                    );
                }
            });
        });
}

/// Render `<sup>` or `<sub>` with smaller type and raised/lowered baseline.
fn render_script_span(
    ui: &mut Ui,
    node: &MarkdownNode,
    source: &mut String,
    edit_state: &mut EditState,
    colors: &EditorColors,
    font_size: f32,
    editor_font: &EditorFont,
    style: TextStyle,
    superscript: bool,
) {
    let script_size = font_size * 0.75;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for child in &node.children {
            match &child.node_type {
                MarkdownNodeType::Text(text) => {
                    let family =
                        fonts::get_styled_font_family(style.bold, style.italic, editor_font);
                    let mut job = egui::text::LayoutJob::simple(
                        text.clone(),
                        FontId::new(script_size, family),
                        colors.text,
                        f32::INFINITY,
                    );
                    if let Some(section) = job.sections.first_mut() {
                        section.format.valign = if superscript {
                            egui::Align::TOP
                        } else {
                            egui::Align::BOTTOM
                        };
                        if style.strikethrough {
                            section.format.strikethrough = Stroke::new(1.0, colors.text);
                        }
                    }
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    ui.add(egui::Label::new(galley).selectable(false));
                }
                _ => {
                    render_inline_node(
                        ui,
                        child,
                        source,
                        edit_state,
                        colors,
                        script_size,
                        editor_font,
                        style,
                    );
                }
            }
        }
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
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Source Synchronization
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
///
/// `end_line` is the inclusive 1-indexed AST span end; when `new_content` spans more
/// lines, the replaced range expands so trailing old lines are not kept alongside
/// new content (rendered-edit grow). When shrinking, pass the full old AST `end_line`
/// so excess lines are removed.
fn update_source_range(source: &mut String, start_line: usize, end_line: usize, new_content: &str) {
    let lines: Vec<&str> = source.lines().collect();
    if start_line == 0 {
        return;
    }

    let committed_line_count = committed_block_line_count(new_content);
    let content_end = if committed_line_count == 0 {
        start_line.saturating_sub(1)
    } else {
        start_line.saturating_add(committed_line_count.saturating_sub(1))
    };
    let effective_end = end_line.max(content_end);

    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the range
    for i in 0..(start_line - 1) {
        if i < lines.len() {
            new_lines.push(lines[i].to_string());
        }
    }

    // Prefix from the original first line when it exists (preserve list markers).
    let original_first_line = lines.get(start_line.saturating_sub(1)).unwrap_or(&"");
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

    // Lines after the replaced range (exclusive index = effective_end)
    for i in effective_end..lines.len() {
        new_lines.push(lines[i].to_string());
    }

    *source = new_lines.join("\n");
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

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Utility Functions
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
// Memory Cleanup
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
/// - `CodeBlockData` - Code block content and edit state
/// - `MermaidBlockData` - Mermaid diagram source and render state
/// - `TableData` - Table cell contents and structure
/// - `TableEditState` - Table cell focus and navigation state
/// - `RenderedLinkState` - Link edit popup state
///
/// Headings, paragraphs, list items, and formatted variants use the per-tab
/// `RenderedEditSession` and are cleared when the editor's egui id is dropped.
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
        mem.data.remove_by_type::<CodeBlockData>();
        mem.data.remove_by_type::<MermaidBlockData>();
        mem.data.remove_by_type::<TableData>();
        mem.data.remove_by_type::<TableEditState>();
        mem.data.remove_by_type::<RenderedLinkState>();
    });

    log::debug!("Cleaned up rendered editor temporary memory");
}

// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
// Tests
// Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

#[cfg(test)]
mod tests {
    use super::*;

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // EditorMode Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // EditorColors Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // EditState Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // TextStyle Tests (for nested emphasis support)
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // Format Heading Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // Source Update Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
    fn test_update_source_range_grow_stale_single_line_end() {
        let mut source = "test".to_string();
        update_source_range(&mut source, 1, 1, "test\ntest2");
        assert_eq!(source, "test\ntest2");
    }

    #[test]
    fn test_update_source_range_grow_no_duplicate_trailing_line() {
        let mut source = "test\ntest2".to_string();
        update_source_range(&mut source, 1, 1, "test\ntest2");
        assert_eq!(source, "test\ntest2");
        assert_ne!(source, "test\ntest2\ntest2");
    }

    #[test]
    fn test_update_source_range_shrink_multi_line_block() {
        let mut source = "line1\nline2\nline3\nnext block".to_string();
        update_source_range(&mut source, 1, 3, "line1");
        assert_eq!(source, "line1\nnext block");
    }

    #[test]
    fn test_block_replace_end_line_grow_and_shrink() {
        assert_eq!(block_replace_end_line(1, 1, "test\ntest2"), 2);
        assert_eq!(block_replace_end_line(1, 3, "line1"), 3);
    }

    #[test]
    fn test_paragraph_soft_newline_commit_roundtrip() {
        let ctx = egui::Context::default();
        let mut source = "test".to_string();
        let mut edit_state = EditState::new();
        edit_state.add_node("test".to_string(), 1, 1);

        let state = rendered_session::BlockEditState {
            text: "test\ntest2".to_string(),
            dirty: true,
            ..Default::default()
        };
        write_session_block_to_source(
            &ctx,
            BlockRef::Paragraph { line: 1 },
            &state,
            &mut source,
            &mut edit_state,
        );
        assert_eq!(source, "test\ntest2");

        // Re-committing the same buffer against an expanded AST span must not duplicate.
        let node = edit_state.nodes.iter().find(|n| n.start_line == 1).unwrap();
        assert_eq!(node.end_line, 2);
        write_session_block_to_source(
            &ctx,
            BlockRef::Paragraph { line: 1 },
            &state,
            &mut source,
            &mut edit_state,
        );
        assert_eq!(source, "test\ntest2");
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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // Char Index Conversion Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // MarkdownEditor Builder Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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
    fn test_markdown_editor_source_epoch_default() {
        let mut content = String::new();
        let editor = MarkdownEditor::new(&mut content);
        assert_eq!(editor.source_epoch, 0);
    }

    #[test]
    fn test_markdown_editor_source_epoch_builder() {
        let mut content = String::new();
        let editor = MarkdownEditor::new(&mut content).source_epoch(7);
        assert_eq!(editor.source_epoch, 7);
    }

    #[test]
    fn test_heading_level_from_source() {
        let source = "# One\n## Two\n### Three\n";
        assert_eq!(heading_level_from_source(source, 1), HeadingLevel::H1);
        assert_eq!(heading_level_from_source(source, 2), HeadingLevel::H2);
        assert_eq!(heading_level_from_source(source, 3), HeadingLevel::H3);
    }

    #[test]
    fn test_rendered_heading_widget_id_stable_under_push_id_epoch_scope() {
        use crate::markdown::rendered_session::{rendered_editor_id, BlockRef};
        use eframe::egui;

        let ctx = egui::Context::default();
        let editor_id = rendered_editor_id(42);
        let mut captured: Vec<egui::Id> = Vec::new();

        for _content in ["# Alpha\n", "# Alpha edited\n"] {
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show_inside(ctx, |ui| {
                    ui.push_id(editor_id, |ui| {
                        ui.push_id(0u64, |ui| {
                            captured.push(
                                BlockRef::Heading {
                                    line: 1,
                                    structural: false,
                                }
                                .widget_id(ui),
                            );
                        });
                    });
                });
            });
        }

        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0], captured[1]);
    }

    #[test]
    fn test_rendered_heading_widget_id_changes_on_epoch_bump() {
        use crate::markdown::rendered_session::{rendered_editor_id, BlockRef};
        use eframe::egui;

        let ctx = egui::Context::default();
        let editor_id = rendered_editor_id(1);
        let mut ids = Vec::new();

        for epoch in [0u64, 1u64] {
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show_inside(ctx, |ui| {
                    ui.push_id(editor_id, |ui| {
                        ui.push_id(epoch, |ui| {
                            ids.push(
                                BlockRef::Heading {
                                    line: 1,
                                    structural: false,
                                }
                                .widget_id(ui),
                            );
                        });
                    });
                });
            });
        }

        assert_ne!(ids[0], ids[1]);
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

    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬
    // Link Update Tests
    // Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

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

    // ─── Regression: consecutive fenced code blocks (issue #129) ────────────
    //
    // Documents the parser-level invariants the rendered-view viewport culling
    // and block-height cache rely on. The actual layout bug — only the first
    // fenced block visible because the inner horizontal `ScrollArea` claimed
    // the full available height (`auto_shrink([false, false])`) — was fixed in
    // `widgets.rs` and `editor.rs` by switching the perpendicular axis to
    // `auto_shrink_y = true`. These tests guard the data the layout depends
    // on so regressions in the AST or per-block source slicing surface early.

    fn consecutive_fenced_doc() -> &'static str {
        "```text\nfirst\nline 2\n```\n\n```python\ndef hello():\n    print(\"hi\")\n```\n\n```rust\nfn main() {}\n```\n"
    }

    #[test]
    fn consecutive_fenced_blocks_parse_as_separate_ast_nodes() {
        let doc = cache::get_or_parse(consecutive_fenced_doc()).expect("markdown parses");
        let code_blocks: Vec<_> = doc
            .root
            .children
            .iter()
            .filter(|n| matches!(n.node_type, MarkdownNodeType::CodeBlock { .. }))
            .collect();
        assert_eq!(
            code_blocks.len(),
            3,
            "three consecutive fenced blocks must parse as three separate AST nodes"
        );
        for b in &code_blocks {
            assert!(
                b.end_line >= b.start_line && b.start_line > 0,
                "block must have a valid 1-indexed line range (got {}..={})",
                b.start_line,
                b.end_line
            );
        }
    }

    #[test]
    fn block_source_slice_extracts_each_consecutive_block_independently() {
        let content = consecutive_fenced_doc();
        let doc = cache::get_or_parse(content).expect("markdown parses");
        let offsets = line_start_byte_offsets(content);

        let blocks: Vec<_> = doc.root.children.iter().collect();
        assert_eq!(blocks.len(), 3);

        let s1 = block_source_slice(content, &offsets, blocks[0].start_line, blocks[0].end_line);
        let s2 = block_source_slice(content, &offsets, blocks[1].start_line, blocks[1].end_line);
        let s3 = block_source_slice(content, &offsets, blocks[2].start_line, blocks[2].end_line);

        // Each block's source must contain its own fence language and not bleed
        // into the next block's content. Distinct sources keep the per-block
        // height cache (`cache::get_block_height`) keyed independently.
        assert!(s1.contains("```text"));
        assert!(s1.contains("first"));
        assert!(!s1.contains("def hello"));

        assert!(s2.contains("```python"));
        assert!(s2.contains("def hello"));
        assert!(!s2.contains("fn main"));

        assert!(s3.contains("```rust"));
        assert!(s3.contains("fn main"));
        assert!(!s3.contains("def hello"));
    }

    #[test]
    fn estimate_block_height_is_finite_and_positive_for_each_block() {
        let doc = cache::get_or_parse(consecutive_fenced_doc()).expect("markdown parses");
        for b in &doc.root.children {
            let h = estimate_block_height(b.start_line, b.end_line);
            assert!(
                h.is_finite() && h > 0.0,
                "estimate must be finite > 0 (block {}..={} → {})",
                b.start_line,
                b.end_line,
                h
            );
            // Heuristic must scale with line count, never collapse to a single
            // line (otherwise viewport culling would think the block is too
            // small to be visible and skip rendering it).
            let lines = b.end_line.saturating_sub(b.start_line) + 1;
            assert!(
                h >= ESTIMATED_LINE_HEIGHT_PX * lines as f32,
                "estimate must reflect block line count"
            );
        }
    }

    #[test]
    fn block_height_cache_keys_distinguish_consecutive_blocks() {
        // The per-block height cache hashes the block's source slice. Two
        // consecutive but textually different code blocks must hash to
        // different keys, otherwise the cache could pollute one block's
        // measured height with another's and the culling state would place
        // them at incorrect Y positions.
        let content = consecutive_fenced_doc();
        let offsets = line_start_byte_offsets(content);
        let doc = cache::get_or_parse(content).expect("markdown parses");
        let rp = cache::render_params_hash(800.0, 14.0);

        let _lock = cache::block_height_cache_test_lock();
        cache::clear_block_height_cache();
        for (i, b) in doc.root.children.iter().enumerate() {
            let s = block_source_slice(content, &offsets, b.start_line, b.end_line);
            // Use distinct, recognisable heights so a key collision would be
            // visible in the assertion below.
            cache::insert_block_height(s, rp, 100.0 + i as f32);
        }
        for (i, b) in doc.root.children.iter().enumerate() {
            let s = block_source_slice(content, &offsets, b.start_line, b.end_line);
            assert_eq!(
                cache::get_block_height(s, rp),
                Some(100.0 + i as f32),
                "block {} key collided with another block's cached height",
                i
            );
        }
    }

    #[test]
    fn test_flush_rendered_edit_session_commits_dirty_paragraph() {
        use crate::config::ViewMode;
        use crate::markdown::rendered_session::{rendered_editor_id, BlockRef, RenderedEditSession};
        use crate::state::Tab;

        let ctx = egui::Context::default();
        crate::fonts::setup_fonts_lazy(&ctx);

        let mut tab = Tab::new(42);
        tab.content = "hello".to_string();
        tab.view_mode = ViewMode::Rendered;

        let editor_id = rendered_editor_id(tab.id);
        let mut session = RenderedEditSession::new();
        let block = BlockRef::Paragraph { line: 1 };
        session.on_text_changed(block, "hello world".to_string());
        session.active = Some(block);
        rendered_session::save_for_epoch_ctx(&ctx, editor_id, tab.source_epoch(), session);

        assert!(flush_rendered_edit_session(&ctx, &mut tab));
        assert_eq!(tab.content, "hello world");

        let reloaded = rendered_session::load_for_epoch_ctx(&ctx, editor_id, tab.source_epoch());
        assert!(
            !reloaded
                .blocks
                .get(&block)
                .map(|s| s.dirty)
                .unwrap_or(true)
        );
    }
}
