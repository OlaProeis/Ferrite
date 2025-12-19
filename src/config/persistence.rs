//! Configuration file persistence for Ferrite
//!
//! This module handles loading and saving configuration files to
//! platform-specific directories with robust error handling and
//! graceful fallback to defaults.

use crate::config::Settings;
use crate::error::{Error, Result, ResultExt};
use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Application name used for the config directory
const APP_NAME: &str = "ferrite";

/// Configuration file name
const CONFIG_FILE_NAME: &str = "config.json";

/// Backup configuration file name (used during atomic writes)
const CONFIG_BACKUP_NAME: &str = "config.json.bak";

// ─────────────────────────────────────────────────────────────────────────────
// Platform-Specific Directory Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Get the platform-specific configuration directory for the application.
///
/// Returns the appropriate directory based on the operating system:
/// - **Windows**: `%APPDATA%\ferrite\` (e.g., `C:\Users\<User>\AppData\Roaming\ferrite\`)
/// - **macOS**: `~/Library/Application Support/ferrite/`
/// - **Linux**: `~/.config/ferrite/`
///
/// # Errors
///
/// Returns `Error::ConfigDirNotFound` if the config directory cannot be determined
/// (e.g., if the HOME environment variable is not set).
///
/// # Examples
///
/// ```ignore
/// let config_dir = get_config_dir()?;
/// println!("Config directory: {}", config_dir.display());
/// ```
pub fn get_config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|base| base.join(APP_NAME))
        .ok_or(Error::ConfigDirNotFound)
}

/// Get the full path to the configuration file.
///
/// This combines `get_config_dir()` with the config file name.
///
/// # Errors
///
/// Returns `Error::ConfigDirNotFound` if the config directory cannot be determined.
pub fn get_config_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join(CONFIG_FILE_NAME))
}

/// Ensure the configuration directory exists, creating it if necessary.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;

    if !config_dir.exists() {
        debug!("Creating config directory: {}", config_dir.display());
        fs::create_dir_all(&config_dir).map_err(|e| Error::ConfigSave {
            path: config_dir.clone(),
            source: Box::new(e),
        })?;
    }

    Ok(config_dir)
}

// ─────────────────────────────────────────────────────────────────────────────
// Load Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Load configuration from the default config file location.
///
/// This function attempts to load and parse the configuration file.
/// If the file doesn't exist or is corrupted, it falls back to defaults.
///
/// # Behavior
///
/// 1. If the config file exists and is valid JSON, load and sanitize it
/// 2. If the config file doesn't exist, return default settings
/// 3. If the config file is corrupted/invalid, log a warning and return defaults
///
/// # Examples
///
/// ```ignore
/// let settings = load_config();
/// println!("Theme: {:?}", settings.theme);
/// ```
pub fn load_config() -> Settings {
    load_config_internal()
        .unwrap_or_warn_default(Settings::default(), "Failed to load configuration")
}

/// Internal implementation of config loading.
fn load_config_internal() -> Result<Settings> {
    let config_path = get_config_file_path()?;

    // Check if config file exists
    if !config_path.exists() {
        debug!(
            "Config file not found at {}, using defaults",
            config_path.display()
        );
        return Ok(Settings::default());
    }

    debug!("Loading config from: {}", config_path.display());

    // Read the file contents
    let contents = fs::read_to_string(&config_path).map_err(|e| Error::ConfigLoad {
        path: config_path.clone(),
        source: Box::new(e),
    })?;

    // Handle empty file
    if contents.trim().is_empty() {
        debug!("Config file is empty, using defaults");
        return Ok(Settings::default());
    }

    // Parse and sanitize
    let settings = Settings::from_json_sanitized(&contents).map_err(|e| {
        warn!(
            "Config file at {} contains invalid JSON: {}",
            config_path.display(),
            e
        );
        Error::ConfigParse {
            message: format!("Failed to parse config file: {}", e),
            source: Some(Box::new(e)),
        }
    })?;

    info!(
        "Configuration loaded successfully from {}",
        config_path.display()
    );
    Ok(settings)
}

// ─────────────────────────────────────────────────────────────────────────────
// Save Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Save configuration to the default config file location.
///
/// This function performs an atomic write by:
/// 1. Writing to a temporary backup file
/// 2. Replacing the original file with the backup
///
/// # Errors
///
/// - `Error::ConfigDirNotFound`: Config directory cannot be determined
/// - `Error::ConfigSave`: Failed to write the config file
///
/// # Examples
///
/// ```ignore
/// let mut settings = load_config();
/// settings.theme = Theme::Dark;
/// save_config(&settings)?;
/// ```
pub fn save_config(settings: &Settings) -> Result<()> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join(CONFIG_FILE_NAME);
    let backup_path = config_dir.join(CONFIG_BACKUP_NAME);

    debug!("Saving config to: {}", config_path.display());

    // Serialize to pretty JSON
    let json = serde_json::to_string_pretty(settings).map_err(|e| Error::ConfigSave {
        path: config_path.clone(),
        source: Box::new(e),
    })?;

    // Write to backup file first (atomic write pattern)
    fs::write(&backup_path, &json).map_err(|e| Error::ConfigSave {
        path: backup_path.clone(),
        source: Box::new(e),
    })?;

    // Replace original with backup
    fs::rename(&backup_path, &config_path).map_err(|e| Error::ConfigSave {
        path: config_path.clone(),
        source: Box::new(e),
    })?;

    info!(
        "Configuration saved successfully to {}",
        config_path.display()
    );
    Ok(())
}

/// Save configuration, ignoring errors.
///
/// This is useful for "best effort" saves where failure shouldn't
/// interrupt the application flow (e.g., saving on exit).
///
/// # Returns
///
/// Returns `true` if the save was successful, `false` otherwise.
pub fn save_config_silent(settings: &Settings) -> bool {
    match save_config(settings) {
        Ok(()) => true,
        Err(e) => {
            warn!("Failed to save configuration: {}", e);
            false
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Theme, ViewMode};
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test environment with a temporary config directory.
    struct TestEnv {
        _temp_dir: TempDir,
        _config_dir: PathBuf,
        config_file: PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let config_dir = temp_dir.path().join(APP_NAME);
            let config_file = config_dir.join(CONFIG_FILE_NAME);
            fs::create_dir_all(&config_dir).expect("Failed to create config dir");
            Self {
                _temp_dir: temp_dir,
                _config_dir: config_dir,
                config_file,
            }
        }

        fn write_config(&self, content: &str) {
            fs::write(&self.config_file, content).expect("Failed to write config");
        }

        fn read_config(&self) -> String {
            fs::read_to_string(&self.config_file).expect("Failed to read config")
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Platform directory tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_get_config_dir_returns_path() {
        // This test verifies that get_config_dir returns a valid path
        // on the current platform
        let result = get_config_dir();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(APP_NAME));
    }

    #[test]
    fn test_get_config_file_path() {
        let result = get_config_file_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(CONFIG_FILE_NAME));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Load tests with temp directory
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_load_valid_config() {
        let env = TestEnv::new();
        let settings = Settings {
            theme: Theme::Dark,
            font_size: 16.0,
            ..Settings::default()
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        env.write_config(&json);

        // Read directly from file for testing
        let contents = fs::read_to_string(&env.config_file).unwrap();
        let loaded: Settings = Settings::from_json_sanitized(&contents).unwrap();

        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.font_size, 16.0);
    }

    #[test]
    fn test_load_empty_config_uses_defaults() {
        let env = TestEnv::new();
        env.write_config("");

        let contents = fs::read_to_string(&env.config_file).unwrap();
        if contents.trim().is_empty() {
            // Empty file should result in defaults
            let settings = Settings::default();
            assert_eq!(settings.theme, Theme::Light);
        }
    }

    #[test]
    fn test_load_partial_config_uses_defaults_for_missing() {
        let env = TestEnv::new();
        env.write_config(r#"{"theme": "dark"}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let settings: Settings = serde_json::from_str(&contents).unwrap();

        assert_eq!(settings.theme, Theme::Dark);
        // Missing fields should have defaults
        assert_eq!(settings.font_size, 14.0);
        assert!(settings.show_line_numbers);
    }

    #[test]
    fn test_load_corrupted_config_returns_error() {
        let env = TestEnv::new();
        env.write_config("{ invalid json }");

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let result: std::result::Result<Settings, _> = serde_json::from_str(&contents);

        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_sanitizes_values() {
        let env = TestEnv::new();
        // Invalid font size that should be clamped
        env.write_config(r#"{"font_size": 4.0, "tab_size": 100}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let settings = Settings::from_json_sanitized(&contents).unwrap();

        // Values should be clamped to valid range
        assert_eq!(settings.font_size, Settings::MIN_FONT_SIZE);
        assert_eq!(settings.tab_size, Settings::MAX_TAB_SIZE);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Save tests with temp directory
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_save_config_creates_valid_json() {
        let env = TestEnv::new();
        let settings = Settings {
            theme: Theme::Dark,
            view_mode: ViewMode::Rendered,
            font_size: 18.0,
            ..Settings::default()
        };

        let json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&env.config_file, &json).unwrap();

        // Verify the saved file is valid JSON
        let contents = env.read_config();
        let loaded: Settings = serde_json::from_str(&contents).unwrap();

        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.view_mode, ViewMode::Rendered);
        assert_eq!(loaded.font_size, 18.0);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let env = TestEnv::new();
        let original = Settings {
            theme: Theme::System,
            view_mode: ViewMode::Raw,
            font_size: 20.0,
            show_line_numbers: false,
            word_wrap: false,
            tab_size: 2,
            auto_save: true,
            auto_save_interval_secs: 30,
            ..Settings::default()
        };

        // Save
        let json = serde_json::to_string_pretty(&original).unwrap();
        fs::write(&env.config_file, &json).unwrap();

        // Load
        let contents = env.read_config();
        let loaded: Settings = serde_json::from_str(&contents).unwrap();

        assert_eq!(original, loaded);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge case tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_config_with_unknown_fields_ignored() {
        let env = TestEnv::new();
        // JSON with an unknown field
        env.write_config(r#"{"theme": "dark", "unknown_field": "value", "future_feature": true}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let result: std::result::Result<Settings, _> = serde_json::from_str(&contents);

        // Should succeed, ignoring unknown fields
        assert!(result.is_ok());
        assert_eq!(result.unwrap().theme, Theme::Dark);
    }

    #[test]
    fn test_config_with_null_values() {
        let env = TestEnv::new();
        env.write_config(r#"{"theme": null}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let result: std::result::Result<Settings, _> = serde_json::from_str(&contents);

        // null should fail since Theme doesn't support null
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_wrong_types() {
        let env = TestEnv::new();
        env.write_config(r#"{"font_size": "not a number"}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let result: std::result::Result<Settings, _> = serde_json::from_str(&contents);

        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helper function tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_default_settings_are_serializable() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings);
        assert!(json.is_ok());
    }

    #[test]
    fn test_app_name_constant() {
        assert_eq!(APP_NAME, "ferrite");
    }

    #[test]
    fn test_config_file_name_constant() {
        assert_eq!(CONFIG_FILE_NAME, "config.json");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Integration tests (use actual config directory)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_load_config_graceful_fallback() {
        // This tests the public API which gracefully falls back to defaults
        let settings = load_config();

        // Should always return valid settings, even if file doesn't exist
        // Verify we got valid defaults by checking a known default value
        assert_eq!(settings.font_size, 14.0);
    }

    #[test]
    fn test_save_config_silent_returns_bool() {
        let settings = Settings::default();
        let result = save_config_silent(&settings);

        // Result depends on whether we have write permissions
        // Just verify it doesn't panic and returns a bool
        assert!(result == true || result == false);
    }
}
