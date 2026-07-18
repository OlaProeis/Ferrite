//! Dialog rendering for the Ferrite application.
//!
//! This module contains the rendering of modal dialogs: go-to-line,
//! close confirmation, file operation dialogs, and find/replace panel.
//!
//! Note: Settings and About/Help panels are now rendered as special tabs
//! in the central panel (see central_panel.rs).

#[allow(unused_imports)]
use super::helpers::modifier_symbol;
use super::FerriteApp;
use crate::markdown::{spawn_run, take_pending_code_execution_consent};
use crate::state::PendingAction;
use crate::ui::phosphor_icons::{phosphor_rich_text, PACKAGE, WARNING};
use eframe::egui;
use log::debug;
use rust_i18n::t;
use std::time::Duration;

impl FerriteApp {
    /// Render dialog windows.
    pub(crate) fn render_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(pending) = take_pending_code_execution_consent(ctx) {
            self.state.ui.pending_code_run = Some(pending);
            self.state.ui.show_code_execution_consent_dialog = true;
            self.state.ui.code_execution_consent_focus_cancel = true;
        }

        // Confirmation dialog for unsaved changes
        if self.state.ui.show_confirm_dialog {
            egui::Window::new(t!("dialog.unsaved_changes.title").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.label(&self.state.ui.confirm_dialog_message);
                    ui.separator();
                    ui.horizontal(|ui| {
                        // Check if this is a tab close action (vs exit)
                        let is_tab_close = matches!(
                            self.state.ui.pending_action,
                            Some(PendingAction::CloseTab(_))
                        );
                        let is_exit = self.state.ui.pending_action == Some(PendingAction::Exit);

                        // Collect tab IDs for cleanup before the action mutates state
                        let tab_ids_to_cleanup: Vec<usize> = match self.state.ui.pending_action {
                            Some(PendingAction::CloseTab(index)) => self
                                .state
                                .tabs()
                                .get(index)
                                .map(|t| vec![t.id])
                                .unwrap_or_default(),
                            Some(PendingAction::CloseAllTabs) => {
                                self.state.tabs().iter().map(|t| t.id).collect()
                            }
                            Some(PendingAction::CloseOtherTabs(keep_tab_id)) => self
                                .state
                                .tabs()
                                .iter()
                                .filter(|tab| tab.id != keep_tab_id)
                                .map(|tab| tab.id)
                                .collect(),
                            _ => Vec::new(),
                        };

                        // "Save" button - save then proceed with action
                        if ui
                            .button(t!("dialog.unsaved_changes.save").to_string())
                            .clicked()
                        {
                            if is_tab_close {
                                // Save the tab first
                                if let Some(PendingAction::CloseTab(index)) =
                                    self.state.ui.pending_action
                                {
                                    // Switch to that tab to save it
                                    self.state.set_active_tab(index);
                                }
                                self.handle_save_file();
                                // If save succeeded (tab is no longer modified), close it
                                if let Some(PendingAction::CloseTab(index)) =
                                    self.state.ui.pending_action
                                {
                                    if !self
                                        .state
                                        .tab(index)
                                        .map(|t| t.is_modified())
                                        .unwrap_or(true)
                                    {
                                        self.state.handle_confirmed_action();
                                        for id in &tab_ids_to_cleanup {
                                            self.cleanup_tab_state(*id, Some(ui.ctx()));
                                        }
                                    } else {
                                        // Save was cancelled or failed, cancel the close
                                        self.state.cancel_pending_action();
                                    }
                                }
                            } else if is_exit {
                                // Save all modified tabs before exit
                                self.handle_save_file();
                                if !self.state.has_unsaved_changes() {
                                    self.state.handle_confirmed_action();
                                    self.should_exit = true;
                                }
                            }
                        }

                        // "Discard" button - proceed without saving
                        if ui
                            .button(t!("dialog.unsaved_changes.dont_save").to_string())
                            .clicked()
                        {
                            self.state.handle_confirmed_action();
                            for id in &tab_ids_to_cleanup {
                                self.cleanup_tab_state(*id, Some(ui.ctx()));
                            }
                            if is_exit {
                                // Clear recovery data since user explicitly chose not to save
                                crate::config::clear_all_recovery_data();
                                self.should_exit = true;
                            }
                        }

                        // "Cancel" button - abort the action
                        if ui.button(t!("dialog.confirm.cancel").to_string()).clicked() {
                            self.state.cancel_pending_action();
                        }
                    });
                });
        }

        // Error modal
        if self.state.ui.show_error_modal {
            egui::Window::new(t!("common.error").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.label(phosphor_rich_text(WARNING, 24.0));
                    ui.label(&self.state.ui.error_message);
                    ui.separator();
                    if ui.button(t!("common.ok").to_string()).clicked() {
                        self.state.dismiss_error();
                    }
                });
        }

        // Portal error dialog (Linux xdg-desktop-portal missing)
        if self.state.ui.show_portal_error_dialog {
            egui::Window::new("File Dialog Failed")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .min_width(450.0)
                .show(ctx, |ui| {
                    ui.label(phosphor_rich_text(PACKAGE, 24.0));
                    ui.add_space(8.0);

                    // Show the main error message
                    for line in self.state.ui.portal_error_message.lines() {
                        ui.label(line);
                    }

                    ui.separator();

                    // Copy command button
                    ui.horizontal(|ui| {
                        if ui.button("Copy Install Command").clicked() {
                            let cmd = self.state.ui.portal_error_command.clone();
                            // Set clipboard via egui's output
                            ui.copy_text(cmd.clone());
                            log::info!("Copied portal install command to clipboard: {}", cmd);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t!("common.ok").to_string()).clicked() {
                                self.state.dismiss_portal_error();
                            }
                        });
                    });
                });
        }

        // Code execution consent (first Run from preview, or rare mismatch).
        if self.state.ui.show_code_execution_consent_dialog {
            egui::Window::new(t!("dialog.code_execution_consent.title").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.label(t!("dialog.code_execution_consent.body_intro"));
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(t!("dialog.code_execution_consent.body_settings_echo"))
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    );

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.state.ui.show_code_execution_consent_dialog = false;
                        self.state.ui.pending_code_run = None;
                        self.state.ui.code_execution_consent_focus_cancel = false;
                        return;
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        let enable_run = ui
                            .button(t!("dialog.code_execution_consent.enable_and_run").to_string())
                            .on_hover_text(
                                t!("dialog.code_execution_consent.enable_and_run_tooltip")
                                    .to_string(),
                            );
                        let just_enable = ui
                            .button(t!("dialog.code_execution_consent.just_enable").to_string())
                            .on_hover_text(
                                t!("dialog.code_execution_consent.just_enable_tooltip").to_string(),
                            );
                        let cancel = ui
                            .button(t!("dialog.confirm.cancel").to_string())
                            .on_hover_text(
                                t!("dialog.code_execution_consent.cancel_tooltip").to_string(),
                            );

                        if self.state.ui.code_execution_consent_focus_cancel {
                            cancel.request_focus();
                            self.state.ui.code_execution_consent_focus_cancel = false;
                        }

                        if enable_run.clicked() {
                            self.state.settings.enable_code_execution = true;
                            self.state.settings.code_execution_consent_acknowledged = true;
                            self.state.mark_settings_dirty();
                            let _ = self.state.save_settings_if_dirty();

                            if let Some(pending) = self.state.ui.pending_code_run.take() {
                                let timeout = Duration::from_secs(pending.timeout_secs as u64);
                                let handle = spawn_run(
                                    pending.code,
                                    pending.language,
                                    pending.cwd,
                                    timeout,
                                    ctx.clone(),
                                );
                                let run_key = pending.block_id.with("run_handle");
                                let toast_emitted_key = pending.block_id.with("run_toast_emitted");
                                ctx.memory_mut(|mem| {
                                    mem.data.insert_temp(run_key, handle);
                                    mem.data.remove::<bool>(toast_emitted_key);
                                });
                                ctx.request_repaint();
                            }

                            self.state.ui.show_code_execution_consent_dialog = false;
                        } else if just_enable.clicked() {
                            self.state.settings.enable_code_execution = true;
                            self.state.settings.code_execution_consent_acknowledged = true;
                            self.state.mark_settings_dirty();
                            let _ = self.state.save_settings_if_dirty();
                            self.state.ui.pending_code_run = None;
                            self.state.ui.show_code_execution_consent_dialog = false;
                        } else if cancel.clicked() {
                            self.state.ui.pending_code_run = None;
                            self.state.ui.show_code_execution_consent_dialog = false;
                        }
                    });
                });
        }

        // Note: About/Help and Settings panels are now rendered as special tabs
        // in the central panel (see central_panel.rs render_special_tab_content).

        // Find/Replace panel
        if self.state.ui.show_find_replace {
            let is_dark = ctx.global_style().visuals.dark_mode;
            let output = self
                .find_replace_panel
                .show(ctx, &mut self.state.ui.find_state, is_dark);

            // Handle search changes with debouncing for large files
            // This prevents running expensive searches on every keystroke
            if output.search_changed {
                // Mark search as pending and record when it was requested
                self.state.ui.find_search_pending = true;
                self.state.ui.find_search_requested_at = Some(std::time::Instant::now());
                // Request repaint after debounce delay
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }

            // Execute pending search after debounce delay (150ms)
            if self.state.ui.find_search_pending {
                let should_search = self
                    .state
                    .ui
                    .find_search_requested_at
                    .map(|t| t.elapsed() >= std::time::Duration::from_millis(150))
                    .unwrap_or(false);

                if should_search {
                    self.state.ui.find_search_pending = false;
                    self.state.ui.find_search_requested_at = None;

                    // Clone content to avoid borrow conflict with find_state
                    // This only happens after debounce delay, not on every keystroke
                    let content = self.state.active_tab().map(|t| t.content.clone());
                    if let Some(content) = content {
                        let match_count = self.state.ui.find_state.find_matches(&content);
                        if match_count > 0 {
                            self.state.ui.scroll_to_match = true;
                        }
                        debug!("Search executed (debounced), found {} matches", match_count);
                    }
                }
            }

            // Handle navigation
            if output.next_requested {
                self.handle_find_next();
            }

            if output.prev_requested {
                self.handle_find_prev();
            }

            // Handle replace actions
            if output.replace_requested {
                self.handle_replace_current(ctx);
            }

            if output.replace_all_requested {
                self.handle_replace_all(ctx);
            }

            // Handle close
            if output.close_requested {
                self.state.ui.show_find_replace = false;
            }
        }

        // PDF / HTML export options dialogs
        self.render_pdf_export_dialog(ctx);
        self.render_html_export_dialog(ctx);
    }

    /// Modal dialog for HTML export options.
    pub(crate) fn render_html_export_dialog(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_html_export_dialog {
            return;
        }

        use crate::export::options::ImageHandling;
        use crate::export::HtmlExportThemeChoice;

        let mut keep_open = true;
        let mut do_export = false;
        let mut opts = self.state.settings.html_export_options.clone();

        egui::Window::new(t!("dialog.html_export.title").to_string())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.label(t!("dialog.html_export.description").to_string());
                ui.add_space(6.0);

                ui.checkbox(
                    &mut opts.self_contained,
                    t!("dialog.html_export.self_contained").to_string(),
                );
                if !opts.self_contained {
                    ui.label(t!("dialog.html_export.image_paths").to_string());
                    egui::ComboBox::from_id_salt("html_img_handling")
                        .selected_text(opts.image_handling.label())
                        .show_ui(ui, |ui| {
                            for h in ImageHandling::all() {
                                ui.selectable_value(&mut opts.image_handling, *h, h.label());
                            }
                        });
                }

                ui.checkbox(
                    &mut opts.include_outline,
                    t!("dialog.html_export.include_outline").to_string(),
                );
                ui.checkbox(
                    &mut opts.include_html_comments,
                    t!("dialog.html_export.include_comments").to_string(),
                );

                ui.horizontal(|ui| {
                    ui.label(t!("dialog.html_export.link_base").to_string());
                    ui.text_edit_singleline(&mut opts.link_base_path);
                });

                ui.horizontal(|ui| {
                    ui.label(t!("dialog.html_export.theme").to_string());
                    egui::ComboBox::from_id_salt("html_export_theme")
                        .selected_text(opts.theme.label())
                        .show_ui(ui, |ui| {
                            for c in HtmlExportThemeChoice::all() {
                                ui.selectable_value(&mut opts.theme, *c, c.label());
                            }
                        });
                });

                ui.checkbox(
                    &mut opts.include_title,
                    t!("dialog.html_export.include_title").to_string(),
                );
                ui.checkbox(
                    &mut opts.include_syntax_highlighting,
                    t!("dialog.html_export.syntax_highlighting").to_string(),
                );
                ui.checkbox(
                    &mut opts.use_theme_colors,
                    t!("dialog.pdf_export.use_theme_colors").to_string(),
                );
                ui.checkbox(
                    &mut opts.open_after_export,
                    t!("dialog.html_export.open_after_export").to_string(),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(t!("dialog.html_export.export").to_string())
                        .clicked()
                    {
                        do_export = true;
                        keep_open = false;
                    }
                    if ui.button(t!("dialog.confirm.cancel").to_string()).clicked() {
                        keep_open = false;
                    }
                });
            });

        if opts.self_contained {
            self.state.settings.export_embed_images = true;
        }
        if opts != self.state.settings.html_export_options {
            self.state.settings.html_export_options = opts;
            self.state.mark_settings_dirty();
        }

        if !keep_open {
            self.state.ui.show_html_export_dialog = false;
        }
        if do_export {
            self.handle_perform_html_export(ctx);
        }
    }

    /// Modal dialog that lets the user pick page size, margins, and a few
    /// rendering toggles before kicking off a PDF export.
    pub(crate) fn render_pdf_export_dialog(&mut self, ctx: &egui::Context) {
        if !self.state.ui.show_pdf_export_dialog {
            return;
        }

        use crate::export::{PdfMarginPreset, PdfPageSize};

        let mut keep_open = true;
        let mut do_export = false;
        let mut opts = self.state.settings.pdf_export_options.clone();

        egui::Window::new(t!("dialog.pdf_export.title").to_string())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .min_width(360.0)
            .show(ctx, |ui| {
                ui.label(t!("dialog.pdf_export.description").to_string());
                ui.add_space(6.0);

                // Page size
                egui::Grid::new("pdf_export_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(t!("dialog.pdf_export.page_size").to_string());
                        egui::ComboBox::from_id_salt("pdf_page_size")
                            .selected_text(opts.page_size.label())
                            .show_ui(ui, |ui| {
                                for size in PdfPageSize::all() {
                                    ui.selectable_value(&mut opts.page_size, *size, size.label());
                                }
                            });
                        ui.end_row();

                        ui.label(t!("dialog.pdf_export.margins").to_string());
                        egui::ComboBox::from_id_salt("pdf_margins")
                            .selected_text(opts.margin_preset.label())
                            .show_ui(ui, |ui| {
                                for preset in PdfMarginPreset::all() {
                                    ui.selectable_value(
                                        &mut opts.margin_preset,
                                        *preset,
                                        preset.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });

                // Custom margin sliders, only when the preset is Custom.
                if opts.margin_preset == PdfMarginPreset::Custom {
                    ui.add_space(4.0);
                    ui.label(t!("dialog.pdf_export.custom_margins_hint").to_string());
                    ui.horizontal(|ui| {
                        ui.label("T");
                        ui.add(
                            egui::DragValue::new(&mut opts.custom_margins.top)
                                .range(0.0..=144.0)
                                .speed(1.0)
                                .suffix(" pt"),
                        );
                        ui.label("R");
                        ui.add(
                            egui::DragValue::new(&mut opts.custom_margins.right)
                                .range(0.0..=144.0)
                                .speed(1.0)
                                .suffix(" pt"),
                        );
                        ui.label("B");
                        ui.add(
                            egui::DragValue::new(&mut opts.custom_margins.bottom)
                                .range(0.0..=144.0)
                                .speed(1.0)
                                .suffix(" pt"),
                        );
                        ui.label("L");
                        ui.add(
                            egui::DragValue::new(&mut opts.custom_margins.left)
                                .range(0.0..=144.0)
                                .speed(1.0)
                                .suffix(" pt"),
                        );
                    });
                }

                ui.add_space(6.0);
                ui.checkbox(
                    &mut opts.page_break_before_h1,
                    t!("dialog.pdf_export.page_break_h1").to_string(),
                );
                ui.checkbox(
                    &mut opts.include_page_numbers,
                    t!("dialog.pdf_export.page_numbers").to_string(),
                );
                ui.checkbox(
                    &mut opts.use_theme_colors,
                    t!("dialog.pdf_export.use_theme_colors").to_string(),
                );
                ui.checkbox(
                    &mut opts.open_after_export,
                    t!("dialog.pdf_export.open_after_export").to_string(),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(t!("dialog.pdf_export.export").to_string())
                        .clicked()
                    {
                        do_export = true;
                        keep_open = false;
                    }
                    if ui.button(t!("dialog.confirm.cancel").to_string()).clicked() {
                        keep_open = false;
                    }
                });
            });

        // Persist option changes whether or not the user clicks Export, so a
        // user that just wanted to fiddle the defaults sees them survive.
        if opts != self.state.settings.pdf_export_options {
            self.state.settings.pdf_export_options = opts;
            self.state.mark_settings_dirty();
        }

        if !keep_open {
            self.state.ui.show_pdf_export_dialog = false;
        }
        if do_export {
            self.handle_export_pdf(ctx);
        }
    }
}
