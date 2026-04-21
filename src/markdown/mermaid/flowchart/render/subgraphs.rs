//! Subgraph rendering for flowchart diagrams.

use std::collections::HashMap;

use egui::{FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use super::super::types::{Flowchart, SubgraphLayout};
use super::colors::FlowchartColors;

/// Draw a subgraph background and title.
pub(crate) fn draw_subgraph(
    painter: &egui::Painter,
    layout: &SubgraphLayout,
    offset: Vec2,
    colors: &FlowchartColors,
    font_size: f32,
    depth: usize,
) {
    let rect = Rect::from_min_size(layout.pos + offset, layout.size);

    // Use alternating fill colors for nested subgraphs
    let fill_color = if depth % 2 == 0 {
        colors.subgraph_fill
    } else {
        colors.subgraph_fill_alt
    };

    // Draw visible background with rounded corners and thicker stroke
    painter.rect(
        rect,
        Rounding::same(8.0),
        fill_color,
        Stroke::new(2.0, colors.subgraph_stroke),
    );

    // Draw prominent title if present
    if let Some(title) = &layout.title {
        let title_pos = Pos2::new(rect.left() + 12.0, rect.top() + 8.0);
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            title,
            FontId::proportional(font_size),
            colors.subgraph_title,
        );
    }
}

/// Compute nesting depth for each subgraph.
/// Depth 0 = top-level subgraph, depth 1 = child of top-level, etc.
pub(crate) fn compute_subgraph_depths(flowchart: &Flowchart) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();

    // Build parent mapping: child_id -> parent_id
    let mut parent_map: HashMap<String, String> = HashMap::new();
    for subgraph in &flowchart.subgraphs {
        for child_id in &subgraph.child_subgraph_ids {
            parent_map.insert(child_id.clone(), subgraph.id.clone());
        }
    }

    // Compute depth for each subgraph by counting ancestors
    for subgraph in &flowchart.subgraphs {
        let mut depth = 0;
        let mut current_id = subgraph.id.clone();

        // Walk up the parent chain
        while let Some(parent_id) = parent_map.get(&current_id) {
            depth += 1;
            current_id = parent_id.clone();
        }

        depths.insert(subgraph.id.clone(), depth);
    }

    depths
}
