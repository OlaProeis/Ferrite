//! Edge routing and rendering for flowchart diagrams.

use std::collections::HashMap;

use egui::{FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use super::super::types::*;
use super::super::utils::{
    bezier_point, draw_arrow_head, draw_bezier_curve, draw_dashed_line, find_node_subgraph,
    line_rect_intersection,
};
use super::colors::FlowchartColors;

/// Pre-computed edge label information for rendering.
pub(crate) struct EdgeLabelInfo {
    pub display_text: String,
    pub size: Vec2,
}

/// Information about how an edge crosses subgraph boundaries.
#[derive(Debug, Clone)]
struct SubgraphCrossingInfo {
    /// Entry point into a subgraph (from outside to inside)
    entry_point: Option<Pos2>,
    /// Exit point from a subgraph (from inside to outside)
    exit_point: Option<Pos2>,
}

/// Draw a single edge between two nodes.
pub(crate) fn draw_edge(
    painter: &egui::Painter,
    edge: &FlowEdge,
    edge_index: usize,
    from_layout: &NodeLayout,
    to_layout: &NodeLayout,
    offset: Vec2,
    colors: &FlowchartColors,
    label_font_size: f32,
    direction: FlowDirection,
    label_info: Option<&EdgeLabelInfo>,
    is_back_edge: bool,
    flowchart: &Flowchart,
    subgraph_layouts: &HashMap<String, SubgraphLayout>,
) {
    let from_rect = Rect::from_min_size(from_layout.pos + offset, from_layout.size);
    let to_rect = Rect::from_min_size(to_layout.pos + offset, to_layout.size);

    // Get custom link style (specific index takes precedence over default)
    let link_style = flowchart
        .link_styles
        .get(&edge_index)
        .or(flowchart.default_link_style.as_ref());

    // Edge style - base width from edge type
    let base_stroke_width = match edge.style {
        EdgeStyle::Solid => 2.0,
        EdgeStyle::Dotted => 1.5,
        EdgeStyle::Thick => 3.0,
    };

    // Apply custom stroke width if specified
    let stroke_width = link_style
        .and_then(|s| s.stroke_width)
        .unwrap_or(base_stroke_width);

    // Apply custom stroke color if specified
    let stroke_color = link_style
        .and_then(|s| s.stroke)
        .unwrap_or(colors.edge_stroke);

    let stroke = Stroke::new(stroke_width, stroke_color);

    // Handle back-edges with curved routing (like Mermaid)
    if is_back_edge {
        draw_back_edge(
            painter,
            edge,
            &from_rect,
            &to_rect,
            direction,
            stroke,
            stroke_color,
            stroke_width,
            label_info,
            label_font_size,
            colors,
        );
    } else {
        draw_normal_edge(
            painter,
            edge,
            edge_index,
            &from_rect,
            &to_rect,
            offset,
            direction,
            stroke,
            stroke_color,
            stroke_width,
            label_info,
            label_font_size,
            colors,
            flowchart,
            subgraph_layouts,
        );
    }
}

/// Draw a back-edge with curved bezier routing.
fn draw_back_edge(
    painter: &egui::Painter,
    edge: &FlowEdge,
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    stroke: Stroke,
    stroke_color: egui::Color32,
    stroke_width: f32,
    label_info: Option<&EdgeLabelInfo>,
    label_font_size: f32,
    colors: &FlowchartColors,
) {
    let (start, end, ctrl1, ctrl2) = match direction {
        FlowDirection::TopDown => {
            // Back-edge goes up: start from top of source, end at BOTTOM-CENTER of target
            let start = Pos2::new(from_rect.left(), from_rect.center().y);
            let end = Pos2::new(to_rect.center().x, to_rect.bottom());
            // Curve: go left from start, then curve up and right to bottom of target
            let curve_x = start.x - 40.0;
            let ctrl1 = Pos2::new(curve_x, start.y);
            let ctrl2 = Pos2::new(curve_x, end.y + 30.0);
            (start, end, ctrl1, ctrl2)
        }
        FlowDirection::BottomUp => {
            let start = Pos2::new(from_rect.right(), from_rect.center().y);
            let end = Pos2::new(to_rect.right(), to_rect.center().y);
            let curve_x = start.x.max(end.x) + 30.0;
            let ctrl1 = Pos2::new(curve_x, start.y);
            let ctrl2 = Pos2::new(curve_x, end.y);
            (start, end, ctrl1, ctrl2)
        }
        FlowDirection::LeftRight => {
            let start = Pos2::new(from_rect.center().x, from_rect.top());
            let end = Pos2::new(to_rect.center().x, to_rect.top());
            let curve_y = start.y.min(end.y) - 30.0;
            let ctrl1 = Pos2::new(start.x, curve_y);
            let ctrl2 = Pos2::new(end.x, curve_y);
            (start, end, ctrl1, ctrl2)
        }
        FlowDirection::RightLeft => {
            let start = Pos2::new(from_rect.center().x, from_rect.bottom());
            let end = Pos2::new(to_rect.center().x, to_rect.bottom());
            let curve_y = start.y.max(end.y) + 30.0;
            let ctrl1 = Pos2::new(start.x, curve_y);
            let ctrl2 = Pos2::new(end.x, curve_y);
            (start, end, ctrl1, ctrl2)
        }
    };

    // Draw cubic bezier curve
    draw_bezier_curve(painter, start, ctrl1, ctrl2, end, stroke);

    // Arrow at end - calculate direction from last curve segment
    if !matches!(edge.arrow_end, ArrowHead::None) {
        draw_arrow_head(
            painter,
            ctrl2,
            end,
            &edge.arrow_end,
            stroke_color,
            stroke_width,
        );
    }

    // Label at midpoint of the curve
    if let Some(info) = label_info {
        let t = 0.5;
        let mid = bezier_point(start, ctrl1, ctrl2, end, t);
        let label_pos = Pos2::new(mid.x - info.size.x / 2.0 - 8.0, mid.y);
        let label_rect = Rect::from_center_size(label_pos, info.size);
        painter.rect_filled(label_rect, Rounding::same(3.0), colors.edge_label_bg);
        painter.text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            &info.display_text,
            FontId::proportional(label_font_size),
            colors.edge_label_text,
        );
    }
}

/// Draw a normal (non-back) edge with optional subgraph boundary routing.
#[allow(clippy::too_many_arguments)]
fn draw_normal_edge(
    painter: &egui::Painter,
    edge: &FlowEdge,
    _edge_index: usize,
    from_rect: &Rect,
    to_rect: &Rect,
    offset: Vec2,
    direction: FlowDirection,
    stroke: Stroke,
    stroke_color: egui::Color32,
    stroke_width: f32,
    label_info: Option<&EdgeLabelInfo>,
    label_font_size: f32,
    colors: &FlowchartColors,
    flowchart: &Flowchart,
    subgraph_layouts: &HashMap<String, SubgraphLayout>,
) {
    // Normal edge - use smart routing based on relative positions
    let (start, end) = compute_edge_endpoints(from_rect, to_rect, direction);

    // Check for subgraph boundary crossing
    let crossing_info = get_subgraph_crossing_info(
        &edge.from,
        &edge.to,
        start,
        end,
        flowchart,
        subgraph_layouts,
        offset,
    );

    // Determine the path segments to draw
    let (path_segments, label_mid) = if let Some(info) = &crossing_info {
        compute_routed_path(start, end, info, direction)
    } else {
        // Simple direct line
        (
            vec![(start, end)],
            Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0),
        )
    };

    // Draw all path segments
    for (seg_start, seg_end) in &path_segments {
        if matches!(edge.style, EdgeStyle::Dotted) {
            draw_dashed_line(painter, *seg_start, *seg_end, stroke, 5.0, 3.0);
        } else {
            painter.line_segment([*seg_start, *seg_end], stroke);
        }
    }

    // Draw arrow head at end (use last segment for direction)
    if !matches!(edge.arrow_end, ArrowHead::None) {
        let default_seg = (start, end);
        let last_seg = path_segments.last().unwrap_or(&default_seg);
        draw_arrow_head(
            painter,
            last_seg.0,
            last_seg.1,
            &edge.arrow_end,
            stroke_color,
            stroke_width,
        );
    }

    // Draw arrow head at start (for bidirectional)
    if !matches!(edge.arrow_start, ArrowHead::None) {
        let default_seg = (start, end);
        let first_seg = path_segments.first().unwrap_or(&default_seg);
        draw_arrow_head(
            painter,
            first_seg.1,
            first_seg.0,
            &edge.arrow_start,
            stroke_color,
            stroke_width,
        );
    }

    // Draw edge label using pre-computed sizes
    if let Some(info) = label_info {
        let label_rect = Rect::from_center_size(label_mid, info.size);

        painter.rect_filled(label_rect, Rounding::same(3.0), colors.edge_label_bg);
        painter.text(
            label_mid,
            egui::Align2::CENTER_CENTER,
            &info.display_text,
            FontId::proportional(label_font_size),
            colors.edge_label_text,
        );
    }
}

/// Compute start and end points for an edge based on flow direction and node positions.
fn compute_edge_endpoints(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
) -> (Pos2, Pos2) {
    match direction {
        FlowDirection::TopDown => {
            let from_center_x = from_rect.center().x;
            let to_center_x = to_rect.center().x;

            let start_x = if (to_center_x - from_center_x).abs() < 10.0 {
                from_center_x
            } else if to_center_x < from_center_x {
                from_rect.center().x - from_rect.width() * 0.25
            } else {
                from_rect.center().x + from_rect.width() * 0.25
            };

            (
                Pos2::new(start_x, from_rect.bottom()),
                Pos2::new(to_rect.center().x, to_rect.top()),
            )
        }
        FlowDirection::BottomUp => {
            let from_center_x = from_rect.center().x;
            let to_center_x = to_rect.center().x;

            let start_x = if (to_center_x - from_center_x).abs() < 10.0 {
                from_center_x
            } else if to_center_x < from_center_x {
                from_rect.center().x - from_rect.width() * 0.25
            } else {
                from_rect.center().x + from_rect.width() * 0.25
            };

            (
                Pos2::new(start_x, from_rect.top()),
                Pos2::new(to_rect.center().x, to_rect.bottom()),
            )
        }
        FlowDirection::LeftRight => {
            let from_center_y = from_rect.center().y;
            let to_center_y = to_rect.center().y;

            let start_y = if (to_center_y - from_center_y).abs() < 10.0 {
                from_center_y
            } else if to_center_y < from_center_y {
                from_rect.center().y - from_rect.height() * 0.25
            } else {
                from_rect.center().y + from_rect.height() * 0.25
            };

            (
                Pos2::new(from_rect.right(), start_y),
                Pos2::new(to_rect.left(), to_rect.center().y),
            )
        }
        FlowDirection::RightLeft => {
            let from_center_y = from_rect.center().y;
            let to_center_y = to_rect.center().y;

            let start_y = if (to_center_y - from_center_y).abs() < 10.0 {
                from_center_y
            } else if to_center_y < from_center_y {
                from_rect.center().y - from_rect.height() * 0.25
            } else {
                from_rect.center().y + from_rect.height() * 0.25
            };

            (
                Pos2::new(from_rect.left(), start_y),
                Pos2::new(to_rect.right(), to_rect.center().y),
            )
        }
    }
}

/// Compute a routed path through subgraph boundaries.
fn compute_routed_path(
    start: Pos2,
    end: Pos2,
    info: &SubgraphCrossingInfo,
    direction: FlowDirection,
) -> (Vec<(Pos2, Pos2)>, Pos2) {
    let mut segments: Vec<(Pos2, Pos2)> = Vec::new();
    let mut waypoints: Vec<Pos2> = vec![start];

    // Add exit point from source subgraph
    if let Some(exit) = info.exit_point {
        match direction {
            FlowDirection::TopDown | FlowDirection::BottomUp => {
                let mid_y = (start.y + exit.y) / 2.0;
                if (start.x - exit.x).abs() > 5.0 {
                    waypoints.push(Pos2::new(start.x, mid_y));
                    waypoints.push(Pos2::new(exit.x, mid_y));
                }
            }
            FlowDirection::LeftRight | FlowDirection::RightLeft => {
                let mid_x = (start.x + exit.x) / 2.0;
                if (start.y - exit.y).abs() > 5.0 {
                    waypoints.push(Pos2::new(mid_x, start.y));
                    waypoints.push(Pos2::new(mid_x, exit.y));
                }
            }
        }
        waypoints.push(exit);
    }

    // Add entry point to target subgraph
    if let Some(entry) = info.entry_point {
        let last = *waypoints.last().unwrap_or(&start);
        match direction {
            FlowDirection::TopDown | FlowDirection::BottomUp => {
                if (last.x - entry.x).abs() > 5.0 {
                    let mid_y = (last.y + entry.y) / 2.0;
                    waypoints.push(Pos2::new(last.x, mid_y));
                    waypoints.push(Pos2::new(entry.x, mid_y));
                }
            }
            FlowDirection::LeftRight | FlowDirection::RightLeft => {
                if (last.y - entry.y).abs() > 5.0 {
                    let mid_x = (last.x + entry.x) / 2.0;
                    waypoints.push(Pos2::new(mid_x, last.y));
                    waypoints.push(Pos2::new(mid_x, entry.y));
                }
            }
        }
        waypoints.push(entry);
    }

    waypoints.push(end);

    // Build segments from waypoints
    for i in 0..waypoints.len() - 1 {
        segments.push((waypoints[i], waypoints[i + 1]));
    }

    // Calculate label position (midpoint of the path)
    let total_len: f32 = segments.iter().map(|(a, b)| (*b - *a).length()).sum();
    let mut accumulated = 0.0;
    let target_len = total_len / 2.0;
    let mut mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);

    for (a, b) in &segments {
        let seg_len = (*b - *a).length();
        if accumulated + seg_len >= target_len {
            let t = (target_len - accumulated) / seg_len;
            mid = *a + (*b - *a) * t;
            break;
        }
        accumulated += seg_len;
    }

    (segments, mid)
}

/// Calculate subgraph boundary crossing information for an edge.
/// Returns crossing info if the edge crosses a subgraph boundary.
fn get_subgraph_crossing_info(
    from_id: &str,
    to_id: &str,
    from_pos: Pos2,
    to_pos: Pos2,
    flowchart: &Flowchart,
    subgraph_layouts: &HashMap<String, SubgraphLayout>,
    offset: Vec2,
) -> Option<SubgraphCrossingInfo> {
    let from_sg = find_node_subgraph(from_id, flowchart);
    let to_sg = find_node_subgraph(to_id, flowchart);

    // Check if nodes are in different subgraphs
    let from_sg_id = from_sg.map(|sg| sg.id.as_str());
    let to_sg_id = to_sg.map(|sg| sg.id.as_str());

    if from_sg_id == to_sg_id {
        // Same subgraph (or both not in any) - no crossing needed
        return None;
    }

    // Case 1: From outside to inside a subgraph
    if from_sg_id.is_none() && to_sg_id.is_some() {
        if let Some(sg_layout) = to_sg_id.and_then(|id| subgraph_layouts.get(id)) {
            let sg_rect = Rect::from_min_size(sg_layout.pos + offset, sg_layout.size);
            if let Some(entry) = line_rect_intersection(from_pos, to_pos, sg_rect) {
                return Some(SubgraphCrossingInfo {
                    entry_point: Some(entry),
                    exit_point: None,
                });
            }
        }
    }

    // Case 2: From inside to outside a subgraph
    if from_sg_id.is_some() && to_sg_id.is_none() {
        if let Some(sg_layout) = from_sg_id.and_then(|id| subgraph_layouts.get(id)) {
            let sg_rect = Rect::from_min_size(sg_layout.pos + offset, sg_layout.size);
            if let Some(exit) = line_rect_intersection(from_pos, to_pos, sg_rect) {
                return Some(SubgraphCrossingInfo {
                    entry_point: None,
                    exit_point: Some(exit),
                });
            }
        }
    }

    // Case 3: From one subgraph to a different subgraph
    if from_sg_id.is_some() && to_sg_id.is_some() && from_sg_id != to_sg_id {
        let mut exit_point = None;
        let mut entry_point = None;

        // Find exit from source subgraph
        if let Some(sg_layout) = from_sg_id.and_then(|id| subgraph_layouts.get(id)) {
            let sg_rect = Rect::from_min_size(sg_layout.pos + offset, sg_layout.size);
            exit_point = line_rect_intersection(from_pos, to_pos, sg_rect);
        }

        // Find entry to target subgraph (using exit point as starting position if available)
        if let Some(sg_layout) = to_sg_id.and_then(|id| subgraph_layouts.get(id)) {
            let sg_rect = Rect::from_min_size(sg_layout.pos + offset, sg_layout.size);
            let start = exit_point.unwrap_or(from_pos);
            entry_point = line_rect_intersection(start, to_pos, sg_rect);
        }

        if exit_point.is_some() || entry_point.is_some() {
            return Some(SubgraphCrossingInfo {
                entry_point,
                exit_point,
            });
        }
    }

    None
}
