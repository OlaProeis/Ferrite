//! Workspace state persistence (runtime state, not settings).

// Allow dead code - includes constructor and state fields for future workspace
// state restoration features
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Workspace State
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime state for a workspace that should be persisted.
///
/// This includes transient state like expanded tree nodes, recent files,
/// and panel sizes. Stored in `{workspace_root}/.ferrite/state.json`.
///
/// Different from `WorkspaceSettings` which contains user configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkspaceState {
    /// Recently opened files within this workspace
    pub recent_files: Vec<PathBuf>,

    /// Paths of expanded directories in the file tree
    pub expanded_paths: Vec<PathBuf>,

    /// Width of the file tree panel
    pub file_tree_width: f32,

    /// Whether the file tree panel is visible
    pub show_file_tree: bool,
}

impl WorkspaceState {
    /// Create a new default workspace state.
    pub fn new() -> Self {
        Self {
            recent_files: Vec::new(),
            expanded_paths: Vec::new(),
            file_tree_width: 250.0,
            show_file_tree: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence
// ─────────────────────────────────────────────────────────────────────────────

/// The subdirectory name for workspace configuration.
const WORKSPACE_CONFIG_DIR: &str = ".ferrite";

/// The state file name.
const STATE_FILE: &str = "state.json";

/// Load workspace state from disk.
///
/// Returns `None` if the state file doesn't exist or is invalid.
pub fn load_workspace_state(workspace_root: &Path) -> Option<WorkspaceState> {
    let state_path = workspace_root.join(WORKSPACE_CONFIG_DIR).join(STATE_FILE);

    if !state_path.exists() {
        log::debug!("No workspace state file at {:?}", state_path);
        return None;
    }

    match std::fs::read_to_string(&state_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => {
                log::debug!("Loaded workspace state from {:?}", state_path);
                Some(state)
            }
            Err(e) => {
                log::warn!("Failed to parse workspace state: {}", e);
                None
            }
        },
        Err(e) => {
            log::warn!("Failed to read workspace state: {}", e);
            None
        }
    }
}

/// Save workspace state to disk.
///
/// Creates the `.ferrite` directory if it doesn't exist.
pub fn save_workspace_state(
    workspace_root: &Path,
    state: &WorkspaceState,
) -> Result<(), std::io::Error> {
    let config_dir = workspace_root.join(WORKSPACE_CONFIG_DIR);

    // Create directory if needed
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    let state_path = config_dir.join(STATE_FILE);
    let content = serde_json::to_string_pretty(state)?;

    std::fs::write(&state_path, content)?;
    log::debug!("Saved workspace state to {:?}", state_path);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_state_default() {
        let state = WorkspaceState::default();
        assert!(state.recent_files.is_empty());
        assert!(state.expanded_paths.is_empty());
        assert_eq!(state.file_tree_width, 0.0); // Default::default() gives 0.0
        assert!(!state.show_file_tree); // Default::default() gives false
    }

    #[test]
    fn test_workspace_state_new() {
        let state = WorkspaceState::new();
        assert!(state.recent_files.is_empty());
        assert!(state.expanded_paths.is_empty());
        assert_eq!(state.file_tree_width, 250.0);
        assert!(state.show_file_tree);
    }

    #[test]
    fn test_workspace_state_serialization() {
        let state = WorkspaceState {
            recent_files: vec![PathBuf::from("/test/file.md")],
            expanded_paths: vec![PathBuf::from("/test/src")],
            file_tree_width: 300.0,
            show_file_tree: true,
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: WorkspaceState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.recent_files, state.recent_files);
        assert_eq!(parsed.expanded_paths, state.expanded_paths);
        assert_eq!(parsed.file_tree_width, state.file_tree_width);
        assert_eq!(parsed.show_file_tree, state.show_file_tree);
    }

    #[test]
    fn test_load_save_workspace_state() {
        let temp_dir = std::env::temp_dir().join("ferrite_test_workspace_state");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let state = WorkspaceState {
            recent_files: vec![PathBuf::from("/test/file.md")],
            expanded_paths: vec![PathBuf::from("/test/src")],
            file_tree_width: 350.0,
            show_file_tree: true,
        };

        // Save
        save_workspace_state(&temp_dir, &state).unwrap();

        // Load
        let loaded = load_workspace_state(&temp_dir).unwrap();
        assert_eq!(loaded.recent_files, state.recent_files);
        assert_eq!(loaded.expanded_paths, state.expanded_paths);
        assert_eq!(loaded.file_tree_width, state.file_tree_width);
        assert_eq!(loaded.show_file_tree, state.show_file_tree);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
