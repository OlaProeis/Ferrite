//! Document Outline Panel Component
//!
//! This module implements a side panel that displays a live-updating,
//! clickable outline of document headings (H1-H6) with collapsible sections.

use crate::config::OutlinePanelSide;
use crate::editor::{DocumentOutline, OutlineItem, OutlineType, StructuredStats};
use eframe::egui::{self, Color32, Response, RichText, ScrollArea, Sense, Ui, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Minimum width for the outline panel.
const MIN_PANEL_WIDTH: f32 = 120.0;

/// Maximum width for the outline panel.
const MAX_PANEL_WIDTH: f32 = 400.0;

/// Indentation per heading level.
const INDENT_PER_LEVEL: f32 = 16.0;

/// Height of each outline item.
const ITEM_HEIGHT: f32 = 24.0;

// ─────────────────────────────────────────────────────────────────────────────
// OutlinePanelOutput
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the outline panel indicating user actions.
#[derive(Debug, Clone, Default)]
pub struct OutlinePanelOutput {
    /// Line number to scroll to (1-indexed), if a heading was clicked
    pub scroll_to_line: Option<usize>,
    /// Character offset to scroll to, if a heading was clicked
    pub scroll_to_char: Option<usize>,
    /// Heading ID that was toggled (collapsed/expanded)
    pub toggled_id: Option<String>,
    /// Whether the close button was clicked
    pub close_requested: bool,
    /// New panel width if resized
    pub new_width: Option<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OutlinePanel
// ─────────────────────────────────────────────────────────────────────────────

/// The document outline panel widget.
#[derive(Debug, Clone)]
pub struct OutlinePanel {
    /// Current panel width
    width: f32,
    /// Which side the panel is on
    side: OutlinePanelSide,
    /// Currently highlighted heading index (based on cursor position)
    current_section: Option<usize>,
}

impl Default for OutlinePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl OutlinePanel {
    /// Create a new outline panel.
    pub fn new() -> Self {
        Self {
            width: 200.0,
            side: OutlinePanelSide::Right,
            current_section: None,
        }
    }

    /// Set the panel width.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        self
    }

    /// Set which side the panel is on.
    pub fn with_side(mut self, side: OutlinePanelSide) -> Self {
        self.side = side;
        self
    }

    /// Set the current section (highlighted heading).
    #[allow(dead_code)]
    pub fn with_current_section(mut self, section: Option<usize>) -> Self {
        self.current_section = section;
        self
    }

    /// Set the panel side (mutable reference version).
    pub fn set_side(&mut self, side: OutlinePanelSide) {
        self.side = side;
    }

    /// Set the current section (mutable reference version).
    pub fn set_current_section(&mut self, section: Option<usize>) {
        self.current_section = section;
    }

    /// Get the current panel width.
    #[allow(dead_code)]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get the panel side.
    #[allow(dead_code)]
    pub fn side(&self) -> OutlinePanelSide {
        self.side
    }

    /// Render the outline panel.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context
    /// * `outline` - The document outline to display
    /// * `is_dark` - Whether using dark theme
    ///
    /// # Returns
    ///
    /// Output indicating any user actions.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        outline: &DocumentOutline,
        is_dark: bool,
    ) -> OutlinePanelOutput {
        let mut output = OutlinePanelOutput::default();

        // Panel colors
        let panel_bg = if is_dark {
            Color32::from_rgb(35, 35, 35)
        } else {
            Color32::from_rgb(250, 250, 250)
        };

        let border_color = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

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

        let highlight_bg = if is_dark {
            Color32::from_rgb(60, 80, 110)
        } else {
            Color32::from_rgb(220, 235, 250)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(235, 235, 240)
        };

        // Create the side panel
        let panel = match self.side {
            OutlinePanelSide::Left => egui::SidePanel::left("outline_panel"),
            OutlinePanelSide::Right => egui::SidePanel::right("outline_panel"),
        };

        panel
            .resizable(true)
            .default_width(self.width)
            .min_width(MIN_PANEL_WIDTH)
            .max_width(MAX_PANEL_WIDTH)
            .frame(
                egui::Frame::none()
                    .fill(panel_bg)
                    .stroke(egui::Stroke::new(1.0, border_color)),
            )
            .show(ctx, |ui| {
                // Update width if resized
                let current_width = ui.available_width();
                if (current_width - self.width).abs() > 1.0 {
                    self.width = current_width;
                    output.new_width = Some(current_width);
                }

                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                // Header section - different label for structured files
                let (header_icon, header_text) = match &outline.outline_type {
                    OutlineType::Markdown => ("📑", "Outline"),
                    OutlineType::Structured(_) => ("📊", "Statistics"),
                };

                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("{} {}", header_icon, header_text))
                            .size(12.0)
                            .strong()
                            .color(text_color),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(14.0).color(muted_color))
                                    .frame(false)
                                    .min_size(Vec2::new(20.0, 20.0)),
                            )
                            .on_hover_text("Close outline (Ctrl+Shift+O)")
                            .clicked()
                        {
                            output.close_requested = true;
                        }
                    });
                });

                ui.add_space(4.0);

                // Different content based on outline type
                match &outline.outline_type {
                    OutlineType::Structured(stats) => {
                        // Show format name as subtitle
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(&stats.format_name)
                                    .size(10.0)
                                    .color(muted_color),
                            );
                        });
                        ui.add_space(4.0);
                        ui.separator();

                        // Show statistics
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                self.render_structured_stats(
                                    ui,
                                    stats,
                                    text_color,
                                    muted_color,
                                    is_dark,
                                );
                            });
                    }
                    OutlineType::Markdown => {
                        // Summary stats for markdown
                        if !outline.is_empty() {
                            let summary = format!(
                                "{} headings • ~{} min read",
                                outline.heading_count, outline.estimated_read_time
                            );

                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(RichText::new(summary).size(10.0).color(muted_color));
                            });
                            ui.add_space(4.0);
                        }

                        ui.separator();

                        // Scrollable heading list
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if outline.is_empty() {
                                    ui.add_space(20.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new("No headings found")
                                                .size(11.0)
                                                .color(muted_color)
                                                .italics(),
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Add headings using # syntax")
                                                .size(10.0)
                                                .color(muted_color),
                                        );
                                    });
                                } else {
                                    ui.add_space(4.0);

                                    for (index, item) in outline.items.iter().enumerate() {
                                        // Check visibility (respects collapsed parents)
                                        if !outline.is_visible(index) {
                                            continue;
                                        }

                                        let is_current = self.current_section == Some(index);
                                        let has_children = outline.has_children(index);

                                        let response = self.render_outline_item(
                                            ui,
                                            item,
                                            is_current,
                                            has_children,
                                            text_color,
                                            muted_color,
                                            highlight_bg,
                                            hover_bg,
                                            is_dark,
                                        );

                                        if response.clicked() {
                                            log::debug!(
                                                "Outline: clicked heading '{}' at line {}",
                                                item.title,
                                                item.line
                                            );
                                            output.scroll_to_line = Some(item.line);
                                            output.scroll_to_char = Some(item.char_offset);
                                        }

                                        // Handle collapse/expand toggle (double-click or icon click)
                                        if has_children && response.double_clicked() {
                                            output.toggled_id = Some(item.id.clone());
                                        }
                                    }

                                    ui.add_space(8.0);
                                }
                            });
                    }
                }
            });

        output
    }

    /// Render statistics for a structured file (JSON/YAML/TOML).
    fn render_structured_stats(
        &self,
        ui: &mut Ui,
        stats: &StructuredStats,
        text_color: Color32,
        muted_color: Color32,
        is_dark: bool,
    ) {
        ui.add_space(8.0);

        // Check for parse error
        if !stats.parse_success {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("⚠ Parse Error")
                        .size(12.0)
                        .color(Color32::from_rgb(220, 80, 80))
                        .strong(),
                );
                ui.add_space(4.0);
                if let Some(ref err) = stats.parse_error {
                    ui.label(RichText::new(err).size(10.0).color(muted_color));
                }
            });
            return;
        }

        // Colors for different stat types
        let key_color = if is_dark {
            Color32::from_rgb(156, 220, 254) // Light blue
        } else {
            Color32::from_rgb(0, 100, 150)
        };

        let number_color = if is_dark {
            Color32::from_rgb(181, 206, 168) // Light green
        } else {
            Color32::from_rgb(0, 128, 0)
        };

        let string_color = if is_dark {
            Color32::from_rgb(206, 145, 120) // Orange
        } else {
            Color32::from_rgb(163, 21, 21)
        };

        let bool_color = if is_dark {
            Color32::from_rgb(86, 156, 214) // Blue
        } else {
            Color32::from_rgb(0, 0, 255)
        };

        // Structure section
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("📁 Structure")
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(ui, "Objects", stats.object_count, key_color, muted_color);
        self.render_stat_row(ui, "Arrays", stats.array_count, key_color, muted_color);
        self.render_stat_row(ui, "Total keys", stats.total_keys, key_color, muted_color);
        self.render_stat_row(ui, "Max depth", stats.max_depth, muted_color, muted_color);

        ui.add_space(12.0);

        // Values section
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("📊 Values")
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(
            ui,
            "Total values",
            stats.value_count,
            text_color,
            muted_color,
        );

        if stats.string_count > 0 {
            self.render_stat_row(ui, "Strings", stats.string_count, string_color, muted_color);
        }
        if stats.number_count > 0 {
            self.render_stat_row(ui, "Numbers", stats.number_count, number_color, muted_color);
        }
        if stats.bool_count > 0 {
            self.render_stat_row(ui, "Booleans", stats.bool_count, bool_color, muted_color);
        }
        if stats.null_count > 0 {
            self.render_stat_row(ui, "Nulls", stats.null_count, muted_color, muted_color);
        }

        if stats.total_array_items > 0 {
            ui.add_space(4.0);
            self.render_stat_row(
                ui,
                "Array items",
                stats.total_array_items,
                key_color,
                muted_color,
            );
        }

        ui.add_space(8.0);
    }

    /// Render a single statistics row.
    fn render_stat_row(
        &self,
        ui: &mut Ui,
        label: &str,
        value: usize,
        value_color: Color32,
        label_color: Color32,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(RichText::new(label).size(10.0).color(label_color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(value.to_string())
                        .size(10.0)
                        .color(value_color)
                        .strong(),
                );
            });
        });
    }

    /// Render a single outline item (for Markdown documents).
    #[allow(clippy::too_many_arguments)]
    fn render_outline_item(
        &self,
        ui: &mut Ui,
        item: &OutlineItem,
        is_current: bool,
        has_children: bool,
        text_color: Color32,
        muted_color: Color32,
        highlight_bg: Color32,
        hover_bg: Color32,
        is_dark: bool,
    ) -> Response {
        let indent = item.indent_level() as f32 * INDENT_PER_LEVEL;

        // Reserve space for the item
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ITEM_HEIGHT), Sense::click());

        // Draw background for current or hovered item
        if is_current {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(3.0), highlight_bg);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(3.0), hover_bg);
        }

        // Draw collapse/expand indicator if has children
        let text_start_x = rect.min.x + 8.0 + indent;
        if has_children {
            let indicator = if item.collapsed { "▶" } else { "▼" };
            let indicator_pos = egui::pos2(rect.min.x + 4.0 + indent, rect.center().y);
            ui.painter().text(
                indicator_pos,
                egui::Align2::LEFT_CENTER,
                indicator,
                egui::FontId::proportional(8.0),
                muted_color,
            );
        }

        // Level indicator (H1, H2, etc.)
        let level_text = format!("H{}", item.level);
        let level_color = heading_level_color(item.level, is_dark);

        let level_pos = egui::pos2(
            text_start_x + (if has_children { 12.0 } else { 0.0 }),
            rect.center().y,
        );
        ui.painter().text(
            level_pos,
            egui::Align2::LEFT_CENTER,
            &level_text,
            egui::FontId::proportional(9.0),
            level_color,
        );

        // Title position
        let title_offset = 24.0;
        let title_x = level_pos.x + title_offset;
        let available_width = rect.max.x - title_x - 8.0;

        // Truncate title if too long
        let title = truncate_text(&item.title, available_width, 11.0);

        let title_color = if is_current {
            if is_dark {
                Color32::WHITE
            } else {
                Color32::from_rgb(30, 30, 30)
            }
        } else {
            text_color
        };

        let font_id = if item.level == 1 {
            egui::FontId::new(11.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(11.0)
        };

        ui.painter().text(
            egui::pos2(title_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &title,
            font_id,
            title_color,
        );

        response.on_hover_text(format!(
            "{}\nLine {} • Click to navigate",
            item.title, item.line
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Get a color for the heading level indicator.
fn heading_level_color(level: u8, is_dark: bool) -> Color32 {
    if is_dark {
        match level {
            1 => Color32::from_rgb(130, 180, 255), // Blue
            2 => Color32::from_rgb(150, 220, 150), // Green
            3 => Color32::from_rgb(220, 180, 120), // Orange
            4 => Color32::from_rgb(200, 150, 200), // Purple
            5 => Color32::from_rgb(180, 180, 180), // Gray
            _ => Color32::from_rgb(150, 150, 150), // Light gray
        }
    } else {
        match level {
            1 => Color32::from_rgb(40, 100, 180),  // Blue
            2 => Color32::from_rgb(50, 140, 50),   // Green
            3 => Color32::from_rgb(180, 120, 40),  // Orange
            4 => Color32::from_rgb(140, 80, 140),  // Purple
            5 => Color32::from_rgb(100, 100, 100), // Gray
            _ => Color32::from_rgb(120, 120, 120), // Dark gray
        }
    }
}

/// Truncate text to fit within a given width.
fn truncate_text(text: &str, max_width: f32, font_size: f32) -> String {
    // Estimate character width (rough approximation)
    let char_width = font_size * 0.55;
    let max_chars = (max_width / char_width) as usize;

    if text.len() <= max_chars || max_chars < 4 {
        text.to_string()
    } else {
        format!("{}…", &text[..max_chars.saturating_sub(1)])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_panel_new() {
        let panel = OutlinePanel::new();
        assert_eq!(panel.width(), 200.0);
        assert_eq!(panel.side(), OutlinePanelSide::Right);
    }

    #[test]
    fn test_outline_panel_with_width() {
        let panel = OutlinePanel::new().with_width(250.0);
        assert_eq!(panel.width(), 250.0);
    }

    #[test]
    fn test_outline_panel_width_clamping() {
        let panel = OutlinePanel::new().with_width(50.0);
        assert_eq!(panel.width(), MIN_PANEL_WIDTH);

        let panel = OutlinePanel::new().with_width(1000.0);
        assert_eq!(panel.width(), MAX_PANEL_WIDTH);
    }

    #[test]
    fn test_outline_panel_with_side() {
        let panel = OutlinePanel::new().with_side(OutlinePanelSide::Left);
        assert_eq!(panel.side(), OutlinePanelSide::Left);
    }

    #[test]
    fn test_truncate_text() {
        let short = "Hello";
        assert_eq!(truncate_text(short, 100.0, 11.0), "Hello");

        let long = "This is a very long heading that should be truncated";
        let truncated = truncate_text(long, 100.0, 11.0);
        assert!(truncated.ends_with('…'));
        assert!(truncated.len() < long.len());
    }

    #[test]
    fn test_heading_level_colors() {
        // Just verify colors are returned without panic
        for level in 1..=6 {
            let _dark = heading_level_color(level, true);
            let _light = heading_level_color(level, false);
        }
    }
}
