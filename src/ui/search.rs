//! Search-in-files panel for workspace mode.
//!
//! Provides Ctrl+Shift+F search functionality across all files in the workspace.
//!
//! ## Viewport Constraints
//!
//! The search panel implements robust viewport constraints to ensure it always
//! renders fully within the main application window. It:
//!
//! - Calculates safe bounds based on the current window client area
//! - Respects minimum/maximum size limits while enabling internal scrolling
//! - Automatically repositions when the window is resized
//! - Works correctly with Split View and other layout modes

// Allow dead code - includes ID counter and layout helpers for future search UI features
#![allow(dead_code)]

use crate::string_utils::floor_char_boundary;
use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::{CARET_DOWN, CARET_RIGHT, FILE};
use crate::ui::{
    center_panel_in_viewport, file_index_progress_ui, search_panel_constraints, PanelConstraints,
};
use crate::workspaces::FileIndexProgress;
use eframe::egui::{self, Color32, Key, Pos2, Rect, RichText, ScrollArea, Sense, TextFormat, Vec2};
use rust_i18n::t;
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
    /// Absolute character offset from start of document
    pub char_offset: usize,
    /// Length of match in characters
    pub match_len: usize,
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

/// Navigation target from a search result click.
#[derive(Debug, Clone)]
pub struct SearchNavigationTarget {
    /// Path to the file containing the match
    pub path: PathBuf,
    /// The line number (1-indexed)
    pub line_number: usize,
    /// Absolute character offset from start of document
    pub char_offset: usize,
    /// Length of match in characters
    pub match_len: usize,
}

/// Output from the search panel.
#[derive(Debug, Default)]
pub struct SearchPanelOutput {
    /// Navigation target (user clicked a result)
    pub navigate_to: Option<SearchNavigationTarget>,
    /// Whether the panel was closed
    pub closed: bool,
    /// Whether search should be triggered
    pub should_search: bool,
    /// Screen rect of the search window (for native WebView occlusion).
    pub screen_rect: Option<egui::Rect>,
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
    /// Last known panel size (for persistence)
    panel_size: Vec2,
    /// Last known panel position (for persistence)
    panel_pos: Option<Pos2>,
    /// Viewport constraints for the panel
    constraints: PanelConstraints,
    /// Last known viewport size (to detect resize)
    last_viewport_size: Vec2,
}

impl Default for SearchPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPanel {
    /// Default panel width.
    const DEFAULT_WIDTH: f32 = 500.0;
    /// Default panel height (compact; results scroll inside).
    const DEFAULT_HEIGHT: f32 = 320.0;

    /// Fixed chrome above the results scroll area (input, summary, separator, hints).
    const RESULTS_SCROLL_CHROME: f32 = 130.0;

    fn window_id() -> egui::Id {
        egui::Id::new("search_in_files_window")
    }

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
            panel_size: Vec2::new(Self::DEFAULT_WIDTH, Self::DEFAULT_HEIGHT),
            panel_pos: None, // Will be computed on first show
            constraints: search_panel_constraints(),
            last_viewport_size: Vec2::ZERO,
        }
    }

    /// Get the panel size for persistence.
    pub fn panel_size(&self) -> Vec2 {
        self.panel_size
    }

    /// Set the panel size (from persistence), respecting constraints.
    pub fn set_panel_size(&mut self, size: Vec2) {
        self.panel_size = Vec2::new(
            size.x
                .clamp(self.constraints.min_width, self.constraints.max_width),
            size.y
                .clamp(self.constraints.min_height, self.constraints.max_height),
        );
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

            // Track absolute character offset from start of document
            let mut line_start_offset = 0usize;

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

                    let match_len = match_end - match_start;
                    let char_offset = line_start_offset + match_start;

                    file_results.matches.push(SearchMatch {
                        line_number,
                        line_content: line.to_string(),
                        match_start,
                        match_end,
                        char_offset,
                        match_len,
                    });
                    self.total_matches += 1;
                }

                if file_results.truncated {
                    break;
                }

                // Update offset for next line (+1 for newline character)
                line_start_offset += line.len() + 1;
            }

            if !file_results.matches.is_empty() {
                self.results.push(file_results);
            }

            if self.results.len() >= MAX_FILES_WITH_RESULTS {
                break;
            }
        }
    }

    /// Show the search panel with viewport constraints.
    ///
    /// The panel automatically constrains itself to fit within the visible
    /// viewport, repositioning and resizing as needed when the window size
    /// changes.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        workspace_root: &PathBuf,
        is_dark: bool,
        index_progress: Option<FileIndexProgress>,
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

        // Root viewport bounds for floating panel placement (not panel-shrunk content_rect).
        let viewport = crate::ui::window::viewport_window_rect(ctx);
        let viewport_size = viewport.size();

        // Check if viewport size changed (window resize, DPI change, split view toggle)
        let viewport_changed = (viewport_size - self.last_viewport_size).length() > 1.0;
        if viewport_changed {
            self.last_viewport_size = viewport_size;
            // Force recalculation of position on next frame
            self.panel_pos = None;
        }

        // Calculate constrained panel bounds
        let constrained = if let Some(pos) = self.panel_pos {
            // Use existing position but ensure it's still valid
            let desired_rect = Rect::from_min_size(pos, self.panel_size);
            crate::ui::constrain_rect_to_viewport(desired_rect, viewport, &self.constraints)
        } else {
            // Center panel in viewport on first show or after viewport change
            center_panel_in_viewport(viewport, self.panel_size, &self.constraints)
        };

        // Update stored position/size
        self.panel_pos = Some(constrained.pos);
        self.panel_size = constrained.size;

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

        let max_window_width = (viewport.width() - self.constraints.margin * 2.0)
            .min(self.constraints.max_width)
            .max(self.constraints.min_width);
        let max_window_height = (viewport.height() - self.constraints.margin * 2.0)
            .min(self.constraints.max_height)
            .max(self.constraints.min_height);

        // Build the window with constrained bounds
        let mut window = egui::Window::new(t!("search.title").to_string())
            .id(Self::window_id())
            .collapsible(false)
            .resizable(true)
            .fade_in(false)
            .fade_out(false)
            .default_pos(constrained.pos)
            .default_size(constrained.size)
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .corner_radius(8.0)
                    .inner_margin(12.0),
            );

        // Apply size constraints to prevent manual resize beyond bounds
        window = window
            .order(egui::Order::Foreground)
            .min_width(self.constraints.min_width)
            .min_height(self.constraints.min_height)
            .max_width(max_window_width)
            .max_height(max_window_height);

        let window_response = window.show(ctx, |ui| {
            // Search input row
            ui.horizontal(|ui| {
                ui.label(t!("search.label"));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text(t!("search.placeholder"))
                        .desired_width(350.0)
                        .id(egui::Id::new("search_query_input")),
                );

                // Auto-focus on open
                response.request_focus();

                ui.checkbox(&mut self.use_regex, t!("find.use_regex"));
                ui.checkbox(&mut self.case_sensitive, t!("find.match_case_short"));
            });

            if let Some(progress) = index_progress {
                ui.add_space(4.0);
                file_index_progress_ui(ui, progress, secondary_color);
            }

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
                        RichText::new(t!("find.no_results"))
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

            // Fixed-height results region so egui's Resize container does not grow the
            // window frame-by-frame as matches load (that reads as a slow animation).
            let scroll_h = (ui.available_height() - 4.0)
                .max(self.constraints.min_height - Self::RESULTS_SCROLL_CHROME);
            let scroll_w = ui.available_width();
            let (scroll_rect, _) =
                ui.allocate_exact_size(Vec2::new(scroll_w, scroll_h), Sense::hover());
            let mut scroll_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(scroll_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            scroll_ui.set_clip_rect(scroll_rect);
            scroll_ui.set_max_width(scroll_w);

            ScrollArea::vertical()
                .id_salt("search_results_scroll")
                .auto_shrink([false, false])
                .show(&mut scroll_ui, |ui| {
                    ui.set_max_width(scroll_w);

                    for (file_idx, file_result) in self.results.iter_mut().enumerate() {
                        // File header
                        let relative_path = file_result
                            .path
                            .strip_prefix(workspace_root)
                            .unwrap_or(&file_result.path)
                            .to_string_lossy();

                        let file_id = egui::Id::new("search_file").with(file_idx);

                        let header_response = ui.horizontal(|ui| {
                            let arrow = if file_result.expanded {
                                CARET_DOWN
                            } else {
                                CARET_RIGHT
                            };
                            ui.label(phosphor_rich_text(arrow, 10.0).color(secondary_color));
                            ui.label(phosphor_rich_text(FILE, 14.0));
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

                            for (match_idx, search_match) in file_result.matches.iter().enumerate()
                            {
                                let _match_id = file_id.with(match_idx);

                                // Build the display text
                                let line = &search_match.line_content;
                                let line_trimmed = line.trim_start();
                                let trim_offset = line.len() - line_trimmed.len();

                                // Adjust match positions for trimmed line
                                let adj_start =
                                    search_match.match_start.saturating_sub(trim_offset);
                                let adj_end = search_match.match_end.saturating_sub(trim_offset);

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
                                let safe_start = floor_char_boundary(&display_line, adj_start);
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
                                let galley = ui.fonts_mut(|f| f.layout_job(job));
                                ui.painter().galley(
                                    rect.left_top() + egui::vec2(8.0, 2.0),
                                    galley,
                                    text_color,
                                );

                                // Handle click
                                if response.clicked() {
                                    output.navigate_to = Some(SearchNavigationTarget {
                                        path: file_result.path.clone(),
                                        line_number: search_match.line_number,
                                        char_offset: search_match.char_offset,
                                        match_len: search_match.match_len,
                                    });
                                    output.closed = true;
                                }
                            }

                            if file_result.truncated {
                                ui.label(
                                    RichText::new(t!("search.more_matches"))
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
                    RichText::new(t!("search.keyboard_hints"))
                        .color(secondary_color)
                        .small(),
                );
            });
        });

        // Persist user resize / drag from egui window state (not content-driven growth).
        if let Some(rect) = ctx.memory(|mem| mem.area_rect(Self::window_id())) {
            self.panel_pos = Some(rect.min);
            self.panel_size = Vec2::new(
                rect.width()
                    .clamp(self.constraints.min_width, max_window_width),
                rect.height()
                    .clamp(self.constraints.min_height, max_window_height),
            );
            output.screen_rect = Some(
                rect.expand(crate::markdown::video_render::VIDEO_OCCLUDER_MARGIN),
            );
        } else if let Some(response) = window_response {
            output.screen_rect = Some(
                crate::markdown::video_render::egui_rect_to_screen(
                    ctx,
                    response.response.layer_id,
                    response.response.rect,
                )
                .expand(crate::markdown::video_render::VIDEO_OCCLUDER_MARGIN),
            );
        }

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
        // Check default size
        assert_eq!(panel.panel_size().x, SearchPanel::DEFAULT_WIDTH);
        assert_eq!(panel.panel_size().y, SearchPanel::DEFAULT_HEIGHT);
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
    fn test_search_panel_size_constraints() {
        let mut panel = SearchPanel::new();
        // Copy constraint values to avoid borrow conflicts
        let min_width = panel.constraints.min_width;
        let min_height = panel.constraints.min_height;
        let max_width = panel.constraints.max_width;
        let max_height = panel.constraints.max_height;

        // Test setting size within bounds
        panel.set_panel_size(Vec2::new(400.0, 300.0));
        assert_eq!(panel.panel_size().x, 400.0);
        assert_eq!(panel.panel_size().y, 300.0);

        // Test size clamped to minimum
        panel.set_panel_size(Vec2::new(100.0, 50.0));
        assert!(panel.panel_size().x >= min_width);
        assert!(panel.panel_size().y >= min_height);

        // Test size clamped to maximum
        panel.set_panel_size(Vec2::new(2000.0, 2000.0));
        assert!(panel.panel_size().x <= max_width);
        assert!(panel.panel_size().y <= max_height);
    }

    #[test]
    fn test_search_match() {
        let m = SearchMatch {
            line_number: 10,
            line_content: "Hello world".to_string(),
            match_start: 6,
            match_end: 11,
            char_offset: 100, // example absolute offset
            match_len: 5,
        };
        assert_eq!(m.line_number, 10);
        assert_eq!(&m.line_content[m.match_start..m.match_end], "world");
        assert_eq!(m.match_len, 5);
    }
}
