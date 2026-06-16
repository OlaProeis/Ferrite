//! Markdown parsing, rendering, and WYSIWYG editing module
//!
//! This module provides markdown parsing, HTML rendering, and WYSIWYG editing
//! functionality using the comrak library, a CommonMark + GFM compatible parser.
//!
//! # Features
//! - Parse markdown text to AST (Abstract Syntax Tree)
//! - Render markdown to HTML
//! - GitHub Flavored Markdown (GFM) support
//! - Configurable parsing options
//! - WYSIWYG editor widget for egui
//! - Editable widgets for headings, paragraphs, and lists
//! - Syntax highlighting for code blocks using syntect
//!
//! # Example
//! ```ignore
//! use crate::markdown::{parse_markdown, render_to_html, MarkdownDocument};
//! use crate::markdown::{MarkdownEditor, EditorMode};
//! use crate::markdown::{EditableHeading, EditableParagraph, EditableList};
//! use crate::markdown::{highlight_code, SyntaxHighlighter};
//!
//! // Parsing
//! let markdown = "# Hello\n\nThis is **bold** text.";
//! let doc = parse_markdown(markdown)?;
//! let html = render_to_html(markdown)?;
//!
//! // WYSIWYG Editing
//! let output = MarkdownEditor::new(&mut content)
//!     .mode(EditorMode::Rendered)
//!     .show(ui);
//!
//! // Individual Widgets
//! let mut text = "Heading".to_string();
//! let mut level = HeadingLevel::H1;
//! let output = EditableHeading::new(&mut text, &mut level).show(ui);
//!
//! // Syntax Highlighting
//! let highlighted = highlight_code("fn main() {}", "rust", true);
//! ```

#[inline]
pub(crate) fn markdown_accent_temp_id() -> eframe::egui::Id {
    // Fixed id: MarkdownEditor stamps accent here each frame for widget fallbacks.
    eframe::egui::Id::new("__ferrite_markdown_accent__")
}

#[inline]
pub(crate) fn preview_locked_temp_id() -> eframe::egui::Id {
    eframe::egui::Id::new("__ferrite_preview_locked__")
}

/// O(1) read of the per-frame preview-lock flag stamped by [`MarkdownEditor`].
#[inline]
pub(crate) fn preview_locked_from_ctx(ctx: &eframe::egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(preview_locked_temp_id())
            .unwrap_or(false)
    })
}

#[inline]
pub(crate) fn preview_locked_from_ui(ui: &eframe::egui::Ui) -> bool {
    preview_locked_from_ctx(ui.ctx())
}

mod ansi_render;
mod ast_ops;
pub mod cache;
mod code_execution;
pub mod csv_viewer;
mod editor;
pub mod formatting;
pub mod mermaid;
mod parser;
pub mod video_embed;
pub(crate) mod video_webview_input;
pub(crate) mod video_render;
pub mod rendered_commit_undo;
pub mod rendered_session;
pub mod syntax;
mod table_cell_nav;
pub mod toc;
pub mod tree_viewer;
mod widgets;

// Only export what's actually used by the app
pub use code_execution::{
    drain_code_execution_toasts, spawn_run, take_pending_code_execution_consent, CodeExecutionUi,
};
pub use csv_viewer::{
    delimiter_display_name, delimiter_symbol, get_tabular_file_type, CsvViewer, CsvViewerState,
    DELIMITERS,
};
pub use editor::{
    cleanup_rendered_editor_memory, flush_rendered_edit_session, EditorMode, LineMapping,
    MarkdownEditor, WikilinkContext,
};
pub use rendered_session::rendered_editor_id;
pub use formatting::{
    apply_raw_format, detect_raw_formatting_state, FormattingState, MarkdownFormatCommand,
};
pub use mermaid::compute_mermaid_diagnostics;
pub use toc::{insert_or_update_toc, TocOptions};
pub use tree_viewer::{get_structured_file_type, TreeViewer, TreeViewerState};
pub use parser::{VideoEmbedInfo, VideoProvider};
pub use video_embed::parse_video_embed_url;
pub use video_render::{
    pop_video_webview_render_slot, push_video_webview_render_slot, VideoWebViewManager,
    VideoWebViewParent,
};
pub use widgets::{detect_mermaid_diagram_type, MermaidDiagramType};
