//! Flowchart layout engine.
//!
//! Implements Sugiyama-style layered graph layout with support for:
//! - Proper branching with side-by-side node placement
//! - Cycle detection and back-edge handling
//! - Edge crossing minimization using barycenter heuristic
//! - Subgraph bounding boxes with padding
//! - All flow directions (TD, BT, LR, RL)

pub(crate) mod config;
pub(crate) mod graph;
pub(crate) mod subgraph;
pub(crate) mod sugiyama;

use std::collections::HashMap;

use egui::{Pos2, Vec2};

use super::types::*;
use crate::markdown::mermaid::text::TextMeasurer;

use config::FlowLayoutConfig;
use graph::FlowGraph;
use sugiyama::SugiyamaLayout;

/// Compute layout for a flowchart using a Sugiyama-style layered graph algorithm.
///
/// The `text_measurer` parameter enables accurate text sizing. Use `EguiTextMeasurer`
/// when a UI context is available, or `EstimatedTextMeasurer` for testing.
pub fn layout_flowchart(
    flowchart: &Flowchart,
    available_width: f32,
    font_size: f32,
    text_measurer: &impl TextMeasurer,
) -> FlowchartLayout {
    if flowchart.nodes.is_empty() {
        return FlowchartLayout::default();
    }

    // Layout configuration
    let config = FlowLayoutConfig {
        node_padding: Vec2::new(24.0, 12.0),
        node_spacing: Vec2::new(50.0, 60.0),
        max_node_width: (available_width * 0.4).max(150.0),
        text_width_factor: 1.15,
        margin: 20.0,
        crossing_reduction_iterations: 4,
        subgraph_padding: 15.0,
        subgraph_title_height: 24.0,
        nested_subgraph_margin: 10.0,
    };

    // Build internal graph representation
    let graph = FlowGraph::from_flowchart(flowchart, font_size, text_measurer, &config);

    // Run the Sugiyama layout algorithm
    let sugiyama = SugiyamaLayout::new(graph, flowchart.direction, config.clone(), available_width);
    let mut layout = sugiyama.compute();

    // Compute subgraph bounding boxes
    compute_subgraph_layouts(&mut layout, flowchart, &config, font_size, text_measurer);

    layout
}

/// Compute bounding boxes for all subgraphs based on positioned nodes.
fn compute_subgraph_layouts(
    layout: &mut FlowchartLayout,
    flowchart: &Flowchart,
    config: &FlowLayoutConfig,
    font_size: f32,
    text_measurer: &impl TextMeasurer,
) {
    let mut subgraph_bounds: HashMap<String, (Pos2, Pos2)> = HashMap::new();

    for subgraph in flowchart.subgraphs.iter().rev() {
        // Check if we already have a pre-computed layout from SubgraphLayoutEngine
        if let Some(existing) = layout.subgraphs.get_mut(&subgraph.id) {
            existing.title = subgraph.title.clone();

            // Ensure subgraph width accommodates the title text
            if let Some(title) = &subgraph.title {
                let title_text_size = text_measurer.measure(title, font_size);
                let min_width_for_title = title_text_size.width + 24.0;
                if existing.size.x < min_width_for_title {
                    existing.size.x = min_width_for_title;
                }
            }

            subgraph_bounds.insert(
                subgraph.id.clone(),
                (
                    existing.pos,
                    Pos2::new(
                        existing.pos.x + existing.size.x,
                        existing.pos.y + existing.size.y,
                    ),
                ),
            );
            continue;
        }

        // No pre-computed layout, compute from node positions (fallback)
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut has_content = false;
        let mut has_nested_children = false;

        for node_id in &subgraph.node_ids {
            if let Some(node_layout) = layout.nodes.get(node_id) {
                min_x = min_x.min(node_layout.pos.x);
                min_y = min_y.min(node_layout.pos.y);
                max_x = max_x.max(node_layout.pos.x + node_layout.size.x);
                max_y = max_y.max(node_layout.pos.y + node_layout.size.y);
                has_content = true;
            }
        }

        for child_id in &subgraph.child_subgraph_ids {
            if let Some(&(child_min, child_max)) = subgraph_bounds.get(child_id) {
                let nested_margin = config.nested_subgraph_margin;
                min_x = min_x.min(child_min.x - nested_margin);
                min_y = min_y.min(child_min.y - nested_margin);
                max_x = max_x.max(child_max.x + nested_margin);
                max_y = max_y.max(child_max.y + nested_margin);
                has_content = true;
                has_nested_children = true;
            }
        }

        if has_content {
            let effective_padding = if has_nested_children {
                config.subgraph_padding + config.nested_subgraph_margin
            } else {
                config.subgraph_padding
            };

            let padded_min = Pos2::new(
                min_x - effective_padding,
                min_y - effective_padding - config.subgraph_title_height,
            );
            let mut padded_max = Pos2::new(max_x + effective_padding, max_y + effective_padding);

            // Ensure subgraph width accommodates the title text
            if let Some(title) = &subgraph.title {
                let title_text_size = text_measurer.measure(title, font_size);
                let min_width_for_title = title_text_size.width + 24.0;
                let current_width = padded_max.x - padded_min.x;
                if current_width < min_width_for_title {
                    padded_max.x = padded_min.x + min_width_for_title;
                }
            }

            subgraph_bounds.insert(subgraph.id.clone(), (padded_min, padded_max));

            let size = Vec2::new(padded_max.x - padded_min.x, padded_max.y - padded_min.y);
            layout.subgraphs.insert(
                subgraph.id.clone(),
                SubgraphLayout {
                    pos: padded_min,
                    size,
                    title: subgraph.title.clone(),
                },
            );
        }
    }

    // Calculate true bounds including all nodes and subgraphs
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node_layout in layout.nodes.values() {
        min_x = min_x.min(node_layout.pos.x);
        min_y = min_y.min(node_layout.pos.y);
        max_x = max_x.max(node_layout.pos.x + node_layout.size.x);
        max_y = max_y.max(node_layout.pos.y + node_layout.size.y);
    }

    for sg_layout in layout.subgraphs.values() {
        min_x = min_x.min(sg_layout.pos.x);
        min_y = min_y.min(sg_layout.pos.y);
        max_x = max_x.max(sg_layout.pos.x + sg_layout.size.x);
        max_y = max_y.max(sg_layout.pos.y + sg_layout.size.y);
    }

    // If any content extends into negative coordinates, shift everything
    let shift_x = if min_x < 0.0 {
        -min_x + config.margin
    } else {
        0.0
    };
    let shift_y = if min_y < 0.0 {
        -min_y + config.margin
    } else {
        0.0
    };

    if shift_x > 0.0 || shift_y > 0.0 {
        for node_layout in layout.nodes.values_mut() {
            node_layout.pos.x += shift_x;
            node_layout.pos.y += shift_y;
        }

        for sg_layout in layout.subgraphs.values_mut() {
            sg_layout.pos.x += shift_x;
            sg_layout.pos.y += shift_y;
        }

        max_x += shift_x;
        max_y += shift_y;
    }

    layout.total_size.x = layout.total_size.x.max(max_x + config.margin);
    layout.total_size.y = layout.total_size.y.max(max_y + config.margin);
}
