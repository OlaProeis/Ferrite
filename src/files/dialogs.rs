//! Native file dialog integration using the rfd crate
//!
//! This module provides functions to open native file picker dialogs
//! for opening and saving files, and for opening workspace folders.

use rfd::FileDialog;
use std::path::PathBuf;

/// File extension filters for supported file types.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn"];
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc"];
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];
const TOML_EXTENSIONS: &[&str] = &["toml"];
const TEXT_EXTENSIONS: &[&str] = &["txt", "text"];

/// Opens a native folder picker dialog for selecting a workspace folder.
///
/// Returns `Some(PathBuf)` if a folder was selected, `None` if cancelled.
pub fn open_folder_dialog(initial_dir: Option<&PathBuf>) -> Option<PathBuf> {
    let mut dialog = FileDialog::new().set_title("Open Workspace Folder");

    if let Some(dir) = initial_dir {
        dialog = dialog.set_directory(dir);
    }

    dialog.pick_folder()
}

/// Opens a native file dialog for selecting multiple files.
///
/// Supports Markdown, JSON, YAML, TOML, and plain text files.
/// Returns a vector of selected file paths. Empty if the dialog was cancelled.
pub fn open_multiple_files_dialog(initial_dir: Option<&PathBuf>) -> Vec<PathBuf> {
    let mut dialog = FileDialog::new()
        .set_title("Open Files")
        .add_filter("Markdown Files", MARKDOWN_EXTENSIONS)
        .add_filter("JSON Files", JSON_EXTENSIONS)
        .add_filter("YAML Files", YAML_EXTENSIONS)
        .add_filter("TOML Files", TOML_EXTENSIONS)
        .add_filter("Text Files", TEXT_EXTENSIONS)
        .add_filter("All Files", &["*"]);

    if let Some(dir) = initial_dir {
        dialog = dialog.set_directory(dir);
    }

    dialog.pick_files().unwrap_or_default()
}

/// Opens a native save dialog for saving a file.
///
/// Returns `Some(PathBuf)` if a location was selected, `None` if cancelled.
pub fn save_file_dialog(
    initial_dir: Option<&PathBuf>,
    default_name: Option<&str>,
) -> Option<PathBuf> {
    let mut dialog = FileDialog::new()
        .set_title("Save File")
        .add_filter("Markdown Files", MARKDOWN_EXTENSIONS)
        .add_filter("JSON Files", JSON_EXTENSIONS)
        .add_filter("YAML Files", YAML_EXTENSIONS)
        .add_filter("TOML Files", TOML_EXTENSIONS)
        .add_filter("Text Files", TEXT_EXTENSIONS)
        .add_filter("All Files", &["*"]);

    if let Some(dir) = initial_dir {
        dialog = dialog.set_directory(dir);
    }

    if let Some(name) = default_name {
        dialog = dialog.set_file_name(name);
    }

    dialog.save_file()
}
