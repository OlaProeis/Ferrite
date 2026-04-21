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
use crate::state::PendingAction;
use eframe::egui;
use log::debug;
use rust_i18n::t;

impl FerriteApp {
    /// Render dialog windows.
    pub(crate) fn render_dialogs(&mut self, ctx: &egui::Context) {
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
                    ui.label(egui::RichText::new("⚠").size(24.0));
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
                    ui.label(egui::RichText::new("📦").size(24.0));
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
                            ui.output_mut(|o| o.copied_text = cmd.clone());
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

        // Note: About/Help and Settings panels are now rendered as special tabs
        // in the central panel (see central_panel.rs render_special_tab_content).

        // Find/Replace panel
        if self.state.ui.show_find_replace {
            let is_dark = ctx.style().visuals.dark_mode;
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
    }
}
