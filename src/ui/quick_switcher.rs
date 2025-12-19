//! Quick file switcher with fuzzy search for workspace mode.
//!
//! Provides a Ctrl+P fuzzy file finder overlay that allows quick navigation
//! to files within the current workspace.

// Allow clippy lints:
// - collapsible_if: Nested if statements are clearer for key handling logic
// - ptr_arg: Using &PathBuf for consistency with PathBuf file icons
#![allow(clippy::collapsible_if)]
#![allow(clippy::ptr_arg)]

use eframe::egui::{self, Color32, Key, RichText, Sense};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::path::PathBuf;

/// Maximum number of results to show in the quick switcher.
const MAX_RESULTS: usize = 15;

/// Output from the quick switcher.
#[derive(Debug, Default)]
pub struct QuickSwitcherOutput {
    /// File selected by the user (should be opened)
    pub selected_file: Option<PathBuf>,
    /// Whether the quick switcher was closed (Escape or click outside)
    pub closed: bool,
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
        egui::Area::new(egui::Id::new("quick_switcher_overlay"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .rounding(8.0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0.0, 4.0].into(),
                        blur: 12.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_width(500.0);

                        ui.add_space(8.0);

                        // Search input
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new("🔍").size(16.0));
                            ui.add_space(4.0);

                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("Search files...")
                                    .frame(false)
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

                        // Results list
                        if results.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new("No matching files")
                                        .color(secondary_color)
                                        .italics(),
                                );
                            });
                            ui.add_space(8.0);
                        } else {
                            for (idx, result) in results.iter().enumerate() {
                                let is_selected = idx == self.selected_index;

                                let response = ui
                                    .horizontal(|ui| {
                                        let row_response = ui.interact(
                                            ui.available_rect_before_wrap(),
                                            ui.id().with(idx),
                                            Sense::click(),
                                        );

                                        // Draw background
                                        if is_selected {
                                            ui.painter().rect_filled(
                                                row_response.rect.expand2(egui::vec2(8.0, 2.0)),
                                                4.0,
                                                selected_bg,
                                            );
                                        } else if row_response.hovered() {
                                            ui.painter().rect_filled(
                                                row_response.rect.expand2(egui::vec2(8.0, 2.0)),
                                                4.0,
                                                hover_bg,
                                            );
                                        }

                                        ui.add_space(16.0);

                                        // File icon
                                        let icon = self.file_icon(&result.path);
                                        ui.label(RichText::new(icon).size(14.0));

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

                                        // Recent indicator
                                        if result.is_recent {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.add_space(16.0);
                                                    ui.label(
                                                        RichText::new("⏱")
                                                            .color(secondary_color)
                                                            .size(12.0),
                                                    )
                                                    .on_hover_text("Recently opened");
                                                },
                                            );
                                        }

                                        row_response
                                    })
                                    .inner;

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
                                RichText::new("↑↓ Navigate  ⏎ Open  Esc Close")
                                    .color(secondary_color)
                                    .small(),
                            );
                        });
                        ui.add_space(6.0);
                    });
            });

        if output.closed {
            self.close();
        }

        output
    }

    /// Filter and score files based on the current query.
    fn filter_files(
        &self,
        all_files: &[PathBuf],
        recent_files: &[PathBuf],
        workspace_root: &PathBuf,
    ) -> Vec<QuickSwitcherResult> {
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

        // Score all files
        let mut scored: Vec<(PathBuf, i64, bool)> = Vec::new();

        for path in all_files {
            let display = path
                .strip_prefix(workspace_root)
                .unwrap_or(path)
                .to_string_lossy();

            if let Some(score) = self.matcher.fuzzy_match(&display, &self.query) {
                let is_recent = recent_files.contains(path);
                // Boost recent files
                let boosted_score = if is_recent { score + 100 } else { score };
                scored.push((path.clone(), boosted_score, is_recent));
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
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext.to_lowercase().as_str() {
            "md" | "markdown" | "mdown" | "mkd" => "📝",
            "txt" | "text" => "📄",
            "rs" => "🦀",
            "js" | "jsx" | "ts" | "tsx" => "📜",
            "json" => "📋",
            "toml" | "yaml" | "yml" => "⚙️",
            "html" | "htm" => "🌐",
            "css" | "scss" | "sass" => "🎨",
            "py" => "🐍",
            "go" => "🐹",
            "java" | "kt" | "kts" => "☕",
            "c" | "cpp" | "h" | "hpp" => "⚡",
            "sh" | "bash" | "zsh" => "💻",
            _ => "📄",
        }
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
}
