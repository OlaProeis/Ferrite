//! Modal dialogs for file operations.
//!
//! This module provides dialogs for creating, renaming, and deleting files/folders
//! in workspace mode.

// Allow clippy lints for dialog functions:
// - too_many_arguments: Dialog functions have many UI configuration parameters
// - ptr_arg: Using &PathBuf for consistency with PathBuf storage in dialog state
// - needless_late_init: Result declaration pattern is intentional for match clarity
// - collapsible_else_if: Nested if/else is clearer for file/folder logic
#![allow(clippy::too_many_arguments)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::collapsible_else_if)]

use eframe::egui::{self, Color32, Key, RichText};
use rust_i18n::t;
use std::path::PathBuf;

/// State for an active file operation dialog.
#[derive(Debug, Clone)]
pub enum FileOperationDialog {
    /// Create a new file in the specified directory
    NewFile {
        parent_dir: PathBuf,
        name_input: String,
        error_message: Option<String>,
    },
    /// Create a new folder in the specified directory
    NewFolder {
        parent_dir: PathBuf,
        name_input: String,
        error_message: Option<String>,
    },
    /// Rename a file or folder
    Rename {
        target_path: PathBuf,
        new_name_input: String,
        error_message: Option<String>,
    },
    /// Confirm deletion of a file or folder
    Delete { target_path: PathBuf },
}

/// Result from showing a file operation dialog.
#[derive(Debug)]
pub enum FileOperationResult {
    /// No action taken (dialog still open)
    None,
    /// Dialog was cancelled
    Cancelled,
    /// Create a new file with the given path
    CreateFile(PathBuf),
    /// Create a new folder with the given path
    CreateFolder(PathBuf),
    /// Rename from old path to new path
    Rename { old: PathBuf, new: PathBuf },
    /// Delete the given path
    Delete(PathBuf),
}

impl FileOperationDialog {
    /// Create a new "New File" dialog.
    pub fn new_file(parent_dir: PathBuf) -> Self {
        Self::NewFile {
            parent_dir,
            name_input: String::new(),
            error_message: None,
        }
    }

    /// Create a new "New Folder" dialog.
    pub fn new_folder(parent_dir: PathBuf) -> Self {
        Self::NewFolder {
            parent_dir,
            name_input: String::new(),
            error_message: None,
        }
    }

    /// Create a new "Rename" dialog.
    pub fn rename(target_path: PathBuf) -> Self {
        let current_name = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        Self::Rename {
            target_path,
            new_name_input: current_name,
            error_message: None,
        }
    }

    /// Create a new "Delete" confirmation dialog.
    pub fn delete(target_path: PathBuf) -> Self {
        Self::Delete { target_path }
    }

    /// Show the dialog and return the result.
    pub fn show(&mut self, ctx: &egui::Context, is_dark: bool) -> FileOperationResult {
        let result;

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(40, 40, 45)
        } else {
            Color32::from_rgb(250, 250, 250)
        };

        let border_color = if is_dark {
            Color32::from_rgb(70, 70, 80)
        } else {
            Color32::from_rgb(180, 180, 190)
        };

        match self {
            FileOperationDialog::NewFile {
                parent_dir,
                name_input,
                error_message,
            } => {
                result = show_create_dialog(
                    ctx,
                    &t!("dialog.file.new_file"),
                    "📄",
                    &t!("dialog.file.enter_file_name"),
                    parent_dir,
                    name_input,
                    error_message,
                    is_dark,
                    bg_color,
                    border_color,
                    true, // is_file
                );
            }
            FileOperationDialog::NewFolder {
                parent_dir,
                name_input,
                error_message,
            } => {
                result = show_create_dialog(
                    ctx,
                    &t!("dialog.file.new_folder"),
                    "📁",
                    &t!("dialog.file.enter_folder_name"),
                    parent_dir,
                    name_input,
                    error_message,
                    is_dark,
                    bg_color,
                    border_color,
                    false, // is_file
                );
            }
            FileOperationDialog::Rename {
                target_path,
                new_name_input,
                error_message,
            } => {
                result = show_rename_dialog(
                    ctx,
                    target_path,
                    new_name_input,
                    error_message,
                    is_dark,
                    bg_color,
                    border_color,
                );
            }
            FileOperationDialog::Delete { target_path } => {
                result = show_delete_dialog(ctx, target_path, is_dark, bg_color, border_color);
            }
        }

        result
    }
}

fn show_create_dialog(
    ctx: &egui::Context,
    title: &str,
    icon: &str,
    label: &str,
    parent_dir: &PathBuf,
    name_input: &mut String,
    error_message: &mut Option<String>,
    is_dark: bool,
    bg_color: Color32,
    border_color: Color32,
    is_file: bool,
) -> FileOperationResult {
    let mut result = FileOperationResult::None;

    // Handle escape key
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return FileOperationResult::Cancelled;
    }

    egui::Window::new(format!("{} {}", icon, title))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(8.0),
        )
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            ui.add_space(8.0);
            ui.label(label);
            ui.add_space(4.0);

            // Text input
            let response = ui.add(
                egui::TextEdit::singleline(name_input)
                    .hint_text(if is_file {
                        t!("dialog.file.hint_file")
                    } else {
                        t!("dialog.file.hint_folder")
                    })
                    .desired_width(330.0),
            );

            // Auto-focus
            if response.gained_focus() || name_input.is_empty() {
                response.request_focus();
            }

            // Show error message if any
            if let Some(error) = error_message {
                ui.add_space(4.0);
                ui.colored_label(Color32::from_rgb(220, 80, 80), error.as_str());
            }

            ui.add_space(12.0);

            // Show parent directory
            ui.label(
                RichText::new(t!(
                    "dialog.file.location",
                    path = parent_dir.display().to_string()
                ))
                .small()
                .color(if is_dark {
                    Color32::from_rgb(150, 150, 160)
                } else {
                    Color32::from_rgb(100, 100, 110)
                }),
            );

            ui.add_space(12.0);

            // Buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Create button
                    let create_enabled = !name_input.trim().is_empty()
                        && !name_input.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']);
                    if ui
                        .add_enabled(create_enabled, egui::Button::new(t!("dialog.file.create")))
                        .clicked()
                        || (response.lost_focus()
                            && ctx.input(|i| i.key_pressed(Key::Enter))
                            && create_enabled)
                    {
                        let new_path = parent_dir.join(name_input.trim());
                        if new_path.exists() {
                            *error_message = Some(t!("dialog.file.exists_error").to_string());
                        } else if is_file {
                            result = FileOperationResult::CreateFile(new_path);
                        } else {
                            result = FileOperationResult::CreateFolder(new_path);
                        }
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button(t!("dialog.confirm.cancel")).clicked() {
                        result = FileOperationResult::Cancelled;
                    }
                });
            });

            ui.add_space(4.0);
        });

    result
}

fn show_rename_dialog(
    ctx: &egui::Context,
    target_path: &PathBuf,
    new_name_input: &mut String,
    error_message: &mut Option<String>,
    _is_dark: bool,
    bg_color: Color32,
    border_color: Color32,
) -> FileOperationResult {
    let mut result = FileOperationResult::None;

    // Handle escape key
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return FileOperationResult::Cancelled;
    }

    let is_dir = target_path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };

    egui::Window::new(format!("{} {}", icon, t!("dialog.file.rename")))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(8.0),
        )
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            ui.add_space(8.0);
            ui.label(t!("dialog.file.enter_new_name"));
            ui.add_space(4.0);

            // Text input
            let response = ui.add(egui::TextEdit::singleline(new_name_input).desired_width(330.0));

            // Auto-focus and select all
            response.request_focus();

            // Show error message if any
            if let Some(error) = error_message {
                ui.add_space(4.0);
                ui.colored_label(Color32::from_rgb(220, 80, 80), error.as_str());
            }

            ui.add_space(12.0);

            // Buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let current_name = target_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let rename_enabled = !new_name_input.trim().is_empty()
                        && !new_name_input.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
                        && new_name_input.trim() != current_name;

                    // Rename button
                    if ui
                        .add_enabled(rename_enabled, egui::Button::new(t!("dialog.file.rename")))
                        .clicked()
                        || (response.lost_focus()
                            && ctx.input(|i| i.key_pressed(Key::Enter))
                            && rename_enabled)
                    {
                        let new_path = target_path
                            .parent()
                            .map(|p| p.join(new_name_input.trim()))
                            .unwrap_or_else(|| PathBuf::from(new_name_input.trim()));

                        if new_path.exists() {
                            *error_message = Some(t!("dialog.file.exists_error").to_string());
                        } else {
                            result = FileOperationResult::Rename {
                                old: target_path.clone(),
                                new: new_path,
                            };
                        }
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button(t!("dialog.confirm.cancel")).clicked() {
                        result = FileOperationResult::Cancelled;
                    }
                });
            });

            ui.add_space(4.0);
        });

    result
}

fn show_delete_dialog(
    ctx: &egui::Context,
    target_path: &PathBuf,
    _is_dark: bool,
    bg_color: Color32,
    border_color: Color32,
) -> FileOperationResult {
    let mut result = FileOperationResult::None;

    // Handle escape key
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return FileOperationResult::Cancelled;
    }

    let is_dir = target_path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };
    let item_type = if is_dir {
        t!("dialog.file.folder")
    } else {
        t!("dialog.file.file")
    };
    let name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    egui::Window::new(format!("🗑️ {}", t!("dialog.file.confirm_delete")))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(8.0),
        )
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            ui.add_space(8.0);

            ui.label(t!(
                "dialog.file.delete_confirm",
                item_type = item_type.to_string()
            ));

            ui.add_space(8.0);

            // Show file/folder name
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(16.0));
                ui.label(RichText::new(name).strong());
            });

            ui.add_space(8.0);

            if is_dir {
                ui.colored_label(
                    Color32::from_rgb(220, 160, 80),
                    t!("dialog.file.delete_folder_warning"),
                );
                ui.add_space(8.0);
            }

            // Buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Delete button (red)
                    let delete_button = egui::Button::new(
                        RichText::new(t!("dialog.file.delete")).color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(200, 60, 60));

                    if ui.add(delete_button).clicked() {
                        result = FileOperationResult::Delete(target_path.clone());
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button(t!("dialog.confirm.cancel")).clicked() {
                        result = FileOperationResult::Cancelled;
                    }
                });
            });

            ui.add_space(4.0);
        });

    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Go to Line Dialog
// ─────────────────────────────────────────────────────────────────────────────

/// State for the Go to Line dialog.
#[derive(Debug, Clone)]
pub struct GoToLineDialog {
    /// Current input value (as string for text editing)
    pub line_input: String,
    /// Current line number (1-indexed) shown as placeholder
    pub current_line: usize,
    /// Maximum line number in the document
    pub max_line: usize,
    /// Error message if input is invalid
    pub error_message: Option<String>,
}

/// Result from showing the Go to Line dialog.
#[derive(Debug, Clone)]
pub enum GoToLineResult {
    /// No action taken (dialog still open)
    None,
    /// Dialog was cancelled
    Cancelled,
    /// Navigate to the specified line (1-indexed)
    GoToLine(usize),
}

impl GoToLineDialog {
    /// Create a new Go to Line dialog.
    pub fn new(current_line: usize, max_line: usize) -> Self {
        Self {
            line_input: String::new(),
            current_line,
            max_line,
            error_message: None,
        }
    }

    /// Show the dialog and return the result.
    pub fn show(&mut self, ctx: &egui::Context, is_dark: bool) -> GoToLineResult {
        let mut result = GoToLineResult::None;

        // Handle escape key globally
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            return GoToLineResult::Cancelled;
        }

        // Check for Enter key press globally (before rendering)
        let enter_pressed = ctx.input(|i| i.key_pressed(Key::Enter));

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(40, 40, 45)
        } else {
            Color32::from_rgb(250, 250, 250)
        };

        let border_color = if is_dark {
            Color32::from_rgb(70, 70, 80)
        } else {
            Color32::from_rgb(180, 180, 190)
        };

        let muted_color = if is_dark {
            Color32::from_rgb(150, 150, 160)
        } else {
            Color32::from_rgb(100, 100, 110)
        };

        egui::Window::new(format!("📍 {}", t!("dialog.go_to_line.title")))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .rounding(8.0),
            )
            .show(ctx, |ui| {
                ui.set_min_width(280.0);

                ui.add_space(8.0);
                ui.label(t!("dialog.go_to_line.enter_line"));
                ui.add_space(4.0);

                // Text input
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.line_input)
                        .hint_text(format!("{}", self.current_line))
                        .desired_width(260.0),
                );

                // Auto-focus on first frame
                response.request_focus();

                // Show error message if any
                if let Some(error) = &self.error_message {
                    ui.add_space(4.0);
                    ui.colored_label(Color32::from_rgb(220, 80, 80), error.as_str());
                }

                ui.add_space(8.0);

                // Show line range hint
                ui.label(
                    RichText::new(t!("dialog.go_to_line.range", max = self.max_line))
                        .small()
                        .color(muted_color),
                );

                ui.add_space(12.0);

                // Buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Parse and validate input
                        let input_text = if self.line_input.trim().is_empty() {
                            // Use current line as default if input is empty
                            self.current_line.to_string()
                        } else {
                            self.line_input.trim().to_string()
                        };

                        let parsed_line = input_text.parse::<usize>();
                        let is_valid = parsed_line.is_ok();

                        // Go button - also triggers on Enter key when input is valid
                        let go_clicked = ui
                            .add_enabled(is_valid, egui::Button::new(t!("dialog.go_to_line.go")))
                            .clicked();

                        if go_clicked || (enter_pressed && is_valid) {
                            if let Ok(line) = parsed_line {
                                // Clamp to valid range
                                let target_line = line.clamp(1, self.max_line);
                                result = GoToLineResult::GoToLine(target_line);
                            }
                        }

                        ui.add_space(8.0);

                        // Cancel button
                        if ui.button(t!("dialog.confirm.cancel")).clicked() {
                            result = GoToLineResult::Cancelled;
                        }
                    });
                });

                ui.add_space(4.0);
            });

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_to_line_dialog() {
        let dialog = GoToLineDialog::new(10, 100);
        assert_eq!(dialog.current_line, 10);
        assert_eq!(dialog.max_line, 100);
        assert!(dialog.line_input.is_empty());
        assert!(dialog.error_message.is_none());
    }

    #[test]
    fn test_new_file_dialog() {
        let dialog = FileOperationDialog::new_file(PathBuf::from("/test"));
        match dialog {
            FileOperationDialog::NewFile {
                parent_dir,
                name_input,
                error_message,
            } => {
                assert_eq!(parent_dir, PathBuf::from("/test"));
                assert!(name_input.is_empty());
                assert!(error_message.is_none());
            }
            _ => panic!("Expected NewFile dialog"),
        }
    }

    #[test]
    fn test_rename_dialog() {
        let dialog = FileOperationDialog::rename(PathBuf::from("/test/file.md"));
        match dialog {
            FileOperationDialog::Rename {
                target_path,
                new_name_input,
                ..
            } => {
                assert_eq!(target_path, PathBuf::from("/test/file.md"));
                assert_eq!(new_name_input, "file.md");
            }
            _ => panic!("Expected Rename dialog"),
        }
    }

    #[test]
    fn test_delete_dialog() {
        let dialog = FileOperationDialog::delete(PathBuf::from("/test/file.md"));
        match dialog {
            FileOperationDialog::Delete { target_path } => {
                assert_eq!(target_path, PathBuf::from("/test/file.md"));
            }
            _ => panic!("Expected Delete dialog"),
        }
    }
}
