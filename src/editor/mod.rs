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
#[allow(unused_imports)] // consumed when editors call `record_with_meta`
pub use ferrite::{
    compute_edit_ops, history_op_kind_for_insert_text, history_record_meta_for_ops, EditHistory,
    HistoryOpKind, HistoryRecordMeta, UNDO_COALESCE_MS,
};

// Re-export other editor types
pub use find_replace::{FindReplacePanel, FindState};
pub use line_numbers::count_lines;
pub use minimap::{
    show_split_sync_footer, Minimap, SemanticMinimap, SplitSyncFooterOutput,
    SPLIT_SYNC_FOOTER_HEIGHT,
};
pub use outline::{
    extract_outline, extract_outline_for_file, ContentType, DocumentOutline, OutlineItem,
    OutlineType, StructuredStats,
};
pub use stats::{DocumentStats, TextStats};
pub use widget::{EditorWidget, SearchHighlights};

// Re-export FerriteEditor access helpers
pub(crate) use widget::type_three_chars_after_line_end;
pub use widget::{cleanup_ferrite_editor, get_ferrite_editor_mut};
