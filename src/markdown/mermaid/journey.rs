//! User journey diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};

/// A task in a user journey.
#[derive(Debug, Clone)]
pub struct JourneyTask {
    pub name: String,
    pub score: i32, // 1-5 satisfaction score
    pub actors: Vec<String>,
}

/// A section in a user journey.
#[derive(Debug, Clone)]
pub struct JourneySection {
    pub name: String,
    pub tasks: Vec<JourneyTask>,
}

/// A user journey diagram.
#[derive(Debug, Clone)]
pub struct UserJourney {
    pub title: Option<String>,
    pub sections: Vec<JourneySection>,
}

/// Parse a user journey from source.
pub fn parse_user_journey(source: &str) -> Result<UserJourney, String> {
    let mut title: Option<String> = None;
    let mut sections: Vec<JourneySection> = Vec::new();
    let mut current_section: Option<JourneySection> = None;

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

        // Parse section
        if line.to_lowercase().starts_with("section") {
            // Save previous section
            if let Some(s) = current_section.take() {
                sections.push(s);
            }
            let name = line[7..].trim().to_string();
            current_section = Some(JourneySection {
                name,
                tasks: Vec::new(),
            });
            continue;
        }

        // Parse task: "Task name: score: Actor1, Actor2"
        if line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim().to_string();
                let score = parts[1].trim().parse::<i32>().unwrap_or(3);
                let actors: Vec<String> = if parts.len() > 2 {
                    parts[2].split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    Vec::new()
                };

                let task = JourneyTask {
                    name,
                    score,
                    actors,
                };

                if let Some(ref mut s) = current_section {
                    s.tasks.push(task);
                } else {
                    // Create default section if none exists
                    current_section = Some(JourneySection {
                        name: "Journey".to_string(),
                        tasks: vec![task],
                    });
                }
            }
        }
    }

    // Save last section
    if let Some(s) = current_section {
        sections.push(s);
    }

    if sections.is_empty() {
        return Err("No sections found in user journey".to_string());
    }

    Ok(UserJourney { title, sections })
}

/// Render a user journey to the UI.
pub fn render_user_journey(ui: &mut Ui, journey: &UserJourney, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let task_width = 140.0_f32;
    let task_spacing = 15.0_f32;
    let section_spacing = 30.0_f32;
    let row_height = 80.0_f32;
    let header_height = 50.0_f32;

    // Count total tasks
    let total_tasks: usize = journey.sections.iter().map(|s| s.tasks.len()).sum();

    let total_width = margin * 2.0
        + total_tasks as f32 * (task_width + task_spacing)
        + journey.sections.len() as f32 * section_spacing;
    let total_height = margin * 2.0 + header_height + row_height + 60.0;

    // Colors - score based (1=red, 5=green)
    let score_colors = if dark_mode {
        vec![
            Color32::from_rgb(200, 80, 80),  // 1 - Bad
            Color32::from_rgb(200, 140, 80), // 2 - Poor
            Color32::from_rgb(200, 200, 80), // 3 - Neutral
            Color32::from_rgb(140, 200, 80), // 4 - Good
            Color32::from_rgb(80, 200, 120), // 5 - Great
        ]
    } else {
        vec![
            Color32::from_rgb(220, 100, 100),
            Color32::from_rgb(220, 160, 100),
            Color32::from_rgb(220, 220, 100),
            Color32::from_rgb(160, 200, 100),
            Color32::from_rgb(100, 180, 120),
        ]
    };
    let text_color = if dark_mode {
        Color32::from_rgb(220, 230, 240)
    } else {
        Color32::from_rgb(30, 40, 50)
    };
    let line_color = if dark_mode {
        Color32::from_rgb(80, 90, 100)
    } else {
        Color32::from_rgb(180, 190, 200)
    };
    let section_color = if dark_mode {
        Color32::from_rgb(100, 140, 180)
    } else {
        Color32::from_rgb(80, 120, 160)
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(150.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    if let Some(ref title) = journey.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
    }

    // Draw journey path
    let path_y = offset.y + header_height + row_height / 2.0 + 10.0;
    let mut x = offset.x + margin + task_width / 2.0;
    let mut prev_x: Option<f32> = None;
    let mut prev_score_y: Option<f32> = None;

    for (section_idx, section) in journey.sections.iter().enumerate() {
        // Draw section label
        let section_start_x = x;

        for task in &section.tasks {
            let score_idx = (task.score.clamp(1, 5) - 1) as usize;
            let color = score_colors[score_idx];

            // Score affects Y position (higher score = higher position)
            let score_offset = (3 - task.score) as f32 * 10.0;
            let task_y = path_y + score_offset;

            // Draw connection line from previous task
            if let (Some(px), Some(py)) = (prev_x, prev_score_y) {
                painter.line_segment(
                    [Pos2::new(px, py), Pos2::new(x, task_y)],
                    Stroke::new(2.0, line_color),
                );
            }

            // Draw task card
            let card_rect =
                Rect::from_center_size(Pos2::new(x, task_y), Vec2::new(task_width - 10.0, 50.0));
            painter.rect_filled(card_rect, 6.0, color.gamma_multiply(0.3));
            painter.rect_stroke(card_rect, 6.0, Stroke::new(2.0, color));

            // Draw task name
            painter.text(
                Pos2::new(x, task_y - 8.0),
                egui::Align2::CENTER_CENTER,
                &task.name,
                FontId::proportional(font_size - 2.0),
                text_color,
            );

            // Draw score indicator (filled circle - size reflects score)
            let indicator_radius = 4.0 + task.score as f32 * 1.0; // 5-9 radius based on score
            painter.circle_filled(Pos2::new(x, task_y + 12.0), indicator_radius, color);

            // Draw actors below
            if !task.actors.is_empty() {
                let actors_text = task.actors.join(", ");
                painter.text(
                    Pos2::new(x, card_rect.max.y + 10.0),
                    egui::Align2::CENTER_TOP,
                    &actors_text,
                    FontId::proportional(font_size - 3.0),
                    text_color.gamma_multiply(0.6),
                );
            }

            prev_x = Some(x);
            prev_score_y = Some(task_y);
            x += task_width + task_spacing;
        }

        // Draw section label above the section
        let section_end_x = x - task_spacing;
        let section_mid_x = (section_start_x + section_end_x) / 2.0;
        painter.text(
            Pos2::new(section_mid_x, offset.y + header_height - 5.0),
            egui::Align2::CENTER_BOTTOM,
            &section.name,
            FontId::proportional(font_size - 1.0),
            section_color,
        );

        // Add section spacing
        if section_idx < journey.sections.len() - 1 {
            x += section_spacing;
        }
    }
}
