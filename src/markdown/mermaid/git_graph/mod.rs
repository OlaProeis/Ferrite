//! Git graph diagram parsing and rendering.

pub mod layout;
mod parser;
pub mod render;
mod types;

pub use parser::parse_git_graph;
pub use render::render_git_graph;
pub use types::*;
