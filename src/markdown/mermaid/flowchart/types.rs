//! Core AST types for flowchart diagrams.
//!
//! This module contains all data structures representing the parsed
//! flowchart model: directions, node shapes, edges, subgraphs, and styles.

use egui::{Color32, Pos2, Vec2};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// AST Types
// ─────────────────────────────────────────────────────────────────────────────

/// Direction of the flowchart layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowDirection {
    #[default]
    TopDown, // TD or TB
    BottomUp,  // BT
    LeftRight, // LR
    RightLeft, // RL
}

/// Shape of a flowchart node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    #[default]
    Rectangle, // [text]
    RoundRect,     // (text)
    Stadium,       // ([text])
    Diamond,       // {text}
    Hexagon,       // {{text}}
    Parallelogram, // [/text/]
    Circle,        // ((text))
    Cylinder,      // [(text)]
    Subroutine,    // [[text]]
    Asymmetric,    // >text]
}

/// A node in the flowchart.
#[derive(Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
}

/// Style of an edge line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeStyle {
    #[default]
    Solid, // ---
    Dotted, // -.-
    Thick,  // ===
}

/// Type of arrow head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    Arrow, // >
    Circle, // o
    Cross,  // x
    None,
}

/// An edge connecting two nodes.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub arrow_start: ArrowHead,
    pub arrow_end: ArrowHead,
}

/// A subgraph (cluster) in a flowchart.
#[derive(Debug, Clone)]
pub struct FlowSubgraph {
    /// Unique identifier for the subgraph
    pub id: String,
    /// Display title (may differ from id)
    pub title: Option<String>,
    /// IDs of nodes directly contained in this subgraph
    pub node_ids: Vec<String>,
    /// IDs of nested subgraphs
    pub child_subgraph_ids: Vec<String>,
    /// Optional direction override for this subgraph (for future use)
    #[allow(dead_code)]
    pub direction: Option<FlowDirection>,
}

/// Custom style for a node defined via classDef.
#[derive(Debug, Clone)]
pub struct NodeStyle {
    /// Fill color (background)
    pub fill: Option<Color32>,
    /// Stroke color (border)
    pub stroke: Option<Color32>,
    /// Stroke width
    pub stroke_width: Option<f32>,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: None,
        }
    }
}

/// Custom style for an edge defined via linkStyle directive.
#[derive(Debug, Clone, Default)]
pub struct LinkStyle {
    /// Stroke color
    pub stroke: Option<Color32>,
    /// Stroke width
    pub stroke_width: Option<f32>,
}

/// A parsed flowchart.
#[derive(Debug, Clone, Default)]
pub struct Flowchart {
    pub direction: FlowDirection,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    /// Subgraphs in the flowchart (order matters for rendering - parents before children)
    pub subgraphs: Vec<FlowSubgraph>,
    /// Class definitions: class_name -> NodeStyle
    pub class_defs: HashMap<String, NodeStyle>,
    /// Node class assignments: node_id -> class_name
    pub node_classes: HashMap<String, String>,
    /// Link styles: edge_index -> LinkStyle
    pub link_styles: HashMap<usize, LinkStyle>,
    /// Default style applied to all edges without explicit style
    pub default_link_style: Option<LinkStyle>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Layout Types
// ─────────────────────────────────────────────────────────────────────────────

/// Layout information for a node.
#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub pos: Pos2,
    pub size: Vec2,
}

/// Layout information for a subgraph.
#[derive(Debug, Clone)]
pub struct SubgraphLayout {
    /// Bounding box position (top-left corner)
    pub pos: Pos2,
    /// Bounding box size
    pub size: Vec2,
    /// Title to display (if any)
    pub title: Option<String>,
}

/// Complete layout for a flowchart.
#[derive(Debug, Clone, Default)]
pub struct FlowchartLayout {
    pub nodes: HashMap<String, NodeLayout>,
    pub subgraphs: HashMap<String, SubgraphLayout>,
    pub total_size: Vec2,
    /// Set of back-edges (cycles): (from_node_id, to_node_id)
    pub back_edges: std::collections::HashSet<(String, String)>,
}
