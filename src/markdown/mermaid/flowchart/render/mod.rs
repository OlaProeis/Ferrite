//! Flowchart rendering using egui.
//!
//! Handles drawing of nodes, edges, subgraphs, and labels.

pub(crate) mod colors;
pub(crate) mod edges;
pub(crate) mod nodes;
pub(crate) mod subgraphs;

use std::collections::{HashMap, HashSet};

use egui::Vec2;

pub use colors::FlowchartColors;
use edges::{
    back_edge_horizontal_padding, compute_back_edge_lanes, compute_forward_edge_lanes, draw_edge,
    mark_bidirectional_drawn, should_skip_bidirectional_duplicate, EdgeLabelInfo,
};
use nodes::draw_node;
use subgraphs::{compute_subgraph_depths, draw_subgraph};

use super::types::*;
use super::utils::layout_content_size;
use crate::markdown::mermaid::text::{EguiTextMeasurer, TextMeasurer};

/// Natural paint size for a flowchart layout (matches [`render_flowchart`] allocation).
pub fn flowchart_diagram_size(flowchart: &Flowchart, layout: &FlowchartLayout) -> Vec2 {
    const LAYOUT_MARGIN: f32 = 20.0;
    let content_size = layout_content_size(layout, LAYOUT_MARGIN);
    let (left_pad, right_pad) = back_edge_horizontal_padding(layout, flowchart.direction);
    Vec2::new(
        content_size.x.max(layout.total_size.x) + left_pad + right_pad,
        content_size.y.max(layout.total_size.y),
    )
}

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

    let alloc_size = flowchart_diagram_size(flowchart, layout);
    let (left_pad, _) = back_edge_horizontal_padding(layout, flowchart.direction);

    ui.set_min_size(alloc_size);
    let (rect, _response) = ui.allocate_exact_size(alloc_size, egui::Sense::hover());
    let offset = rect.min.to_vec2() + Vec2::new(left_pad, 0.0);
    let painter = ui.painter_at(rect);
    let back_edge_lanes = compute_back_edge_lanes(layout, flowchart.direction, offset);
    let forward_edge_lanes = compute_forward_edge_lanes(flowchart, layout, flowchart.direction);
    let mut drawn_bidirectional: HashSet<(String, String)> = HashSet::new();

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
        if should_skip_bidirectional_duplicate(edge, &drawn_bidirectional) {
            continue;
        }

        if let (Some(from_layout), Some(to_layout)) =
            (layout.nodes.get(&edge.from), layout.nodes.get(&edge.to))
        {
            mark_bidirectional_drawn(edge, &mut drawn_bidirectional);
            let label_info = edge_labels.get(&idx);
            let is_back_edge = layout
                .back_edges
                .contains(&(edge.from.clone(), edge.to.clone()));
            let back_edge_lane = if is_back_edge {
                back_edge_lanes
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied()
            } else {
                None
            };
            let forward_edge_lane = if is_back_edge {
                None
            } else {
                forward_edge_lanes
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied()
            };
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
                back_edge_lane,
                forward_edge_lane,
                flowchart,
                &layout.subgraphs,
                &layout.nodes,
            );
        }
    }

    // Draw nodes (on top)
    for node in &flowchart.nodes {
        if let Some(node_layout) = layout.nodes.get(&node.id) {
            // ClassDef + per-node `style` (inline wins per field)
            let class_style = flowchart
                .node_classes
                .get(&node.id)
                .and_then(|class_name| flowchart.class_defs.get(class_name));
            let inline_style = flowchart.node_styles.get(&node.id);
            let merged = NodeStyle::merge_class_and_inline(class_style, inline_style);
            draw_node(
                &painter,
                node,
                node_layout,
                offset,
                colors,
                font_size,
                merged.as_ref(),
            );
        }
    }
}
