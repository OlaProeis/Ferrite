//! Terminal setting render functions.

use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;

use crate::config::EditorFont;

use super::appearance::custom_font_picker;
use super::registry::SettingsCtx;
use super::SettingsPanel;

/// Terminal enabled toggle.
pub(super) fn render_enabled(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.terminal_enabled,
        t!("settings.terminal.enable").to_string(),
    )
    .changed()
}

/// Terminal font size slider.
pub(super) fn render_font_size(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.terminal.font_size").to_string()).strong());
        ui.add_space(8.0);
        ui.label(format!("{}px", settings.terminal_font_size as u32));
    });
    ui.add_space(4.0);

    if ui
        .add(
            egui::Slider::new(&mut settings.terminal_font_size, 10.0..=32.0)
                .show_value(false)
                .step_by(1.0),
        )
        .changed()
    {
        changed = true;
    }

    changed
}

/// Terminal font family: default JetBrains Mono, or a system face (prefer Mono Nerd Fonts).
pub(super) fn render_font_family(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.font_family").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.font_family_hint").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    let use_default = settings.terminal_font_family.is_none();
    if ui
        .selectable_label(
            use_default,
            t!("settings.terminal.font_family_default").to_string(),
        )
        .clicked()
        && !use_default
    {
        settings.terminal_font_family = None;
        changed = true;
    }

    let is_custom = settings.terminal_font_family.is_some();
    if ui
        .selectable_label(is_custom, t!("settings.editor.custom_font").to_string())
        .clicked()
        && !is_custom
    {
        settings.terminal_font_family = Some(String::new());
        changed = true;
    }

    if let Some(name) = settings.terminal_font_family.as_mut() {
        let mut font = EditorFont::Custom(name.clone());
        if custom_font_picker(ui, &mut font, "terminal_system_font_combo") {
            if let EditorFont::Custom(picked) = font {
                *name = picked;
                changed = true;
            }
        }
    }

    changed
}

/// Scrollback lines slider.
pub(super) fn render_scrollback(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.terminal.scrollback").to_string()).strong());
        ui.add_space(8.0);
        ui.label(format!("{}", settings.terminal_scrollback_lines));
    });
    ui.add_space(4.0);

    let mut scrollback_val = settings.terminal_scrollback_lines as f64;
    let scrollback_slider = ui.add(
        egui::Slider::new(&mut scrollback_val, 1000.0..=50000.0)
            .show_value(false)
            .step_by(1000.0),
    );
    if scrollback_slider.changed() {
        settings.terminal_scrollback_lines = scrollback_val as usize;
        changed = true;
    }

    changed
}

/// Copy-on-select toggle.
pub(super) fn render_copy_on_select(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.terminal_copy_on_select,
        t!("settings.terminal.copy_selection").to_string(),
    )
    .on_hover_text(t!("settings.terminal.copy_selection_tooltip").to_string())
    .changed()
}

/// Terminal color theme combo.
pub(super) fn render_theme(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.theme").to_string()).strong());
    ui.add_space(4.0);

    egui::ComboBox::from_id_salt("terminal_theme_combo")
        .selected_text(&settings.terminal_theme_name)
        .show_ui(ui, |ui| {
            for theme in crate::terminal::TerminalTheme::all() {
                if ui
                    .selectable_value(
                        &mut settings.terminal_theme_name,
                        theme.name.clone(),
                        &theme.name,
                    )
                    .changed()
                {
                    changed = true;
                }
            }
        });

    changed
}

/// Terminal opacity slider.
pub(super) fn render_opacity(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.terminal.opacity").to_string()).strong());
        ui.add_space(8.0);
        ui.label(format!("{:.0}%", settings.terminal_opacity * 100.0));
    });
    ui.add_space(4.0);

    if ui
        .add(egui::Slider::new(&mut settings.terminal_opacity, 0.1..=1.0).show_value(false))
        .changed()
    {
        changed = true;
    }

    changed
}

/// Startup command text field.
pub(super) fn render_startup_command(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.startup_command").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.startup_command_desc").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    if ui
        .add(
            egui::TextEdit::singleline(&mut settings.terminal_startup_command)
                .hint_text(t!("settings.terminal.startup_command_hint").to_string()),
        )
        .changed()
    {
        changed = true;
    }

    changed
}

/// Detected monitor information (read-only).
pub(super) fn render_monitors(
    panel: &mut SettingsPanel,
    ui: &mut Ui,
    _ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.label(RichText::new(t!("settings.terminal.monitors").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.monitors_desc").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    if panel.cached_monitor_info.is_none() {
        panel.cached_monitor_info = Some(crate::terminal::detect_monitors());
    }
    let monitors = panel.cached_monitor_info.as_ref().unwrap();

    egui::Frame::NONE
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            for (i, m) in monitors.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(t!("settings.terminal.monitor_label", index = i + 1).to_string());
                    ui.label(RichText::new(&m.name).strong());
                    ui.label(
                        t!(
                            "settings.terminal.monitor_geometry",
                            width = m.width as u32,
                            height = m.height as u32,
                            x = m.x as i32,
                            y = m.y as i32
                        )
                        .to_string(),
                    );
                });
            }
        });

    false
}

/// Breathing color picker.
pub(super) fn render_breathing_color(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.terminal.breathing_color").to_string()).strong());
        ui.add_space(8.0);
        if ui
            .color_edit_button_srgba(&mut settings.terminal_breathing_color)
            .changed()
        {
            changed = true;
        }
    });

    changed
}

/// Prompt detection patterns (multiline).
pub(super) fn render_prompt_patterns(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.prompt_patterns").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.prompt_patterns_desc").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    let mut patterns_text = settings.terminal_prompt_patterns.join("\n");
    if ui
        .add(
            egui::TextEdit::multiline(&mut patterns_text)
                .desired_rows(3)
                .hint_text(t!("settings.terminal.prompt_patterns_hint").to_string()),
        )
        .changed()
    {
        settings.terminal_prompt_patterns = patterns_text
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();
        changed = true;
    }

    changed
}

/// Auto-load saved terminal layout toggle.
pub(super) fn render_auto_load_layout(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.terminal_auto_load_layout,
        t!("settings.terminal.auto_load_layout").to_string(),
    )
    .on_hover_text(t!("settings.terminal.auto_load_layout_tooltip").to_string())
    .changed()
}

/// Sound notification toggle plus custom sound file.
pub(super) fn render_sound(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.sound_notification").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.sound_notification_desc").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    if ui
        .checkbox(
            &mut settings.terminal_sound_enabled,
            t!("settings.terminal.enable_sound").to_string(),
        )
        .on_hover_text(t!("settings.terminal.enable_sound_tooltip").to_string())
        .changed()
    {
        changed = true;
    }

    if settings.terminal_sound_enabled {
        ui.add_space(4.0);
        ui.indent("sound_file_settings", |ui| {
            ui.label(RichText::new(t!("settings.terminal.custom_sound").to_string()).small());
            let mut sound_path = settings.terminal_sound_file.clone().unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut sound_path)
                        .hint_text(t!("settings.terminal.custom_sound_hint").to_string()),
                )
                .changed()
            {
                settings.terminal_sound_file = if sound_path.is_empty() {
                    None
                } else {
                    Some(sound_path)
                };
                changed = true;
            }
        });
    }

    changed
}

/// Focus terminal on prompt detection toggle.
pub(super) fn render_focus_on_detect(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.terminal.auto_focus").to_string()).strong());
    ui.label(
        RichText::new(t!("settings.terminal.auto_focus_desc").to_string())
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    if ui
        .checkbox(
            &mut settings.terminal_focus_on_detect,
            t!("settings.terminal.focus_on_prompt").to_string(),
        )
        .on_hover_text(t!("settings.terminal.focus_on_prompt_tooltip").to_string())
        .changed()
    {
        changed = true;
    }

    changed
}
