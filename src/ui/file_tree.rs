//! File tree sidebar panel for workspace mode.
//!
//! This module provides a collapsible left sidebar that displays
//! the workspace file tree with icons, expand/collapse, and click-to-open.
//! Supports Git status badges when in a Git repository.

// Allow dead code - includes panel sizing methods and constants for future
// configurable panel width and drag-to-resize functionality
#![allow(dead_code)]

use crate::vcs::GitFileStatus;
use crate::workspaces::{FileTreeNode, FileTreeNodeKind};
use eframe::egui::{self, Color32, RichText, Sense, Ui, Vec2};
use rust_i18n::t;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default width of the file tree panel.
const DEFAULT_PANEL_WIDTH: f32 = 250.0;

/// Minimum width of the file tree panel.
const MIN_PANEL_WIDTH: f32 = 150.0;

/// Maximum width of the file tree panel.
const MAX_PANEL_WIDTH: f32 = 500.0;

/// Indentation per tree level.
const INDENT_PER_LEVEL: f32 = 16.0;

/// Height of each tree item row.
const ROW_HEIGHT: f32 = 22.0;

/// Output from the file tree panel.
#[derive(Debug, Default)]
pub struct FileTreeOutput {
    /// File that was clicked (should be opened in a tab)
    pub file_clicked: Option<PathBuf>,

    /// Path that was toggled (expand/collapse)
    pub path_toggled: Option<PathBuf>,

    /// Directory that needs its children loaded (lazy loading)
    pub needs_loading: Option<PathBuf>,

    /// Whether close button was clicked
    pub close_requested: bool,

    /// New panel width if resized
    pub new_width: Option<f32>,

    /// Context menu action requested
    pub context_action: Option<FileTreeContextAction>,
}

/// Actions from the file tree context menu.
#[derive(Debug, Clone)]
pub enum FileTreeContextAction {
    /// Create a new file in the selected directory
    NewFile(PathBuf),
    /// Create a new folder in the selected directory
    NewFolder(PathBuf),
    /// Rename the selected item
    Rename(PathBuf),
    /// Delete the selected item
    Delete(PathBuf),
    /// Reveal in system file explorer
    RevealInExplorer(PathBuf),
    /// Refresh the file tree
    Refresh,
}

/// File tree sidebar panel.
pub struct FileTreePanel {
    /// Current panel width
    width: f32,
    /// Whether we're currently resizing
    is_resizing: bool,
}

impl Default for FileTreePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTreePanel {
    /// Create a new file tree panel with default width.
    pub fn new() -> Self {
        Self {
            width: DEFAULT_PANEL_WIDTH,
            is_resizing: false,
        }
    }

    /// Create with a specific width.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        self
    }

    /// Set the panel width.
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
    }

    /// Get the current panel width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Render the file tree panel and return any output.
    ///
    /// # Arguments
    /// * `ctx` - The egui context
    /// * `file_tree` - The file tree root node
    /// * `workspace_name` - Name to display in the panel header
    /// * `is_dark` - Whether dark theme is active
    /// * `git_statuses` - Optional map of file paths to Git statuses
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        file_tree: &FileTreeNode,
        workspace_name: &str,
        is_dark: bool,
        git_statuses: Option<&HashMap<PathBuf, GitFileStatus>>,
    ) -> FileTreeOutput {
        let mut output = FileTreeOutput::default();

        // Panel colors
        let panel_bg = if is_dark {
            Color32::from_rgb(30, 30, 30)
        } else {
            Color32::from_rgb(245, 245, 245)
        };

        let border_color = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let _header_bg = if is_dark {
            Color32::from_rgb(40, 40, 40)
        } else {
            Color32::from_rgb(235, 235, 235)
        };

        egui::SidePanel::left("file_tree_panel")
            .resizable(true)
            .default_width(self.width)
            .width_range(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH)
            .frame(
                egui::Frame::none()
                    .fill(panel_bg)
                    .stroke(egui::Stroke::new(1.0, border_color)),
            )
            .show(ctx, |ui| {
                // Update width from panel
                let panel_width = ui.available_width();
                if (panel_width - self.width).abs() > 1.0 {
                    self.width = panel_width;
                    output.new_width = Some(panel_width);
                }

                // Header with workspace name and close button
                ui.horizontal(|ui| {
                    ui.add_space(4.0);

                    // Folder icon
                    ui.label(RichText::new("📁").size(14.0));

                    // Workspace name (truncated if needed)
                    let _name_width = ui.available_width() - 30.0;
                    ui.add(
                        egui::Label::new(RichText::new(workspace_name).size(12.0).strong())
                            .truncate(),
                    );

                    // Close button (right-aligned with padding from resize handle)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Add spacing to move button away from panel edge/resize handle
                        ui.add_space(8.0);
                        if ui
                            .add(egui::Button::new("×").frame(false))
                            .on_hover_text(t!("workspace.close_folder").to_string())
                            .clicked()
                        {
                            output.close_requested = true;
                        }
                    });
                });

                ui.add_space(2.0);
                ui.separator();

                // Scrollable tree area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        self.render_tree_node(ui, file_tree, 0, is_dark, &mut output, git_statuses);
                        ui.add_space(4.0);
                    });
            });

        output
    }

    /// Render a single tree node and its children (if expanded).
    fn render_tree_node(
        &self,
        ui: &mut Ui,
        node: &FileTreeNode,
        depth: usize,
        is_dark: bool,
        output: &mut FileTreeOutput,
        git_statuses: Option<&HashMap<PathBuf, GitFileStatus>>,
    ) {
        let indent = depth as f32 * INDENT_PER_LEVEL;

        // Colors
        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(50, 50, 60)
        } else {
            Color32::from_rgb(220, 225, 235)
        };

        let _selected_bg = if is_dark {
            Color32::from_rgb(45, 55, 75)
        } else {
            Color32::from_rgb(200, 210, 230)
        };

        // Determine if this is a directory (loaded or not)
        let is_dir = matches!(
            node.kind,
            FileTreeNodeKind::Directory { .. } | FileTreeNodeKind::DirectoryNotLoaded
        );

        // Check if this directory needs to be loaded
        let needs_loading = matches!(node.kind, FileTreeNodeKind::DirectoryNotLoaded);

        // Get Git status for this node
        let git_status = git_statuses
            .and_then(|statuses| {
                if is_dir {
                    // For directories, aggregate child statuses
                    Self::get_directory_status(&node.path, statuses)
                } else {
                    statuses.get(&node.path).copied()
                }
            })
            .unwrap_or(GitFileStatus::Clean);

        // Calculate row height for consistent sizing
        let row_height = 20.0;

        // Allocate space for the entire row first to detect hover
        let row_width = ui.available_width();
        let (row_rect, row_response) =
            ui.allocate_exact_size(Vec2::new(row_width, row_height), Sense::click());

        // Paint hover background FIRST (before text)
        if row_response.hovered() {
            ui.painter().rect_filled(row_rect, 2.0, hover_bg);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Now render the row content on top of the background
        let mut content_pos = row_rect.left_top() + Vec2::new(indent + 4.0, 2.0);

        // Expand/collapse arrow for directories
        if is_dir {
            let arrow = if node.is_expanded { "▼" } else { "▶" };
            ui.painter().text(
                content_pos + Vec2::new(0.0, 0.0),
                egui::Align2::LEFT_TOP,
                arrow,
                egui::FontId::proportional(10.0),
                text_color,
            );
        }
        content_pos.x += 14.0; // Space for arrow

        // Icon
        let icon = node.icon();
        ui.painter().text(
            content_pos,
            egui::Align2::LEFT_TOP,
            icon,
            egui::FontId::proportional(14.0),
            text_color,
        );
        content_pos.x += 18.0; // Space for icon

        // Name - color based on Git status
        let name_color = Self::get_status_color(git_status, text_color, is_dark);
        ui.painter().text(
            content_pos,
            egui::Align2::LEFT_TOP,
            &node.name,
            egui::FontId::proportional(12.0),
            name_color,
        );

        // Calculate name width for badge positioning
        let name_galley = ui.fonts(|f| {
            f.layout_no_wrap(
                node.name.clone(),
                egui::FontId::proportional(12.0),
                text_color,
            )
        });
        content_pos.x += name_galley.size().x + 4.0;

        // Git status badge (if not clean)
        if git_status.is_visible() {
            let badge_color = Self::get_badge_color(git_status, is_dark);
            ui.painter().text(
                content_pos,
                egui::Align2::LEFT_TOP,
                git_status.icon(),
                egui::FontId::proportional(10.0),
                badge_color,
            );
        }

        // Handle click
        if row_response.clicked() {
            if is_dir {
                // Toggle expansion for directories
                output.path_toggled = Some(node.path.clone());

                // If expanding and needs loading, signal that
                if !node.is_expanded && needs_loading {
                    output.needs_loading = Some(node.path.clone());
                }
            } else {
                // Open file for files
                output.file_clicked = Some(node.path.clone());
            }
        }

        // Context menu with Git status tooltip
        let tooltip = if git_status.is_visible() {
            format!("{} ({})", node.name, Self::status_description(git_status))
        } else {
            node.name.clone()
        };
        row_response.clone().on_hover_text(&tooltip);

        row_response.context_menu(|ui| {
            self.render_context_menu(ui, node, output);
        });

        // Render children if expanded (only for loaded directories)
        if let FileTreeNodeKind::Directory { children } = &node.kind {
            if node.is_expanded {
                for child in children {
                    self.render_tree_node(ui, child, depth + 1, is_dark, output, git_statuses);
                }
            }
        }
        // Note: DirectoryNotLoaded nodes won't render children until loaded
    }

    /// Get the Git status color for file/folder names.
    fn get_status_color(status: GitFileStatus, default: Color32, is_dark: bool) -> Color32 {
        match status {
            GitFileStatus::Clean => default,
            GitFileStatus::Modified | GitFileStatus::StagedModified => {
                if is_dark {
                    Color32::from_rgb(230, 180, 80) // Yellow/orange for dark
                } else {
                    Color32::from_rgb(180, 120, 0) // Darker orange for light
                }
            }
            GitFileStatus::Staged => {
                if is_dark {
                    Color32::from_rgb(100, 200, 120) // Green for dark
                } else {
                    Color32::from_rgb(40, 140, 60) // Darker green for light
                }
            }
            GitFileStatus::Untracked => {
                if is_dark {
                    Color32::from_rgb(150, 200, 150) // Light green for dark
                } else {
                    Color32::from_rgb(80, 140, 80) // Medium green for light
                }
            }
            GitFileStatus::Ignored => {
                if is_dark {
                    Color32::from_rgb(120, 120, 120) // Gray for dark
                } else {
                    Color32::from_rgb(160, 160, 160) // Gray for light
                }
            }
            GitFileStatus::Deleted => {
                if is_dark {
                    Color32::from_rgb(220, 100, 100) // Red for dark
                } else {
                    Color32::from_rgb(180, 60, 60) // Darker red for light
                }
            }
            GitFileStatus::Renamed => {
                if is_dark {
                    Color32::from_rgb(130, 180, 240) // Blue for dark
                } else {
                    Color32::from_rgb(50, 100, 170) // Darker blue for light
                }
            }
            GitFileStatus::Conflict => {
                if is_dark {
                    Color32::from_rgb(240, 80, 80) // Bright red for dark
                } else {
                    Color32::from_rgb(200, 40, 40) // Red for light
                }
            }
        }
    }

    /// Get the badge color for Git status icons.
    fn get_badge_color(status: GitFileStatus, is_dark: bool) -> Color32 {
        match status {
            GitFileStatus::Clean => Color32::TRANSPARENT,
            GitFileStatus::Modified => {
                if is_dark {
                    Color32::from_rgb(255, 200, 100) // Yellow
                } else {
                    Color32::from_rgb(200, 140, 0)
                }
            }
            GitFileStatus::Staged => {
                if is_dark {
                    Color32::from_rgb(100, 220, 100) // Green
                } else {
                    Color32::from_rgb(40, 160, 40)
                }
            }
            GitFileStatus::StagedModified => {
                if is_dark {
                    Color32::from_rgb(200, 180, 100) // Yellow-green
                } else {
                    Color32::from_rgb(160, 140, 40)
                }
            }
            GitFileStatus::Untracked => {
                if is_dark {
                    Color32::from_rgb(160, 220, 160) // Light green
                } else {
                    Color32::from_rgb(100, 160, 100)
                }
            }
            GitFileStatus::Ignored => {
                if is_dark {
                    Color32::from_rgb(140, 140, 140) // Gray
                } else {
                    Color32::from_rgb(120, 120, 120)
                }
            }
            GitFileStatus::Deleted => {
                if is_dark {
                    Color32::from_rgb(255, 120, 120) // Red
                } else {
                    Color32::from_rgb(200, 60, 60)
                }
            }
            GitFileStatus::Renamed => {
                if is_dark {
                    Color32::from_rgb(140, 190, 255) // Blue
                } else {
                    Color32::from_rgb(60, 120, 200)
                }
            }
            GitFileStatus::Conflict => {
                if is_dark {
                    Color32::from_rgb(255, 80, 80) // Bright red
                } else {
                    Color32::from_rgb(220, 40, 40)
                }
            }
        }
    }

    /// Get a human-readable description for a Git status.
    fn status_description(status: GitFileStatus) -> String {
        match status {
            GitFileStatus::Clean => t!("git.tracked").to_string(),
            GitFileStatus::Modified => t!("git.modified").to_string(),
            GitFileStatus::Staged => t!("git.staged").to_string(),
            GitFileStatus::StagedModified => t!("git.staged_modified").to_string(),
            GitFileStatus::Untracked => t!("git.untracked").to_string(),
            GitFileStatus::Ignored => t!("git.ignored").to_string(),
            GitFileStatus::Deleted => t!("git.deleted").to_string(),
            GitFileStatus::Renamed => t!("git.renamed").to_string(),
            GitFileStatus::Conflict => t!("git.conflict").to_string(),
        }
    }

    /// Get the aggregated Git status for a directory.
    ///
    /// Returns the "worst" status among all files in the directory.
    fn get_directory_status(
        dir_path: &Path,
        statuses: &HashMap<PathBuf, GitFileStatus>,
    ) -> Option<GitFileStatus> {
        let mut worst = GitFileStatus::Clean;
        let mut found_any = false;

        for (path, status) in statuses {
            if path.starts_with(dir_path) {
                found_any = true;
                worst = Self::worse_status(worst, *status);
                // Conflict is worst, can stop early
                if matches!(worst, GitFileStatus::Conflict) {
                    break;
                }
            }
        }

        if found_any && worst.is_visible() {
            Some(worst)
        } else {
            None
        }
    }

    /// Compare two statuses and return the "worse" one for aggregation.
    fn worse_status(a: GitFileStatus, b: GitFileStatus) -> GitFileStatus {
        use GitFileStatus::*;

        let priority = |s: GitFileStatus| -> u8 {
            match s {
                Clean => 0,
                Ignored => 1,
                Untracked => 2,
                Deleted => 3,
                Renamed => 4,
                Modified => 5,
                Staged => 6,
                StagedModified => 7,
                Conflict => 8,
            }
        };

        if priority(a) >= priority(b) {
            a
        } else {
            b
        }
    }

    /// Render the context menu for a tree node.
    fn render_context_menu(&self, ui: &mut Ui, node: &FileTreeNode, output: &mut FileTreeOutput) {
        let is_dir = matches!(
            node.kind,
            FileTreeNodeKind::Directory { .. } | FileTreeNodeKind::DirectoryNotLoaded
        );

        if is_dir {
            if ui.button(format!("📄 {}", t!("workspace.new_file"))).clicked() {
                output.context_action = Some(FileTreeContextAction::NewFile(node.path.clone()));
                ui.close_menu();
            }
            if ui.button(format!("📁 {}", t!("workspace.new_folder"))).clicked() {
                output.context_action = Some(FileTreeContextAction::NewFolder(node.path.clone()));
                ui.close_menu();
            }
            ui.separator();
        }

        if ui.button(format!("✏️ {}", t!("workspace.rename"))).clicked() {
            output.context_action = Some(FileTreeContextAction::Rename(node.path.clone()));
            ui.close_menu();
        }

        if ui.button(format!("🗑️ {}", t!("workspace.delete"))).clicked() {
            output.context_action = Some(FileTreeContextAction::Delete(node.path.clone()));
            ui.close_menu();
        }

        ui.separator();

        if ui.button(format!("📂 {}", t!("tab.reveal_in_explorer"))).clicked() {
            output.context_action =
                Some(FileTreeContextAction::RevealInExplorer(node.path.clone()));
            ui.close_menu();
        }

        ui.separator();

        if ui.button(format!("🔄 {}", t!("workspace.refresh"))).clicked() {
            output.context_action = Some(FileTreeContextAction::Refresh);
            ui.close_menu();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_panel_new() {
        let panel = FileTreePanel::new();
        assert_eq!(panel.width(), DEFAULT_PANEL_WIDTH);
    }

    #[test]
    fn test_file_tree_panel_with_width() {
        let panel = FileTreePanel::new().with_width(300.0);
        assert_eq!(panel.width(), 300.0);
    }

    #[test]
    fn test_file_tree_panel_width_clamping() {
        let panel = FileTreePanel::new().with_width(50.0); // Below min
        assert_eq!(panel.width(), MIN_PANEL_WIDTH);

        let panel = FileTreePanel::new().with_width(1000.0); // Above max
        assert_eq!(panel.width(), MAX_PANEL_WIDTH);
    }

    #[test]
    fn test_file_tree_output_default() {
        let output = FileTreeOutput::default();
        assert!(output.file_clicked.is_none());
        assert!(output.path_toggled.is_none());
        assert!(!output.close_requested);
        assert!(output.new_width.is_none());
        assert!(output.context_action.is_none());
    }
}
