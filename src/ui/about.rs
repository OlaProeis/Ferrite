//! About/Help Panel Component for Ferrite
//!
//! This module implements a modal About/Help panel that displays:
//! - Application information and version
//! - GitHub and documentation links
//! - Complete keyboard shortcuts reference
//! - Credits and license information

use crate::app::modifier_symbol;
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};
use rust_i18n::t;

/// Keyboard shortcut category for organized display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutCategory {
    File,
    Edit,
    View,
    Formatting,
    Workspace,
    Navigation,
}

impl ShortcutCategory {
    /// Get all categories in display order.
    pub fn all() -> &'static [ShortcutCategory] {
        &[
            ShortcutCategory::File,
            ShortcutCategory::Edit,
            ShortcutCategory::View,
            ShortcutCategory::Formatting,
            ShortcutCategory::Workspace,
            ShortcutCategory::Navigation,
        ]
    }

    /// Get the display label for the category.
    pub fn label(&self) -> String {
        match self {
            ShortcutCategory::File => t!("shortcuts.category.file"),
            ShortcutCategory::Edit => t!("shortcuts.category.edit"),
            ShortcutCategory::View => t!("shortcuts.category.view"),
            ShortcutCategory::Formatting => t!("shortcuts.category.formatting"),
            ShortcutCategory::Workspace => t!("shortcuts.category.workspace"),
            ShortcutCategory::Navigation => t!("shortcuts.category.navigation"),
        }
        .to_string()
    }

    /// Get the icon for the category.
    pub fn icon(&self) -> &'static str {
        match self {
            ShortcutCategory::File => "📄",
            ShortcutCategory::Edit => "/",
            ShortcutCategory::View => "👁",
            ShortcutCategory::Formatting => "Aa",
            ShortcutCategory::Workspace => "📁",
            ShortcutCategory::Navigation => "↔",
        }
    }
}

/// A keyboard shortcut entry.
struct Shortcut {
    keys: String,
    /// i18n key for the action description
    action_key: &'static str,
}

impl Shortcut {
    fn new(keys: impl Into<String>, action_key: &'static str) -> Self {
        Self {
            keys: keys.into(),
            action_key,
        }
    }

    /// Get the localized action description.
    fn action(&self) -> String {
        t!(self.action_key).to_string()
    }
}

/// Get shortcuts for a given category.
fn get_shortcuts(category: ShortcutCategory) -> Vec<Shortcut> {
    let m = modifier_symbol();
    match category {
        ShortcutCategory::File => vec![
            Shortcut::new(format!("{}+N", m), "shortcuts.file.new"),
            Shortcut::new(format!("{}+O", m), "shortcuts.file.open"),
            Shortcut::new(format!("{}+S", m), "shortcuts.file.save"),
            Shortcut::new(format!("{}+Shift+S", m), "shortcuts.file.save_as"),
            Shortcut::new(format!("{}+W", m), "shortcuts.file.close_tab"),
        ],
        ShortcutCategory::Edit => vec![
            Shortcut::new(format!("{}+Z", m), "shortcuts.edit.undo"),
            Shortcut::new(format!("{}+Y", m), "shortcuts.edit.redo"),
            Shortcut::new(format!("{}+F", m), "shortcuts.edit.find"),
            Shortcut::new(format!("{}+H", m), "shortcuts.edit.find_replace"),
            Shortcut::new(format!("{}+A", m), "shortcuts.edit.select_all"),
            Shortcut::new(format!("{}+C", m), "shortcuts.edit.copy"),
            Shortcut::new(format!("{}+X", m), "shortcuts.edit.cut"),
            Shortcut::new(format!("{}+V", m), "shortcuts.edit.paste"),
            Shortcut::new(format!("{}+D", m), "shortcuts.edit.delete_line"),
            Shortcut::new(format!("{}+Shift+D", m), "shortcuts.edit.duplicate_line"),
            Shortcut::new("Alt+Up", "shortcuts.edit.move_line_up"),
            Shortcut::new("Alt+Down", "shortcuts.edit.move_line_down"),
            Shortcut::new("Ctrl+G", "shortcuts.edit.select_next_occurrence"),
        ],
        ShortcutCategory::View => vec![
            Shortcut::new(format!("{}+E", m), "shortcuts.view.toggle_view"),
            Shortcut::new(format!("{}+Shift+O", m), "shortcuts.view.toggle_outline"),
            Shortcut::new(format!("{}++", m), "shortcuts.view.zoom_in"),
            Shortcut::new(format!("{}+-", m), "shortcuts.view.zoom_out"),
            Shortcut::new(format!("{}+0", m), "shortcuts.view.reset_zoom"),
            Shortcut::new(format!("{}+,", m), "shortcuts.view.settings"),
            Shortcut::new("F1", "shortcuts.view.about"),
        ],
        ShortcutCategory::Formatting => vec![
            Shortcut::new(format!("{}+B", m), "shortcuts.format.bold"),
            Shortcut::new(format!("{}+I", m), "shortcuts.format.italic"),
            Shortcut::new(format!("{}+U", m), "shortcuts.format.underline"),
            Shortcut::new(format!("{}+K", m), "shortcuts.format.link"),
            Shortcut::new(format!("{}+`", m), "shortcuts.format.code"),
        ],
        ShortcutCategory::Workspace => vec![
            Shortcut::new(format!("{}+P", m), "shortcuts.workspace.quick_switcher"),
            Shortcut::new(format!("{}+Shift+F", m), "shortcuts.workspace.search_files"),
            Shortcut::new(format!("{}+Shift+E", m), "shortcuts.workspace.toggle_tree"),
        ],
        ShortcutCategory::Navigation => vec![
            Shortcut::new(format!("{}+Tab", m), "shortcuts.nav.next_tab"),
            Shortcut::new(format!("{}+Shift+Tab", m), "shortcuts.nav.prev_tab"),
            Shortcut::new(format!("{}+Shift+G", m), "shortcuts.nav.go_to_line"),
            Shortcut::new("F3", "shortcuts.nav.find_next"),
            Shortcut::new("Shift+F3", "shortcuts.nav.find_prev"),
        ],
    }
}

/// About panel sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AboutSection {
    #[default]
    About,
    Shortcuts,
}

impl AboutSection {
    /// Get the display label for the section.
    pub fn label(&self) -> String {
        match self {
            AboutSection::About => t!("about.tab.about"),
            AboutSection::Shortcuts => t!("about.tab.shortcuts"),
        }
        .to_string()
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        match self {
            AboutSection::About => "○",
            AboutSection::Shortcuts => "⌘",
        }
    }
}

/// Result of showing the about panel.
#[derive(Debug, Clone, Default)]
pub struct AboutPanelOutput {
    /// Whether the panel should be closed.
    pub close_requested: bool,
}

/// About/Help panel state and rendering.
#[derive(Debug, Clone)]
pub struct AboutPanel {
    /// Currently active section.
    active_section: AboutSection,
    /// Which shortcut categories are collapsed.
    collapsed_categories: Vec<ShortcutCategory>,
}

impl Default for AboutPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AboutPanel {
    /// Create a new about panel instance.
    pub fn new() -> Self {
        Self {
            active_section: AboutSection::default(),
            collapsed_categories: Vec::new(),
        }
    }

    /// Show the about panel as a modal window.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Output indicating what actions to take
    pub fn show(&mut self, ctx: &egui::Context, is_dark: bool) -> AboutPanelOutput {
        let mut output = AboutPanelOutput::default();

        // Semi-transparent overlay
        let screen_rect = ctx.screen_rect();
        let overlay_color = if is_dark {
            Color32::from_rgba_unmultiplied(0, 0, 0, 180)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 120)
        };

        egui::Area::new(egui::Id::new("about_overlay"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(screen_rect, 0.0, overlay_color);

                // Close on click outside
                if response.clicked() {
                    output.close_requested = true;
                }
            });

        // About modal window
        egui::Window::new(format!("❓ {}", t!("about.title")))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(550.0)
            .max_width(650.0)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Handle escape key to close
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    output.close_requested = true;
                }

                ui.horizontal(|ui| {
                    // Left side: Section tabs
                    ui.vertical(|ui| {
                        ui.set_min_width(100.0);

                        for section in [AboutSection::About, AboutSection::Shortcuts] {
                            let selected = self.active_section == section;
                            let text = format!("{} {}", section.icon(), section.label());

                            let btn = ui.add_sized(
                                [95.0, 32.0],
                                egui::SelectableLabel::new(
                                    selected,
                                    RichText::new(text).size(14.0),
                                ),
                            );

                            if btn.clicked() {
                                self.active_section = section;
                            }
                        }
                    });

                    ui.separator();

                    // Right side: Section content
                    ui.vertical(|ui| {
                        ui.set_min_width(420.0);
                        ui.set_min_height(380.0);

                        match self.active_section {
                            AboutSection::About => {
                                self.show_about_section(ui, is_dark);
                            }
                            AboutSection::Shortcuts => {
                                self.show_shortcuts_section(ui, is_dark);
                            }
                        }
                    });
                });

                ui.separator();

                // Bottom buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("dialog.confirm.close")).clicked() {
                            output.close_requested = true;
                        }
                        ui.label(RichText::new(t!("about.close_hint")).small().weak());
                    });
                });
            });

        output
    }

    /// Show the About section with app info and links.
    fn show_about_section(&self, ui: &mut Ui, is_dark: bool) {
        ScrollArea::vertical().show(ui, |ui| {
            // App name and version
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(RichText::new(t!("app.name")).size(24.0).strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t!("about.version", version = env!("CARGO_PKG_VERSION")))
                        .size(14.0)
                        .weak(),
                );
                ui.add_space(8.0);
                ui.label(t!("about.description"));
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Links section
            ui.label(RichText::new(format!("🔗 {}", t!("about.links"))).strong().size(16.0));
            ui.add_space(8.0);

            const GITHUB_REPO: &str = "https://github.com/OlaProeis/Ferrite";

            ui.horizontal(|ui| {
                ui.label(t!("about.github_label"));
                if ui
                    .link(t!("about.view_on_github"))
                    .on_hover_text(t!("about.github_tooltip"))
                    .clicked()
                {
                    let _ = open::that(GITHUB_REPO);
                }
            });

            ui.horizontal(|ui| {
                ui.label(t!("about.report_issue_label"));
                if ui
                    .link(t!("about.submit_bug"))
                    .on_hover_text(t!("about.issue_tooltip"))
                    .clicked()
                {
                    let _ = open::that(format!("{}/issues/new", GITHUB_REPO));
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Built with section
            ui.label(RichText::new(format!("⚙ {}", t!("about.built_with"))).strong().size(16.0));
            ui.add_space(8.0);

            let libraries = [
                ("egui", t!("about.lib.egui")),
                ("comrak", t!("about.lib.comrak")),
                ("syntect", t!("about.lib.syntect")),
                ("serde", t!("about.lib.serde")),
                ("notify", t!("about.lib.notify")),
            ];

            egui::Grid::new("libraries_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    for (name, desc) in &libraries {
                        let text_color = if is_dark {
                            Color32::from_rgb(130, 180, 255)
                        } else {
                            Color32::from_rgb(0, 102, 204)
                        };
                        ui.label(RichText::new(*name).color(text_color).strong());
                        ui.label(RichText::new(desc.to_string()).weak());
                        ui.end_row();
                    }
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // License section
            ui.label(RichText::new(format!("📜 {}", t!("about.license"))).strong().size(16.0));
            ui.add_space(8.0);
            ui.label(t!("about.license_type"));
            ui.label(RichText::new(t!("about.copyright")).weak());

            ui.add_space(16.0);
        });
    }

    /// Show the Shortcuts section with categorized keyboard shortcuts.
    fn show_shortcuts_section(&mut self, ui: &mut Ui, is_dark: bool) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(t!("shortcuts.title")).size(16.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(t!("shortcuts.expand_hint"))
                    .weak()
                    .small(),
            );
            ui.add_space(12.0);

            for category in ShortcutCategory::all() {
                let is_collapsed = self.collapsed_categories.contains(category);

                // Category header (clickable)
                let header_text = format!(
                    "{} {} {}",
                    if is_collapsed { "▶" } else { "▼" },
                    category.icon(),
                    category.label()
                );

                let header_response = ui.add(
                    egui::Button::new(RichText::new(header_text).strong().size(14.0))
                        .frame(false)
                        .min_size(egui::vec2(ui.available_width(), 24.0)),
                );

                if header_response.clicked() {
                    if is_collapsed {
                        self.collapsed_categories.retain(|c| c != category);
                    } else {
                        self.collapsed_categories.push(*category);
                    }
                }

                // Show shortcuts if not collapsed
                if !is_collapsed {
                    ui.indent(category.label(), |ui| {
                        let shortcuts = get_shortcuts(*category);

                        egui::Grid::new(format!("shortcuts_{:?}", category))
                            .num_columns(2)
                            .spacing([16.0, 4.0])
                            .min_col_width(100.0)
                            .show(ui, |ui| {
                                for shortcut in shortcuts {
                                    // Shortcut keys with styled background
                                    let key_bg = if is_dark {
                                        Color32::from_rgb(60, 60, 70)
                                    } else {
                                        Color32::from_rgb(230, 230, 235)
                                    };
                                    let key_color = if is_dark {
                                        Color32::from_rgb(255, 200, 100)
                                    } else {
                                        Color32::from_rgb(150, 80, 0)
                                    };

                                    ui.horizontal(|ui| {
                                        egui::Frame::none()
                                            .fill(key_bg)
                                            .rounding(3.0)
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&shortcut.keys)
                                                        .color(key_color)
                                                        .family(egui::FontFamily::Monospace)
                                                        .size(12.0),
                                                );
                                            });
                                    });

                                    ui.label(shortcut.action());
                                    ui.end_row();
                                }
                            });
                    });
                }

                ui.add_space(4.0);
            }

            ui.add_space(8.0);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_about_panel_new() {
        let panel = AboutPanel::new();
        assert_eq!(panel.active_section, AboutSection::About);
        assert!(panel.collapsed_categories.is_empty());
    }

    #[test]
    fn test_about_panel_default() {
        let panel = AboutPanel::default();
        assert_eq!(panel.active_section, AboutSection::About);
    }

    #[test]
    fn test_shortcut_category_all() {
        let categories = ShortcutCategory::all();
        assert_eq!(categories.len(), 6);
        assert_eq!(categories[0], ShortcutCategory::File);
    }

    #[test]
    fn test_get_shortcuts_file() {
        let shortcuts = get_shortcuts(ShortcutCategory::File);
        assert!(!shortcuts.is_empty());
        assert_eq!(shortcuts[0].keys, format!("{}+N", modifier_symbol()));
        assert_eq!(shortcuts[0].action_key, "shortcuts.file.new");
    }

    #[test]
    fn test_about_panel_output_default() {
        let output = AboutPanelOutput::default();
        assert!(!output.close_requested);
    }
}
