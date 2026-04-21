//! Gantt chart diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};
use std::collections::HashMap;

/// A task in a Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub name: String,
    pub section: Option<String>,
    pub start_day: i32,
    pub duration: i32,
    pub is_milestone: bool,
    pub is_done: bool,
    pub is_active: bool,
    pub is_crit: bool,
}

/// A section in a Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttSection {
    pub name: String,
}

/// A Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttChart {
    pub title: Option<String>,
    pub tasks: Vec<GanttTask>,
    pub sections: Vec<GanttSection>,
}

/// Parse a Gantt chart from source.
pub fn parse_gantt_chart(source: &str) -> Result<GanttChart, String> {
    let mut title: Option<String> = None;
    let mut tasks: Vec<GanttTask> = Vec::new();
    let mut sections: Vec<GanttSection> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut task_map: HashMap<String, i32> = HashMap::new(); // task_id -> end_day
    let mut current_day = 0;
    let mut task_counter = 0;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.to_lowercase().starts_with("title") {
            title = Some(line[5..].trim().to_string());
            continue;
        }

        // Skip dateFormat and other directives
        if line.to_lowercase().starts_with("dateformat")
            || line.to_lowercase().starts_with("excludes")
            || line.to_lowercase().starts_with("todaymarker")
            || line.to_lowercase().starts_with("axisformat")
        {
            continue;
        }

        // Parse section
        if line.to_lowercase().starts_with("section") {
            let name = line[7..].trim().to_string();
            sections.push(GanttSection { name: name.clone() });
            current_section = Some(name);
            continue;
        }

        // Parse task: name :id, start, duration or name :id, after id2, duration
        if line.contains(':') {
            task_counter += 1;
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() < 2 {
                continue;
            }

            let name = parts[0].trim().to_string();
            let spec = parts[1].trim();
            let spec_parts: Vec<&str> = spec.split(',').map(|s| s.trim()).collect();

            let mut id = format!("t{}", task_counter);
            let mut start_day = current_day;
            let mut duration = 1;
            let mut is_milestone = false;
            let mut is_done = false;
            let mut is_active = false;
            let mut is_crit = false;

            for (i, part) in spec_parts.iter().enumerate() {
                let part_lower = part.to_lowercase();

                if part_lower == "done" {
                    is_done = true;
                } else if part_lower == "active" {
                    is_active = true;
                } else if part_lower == "crit" {
                    is_crit = true;
                } else if part_lower == "milestone" {
                    is_milestone = true;
                    duration = 0;
                } else if part_lower.starts_with("after") {
                    // after task_id
                    let after_id = part[5..].trim();
                    if let Some(&end_day) = task_map.get(after_id) {
                        start_day = end_day;
                    }
                } else if part.ends_with('d') {
                    // Duration like "7d"
                    if let Ok(d) = part[..part.len() - 1].parse::<i32>() {
                        duration = d;
                    }
                } else if i == 0 && !part.contains(' ') {
                    // First part might be ID
                    id = part.to_string();
                } else if let Ok(d) = part.parse::<i32>() {
                    // Plain number as duration
                    duration = d;
                }
            }

            let task = GanttTask {
                id: id.clone(),
                name,
                section: current_section.clone(),
                start_day,
                duration,
                is_milestone,
                is_done,
                is_active,
                is_crit,
            };

            task_map.insert(id, start_day + duration);
            current_day = start_day + duration;
            tasks.push(task);
        }
    }

    if tasks.is_empty() {
        return Err("No tasks found in Gantt chart".to_string());
    }

    Ok(GanttChart {
        title,
        tasks,
        sections,
    })
}

/// Render a Gantt chart to the UI.
pub fn render_gantt_chart(ui: &mut Ui, chart: &GanttChart, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let row_height = 28.0_f32;
    let row_spacing = 6.0_f32;
    let label_width = 150.0_f32;
    let day_width = 20.0_f32;
    let header_height = 30.0_f32;

    // Find total duration
    let max_day = chart
        .tasks
        .iter()
        .map(|t| t.start_day + t.duration)
        .max()
        .unwrap_or(10);

    let total_width = margin * 2.0 + label_width + (max_day as f32 + 2.0) * day_width;
    let total_height =
        margin * 2.0 + header_height + chart.tasks.len() as f32 * (row_height + row_spacing);

    // Colors
    let (
        bg_color,
        grid_color,
        text_color,
        task_done,
        task_active,
        task_normal,
        task_crit,
        milestone_color,
    ) = if dark_mode {
        (
            Color32::from_rgb(35, 40, 50),
            Color32::from_rgb(60, 65, 75),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(80, 140, 80),
            Color32::from_rgb(100, 150, 200),
            Color32::from_rgb(80, 100, 140),
            Color32::from_rgb(200, 80, 80),
            Color32::from_rgb(220, 180, 60),
        )
    } else {
        (
            Color32::from_rgb(250, 252, 255),
            Color32::from_rgb(220, 225, 235),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(100, 180, 100),
            Color32::from_rgb(100, 160, 220),
            Color32::from_rgb(140, 160, 200),
            Color32::from_rgb(220, 100, 100),
            Color32::from_rgb(240, 200, 80),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    let mut start_y = margin;
    if let Some(ref title) = chart.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin / 2.0),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
        start_y += 10.0;
    }

    // Draw grid background
    let grid_rect = Rect::from_min_size(
        Pos2::new(offset.x + margin + label_width, offset.y + start_y),
        Vec2::new(
            (max_day as f32 + 1.0) * day_width,
            total_height - margin - start_y,
        ),
    );
    painter.rect_filled(grid_rect, 0.0, bg_color);

    // Draw vertical grid lines (days)
    for day in 0..=max_day {
        let x = offset.x + margin + label_width + day as f32 * day_width;
        painter.line_segment(
            [
                Pos2::new(x, offset.y + start_y),
                Pos2::new(x, offset.y + total_height - margin),
            ],
            Stroke::new(1.0, grid_color),
        );

        // Day labels (every 5 days or if small chart)
        if day % 5 == 0 || max_day <= 10 {
            painter.text(
                Pos2::new(
                    x + day_width / 2.0,
                    offset.y + start_y + header_height / 2.0,
                ),
                egui::Align2::CENTER_CENTER,
                format!("{}", day),
                FontId::proportional(font_size - 3.0),
                text_color.gamma_multiply(0.6),
            );
        }
    }

    // Draw tasks
    let mut y = start_y + header_height;
    let mut current_section: Option<String> = None;

    for task in &chart.tasks {
        // Draw section header if changed
        if task.section != current_section {
            if let Some(ref section) = task.section {
                painter.text(
                    Pos2::new(offset.x + margin, offset.y + y + row_height / 2.0),
                    egui::Align2::LEFT_CENTER,
                    section,
                    FontId::proportional(font_size - 1.0),
                    text_color.gamma_multiply(0.7),
                );
            }
            current_section = task.section.clone();
        }

        // Draw horizontal grid line
        painter.line_segment(
            [
                Pos2::new(
                    offset.x + margin + label_width,
                    offset.y + y + row_height + row_spacing / 2.0,
                ),
                Pos2::new(
                    offset.x + margin + label_width + (max_day as f32 + 1.0) * day_width,
                    offset.y + y + row_height + row_spacing / 2.0,
                ),
            ],
            Stroke::new(0.5, grid_color),
        );

        // Draw task label
        painter.text(
            Pos2::new(
                offset.x + margin + label_width - 8.0,
                offset.y + y + row_height / 2.0,
            ),
            egui::Align2::RIGHT_CENTER,
            &task.name,
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        // Draw task bar or milestone
        let task_x = offset.x + margin + label_width + task.start_day as f32 * day_width;
        let task_y = offset.y + y + 4.0;

        if task.is_milestone {
            // Diamond for milestone
            let center = Pos2::new(task_x, task_y + row_height / 2.0 - 4.0);
            let size = 8.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + Vec2::new(0.0, -size),
                    center + Vec2::new(size, 0.0),
                    center + Vec2::new(0.0, size),
                    center + Vec2::new(-size, 0.0),
                ],
                milestone_color,
                Stroke::NONE,
            ));
        } else {
            // Bar for task
            let bar_width = task.duration as f32 * day_width;
            let bar_height = row_height - 8.0;
            let bar_rect = Rect::from_min_size(
                Pos2::new(task_x, task_y),
                Vec2::new(bar_width.max(4.0), bar_height),
            );

            let bar_color = if task.is_crit {
                task_crit
            } else if task.is_done {
                task_done
            } else if task.is_active {
                task_active
            } else {
                task_normal
            };

            painter.rect_filled(bar_rect, 3.0, bar_color);

            // Progress indicator for done tasks
            if task.is_done {
                painter.text(
                    bar_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "✓",
                    FontId::proportional(font_size - 3.0),
                    Color32::WHITE,
                );
            }
        }

        y += row_height + row_spacing;
    }
}
