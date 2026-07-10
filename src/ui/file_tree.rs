//! File tree sidebar panel for workspace mode.
//!
//! This module provides a collapsible left sidebar that displays
//! the workspace file tree with icons, expand/collapse, and click-to-open.
//! Supports Git status badges when in a Git repository.

// Allow dead code - includes panel sizing methods and constants for future
// configurable panel width and drag-to-resize functionality
#![allow(dead_code)]

use crate::theme::accent;
use crate::ui::docked_sidebar::{self, DockedSidebarEdge};
use crate::ui::icons::{phosphor_font, phosphor_rich_text};
use crate::ui::phosphor_icons::{CARET_DOWN, CARET_RIGHT, FOLDER, X};
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

/// Height of the workspace name header row (matches ribbon row height).
const WORKSPACE_HEADER_HEIGHT: f32 = 28.0;

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
    /// * `active_tab_path` - Path of the active document tab (for row emphasis)
    /// * `ui_accent` - User accent color for active-row highlight
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        file_tree: &FileTreeNode,
        workspace_name: &str,
        is_dark: bool,
        git_statuses: Option<&HashMap<PathBuf, GitFileStatus>>,
        active_tab_path: Option<&Path>,
        ui_accent: Color32,
    ) -> FileTreeOutput {
        let mut output = FileTreeOutput::default();
        let aggregated_git_statuses = git_statuses.map(Self::build_git_status_index);

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

        egui::Panel::left("file_tree_panel")
            .resizable(true)
            .default_size(self.width)
            .size_range(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH)
            .frame(docked_sidebar::frame(panel_bg))
            .show_inside(ui, |ui| {
                // Update width from panel
                let panel_width = ui.available_width();
                if (panel_width - self.width).abs() > 1.0 {
                    self.width = panel_width;
                    output.new_width = Some(panel_width);
                }

                // Header with workspace name and close button
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), WORKSPACE_HEADER_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add_space(4.0);

                        ui.label(phosphor_rich_text(FOLDER, 14.0));

                        ui.add(
                            egui::Label::new(RichText::new(workspace_name).size(12.0).strong())
                                .truncate(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            if ui
                                .add(
                                    egui::Button::new(phosphor_rich_text(X, 12.0))
                                        .frame(false)
                                        .min_size(Vec2::new(20.0, 20.0)),
                                )
                                .on_hover_text(t!("workspace.close_folder").to_string())
                                .clicked()
                            {
                                output.close_requested = true;
                            }
                        });
                    },
                );

                ui.separator();

                // Scrollable tree area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        self.render_tree_node(
                            ui,
                            file_tree,
                            0,
                            is_dark,
                            &mut output,
                            aggregated_git_statuses.as_ref(),
                            active_tab_path,
                            ui_accent,
                            panel_bg,
                        );
                        ui.add_space(4.0);
                    });

                docked_sidebar::paint_vertical_divider(ui, border_color, DockedSidebarEdge::Left);
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
        active_tab_path: Option<&Path>,
        ui_accent: Color32,
        panel_bg: Color32,
    ) {
        let indent = depth as f32 * INDENT_PER_LEVEL;

        // Colors
        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(235, 235, 240)
        };

        let active_bg = accent::panel_highlight_fill(
            panel_bg,
            ui_accent,
            is_dark,
            if is_dark { 0.38 } else { 0.31 },
        );

        // Determine if this is a directory (loaded or not)
        let is_dir = matches!(
            node.kind,
            FileTreeNodeKind::Directory { .. } | FileTreeNodeKind::DirectoryNotLoaded
        );

        // Check if this directory needs to be loaded
        let needs_loading = matches!(node.kind, FileTreeNodeKind::DirectoryNotLoaded);

        // Get Git status for this node
        let git_status = git_statuses
            .map(|statuses| Self::status_for_path(&node.path, statuses))
            .unwrap_or(GitFileStatus::Clean);

        // Calculate row height for consistent sizing
        let row_height = 20.0;

        let is_active = is_active_file_row(active_tab_path, &node.path, is_dir);

        // Allocate space for the entire row first to detect hover
        let row_width = ui.available_width();
        let (row_rect, row_response) =
            ui.allocate_exact_size(Vec2::new(row_width, row_height), Sense::click());

        // Paint row background (active takes precedence over hover)
        if is_active {
            ui.painter()
                .rect_filled(row_rect, egui::CornerRadius::same(3), active_bg);
        } else if row_response.hovered() {
            ui.painter()
                .rect_filled(row_rect, egui::CornerRadius::same(3), hover_bg);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        // Now render the row content on top of the background
        let mut content_pos = row_rect.left_top() + Vec2::new(indent + 4.0, 2.0);

        // Expand/collapse arrow for directories
        if is_dir {
            let arrow = if node.is_expanded {
                CARET_DOWN
            } else {
                CARET_RIGHT
            };
            ui.painter().text(
                content_pos + Vec2::new(0.0, 0.0),
                egui::Align2::LEFT_TOP,
                arrow,
                phosphor_font(10.0),
                text_color,
            );
        }
        content_pos.x += 14.0; // Space for arrow

        // Icon — accent tint when active; slightly brighter on hover
        let icon_color = if is_active {
            ui_accent
        } else if row_response.hovered() {
            if is_dark {
                Color32::from_rgb(240, 240, 240)
            } else {
                Color32::from_rgb(20, 20, 20)
            }
        } else {
            text_color
        };
        let icon = node.icon();
        ui.painter().text(
            content_pos,
            egui::Align2::LEFT_TOP,
            icon,
            phosphor_font(14.0),
            icon_color,
        );
        content_pos.x += 18.0; // Space for icon

        // Name - color based on Git status; bold when active tab matches
        let name_color = if is_active {
            if is_dark {
                Color32::WHITE
            } else {
                Color32::from_rgb(30, 30, 30)
            }
        } else {
            Self::get_status_color(git_status, text_color, is_dark)
        };
        let name_font = if is_active {
            egui::FontId::new(12.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(12.0)
        };
        ui.painter().text(
            content_pos,
            egui::Align2::LEFT_TOP,
            &node.name,
            name_font,
            name_color,
        );

        // Calculate name width for badge positioning
        let name_galley = ui.fonts_mut(|f| {
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
                    self.render_tree_node(
                        ui,
                        child,
                        depth + 1,
                        is_dark,
                        output,
                        git_statuses,
                        active_tab_path,
                        ui_accent,
                        panel_bg,
                    );
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

    /// Build a single lookup table for both file and directory git statuses.
    ///
    /// This avoids scanning the full status map for every directory row during
    /// immediate-mode repaint, which makes large workspaces feel laggy.
    fn build_git_status_index(
        statuses: &HashMap<PathBuf, GitFileStatus>,
    ) -> HashMap<PathBuf, GitFileStatus> {
        let mut aggregated = HashMap::with_capacity(statuses.len());

        for (path, status) in statuses {
            aggregated
                .entry(path.clone())
                .and_modify(|current| *current = Self::worse_status(*current, *status))
                .or_insert(*status);

            let mut parent = path.parent();
            while let Some(dir) = parent {
                aggregated
                    .entry(dir.to_path_buf())
                    .and_modify(|current| *current = Self::worse_status(*current, *status))
                    .or_insert(*status);
                parent = dir.parent();
            }
        }

        aggregated
    }

    fn status_for_path(path: &Path, statuses: &HashMap<PathBuf, GitFileStatus>) -> GitFileStatus {
        if let Some(status) = statuses.get(path).copied() {
            return status;
        }

        let mut parent = path.parent();
        while let Some(dir) = parent {
            if dir.as_os_str().is_empty() {
                break;
            }

            if let Some(status @ (GitFileStatus::Untracked | GitFileStatus::Ignored)) =
                statuses.get(dir).copied()
            {
                return status;
            }

            parent = dir.parent();
        }

        GitFileStatus::Clean
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
    ///
    /// Uses simple Unicode symbols (from COMMON_SYMBOLS in fonts.rs) instead of
    /// emoji characters that may not render correctly in Inter/JetBrains fonts.
    /// Emoji like ✏️ contain variation selectors (U+FE0F) that cause doubled or
    /// square glyph rendering when the font doesn't support the emoji variant.
    fn render_context_menu(&self, ui: &mut Ui, node: &FileTreeNode, output: &mut FileTreeOutput) {
        let is_dir = matches!(
            node.kind,
            FileTreeNodeKind::Directory { .. } | FileTreeNodeKind::DirectoryNotLoaded
        );

        if is_dir {
            // + for "new/add" action (universally understood)
            if ui
                .button(format!("+ {}", t!("workspace.new_file")))
                .clicked()
            {
                output.context_action = Some(FileTreeContextAction::NewFile(node.path.clone()));
                ui.close();
            }
            // ▪ (small filled square) for folder icon
            if ui
                .button(format!("▪ {}", t!("workspace.new_folder")))
                .clicked()
            {
                output.context_action = Some(FileTreeContextAction::NewFolder(node.path.clone()));
                ui.close();
            }
            ui.separator();
        }

        // Text-only for rename (no simple icon fits well)
        if ui.button(t!("workspace.rename").to_string()).clicked() {
            output.context_action = Some(FileTreeContextAction::Rename(node.path.clone()));
            ui.close();
        }

        // ✕ (multiplication x) for delete action - in COMMON_SYMBOLS
        if ui
            .button(format!("{} {}", X, t!("workspace.delete")))
            .clicked()
        {
            output.context_action = Some(FileTreeContextAction::Delete(node.path.clone()));
            ui.close();
        }

        ui.separator();

        // → (right arrow) for "reveal/navigate" action - in COMMON_SYMBOLS
        if ui
            .button(format!("→ {}", t!("tab.reveal_in_explorer")))
            .clicked()
        {
            output.context_action =
                Some(FileTreeContextAction::RevealInExplorer(node.path.clone()));
            ui.close();
        }

        ui.separator();

        // ↻ for refresh - simple clockwise arrow (not in COMMON_SYMBOLS but widely supported)
        // Fallback: text-only if this still causes issues
        if ui
            .button(format!("↻ {}", t!("workspace.refresh")))
            .clicked()
        {
            output.context_action = Some(FileTreeContextAction::Refresh);
            ui.close();
        }
    }
}

/// Whether a tree row should show active-tab emphasis (files only).
fn is_active_file_row(active_tab_path: Option<&Path>, node_path: &Path, is_dir: bool) -> bool {
    if is_dir {
        return false;
    }
    active_tab_path.is_some_and(|active| active == node_path)
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

    #[test]
    fn test_is_active_file_row() {
        let file_path = PathBuf::from("/workspace/readme.md");
        let dir_path = PathBuf::from("/workspace/src");

        assert!(is_active_file_row(
            Some(file_path.as_path()),
            &file_path,
            false,
        ));
        assert!(!is_active_file_row(
            Some(file_path.as_path()),
            &dir_path,
            true,
        ));
        assert!(!is_active_file_row(None, &file_path, false));
        assert!(!is_active_file_row(
            Some(Path::new("/other/file.md")),
            &file_path,
            false,
        ));
    }

    #[test]
    fn status_for_path_inherits_untracked_parent_directory() {
        let repo_root = PathBuf::from("repo");
        let nested_dir = repo_root.join("nested");
        let nested_file = nested_dir.join("child.txt");
        let sibling_file = repo_root.join("sibling.txt");

        let mut statuses = HashMap::new();
        statuses.insert(nested_dir, GitFileStatus::Untracked);

        assert_eq!(
            FileTreePanel::status_for_path(&nested_file, &statuses),
            GitFileStatus::Untracked
        );
        assert_eq!(
            FileTreePanel::status_for_path(&sibling_file, &statuses),
            GitFileStatus::Clean
        );
    }

    #[test]
    fn status_for_path_does_not_inherit_modified_parent_aggregation() {
        let repo_root = PathBuf::from("repo");
        let nested_dir = repo_root.join("nested");
        let nested_file = nested_dir.join("child.txt");

        let mut statuses = HashMap::new();
        statuses.insert(nested_dir, GitFileStatus::Modified);

        assert_eq!(
            FileTreePanel::status_for_path(&nested_file, &statuses),
            GitFileStatus::Clean
        );
    }
}
