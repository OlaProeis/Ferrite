//! Quick file switcher with fuzzy search for workspace mode.
//!
//! Provides a Ctrl+P fuzzy file finder overlay that allows quick navigation
//! to files within the current workspace.

// Allow clippy lints:
// - collapsible_if: Nested if statements are clearer for key handling logic
// - ptr_arg: Using &PathBuf for consistency with PathBuf file icons
#![allow(clippy::collapsible_if)]
#![allow(clippy::ptr_arg)]

use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::{self, MAGNIFYING_GLASS, TIMER};
use crate::workspaces::FileIndexProgress;
use eframe::egui::{self, Color32, Key, LayerId, Order, RichText, Sense};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rust_i18n::t;
use std::path::PathBuf;

/// Maximum number of results to show in the quick switcher.
const MAX_RESULTS: usize = 15;

/// Treat common filename/path separators as spaces so `tables` matches `test-tables.md`.
fn normalize_for_search(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = true;

    for c in s.chars() {
        let is_separator = matches!(c, '-' | '_' | '.' | ' ' | '/' | '\\');
        if is_separator {
            if !prev_was_space {
                result.push(' ');
            }
            prev_was_space = true;
        } else {
            result.push(c.to_ascii_lowercase());
            prev_was_space = false;
        }
    }

    result
}

/// Whether two path tokens are close enough to treat as a match (minor typos).
fn tokens_similar(word: &str, query: &str) -> bool {
    if word.eq_ignore_ascii_case(query) {
        return true;
    }
    if word.len() < 3 || query.len() < 3 {
        return false;
    }
    if word.len().abs_diff(query.len()) > 2 {
        return false;
    }
    damerau_levenshtein_at_most(word, query, 2)
}

/// Damerau-Levenshtein distance with early exit when above `max`.
fn damerau_levenshtein_at_most(a: &str, b: &str, max: usize) -> bool {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    let la = a.len();
    let lb = b.len();

    if la.abs_diff(lb) > max {
        return false;
    }
    if la == 0 {
        return lb <= max;
    }
    if lb == 0 {
        return la <= max;
    }

    let mut prev_prev = vec![0usize; lb + 1];
    let mut prev = (0..=lb).collect::<Vec<_>>();
    let mut curr = vec![0usize; lb + 1];

    for i in 1..=la {
        curr[0] = i;
        let mut row_min = curr[0];

        for j in 1..=lb {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut dist = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);

            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                dist = dist.min(prev_prev[j - 2] + 1);
            }

            curr[j] = dist;
            row_min = row_min.min(dist);
        }

        if row_min > max {
            return false;
        }

        std::mem::swap(&mut prev_prev, &mut prev);
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[lb] <= max
}

/// Score how well a single path token matches the query. Higher is better.
fn score_token_match(word: &str, query: &str, matcher: &SkimMatcherV2) -> Option<i64> {
    if word.is_empty() || query.is_empty() {
        return None;
    }

    if word == query {
        return Some(1000);
    }
    if word.starts_with(query) {
        let length_bonus = 50_i64.saturating_sub((word.len() - query.len()) as i64);
        return Some(800 + length_bonus);
    }
    if query.len() >= 3 && word.contains(query) {
        return Some(600);
    }
    if query.len() >= 4 && tokens_similar(word, query) {
        return Some(400);
    }

    matcher.fuzzy_match(word, query).map(|score| score + 50)
}

/// Score a workspace-relative path for the quick switcher. `None` means no match.
fn score_path_match(path_display: &str, query: &str, matcher: &SkimMatcherV2) -> Option<i64> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }

    let norm_path = normalize_for_search(path_display);
    let norm_query = normalize_for_search(query);
    if norm_query.is_empty() {
        return None;
    }

    let query_words: Vec<&str> = norm_query.split_whitespace().collect();
    let path_tokens: Vec<&str> = norm_path.split_whitespace().collect();

    if query_words.is_empty() || path_tokens.is_empty() {
        return None;
    }

    // Multi-word query: every word must match some token.
    if query_words.len() > 1 {
        let mut total = 0_i64;
        for qw in &query_words {
            let best = path_tokens
                .iter()
                .filter_map(|token| score_token_match(token, qw, matcher))
                .max()?;
            total += best;
        }
        return Some(total);
    }

    let qw = query_words[0];
    path_tokens
        .iter()
        .filter_map(|token| score_token_match(token, qw, matcher))
        .max()
}

/// Output from the quick switcher.
#[derive(Debug, Default)]
pub struct QuickSwitcherOutput {
    /// File selected by the user (should be opened)
    pub selected_file: Option<PathBuf>,
    /// Whether the quick switcher was closed (Escape or click outside)
    pub closed: bool,
    /// Screen rect of the overlay panel (for native WebView occlusion).
    pub screen_rect: Option<egui::Rect>,
}

/// Quick file switcher state.
pub struct QuickSwitcher {
    /// Whether the quick switcher is open
    is_open: bool,
    /// Current search query
    query: String,
    /// Currently selected result index
    selected_index: usize,
    /// Fuzzy matcher
    matcher: SkimMatcherV2,
}

impl Default for QuickSwitcher {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickSwitcher {
    /// Create a new quick switcher.
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Check if the quick switcher is currently open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Open the quick switcher.
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Close the quick switcher.
    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Toggle the quick switcher visibility.
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Render the quick switcher and return any output.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        all_files: &[PathBuf],
        recent_files: &[PathBuf],
        workspace_root: &PathBuf,
        is_dark: bool,
        index_progress: Option<FileIndexProgress>,
    ) -> QuickSwitcherOutput {
        let mut output = QuickSwitcherOutput::default();

        if !self.is_open {
            return output;
        }

        // Filter and score files based on query
        let results = self.filter_files(all_files, recent_files, workspace_root);

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(35, 35, 40)
        } else {
            Color32::from_rgb(255, 255, 255)
        };

        let border_color = if is_dark {
            Color32::from_rgb(80, 80, 90)
        } else {
            Color32::from_rgb(180, 180, 190)
        };

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let secondary_color = if is_dark {
            Color32::from_rgb(140, 140, 150)
        } else {
            Color32::from_rgb(100, 100, 110)
        };

        let selected_bg = if is_dark {
            Color32::from_rgb(55, 65, 85)
        } else {
            Color32::from_rgb(220, 230, 245)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(45, 50, 60)
        } else {
            Color32::from_rgb(235, 240, 248)
        };

        // Handle keyboard shortcuts while open
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                output.closed = true;
            }
            if i.key_pressed(Key::ArrowDown) && !results.is_empty() {
                self.selected_index = (self.selected_index + 1) % results.len();
            }
            if i.key_pressed(Key::ArrowUp) && !results.is_empty() {
                self.selected_index = if self.selected_index == 0 {
                    results.len() - 1
                } else {
                    self.selected_index - 1
                };
            }
            if i.key_pressed(Key::Enter) {
                if let Some(result) = results.get(self.selected_index) {
                    output.selected_file = Some(result.path.clone());
                    output.closed = true;
                }
            }
        });

        // Show the overlay
        let area_response = egui::Area::new(egui::Id::new("quick_switcher_overlay"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .corner_radius(8)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_width(500.0);

                        ui.add_space(8.0);

                        // Search input
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(phosphor_rich_text(MAGNIFYING_GLASS, 16.0));
                            ui.add_space(4.0);

                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text(t!("quick_switcher.placeholder"))
                                    .frame(egui::Frame::NONE)
                                    .desired_width(450.0)
                                    .font(egui::TextStyle::Body),
                            );

                            // Auto-focus the input
                            response.request_focus();

                            // Reset selection when query changes
                            if response.changed() {
                                self.selected_index = 0;
                            }

                            ui.add_space(8.0);
                        });

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if let Some(progress) = index_progress {
                            crate::ui::file_index_progress_ui(ui, progress, secondary_color);
                            ui.add_space(4.0);
                        }

                        // Results list
                        if results.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new(t!("quick_switcher.no_results"))
                                        .color(secondary_color)
                                        .italics(),
                                );
                            });
                            ui.add_space(8.0);
                        } else {
                            for (idx, result) in results.iter().enumerate() {
                                let is_selected = idx == self.selected_index;

                                // Draw content first with horizontal layout
                                let row_response = ui
                                    .horizontal(|ui| {
                                        ui.add_space(16.0);

                                        // File icon
                                        let icon = self.file_icon(&result.path);
                                        ui.label(phosphor_rich_text(icon, 14.0));

                                        ui.add_space(8.0);

                                        // File name
                                        ui.label(
                                            RichText::new(&result.display_name)
                                                .color(text_color)
                                                .strong(),
                                        );

                                        // Relative path
                                        if !result.relative_path.is_empty()
                                            && result.relative_path != result.display_name
                                        {
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new(&result.relative_path)
                                                    .color(secondary_color)
                                                    .small(),
                                            );
                                        }

                                        // Recent indicator (right-aligned)
                                        if result.is_recent {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.add_space(16.0);
                                                    ui.label(
                                                        phosphor_rich_text(TIMER, 12.0)
                                                            .color(secondary_color),
                                                    )
                                                    .on_hover_text(t!(
                                                        "quick_switcher.recent_tooltip"
                                                    ));
                                                },
                                            );
                                        }
                                    })
                                    .response;

                                // Create clickable interaction over the entire row
                                // This is placed AFTER content so it captures all clicks
                                let row_rect = row_response.rect.expand2(egui::vec2(8.0, 2.0));
                                let response = ui.interact(
                                    row_rect,
                                    ui.id().with(("row_click", idx)),
                                    Sense::click(),
                                );

                                // Sync selection with hover for consistent mouse support
                                if response.hovered() {
                                    self.selected_index = idx;
                                }

                                // Draw background behind content using background layer
                                let show_highlight = is_selected || response.hovered();
                                if show_highlight {
                                    // Paint to background layer so it appears behind text
                                    let bg_layer = LayerId::new(
                                        Order::Background,
                                        ui.id().with(("row_bg", idx)),
                                    );
                                    ui.ctx().layer_painter(bg_layer).rect_filled(
                                        row_rect,
                                        4.0,
                                        if is_selected { selected_bg } else { hover_bg },
                                    );
                                }

                                // Handle click to open file
                                if response.clicked() {
                                    output.selected_file = Some(result.path.clone());
                                    output.closed = true;
                                }

                                ui.add_space(2.0);
                            }
                            ui.add_space(4.0);
                        }

                        // Keyboard hints
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(t!("quick_switcher.keyboard_hints"))
                                    .color(secondary_color)
                                    .small(),
                            );
                        });
                        ui.add_space(6.0);
                    });
            });
        let area_id = egui::Id::new("quick_switcher_overlay");
        output.screen_rect = ctx.memory(|m| m.area_rect(area_id)).or_else(|| {
            Some(crate::markdown::video_render::egui_rect_to_screen(
                ctx,
                area_response.response.layer_id,
                area_response.response.rect,
            ))
        });
        if let Some(rect) = output.screen_rect {
            output.screen_rect = Some(
                rect.expand(crate::markdown::video_render::VIDEO_OCCLUDER_MARGIN),
            );
        }

        if output.closed {
            self.close();
        }

        output
    }

    /// Paths to search: indexed tree files plus recent files (may include unexpanded folders).
    fn search_paths(all_files: &[PathBuf], recent_files: &[PathBuf]) -> Vec<PathBuf> {
        use std::collections::HashSet;
        let mut seen: HashSet<&PathBuf> =
            HashSet::with_capacity(all_files.len() + recent_files.len());
        let mut paths: Vec<PathBuf> = Vec::with_capacity(all_files.len() + recent_files.len());
        for path in all_files {
            if seen.insert(path) {
                paths.push(path.clone());
            }
        }
        for path in recent_files {
            if seen.insert(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// Filter and score files based on the current query.
    fn filter_files(
        &self,
        all_files: &[PathBuf],
        recent_files: &[PathBuf],
        workspace_root: &PathBuf,
    ) -> Vec<QuickSwitcherResult> {
        let _diag = crate::diag::SlowScope::new("quick_switcher::filter_files", 16);
        let mut results: Vec<QuickSwitcherResult> = Vec::new();

        // If query is empty, show recent files first, then other files
        if self.query.is_empty() {
            // Add recent files first
            for path in recent_files.iter().take(MAX_RESULTS) {
                if path.exists() {
                    results.push(QuickSwitcherResult::new(
                        path.clone(),
                        workspace_root,
                        true,
                        0,
                    ));
                }
            }

            // Fill remaining slots with other files
            let remaining = MAX_RESULTS.saturating_sub(results.len());
            for path in all_files.iter().take(remaining * 2) {
                if !results.iter().any(|r| r.path == *path) {
                    results.push(QuickSwitcherResult::new(
                        path.clone(),
                        workspace_root,
                        false,
                        0,
                    ));
                    if results.len() >= MAX_RESULTS {
                        break;
                    }
                }
            }

            return results;
        }

        // Score indexed + recent files (recent may include paths not yet loaded in the tree).
        let mut scored: Vec<(PathBuf, i64, bool)> = Vec::new();
        let search_paths = Self::search_paths(all_files, recent_files);
        if crate::diag::enabled() && search_paths.len() > 500 {
            crate::diag::event(
                "quick_switcher_large_scan",
                format!(
                    "scoring {} paths (query {:?})",
                    search_paths.len(),
                    self.query
                ),
            );
        }

        for path in search_paths {
            let relative = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy();

            let mut score = score_path_match(&relative, &self.query, &self.matcher);
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(name_score) = score_path_match(name, &self.query, &self.matcher) {
                    score = Some(score.map_or(name_score, |s| s.max(name_score)));
                }
            }

            if let Some(score) = score {
                let is_recent = recent_files.contains(&path);
                // Small recent tiebreaker only — must not swamp match quality.
                let boosted_score = if is_recent { score + 5 } else { score };
                scored.push((path, boosted_score, is_recent));
            }
        }

        // Sort by score (descending)
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top results
        for (path, score, is_recent) in scored.into_iter().take(MAX_RESULTS) {
            results.push(QuickSwitcherResult::new(
                path,
                workspace_root,
                is_recent,
                score,
            ));
        }

        results
    }

    /// Get an icon for a file based on its extension.
    fn file_icon(&self, path: &PathBuf) -> &'static str {
        phosphor_icons::file_icon_for_path(path)
    }
}

/// A single result in the quick switcher.
struct QuickSwitcherResult {
    /// Full path to the file
    path: PathBuf,
    /// Display name (filename)
    display_name: String,
    /// Relative path from workspace root
    relative_path: String,
    /// Whether this is a recently opened file
    is_recent: bool,
    /// Fuzzy match score (for debugging)
    #[allow(dead_code)]
    score: i64,
}

impl QuickSwitcherResult {
    fn new(path: PathBuf, workspace_root: &PathBuf, is_recent: bool, score: i64) -> Self {
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let relative_path = path
            .strip_prefix(workspace_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        Self {
            path,
            display_name,
            relative_path,
            is_recent,
            score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_switcher_new() {
        let switcher = QuickSwitcher::new();
        assert!(!switcher.is_open());
    }

    #[test]
    fn test_quick_switcher_toggle() {
        let mut switcher = QuickSwitcher::new();
        assert!(!switcher.is_open());

        switcher.toggle();
        assert!(switcher.is_open());

        switcher.toggle();
        assert!(!switcher.is_open());
    }

    #[test]
    fn test_quick_switcher_open_close() {
        let mut switcher = QuickSwitcher::new();

        switcher.open();
        assert!(switcher.is_open());

        switcher.close();
        assert!(!switcher.is_open());
    }

    #[test]
    fn test_quick_switcher_result() {
        let path = PathBuf::from("/workspace/src/main.rs");
        let root = PathBuf::from("/workspace");
        let result = QuickSwitcherResult::new(path.clone(), &root, true, 100);

        assert_eq!(result.path, path);
        assert_eq!(result.display_name, "main.rs");
        assert_eq!(result.relative_path, "src/main.rs");
        assert!(result.is_recent);
    }

    #[test]
    fn test_normalize_for_search_replaces_separators() {
        assert_eq!(normalize_for_search("test-tabels.md"), "test tabels md");
        assert_eq!(normalize_for_search("my_cool_file"), "my cool file");
        assert_eq!(normalize_for_search("a--b___c"), "a b c");
        assert_eq!(
            normalize_for_search("test_md\\test_box_drawing.md"),
            "test md test box drawing md"
        );
    }

    #[test]
    fn test_tokens_similar_allows_minor_typos() {
        assert!(tokens_similar("tabels", "tables"));
        assert!(tokens_similar("tables", "tabels"));
        assert!(!tokens_similar("totally", "tables"));
    }

    #[test]
    fn test_score_path_match_finds_dash_separated_names() {
        let matcher = SkimMatcherV2::default();
        assert!(score_path_match("docs/test-tables.md", "tables", &matcher).is_some());
        assert!(score_path_match("docs/test-tabels.md", "tables", &matcher).is_some());
        assert!(score_path_match("src/my_module/foo.rs", "my module", &matcher).is_some());
        assert!(score_path_match("notes/quick_start.md", "quick start", &matcher).is_some());
    }

    #[test]
    fn test_score_path_match_still_matches_literal_paths() {
        let matcher = SkimMatcherV2::default();
        assert!(score_path_match("src/main.rs", "main.rs", &matcher).is_some());
    }

    #[test]
    fn test_box_query_prefers_box_drawing() {
        let matcher = SkimMatcherV2::default();
        let box_drawing = score_path_match("test_md\\test_box_drawing.md", "box", &matcher);
        let tables = score_path_match("test_md\\test_tables.md", "box", &matcher);
        let code_blocks =
            score_path_match("test_md\\test_consecutive_code_blocks.md", "box", &matcher);
        let readme = score_path_match("README.md", "box", &matcher);

        assert!(box_drawing.is_some());
        assert!(box_drawing.unwrap() > tables.unwrap_or(0));
        assert!(code_blocks.is_none());
        assert!(readme.is_none());
    }

    #[test]
    fn test_filter_files_searches_recent_when_not_in_tree() {
        let mut switcher = QuickSwitcher::new();
        switcher.query = "box".to_string();

        let box_path = PathBuf::from("test_md/test_box_drawing.md");
        let root = PathBuf::from(".");
        let results = switcher.filter_files(&[], &[box_path.clone()], &root);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, box_path);
    }

    #[test]
    fn test_search_paths_merges_without_duplicates() {
        let a = PathBuf::from("a.md");
        let b = PathBuf::from("b.md");
        let paths = QuickSwitcher::search_paths(&[a.clone(), b.clone()], &[a.clone()]);
        assert_eq!(paths.len(), 2);
    }
}
