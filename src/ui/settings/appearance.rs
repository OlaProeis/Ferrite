//! Appearance setting render functions (theme, colors, language, fonts).

use eframe::egui::{self, RichText, Ui};
use rust_i18n::{set_locale, t};

use crate::config::{CjkFontPreference, EditorFont, Language, Settings, Theme, ViewMode};
use crate::fonts;
use crate::markdown::syntax::get_available_themes;
use crate::ui::phosphor_icons::{DESKTOP, GLOBE, MOON, SUN};

use super::registry::SettingsCtx;
use super::SettingsPanel;

/// Localized font description.
fn font_description(font: &EditorFont) -> String {
    match font {
        EditorFont::Inter => t!("settings.editor.font_inter_desc").to_string(),
        EditorFont::JetBrainsMono => t!("settings.editor.font_jetbrains_desc").to_string(),
        EditorFont::Custom(_) => t!("settings.editor.custom_font_desc").to_string(),
    }
}

/// Localized view mode description.
fn view_mode_description(mode: &ViewMode) -> String {
    match mode {
        ViewMode::Raw => t!("view_mode.raw_desc").to_string(),
        ViewMode::Rendered => t!("view_mode.rendered_desc").to_string(),
        ViewMode::Split => t!("view_mode.split_desc").to_string(),
    }
}

/// Theme selector (Light / Dark / System).
pub(super) fn render_theme(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.general.theme")).strong());
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        for theme in [Theme::Light, Theme::Dark, Theme::System] {
            let label = match theme {
                Theme::Light => format!("{} {}", SUN, t!("settings.general.theme_light")),
                Theme::Dark => format!("{} {}", MOON, t!("settings.general.theme_dark")),
                Theme::System => format!("{} {}", DESKTOP, t!("settings.general.theme_system")),
            };
            if ui
                .selectable_value(&mut settings.theme, theme, label)
                .changed()
            {
                changed = true;
            }
        }
    });

    changed
}

/// System title bar toggle (native decorations; unsupported on Windows).
pub(super) fn render_system_title_bar(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    let windows_only_custom_chrome = cfg!(target_os = "windows");
    let response = if windows_only_custom_chrome {
        ui.add_enabled(
            false,
            egui::Checkbox::new(
                &mut settings.use_system_title_bar,
                t!("settings.appearance.system_title_bar"),
            ),
        )
        .on_hover_text(t!("settings.appearance.system_title_bar_windows_tooltip"))
    } else {
        ui.checkbox(
            &mut settings.use_system_title_bar,
            t!("settings.appearance.system_title_bar"),
        )
    };
    if response.changed() {
        changed = true;
    }
    ui.label(
        RichText::new(t!("settings.appearance.system_title_bar_hint"))
            .weak()
            .small(),
    );
    if !windows_only_custom_chrome {
        ui.label(
            RichText::new(t!("settings.appearance.system_title_bar_restart"))
                .weak()
                .small(),
        );
    }

    changed
}

/// Accent color picker with reset.
pub(super) fn render_accent_color(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.appearance.accent_color")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.appearance.accent_color_hint"))
            .weak()
            .small(),
    );
    ui.horizontal(|ui| {
        let mut c = egui::Color32::from_rgb(
            settings.accent_color[0],
            settings.accent_color[1],
            settings.accent_color[2],
        );
        if ui.color_edit_button_srgba(&mut c).changed() {
            settings.accent_color = [c.r(), c.g(), c.b()];
            changed = true;
        }
        ui.add_space(8.0);
        if ui
            .small_button(t!("settings.appearance.accent_reset"))
            .clicked()
        {
            settings.accent_color = crate::theme::accent::DEFAULT_ACCENT_RGB;
            changed = true;
        }
    });

    changed
}

/// Syntax highlighting theme combo.
pub(super) fn render_syntax_theme(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.appearance.syntax_theme")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.appearance.syntax_theme_hint"))
            .weak()
            .small(),
    );
    ui.add_space(4.0);

    let themes = get_available_themes();
    let current_display = if settings.syntax_theme.is_empty() {
        t!("settings.appearance.syntax_theme_auto").to_string()
    } else {
        themes
            .iter()
            .find(|(name, _)| name == &settings.syntax_theme)
            .map(|(_, display)| display.clone())
            .unwrap_or_else(|| settings.syntax_theme.clone())
    };

    egui::ComboBox::from_id_salt("syntax_theme_combo")
        .selected_text(&current_display)
        .width(200.0)
        .show_ui(ui, |ui| {
            ui.set_min_width(200.0);
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    // Auto option (empty string = use dark/light default)
                    if ui
                        .selectable_label(
                            settings.syntax_theme.is_empty(),
                            t!("settings.appearance.syntax_theme_auto"),
                        )
                        .clicked()
                    {
                        settings.syntax_theme = String::new();
                        changed = true;
                    }

                    ui.separator();

                    for (theme_name, display_name) in &themes {
                        if ui
                            .selectable_label(&settings.syntax_theme == theme_name, display_name)
                            .clicked()
                        {
                            settings.syntax_theme = theme_name.clone();
                            changed = true;
                        }
                    }
                });
        });

    changed
}

/// UI language selector.
pub(super) fn render_language(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.appearance.language")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.appearance.language_hint"))
            .weak()
            .small(),
    );
    ui.add_space(4.0);

    let current_lang = settings.language;
    egui::ComboBox::from_id_salt("language_combo")
        .selected_text(format!(
            "{} {}",
            GLOBE,
            current_lang.selector_display_name()
        ))
        .show_ui(ui, |ui| {
            for lang in Language::all() {
                if ui
                    .selectable_value(&mut settings.language, *lang, lang.selector_display_name())
                    .changed()
                {
                    // Apply language change immediately
                    set_locale(settings.language.locale_code());
                    changed = true;
                }
            }
        });

    changed
}

/// Default view mode for new tabs (Raw / Rendered / Split).
pub(super) fn render_default_view_mode(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.preview.default_view")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.default_view_hint"))
            .weak()
            .small(),
    );
    ui.add_space(4.0);

    for view_mode in ViewMode::all() {
        ui.horizontal(|ui| {
            if ui
                .selectable_value(
                    &mut settings.default_view_mode,
                    *view_mode,
                    format!("{} {}", view_mode.icon(), view_mode.label()),
                )
                .changed()
            {
                changed = true;
            }
            ui.label(
                RichText::new(view_mode_description(view_mode))
                    .weak()
                    .small(),
            );
        });
    }

    changed
}

/// Editor font family selection (built-in and custom system fonts).
pub(super) fn render_font_family(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.editor.font_family")).strong());
    ui.add_space(4.0);

    // Built-in fonts
    for font in EditorFont::builtin_fonts() {
        ui.horizontal(|ui| {
            if ui
                .selectable_value(&mut settings.font_family, font.clone(), font.display_name())
                .changed()
            {
                changed = true;
            }
            ui.label(RichText::new(font_description(font)).weak().small());
        });
    }

    // Custom font option
    let is_custom = settings.font_family.is_custom();
    let custom_label = t!("settings.editor.custom_font");
    ui.horizontal(|ui| {
        if ui
            .selectable_label(is_custom, custom_label.to_string())
            .clicked()
            && !is_custom
        {
            // Defer loading until the user picks a family — the first sorted name from
            // enumeration may not resolve via font-kit on some macOS setups (GitHub #133).
            settings.font_family = EditorFont::Custom(String::new());
            changed = true;
        }
        ui.label(
            RichText::new(t!("settings.editor.custom_font_desc"))
                .weak()
                .small(),
        );
    });

    if settings.font_family.is_custom()
        && custom_font_picker(ui, &mut settings.font_family, "system_font_combo")
    {
        changed = true;
    }

    changed
}

/// Rendered-view font: same as editor, or an independent built-in/custom pick.
pub(super) fn render_rendered_font_family(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.editor.rendered_font_family")).strong());
    ui.add_space(4.0);

    let mut same_as_editor = settings.rendered_font_family.is_none();
    if ui
        .checkbox(
            &mut same_as_editor,
            t!("settings.editor.same_as_editor").to_string(),
        )
        .changed()
    {
        if same_as_editor {
            settings.rendered_font_family = None;
        } else {
            settings.rendered_font_family = Some(settings.font_family.clone());
        }
        changed = true;
    }

    if let Some(ref mut rendered) = settings.rendered_font_family {
        ui.add_space(4.0);
        for font in EditorFont::builtin_fonts() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(rendered, font.clone(), font.display_name())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(font_description(font)).weak().small());
            });
        }

        let is_custom = rendered.is_custom();
        let custom_label = t!("settings.editor.custom_font");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(is_custom, custom_label.to_string())
                .clicked()
                && !is_custom
            {
                *rendered = EditorFont::Custom(String::new());
                changed = true;
            }
            ui.label(
                RichText::new(t!("settings.editor.custom_font_desc"))
                    .weak()
                    .small(),
            );
        });

        if rendered.is_custom() && custom_font_picker(ui, rendered, "rendered_system_font_combo") {
            changed = true;
        }
    }

    changed
}

/// System-font ComboBox + preview for a `Custom` [`EditorFont`].
pub(super) fn custom_font_picker(ui: &mut Ui, font: &mut EditorFont, id_salt: &str) -> bool {
    let current_font_name = match font {
        EditorFont::Custom(name) => name.clone(),
        _ => return false,
    };
    let system_fonts = fonts::list_system_fonts();
    let font_found = !current_font_name.trim().is_empty()
        && system_fonts.iter().any(|f| f == &current_font_name);

    let mut changed = false;
    ui.add_space(4.0);
    ui.indent(id_salt, |ui| {
        ui.label(RichText::new(t!("settings.editor.select_system_font")).small());
        ui.add_space(2.0);

        let combo_label = if current_font_name.trim().is_empty() {
            t!("settings.editor.custom_font_pick_placeholder").to_string()
        } else {
            current_font_name.clone()
        };

        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(combo_label)
            .width(200.0)
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for font_name in system_fonts {
                            if ui
                                .selectable_label(font_name == &current_font_name, font_name)
                                .clicked()
                            {
                                *font = EditorFont::Custom(font_name.to_string());
                                changed = true;
                            }
                        }
                    });
            });

        ui.add_space(4.0);
        ui.label(RichText::new(t!("settings.editor.font_preview")).small());
        ui.label(
            RichText::new("The quick brown fox jumps over the lazy dog. 0123456789").size(14.0),
        );
        if !current_font_name.trim().is_empty() && !font_found {
            ui.label(
                RichText::new(t!("settings.editor.font_not_found"))
                    .color(ui.visuals().error_fg_color)
                    .small(),
            );
        }
    });

    changed
}

/// Editor font size slider with presets.
pub(super) fn render_font_size(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.font_size")).strong());
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
        for (label, size) in [
            (t!("settings.font_size.small"), 12.0),
            (t!("settings.font_size.medium"), 14.0),
            (t!("settings.font_size.large"), 18.0),
        ] {
            if ui.small_button(label).clicked() {
                settings.font_size = size;
                changed = true;
            }
        }
    });

    changed
}

/// Per-script font preferences for complex scripts.
pub(super) fn render_complex_scripts(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.editor.complex_scripts")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.editor.complex_scripts_hint"))
            .weak()
            .small(),
    );
    ui.add_space(4.0);

    const COMPLEX_SCRIPTS: &[(&str, &str)] = &[
        ("arabic", "Arabic"),
        ("bengali", "Bengali"),
        ("devanagari", "Devanagari"),
        ("thai", "Thai"),
        ("hebrew", "Hebrew"),
        ("tamil", "Tamil"),
        ("georgian", "Georgian"),
        ("armenian", "Armenian"),
        ("ethiopic", "Ethiopic"),
        ("other_indic", "Other Indic"),
        ("southeast_asian", "Southeast Asian"),
    ];

    for (key, display_name) in COMPLEX_SCRIPTS {
        let current = settings
            .complex_script_font_preferences
            .get(*key)
            .cloned()
            .unwrap_or_default();
        let display = if current.is_empty() {
            t!("settings.editor.complex_script_default").to_string()
        } else {
            current.clone()
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new(*display_name).small());
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt(format!("complex_script_{}", key))
                .selected_text(&display)
                .width(180.0)
                .show_ui(ui, |ui| {
                    ui.set_min_width(180.0);
                    if ui
                        .selectable_label(
                            current.is_empty(),
                            t!("settings.editor.complex_script_default"),
                        )
                        .clicked()
                    {
                        settings.complex_script_font_preferences.remove(*key);
                        changed = true;
                    }
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for font_name in fonts::list_system_fonts() {
                                if ui
                                    .selectable_label(font_name == &current, font_name)
                                    .clicked()
                                {
                                    settings
                                        .complex_script_font_preferences
                                        .insert((*key).to_string(), font_name.clone());
                                    changed = true;
                                }
                            }
                        });
                });
        });
    }

    changed
}

/// CJK regional font preference combo.
pub(super) fn render_cjk_preference(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.editor.cjk_preference")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.editor.cjk_preference_hint"))
            .weak()
            .small(),
    );
    ui.add_space(4.0);

    egui::ComboBox::from_id_salt("cjk_preference_combo")
        .selected_text(
            settings
                .cjk_font_preference
                .selector_display_name()
                .to_string(),
        )
        .show_ui(ui, |ui| {
            for pref in CjkFontPreference::all() {
                let label = format!("{} - {}", pref.selector_display_name(), pref.description());
                if ui
                    .selectable_value(&mut settings.cjk_font_preference, *pref, label)
                    .changed()
                {
                    changed = true;
                }
            }
        });

    changed
}
