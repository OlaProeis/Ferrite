//! Pie chart diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Ui, Vec2};

/// A slice of a pie chart.
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}

/// A parsed pie chart.
#[derive(Debug, Clone, Default)]
pub struct PieChart {
    pub title: Option<String>,
    pub slices: Vec<PieSlice>,
}

/// Parse mermaid pie chart source.
pub fn parse_pie_chart(source: &str) -> Result<PieChart, String> {
    let mut chart = PieChart::default();

    for line in source.lines().skip(1) {
        let line = line.trim();

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.starts_with("title ") {
            chart.title = Some(line[6..].trim().to_string());
            continue;
        }

        // Parse slice: "Label" : value
        if let Some(colon_pos) = line.find(':') {
            let label = line[..colon_pos].trim().trim_matches('"').to_string();
            let value_str = line[colon_pos + 1..].trim();
            if let Ok(value) = value_str.parse::<f32>() {
                chart.slices.push(PieSlice { label, value });
            }
        }
    }

    if chart.slices.is_empty() {
        return Err("No data found in pie chart".to_string());
    }

    Ok(chart)
}

/// Render a pie chart to the UI.
pub fn render_pie_chart(ui: &mut Ui, chart: &PieChart, dark_mode: bool, font_size: f32) {
    use std::f32::consts::PI;

    let margin = 20.0_f32;
    let pie_radius = 80.0_f32;
    let legend_width = 120.0_f32;

    let total_width = margin * 3.0 + pie_radius * 2.0 + legend_width;
    let total_height =
        margin * 2.0 + pie_radius * 2.0 + if chart.title.is_some() { 30.0 } else { 0.0 };

    let text_color = if dark_mode {
        Color32::from_rgb(220, 230, 240)
    } else {
        Color32::from_rgb(30, 40, 50)
    };

    // Pie colors
    let colors = [
        Color32::from_rgb(66, 133, 244), // Blue
        Color32::from_rgb(234, 67, 53),  // Red
        Color32::from_rgb(251, 188, 4),  // Yellow
        Color32::from_rgb(52, 168, 83),  // Green
        Color32::from_rgb(155, 89, 182), // Purple
        Color32::from_rgb(230, 126, 34), // Orange
        Color32::from_rgb(26, 188, 156), // Teal
        Color32::from_rgb(241, 196, 15), // Gold
    ];

    let (response, painter) =
        ui.allocate_painter(Vec2::new(total_width, total_height), egui::Sense::hover());
    let offset = response.rect.min.to_vec2();

    // Draw title
    let mut y_offset = margin;
    if let Some(title) = &chart.title {
        painter.text(
            Pos2::new(total_width / 2.0, margin / 2.0) + offset,
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
        y_offset += 20.0;
    }

    // Calculate total and draw pie
    let total: f32 = chart.slices.iter().map(|s| s.value).sum();
    if total <= 0.0 {
        return;
    }

    let center = Pos2::new(margin + pie_radius, y_offset + pie_radius) + offset;
    let mut start_angle = -PI / 2.0; // Start from top
    let border_color = if dark_mode {
        Color32::from_rgb(25, 30, 40)
    } else {
        Color32::WHITE
    };

    // Draw each slice as a filled path
    for (i, slice) in chart.slices.iter().enumerate() {
        let sweep_angle = (slice.value / total) * 2.0 * PI;
        let color = colors[i % colors.len()];
        let end_angle = start_angle + sweep_angle;

        // Build path for this slice
        let mut path = vec![center];

        // Add arc points - use enough segments for smooth curve
        let arc_segments = ((sweep_angle / (2.0 * PI)) * 100.0).max(8.0) as usize;
        for j in 0..=arc_segments {
            let t = j as f32 / arc_segments as f32;
            let angle = start_angle + sweep_angle * t;
            path.push(center + Vec2::new(angle.cos(), angle.sin()) * pie_radius);
        }

        // Draw the slice as a filled mesh (handles non-convex shapes)
        // We'll use triangles from center to each pair of adjacent arc points
        for j in 0..path.len() - 2 {
            let p0 = path[0]; // center
            let p1 = path[j + 1];
            let p2 = path[j + 2];

            // Create a mesh for this triangle
            let mesh = egui::Mesh {
                indices: vec![0, 1, 2],
                vertices: vec![
                    egui::epaint::Vertex {
                        pos: p0,
                        uv: egui::epaint::WHITE_UV,
                        color,
                    },
                    egui::epaint::Vertex {
                        pos: p1,
                        uv: egui::epaint::WHITE_UV,
                        color,
                    },
                    egui::epaint::Vertex {
                        pos: p2,
                        uv: egui::epaint::WHITE_UV,
                        color,
                    },
                ],
                texture_id: egui::TextureId::default(),
            };
            painter.add(egui::Shape::mesh(mesh));
        }

        // Draw slice border lines
        let start_edge = center + Vec2::new(start_angle.cos(), start_angle.sin()) * pie_radius;
        let end_edge = center + Vec2::new(end_angle.cos(), end_angle.sin()) * pie_radius;
        painter.line_segment([center, start_edge], egui::Stroke::new(1.5, border_color));
        painter.line_segment([center, end_edge], egui::Stroke::new(1.5, border_color));

        start_angle = end_angle;
    }

    // Draw outer circle border
    painter.circle_stroke(center, pie_radius, egui::Stroke::new(1.5, border_color));

    // Draw legend
    let legend_x = margin * 2.0 + pie_radius * 2.0 + offset.x;
    let mut legend_y = y_offset + 10.0 + offset.y;

    for (i, slice) in chart.slices.iter().enumerate() {
        let color = colors[i % colors.len()];
        let percentage = (slice.value / total * 100.0).round();

        // Color box
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(legend_x, legend_y), Vec2::new(12.0, 12.0)),
            2.0,
            color,
        );

        // Label
        painter.text(
            Pos2::new(legend_x + 18.0, legend_y + 6.0),
            egui::Align2::LEFT_CENTER,
            format!("{} ({}%)", slice.label, percentage),
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        legend_y += 20.0;
    }
}
