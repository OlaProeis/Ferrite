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

use std::collections::{HashMap, HashSet};

use egui::{Pos2, Vec2};

use super::types::*;
use crate::markdown::mermaid::text::TextMeasurer;

use config::FlowLayoutConfig;
use graph::FlowGraph;
use sugiyama::SugiyamaLayout;

fn stable_min_width_for_title(
    title: &str,
    font_size: f32,
    text_measurer: &impl TextMeasurer,
) -> f32 {
    let title_text_size = text_measurer.measure(title, font_size);
    (title_text_size.width + 24.0).ceil()
}

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
    let hint_ids: HashSet<String> = flowchart.position_hints.keys().cloned().collect();
    let sugiyama = SugiyamaLayout::new(
        graph,
        flowchart.direction,
        config.clone(),
        available_width,
        hint_ids,
    );
    let mut layout = sugiyama.compute();

    // Compute subgraph bounding boxes
    compute_subgraph_layouts(&mut layout, flowchart, &config, font_size, text_measurer);

    // Override Sugiyama positions with manual `%% @pos` hints (layout-space top-left).
    apply_position_hints(&mut layout, &flowchart.position_hints);
    recompute_layout_bounds(&mut layout, config.margin);

    layout
}

/// Apply manual `%% @pos` overrides after automatic layout normalization.
fn apply_position_hints(layout: &mut FlowchartLayout, hints: &HashMap<String, Pos2>) {
    for (node_id, hint_pos) in hints {
        if let Some(node_layout) = layout.nodes.get_mut(node_id) {
            node_layout.pos = *hint_pos;
        }
    }
}

/// Recompute diagram bounds after manual position overrides.
fn recompute_layout_bounds(layout: &mut FlowchartLayout, margin: f32) {
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;

    for node_layout in layout.nodes.values() {
        max_x = max_x.max(node_layout.pos.x + node_layout.size.x);
        max_y = max_y.max(node_layout.pos.y + node_layout.size.y);
    }

    for sg_layout in layout.subgraphs.values() {
        max_x = max_x.max(sg_layout.pos.x + sg_layout.size.x);
        max_y = max_y.max(sg_layout.pos.y + sg_layout.size.y);
    }

    layout.total_size = Vec2::new(max_x + margin, max_y + margin);
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
                let min_width_for_title =
                    stable_min_width_for_title(title, font_size, text_measurer);
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
                let min_width_for_title =
                    stable_min_width_for_title(title, font_size, text_measurer);
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

    // Shift so content starts at margin (handles negative coords and leftover
    // slack from older layouts that centered layers in available_width).
    let shift_x = config.margin - min_x;
    let shift_y = config.margin - min_y;

    if shift_x.abs() > 0.01 || shift_y.abs() > 0.01 {
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

    layout.total_size.x = max_x + config.margin;
    layout.total_size.y = max_y + config.margin;
}

#[cfg(test)]
mod pos_hint_layout_tests {
    use super::*;
    use egui::Rect;

    use crate::markdown::mermaid::flowchart::parse_flowchart;
    use crate::markdown::mermaid::text::EstimatedTextMeasurer;

    const EPS: f32 = 0.01;

    fn pos2(x: f32, y: f32) -> Pos2 {
        Pos2::new(x, y)
    }

    fn assert_pos_close(actual: Pos2, expected: Pos2) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "expected ({}, {}), got ({}, {})",
            expected.x,
            expected.y,
            actual.x,
            actual.y
        );
    }

    fn td_edge_endpoints(from_rect: Rect, to_rect: Rect) -> (Pos2, Pos2) {
        (
            Pos2::new(from_rect.center().x, from_rect.bottom()),
            Pos2::new(to_rect.center().x, to_rect.top()),
        )
    }

    fn assert_point_on_rect_edge(point: Pos2, rect: Rect) {
        let on_vertical = (point.x - rect.left()).abs() < EPS || (point.x - rect.right()).abs() < EPS;
        let on_horizontal = (point.y - rect.top()).abs() < EPS || (point.y - rect.bottom()).abs() < EPS;
        assert!(
            on_vertical || on_horizontal,
            "point ({}, {}) is not on rect {:?}",
            point.x,
            point.y,
            rect
        );
        assert!(
            point.x >= rect.left() - EPS
                && point.x <= rect.right() + EPS
                && point.y >= rect.top() - EPS
                && point.y <= rect.bottom() + EPS,
            "point ({}, {}) lies outside rect {:?}",
            point.x,
            point.y,
            rect
        );
    }

    #[test]
    fn pos_hint_overrides_land_at_hint_coordinates() {
        let source = r#"flowchart TD
    A[Start]
    B[Middle]
    C[End]
%% @pos A 120 80
%% @pos C 420 280
    A --> B --> C"#;

        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        assert_pos_close(layout.nodes["A"].pos, pos2(120.0, 80.0));
        assert_pos_close(layout.nodes["C"].pos, pos2(420.0, 280.0));
    }

    #[test]
    fn pos_hint_unhinted_nodes_match_auto_layout() {
        let base = r#"flowchart TD
    A[Start]
    B[Middle]
    C[End]
    A --> B --> C"#;

        let hinted_source = r#"flowchart TD
    A[Start]
    B[Middle]
    C[End]
%% @pos A 120 80
%% @pos C 420 280
    A --> B --> C"#;

        let text_measurer = EstimatedTextMeasurer::new();
        let auto = layout_flowchart(&parse_flowchart(base).unwrap(), 800.0, 14.0, &text_measurer);
        let hinted =
            layout_flowchart(&parse_flowchart(hinted_source).unwrap(), 800.0, 14.0, &text_measurer);

        assert_pos_close(hinted.nodes["B"].pos, auto.nodes["B"].pos);
    }

    #[test]
    fn pos_hint_edges_attach_to_overridden_rects() {
        let source = r#"flowchart TD
    A[Start]
    B[Middle]
    C[End]
%% @pos A 120 80
%% @pos C 420 280
    A --> B --> C"#;

        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let a_rect = Rect::from_min_size(layout.nodes["A"].pos, layout.nodes["A"].size);
        let b_rect = Rect::from_min_size(layout.nodes["B"].pos, layout.nodes["B"].size);
        let c_rect = Rect::from_min_size(layout.nodes["C"].pos, layout.nodes["C"].size);

        let (ab_start, ab_end) = td_edge_endpoints(a_rect, b_rect);
        let (bc_start, bc_end) = td_edge_endpoints(b_rect, c_rect);

        assert_point_on_rect_edge(ab_start, a_rect);
        assert_point_on_rect_edge(ab_end, b_rect);
        assert_point_on_rect_edge(bc_start, b_rect);
        assert_point_on_rect_edge(bc_end, c_rect);
    }

    #[test]
    fn pos_hint_invalid_id_does_not_affect_layout() {
        let base = r#"flowchart TD
    A[Start] --> B[End]"#;

        let warned_source = r#"flowchart TD
    A[Start] --> B[End]
%% @pos Missing 10 20"#;

        let base_fc = parse_flowchart(base).unwrap();
        let warned_fc = parse_flowchart(warned_source).unwrap();
        assert!(
            warned_fc
                .warnings
                .iter()
                .any(|w| w.message.contains("Unknown node id 'Missing'")),
            "expected unknown-node warning"
        );

        let text_measurer = EstimatedTextMeasurer::new();
        let base_layout = layout_flowchart(&base_fc, 800.0, 14.0, &text_measurer);
        let warned_layout = layout_flowchart(&warned_fc, 800.0, 14.0, &text_measurer);

        assert_pos_close(warned_layout.nodes["A"].pos, base_layout.nodes["A"].pos);
        assert_pos_close(warned_layout.nodes["B"].pos, base_layout.nodes["B"].pos);
    }
}
