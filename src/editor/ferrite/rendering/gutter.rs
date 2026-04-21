//! Gutter (line numbers) rendering for FerriteEditor.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Minimum width of the line number gutter in characters.
/// Set to 3 to provide stability for files up to 999 lines without jumping.
pub const GUTTER_CHARS: usize = 3;

/// Padding between gutter and text content.
pub const GUTTER_PADDING: f32 = 8.0;

/// Renders the gutter background.
pub fn render_gutter_background(
    painter: &egui::Painter,
    rect: Rect,
    gutter_width: f32,
    gutter_bg_color: Color32,
    separator_color: Color32,
) {
    // Draw gutter background
    let gutter_rect = Rect::from_min_size(rect.min, Vec2::new(gutter_width, rect.height()));
    painter.rect_filled(gutter_rect, 0.0, gutter_bg_color);

    // Draw separator line between gutter and content
    let separator_x = rect.min.x + gutter_width;
    painter.line_segment(
        [
            Pos2::new(separator_x, rect.min.y),
            Pos2::new(separator_x, rect.max.y),
        ],
        Stroke::new(1.0, separator_color),
    );
}

/// Renders a line number in the gutter.
pub fn render_line_number(
    painter: &egui::Painter,
    line_idx: usize,
    x: f32,
    y: f32,
    width: f32,
    font_id: &FontId,
    color: Color32,
) {
    let line_num = (line_idx + 1).to_string(); // 1-indexed display

    // Create galley for line number (not cached, simple text)
    let galley = painter.layout_no_wrap(line_num, font_id.clone(), color);

    // Right-align the line number
    let text_width = galley.size().x;
    let text_x = x + width - text_width;

    painter.galley(Pos2::new(text_x, y), galley, color);
}

/// Width of fold indicator area in pixels.
/// Sized to fit the fold indicator character (▼/▶) with minimal padding.
pub const FOLD_INDICATOR_WIDTH: f32 = 12.0;

/// Calculates the width of the line number gutter.
///
/// Returns the total width needed for the gutter area, which can include:
/// - Line numbers (when `show_line_numbers` is true)
/// - Fold indicators (when `show_fold_indicators` is true)
///
/// When both are shown, fold indicators get dedicated space on the left.
/// When only fold indicators are shown (no line numbers), a minimal width is used.
/// Returns 0.0 when both are disabled.
pub fn calculate_gutter_width(
    ui: &egui::Ui,
    font_id: &FontId,
    line_count: usize,
    show_line_numbers: bool,
    show_fold_indicators: bool,
) -> f32 {
    // If neither is shown, no gutter needed
    if !show_line_numbers && !show_fold_indicators {
        return 0.0;
    }

    // If only fold indicators (no line numbers), use minimal width for just the indicators
    if !show_line_numbers && show_fold_indicators {
        return FOLD_INDICATOR_WIDTH;
    }

    // Line numbers are shown - calculate width based on digit count
    let max_line_num = line_count;
    let digits = if max_line_num == 0 {
        1
    } else {
        (max_line_num as f32).log10().floor() as usize + 1
    };

    // Use a sample string to measure width
    let sample = "0".repeat(digits.max(GUTTER_CHARS));
    let galley = ui.fonts(|f| f.layout_no_wrap(sample, font_id.clone(), Color32::WHITE));
    let line_num_width = galley.size().x + 8.0; // Add padding

    // When fold indicators are also shown, add space for them on the left
    if show_fold_indicators {
        line_num_width + FOLD_INDICATOR_WIDTH
    } else {
        line_num_width
    }
}

/// Renders a fold indicator in the gutter.
///
/// Shows ▶ for collapsed folds, ▼ for expanded folds.
pub fn render_fold_indicator(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    line_height: f32,
    is_collapsed: bool,
    color: Color32,
) {
    let indicator = if is_collapsed { "▶" } else { "▼" };
    let font_id = FontId::proportional(line_height * 0.7);
    let galley = painter.layout_no_wrap(indicator.to_string(), font_id, color);

    // Center the indicator vertically
    let indicator_y = y + (line_height - galley.size().y) / 2.0;

    painter.galley(Pos2::new(x, indicator_y), galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gutter_chars_constant() {
        // Verify gutter width constant for line number display
        assert_eq!(GUTTER_CHARS, 3);
    }

    #[test]
    fn test_gutter_padding_constant() {
        // Verify gutter padding constant
        assert!((GUTTER_PADDING - 8.0).abs() < 0.01);
    }
}
