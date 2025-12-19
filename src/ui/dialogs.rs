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
                    "New File",
                    "📄",
                    "Enter file name:",
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
                    "New Folder",
                    "📁",
                    "Enter folder name:",
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
                        "filename.md"
                    } else {
                        "folder-name"
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
                RichText::new(format!("Location: {}", parent_dir.display()))
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
                        .add_enabled(create_enabled, egui::Button::new("Create"))
                        .clicked()
                        || (response.lost_focus()
                            && ctx.input(|i| i.key_pressed(Key::Enter))
                            && create_enabled)
                    {
                        let new_path = parent_dir.join(name_input.trim());
                        if new_path.exists() {
                            *error_message =
                                Some("A file or folder with this name already exists".to_string());
                        } else if is_file {
                            result = FileOperationResult::CreateFile(new_path);
                        } else {
                            result = FileOperationResult::CreateFolder(new_path);
                        }
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button("Cancel").clicked() {
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

    egui::Window::new(format!("{} Rename", icon))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(8.0),
        )
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            ui.add_space(8.0);
            ui.label("Enter new name:");
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
                        .add_enabled(rename_enabled, egui::Button::new("Rename"))
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
                            *error_message =
                                Some("A file or folder with this name already exists".to_string());
                        } else {
                            result = FileOperationResult::Rename {
                                old: target_path.clone(),
                                new: new_path,
                            };
                        }
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button("Cancel").clicked() {
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
    let item_type = if is_dir { "folder" } else { "file" };
    let name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    egui::Window::new("🗑️ Confirm Delete")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(bg_color)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(8.0),
        )
        .show(ctx, |ui| {
            ui.set_min_width(350.0);

            ui.add_space(8.0);

            ui.label(format!(
                "Are you sure you want to delete this {}?",
                item_type
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
                    "⚠ This will delete the folder and all its contents!",
                );
                ui.add_space(8.0);
            }

            // Buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Delete button (red)
                    let delete_button =
                        egui::Button::new(RichText::new("Delete").color(Color32::WHITE))
                            .fill(Color32::from_rgb(200, 60, 60));

                    if ui.add(delete_button).clicked() {
                        result = FileOperationResult::Delete(target_path.clone());
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui.button("Cancel").clicked() {
                        result = FileOperationResult::Cancelled;
                    }
                });
            });

            ui.add_space(4.0);
        });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
