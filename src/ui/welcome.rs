//! Welcome Panel Component for Ferrite
//!
//! This module implements a modal Welcome panel that displays:
//! - Application information and version
//! - GitHub and documentation links
//! - Complete keyboard shortcuts reference
//! - Credits and license information

use crate::app::modifier_symbol;
use crate::config::{ Settings, Theme, CjkFontPreference, MaxLineWidth };
use eframe::egui::{ self, Color32, RichText, ScrollArea, Ui };
use rust_i18n::t;

/// Get localized CJK preference display name
fn cjk_display_name(pref: &CjkFontPreference) -> String {
    match pref {
        CjkFontPreference::Auto => t!("settings.editor.cjk_auto").to_string(),
        CjkFontPreference::Korean => t!("settings.editor.cjk_korean").to_string(),
        CjkFontPreference::SimplifiedChinese => {
            t!("settings.editor.cjk_simplified_chinese").to_string()
        }
        CjkFontPreference::TraditionalChinese => {
            t!("settings.editor.cjk_traditional_chinese").to_string()
        }
        CjkFontPreference::Japanese => t!("settings.editor.cjk_japanese").to_string(),
    }
}

/// Get localized CJK preference description
fn cjk_description(pref: &CjkFontPreference) -> String {
    match pref {
        CjkFontPreference::Auto => t!("settings.editor.cjk_auto_desc").to_string(),
        CjkFontPreference::Korean => t!("settings.editor.cjk_korean_desc").to_string(),
        CjkFontPreference::SimplifiedChinese => {
            t!("settings.editor.cjk_simplified_chinese_desc").to_string()
        }
        CjkFontPreference::TraditionalChinese => {
            t!("settings.editor.cjk_traditional_chinese_desc").to_string()
        }
        CjkFontPreference::Japanese => t!("settings.editor.cjk_japanese_desc").to_string(),
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

/// Welcome panel sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WelcomeSection {
    #[default]
    Welcome,
}

impl WelcomeSection {
    /// Get the display label for the section.
    pub fn label(&self) -> String {
        (
            match self {
                WelcomeSection::Welcome => t!("welcome.tab.welcome"),
            }
        ).to_string()
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        match self {
            WelcomeSection::Welcome => "○",
        }
    }
}

/// Result of showing the welcome panel.
#[derive(Debug, Clone, Default)]
pub struct WelcomePanelOutput {
    /// Whether the panel should be closed.
    pub close_requested: bool,
}

/// welcome panel state and rendering.
#[derive(Debug, Clone)]
pub struct WelcomePanel {
    /// Currently active section.
    active_section: WelcomeSection,
}

impl Default for WelcomePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl WelcomePanel {
    /// Create a new welcome panel instance.
    pub fn new() -> Self {
        Self {
            active_section: WelcomeSection::default(),
        }
    }

    /// # Arguments
    ///
    /// * `ctx` - The egui context
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Output indicating what actions to take
    pub fn show(&mut self, ctx: &egui::Context, is_dark: bool) -> WelcomePanelOutput {
        let mut output = WelcomePanelOutput::default();

        // Semi-transparent overlay
        let screen_rect = ctx.screen_rect();
        let overlay_color = if is_dark {
            Color32::from_rgba_unmultiplied(0, 0, 0, 180)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 120)
        };

        egui::Area
            ::new(egui::Id::new("about_overlay"))
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

        output
    }

    /// Render the welcome panel inline within a tab (not as a modal window).
    ///
    /// This is used when welcome is displayed as a special tab in the main
    /// editor area, giving more screen real estate than the modal version.

    /// Get localized CJK preference display name

    // ... your layout code ...

    pub fn show_inline(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        // Padding around the whole welcome panel
        egui::Frame
            ::none()
            .inner_margin(egui::Margin {
                left: 100.0,
                right: 40.0,
                top: 100.0,
                bottom: 40.0,
            }) // <- change this value for more/less padding
            .show(ui, |ui| {
                // (Optional) keep content from stretching too wide
                ui.set_max_width(520.0);

                let system_dark = ui.ctx().style().visuals.dark_mode;

                let is_dark = match settings.theme {
                    Theme::Dark => true,
                    Theme::Light => false,
                    Theme::System => system_dark,
                };

                let text_color = if is_dark {
                    Color32::from_rgb(235, 235, 235)
                } else {
                    Color32::from_rgb(25, 25, 25)
                };

                ui.label(RichText::new("Ferrite").size(40.0).strong().color(text_color));

                ui.add_space(30.0);

                ui.horizontal(|ui| {
                    for theme in [Theme::Light, Theme::Dark, Theme::System] {
                        let label = match theme {
                            Theme::Light => format!("☀ {}", t!("settings.general.theme_light")),
                            Theme::Dark => format!("🌙 {}", t!("settings.general.theme_dark")),
                            Theme::System => format!("💻 {}", t!("settings.general.theme_system")),
                        };
                        if ui.selectable_value(&mut settings.theme, theme, label).changed() {
                            changed = true;
                        }
                    }
                });

                ui.add_space(30.0);

                ui.label(RichText::new(t!("settings.editor.cjk_preference_hint")).weak().small());

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t!("settings.editor.max_line_width"))
                            .strong()
                            .color(text_color)
                    );
                    ui.add_space(8.0);

                    let current_display = settings.max_line_width.display_name();
                    egui::ComboBox
                        ::from_id_source("max_line_width_combo")
                        .selected_text(current_display)
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for preset in MaxLineWidth::presets() {
                                let label = format!(
                                    "{} - {}",
                                    preset.display_name(),
                                    preset.description()
                                );
                                if
                                    ui
                                        .selectable_value(
                                            &mut settings.max_line_width,
                                            *preset,
                                            label
                                        )
                                        .changed()
                                {
                                    changed = true;
                                }
                            }
                            let is_custom = settings.max_line_width.is_custom();
                            let custom_label = t!("settings.editor.custom_width");
                            if
                                ui
                                    .selectable_label(is_custom, custom_label.to_string())
                                    .clicked() &&
                                !is_custom
                            {
                                settings.max_line_width = MaxLineWidth::Custom(800);
                                changed = true;
                            }
                        });

                    // Show inline numeric input when custom is selected
                    if let MaxLineWidth::Custom(px) = &mut settings.max_line_width {
                        let mut px_value = *px as f32;
                        let drag = ui.add(
                            egui::DragValue
                                ::new(&mut px_value)
                                .speed(10.0)
                                .range(
                                    Settings::MIN_CUSTOM_LINE_WIDTH as f32..=Settings::MAX_CUSTOM_LINE_WIDTH as f32
                                )
                                .suffix("px")
                        );
                        if drag.changed() {
                            *px = px_value as u32;
                            changed = true;
                        }
                    }
                });

                ui.add_space(30.0);

                // Two-column grid for basic toggles using egui::Grid for proper alignment
                egui::Grid
                    ::new("editor_toggles_grid")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .min_col_width(180.0)
                    .show(ui, |ui| {
                        // Row 1: Word Wrap | Show Line Numbers
                        if
                            ui
                                .checkbox(&mut settings.word_wrap, t!("settings.editor.word_wrap"))
                                .on_hover_text(t!("settings.editor.word_wrap_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        if
                            ui
                                .checkbox(
                                    &mut settings.show_line_numbers,
                                    t!("settings.editor.show_line_numbers")
                                )
                                .on_hover_text(t!("settings.editor.line_numbers_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 2: Show Minimap
                        if
                            ui
                                .checkbox(
                                    &mut settings.minimap_enabled,
                                    t!("settings.editor.show_minimap")
                                )
                                .on_hover_text(t!("settings.editor.minimap_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 3: Highlight Brackets | Auto-close Brackets
                        if
                            ui
                                .checkbox(
                                    &mut settings.highlight_matching_pairs,
                                    t!("settings.editor.highlight_brackets")
                                )
                                .on_hover_text(t!("settings.editor.brackets_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        if
                            ui
                                .checkbox(
                                    &mut settings.auto_close_brackets,
                                    t!("settings.editor.auto_close_brackets")
                                )
                                .on_hover_text(t!("settings.editor.auto_close_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 4: Syntax Highlighting | Use Spaces
                        if
                            ui
                                .checkbox(
                                    &mut settings.syntax_highlighting_enabled,
                                    t!("settings.editor.syntax_highlighting")
                                )
                                .on_hover_text(t!("settings.editor.syntax_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        if
                            ui
                                .checkbox(
                                    &mut settings.use_spaces,
                                    t!("settings.editor.use_spaces")
                                )
                                .on_hover_text(t!("settings.editor.use_spaces_tooltip"))
                                .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();
                    });

                ui.add_space(30.0);

                egui::ComboBox
                    ::from_id_source("cjk_preference_combo")
                    .selected_text(cjk_display_name(&settings.cjk_font_preference))
                    .show_ui(ui, |ui| {
                        for pref in CjkFontPreference::all() {
                            let label = format!(
                                "{} - {}",
                                cjk_display_name(pref),
                                cjk_description(pref)
                            );
                            if
                                ui
                                    .selectable_value(
                                        &mut settings.cjk_font_preference,
                                        *pref,
                                        label
                                    )
                                    .changed()
                            {
                                changed = true;
                            }
                        }
                    });

                ui.add_space(30.0);

                if
                    ui
                        .checkbox(
                            &mut settings.auto_save_enabled_default,
                            t!("settings.files.enable_auto_save")
                        )
                        .on_hover_text(t!("settings.files.auto_save_tooltip"))
                        .changed()
                {
                    changed = true;
                }
            });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_panel_new() {
        let panel = WelcomePanel::new();
        assert_eq!(panel.active_section, WelcomeSection::Welcome);
    }

    #[test]
    fn test_welcome_panel_default() {
        let panel = WelcomePanel::default();
        assert_eq!(panel.active_section, WelcomeSection::Welcome);
    }

    #[test]
    fn test_welcome_default() {
        let output = WelcomePanelOutput::default();
    }
}
