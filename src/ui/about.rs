//! About/Help Panel Component for Ferrite
//!
//! This module implements a modal About/Help panel that displays:
//! - Application information and version
//! - GitHub and documentation links
//! - Complete keyboard shortcuts reference
//! - Credits and license information

use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};

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
    pub fn label(&self) -> &'static str {
        match self {
            ShortcutCategory::File => "File",
            ShortcutCategory::Edit => "Edit",
            ShortcutCategory::View => "View",
            ShortcutCategory::Formatting => "Formatting",
            ShortcutCategory::Workspace => "Workspace",
            ShortcutCategory::Navigation => "Navigation",
        }
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
    keys: &'static str,
    action: &'static str,
}

impl Shortcut {
    const fn new(keys: &'static str, action: &'static str) -> Self {
        Self { keys, action }
    }
}

/// Get shortcuts for a given category.
fn get_shortcuts(category: ShortcutCategory) -> Vec<Shortcut> {
    match category {
        ShortcutCategory::File => vec![
            Shortcut::new("Ctrl+N", "New File"),
            Shortcut::new("Ctrl+O", "Open File"),
            Shortcut::new("Ctrl+S", "Save"),
            Shortcut::new("Ctrl+Shift+S", "Save As"),
            Shortcut::new("Ctrl+W", "Close Tab"),
        ],
        ShortcutCategory::Edit => vec![
            Shortcut::new("Ctrl+Z", "Undo"),
            Shortcut::new("Ctrl+Y", "Redo"),
            Shortcut::new("Ctrl+F", "Find"),
            Shortcut::new("Ctrl+H", "Find & Replace"),
            Shortcut::new("Ctrl+A", "Select All"),
            Shortcut::new("Ctrl+C", "Copy"),
            Shortcut::new("Ctrl+X", "Cut"),
            Shortcut::new("Ctrl+V", "Paste"),
        ],
        ShortcutCategory::View => vec![
            Shortcut::new("Ctrl+E", "Toggle Raw/Rendered"),
            Shortcut::new("Ctrl+Shift+O", "Toggle Outline"),
            Shortcut::new("Ctrl++", "Zoom In"),
            Shortcut::new("Ctrl+-", "Zoom Out"),
            Shortcut::new("Ctrl+0", "Reset Zoom"),
            Shortcut::new("Ctrl+,", "Settings"),
            Shortcut::new("F1", "About / Help"),
        ],
        ShortcutCategory::Formatting => vec![
            Shortcut::new("Ctrl+B", "Bold"),
            Shortcut::new("Ctrl+I", "Italic"),
            Shortcut::new("Ctrl+U", "Underline"),
            Shortcut::new("Ctrl+K", "Insert Link"),
            Shortcut::new("Ctrl+`", "Inline Code"),
        ],
        ShortcutCategory::Workspace => vec![
            Shortcut::new("Ctrl+P", "Quick File Switcher"),
            Shortcut::new("Ctrl+Shift+F", "Search in Files"),
            Shortcut::new("Ctrl+Shift+E", "Toggle File Tree"),
        ],
        ShortcutCategory::Navigation => vec![
            Shortcut::new("Ctrl+Tab", "Next Tab"),
            Shortcut::new("Ctrl+Shift+Tab", "Previous Tab"),
            Shortcut::new("Ctrl+G", "Go to Line"),
            Shortcut::new("F3", "Find Next"),
            Shortcut::new("Shift+F3", "Find Previous"),
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
    pub fn label(&self) -> &'static str {
        match self {
            AboutSection::About => "About",
            AboutSection::Shortcuts => "Shortcuts",
        }
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
        egui::Window::new("❓ About / Help")
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
                        if ui.button("Close").clicked() {
                            output.close_requested = true;
                        }
                        ui.label(RichText::new("Press F1 or Escape to close").small().weak());
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
                ui.heading(RichText::new("Ferrite").size(24.0).strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .size(14.0)
                        .weak(),
                );
                ui.add_space(8.0);
                ui.label("A fast, lightweight text editor for Markdown, JSON, and more");
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Links section
            ui.label(RichText::new("🔗 Links").strong().size(16.0));
            ui.add_space(8.0);

            const GITHUB_REPO: &str = "https://github.com/OlaProeis/Ferrite";

            ui.horizontal(|ui| {
                ui.label("GitHub:");
                if ui
                    .link("View on GitHub")
                    .on_hover_text("Open repository in browser")
                    .clicked()
                {
                    let _ = open::that(GITHUB_REPO);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Report Issue:");
                if ui
                    .link("Submit a bug report")
                    .on_hover_text("Open issue tracker in browser")
                    .clicked()
                {
                    let _ = open::that(format!("{}/issues/new", GITHUB_REPO));
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Built with section
            ui.label(RichText::new("⚙ Built With").strong().size(16.0));
            ui.add_space(8.0);

            let libraries = [
                ("egui", "Immediate mode GUI framework"),
                ("comrak", "GitHub-flavored Markdown parser"),
                ("syntect", "Syntax highlighting"),
                ("serde", "Serialization framework"),
                ("notify", "File system watcher"),
            ];

            egui::Grid::new("libraries_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    for (name, desc) in libraries {
                        let text_color = if is_dark {
                            Color32::from_rgb(130, 180, 255)
                        } else {
                            Color32::from_rgb(0, 102, 204)
                        };
                        ui.label(RichText::new(name).color(text_color).strong());
                        ui.label(RichText::new(desc).weak());
                        ui.end_row();
                    }
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // License section
            ui.label(RichText::new("📜 License").strong().size(16.0));
            ui.add_space(8.0);
            ui.label("MIT License");
            ui.label(RichText::new("© 2024 Ferrite Contributors").weak());

            ui.add_space(16.0);
        });
    }

    /// Show the Shortcuts section with categorized keyboard shortcuts.
    fn show_shortcuts_section(&mut self, ui: &mut Ui, is_dark: bool) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new("Keyboard Shortcuts").size(16.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new("Click a category to expand/collapse")
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
                                                    RichText::new(shortcut.keys)
                                                        .color(key_color)
                                                        .family(egui::FontFamily::Monospace)
                                                        .size(12.0),
                                                );
                                            });
                                    });

                                    ui.label(shortcut.action);
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
    fn test_about_section_label() {
        assert_eq!(AboutSection::About.label(), "About");
        assert_eq!(AboutSection::Shortcuts.label(), "Shortcuts");
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
        assert_eq!(shortcuts[0].keys, "Ctrl+N");
        assert_eq!(shortcuts[0].action, "New File");
    }

    #[test]
    fn test_about_panel_output_default() {
        let output = AboutPanelOutput::default();
        assert!(!output.close_requested);
    }
}
