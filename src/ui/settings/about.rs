//! About popup for the settings panel: version info, update check, and links.
//!
//! Opened from the corner button in the settings header (or via search).

use eframe::egui::{self, Color32, RichText, Ui};
use rust_i18n::t;

use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::{CHECK, CONFETTI, INFO, WARNING};
use crate::update::{self, UpdateCheckResult, UpdateState};

use super::registry::SettingsCtx;
use super::SettingsPanel;

/// Search-result row for "About": short version line plus a button that opens
/// the About popup.
pub(super) fn render_about_link(
    panel: &mut SettingsPanel,
    ui: &mut Ui,
    _ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ferrite").strong());
        ui.label(
            RichText::new(format!("v{}", update::current_version()))
                .monospace()
                .small(),
        );
        ui.add_space(8.0);
        if ui
            .button(format!("{} {}", INFO, t!("settings.about.title")))
            .clicked()
        {
            panel.about_open = true;
        }
    });
    false
}

/// Render the About popup window (no-op unless open).
pub(super) fn about_window(panel: &mut SettingsPanel, ctx: &egui::Context) {
    if !panel.about_open {
        return;
    }

    let mut open = true;
    egui::Window::new(format!("{} {}", INFO, t!("settings.about.title")))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            about_contents(panel, ui, ctx);
        });
    panel.about_open = open;
}

/// About contents: version info, update check, and project links.
fn about_contents(panel: &mut SettingsPanel, ui: &mut Ui, ctx: &egui::Context) {
    // Poll for update check result if we have a pending check
    if let Some(rx) = &panel.update_check_rx {
        if let Ok(result) = rx.try_recv() {
            match result {
                UpdateCheckResult::UpToDate => {
                    panel.update_state = UpdateState::UpToDate;
                }
                UpdateCheckResult::UpdateAvailable {
                    version,
                    release_url,
                    ..
                } => {
                    panel.update_state = UpdateState::UpdateAvailable {
                        version,
                        release_url,
                    };
                }
                UpdateCheckResult::Error(msg) => {
                    panel.update_state = UpdateState::Error(msg);
                }
            }
            panel.update_check_rx = None;
        }
    }

    // Request repaint while checking so we poll the channel
    if matches!(panel.update_state, UpdateState::Checking) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    // Application info
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ferrite").strong().size(16.0));
        ui.label(
            RichText::new(format!("v{}", update::current_version()))
                .monospace()
                .size(14.0),
        );
    });
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.about.description"))
            .weak()
            .small(),
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    // Check for Updates section
    ui.label(RichText::new(t!("settings.about.updates")).strong());
    ui.add_space(8.0);

    match &panel.update_state {
        UpdateState::Idle => {
            if ui
                .button(t!("settings.about.check_for_updates").to_string())
                .clicked()
            {
                panel.update_state = UpdateState::Checking;
                panel.update_check_rx = Some(update::spawn_update_check());
            }
        }
        UpdateState::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("settings.about.checking"));
            });
        }
        UpdateState::UpToDate => {
            let success_color = if ui.visuals().dark_mode {
                Color32::from_rgb(75, 210, 100)
            } else {
                Color32::from_rgb(40, 167, 69)
            };
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(CHECK, 14.0).color(success_color));
                ui.label(
                    RichText::new(t!("settings.about.up_to_date").to_string()).color(success_color),
                );
            });
            ui.add_space(8.0);
            if ui.small_button(t!("settings.about.check_again")).clicked() {
                panel.update_state = UpdateState::Checking;
                panel.update_check_rx = Some(update::spawn_update_check());
            }
        }
        UpdateState::UpdateAvailable {
            version,
            release_url,
        } => {
            let version = version.clone();
            let url = release_url.clone();

            egui::Frame::NONE
                .fill(ui.visuals().faint_bg_color)
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(phosphor_rich_text(CONFETTI, 14.0).strong());
                        ui.label(
                            RichText::new(t!("settings.about.update_available").to_string())
                                .strong()
                                .size(14.0),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(format!(
                        "{}: v{} → v{}",
                        t!("settings.about.new_version"),
                        update::current_version(),
                        version
                    ));
                    ui.add_space(8.0);
                    if ui
                        .button(t!("settings.about.view_release").to_string())
                        .clicked()
                    {
                        let _ = open::that(&url);
                    }
                });
            ui.add_space(8.0);
            if ui.small_button(t!("settings.about.check_again")).clicked() {
                panel.update_state = UpdateState::Checking;
                panel.update_check_rx = Some(update::spawn_update_check());
            }
        }
        UpdateState::Error(msg) => {
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(WARNING, 14.0).color(ui.visuals().error_fg_color));
                ui.label(
                    RichText::new(t!("settings.about.check_failed").to_string())
                        .color(ui.visuals().error_fg_color),
                );
            });
            ui.label(RichText::new(msg).small().weak());
            ui.add_space(8.0);
            if ui
                .button(t!("settings.about.try_again").to_string())
                .clicked()
            {
                panel.update_state = UpdateState::Checking;
                panel.update_check_rx = Some(update::spawn_update_check());
            }
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    // Links
    ui.label(RichText::new(t!("settings.about.links")).strong());
    ui.add_space(4.0);

    if ui
        .link(t!("settings.about.all_releases").to_string())
        .clicked()
    {
        let _ = open::that("https://github.com/OlaProeis/Ferrite/releases");
    }
    if ui
        .link(format!("🐛 {}", t!("settings.about.report_issue")))
        .clicked()
    {
        let _ = open::that("https://github.com/OlaProeis/Ferrite/issues");
    }
    if ui
        .link(format!("📖 {}", t!("settings.about.source_code")))
        .clicked()
    {
        let _ = open::that("https://github.com/OlaProeis/Ferrite");
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    // License
    ui.label(RichText::new(t!("settings.about.license")).small().weak());
}
