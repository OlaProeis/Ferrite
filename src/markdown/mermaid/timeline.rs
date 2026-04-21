//! Timeline diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};

/// A period in a timeline.
#[derive(Debug, Clone)]
pub struct TimelinePeriod {
    pub label: String,
    pub events: Vec<String>,
}

/// A timeline diagram.
#[derive(Debug, Clone)]
pub struct Timeline {
    pub title: Option<String>,
    pub periods: Vec<TimelinePeriod>,
}

/// Parse a timeline from source.
pub fn parse_timeline(source: &str) -> Result<Timeline, String> {
    let mut title: Option<String> = None;
    let mut periods: Vec<TimelinePeriod> = Vec::new();
    let mut current_period: Option<TimelinePeriod> = None;

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

        // Check if this is a period label (doesn't start with whitespace in original)
        // or a section marker
        if line.to_lowercase().starts_with("section") {
            // Save previous period
            if let Some(p) = current_period.take() {
                if !p.events.is_empty() || periods.is_empty() {
                    periods.push(p);
                }
            }
            let label = line[7..].trim().to_string();
            current_period = Some(TimelinePeriod {
                label,
                events: Vec::new(),
            });
            continue;
        }

        // Check if line starts with a date/period pattern or is indented event
        if line.contains(':') {
            // Period with events: "2024-Q1 : Event 1 : Event 2"
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                // Save previous period
                if let Some(p) = current_period.take() {
                    periods.push(p);
                }

                let label = parts[0].trim().to_string();
                let events: Vec<String> = parts[1..]
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                periods.push(TimelinePeriod { label, events });
            }
        } else if current_period.is_some() {
            // Event for current period
            if let Some(ref mut p) = current_period {
                p.events.push(line.to_string());
            }
        } else {
            // New period without colon
            if let Some(p) = current_period.take() {
                periods.push(p);
            }
            current_period = Some(TimelinePeriod {
                label: line.to_string(),
                events: Vec::new(),
            });
        }
    }

    // Save last period
    if let Some(p) = current_period {
        periods.push(p);
    }

    if periods.is_empty() {
        return Err("No periods found in timeline".to_string());
    }

    Ok(Timeline { title, periods })
}

/// Render a timeline to the UI.
pub fn render_timeline(ui: &mut Ui, timeline: &Timeline, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let period_width = 160.0_f32;
    let period_spacing = 20.0_f32;
    let event_height = font_size + 6.0;
    let timeline_y = 80.0_f32;

    // Calculate max events per period for height
    let max_events = timeline
        .periods
        .iter()
        .map(|p| p.events.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let total_width =
        margin * 2.0 + timeline.periods.len() as f32 * (period_width + period_spacing);
    let total_height = margin * 2.0 + timeline_y + max_events as f32 * (event_height + 8.0) + 40.0;

    // Colors
    let (line_color, text_color, period_colors) = if dark_mode {
        (
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(220, 230, 240),
            vec![
                Color32::from_rgb(80, 140, 200),
                Color32::from_rgb(100, 180, 140),
                Color32::from_rgb(200, 160, 100),
                Color32::from_rgb(180, 120, 160),
                Color32::from_rgb(140, 160, 200),
            ],
        )
    } else {
        (
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(30, 40, 50),
            vec![
                Color32::from_rgb(70, 130, 180),
                Color32::from_rgb(80, 160, 120),
                Color32::from_rgb(180, 140, 80),
                Color32::from_rgb(160, 100, 140),
                Color32::from_rgb(120, 140, 180),
            ],
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(150.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    if let Some(ref title) = timeline.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
    }

    // Draw timeline line
    let line_y = offset.y + timeline_y;
    let line_start = offset.x + margin + period_width / 2.0;
    let line_end = offset.x
        + margin
        + (timeline.periods.len() - 1) as f32 * (period_width + period_spacing)
        + period_width / 2.0;
    painter.line_segment(
        [Pos2::new(line_start, line_y), Pos2::new(line_end, line_y)],
        Stroke::new(3.0, line_color),
    );

    // Draw periods and events
    for (i, period) in timeline.periods.iter().enumerate() {
        let x = offset.x + margin + i as f32 * (period_width + period_spacing) + period_width / 2.0;
        let color = period_colors[i % period_colors.len()];

        // Draw period marker (circle on timeline)
        painter.circle_filled(Pos2::new(x, line_y), 8.0, color);
        painter.circle_stroke(
            Pos2::new(x, line_y),
            8.0,
            Stroke::new(
                2.0,
                if dark_mode {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                },
            ),
        );

        // Draw period label above
        painter.text(
            Pos2::new(x, line_y - 20.0),
            egui::Align2::CENTER_BOTTOM,
            &period.label,
            FontId::proportional(font_size),
            color,
        );

        // Draw events below
        let mut event_y = line_y + 25.0;
        for event in &period.events {
            let event_rect = Rect::from_center_size(
                Pos2::new(x, event_y + event_height / 2.0),
                Vec2::new(period_width - 10.0, event_height),
            );
            painter.rect_filled(event_rect, 4.0, color.gamma_multiply(0.2));
            painter.rect_stroke(event_rect, 4.0, Stroke::new(1.0, color.gamma_multiply(0.5)));
            painter.text(
                event_rect.center(),
                egui::Align2::CENTER_CENTER,
                event,
                FontId::proportional(font_size - 2.0),
                text_color,
            );
            event_y += event_height + 8.0;
        }
    }
}
