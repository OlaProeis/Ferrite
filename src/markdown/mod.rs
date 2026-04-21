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

mod ast_ops;
pub mod cache;
pub mod csv_viewer;
mod editor;
pub mod formatting;
pub mod mermaid;
mod parser;
pub mod syntax;
pub mod toc;
pub mod tree_viewer;
mod widgets;

// Only export what's actually used by the app
pub use csv_viewer::{
    delimiter_display_name, delimiter_symbol, get_tabular_file_type, CsvViewer, CsvViewerState,
    DELIMITERS,
};
pub use editor::{
    cleanup_rendered_editor_memory, EditorMode, LineMapping, MarkdownEditor, WikilinkContext,
};
pub use formatting::{
    apply_raw_format, detect_raw_formatting_state, FormattingState, MarkdownFormatCommand,
};
pub use toc::{insert_or_update_toc, TocOptions};
pub use tree_viewer::{get_structured_file_type, TreeViewer, TreeViewerState};
