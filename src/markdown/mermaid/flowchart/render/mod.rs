//! Flowchart rendering using egui.
//!
//! Handles drawing of nodes, edges, subgraphs, and labels.

pub(crate) mod colors;
pub(crate) mod edges;
pub(crate) mod nodes;
pub(crate) mod subgraphs;

use std::collections::HashMap;

use egui::Vec2;

pub use colors::FlowchartColors;
use edges::{draw_edge, EdgeLabelInfo};
use nodes::draw_node;
use subgraphs::{compute_subgraph_depths, draw_subgraph};

use super::types::*;
use crate::markdown::mermaid::text::{EguiTextMeasurer, TextMeasurer};

/// Render a flowchart to the UI.
pub fn render_flowchart(
    ui: &mut egui::Ui,
    flowchart: &Flowchart,
    layout: &FlowchartLayout,
    colors: &FlowchartColors,
    font_size: f32,
) {
    if flowchart.nodes.is_empty() {
        return;
    }

    // Pre-compute edge label sizes before allocating painter
    // This avoids borrow checker issues with text measurement during drawing
    let label_font_size = font_size - 2.0;
    let edge_labels: HashMap<usize, EdgeLabelInfo> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        flowchart
            .edges
            .iter()
            .enumerate()
            .filter_map(|(idx, edge)| {
                edge.label.as_ref().map(|label| {
                    // Calculate max label width based on edge geometry
                    let (from_layout, to_layout) =
                        match (layout.nodes.get(&edge.from), layout.nodes.get(&edge.to)) {
                            (Some(f), Some(t)) => (f, t),
                            _ => return None,
                        };
                    let from_center = from_layout.pos + from_layout.size / 2.0;
                    let to_center = to_layout.pos + to_layout.size / 2.0;
                    let edge_length = ((to_center.x - from_center.x).powi(2)
                        + (to_center.y - from_center.y).powi(2))
                    .sqrt();
                    let max_label_width = edge_length.max(60.0).min(200.0) * 0.8;

                    // Measure and potentially truncate
                    let text_size = text_measurer.measure(label, label_font_size);
                    let display_text = if text_size.width > max_label_width {
                        text_measurer.truncate_with_ellipsis(
                            label,
                            label_font_size,
                            max_label_width,
                        )
                    } else {
                        label.clone()
                    };

                    let display_size = text_measurer.measure(&display_text, label_font_size);
                    let label_padding = Vec2::new(8.0, 4.0);
                    let size = Vec2::new(
                        display_size.width + label_padding.x,
                        display_size.height + label_padding.y,
                    );

                    Some((idx, EdgeLabelInfo { display_text, size }))
                })?
            })
            .collect()
    };

    // Allocate space for the diagram
    let (response, painter) = ui.allocate_painter(layout.total_size, egui::Sense::hover());
    let offset = response.rect.min.to_vec2();

    // Compute actual nesting depth for each subgraph
    let subgraph_depths = compute_subgraph_depths(flowchart);

    // Draw subgraphs first (behind everything else)
    // Draw in reverse order so parent subgraphs are behind children
    for subgraph in flowchart.subgraphs.iter().rev() {
        if let Some(sg_layout) = layout.subgraphs.get(&subgraph.id) {
            let depth = subgraph_depths.get(&subgraph.id).copied().unwrap_or(0);
            draw_subgraph(&painter, sg_layout, offset, colors, font_size, depth);
        }
    }

    // Draw edges (behind nodes but above subgraphs)
    for (idx, edge) in flowchart.edges.iter().enumerate() {
        if let (Some(from_layout), Some(to_layout)) =
            (layout.nodes.get(&edge.from), layout.nodes.get(&edge.to))
        {
            let label_info = edge_labels.get(&idx);
            let is_back_edge = layout
                .back_edges
                .contains(&(edge.from.clone(), edge.to.clone()));
            draw_edge(
                &painter,
                edge,
                idx,
                from_layout,
                to_layout,
                offset,
                colors,
                label_font_size,
                flowchart.direction,
                label_info,
                is_back_edge,
                flowchart,
                &layout.subgraphs,
            );
        }
    }

    // Draw nodes (on top)
    for node in &flowchart.nodes {
        if let Some(node_layout) = layout.nodes.get(&node.id) {
            // Look up custom style for this node
            let custom_style = flowchart
                .node_classes
                .get(&node.id)
                .and_then(|class_name| flowchart.class_defs.get(class_name));
            draw_node(
                &painter,
                node,
                node_layout,
                offset,
                colors,
                font_size,
                custom_style,
            );
        }
    }
}
