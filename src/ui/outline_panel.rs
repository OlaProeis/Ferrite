//! Document Outline Panel Component
//!
//! This module implements a side panel that displays a live-updating,
//! clickable outline of document headings (H1-H6) with collapsible sections,
//! and a statistics tab showing document metrics.

use crate::config::OutlinePanelSide;
use crate::editor::{
    ContentType, DocumentOutline, DocumentStats, OutlineItem, OutlineType, StructuredStats,
};
use crate::theme::accent;
use crate::ui::backlinks_panel::BacklinksPanel;
use crate::ui::docked_sidebar::{self, DockedSidebarEdge};
use crate::ui::frontmatter_panel::FrontmatterPanel;
use crate::ui::phosphor_icons::{
    phosphor_font, phosphor_rich_text, CARET_DOWN, CARET_RIGHT, CHART_BAR, LINK, LIST, LIST_CHECKS,
    NOTE_PENCIL, TEXT_T, TIMER, X,
};
use crate::ui::productivity_panel::ProductivityPanel;
use eframe::egui::{self, Color32, FontId, Response, RichText, ScrollArea, Sense, Ui, Vec2};
use rust_i18n::t;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Minimum width for the outline panel.
///
/// Must match `Settings::MIN_OUTLINE_WIDTH`. Sized to keep the 5 tab
/// labels (Outline / Stats / Links / FM / Hub) readable without clipping.
const MIN_PANEL_WIDTH: f32 = 260.0;

/// Maximum width for the outline panel.
const MAX_PANEL_WIDTH: f32 = 500.0;

/// Indentation per heading level.
const INDENT_PER_LEVEL: f32 = 18.0;

/// Height of each outline item.
const ITEM_HEIGHT: f32 = 26.0;

/// Height of the tab bar.
const TAB_HEIGHT: f32 = 28.0;

// ─────────────────────────────────────────────────────────────────────────────
// OutlinePanelTab
// ─────────────────────────────────────────────────────────────────────────────

/// The active tab in the outline panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutlinePanelTab {
    /// Document outline view (headings, etc.)
    #[default]
    Outline,
    /// Document statistics view
    Statistics,
    /// Productivity hub (tasks, pomodoro, notes)
    Productivity,
    /// Backlinks view (files linking to the current file)
    Backlinks,
    /// YAML frontmatter editor (markdown files only)
    Frontmatter,
}

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
    /// Heading title text for the clicked item (for text-based navigation)
    pub scroll_to_title: Option<String>,
    /// Heading level (1-6) for the clicked item
    pub scroll_to_level: Option<u8>,
    /// Heading ID that was toggled (collapsed/expanded)
    pub toggled_id: Option<String>,
    /// Whether the close button was clicked
    pub close_requested: bool,
    /// New panel width if resized
    pub new_width: Option<f32>,
    /// Whether the productivity panel requested to be detached (undocked)
    pub detach_productivity: bool,
    /// Whether the productivity panel needs a repaint (e.g. timer active)
    pub needs_repaint: bool,
    /// File path to navigate to from backlinks panel
    pub backlink_navigate_to: Option<std::path::PathBuf>,
    /// New document content from frontmatter edits
    pub frontmatter_new_content: Option<String>,
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
    /// Currently active tab
    active_tab: OutlinePanelTab,
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
            width: 300.0,
            side: OutlinePanelSide::Right,
            current_section: None,
            active_tab: OutlinePanelTab::Outline,
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

    /// Get the active tab.
    #[allow(dead_code)]
    pub fn active_tab(&self) -> OutlinePanelTab {
        self.active_tab
    }

    /// Set the active tab.
    #[allow(dead_code)] // Public API for programmatic tab switching
    pub fn set_active_tab(&mut self, tab: OutlinePanelTab) {
        self.active_tab = tab;
    }

    /// Render the outline panel.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        outline: &DocumentOutline,
        doc_stats: Option<&DocumentStats>,
        is_dark: bool,
        ui_accent: Color32,
        productivity_panel: Option<&mut ProductivityPanel>,
        backlinks_panel: Option<&BacklinksPanel>,
        frontmatter_panel: Option<&mut FrontmatterPanel>,
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

        let highlight_bg = accent::panel_highlight_fill(
            panel_bg,
            ui_accent,
            is_dark,
            if is_dark { 0.38 } else { 0.31 },
        );

        let hover_bg = if is_dark {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(235, 235, 240)
        };

        // Use a single, stable width range for all tabs. Re-applying a
        // tab-specific minimum each frame caused the panel to snap back when
        // dragged narrow on the Productivity / Frontmatter tabs (felt like a
        // slow animation as `default_width` and `min_width` recalculated).
        // The new card-based Hub layout adapts cleanly down to MIN_PANEL_WIDTH.
        let panel = match self.side {
            OutlinePanelSide::Left => egui::Panel::left("outline_panel"),
            OutlinePanelSide::Right => egui::Panel::right("outline_panel"),
        };

        let sidebar_edge = match self.side {
            OutlinePanelSide::Left => DockedSidebarEdge::Left,
            OutlinePanelSide::Right => DockedSidebarEdge::Right,
        };

        panel
            .resizable(true)
            .default_size(self.width)
            .size_range(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH)
            .frame(docked_sidebar::frame(panel_bg))
            .show_inside(ui, |ui| {
                // Update width if resized
                let current_width = ui.available_width();
                if (current_width - self.width).abs() > 1.0 {
                    self.width = current_width;
                    output.new_width = Some(current_width);
                }

                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                // Header section with close button
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let header_text = match self.active_tab {
                        OutlinePanelTab::Productivity => t!("productivity.title").to_string(),
                        OutlinePanelTab::Frontmatter => "Frontmatter".to_string(),
                        _ => match &outline.outline_type {
                            OutlineType::Markdown => t!("outline.panel_title").to_string(),
                            OutlineType::Structured(_) => t!("outline.statistics").to_string(),
                        },
                    };
                    ui.label(
                        RichText::new(header_text)
                            .size(12.0)
                            .strong()
                            .color(text_color),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(phosphor_rich_text(X, 14.0).color(muted_color))
                                    .frame(false)
                                    .min_size(Vec2::new(20.0, 20.0)),
                            )
                            .on_hover_text(t!("outline.close_tooltip"))
                            .clicked()
                        {
                            output.close_requested = true;
                        }
                    });
                });

                ui.add_space(2.0);

                // Always show tab bar (Outline/Stats tabs + Productivity tab)
                self.render_tab_bar(ui, text_color, muted_color, highlight_bg, is_dark);
                ui.add_space(4.0);

                // Render content based on active tab
                if self.active_tab == OutlinePanelTab::Backlinks {
                    // Backlinks content
                    if let Some(bl_panel) = backlinks_panel {
                        let bl_output = bl_panel.show_content(ui, is_dark);
                        if bl_output.navigate_to.is_some() {
                            output.backlink_navigate_to = bl_output.navigate_to;
                        }
                    } else {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(t!("outline.backlinks_unavailable").to_string())
                                    .size(11.0)
                                    .color(muted_color)
                                    .italics(),
                            );
                        });
                    }
                } else if self.active_tab == OutlinePanelTab::Frontmatter {
                    if let Some(fm_panel) = frontmatter_panel {
                        let fm_output = fm_panel.show_content(ui, is_dark);
                        if fm_output.new_content.is_some() {
                            output.frontmatter_new_content = fm_output.new_content;
                        }
                    } else {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Frontmatter editing is only available for Markdown files")
                                    .size(11.0)
                                    .color(muted_color)
                                    .italics(),
                            );
                        });
                    }
                } else if self.active_tab == OutlinePanelTab::Productivity {
                    // Productivity Hub content
                    if let Some(panel) = productivity_panel {
                        let accent_btn = ui_accent;

                        // Detach action row, styled to match the floating
                        // window's Dock button so the affordance reads as
                        // bidirectional.
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(6.0);
                                if ui
                                    .add_sized(
                                        Vec2::new(70.0, 22.0),
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "⤴ {}",
                                                t!("outline.detach")
                                            ))
                                            .size(11.0)
                                            .color(accent_btn),
                                        ),
                                    )
                                    .on_hover_text(t!("outline.detach_tooltip").to_string())
                                    .clicked()
                                {
                                    output.detach_productivity = true;
                                }
                            });
                        });

                        ui.add_space(4.0);

                        // Strictly allocate a fixed-width rectangle for
                        // productivity content. egui's `SidePanel` stores
                        // the rendered content's `min_rect` in `PanelState`,
                        // so any widget inside that asks for more width
                        // would permanently grow the sidebar (and prevent
                        // the user from shrinking it back). By using an
                        // explicit `allocate_exact_size` and rendering
                        // inside a clipped child UI, we lock the outer
                        // footprint to exactly the panel width regardless
                        // of what any individual widget reports.
                        let avail_w = ui.available_width();
                        let avail_h = ui.available_height();
                        let (content_rect, _) = ui.allocate_exact_size(
                            Vec2::new(avail_w, avail_h),
                            Sense::hover(),
                        );

                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(content_rect)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        child_ui.set_clip_rect(content_rect);
                        child_ui.set_max_width(avail_w);

                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(&mut child_ui, |ui| {
                                ui.set_max_width(avail_w);
                                egui::Frame::new()
                                    .inner_margin(egui::Margin::symmetric(6, 4))
                                    .show(ui, |ui| {
                                        ui.set_max_width(avail_w - 12.0);
                                        let panel_ctx = ui.ctx().clone();
                                        let repaint = panel.show_content(ui, &panel_ctx, ui_accent);
                                        output.needs_repaint = repaint;
                                    });
                            });
                    } else {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(t!("outline.productivity_unavailable").to_string())
                                    .size(11.0)
                                    .color(muted_color)
                                    .italics(),
                            );
                        });
                    }
                } else {
                    // Original outline/statistics content
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
                            match self.active_tab {
                                OutlinePanelTab::Outline => {
                                    // Summary stats for markdown
                                    if !outline.is_empty() {
                                        let summary = t!(
                                            "outline.summary",
                                            headings = outline.heading_count,
                                            minutes = outline.estimated_read_time
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
                                                        RichText::new(t!("outline.no_headings"))
                                                            .size(11.0)
                                                            .color(muted_color)
                                                            .italics(),
                                                    );
                                                    ui.add_space(8.0);
                                                    ui.label(
                                                        RichText::new(t!("outline.add_headings_hint"))
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
                                                        ui_accent,
                                                    );

                                                    if response.clicked() {
                                                        log::debug!(
                                                            "Outline: clicked heading '{}' at line {}",
                                                            item.title,
                                                            item.line
                                                        );
                                                        output.scroll_to_line = Some(item.line);
                                                        output.scroll_to_char = Some(item.char_offset);
                                                        output.scroll_to_title = Some(item.title.clone());
                                                        output.scroll_to_level = Some(item.level);
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
                                OutlinePanelTab::Statistics => {
                                    ui.separator();
                                    ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            if let Some(stats) = doc_stats {
                                                self.render_document_stats(
                                                    ui,
                                                    stats,
                                                    text_color,
                                                    muted_color,
                                                    is_dark,
                                                );
                                            } else {
                                                ui.add_space(20.0);
                                                ui.vertical_centered(|ui| {
                                                    ui.label(
                                                        RichText::new(t!("stats.no_data"))
                                                            .size(11.0)
                                                            .color(muted_color)
                                                            .italics(),
                                                    );
                                                });
                                            }
                                        });
                                }
                                OutlinePanelTab::Productivity => {
                                    // Already handled above, shouldn't reach here
                                }
                                OutlinePanelTab::Backlinks => {
                                    // Already handled above, shouldn't reach here
                                }
                                OutlinePanelTab::Frontmatter => {
                                    // Already handled above, shouldn't reach here
                                }
                            }
                        }
                    }
                }

                docked_sidebar::paint_vertical_divider(ui, border_color, sidebar_edge);
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
                    RichText::new(t!("outline.parse_error"))
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
                RichText::new(t!("outline.json_structure").to_string())
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(
            ui,
            &t!("outline.json_objects"),
            stats.object_count,
            key_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("outline.json_arrays"),
            stats.array_count,
            key_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("outline.json_total_keys"),
            stats.total_keys,
            key_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("outline.json_max_depth"),
            stats.max_depth,
            muted_color,
            muted_color,
        );

        ui.add_space(12.0);

        // Values section
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                phosphor_rich_text(CHART_BAR, 11.0)
                    .strong()
                    .color(text_color),
            );
            ui.label(
                RichText::new(t!("outline.json_values").to_string())
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(
            ui,
            &t!("outline.json_total_values"),
            stats.value_count,
            text_color,
            muted_color,
        );

        if stats.string_count > 0 {
            self.render_stat_row(
                ui,
                &t!("outline.json_strings"),
                stats.string_count,
                string_color,
                muted_color,
            );
        }
        if stats.number_count > 0 {
            self.render_stat_row(
                ui,
                &t!("outline.json_numbers"),
                stats.number_count,
                number_color,
                muted_color,
            );
        }
        if stats.bool_count > 0 {
            self.render_stat_row(
                ui,
                &t!("outline.json_booleans"),
                stats.bool_count,
                bool_color,
                muted_color,
            );
        }
        if stats.null_count > 0 {
            self.render_stat_row(
                ui,
                &t!("outline.json_nulls"),
                stats.null_count,
                muted_color,
                muted_color,
            );
        }

        if stats.total_array_items > 0 {
            ui.add_space(4.0);
            self.render_stat_row(
                ui,
                &t!("outline.json_array_items"),
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

    /// Render the tab bar for switching between Outline, Statistics, and Productivity.
    fn render_tab_bar(
        &mut self,
        ui: &mut Ui,
        text_color: Color32,
        muted_color: Color32,
        highlight_bg: Color32,
        is_dark: bool,
    ) {
        let tab_bg = if is_dark {
            Color32::from_rgb(45, 45, 50)
        } else {
            Color32::from_rgb(240, 240, 245)
        };

        let active_tab_bg = if is_dark {
            Color32::from_rgb(55, 55, 65)
        } else {
            Color32::from_rgb(255, 255, 255)
        };

        // Tab definitions: (tab enum, icon, label)
        let tabs: Vec<(OutlinePanelTab, &str, String)> = vec![
            (
                OutlinePanelTab::Outline,
                LIST,
                t!("outline.tab_outline").to_string(),
            ),
            (
                OutlinePanelTab::Statistics,
                CHART_BAR,
                t!("outline.tab_statistics").to_string(),
            ),
            (
                OutlinePanelTab::Backlinks,
                LINK,
                t!("outline.tab_links").to_string(),
            ),
            (OutlinePanelTab::Frontmatter, TEXT_T, "FM".to_string()),
            (
                OutlinePanelTab::Productivity,
                LIST_CHECKS,
                t!("outline.tab_hub").to_string(),
            ),
        ];

        ui.horizontal(|ui| {
            ui.add_space(4.0);

            // Calculate tab width to fit all tabs
            let num_tabs = tabs.len() as f32;
            let gap = 2.0 * (num_tabs - 1.0);
            let available_width = ui.available_width() - 8.0 - gap;
            let tab_width = (available_width / num_tabs).min(100.0);

            let mut active_rect_opt: Option<egui::Rect> = None;

            for (i, (tab, icon, label)) in tabs.iter().enumerate() {
                if i > 0 {
                    ui.add_space(2.0);
                }

                let is_active = self.active_tab == *tab;
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(tab_width, TAB_HEIGHT), Sense::click());

                let bg = if is_active { active_tab_bg } else { tab_bg };
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    bg,
                );

                let tab_text_color = if is_active { text_color } else { muted_color };
                paint_tab_label(ui, rect, icon, label.as_str(), tab_text_color);

                if response.clicked() {
                    self.active_tab = *tab;
                }

                if is_active {
                    active_rect_opt = Some(rect);
                }
            }

            // Draw underline for active tab
            if let Some(active_rect) = active_rect_opt {
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(active_rect.min.x, active_rect.max.y - 2.0),
                        Vec2::new(active_rect.width(), 2.0),
                    ),
                    0.0,
                    highlight_bg,
                );
            }
        });
    }

    /// Render document statistics for Markdown files.
    fn render_document_stats(
        &self,
        ui: &mut Ui,
        stats: &DocumentStats,
        text_color: Color32,
        muted_color: Color32,
        is_dark: bool,
    ) {
        ui.add_space(8.0);

        // Colors for different stat types
        let word_color = if is_dark {
            Color32::from_rgb(156, 220, 254) // Light blue
        } else {
            Color32::from_rgb(0, 100, 150)
        };

        let heading_color = if is_dark {
            Color32::from_rgb(181, 206, 168) // Light green
        } else {
            Color32::from_rgb(0, 128, 0)
        };

        let link_color = if is_dark {
            Color32::from_rgb(206, 145, 120) // Orange
        } else {
            Color32::from_rgb(163, 21, 21)
        };

        let code_color = if is_dark {
            Color32::from_rgb(86, 156, 214) // Blue
        } else {
            Color32::from_rgb(0, 0, 255)
        };

        // ─────────────────────────────────────────────────────────────────────
        // Reading Time (prominent at top)
        // ─────────────────────────────────────────────────────────────────────
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(TIMER, 14.0).strong().color(text_color));
                ui.label(
                    RichText::new(stats.format_reading_time())
                        .size(14.0)
                        .strong()
                        .color(text_color),
                );
            });
            ui.add_space(4.0);
        });

        ui.add_space(4.0);

        // ─────────────────────────────────────────────────────────────────────
        // Text Statistics Section
        // ─────────────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                phosphor_rich_text(NOTE_PENCIL, 11.0)
                    .strong()
                    .color(text_color),
            );
            ui.label(
                RichText::new(t!("stats.text_stats").to_string())
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(
            ui,
            &t!("stats.words"),
            stats.text.words,
            word_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("stats.characters"),
            stats.text.characters,
            word_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("stats.characters_no_spaces"),
            stats.text.characters_no_spaces,
            muted_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("stats.lines"),
            stats.text.lines,
            muted_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("stats.paragraphs"),
            stats.text.paragraphs,
            muted_color,
            muted_color,
        );

        ui.add_space(12.0);

        // ─────────────────────────────────────────────────────────────────────
        // Structure Section
        // ─────────────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(phosphor_rich_text(LIST, 11.0).strong().color(text_color));
            ui.label(
                RichText::new(t!("stats.structure").to_string())
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        // Headings breakdown
        if stats.heading_count > 0 {
            self.render_stat_row(
                ui,
                &t!("stats.headings_total"),
                stats.heading_count,
                heading_color,
                muted_color,
            );

            // Show per-level counts if any are non-zero
            for (i, &count) in stats.headings_by_level.iter().enumerate() {
                if count > 0 {
                    let label = format!("  H{}", i + 1);
                    self.render_stat_row(ui, &label, count, muted_color, muted_color);
                }
            }
        } else {
            self.render_stat_row(ui, &t!("stats.headings_total"), 0, muted_color, muted_color);
        }

        ui.add_space(4.0);

        if stats.list_item_count > 0 {
            self.render_stat_row(
                ui,
                &t!("stats.list_items"),
                stats.list_item_count,
                muted_color,
                muted_color,
            );
        }

        if stats.horizontal_rule_count > 0 {
            self.render_stat_row(
                ui,
                &t!("stats.horizontal_rules"),
                stats.horizontal_rule_count,
                muted_color,
                muted_color,
            );
        }

        ui.add_space(12.0);

        // ─────────────────────────────────────────────────────────────────────
        // Media & Links Section
        // ─────────────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(phosphor_rich_text(LINK, 11.0).strong().color(text_color));
            ui.label(
                RichText::new(t!("stats.media_links").to_string())
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
        ui.add_space(4.0);

        self.render_stat_row(
            ui,
            &t!("stats.links"),
            stats.link_count,
            link_color,
            muted_color,
        );
        self.render_stat_row(
            ui,
            &t!("stats.images"),
            stats.image_count,
            link_color,
            muted_color,
        );

        ui.add_space(12.0);

        // ─────────────────────────────────────────────────────────────────────
        // Code & Diagrams Section
        // ─────────────────────────────────────────────────────────────────────
        if stats.code_block_count > 0
            || stats.mermaid_count > 0
            || stats.table_count > 0
            || stats.blockquote_count > 0
        {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("</> {}", t!("stats.code_diagrams")))
                        .size(11.0)
                        .strong()
                        .color(text_color),
                );
            });
            ui.add_space(4.0);

            if stats.code_block_count > 0 {
                self.render_stat_row(
                    ui,
                    &t!("stats.code_blocks"),
                    stats.code_block_count,
                    code_color,
                    muted_color,
                );
            }
            if stats.mermaid_count > 0 {
                self.render_stat_row(
                    ui,
                    &t!("stats.mermaid_diagrams"),
                    stats.mermaid_count,
                    code_color,
                    muted_color,
                );
            }
            if stats.table_count > 0 {
                self.render_stat_row(
                    ui,
                    &t!("stats.tables"),
                    stats.table_count,
                    code_color,
                    muted_color,
                );
            }
            if stats.blockquote_count > 0 {
                self.render_stat_row(
                    ui,
                    &t!("stats.blockquotes"),
                    stats.blockquote_count,
                    muted_color,
                    muted_color,
                );
            }
        }

        ui.add_space(8.0);
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
        ui_accent: Color32,
    ) -> Response {
        let indent_level = item.indent_level() as usize;

        // Reserve space for the item
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ITEM_HEIGHT), Sense::click());

        let is_hot = is_current || response.hovered();

        let base_x = rect.min.x + 22.0;
        let bullet_center_x = base_x + indent_level as f32 * INDENT_PER_LEVEL;
        let bullet_center = egui::pos2(bullet_center_x, rect.center().y);

        let tree_color = if is_hot {
            ui_accent
        } else if is_dark {
            accent::lerp_color(ui_accent, Color32::from_rgb(70, 78, 92), 0.55)
        } else {
            accent::lerp_color(ui_accent, Color32::from_rgb(214, 220, 230), 0.58)
        };
        let guide_stroke = egui::Stroke::new(if is_hot { 1.4 } else { 1.0 }, tree_color);

        for level in 0..indent_level {
            let x = base_x + level as f32 * INDENT_PER_LEVEL;
            let vertical_bottom = if level + 1 == indent_level {
                bullet_center.y
            } else {
                rect.bottom() - 2.0
            };
            ui.painter().line_segment(
                [
                    egui::pos2(x, rect.top() + 1.0),
                    egui::pos2(x, vertical_bottom),
                ],
                guide_stroke,
            );
        }

        if indent_level > 0 {
            let branch_start_x = base_x + (indent_level - 1) as f32 * INDENT_PER_LEVEL;
            ui.painter().line_segment(
                [
                    egui::pos2(branch_start_x, bullet_center.y),
                    egui::pos2(bullet_center.x - 5.0, bullet_center.y),
                ],
                guide_stroke,
            );
        }
        if has_children && !item.collapsed {
            ui.painter().line_segment(
                [
                    egui::pos2(bullet_center.x, bullet_center.y + 5.0),
                    egui::pos2(bullet_center.x, rect.bottom() - 1.0),
                ],
                guide_stroke,
            );
        }

        let semantic_color = if item.is_heading() {
            ui_accent
        } else {
            item_badge_color(item, is_dark, ui_accent)
        };
        let node_color = if is_hot { ui_accent } else { semantic_color };
        let bullet_fill = if is_current {
            ui_accent
        } else if is_dark {
            Color32::from_rgb(28, 33, 42)
        } else {
            Color32::WHITE
        };

        ui.painter().circle_filled(
            bullet_center,
            if is_current { 4.0 } else { 3.2 },
            bullet_fill,
        );
        ui.painter().circle_stroke(
            bullet_center,
            if is_current { 5.8 } else { 4.2 },
            egui::Stroke::new(if is_hot { 1.6 } else { 1.2 }, node_color),
        );

        // Draw collapse/expand indicator as a subtle affordance, not a badge.
        let title_x = bullet_center.x + if has_children { 17.0 } else { 13.0 };
        if has_children {
            let indicator = if item.collapsed {
                CARET_RIGHT
            } else {
                CARET_DOWN
            };
            ui.painter().text(
                egui::pos2(bullet_center.x + 7.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                indicator,
                phosphor_font(7.5),
                if is_hot { ui_accent } else { muted_color },
            );
        }

        let available_width = rect.max.x - title_x - 10.0;
        let title = truncate_text(&item.title, available_width, 12.5);

        let font_size = if item.is_heading() && item.level == 1 {
            12.5
        } else {
            12.0
        };
        let font_id = if item.is_heading() && item.level <= 2 {
            egui::FontId::new(font_size, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(font_size)
        };
        let title_galley =
            ui.fonts_mut(|fonts| fonts.layout_no_wrap(title.clone(), font_id.clone(), text_color));
        let pill_width = (title_galley.size().x + 16.0).min(rect.max.x - title_x - 6.0);
        let pill_rect = egui::Rect::from_min_size(
            egui::pos2(title_x - 7.0, rect.center().y - 10.0),
            Vec2::new(pill_width.max(24.0), 20.0),
        );

        if is_current {
            ui.painter()
                .rect_filled(pill_rect, egui::CornerRadius::same(6), highlight_bg);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(pill_rect, egui::CornerRadius::same(6), hover_bg);
        }

        let title_color = if is_current || response.hovered() {
            ui_accent
        } else if item.is_heading() {
            text_color
        } else {
            muted_color
        };

        ui.painter().text(
            egui::pos2(title_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &title,
            font_id,
            title_color,
        );

        response.on_hover_text(
            t!(
                "outline.item_tooltip",
                item_type = readable_item_type(item),
                title = item.title.clone(),
                line = item.line
            )
            .to_string(),
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Get a color for the heading level indicator.
fn heading_level_color(level: u8, is_dark: bool, accent: Color32) -> Color32 {
    if is_dark {
        match level {
            1 => accent,
            2 => Color32::from_rgb(150, 220, 150), // Green
            3 => Color32::from_rgb(220, 180, 120), // Orange
            4 => Color32::from_rgb(200, 150, 200), // Purple
            5 => Color32::from_rgb(180, 180, 180), // Gray
            _ => Color32::from_rgb(150, 150, 150), // Light gray
        }
    } else {
        match level {
            1 => accent::lerp_color(accent, Color32::BLACK, 0.08),
            2 => Color32::from_rgb(50, 140, 50),   // Green
            3 => Color32::from_rgb(180, 120, 40),  // Orange
            4 => Color32::from_rgb(140, 80, 140),  // Purple
            5 => Color32::from_rgb(100, 100, 100), // Gray
            _ => Color32::from_rgb(120, 120, 120), // Dark gray
        }
    }
}

fn readable_item_type(item: &OutlineItem) -> String {
    match item.content_type {
        ContentType::Heading(1) => t!("shortcuts.commands.heading_1").to_string(),
        ContentType::Heading(2) => t!("shortcuts.commands.heading_2").to_string(),
        ContentType::Heading(3) => t!("shortcuts.commands.heading_3").to_string(),
        ContentType::Heading(4) => t!("shortcuts.commands.heading_4").to_string(),
        ContentType::Heading(5) => t!("shortcuts.commands.heading_5").to_string(),
        ContentType::Heading(_) => t!("shortcuts.commands.heading_6").to_string(),
        ContentType::CodeBlock => t!("shortcuts.commands.code_block").to_string(),
        ContentType::MermaidDiagram => t!("outline.mermaid_diagrams").to_string(),
        ContentType::Table => t!("outline.tables").to_string(),
        ContentType::Image => t!("shortcuts.commands.image").to_string(),
        ContentType::Blockquote => t!("shortcuts.commands.blockquote").to_string(),
    }
}

fn item_badge_color(item: &OutlineItem, is_dark: bool, accent: Color32) -> Color32 {
    match item.content_type {
        crate::editor::ContentType::Heading(level) => heading_level_color(level, is_dark, accent),
        crate::editor::ContentType::CodeBlock => {
            if is_dark {
                Color32::from_rgb(180, 180, 180)
            } else {
                Color32::from_rgb(90, 90, 90)
            }
        }
        crate::editor::ContentType::MermaidDiagram => {
            if is_dark {
                Color32::from_rgb(120, 220, 180)
            } else {
                Color32::from_rgb(30, 140, 110)
            }
        }
        crate::editor::ContentType::Table => {
            if is_dark {
                Color32::from_rgb(230, 190, 110)
            } else {
                Color32::from_rgb(170, 110, 0)
            }
        }
        crate::editor::ContentType::Image => {
            if is_dark {
                Color32::from_rgb(210, 150, 220)
            } else {
                Color32::from_rgb(140, 70, 155)
            }
        }
        crate::editor::ContentType::Blockquote => {
            if is_dark {
                Color32::from_rgb(160, 170, 210)
            } else {
                Color32::from_rgb(85, 95, 135)
            }
        }
    }
}

/// Paint a tab label with a Phosphor icon and proportional text, centered in `rect`.
fn paint_tab_label(ui: &Ui, rect: egui::Rect, icon: &str, label: &str, color: Color32) {
    const ICON_SIZE: f32 = 10.0;
    const GAP: f32 = 3.0;

    let text_font = FontId::proportional(ICON_SIZE);
    let label_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(label.to_string(), text_font.clone(), color)
            .size()
            .x
    });
    let total_width = ICON_SIZE + GAP + label_width;
    let start_x = rect.center().x - total_width * 0.5;
    let center_y = rect.center().y;

    ui.painter().text(
        egui::pos2(start_x + ICON_SIZE * 0.5, center_y),
        egui::Align2::CENTER_CENTER,
        icon,
        phosphor_font(ICON_SIZE),
        color,
    );
    ui.painter().text(
        egui::pos2(start_x + ICON_SIZE + GAP + label_width * 0.5, center_y),
        egui::Align2::CENTER_CENTER,
        label,
        text_font,
        color,
    );
}

/// Truncate text to fit within a given width.
fn truncate_text(text: &str, max_width: f32, font_size: f32) -> String {
    // Estimate character width (rough approximation)
    let char_width = font_size * 0.55;
    let max_chars = (max_width / char_width) as usize;

    // Use char count for proper UTF-8 handling (Korean, Chinese, Japanese, etc.)
    let char_count = text.chars().count();

    if char_count <= max_chars || max_chars < 4 {
        text.to_string()
    } else {
        // Take characters (not bytes) to avoid splitting multi-byte characters
        let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
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
        assert_eq!(panel.width(), 300.0);
        assert_eq!(panel.side(), OutlinePanelSide::Right);
    }

    #[test]
    fn test_outline_panel_with_width() {
        let panel = OutlinePanel::new().with_width(320.0);
        assert_eq!(panel.width(), 320.0);
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
        let a = crate::theme::accent::default_accent();
        for level in 1..=6 {
            let _dark = heading_level_color(level, true, a);
            let _light = heading_level_color(level, false, a);
        }
    }

}
