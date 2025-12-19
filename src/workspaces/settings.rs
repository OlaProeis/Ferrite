//! Workspace settings management.

// Allow dead code - includes settings methods and save functionality for future
// workspace configuration features
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Workspace Settings
// ─────────────────────────────────────────────────────────────────────────────

/// Settings specific to a workspace.
///
/// Stored in `{workspace_root}/.ferrite/settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceSettings {
    /// Additional folders/patterns to hide in the file tree
    pub hidden_folders: Vec<String>,

    /// Theme override for this workspace (None = use global setting)
    pub theme_override: Option<String>,

    /// Font size override for this workspace
    pub font_size_override: Option<f32>,

    /// Whether to show line numbers (workspace override)
    pub show_line_numbers: Option<bool>,

    /// Default view mode for new files in this workspace
    pub default_view_mode: Option<String>,

    /// File extensions to treat as markdown
    pub markdown_extensions: Vec<String>,

    /// Custom file associations (extension -> language)
    #[serde(default)]
    pub file_associations: std::collections::HashMap<String, String>,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            hidden_folders: Vec::new(),
            theme_override: None,
            font_size_override: None,
            show_line_numbers: None,
            default_view_mode: None,
            markdown_extensions: vec![
                "md".to_string(),
                "markdown".to_string(),
                "mdown".to_string(),
                "mkd".to_string(),
            ],
            file_associations: std::collections::HashMap::new(),
        }
    }
}

impl WorkspaceSettings {
    /// Check if a file extension should be treated as markdown.
    pub fn is_markdown_extension(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.markdown_extensions
            .iter()
            .any(|e| e.to_lowercase() == ext_lower)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence
// ─────────────────────────────────────────────────────────────────────────────

/// The subdirectory name for workspace configuration.
const WORKSPACE_CONFIG_DIR: &str = ".ferrite";

/// The settings file name.
const SETTINGS_FILE: &str = "settings.json";

/// Load workspace settings from disk.
///
/// Returns `None` if the settings file doesn't exist or is invalid.
pub fn load_workspace_settings(workspace_root: &Path) -> Option<WorkspaceSettings> {
    let settings_path = workspace_root
        .join(WORKSPACE_CONFIG_DIR)
        .join(SETTINGS_FILE);

    if !settings_path.exists() {
        log::debug!("No workspace settings file at {:?}", settings_path);
        return None;
    }

    match std::fs::read_to_string(&settings_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(settings) => {
                log::debug!("Loaded workspace settings from {:?}", settings_path);
                Some(settings)
            }
            Err(e) => {
                log::warn!("Failed to parse workspace settings: {}", e);
                None
            }
        },
        Err(e) => {
            log::warn!("Failed to read workspace settings: {}", e);
            None
        }
    }
}

/// Save workspace settings to disk.
///
/// Creates the `.ferrite` directory if it doesn't exist.
pub fn save_workspace_settings(
    workspace_root: &Path,
    settings: &WorkspaceSettings,
) -> Result<(), std::io::Error> {
    let config_dir = workspace_root.join(WORKSPACE_CONFIG_DIR);

    // Create directory if needed
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    let settings_path = config_dir.join(SETTINGS_FILE);
    let content = serde_json::to_string_pretty(settings)?;

    std::fs::write(&settings_path, content)?;
    log::info!("Saved workspace settings to {:?}", settings_path);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_settings_default() {
        let settings = WorkspaceSettings::default();
        assert!(settings.hidden_folders.is_empty());
        assert!(settings.theme_override.is_none());
        assert!(settings.is_markdown_extension("md"));
        assert!(settings.is_markdown_extension("MD"));
        assert!(settings.is_markdown_extension("markdown"));
        assert!(!settings.is_markdown_extension("txt"));
    }

    #[test]
    fn test_workspace_settings_serialization() {
        let settings = WorkspaceSettings {
            hidden_folders: vec!["build".to_string(), "dist".to_string()],
            theme_override: Some("dark".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: WorkspaceSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.hidden_folders, settings.hidden_folders);
        assert_eq!(parsed.theme_override, settings.theme_override);
    }

    #[test]
    fn test_load_save_workspace_settings() {
        let temp_dir = std::env::temp_dir().join("ferrite_test_workspace_settings");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let settings = WorkspaceSettings {
            hidden_folders: vec!["test_hidden".to_string()],
            ..Default::default()
        };

        // Save
        save_workspace_settings(&temp_dir, &settings).unwrap();

        // Load
        let loaded = load_workspace_settings(&temp_dir).unwrap();
        assert_eq!(loaded.hidden_folders, settings.hidden_folders);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
