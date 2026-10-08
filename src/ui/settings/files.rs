//! Files setting render functions (session, quick notes, auto-save, recents).

use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;

use super::registry::SettingsCtx;
use super::SettingsPanel;

/// Session restore toggle.
pub(super) fn render_restore_session(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.restore_session,
        t!("settings.general.restore_session"),
    )
    .on_hover_text(t!("settings.files.restore_session_tooltip"))
    .changed()
}

/// Show Welcome on empty launch toggle.
pub(super) fn render_show_welcome(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.show_welcome_on_empty_launch,
        t!("settings.files.show_welcome"),
    )
    .on_hover_text(t!("settings.files.show_welcome_tooltip"))
    .changed()
}

/// Allow multiple Ferrite processes / windows (skip single-instance forwarding).
pub(super) fn render_allow_multiple_instances(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.allow_multiple_instances,
        t!("settings.files.allow_multiple_instances"),
    )
    .on_hover_text(t!("settings.files.allow_multiple_instances_tooltip"))
    .changed()
}

/// Quick note workflow toggle.
pub(super) fn render_quick_note_workflow(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let changed = ui
        .checkbox(
            &mut ctx.settings.quick_note_workflow,
            t!("settings.files.quick_note_workflow"),
        )
        .on_hover_text(t!("settings.files.quick_note_workflow_tooltip"))
        .changed();
    ui.label(
        RichText::new(t!("settings.files.quick_note_workflow_hint"))
            .weak()
            .small(),
    );
    changed
}

/// Auto-save toggle plus delay slider and presets.
pub(super) fn render_auto_save(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    if ui
        .checkbox(
            &mut settings.auto_save_enabled_default,
            t!("settings.files.enable_auto_save"),
        )
        .on_hover_text(t!("settings.files.auto_save_tooltip"))
        .changed()
    {
        changed = true;
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(t!("settings.files.auto_save_delay"));
        ui.add_space(8.0);
        let secs = settings.auto_save_delay_ms / 1000;
        ui.label(t!("settings.files.seconds", count = secs));
    });
    ui.add_space(4.0);

    // Convert ms to seconds for slider display
    let mut delay_secs = (settings.auto_save_delay_ms / 1000) as f32;
    let delay_slider = ui.add(
        egui::Slider::new(&mut delay_secs, 5.0..=300.0)
            .show_value(false)
            .step_by(5.0),
    );
    if delay_slider.changed() {
        settings.auto_save_delay_ms = (delay_secs as u32) * 1000;
        changed = true;
    }

    // Delay presets
    ui.horizontal(|ui| {
        for (label, ms) in [("15s", 15000), ("30s", 30000), ("1m", 60000)] {
            if ui.small_button(label).clicked() {
                settings.auto_save_delay_ms = ms;
                changed = true;
            }
        }
    });

    changed
}

/// Recent files count slider and clear button.
pub(super) fn render_recent_files(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.files.recent_files")).strong());
        ui.add_space(8.0);
        ui.label(t!(
            "settings.files.remember_files",
            count = settings.max_recent_files
        ));
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

    ui.horizontal(|ui| {
        if ui
            .button(t!("settings.files.clear_recent"))
            .on_hover_text(t!("settings.files.clear_recent_tooltip"))
            .clicked()
        {
            settings.recent_files.clear();
            changed = true;
        }

        if !settings.recent_files.is_empty() {
            ui.label(
                RichText::new(t!(
                    "settings.files.files_count",
                    count = settings.recent_files.len()
                ))
                .small()
                .weak(),
            );
        }
    });

    changed
}
