//! Search and bracket matching highlight rendering for FerriteEditor.
//!
//! This module contains:
//! - Search match highlight rendering (`render_search_highlights`)
//! - Bracket matching highlight rendering (`render_bracket_matching`)
//! - Range highlight helper (`render_range_highlight`)
//! - Position conversion utilities
//! - Bracket matching API methods

use egui::{Color32, FontId, Pos2, Rect};

use super::cursor::Cursor;
use super::editor::FerriteEditor;

// Import bracket matching
use crate::editor::matching::DelimiterMatcher;

/// Maximum number of search matches to display/highlight.
/// Matches beyond this limit are counted but not rendered for performance.
/// This matches VS Code's behavior.
pub(crate) const MAX_DISPLAYED_MATCHES: usize = 1000;

impl FerriteEditor {
    // ─────────────────────────────────────────────────────────────────────────────
    // Search Highlight Rendering
    // ─────────────────────────────────────────────────────────────────────────────

    /// Renders search match highlights for visible lines.
    ///
    /// Current match is rendered with a brighter color, other matches with dimmer color.
    /// Only up to MAX_DISPLAYED_MATCHES (1000) are rendered for performance.
    ///
    /// # Performance
    /// - Uses pre-computed line numbers from SearchMatch struct
    /// - Uses rope's native byte_to_char method - O(log N), no string allocation
    /// - Caps rendering at 1000 matches to avoid lag with common single-char searches
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_search_highlights(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        text_start_x: f32,
        font_id: &FontId,
        wrap_width: f32,
        start_line: usize,
        end_line: usize,
        is_dark: bool,
    ) {
        // Define highlight colors (current match brighter, others dimmer)
        let current_match_color = if is_dark {
            Color32::from_rgba_unmultiplied(255, 200, 0, 150) // Bright yellow/orange
        } else {
            Color32::from_rgba_unmultiplied(255, 220, 0, 180)
        };
        let other_match_color = if is_dark {
            Color32::from_rgba_unmultiplied(180, 150, 50, 80) // Dimmer yellow
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 100, 120)
        };

        // Only render up to MAX_DISPLAYED_MATCHES to avoid performance issues
        let matches_to_render = self.search_matches.len().min(MAX_DISPLAYED_MATCHES);

        for (idx, search_match) in self
            .search_matches
            .iter()
            .take(matches_to_render)
            .enumerate()
        {
            // Use pre-computed line number for efficient visibility check
            let match_start_line = search_match.line;
            let match_end_line = self.byte_pos_to_line(search_match.end_byte.saturating_sub(1));

            // Skip matches outside visible range
            if match_end_line < start_line || match_start_line >= end_line {
                continue;
            }

            let is_current = idx == self.current_search_match;
            let color = if is_current {
                current_match_color
            } else {
                other_match_color
            };

            // Convert byte positions to character positions using rope's native O(log n) method
            let char_start = self
                .buffer
                .try_byte_to_char(search_match.start_byte)
                .unwrap_or(0);
            let char_end = self
                .buffer
                .try_byte_to_char(search_match.end_byte)
                .unwrap_or(char_start);

            // Render the highlight
            self.render_range_highlight(
                painter,
                rect,
                text_start_x,
                font_id,
                wrap_width,
                char_start,
                char_end,
                color,
                2.0,  // corner radius
                None, // no border
            );
        }
    }

    /// Renders bracket matching highlights.
    ///
    /// # Performance
    /// Uses windowed search (O(window) complexity) instead of full-file search.
    /// Only extracts cursor ± MAX_BRACKET_SEARCH_LINES as a string, making this
    /// efficient even for very large files (100MB+).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_bracket_matching(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        text_start_x: f32,
        font_id: &FontId,
        wrap_width: f32,
        start_line: usize,
        end_line: usize,
        is_dark: bool,
    ) {
        /// Maximum number of lines to search in each direction from cursor.
        /// This bounds the search to O(window) instead of O(N).
        const MAX_BRACKET_SEARCH_LINES: usize = 100;

        // Get cursor position (use primary cursor for bracket matching)
        let cursor = self.primary_selection().head;
        let cursor_line = cursor.line;
        let total_lines = self.buffer.line_count();

        // Calculate the search window (cursor ± MAX_BRACKET_SEARCH_LINES)
        let window_start_line = cursor_line.saturating_sub(MAX_BRACKET_SEARCH_LINES);
        let window_end_line = (cursor_line + MAX_BRACKET_SEARCH_LINES + 1).min(total_lines);

        // Extract only the window as a string (O(window) instead of O(N))
        let (window_content, window_start_char) = self
            .buffer
            .slice_lines_to_string(window_start_line, window_end_line);

        if window_content.is_empty() {
            return;
        }

        // Calculate cursor position relative to the window
        let cursor_char_pos = self.cursor_to_char_pos(cursor);
        let cursor_in_window = cursor_char_pos.saturating_sub(window_start_char);

        // Create a delimiter matcher for the window
        let matcher = DelimiterMatcher::new(&window_content);

        // Find matching bracket at cursor position (relative to window)
        if let Some(matching_pair) = matcher.find_match(cursor_in_window) {
            // Get colors (from settings or theme defaults)
            let (bg_color, border_color) = self.bracket_colors.unwrap_or_else(|| {
                if is_dark {
                    (
                        Color32::from_rgba_unmultiplied(80, 180, 220, 60),
                        Color32::from_rgb(100, 180, 220),
                    )
                } else {
                    (
                        Color32::from_rgba_unmultiplied(255, 220, 100, 80),
                        Color32::from_rgb(200, 170, 50),
                    )
                }
            });

            // Calculate the byte offset of the window start for position adjustment
            let window_start_byte = self.buffer.try_char_to_byte(window_start_char).unwrap_or(0);

            // Draw highlights for both source and target delimiters
            for token in [&matching_pair.source, &matching_pair.target] {
                // Adjust byte positions from window-relative to full document
                let doc_byte_start = token.start + window_start_byte;
                let doc_byte_end = token.end + window_start_byte;

                // Convert byte positions to character positions using rope's native O(log N) method
                let char_start = self.buffer.try_byte_to_char(doc_byte_start).unwrap_or(0);
                let char_end = self
                    .buffer
                    .try_byte_to_char(doc_byte_end)
                    .unwrap_or(char_start);

                // Check if token is in visible range using rope's native O(log N) method
                let token_line = self.buffer.try_byte_to_line(doc_byte_start).unwrap_or(0);
                if token_line < start_line || token_line >= end_line {
                    continue;
                }

                // Render the highlight with both fill and border
                self.render_range_highlight(
                    painter,
                    rect,
                    text_start_x,
                    font_id,
                    wrap_width,
                    char_start,
                    char_end,
                    bg_color,
                    2.0, // corner radius
                    Some(egui::Stroke::new(1.0, border_color)),
                );
            }
        }
    }

    /// Renders a highlight rectangle for a character range.
    ///
    /// This helper method handles both wrapped and non-wrapped text modes.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_range_highlight(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        text_start_x: f32,
        font_id: &FontId,
        wrap_width: f32,
        char_start: usize,
        char_end: usize,
        fill_color: Color32,
        corner_radius: f32,
        border_stroke: Option<egui::Stroke>,
    ) {
        // Find which line(s) the range spans
        let start_line = self.char_pos_to_line(char_start);
        let end_line = self.char_pos_to_line(char_end.saturating_sub(1).max(char_start));

        for line_idx in start_line..=end_line {
            if let Some(line_content) = self.buffer.get_line(line_idx) {
                let line = line_content.trim_end_matches(['\r', '\n']);
                let line_chars: Vec<char> = line.chars().collect();
                let line_len = line_chars.len();

                // Get the character offset for this line
                let line_start_char = self.line_to_char_pos(line_idx);

                // Calculate the range within this line
                let line_range_start = char_start.saturating_sub(line_start_char).min(line_len);
                let line_range_end = char_end.saturating_sub(line_start_char).min(line_len);

                if line_range_start >= line_range_end {
                    continue;
                }

                // Calculate line Y position
                let line_y = if self.wrap_enabled && !self.view.wrap_info().is_empty() {
                    rect.min.y + self.view.get_line_y_offset(line_idx)
                        - self.view.get_line_y_offset(self.view.first_visible_line())
                        - self.view.scroll_offset_y()
                } else {
                    rect.min.y + self.view.line_to_pixel(line_idx)
                };

                if self.wrap_enabled && wrap_width > 0.0 {
                    // For wrapped text, use galley-based positioning
                    let galley = painter.layout(
                        line.to_string(),
                        font_id.clone(),
                        Color32::WHITE,
                        wrap_width,
                    );

                    let start_ccursor = egui::text::CCursor::new(line_range_start);
                    let end_ccursor = egui::text::CCursor::new(line_range_end);

                    let start_cursor = galley.from_ccursor(start_ccursor);
                    let end_cursor = galley.from_ccursor(end_ccursor);
                    let start_rcursor = start_cursor.rcursor;
                    let end_rcursor = end_cursor.rcursor;

                    // Handle single-row or multi-row highlights
                    if start_rcursor.row == end_rcursor.row {
                        if let Some(row) = galley.rows.get(start_rcursor.row) {
                            let row_rect = row.rect;
                            let x_start = row.x_offset(start_rcursor.column);
                            let x_end = row.x_offset(end_rcursor.column);

                            let highlight_rect = Rect::from_min_max(
                                Pos2::new(text_start_x + x_start, line_y + row_rect.min.y),
                                Pos2::new(text_start_x + x_end, line_y + row_rect.max.y),
                            );
                            painter.rect_filled(highlight_rect, corner_radius, fill_color);
                            if let Some(stroke) = border_stroke {
                                painter.rect_stroke(highlight_rect, corner_radius, stroke);
                            }
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

                                let highlight_rect = Rect::from_min_max(
                                    Pos2::new(text_start_x + x_start, line_y + row_rect.min.y),
                                    Pos2::new(text_start_x + x_end, line_y + row_rect.max.y),
                                );
                                painter.rect_filled(highlight_rect, corner_radius, fill_color);
                                if let Some(stroke) = border_stroke {
                                    painter.rect_stroke(highlight_rect, corner_radius, stroke);
                                }
                            }
                        }
                    }
                } else {
                    // Non-wrapped mode: simple rectangle calculation
                    let start_x = if line_range_start == 0 {
                        text_start_x - self.view.horizontal_scroll()
                    } else {
                        let prefix: String = line_chars.iter().take(line_range_start).collect();
                        let galley =
                            painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
                        text_start_x + galley.size().x - self.view.horizontal_scroll()
                    };

                    let end_x = {
                        let prefix: String = line_chars.iter().take(line_range_end).collect();
                        let galley =
                            painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
                        text_start_x + galley.size().x - self.view.horizontal_scroll()
                    };

                    let highlight_rect = Rect::from_min_max(
                        Pos2::new(start_x, line_y),
                        Pos2::new(end_x, line_y + self.view.line_height()),
                    );
                    painter.rect_filled(highlight_rect, corner_radius, fill_color);
                    if let Some(stroke) = border_stroke {
                        painter.rect_stroke(highlight_rect, corner_radius, stroke);
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Position Conversion Utilities
    // ─────────────────────────────────────────────────────────────────────────────

    /// Converts a character position to a line number.
    ///
    /// # Performance
    /// Uses the rope's native char_to_line method - O(log N), no allocation.
    pub(crate) fn char_pos_to_line(&self, char_pos: usize) -> usize {
        self.buffer.try_char_to_line(char_pos).unwrap_or_else(|| {
            // Fallback for out-of-bounds: return last line
            self.buffer.line_count().saturating_sub(1)
        })
    }

    /// Converts a line number to the starting character position.
    ///
    /// # Performance
    /// Uses the rope's native line_to_char method - O(log N), no allocation.
    pub(crate) fn line_to_char_pos(&self, line: usize) -> usize {
        self.buffer.try_line_to_char(line).unwrap_or_else(|| {
            // Fallback for out-of-bounds: return total character count
            self.buffer.len()
        })
    }

    /// Converts a cursor position to a character position in the buffer.
    pub(crate) fn cursor_to_char_pos(&self, cursor: Cursor) -> usize {
        let mut char_pos = 0;
        for line_idx in 0..cursor.line {
            if let Some(line_content) = self.buffer.get_line(line_idx) {
                char_pos += line_content.chars().count();
            }
        }
        char_pos + cursor.column
    }

    /// Converts a byte position to a line number.
    ///
    /// # Performance
    /// Uses the rope's native O(log n) method instead of string allocation.
    pub(crate) fn byte_pos_to_line(&self, byte_pos: usize) -> usize {
        // Use the rope's native O(log n) method instead of string allocation
        self.buffer.try_byte_to_line(byte_pos).unwrap_or_else(|| {
            // Fallback for out-of-bounds: return last line
            self.buffer.line_count().saturating_sub(1)
        })
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Bracket Matching API
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns whether bracket matching is enabled.
    #[must_use]
    pub fn is_bracket_matching_enabled(&self) -> bool {
        self.bracket_matching_enabled
    }

    /// Enables or disables bracket matching.
    ///
    /// When enabled, the editor highlights matching bracket pairs when the
    /// cursor is adjacent to a bracket character.
    pub fn set_bracket_matching_enabled(&mut self, enabled: bool) {
        self.bracket_matching_enabled = enabled;
    }

    /// Sets custom colors for bracket matching.
    ///
    /// # Arguments
    /// * `colors` - Tuple of (background_color, border_color), or None to use theme defaults
    pub fn set_bracket_colors(&mut self, colors: Option<(Color32, Color32)>) {
        self.bracket_colors = colors;
    }

    /// Returns the current bracket matching colors.
    #[must_use]
    pub fn bracket_colors(&self) -> Option<(Color32, Color32)> {
        self.bracket_colors
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Diagnostic Squiggle Rendering
    // ─────────────────────────────────────────────────────────────────────────────

    /// Renders wavy underlines for LSP diagnostics on visible lines.
    ///
    /// Red for errors, yellow for warnings; only visible-line diagnostics are drawn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_diagnostic_squiggles(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        text_start_x: f32,
        font_id: &FontId,
        _wrap_width: f32,
        start_line: usize,
        end_line: usize,
        line_y_positions: &[f32],
        is_dark: bool,
    ) {
        use crate::lsp::state::DiagnosticSeverity;

        for diag in &self.diagnostics {
            // Quick reject: diagnostic completely outside visible range
            if diag.end_line < start_line || diag.start_line >= end_line {
                continue;
            }

            let color = match diag.severity {
                DiagnosticSeverity::Error => {
                    if is_dark {
                        Color32::from_rgb(255, 80, 80)
                    } else {
                        Color32::from_rgb(220, 30, 30)
                    }
                }
                DiagnosticSeverity::Warning => {
                    if is_dark {
                        Color32::from_rgb(255, 200, 50)
                    } else {
                        Color32::from_rgb(200, 150, 0)
                    }
                }
                DiagnosticSeverity::Information => {
                    if is_dark {
                        Color32::from_rgb(80, 160, 255)
                    } else {
                        Color32::from_rgb(40, 100, 200)
                    }
                }
                DiagnosticSeverity::Hint => {
                    if is_dark {
                        Color32::from_rgb(140, 140, 140)
                    } else {
                        Color32::from_rgb(100, 100, 100)
                    }
                }
            };

            // Draw on each visible line the diagnostic spans
            let first = diag.start_line.max(start_line);
            let last = diag.end_line.min(end_line.saturating_sub(1));

            for line_idx in first..=last {
                let Some(line_content) = self.buffer.get_line(line_idx) else {
                    continue;
                };
                let line = line_content.trim_end_matches(['\r', '\n']);
                let line_chars: Vec<char> = line.chars().collect();
                let line_len = line_chars.len();

                if line_len == 0 {
                    continue;
                }

                // Column range on this line
                let col_start = if line_idx == diag.start_line {
                    diag.start_col.min(line_len)
                } else {
                    0
                };
                let col_end = if line_idx == diag.end_line {
                    diag.end_col.min(line_len)
                } else {
                    line_len
                };

                if col_start >= col_end {
                    // If range is empty on this line, underline at least one char
                    let col_end_fixed = (col_start + 1).min(line_len);
                    if col_start >= col_end_fixed {
                        continue;
                    }
                    self.draw_squiggle(
                        painter,
                        rect,
                        text_start_x,
                        font_id,
                        &line_chars,
                        line_idx,
                        start_line,
                        line_y_positions,
                        col_start,
                        col_end_fixed,
                        color,
                    );
                } else {
                    self.draw_squiggle(
                        painter,
                        rect,
                        text_start_x,
                        font_id,
                        &line_chars,
                        line_idx,
                        start_line,
                        line_y_positions,
                        col_start,
                        col_end,
                        color,
                    );
                }
            }
        }
    }

    /// Draw a wavy underline under columns `col_start..col_end` on `line_idx`.
    #[allow(clippy::too_many_arguments)]
    fn draw_squiggle(
        &self,
        painter: &egui::Painter,
        _rect: Rect,
        text_start_x: f32,
        font_id: &FontId,
        line_chars: &[char],
        line_idx: usize,
        start_line: usize,
        line_y_positions: &[f32],
        col_start: usize,
        col_end: usize,
        color: Color32,
    ) {
        let pos_idx = line_idx.saturating_sub(start_line);
        if pos_idx >= line_y_positions.len() {
            return;
        }
        let line_y = line_y_positions[pos_idx];
        let line_height = self.view.get_line_height(line_idx);
        let baseline_y = line_y + line_height - 2.0;

        // X offsets via galley measurement
        let x_start = if col_start == 0 {
            text_start_x - self.view.horizontal_scroll()
        } else {
            let prefix: String = line_chars.iter().take(col_start).collect();
            let g = painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
            text_start_x + g.size().x - self.view.horizontal_scroll()
        };

        let x_end = {
            let prefix: String = line_chars.iter().take(col_end).collect();
            let g = painter.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE);
            text_start_x + g.size().x - self.view.horizontal_scroll()
        };

        if x_end <= x_start {
            return;
        }

        // Draw a wavy line: period ~4px, amplitude ~1.5px
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

    /// Check if a cursor position falls within a diagnostic's range.
    pub(crate) fn cursor_in_diagnostic_range(
        &self,
        cursor: &Cursor,
        diag: &crate::lsp::state::DiagnosticEntry,
    ) -> bool {
        let line = cursor.line;
        let col = cursor.column;

        if line < diag.start_line || line > diag.end_line {
            return false;
        }
        if line == diag.start_line && col < diag.start_col {
            return false;
        }
        if line == diag.end_line && col >= diag.end_col {
            return false;
        }
        true
    }

    /// Configures search highlights and bracket matching at once.
    ///
    /// This is a convenience method for setting up all Phase 2 highlight options.
    ///
    /// # Arguments
    /// * `search_matches` - Vector of (start_byte, end_byte) positions, or None to clear
    /// * `current_match` - Index of the current search match
    /// * `scroll_to_match` - Whether to scroll to the current match
    /// * `bracket_matching` - Whether to enable bracket matching
    /// * `bracket_colors` - Custom bracket colors, or None for theme defaults
    pub fn configure_highlights(
        &mut self,
        search_matches: Option<Vec<(usize, usize)>>,
        current_match: usize,
        scroll_to_match: bool,
        bracket_matching: bool,
        bracket_colors: Option<(Color32, Color32)>,
    ) {
        if let Some(matches) = search_matches {
            self.set_search_matches(matches, current_match, scroll_to_match);
        } else {
            self.clear_search_matches();
        }
        self.bracket_matching_enabled = bracket_matching;
        self.bracket_colors = bracket_colors;
    }
}
