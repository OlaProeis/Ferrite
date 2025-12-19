//! Settings Panel Component for Ferrite
//!
//! This module implements a modal settings panel that allows users to configure
//! appearance, editor behavior, and file handling options with live preview.

use crate::config::{EditorFont, Settings, Theme};
use eframe::egui::{self, Color32, RichText, Ui};

/// Settings panel sections for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Appearance,
    Editor,
    Files,
}

impl SettingsSection {
    /// Get the display label for the section.
    pub fn label(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Editor => "Editor",
            SettingsSection::Files => "Files",
        }
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "🎨",
            SettingsSection::Editor => "📝",
            SettingsSection::Files => "📁",
        }
    }
}

/// Result of showing the settings panel.
#[derive(Debug, Clone, Default)]
pub struct SettingsPanelOutput {
    /// Whether settings were modified.
    pub changed: bool,
    /// Whether the panel should be closed.
    pub close_requested: bool,
    /// Whether a reset to defaults was requested.
    pub reset_requested: bool,
}

/// Settings panel state and rendering.
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    /// Currently active settings section.
    active_section: SettingsSection,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    /// Create a new settings panel instance.
    pub fn new() -> Self {
        Self {
            active_section: SettingsSection::default(),
        }
    }

    /// Show the settings panel as a modal window.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context
    /// * `settings` - The current settings (mutable for live preview)
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Output indicating what actions to take
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        is_dark: bool,
    ) -> SettingsPanelOutput {
        let mut output = SettingsPanelOutput::default();

        // Semi-transparent overlay
        let screen_rect = ctx.screen_rect();
        let overlay_color = if is_dark {
            Color32::from_rgba_unmultiplied(0, 0, 0, 180)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 120)
        };

        egui::Area::new(egui::Id::new("settings_overlay"))
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

        // Settings modal window
        egui::Window::new("⚙ Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(500.0)
            .max_width(600.0)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Handle escape key to close
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    output.close_requested = true;
                }

                ui.horizontal(|ui| {
                    // Left side: Section tabs
                    ui.vertical(|ui| {
                        ui.set_min_width(120.0);

                        for section in [
                            SettingsSection::Appearance,
                            SettingsSection::Editor,
                            SettingsSection::Files,
                        ] {
                            let selected = self.active_section == section;
                            let text = format!("{} {}", section.icon(), section.label());

                            let btn = ui.add_sized(
                                [110.0, 32.0],
                                egui::SelectableLabel::new(
                                    selected,
                                    RichText::new(text).size(14.0),
                                ),
                            );

                            if btn.clicked() {
                                self.active_section = section;
                            }
                        }

                        ui.add_space(ui.available_height() - 40.0);

                        // Reset button at bottom of sidebar
                        if ui
                            .add_sized([110.0, 28.0], egui::Button::new("↺ Reset All"))
                            .on_hover_text("Reset all settings to defaults")
                            .clicked()
                        {
                            output.reset_requested = true;
                        }
                    });

                    ui.separator();

                    // Right side: Section content
                    ui.vertical(|ui| {
                        ui.set_min_width(350.0);
                        ui.set_min_height(320.0);

                        match self.active_section {
                            SettingsSection::Appearance => {
                                if self.show_appearance_section(ui, settings, is_dark) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Editor => {
                                if self.show_editor_section(ui, settings) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Files => {
                                if self.show_files_section(ui, settings) {
                                    output.changed = true;
                                }
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
                        ui.label(
                            RichText::new("Settings are saved automatically")
                                .small()
                                .weak(),
                        );
                    });
                });
            });

        output
    }

    /// Show the Appearance settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_appearance_section(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        _is_dark: bool,
    ) -> bool {
        let mut changed = false;

        ui.heading("Appearance");
        ui.add_space(8.0);

        // Theme selection
        ui.label(RichText::new("Theme").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for theme in [Theme::Light, Theme::Dark, Theme::System] {
                let label = match theme {
                    Theme::Light => "☀ Light",
                    Theme::Dark => "🌙 Dark",
                    Theme::System => "💻 System",
                };
                if ui
                    .selectable_value(&mut settings.theme, theme, label)
                    .changed()
                {
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font family selection
        ui.label(RichText::new("Font").strong());
        ui.add_space(4.0);

        for font in EditorFont::all() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut settings.font_family, *font, font.display_name())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(font.description()).weak().small());
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font size slider
        ui.horizontal(|ui| {
            ui.label(RichText::new("Font Size").strong());
            ui.add_space(8.0);
            ui.label(format!("{}px", settings.font_size as u32));
        });
        ui.add_space(4.0);

        let font_slider = ui.add(
            egui::Slider::new(
                &mut settings.font_size,
                Settings::MIN_FONT_SIZE..=Settings::MAX_FONT_SIZE,
            )
            .show_value(false)
            .step_by(1.0),
        );
        if font_slider.changed() {
            changed = true;
        }

        // Font size presets
        ui.horizontal(|ui| {
            for (label, size) in [("Small", 12.0), ("Medium", 14.0), ("Large", 18.0)] {
                if ui.small_button(label).clicked() {
                    settings.font_size = size;
                    changed = true;
                }
            }
        });

        changed
    }

    /// Show the Editor settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_editor_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        ui.heading("Editor");
        ui.add_space(8.0);

        // Word wrap toggle
        if ui
            .checkbox(&mut settings.word_wrap, "Word Wrap")
            .on_hover_text("Wrap long lines instead of horizontal scrolling")
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Line numbers toggle
        if ui
            .checkbox(&mut settings.show_line_numbers, "Show Line Numbers")
            .on_hover_text("Display line numbers in the editor gutter")
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Sync scroll toggle
        if ui
            .checkbox(&mut settings.sync_scroll_enabled, "Sync Scroll")
            .on_hover_text(
                "Synchronize scroll position when switching between Raw and Rendered views",
            )
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Use spaces toggle
        if ui
            .checkbox(&mut settings.use_spaces, "Use Spaces for Indentation")
            .on_hover_text("Use spaces instead of tabs for indentation")
            .changed()
        {
            changed = true;
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Tab size slider
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tab Size").strong());
            ui.add_space(8.0);
            ui.label(format!("{} spaces", settings.tab_size));
        });
        ui.add_space(4.0);

        let mut tab_size_f32 = settings.tab_size as f32;
        let tab_slider = ui.add(
            egui::Slider::new(
                &mut tab_size_f32,
                Settings::MIN_TAB_SIZE as f32..=Settings::MAX_TAB_SIZE as f32,
            )
            .show_value(false)
            .step_by(1.0),
        );
        if tab_slider.changed() {
            settings.tab_size = tab_size_f32 as u8;
            changed = true;
        }

        // Tab size presets
        ui.horizontal(|ui| {
            for size in [2u8, 4, 8] {
                if ui.small_button(format!("{}", size)).clicked() {
                    settings.tab_size = size;
                    changed = true;
                }
            }
        });

        changed
    }

    /// Show the Files settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_files_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        ui.heading("Files");
        ui.add_space(8.0);

        // Auto-save toggle
        if ui
            .checkbox(&mut settings.auto_save, "Enable Auto-Save")
            .on_hover_text("Automatically save files at regular intervals")
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Auto-save interval (only enabled when auto-save is on)
        ui.add_enabled_ui(settings.auto_save, |ui| {
            ui.horizontal(|ui| {
                ui.label("Auto-save interval:");
                ui.add_space(8.0);
                ui.label(format!("{} seconds", settings.auto_save_interval_secs));
            });
            ui.add_space(4.0);

            let interval_slider = ui.add(
                egui::Slider::new(&mut settings.auto_save_interval_secs, 5..=300)
                    .show_value(false)
                    .step_by(5.0),
            );
            if interval_slider.changed() {
                changed = true;
            }

            // Interval presets
            ui.horizontal(|ui| {
                for (label, secs) in [("30s", 30), ("1m", 60), ("5m", 300)] {
                    if ui.small_button(label).clicked() {
                        settings.auto_save_interval_secs = secs;
                        changed = true;
                    }
                }
            });
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Recent files count
        ui.horizontal(|ui| {
            ui.label(RichText::new("Recent Files").strong());
            ui.add_space(8.0);
            ui.label(format!("Remember {} files", settings.max_recent_files));
        });
        ui.add_space(4.0);

        let mut recent_count_f32 = settings.max_recent_files as f32;
        let recent_slider = ui.add(
            egui::Slider::new(&mut recent_count_f32, 0.0..=20.0)
                .show_value(false)
                .step_by(1.0),
        );
        if recent_slider.changed() {
            settings.max_recent_files = recent_count_f32 as usize;
            changed = true;
        }

        ui.add_space(8.0);

        // Clear recent files button
        ui.horizontal(|ui| {
            if ui
                .button("Clear Recent Files")
                .on_hover_text("Remove all files from the recent files list")
                .clicked()
            {
                settings.recent_files.clear();
                changed = true;
            }

            if !settings.recent_files.is_empty() {
                ui.label(
                    RichText::new(format!("({} files)", settings.recent_files.len()))
                        .small()
                        .weak(),
                );
            }
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_panel_new() {
        let panel = SettingsPanel::new();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_default() {
        let panel = SettingsPanel::default();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_section_label() {
        assert_eq!(SettingsSection::Appearance.label(), "Appearance");
        assert_eq!(SettingsSection::Editor.label(), "Editor");
        assert_eq!(SettingsSection::Files.label(), "Files");
    }

    #[test]
    fn test_settings_section_icon() {
        assert_eq!(SettingsSection::Appearance.icon(), "🎨");
        assert_eq!(SettingsSection::Editor.icon(), "📝");
        assert_eq!(SettingsSection::Files.icon(), "📁");
    }

    #[test]
    fn test_settings_section_default() {
        let section = SettingsSection::default();
        assert_eq!(section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_output_default() {
        let output = SettingsPanelOutput::default();
        assert!(!output.changed);
        assert!(!output.close_requested);
        assert!(!output.reset_requested);
    }
}
