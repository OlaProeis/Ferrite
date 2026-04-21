//! Backlinks Panel Component
//!
//! This module implements a panel that displays files linking to the current file
//! via `[[wikilinks]]` or `[text](file.md)` markdown links. Uses an in-memory
//! graph index for efficient lookup in large workspaces.

use crate::state::BacklinkEntry;
use eframe::egui::{self, Color32, RichText, ScrollArea, Sense, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Height of each backlink item row.
const ITEM_HEIGHT: f32 = 26.0;

// ─────────────────────────────────────────────────────────────────────────────
// BacklinksPanelOutput
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the backlinks panel indicating user actions.
#[derive(Debug, Clone, Default)]
pub struct BacklinksPanelOutput {
    /// File path to navigate to (if a backlink entry was clicked)
    pub navigate_to: Option<std::path::PathBuf>,
}

// ─────────────────────────────────────────────────────────────────────────────
// BacklinksPanel
// ─────────────────────────────────────────────────────────────────────────────

/// Panel showing files that link to the currently active file.
#[derive(Debug, Clone, Default)]
pub struct BacklinksPanel {
    /// Cached backlink entries for the current file
    cached_backlinks: Vec<BacklinkEntry>,
    /// The filename (stem) for which backlinks are currently cached
    cached_for_file: Option<String>,
}

impl BacklinksPanel {
    /// Create a new backlinks panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the cached backlinks for a new target file.
    pub fn set_backlinks(&mut self, filename: Option<String>, entries: Vec<BacklinkEntry>) {
        self.cached_for_file = filename;
        self.cached_backlinks = entries;
    }

    /// Get the filename for which backlinks are currently cached.
    pub fn cached_for_file(&self) -> Option<&str> {
        self.cached_for_file.as_deref()
    }

    /// Get the number of cached backlinks.
    pub fn backlink_count(&self) -> usize {
        self.cached_backlinks.len()
    }

    /// Clear cached backlinks.
    pub fn clear(&mut self) {
        self.cached_backlinks.clear();
        self.cached_for_file = None;
    }

    /// Render the backlinks panel content inside a parent UI.
    ///
    /// This is designed to be rendered inside the outline panel's tab area
    /// or as a standalone section. Returns output indicating any user actions.
    pub fn show_content(&self, ui: &mut egui::Ui, is_dark: bool) -> BacklinksPanelOutput {
        let mut output = BacklinksPanelOutput::default();

        let text_color = if is_dark {
            Color32::from_rgb(200, 200, 200)
        } else {
            Color32::from_rgb(50, 50, 50)
        };

        let muted_color = if is_dark {
            Color32::from_rgb(130, 130, 130)
        } else {
            Color32::from_rgb(120, 120, 120)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(235, 235, 240)
        };

        let link_color = if is_dark {
            Color32::from_rgb(130, 180, 255)
        } else {
            Color32::from_rgb(40, 100, 180)
        };

        // Header: count of backlinks
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let count = self.cached_backlinks.len();
            let label = if count == 1 {
                "1 backlink".to_string()
            } else {
                format!("{} backlinks", count)
            };
            ui.label(RichText::new(label).size(10.0).color(muted_color));
        });
        ui.add_space(4.0);
        ui.separator();

        // Backlinks list
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.cached_backlinks.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        let message = if self.cached_for_file.is_some() {
                            "No files link to this document"
                        } else {
                            "Open a file to see backlinks"
                        };
                        ui.label(
                            RichText::new(message)
                                .size(11.0)
                                .color(muted_color)
                                .italics(),
                        );
                    });
                } else {
                    ui.add_space(4.0);

                    for entry in &self.cached_backlinks {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), ITEM_HEIGHT),
                            Sense::click(),
                        );

                        // Hover background
                        if response.hovered() {
                            ui.painter()
                                .rect_filled(rect, egui::Rounding::same(3.0), hover_bg);
                        }

                        // File icon + name
                        let icon_pos = egui::pos2(rect.min.x + 8.0, rect.center().y);
                        ui.painter().text(
                            icon_pos,
                            egui::Align2::LEFT_CENTER,
                            "📄",
                            egui::FontId::proportional(11.0),
                            text_color,
                        );

                        let name_pos = egui::pos2(rect.min.x + 26.0, rect.center().y);
                        let available_width = rect.max.x - name_pos.x - 8.0;
                        let display = truncate_text(&entry.display_name, available_width, 11.0);

                        ui.painter().text(
                            name_pos,
                            egui::Align2::LEFT_CENTER,
                            &display,
                            egui::FontId::proportional(11.0),
                            link_color,
                        );

                        // Tooltip with full path
                        response
                            .clone()
                            .on_hover_text(entry.source_path.display().to_string());

                        if response.clicked() {
                            output.navigate_to = Some(entry.source_path.clone());
                        }
                    }

                    ui.add_space(8.0);
                }
            });

        output
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Truncate text to fit within a given width.
fn truncate_text(text: &str, max_width: f32, font_size: f32) -> String {
    let char_width = font_size * 0.55;
    let max_chars = (max_width / char_width) as usize;
    let char_count = text.chars().count();

    if char_count <= max_chars || max_chars < 4 {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
