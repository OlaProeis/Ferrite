//! Flowchart diagram parsing, layout, and rendering.
//!
//! This module implements Mermaid flowchart/graph diagrams with support for:
//! - Multiple flow directions (TD, TB, LR, RL, BT)
//! - Various node shapes (rectangle, diamond, circle, etc.)
//! - Edge styles (solid, dotted, thick)
//! - Arrow types (arrow, circle, cross, bidirectional)
//! - Subgraphs with nesting
//! - Chained edges (A --> B --> C)
//! - Cycle detection and back-edge rendering
//!
//! # Module Structure
//!
//! - `types` - Core AST types and layout result types
//! - `parser` - Mermaid source text to AST conversion
//! - `layout` - Sugiyama-style layered graph layout algorithm
//! - `render` - egui drawing of nodes, edges, and subgraphs
//! - `utils` - Shared utility functions (bezier curves, arrow heads, etc.)

pub(crate) mod layout;
pub(crate) mod parser;
pub(crate) mod render;
pub(crate) mod types;
pub(crate) mod utils;

// Re-export public API
pub use layout::layout_flowchart;
pub use parser::parse_flowchart;
pub use render::{render_flowchart, FlowchartColors};
pub use types::*;
