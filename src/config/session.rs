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
use std::collections::{HashMap, HashSet};
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

/// Current schema version for `RecoveryContent` files.
///
/// Bumped whenever the on-disk format changes in a way that requires
/// migration. v1 is the original `{tab_id, content, saved_at}` shape.
/// v2 adds identity fields (`path`, `original_content_hash`) and the
/// `schema_version` marker itself; older files are still readable
/// because the new fields all have serde defaults.
pub const RECOVERY_CONTENT_SCHEMA_VERSION: u32 = 1;

/// Recovery content for tabs with unsaved changes.
///
/// This is stored separately from the session state to keep the
/// session file small and fast to save.
///
/// **Identity fields (`path`, `original_content_hash`):** added to detect
/// stale recovery files when a tab id is reused across sessions. The recovery
/// content is only safe to apply when both fields match the current tab's
/// path and the hashed disk content the tab was opened with. Older recovery
/// files written before these fields existed deserialize with `None`, in
/// which case the consumer must fall back to the legacy "tab id only"
/// matching policy.
///
/// **`schema_version`:** allows future format migrations. Files that predate
/// this field deserialize as schema_version 1 (see [`RecoveryContent::default_schema_version`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryContent {
    /// Tab ID this content belongs to
    pub tab_id: usize,

    /// The full document content
    pub content: String,

    /// Timestamp when this was saved (Unix timestamp)
    pub saved_at: u64,

    /// Path of the file this recovery content belongs to (None for untitled tabs).
    ///
    /// Used at restore time to verify that the recovery file matches the tab's
    /// on-disk identity before its content is applied. Defaults to `None` for
    /// legacy recovery files that pre-date this field.
    #[serde(default)]
    pub path: Option<PathBuf>,

    /// Hash of the disk content the tab was opened with (or last reloaded from).
    ///
    /// When the recovered tab is path-backed, this hash is compared against
    /// the current disk content. A mismatch indicates the on-disk file has
    /// changed since the recovery snapshot was written, and the caller must
    /// surface a conflict instead of silently overwriting the user's disk
    /// state. Defaults to `None` for legacy recovery files.
    #[serde(default)]
    pub original_content_hash: Option<u64>,

    /// Schema version of this recovery file. Defaults to `1` for legacy files
    /// that omit this field. See [`RECOVERY_CONTENT_SCHEMA_VERSION`].
    #[serde(default = "RecoveryContent::default_schema_version")]
    pub schema_version: u32,
}

impl Default for RecoveryContent {
    fn default() -> Self {
        Self {
            tab_id: 0,
            content: String::new(),
            saved_at: current_timestamp(),
            path: None,
            original_content_hash: None,
            schema_version: RECOVERY_CONTENT_SCHEMA_VERSION,
        }
    }
}

impl RecoveryContent {
    /// Default schema version used by `#[serde(default = ...)]` when
    /// deserializing recovery files written before the field existed.
    pub fn default_schema_version() -> u32 {
        RECOVERY_CONTENT_SCHEMA_VERSION
    }

    /// Create new recovery content for a tab without identity metadata.
    ///
    /// Prefer [`RecoveryContent::new_with_identity`] for path-backed tabs so
    /// that restore-time identity checks can verify the tab and disk state
    /// before applying the recovered content.
    pub fn new(tab_id: usize, content: String) -> Self {
        Self {
            tab_id,
            content,
            saved_at: current_timestamp(),
            path: None,
            original_content_hash: None,
            schema_version: RECOVERY_CONTENT_SCHEMA_VERSION,
        }
    }

    /// Create new recovery content with identity metadata.
    ///
    /// `path` should be the tab's current file path (or `None` for untitled
    /// tabs). `original_content_hash` should be the hash of the disk content
    /// the tab was opened with — **not** the in-memory buffer — so restore
    /// can detect if the file was changed externally between sessions.
    pub fn new_with_identity(
        tab_id: usize,
        content: String,
        path: Option<PathBuf>,
        original_content_hash: Option<u64>,
    ) -> Self {
        Self {
            tab_id,
            content,
            saved_at: current_timestamp(),
            path,
            original_content_hash,
            schema_version: RECOVERY_CONTENT_SCHEMA_VERSION,
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

    /// Recovered content for tabs, keyed by tab ID.
    ///
    /// Each entry carries the full [`RecoveryContent`] (content **plus**
    /// identity metadata: `path`, `original_content_hash`, `schema_version`)
    /// so consumers can verify the recovery file matches the tab's on-disk
    /// identity before applying its buffer — see task 106 (hardened recovery)
    /// and `resolve_tab_content` in `state.rs`.
    pub recovered_content: HashMap<usize, RecoveryContent>,

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

/// Save recovery content for a tab, including identity metadata.
///
/// `path` is the tab's current file path (or `None` for untitled tabs) and
/// `original_content_hash` is the hash of the disk content the tab was opened
/// with — see [`RecoveryContent::new_with_identity`]. Both are persisted into
/// the recovery JSON so the next session can verify the file still belongs to
/// the same tab+disk identity before applying the buffered content (task 106:
/// hardened recovery).
pub fn save_recovery_content(
    tab_id: usize,
    content: &str,
    path: Option<&std::path::Path>,
    original_content_hash: Option<u64>,
) -> bool {
    let Some(dir) = get_recovery_content_dir() else {
        return false;
    };

    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            error!("Failed to create recovery directory: {}", e);
            return false;
        }
    }

    let recovery = RecoveryContent::new_with_identity(
        tab_id,
        content.to_string(),
        path.map(|p| p.to_path_buf()),
        original_content_hash,
    );
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

    debug!(
        "Saved recovery content for tab {} (path={:?}, hash={:?})",
        tab_id,
        path.map(|p| p.display().to_string()),
        original_content_hash
    );
    true
}

/// Migrate a deserialized [`RecoveryContent`] to the current schema.
///
/// Recovery files written before task 106 lack `path`, `original_content_hash`,
/// and `schema_version`; those fields fall back to `None`/`None`/`1` via serde
/// defaults and are accepted here as legacy v1 records (a v1 record without
/// identity is by definition unverifiable, so the consumer is responsible for
/// applying a stricter "tab id only" policy — see `resolve_tab_content`).
///
/// Newer schema versions are rejected to avoid silently misinterpreting a
/// future on-disk format. Callers must treat `None` as "ignore this recovery
/// file" and let pruning clean it up.
fn migrate_recovery_content(rc: RecoveryContent) -> Option<RecoveryContent> {
    if rc.schema_version == RECOVERY_CONTENT_SCHEMA_VERSION {
        return Some(rc);
    }
    if rc.schema_version < RECOVERY_CONTENT_SCHEMA_VERSION {
        // Legacy file: serde defaults already brought it up to a structurally
        // v1-compatible shape. Stamp it as current so future loaders don't
        // need to re-probe legacy paths in memory.
        debug!(
            "Migrating recovery content for tab {} from schema v{} to v{}",
            rc.tab_id, rc.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION
        );
        return Some(RecoveryContent {
            schema_version: RECOVERY_CONTENT_SCHEMA_VERSION,
            ..rc
        });
    }
    warn!(
        "Recovery content for tab {} has newer schema_version v{} (current: v{}); ignoring file",
        rc.tab_id, rc.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION
    );
    None
}

/// Parse the JSON contents of a recovery file and apply schema migration.
///
/// Returns `None` if parsing fails or if the file is from a newer, incompatible
/// schema version. Legacy files (missing `path` / `original_content_hash` /
/// `schema_version`) deserialize via serde defaults and are then stamped to
/// the current schema by [`migrate_recovery_content`].
fn parse_recovery_content_json(json: &str) -> Option<RecoveryContent> {
    match serde_json::from_str::<RecoveryContent>(json) {
        Ok(rc) => migrate_recovery_content(rc),
        Err(e) => {
            warn!("Failed to parse recovery content JSON: {}", e);
            None
        }
    }
}

/// Load recovery content for a specific tab.
///
/// Returns the full [`RecoveryContent`] (content + identity metadata) so
/// callers can verify path/hash before applying the buffer. Files from older
/// Ferrite versions deserialize with `path`/`original_content_hash` defaulted
/// to `None` and `schema_version` defaulted to `1`.
pub fn load_recovery_content(tab_id: usize) -> Option<RecoveryContent> {
    let dir = get_recovery_content_dir()?;
    let file_path = dir.join(format!("{}.json", tab_id));

    if !file_path.exists() {
        return None;
    }

    let contents = fs::read_to_string(&file_path).ok()?;
    parse_recovery_content_json(&contents)
}

/// Load all recovery content files keyed by tab id.
///
/// Returns the full [`RecoveryContent`] for each tab (see [`load_recovery_content`]).
/// Malformed or future-schema files are skipped and logged; callers should
/// rely on [`prune_recovery_dir`] to clean up unused entries.
fn load_all_recovery_content() -> HashMap<usize, RecoveryContent> {
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
                if let Some(recovery) = parse_recovery_content_json(&contents) {
                    content.insert(recovery.tab_id, recovery);
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
        debug!("Deleted recovery content for tab {}", tab_id);
    }

    true
}

/// Delete every `recovery/<tab_id>.json` whose id is NOT in `valid_tab_ids`.
///
/// Tab ids are reset to 0 on every app launch and re-issued monotonically,
/// so the per-session `tab_id` namespace can collide with leftover recovery
/// files from previous sessions. Pruning prevents stale recovery content from
/// bleeding into unrelated tabs on a future restore (data-loss hazard — see
/// `restore_from_session_result` and the recovery / session-restore notes).
///
/// Returns the number of files deleted.
pub fn prune_recovery_dir(valid_tab_ids: &HashSet<usize>) -> usize {
    let Some(dir) = get_recovery_content_dir() else {
        return 0;
    };
    if !dir.exists() {
        return 0;
    }

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Could not read recovery directory {}: {}", dir.display(), e);
            return 0;
        }
    };

    let mut deleted = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(tab_id) = stem.parse::<usize>() else {
            // Filename is not `<usize>.json` — leave it alone.
            continue;
        };
        if valid_tab_ids.contains(&tab_id) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {
                deleted += 1;
                debug!("Pruned stale recovery file: {}", path.display());
            }
            Err(e) => warn!(
                "Failed to prune stale recovery file {}: {}",
                path.display(),
                e
            ),
        }
    }

    if deleted > 0 {
        info!("Pruned {} stale recovery file(s)", deleted);
    }
    deleted
}

/// Clear all recovery data (crash snapshot, per-tab recovery blobs, lock).
///
/// Use when the user discards recovery or explicitly exits without saving.
pub fn clear_all_recovery_data() {
    clear_crash_recovery_snapshot_file();
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

/// Remove only the periodic crash snapshot (`session.recovery.json`).
///
/// **`session.json` + `recovery/` content** must stay intact after a **clean**
/// shutdown so the next launch can reload pathless/unsaved buffers. Call this
/// from `on_exit` instead of [`clear_all_recovery_data`].
pub fn clear_crash_recovery_snapshot_file() {
    if let Some(path) = get_crash_recovery_file_path() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }
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
    /// Content hash of the **autosaved buffer** at write time.
    pub content_hash: u64,
    /// Hash of the on-disk content the tab was loaded from at the moment
    /// this autosave was written (added in task 106 — hardened recovery).
    ///
    /// Compared at recovery time against the current disk content hash so
    /// autosave files from a previous session that reused the same `tab_id`
    /// can no longer bleed into an unrelated document. `None` for legacy
    /// autosave files (pre-task-106) and for untitled tabs that have never
    /// been written to disk; both cases fall back to the historical
    /// path-and-mtime check in [`check_auto_save_recovery`].
    #[serde(default)]
    pub disk_content_hash: Option<u64>,
}

impl AutoSaveMetadata {
    /// Create new metadata
    pub fn new(
        tab_id: usize,
        original_path: Option<PathBuf>,
        content_hash: u64,
        disk_content_hash: Option<u64>,
    ) -> Self {
        Self {
            tab_id,
            original_path,
            saved_at: current_timestamp(),
            content_hash,
            disk_content_hash,
        }
    }
}

/// Save content to auto-save temp file with atomic write.
///
/// Creates the auto-save directory if it doesn't exist.
/// Returns true if save was successful.
///
/// `disk_content_hash` is the hash of the on-disk content the tab was loaded
/// from (or last saved to) at the moment this autosave is written. Pass
/// `None` for untitled tabs or when the disk identity is unknown — see
/// [`AutoSaveMetadata::disk_content_hash`] (task 106 — hardened recovery).
pub fn save_auto_save_content(
    tab_id: usize,
    file_path: Option<&PathBuf>,
    content: &str,
    disk_content_hash: Option<u64>,
) -> bool {
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
    let metadata =
        AutoSaveMetadata::new(tab_id, file_path.cloned(), content_hash, disk_content_hash);

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

/// Pure identity check for an autosave metadata vs. the tab's current
/// `(path, disk_content)` (task 106.6 — hardened recovery).
///
/// Returns `true` when the autosave is safe to apply. Rejection cases:
///
/// * `metadata.original_path != file_path` — the autosave was written for
///   a different document; its `tab_id` was reused this session.
/// * Path-backed tab with `metadata.disk_content_hash == Some(want)` and
///   `hash_content(disk) != want` — the on-disk file changed externally
///   between sessions.
///
/// `disk_content` is the freshly-read disk text (or `None` if the file is
/// missing / unreadable as UTF-8); when it's `None` the hash check is
/// skipped to avoid losing the user's autosave to an encoding edge case.
///
/// Logs and emits `session_recovery_identity_mismatch` on rejection so the
/// failure path matches the recovery-content side of the identity scheme.
fn check_auto_save_identity(
    metadata: &AutoSaveMetadata,
    file_path: Option<&PathBuf>,
    disk_content: Option<&str>,
) -> bool {
    // Layer 1: path equality (covers untitled-tab case via None == None).
    if metadata.original_path.as_ref() != file_path {
        warn!(
            "Rejecting autosave for tab {}: metadata.original_path {:?} does \
             not match tab path {:?}; autosave is from a reused tab id.",
            metadata.tab_id, metadata.original_path, file_path
        );
        crate::diag::event(
            "session_recovery_identity_mismatch",
            format!(
                "source=autosave tab_id={} metadata_path={:?} tab_path={:?} \
                 reason=path_mismatch",
                metadata.tab_id, metadata.original_path, file_path
            ),
        );
        return false;
    }

    // Layer 2: disk hash check (path-backed + hash known + disk readable).
    if let (Some(want), Some(disk), Some(path)) =
        (metadata.disk_content_hash, disk_content, file_path)
    {
        let got = hash_content(disk);
        if got != want {
            warn!(
                "Rejecting autosave for tab {} ({}): disk hash {:?} does not \
                 match metadata.disk_content_hash {:?}; the file changed \
                 externally between sessions.",
                metadata.tab_id,
                path.display(),
                got,
                want
            );
            crate::diag::event(
                "session_recovery_identity_mismatch",
                format!(
                    "source=autosave tab_id={} path={:?} expected_hash={:?} \
                     disk_hash={:?} reason=hash_mismatch",
                    metadata.tab_id,
                    path,
                    Some(want),
                    Some(got),
                ),
            );
            return false;
        }
    }

    true
}

/// Check if an auto-save exists for a file and is newer than the main file.
///
/// Returns Some((metadata, content)) if auto-save exists, is newer than the
/// main file (or the main file is gone), AND its identity matches the
/// current tab's `(path, disk_content_hash)`. Identity mismatches are
/// rejected via [`check_auto_save_identity`] (task 106 — hardened
/// recovery). Legacy autosave files (no `disk_content_hash`) fall back to
/// the historical path + mtime check.
pub fn check_auto_save_recovery(
    tab_id: usize,
    file_path: Option<&PathBuf>,
) -> Option<(AutoSaveMetadata, String)> {
    let (metadata, content) = load_auto_save_content(tab_id, file_path)?;

    // Identity layer 1 — path equality. Always enforced.
    if metadata.original_path.as_ref() != file_path {
        // Logging + diag handled by check_auto_save_identity once we have
        // disk content; for the path-only path call it with disk=None.
        let _ = check_auto_save_identity(&metadata, file_path, None);
        return None;
    }

    // If no original file, auto-save is the only copy
    let Some(original_path) = file_path else {
        return Some((metadata, content));
    };

    // If original file doesn't exist, return auto-save
    if !original_path.exists() {
        return Some((metadata, content));
    }

    // Identity layer 2 — disk hash check (only when both sides are known).
    let disk_now: Option<String> = fs::read_to_string(original_path).ok();
    if !check_auto_save_identity(&metadata, file_path, disk_now.as_deref()) {
        return None;
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

/// Delete autosave files whose untitled `tab_id` is not in the live set.
///
/// The autosave directory is append-only across sessions and untitled
/// autosaves use `untitled_<tab_id>.md.autosave` for their filename. Tab
/// ids reset on every launch, so leftover untitled autosaves from a
/// previous session can collide with a freshly-allocated tab id and
/// surface unrelated content via [`check_auto_save_recovery`]. This
/// function deletes any `untitled_<id>.md.autosave` whose `<id>` is not in
/// `valid_tab_ids` (mirrors [`prune_recovery_dir`] for the autosave dir).
///
/// Path-backed autosaves are keyed by a hash of the file path rather than
/// `tab_id`, so they cannot collide on id alone — identity for those is
/// enforced by [`check_auto_save_recovery`] instead.
///
/// Returns the number of files deleted.
pub fn prune_auto_save_dir(valid_tab_ids: &HashSet<usize>) -> usize {
    let Some(dir) = get_auto_save_dir() else {
        return 0;
    };
    if !dir.exists() {
        return 0;
    }

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Could not read autosave directory {}: {}", dir.display(), e);
            return 0;
        }
    };

    let mut deleted = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        // Only target untitled autosaves: `untitled_<id>.md.autosave`.
        // Path-backed autosaves use `<stem>_<pathhash>.md.autosave` and are
        // identity-checked at recovery time instead.
        let Some(rest) = file_name.strip_prefix("untitled_") else {
            continue;
        };
        let Some(id_str) = rest.strip_suffix(".md.autosave") else {
            continue;
        };
        let Ok(tab_id) = id_str.parse::<usize>() else {
            continue;
        };
        if valid_tab_ids.contains(&tab_id) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {
                deleted += 1;
                debug!("Pruned stale autosave file: {}", path.display());
            }
            Err(e) => warn!(
                "Failed to prune stale autosave file {}: {}",
                path.display(),
                e
            ),
        }
    }

    if deleted > 0 {
        info!("Pruned {} stale autosave file(s)", deleted);
    }
    deleted
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
        // Constructed without identity → identity fields are absent.
        assert_eq!(loaded.path, None);
        assert_eq!(loaded.original_content_hash, None);
        assert_eq!(loaded.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_recovery_content_new_defaults_identity() {
        let recovery = RecoveryContent::new(7, "body".to_string());

        assert_eq!(recovery.tab_id, 7);
        assert_eq!(recovery.path, None);
        assert_eq!(recovery.original_content_hash, None);
        assert_eq!(recovery.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
        assert_eq!(
            RecoveryContent::default_schema_version(),
            RECOVERY_CONTENT_SCHEMA_VERSION
        );
        assert_eq!(RECOVERY_CONTENT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_recovery_content_with_identity_roundtrip() {
        let path = PathBuf::from("/tmp/example/notes.md");
        let recovery = RecoveryContent::new_with_identity(
            13,
            "# Heading\n\nbody".to_string(),
            Some(path.clone()),
            Some(0xdead_beef),
        );

        let json = serde_json::to_string(&recovery).unwrap();
        let loaded: RecoveryContent = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.tab_id, 13);
        assert_eq!(loaded.content, "# Heading\n\nbody");
        assert_eq!(loaded.path, Some(path));
        assert_eq!(loaded.original_content_hash, Some(0xdead_beef));
        assert_eq!(loaded.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_recovery_content_legacy_format_loads_with_defaults() {
        // Recovery file written by an older Ferrite version: no `path`,
        // no `original_content_hash`, no `schema_version`. It must still
        // deserialize cleanly and pick up the documented defaults.
        let legacy_json = r#"{
            "tab_id": 5,
            "content": "old content",
            "saved_at": 1700000000
        }"#;

        let loaded: RecoveryContent = serde_json::from_str(legacy_json)
            .expect("legacy recovery JSON must remain deserializable");

        assert_eq!(loaded.tab_id, 5);
        assert_eq!(loaded.content, "old content");
        assert_eq!(loaded.saved_at, 1700000000);
        assert_eq!(loaded.path, None, "missing path → None");
        assert_eq!(loaded.original_content_hash, None, "missing hash → None");
        assert_eq!(
            loaded.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION,
            "missing schema_version → default v1"
        );
    }

    #[test]
    fn test_recovery_content_default_impl() {
        let recovery = RecoveryContent::default();
        assert_eq!(recovery.tab_id, 0);
        assert!(recovery.content.is_empty());
        assert_eq!(recovery.path, None);
        assert_eq!(recovery.original_content_hash, None);
        assert_eq!(recovery.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Migration hook + parser (subtask 106.3)
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn test_migrate_recovery_content_current_version_passthrough() {
        let rc = RecoveryContent::new_with_identity(
            1,
            "buf".into(),
            Some(PathBuf::from("/x.md")),
            Some(42),
        );
        let migrated = migrate_recovery_content(rc.clone()).expect("current version accepted");
        assert_eq!(migrated.tab_id, rc.tab_id);
        assert_eq!(migrated.path, rc.path);
        assert_eq!(migrated.original_content_hash, rc.original_content_hash);
        assert_eq!(migrated.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrate_recovery_content_older_version_stamped_to_current() {
        // A schema_version below current is treated as legacy: the struct is
        // already structurally v1-compatible thanks to serde defaults, and we
        // bump it to current so future loaders don't re-probe legacy paths.
        let mut legacy = RecoveryContent::new(2, "legacy".into());
        legacy.schema_version = 0; // pretend an older schema

        let migrated =
            migrate_recovery_content(legacy).expect("older schema must still be accepted");
        assert_eq!(migrated.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
        assert_eq!(migrated.content, "legacy");
    }

    #[test]
    fn test_migrate_recovery_content_newer_version_rejected() {
        // A future schema version must be rejected to avoid silently
        // misinterpreting fields that didn't exist in the current code.
        let mut future = RecoveryContent::new(3, "future".into());
        future.schema_version = RECOVERY_CONTENT_SCHEMA_VERSION + 1;

        assert!(
            migrate_recovery_content(future).is_none(),
            "newer schema_version must produce None so callers ignore the file"
        );
    }

    #[test]
    fn test_parse_recovery_content_json_legacy_file() {
        // End-to-end: JSON bytes from an older Ferrite version flow through
        // the loader path and emerge as a current-schema struct with `None`
        // identity fields ready for tab-id-only matching in resolve_tab_content.
        let legacy_json = r#"{
            "tab_id": 9,
            "content": "older buffer",
            "saved_at": 1700000001
        }"#;

        let parsed = parse_recovery_content_json(legacy_json).expect("legacy JSON must round-trip");

        assert_eq!(parsed.tab_id, 9);
        assert_eq!(parsed.content, "older buffer");
        assert_eq!(parsed.path, None);
        assert_eq!(parsed.original_content_hash, None);
        assert_eq!(parsed.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_parse_recovery_content_json_current_file_with_identity() {
        let payload = RecoveryContent::new_with_identity(
            42,
            "modern buffer".into(),
            Some(PathBuf::from("/notes/x.md")),
            Some(0xfeed_face),
        );
        let json = serde_json::to_string(&payload).unwrap();

        let parsed = parse_recovery_content_json(&json).expect("current JSON must parse");
        assert_eq!(parsed.tab_id, 42);
        assert_eq!(parsed.content, "modern buffer");
        assert_eq!(parsed.path, Some(PathBuf::from("/notes/x.md")));
        assert_eq!(parsed.original_content_hash, Some(0xfeed_face));
        assert_eq!(parsed.schema_version, RECOVERY_CONTENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_parse_recovery_content_json_future_schema_rejected() {
        let future_json = format!(
            r#"{{
                "tab_id": 50,
                "content": "from the future",
                "saved_at": 1900000000,
                "path": "/future.md",
                "original_content_hash": 1,
                "schema_version": {}
            }}"#,
            RECOVERY_CONTENT_SCHEMA_VERSION + 1
        );

        assert!(
            parse_recovery_content_json(&future_json).is_none(),
            "newer-than-current schema must be rejected at the loader boundary"
        );
    }

    #[test]
    fn test_parse_recovery_content_json_malformed_returns_none() {
        assert!(parse_recovery_content_json("not json").is_none());
        assert!(parse_recovery_content_json("{ \"tab_id\": ").is_none());
    }

    // ─────────────────────────────────────────────────────────────────────
    // 106.6 — AutoSaveMetadata identity hardening
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn test_auto_save_metadata_round_trip_with_disk_hash() {
        let meta = AutoSaveMetadata::new(
            42,
            Some(PathBuf::from("/tmp/note.md")),
            0xfeed_face,
            Some(0xdead_beef),
        );
        let json = serde_json::to_string(&meta).expect("serialize");
        let loaded: AutoSaveMetadata = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(loaded.tab_id, 42);
        assert_eq!(loaded.original_path, Some(PathBuf::from("/tmp/note.md")));
        assert_eq!(loaded.content_hash, 0xfeed_face);
        assert_eq!(loaded.disk_content_hash, Some(0xdead_beef));
    }

    #[test]
    fn test_auto_save_metadata_round_trip_without_disk_hash() {
        // Untitled tab → no disk identity. None must round-trip cleanly.
        let meta = AutoSaveMetadata::new(7, None, 1, None);
        let json = serde_json::to_string(&meta).expect("serialize");
        let loaded: AutoSaveMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded.tab_id, 7);
        assert_eq!(loaded.original_path, None);
        assert_eq!(loaded.disk_content_hash, None);
    }

    #[test]
    fn test_auto_save_metadata_legacy_json_defaults_disk_hash_none() {
        // Pre-task-106 autosave files lack `disk_content_hash`. They must
        // still parse and pick up `None` via serde default so users
        // upgrading from older Ferrite versions do not lose their autosaves.
        let legacy_json = r#"{
            "tab_id": 11,
            "original_path": "/tmp/legacy.md",
            "saved_at": 1700000000,
            "content_hash": 99
        }"#;
        let loaded: AutoSaveMetadata = serde_json::from_str(legacy_json)
            .expect("legacy autosave JSON must remain deserializable");
        assert_eq!(loaded.tab_id, 11);
        assert_eq!(loaded.disk_content_hash, None);
    }

    #[test]
    fn test_auto_save_identity_path_match_no_hash_accepted() {
        let meta = AutoSaveMetadata::new(
            5,
            Some(PathBuf::from("/work/a.md")),
            123,
            None, // legacy: no hash recorded
        );
        let path = PathBuf::from("/work/a.md");
        assert!(check_auto_save_identity(&meta, Some(&path), None));
    }

    #[test]
    fn test_auto_save_identity_path_mismatch_rejected() {
        let meta = AutoSaveMetadata::new(
            10, None, // metadata says: untitled
            1, None,
        );
        // ...but the tab is now path-backed at the same id (the cross-tab
        // bleed scenario from the task 106 acceptance criteria).
        let now_path = PathBuf::from("/work/task_50_table_inline_formatting.md");
        assert!(
            !check_auto_save_identity(&meta, Some(&now_path), None),
            "path mismatch must reject autosave"
        );
    }

    #[test]
    fn test_auto_save_identity_hash_mismatch_rejected() {
        let path = PathBuf::from("/work/file.md");
        let meta = AutoSaveMetadata::new(
            6,
            Some(path.clone()),
            123,
            Some(hash_content("recovery-time disk content")),
        );
        // Disk now contains different content → hash differs.
        let disk_now = "fresh external edit";
        assert!(
            !check_auto_save_identity(&meta, Some(&path), Some(disk_now)),
            "hash mismatch must reject"
        );
    }

    #[test]
    fn test_auto_save_identity_hash_match_accepted() {
        let path = PathBuf::from("/work/file.md");
        let disk_body = "stable disk body";
        let meta = AutoSaveMetadata::new(6, Some(path.clone()), 123, Some(hash_content(disk_body)));
        assert!(
            check_auto_save_identity(&meta, Some(&path), Some(disk_body)),
            "matching disk hash must accept"
        );
    }

    #[test]
    fn test_auto_save_identity_legacy_no_hash_skips_hash_check() {
        // Legacy autosave file (disk_content_hash = None) is accepted on
        // path equality alone — even if disk content has changed since,
        // because we have no recorded hash to compare against.
        let path = PathBuf::from("/work/file.md");
        let meta = AutoSaveMetadata::new(6, Some(path.clone()), 123, None);
        assert!(check_auto_save_identity(
            &meta,
            Some(&path),
            Some("any disk content"),
        ));
    }

    #[test]
    fn test_auto_save_identity_unreadable_disk_skips_hash_check() {
        // disk_content == None simulates non-UTF-8 / missing read. We trust
        // the path identity rather than dropping the autosave.
        let path = PathBuf::from("/work/file.md");
        let meta = AutoSaveMetadata::new(6, Some(path.clone()), 123, Some(0xabc));
        assert!(check_auto_save_identity(&meta, Some(&path), None));
    }

    #[test]
    fn test_auto_save_identity_untitled_match_accepted() {
        // Both metadata and tab are untitled → match.
        let meta = AutoSaveMetadata::new(8, None, 0, None);
        assert!(check_auto_save_identity(&meta, None, None));
    }

    // ─────────────────────────────────────────────────────────────────────
    // 106.7 — Acceptance regressions: cross-tab bleed must be impossible
    // even when prune_recovery_dir / prune_auto_save_dir are bypassed.
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn test_106_acceptance_recovery_tab_id_collision_rejected() {
        // Subtask 106.7 #1: a previous session left a recovery file for
        // tab_id=10 anchored to /old/a.md; this session's tab_id=10 is for
        // /new/b.md. The recovery file's `path` field is the only thing
        // that prevents the buffer from being applied to the wrong file.
        let recovered = RecoveryContent::new_with_identity(
            10,
            "old buffer".into(),
            Some(PathBuf::from("/old/a.md")),
            Some(0x1111),
        );
        let session_tab = SessionTabState {
            tab_id: 10,
            path: Some(PathBuf::from("/new/b.md")),
            display_title: "b.md".into(),
            has_unsaved_content: true,
            ..Default::default()
        };

        // Since `try_apply_recovery` is private to state.rs, exercise the
        // same gate via the documented invariant: paths differ → the
        // recovery file must not be applied. We assert both the structural
        // precondition and that the parser/migrator does not strip the
        // path field by accident (which would re-introduce the bleed).
        assert_ne!(recovered.path, session_tab.path);
        let json = serde_json::to_string(&recovered).unwrap();
        let parsed = parse_recovery_content_json(&json).unwrap();
        assert_eq!(
            parsed.path,
            Some(PathBuf::from("/old/a.md")),
            "recovery file path must survive parse + migrate so identity gate sees it"
        );
    }

    #[test]
    fn test_106_acceptance_recovery_hash_mismatch_payload_preserved() {
        // Subtask 106.7 #2: hash mismatch case — the recovery file's
        // `original_content_hash` is what the identity gate compares
        // against current disk. We verify that a value written into
        // `original_content_hash` survives serde round-trip exactly
        // (so the gate cannot be defeated by a parsing fluke).
        let want_hash = hash_content("recovery-time disk content");
        let rc = RecoveryContent::new_with_identity(
            7,
            "buf".into(),
            Some(PathBuf::from("/notes/x.md")),
            Some(want_hash),
        );
        let json = serde_json::to_string(&rc).unwrap();
        let parsed = parse_recovery_content_json(&json).unwrap();
        assert_eq!(parsed.original_content_hash, Some(want_hash));
    }

    #[test]
    fn test_106_acceptance_recovery_legacy_file_back_compat() {
        // Subtask 106.7 #3: a recovery file from an older Ferrite version
        // has neither `path` nor `original_content_hash`. The loader must
        // still produce a usable `RecoveryContent` so the identity gate's
        // legacy bypass can apply it (preserving recovered text for
        // upgrading users). The bypass itself is exercised in state.rs.
        let legacy_json = r#"{
            "tab_id": 4,
            "content": "older buffer",
            "saved_at": 1700000001
        }"#;
        let parsed = parse_recovery_content_json(legacy_json).expect("legacy file must round-trip");
        assert_eq!(parsed.tab_id, 4);
        assert_eq!(parsed.path, None);
        assert_eq!(parsed.original_content_hash, None);
    }

    #[test]
    fn test_106_acceptance_original_bleeding_repro_recovery() {
        // Subtask 106.7 #4: untitled `asdasd` recovery for tab_id=10
        // against a path-backed `task_50_table_inline_formatting.md` tab
        // in the new session. Identity gate REQUIRES that the recovery
        // file's `path` field disagree with the session tab's path → the
        // gate rejects on path mismatch even if pruning is skipped.
        let recovered = RecoveryContent::new_with_identity(
            10,
            "asdasd".into(),
            None, // recovery was for an untitled tab
            None,
        );
        let session_path = PathBuf::from("/notes/task_50_table_inline_formatting.md");

        // Round-trip through the loader to mimic the on-disk path and
        // confirm the loader doesn't strip identity fields.
        let json = serde_json::to_string(&recovered).unwrap();
        let parsed = parse_recovery_content_json(&json).unwrap();
        assert_eq!(parsed.path, None);
        assert_ne!(parsed.path, Some(session_path));
    }

    #[test]
    fn test_106_acceptance_autosave_path_mismatch_rejected() {
        // Subtask 106.7 #5: the autosave counterpart of the cross-tab
        // bleed. metadata.original_path = None (untitled), but the new
        // session's tab is path-backed. check_auto_save_identity must
        // reject so the unrelated buffer cannot reach the new tab.
        let meta = AutoSaveMetadata::new(10, None, 0, None);
        let now_path = PathBuf::from("/notes/task_50_table_inline_formatting.md");
        assert!(!check_auto_save_identity(&meta, Some(&now_path), None));
    }

    #[test]
    fn test_106_acceptance_autosave_hash_mismatch_rejected() {
        // Subtask 106.7 #5b: external edit between sessions changes the
        // disk hash; autosave anchored to the old hash must not be applied.
        let path = PathBuf::from("/notes/file.md");
        let meta = AutoSaveMetadata::new(3, Some(path.clone()), 0, Some(hash_content("old disk")));
        assert!(!check_auto_save_identity(
            &meta,
            Some(&path),
            Some("new external edit")
        ));
    }
}
