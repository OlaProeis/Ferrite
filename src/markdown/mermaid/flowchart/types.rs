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
    Trapezoid,     // [/text\]
    TrapezoidInv,  // [\text/]
    Circle,        // ((text))
    DoubleCircle,  // (((text)))
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
    /// Label text color (`color:` in classDef / style)
    pub color: Option<Color32>,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: None,
            color: None,
        }
    }
}

impl NodeStyle {
    /// Merge class-based style with per-node `style` directive; explicit `style` fields win.
    pub fn merge_class_and_inline(
        class: Option<&NodeStyle>,
        inline: Option<&NodeStyle>,
    ) -> Option<NodeStyle> {
        match (class, inline) {
            (None, None) => None,
            (Some(c), None) => Some(c.clone()),
            (None, Some(i)) => Some(i.clone()),
            (Some(c), Some(i)) => Some(NodeStyle {
                fill: i.fill.or(c.fill),
                stroke: i.stroke.or(c.stroke),
                stroke_width: i.stroke_width.or(c.stroke_width),
                color: i.color.or(c.color),
            }),
        }
    }

    /// Apply only `Some` fields from `overlay` (e.g. merging multiple `style` lines for one node).
    pub fn apply_overlay(&mut self, overlay: &NodeStyle) {
        if overlay.fill.is_some() {
            self.fill = overlay.fill;
        }
        if overlay.stroke.is_some() {
            self.stroke = overlay.stroke;
        }
        if overlay.stroke_width.is_some() {
            self.stroke_width = overlay.stroke_width;
        }
        if overlay.color.is_some() {
            self.color = overlay.color;
        }
    }
}

/// A parse-time warning (non-fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowchartWarning {
    /// 1-indexed line within the diagram source (header is line 1).
    pub line: usize,
    pub message: String,
}

/// Custom style for an edge defined via linkStyle directive.
#[derive(Debug, Clone, Default)]
pub struct LinkStyle {
    /// Stroke color
    pub stroke: Option<Color32>,
    /// Stroke width
    pub stroke_width: Option<f32>,
    /// When true, edges use smooth Catmull-Rom curves (`linkStyle … interpolate basis`).
    pub interpolate_basis: bool,
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
    /// Per-node `style nodeId …` directives (override class fields where set)
    pub node_styles: HashMap<String, NodeStyle>,
    /// Link styles: edge_index -> LinkStyle
    pub link_styles: HashMap<usize, LinkStyle>,
    /// Default style applied to all edges without explicit style
    pub default_link_style: Option<LinkStyle>,
    /// Manual position hints from `%% @pos <node_id> <x> <y>` comments.
    pub position_hints: HashMap<String, Pos2>,
    /// Non-fatal parse warnings (e.g. invalid `@pos` hints).
    pub warnings: Vec<FlowchartWarning>,
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
