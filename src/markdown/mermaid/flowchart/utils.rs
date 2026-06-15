//! Shared utility functions for flowchart rendering.
//!
//! Contains geometry helpers, drawing primitives, and lookup functions
//! used across the render module.

use egui::{Pos2, Rect, Stroke, Vec2};

use super::types::{FlowSubgraph, Flowchart};

/// Draw a dashed line between two points.
pub(crate) fn draw_dashed_line(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dir = (end - start).normalized();
    let total_len = (end - start).length();
    let mut pos = 0.0;

    while pos < total_len {
        let seg_start = start + dir * pos;
        let seg_end_pos = (pos + dash_len).min(total_len);
        let seg_end = start + dir * seg_end_pos;
        painter.line_segment([seg_start, seg_end], stroke);
        pos = seg_end_pos + gap_len;
    }
}

/// Calculate a point on a cubic bezier curve at parameter t (0..1).
pub(crate) fn bezier_point(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    Pos2::new(
        mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
        mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
    )
}

/// Draw a cubic bezier curve by sampling points.
pub(crate) fn draw_bezier_curve(
    painter: &egui::Painter,
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    p3: Pos2,
    stroke: Stroke,
) {
    let segments = 20;
    let mut prev = p0;

    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let curr = bezier_point(p0, p1, p2, p3, t);
        painter.line_segment([prev, curr], stroke);
        prev = curr;
    }
}

/// Flatten polyline segments into ordered waypoints.
pub(crate) fn segments_to_waypoints(segments: &[(Pos2, Pos2)]) -> Vec<Pos2> {
    if segments.is_empty() {
        return Vec::new();
    }
    let mut points = vec![segments[0].0];
    for &(start, end) in segments {
        if points.last().copied() != Some(start) {
            points.push(start);
        }
        points.push(end);
    }
    points
}

fn catmull_rom_endpoint(points: &[Pos2], index: isize) -> Pos2 {
    if index <= 0 {
        return points[0];
    }
    let last = points.len() as isize - 1;
    if index >= last {
        return points[last as usize];
    }
    points[index as usize]
}

/// Sample a Catmull-Rom spline through `waypoints` (open curve).
pub(crate) fn sample_catmull_rom_path(waypoints: &[Pos2], samples_per_span: usize) -> Vec<Pos2> {
    if waypoints.is_empty() {
        return Vec::new();
    }
    if waypoints.len() == 1 {
        return vec![waypoints[0]];
    }
    if waypoints.len() == 2 {
        let p0 = waypoints[0];
        let p1 = waypoints[1];
        let dir = p1 - p0;
        let cp1 = p0 + dir * 0.33;
        let cp2 = p0 + dir * 0.66;
        let mut out = Vec::with_capacity(samples_per_span + 1);
        for i in 0..=samples_per_span {
            let t = i as f32 / samples_per_span as f32;
            out.push(bezier_point(p0, cp1, cp2, p1, t));
        }
        return out;
    }

    let mut out = Vec::new();
    for span in 0..waypoints.len() - 1 {
        let p0 = catmull_rom_endpoint(waypoints, span as isize - 1);
        let p1 = waypoints[span];
        let p2 = waypoints[span + 1];
        let p3 = catmull_rom_endpoint(waypoints, span as isize + 2);
        let cp1 = p1 + (p2 - p0) / 6.0;
        let cp2 = p2 - (p3 - p1) / 6.0;

        let start_i = if span == 0 { 0 } else { 1 };
        for i in start_i..=samples_per_span {
            let t = i as f32 / samples_per_span as f32;
            out.push(bezier_point(p1, cp1, cp2, p2, t));
        }
    }
    out
}

/// Draw a smooth Catmull-Rom path through polyline segments.
pub(crate) fn draw_catmull_rom_path(
    painter: &egui::Painter,
    segments: &[(Pos2, Pos2)],
    stroke: Stroke,
) {
    let waypoints = segments_to_waypoints(segments);
    let samples = sample_catmull_rom_path(&waypoints, 16);
    for window in samples.windows(2) {
        painter.line_segment([window[0], window[1]], stroke);
    }
}

/// Draw a dashed Catmull-Rom path through polyline segments.
pub(crate) fn draw_dashed_catmull_rom_path(
    painter: &egui::Painter,
    segments: &[(Pos2, Pos2)],
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let waypoints = segments_to_waypoints(segments);
    let samples = sample_catmull_rom_path(&waypoints, 16);
    for window in samples.windows(2) {
        draw_dashed_line(painter, window[0], window[1], stroke, dash_len, gap_len);
    }
}

/// Midpoint along a Catmull-Rom path (for edge labels).
pub(crate) fn catmull_rom_path_midpoint(segments: &[(Pos2, Pos2)]) -> Pos2 {
    let waypoints = segments_to_waypoints(segments);
    let samples = sample_catmull_rom_path(&waypoints, 16);
    if samples.len() < 2 {
        return waypoints
            .first()
            .copied()
            .unwrap_or(Pos2::ZERO);
    }
    let total_len: f32 = samples
        .windows(2)
        .map(|w| (w[1] - w[0]).length())
        .sum();
    if total_len <= f32::EPSILON {
        return samples[0];
    }
    let target = total_len / 2.0;
    let mut accumulated = 0.0;
    for window in samples.windows(2) {
        let seg_len = (window[1] - window[0]).length();
        if accumulated + seg_len >= target {
            let t = (target - accumulated) / seg_len;
            return window[0] + (window[1] - window[0]) * t;
        }
        accumulated += seg_len;
    }
    *samples.last().unwrap_or(&samples[0])
}

/// Unit direction at the end of a Catmull-Rom path (for arrow heads).
pub(crate) fn catmull_rom_path_end_tangent(segments: &[(Pos2, Pos2)]) -> Vec2 {
    let waypoints = segments_to_waypoints(segments);
    let samples = sample_catmull_rom_path(&waypoints, 16);
    if samples.len() < 2 {
        return Vec2::new(1.0, 0.0);
    }
    let a = samples[samples.len() - 2];
    let b = samples[samples.len() - 1];
    (b - a).normalized()
}

/// Unit direction at the start of a Catmull-Rom path (for bidirectional arrows).
pub(crate) fn catmull_rom_path_start_tangent(segments: &[(Pos2, Pos2)]) -> Vec2 {
    let waypoints = segments_to_waypoints(segments);
    let samples = sample_catmull_rom_path(&waypoints, 16);
    if samples.len() < 2 {
        return Vec2::new(-1.0, 0.0);
    }
    (samples[0] - samples[1]).normalized()
}

/// Draw an arrow head at the end of a line segment.
pub(crate) fn draw_arrow_head(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    head_type: &super::types::ArrowHead,
    color: egui::Color32,
    stroke_width: f32,
) {
    use super::types::ArrowHead;

    let dir = (to - from).normalized();
    let perp = Vec2::new(-dir.y, dir.x);
    let arrow_size = 8.0 + stroke_width;

    match head_type {
        ArrowHead::Arrow => {
            let tip = to;
            let left = to - dir * arrow_size + perp * (arrow_size * 0.5);
            let right = to - dir * arrow_size - perp * (arrow_size * 0.5);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, left, right],
                color,
                Stroke::NONE,
            ));
        }
        ArrowHead::Circle => {
            painter.circle_filled(to - dir * 4.0, 4.0, color);
        }
        ArrowHead::Cross => {
            let center = to - dir * 4.0;
            let size = 4.0;
            painter.line_segment(
                [
                    center - perp * size - dir * size,
                    center + perp * size + dir * size,
                ],
                Stroke::new(stroke_width, color),
            );
            painter.line_segment(
                [
                    center + perp * size - dir * size,
                    center - perp * size + dir * size,
                ],
                Stroke::new(stroke_width, color),
            );
        }
        ArrowHead::None => {}
    }
}

/// Find the innermost subgraph that contains a given node.
/// Returns None if the node is not in any subgraph.
pub(crate) fn find_node_subgraph<'a>(
    node_id: &str,
    flowchart: &'a Flowchart,
) -> Option<&'a FlowSubgraph> {
    // Subgraphs are ordered children-before-parents, so iterate in order
    // to find the innermost (most specific) subgraph first
    for subgraph in &flowchart.subgraphs {
        if subgraph.node_ids.contains(&node_id.to_string()) {
            return Some(subgraph);
        }
    }
    None
}

/// Calculate intersection point of a line segment with a rectangle's border.
/// Returns the intersection point closest to `from` on the way to `to`.
pub(crate) fn line_rect_intersection(from: Pos2, to: Pos2, rect: Rect) -> Option<Pos2> {
    let dir = to - from;

    if dir.length_sq() < 0.001 {
        return None;
    }

    let mut best_t: Option<f32> = None;

    // Left edge (x = rect.left())
    if dir.x.abs() > 0.001 {
        let t = (rect.left() - from.x) / dir.x;
        if t > 0.0 && t < 1.0 {
            let y = from.y + t * dir.y;
            if y >= rect.top() && y <= rect.bottom() {
                if best_t.is_none() || t < best_t.unwrap() {
                    best_t = Some(t);
                }
            }
        }
    }

    // Right edge (x = rect.right())
    if dir.x.abs() > 0.001 {
        let t = (rect.right() - from.x) / dir.x;
        if t > 0.0 && t < 1.0 {
            let y = from.y + t * dir.y;
            if y >= rect.top() && y <= rect.bottom() {
                if best_t.is_none() || t < best_t.unwrap() {
                    best_t = Some(t);
                }
            }
        }
    }

    // Top edge (y = rect.top())
    if dir.y.abs() > 0.001 {
        let t = (rect.top() - from.y) / dir.y;
        if t > 0.0 && t < 1.0 {
            let x = from.x + t * dir.x;
            if x >= rect.left() && x <= rect.right() {
                if best_t.is_none() || t < best_t.unwrap() {
                    best_t = Some(t);
                }
            }
        }
    }

    // Bottom edge (y = rect.bottom())
    if dir.y.abs() > 0.001 {
        let t = (rect.bottom() - from.y) / dir.y;
        if t > 0.0 && t < 1.0 {
            let x = from.x + t * dir.x;
            if x >= rect.left() && x <= rect.right() {
                if best_t.is_none() || t < best_t.unwrap() {
                    best_t = Some(t);
                }
            }
        }
    }

    best_t.map(|t| from + dir * t)
}

/// Padding around node rects when testing edge–obstacle collisions.
pub(crate) const NODE_OBSTACLE_PADDING: f32 = 8.0;

/// Margin outside the graph bounding box for back-edge loop routing.
pub(crate) const BACK_EDGE_LOOP_MARGIN: f32 = 24.0;

/// Extra horizontal spacing between parallel back-edge loops on the same side.
pub(crate) const BACK_EDGE_LANE_SPACING: f32 = 36.0;

/// Expand a rectangle by uniform padding on all sides.
pub(crate) fn expand_rect(rect: Rect, padding: f32) -> Rect {
    rect.expand(padding)
}

/// Returns true if the open segment `(from, to)` crosses `rect` interior (not merely touching at endpoints).
pub(crate) fn segment_intersects_rect(from: Pos2, to: Pos2, rect: Rect) -> bool {
    let inner = rect.shrink(1.0);
    let Some(hit) = line_rect_intersection(from, to, inner) else {
        return false;
    };
    // Ignore grazing hits at endpoints (adjacent node padding overlap).
    let dir = to - from;
    if dir.length_sq() < 0.001 {
        return false;
    }
    let t = ((hit.x - from.x) * dir.x + (hit.y - from.y) * dir.y) / dir.length_sq();
    t > 0.02 && t < 0.98
}

/// Returns true if any segment in `path` hits an obstacle rect.
pub(crate) fn path_intersects_any(path: &[(Pos2, Pos2)], obstacles: &[Rect]) -> bool {
    path.iter().any(|&(a, b)| {
        obstacles
            .iter()
            .any(|&rect| segment_intersects_rect(a, b, rect))
    })
}

/// Sample a cubic bezier and test sub-segments against obstacle rects.
pub(crate) fn bezier_intersects_any(
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    p3: Pos2,
    obstacles: &[Rect],
    samples: usize,
) -> bool {
    if obstacles.is_empty() {
        return false;
    }
    let samples = samples.max(4);
    let mut prev = p0;
    for i in 1..=samples {
        let t = i as f32 / samples as f32;
        let curr = bezier_point(p0, p1, p2, p3, t);
        if obstacles
            .iter()
            .any(|&rect| segment_intersects_rect(prev, curr, rect))
        {
            return true;
        }
        prev = curr;
    }
    false
}

/// Bounding box enclosing all rects, or zero if empty.
pub(crate) fn union_rect_bounds(rects: &[Rect]) -> Rect {
    let mut iter = rects.iter();
    let Some(first) = iter.next() else {
        return Rect::NOTHING;
    };
    let mut bounds = *first;
    for rect in iter {
        bounds = bounds.union(*rect);
    }
    bounds
}

/// Build padded obstacle rects for every node except the edge endpoints.
pub(crate) fn collect_node_obstacles(
    nodes: &std::collections::HashMap<String, super::types::NodeLayout>,
    offset: Vec2,
    exclude_from: &str,
    exclude_to: &str,
) -> Vec<Rect> {
    nodes
        .iter()
        .filter(|(id, _)| id.as_str() != exclude_from && id.as_str() != exclude_to)
        .map(|(_, layout)| {
            expand_rect(
                Rect::from_min_size(layout.pos + offset, layout.size),
                NODE_OBSTACLE_PADDING,
            )
        })
        .collect()
}

/// Content bounding size from positioned nodes and subgraphs.
pub(crate) fn layout_content_size(layout: &super::types::FlowchartLayout, margin: f32) -> Vec2 {
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    for node in layout.nodes.values() {
        max_x = max_x.max(node.pos.x + node.size.x);
        max_y = max_y.max(node.pos.y + node.size.y);
    }
    for sg in layout.subgraphs.values() {
        max_x = max_x.max(sg.pos.x + sg.size.x);
        max_y = max_y.max(sg.pos.y + sg.size.y);
    }
    Vec2::new(max_x + margin, max_y + margin)
}

#[cfg(test)]
mod obstacle_tests {
    use super::*;

    #[test]
    fn segment_through_rect_center_is_intersection() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(20.0, 20.0));
        assert!(segment_intersects_rect(
            Pos2::new(0.0, 20.0),
            Pos2::new(40.0, 20.0),
            rect
        ));
    }

    #[test]
    fn segment_bypassing_rect_is_clear() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(20.0, 20.0));
        assert!(!segment_intersects_rect(
            Pos2::new(0.0, 0.0),
            Pos2::new(40.0, 0.0),
            rect
        ));
    }

    #[test]
    fn orthogonal_detour_avoids_obstacle() {
        let obstacle = expand_rect(
            Rect::from_min_size(Pos2::new(40.0, 40.0), Vec2::new(30.0, 30.0)),
            NODE_OBSTACLE_PADDING,
        );
        let start = Pos2::new(55.0, 20.0);
        let end = Pos2::new(55.0, 90.0);
        // Route via left corridor — never crosses the obstacle column
        let path = [
            (start, Pos2::new(10.0, start.y)),
            (Pos2::new(10.0, start.y), Pos2::new(10.0, end.y)),
            (Pos2::new(10.0, end.y), end),
        ];
        assert!(!path_intersects_any(&path, &[obstacle]));
    }

    #[test]
    fn catmull_rom_corner_bulges_away_from_sharp_turn() {
        let waypoints = vec![
            Pos2::new(0.0, 100.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
        ];
        let samples = sample_catmull_rom_path(&waypoints, 16);
        let deviates_from_orthogonal = samples.iter().any(|p| p.x < -1.0 || p.y < -1.0);
        assert!(
            deviates_from_orthogonal,
            "expected smooth curve to deviate from orthogonal legs, samples: {:?}",
            samples
        );
    }
}
