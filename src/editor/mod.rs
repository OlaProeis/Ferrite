//! Editor module for Ferrite
//!
//! This module contains the text editor widget and related functionality
//! for editing markdown documents.

// Ferrite editor module (custom high-performance editor)
mod ferrite;

// Other editor modules
mod find_replace;
pub mod folding;
mod line_numbers;
pub mod matching;
mod minimap;
mod outline;
mod stats;
mod widget;

// Re-export Ferrite editor types
pub use ferrite::{compute_edit_ops, EditHistory};

// Re-export other editor types
pub use find_replace::{FindReplacePanel, FindState};
pub use line_numbers::count_lines;
pub use minimap::{Minimap, SemanticMinimap};
pub use outline::{
    extract_outline, extract_outline_for_file, ContentType, DocumentOutline, OutlineItem,
    OutlineType, StructuredStats,
};
pub use stats::{DocumentStats, TextStats};
pub use widget::{EditorWidget, SearchHighlights};

// Re-export FerriteEditor access helpers
pub use widget::{cleanup_ferrite_editor, get_ferrite_editor_mut};
