//! Ferrite editor module - custom text editor widget for Ferrite.
//!
//! This module provides a high-performance text editor with:
//! - Rope-based text storage (`TextBuffer`)
//! - Virtual scrolling (`ViewState`)
//! - Galley caching (`LineCache`)
//! - Operation-based undo/redo (`EditHistory`)
//! - Modular input handling and rendering

mod buffer;
mod cursor;
mod editor;
mod find_replace;
pub(crate) mod grapheme;
mod highlights;
mod history;
mod input;
mod line_cache;
mod mouse;
mod rendering;
mod search;
mod selection;
mod shaping;
mod view;
pub mod vim;

// Re-export the main types for external use
pub use buffer::TextBuffer;
pub use cursor::{Cursor, Selection};
pub use editor::FerriteEditor;
pub use history::{compute_edit_ops, EditHistory, EditOperation};
pub use line_cache::LineCache;
pub use view::ViewState;
