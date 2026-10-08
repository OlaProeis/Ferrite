//! Preview and sync scrolling module for Ferrite
//!
//! This module provides synchronized scrolling between Raw and Rendered
//! markdown views, allowing users to see corresponding content in both panes.

mod sync_scroll;

// Note: ScrollOrigin is available for future split-view bidirectional sync scrolling
#[allow(unused_imports)]
pub use sync_scroll::{ScrollOrigin, SyncScrollState, SPLIT_SCROLL_IDLE};
