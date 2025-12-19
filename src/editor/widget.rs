//! Text editor widget for Ferrite
//!
//! This module implements the main text editor widget using egui's TextEdit,
//! with support for text input, cursor movement, selection, clipboard operations,
//! scrolling, and optional line numbers.

use crate::config::EditorFont;
use crate::fonts;
use crate::state::Tab;
use crate::theme::ThemeColors;
use eframe::egui::{self, FontId, ScrollArea, TextEdit, Ui};
use log::debug;
use std::sync::Arc;

/// Result of showing the editor widget.
pub struct EditorOutput {
    /// Whether the content was modified.
    pub changed: bool,
}

/// Search match highlight information.
#[derive(Debug, Clone, Default)]
pub struct SearchHighlights {
    /// All matches as (start, end) byte positions
    pub matches: Vec<(usize, usize)>,
    /// Index of the current match (for distinct highlighting)
    pub current_match: usize,
    /// Whether to scroll to the current match
    pub scroll_to_match: bool,
}

/// A text editor widget that integrates with the Tab state.
///
/// This widget wraps egui's TextEdit with additional functionality:
/// - Integration with Tab's undo/redo stack
/// - Cursor position tracking (line, column)
/// - Scroll offset persistence
/// - Font size and styling from Settings
/// - Optional line number gutter
/// - Search match highlighting
/// - Scroll-to-line navigation (for outline panel)
///
/// # Example
///
/// ```ignore
/// EditorWidget::new(&mut tab)
///     .font_size(settings.font_size)
///     .show_line_numbers(true)
///     .search_highlights(highlights)
///     .scroll_to_line(Some(42))
///     .show(ui);
/// ```
pub struct EditorWidget<'a> {
    /// The tab being edited.
    tab: &'a mut Tab,
    /// Font size for the editor.
    font_size: f32,
    /// Whether to show a frame around the editor.
    frame: bool,
    /// Whether word wrap is enabled.
    word_wrap: bool,
    /// ID for the editor (for state persistence).
    id: Option<egui::Id>,
    /// Whether to show line numbers.
    show_line_numbers: bool,
    /// Theme colors for styling line numbers.
    theme_colors: Option<ThemeColors>,
    /// Search match highlights to render.
    search_highlights: Option<SearchHighlights>,
    /// Font family for the editor.
    font_family: EditorFont,
    /// Line number to scroll to (1-indexed, from outline navigation).
    scroll_to_line: Option<usize>,
}

impl<'a> EditorWidget<'a> {
    /// Create a new editor widget for the given tab.
    pub fn new(tab: &'a mut Tab) -> Self {
        Self {
            tab,
            font_size: 14.0,
            frame: false,
            word_wrap: true,
            id: None,
            show_line_numbers: true,
            theme_colors: None,
            search_highlights: None,
            font_family: EditorFont::default(),
            scroll_to_line: None,
        }
    }

    /// Set the font size for the editor.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set whether word wrap is enabled.
    #[must_use]
    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    /// Set a custom ID for the editor.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set whether to show line numbers.
    #[must_use]
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set theme colors for styling (used for line numbers).
    #[must_use]
    pub fn theme_colors(mut self, colors: ThemeColors) -> Self {
        self.theme_colors = Some(colors);
        self
    }

    /// Set search highlights to render.
    #[must_use]
    pub fn search_highlights(mut self, highlights: SearchHighlights) -> Self {
        self.search_highlights = Some(highlights);
        self
    }

    /// Set the font family for the editor.
    #[must_use]
    pub fn font_family(mut self, font_family: EditorFont) -> Self {
        self.font_family = font_family;
        self
    }

    /// Set a line to scroll to (1-indexed, for outline navigation).
    #[must_use]
    pub fn scroll_to_line(mut self, line: Option<usize>) -> Self {
        self.scroll_to_line = line;
        self
    }

    /// Show the editor widget and return the output.
    pub fn show(self, ui: &mut Ui) -> EditorOutput {
        // Include content_version in the ID so that egui treats the TextEdit as
        // a new widget when content changes externally (e.g., via undo/redo).
        // This forces the TextEdit to re-read from the source string.
        let base_id = self.id.unwrap_or_else(|| ui.id().with("editor"));
        let id = base_id.with(self.tab.content_version());

        // Check if we need to request focus (new tab) and clear the flag
        let needs_focus = self.tab.needs_focus;
        if needs_focus {
            self.tab.needs_focus = false;
        }

        // Store original content for change detection
        let original_content = self.tab.content.clone();

        // Capture values for closures
        let font_size = self.font_size;
        let word_wrap = self.word_wrap;
        let show_line_numbers = self.show_line_numbers;
        let theme_colors = self.theme_colors.clone();
        let search_highlights = self.search_highlights.clone();

        // Calculate gutter width if line numbers are enabled
        let gutter_width = if show_line_numbers {
            let line_count = super::line_numbers::count_lines(&self.tab.content);
            let digit_count = if line_count == 0 {
                1
            } else {
                (line_count as f32).log10().floor() as usize + 1
            };
            let char_width = font_size * 0.6;
            let content_width = char_width * digit_count as f32;
            (content_width + 20.0).max(30.0) // padding + min width
        } else {
            0.0
        };

        // Create a mutable reference to the content
        let content = &mut self.tab.content;

        // Get font family for the editor
        let font_family = fonts::get_styled_font_family(false, false, self.font_family);

        // Configure the text layout based on word wrap
        let font_family_clone = font_family.clone();
        let mut layouter = move |ui: &Ui, text: &str, wrap_width: f32| -> Arc<egui::Galley> {
            let font_id = FontId::new(font_size, font_family_clone.clone());
            let layout_job = if word_wrap {
                egui::text::LayoutJob::simple(
                    text.to_owned(),
                    font_id,
                    ui.visuals().text_color(),
                    wrap_width,
                )
            } else {
                egui::text::LayoutJob::simple_singleline(
                    text.to_owned(),
                    font_id,
                    ui.visuals().text_color(),
                )
            };
            ui.fonts(|f| f.layout_job(layout_job))
        };

        // Calculate scroll offset for current match if needed
        let mut target_scroll_offset: Option<f32> = None;

        // Priority 1: Scroll to specific line (from outline navigation)
        if let Some(target_line) = self.scroll_to_line {
            let line_height =
                ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family.clone())));
            // Target scroll position: put the line roughly 1/3 from top of viewport
            let viewport_height = ui.available_height();
            // target_line is 1-indexed, convert to 0-indexed for calculation
            let target_y = (target_line.saturating_sub(1)) as f32 * line_height;
            target_scroll_offset = Some((target_y - viewport_height / 3.0).max(0.0));
            debug!("Scrolling to line {} (y offset {})", target_line, target_y);
        }
        // Priority 2: Scroll to search match
        else if let Some(ref highlights) = search_highlights {
            if highlights.scroll_to_match && !highlights.matches.is_empty() {
                if let Some(&(match_start, _)) = highlights.matches.get(highlights.current_match) {
                    // Calculate line number of the match
                    let (match_line, _) = char_index_to_line_col(content, match_start);
                    let line_height =
                        ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family.clone())));
                    // Target scroll position: put the match roughly 1/3 from top of viewport
                    let viewport_height = ui.available_height();
                    let match_y = match_line as f32 * line_height;
                    target_scroll_offset = Some((match_y - viewport_height / 3.0).max(0.0));
                }
            }
        }

        // Use ScrollArea for viewport management - line numbers scroll with content
        let mut scroll_area = ScrollArea::vertical()
            .id_source(id.with("scroll"))
            .auto_shrink([false, false]);

        // Apply scroll offset if we need to jump to a match
        if let Some(offset) = target_scroll_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        let scroll_output = scroll_area.show(ui, |ui| {
            // Use horizontal layout inside ScrollArea so gutter and editor scroll together
            ui.horizontal_top(|ui| {
                // Reserve space for the gutter (will be drawn after we know text positions)
                let gutter_rect = if show_line_numbers {
                    let line_count = super::line_numbers::count_lines(content);
                    let line_height =
                        ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family.clone())));
                    let total_height = line_count as f32 * line_height;

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(gutter_width, total_height.max(ui.available_height())),
                        egui::Sense::hover(),
                    );
                    Some(rect)
                } else {
                    None
                };

                // Create the multiline text editor
                let text_edit = TextEdit::multiline(content)
                    .id(id)
                    .frame(self.frame)
                    .font(FontId::new(font_size, font_family.clone()))
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter);

                // Show the editor and get the output
                let text_output = text_edit.show(ui);

                // Request focus if this is a new tab that needs it
                if needs_focus {
                    text_output.response.request_focus();
                }

                // Draw search match highlights
                if let Some(ref highlights) = search_highlights {
                    if !highlights.matches.is_empty() {
                        let galley = &text_output.galley;
                        let galley_pos = text_output.galley_pos;
                        let painter = ui.painter();
                        let is_dark = theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);

                        // Highlight colors
                        let current_match_color = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 200, 0, 150)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 220, 0, 180)
                        };
                        let other_match_color = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(180, 150, 50, 80)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 100, 120)
                        };

                        for (idx, &(match_start, match_end)) in
                            highlights.matches.iter().enumerate()
                        {
                            let is_current = idx == highlights.current_match;
                            let color = if is_current {
                                current_match_color
                            } else {
                                other_match_color
                            };

                            // Get rectangles for this text range from the galley
                            let cursor_start = egui::text::CCursor::new(match_start);
                            let cursor_end = egui::text::CCursor::new(match_end);

                            // Get the row and position for start and end
                            let start_cursor = galley.from_ccursor(cursor_start);
                            let end_cursor = galley.from_ccursor(cursor_end);
                            let start_rcursor = start_cursor.rcursor;
                            let end_rcursor = end_cursor.rcursor;

                            // Handle single-row or multi-row highlights
                            if start_rcursor.row == end_rcursor.row {
                                // Single row - draw one rectangle
                                if let Some(row) = galley.rows.get(start_rcursor.row) {
                                    let row_rect = row.rect;
                                    let x_start = row.x_offset(start_rcursor.column);
                                    let x_end = row.x_offset(end_rcursor.column);

                                    let highlight_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            galley_pos.x + x_start,
                                            galley_pos.y + row_rect.min.y,
                                        ),
                                        egui::pos2(
                                            galley_pos.x + x_end,
                                            galley_pos.y + row_rect.max.y,
                                        ),
                                    );
                                    painter.rect_filled(highlight_rect, 2.0, color);
                                }
                            } else {
                                // Multi-row highlight
                                for row_idx in start_rcursor.row..=end_rcursor.row {
                                    if let Some(row) = galley.rows.get(row_idx) {
                                        let row_rect = row.rect;

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

                                        let highlight_rect = egui::Rect::from_min_max(
                                            egui::pos2(
                                                galley_pos.x + x_start,
                                                galley_pos.y + row_rect.min.y,
                                            ),
                                            egui::pos2(
                                                galley_pos.x + x_end,
                                                galley_pos.y + row_rect.max.y,
                                            ),
                                        );
                                        painter.rect_filled(highlight_rect, 2.0, color);
                                    }
                                }
                            }
                        }
                    }
                }

                // Now draw line numbers using the actual galley positions
                if let Some(gutter_rect) = gutter_rect {
                    let galley = &text_output.galley;
                    let galley_pos = text_output.galley_pos;

                    // Get colors for styling
                    let line_color = theme_colors
                        .as_ref()
                        .map(|c| c.text.muted)
                        .unwrap_or(egui::Color32::from_rgb(120, 120, 120));
                    let bg_color = theme_colors
                        .as_ref()
                        .map(|c| c.base.background_secondary)
                        .unwrap_or(egui::Color32::from_rgb(245, 245, 245));
                    let border_color = theme_colors
                        .as_ref()
                        .map(|c| c.base.border_subtle)
                        .unwrap_or(egui::Color32::from_rgb(200, 200, 200));

                    let painter = ui.painter();

                    // Draw gutter background
                    painter.rect_filled(gutter_rect, 0.0, bg_color);

                    // Draw separator line
                    painter.line_segment(
                        [
                            gutter_rect.right_top() + egui::vec2(-1.0, 0.0),
                            gutter_rect.right_bottom() + egui::vec2(-1.0, 0.0),
                        ],
                        egui::Stroke::new(1.0, border_color),
                    );

                    // Draw line numbers aligned with actual galley rows
                    // Always use monospace font for line numbers for proper alignment
                    let line_number_font_id = FontId::monospace(font_size);

                    // Track logical line number
                    // With word wrap, multiple rows can belong to the same logical line
                    // A row ends a logical line when ends_with_newline is true
                    let mut logical_line = 0usize;
                    let mut line_number_drawn_for_line = false;

                    for row in galley.rows.iter() {
                        // Get the absolute Y position of this row (screen coordinates)
                        let row_y = galley_pos.y + row.min_y();

                        // Draw line number only once per logical line (at the first row of a wrapped line)
                        if !line_number_drawn_for_line {
                            let display_num = logical_line + 1; // 1-indexed

                            // Position line number at EXACT same Y as the text row
                            // Use absolute row_y to ensure perfect alignment regardless of
                            // any offset between gutter_rect and galley_pos
                            let text_pos = egui::pos2(
                                gutter_rect.right() - 12.0, // Right padding
                                row_y,                      // Absolute Y position from galley row
                            );

                            painter.text(
                                text_pos,
                                egui::Align2::RIGHT_TOP,
                                format!("{}", display_num),
                                line_number_font_id.clone(),
                                line_color,
                            );

                            line_number_drawn_for_line = true;
                        }

                        // Check if this row ends a logical line (has newline at the end)
                        if row.ends_with_newline {
                            logical_line += 1;
                            line_number_drawn_for_line = false;
                        }
                    }

                    // Handle empty content (no rows in galley)
                    if galley.rows.is_empty() {
                        let text_pos = egui::pos2(
                            gutter_rect.right() - 12.0,
                            galley_pos.y, // Use galley position for empty content
                        );
                        painter.text(
                            text_pos,
                            egui::Align2::RIGHT_TOP,
                            "1",
                            line_number_font_id,
                            line_color,
                        );
                    }
                }

                text_output
            })
            .inner
        });

        let text_output = scroll_output.inner;
        let cursor_range_opt = text_output.cursor_range;

        // Determine if content changed
        let changed = self.tab.content != original_content;

        // If content changed, record for undo tracking
        if changed {
            // TextEdit modifies content directly, so we need to manually
            // record the edit for undo/redo functionality
            self.tab.record_edit(original_content);
            debug!("Editor content changed, recorded for undo");
        }

        // Calculate cursor position (line, column) and selection from cursor range
        let (cursor_position, selection) = if let Some(cursor_range) = cursor_range_opt {
            let primary = cursor_range.primary.ccursor.index;
            let secondary = cursor_range.secondary.ccursor.index;

            // Convert character index to (line, column)
            let pos = char_index_to_line_col(&self.tab.content, primary);

            // Check if there's a selection (primary != secondary)
            let sel = if primary != secondary {
                let (start, end) = if primary < secondary {
                    (primary, secondary)
                } else {
                    (secondary, primary)
                };
                Some((start, end))
            } else {
                None
            };

            (pos, sel)
        } else {
            (self.tab.cursor_position, self.tab.selection)
        };

        // Update tab's cursor position and selection
        self.tab.cursor_position = cursor_position;
        self.tab.selection = selection;

        // Update scroll offset from ScrollArea state
        self.tab.scroll_offset = scroll_output.state.offset.y;

        EditorOutput { changed }
    }
}

/// Convert a character index to (line, column) position.
///
/// Both line and column are 0-indexed.
fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in text.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Convert (line, column) position to a character index.
///
/// Both line and column are 0-indexed.
/// Returns the closest valid index if position is out of bounds.
#[allow(dead_code)]
fn line_col_to_char_index(text: &str, line: usize, col: usize) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;

    for (i, ch) in text.chars().enumerate() {
        if current_line == line && current_col == col {
            return i;
        }
        if ch == '\n' {
            if current_line == line {
                // Reached end of target line before reaching column
                return i;
            }
            current_line += 1;
            current_col = 0;
        } else if current_line == line {
            current_col += 1;
        }
    }

    // Return end of text if position is beyond
    text.chars().count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_index_to_line_col_empty() {
        assert_eq!(char_index_to_line_col("", 0), (0, 0));
    }

    #[test]
    fn test_char_index_to_line_col_single_line() {
        let text = "Hello, World!";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0));
        assert_eq!(char_index_to_line_col(text, 5), (0, 5));
        assert_eq!(char_index_to_line_col(text, 13), (0, 13));
    }

    #[test]
    fn test_char_index_to_line_col_multiline() {
        let text = "Hello\nWorld\n!";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0)); // 'H'
        assert_eq!(char_index_to_line_col(text, 5), (0, 5)); // '\n'
        assert_eq!(char_index_to_line_col(text, 6), (1, 0)); // 'W'
        assert_eq!(char_index_to_line_col(text, 11), (1, 5)); // '\n'
        assert_eq!(char_index_to_line_col(text, 12), (2, 0)); // '!'
    }

    #[test]
    fn test_line_col_to_char_index_empty() {
        assert_eq!(line_col_to_char_index("", 0, 0), 0);
    }

    #[test]
    fn test_line_col_to_char_index_single_line() {
        let text = "Hello, World!";
        assert_eq!(line_col_to_char_index(text, 0, 0), 0);
        assert_eq!(line_col_to_char_index(text, 0, 5), 5);
        assert_eq!(line_col_to_char_index(text, 0, 13), 13);
    }

    #[test]
    fn test_line_col_to_char_index_multiline() {
        let text = "Hello\nWorld\n!";
        assert_eq!(line_col_to_char_index(text, 0, 0), 0); // 'H'
        assert_eq!(line_col_to_char_index(text, 1, 0), 6); // 'W'
        assert_eq!(line_col_to_char_index(text, 2, 0), 12); // '!'
    }

    #[test]
    fn test_line_col_to_char_index_out_of_bounds() {
        let text = "Hi\nBye";
        // Column beyond line length
        assert_eq!(line_col_to_char_index(text, 0, 10), 2); // end of first line
                                                            // Line beyond text
        assert_eq!(line_col_to_char_index(text, 5, 0), 6); // end of text
    }

    #[test]
    fn test_roundtrip_conversion() {
        let text = "Line 1\nLine 2\nLine 3";

        // Test various positions
        for char_idx in [0, 3, 6, 7, 10, 13, 14, 17, 20] {
            if char_idx <= text.chars().count() {
                let (line, col) = char_index_to_line_col(text, char_idx);
                let back = line_col_to_char_index(text, line, col);
                assert_eq!(back, char_idx, "Roundtrip failed for index {}", char_idx);
            }
        }
    }
}
