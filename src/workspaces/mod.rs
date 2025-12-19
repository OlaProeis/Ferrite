//! Workspace management for Ferrite
//!
//! This module provides folder/workspace support including:
//! - File tree data structures and scanning
//! - Workspace settings and state persistence
//! - File watching for external changes

// Allow dead code - workspace module contains complete API for settings
// persistence and tree operations that may not all be used yet
// - only_used_in_recursion: Recursive file collection uses self for future state
#![allow(dead_code)]
#![allow(clippy::only_used_in_recursion)]

mod file_tree;
mod persistence;
mod settings;
mod watcher;

pub use file_tree::{FileTreeNode, FileTreeNodeKind};
pub use persistence::{load_workspace_state, save_workspace_state, WorkspaceState};
pub use settings::{load_workspace_settings, save_workspace_settings, WorkspaceSettings};
pub use watcher::{filter_events, WorkspaceEvent, WorkspaceWatcher};

use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// App Mode
// ─────────────────────────────────────────────────────────────────────────────

/// The application's current operating mode.
///
/// Determines whether the app is in single-file editing mode or
/// workspace/folder mode with full project management features.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Single file mode - traditional editor behavior
    ///
    /// The app operates on individual files without workspace features.
    /// This is the default mode when opening files directly.
    SingleFile,

    /// Workspace mode - folder-based project management
    ///
    /// The app has an open workspace/folder with file tree, settings,
    /// and project-level features enabled.
    Workspace {
        /// Root path of the workspace folder
        root: PathBuf,
        /// Path to workspace settings file (.ferrite/settings.json)
        settings_path: PathBuf,
    },
}

impl Default for AppMode {
    fn default() -> Self {
        Self::SingleFile
    }
}

impl AppMode {
    /// Check if currently in workspace mode.
    pub fn is_workspace(&self) -> bool {
        matches!(self, Self::Workspace { .. })
    }

    /// Get the workspace root path if in workspace mode.
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        match self {
            Self::Workspace { root, .. } => Some(root),
            Self::SingleFile => None,
        }
    }

    /// Create a new workspace mode from a folder path.
    pub fn from_folder(root: PathBuf) -> Self {
        let settings_path = root.join(".ferrite").join("settings.json");
        Self::Workspace {
            root,
            settings_path,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Workspace
// ─────────────────────────────────────────────────────────────────────────────

/// A workspace representing an open folder/project.
///
/// Contains the file tree, settings, and state for workspace-mode features.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Root path of the workspace
    pub root_path: PathBuf,

    /// The file tree structure
    pub file_tree: FileTreeNode,

    /// Patterns for hidden/ignored files and folders
    pub hidden_patterns: Vec<String>,

    /// Recently opened files within this workspace
    pub recent_files: Vec<PathBuf>,

    /// Workspace-specific settings
    pub settings: WorkspaceSettings,

    /// Whether the file tree panel is visible
    pub show_file_tree: bool,

    /// Width of the file tree panel in pixels
    pub file_tree_width: f32,
}

impl Workspace {
    /// Default patterns that are always hidden unless explicitly shown.
    pub const DEFAULT_HIDDEN_PATTERNS: &'static [&'static str] = &[
        ".git",
        ".svn",
        ".hg",
        "node_modules",
        "target",
        ".idea",
        ".vscode",
        "__pycache__",
        ".DS_Store",
        "Thumbs.db",
    ];

    /// Create a new workspace from a root folder path.
    ///
    /// Loads settings from disk if available, otherwise uses defaults.
    /// Scans the directory to build the initial file tree.
    pub fn new(root_path: PathBuf) -> Self {
        // Load settings if they exist
        let settings = load_workspace_settings(&root_path).unwrap_or_default();

        // Build hidden patterns from defaults and settings
        let mut hidden_patterns: Vec<String> = Self::DEFAULT_HIDDEN_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect();
        hidden_patterns.extend(settings.hidden_folders.clone());

        // Scan the file tree
        let file_tree = file_tree::scan_directory(&root_path, &hidden_patterns);

        // Load workspace state (recent files, expanded nodes, etc.)
        let state = load_workspace_state(&root_path).unwrap_or_default();

        Self {
            root_path,
            file_tree,
            hidden_patterns,
            recent_files: state.recent_files,
            settings,
            show_file_tree: true,
            file_tree_width: 250.0,
        }
    }

    /// Refresh the file tree from disk.
    pub fn refresh_file_tree(&mut self) {
        self.file_tree = file_tree::scan_directory(&self.root_path, &self.hidden_patterns);
    }

    /// Add a file to the recent files list.
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists (to move to front)
        self.recent_files.retain(|p| p != &path);
        // Add to front
        self.recent_files.insert(0, path);
        // Cap at 20 entries
        self.recent_files.truncate(20);
    }

    /// Get a flat list of all files in the workspace (for quick switcher).
    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_files(&self.file_tree, &mut files);
        files
    }

    fn collect_files(&self, node: &FileTreeNode, files: &mut Vec<PathBuf>) {
        match &node.kind {
            FileTreeNodeKind::File => {
                files.push(node.path.clone());
            }
            FileTreeNodeKind::Directory { children } => {
                for child in children {
                    self.collect_files(child, files);
                }
            }
        }
    }

    /// Get the workspace state for persistence.
    pub fn get_state(&self) -> WorkspaceState {
        WorkspaceState {
            recent_files: self.recent_files.clone(),
            expanded_paths: self.file_tree.get_expanded_paths(),
            file_tree_width: self.file_tree_width,
            show_file_tree: self.show_file_tree,
        }
    }

    /// Save the workspace state to disk.
    pub fn save_state(&self) -> Result<(), std::io::Error> {
        save_workspace_state(&self.root_path, &self.get_state())
    }

    /// Save the workspace settings to disk.
    pub fn save_settings(&self) -> Result<(), std::io::Error> {
        save_workspace_settings(&self.root_path, &self.settings)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_default() {
        let mode = AppMode::default();
        assert_eq!(mode, AppMode::SingleFile);
        assert!(!mode.is_workspace());
        assert!(mode.workspace_root().is_none());
    }

    #[test]
    fn test_app_mode_workspace() {
        let root = PathBuf::from("/test/project");
        let mode = AppMode::from_folder(root.clone());

        assert!(mode.is_workspace());
        assert_eq!(mode.workspace_root(), Some(&root));
    }

    #[test]
    fn test_app_mode_settings_path() {
        let root = PathBuf::from("/test/project");
        let mode = AppMode::from_folder(root);

        if let AppMode::Workspace { settings_path, .. } = mode {
            assert!(settings_path.ends_with(".ferrite/settings.json"));
        } else {
            panic!("Expected Workspace mode");
        }
    }
}
