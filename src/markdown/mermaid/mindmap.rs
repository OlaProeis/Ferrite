//! Mindmap diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

use super::text::{EguiTextMeasurer, TextMeasurer};

/// A node in a mindmap.
#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub text: String,
    pub level: usize,
    pub children: Vec<MindmapNode>,
}

/// A parsed mindmap.
#[derive(Debug, Clone)]
pub struct Mindmap {
    pub root: Option<MindmapNode>,
}

/// Parse mermaid mindmap source.
pub fn parse_mindmap(source: &str) -> Result<Mindmap, String> {
    let mut root: Option<MindmapNode> = None;
    let mut stack: Vec<(usize, MindmapNode)> = Vec::new();

    for line in source.lines().skip(1) {
        if line.trim().is_empty() || line.trim().starts_with("%%") {
            continue;
        }

        // Count indentation (spaces)
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let level = indent / 4; // Assume 4 spaces per level
        let text = line.trim();

        // Handle root node with (( )) or just text
        let text = if text.starts_with("root") {
            let inner = text.strip_prefix("root").unwrap_or(text).trim();
            if inner.starts_with("((") && inner.ends_with("))") {
                inner[2..inner.len() - 2].to_string()
            } else if inner.starts_with('(') && inner.ends_with(')') {
                inner[1..inner.len() - 1].to_string()
            } else {
                inner.to_string()
            }
        } else {
            text.to_string()
        };

        if text.is_empty() {
            continue;
        }

        let node = MindmapNode {
            text,
            level,
            children: Vec::new(),
        };

        // Find parent
        while let Some((parent_level, _)) = stack.last() {
            if *parent_level >= level {
                let (_, finished_node) = stack.pop().unwrap();
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push(finished_node);
                } else {
                    root = Some(finished_node);
                }
            } else {
                break;
            }
        }

        stack.push((level, node));
    }

    // Pop remaining nodes
    while let Some((_, finished_node)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(finished_node);
        } else {
            root = Some(finished_node);
        }
    }

    if root.is_none() {
        return Err("No root node found in mindmap".to_string());
    }

    Ok(Mindmap { root })
}

/// Layout info for a mindmap node.
#[derive(Debug, Clone)]
struct MindmapLayout {
    center: Pos2,
    width: f32,
    children: Vec<MindmapLayout>,
}

/// Render a mindmap to the UI.
pub fn render_mindmap(ui: &mut Ui, mindmap: &Mindmap, dark_mode: bool, font_size: f32) {
    let root = match &mindmap.root {
        Some(r) => r,
        None => return,
    };

    let margin = 30.0_f32;
    let node_height = 28.0_f32;
    let level_width = 160.0_f32;
    let vertical_spacing = 12.0_f32;
    let node_padding = 24.0_f32;
    let min_node_width = 60.0_f32;
    let max_node_width = 180.0_f32;

    // Pre-measure all node text widths
    fn collect_texts(node: &MindmapNode, texts: &mut Vec<String>) {
        texts.push(node.text.clone());
        for child in &node.children {
            collect_texts(child, texts);
        }
    }
    let mut all_texts = Vec::new();
    collect_texts(root, &mut all_texts);

    let text_widths: HashMap<String, f32> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        all_texts
            .into_iter()
            .map(|text| {
                let size = text_measurer.measure(&text, font_size);
                let width = (size.width * 1.15 + node_padding)
                    .max(min_node_width)
                    .min(max_node_width);
                (text, width)
            })
            .collect()
    };

    // First pass: calculate layout WITHOUT drawing
    fn calc_layout(
        node: &MindmapNode,
        x: f32,
        y: &mut f32,
        node_height: f32,
        level_width: f32,
        vertical_spacing: f32,
        text_widths: &HashMap<String, f32>,
        min_node_width: f32,
    ) -> MindmapLayout {
        let node_width = text_widths
            .get(&node.text)
            .copied()
            .unwrap_or(min_node_width);

        // First, layout all children
        let mut children_layouts: Vec<MindmapLayout> = Vec::new();
        for child in &node.children {
            let child_layout = calc_layout(
                child,
                x + level_width,
                y,
                node_height,
                level_width,
                vertical_spacing,
                text_widths,
                min_node_width,
            );
            children_layouts.push(child_layout);
        }

        // Calculate this node's center Y
        let center_y = if children_layouts.is_empty() {
            let cy = *y + node_height / 2.0;
            *y += node_height + vertical_spacing;
            cy
        } else {
            // Center among children
            let first_y = children_layouts.first().map(|c| c.center.y).unwrap_or(*y);
            let last_y = children_layouts.last().map(|c| c.center.y).unwrap_or(*y);
            (first_y + last_y) / 2.0
        };

        MindmapLayout {
            center: Pos2::new(x + node_width / 2.0, center_y),
            width: node_width,
            children: children_layouts,
        }
    }

    // Calculate layout
    let mut y = margin;
    let layout = calc_layout(
        root,
        margin,
        &mut y,
        node_height,
        level_width,
        vertical_spacing,
        &text_widths,
        min_node_width,
    );

    // Calculate total size from layout
    fn calc_bounds(layout: &MindmapLayout, node_height: f32) -> (f32, f32) {
        let mut max_x = layout.center.x + layout.width / 2.0;
        let mut max_y = layout.center.y + node_height / 2.0;
        for child in &layout.children {
            let (cx, cy) = calc_bounds(child, node_height);
            max_x = max_x.max(cx);
            max_y = max_y.max(cy);
        }
        (max_x, max_y)
    }
    let (max_x, max_y) = calc_bounds(&layout, node_height);
    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let colors_by_level = if dark_mode {
        vec![
            Color32::from_rgb(100, 160, 220),
            Color32::from_rgb(120, 180, 140),
            Color32::from_rgb(200, 160, 100),
            Color32::from_rgb(180, 120, 160),
            Color32::from_rgb(140, 140, 180),
        ]
    } else {
        vec![
            Color32::from_rgb(50, 120, 180),
            Color32::from_rgb(60, 140, 80),
            Color32::from_rgb(180, 130, 50),
            Color32::from_rgb(150, 80, 130),
            Color32::from_rgb(100, 100, 150),
        ]
    };
    let text_color = if dark_mode {
        Color32::from_rgb(220, 230, 240)
    } else {
        Color32::from_rgb(30, 40, 50)
    };
    let bg_color = if dark_mode {
        Color32::from_rgb(40, 45, 55)
    } else {
        Color32::from_rgb(245, 248, 252)
    };

    // Allocate space based on calculated layout
    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Second pass: draw using pre-calculated layout
    fn draw_node(
        painter: &egui::Painter,
        node: &MindmapNode,
        layout: &MindmapLayout,
        offset: Vec2,
        level: usize,
        font_size: f32,
        node_height: f32,
        colors: &[Color32],
        text_color: Color32,
        bg_color: Color32,
    ) {
        let color = colors[level % colors.len()];
        let center = layout.center + offset;
        let rect = Rect::from_center_size(center, Vec2::new(layout.width, node_height));

        // Draw connections to children first (behind nodes)
        for (child_node, child_layout) in node.children.iter().zip(layout.children.iter()) {
            let child_center = child_layout.center + offset;
            let start = Pos2::new(rect.right(), center.y);
            let end = Pos2::new(child_center.x - child_layout.width / 2.0, child_center.y);
            painter.line_segment(
                [start, end],
                Stroke::new(1.5, colors[(level + 1) % colors.len()].gamma_multiply(0.6)),
            );

            // Recursively draw children
            draw_node(
                painter,
                child_node,
                child_layout,
                offset,
                level + 1,
                font_size,
                node_height,
                colors,
                text_color,
                bg_color,
            );
        }

        // Draw this node
        painter.rect(
            rect,
            Rounding::same(node_height / 2.0),
            bg_color,
            Stroke::new(2.0, color),
        );
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            &node.text,
            FontId::proportional(font_size - 1.0),
            text_color,
        );
    }

    draw_node(
        &painter,
        root,
        &layout,
        offset,
        0,
        font_size,
        node_height,
        &colors_by_level,
        text_color,
        bg_color,
    );
}
