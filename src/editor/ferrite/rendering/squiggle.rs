//! Diagnostic squiggle positioning (wrapped and unwrapped text).
//!
//! Uses the same wrapped [`egui::Galley`] row/column math as the caret
//! ([`super::cursor`]) and search highlights ([`crate::editor::ferrite::highlights`]).

use egui::{Color32, FontId, Pos2};

/// One horizontal squiggle segment in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SquiggleSegment {
    pub x_start: f32,
    pub x_end: f32,
    pub baseline_y: f32,
}

/// Compute squiggle segments for `col_start..col_end` (char indices) on one logical line.
#[allow(clippy::too_many_arguments)]
pub fn squiggle_segments_for_columns(
    painter: &egui::Painter,
    line: &str,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    line_height: f32,
    horizontal_scroll: f32,
    wrap_width: f32,
    wrap_active: bool,
    col_start: usize,
    col_end: usize,
) -> Vec<SquiggleSegment> {
    let line_len = line.chars().count();
    if line_len == 0 {
        return Vec::new();
    }
    let col_start = col_start.min(line_len);
    let col_end = col_end.min(line_len);
    if col_start >= col_end {
        return Vec::new();
    }

    if wrap_active && wrap_width > 0.0 {
        segments_wrapped(
            painter,
            line,
            font_id,
            text_start_x,
            line_top_y,
            wrap_width,
            col_start,
            col_end,
        )
    } else {
        segments_unwrapped(
            painter,
            line,
            font_id,
            text_start_x,
            line_top_y,
            line_height,
            horizontal_scroll,
            col_start,
            col_end,
        )
    }
}

fn segments_wrapped(
    painter: &egui::Painter,
    line: &str,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    wrap_width: f32,
    col_start: usize,
    col_end: usize,
) -> Vec<SquiggleSegment> {
    let galley = painter.layout(
        line.to_string(),
        font_id.clone(),
        Color32::WHITE,
        wrap_width,
    );

    let start_ccursor = egui::text::CCursor::new(col_start);
    let end_ccursor = egui::text::CCursor::new(col_end);
    let start_rcursor = galley.layout_from_cursor(start_ccursor);
    let end_rcursor = galley.layout_from_cursor(end_ccursor);

    let mut segments = Vec::new();
    for row_idx in start_rcursor.row..=end_rcursor.row {
        let Some(row) = galley.rows.get(row_idx) else {
            continue;
        };
        let row_rect = row.rect();

        let x_start = if row_idx == start_rcursor.row {
            row.x_offset(start_rcursor.column)
        } else {
            0.0
        };

        let x_end = if row_idx == end_rcursor.row {
            row.x_offset(end_rcursor.column)
        } else {
            row_rect.width()
        };

        if x_end <= x_start {
            continue;
        }

        let baseline_y = line_top_y + row_rect.max.y - 2.0;
        segments.push(SquiggleSegment {
            x_start: text_start_x + x_start,
            x_end: text_start_x + x_end,
            baseline_y,
        });
    }
    segments
}

#[allow(clippy::too_many_arguments)]
fn segments_unwrapped(
    painter: &egui::Painter,
    line: &str,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    line_height: f32,
    horizontal_scroll: f32,
    col_start: usize,
    col_end: usize,
) -> Vec<SquiggleSegment> {
    let line_chars: Vec<char> = line.chars().collect();

    let x_start = if col_start == 0 {
        text_start_x - horizontal_scroll
    } else {
        let prefix: String = line_chars.iter().take(col_start).collect();
        let g = painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
        text_start_x + g.size().x - horizontal_scroll
    };

    let x_end = {
        let prefix: String = line_chars.iter().take(col_end).collect();
        let g = painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
        text_start_x + g.size().x - horizontal_scroll
    };

    if x_end <= x_start {
        return Vec::new();
    }

    vec![SquiggleSegment {
        x_start,
        x_end,
        baseline_y: line_top_y + line_height - 2.0,
    }]
}

/// Draw a wavy underline between `x_start` and `x_end` at `baseline_y`.
pub fn paint_squiggle_wave(
    painter: &egui::Painter,
    x_start: f32,
    x_end: f32,
    baseline_y: f32,
    color: Color32,
) {
    if x_end <= x_start {
        return;
    }

    let wave_period = 4.0_f32;
    let amplitude = 1.5_f32;
    let stroke = egui::Stroke::new(1.2, color);
    let mut points = Vec::new();
    let mut x = x_start;
    while x <= x_end {
        let phase = (x - x_start) / wave_period * std::f32::consts::TAU;
        let dy = amplitude * phase.sin();
        points.push(Pos2::new(x, baseline_y + dy));
        x += 1.0;
    }
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::FontDefinitions;
    use std::sync::OnceLock;

    fn test_painter() -> (egui::Context, egui::Painter) {
        static CTX: OnceLock<egui::Context> = OnceLock::new();
        let ctx = CTX.get_or_init(|| {
            let ctx = egui::Context::default();
            ctx.set_fonts(FontDefinitions::default());
            ctx
        });

        let mut painter_out = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let (_id, painter) =
                ui.allocate_painter(egui::Vec2::new(400.0, 200.0), egui::Sense::hover());
            painter_out = Some(painter);
        });
        (ctx.clone(), painter_out.expect("painter"))
    }

    fn char_col(line: &str, needle: &str) -> (usize, usize) {
        let byte_start = line.find(needle).expect("needle");
        let col_start = line[..byte_start].chars().count();
        let col_end = col_start + needle.chars().count();
        (col_start, col_end)
    }

    #[test]
    fn squiggle_wrapped_word_on_second_row_uses_row_y_and_x() {
        let (_ctx, painter) = test_painter();
        let font_id = FontId::proportional(14.0);
        let line = "aaaa aaaa aaaa misspelled viewport";
        let wrap_width = 72.0;
        let text_start_x = 10.0;
        let line_top_y = 20.0;

        let (col_start, col_end) = char_col(line, "misspelled");

        let galley = painter.layout(
            line.to_string(),
            font_id.clone(),
            Color32::WHITE,
            wrap_width,
        );
        assert!(
            galley.rows.len() >= 2,
            "test line must wrap to at least 2 rows at wrap_width={wrap_width}"
        );

        let word_start = galley.layout_from_cursor(egui::text::CCursor::new(col_start));
        assert!(
            word_start.row >= 1,
            "misspelled should start on second visual row, row={}",
            word_start.row
        );

        let segments = squiggle_segments_for_columns(
            &painter,
            line,
            &font_id,
            text_start_x,
            line_top_y,
            16.0,
            0.0,
            wrap_width,
            true,
            col_start,
            col_end,
        );
        assert_eq!(segments.len(), 1, "word should fit on one wrapped row");
        let seg = segments[0];
        assert!(
            seg.x_start >= text_start_x && seg.x_end <= text_start_x + wrap_width + 1.0,
            "x range [{}, {}] should sit within wrap column",
            seg.x_start,
            seg.x_end
        );
        let first_row_baseline = line_top_y + galley.rows[0].rect().max.y - 2.0;
        assert!(
            seg.baseline_y > first_row_baseline + 0.5,
            "squiggle y should be on the word's visual row"
        );
    }

    #[test]
    fn squiggle_unwrapped_single_segment_under_word() {
        let (_ctx, painter) = test_painter();
        let font_id = FontId::proportional(14.0);
        let line = "hello misspelled";
        let text_start_x = 10.0;
        let line_top_y = 5.0;
        let line_height = 18.0;

        let (col_start, col_end) = char_col(line, "misspelled");

        let segments = squiggle_segments_for_columns(
            &painter,
            line,
            &font_id,
            text_start_x,
            line_top_y,
            line_height,
            0.0,
            0.0,
            false,
            col_start,
            col_end,
        );
        assert_eq!(segments.len(), 1);
        let seg = segments[0];
        assert!(seg.x_end > seg.x_start);

        let prefix_before: String = line.chars().take(col_start).collect();
        let prefix_after: String = line.chars().take(col_end).collect();
        let g0 = painter.layout_no_wrap(prefix_before, font_id.clone(), Color32::WHITE);
        let g1 = painter.layout_no_wrap(prefix_after, font_id.clone(), Color32::WHITE);
        assert!((seg.x_start - (text_start_x + g0.size().x)).abs() < 1.0);
        assert!((seg.x_end - (text_start_x + g1.size().x)).abs() < 1.0);
        assert!((seg.baseline_y - (line_top_y + line_height - 2.0)).abs() < 0.01);
    }

    #[test]
    fn squiggle_spanning_wrap_rows_splits_segments() {
        let (_ctx, painter) = test_painter();
        let font_id = FontId::proportional(14.0);
        let line = "aa bb cc dddd eeee ffff";
        let wrap_width = 40.0;
        let col_start = 0;
        let col_end = line.chars().count();

        let galley = painter.layout(
            line.to_string(),
            font_id.clone(),
            Color32::WHITE,
            wrap_width,
        );
        if galley.rows.len() < 2 {
            return;
        }

        let segments = squiggle_segments_for_columns(
            &painter, line, &font_id, 0.0, 0.0, 16.0, 0.0, wrap_width, true, col_start, col_end,
        );
        assert!(
            segments.len() >= 2,
            "full-line span across wrap should produce multiple segments"
        );
    }
}
