//! Mouse position conversion for FerriteEditor.
//!
//! This module contains:
//! - `pos_to_cursor` - Convert screen position to cursor position
//! - `calculate_column_from_pos` - Calculate column from click coordinates

use egui::{Color32, Pos2, Rect, Ui};

use super::cursor::Cursor;
use super::editor::FerriteEditor;

impl FerriteEditor {
    /// Calculates the column position from click coordinates.
    /// For wrapped text, uses both x and y_in_line to find the correct character.
    pub(crate) fn calculate_column_from_pos(
        &self,
        x: f32,
        y_in_line: f32,
        line: usize,
        font_id: &egui::FontId,
        wrap_width: f32,
        ui: &Ui,
    ) -> usize {
        if let Some(line_content) = self.buffer.get_line(line) {
            let line_content = line_content.trim_end_matches(['\r', '\n']);

            if self.wrap_enabled && wrap_width > 0.0 {
                // For wrapped text, create a wrapped galley and use cursor_from_pos
                let galley = ui.fonts(|f| {
                    f.layout(
                        line_content.to_string(),
                        font_id.clone(),
                        Color32::WHITE,
                        wrap_width,
                    )
                });

                // cursor_from_pos takes a Vec2 position relative to the galley
                let pos = egui::vec2(x.max(0.0), y_in_line.max(0.0));
                let cursor = galley.cursor_from_pos(pos);
                cursor.ccursor.index
            } else {
                // For non-wrapped text, use x-based calculation
                if x <= 0.0 {
                    return 0;
                }

                // Use HarfRust shaped advances for complex-script text
                if crate::fonts::needs_complex_script_fonts(line_content) {
                    let font_bytes = crate::fonts::ttf_bytes_for_font_id_shaping(font_id);
                    if let Some(col) = super::shaping::shaped_x_to_column(
                        line_content,
                        font_bytes,
                        font_id.size,
                        x,
                    ) {
                        return col.min(line_content.chars().count());
                    }
                }

                let chars: Vec<char> = line_content.chars().collect();
                let mut best_col = 0;
                let mut prev_width = 0.0;

                for (i, _) in chars.iter().enumerate() {
                    let prefix: String = chars[..=i].iter().collect();
                    let galley =
                        ui.fonts(|f| f.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE));
                    let width = galley.size().x;

                    let mid_point = (prev_width + width) / 2.0;
                    if x > mid_point {
                        best_col = i + 1;
                    }

                    prev_width = width;

                    if width > x {
                        break;
                    }
                }

                best_col.min(chars.len())
            }
        } else {
            0
        }
    }

    /// Converts a y-coordinate to a line number.
    /// Used for fold indicator click detection.
    ///
    /// This function accounts for folded (hidden) lines - when lines are collapsed,
    /// the visual y-position doesn't map 1-to-1 with document lines.
    pub(crate) fn y_to_line(&self, y: f32, rect_min_y: f32, total_lines: usize) -> usize {
        let relative_y = y - rect_min_y;
        let first_visible = self.view.first_visible_line();
        let mut y_acc = -self.view.scroll_offset_y();

        // Iterate through lines, skipping hidden lines (same as rendering)
        for line_idx in first_visible..total_lines {
            // Skip lines hidden by collapsed folds
            if self.fold_state.is_line_hidden(line_idx) {
                continue;
            }

            let line_height = self.view.get_line_height(line_idx);
            if relative_y < y_acc + line_height {
                return line_idx;
            }
            y_acc += line_height;
        }

        // Past the last visible line - find the last non-hidden line
        for line_idx in (0..total_lines).rev() {
            if !self.fold_state.is_line_hidden(line_idx) {
                return line_idx;
            }
        }

        // Fallback (shouldn't happen unless document is empty)
        total_lines.saturating_sub(1)
    }

    /// Converts a screen position to a cursor position.
    ///
    /// This function accounts for folded (hidden) lines - when lines are collapsed,
    /// the visual y-position doesn't map 1-to-1 with document lines.
    pub(crate) fn pos_to_cursor(
        &self,
        pos: Pos2,
        rect: Rect,
        text_start_x: f32,
        font_id: &egui::FontId,
        wrap_width: f32,
        total_lines: usize,
        ui: &Ui,
    ) -> Cursor {
        let relative_y = pos.y - rect.min.y;
        let text_color = ui.visuals().text_color();
        let first_visible = self.view.first_visible_line();

        // Calculate clicked line and y position within that line
        // Both wrapped and non-wrapped modes need to account for folded lines
        let (clicked_line, y_in_line) = {
            // Start y_acc at -scroll_offset_y to match rendering which places first_visible_line
            // at rect.min.y - scroll_offset_y (i.e., scroll_offset_y pixels ABOVE rect.min.y)
            let mut y_acc = -self.view.scroll_offset_y();
            let mut result_line = first_visible;
            let mut result_y_in_line = 0.0;

            for line_idx in first_visible..total_lines {
                // Skip lines hidden by collapsed folds (same as rendering)
                if self.fold_state.is_line_hidden(line_idx) {
                    continue;
                }

                let line_height = if self.wrap_enabled {
                    // For wrapped text, calculate actual line height from galley
                    if let Some(line_content) = self.buffer.get_line(line_idx) {
                        let display_content = line_content.trim_end_matches(['\r', '\n']);
                        let galley = ui.fonts(|f| {
                            f.layout(
                                display_content.to_string(),
                                font_id.clone(),
                                text_color,
                                wrap_width,
                            )
                        });
                        galley.size().y
                    } else {
                        self.view.line_height()
                    }
                } else {
                    // For non-wrapped text, use uniform line height
                    self.view.line_height()
                };

                if relative_y < y_acc + line_height {
                    result_line = line_idx;
                    result_y_in_line = relative_y - y_acc;
                    break;
                }
                y_acc += line_height;
                result_line = line_idx;
            }

            (result_line, result_y_in_line)
        };

        let clicked_line = clicked_line.min(total_lines.saturating_sub(1));
        let relative_x = pos.x - text_start_x + self.view.horizontal_scroll();
        let clicked_col = self.calculate_column_from_pos(
            relative_x,
            y_in_line,
            clicked_line,
            font_id,
            wrap_width,
            ui,
        );

        Cursor::new(clicked_line, clicked_col)
    }
}
