//! Node shape rendering for flowchart diagrams.

use egui::{FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use super::super::types::{FlowNode, NodeLayout, NodeShape, NodeStyle};
use super::colors::FlowchartColors;

/// Draw a single flowchart node with its shape and label.
pub(crate) fn draw_node(
    painter: &egui::Painter,
    node: &FlowNode,
    layout: &NodeLayout,
    offset: Vec2,
    colors: &FlowchartColors,
    font_size: f32,
    custom_style: Option<&NodeStyle>,
) {
    let rect = Rect::from_min_size(layout.pos + offset, layout.size);
    let center = rect.center();

    // Determine colors and stroke, using custom style if available
    let fill_color = custom_style
        .and_then(|s| s.fill)
        .unwrap_or(colors.node_fill);
    let stroke_color = custom_style
        .and_then(|s| s.stroke)
        .unwrap_or(colors.node_stroke);
    let stroke_width = custom_style.and_then(|s| s.stroke_width).unwrap_or(2.0);
    let stroke = Stroke::new(stroke_width, stroke_color);

    // For diamond and circle, also check custom fill
    let diamond_fill = custom_style
        .and_then(|s| s.fill)
        .unwrap_or(colors.diamond_fill);
    let circle_fill = custom_style
        .and_then(|s| s.fill)
        .unwrap_or(colors.circle_fill);

    match node.shape {
        NodeShape::Rectangle | NodeShape::Subroutine => {
            painter.rect(rect, Rounding::same(4.0), fill_color, stroke);
            if matches!(node.shape, NodeShape::Subroutine) {
                // Draw double vertical lines
                let inset = 8.0;
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + inset, rect.top()),
                        Pos2::new(rect.left() + inset, rect.bottom()),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.right() - inset, rect.top()),
                        Pos2::new(rect.right() - inset, rect.bottom()),
                    ],
                    stroke,
                );
            }
        }
        NodeShape::RoundRect | NodeShape::Stadium => {
            let rounding = if matches!(node.shape, NodeShape::Stadium) {
                Rounding::same(layout.size.y / 2.0)
            } else {
                Rounding::same(12.0)
            };
            painter.rect(rect, rounding, fill_color, stroke);
        }
        NodeShape::Diamond => {
            let points = [
                Pos2::new(center.x, rect.top()),
                Pos2::new(rect.right(), center.y),
                Pos2::new(center.x, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                diamond_fill,
                stroke,
            ));
        }
        NodeShape::Circle => {
            let radius = layout.size.x.min(layout.size.y) / 2.0;
            painter.circle(center, radius, circle_fill, stroke);
        }
        NodeShape::Hexagon => {
            let inset = layout.size.x * 0.15;
            let points = [
                Pos2::new(rect.left() + inset, rect.top()),
                Pos2::new(rect.right() - inset, rect.top()),
                Pos2::new(rect.right(), center.y),
                Pos2::new(rect.right() - inset, rect.bottom()),
                Pos2::new(rect.left() + inset, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                fill_color,
                stroke,
            ));
        }
        NodeShape::Cylinder => {
            // Simplified cylinder as rounded rect with ellipse hints
            painter.rect(rect, Rounding::same(4.0), fill_color, stroke);
            let ellipse_height = 8.0;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), rect.top() + ellipse_height),
                    Pos2::new(rect.right(), rect.top() + ellipse_height),
                ],
                Stroke::new(1.0, stroke_color.gamma_multiply(0.5)),
            );
        }
        NodeShape::Parallelogram => {
            let skew = layout.size.x * 0.15;
            let points = [
                Pos2::new(rect.left() + skew, rect.top()),
                Pos2::new(rect.right(), rect.top()),
                Pos2::new(rect.right() - skew, rect.bottom()),
                Pos2::new(rect.left(), rect.bottom()),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                fill_color,
                stroke,
            ));
        }
        NodeShape::Asymmetric => {
            // Asymmetric shape: flag/banner pointing left
            // Notch depth is proportional to height for consistent appearance
            let indent = layout.size.y * 0.25;
            let points = [
                Pos2::new(rect.left() + indent, rect.top()),
                Pos2::new(rect.right(), rect.top()),
                Pos2::new(rect.right(), rect.bottom()),
                Pos2::new(rect.left() + indent, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                fill_color,
                stroke,
            ));
        }
    }

    // Draw text - offset for asymmetric shape to center in visible area
    let text_center = if matches!(node.shape, NodeShape::Asymmetric) {
        // Offset text to the right by half the indent to center within visible portion
        let indent = layout.size.y * 0.25;
        Pos2::new(center.x + indent / 2.0, center.y)
    } else {
        center
    };

    painter.text(
        text_center,
        egui::Align2::CENTER_CENTER,
        &node.label,
        FontId::proportional(font_size),
        colors.node_text,
    );
}
