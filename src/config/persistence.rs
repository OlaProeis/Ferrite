//! Configuration file persistence for Ferrite
//!
//! This module handles loading and saving configuration files to
//! platform-specific directories with robust error handling and
//! graceful fallback to defaults.

use crate::config::Settings;
use crate::error::{Error, Result};
use chrono::Local;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Application name used for the config directory
const APP_NAME: &str = "ferrite";

/// Configuration file name
const CONFIG_FILE_NAME: &str = "config.json";

/// Prefix for a syntax-invalid config preserved beside the live file.
const CORRUPT_CONFIG_PREFIX: &str = "config.json.corrupt-";

/// How many `config.json.corrupt-*` copies to keep.
const MAX_CORRUPT_CONFIG_COPIES: usize = 3;

/// Path of a corrupt config renamed during this process, until [`take_pending_corrupt_config_rename`].
///
/// `main` loads settings before `AppState::new` loads them again. The first load
/// renames the file, so the second load would otherwise miss the toast path.
static PENDING_CORRUPT_CONFIG_RENAME: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Last-good backup of `config.json` (previous file contents, not the new write).
const CONFIG_BACKUP_NAME: &str = "config.json.bak";

/// Temporary file for the atomic config write (`config.json.tmp` → `config.json`).
const CONFIG_TEMP_NAME: &str = "config.json.tmp";

/// Whether this process started from a failed full config load, and whether
/// the user has edited settings since then.
///
/// A failed full load is syntax-invalid JSON (renamed aside) or any load error
/// that falls back to all-default settings. Partial field recovery is not a
/// failed full load. Until the user edits settings, saves must not copy those
/// defaults over `config.json.bak`.
struct ConfigSaveSession {
    failed_full_load: bool,
    user_edited_settings: bool,
}

static CONFIG_SAVE_SESSION: Mutex<ConfigSaveSession> = Mutex::new(ConfigSaveSession {
    failed_full_load: false,
    user_edited_settings: false,
});

/// Serializes session-flag updates with persistence tests that snapshot the flag.
static CONFIG_SAVE_SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Portable mode folder name (when present next to exe, enables portable mode)
const PORTABLE_DIR_NAME: &str = "portable";

/// Environment variable for overriding the data directory.
/// Used by PortableApps.com launcher and custom portable setups.
pub(crate) const DATA_DIR_ENV_VAR: &str = "FERRITE_DATA_DIR";

// ─────────────────────────────────────────────────────────────────────────────
// Portable Mode Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Check if we're running in portable mode.
///
/// Portable mode is resolved with the following priority:
///
/// 1. **`FERRITE_DATA_DIR` environment variable** — if set and pointing to an
///    existing directory, that path is used. This supports the PortableApps.com
///    launcher and other external packaging systems that set the data path.
/// 2. **`portable/` folder next to the executable** — the original detection
///    method for self-contained portable installs (e.g., USB drive).
///
/// In portable mode, all configuration and data is stored in the resolved
/// folder instead of the system's AppData/config directory.
fn get_portable_dir() -> Option<PathBuf> {
    // Priority 1: FERRITE_DATA_DIR env var (PortableApps.com / custom setups)
    if let Ok(dir) = std::env::var(DATA_DIR_ENV_VAR) {
        if !dir.is_empty() {
            let path = PathBuf::from(&dir);
            if path.is_dir() {
                debug!("Portable mode via {}: {}", DATA_DIR_ENV_VAR, path.display());
                return Some(path);
            }
            // Directory doesn't exist yet — create it so new installs work
            if std::fs::create_dir_all(&path).is_ok() {
                debug!(
                    "Portable mode via {} (created): {}",
                    DATA_DIR_ENV_VAR,
                    path.display()
                );
                return Some(path);
            }
            warn!(
                "{} set to '{}' but directory could not be created, ignoring",
                DATA_DIR_ENV_VAR, dir
            );
        }
    }

    // Priority 2: portable/ folder next to executable
    std::env::current_exe().ok().and_then(|exe| {
        let portable_dir = exe.parent()?.join(PORTABLE_DIR_NAME);
        if portable_dir.exists() && portable_dir.is_dir() {
            debug!("Portable mode detected: {}", portable_dir.display());
            Some(portable_dir)
        } else {
            None
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Platform-Specific Directory Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Get the configuration directory for the application.
///
/// Resolution order:
/// 1. **`FERRITE_DATA_DIR` env var** — used by PortableApps.com and similar launchers
/// 2. **`portable/` folder** next to the executable — self-contained portable installs
/// 3. **Platform-specific directory** (standard install):
///    - **Windows**: `%APPDATA%\ferrite\`
///    - **macOS**: `~/Library/Application Support/ferrite/`
///    - **Linux**: `~/.config/ferrite/`
///
/// # Errors
///
/// Returns `Error::ConfigDirNotFound` if the config directory cannot be determined
/// (e.g., if the HOME environment variable is not set and not in portable mode).
///
/// # Examples
///
/// ```ignore
/// let config_dir = get_config_dir()?;
/// println!("Config directory: {}", config_dir.display());
/// ```
pub fn get_config_dir() -> Result<PathBuf> {
    // Check for portable mode first
    if let Some(portable_dir) = get_portable_dir() {
        return Ok(portable_dir);
    }

    // Fall back to system config directory
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
    load_config_outcome().into_settings()
}

/// Result of reading `config.json`.
///
/// [`ConfigLoadOutcome::CorruptFileRenamed`] is a one-shot signal for a later
/// toast. This load does not write a replacement `config.json`.
#[derive(Debug, Clone)]
pub enum ConfigLoadOutcome {
    /// Settings from disk, or defaults when the file is missing or empty.
    Loaded {
        settings: Settings,
        /// Invalid fields removed by [`Settings::from_json_tolerant`].
        dropped_field_count: usize,
    },
    /// `config.json` was not valid JSON and was renamed to `path`.
    CorruptFileRenamed { settings: Settings, path: PathBuf },
}

/// Which one-shot config load toasts the UI should show (locale keys applied later).
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigLoadToastKind {
    CorruptRenamed,
    InvalidFieldsReset(usize),
}

/// Map startup config load signals to toast kinds (for tests and UI wiring).
#[cfg(test)]
pub fn config_load_toast_kinds(
    corrupt_renamed: bool,
    dropped_field_count: usize,
) -> Vec<ConfigLoadToastKind> {
    let mut kinds = Vec::new();
    if corrupt_renamed {
        kinds.push(ConfigLoadToastKind::CorruptRenamed);
    }
    if dropped_field_count >= 1 {
        kinds.push(ConfigLoadToastKind::InvalidFieldsReset(dropped_field_count));
    }
    kinds
}

impl ConfigLoadOutcome {
    pub fn into_settings(self) -> Settings {
        match self {
            ConfigLoadOutcome::Loaded { settings, .. }
            | ConfigLoadOutcome::CorruptFileRenamed { settings, .. } => settings,
        }
    }

    pub fn dropped_field_count(&self) -> usize {
        match self {
            ConfigLoadOutcome::Loaded {
                dropped_field_count,
                ..
            } => *dropped_field_count,
            ConfigLoadOutcome::CorruptFileRenamed { .. } => 0,
        }
    }

    pub fn corrupt_renamed_path(&self) -> Option<&Path> {
        match self {
            ConfigLoadOutcome::CorruptFileRenamed { path, .. } => Some(path),
            ConfigLoadOutcome::Loaded { .. } => None,
        }
    }
}

/// Load configuration and report a syntax-invalid file that was renamed aside.
pub fn load_config_outcome() -> ConfigLoadOutcome {
    match load_config_internal() {
        Ok(outcome) => outcome,
        Err(e) => {
            note_failed_full_load();
            warn!("Failed to load configuration: {}", e);
            ConfigLoadOutcome::Loaded {
                settings: Settings::default(),
                dropped_field_count: 0,
            }
        }
    }
}

/// Record that this process fell back to all-default settings.
///
/// Sticky for the process: a later load of a missing file (the corrupt file
/// was already renamed) must not clear it.
fn note_failed_full_load() {
    let _test = CONFIG_SAVE_SESSION_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Ok(mut session) = CONFIG_SAVE_SESSION.lock() {
        session.failed_full_load = true;
    }
}

/// Record that the user changed settings.
///
/// Called from the settings-dirty path. Session bookkeeping that only marks
/// the config dirty (recent files, exit save) does not call this, so a failed
/// load still refuses to overwrite `config.json.bak`.
pub fn note_user_settings_edit() {
    if let Ok(mut session) = CONFIG_SAVE_SESSION.lock() {
        session.user_edited_settings = true;
    }
}

/// Skip replacing `.bak` when defaults came from a failed full load and the
/// user has not edited settings. A poisoned lock preserves the backup.
fn should_preserve_existing_bak() -> bool {
    match CONFIG_SAVE_SESSION.lock() {
        Ok(session) => session.failed_full_load && !session.user_edited_settings,
        Err(_) => true,
    }
}

/// Take the corrupt-config path recorded by an earlier load in this process.
///
/// One-shot: a second call returns `None`.
pub fn take_pending_corrupt_config_rename() -> Option<PathBuf> {
    PENDING_CORRUPT_CONFIG_RENAME
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

fn remember_corrupt_rename(path: PathBuf) {
    if let Ok(mut slot) = PENDING_CORRUPT_CONFIG_RENAME.lock() {
        if slot.is_none() {
            *slot = Some(path);
        }
    }
}

/// Internal implementation of config loading.
fn load_config_internal() -> Result<ConfigLoadOutcome> {
    load_config_at(&get_config_file_path()?)
}

fn load_config_at(config_path: &Path) -> Result<ConfigLoadOutcome> {
    // Check if config file exists
    if !config_path.exists() {
        debug!(
            "Config file not found at {}, creating defaults with system locale",
            config_path.display()
        );
        return Ok(ConfigLoadOutcome::Loaded {
            settings: Settings::default_with_system_locale(),
            dropped_field_count: 0,
        });
    }

    debug!("Loading config from: {}", config_path.display());

    // Read the file contents
    let contents = fs::read_to_string(config_path).map_err(|e| Error::ConfigLoad {
        path: config_path.to_path_buf(),
        source: Box::new(e),
    })?;

    // Handle empty file
    if contents.trim().is_empty() {
        debug!("Config file is empty, using defaults");
        return Ok(ConfigLoadOutcome::Loaded {
            settings: Settings::default(),
            dropped_field_count: 0,
        });
    }

    if serde_json::from_str::<serde_json::Value>(&contents).is_err() {
        warn!(
            "Config file at {} contains invalid JSON; renaming aside and using defaults",
            config_path.display()
        );
        let dest = rename_corrupt_config(config_path)?;
        remember_corrupt_rename(dest.clone());
        note_failed_full_load();
        return Ok(ConfigLoadOutcome::CorruptFileRenamed {
            settings: Settings::default(),
            path: dest,
        });
    }

    let load = Settings::from_json_tolerant(&contents).map_err(|e| {
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

    if load.dropped_field_count > 0 {
        warn!(
            "Config at {} recovered with {} invalid field(s) reset to default",
            config_path.display(),
            load.dropped_field_count
        );
    }

    let mut settings = load.settings;

    // Prune recent files and workspaces that no longer exist on disk
    let pruned_files = settings.prune_stale_recent_files();
    let pruned_workspaces = settings.prune_stale_recent_workspaces();
    if pruned_files > 0 || pruned_workspaces > 0 {
        // Save config immediately if we pruned any entries to keep it in sync
        if let Err(e) = save_config(&settings) {
            warn!("Failed to save config after pruning stale entries: {}", e);
        }
    }

    info!(
        "Configuration loaded successfully from {}",
        config_path.display()
    );
    Ok(ConfigLoadOutcome::Loaded {
        settings,
        dropped_field_count: load.dropped_field_count,
    })
}

/// Rename `config.json` to `config.json.corrupt-YYYYMMDD-HHMMSS` and keep 3 copies.
fn rename_corrupt_config(config_path: &Path) -> Result<PathBuf> {
    let config_dir = config_path.parent().ok_or_else(|| Error::ConfigLoad {
        path: config_path.to_path_buf(),
        source: Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "config path has no parent directory",
        )),
    })?;

    let dest = unique_corrupt_config_path(config_dir);
    fs::rename(config_path, &dest).map_err(|e| Error::ConfigLoad {
        path: config_path.to_path_buf(),
        source: Box::new(e),
    })?;
    prune_corrupt_configs(config_dir);
    info!("Preserved invalid config as {}", dest.display());
    Ok(dest)
}

fn unique_corrupt_config_path(config_dir: &Path) -> PathBuf {
    let now = Local::now();
    for extra_secs in 0..60 {
        let stamped = now + chrono::Duration::seconds(extra_secs);
        let name = format!(
            "{}{}",
            CORRUPT_CONFIG_PREFIX,
            stamped.format("%Y%m%d-%H%M%S")
        );
        let dest = config_dir.join(name);
        if !dest.exists() {
            return dest;
        }
    }
    config_dir.join(format!(
        "{}{}-{}",
        CORRUPT_CONFIG_PREFIX,
        now.format("%Y%m%d-%H%M%S"),
        std::process::id()
    ))
}

fn prune_corrupt_configs(config_dir: &Path) {
    let mut corrupt: Vec<PathBuf> = match fs::read_dir(config_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(CORRUPT_CONFIG_PREFIX))
            })
            .collect(),
        Err(e) => {
            warn!(
                "Could not list {} to prune corrupt configs: {}",
                config_dir.display(),
                e
            );
            return;
        }
    };

    corrupt.sort();
    let overflow = corrupt.len().saturating_sub(MAX_CORRUPT_CONFIG_COPIES);
    for path in corrupt.into_iter().take(overflow) {
        if let Err(e) = fs::remove_file(&path) {
            warn!(
                "Failed to remove old corrupt config {}: {}",
                path.display(),
                e
            );
        } else {
            debug!("Removed old corrupt config {}", path.display());
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Save Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Save configuration to the default config file location.
///
/// Write order:
/// 1. Serialize into `config.json.tmp`
/// 2. If `config.json` is readable, copy those bytes to `config.json.bak`
///    (the previous file, not the new payload)
/// 3. Rename `config.json.tmp` onto `config.json`
///
/// When this process started from a failed full load and the user has not
/// edited settings, step 2 is skipped so defaults do not replace a last-good
/// `.bak`.
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
    save_config_in_dir(&config_dir, settings)
}

fn save_config_in_dir(config_dir: &Path, settings: &Settings) -> Result<()> {
    let config_path = config_dir.join(CONFIG_FILE_NAME);
    let backup_path = config_dir.join(CONFIG_BACKUP_NAME);
    let temp_path = config_dir.join(CONFIG_TEMP_NAME);

    debug!("Saving config to: {}", config_path.display());

    let json = serde_json::to_string_pretty(settings).map_err(|e| Error::ConfigSave {
        path: config_path.clone(),
        source: Box::new(e),
    })?;

    if let Err(e) = fs::write(&temp_path, &json) {
        return Err(Error::ConfigSave {
            path: temp_path,
            source: Box::new(e),
        });
    }

    if !should_preserve_existing_bak() {
        match fs::read(&config_path) {
            Ok(previous) => {
                if let Err(e) = fs::write(&backup_path, previous) {
                    let _ = fs::remove_file(&temp_path);
                    return Err(Error::ConfigSave {
                        path: backup_path,
                        source: Box::new(e),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                debug!(
                    "Skipping config backup; {} is not readable: {}",
                    config_path.display(),
                    e
                );
            }
        }
    } else {
        debug!("Preserving existing config.json.bak after failed load without settings edits");
    }

    if let Err(e) = replace_file(&temp_path, &config_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(Error::ConfigSave {
            path: config_path,
            source: Box::new(e),
        });
    }

    info!(
        "Configuration saved successfully to {}",
        config_path.display()
    );
    Ok(())
}

/// Rename `from` onto `to`. If the destination already exists, replace it.
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if to.exists() => {
            debug!(
                "Replacing existing {} after rename failed ({}): {}",
                to.display(),
                e,
                from.display()
            );
            fs::remove_file(to)?;
            fs::rename(from, to)
        }
        Err(e) => Err(e),
    }
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
        config_dir: PathBuf,
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
                config_dir,
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
        // Resolves under the hermetic data dir so the suite does not touch
        // the real user config directory.
        crate::config::with_temp_data_dir(|dir| {
            let path = get_config_dir().expect("config dir");
            assert!(
                path.starts_with(dir),
                "config dir {} is not under {}",
                path.display(),
                dir.display()
            );
            assert!(path.is_dir());
        });
    }

    #[test]
    fn test_get_config_file_path() {
        crate::config::with_temp_data_dir(|dir| {
            let path = get_config_file_path().expect("config file path");
            assert!(path.starts_with(dir));
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some(CONFIG_FILE_NAME)
            );
        });
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

    fn lock_save_session() -> std::sync::MutexGuard<'static, ()> {
        CONFIG_SAVE_SESSION_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn reset_save_session() {
        let mut session = CONFIG_SAVE_SESSION
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        session.failed_full_load = false;
        session.user_edited_settings = false;
    }

    fn set_failed_full_load() {
        let mut session = CONFIG_SAVE_SESSION
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        session.failed_full_load = true;
        session.user_edited_settings = false;
    }

    #[test]
    fn test_save_config_bak_keeps_previous_file() {
        let _guard = lock_save_session();
        reset_save_session();
        let env = TestEnv::new();

        let first = Settings {
            theme: Theme::Dark,
            font_size: 16.0,
            ..Settings::default()
        };
        let second = Settings {
            theme: Theme::Light,
            font_size: 18.0,
            ..Settings::default()
        };
        let first_json = serde_json::to_string_pretty(&first).unwrap();
        let second_json = serde_json::to_string_pretty(&second).unwrap();

        save_config_in_dir(&env.config_dir, &first).unwrap();
        save_config_in_dir(&env.config_dir, &second).unwrap();

        let bak = fs::read_to_string(env.config_dir.join(CONFIG_BACKUP_NAME)).unwrap();
        let live = fs::read_to_string(&env.config_file).unwrap();
        assert_eq!(bak, first_json);
        assert_eq!(live, second_json);
        assert!(!env.config_dir.join(CONFIG_TEMP_NAME).exists());
    }

    #[test]
    fn test_save_config_failed_load_without_edits_keeps_bak() {
        let _guard = lock_save_session();
        reset_save_session();
        set_failed_full_load();
        let env = TestEnv::new();
        let previous = "{\"theme\":\"dark\"}\n";
        fs::write(env.config_dir.join(CONFIG_BACKUP_NAME), previous).unwrap();
        fs::write(&env.config_file, previous).unwrap();

        let defaults = Settings {
            font_size: 20.0,
            ..Settings::default()
        };
        save_config_in_dir(&env.config_dir, &defaults).unwrap();

        let bak = fs::read_to_string(env.config_dir.join(CONFIG_BACKUP_NAME)).unwrap();
        let live = fs::read_to_string(&env.config_file).unwrap();
        assert_eq!(bak, previous);
        assert_eq!(live, serde_json::to_string_pretty(&defaults).unwrap());
        reset_save_session();
    }

    #[test]
    fn test_save_config_failed_load_with_edit_updates_bak() {
        let _guard = lock_save_session();
        reset_save_session();
        set_failed_full_load();
        note_user_settings_edit();
        let env = TestEnv::new();
        let previous = "{\"theme\":\"dark\"}\n";
        fs::write(env.config_dir.join(CONFIG_BACKUP_NAME), previous).unwrap();
        fs::write(&env.config_file, previous).unwrap();

        let edited = Settings {
            theme: Theme::Dark,
            font_size: 22.0,
            ..Settings::default()
        };
        save_config_in_dir(&env.config_dir, &edited).unwrap();

        let bak = fs::read_to_string(env.config_dir.join(CONFIG_BACKUP_NAME)).unwrap();
        let live = fs::read_to_string(&env.config_file).unwrap();
        assert_eq!(bak, previous);
        assert_eq!(live, serde_json::to_string_pretty(&edited).unwrap());
        reset_save_session();
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
            auto_save_enabled_default: true,
            auto_save_delay_ms: 30000,
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
        env.write_config(r#"{"font_size": "not a number", "theme": "dark"}"#);

        let contents = fs::read_to_string(&env.config_file).unwrap();
        let load = Settings::from_json_tolerant(&contents).unwrap();

        assert_eq!(load.dropped_field_count, 1);
        assert_eq!(load.settings.theme, Theme::Dark);
        assert_eq!(load.settings.font_size, 14.0);
    }

    #[test]
    fn test_tolerant_load_multiple_bad_fields_independence() {
        let json = r#"{"theme": "dark", "font_size": "x", "tab_size": false}"#;
        let load = Settings::from_json_tolerant(json).unwrap();
        assert_eq!(load.dropped_field_count, 2);
        assert_eq!(load.settings.theme, Theme::Dark);
        assert_eq!(load.settings.font_size, 14.0);
        assert_eq!(load.settings.tab_size, 4);
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
    fn test_config_load_toast_kinds() {
        assert_eq!(config_load_toast_kinds(false, 0), vec![]);
        assert_eq!(
            config_load_toast_kinds(true, 0),
            vec![ConfigLoadToastKind::CorruptRenamed]
        );
        assert_eq!(
            config_load_toast_kinds(false, 2),
            vec![ConfigLoadToastKind::InvalidFieldsReset(2)]
        );
        assert_eq!(
            config_load_toast_kinds(true, 3),
            vec![
                ConfigLoadToastKind::CorruptRenamed,
                ConfigLoadToastKind::InvalidFieldsReset(3),
            ]
        );
    }

    #[test]
    fn test_load_config_graceful_fallback() {
        // Public API falls back to defaults when the hermetic dir has no file.
        crate::config::with_temp_data_dir(|_dir| {
            let settings = load_config();
            assert_eq!(settings.font_size, 14.0);
        });
    }

    /// `PENDING_CORRUPT_CONFIG_RENAME` is process-global and first-writer-wins,
    /// so tests that trigger a corrupt-config rename must not run concurrently.
    static CORRUPT_RENAME_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_syntax_invalid_config_is_renamed_and_defaults_loaded() {
        let _serial = CORRUPT_RENAME_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let _ = take_pending_corrupt_config_rename();
        let env = TestEnv::new();
        env.write_config("{ not json");

        let outcome = load_config_at(&env.config_file).unwrap();
        let path = outcome
            .corrupt_renamed_path()
            .expect("corrupt rename path")
            .to_path_buf();
        let settings = outcome.into_settings();

        assert!(
            !env.config_file.exists(),
            "original config.json must be renamed"
        );
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(
            name.starts_with("config.json.corrupt-"),
            "renamed file was {name}"
        );
        assert!(path.is_file());
        assert_eq!(settings.font_size, Settings::default().font_size);
        assert_eq!(settings.theme, Settings::default().theme);
        assert_eq!(take_pending_corrupt_config_rename().as_ref(), Some(&path));
        assert!(take_pending_corrupt_config_rename().is_none());
    }

    #[test]
    fn test_corrupt_config_retention_keeps_three() {
        let _serial = CORRUPT_RENAME_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let _ = take_pending_corrupt_config_rename();
        let env = TestEnv::new();

        for _ in 0..5 {
            env.write_config("{ not json");
            let outcome = load_config_at(&env.config_file).unwrap();
            assert!(matches!(
                outcome,
                ConfigLoadOutcome::CorruptFileRenamed { .. }
            ));
        }

        let corrupt: Vec<_> = fs::read_dir(&env.config_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("config.json.corrupt-"))
            })
            .collect();
        assert_eq!(corrupt.len(), 3);
        assert!(
            !env.config_file.exists(),
            "corrupt load must not write a replacement config.json"
        );
        let _ = take_pending_corrupt_config_rename();
    }

    #[test]
    fn test_save_config_silent_returns_bool() {
        crate::config::with_temp_data_dir(|dir| {
            let settings = Settings::default();
            let result = save_config_silent(&settings);
            assert!(result);
            assert!(dir.join(CONFIG_FILE_NAME).is_file());
        });
    }
}
