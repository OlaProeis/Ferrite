//! Search-in-files panel for workspace mode.
//!
//! Provides Ctrl+Shift+F search functionality across all files in the workspace.

// Allow dead code - includes ID counter and layout helpers for future search UI features
#![allow(dead_code)]

use crate::string_utils::floor_char_boundary;
use eframe::egui::{self, Color32, Key, RichText, ScrollArea, Sense, TextFormat};
use std::path::PathBuf;

/// Maximum number of results to show per file.
const MAX_RESULTS_PER_FILE: usize = 10;

/// Maximum number of files to show results for.
const MAX_FILES_WITH_RESULTS: usize = 50;

/// A single search match result.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// The line number (1-indexed)
    pub line_number: usize,
    /// The line content
    pub line_content: String,
    /// Start position of match in line
    pub match_start: usize,
    /// End position of match in line
    pub match_end: usize,
}

/// Results for a single file.
#[derive(Debug, Clone)]
pub struct FileSearchResults {
    /// Path to the file
    pub path: PathBuf,
    /// Matches in this file
    pub matches: Vec<SearchMatch>,
    /// Whether there are more matches than shown
    pub truncated: bool,
    /// Whether this file's results are expanded in the UI
    pub expanded: bool,
}

/// Output from the search panel.
#[derive(Debug, Default)]
pub struct SearchPanelOutput {
    /// File+line to navigate to (user clicked a result)
    pub navigate_to: Option<(PathBuf, usize)>,
    /// Whether the panel was closed
    pub closed: bool,
    /// Whether search should be triggered
    pub should_search: bool,
}

/// State for the search panel.
pub struct SearchPanel {
    /// Whether the panel is open
    is_open: bool,
    /// Current search query
    query: String,
    /// Previous query (to detect changes)
    last_query: String,
    /// Whether to use regex search
    use_regex: bool,
    /// Whether search is case-sensitive
    case_sensitive: bool,
    /// Current search results
    results: Vec<FileSearchResults>,
    /// Total match count across all files
    total_matches: usize,
    /// Error message if search failed
    error_message: Option<String>,
    /// Counter for unique IDs
    id_counter: usize,
}

impl Default for SearchPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPanel {
    /// Create a new search panel.
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            last_query: String::new(),
            use_regex: false,
            case_sensitive: false,
            results: Vec::new(),
            total_matches: 0,
            error_message: None,
            id_counter: 0,
        }
    }

    /// Check if the panel is open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Open the search panel.
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Close the search panel.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Toggle the search panel.
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Perform search across workspace files.
    pub fn search(&mut self, files: &[PathBuf], hidden_patterns: &[String]) {
        self.results.clear();
        self.total_matches = 0;
        self.error_message = None;
        self.last_query = self.query.clone();

        if self.query.is_empty() {
            return;
        }

        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        // Compile regex if needed
        let regex = if self.use_regex {
            match regex::RegexBuilder::new(&self.query)
                .case_insensitive(!self.case_sensitive)
                .build()
            {
                Ok(r) => Some(r),
                Err(e) => {
                    self.error_message = Some(format!("Invalid regex: {}", e));
                    return;
                }
            }
        } else {
            None
        };

        for file_path in files {
            // Skip hidden files
            let should_skip = file_path.components().any(|comp| {
                if let std::path::Component::Normal(name) = comp {
                    let name_str = name.to_string_lossy();
                    hidden_patterns
                        .iter()
                        .any(|p| name_str.contains(p) || name_str == *p)
                } else {
                    false
                }
            });
            if should_skip {
                continue;
            }

            // Only search text files
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let is_text = matches!(
                ext.to_lowercase().as_str(),
                "md" | "markdown"
                    | "txt"
                    | "rs"
                    | "toml"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "js"
                    | "ts"
                    | "jsx"
                    | "tsx"
                    | "html"
                    | "css"
                    | "scss"
                    | "py"
                    | "go"
                    | "java"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "sh"
                    | "bash"
                    | "zsh"
                    | "xml"
                    | "svg"
            );
            if !is_text {
                continue;
            }

            // Read file content
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue, // Skip unreadable files
            };

            let mut file_results = FileSearchResults {
                path: file_path.clone(),
                matches: Vec::new(),
                truncated: false,
                expanded: true,
            };

            for (line_idx, line) in content.lines().enumerate() {
                let line_number = line_idx + 1;

                let matches_found: Vec<(usize, usize)> = if let Some(ref re) = regex {
                    re.find_iter(line).map(|m| (m.start(), m.end())).collect()
                } else {
                    let search_line = if self.case_sensitive {
                        line.to_string()
                    } else {
                        line.to_lowercase()
                    };
                    let mut positions = Vec::new();
                    let mut start = 0;
                    while let Some(pos) = search_line[start..].find(&query) {
                        let abs_pos = start + pos;
                        positions.push((abs_pos, abs_pos + query.len()));
                        start = abs_pos + 1;
                    }
                    positions
                };

                for (match_start, match_end) in matches_found {
                    if file_results.matches.len() >= MAX_RESULTS_PER_FILE {
                        file_results.truncated = true;
                        break;
                    }

                    file_results.matches.push(SearchMatch {
                        line_number,
                        line_content: line.to_string(),
                        match_start,
                        match_end,
                    });
                    self.total_matches += 1;
                }

                if file_results.truncated {
                    break;
                }
            }

            if !file_results.matches.is_empty() {
                self.results.push(file_results);
            }

            if self.results.len() >= MAX_FILES_WITH_RESULTS {
                break;
            }
        }
    }

    /// Show the search panel.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        workspace_root: &PathBuf,
        is_dark: bool,
    ) -> SearchPanelOutput {
        let mut output = SearchPanelOutput::default();

        if !self.is_open {
            return output;
        }

        // Handle escape key
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            output.closed = true;
            self.close();
            return output;
        }

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(35, 35, 40)
        } else {
            Color32::from_rgb(250, 250, 250)
        };

        let border_color = if is_dark {
            Color32::from_rgb(70, 70, 80)
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

        let highlight_color = if is_dark {
            Color32::from_rgb(255, 220, 100)
        } else {
            Color32::from_rgb(200, 150, 30)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(55, 60, 70)
        } else {
            Color32::from_rgb(230, 235, 245)
        };

        let result_bg = if is_dark {
            Color32::from_rgb(45, 48, 55)
        } else {
            Color32::from_rgb(245, 247, 250)
        };

        egui::Window::new("🔍 Search in Files")
            .id(egui::Id::new("search_in_files_window"))
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .default_height(350.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .rounding(8.0)
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                // Search input row
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Enter search text...")
                            .desired_width(350.0)
                            .id(egui::Id::new("search_query_input")),
                    );

                    // Auto-focus on open
                    response.request_focus();

                    ui.checkbox(&mut self.use_regex, "Regex");
                    ui.checkbox(&mut self.case_sensitive, "Aa");
                });

                ui.add_space(8.0);

                // Check if search should be triggered (Enter pressed or query changed)
                let enter_pressed = ctx.input(|i| i.key_pressed(Key::Enter));
                if enter_pressed && !self.query.is_empty() {
                    output.should_search = true;
                }

                // Results summary
                if !self.query.is_empty() {
                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::from_rgb(220, 80, 80), error);
                    } else if self.results.is_empty() && self.last_query == self.query {
                        ui.label(
                            RichText::new("No results found")
                                .color(secondary_color)
                                .italics(),
                        );
                    } else if !self.results.is_empty() {
                        let file_count = self.results.len();
                        ui.label(format!(
                            "{} match{} in {} file{}",
                            self.total_matches,
                            if self.total_matches == 1 { "" } else { "es" },
                            file_count,
                            if file_count == 1 { "" } else { "s" }
                        ));
                    }
                }

                ui.separator();

                // Results list
                ScrollArea::vertical()
                    .id_source("search_results_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        for (file_idx, file_result) in self.results.iter_mut().enumerate() {
                            // File header
                            let relative_path = file_result
                                .path
                                .strip_prefix(workspace_root)
                                .unwrap_or(&file_result.path)
                                .to_string_lossy();

                            let file_id = egui::Id::new("search_file").with(file_idx);

                            let header_response = ui.horizontal(|ui| {
                                let arrow = if file_result.expanded { "▼" } else { "▶" };
                                ui.label(RichText::new(arrow).size(10.0).color(secondary_color));
                                ui.label(RichText::new("📄").size(14.0));
                                ui.label(
                                    RichText::new(relative_path.as_ref())
                                        .color(text_color)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(format!("({})", file_result.matches.len()))
                                        .color(secondary_color)
                                        .small(),
                                );
                            });

                            let header_interact =
                                ui.interact(header_response.response.rect, file_id, Sense::click());
                            if header_interact.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if header_interact.clicked() {
                                file_result.expanded = !file_result.expanded;
                            }

                            // File matches
                            if file_result.expanded {
                                ui.add_space(2.0);

                                for (match_idx, search_match) in
                                    file_result.matches.iter().enumerate()
                                {
                                    let _match_id = file_id.with(match_idx);

                                    // Build the display text
                                    let line = &search_match.line_content;
                                    let line_trimmed = line.trim_start();
                                    let trim_offset = line.len() - line_trimmed.len();

                                    // Adjust match positions for trimmed line
                                    let adj_start =
                                        search_match.match_start.saturating_sub(trim_offset);
                                    let adj_end =
                                        search_match.match_end.saturating_sub(trim_offset);

                                    // Truncate display (use char boundary to avoid UTF-8 panic)
                                    let max_len = 80;
                                    let display_line = if line_trimmed.len() > max_len {
                                        let safe_end = floor_char_boundary(line_trimmed, max_len);
                                        format!("{}...", &line_trimmed[..safe_end])
                                    } else {
                                        line_trimmed.to_string()
                                    };

                                    // Create a layout job for highlighted text
                                    let mut job = egui::text::LayoutJob::default();

                                    // Line number
                                    job.append(
                                        &format!("{:>4}: ", search_match.line_number),
                                        0.0,
                                        TextFormat {
                                            color: secondary_color,
                                            font_id: egui::FontId::monospace(12.0),
                                            ..Default::default()
                                        },
                                    );

                                    // Ensure indices are on UTF-8 char boundaries
                                    let safe_start =
                                        floor_char_boundary(&display_line, adj_start);
                                    let safe_end = floor_char_boundary(&display_line, adj_end);

                                    // Text before match
                                    if safe_start > 0 && safe_start <= display_line.len() {
                                        job.append(
                                            &display_line[..safe_start],
                                            0.0,
                                            TextFormat {
                                                color: text_color,
                                                font_id: egui::FontId::monospace(12.0),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    // Highlighted match
                                    if safe_start < display_line.len()
                                        && safe_end <= display_line.len()
                                        && safe_start < safe_end
                                    {
                                        job.append(
                                            &display_line[safe_start..safe_end],
                                            0.0,
                                            TextFormat {
                                                color: Color32::BLACK,
                                                background: highlight_color,
                                                font_id: egui::FontId::monospace(12.0),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    // Text after match
                                    if safe_end < display_line.len() {
                                        job.append(
                                            &display_line[safe_end..],
                                            0.0,
                                            TextFormat {
                                                color: text_color,
                                                font_id: egui::FontId::monospace(12.0),
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    // Draw result row
                                    let _row_rect = ui.available_rect_before_wrap();
                                    let desired_size = egui::vec2(ui.available_width(), 20.0);
                                    let (rect, response) =
                                        ui.allocate_exact_size(desired_size, Sense::click());

                                    // Background on hover
                                    if response.hovered() {
                                        ui.painter().rect_filled(rect, 3.0, hover_bg);
                                        // Show pointer cursor for clickable results
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    } else {
                                        ui.painter().rect_filled(rect, 3.0, result_bg);
                                    }

                                    // Draw the text
                                    let galley = ui.fonts(|f| f.layout_job(job));
                                    ui.painter().galley(
                                        rect.left_top() + egui::vec2(8.0, 2.0),
                                        galley,
                                        text_color,
                                    );

                                    // Handle click
                                    if response.clicked() {
                                        output.navigate_to = Some((
                                            file_result.path.clone(),
                                            search_match.line_number,
                                        ));
                                        output.closed = true;
                                    }
                                }

                                if file_result.truncated {
                                    ui.label(
                                        RichText::new("    ... more matches not shown")
                                            .color(secondary_color)
                                            .small()
                                            .italics(),
                                    );
                                }

                                ui.add_space(6.0);
                            }
                        }
                    });

                // Keyboard hints
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            "Press Enter to search • Click result to open • Esc to close",
                        )
                        .color(secondary_color)
                        .small(),
                    );
                });
            });

        if output.closed {
            self.close();
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_panel_new() {
        let panel = SearchPanel::new();
        assert!(!panel.is_open());
        assert!(panel.query.is_empty());
    }

    #[test]
    fn test_search_panel_toggle() {
        let mut panel = SearchPanel::new();
        assert!(!panel.is_open());

        panel.toggle();
        assert!(panel.is_open());

        panel.toggle();
        assert!(!panel.is_open());
    }

    #[test]
    fn test_search_match() {
        let m = SearchMatch {
            line_number: 10,
            line_content: "Hello world".to_string(),
            match_start: 6,
            match_end: 11,
        };
        assert_eq!(m.line_number, 10);
        assert_eq!(&m.line_content[m.match_start..m.match_end], "world");
    }
}
