//! Application state management for Ferrite
//!
//! This module defines the central `AppState` struct that manages all
//! application data and UI state, including the current file, open tabs,
//! settings, and editor state.

// Allow dead code - this module has many state management methods for future use
// - redundant_closure: Sometimes closure is clearer for method reference
#![allow(dead_code)]
#![allow(clippy::redundant_closure)]

use crate::config::{load_config, save_config_silent, Settings, TabInfo, ViewMode};
use crate::workspaces::{filter_events, AppMode, Workspace, WorkspaceEvent, WorkspaceWatcher};
use log::{debug, info, warn};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// File Type Detection
// ─────────────────────────────────────────────────────────────────────────────

/// File types supported by the editor for adaptive UI.
///
/// The editor uses this enum to determine which toolbar buttons and
/// menu items to display based on the active tab's file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    /// Markdown files (.md, .markdown)
    #[default]
    Markdown,
    /// JSON files (.json)
    Json,
    /// YAML files (.yaml, .yml)
    Yaml,
    /// TOML files (.toml)
    Toml,
    /// Unknown or unsupported file type
    Unknown,
}

impl FileType {
    /// Detect file type from a file path based on extension.
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Unknown)
    }

    /// Detect file type from file extension string.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "md" | "markdown" => Self::Markdown,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a markdown file type.
    pub fn is_markdown(&self) -> bool {
        matches!(self, Self::Markdown)
    }

    /// Check if this is a structured data file (JSON, YAML, or TOML).
    pub fn is_structured(&self) -> bool {
        matches!(self, Self::Json | Self::Yaml | Self::Toml)
    }

    /// Get a display name for this file type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Unknown => "Unknown",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tab State (Runtime)
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime state for an open tab.
///
/// This struct holds the complete state of an open document tab,
/// including content and editing state. Different from `TabInfo` which
/// is used for persistence/session restoration.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique identifier for this tab
    pub id: usize,
    /// File path (None for unsaved/new documents)
    pub path: Option<PathBuf>,
    /// Document content
    pub content: String,
    /// Original content (for detecting modifications)
    original_content: String,
    /// Cursor position (line, column) - 0-indexed
    pub cursor_position: (usize, usize),
    /// Text selection range (start_char_index, end_char_index) - None if no selection
    pub selection: Option<(usize, usize)>,
    /// Scroll offset in the editor
    pub scroll_offset: f32,
    /// View mode for this tab (raw or rendered)
    pub view_mode: ViewMode,
    /// Undo history stack
    undo_stack: Vec<String>,
    /// Redo history stack
    redo_stack: Vec<String>,
    /// Maximum undo history size
    max_undo_size: usize,
    /// Content version counter - incremented on undo/redo to signal
    /// external content changes to the editor widget
    content_version: u64,
    /// Cached file type (computed from path, updated on path change)
    file_type: FileType,
    /// Whether the editor should request focus on next frame
    pub needs_focus: bool,
}

impl Tab {
    /// Create a new empty tab.
    ///
    /// New tabs default to Raw view mode and Markdown file type.
    /// The editor will automatically receive focus on the next frame.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            path: None,
            content: String::new(),
            original_content: String::new(),
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw, // New documents default to raw mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size: 100,
            content_version: 0,
            file_type: FileType::Markdown, // New tabs default to markdown
            needs_focus: true, // Auto-focus new tabs
        }
    }

    /// Create a tab with content from a file.
    ///
    /// Newly opened files default to Raw view mode.
    /// File type is detected from the path extension.
    /// The editor will automatically receive focus on the next frame.
    pub fn with_file(id: usize, path: PathBuf, content: String) -> Self {
        let file_type = FileType::from_path(&path);
        Self {
            id,
            path: Some(path),
            content: content.clone(),
            original_content: content,
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw, // Newly opened files default to raw mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size: 100,
            content_version: 0,
            file_type,
            needs_focus: true, // Auto-focus newly opened files
        }
    }

    /// Create a tab from saved session info.
    ///
    /// Restores the view mode from the saved session.
    /// File type is detected from the path extension.
    /// Restored tabs don't auto-focus since we're restoring previous state.
    pub fn from_tab_info(id: usize, info: &TabInfo, content: String) -> Self {
        let file_type = info
            .path
            .as_ref()
            .map(|p| FileType::from_path(p))
            .unwrap_or(FileType::Markdown);
        Self {
            id,
            path: info.path.clone(),
            content: content.clone(),
            original_content: content,
            cursor_position: info.cursor_position,
            selection: None,
            scroll_offset: info.scroll_offset,
            view_mode: info.view_mode, // Restore saved view mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size: 100,
            content_version: 0,
            file_type,
            needs_focus: false, // Don't auto-focus restored tabs
        }
    }

    /// Check if the tab has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.content != self.original_content
    }

    /// Get the display title for this tab.
    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");

        if self.is_modified() {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }

    /// Mark the current content as saved (updates original_content).
    pub fn mark_saved(&mut self) {
        self.original_content = self.content.clone();
    }

    /// Set new content and push current to undo stack.
    pub fn set_content(&mut self, new_content: String) {
        if new_content != self.content {
            // Push current state to undo stack
            self.undo_stack.push(self.content.clone());
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            // Clear redo stack on new edit
            self.redo_stack.clear();
            self.content = new_content;
        }
    }

    /// Undo the last edit.
    ///
    /// Returns `true` if undo was performed.
    /// Increments `content_version` to signal external content change to UI widgets.
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = previous;
            self.content_version = self.content_version.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Redo the last undone edit.
    ///
    /// Returns `true` if redo was performed.
    /// Increments `content_version` to signal external content change to UI widgets.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            self.content_version = self.content_version.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of items in the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of items in the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get the content version counter.
    ///
    /// This counter is incremented whenever content is modified externally
    /// (e.g., via undo/redo). UI widgets can use this to detect when they
    /// need to re-read content from the source.
    pub fn content_version(&self) -> u64 {
        self.content_version
    }

    /// Record that an edit was made externally (e.g., by egui's TextEdit).
    ///
    /// Call this AFTER content has been modified, passing the OLD content
    /// before the modification. This is needed because TextEdit modifies
    /// the content string directly, bypassing `set_content()`.
    ///
    /// This method:
    /// - Pushes the old content to the undo stack
    /// - Clears the redo stack (new edits invalidate redo history)
    /// - Enforces the maximum undo history size
    pub fn record_edit(&mut self, old_content: String) {
        // Only record if content actually changed
        if old_content != self.content {
            self.undo_stack.push(old_content);
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
    }

    /// Convert to TabInfo for session persistence.
    pub fn to_tab_info(&self) -> TabInfo {
        TabInfo {
            path: self.path.clone(),
            modified: self.is_modified(),
            cursor_position: self.cursor_position,
            scroll_offset: self.scroll_offset,
            view_mode: self.view_mode,
        }
    }

    /// Get the current view mode for this tab.
    pub fn get_view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Set the view mode for this tab.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    /// Toggle the view mode between Raw and Rendered.
    pub fn toggle_view_mode(&mut self) -> ViewMode {
        self.view_mode = self.view_mode.toggle();
        self.view_mode
    }

    /// Get the file type for this tab.
    ///
    /// Returns the cached file type, which is determined from the
    /// file path extension. New/unsaved tabs default to Markdown.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Set the file path and update the cached file type.
    ///
    /// This should be called when saving a file with a new path
    /// (e.g., "Save As") to ensure the file type is updated.
    pub fn set_path(&mut self, path: PathBuf) {
        self.file_type = FileType::from_path(&path);
        self.path = Some(path);
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self::new(0) // Defaults to Raw view mode and Markdown file type
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI State
// ─────────────────────────────────────────────────────────────────────────────

/// UI-related state flags.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Whether the settings panel is open
    pub show_settings: bool,
    /// Whether the file browser/open dialog is active
    pub show_file_dialog: bool,
    /// Whether the "save as" dialog is active
    pub show_save_as_dialog: bool,
    /// Whether the about dialog is open
    pub show_about: bool,
    /// Whether a confirmation dialog is open (e.g., unsaved changes)
    pub show_confirm_dialog: bool,
    /// Message for the confirmation dialog
    pub confirm_dialog_message: String,
    /// Pending action after confirmation
    pub pending_action: Option<PendingAction>,
    /// Status bar message (deprecated, use toast_message instead)
    pub status_message: Option<String>,
    /// Whether the find/replace panel is open
    pub show_find_replace: bool,
    /// Find/replace state
    pub find_state: crate::editor::FindState,
    /// Whether to scroll to the current match (set when navigating)
    pub scroll_to_match: bool,
    /// Whether to show error modal
    pub show_error_modal: bool,
    /// Error message for modal
    pub error_message: String,
    /// Temporary toast message (shown in center of status bar)
    pub toast_message: Option<String>,
    /// When the toast message should expire (as seconds since app start)
    pub toast_expires_at: Option<f64>,
    /// Whether the recent files popup is open
    pub show_recent_files_popup: bool,
}

/// Actions that may need confirmation before execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    /// Close a specific tab
    CloseTab(usize),
    /// Close all tabs
    CloseAllTabs,
    /// Exit the application
    Exit,
    /// Open a new file (replacing current)
    OpenFile(PathBuf),
    /// Create a new document
    NewDocument,
}

// ─────────────────────────────────────────────────────────────────────────────
// Application State
// ─────────────────────────────────────────────────────────────────────────────

/// Central application state struct.
///
/// This struct holds all runtime state for the application including:
/// - Open tabs and their content
/// - User settings (loaded from config)
/// - UI state (dialogs, panels, etc.)
/// - Application mode (single file or workspace)
///
/// # Example
///
/// ```ignore
/// let mut state = AppState::new();
/// state.new_tab();
/// state.active_tab_mut().set_content("# Hello".to_string());
/// ```
#[derive(Debug)]
pub struct AppState {
    /// All open tabs
    tabs: Vec<Tab>,
    /// Index of the currently active tab
    active_tab_index: usize,
    /// Next tab ID (for unique identification)
    next_tab_id: usize,
    /// User settings (loaded from config)
    pub settings: Settings,
    /// UI-related state
    pub ui: UiState,
    /// Whether settings have been modified and need saving
    settings_dirty: bool,
    /// Current application mode (single file or workspace)
    pub app_mode: AppMode,
    /// Active workspace (only populated when app_mode is Workspace)
    pub workspace: Option<Workspace>,
    /// File system watcher for workspace mode
    workspace_watcher: Option<WorkspaceWatcher>,
    /// Pending file events from the watcher that need to be processed
    pub pending_file_events: Vec<WorkspaceEvent>,
}

impl AppState {
    /// Create a new AppState with settings loaded from config.
    ///
    /// This initializes the application state by:
    /// 1. Loading settings from the config file (with graceful fallback to defaults)
    /// 2. Restoring previously open tabs from session data (if available)
    /// 3. Creating an initial empty tab if no tabs were restored
    /// 4. Setting up default UI state
    pub fn new() -> Self {
        let settings = load_config();
        info!("AppState initialized with settings");
        debug!(
            "Theme: {:?}, View mode: {:?}",
            settings.theme, settings.view_mode
        );

        let mut state = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            settings,
            ui: UiState::default(),
            settings_dirty: false,
            app_mode: AppMode::default(),
            workspace: None,
            workspace_watcher: None,
            pending_file_events: Vec::new(),
        };

        // Try to restore tabs from previous session
        state.restore_session_tabs();

        // If no tabs were restored, create an initial empty tab
        if state.tabs.is_empty() {
            state.new_tab();
        }

        state
    }

    /// Restore tabs from the previous session.
    ///
    /// This attempts to restore tabs from `settings.last_open_tabs`.
    /// Files that no longer exist are skipped with a warning.
    /// Unsaved tabs (no path) are not restored.
    fn restore_session_tabs(&mut self) {
        let tab_infos: Vec<TabInfo> = self.settings.last_open_tabs.clone();
        let saved_active_index = self.settings.active_tab_index;

        if tab_infos.is_empty() {
            debug!("No tabs to restore from previous session");
            return;
        }

        info!("Restoring {} tab(s) from previous session", tab_infos.len());

        for tab_info in &tab_infos {
            if let Some(path) = &tab_info.path {
                // Try to read the file
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let tab = Tab::from_tab_info(self.next_tab_id, tab_info, content);
                        self.next_tab_id += 1;
                        self.tabs.push(tab);
                        debug!("Restored tab: {}", path.display());
                    }
                    Err(e) => {
                        warn!(
                            "Could not restore tab for '{}': {}. File may have been moved or deleted.",
                            path.display(),
                            e
                        );
                        // Skip this tab - file no longer exists
                    }
                }
            } else {
                // Skip tabs without a path (unsaved documents)
                debug!("Skipping unsaved tab from session restore");
            }
        }

        // Restore active tab index (clamped to valid range)
        if !self.tabs.is_empty() {
            self.active_tab_index = saved_active_index.min(self.tabs.len() - 1);
            info!(
                "Restored {} tab(s), active tab index: {}",
                self.tabs.len(),
                self.active_tab_index
            );
        }
    }

    /// Create AppState with custom settings (useful for testing).
    ///
    /// This also restores tabs from `settings.last_open_tabs` if available.
    pub fn with_settings(settings: Settings) -> Self {
        let mut state = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            settings,
            ui: UiState::default(),
            settings_dirty: false,
            app_mode: AppMode::default(),
            workspace: None,
            workspace_watcher: None,
            pending_file_events: Vec::new(),
        };

        // Try to restore tabs from session data
        state.restore_session_tabs();

        // If no tabs were restored, create an empty tab
        if state.tabs.is_empty() {
            state.new_tab();
        }

        state
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Get the number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get all tabs (read-only).
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get the active tab index.
    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }

    /// Get a reference to the active tab.
    ///
    /// Returns `None` if there are no tabs.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Get a mutable reference to the active tab.
    ///
    /// Returns `None` if there are no tabs.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Get a tab by index.
    pub fn tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Get a mutable tab by index.
    pub fn tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    /// Create a new empty tab and make it active.
    ///
    /// Returns the index of the new tab.
    pub fn new_tab(&mut self) -> usize {
        let tab = Tab::new(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        debug!("Created new tab at index {}", self.active_tab_index);
        self.active_tab_index
    }

    /// Open a file in a new tab.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    pub fn open_file(&mut self, path: PathBuf) -> Result<usize, std::io::Error> {
        self.open_file_with_focus(path, true)
    }

    /// Open a file in a new tab with optional focus control.
    ///
    /// If `focus` is true, the new tab becomes active. If false, the file opens
    /// in the background without switching tabs.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    pub fn open_file_with_focus(
        &mut self,
        path: PathBuf,
        focus: bool,
    ) -> Result<usize, std::io::Error> {
        // Check if file is already open
        if let Some(index) = self.find_tab_by_path(&path) {
            if focus {
                self.active_tab_index = index;
                info!("File already open, switching to tab {}", index);
            } else {
                info!("File already open at tab {} (no focus change)", index);
            }
            return Ok(index);
        }

        // Read file content
        let content = std::fs::read_to_string(&path)?;

        // Create new tab
        let tab = Tab::with_file(self.next_tab_id, path.clone(), content);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
            info!("Opened file: {} (with focus)", path.display());
        } else {
            info!("Opened file: {} (in background)", path.display());
        }

        // Update recent files
        self.settings.add_recent_file(path.clone());
        self.settings_dirty = true;

        Ok(new_index)
    }

    /// Find a tab by file path.
    pub fn find_tab_by_path(&self, path: &PathBuf) -> Option<usize> {
        self.tabs.iter().position(|t| t.path.as_ref() == Some(path))
    }

    /// Set the active tab by index.
    ///
    /// Returns `true` if the index was valid and the tab was switched.
    pub fn set_active_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            debug!("Switched to tab {}", index);
            true
        } else {
            warn!("Invalid tab index: {}", index);
            false
        }
    }

    /// Close a tab by index.
    ///
    /// Returns `true` if the tab was closed, `false` if it has unsaved changes
    /// (use `force_close_tab` to close anyway).
    pub fn close_tab(&mut self, index: usize) -> bool {
        if let Some(tab) = self.tabs.get(index) {
            if tab.is_modified() {
                // Set up confirmation dialog
                self.ui.show_confirm_dialog = true;
                self.ui.confirm_dialog_message =
                    format!("'{}' has unsaved changes. Close anyway?", tab.title());
                self.ui.pending_action = Some(PendingAction::CloseTab(index));
                return false;
            }
        }
        self.force_close_tab(index)
    }

    /// Force close a tab by index, ignoring unsaved changes.
    ///
    /// Returns `true` if the tab existed and was closed.
    pub fn force_close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }

        self.tabs.remove(index);

        // Adjust active tab index
        if self.tabs.is_empty() {
            // Create a new empty tab if all tabs are closed
            self.new_tab();
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }

        debug!(
            "Closed tab {}, active is now {}",
            index, self.active_tab_index
        );
        true
    }

    /// Close the active tab.
    pub fn close_active_tab(&mut self) -> bool {
        self.close_tab(self.active_tab_index)
    }

    /// Check if any tabs have unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.iter().any(|t| t.is_modified())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // File Operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Save the active tab to its file path.
    ///
    /// Returns an error if the tab has no path (use `save_as` instead).
    pub fn save_active_tab(&mut self) -> Result<(), crate::error::Error> {
        let tab = self
            .active_tab_mut()
            .ok_or_else(|| crate::error::Error::Application("No active tab".to_string()))?;

        let path = tab.path.clone().ok_or_else(|| {
            crate::error::Error::Application("No file path set. Use 'Save As' instead.".to_string())
        })?;

        std::fs::write(&path, &tab.content).map_err(|e| crate::error::Error::FileWrite {
            path: path.clone(),
            source: e,
        })?;

        tab.mark_saved();
        info!("Saved file: {}", path.display());
        Ok(())
    }

    /// Save the active tab to a new path.
    pub fn save_active_tab_as(&mut self, path: PathBuf) -> Result<(), crate::error::Error> {
        let tab = self
            .active_tab_mut()
            .ok_or_else(|| crate::error::Error::Application("No active tab".to_string()))?;

        std::fs::write(&path, &tab.content).map_err(|e| crate::error::Error::FileWrite {
            path: path.clone(),
            source: e,
        })?;

        tab.path = Some(path.clone());
        tab.mark_saved();

        // Update recent files
        self.settings.add_recent_file(path.clone());
        self.settings_dirty = true;

        info!("Saved file as: {}", path.display());
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Workspace Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if the app is in workspace mode.
    pub fn is_workspace_mode(&self) -> bool {
        self.app_mode.is_workspace()
    }

    /// Get the workspace root path if in workspace mode.
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.app_mode.workspace_root()
    }

    /// Open a folder as a workspace.
    ///
    /// This switches the app to workspace mode and initializes the file tree.
    /// Returns `Ok(())` if successful, or an error if the folder can't be opened.
    pub fn open_workspace(&mut self, root: PathBuf) -> Result<(), crate::error::Error> {
        if !root.is_dir() {
            return Err(crate::error::Error::Application(format!(
                "Path is not a directory: {}",
                root.display()
            )));
        }

        info!("Opening workspace: {}", root.display());

        // Create the workspace
        let workspace = Workspace::new(root.clone());

        // Create the file watcher
        let watcher = match WorkspaceWatcher::new(root.clone()) {
            Ok(w) => {
                info!("File watcher started for workspace");
                Some(w)
            }
            Err(e) => {
                warn!("Failed to start file watcher: {}", e);
                None
            }
        };

        // Update app mode
        self.app_mode = AppMode::from_folder(root.clone());
        self.workspace = Some(workspace);
        self.workspace_watcher = watcher;
        self.pending_file_events.clear();

        // Add to recent workspaces
        self.settings.add_recent_workspace(root);
        self.settings_dirty = true;

        info!("Workspace opened successfully");
        Ok(())
    }

    /// Close the current workspace and return to single-file mode.
    ///
    /// This saves the workspace state before closing.
    pub fn close_workspace(&mut self) {
        if let Some(workspace) = &self.workspace {
            // Save workspace state before closing
            if let Err(e) = workspace.save_state() {
                warn!("Failed to save workspace state: {}", e);
            }
        }

        self.app_mode = AppMode::SingleFile;
        self.workspace = None;
        self.workspace_watcher = None;
        self.pending_file_events.clear();

        info!("Workspace closed, returned to single-file mode");
    }

    /// Poll the file watcher for new events.
    ///
    /// This should be called periodically (e.g., in the update loop).
    /// Events are stored in pending_file_events for processing.
    pub fn poll_file_watcher(&mut self) {
        if let Some(watcher) = &self.workspace_watcher {
            if let Some(workspace) = &self.workspace {
                let raw_events = watcher.poll_events();
                if !raw_events.is_empty() {
                    // Filter out events for hidden paths
                    let filtered = filter_events(raw_events, &workspace.hidden_patterns);
                    self.pending_file_events.extend(filtered);
                }
            }
        }
    }

    /// Take pending file events (clears the list).
    pub fn take_file_events(&mut self) -> Vec<WorkspaceEvent> {
        std::mem::take(&mut self.pending_file_events)
    }

    /// Get a reference to the current workspace (if any).
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// Get a mutable reference to the current workspace (if any).
    pub fn workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspace.as_mut()
    }

    /// Refresh the workspace file tree.
    ///
    /// Call this after file operations that change the directory structure.
    pub fn refresh_workspace(&mut self) {
        if let Some(workspace) = &mut self.workspace {
            workspace.refresh_file_tree();
            debug!("Workspace file tree refreshed");
        }
    }

    /// Toggle the file tree panel visibility.
    pub fn toggle_file_tree(&mut self) {
        if let Some(workspace) = &mut self.workspace {
            workspace.show_file_tree = !workspace.show_file_tree;
            debug!("File tree visibility: {}", workspace.show_file_tree);
        }
    }

    /// Check if the file tree should be visible.
    pub fn should_show_file_tree(&self) -> bool {
        self.workspace
            .as_ref()
            .map(|w| w.show_file_tree)
            .unwrap_or(false)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Settings Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Update settings and mark as dirty.
    pub fn update_settings<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Settings),
    {
        f(&mut self.settings);
        self.settings_dirty = true;
    }

    /// Mark settings as dirty (needing to be saved).
    pub fn mark_settings_dirty(&mut self) {
        self.settings_dirty = true;
    }

    /// Save settings to config file if modified.
    ///
    /// Returns `true` if settings were saved.
    pub fn save_settings_if_dirty(&mut self) -> bool {
        if self.settings_dirty {
            // Update session restoration data
            self.settings.last_open_tabs = self.tabs.iter().map(|t| t.to_tab_info()).collect();
            self.settings.active_tab_index = self.active_tab_index;

            if save_config_silent(&self.settings) {
                self.settings_dirty = false;
                info!("Settings saved");
                return true;
            }
            warn!("Failed to save settings");
        }
        false
    }

    /// Force save settings to config file.
    pub fn save_settings(&mut self) -> bool {
        self.settings_dirty = true;
        self.save_settings_if_dirty()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Event Handling
    // ─────────────────────────────────────────────────────────────────────────

    /// Handle a confirmed pending action.
    pub fn handle_confirmed_action(&mut self) {
        if let Some(action) = self.ui.pending_action.take() {
            match action {
                PendingAction::CloseTab(index) => {
                    self.force_close_tab(index);
                }
                PendingAction::CloseAllTabs => {
                    self.tabs.clear();
                    self.new_tab();
                }
                PendingAction::Exit => {
                    // Caller should handle exit
                    debug!("Exit confirmed");
                }
                PendingAction::OpenFile(path) => {
                    if let Err(e) = self.open_file(path) {
                        self.show_error(format!("Failed to open file:\n{}", e));
                    }
                }
                PendingAction::NewDocument => {
                    self.new_tab();
                }
            }
        }
        self.ui.show_confirm_dialog = false;
        self.ui.confirm_dialog_message.clear();
    }

    /// Cancel the pending action.
    pub fn cancel_pending_action(&mut self) {
        self.ui.pending_action = None;
        self.ui.show_confirm_dialog = false;
        self.ui.confirm_dialog_message.clear();
    }

    /// Request application exit.
    ///
    /// Returns `true` if exit can proceed immediately, `false` if confirmation is needed.
    pub fn request_exit(&mut self) -> bool {
        if self.has_unsaved_changes() {
            self.ui.show_confirm_dialog = true;
            self.ui.confirm_dialog_message = "You have unsaved changes. Exit anyway?".to_string();
            self.ui.pending_action = Some(PendingAction::Exit);
            false
        } else {
            true
        }
    }

    /// Prepare state for application shutdown.
    ///
    /// This saves settings, workspace state, and performs any necessary cleanup.
    pub fn shutdown(&mut self) {
        // Save workspace state if in workspace mode
        if let Some(workspace) = &self.workspace {
            if let Err(e) = workspace.save_state() {
                warn!("Failed to save workspace state during shutdown: {}", e);
            }
        }

        self.save_settings();
        info!("AppState shutdown complete");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UI State Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Set the status message.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.ui.status_message = Some(message.into());
    }

    /// Clear the status message.
    pub fn clear_status(&mut self) {
        self.ui.status_message = None;
    }

    /// Toggle the settings panel.
    pub fn toggle_settings(&mut self) {
        self.ui.show_settings = !self.ui.show_settings;
    }

    /// Toggle the find/replace panel.
    pub fn toggle_find_replace(&mut self) {
        self.ui.show_find_replace = !self.ui.show_find_replace;
    }

    /// Toggle the about/help panel.
    pub fn toggle_about(&mut self) {
        self.ui.show_about = !self.ui.show_about;
    }

    /// Show an error in a modal dialog.
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.ui.error_message = message.into();
        self.ui.show_error_modal = true;
    }

    /// Dismiss the error modal.
    pub fn dismiss_error(&mut self) {
        self.ui.show_error_modal = false;
        self.ui.error_message.clear();
    }

    /// Show a temporary toast message (disappears after duration).
    ///
    /// `current_time` should be the current app time in seconds.
    /// `duration` is how long to show the message in seconds.
    pub fn show_toast(&mut self, message: impl Into<String>, current_time: f64, duration: f64) {
        self.ui.toast_message = Some(message.into());
        self.ui.toast_expires_at = Some(current_time + duration);
    }

    /// Update toast state - clears expired toasts.
    ///
    /// Call this each frame with the current time.
    pub fn update_toast(&mut self, current_time: f64) {
        if let Some(expires_at) = self.ui.toast_expires_at {
            if current_time >= expires_at {
                self.ui.toast_message = None;
                self.ui.toast_expires_at = None;
            }
        }
    }

    /// Clear any active toast message.
    pub fn clear_toast(&mut self) {
        self.ui.toast_message = None;
        self.ui.toast_expires_at = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Tests
    // ─────────────────────────────────────────────────────────────────────────

    // ─────────────────────────────────────────────────────────────────────────
    // FileType Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("md"), FileType::Markdown);
        assert_eq!(FileType::from_extension("markdown"), FileType::Markdown);
        assert_eq!(FileType::from_extension("MD"), FileType::Markdown);
        assert_eq!(FileType::from_extension("json"), FileType::Json);
        assert_eq!(FileType::from_extension("JSON"), FileType::Json);
        assert_eq!(FileType::from_extension("yaml"), FileType::Yaml);
        assert_eq!(FileType::from_extension("yml"), FileType::Yaml);
        assert_eq!(FileType::from_extension("toml"), FileType::Toml);
        assert_eq!(FileType::from_extension("txt"), FileType::Unknown);
        assert_eq!(FileType::from_extension("rs"), FileType::Unknown);
    }

    #[test]
    fn test_file_type_from_path() {
        assert_eq!(
            FileType::from_path(Path::new("readme.md")),
            FileType::Markdown
        );
        assert_eq!(
            FileType::from_path(Path::new("config.json")),
            FileType::Json
        );
        assert_eq!(
            FileType::from_path(Path::new("docker-compose.yaml")),
            FileType::Yaml
        );
        assert_eq!(FileType::from_path(Path::new("Cargo.toml")), FileType::Toml);
        assert_eq!(FileType::from_path(Path::new("main.rs")), FileType::Unknown);
        assert_eq!(
            FileType::from_path(Path::new("no_extension")),
            FileType::Unknown
        );
    }

    #[test]
    fn test_file_type_helpers() {
        assert!(FileType::Markdown.is_markdown());
        assert!(!FileType::Json.is_markdown());

        assert!(FileType::Json.is_structured());
        assert!(FileType::Yaml.is_structured());
        assert!(FileType::Toml.is_structured());
        assert!(!FileType::Markdown.is_structured());
        assert!(!FileType::Unknown.is_structured());

        assert_eq!(FileType::Markdown.display_name(), "Markdown");
        assert_eq!(FileType::Json.display_name(), "JSON");
        assert_eq!(FileType::Yaml.display_name(), "YAML");
        assert_eq!(FileType::Toml.display_name(), "TOML");
        assert_eq!(FileType::Unknown.display_name(), "Unknown");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_new() {
        let tab = Tab::new(1);
        assert_eq!(tab.id, 1);
        assert!(tab.path.is_none());
        assert!(tab.content.is_empty());
        assert!(!tab.is_modified());
        assert_eq!(tab.view_mode, ViewMode::Raw); // New tabs default to raw mode
        assert_eq!(tab.file_type(), FileType::Markdown); // New tabs default to markdown
    }

    #[test]
    fn test_tab_with_file() {
        let path = PathBuf::from("/test/file.md");
        let content = "# Hello".to_string();
        let tab = Tab::with_file(1, path.clone(), content.clone());

        assert_eq!(tab.id, 1);
        assert_eq!(tab.path, Some(path));
        assert_eq!(tab.content, content);
        assert!(!tab.is_modified());
        assert_eq!(tab.file_type(), FileType::Markdown);
    }

    #[test]
    fn test_tab_file_type_detection() {
        // Markdown file
        let md_tab = Tab::with_file(1, PathBuf::from("readme.md"), String::new());
        assert_eq!(md_tab.file_type(), FileType::Markdown);

        // JSON file
        let json_tab = Tab::with_file(2, PathBuf::from("config.json"), String::new());
        assert_eq!(json_tab.file_type(), FileType::Json);

        // YAML file
        let yaml_tab = Tab::with_file(3, PathBuf::from("docker-compose.yml"), String::new());
        assert_eq!(yaml_tab.file_type(), FileType::Yaml);

        // TOML file
        let toml_tab = Tab::with_file(4, PathBuf::from("Cargo.toml"), String::new());
        assert_eq!(toml_tab.file_type(), FileType::Toml);

        // Unknown file
        let rs_tab = Tab::with_file(5, PathBuf::from("main.rs"), String::new());
        assert_eq!(rs_tab.file_type(), FileType::Unknown);
    }

    #[test]
    fn test_tab_set_path_updates_file_type() {
        let mut tab = Tab::new(1);
        assert_eq!(tab.file_type(), FileType::Markdown);

        tab.set_path(PathBuf::from("config.json"));
        assert_eq!(tab.file_type(), FileType::Json);
        assert_eq!(tab.path, Some(PathBuf::from("config.json")));

        tab.set_path(PathBuf::from("data.yaml"));
        assert_eq!(tab.file_type(), FileType::Yaml);
    }

    #[test]
    fn test_tab_modification_tracking() {
        let mut tab = Tab::new(0);
        assert!(!tab.is_modified());

        tab.set_content("new content".to_string());
        assert!(tab.is_modified());

        tab.mark_saved();
        assert!(!tab.is_modified());
    }

    #[test]
    fn test_tab_title() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.title(), "Untitled");

        tab.set_content("modified".to_string());
        assert_eq!(tab.title(), "Untitled*");

        tab.path = Some(PathBuf::from("/test/document.md"));
        assert_eq!(tab.title(), "document.md*");

        tab.mark_saved();
        assert_eq!(tab.title(), "document.md");
    }

    #[test]
    fn test_tab_undo_redo() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.set_content("second".to_string());
        tab.set_content("third".to_string());

        assert!(tab.can_undo());
        assert!(!tab.can_redo());

        tab.undo();
        assert_eq!(tab.content, "second");
        assert!(tab.can_redo());

        tab.undo();
        assert_eq!(tab.content, "first");

        tab.redo();
        assert_eq!(tab.content, "second");
    }

    #[test]
    fn test_tab_undo_clears_redo_on_edit() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.set_content("second".to_string());

        tab.undo();
        assert!(tab.can_redo());

        tab.set_content("new edit".to_string());
        assert!(!tab.can_redo());
    }

    #[test]
    fn test_tab_record_edit() {
        let mut tab = Tab::new(0);

        // Simulate external edit (like TextEdit does)
        let old_content = tab.content.clone();
        tab.content = "first edit".to_string();
        tab.record_edit(old_content);

        assert!(tab.can_undo());
        assert_eq!(tab.undo_count(), 1);

        // Simulate another edit
        let old_content = tab.content.clone();
        tab.content = "second edit".to_string();
        tab.record_edit(old_content);

        assert_eq!(tab.undo_count(), 2);
        assert!(!tab.can_redo());

        // Undo should restore previous state
        tab.undo();
        assert_eq!(tab.content, "first edit");
        assert!(tab.can_redo());
    }

    #[test]
    fn test_tab_record_edit_no_change() {
        let mut tab = Tab::new(0);
        tab.content = "same content".to_string();

        // Recording with same content should not add to undo stack
        let old_content = tab.content.clone();
        tab.record_edit(old_content);

        assert!(!tab.can_undo());
        assert_eq!(tab.undo_count(), 0);
    }

    #[test]
    fn test_tab_record_edit_clears_redo() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.set_content("second".to_string());
        tab.undo();

        assert!(tab.can_redo());

        // New edit via record_edit should clear redo
        let old_content = tab.content.clone();
        tab.content = "new edit".to_string();
        tab.record_edit(old_content);

        assert!(!tab.can_redo());
    }

    #[test]
    fn test_tab_undo_redo_counts() {
        let mut tab = Tab::new(0);

        assert_eq!(tab.undo_count(), 0);
        assert_eq!(tab.redo_count(), 0);

        tab.set_content("first".to_string());
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 0);

        tab.set_content("second".to_string());
        assert_eq!(tab.undo_count(), 2);

        tab.undo();
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 1);

        tab.undo();
        assert_eq!(tab.undo_count(), 0);
        assert_eq!(tab.redo_count(), 2);

        tab.redo();
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 1);
    }

    #[test]
    fn test_tab_max_undo_size() {
        let mut tab = Tab::new(0);
        // Max undo size is 100 by default

        // Add 105 edits
        for i in 0..105 {
            tab.set_content(format!("edit {}", i));
        }

        // Should be capped at 100
        assert_eq!(tab.undo_count(), 100);

        // Oldest edits should be dropped, so undoing 100 times
        // should get us back to edit 4 (edits 0-4 were dropped)
        for _ in 0..100 {
            tab.undo();
        }

        // After 100 undos, we should be at the oldest kept state
        assert_eq!(tab.content, "edit 4");
        assert!(!tab.can_undo());
    }

    #[test]
    fn test_tab_to_tab_info() {
        let mut tab = Tab::with_file(1, PathBuf::from("/test/file.md"), "content".to_string());
        tab.cursor_position = (10, 5);
        tab.scroll_offset = 100.0;
        tab.view_mode = ViewMode::Rendered;

        let info = tab.to_tab_info();
        assert_eq!(info.path, tab.path);
        assert!(!info.modified);
        assert_eq!(info.cursor_position, (10, 5));
        assert_eq!(info.scroll_offset, 100.0);
        assert_eq!(info.view_mode, ViewMode::Rendered);
    }

    #[test]
    fn test_tab_view_mode_toggle() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.view_mode, ViewMode::Raw);

        let new_mode = tab.toggle_view_mode();
        assert_eq!(new_mode, ViewMode::Rendered);
        assert_eq!(tab.view_mode, ViewMode::Rendered);

        let new_mode = tab.toggle_view_mode();
        assert_eq!(new_mode, ViewMode::Raw);
        assert_eq!(tab.view_mode, ViewMode::Raw);
    }

    #[test]
    fn test_tab_view_mode_get_set() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.get_view_mode(), ViewMode::Raw);

        tab.set_view_mode(ViewMode::Rendered);
        assert_eq!(tab.get_view_mode(), ViewMode::Rendered);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // AppState Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_appstate_new_has_one_tab() {
        let state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0);
    }

    #[test]
    fn test_appstate_with_custom_settings() {
        let mut settings = Settings::default();
        settings.theme = Theme::Dark;
        settings.font_size = 18.0;

        let state = AppState::with_settings(settings);
        assert_eq!(state.settings.theme, Theme::Dark);
        assert_eq!(state.settings.font_size, 18.0);
    }

    #[test]
    fn test_appstate_new_tab() {
        let mut state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);

        let index = state.new_tab();
        assert_eq!(state.tab_count(), 2);
        assert_eq!(state.active_tab_index(), index);
    }

    #[test]
    fn test_appstate_set_active_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        state.new_tab();

        assert!(state.set_active_tab(1));
        assert_eq!(state.active_tab_index(), 1);

        assert!(!state.set_active_tab(10)); // Invalid index
        assert_eq!(state.active_tab_index(), 1); // Unchanged
    }

    #[test]
    fn test_appstate_force_close_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        state.new_tab();
        assert_eq!(state.tab_count(), 3);

        state.force_close_tab(1);
        assert_eq!(state.tab_count(), 2);
    }

    #[test]
    fn test_appstate_close_last_tab_creates_new() {
        let mut state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);

        state.force_close_tab(0);
        // Should have created a new empty tab
        assert_eq!(state.tab_count(), 1);
    }

    #[test]
    fn test_appstate_active_tab_mut() {
        let mut state = AppState::with_settings(Settings::default());
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("Hello, World!".to_string());
        }

        assert_eq!(state.active_tab().unwrap().content, "Hello, World!");
    }

    #[test]
    fn test_appstate_has_unsaved_changes() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(!state.has_unsaved_changes());

        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("modified".to_string());
        }
        assert!(state.has_unsaved_changes());
    }

    #[test]
    fn test_appstate_update_settings() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(!state.settings_dirty);

        state.update_settings(|s| {
            s.theme = Theme::Dark;
        });

        assert_eq!(state.settings.theme, Theme::Dark);
        assert!(state.settings_dirty);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UI State Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ui_state_default() {
        let ui = UiState::default();
        assert!(!ui.show_settings);
        assert!(!ui.show_file_dialog);
        assert!(!ui.show_confirm_dialog);
        assert!(ui.status_message.is_none());
    }

    #[test]
    fn test_appstate_toggle_settings() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(!state.ui.show_settings);

        state.toggle_settings();
        assert!(state.ui.show_settings);

        state.toggle_settings();
        assert!(!state.ui.show_settings);
    }

    #[test]
    fn test_appstate_set_status() {
        let mut state = AppState::with_settings(Settings::default());
        state.set_status("File saved");
        assert_eq!(state.ui.status_message, Some("File saved".to_string()));

        state.clear_status();
        assert!(state.ui.status_message.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Event Handling Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_appstate_request_exit_clean() {
        let mut state = AppState::with_settings(Settings::default());
        // No modifications, should exit immediately
        assert!(state.request_exit());
    }

    #[test]
    fn test_appstate_request_exit_with_changes() {
        let mut state = AppState::with_settings(Settings::default());
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("modified".to_string());
        }

        // Has modifications, should show confirmation
        assert!(!state.request_exit());
        assert!(state.ui.show_confirm_dialog);
        assert_eq!(state.ui.pending_action, Some(PendingAction::Exit));
    }

    #[test]
    fn test_appstate_handle_confirmed_close_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        assert_eq!(state.tab_count(), 2);

        state.ui.pending_action = Some(PendingAction::CloseTab(0));
        state.handle_confirmed_action();

        assert_eq!(state.tab_count(), 1);
        assert!(state.ui.pending_action.is_none());
    }

    #[test]
    fn test_appstate_cancel_pending_action() {
        let mut state = AppState::with_settings(Settings::default());
        state.ui.pending_action = Some(PendingAction::Exit);
        state.ui.show_confirm_dialog = true;

        state.cancel_pending_action();

        assert!(state.ui.pending_action.is_none());
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_pending_action_equality() {
        assert_eq!(PendingAction::Exit, PendingAction::Exit);
        assert_eq!(PendingAction::CloseTab(1), PendingAction::CloseTab(1));
        assert_ne!(PendingAction::CloseTab(1), PendingAction::CloseTab(2));
        assert_ne!(PendingAction::Exit, PendingAction::NewDocument);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Session Restoration Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_from_tab_info() {
        let info = TabInfo {
            path: Some(PathBuf::from("/test/file.md")),
            modified: false,
            cursor_position: (10, 5),
            scroll_offset: 100.0,
            view_mode: ViewMode::Rendered, // Test restoring rendered mode
        };
        let content = "# Test Content".to_string();

        let tab = Tab::from_tab_info(42, &info, content.clone());

        assert_eq!(tab.id, 42);
        assert_eq!(tab.path, info.path);
        assert_eq!(tab.content, content);
        assert_eq!(tab.cursor_position, (10, 5));
        assert_eq!(tab.scroll_offset, 100.0);
        assert_eq!(tab.view_mode, ViewMode::Rendered); // View mode restored
        assert!(!tab.is_modified()); // Content matches original
    }

    #[test]
    fn test_restore_session_tabs_empty_settings() {
        // When last_open_tabs is empty, should create one empty tab
        let settings = Settings::default();
        let state = AppState::with_settings(settings);

        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_with_missing_file() {
        // When a saved tab's file no longer exists, it should be skipped
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: Some(PathBuf::from("/nonexistent/file/that/does/not/exist.md")),
            modified: false,
            cursor_position: (0, 0),
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw,
        }];

        let state = AppState::with_settings(settings);

        // Should fall back to creating an empty tab since the file doesn't exist
        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_skips_unsaved() {
        // Tabs without a path (unsaved) should be skipped during restore
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: None, // Unsaved tab
            modified: true,
            cursor_position: (5, 10),
            scroll_offset: 50.0,
            view_mode: ViewMode::Raw,
        }];

        let state = AppState::with_settings(settings);

        // Should fall back to creating an empty tab since unsaved tabs are skipped
        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_active_index_clamped() {
        // Active tab index should be clamped to valid range
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![]; // No tabs to restore
        settings.active_tab_index = 100; // Invalid index

        let state = AppState::with_settings(settings);

        // Should create one empty tab and active_tab_index should be 0
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0);
    }

    #[test]
    fn test_restore_session_tabs_with_temp_file() {
        use std::io::Write;

        // Create a temporary file to test actual restoration
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_restore.md");
        let test_content = "# Test Restored Content\n\nThis is a test.";

        // Write the test file
        let mut file = std::fs::File::create(&temp_file).expect("Failed to create temp file");
        file.write_all(test_content.as_bytes())
            .expect("Failed to write temp file");
        drop(file);

        // Set up settings with this file (with Rendered view mode)
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: Some(temp_file.clone()),
            modified: false,
            cursor_position: (1, 5),
            scroll_offset: 25.0,
            view_mode: ViewMode::Rendered, // Test restoring view mode
        }];
        settings.active_tab_index = 0;

        let state = AppState::with_settings(settings);

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        // Verify restoration
        assert_eq!(state.tab_count(), 1);
        let tab = state.active_tab().unwrap();
        assert_eq!(tab.path, Some(temp_file));
        assert_eq!(tab.content, test_content);
        assert_eq!(tab.cursor_position, (1, 5));
        assert_eq!(tab.scroll_offset, 25.0);
        assert_eq!(tab.view_mode, ViewMode::Rendered); // View mode restored
        assert!(!tab.is_modified());
    }

    #[test]
    fn test_restore_multiple_tabs_with_temp_files() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file1 = temp_dir.join("ferrite_test_restore1.md");
        let temp_file2 = temp_dir.join("ferrite_test_restore2.md");

        // Write test files
        std::fs::File::create(&temp_file1)
            .unwrap()
            .write_all(b"# File 1")
            .unwrap();
        std::fs::File::create(&temp_file2)
            .unwrap()
            .write_all(b"# File 2")
            .unwrap();

        let mut settings = Settings::default();
        settings.last_open_tabs = vec![
            TabInfo {
                path: Some(temp_file1.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Raw, // First tab in raw mode
            },
            TabInfo {
                path: Some(temp_file2.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Rendered, // Second tab in rendered mode
            },
        ];
        settings.active_tab_index = 1; // Second tab active

        let state = AppState::with_settings(settings);

        // Clean up
        let _ = std::fs::remove_file(&temp_file1);
        let _ = std::fs::remove_file(&temp_file2);

        // Verify
        assert_eq!(state.tab_count(), 2);
        assert_eq!(state.active_tab_index(), 1);
        assert_eq!(state.tab(0).unwrap().content, "# File 1");
        assert_eq!(state.tab(0).unwrap().view_mode, ViewMode::Raw);
        assert_eq!(state.tab(1).unwrap().content, "# File 2");
        assert_eq!(state.tab(1).unwrap().view_mode, ViewMode::Rendered);
    }

    #[test]
    fn test_restore_partial_tabs_missing_file() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_restore_partial.md");

        // Write only one test file
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Existing File")
            .unwrap();

        let mut settings = Settings::default();
        settings.last_open_tabs = vec![
            TabInfo {
                path: Some(PathBuf::from("/nonexistent/file.md")),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Raw,
            },
            TabInfo {
                path: Some(temp_file.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Rendered,
            },
        ];
        settings.active_tab_index = 1;

        let state = AppState::with_settings(settings);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        // Only the existing file should be restored
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0); // Clamped since only 1 tab
        assert_eq!(state.active_tab().unwrap().content, "# Existing File");
        assert_eq!(state.active_tab().unwrap().view_mode, ViewMode::Rendered);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Open File with Focus Control Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_open_file_with_focus_true() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_open_focus_true.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Test Content")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        let initial_tab_count = state.tab_count();

        // Open with focus=true
        let result = state.open_file_with_focus(temp_file.clone(), true);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        let new_index = result.unwrap();
        assert_eq!(state.tab_count(), initial_tab_count + 1);
        assert_eq!(state.active_tab_index(), new_index); // Should be focused
        assert_eq!(state.active_tab().unwrap().content, "# Test Content");
    }

    #[test]
    fn test_open_file_with_focus_false() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_open_focus_false.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Background File")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        let initial_active_index = state.active_tab_index();
        let initial_tab_count = state.tab_count();

        // Open with focus=false
        let result = state.open_file_with_focus(temp_file.clone(), false);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        let new_index = result.unwrap();
        assert_eq!(state.tab_count(), initial_tab_count + 1);
        // Active tab should NOT have changed
        assert_eq!(state.active_tab_index(), initial_active_index);
        // But the file should be in a new tab
        assert_eq!(state.tab(new_index).unwrap().content, "# Background File");
    }

    #[test]
    fn test_open_file_already_open_with_focus() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_already_open.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Already Open")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());

        // Open the file first
        let first_result = state.open_file_with_focus(temp_file.clone(), true);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        assert_ne!(state.active_tab_index(), first_index);

        // Open the same file again with focus=true
        let second_result = state.open_file_with_focus(temp_file.clone(), true);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(second_result.is_ok());
        let second_index = second_result.unwrap();
        // Should return the same index
        assert_eq!(first_index, second_index);
        // Should have switched focus to the existing tab
        assert_eq!(state.active_tab_index(), first_index);
    }

    #[test]
    fn test_open_file_already_open_without_focus() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_already_open_no_focus.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Already Open No Focus")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());

        // Open the file first
        let first_result = state.open_file_with_focus(temp_file.clone(), true);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        let new_tab_index = state.active_tab_index();
        assert_ne!(new_tab_index, first_index);

        // Open the same file again with focus=false
        let second_result = state.open_file_with_focus(temp_file.clone(), false);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(second_result.is_ok());
        let second_index = second_result.unwrap();
        // Should return the same index
        assert_eq!(first_index, second_index);
        // Should NOT have switched focus
        assert_eq!(state.active_tab_index(), new_tab_index);
    }

    #[test]
    fn test_open_file_updates_recent_files() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_recent_update.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Recent Test")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        assert!(state.settings.recent_files.is_empty());

        // Open file (either focus mode should update recent files)
        let result = state.open_file_with_focus(temp_file.clone(), false);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        // Recent files should now contain the opened file
        assert!(!state.settings.recent_files.is_empty());
        assert_eq!(state.settings.recent_files[0], temp_file);
    }
}
