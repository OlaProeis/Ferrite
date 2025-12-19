//! Editor module for Ferrite
//!
//! This module contains the text editor widget and related functionality
//! for editing markdown documents.

mod find_replace;
mod line_numbers;
mod outline;
mod stats;
mod widget;

// Only export what's actually used by the app
pub use find_replace::{FindReplacePanel, FindState};
pub use line_numbers::count_lines;
pub use outline::{
    extract_outline_for_file, DocumentOutline, OutlineItem, OutlineType, StructuredStats,
};
pub use stats::TextStats;
pub use widget::{EditorWidget, SearchHighlights};
