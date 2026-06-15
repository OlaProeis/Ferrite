//! Edge routing and rendering for flowchart diagrams.

use std::collections::HashMap;

use egui::{CornerRadius, FontId, Pos2, Rect, Stroke, Vec2};

use super::super::types::*;
use super::super::utils::{
    catmull_rom_path_end_tangent, catmull_rom_path_midpoint, catmull_rom_path_start_tangent,
    collect_node_obstacles, draw_arrow_head, draw_catmull_rom_path, draw_dashed_catmull_rom_path,
    draw_dashed_line, find_node_subgraph, line_rect_intersection, path_intersects_any,
    segment_intersects_rect, union_rect_bounds, BACK_EDGE_LANE_SPACING, BACK_EDGE_LOOP_MARGIN,
    NODE_OBSTACLE_PADDING,
};
use super::colors::FlowchartColors;

/// Pre-computed edge label information for rendering.
pub(crate) struct EdgeLabelInfo {
    pub display_text: String,
    pub size: Vec2,
}

/// Side-channel lane assignment for a back-edge (avoids merging parallel loops).
#[derive(Debug, Clone, Copy)]
pub(crate) struct BackEdgeLane {
    /// `-1.0` = left side, `+1.0` = right side.
    pub side_sign: f32,
    /// `0` = innermost loop (closest to graph), higher = further out.
    pub lane_index: u32,
}

/// Assign distinct loop lanes for back-edges that share a target and side.
pub(crate) fn compute_back_edge_lanes(
    layout: &FlowchartLayout,
    direction: FlowDirection,
    offset: Vec2,
) -> HashMap<(String, String), BackEdgeLane> {
    let node_rects: HashMap<String, Rect> = layout
        .nodes
        .iter()
        .map(|(id, nl)| (id.clone(), Rect::from_min_size(nl.pos + offset, nl.size)))
        .collect();

    let graph_bounds = union_rect_bounds(&node_rects.values().copied().collect::<Vec<_>>());

    // Group by (target, side) → list of (source, sort key along main axis)
    let mut groups: HashMap<(String, i8), Vec<(String, f32)>> = HashMap::new();

    for (from, to) in &layout.back_edges {
        let Some(from_rect) = node_rects.get(from) else {
            continue;
        };
        let side = back_edge_side_sign(from_rect, &graph_bounds, direction);
        let sort_key = back_edge_sort_key(from_rect, direction);
        groups
            .entry((to.clone(), side))
            .or_default()
            .push((from.clone(), sort_key));
    }

    let mut lanes = HashMap::new();
    for ((to, side), mut edges) in groups {
        edges.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (lane_index, (from, _)) in edges.into_iter().enumerate() {
            lanes.insert(
                (from, to.clone()),
                BackEdgeLane {
                    side_sign: side as f32,
                    lane_index: lane_index as u32,
                },
            );
        }
    }
    lanes
}

/// Horizontal padding (left, right) so back-edge side loops are not clipped.
///
/// Only adds padding on sides where back-edges actually route; symmetric padding
/// on both sides left a visible gap when all loops use one side (e.g. FC-83a).
pub(crate) fn back_edge_horizontal_padding(
    layout: &FlowchartLayout,
    direction: FlowDirection,
) -> (f32, f32) {
    if layout.back_edges.is_empty() {
        return (0.0, 0.0);
    }

    let side_pad = back_edge_loop_padding(layout, direction);

    match direction {
        FlowDirection::TopDown | FlowDirection::BottomUp => {
            let node_rects: HashMap<String, Rect> = layout
                .nodes
                .iter()
                .map(|(id, nl)| (id.clone(), Rect::from_min_size(nl.pos, nl.size)))
                .collect();
            let graph_bounds = union_rect_bounds(&node_rects.values().copied().collect::<Vec<_>>());

            let mut left = false;
            let mut right = false;
            for (from, _) in &layout.back_edges {
                let Some(from_rect) = node_rects.get(from) else {
                    continue;
                };
                if back_edge_side_sign(from_rect, &graph_bounds, direction) < 0 {
                    left = true;
                } else {
                    right = true;
                }
            }

            (
                if left { side_pad } else { 0.0 },
                if right { side_pad } else { 0.0 },
            )
        }
        // LR/RL loops run above/below; keep prior symmetric X padding (separate issue).
        FlowDirection::LeftRight | FlowDirection::RightLeft => (side_pad, side_pad),
    }
}

fn back_edge_loop_padding(layout: &FlowchartLayout, direction: FlowDirection) -> f32 {
    let max_lanes = max_back_edge_lane_count(layout, direction, Vec2::ZERO);
    BACK_EDGE_LOOP_MARGIN
        + NODE_OBSTACLE_PADDING
        + (max_lanes.saturating_sub(1)) as f32 * BACK_EDGE_LANE_SPACING
}

/// Maximum parallel back-edge lane count (same target, same side).
pub(crate) fn max_back_edge_lane_count(
    layout: &FlowchartLayout,
    direction: FlowDirection,
    offset: Vec2,
) -> u32 {
    let lanes = compute_back_edge_lanes(layout, direction, offset);
    let mut per_group: HashMap<(String, i8), u32> = HashMap::new();
    for ((_, to), lane) in &lanes {
        let side = if lane.side_sign < 0.0 { -1 } else { 1 };
        let count = lane.lane_index + 1;
        per_group
            .entry((to.clone(), side))
            .and_modify(|max| *max = (*max).max(count))
            .or_insert(count);
    }
    per_group.values().copied().max().unwrap_or(1)
}

fn back_edge_side_sign(from_rect: &Rect, graph_bounds: &Rect, direction: FlowDirection) -> i8 {
    match direction {
        FlowDirection::TopDown | FlowDirection::BottomUp => {
            if from_rect.center().x >= graph_bounds.center().x {
                1
            } else {
                -1
            }
        }
        FlowDirection::LeftRight | FlowDirection::RightLeft => {
            if from_rect.center().y >= graph_bounds.center().y {
                1
            } else {
                -1
            }
        }
    }
}

fn back_edge_sort_key(from_rect: &Rect, direction: FlowDirection) -> f32 {
    match direction {
        FlowDirection::TopDown => from_rect.top(),
        FlowDirection::BottomUp => -from_rect.bottom(),
        FlowDirection::LeftRight => from_rect.left(),
        FlowDirection::RightLeft => -from_rect.right(),
    }
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
    back_edge_lane: Option<BackEdgeLane>,
    flowchart: &Flowchart,
    subgraph_layouts: &HashMap<String, SubgraphLayout>,
    all_nodes: &HashMap<String, NodeLayout>,
) {
    let from_rect = Rect::from_min_size(from_layout.pos + offset, from_layout.size);
    let to_rect = Rect::from_min_size(to_layout.pos + offset, to_layout.size);
    let obstacles = collect_node_obstacles(all_nodes, offset, &edge.from, &edge.to);
    let graph_bounds = union_rect_bounds(
        &all_nodes
            .values()
            .map(|layout| Rect::from_min_size(layout.pos + offset, layout.size))
            .collect::<Vec<_>>(),
    );

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

    let interpolate = link_style
        .map(|s| s.interpolate_basis)
        .unwrap_or(false);

    // Handle back-edges with curved routing (like Mermaid)
    if is_back_edge {
        let lane = back_edge_lane.unwrap_or(BackEdgeLane {
            side_sign: back_edge_side_sign(&from_rect, &graph_bounds, direction) as f32,
            lane_index: 0,
        });
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
            &obstacles,
            graph_bounds,
            lane,
            interpolate,
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
            &obstacles,
            interpolate,
        );
    }
}

/// Draw path segments as straight lines or a smooth Catmull-Rom spline.
fn draw_path_segments(
    painter: &egui::Painter,
    segments: &[(Pos2, Pos2)],
    edge_style: EdgeStyle,
    stroke: Stroke,
    interpolate: bool,
) {
    if interpolate {
        if matches!(edge_style, EdgeStyle::Dotted) {
            draw_dashed_catmull_rom_path(painter, segments, stroke, 5.0, 3.0);
        } else {
            draw_catmull_rom_path(painter, segments, stroke);
        }
        return;
    }

    for (seg_start, seg_end) in segments {
        if matches!(edge_style, EdgeStyle::Dotted) {
            draw_dashed_line(painter, *seg_start, *seg_end, stroke, 5.0, 3.0);
        } else {
            painter.line_segment([*seg_start, *seg_end], stroke);
        }
    }
}

/// Draw an arrow head aligned to a path tangent at `tip`.
fn draw_arrow_at_tangent(
    painter: &egui::Painter,
    tip: Pos2,
    tangent: Vec2,
    head_type: &ArrowHead,
    color: egui::Color32,
    stroke_width: f32,
) {
    if tangent.length_sq() <= f32::EPSILON {
        return;
    }
    let from = tip - tangent * (8.0 + stroke_width);
    draw_arrow_head(painter, from, tip, head_type, color, stroke_width);
}

/// Draw a back-edge with side-channel routing (orthogonal loop outside the graph bounds).
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
    obstacles: &[Rect],
    graph_bounds: Rect,
    lane: BackEdgeLane,
    interpolate: bool,
) {
    let path_segments =
        compute_back_edge_path(from_rect, to_rect, direction, obstacles, graph_bounds, lane);

    draw_path_segments(painter, &path_segments, edge.style, stroke, interpolate);

    if !matches!(edge.arrow_end, ArrowHead::None) {
        let last = path_segments.last().expect("back-edge path is non-empty");
        if interpolate {
            let tangent = catmull_rom_path_end_tangent(&path_segments);
            draw_arrow_at_tangent(
                painter,
                last.1,
                tangent,
                &edge.arrow_end,
                stroke_color,
                stroke_width,
            );
        } else {
            draw_arrow_head(
                painter,
                last.0,
                last.1,
                &edge.arrow_end,
                stroke_color,
                stroke_width,
            );
        }
    }

    if let Some(info) = label_info {
        let label_mid = if interpolate {
            catmull_rom_path_midpoint(&path_segments)
        } else {
            let start = path_segments
                .first()
                .map(|s| s.0)
                .unwrap_or(from_rect.center());
            let end = path_segments
                .last()
                .map(|s| s.1)
                .unwrap_or(to_rect.center());
            path_midpoint(&path_segments, start, end)
        };
        let label_pos = Pos2::new(label_mid.x - info.size.x / 2.0 - 8.0, label_mid.y);
        let label_rect = Rect::from_center_size(label_pos, info.size);
        painter.rect_filled(label_rect, CornerRadius::same(3), colors.edge_label_bg);
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
    obstacles: &[Rect],
    interpolate: bool,
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

    // Determine the path segments to draw (subgraph routing + obstacle avoidance)
    let (path_segments, label_mid) = if let Some(info) = &crossing_info {
        let (segments, _) = compute_routed_path(start, end, info, direction);
        let segments = route_around_obstacles(segments, direction, obstacles);
        let mid = if interpolate {
            catmull_rom_path_midpoint(&segments)
        } else {
            path_midpoint(&segments, start, end)
        };
        (segments, mid)
    } else {
        let (segments, mid) = route_forward_edge(start, end, direction, obstacles);
        let mid = if interpolate {
            catmull_rom_path_midpoint(&segments)
        } else {
            mid
        };
        (segments, mid)
    };

    draw_path_segments(painter, &path_segments, edge.style, stroke, interpolate);

    // Draw arrow head at end (use last segment for direction)
    if !matches!(edge.arrow_end, ArrowHead::None) {
        if interpolate {
            let tangent = catmull_rom_path_end_tangent(&path_segments);
            draw_arrow_at_tangent(
                painter,
                path_segments.last().map(|s| s.1).unwrap_or(end),
                tangent,
                &edge.arrow_end,
                stroke_color,
                stroke_width,
            );
        } else {
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
    }

    // Draw arrow head at start (for bidirectional)
    if !matches!(edge.arrow_start, ArrowHead::None) {
        if interpolate {
            let tangent = catmull_rom_path_start_tangent(&path_segments);
            draw_arrow_at_tangent(
                painter,
                path_segments.first().map(|s| s.0).unwrap_or(start),
                tangent,
                &edge.arrow_start,
                stroke_color,
                stroke_width,
            );
        } else {
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
    }

    // Draw edge label using pre-computed sizes
    if let Some(info) = label_info {
        let label_rect = Rect::from_center_size(label_mid, info.size);

        painter.rect_filled(label_rect, CornerRadius::same(3), colors.edge_label_bg);
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
            let vertically_forward = to_rect.top() >= from_rect.bottom() - 1.0;

            let start_x = if vertically_forward {
                from_center_x
            } else if (to_center_x - from_center_x).abs() < 10.0 {
                from_center_x
            } else if to_center_x < from_center_x {
                from_rect.center().x - from_rect.width() * 0.25
            } else {
                from_rect.center().x + from_rect.width() * 0.25
            };

            let end_x = if vertically_forward {
                to_center_x
            } else {
                to_center_x
            };

            (
                Pos2::new(start_x, from_rect.bottom()),
                Pos2::new(end_x, to_rect.top()),
            )
        }
        FlowDirection::BottomUp => {
            let from_center_x = from_rect.center().x;
            let to_center_x = to_rect.center().x;
            let vertically_forward = to_rect.bottom() <= from_rect.top() + 1.0;

            let start_x = if vertically_forward {
                from_center_x
            } else if (to_center_x - from_center_x).abs() < 10.0 {
                from_center_x
            } else if to_center_x < from_center_x {
                from_rect.center().x - from_rect.width() * 0.25
            } else {
                from_rect.center().x + from_rect.width() * 0.25
            };

            let end_x = if vertically_forward {
                to_center_x
            } else {
                to_center_x
            };

            (
                Pos2::new(start_x, from_rect.top()),
                Pos2::new(end_x, to_rect.bottom()),
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

/// Side-channel back-edge path: exit source → run along graph margin → enter target.
/// Inner lane (lane 0) tries a tight side return (up/along source edge, then into target)
/// before falling back to the outer margin corridor.
fn compute_back_edge_path(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    obstacles: &[Rect],
    graph_bounds: Rect,
    lane: BackEdgeLane,
) -> Vec<(Pos2, Pos2)> {
    let path = build_back_edge_side_path(
        from_rect,
        to_rect,
        direction,
        lane.side_sign,
        lane.lane_index,
        obstacles,
        graph_bounds,
    );
    if !path.is_empty() && !path_intersects_any(&path, obstacles) {
        return path;
    }

    // Fallback: try adjacent lane outward
    for extra in 1..=2 {
        let alt = BackEdgeLane {
            lane_index: lane.lane_index + extra,
            ..lane
        };
        let alt_path = build_back_edge_side_path(
            from_rect,
            to_rect,
            direction,
            alt.side_sign,
            alt.lane_index,
            obstacles,
            graph_bounds,
        );
        if !alt_path.is_empty() && !path_intersects_any(&alt_path, obstacles) {
            return alt_path;
        }
    }

    path
}

/// Inner-lane shortcut: rise along the source's outer edge (clears branch siblings like `decide`),
/// then enter the target on the matching side.
fn try_inner_back_edge_direct_path(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    side_sign: f32,
    obstacles: &[Rect],
) -> Option<Vec<(Pos2, Pos2)>> {
    let candidates = inner_back_edge_path_candidates(from_rect, to_rect, direction, side_sign);
    for waypoints in candidates {
        let segments: Vec<(Pos2, Pos2)> = waypoints.windows(2).map(|w| (w[0], w[1])).collect();
        if !path_intersects_any(&segments, obstacles) {
            return Some(segments);
        }
    }
    None
}

/// Candidate inner return paths, best-first (FC-83a: E → Preview clears `decide`).
fn inner_back_edge_path_candidates(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    side_sign: f32,
) -> Vec<Vec<Pos2>> {
    let on_positive_side = side_sign > 0.0;
    let mut out = Vec::new();

    match direction {
        FlowDirection::TopDown if from_rect.center().y > to_rect.center().y => {
            let entry_y = to_rect.center().y;
            let end = if on_positive_side {
                Pos2::new(to_rect.right(), entry_y)
            } else {
                Pos2::new(to_rect.left(), entry_y)
            };
            let edge_x = if on_positive_side {
                from_rect.right()
            } else {
                from_rect.left()
            };
            let start_top = if on_positive_side {
                Pos2::new(from_rect.right(), from_rect.top())
            } else {
                Pos2::new(from_rect.left(), from_rect.top())
            };

            // 1) Top-outer corner → rise along source east/west edge → enter target side.
            out.push(vec![start_top, Pos2::new(edge_x, entry_y), end]);

            // 2) Side-centre exit → rise along outer edge (same column).
            out.push(vec![
                Pos2::new(edge_x, from_rect.center().y),
                Pos2::new(edge_x, entry_y),
                end,
            ]);

            // 3–6) Step outward in tight increments before rising (sibling clearance).
            for step in 1..=4 {
                let pad = NODE_OBSTACLE_PADDING * step as f32;
                let loop_x = if on_positive_side {
                    from_rect.right() + pad
                } else {
                    from_rect.left() - pad
                };
                out.push(vec![
                    start_top,
                    Pos2::new(loop_x, from_rect.top()),
                    Pos2::new(loop_x, entry_y),
                    end,
                ]);
            }
        }
        FlowDirection::BottomUp if from_rect.center().y < to_rect.center().y => {
            let entry_y = to_rect.center().y;
            let end = if on_positive_side {
                Pos2::new(to_rect.right(), entry_y)
            } else {
                Pos2::new(to_rect.left(), entry_y)
            };
            let edge_x = if on_positive_side {
                from_rect.right()
            } else {
                from_rect.left()
            };
            let start_bottom = if on_positive_side {
                Pos2::new(from_rect.right(), from_rect.bottom())
            } else {
                Pos2::new(from_rect.left(), from_rect.bottom())
            };

            out.push(vec![start_bottom, Pos2::new(edge_x, entry_y), end]);
            out.push(vec![
                Pos2::new(edge_x, from_rect.center().y),
                Pos2::new(edge_x, entry_y),
                end,
            ]);

            for step in 1..=4 {
                let pad = NODE_OBSTACLE_PADDING * step as f32;
                let loop_x = if on_positive_side {
                    from_rect.right() + pad
                } else {
                    from_rect.left() - pad
                };
                out.push(vec![
                    start_bottom,
                    Pos2::new(loop_x, from_rect.bottom()),
                    Pos2::new(loop_x, entry_y),
                    end,
                ]);
            }
        }
        FlowDirection::LeftRight if from_rect.center().x > to_rect.center().x => {
            let entry_x = to_rect.center().x;
            let end = if on_positive_side {
                Pos2::new(entry_x, to_rect.bottom())
            } else {
                Pos2::new(entry_x, to_rect.top())
            };
            let edge_y = if on_positive_side {
                from_rect.bottom()
            } else {
                from_rect.top()
            };
            let start = if on_positive_side {
                Pos2::new(from_rect.left(), from_rect.bottom())
            } else {
                Pos2::new(from_rect.left(), from_rect.top())
            };
            out.push(vec![start, Pos2::new(entry_x, edge_y), end]);
        }
        FlowDirection::RightLeft if from_rect.center().x < to_rect.center().x => {
            let entry_x = to_rect.center().x;
            let end = if on_positive_side {
                Pos2::new(entry_x, to_rect.bottom())
            } else {
                Pos2::new(entry_x, to_rect.top())
            };
            let edge_y = if on_positive_side {
                from_rect.bottom()
            } else {
                from_rect.top()
            };
            let start = if on_positive_side {
                Pos2::new(from_rect.right(), from_rect.bottom())
            } else {
                Pos2::new(from_rect.right(), from_rect.top())
            };
            out.push(vec![start, Pos2::new(entry_x, edge_y), end]);
        }
        _ => {}
    }

    out
}

/// Last-resort inner loop: one lane width outside the source before joining the target.
fn try_inner_back_edge_tight_loop(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    side_sign: f32,
    obstacles: &[Rect],
) -> Option<Vec<(Pos2, Pos2)>> {
    let on_positive = side_sign > 0.0;
    match direction {
        FlowDirection::TopDown if from_rect.center().y > to_rect.center().y => {
            let loop_x = if on_positive {
                from_rect.right() + BACK_EDGE_LOOP_MARGIN
            } else {
                from_rect.left() - BACK_EDGE_LOOP_MARGIN
            };
            let entry_y = to_rect.center().y;
            let start = if on_positive {
                Pos2::new(from_rect.right(), from_rect.top())
            } else {
                Pos2::new(from_rect.left(), from_rect.top())
            };
            let end = if on_positive {
                Pos2::new(to_rect.right(), entry_y)
            } else {
                Pos2::new(to_rect.left(), entry_y)
            };
            let waypoints = vec![
                start,
                Pos2::new(loop_x, from_rect.top()),
                Pos2::new(loop_x, entry_y),
                end,
            ];
            let segments: Vec<(Pos2, Pos2)> = waypoints.windows(2).map(|w| (w[0], w[1])).collect();
            if path_intersects_any(&segments, obstacles) {
                None
            } else {
                Some(segments)
            }
        }
        _ => None,
    }
}

fn build_back_edge_side_path(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    side_sign: f32,
    lane_index: u32,
    obstacles: &[Rect],
    graph_bounds: Rect,
) -> Vec<(Pos2, Pos2)> {
    if lane_index == 0 {
        if let Some(direct) =
            try_inner_back_edge_direct_path(from_rect, to_rect, direction, side_sign, obstacles)
        {
            return direct;
        }
        // Inner lane failed: use a tight side loop just outside the source (not the graph margin).
        if let Some(tight) =
            try_inner_back_edge_tight_loop(from_rect, to_rect, direction, side_sign, obstacles)
        {
            return tight;
        }
    }

    let margin = BACK_EDGE_LOOP_MARGIN;
    let lane_offset = lane_index as f32 * BACK_EDGE_LANE_SPACING;
    let loop_coord = match direction {
        FlowDirection::TopDown | FlowDirection::BottomUp => {
            if side_sign < 0.0 {
                graph_bounds.left() - margin - lane_offset
            } else {
                graph_bounds.right() + margin + lane_offset
            }
        }
        FlowDirection::LeftRight | FlowDirection::RightLeft => {
            if side_sign < 0.0 {
                graph_bounds.top() - margin - lane_offset
            } else {
                graph_bounds.bottom() + margin + lane_offset
            }
        }
    };

    let (start, end) = back_edge_anchors(from_rect, to_rect, direction, side_sign, lane_index);

    let horizontal_target = match direction {
        FlowDirection::TopDown | FlowDirection::BottomUp => Pos2::new(loop_coord, start.y),
        FlowDirection::LeftRight | FlowDirection::RightLeft => Pos2::new(start.x, loop_coord),
    };

    let waypoints = if segment_intersects_any_obstacle(start, horizontal_target, obstacles) {
        // Same-row collision (e.g. outer lane crossing a sibling): detour before margin.
        match direction {
            FlowDirection::TopDown => {
                let drop_y = from_rect.bottom() + NODE_OBSTACLE_PADDING;
                vec![
                    start,
                    Pos2::new(start.x, drop_y),
                    Pos2::new(loop_coord, drop_y),
                    Pos2::new(loop_coord, end.y),
                    end,
                ]
            }
            FlowDirection::BottomUp => {
                let drop_y = from_rect.top() - NODE_OBSTACLE_PADDING;
                vec![
                    start,
                    Pos2::new(start.x, drop_y),
                    Pos2::new(loop_coord, drop_y),
                    Pos2::new(loop_coord, end.y),
                    end,
                ]
            }
            FlowDirection::LeftRight => {
                let drop_x = from_rect.right() + NODE_OBSTACLE_PADDING;
                vec![
                    start,
                    Pos2::new(drop_x, start.y),
                    Pos2::new(drop_x, loop_coord),
                    Pos2::new(end.x, loop_coord),
                    end,
                ]
            }
            FlowDirection::RightLeft => {
                let drop_x = from_rect.left() - NODE_OBSTACLE_PADDING;
                vec![
                    start,
                    Pos2::new(drop_x, start.y),
                    Pos2::new(drop_x, loop_coord),
                    Pos2::new(end.x, loop_coord),
                    end,
                ]
            }
        }
    } else {
        match direction {
            FlowDirection::TopDown | FlowDirection::BottomUp => {
                vec![start, horizontal_target, Pos2::new(loop_coord, end.y), end]
            }
            FlowDirection::LeftRight | FlowDirection::RightLeft => {
                vec![start, horizontal_target, Pos2::new(end.x, loop_coord), end]
            }
        }
    };

    waypoints.windows(2).map(|w| (w[0], w[1])).collect()
}

fn back_edge_anchors(
    from_rect: &Rect,
    to_rect: &Rect,
    direction: FlowDirection,
    side_sign: f32,
    lane_index: u32,
) -> (Pos2, Pos2) {
    let inner_lane = lane_index == 0;

    match direction {
        FlowDirection::TopDown => {
            let on_right = side_sign > 0.0;

            let start = if inner_lane {
                if on_right {
                    Pos2::new(from_rect.right(), from_rect.top())
                } else {
                    Pos2::new(from_rect.left(), from_rect.top())
                }
            } else if on_right {
                Pos2::new(from_rect.right(), from_rect.center().y)
            } else {
                Pos2::new(from_rect.left(), from_rect.center().y)
            };

            let end = if inner_lane {
                if on_right {
                    Pos2::new(to_rect.right(), to_rect.center().y)
                } else {
                    Pos2::new(to_rect.left(), to_rect.center().y)
                }
            } else if on_right {
                Pos2::new(to_rect.right(), to_rect.bottom())
            } else {
                Pos2::new(to_rect.left(), to_rect.bottom())
            };
            (start, end)
        }
        FlowDirection::BottomUp => {
            let on_right = side_sign > 0.0;

            let start = if inner_lane {
                if on_right {
                    Pos2::new(from_rect.right(), from_rect.bottom())
                } else {
                    Pos2::new(from_rect.left(), from_rect.bottom())
                }
            } else if on_right {
                Pos2::new(from_rect.right(), from_rect.center().y)
            } else {
                Pos2::new(from_rect.left(), from_rect.center().y)
            };

            let end = if inner_lane {
                if on_right {
                    Pos2::new(to_rect.right(), to_rect.center().y)
                } else {
                    Pos2::new(to_rect.left(), to_rect.center().y)
                }
            } else if on_right {
                Pos2::new(to_rect.right(), to_rect.top())
            } else {
                Pos2::new(to_rect.left(), to_rect.top())
            };
            (start, end)
        }
        FlowDirection::LeftRight => {
            let on_bottom = side_sign > 0.0;

            let start = if on_bottom {
                Pos2::new(from_rect.center().x, from_rect.bottom())
            } else {
                Pos2::new(from_rect.center().x, from_rect.top())
            };

            let end = if inner_lane && on_bottom {
                Pos2::new(to_rect.center().x, to_rect.bottom())
            } else if on_bottom {
                Pos2::new(to_rect.right(), to_rect.bottom())
            } else if inner_lane {
                Pos2::new(to_rect.center().x, to_rect.top())
            } else {
                Pos2::new(to_rect.left(), to_rect.top())
            };
            (start, end)
        }
        FlowDirection::RightLeft => {
            let on_bottom = side_sign > 0.0;

            let start = if on_bottom {
                Pos2::new(from_rect.center().x, from_rect.bottom())
            } else {
                Pos2::new(from_rect.center().x, from_rect.top())
            };

            let end = if inner_lane && on_bottom {
                Pos2::new(to_rect.center().x, to_rect.bottom())
            } else if on_bottom {
                Pos2::new(to_rect.left(), to_rect.bottom())
            } else if inner_lane {
                Pos2::new(to_rect.center().x, to_rect.top())
            } else {
                Pos2::new(to_rect.right(), to_rect.top())
            };
            (start, end)
        }
    }
}

/// Route a forward edge around node obstacles using orthogonal waypoints.
fn route_forward_edge(
    start: Pos2,
    end: Pos2,
    direction: FlowDirection,
    obstacles: &[Rect],
) -> (Vec<(Pos2, Pos2)>, Pos2) {
    if obstacles.is_empty() || !segment_intersects_any_obstacle(start, end, obstacles) {
        let segments = vec![(start, end)];
        let mid = path_midpoint(&segments, start, end);
        return (segments, mid);
    }

    // Try progressively wider orthogonal detours
    for attempt in 0..8 {
        let extra = NODE_OBSTACLE_PADDING * (attempt as f32 + 1.0);
        if let Some(segments) = try_orthogonal_route(start, end, direction, obstacles, extra) {
            let mid = path_midpoint(&segments, start, end);
            return (segments, mid);
        }
    }

    // Last resort: route via graph-side corridor matching back-edge side preference
    let side_segments = route_via_side_corridor(start, end, direction, obstacles);
    (
        side_segments.clone(),
        path_midpoint(&side_segments, start, end),
    )
}

fn segment_intersects_any_obstacle(from: Pos2, to: Pos2, obstacles: &[Rect]) -> bool {
    obstacles
        .iter()
        .any(|&rect| segment_intersects_rect(from, to, rect))
}

/// Re-route each segment of an existing polyline if it still hits obstacles.
fn route_around_obstacles(
    segments: Vec<(Pos2, Pos2)>,
    direction: FlowDirection,
    obstacles: &[Rect],
) -> Vec<(Pos2, Pos2)> {
    if obstacles.is_empty() {
        return segments;
    }

    let mut result = Vec::new();
    for (a, b) in segments {
        if segment_intersects_any_obstacle(a, b, obstacles) {
            let (detour, _) = route_forward_edge(a, b, direction, obstacles);
            result.extend(detour);
        } else {
            result.push((a, b));
        }
    }
    result
}

fn try_orthogonal_route(
    start: Pos2,
    end: Pos2,
    direction: FlowDirection,
    obstacles: &[Rect],
    extra: f32,
) -> Option<Vec<(Pos2, Pos2)>> {
    let candidates: Vec<Vec<(Pos2, Pos2)>> = match direction {
        FlowDirection::TopDown | FlowDirection::BottomUp => {
            let mid_y = (start.y + end.y) / 2.0;
            let bounds = union_rect_bounds(obstacles);
            vec![
                ortho_path(
                    start,
                    end,
                    vec![Pos2::new(start.x, mid_y), Pos2::new(end.x, mid_y)],
                ),
                ortho_path(start, end, vec![Pos2::new(start.x, end.y)]),
                ortho_path(
                    start,
                    end,
                    vec![
                        Pos2::new(bounds.left() - extra, start.y),
                        Pos2::new(bounds.left() - extra, end.y),
                    ],
                ),
                ortho_path(
                    start,
                    end,
                    vec![
                        Pos2::new(bounds.right() + extra, start.y),
                        Pos2::new(bounds.right() + extra, end.y),
                    ],
                ),
            ]
        }
        FlowDirection::LeftRight | FlowDirection::RightLeft => {
            let mid_x = (start.x + end.x) / 2.0;
            let bounds = union_rect_bounds(obstacles);
            vec![
                ortho_path(
                    start,
                    end,
                    vec![Pos2::new(mid_x, start.y), Pos2::new(mid_x, end.y)],
                ),
                ortho_path(start, end, vec![Pos2::new(end.x, start.y)]),
                ortho_path(
                    start,
                    end,
                    vec![
                        Pos2::new(start.x, bounds.top() - extra),
                        Pos2::new(end.x, bounds.top() - extra),
                    ],
                ),
                ortho_path(
                    start,
                    end,
                    vec![
                        Pos2::new(start.x, bounds.bottom() + extra),
                        Pos2::new(end.x, bounds.bottom() + extra),
                    ],
                ),
            ]
        }
    };

    candidates
        .into_iter()
        .find(|path| !path_intersects_any(path, obstacles))
}

fn route_via_side_corridor(
    start: Pos2,
    end: Pos2,
    direction: FlowDirection,
    obstacles: &[Rect],
) -> Vec<(Pos2, Pos2)> {
    let bounds = union_rect_bounds(obstacles);
    let margin = BACK_EDGE_LOOP_MARGIN * 2.0;

    let waypoints = match direction {
        FlowDirection::TopDown => vec![
            Pos2::new(bounds.left() - margin, start.y),
            Pos2::new(bounds.left() - margin, end.y),
        ],
        FlowDirection::BottomUp => vec![
            Pos2::new(bounds.right() + margin, start.y),
            Pos2::new(bounds.right() + margin, end.y),
        ],
        FlowDirection::LeftRight => vec![
            Pos2::new(start.x, bounds.top() - margin),
            Pos2::new(end.x, bounds.top() - margin),
        ],
        FlowDirection::RightLeft => vec![
            Pos2::new(start.x, bounds.bottom() + margin),
            Pos2::new(end.x, bounds.bottom() + margin),
        ],
    };

    ortho_path(start, end, waypoints)
}

fn ortho_path(start: Pos2, end: Pos2, waypoints: Vec<Pos2>) -> Vec<(Pos2, Pos2)> {
    let mut points = Vec::with_capacity(waypoints.len() + 2);
    points.push(start);
    points.extend(waypoints);
    points.push(end);

    points.windows(2).map(|w| (w[0], w[1])).collect()
}

fn path_midpoint(segments: &[(Pos2, Pos2)], start: Pos2, end: Pos2) -> Pos2 {
    if segments.is_empty() {
        return Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    }

    let total_len: f32 = segments.iter().map(|(a, b)| (*b - *a).length()).sum();
    if total_len <= f32::EPSILON {
        return Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    }

    let mut accumulated = 0.0;
    let target_len = total_len / 2.0;

    for (a, b) in segments {
        let seg_len = (*b - *a).length();
        if accumulated + seg_len >= target_len {
            let t = (target_len - accumulated) / seg_len;
            return *a + (*b - *a) * t;
        }
        accumulated += seg_len;
    }

    segments
        .last()
        .map(|(_, b)| *b)
        .unwrap_or(Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0))
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

#[cfg(test)]
mod back_edge_tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};

    use crate::markdown::mermaid::flowchart::{layout_flowchart, parse_flowchart};
    use crate::markdown::mermaid::flowchart::utils::expand_rect;
    use crate::markdown::mermaid::text::EstimatedTextMeasurer;

    #[test]
    fn fc_83a_inner_e_to_b_goes_up_first() {
        let source = r#"graph TD
    A[Enter Chart Definition] --> B(Preview)
    B --> C{decide}
    C --> D[Keep]
    C --> E[Edit Definition]
    E --> B
    D --> F[Save Image and Code]
    F --> B"#;

        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let e = layout.nodes.get("E").unwrap();
        let b = layout.nodes.get("B").unwrap();
        let c = layout.nodes.get("C").unwrap();
        let e_rect = Rect::from_min_size(e.pos, e.size);
        let b_rect = Rect::from_min_size(b.pos, b.size);
        let c_obstacle = expand_rect(Rect::from_min_size(c.pos, c.size), NODE_OBSTACLE_PADDING);

        let obstacles: Vec<Rect> = layout
            .nodes
            .iter()
            .filter(|(id, _)| id.as_str() != "E" && id.as_str() != "B")
            .map(|(_, nl)| expand_rect(Rect::from_min_size(nl.pos, nl.size), NODE_OBSTACLE_PADDING))
            .collect();

        let path = try_inner_back_edge_direct_path(
            &e_rect,
            &b_rect,
            FlowDirection::TopDown,
            1.0,
            &obstacles,
        )
        .expect("E→B inner direct path should be clear after branch layout");

        assert!(path.len() >= 2, "inner path has vertical + horizontal legs");
        let (v_start, v_end) = path[0];
        assert!(
            (v_start.x - v_end.x).abs() < 0.1,
            "first leg must be vertical along Edit Definition outer edge"
        );
        assert!(v_end.y < v_start.y, "first leg must go up toward Preview");
        assert!(
            v_start.x >= e_rect.right() - 0.1,
            "rise must stay on or outside E's right edge, not through its column"
        );
        assert!(
            !path_intersects_any(&path, &[c_obstacle]),
            "must not pass through decide"
        );

        let entry = path.last().unwrap().1;
        assert!((entry.x - b_rect.right()).abs() < 0.1);
        assert!((entry.y - b_rect.center().y).abs() < 0.1);
    }

    #[test]
    fn fc_83a_back_edge_padding_is_right_only() {
        let source = r#"graph TD
    A[Enter Chart Definition] --> B(Preview)
    B --> C{decide}
    C --> D[Keep]
    C --> E[Edit Definition]
    E --> B
    D --> F[Save Image and Code]
    F --> B"#;

        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let (left, right) = back_edge_horizontal_padding(&layout, FlowDirection::TopDown);
        assert_eq!(left, 0.0, "FC-83a loops are on the right; no left gutter");
        assert!(right > 0.0, "right-side loops need clearance padding");
    }
}
