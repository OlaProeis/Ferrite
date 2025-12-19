//! Preview and sync scrolling module for Ferrite
//!
//! This module provides synchronized scrolling between Raw and Rendered
//! markdown views, allowing users to see corresponding content in both panes.

mod sync_scroll;

pub use sync_scroll::{ScrollOrigin, SyncScrollState};
