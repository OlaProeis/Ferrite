//! Status bar rendering for the Ferrite application.
//!
//! This module renders the bottom status bar with file path, encoding selector,
//! line/column info, word count, CSV controls, and toast messages.

use super::FerriteApp;
use crate::config::ViewMode;
use crate::markdown::{delimiter_display_name, delimiter_symbol, get_tabular_file_type, DELIMITERS};
use eframe::egui;
use log::{debug, warn};
use rust_i18n::t;

impl FerriteApp {
    /// Render the bottom status bar panel.
    ///
    /// Returns (toggle_rainbow_columns, pending_encoding_change).
    pub(crate) fn render_status_bar(
        &mut self,
        ctx: &egui::Context,
        is_dark: bool,
    ) -> (bool, Option<&'static str>) {
        let mut toggle_rainbow_columns = false;
        let mut pending_encoding_change: Option<&'static str> = None;

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left side: File path (clickable for recent files popup)
                let path_display = if let Some(tab) = self.state.active_tab() {
                    tab.path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| t!("status.untitled").to_string())
                } else {
                    t!("status.no_file").to_string()
                };

                // Make the file path a clickable button that opens the recent items popup
                let has_recent_files = !self.state.settings.recent_files.is_empty();
                let has_recent_folders = !self.state.settings.recent_workspaces.is_empty();
                let has_recent_items = has_recent_files || has_recent_folders;
                let popup_id = ui.make_persistent_id("recent_items_popup");

                let button_response = ui.add(
                    egui::Button::new(&path_display)
                        .frame(false)
                        .sense(if has_recent_items {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        })
                );

                if has_recent_items {
                    button_response.clone().on_hover_text(t!("tooltip.recent_items").to_string());
                }

                // Toggle popup on click
                let just_opened = if button_response.clicked() && has_recent_items {
                    self.state.ui.show_recent_files_popup = !self.state.ui.show_recent_files_popup;
                    self.state.ui.show_recent_files_popup // true if we just opened it
                } else {
                    false
                };

                // Show recent items popup (files and folders)
                if self.state.ui.show_recent_files_popup && has_recent_items {
                    // Collect recent items before creating the popup to avoid borrow issues
                    let recent_files: Vec<_> = self.state.settings.recent_files
                        .iter()
                        .take(10)
                        .cloned()
                        .collect();
                    let recent_folders: Vec<_> = self.state.settings.recent_workspaces
                        .iter()
                        .take(5)
                        .cloned()
                        .collect();

                    // Position popup above the button so it doesn't cover the filename
                    // Calculate position: start from left edge, place above with some margin
                    let popup_pos = egui::pos2(
                        button_response.rect.left(),
                        button_response.rect.top() - 8.0, // Small gap above button
                    );
                    let popup_response = egui::Area::new(popup_id)
                        .order(egui::Order::Foreground)
                        .fixed_pos(popup_pos)
                        .pivot(egui::Align2::LEFT_BOTTOM) // Anchor at bottom-left so it grows upward
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                // Use two-column layout if we have both files and folders
                                let show_both_columns = !recent_files.is_empty() && !recent_folders.is_empty();
                                
                                // Action to perform after popup closes
                                // (PathBuf, is_file, with_focus)
                                let mut action: Option<(std::path::PathBuf, bool, bool)> = None;
                                
                                // Theme-aware colors
                                let name_color = if is_dark {
                                    egui::Color32::from_rgb(220, 220, 220)
                                } else {
                                    egui::Color32::from_rgb(30, 30, 30)
                                };
                                let secondary_color = if is_dark {
                                    egui::Color32::from_rgb(160, 160, 160)
                                } else {
                                    egui::Color32::from_rgb(80, 80, 80)
                                };
                                let folder_icon_color = if is_dark {
                                    egui::Color32::from_rgb(255, 200, 100) // Yellow/gold for folders
                                } else {
                                    egui::Color32::from_rgb(180, 140, 50)
                                };

                                if show_both_columns {
                                    // Stacked vertical layout: Files section, then Folders section
                                    ui.set_min_width(300.0);
                                    
                                    // Recent Files section
                                    ui.label(egui::RichText::new(t!("menu.file.recent").to_string()).strong());
                                    ui.separator();
                                    
                                    for path in &recent_files {
                                        let file_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(file_name).strong().color(name_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(280.0, 0.0))
                                        );
                                        item_response.clone().on_hover_text(t!("status.file_tooltip", path = path.display().to_string()).to_string());

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(2.0);

                                        if item_response.clicked() {
                                            let shift_held = ui.input(|i| i.modifiers.shift);
                                            action = Some((path.clone(), true, !shift_held));
                                        }
                                    }

                                    // Separator between sections
                                    ui.add_space(8.0);
                                    
                                    // Recent Folders section
                                    ui.label(egui::RichText::new(t!("workspace.recent_folders").to_string()).strong());
                                    ui.separator();
                                    
                                    for path in &recent_folders {
                                        let folder_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(format!("📁 {}", folder_name))
                                                    .strong()
                                                    .color(folder_icon_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(280.0, 0.0))
                                        );
                                        item_response.clone().on_hover_text(t!("status.folder_tooltip", path = path.display().to_string()).to_string());

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(2.0);

                                        if item_response.clicked() {
                                            action = Some((path.clone(), false, true));
                                        }
                                    }
                                } else if !recent_files.is_empty() {
                                    // Only files
                                    ui.set_min_width(300.0);
                                    ui.label(egui::RichText::new(t!("status.recent_files_heading").to_string()).strong());
                                    ui.separator();

                                    for path in &recent_files {
                                        let file_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(file_name).strong().color(name_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(ui.available_width(), 0.0))
                                        );
                                        item_response.clone().on_hover_text(t!("status.file_tooltip", path = path.display().to_string()).to_string());

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(4.0);

                                        if item_response.clicked() {
                                            let shift_held = ui.input(|i| i.modifiers.shift);
                                            action = Some((path.clone(), true, !shift_held));
                                        }
                                    }
                                } else {
                                    // Only folders
                                    ui.set_min_width(300.0);
                                    ui.label(egui::RichText::new(t!("status.recent_folders_heading").to_string()).strong());
                                    ui.separator();

                                    for path in &recent_folders {
                                        let folder_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(format!("📁 {}", folder_name))
                                                    .strong()
                                                    .color(folder_icon_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(ui.available_width(), 0.0))
                                        );
                                        item_response.clone().on_hover_text(t!("status.folder_tooltip", path = path.display().to_string()).to_string());

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(4.0);

                                        if item_response.clicked() {
                                            action = Some((path.clone(), false, true));
                                        }
                                    }
                                }

                                action
                            })
                        });

                    // Handle action after UI is done
                    if let Some((path, is_file, focus)) = popup_response.inner.inner {
                        if is_file {
                            // Only close popup on normal click (focus=true)
                            // Keep open on shift+click to allow opening multiple files
                            if focus {
                                self.state.ui.show_recent_files_popup = false;
                            }
                            let time = self.get_app_time();
                            match self.open_file_smart(path.clone(), focus, Some(time)) {
                                Ok(_) => {
                                    self.pending_cjk_check = true;
                                    if focus {
                                        debug!("Opened recent file with focus: {}", path.display());
                                    } else {
                                        let time = self.get_app_time();
                                        self.state.show_toast(
                                            t!("notification.opened_background", name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file")).to_string(),
                                            time,
                                            2.0
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to open recent file: {}", e);
                                    self.state.show_error(t!("error.open_file_failed", error = e.to_string()).to_string());
                                }
                            }
                        } else {
                            // Open folder as workspace
                            self.state.ui.show_recent_files_popup = false;
                            match self.state.open_workspace(path.clone()) {
                                Ok(_) => {
                                    let time = self.get_app_time();
                                    let folder_name = path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("folder");
                                    self.state.show_toast(
                                        t!("notification.opened_workspace", name = folder_name).to_string(),
                                        time,
                                        2.5
                                    );
                                    debug!("Opened recent workspace: {}", path.display());
                                }
                                Err(e) => {
                                    warn!("Failed to open recent workspace: {}", e);
                                    self.state.show_error(t!("error.open_workspace_failed", error = e.to_string()).to_string());
                                }
                            }
                        }
                    }

                    // Close popup when clicking outside (but not on the same frame we opened it)
                    if popup_response.response.clicked_elsewhere() && !just_opened {
                        self.state.ui.show_recent_files_popup = false;
                    }
                }

                // Add separator between left and right sections
                ui.separator();

                // LSP status (compact; hover for per-server detail)
                let (lsp_summary, lsp_detail) = self.lsp_status_bar_text();
                ui.label(
                    egui::RichText::new(lsp_summary)
                        .small()
                        .color(if is_dark {
                            egui::Color32::from_rgb(160, 200, 255)
                        } else {
                            egui::Color32::from_rgb(50, 90, 140)
                        }),
                )
                .on_hover_text(lsp_detail);

                // Diagnostic counts (errors/warnings across all files)
                let (err_count, warn_count) = self.state.diagnostics.counts();
                if err_count > 0 || warn_count > 0 {
                    let err_color = if is_dark {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_rgb(200, 40, 40)
                    };
                    let warn_color = if is_dark {
                        egui::Color32::from_rgb(255, 200, 80)
                    } else {
                        egui::Color32::from_rgb(180, 130, 0)
                    };
                    let mut parts = Vec::new();
                    if err_count > 0 {
                        parts.push(format!("{} err", err_count));
                    }
                    if warn_count > 0 {
                        parts.push(format!("{} warn", warn_count));
                    }
                    let primary_color = if err_count > 0 { err_color } else { warn_color };
                    ui.label(
                        egui::RichText::new(parts.join(" "))
                            .small()
                            .color(primary_color),
                    );
                }

                ui.separator();

                // Center: Toast message (temporary notifications) - shown inline, not expanding
                if let Some(toast) = &self.state.ui.toast_message {
                    ui.label(egui::RichText::new(format!("✔ {}", toast)).italics().color(
                        if is_dark { egui::Color32::from_rgb(120, 200, 120) } 
                        else { egui::Color32::from_rgb(40, 140, 40) }
                    ));
                    ui.separator();
                }

                // Pre-compute cached text stats via &mut borrow before entering immutable tab borrow
                let cached_stats = self.state.active_tab_mut().map(|tab| tab.text_stats());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Help button (rightmost in right-to-left layout)
                    if ui
                        .button("?")
                        .on_hover_text(t!("tooltip.about_help").to_string())
                        .clicked()
                    {
                        self.state.toggle_about();
                    }

                    // Git branch display (if in a git repository)
                    if let Some(branch) = self.state.git_service.current_branch() {
                        ui.separator();
                        
                        // Branch icon and name with theme-appropriate color
                        let branch_color = if is_dark {
                            egui::Color32::from_rgb(130, 180, 240) // Light blue for dark mode
                        } else {
                            egui::Color32::from_rgb(50, 100, 170) // Dark blue for light mode
                        };
                        
                        ui.label(
                            egui::RichText::new(format!("⎇ {}", branch))
                                .color(branch_color)
                                .size(12.0)
                        ).on_hover_text(t!("tooltip.git_branch").to_string());
                    }

                    // Vim mode indicator (shown before tab-specific items)
                    if let Some(vim_label) = self.state.ui.vim_mode_indicator {
                        ui.separator();
                        let vim_color = if is_dark {
                            match vim_label {
                                "INSERT" => egui::Color32::from_rgb(130, 200, 130),
                                "VISUAL" | "V-LINE" => egui::Color32::from_rgb(200, 160, 100),
                                _ => egui::Color32::from_rgb(130, 180, 240),
                            }
                        } else {
                            match vim_label {
                                "INSERT" => egui::Color32::from_rgb(40, 120, 40),
                                "VISUAL" | "V-LINE" => egui::Color32::from_rgb(150, 100, 30),
                                _ => egui::Color32::from_rgb(50, 100, 170),
                            }
                        };
                        ui.label(
                            egui::RichText::new(format!("[{}]", vim_label))
                                .color(vim_color)
                                .strong()
                                .size(12.0)
                        ).on_hover_text(t!("status.vim_mode").to_string());
                    }

                    if let Some(tab) = self.state.active_tab() {
                        ui.separator();

                        // Cursor position
                        let (line, col) = tab.cursor_position;
                        ui.label(format!("Ln {}, Col {}", line + 1, col + 1));

                        // Delimiter picker for CSV/TSV files in rendered or split mode
                        if tab.view_mode == ViewMode::Rendered || tab.view_mode == ViewMode::Split {
                            if let Some(tabular_type) = tab.path.as_ref().and_then(|p| get_tabular_file_type(p)) {
                                ui.separator();
                                
                                let tab_id = tab.id;
                                
                                // Capture all state values upfront to avoid borrow conflicts with popups
                                let (current_delimiter, is_overridden, has_headers, header_overridden) = {
                                    let csv_state = self.csv_viewer_states.entry(tab_id).or_default();
                                    (
                                        csv_state.effective_delimiter().unwrap_or(tabular_type.delimiter()),
                                        csv_state.has_delimiter_override(),
                                        csv_state.has_headers(),
                                        csv_state.has_header_override(),
                                    )
                                };
                                
                                // Delimiter indicator with dropdown
                                let delimiter_label = format!(
                                    "Delim: {}{}",
                                    delimiter_symbol(current_delimiter),
                                    if is_overridden { " ✔" } else { "" }
                                );
                                
                                let popup_id = ui.make_persistent_id("delimiter_picker_popup");
                                let button_response = ui.add(
                                    egui::Button::new(&delimiter_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );
                                
                                button_response.clone().on_hover_text(format!(
                                    "Delimiter: {}\n{}Click to change",
                                    delimiter_display_name(current_delimiter),
                                    if is_overridden { "Manually set. " } else { "Auto-detected. " }
                                ));
                                
                                if button_response.clicked() {
                                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                }
                                
                                // Delimiter picker popup
                                egui::popup_below_widget(ui, popup_id, &button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                    ui.set_min_width(120.0);
                                    ui.label(egui::RichText::new(t!("csv.select_delimiter").to_string()).strong());
                                    ui.separator();
                                    
                                    // Auto-detect option
                                    let auto_selected = !is_overridden;
                                    if ui.selectable_label(auto_selected, t!("csv.delimiter_auto").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.clear_delimiter_override();
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    ui.separator();
                                    
                                    // Manual delimiter options
                                    for &delim in DELIMITERS {
                                        let selected = is_overridden && current_delimiter == delim;
                                        let label = format!("{} {}", delimiter_symbol(delim), delimiter_display_name(delim));
                                        if ui.selectable_label(selected, label).clicked() {
                                            if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                                state.set_delimiter(delim);
                                            }
                                            ui.memory_mut(|mem| mem.close_popup());
                                        }
                                    }
                                });
                                
                                ui.separator();
                                
                                // Header row toggle
                                let header_label = format!(
                                    "Headers: {}{}",
                                    if has_headers { "✔" } else { "✗" },
                                    if header_overridden { " ✔" } else { "" }
                                );
                                
                                let header_popup_id = ui.make_persistent_id("header_picker_popup");
                                let header_button_response = ui.add(
                                    egui::Button::new(&header_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );
                                
                                header_button_response.clone().on_hover_text(format!(
                                    "First row as headers: {}\n{}Click to change",
                                    if has_headers { "Yes" } else { "No" },
                                    if header_overridden { "Manually set. " } else { "Auto-detected. " }
                                ));
                                
                                if header_button_response.clicked() {
                                    ui.memory_mut(|mem| mem.toggle_popup(header_popup_id));
                                }
                                
                                // Header picker popup
                                egui::popup_below_widget(ui, header_popup_id, &header_button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                    ui.set_min_width(120.0);
                                    ui.label(egui::RichText::new(t!("csv.header_row").to_string()).strong());
                                    ui.separator();
                                    
                                    // Auto-detect option
                                    let auto_selected = !header_overridden;
                                    if ui.selectable_label(auto_selected, t!("csv.delimiter_auto").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.clear_header_override();
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    ui.separator();
                                    
                                    // Manual options
                                    if ui.selectable_label(header_overridden && has_headers, t!("csv.has_headers_yes").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.set_header_override(true);
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    if ui.selectable_label(header_overridden && !has_headers, t!("csv.has_headers_no").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.set_header_override(false);
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                });

                                ui.separator();

                                // Rainbow columns toggle
                                // (capture values and defer mutation to avoid borrow conflict)
                                let rainbow_enabled = self.state.settings.csv_rainbow_columns;
                                let rainbow_label = format!(
                                    "Colors: {}",
                                    if rainbow_enabled { "🌈" } else { "○" }
                                );

                                let rainbow_button_response = ui.add(
                                    egui::Button::new(&rainbow_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );

                                rainbow_button_response.clone().on_hover_text(format!(
                                    "Rainbow column coloring: {}\nClick to toggle",
                                    if rainbow_enabled { "On" } else { "Off" }
                                ));

                                if rainbow_button_response.clicked() {
                                    toggle_rainbow_columns = true;
                                }
                            }
                        }

                        ui.separator();

                        // Encoding display and picker
                        let encoding_display = tab.encoding_display_name();
                        let encoding_popup_id = ui.make_persistent_id("encoding_picker_popup");
                        let encoding_button_response = ui.add(
                            egui::Button::new(&encoding_display)
                                .frame(false)
                                .sense(egui::Sense::click())
                        );

                        encoding_button_response.clone().on_hover_text(format!(
                            "File encoding: {}\n{}Click to change",
                            encoding_display,
                            if tab.detected_encoding.is_some() { "Detected. " } else { "Default. " }
                        ));

                        if encoding_button_response.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(encoding_popup_id));
                        }

                        // Encoding picker popup
                        egui::popup_below_widget(ui, encoding_popup_id, &encoding_button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                            ui.set_min_width(150.0);
                            ui.label(egui::RichText::new(t!("status.encoding_heading").to_string()).strong());
                            ui.separator();

                            // Show common encodings
                            for &enc in crate::state::Tab::COMMON_ENCODINGS {
                                let selected = tab.current_encoding.eq_ignore_ascii_case(enc);
                                let label = enc.to_uppercase();
                                if ui.selectable_label(selected, label).clicked() {
                                    pending_encoding_change = Some(enc);
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            }
                        });

                        ui.separator();

                        // Text statistics (cached via content_version)
                        if let Some(stats) = cached_stats {
                            ui.label(stats.format_compact());
                        }
                    }
                });
            });
        });

        // Apply deferred encoding change (outside tab borrow scope)
        if let Some(new_encoding) = pending_encoding_change {
            if let Some(tab) = self.state.active_tab_mut() {
                if let Err(e) = tab.set_encoding(new_encoding) {
                    warn!("Failed to change encoding: {}", e);
                    let time = self.get_app_time();
                    self.state.show_toast(t!("error.encoding_failed", error = e.to_string()).to_string(), time, 3.0);
                } else {
                    let time = self.get_app_time();
                    self.state.show_toast(t!("notification.encoding_changed", encoding = new_encoding.to_uppercase()).to_string(), time, 2.0);
                }
            }
        }

        (toggle_rainbow_columns, pending_encoding_change)
    }
}
