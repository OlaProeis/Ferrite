//! Welcome Panel Component for Ferrite
//!
//! This module implements the Welcome panel displayed on first launch,
//! allowing users to configure theme, language, fonts, and editor preferences.

use crate::config::{
    CjkFontPreference, KeyBinding, KeyCode, KeyModifiers, Language, MaxLineWidth, Settings,
    ShortcutCommand, Theme, ViewMode,
};
use eframe::egui::{self, Color32, RichText, Ui};
use rust_i18n::{set_locale, t};

/// Welcome panel state and rendering.
#[derive(Debug, Clone, Default)]
pub struct WelcomePanel;

impl WelcomePanel {
    /// Create a new welcome panel instance.
    pub fn new() -> Self {
        Self
    }

    /// Render a section heading with consistent styling.
    fn section_heading(ui: &mut Ui, text: &str, text_color: Color32) {
        ui.add_space(24.0);
        ui.label(RichText::new(text).size(15.0).strong().color(text_color));
        ui.add_space(6.0);
    }

    /// Render a setting row: checkbox on the left, description on the right.
    fn setting_toggle(
        ui: &mut Ui,
        value: &mut bool,
        label: &str,
        description: &str,
        weak_color: Color32,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui.checkbox(value, label).changed() {
                changed = true;
            }
            ui.label(RichText::new(description).weak().small().color(weak_color));
        });
        changed
    }

    /// Render the welcome panel inline within a tab.
    ///
    /// Returns `true` if any settings were changed.
    pub fn show_inline(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin {
                    left: 80.0,
                    right: 40.0,
                    top: 60.0,
                    bottom: 40.0,
                })
                .show(ui, |ui| {
                    ui.set_max_width(560.0);

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
                    let weak_color = if is_dark {
                        Color32::from_rgb(160, 160, 160)
                    } else {
                        Color32::from_rgb(110, 110, 110)
                    };

                    // ── Title ──────────────────────────────────────────
                    ui.label(
                        RichText::new("Ferrite")
                            .size(36.0)
                            .strong()
                            .color(text_color),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(t!("welcome.subtitle").to_string())
                            .size(14.0)
                            .color(weak_color),
                    );

                    // ── Theme ─────────────────────────────────────────
                    Self::section_heading(ui, &t!("welcome.section.appearance").to_string(), text_color);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t!("welcome.label.theme").to_string())
                                .strong()
                                .color(text_color),
                        );
                        ui.add_space(8.0);
                        for theme in [Theme::Light, Theme::Dark, Theme::System] {
                            let label = match theme {
                                Theme::Light => {
                                    format!("  {}  ", t!("settings.general.theme_light").to_string())
                                }
                                Theme::Dark => {
                                    format!("  {}  ", t!("settings.general.theme_dark").to_string())
                                }
                                Theme::System => {
                                    format!("  {}  ", t!("settings.general.theme_system").to_string())
                                }
                            };
                            if ui
                                .selectable_value(&mut settings.theme, theme, label)
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });

                    // ── Language ───────────────────────────────────────
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t!("welcome.label.language").to_string())
                                .strong()
                                .color(text_color),
                        );
                        ui.add_space(8.0);

                        let current_lang = settings.language;
                        egui::ComboBox::from_id_source("welcome_language_combo")
                            .selected_text(current_lang.selector_display_name())
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for lang in Language::all() {
                                    if ui
                                        .selectable_value(
                                            &mut settings.language,
                                            *lang,
                                            lang.selector_display_name(),
                                        )
                                        .changed()
                                    {
                                        set_locale(settings.language.locale_code());
                                        changed = true;
                                    }
                                }
                            });
                    });

                    // ── Editor ─────────────────────────────────────────
                    Self::section_heading(ui, &t!("welcome.section.editor").to_string(), text_color);

                    if Self::setting_toggle(
                        ui,
                        &mut settings.word_wrap,
                        &t!("settings.editor.word_wrap").to_string(),
                        &t!("settings.editor.word_wrap_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.show_line_numbers,
                        &t!("settings.editor.show_line_numbers").to_string(),
                        &t!("settings.editor.line_numbers_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.minimap_enabled,
                        &t!("settings.editor.show_minimap").to_string(),
                        &t!("settings.editor.minimap_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.highlight_matching_pairs,
                        &t!("settings.editor.highlight_brackets").to_string(),
                        &t!("settings.editor.brackets_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.auto_close_brackets,
                        &t!("settings.editor.auto_close_brackets").to_string(),
                        &t!("settings.editor.auto_close_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.syntax_highlighting_enabled,
                        &t!("settings.editor.syntax_highlighting").to_string(),
                        &t!("settings.editor.syntax_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.use_spaces,
                        &t!("settings.editor.use_spaces").to_string(),
                        &t!("settings.editor.use_spaces_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    if Self::setting_toggle(
                        ui,
                        &mut settings.strict_line_breaks,
                        "Strict Line Breaks",
                        "Treat single newlines as hard line breaks in rendered view",
                        weak_color,
                    ) {
                        changed = true;
                    }

                    // ── Default View Mode ───────────────────────────────
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t!("settings.preview.default_view").to_string())
                                .strong()
                                .color(text_color),
                        );
                        ui.add_space(8.0);
                        for mode in ViewMode::all() {
                            let label = format!("  {} {}  ", mode.icon(), mode.label());
                            if ui
                                .selectable_value(&mut settings.default_view_mode, *mode, label)
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                    ui.label(
                        RichText::new(t!("settings.default_view_hint").to_string())
                            .weak()
                            .small()
                            .color(weak_color),
                    );

                    // ── Line Width ─────────────────────────────────────
                    Self::section_heading(ui, &t!("settings.editor.max_line_width").to_string(), text_color);

                    ui.label(
                        RichText::new(t!("welcome.line_width_hint").to_string())
                            .weak()
                            .small()
                            .color(weak_color),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        let current_display = settings.max_line_width.display_name();
                        egui::ComboBox::from_id_source("welcome_max_line_width_combo")
                            .selected_text(current_display)
                            .width(160.0)
                            .show_ui(ui, |ui| {
                                for preset in MaxLineWidth::presets() {
                                    let label = format!(
                                        "{} - {}",
                                        preset.display_name(),
                                        preset.description()
                                    );
                                    if ui
                                        .selectable_value(
                                            &mut settings.max_line_width,
                                            *preset,
                                            label,
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                }
                                let is_custom = settings.max_line_width.is_custom();
                                let custom_label = t!("settings.editor.custom_width").to_string();
                                if ui
                                    .selectable_label(is_custom, custom_label)
                                    .clicked()
                                    && !is_custom
                                {
                                    settings.max_line_width = MaxLineWidth::Custom(800);
                                    changed = true;
                                }
                            });

                        // Show inline numeric input when custom is selected
                        if let MaxLineWidth::Custom(px) = &mut settings.max_line_width {
                            let mut px_value = *px as f32;
                            let drag = ui.add(
                                egui::DragValue::new(&mut px_value)
                                    .speed(10.0)
                                    .range(
                                        Settings::MIN_CUSTOM_LINE_WIDTH as f32
                                            ..=Settings::MAX_CUSTOM_LINE_WIDTH as f32,
                                    )
                                    .suffix("px"),
                            );
                            if drag.changed() {
                                *px = px_value as u32;
                                changed = true;
                            }
                        }
                    });

                    // ── CJK Font Preference ───────────────────────────
                    Self::section_heading(
                        ui,
                        &t!("welcome.section.cjk").to_string(),
                        text_color,
                    );

                    ui.label(
                        RichText::new(t!("settings.editor.cjk_preference_hint").to_string())
                            .weak()
                            .small()
                            .color(weak_color),
                    );
                    ui.add_space(6.0);

                    egui::ComboBox::from_id_source("welcome_cjk_preference_combo")
                        .selected_text(settings.cjk_font_preference.selector_display_name())
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for pref in CjkFontPreference::all() {
                                if ui
                                    .selectable_value(
                                        &mut settings.cjk_font_preference,
                                        *pref,
                                        pref.selector_display_name(),
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });

                    // ── Command Palette ──────────────────────────────
                    Self::section_heading(ui, "Command Palette", text_color);

                    ui.label(
                        RichText::new("Quick access to all commands. Press the shortcut to open a searchable command list.")
                            .weak()
                            .small()
                            .color(weak_color),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Shortcut")
                                .strong()
                                .color(text_color),
                        );
                        ui.add_space(8.0);

                        let current_binding = settings.keyboard_shortcuts.get(ShortcutCommand::CommandPalette);
                        let binding_str = current_binding.display_string();

                        // Show preset options
                        let presets: &[(&str, KeyBinding)] = &[
                            ("Alt+Space", KeyBinding::new(KeyModifiers::alt(), KeyCode::Space)),
                            ("Ctrl+Shift+P", KeyBinding::new(KeyModifiers::ctrl_shift(), KeyCode::P)),
                        ];

                        for (label, binding) in presets {
                            let is_current = current_binding == *binding;
                            if ui.selectable_label(is_current, *label).clicked() && !is_current {
                                settings.keyboard_shortcuts.set(ShortcutCommand::CommandPalette, *binding);
                                changed = true;
                            }
                        }

                        // Show current if custom (not matching any preset)
                        let is_custom = !presets.iter().any(|(_, b)| current_binding == *b);
                        if is_custom {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("Custom: {}", binding_str))
                                    .color(weak_color)
                                    .small(),
                            );
                        }
                    });

                    ui.label(
                        RichText::new("You can change this later in Settings > Keyboard Shortcuts")
                            .weak()
                            .small()
                            .color(weak_color),
                    );

                    // ── Files ──────────────────────────────────────────
                    Self::section_heading(ui, &t!("welcome.section.files").to_string(), text_color);

                    if Self::setting_toggle(
                        ui,
                        &mut settings.auto_save_enabled_default,
                        &t!("settings.files.enable_auto_save").to_string(),
                        &t!("settings.files.auto_save_tooltip").to_string(),
                        weak_color,
                    ) {
                        changed = true;
                    }

                    ui.add_space(40.0);
                });
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_panel_new() {
        let _panel = WelcomePanel::new();
    }

    #[test]
    fn test_welcome_panel_default() {
        let _panel = WelcomePanel::default();
    }
}
