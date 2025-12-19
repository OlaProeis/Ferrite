//! Find and Replace functionality for Ferrite
//!
//! This module provides comprehensive search and replace capabilities including:
//! - Real-time incremental search with match highlighting
//! - Case-sensitive, whole word, and regex matching modes
//! - Keyboard navigation (F3/Shift+F3) between matches
//! - Replace and Replace All functionality
//! - Integration with the undo/redo system

// Allow dead code - some methods are for future use or completeness
#![allow(dead_code)]

use eframe::egui::{self, Color32, Key, RichText, Ui, Vec2};
use log::debug;
use regex::Regex;

// ─────────────────────────────────────────────────────────────────────────────
// Find State
// ─────────────────────────────────────────────────────────────────────────────

/// State for find/replace functionality within a tab.
///
/// This struct maintains all the state needed for find/replace operations,
/// including search terms, options, and match positions.
#[derive(Debug, Clone, Default)]
pub struct FindState {
    /// Current search term
    pub search_term: String,
    /// Current replacement text
    pub replace_term: String,
    /// Whether search is case-sensitive
    pub case_sensitive: bool,
    /// Whether to match whole words only
    pub whole_word: bool,
    /// Whether to use regex matching
    pub use_regex: bool,
    /// Current match index (0-indexed)
    pub current_match: usize,
    /// All matches as (start, end) byte positions
    pub matches: Vec<(usize, usize)>,
    /// Whether replace mode is active (vs. find-only)
    pub is_replace_mode: bool,
    /// Cached regex (to avoid recompilation)
    #[allow(dead_code)]
    cached_regex: Option<Regex>,
    /// Last search term used to build cache
    #[allow(dead_code)]
    last_search_term: String,
}

impl FindState {
    /// Create a new FindState.
    pub fn new() -> Self {
        Self::default()
    }

    /// Find all matches in the given text.
    ///
    /// Updates `self.matches` with the positions of all matches.
    /// Returns the number of matches found.
    pub fn find_matches(&mut self, text: &str) -> usize {
        self.matches.clear();

        if self.search_term.is_empty() {
            return 0;
        }

        if self.use_regex {
            self.find_regex_matches(text);
        } else {
            self.find_literal_matches(text);
        }

        // Clamp current_match to valid range
        if !self.matches.is_empty() && self.current_match >= self.matches.len() {
            self.current_match = 0;
        }

        self.matches.len()
    }

    /// Find literal (non-regex) matches.
    fn find_literal_matches(&mut self, text: &str) {
        let search_text = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        let search_term = if self.case_sensitive {
            self.search_term.clone()
        } else {
            self.search_term.to_lowercase()
        };

        let mut start = 0;
        while let Some(pos) = search_text[start..].find(&search_term) {
            let match_start = start + pos;
            let match_end = match_start + self.search_term.len();

            // Check whole word boundary if enabled
            if self.whole_word {
                let is_start_boundary = match_start == 0
                    || !text[..match_start]
                        .chars()
                        .last()
                        .map(|c| c.is_alphanumeric() || c == '_')
                        .unwrap_or(false);

                let is_end_boundary = match_end >= text.len()
                    || !text[match_end..]
                        .chars()
                        .next()
                        .map(|c| c.is_alphanumeric() || c == '_')
                        .unwrap_or(false);

                if !is_start_boundary || !is_end_boundary {
                    start = match_start + 1;
                    continue;
                }
            }

            self.matches.push((match_start, match_end));
            start = match_end;
        }
    }

    /// Find regex matches.
    fn find_regex_matches(&mut self, text: &str) {
        let pattern = if self.case_sensitive {
            self.search_term.clone()
        } else {
            format!("(?i){}", self.search_term)
        };

        // Apply whole word boundaries if needed
        let pattern = if self.whole_word {
            format!(r"\b{}\b", pattern)
        } else {
            pattern
        };

        match Regex::new(&pattern) {
            Ok(re) => {
                for m in re.find_iter(text) {
                    self.matches.push((m.start(), m.end()));
                }
            }
            Err(e) => {
                debug!("Invalid regex pattern '{}': {}", self.search_term, e);
                // Invalid regex - return no matches
            }
        }
    }

    /// Move to the next match.
    ///
    /// Returns the new current match index, or None if no matches.
    pub fn next_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match = (self.current_match + 1) % self.matches.len();
        Some(self.current_match)
    }

    /// Move to the previous match.
    ///
    /// Returns the new current match index, or None if no matches.
    pub fn prev_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match = if self.current_match == 0 {
            self.matches.len() - 1
        } else {
            self.current_match - 1
        };
        Some(self.current_match)
    }

    /// Get the current match position.
    ///
    /// Returns (start, end) byte positions or None if no matches.
    pub fn current_match_position(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current_match).copied()
    }

    /// Replace the current match in the text.
    ///
    /// Returns the new text if a replacement was made, or None if no current match.
    pub fn replace_current(&self, text: &str) -> Option<String> {
        let (start, end) = self.current_match_position()?;

        let mut new_text = String::with_capacity(text.len());
        new_text.push_str(&text[..start]);
        new_text.push_str(&self.replace_term);
        new_text.push_str(&text[end..]);

        Some(new_text)
    }

    /// Replace all matches in the text.
    ///
    /// Returns the new text with all replacements made.
    pub fn replace_all(&self, text: &str) -> String {
        if self.matches.is_empty() {
            return text.to_string();
        }

        let mut new_text = String::with_capacity(text.len());
        let mut last_end = 0;

        for &(start, end) in &self.matches {
            new_text.push_str(&text[last_end..start]);
            new_text.push_str(&self.replace_term);
            last_end = end;
        }

        new_text.push_str(&text[last_end..]);
        new_text
    }

    /// Clear all matches and reset state.
    pub fn clear(&mut self) {
        self.matches.clear();
        self.current_match = 0;
    }

    /// Check if there are any matches.
    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty()
    }

    /// Get the total number of matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Find/Replace Panel
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the FindReplacePanel.
#[derive(Debug, Clone, Default)]
pub struct FindReplacePanelOutput {
    /// Whether the search term or options changed (need to re-search)
    pub search_changed: bool,
    /// Whether to move to next match
    pub next_requested: bool,
    /// Whether to move to previous match
    pub prev_requested: bool,
    /// Whether to replace current match
    pub replace_requested: bool,
    /// Whether to replace all matches
    pub replace_all_requested: bool,
    /// Whether to close the panel
    pub close_requested: bool,
}

/// A floating find/replace panel for the editor.
///
/// This panel provides a modern search interface with:
/// - Search input with real-time incremental search
/// - Replace input (in replace mode)
/// - Match counter showing current/total
/// - Navigation buttons (Next/Previous)
/// - Replace/Replace All buttons
/// - Option toggles (Case, Word, Regex)
pub struct FindReplacePanel {
    /// Whether the search input should be focused
    focus_search: bool,
}

impl Default for FindReplacePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FindReplacePanel {
    /// Create a new find/replace panel.
    pub fn new() -> Self {
        Self { focus_search: true }
    }

    /// Request focus on the search input.
    pub fn request_focus(&mut self) {
        self.focus_search = true;
    }

    /// Show the find/replace panel.
    ///
    /// Returns output indicating any actions to perform.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        find_state: &mut FindState,
        is_dark: bool,
    ) -> FindReplacePanelOutput {
        let mut output = FindReplacePanelOutput::default();

        // Panel colors
        let panel_bg = if is_dark {
            Color32::from_rgb(45, 45, 45)
        } else {
            Color32::from_rgb(250, 250, 250)
        };

        let border_color = if is_dark {
            Color32::from_rgb(70, 70, 70)
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(30, 30, 30)
        };

        let muted_color = if is_dark {
            Color32::from_rgb(140, 140, 140)
        } else {
            Color32::from_rgb(120, 120, 120)
        };

        let accent_color = if is_dark {
            Color32::from_rgb(100, 180, 255)
        } else {
            Color32::from_rgb(0, 120, 212)
        };

        // Panel frame
        let frame = egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .rounding(egui::Rounding::same(6.0))
            .shadow(egui::epaint::Shadow {
                offset: egui::vec2(0.0, 2.0),
                blur: 8.0,
                spread: 0.0,
                color: Color32::from_black_alpha(40),
            });

        // Show as floating window at top of screen
        egui::Window::new("Find and Replace")
            .id(egui::Id::new("find_replace_panel"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_min_width(450.0);

                // Handle keyboard shortcuts within the panel
                let input = ui.input(|i| {
                    (
                        i.key_pressed(Key::Escape),
                        i.key_pressed(Key::Enter),
                        i.key_pressed(Key::F3) && !i.modifiers.shift,
                        i.key_pressed(Key::F3) && i.modifiers.shift,
                        i.modifiers.ctrl && i.key_pressed(Key::H),
                    )
                });

                let (escape, enter, f3_next, f3_prev, ctrl_h) = input;

                if escape {
                    output.close_requested = true;
                }
                if enter || f3_next {
                    output.next_requested = true;
                }
                if f3_prev {
                    output.prev_requested = true;
                }
                if ctrl_h {
                    find_state.is_replace_mode = !find_state.is_replace_mode;
                }

                // Header row with close button
                ui.horizontal(|ui| {
                    let title = if find_state.is_replace_mode {
                        "Find and Replace"
                    } else {
                        "Find"
                    };
                    ui.label(RichText::new(title).size(14.0).color(text_color).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Close button
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(16.0).color(muted_color))
                                    .frame(false),
                            )
                            .on_hover_text("Close (Escape)")
                            .clicked()
                        {
                            output.close_requested = true;
                        }

                        // Toggle replace mode button
                        let mode_icon = if find_state.is_replace_mode {
                            "⇅"
                        } else {
                            "⇄"
                        };
                        let mode_tooltip = if find_state.is_replace_mode {
                            "Hide Replace (Ctrl+H)"
                        } else {
                            "Show Replace (Ctrl+H)"
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(mode_icon).size(14.0).color(muted_color),
                                )
                                .frame(false),
                            )
                            .on_hover_text(mode_tooltip)
                            .clicked()
                        {
                            find_state.is_replace_mode = !find_state.is_replace_mode;
                        }
                    });
                });

                ui.add_space(6.0);

                // Search input row
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(14.0));

                    let search_id = egui::Id::new("find_replace_search_input");
                    let search_response = ui.add_sized(
                        Vec2::new(280.0, 24.0),
                        egui::TextEdit::singleline(&mut find_state.search_term)
                            .id(search_id)
                            .hint_text("Search...")
                            .font(egui::FontId::proportional(13.0)),
                    );

                    // Auto-focus search input
                    if self.focus_search {
                        search_response.request_focus();
                        self.focus_search = false;
                    }

                    if search_response.changed() {
                        output.search_changed = true;
                    }

                    // Match counter
                    let match_text = if find_state.matches.is_empty() {
                        if find_state.search_term.is_empty() {
                            String::new()
                        } else {
                            "No matches".to_string()
                        }
                    } else {
                        format!(
                            "{} of {}",
                            find_state.current_match + 1,
                            find_state.matches.len()
                        )
                    };

                    ui.label(RichText::new(match_text).size(12.0).color(muted_color));
                });

                // Replace input row (if in replace mode)
                if find_state.is_replace_mode {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("↳").size(14.0).color(muted_color));

                        ui.add_sized(
                            Vec2::new(280.0, 24.0),
                            egui::TextEdit::singleline(&mut find_state.replace_term)
                                .hint_text("Replace with...")
                                .font(egui::FontId::proportional(13.0)),
                        );
                    });
                }

                ui.add_space(8.0);

                // Options row
                ui.horizontal(|ui| {
                    // Case sensitive toggle
                    let case_btn = ui.add(toggle_button(
                        "Aa",
                        "Case Sensitive",
                        find_state.case_sensitive,
                        is_dark,
                        accent_color,
                    ));
                    if case_btn.clicked() {
                        find_state.case_sensitive = !find_state.case_sensitive;
                        output.search_changed = true;
                    }

                    ui.add_space(4.0);

                    // Whole word toggle
                    let word_btn = ui.add(toggle_button(
                        "W",
                        "Whole Word",
                        find_state.whole_word,
                        is_dark,
                        accent_color,
                    ));
                    if word_btn.clicked() {
                        find_state.whole_word = !find_state.whole_word;
                        output.search_changed = true;
                    }

                    ui.add_space(4.0);

                    // Regex toggle
                    let regex_btn = ui.add(toggle_button(
                        ".*",
                        "Use Regex",
                        find_state.use_regex,
                        is_dark,
                        accent_color,
                    ));
                    if regex_btn.clicked() {
                        find_state.use_regex = !find_state.use_regex;
                        output.search_changed = true;
                    }

                    ui.add_space(16.0);

                    // Navigation buttons
                    let has_matches = find_state.has_matches();

                    if ui
                        .add_enabled(
                            has_matches,
                            egui::Button::new(RichText::new("◀").size(12.0))
                                .min_size(Vec2::new(28.0, 24.0)),
                        )
                        .on_hover_text("Previous (Shift+F3)")
                        .clicked()
                    {
                        output.prev_requested = true;
                    }

                    if ui
                        .add_enabled(
                            has_matches,
                            egui::Button::new(RichText::new("▶").size(12.0))
                                .min_size(Vec2::new(28.0, 24.0)),
                        )
                        .on_hover_text("Next (F3 or Enter)")
                        .clicked()
                    {
                        output.next_requested = true;
                    }

                    // Replace buttons (if in replace mode)
                    if find_state.is_replace_mode {
                        ui.add_space(8.0);

                        if ui
                            .add_enabled(
                                has_matches,
                                egui::Button::new("Replace").min_size(Vec2::new(60.0, 24.0)),
                            )
                            .on_hover_text("Replace current match")
                            .clicked()
                        {
                            output.replace_requested = true;
                        }

                        if ui
                            .add_enabled(
                                has_matches,
                                egui::Button::new("Replace All").min_size(Vec2::new(80.0, 24.0)),
                            )
                            .on_hover_text("Replace all matches")
                            .clicked()
                        {
                            output.replace_all_requested = true;
                        }
                    }
                });

                // Keyboard hints
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Enter/F3: Next • Shift+F3: Prev • Esc: Close")
                        .size(10.0)
                        .color(muted_color),
                );
            });

        output
    }
}

/// Create a toggle button widget.
fn toggle_button<'a>(
    label: &'a str,
    tooltip: &'a str,
    active: bool,
    is_dark: bool,
    accent_color: Color32,
) -> impl egui::Widget + 'a {
    move |ui: &mut Ui| -> egui::Response {
        let text_color = if active {
            accent_color
        } else if is_dark {
            Color32::from_rgb(160, 160, 160)
        } else {
            Color32::from_rgb(100, 100, 100)
        };

        let bg_color = if active {
            if is_dark {
                Color32::from_rgb(50, 70, 90)
            } else {
                Color32::from_rgb(220, 235, 250)
            }
        } else {
            Color32::TRANSPARENT
        };

        let border_color = if active {
            accent_color
        } else if is_dark {
            Color32::from_rgb(70, 70, 70)
        } else {
            Color32::from_rgb(180, 180, 180)
        };

        let response = ui.add(
            egui::Button::new(RichText::new(label).size(12.0).color(text_color).strong())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .min_size(Vec2::new(28.0, 24.0)),
        );

        response.on_hover_text(tooltip)
    }
}

/// Get highlight colors for search matches.
///
/// Returns (current_match_bg, other_matches_bg).
pub fn get_match_highlight_colors(is_dark: bool) -> (Color32, Color32) {
    if is_dark {
        // Dark theme: bright yellow for current, dim yellow for others
        (
            Color32::from_rgba_unmultiplied(255, 230, 0, 180), // Current match
            Color32::from_rgba_unmultiplied(200, 180, 80, 80), // Other matches
        )
    } else {
        // Light theme: bright yellow for current, pale yellow for others
        (
            Color32::from_rgba_unmultiplied(255, 220, 0, 200), // Current match
            Color32::from_rgba_unmultiplied(255, 255, 150, 150), // Other matches
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // FindState Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_find_state_new() {
        let state = FindState::new();
        assert!(state.search_term.is_empty());
        assert!(state.replace_term.is_empty());
        assert!(!state.case_sensitive);
        assert!(!state.whole_word);
        assert!(!state.use_regex);
        assert_eq!(state.current_match, 0);
        assert!(state.matches.is_empty());
    }

    #[test]
    fn test_find_matches_empty_search() {
        let mut state = FindState::new();
        state.search_term = String::new();
        let count = state.find_matches("Hello, World!");
        assert_eq!(count, 0);
        assert!(state.matches.is_empty());
    }

    #[test]
    fn test_find_matches_basic() {
        let mut state = FindState::new();
        state.search_term = "Hello".to_string();
        let count = state.find_matches("Hello, Hello, Hello!");
        assert_eq!(count, 3);
        assert_eq!(state.matches, vec![(0, 5), (7, 12), (14, 19)]);
    }

    #[test]
    fn test_find_matches_case_insensitive() {
        let mut state = FindState::new();
        state.search_term = "hello".to_string();
        state.case_sensitive = false;
        let count = state.find_matches("Hello, HELLO, hello!");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_find_matches_case_sensitive() {
        let mut state = FindState::new();
        state.search_term = "Hello".to_string();
        state.case_sensitive = true;
        let count = state.find_matches("Hello, HELLO, hello!");
        assert_eq!(count, 1);
        assert_eq!(state.matches, vec![(0, 5)]);
    }

    #[test]
    fn test_find_matches_whole_word() {
        let mut state = FindState::new();
        state.search_term = "test".to_string();
        state.whole_word = true;
        let count = state.find_matches("test testing tested test");
        assert_eq!(count, 2); // Only standalone "test" matches
        assert_eq!(state.matches, vec![(0, 4), (20, 24)]);
    }

    #[test]
    fn test_find_matches_regex() {
        let mut state = FindState::new();
        state.search_term = r"\d+".to_string();
        state.use_regex = true;
        let count = state.find_matches("abc123def456ghi");
        assert_eq!(count, 2);
        assert_eq!(state.matches, vec![(3, 6), (9, 12)]);
    }

    #[test]
    fn test_find_matches_regex_invalid() {
        let mut state = FindState::new();
        state.search_term = r"[invalid".to_string(); // Invalid regex
        state.use_regex = true;
        let count = state.find_matches("test text");
        assert_eq!(count, 0); // Invalid regex returns no matches
    }

    #[test]
    fn test_next_match() {
        let mut state = FindState::new();
        state.search_term = "x".to_string();
        state.find_matches("axbxcx");
        assert_eq!(state.current_match, 0);

        state.next_match();
        assert_eq!(state.current_match, 1);

        state.next_match();
        assert_eq!(state.current_match, 2);

        state.next_match();
        assert_eq!(state.current_match, 0); // Wraps around
    }

    #[test]
    fn test_prev_match() {
        let mut state = FindState::new();
        state.search_term = "x".to_string();
        state.find_matches("axbxcx");
        assert_eq!(state.current_match, 0);

        state.prev_match();
        assert_eq!(state.current_match, 2); // Wraps to end

        state.prev_match();
        assert_eq!(state.current_match, 1);
    }

    #[test]
    fn test_next_prev_no_matches() {
        let mut state = FindState::new();
        assert!(state.next_match().is_none());
        assert!(state.prev_match().is_none());
    }

    #[test]
    fn test_current_match_position() {
        let mut state = FindState::new();
        state.search_term = "world".to_string();
        state.find_matches("hello world wide world");

        assert_eq!(state.current_match_position(), Some((6, 11)));

        state.next_match();
        assert_eq!(state.current_match_position(), Some((17, 22)));
    }

    #[test]
    fn test_replace_current() {
        let mut state = FindState::new();
        state.search_term = "world".to_string();
        state.replace_term = "universe".to_string();
        state.find_matches("hello world!");

        let result = state.replace_current("hello world!");
        assert_eq!(result, Some("hello universe!".to_string()));
    }

    #[test]
    fn test_replace_current_no_match() {
        let state = FindState::new();
        let result = state.replace_current("hello world!");
        assert!(result.is_none());
    }

    #[test]
    fn test_replace_all() {
        let mut state = FindState::new();
        state.search_term = "a".to_string();
        state.replace_term = "X".to_string();
        state.find_matches("abracadabra");

        let result = state.replace_all("abracadabra");
        assert_eq!(result, "XbrXcXdXbrX");
    }

    #[test]
    fn test_replace_all_no_matches() {
        let state = FindState::new();
        let result = state.replace_all("hello world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_replace_all_empty_replacement() {
        let mut state = FindState::new();
        state.search_term = " ".to_string();
        state.replace_term = String::new();
        state.find_matches("hello world");

        let result = state.replace_all("hello world");
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn test_clear() {
        let mut state = FindState::new();
        state.search_term = "test".to_string();
        state.find_matches("test test test");
        state.next_match();

        state.clear();
        assert!(state.matches.is_empty());
        assert_eq!(state.current_match, 0);
    }

    #[test]
    fn test_has_matches() {
        let mut state = FindState::new();
        assert!(!state.has_matches());

        state.search_term = "test".to_string();
        state.find_matches("test");
        assert!(state.has_matches());
    }

    #[test]
    fn test_match_count() {
        let mut state = FindState::new();
        assert_eq!(state.match_count(), 0);

        state.search_term = "o".to_string();
        state.find_matches("hello world");
        assert_eq!(state.match_count(), 2);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FindReplacePanel Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_panel_new() {
        let panel = FindReplacePanel::new();
        assert!(panel.focus_search);
    }

    #[test]
    fn test_panel_default() {
        let panel = FindReplacePanel::default();
        assert!(panel.focus_search);
    }

    #[test]
    fn test_panel_request_focus() {
        let mut panel = FindReplacePanel::new();
        panel.focus_search = false;
        panel.request_focus();
        assert!(panel.focus_search);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Color Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_highlight_colors_dark() {
        let (current, other) = get_match_highlight_colors(true);
        // Current match should be brighter/more opaque
        assert!(current.a() > other.a());
    }

    #[test]
    fn test_highlight_colors_light() {
        let (current, other) = get_match_highlight_colors(false);
        // Current match should be more visible
        assert!(current.a() > other.a());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_find_unicode() {
        let mut state = FindState::new();
        state.search_term = "🎉".to_string();
        let count = state.find_matches("Hello 🎉 World 🎉!");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_find_multiline() {
        let mut state = FindState::new();
        state.search_term = "line".to_string();
        let text = "first line\nsecond line\nthird line";
        let count = state.find_matches(text);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_replace_all_multiline() {
        let mut state = FindState::new();
        state.search_term = "\n".to_string();
        state.replace_term = " | ".to_string();
        state.find_matches("a\nb\nc");

        let result = state.replace_all("a\nb\nc");
        assert_eq!(result, "a | b | c");
    }

    #[test]
    fn test_whole_word_with_underscore() {
        let mut state = FindState::new();
        state.search_term = "test".to_string();
        state.whole_word = true;
        // Underscores are considered part of the word
        let count = state.find_matches("test test_case _test test");
        assert_eq!(count, 2); // Only first and last "test"
    }

    #[test]
    fn test_regex_whole_word() {
        let mut state = FindState::new();
        state.search_term = "test".to_string();
        state.use_regex = true;
        state.whole_word = true;
        let count = state.find_matches("test testing tested test");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_replace_current_second_match() {
        let mut state = FindState::new();
        state.search_term = "foo".to_string();
        state.replace_term = "bar".to_string();
        state.find_matches("foo and foo");
        state.next_match(); // Move to second match

        let result = state.replace_current("foo and foo");
        assert_eq!(result, Some("foo and bar".to_string()));
    }

    #[test]
    fn test_find_overlapping_not_supported() {
        // Note: Our implementation doesn't find overlapping matches
        let mut state = FindState::new();
        state.search_term = "aa".to_string();
        let count = state.find_matches("aaaa");
        // Standard behavior: non-overlapping matches
        assert_eq!(count, 2);
        assert_eq!(state.matches, vec![(0, 2), (2, 4)]);
    }

    #[test]
    fn test_current_match_clamp_after_reearch() {
        let mut state = FindState::new();
        state.search_term = "x".to_string();
        state.find_matches("xxxxx"); // 5 matches
        state.current_match = 4; // Go to last match

        // Search for something with fewer matches
        state.search_term = "y".to_string();
        state.find_matches("xy"); // 1 match
                                  // current_match should be clamped to valid range
        assert_eq!(state.current_match, 0);
    }
}
