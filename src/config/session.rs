//! Session state persistence for crash-safe recovery
//!
//! This module provides the data structures and persistence logic for
//! saving and restoring the full editor session state, including:
//! - Open tabs with their content and editor state
//! - Active tab and scroll positions
//! - Unsaved content for crash recovery
//! - File modification time tracking for conflict detection

// Allow unused code - these are public API functions that may be used
// in the future or are intentionally kept for API completeness
#![allow(dead_code)]

use crate::config::{persistence::get_config_dir, ViewMode};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Current session state schema version
const SESSION_VERSION: u32 = 1;

/// Session state file name (clean shutdown)
const SESSION_FILE_NAME: &str = "session.json";

/// Crash recovery session file name (periodic saves while running)
const CRASH_RECOVERY_FILE_NAME: &str = "session.recovery.json";

/// Recovery content directory (stores unsaved content per tab)
const RECOVERY_CONTENT_DIR: &str = "recovery";

/// Lock file name (indicates app is running)
const LOCK_FILE_NAME: &str = "session.lock";

/// Default debounce interval for session saves (in seconds)
pub const SESSION_SAVE_DEBOUNCE_SECS: u64 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// Session State Structures
// ─────────────────────────────────────────────────────────────────────────────

/// The full session state that is persisted to disk.
///
/// This captures all information needed to restore the editor session,
/// including tabs, editor states, and recovery information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Schema version for migration support
    pub version: u32,

    /// Timestamp when this session state was saved (Unix timestamp in seconds)
    pub saved_at: u64,

    /// Whether this was a clean shutdown (false = crash recovery needed)
    pub clean_shutdown: bool,

    /// All open tabs with their full state
    pub tabs: Vec<SessionTabState>,

    /// Index of the active tab
    pub active_tab_index: usize,

    /// Application mode at time of save
    #[serde(default)]
    pub app_mode: SessionAppMode,

    /// Whether Zen Mode was enabled at time of save
    #[serde(default)]
    pub zen_mode: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: SESSION_VERSION,
            saved_at: current_timestamp(),
            clean_shutdown: true,
            tabs: Vec::new(),
            active_tab_index: 0,
            app_mode: SessionAppMode::SingleFile,
            zen_mode: false,
        }
    }
}

impl SessionState {
    /// Create a new empty session state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this session has any tabs to restore
    pub fn has_tabs(&self) -> bool {
        !self.tabs.is_empty()
    }

    /// Check if any tabs have unsaved changes that need recovery
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.iter().any(|t| t.has_unsaved_content)
    }

    /// Get tabs that have unsaved content
    pub fn tabs_with_unsaved_content(&self) -> Vec<&SessionTabState> {
        self.tabs.iter().filter(|t| t.has_unsaved_content).collect()
    }

    /// Mark this session as having had a clean shutdown
    pub fn mark_clean_shutdown(&mut self) {
        self.clean_shutdown = true;
        self.saved_at = current_timestamp();
    }

    /// Mark this session as crash recovery (not clean shutdown)
    pub fn mark_crash_recovery(&mut self) {
        self.clean_shutdown = false;
        self.saved_at = current_timestamp();
    }
}

/// Application mode at time of session save
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionAppMode {
    #[default]
    SingleFile,
    /// Workspace mode with the root folder path
    #[serde(rename = "workspace")]
    Workspace {
        /// Root path of the workspace folder
        #[serde(default)]
        root: Option<PathBuf>,
    },
}

impl SessionAppMode {
    /// Check if this is a valid workspace mode with a non-empty path.
    pub fn is_valid_workspace(&self) -> bool {
        match self {
            Self::Workspace { root: Some(path) } => {
                // Path must be non-empty and have valid components
                !path.as_os_str().is_empty() && path.components().count() > 0
            }
            _ => false,
        }
    }

    /// Get the workspace root path if in workspace mode and path is valid.
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        match self {
            Self::Workspace { root: Some(path) } if !path.as_os_str().is_empty() => Some(path),
            _ => None,
        }
    }
}

/// State of a single tab in the session.
///
/// This captures all the information needed to restore a tab,
/// including editor state, scroll positions, and unsaved content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTabState {
    /// Unique tab ID (used for recovery content lookup)
    pub tab_id: usize,

    /// File path (None for unsaved/new files)
    pub path: Option<PathBuf>,

    /// Title for display (used when path is None)
    pub display_title: String,

    /// View mode (raw or rendered)
    pub view_mode: ViewMode,

    /// Primary cursor position as character index
    pub cursor_char_index: usize,

    /// Cursor position as (line, column) for display
    pub cursor_position: (usize, usize),

    /// Selection range if any (start, end) as character indices
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<(usize, usize)>,

    /// Raw mode scroll offset
    pub scroll_offset: f32,

    /// Rendered mode scroll offset (for preserving scroll across mode switches)
    #[serde(default)]
    pub rendered_scroll_offset: f32,

    /// Whether this tab has unsaved content that needs recovery
    pub has_unsaved_content: bool,

    /// File modification time when last read (for conflict detection)
    /// Stored as Unix timestamp in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_mtime: Option<u64>,

    /// Hash of original content when file was opened (for quick conflict check)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_content_hash: Option<u64>,

    /// CSV/TSV delimiter override (None = auto-detect)
    /// Stored as single byte: ',' = 44, '\t' = 9, ';' = 59, '|' = 124
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub csv_delimiter: Option<u8>,
}

impl Default for SessionTabState {
    fn default() -> Self {
        Self {
            tab_id: 0,
            path: None,
            display_title: "Untitled".to_string(),
            view_mode: ViewMode::Raw,
            cursor_char_index: 0,
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            rendered_scroll_offset: 0.0,
            has_unsaved_content: false,
            file_mtime: None,
            original_content_hash: None,
            csv_delimiter: None,
        }
    }
}

impl SessionTabState {
    /// Create a new tab state with the given ID
    pub fn new(tab_id: usize) -> Self {
        Self {
            tab_id,
            ..Default::default()
        }
    }

    /// Check if the file on disk has been modified since we last read it
    pub fn check_file_conflict(&self) -> FileConflictStatus {
        let Some(path) = &self.path else {
            return FileConflictStatus::NoFile;
        };

        if !path.exists() {
            return FileConflictStatus::FileDeleted;
        }

        let Some(saved_mtime) = self.file_mtime else {
            return FileConflictStatus::Unknown;
        };

        match get_file_mtime(path) {
            Some(current_mtime) if current_mtime > saved_mtime => {
                FileConflictStatus::ModifiedOnDisk
            }
            Some(_) => FileConflictStatus::NoConflict,
            None => FileConflictStatus::Unknown,
        }
    }
}

/// Status of file conflict detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileConflictStatus {
    /// No file associated with this tab
    NoFile,
    /// File was deleted from disk
    FileDeleted,
    /// File was modified on disk since our snapshot
    ModifiedOnDisk,
    /// No conflict detected
    NoConflict,
    /// Could not determine conflict status
    Unknown,
}

// ─────────────────────────────────────────────────────────────────────────────
// Recovery Content
// ─────────────────────────────────────────────────────────────────────────────

/// Recovery content for tabs with unsaved changes.
///
/// This is stored separately from the session state to keep the
/// session file small and fast to save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryContent {
    /// Tab ID this content belongs to
    pub tab_id: usize,

    /// The full document content
    pub content: String,

    /// Timestamp when this was saved (Unix timestamp)
    pub saved_at: u64,
}

impl RecoveryContent {
    /// Create new recovery content for a tab
    pub fn new(tab_id: usize, content: String) -> Self {
        Self {
            tab_id,
            content,
            saved_at: current_timestamp(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Recovery Result
// ─────────────────────────────────────────────────────────────────────────────

/// Result of attempting to restore a session
#[derive(Debug, Clone)]
pub struct SessionRestoreResult {
    /// The session state (if found)
    pub session: Option<SessionState>,

    /// Whether this is a crash recovery (not clean shutdown)
    pub is_crash_recovery: bool,

    /// Recovered content for tabs (keyed by tab ID)
    pub recovered_content: HashMap<usize, String>,

    /// Tabs that have file conflicts
    pub conflicted_tabs: Vec<usize>,

    /// Tabs whose files no longer exist
    pub missing_file_tabs: Vec<usize>,
}

impl Default for SessionRestoreResult {
    fn default() -> Self {
        Self {
            session: None,
            is_crash_recovery: false,
            recovered_content: HashMap::new(),
            conflicted_tabs: Vec::new(),
            missing_file_tabs: Vec::new(),
        }
    }
}

impl SessionRestoreResult {
    /// Check if there's anything to restore
    pub fn has_content(&self) -> bool {
        self.session.as_ref().map(|s| s.has_tabs()).unwrap_or(false)
    }

    /// Check if recovery requires user attention (conflicts, missing files, or crash)
    pub fn needs_user_attention(&self) -> bool {
        self.is_crash_recovery
            || !self.conflicted_tabs.is_empty()
            || !self.missing_file_tabs.is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Get the session file path
fn get_session_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(SESSION_FILE_NAME))
}

/// Get the crash recovery file path
fn get_crash_recovery_file_path() -> Option<PathBuf> {
    get_config_dir()
        .ok()
        .map(|dir| dir.join(CRASH_RECOVERY_FILE_NAME))
}

/// Get the recovery content directory path
fn get_recovery_content_dir() -> Option<PathBuf> {
    get_config_dir()
        .ok()
        .map(|dir| dir.join(RECOVERY_CONTENT_DIR))
}

/// Get the lock file path
fn get_lock_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(LOCK_FILE_NAME))
}

/// Save session state to disk (clean shutdown version)
pub fn save_session_state(state: &SessionState) -> bool {
    save_session_to_file(state, false)
}

/// Save session state for crash recovery (periodic saves)
pub fn save_crash_recovery_state(state: &SessionState) -> bool {
    save_session_to_file(state, true)
}

/// Internal function to save session state
fn save_session_to_file(state: &SessionState, is_recovery: bool) -> bool {
    let file_path = if is_recovery {
        get_crash_recovery_file_path()
    } else {
        get_session_file_path()
    };

    let Some(path) = file_path else {
        warn!("Could not determine session file path");
        return false;
    };

    // Log detailed session state info for debugging
    let workspace_info = match &state.app_mode {
        SessionAppMode::SingleFile => "SingleFile".to_string(),
        SessionAppMode::Workspace { root } => {
            if let Some(p) = root {
                format!("Workspace({})", p.display())
            } else {
                "Workspace(None)".to_string()
            }
        }
    };
    debug!(
        "Saving session state: is_recovery={}, app_mode={}, tabs={}, clean_shutdown={}",
        is_recovery,
        workspace_info,
        state.tabs.len(),
        state.clean_shutdown
    );

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            debug!("Creating session directory: {}", parent.display());
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create config directory: {}", e);
                return false;
            }
        }
    }

    // Serialize to JSON
    let json = match serde_json::to_string_pretty(state) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize session state: {}", e);
            return false;
        }
    };

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("tmp");
    debug!("Writing session to temp file: {}", temp_path.display());
    if let Err(e) = fs::write(&temp_path, &json) {
        error!("Failed to write session temp file: {}", e);
        return false;
    }

    debug!("Renaming temp file to: {}", path.display());
    if let Err(e) = fs::rename(&temp_path, &path) {
        error!("Failed to rename session temp file: {}", e);
        // Try to clean up temp file
        let _ = fs::remove_file(&temp_path);
        return false;
    }

    debug!(
        "Successfully saved session state to {} ({} tabs, {})",
        path.display(),
        state.tabs.len(),
        workspace_info
    );
    true
}

/// Load session state from disk
pub fn load_session_state() -> SessionRestoreResult {
    let mut result = SessionRestoreResult::default();

    // Check for crash recovery file first
    let recovery_path = get_crash_recovery_file_path();
    let session_path = get_session_file_path();

    debug!(
        "Loading session state: recovery_path={:?}, session_path={:?}",
        recovery_path, session_path
    );

    // Check if there's a lock file (indicates previous crash)
    let is_crash = check_and_clear_lock_file();
    debug!("Lock file check: is_crash={}", is_crash);

    // Try to load crash recovery file if it exists and is newer
    let (session, from_recovery) = match (&recovery_path, &session_path) {
        (Some(recovery), Some(session)) => {
            let recovery_exists = recovery.exists();
            let session_exists = session.exists();

            debug!(
                "Session file existence: recovery={}, session={}",
                recovery_exists, session_exists
            );

            if recovery_exists && session_exists {
                // Compare modification times
                let recovery_mtime = get_file_mtime(recovery);
                let session_mtime = get_file_mtime(session);

                debug!(
                    "Session file mtimes: recovery={:?}, session={:?}",
                    recovery_mtime, session_mtime
                );

                if recovery_mtime > session_mtime {
                    debug!("Using recovery file (newer)");
                    (load_session_from_file(recovery), true)
                } else {
                    debug!("Using session file (newer or equal)");
                    (load_session_from_file(session), false)
                }
            } else if recovery_exists {
                debug!("Using recovery file (only one exists)");
                (load_session_from_file(recovery), true)
            } else if session_exists {
                debug!("Using session file (only one exists)");
                (load_session_from_file(session), false)
            } else {
                debug!("No session files found");
                (None, false)
            }
        }
        (Some(recovery), None) if recovery.exists() => {
            debug!("Using recovery file (session path not available)");
            (load_session_from_file(recovery), true)
        }
        (None, Some(session)) if session.exists() => {
            debug!("Using session file (recovery path not available)");
            (load_session_from_file(session), false)
        }
        _ => {
            debug!("No session files available");
            (None, false)
        }
    };

    // Determine if this is a crash recovery situation
    result.is_crash_recovery =
        is_crash || (from_recovery && session.as_ref().map(|s| !s.clean_shutdown).unwrap_or(false));
    debug!(
        "Session recovery status: is_crash_recovery={}, from_recovery={}",
        result.is_crash_recovery, from_recovery
    );

    if let Some(mut session) = session {
        // Log workspace mode info
        let workspace_info = match &session.app_mode {
            SessionAppMode::SingleFile => "SingleFile".to_string(),
            SessionAppMode::Workspace { root } => {
                if let Some(p) = root {
                    format!("Workspace({})", p.display())
                } else {
                    "Workspace(None)".to_string()
                }
            }
        };
        debug!(
            "Loaded session: app_mode={}, tabs={}, clean_shutdown={}",
            workspace_info,
            session.tabs.len(),
            session.clean_shutdown
        );

        // Load recovery content for tabs with unsaved changes
        result.recovered_content = load_all_recovery_content();
        debug!(
            "Loaded recovery content for {} tabs",
            result.recovered_content.len()
        );

        // Check for file conflicts
        for tab in &session.tabs {
            match tab.check_file_conflict() {
                FileConflictStatus::ModifiedOnDisk => {
                    result.conflicted_tabs.push(tab.tab_id);
                }
                FileConflictStatus::FileDeleted => {
                    result.missing_file_tabs.push(tab.tab_id);
                }
                _ => {}
            }
        }

        if !result.conflicted_tabs.is_empty() || !result.missing_file_tabs.is_empty() {
            debug!(
                "File status: {} conflicted, {} missing",
                result.conflicted_tabs.len(),
                result.missing_file_tabs.len()
            );
        }

        // Update recovery flag based on content
        if result.is_crash_recovery && session.has_unsaved_changes() {
            session.mark_crash_recovery();
        }

        result.session = Some(session);
    } else {
        debug!("No session to restore");
    }

    result
}

/// Load session from a specific file
fn load_session_from_file(path: &PathBuf) -> Option<SessionState> {
    debug!("Loading session from file: {}", path.display());

    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read session file {}: {}", path.display(), e);
            return None;
        }
    };

    match serde_json::from_str::<SessionState>(&contents) {
        Ok(state) => {
            let workspace_info = match &state.app_mode {
                SessionAppMode::SingleFile => "SingleFile".to_string(),
                SessionAppMode::Workspace { root } => {
                    if let Some(p) = root {
                        format!("Workspace({})", p.display())
                    } else {
                        "Workspace(None)".to_string()
                    }
                }
            };
            info!(
                "Loaded session state from {} ({} tabs, {})",
                path.display(),
                state.tabs.len(),
                workspace_info
            );
            Some(state)
        }
        Err(e) => {
            warn!("Failed to parse session file {}: {}", path.display(), e);
            None
        }
    }
}

/// Save recovery content for a tab
pub fn save_recovery_content(tab_id: usize, content: &str) -> bool {
    let Some(dir) = get_recovery_content_dir() else {
        return false;
    };

    // Ensure directory exists
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            error!("Failed to create recovery directory: {}", e);
            return false;
        }
    }

    let recovery = RecoveryContent::new(tab_id, content.to_string());
    let json = match serde_json::to_string(&recovery) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize recovery content: {}", e);
            return false;
        }
    };

    let file_path = dir.join(format!("{}.json", tab_id));
    let temp_path = file_path.with_extension("tmp");

    if let Err(e) = fs::write(&temp_path, &json) {
        error!("Failed to write recovery content: {}", e);
        return false;
    }

    if let Err(e) = fs::rename(&temp_path, &file_path) {
        error!("Failed to rename recovery content file: {}", e);
        let _ = fs::remove_file(&temp_path);
        return false;
    }

    debug!("Saved recovery content for tab {}", tab_id);
    true
}

/// Load recovery content for a specific tab
pub fn load_recovery_content(tab_id: usize) -> Option<String> {
    let dir = get_recovery_content_dir()?;
    let file_path = dir.join(format!("{}.json", tab_id));

    if !file_path.exists() {
        return None;
    }

    let contents = fs::read_to_string(&file_path).ok()?;
    let recovery: RecoveryContent = serde_json::from_str(&contents).ok()?;

    Some(recovery.content)
}

/// Load all recovery content files
fn load_all_recovery_content() -> HashMap<usize, String> {
    let mut content = HashMap::new();

    let Some(dir) = get_recovery_content_dir() else {
        return content;
    };

    if !dir.exists() {
        return content;
    }

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return content,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(recovery) = serde_json::from_str::<RecoveryContent>(&contents) {
                    content.insert(recovery.tab_id, recovery.content);
                }
            }
        }
    }

    content
}

/// Delete recovery content for a specific tab
pub fn delete_recovery_content(tab_id: usize) -> bool {
    let Some(dir) = get_recovery_content_dir() else {
        return false;
    };

    let file_path = dir.join(format!("{}.json", tab_id));
    if file_path.exists() {
        if let Err(e) = fs::remove_file(&file_path) {
            warn!(
                "Failed to delete recovery content for tab {}: {}",
                tab_id, e
            );
            return false;
        }
    }

    true
}

/// Clear all recovery data (session files and content)
pub fn clear_all_recovery_data() {
    // Delete crash recovery file
    if let Some(path) = get_crash_recovery_file_path() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    // Delete recovery content directory
    if let Some(dir) = get_recovery_content_dir() {
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
    }

    // Clear lock file
    if let Some(path) = get_lock_file_path() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    info!("Cleared all session recovery data");
}

/// Delete the clean session file (after successful restore)
pub fn delete_session_file() {
    if let Some(path) = get_session_file_path() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lock File Management
// ─────────────────────────────────────────────────────────────────────────────

/// Create a lock file to indicate the app is running
pub fn create_lock_file() -> bool {
    let Some(path) = get_lock_file_path() else {
        return false;
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create config directory: {}", e);
                return false;
            }
        }
    }

    let content = format!("{}", std::process::id());
    if let Err(e) = fs::write(&path, content) {
        error!("Failed to create lock file: {}", e);
        return false;
    }

    debug!("Created session lock file");
    true
}

/// Remove the lock file (on clean shutdown)
pub fn remove_lock_file() -> bool {
    let Some(path) = get_lock_file_path() else {
        return false;
    };

    if path.exists() {
        if let Err(e) = fs::remove_file(&path) {
            warn!("Failed to remove lock file: {}", e);
            return false;
        }
    }

    debug!("Removed session lock file");
    true
}

/// Check if lock file exists (indicates crash) and clear it
fn check_and_clear_lock_file() -> bool {
    let Some(path) = get_lock_file_path() else {
        return false;
    };

    if path.exists() {
        // Lock file exists - previous session crashed
        let _ = fs::remove_file(&path);
        info!("Found stale lock file - previous session may have crashed");
        true
    } else {
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Get file modification time as Unix timestamp
fn get_file_mtime(path: &PathBuf) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Simple hash function for content (for quick change detection)
pub fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Save Throttle
// ─────────────────────────────────────────────────────────────────────────────

/// Tracks when the last session save occurred for throttling
#[derive(Debug, Clone)]
pub struct SessionSaveThrottle {
    /// Last save time
    last_save: Option<std::time::Instant>,

    /// Minimum interval between saves
    interval: Duration,

    /// Whether a save is pending (content changed since last save)
    pending: bool,
}

impl Default for SessionSaveThrottle {
    fn default() -> Self {
        Self::new(Duration::from_secs(SESSION_SAVE_DEBOUNCE_SECS))
    }
}

impl SessionSaveThrottle {
    /// Create a new throttle with the given interval
    pub fn new(interval: Duration) -> Self {
        Self {
            last_save: None,
            interval,
            pending: false,
        }
    }

    /// Mark that content has changed and needs saving
    pub fn mark_dirty(&mut self) {
        self.pending = true;
    }

    /// Check if enough time has passed for a save
    pub fn should_save(&self) -> bool {
        if !self.pending {
            return false;
        }

        match self.last_save {
            Some(last) => last.elapsed() >= self.interval,
            None => true,
        }
    }

    /// Record that a save occurred
    pub fn record_save(&mut self) {
        self.last_save = Some(std::time::Instant::now());
        self.pending = false;
    }

    /// Force a save regardless of throttling (for shutdown)
    pub fn force_pending(&mut self) {
        self.pending = true;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Auto-Save Temp File Management
// ─────────────────────────────────────────────────────────────────────────────

/// Directory name for auto-save temp files
const AUTO_SAVE_DIR: &str = "autosave";

/// Get the auto-save directory path (within config dir)
pub fn get_auto_save_dir() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(AUTO_SAVE_DIR))
}

/// Generate a temp file path for auto-saving a document.
///
/// For files with a path, uses a hash of the path to create a unique filename.
/// For unsaved documents, uses the tab ID.
pub fn get_auto_save_path(tab_id: usize, file_path: Option<&PathBuf>) -> Option<PathBuf> {
    let dir = get_auto_save_dir()?;

    let filename = if let Some(path) = file_path {
        // Use path hash + original filename for saved files
        let path_hash = hash_content(&path.to_string_lossy());
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");
        format!("{}_{:016x}.md.autosave", stem, path_hash)
    } else {
        // Use tab ID for unsaved documents
        format!("untitled_{}.md.autosave", tab_id)
    };

    Some(dir.join(filename))
}

/// Auto-save metadata stored alongside content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveMetadata {
    /// Tab ID this auto-save belongs to
    pub tab_id: usize,
    /// Original file path (if any)
    pub original_path: Option<PathBuf>,
    /// Timestamp when auto-saved (Unix timestamp)
    pub saved_at: u64,
    /// Content hash at time of auto-save
    pub content_hash: u64,
}

impl AutoSaveMetadata {
    /// Create new metadata
    pub fn new(tab_id: usize, original_path: Option<PathBuf>, content_hash: u64) -> Self {
        Self {
            tab_id,
            original_path,
            saved_at: current_timestamp(),
            content_hash,
        }
    }
}

/// Save content to auto-save temp file with atomic write.
///
/// Creates the auto-save directory if it doesn't exist.
/// Returns true if save was successful.
pub fn save_auto_save_content(tab_id: usize, file_path: Option<&PathBuf>, content: &str) -> bool {
    let Some(dir) = get_auto_save_dir() else {
        warn!("Could not determine auto-save directory");
        return false;
    };

    // Ensure directory exists
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            error!("Failed to create auto-save directory: {}", e);
            return false;
        }
    }

    let Some(save_path) = get_auto_save_path(tab_id, file_path) else {
        return false;
    };

    // Create metadata
    let content_hash = hash_content(content);
    let metadata = AutoSaveMetadata::new(tab_id, file_path.cloned(), content_hash);

    // Serialize metadata as JSON header followed by content
    let metadata_json = match serde_json::to_string(&metadata) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize auto-save metadata: {}", e);
            return false;
        }
    };

    // Format: metadata JSON on first line, blank line, then content
    let full_content = format!("{}\n\n{}", metadata_json, content);

    // Atomic write: write to temp file, then rename
    let temp_path = save_path.with_extension("tmp");
    if let Err(e) = fs::write(&temp_path, &full_content) {
        error!("Failed to write auto-save temp file: {}", e);
        return false;
    }

    if let Err(e) = fs::rename(&temp_path, &save_path) {
        error!("Failed to rename auto-save temp file: {}", e);
        let _ = fs::remove_file(&temp_path);
        return false;
    }

    debug!("Auto-saved tab {} to {}", tab_id, save_path.display());
    true
}

/// Load auto-save content for a specific file path or tab ID.
///
/// Returns (metadata, content) if found.
pub fn load_auto_save_content(
    tab_id: usize,
    file_path: Option<&PathBuf>,
) -> Option<(AutoSaveMetadata, String)> {
    let save_path = get_auto_save_path(tab_id, file_path)?;

    if !save_path.exists() {
        return None;
    }

    let contents = fs::read_to_string(&save_path).ok()?;

    // Parse: first line is JSON metadata, then blank line, then content
    let mut lines = contents.splitn(3, '\n');
    let metadata_line = lines.next()?;
    let _blank = lines.next()?; // Skip blank line
    let content = lines.next().unwrap_or("");

    let metadata: AutoSaveMetadata = serde_json::from_str(metadata_line).ok()?;

    Some((metadata, content.to_string()))
}

/// Check if an auto-save exists for a file and is newer than the main file.
///
/// Returns Some((metadata, content)) if auto-save exists and is newer,
/// or if the main file doesn't exist but auto-save does.
pub fn check_auto_save_recovery(
    tab_id: usize,
    file_path: Option<&PathBuf>,
) -> Option<(AutoSaveMetadata, String)> {
    let (metadata, content) = load_auto_save_content(tab_id, file_path)?;

    // If no original file, auto-save is the only copy
    let Some(original_path) = file_path else {
        return Some((metadata, content));
    };

    // If original file doesn't exist, return auto-save
    if !original_path.exists() {
        return Some((metadata, content));
    }

    // Compare modification times
    let auto_save_time = metadata.saved_at;
    let file_mtime = get_file_mtime(original_path).unwrap_or(0);

    if auto_save_time > file_mtime {
        // Auto-save is newer
        Some((metadata, content))
    } else {
        // Original file is newer or same, no recovery needed
        None
    }
}

/// Delete the auto-save temp file for a document.
///
/// Call this after manual save to clean up.
pub fn delete_auto_save(tab_id: usize, file_path: Option<&PathBuf>) -> bool {
    let Some(save_path) = get_auto_save_path(tab_id, file_path) else {
        return false;
    };

    if save_path.exists() {
        if let Err(e) = fs::remove_file(&save_path) {
            warn!(
                "Failed to delete auto-save file {}: {}",
                save_path.display(),
                e
            );
            return false;
        }
        debug!("Deleted auto-save file: {}", save_path.display());
    }

    true
}

/// Clear all auto-save temp files.
///
/// Call on clean shutdown if desired.
pub fn clear_all_auto_saves() {
    let Some(dir) = get_auto_save_dir() else {
        return;
    };

    if dir.exists() {
        if let Err(e) = fs::remove_dir_all(&dir) {
            warn!("Failed to clear auto-save directory: {}", e);
        } else {
            info!("Cleared all auto-save temp files");
        }
    }
}

/// List all pending auto-save files.
///
/// Returns list of (tab_id, original_path, metadata) for each auto-save.
pub fn list_auto_saves() -> Vec<(usize, Option<PathBuf>, AutoSaveMetadata)> {
    let mut results = Vec::new();

    let Some(dir) = get_auto_save_dir() else {
        return results;
    };

    if !dir.exists() {
        return results;
    }

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "autosave").unwrap_or(false) {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Some(metadata_line) = contents.lines().next() {
                    if let Ok(metadata) = serde_json::from_str::<AutoSaveMetadata>(metadata_line) {
                        results.push((metadata.tab_id, metadata.original_path.clone(), metadata));
                    }
                }
            }
        }
    }

    results
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_default() {
        let state = SessionState::default();
        assert_eq!(state.version, SESSION_VERSION);
        assert!(state.clean_shutdown);
        assert!(state.tabs.is_empty());
        assert_eq!(state.active_tab_index, 0);
    }

    #[test]
    fn test_session_state_has_unsaved() {
        let mut state = SessionState::default();
        state.tabs.push(SessionTabState {
            has_unsaved_content: false,
            ..Default::default()
        });
        assert!(!state.has_unsaved_changes());

        state.tabs.push(SessionTabState {
            has_unsaved_content: true,
            ..Default::default()
        });
        assert!(state.has_unsaved_changes());
    }

    #[test]
    fn test_session_tab_state_default() {
        let tab = SessionTabState::default();
        assert_eq!(tab.tab_id, 0);
        assert!(tab.path.is_none());
        assert_eq!(tab.view_mode, ViewMode::Raw);
        assert!(!tab.has_unsaved_content);
    }

    #[test]
    fn test_hash_content() {
        let content1 = "Hello, World!";
        let content2 = "Hello, World!";
        let content3 = "Hello, World?";

        assert_eq!(hash_content(content1), hash_content(content2));
        assert_ne!(hash_content(content1), hash_content(content3));
    }

    #[test]
    fn test_session_save_throttle() {
        let mut throttle = SessionSaveThrottle::new(Duration::from_millis(100));

        // Initially should save (first save)
        throttle.mark_dirty();
        assert!(throttle.should_save());

        throttle.record_save();
        assert!(!throttle.should_save());

        // Mark dirty again, but interval hasn't passed
        throttle.mark_dirty();
        assert!(!throttle.should_save()); // Still within interval

        // Wait for interval
        std::thread::sleep(Duration::from_millis(150));
        assert!(throttle.should_save());
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let mut state = SessionState::default();
        state.tabs.push(SessionTabState {
            tab_id: 1,
            path: Some(PathBuf::from("/test/file.md")),
            display_title: "file.md".to_string(),
            view_mode: ViewMode::Rendered,
            cursor_char_index: 100,
            cursor_position: (5, 10),
            selection: Some((50, 100)),
            scroll_offset: 150.0,
            rendered_scroll_offset: 200.0,
            has_unsaved_content: true,
            file_mtime: Some(1234567890),
            original_content_hash: Some(12345),
            csv_delimiter: None,
        });
        state.active_tab_index = 0;

        let json = serde_json::to_string(&state).unwrap();
        let loaded: SessionState = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.tabs.len(), 1);
        assert_eq!(loaded.tabs[0].tab_id, 1);
        assert_eq!(loaded.tabs[0].path, Some(PathBuf::from("/test/file.md")));
        assert_eq!(loaded.tabs[0].view_mode, ViewMode::Rendered);
        assert_eq!(loaded.tabs[0].has_unsaved_content, true);
    }

    #[test]
    fn test_recovery_content_serialization() {
        let recovery = RecoveryContent::new(42, "# Hello\n\nWorld".to_string());

        let json = serde_json::to_string(&recovery).unwrap();
        let loaded: RecoveryContent = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.tab_id, 42);
        assert_eq!(loaded.content, "# Hello\n\nWorld");
    }
}
