//! Main application module for Ferrite
//!
//! This module implements the eframe App trait for the main application,
//! handling window management, UI updates, and event processing.

// Allow clippy lints for this large application module:
// - if_same_then_else: Tab hover cursor handling intentionally uses same code for clarity
// - option_map_unit_fn: Keyboard handling closure pattern is clearer than suggested alternative
// - explicit_counter_loop: Loop counter pattern is clearer for some string processing
#![allow(clippy::if_same_then_else)]
#![allow(clippy::option_map_unit_fn)]
#![allow(clippy::explicit_counter_loop)]

use crate::config::{
    apply_snippet, find_trigger_at_cursor, CjkFontPreference, Settings, ShortcutCommand, SnippetManager, Theme, ViewMode, WindowSize,
};
use crate::editor::{
    extract_outline_for_file, DocumentOutline, DocumentStats, EditorWidget, FindReplacePanel,
    Minimap, OutlineType, SearchHighlights, SemanticMinimap, TextStats,
};
use crate::export::{copy_html_to_clipboard, generate_html_document};
use crate::files::dialogs::{open_multiple_files_dialog, save_file_dialog};
use crate::fonts;
use crate::markdown::{
    apply_raw_format, cleanup_rendered_editor_memory, delimiter_display_name, delimiter_symbol,
    detect_raw_formatting_state, get_structured_file_type, get_tabular_file_type,
    insert_or_update_toc, CsvViewer, CsvViewerState, EditorMode, FormattingState, MarkdownEditor,
    MarkdownFormatCommand, TocOptions, TreeViewer, TreeViewerState, DELIMITERS,
};
// Note: SyncScrollState is available for future split-view sync scrolling
#[allow(unused_imports)]
use crate::preview::SyncScrollState;
use crate::state::{AppState, FileType, PendingAction, Selection};
use crate::theme::{ThemeColors, ThemeManager};
use crate::vcs::GitAutoRefresh;
use crate::ui::{
    handle_window_resize, load_app_logo_texture, AboutPanel, FileOperationDialog, FileOperationResult,
    FileTreeContextAction, FileTreePanel, GoToLineResult, OutlinePanel, QuickSwitcher, Ribbon,
    RibbonAction, SearchNavigationTarget, SearchPanel, SettingsPanel, TitleBarButton,
    ViewModeSegment, ViewSegmentAction, WindowResizeState,
};
use eframe::egui;
use log::{debug, info, trace, warn};
use rust_i18n::t;
use std::collections::HashMap;

/// Get the display name for the primary modifier key.
/// Returns "Cmd" on macOS, "Ctrl" on Windows/Linux.
///
/// This is used for displaying keyboard shortcuts in the UI.
/// The actual keyboard handling uses `egui::Modifiers::command` which
/// automatically maps to the correct key per platform.
pub fn modifier_symbol() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// Keyboard shortcut actions that need to be deferred.
///
/// These actions are detected in the input handling closure and executed
/// afterwards to avoid borrow conflicts.
#[derive(Debug, Clone, Copy)]
enum KeyboardAction {
    /// Save current file (Ctrl+S)
    Save,
    /// Save As dialog (Ctrl+Shift+S)
    SaveAs,
    /// Open file dialog (Ctrl+O)
    Open,
    /// New file (Ctrl+N)
    New,
    /// New tab (Ctrl+T)
    NewTab,
    /// Close current tab (Ctrl+W)
    CloseTab,
    /// Next tab (Ctrl+Tab)
    NextTab,
    /// Previous tab (Ctrl+Shift+Tab)
    PrevTab,
    /// Toggle view mode (Ctrl+E)
    ToggleViewMode,
    /// Cycle theme (Ctrl+Shift+T)
    CycleTheme,
    /// Open settings panel (Ctrl+,)
    OpenSettings,
    /// Open find panel (Ctrl+F)
    OpenFind,
    /// Open find and replace panel (Ctrl+H)
    OpenFindReplace,
    /// Find next match (F3)
    FindNext,
    /// Find previous match (Shift+F3)
    FindPrev,
    /// Apply markdown formatting
    Format(MarkdownFormatCommand),
    /// Toggle outline panel (Ctrl+Shift+O)
    ToggleOutline,
    /// Toggle file tree panel (Ctrl+B)
    ToggleFileTree,
    /// Open quick file switcher (Ctrl+P)
    QuickOpen,
    /// Search in files (Ctrl+Shift+F)
    SearchInFiles,
    /// Export as HTML (Ctrl+Shift+E)
    ExportHtml,
    /// Open about/help panel (F1)
    OpenAbout,
    /// Select next occurrence of current word/selection (Ctrl+D)
    SelectNextOccurrence,
    /// Exit multi-cursor mode (Escape when multi-cursor active)
    ExitMultiCursor,
    /// Toggle Zen Mode (F11)
    ToggleZenMode,
    /// Toggle OS fullscreen (F10)
    ToggleFullscreen,
    /// Fold all regions (Ctrl+Shift+[)
    FoldAll,
    /// Unfold all regions (Ctrl+Shift+])
    UnfoldAll,
    /// Toggle fold at cursor (Ctrl+Shift+.)
    ToggleFoldAtCursor,
    /// Toggle Live Pipeline panel (Ctrl+Shift+L)
    TogglePipeline,
    /// Open Go to Line dialog (Ctrl+G)
    GoToLine,
    /// Duplicate current line or selection (Ctrl+Shift+D)
    DuplicateLine,
    /// Delete current line (Ctrl+D)
    DeleteLine,
    /// Insert/Update Table of Contents (Ctrl+Shift+U)
    InsertToc,
}

/// Request to navigate to a heading in the document.
/// Used for both outline panel and semantic minimap navigation.
#[derive(Debug, Clone)]
struct HeadingNavRequest {
    /// Target line number (1-indexed)
    line: usize,
    /// Character offset in the document (for precise positioning)
    char_offset: Option<usize>,
    /// Heading title text (for text-based search and matching)
    title: Option<String>,
    /// Heading level (1-6) for constructing the markdown pattern
    level: Option<u8>,
}

/// Information about a pending auto-save recovery for user confirmation.
#[derive(Debug, Clone)]
struct AutoSaveRecoveryInfo {
    /// Tab ID that has recovery available
    tab_id: usize,
    /// Tab index in the tabs array
    tab_index: usize,
    /// File path (if any)
    path: Option<std::path::PathBuf>,
    /// Recovered content from auto-save
    recovered_content: String,
    /// Timestamp when auto-save was created
    saved_at: u64,
}

/// The main application struct that holds all state and implements eframe::App.
pub struct FerriteApp {
    /// Central application state
    state: AppState,
    /// Theme manager for handling theme switching
    theme_manager: ThemeManager,
    /// Ribbon UI component
    ribbon: Ribbon,
    /// Settings panel component
    settings_panel: SettingsPanel,
    /// About/Help panel component
    about_panel: AboutPanel,
    /// Find/replace panel component
    find_replace_panel: FindReplacePanel,
    /// Outline panel component
    outline_panel: OutlinePanel,
    /// File tree panel component (for workspace mode)
    file_tree_panel: FileTreePanel,
    /// Quick file switcher (Ctrl+P) for workspace mode
    quick_switcher: QuickSwitcher,
    /// Active file operation dialog (New File, Rename, Delete, etc.)
    file_operation_dialog: Option<FileOperationDialog>,
    /// Search in files panel (Ctrl+Shift+F)
    search_panel: SearchPanel,
    /// Live Pipeline panel for JSON/YAML command piping
    pipeline_panel: crate::ui::PipelinePanel,
    /// Cached document outline (updated when content changes)
    cached_outline: DocumentOutline,
    /// Cached document statistics for markdown files (updated when content changes)
    cached_doc_stats: Option<DocumentStats>,
    /// Hash of the last content used to generate outline (for change detection)
    last_outline_content_hash: u64,
    /// Pending scroll-to-line request from outline navigation (1-indexed)
    pending_scroll_to_line: Option<usize>,
    /// Tree viewer states per tab (keyed by tab ID)
    tree_viewer_states: HashMap<usize, TreeViewerState>,
    /// CSV viewer states per tab (keyed by tab ID)
    csv_viewer_states: HashMap<usize, CsvViewerState>,
    /// Sync scroll states per tab (keyed by tab ID)
    /// Note: Reserved for future split-view bidirectional sync scrolling
    #[allow(dead_code)]
    sync_scroll_states: HashMap<usize, SyncScrollState>,
    /// Track if we should exit (after confirmation)
    should_exit: bool,
    /// Last known window size (for detecting changes)
    last_window_size: Option<egui::Vec2>,
    /// Last known window position (for detecting changes)
    last_window_pos: Option<egui::Pos2>,
    /// Application start time for timing toast messages
    start_time: std::time::Instant,
    /// Previous view mode for detecting mode switches (for sync scroll)
    #[allow(dead_code)]
    previous_view_mode: Option<ViewMode>,
    /// Window resize state for borderless window edge dragging
    window_resize_state: WindowResizeState,
    /// Session save throttle for crash recovery persistence
    session_save_throttle: crate::config::SessionSaveThrottle,
    /// Git auto-refresh manager for automatic status updates
    git_auto_refresh: GitAutoRefresh,
    /// Whether we're showing the crash recovery dialog
    show_recovery_dialog: bool,
    /// Pending session restore result (set on startup if crash recovery detected)
    pending_recovery: Option<crate::config::SessionRestoreResult>,
    /// Pending auto-save recovery info (for showing recovery dialog)
    pending_auto_save_recovery: Option<AutoSaveRecoveryInfo>,
    /// Snippet manager for text expansion
    snippet_manager: SnippetManager,
    /// Frame counter for FPS tracking (diagnostic for repaint optimization)
    #[cfg(debug_assertions)]
    frame_count: u64,
    /// Last time we logged FPS (diagnostic for repaint optimization)
    #[cfg(debug_assertions)]
    last_fps_log: std::time::Instant,
    /// Last time user interacted with the app (for idle detection)
    last_interaction_time: std::time::Instant,
    /// Last window title (to avoid sending viewport commands every frame)
    last_window_title: String,
    /// App logo texture for title bar display (with transparent background)
    app_logo_texture: Option<egui::TextureHandle>,
}

impl FerriteApp {
    /// Create a new FerriteApp instance.
    ///
    /// This initializes the application state from the config file and applies
    /// the saved theme preference. It also checks for crash recovery and
    /// restores the previous session if needed.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        use crate::config::{create_lock_file, load_session_state, SessionSaveThrottle};

        info!("Initializing Ferrite");

        // Create lock file to detect crashes on next startup
        create_lock_file();

        // Set up custom fonts with lazy CJK loading for faster startup
        // CJK fonts will be loaded on-demand when CJK text is detected
        fonts::setup_fonts_lazy(&cc.egui_ctx);

        // Set snappy/instant animations (default is ~83ms, we want instant)
        let mut style = (*cc.egui_ctx.style()).clone();
        style.animation_time = 0.0; // Instant - no animations
        cc.egui_ctx.set_style(style);

        // Configure egui options to reduce unnecessary repaints
        cc.egui_ctx.options_mut(|options| {
            // Disable repaint on widget change - this can cause constant repaints
            // when widgets move or IDs change
            options.repaint_on_widget_change = false;
        });

        // Check for crash recovery before creating AppState
        let recovery_result = load_session_state();
        let needs_recovery_dialog = recovery_result.is_crash_recovery 
            && recovery_result.session.as_ref().map(|s| s.has_unsaved_changes()).unwrap_or(false);

        let mut state = AppState::new();

        // If we have a valid session to restore (but no crash with unsaved changes),
        // restore it silently - but only if restore_session is enabled in settings
        if !needs_recovery_dialog && recovery_result.session.is_some() && state.settings.restore_session {
            if state.restore_from_session_result(&recovery_result) {
                info!("Session restored successfully");
            }
        }

        // Initialize theme manager with saved theme preference
        let mut theme_manager = ThemeManager::new(state.settings.theme);

        // Apply initial theme to egui context
        theme_manager.apply(&cc.egui_ctx);
        info!("Applied initial theme: {:?}", state.settings.theme);

        // Reload fonts with saved settings if different from defaults
        let custom_font = state.settings.font_family.custom_name().map(|s| s.to_string());
        if custom_font.is_some() || state.settings.cjk_font_preference != CjkFontPreference::Auto {
            fonts::reload_fonts(
                &cc.egui_ctx,
                custom_font.as_deref(),
                state.settings.cjk_font_preference,
            );
            info!("Loaded custom font settings: font={:?}, cjk_preference={:?}",
                  state.settings.font_family, state.settings.cjk_font_preference);
        }

        // Initialize outline panel with saved settings
        let outline_panel = OutlinePanel::new()
            .with_width(state.settings.outline_width)
            .with_side(state.settings.outline_side);

        // Initialize pipeline panel with saved settings
        let mut pipeline_panel = crate::ui::PipelinePanel::new();
        pipeline_panel.set_height(state.settings.pipeline_panel_height);
        pipeline_panel.set_enabled(state.settings.pipeline_enabled);
        pipeline_panel.configure(
            state.settings.pipeline_debounce_ms,
            state.settings.pipeline_max_output_bytes as usize,
            state.settings.pipeline_max_runtime_ms as u64,
        );
        pipeline_panel.set_recent_commands(state.settings.pipeline_recent_commands.clone());

        // Determine if we need to show recovery dialog
        // Clone the session for CSV delimiter restoration if needed
        let session_for_csv = if !needs_recovery_dialog {
            recovery_result.session.clone()
        } else {
            None
        };

        let (show_recovery_dialog, pending_recovery) = if needs_recovery_dialog {
            info!("Crash recovery detected with unsaved changes - will prompt user");
            (true, Some(recovery_result))
        } else {
            (false, None)
        };

        // Initialize snippet manager and sync with settings
        let mut snippet_manager = SnippetManager::new();
        snippet_manager.set_enabled(state.settings.snippets_enabled);

        // Load app logo texture for title bar display
        let app_logo_texture = load_app_logo_texture(&cc.egui_ctx);
        if app_logo_texture.is_some() {
            info!("Loaded app logo texture for title bar");
        }

        let mut app = Self {
            state,
            theme_manager,
            ribbon: Ribbon::new(),
            settings_panel: SettingsPanel::new(),
            about_panel: AboutPanel::new(),
            find_replace_panel: FindReplacePanel::new(),
            outline_panel,
            file_tree_panel: FileTreePanel::new(),
            quick_switcher: QuickSwitcher::new(),
            file_operation_dialog: None,
            search_panel: SearchPanel::new(),
            pipeline_panel,
            cached_outline: DocumentOutline::new(),
            cached_doc_stats: None,
            last_outline_content_hash: 0,
            pending_scroll_to_line: None,
            tree_viewer_states: HashMap::new(),
            csv_viewer_states: HashMap::new(),
            sync_scroll_states: HashMap::new(),
            should_exit: false,
            last_window_size: None,
            last_window_pos: None,
            start_time: std::time::Instant::now(),
            previous_view_mode: None,
            window_resize_state: WindowResizeState::new(),
            session_save_throttle: SessionSaveThrottle::default(),
            git_auto_refresh: GitAutoRefresh::new(),
            show_recovery_dialog,
            pending_recovery,
            pending_auto_save_recovery: None,
            snippet_manager,
            #[cfg(debug_assertions)]
            frame_count: 0,
            #[cfg(debug_assertions)]
            last_fps_log: std::time::Instant::now(),
            last_interaction_time: std::time::Instant::now(),
            last_window_title: String::new(),
            app_logo_texture,
        };

        // Restore CSV delimiter overrides from session if available
        if let Some(session) = session_for_csv {
            app.restore_csv_delimiters(&session);
        }

        app
    }

    /// Open files or directories from CLI arguments.
    ///
    /// This is called after construction to handle paths passed via command line.
    /// - Single directory: opens as workspace
    /// - Files: opens each as a new tab
    /// - Mixed: directory sets workspace, files open as tabs
    ///
    /// Non-existent paths are logged and skipped.
    pub fn open_initial_paths(&mut self, paths: Vec<std::path::PathBuf>) {
        use log::warn;

        if paths.is_empty() {
            return;
        }

        // Canonicalize, normalize, and validate paths
        // normalize_path removes Windows \\?\ prefix from canonicalized paths
        let mut valid_files: Vec<std::path::PathBuf> = Vec::new();
        let mut workspace_dir: Option<std::path::PathBuf> = None;

        for path in paths {
            // Try to canonicalize and normalize the path
            let canonical = match path.canonicalize() {
                Ok(p) => crate::path_utils::normalize_path(p),
                Err(e) => {
                    warn!("Skipping non-existent path '{}': {}", path.display(), e);
                    continue;
                }
            };

            if canonical.is_dir() {
                // Only take the first directory as workspace
                if workspace_dir.is_none() {
                    workspace_dir = Some(canonical);
                } else {
                    warn!(
                        "Multiple directories provided; ignoring '{}'",
                        path.display()
                    );
                }
            } else if canonical.is_file() {
                valid_files.push(canonical);
            } else {
                warn!("Path '{}' is neither a file nor directory", path.display());
            }
        }

        // Open workspace if provided
        if let Some(dir) = workspace_dir {
            info!("Opening workspace from CLI: {}", dir.display());
            match self.state.open_workspace(dir.clone()) {
                Ok(_) => {
                    // Immediately save session to persist the workspace path
                    self.force_session_save();
                }
                Err(e) => {
                    warn!("Failed to open workspace '{}': {}", dir.display(), e);
                }
            }
        }

        // Open files as tabs
        if !valid_files.is_empty() {
            // If we have CLI files, don't use the restored session tabs
            // Clear the default/restored empty tab if we're opening files
            if self.state.tab_count() == 1 {
                if let Some(tab) = self.state.active_tab() {
                    if tab.path.is_none() && tab.content.is_empty() {
                        // Remove the empty default tab since we're opening specific files
                        let tab_id = tab.id;
                        self.state.close_tab(0);
                        // No ctx available here during startup, skip egui cleanup
                        // (empty tab has no temp data anyway)
                        self.cleanup_tab_state(tab_id, None);
                    }
                }
            }

            let mut first_opened_tab_idx: Option<usize> = None;
            for file_path in valid_files.iter() {
                info!("Opening file from CLI: {}", file_path.display());
                match self.state.open_file(file_path.clone()) {
                    Ok(tab_idx) => {
                        if first_opened_tab_idx.is_none() {
                            first_opened_tab_idx = Some(tab_idx);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to open file '{}': {}", file_path.display(), e);
                    }
                }
            }
            // Focus on the first successfully opened file
            if let Some(tab_idx) = first_opened_tab_idx {
                self.state.set_active_tab(tab_idx);
            }
        }

        info!(
            "CLI initialization complete: {} files opened{}",
            valid_files.len(),
            if self.state.is_workspace_mode() {
                ", workspace mode active"
            } else {
                ""
            }
        );
    }

    /// Get elapsed time since app start in seconds.
    fn get_app_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Load CJK fonts on-demand for specific text content.
    ///
    /// This enables lazy CJK font loading - only the fonts needed for the detected
    /// scripts are loaded:
    /// - Korean text → loads only Korean font (~15-20MB)
    /// - Japanese text → loads only Japanese font (~15-20MB)
    /// - Chinese text → loads only Chinese font based on preference (~15-20MB)
    ///
    /// This is much more memory efficient than loading all CJK fonts at once.
    fn load_cjk_fonts_for_content(&self, ctx: &egui::Context, content: &str) {
        let custom_font = self
            .state
            .settings
            .font_family
            .custom_name()
            .map(|s| s.to_string());
        fonts::load_cjk_for_text(
            content,
            ctx,
            custom_font.as_deref(),
            self.state.settings.cjk_font_preference,
        );
    }

    /// Update window size in settings if changed.
    ///
    /// Returns `true` if the window state was updated.
    fn update_window_state(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;

        ctx.input(|i| {
            if let Some(rect) = i.viewport().outer_rect {
                let current_size = rect.size();
                let current_pos = rect.min;

                // Check if size changed
                let size_changed = self
                    .last_window_size
                    .map(|s| (s - current_size).length() > 1.0)
                    .unwrap_or(true);

                // Check if position changed
                let pos_changed = self
                    .last_window_pos
                    .map(|p| (p - current_pos).length() > 1.0)
                    .unwrap_or(true);

                if size_changed || pos_changed {
                    self.last_window_size = Some(current_size);
                    self.last_window_pos = Some(current_pos);
                    changed = true;
                }
            }
        });

        // Update settings with new window state
        if changed {
            if let (Some(size), Some(pos)) = (self.last_window_size, self.last_window_pos) {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

                self.state.settings.window_size = WindowSize {
                    width: size.x,
                    height: size.y,
                    x: Some(pos.x),
                    y: Some(pos.y),
                    maximized,
                };

                debug!(
                    "Window state updated: {}x{} at ({}, {}), maximized: {}",
                    size.x, size.y, pos.x, pos.y, maximized
                );

                // Mark settings dirty so window state gets persisted
                self.state.mark_settings_dirty();
            }
        }

        changed
    }

    /// Get the window title based on current state.
    ///
    /// Returns a title in the format: "Filename - Ferrite"
    /// or "Ferrite" if no file is open.
    fn window_title(&self) -> String {
        const APP_NAME: &str = "Ferrite";

        if let Some(tab) = self.state.active_tab() {
            let tab_title = tab.title();
            format!("{} - {}", tab_title, APP_NAME)
        } else {
            APP_NAME.to_string()
        }
    }

    /// Handle close request from the window.
    ///
    /// Returns `true` if the application should close.
    fn handle_close_request(&mut self) -> bool {
        if self.should_exit {
            return true;
        }

        if self.state.request_exit() {
            // No unsaved changes, safe to exit
            self.state.shutdown();
            true
        } else {
            // Confirmation dialog will be shown
            false
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Session Persistence (Crash Recovery)
    // ─────────────────────────────────────────────────────────────────────────

    /// Update session recovery state - called every frame.
    ///
    /// This checks if enough time has passed since the last session save
    /// and saves a crash recovery snapshot if needed.
    fn update_session_recovery(&mut self) {
        use crate::config::save_crash_recovery_state;

        // Mark session dirty if there are unsaved changes
        if self.state.has_unsaved_changes() {
            self.session_save_throttle.mark_dirty();
        }

        // Check if we should save
        if self.session_save_throttle.should_save() {
            let mut session_state = self.state.capture_session_state();
            session_state.clean_shutdown = false; // This is a crash recovery snapshot
            self.inject_csv_delimiters(&mut session_state);

            if save_crash_recovery_state(&session_state) {
                // Also save recovery content for tabs with unsaved changes
                self.state.save_recovery_content();
                self.session_save_throttle.record_save();
                debug!("Crash recovery snapshot saved");
            }
        }
    }

    /// Mark that session state has changed (for throttled saves).
    ///
    /// Call this when tabs are opened, closed, switched, or content changes.
    #[allow(dead_code)]
    fn mark_session_dirty(&mut self) {
        self.session_save_throttle.mark_dirty();
    }

    /// Inject CSV delimiter overrides into session state from csv_viewer_states.
    ///
    /// This transfers any manually-set delimiter preferences from the UI state
    /// to the session state for persistence.
    fn inject_csv_delimiters(&self, session_state: &mut crate::config::SessionState) {
        for tab in &mut session_state.tabs {
            if let Some(csv_state) = self.csv_viewer_states.get(&tab.tab_id) {
                tab.csv_delimiter = csv_state.delimiter_override();
            }
        }
    }

    /// Restore CSV delimiter overrides from session state into csv_viewer_states.
    ///
    /// This is called after session restoration to apply any saved delimiter
    /// preferences to the CSV viewer state.
    fn restore_csv_delimiters(&mut self, session: &crate::config::SessionState) {
        for session_tab in &session.tabs {
            if let Some(delimiter) = session_tab.csv_delimiter {
                // Find the corresponding tab in the current state
                // Note: tab IDs may have changed during restoration, so we match by path
                if let Some(tab) = self.state.tabs().iter().find(|t| t.path == session_tab.path) {
                    let csv_state = self.csv_viewer_states.entry(tab.id).or_default();
                    csv_state.set_delimiter(delimiter);
                    debug!(
                        "Restored CSV delimiter override for tab {}: {}",
                        tab.id,
                        delimiter_display_name(delimiter)
                    );
                }
            }
        }
    }

    /// Clean up viewer state HashMap entries and egui temporary data when a tab is closed.
    ///
    /// This prevents memory leaks by removing entries for closed tabs from:
    /// - `tree_viewer_states` (JSON/YAML/TOML tree view state)
    /// - `csv_viewer_states` (CSV/TSV viewer state with delimiter overrides)
    /// - `sync_scroll_states` (split-view sync scroll state)
    /// - egui memory temp data (rendered editor widget states like FormattedItemEditState,
    ///   CodeBlockData, MermaidBlockData, TableData, TableEditState, RenderedLinkState)
    ///
    /// # Parameters
    /// - `tab_id` - The unique ID of the closed tab
    /// - `ctx` - Optional egui Context for cleaning up egui memory. If None, only HashMap
    ///   cleanup is performed (useful during startup when context isn't available).
    ///
    /// Should be called after a tab is closed, using the tab's unique ID.
    fn cleanup_tab_state(&mut self, tab_id: usize, ctx: Option<&egui::Context>) {
        self.tree_viewer_states.remove(&tab_id);
        self.csv_viewer_states.remove(&tab_id);
        self.sync_scroll_states.remove(&tab_id);

        // Clean up egui temporary data for rendered editor widgets
        if let Some(ctx) = ctx {
            cleanup_rendered_editor_memory(ctx);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Snippet Expansion
    // ─────────────────────────────────────────────────────────────────────────

    /// Check for and apply snippet expansion after a space or tab is typed.
    ///
    /// This is called after the editor processes input. It checks if:
    /// 1. Snippets are enabled in settings
    /// 2. A space or tab was just typed (content ends with one of these)
    /// 3. There's a trigger word before the space/tab that matches a snippet
    ///
    /// If all conditions are met, the trigger word is replaced with the expansion,
    /// keeping the trailing space/tab.
    ///
    /// Returns `true` if a snippet was expanded.
    fn try_expand_snippet(&mut self, tab_index: usize) -> bool {
        // Check if snippets are enabled
        if !self.state.settings.snippets_enabled {
            return false;
        }

        // Get the tab
        let tab = match self.state.tab(tab_index) {
            Some(t) => t,
            None => return false,
        };

        // Get cursor position (in characters)
        let cursor_char = tab.cursors.primary().head;

        // Convert to byte position for snippet detection
        let cursor_byte = tab.content
            .char_indices()
            .nth(cursor_char)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(tab.content.len());

        // Check if the last character is a space or tab (trigger character)
        let content = &tab.content;
        if cursor_byte == 0 || cursor_byte > content.len() {
            return false;
        }

        // Get the character just before cursor
        let trigger_char = content[..cursor_byte].chars().last();
        let is_trigger_char = matches!(trigger_char, Some(' ') | Some('\t'));

        if !is_trigger_char {
            return false;
        }

        // Position before the space/tab
        let before_trigger_byte = cursor_byte - 1;

        // Look for snippet trigger
        if let Some(snippet_match) = find_trigger_at_cursor(content, before_trigger_byte, &self.snippet_manager) {
            // Apply the snippet expansion
            let (new_content, new_cursor_byte) = apply_snippet(content, &snippet_match);

            // Add back the trigger character (space/tab)
            let final_content = format!("{}{}", &new_content[..new_cursor_byte], &content[before_trigger_byte..]);
            let final_cursor_byte = new_cursor_byte + 1; // +1 for the space/tab

            // Convert new cursor position back to character position
            let final_cursor_char = final_content[..final_cursor_byte.min(final_content.len())].chars().count();

            // Update the tab content
            if let Some(tab) = self.state.tab_mut(tab_index) {
                // Record the edit for undo (use old content and old cursor)
                let old_content = tab.content.clone();
                let old_cursor = tab.cursors.primary().head;

                // Update content
                tab.content = final_content;

                // Update cursor position
                tab.cursors.set_single(Selection::cursor(final_cursor_char));
                tab.sync_cursor_from_primary();

                // Record the edit for undo/redo
                tab.record_edit(old_content, old_cursor);

                // Mark content version changed
                tab.increment_content_version();

                // Mark as modified
                tab.mark_content_edited();

                debug!(
                    "Snippet expanded: '{}' -> '{}' (cursor at {})",
                    snippet_match.trigger, snippet_match.expansion, final_cursor_char
                );
            }

            return true;
        }

        false
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Idle Detection for CPU Optimization
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if the application needs continuous repainting.
    ///
    /// Returns true if there's ongoing activity that requires immediate repaints,
    /// such as running pipelines, active toasts, animations, or scroll animations.
    /// When false, we can schedule delayed repaints to reduce CPU usage.
    fn needs_continuous_repaint(&self) -> bool {
        // Pipeline running requires continuous updates for output streaming
        if self.pipeline_panel.is_running() {
            return true;
        }

        // Toast message displayed needs checking for expiry
        if self.state.ui.toast_message.is_some() {
            return true;
        }

        // Recovery dialog showing requires user interaction tracking
        if self.show_recovery_dialog || self.pending_auto_save_recovery.is_some() {
            return true;
        }

        // Any modal dialog open
        if self.state.ui.show_confirm_dialog
            || self.state.ui.show_error_modal
            || self.state.ui.show_settings
            || self.state.ui.show_about
        {
            return true;
        }

        // Check if any sync scroll animation is running
        for state in self.sync_scroll_states.values() {
            if state.is_animating() {
                return true;
            }
        }

        false
    }

    /// Get the appropriate repaint interval based on idle state.
    ///
    /// Returns the duration to wait before the next repaint:
    /// - 100ms (light idle): Recent user interaction or activity
    /// - 500ms (deep idle): No activity for 2+ seconds
    ///
    /// This tiered approach significantly reduces CPU usage when the app
    /// is truly idle while maintaining responsiveness during use.
    fn get_idle_repaint_interval(&self) -> std::time::Duration {
        let idle_duration = self.last_interaction_time.elapsed();
        
        // Deep idle: no interaction for 2+ seconds
        // Use 500ms interval (~2 FPS) for periodic tasks like git refresh
        if idle_duration.as_secs() >= 2 {
            std::time::Duration::from_millis(500)
        } else {
            // Light idle: recent interaction
            // Use 100ms interval (~10 FPS) for responsive feel
            std::time::Duration::from_millis(100)
        }
    }

    /// Update interaction time when user activity is detected.
    ///
    /// This should be called when mouse clicks, key presses, or other
    /// user interactions occur to reset the idle timer.
    fn update_interaction_time(&mut self) {
        self.last_interaction_time = std::time::Instant::now();
    }

    /// Check if there was recent user input in this frame.
    ///
    /// Returns true if there were any keyboard or mouse events this frame.
    fn had_user_input(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            // Check for any key press
            if !i.keys_down.is_empty() {
                return true;
            }
            // Check for mouse button press
            if i.pointer.any_down() || i.pointer.any_pressed() {
                return true;
            }
            // Check for scroll
            if i.raw_scroll_delta != egui::Vec2::ZERO {
                return true;
            }
            // Check for any events (key, mouse, paste, etc.)
            !i.events.is_empty()
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Auto-Save Processing
    // ─────────────────────────────────────────────────────────────────────────

    /// Process auto-save for all tabs that need it.
    ///
    /// This is called every frame and checks each tab to see if auto-save
    /// should trigger based on idle time. Uses temp files to avoid
    /// overwriting the main file prematurely.
    fn process_auto_saves(&mut self) {
        use crate::config::save_auto_save_content;

        let delay_ms = self.state.settings.auto_save_delay_ms;
        let tab_count = self.state.tab_count();

        // Collect tabs that need auto-save (indices and info)
        let mut tabs_to_save: Vec<(usize, usize, Option<std::path::PathBuf>, String)> = Vec::new();
        
        for i in 0..tab_count {
            if let Some(tab) = self.state.tab(i) {
                if tab.should_auto_save(delay_ms) {
                    tabs_to_save.push((i, tab.id, tab.path.clone(), tab.content.clone()));
                }
            }
        }

        // Process auto-saves
        for (index, tab_id, path, content) in tabs_to_save {
            // Save to temp file
            if save_auto_save_content(tab_id, path.as_ref(), &content) {
                // Mark as auto-saved to prevent repeated saves
                if let Some(tab) = self.state.tab_mut(index) {
                    if tab.id == tab_id {
                        tab.mark_auto_saved();
                        debug!("Auto-saved tab {} to temp file", tab_id);
                    }
                }
            }
        }
    }

    /// Delete auto-save temp file for a tab after manual save.
    ///
    /// Called when user manually saves a file to clean up the temp backup.
    fn cleanup_auto_save_for_tab(&mut self, tab_id: usize) {
        use crate::config::delete_auto_save;

        // Find the tab by ID to get its path
        let tab_count = self.state.tab_count();
        for i in 0..tab_count {
            if let Some(tab) = self.state.tab(i) {
                if tab.id == tab_id {
                    delete_auto_save(tab_id, tab.path.as_ref());
                    debug!("Cleaned up auto-save temp file for tab {}", tab_id);
                    break;
                }
            }
        }
    }

    /// Check for auto-save recovery for a newly opened file.
    ///
    /// If an auto-save temp file exists that is newer than the file on disk,
    /// prompts the user to restore from the auto-save or discard it.
    ///
    /// This is called after opening a file to check if there's a recovery available.
    fn check_auto_save_recovery(&mut self, tab_index: usize) {
        use crate::config::check_auto_save_recovery;

        let Some(tab) = self.state.tab(tab_index) else {
            return;
        };

        let tab_id = tab.id;
        let path = tab.path.clone();

        // Check if there's a newer auto-save
        if let Some((metadata, recovered_content)) = check_auto_save_recovery(tab_id, path.as_ref()) {
            info!(
                "Found auto-save recovery for tab {} (saved at: {})",
                tab_id, metadata.saved_at
            );

            // Store recovery info for showing dialog
            self.pending_auto_save_recovery = Some(AutoSaveRecoveryInfo {
                tab_id,
                tab_index,
                path: path.clone(),
                recovered_content,
                saved_at: metadata.saved_at,
            });
        }
    }

    /// Show auto-save recovery dialog if needed.
    fn show_auto_save_recovery_dialog(&mut self, ctx: &egui::Context) {
        use crate::config::delete_auto_save;

        let Some(recovery_info) = self.pending_auto_save_recovery.take() else {
            return;
        };

        // Show a modal dialog
        let mut should_restore = false;
        let mut should_discard = false;

        egui::Window::new(format!("🔄 {}", t!("recovery.auto_save.title")))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(t!("recovery.auto_save.backup_found").to_string());
                ui.add_space(4.0);

                if let Some(path) = &recovery_info.path {
                    ui.label(format!("File: {}", path.display()));
                } else {
                    ui.label(t!("recovery.untitled").to_string());
                }

                // Format timestamp
                let saved_time = std::time::UNIX_EPOCH
                    + std::time::Duration::from_secs(recovery_info.saved_at);
                if let Ok(elapsed) = std::time::SystemTime::now().duration_since(saved_time) {
                    let secs = elapsed.as_secs();
                    let time_str = if secs < 60 {
                        t!("time.seconds_ago", count = secs).to_string()
                    } else if secs < 3600 {
                        t!("time.minutes_ago", count = secs / 60).to_string()
                    } else if secs < 86400 {
                        t!("time.hours_ago", count = secs / 3600).to_string()
                    } else {
                        t!("time.days_ago", count = secs / 86400).to_string()
                    };
                    ui.label(t!("recovery.auto_save.time_label", time = time_str).to_string());
                }

                ui.add_space(12.0);
                ui.label(t!("recovery.auto_save.restore_question").to_string());
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button(format!("✅ {}", t!("recovery.auto_save.restore"))).clicked() {
                        should_restore = true;
                    }
                    if ui.button(format!("🗑 {}", t!("recovery.auto_save.discard"))).clicked() {
                        should_discard = true;
                    }
                });
            });

        if should_restore {
            // Restore the auto-saved content
            if let Some(tab) = self.state.tab_mut(recovery_info.tab_index) {
                if tab.id == recovery_info.tab_id {
                    tab.set_content(recovery_info.recovered_content);
                    let time = self.get_app_time();
                    self.state.show_toast(t!("notification.restored_auto_save").to_string(), time, 3.0);
                    info!("Restored auto-save content for tab {}", recovery_info.tab_id);
                }
            }
            // Delete the auto-save file after restore
            delete_auto_save(recovery_info.tab_id, recovery_info.path.as_ref());
        } else if should_discard {
            // Delete the auto-save file
            delete_auto_save(recovery_info.tab_id, recovery_info.path.as_ref());
            let time = self.get_app_time();
            self.state.show_toast(t!("notification.auto_save_discarded").to_string(), time, 2.0);
            info!("Discarded auto-save for tab {}", recovery_info.tab_id);
        } else {
            // Dialog still open, put recovery info back
            self.pending_auto_save_recovery = Some(recovery_info);
        }
    }

    /// Show the crash recovery dialog if needed.
    ///
    /// This renders a modal dialog asking the user whether to restore
    /// the previous session with unsaved changes.
    fn show_recovery_dialog_if_needed(&mut self, ctx: &egui::Context) {
        use crate::config::clear_all_recovery_data;

        if !self.show_recovery_dialog {
            return;
        }

        let Some(recovery_result) = &self.pending_recovery else {
            self.show_recovery_dialog = false;
            return;
        };

        let num_unsaved = recovery_result
            .session
            .as_ref()
            .map(|s| s.tabs_with_unsaved_content().len())
            .unwrap_or(0);

        let mut restore = false;
        let mut discard = false;

        egui::Window::new(format!("🔄 {}?", t!("recovery.session.title")))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    ui.label(t!("recovery.session.crash_detected").to_string());
                    ui.add_space(4.0);

                    if num_unsaved > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 0),
                            t!("recovery.session.tabs_unsaved", count = num_unsaved).to_string(),
                        );
                    }

                    ui.add_space(8.0);

                    ui.label(t!("recovery.session.restore_question").to_string());

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("✓ {}", t!("recovery.session.restore")))
                            .on_hover_text(t!("recovery.session.restore_tooltip").to_string())
                            .clicked()
                        {
                            restore = true;
                        }

                        ui.add_space(8.0);

                        if ui
                            .button(format!("✗ {}", t!("recovery.session.start_fresh")))
                            .on_hover_text(t!("recovery.session.start_fresh_tooltip").to_string())
                            .clicked()
                        {
                            discard = true;
                        }
                    });
                });
            });

        if restore {
            if let Some(result) = self.pending_recovery.take() {
                // Save session reference before restoration for CSV delimiter restoration
                let session = result.session.clone();
                if self.state.restore_from_session_result(&result) {
                    info!("Session restored from crash recovery");
                    let current_time = self.get_app_time();
                    self.state.show_toast(t!("notification.session_restored").to_string(), current_time, 3.0);
                    // Restore CSV delimiter overrides
                    if let Some(session) = session {
                        self.restore_csv_delimiters(&session);
                    }
                }
            }
            // Clear recovery data after successful restore
            clear_all_recovery_data();
            self.show_recovery_dialog = false;
        } else if discard {
            info!("User discarded crash recovery");
            clear_all_recovery_data();
            self.pending_recovery = None;
            self.show_recovery_dialog = false;
        }
    }

    /// Render the main UI content.
    /// Returns a deferred format command if one was requested from the ribbon.
    fn render_ui(&mut self, ctx: &egui::Context) -> Option<MarkdownFormatCommand> {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_dark = ctx.style().visuals.dark_mode;
        let zen_mode = self.state.is_zen_mode();

        // Title bar colors based on theme
        let title_bar_color = if is_dark {
            egui::Color32::from_rgb(32, 32, 32)
        } else {
            egui::Color32::from_rgb(240, 240, 240)
        };

        let button_hover_color = if is_dark {
            egui::Color32::from_rgb(60, 60, 60)
        } else {
            egui::Color32::from_rgb(210, 210, 210)
        };

        let close_hover_color = egui::Color32::from_rgb(232, 17, 35);

        let text_color = if is_dark {
            egui::Color32::from_rgb(220, 220, 220)
        } else {
            egui::Color32::from_rgb(30, 30, 30)
        };

        // Title bar panel (custom window controls)
        egui::TopBottomPanel::top("title_bar")
            .frame(
                egui::Frame::none()
                    .fill(title_bar_color)
                    .stroke(egui::Stroke::NONE)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                // Remove spacing between elements
                ui.spacing_mut().item_spacing.y = 0.0;

                // Add top padding for title bar
                ui.add_space(5.0);

                // Get state needed for title bar controls
                let has_editor = self.state.active_tab().is_some();
                let auto_save_enabled = self.state.active_tab()
                    .map(|t| t.auto_save_enabled)
                    .unwrap_or(false);
                let current_view_mode = self.state.active_tab()
                    .map(|t| t.view_mode)
                    .unwrap_or(ViewMode::Raw);
                let current_file_type = self.state.active_tab()
                    .map(|t| t.file_type())
                    .unwrap_or(FileType::Unknown);
                let zen_mode_active = self.state.is_zen_mode();

                // Track title bar actions
                let mut title_bar_toggle_auto_save = false;
                let mut title_bar_toggle_zen = false;
                let mut title_bar_open_settings = false;
                let mut title_bar_view_action: Option<ViewSegmentAction> = None;

                // Title bar row - set consistent height and center alignment
                let title_bar_height = 28.0;
                ui.set_height(title_bar_height);
                
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // App icon/logo - display texture if available, fallback to emoji
                    if let Some(texture) = &self.app_logo_texture {
                        let logo_size = 18.0; // Match title bar height nicely
                        ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(logo_size, logo_size)));
                    } else {
                        ui.label(egui::RichText::new("📝").size(14.0));
                    }

                    ui.add_space(4.0); // Reduced spacing between icon and title

                    // Window title (dynamically generated) - use consistent sizing
                    // Offset text slightly upward to better align with icon center
                    let title = self.window_title();
                    ui.add(egui::Label::new(egui::RichText::new(title).size(12.0).color(text_color)).selectable(false));

                    // Auto-save indicator (after filename) - only show if there's an active editor
                    if has_editor {
                        ui.add_space(8.0);
                        if TitleBarButton::show_auto_save(ui, auto_save_enabled, is_dark).clicked() {
                            title_bar_toggle_auto_save = true;
                        }
                    }

                    // Fill remaining space with draggable area, but EXCLUDE the button area
                    // on the right side to prevent drag response from consuming clicks
                    // intended for window control buttons. This fixes Linux hit-testing issues.
                    //
                    // Button area width calculation (right-to-left):
                    // - 4.0 spacing + Close(46) + Max(46) + Min(46) + Fullscreen(46) + 8.0 spacing = 196px
                    // - Settings(28) + 4.0 + Zen(28) + 4.0 = 64px
                    // - ViewModeSegment (3 × 26px) = 78px (or 2 × 26px = 52px for 2-mode)
                    // Total ~338px + extra margin for safety = 400px
                    const WINDOW_BUTTON_AREA_WIDTH: f32 = 400.0;
                    
                    let available = ui.available_rect_before_wrap();
                    let drag_width = (available.width() - WINDOW_BUTTON_AREA_WIDTH).max(0.0);
                    let drag_rect = egui::Rect::from_min_size(
                        available.min,
                        egui::vec2(drag_width, available.height()),
                    );
                    
                    // IMPORTANT: We use Sense::hover() and handle drag detection manually via
                    // raw input state. This is necessary because:
                    //
                    // 1. When StartDrag is sent, the window manager takes over the drag operation
                    // 2. egui doesn't receive the mouse release event (WM handles it)
                    // 3. egui's widget interaction state gets confused, thinking the widget
                    //    is still being interacted with
                    // 4. On the next click, drag_started() doesn't fire because egui thinks
                    //    we're continuing an existing interaction
                    //
                    // By using raw input state (primary_pressed), we bypass egui's widget-level
                    // tracking entirely and get reliable drag detection every time.
                    let drag_response = ui.allocate_rect(drag_rect, egui::Sense::hover());
                    
                    // Get raw pointer state - this is always accurate regardless of widget state
                    let (primary_pressed, double_clicked, pointer_pos) = ctx.input(|i| (
                        i.pointer.primary_pressed(),
                        i.pointer.button_double_clicked(egui::PointerButton::Primary),
                        i.pointer.interact_pos(),
                    ));
                    
                    // Check if pointer is in the drag area
                    let pointer_in_drag_area = pointer_pos
                        .map(|pos| drag_rect.contains(pos))
                        .unwrap_or(false);

                    // Handle double-click to maximize/restore
                    if double_clicked && pointer_in_drag_area {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // Handle drag to move window (but not if we're in a resize zone)
                    //
                    // We use primary_pressed() which is only true on the FRAME the button
                    // is pressed down. This ensures StartDrag is sent exactly once per click,
                    // preventing the "mouse stuck" bug on Linux.
                    let is_in_resize = self.window_resize_state.current_direction().is_some()
                        || self.window_resize_state.is_resizing();
                    
                    if primary_pressed && pointer_in_drag_area && !is_in_resize {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    
                    // Still use the response for hover effects if needed
                    let _ = drag_response;

                    // Window control buttons (right-to-left)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);

                        // Close button (×)
                        let close_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("×").size(16.0).color(text_color),
                            )
                            .frame(false)
                            .min_size(egui::vec2(46.0, 28.0)),
                        );
                        if close_btn.hovered() {
                            ui.painter()
                                .rect_filled(close_btn.rect, 0.0, close_hover_color);
                            ui.painter().text(
                                close_btn.rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                egui::FontId::proportional(16.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if close_btn.clicked() && self.state.request_exit() {
                            self.should_exit = true;
                        }
                        close_btn.on_hover_text(t!("a11y.close_button").to_string());

                        // Maximize/Restore button
                        let max_icon = if is_maximized { "❐" } else { "□" };
                        let max_tooltip = if is_maximized { "Restore" } else { "Maximize" };
                        let max_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(max_icon).size(14.0).color(text_color),
                            )
                            .frame(false)
                            .min_size(egui::vec2(46.0, 28.0)),
                        );
                        if max_btn.hovered() {
                            ui.painter()
                                .rect_filled(max_btn.rect, 0.0, button_hover_color);
                        }
                        if max_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }
                        max_btn.on_hover_text(max_tooltip);

                        // Minimize button - draw a line
                        let min_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(egui::vec2(46.0, 28.0)),
                        );
                        if min_btn.hovered() {
                            ui.painter()
                                .rect_filled(min_btn.rect, 0.0, button_hover_color);
                        }
                        let center = min_btn.rect.center();
                        ui.painter().line_segment(
                            [
                                egui::pos2(center.x - 5.0, center.y),
                                egui::pos2(center.x + 5.0, center.y),
                            ],
                            egui::Stroke::new(1.5, text_color),
                        );
                        if min_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        min_btn.on_hover_text(t!("a11y.minimize_button").to_string());

                        // Fullscreen button - draw expand arrows icon
                        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        let fullscreen_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(egui::vec2(46.0, 28.0)),
                        );
                        if fullscreen_btn.hovered() || is_fullscreen {
                            ui.painter()
                                .rect_filled(fullscreen_btn.rect, 0.0, button_hover_color);
                        }
                        // Draw fullscreen icon (4 arrows pointing outward, or inward when in fullscreen)
                        let fs_center = fullscreen_btn.rect.center();
                        let arrow_len = 3.5;
                        let arrow_offset = 4.0;
                        let stroke = egui::Stroke::new(1.5, text_color);
                        if is_fullscreen {
                            // Inward arrows (exit fullscreen) - draw 4 arrows pointing to center
                            ui.painter().line_segment([egui::pos2(fs_center.x - arrow_offset, fs_center.y - arrow_offset), egui::pos2(fs_center.x - arrow_offset + arrow_len, fs_center.y - arrow_offset + arrow_len)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x + arrow_offset, fs_center.y - arrow_offset), egui::pos2(fs_center.x + arrow_offset - arrow_len, fs_center.y - arrow_offset + arrow_len)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x - arrow_offset, fs_center.y + arrow_offset), egui::pos2(fs_center.x - arrow_offset + arrow_len, fs_center.y + arrow_offset - arrow_len)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x + arrow_offset, fs_center.y + arrow_offset), egui::pos2(fs_center.x + arrow_offset - arrow_len, fs_center.y + arrow_offset - arrow_len)], stroke);
                        } else {
                            // Outward arrows (enter fullscreen) - draw 4 arrows pointing away from center
                            ui.painter().line_segment([egui::pos2(fs_center.x - arrow_offset + arrow_len, fs_center.y - arrow_offset + arrow_len), egui::pos2(fs_center.x - arrow_offset, fs_center.y - arrow_offset)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x + arrow_offset - arrow_len, fs_center.y - arrow_offset + arrow_len), egui::pos2(fs_center.x + arrow_offset, fs_center.y - arrow_offset)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x - arrow_offset + arrow_len, fs_center.y + arrow_offset - arrow_len), egui::pos2(fs_center.x - arrow_offset, fs_center.y + arrow_offset)], stroke);
                            ui.painter().line_segment([egui::pos2(fs_center.x + arrow_offset - arrow_len, fs_center.y + arrow_offset - arrow_len), egui::pos2(fs_center.x + arrow_offset, fs_center.y + arrow_offset)], stroke);
                        }
                        if fullscreen_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
                        }
                        let fs_tooltip = if is_fullscreen { t!("tooltip.fullscreen_exit") } else { t!("tooltip.fullscreen_enter") };
                        fullscreen_btn.on_hover_text(fs_tooltip.to_string());

                        ui.add_space(8.0);

                        // ═══════════════════════════════════════════════════════════════
                        // Title Bar Controls (before window buttons, right-to-left)
                        // Settings → Zen Mode → View Mode Segment
                        // ═══════════════════════════════════════════════════════════════

                        // Settings button
                        if TitleBarButton::show(ui, "⚙", &t!("tooltip.settings").to_string(), false, is_dark).clicked() {
                            title_bar_open_settings = true;
                        }

                        ui.add_space(4.0);

                        // Zen Mode toggle - use simple "Z" icon for cross-platform compatibility
                        let zen_icon = if zen_mode_active { "Z" } else { "Z" };
                        let zen_tooltip = if zen_mode_active {
                            t!("zen.exit")
                        } else {
                            t!("zen.enter")
                        };
                        if TitleBarButton::show(ui, zen_icon, &format!("{} (F11)", zen_tooltip), zen_mode_active, is_dark).clicked() {
                            title_bar_toggle_zen = true;
                        }

                        ui.add_space(4.0);

                        // View Mode segmented control (only if there's an active editor with renderable content)
                        if has_editor && (current_file_type.is_markdown() || current_file_type.is_structured() || current_file_type.is_tabular()) {
                            // Show the segmented pill control for view mode selection
                            let segment = ViewModeSegment::new();
                            
                            // Use 3-mode segment for markdown/tabular, 2-mode for structured files
                            if current_file_type.is_markdown() || current_file_type.is_tabular() {
                                if let Some(action) = segment.show(ui, current_view_mode, current_file_type, is_dark) {
                                    title_bar_view_action = Some(action);
                                }
                            } else {
                                // Structured files (JSON/YAML/TOML): only Raw <-> Rendered
                                if let Some(action) = segment.show_two_mode(ui, current_view_mode, is_dark) {
                                    title_bar_view_action = Some(action);
                                }
                            }
                        }
                    });
                });

                ui.add_space(2.0);

                // Handle title bar actions (deferred to avoid borrow conflicts)
                if title_bar_toggle_auto_save {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.auto_save_enabled = !tab.auto_save_enabled;
                        debug!("Title bar: Toggle auto-save -> {}", tab.auto_save_enabled);
                    }
                }
                if title_bar_toggle_zen {
                    self.state.toggle_zen_mode();
                    debug!("Title bar: Toggle Zen Mode");
                }
                if title_bar_open_settings {
                    self.state.ui.show_settings = true;
                    debug!("Title bar: Open Settings");
                }
                if let Some(view_action) = title_bar_view_action {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let new_mode = match view_action {
                            ViewSegmentAction::SetRaw => ViewMode::Raw,
                            ViewSegmentAction::SetSplit => ViewMode::Split,
                            ViewSegmentAction::SetRendered => ViewMode::Rendered,
                        };
                        tab.view_mode = new_mode;
                        debug!("Title bar: Set view mode to {:?}", new_mode);
                    }
                }
            });

        // Ribbon panel (below title bar) - hidden in Zen Mode
        let ribbon_action = if !zen_mode {
            // Get state needed for ribbon
            let theme = self.state.settings.theme;
            let view_mode = self
                .state
                .active_tab()
                .map(|t| t.view_mode)
                .unwrap_or(ViewMode::Raw);
            let show_line_numbers = self.state.settings.show_line_numbers;
            let can_undo = self
                .state
                .active_tab()
                .map(|t| t.can_undo())
                .unwrap_or(false);
            let can_redo = self
                .state
                .active_tab()
                .map(|t| t.can_redo())
                .unwrap_or(false);
            let can_save = self
                .state
                .active_tab()
                .map(|t| t.path.is_some() && t.is_modified())
                .unwrap_or(false);

            let theme_colors = ThemeColors::from_theme(theme, &ctx.style().visuals);

            let ribbon_bg = if is_dark {
                egui::Color32::from_rgb(40, 40, 40)
            } else {
                egui::Color32::from_rgb(248, 248, 248)
            };

            let mut action = None;
            egui::TopBottomPanel::top("ribbon")
                .frame(
                    egui::Frame::none()
                        .fill(ribbon_bg)
                        .stroke(egui::Stroke::NONE)
                        .inner_margin(egui::Margin::symmetric(4.0, 4.0)),
                )
                .show_separator_line(false)
                .show(ctx, |ui| {
                    // Get formatting state for active editor
                    let formatting_state = self.get_formatting_state();

                    // Get file type for adaptive toolbar
                    let file_type = self
                        .state
                        .active_tab()
                        .map(|t| t.file_type())
                        .unwrap_or_default();

                    // Get auto-save state for current tab
                    let auto_save_enabled = self
                        .state
                        .active_tab()
                        .map(|t| t.auto_save_enabled)
                        .unwrap_or(false);

                    action = self.ribbon.show(
                        ui,
                        &theme_colors,
                        view_mode,
                        show_line_numbers,
                        can_undo,
                        can_redo,
                        can_save,
                        self.state.active_tab().is_some(),
                        formatting_state.as_ref(),
                        self.state.settings.outline_enabled,
                        self.state.settings.sync_scroll_enabled,
                        self.state.is_workspace_mode(),
                        file_type,
                        self.state.is_zen_mode(),
                        auto_save_enabled,
                        self.state.settings.pipeline_enabled,
                    );
                });
            action
        } else {
            None
        };

        // Handle ribbon actions - defer format actions until after editor renders
        let deferred_format_action = if let Some(action) = ribbon_action {
            match action {
                RibbonAction::Format(cmd) => Some(cmd), // Defer format actions
                other => {
                    self.handle_ribbon_action(other, ctx);
                    None
                }
            }
        } else {
            None
        };

        // Track deferred actions for status bar (to avoid borrow conflicts)
        let mut toggle_rainbow_columns = false;
        let mut pending_encoding_change: Option<&'static str> = None;

        // Bottom panel for status bar - hidden in Zen Mode
        if !zen_mode {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left side: File path (clickable for recent files popup)
                let path_display = if let Some(tab) = self.state.active_tab() {
                    tab.path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| t!("status.untitled").to_string())
                } else {
                    t!("status.no_file").to_string()
                };

                // Make the file path a clickable button that opens the recent items popup
                let has_recent_files = !self.state.settings.recent_files.is_empty();
                let has_recent_folders = !self.state.settings.recent_workspaces.is_empty();
                let has_recent_items = has_recent_files || has_recent_folders;
                let popup_id = ui.make_persistent_id("recent_items_popup");

                let button_response = ui.add(
                    egui::Button::new(&path_display)
                        .frame(false)
                        .sense(if has_recent_items {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        })
                );

                if has_recent_items {
                    button_response.clone().on_hover_text(t!("tooltip.recent_items").to_string());
                }

                // Toggle popup on click
                let just_opened = if button_response.clicked() && has_recent_items {
                    self.state.ui.show_recent_files_popup = !self.state.ui.show_recent_files_popup;
                    self.state.ui.show_recent_files_popup // true if we just opened it
                } else {
                    false
                };

                // Show recent items popup (files and folders)
                if self.state.ui.show_recent_files_popup && has_recent_items {
                    // Collect recent items before creating the popup to avoid borrow issues
                    let recent_files: Vec<_> = self.state.settings.recent_files
                        .iter()
                        .take(10)
                        .cloned()
                        .collect();
                    let recent_folders: Vec<_> = self.state.settings.recent_workspaces
                        .iter()
                        .take(5)
                        .cloned()
                        .collect();

                    // Position popup above the button so it doesn't cover the filename
                    // Calculate position: start from left edge, place above with some margin
                    let popup_pos = egui::pos2(
                        button_response.rect.left(),
                        button_response.rect.top() - 8.0, // Small gap above button
                    );
                    let popup_response = egui::Area::new(popup_id)
                        .order(egui::Order::Foreground)
                        .fixed_pos(popup_pos)
                        .pivot(egui::Align2::LEFT_BOTTOM) // Anchor at bottom-left so it grows upward
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                // Use two-column layout if we have both files and folders
                                let show_both_columns = !recent_files.is_empty() && !recent_folders.is_empty();
                                
                                // Action to perform after popup closes
                                // (PathBuf, is_file, with_focus)
                                let mut action: Option<(std::path::PathBuf, bool, bool)> = None;
                                
                                // Theme-aware colors
                                let name_color = if is_dark {
                                    egui::Color32::from_rgb(220, 220, 220)
                                } else {
                                    egui::Color32::from_rgb(30, 30, 30)
                                };
                                let secondary_color = if is_dark {
                                    egui::Color32::from_rgb(160, 160, 160)
                                } else {
                                    egui::Color32::from_rgb(80, 80, 80)
                                };
                                let folder_icon_color = if is_dark {
                                    egui::Color32::from_rgb(255, 200, 100) // Yellow/gold for folders
                                } else {
                                    egui::Color32::from_rgb(180, 140, 50)
                                };

                                if show_both_columns {
                                    // Stacked vertical layout: Files section, then Folders section
                                    ui.set_min_width(300.0);
                                    
                                    // Recent Files section
                                    ui.label(egui::RichText::new(t!("menu.file.recent").to_string()).strong());
                                    ui.separator();
                                    
                                    for path in &recent_files {
                                        let file_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(file_name).strong().color(name_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(280.0, 0.0))
                                        );
                                        item_response.clone().on_hover_text(format!(
                                            "{}\n\nClick: Open\nShift+Click: Open in background",
                                            path.display()
                                        ));

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(2.0);

                                        if item_response.clicked() {
                                            let shift_held = ui.input(|i| i.modifiers.shift);
                                            action = Some((path.clone(), true, !shift_held));
                                        }
                                    }

                                    // Separator between sections
                                    ui.add_space(8.0);
                                    
                                    // Recent Folders section
                                    ui.label(egui::RichText::new(t!("workspace.recent_folders").to_string()).strong());
                                    ui.separator();
                                    
                                    for path in &recent_folders {
                                        let folder_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(format!("📁 {}", folder_name))
                                                    .strong()
                                                    .color(folder_icon_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(280.0, 0.0))
                                        );
                                        item_response.clone().on_hover_text(format!(
                                            "{}\n\nClick: Open as workspace",
                                            path.display()
                                        ));

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(2.0);

                                        if item_response.clicked() {
                                            action = Some((path.clone(), false, true));
                                        }
                                    }
                                } else if !recent_files.is_empty() {
                                    // Only files
                                    ui.set_min_width(300.0);
                                    ui.label(egui::RichText::new("📄 Recent Files").strong());
                                    ui.separator();

                                    for path in &recent_files {
                                        let file_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(file_name).strong().color(name_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(ui.available_width(), 0.0))
                                        );
                                        item_response.clone().on_hover_text(format!(
                                            "{}\n\nClick: Open\nShift+Click: Open in background",
                                            path.display()
                                        ));

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(4.0);

                                        if item_response.clicked() {
                                            let shift_held = ui.input(|i| i.modifiers.shift);
                                            action = Some((path.clone(), true, !shift_held));
                                        }
                                    }
                                } else {
                                    // Only folders
                                    ui.set_min_width(300.0);
                                    ui.label(egui::RichText::new("📁 Recent Folders").strong());
                                    ui.separator();

                                    for path in &recent_folders {
                                        let folder_name = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown");
                                        let parent_dir = path.parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        let item_response = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(format!("📁 {}", folder_name))
                                                    .strong()
                                                    .color(folder_icon_color)
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(ui.available_width(), 0.0))
                                        );
                                        item_response.clone().on_hover_text(format!(
                                            "{}\n\nClick: Open as workspace",
                                            path.display()
                                        ));

                                        if !parent_dir.is_empty() {
                                            ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                        }
                                        ui.add_space(4.0);

                                        if item_response.clicked() {
                                            action = Some((path.clone(), false, true));
                                        }
                                    }
                                }

                                action
                            })
                        });

                    // Handle action after UI is done
                    if let Some((path, is_file, focus)) = popup_response.inner.inner {
                        if is_file {
                            // Only close popup on normal click (focus=true)
                            // Keep open on shift+click to allow opening multiple files
                            if focus {
                                self.state.ui.show_recent_files_popup = false;
                            }
                            match self.state.open_file_with_focus(path.clone(), focus) {
                                Ok(_) => {
                                    if focus {
                                        debug!("Opened recent file with focus: {}", path.display());
                                    } else {
                                        let time = self.get_app_time();
                                        self.state.show_toast(
                                            format!("Opened in background: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("file")),
                                            time,
                                            2.0
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to open recent file: {}", e);
                                    self.state.show_error(format!("Failed to open file:\n{}", e));
                                }
                            }
                        } else {
                            // Open folder as workspace
                            self.state.ui.show_recent_files_popup = false;
                            match self.state.open_workspace(path.clone()) {
                                Ok(_) => {
                                    let time = self.get_app_time();
                                    let folder_name = path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("folder");
                                    self.state.show_toast(
                                        format!("Opened workspace: {}", folder_name),
                                        time,
                                        2.5
                                    );
                                    debug!("Opened recent workspace: {}", path.display());
                                }
                                Err(e) => {
                                    warn!("Failed to open recent workspace: {}", e);
                                    self.state.show_error(format!("Failed to open workspace:\n{}", e));
                                }
                            }
                        }
                    }

                    // Close popup when clicking outside (but not on the same frame we opened it)
                    if popup_response.response.clicked_elsewhere() && !just_opened {
                        self.state.ui.show_recent_files_popup = false;
                    }
                }

                // Center: Toast message (temporary notifications)
                if let Some(toast) = &self.state.ui.toast_message {
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.label(egui::RichText::new(toast).italics());
                    });
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Help button (rightmost in right-to-left layout)
                    if ui
                        .button("?")
                        .on_hover_text(t!("tooltip.about_help").to_string())
                        .clicked()
                    {
                        self.state.toggle_about();
                    }

                    // Git branch display (if in a git repository)
                    if let Some(branch) = self.state.git_service.current_branch() {
                        ui.separator();
                        
                        // Branch icon and name with theme-appropriate color
                        let branch_color = if is_dark {
                            egui::Color32::from_rgb(130, 180, 240) // Light blue for dark mode
                        } else {
                            egui::Color32::from_rgb(50, 100, 170) // Dark blue for light mode
                        };
                        
                        ui.label(
                            egui::RichText::new(format!("⎇ {}", branch))
                                .color(branch_color)
                                .size(12.0)
                        ).on_hover_text(t!("tooltip.git_branch").to_string());
                    }

                    if let Some(tab) = self.state.active_tab() {
                        ui.separator();

                        // Cursor position
                        let (line, col) = tab.cursor_position;
                        ui.label(format!("Ln {}, Col {}", line + 1, col + 1));

                        // Delimiter picker for CSV/TSV files in rendered or split mode
                        if tab.view_mode == ViewMode::Rendered || tab.view_mode == ViewMode::Split {
                            if let Some(tabular_type) = tab.path.as_ref().and_then(|p| get_tabular_file_type(p)) {
                                ui.separator();
                                
                                let tab_id = tab.id;
                                
                                // Capture all state values upfront to avoid borrow conflicts with popups
                                let (current_delimiter, is_overridden, has_headers, header_overridden) = {
                                    let csv_state = self.csv_viewer_states.entry(tab_id).or_default();
                                    (
                                        csv_state.effective_delimiter().unwrap_or(tabular_type.delimiter()),
                                        csv_state.has_delimiter_override(),
                                        csv_state.has_headers(),
                                        csv_state.has_header_override(),
                                    )
                                };
                                
                                // Delimiter indicator with dropdown
                                let delimiter_label = format!(
                                    "Delim: {}{}",
                                    delimiter_symbol(current_delimiter),
                                    if is_overridden { " ✓" } else { "" }
                                );
                                
                                let popup_id = ui.make_persistent_id("delimiter_picker_popup");
                                let button_response = ui.add(
                                    egui::Button::new(&delimiter_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );
                                
                                button_response.clone().on_hover_text(format!(
                                    "Delimiter: {}\n{}Click to change",
                                    delimiter_display_name(current_delimiter),
                                    if is_overridden { "Manually set. " } else { "Auto-detected. " }
                                ));
                                
                                if button_response.clicked() {
                                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                }
                                
                                // Delimiter picker popup
                                egui::popup_below_widget(ui, popup_id, &button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                    ui.set_min_width(120.0);
                                    ui.label(egui::RichText::new(t!("csv.select_delimiter").to_string()).strong());
                                    ui.separator();
                                    
                                    // Auto-detect option
                                    let auto_selected = !is_overridden;
                                    if ui.selectable_label(auto_selected, t!("csv.delimiter_auto").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.clear_delimiter_override();
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    ui.separator();
                                    
                                    // Manual delimiter options
                                    for &delim in DELIMITERS {
                                        let selected = is_overridden && current_delimiter == delim;
                                        let label = format!("{} {}", delimiter_symbol(delim), delimiter_display_name(delim));
                                        if ui.selectable_label(selected, label).clicked() {
                                            if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                                state.set_delimiter(delim);
                                            }
                                            ui.memory_mut(|mem| mem.close_popup());
                                        }
                                    }
                                });
                                
                                ui.separator();
                                
                                // Header row toggle
                                let header_label = format!(
                                    "Headers: {}{}",
                                    if has_headers { "✓" } else { "✗" },
                                    if header_overridden { " ✓" } else { "" }
                                );
                                
                                let header_popup_id = ui.make_persistent_id("header_picker_popup");
                                let header_button_response = ui.add(
                                    egui::Button::new(&header_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );
                                
                                header_button_response.clone().on_hover_text(format!(
                                    "First row as headers: {}\n{}Click to change",
                                    if has_headers { "Yes" } else { "No" },
                                    if header_overridden { "Manually set. " } else { "Auto-detected. " }
                                ));
                                
                                if header_button_response.clicked() {
                                    ui.memory_mut(|mem| mem.toggle_popup(header_popup_id));
                                }
                                
                                // Header picker popup
                                egui::popup_below_widget(ui, header_popup_id, &header_button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                    ui.set_min_width(120.0);
                                    ui.label(egui::RichText::new(t!("csv.header_row").to_string()).strong());
                                    ui.separator();
                                    
                                    // Auto-detect option
                                    let auto_selected = !header_overridden;
                                    if ui.selectable_label(auto_selected, t!("csv.delimiter_auto").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.clear_header_override();
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    ui.separator();
                                    
                                    // Manual options
                                    if ui.selectable_label(header_overridden && has_headers, t!("csv.has_headers_yes").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.set_header_override(true);
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    
                                    if ui.selectable_label(header_overridden && !has_headers, t!("csv.has_headers_no").to_string()).clicked() {
                                        if let Some(state) = self.csv_viewer_states.get_mut(&tab_id) {
                                            state.set_header_override(false);
                                        }
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                });

                                ui.separator();

                                // Rainbow columns toggle
                                // (capture values and defer mutation to avoid borrow conflict)
                                let rainbow_enabled = self.state.settings.csv_rainbow_columns;
                                let rainbow_label = format!(
                                    "Colors: {}",
                                    if rainbow_enabled { "🌈" } else { "○" }
                                );

                                let rainbow_button_response = ui.add(
                                    egui::Button::new(&rainbow_label)
                                        .frame(false)
                                        .sense(egui::Sense::click())
                                );

                                rainbow_button_response.clone().on_hover_text(format!(
                                    "Rainbow column coloring: {}\nClick to toggle",
                                    if rainbow_enabled { "On" } else { "Off" }
                                ));

                                if rainbow_button_response.clicked() {
                                    toggle_rainbow_columns = true;
                                }
                            }
                        }

                        ui.separator();

                        // Encoding display and picker
                        let encoding_display = tab.encoding_display_name();
                        let encoding_popup_id = ui.make_persistent_id("encoding_picker_popup");
                        let encoding_button_response = ui.add(
                            egui::Button::new(&encoding_display)
                                .frame(false)
                                .sense(egui::Sense::click())
                        );

                        encoding_button_response.clone().on_hover_text(format!(
                            "File encoding: {}\n{}Click to change",
                            encoding_display,
                            if tab.detected_encoding.is_some() { "Detected. " } else { "Default. " }
                        ));

                        if encoding_button_response.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(encoding_popup_id));
                        }

                        // Encoding picker popup
                        egui::popup_below_widget(ui, encoding_popup_id, &encoding_button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                            ui.set_min_width(150.0);
                            ui.label(egui::RichText::new("File Encoding").strong());
                            ui.separator();

                            // Show common encodings
                            for &enc in crate::state::Tab::COMMON_ENCODINGS {
                                let selected = tab.current_encoding.eq_ignore_ascii_case(enc);
                                let label = enc.to_uppercase();
                                if ui.selectable_label(selected, label).clicked() {
                                    pending_encoding_change = Some(enc);
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            }
                        });

                        ui.separator();

                        // Text statistics
                        let stats = TextStats::from_text(&tab.content);
                        ui.label(stats.format_compact());
                    }
                });
            });
        });

        // Apply deferred encoding change (outside tab borrow scope)
        if let Some(new_encoding) = pending_encoding_change {
            if let Some(tab) = self.state.active_tab_mut() {
                if let Err(e) = tab.set_encoding(new_encoding) {
                    warn!("Failed to change encoding: {}", e);
                    let time = self.get_app_time();
                    self.state.show_toast(format!("Failed to change encoding: {}", e), time, 3.0);
                } else {
                    let time = self.get_app_time();
                    self.state.show_toast(format!("Encoding changed to {}", new_encoding.to_uppercase()), time, 2.0);
                }
            }
        }

        // Apply deferred rainbow columns toggle (outside tab borrow scope)
        if toggle_rainbow_columns {
            self.state.settings.csv_rainbow_columns = !self.state.settings.csv_rainbow_columns;
            self.state.mark_settings_dirty();
        }

        } // End of status bar (hidden in Zen Mode)

        // ═══════════════════════════════════════════════════════════════════
        // Outline Panel (if enabled) - hidden in Zen Mode
        // ═══════════════════════════════════════════════════════════════════
        let mut outline_nav_request: Option<HeadingNavRequest> = None;
        let mut outline_toggled_id: Option<String> = None;
        let mut outline_new_width: Option<f32> = None;
        let mut outline_close_requested = false;

        if self.state.settings.outline_enabled && !zen_mode {
            // Update outline if content changed
            self.update_outline_if_needed();

            // Determine current section based on cursor position
            let current_line = self
                .state
                .active_tab()
                .map(|t| t.cursor_position.0 + 1) // Convert to 1-indexed
                .unwrap_or(0);
            let current_section = self.cached_outline.find_current_section(current_line);

            // Configure and render the outline panel
            self.outline_panel
                .set_side(self.state.settings.outline_side);
            self.outline_panel.set_current_section(current_section);
            let outline_output = self.outline_panel.show(
                ctx,
                &self.cached_outline,
                self.cached_doc_stats.as_ref(),
                is_dark,
            );

            // Capture output for processing after render
            if let Some(line) = outline_output.scroll_to_line {
                outline_nav_request = Some(HeadingNavRequest {
                    line,
                    char_offset: outline_output.scroll_to_char,
                    title: outline_output.scroll_to_title,
                    level: outline_output.scroll_to_level,
                });
            }
            outline_toggled_id = outline_output.toggled_id;
            outline_new_width = outline_output.new_width;
            outline_close_requested = outline_output.close_requested;
        }

        // Handle outline panel interactions - navigate with text matching and transient highlight
        if let Some(nav) = outline_nav_request {
            self.navigate_to_heading(nav);
        }

        if let Some(id) = outline_toggled_id {
            self.cached_outline.toggle_collapsed(&id);
        }

        if let Some(width) = outline_new_width {
            self.state.settings.outline_width = width;
            self.state.mark_settings_dirty();
        }

        if outline_close_requested {
            self.state.settings.outline_enabled = false;
            self.state.mark_settings_dirty();
        }

        // ═══════════════════════════════════════════════════════════════════
        // File Tree Panel (workspace mode only) - hidden in Zen Mode
        // ═══════════════════════════════════════════════════════════════════
        let mut file_tree_file_clicked: Option<std::path::PathBuf> = None;
        let mut file_tree_path_toggled: Option<std::path::PathBuf> = None;
        let mut file_tree_needs_loading: Option<std::path::PathBuf> = None;
        let mut file_tree_close_requested = false;
        let mut file_tree_new_width: Option<f32> = None;
        let mut file_tree_context_action: Option<FileTreeContextAction> = None;

        if self.state.should_show_file_tree() && !zen_mode {
            // Get Git statuses first (needs mutable borrow)
            let git_statuses = if self.state.git_service.is_open() {
                Some(self.state.git_service.get_all_statuses())
            } else {
                None
            };

            if let Some(workspace) = &self.state.workspace {
                let workspace_name = workspace
                    .root_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Workspace");

                let output = self.file_tree_panel.show(
                    ctx,
                    &workspace.file_tree,
                    workspace_name,
                    is_dark,
                    git_statuses.as_ref(),
                );

                file_tree_file_clicked = output.file_clicked;
                file_tree_path_toggled = output.path_toggled;
                file_tree_needs_loading = output.needs_loading;
                file_tree_close_requested = output.close_requested;
                file_tree_new_width = output.new_width;
                file_tree_context_action = output.context_action;
            }
        }

        // Handle file tree interactions
        if let Some(file_path) = file_tree_file_clicked {
            match self.state.open_file(file_path.clone()) {
                Ok(_) => {
                    debug!("Opened file from tree: {}", file_path.display());
                    // Add to workspace recent files
                    if let Some(workspace) = self.state.workspace_mut() {
                        workspace.add_recent_file(file_path);
                    }
                }
                Err(e) => {
                    warn!("Failed to open file: {}", e);
                    self.state
                        .show_error(format!("Failed to open file:\n{}", e));
                }
            }
        }

        // Handle lazy loading of directory children
        if let Some(path) = file_tree_needs_loading {
            if let Some(workspace) = self.state.workspace_mut() {
                workspace.load_directory(&path);
            }
        }

        if let Some(path) = file_tree_path_toggled {
            // Toggle expand/collapse for the path
            if let Some(workspace) = self.state.workspace_mut() {
                if let Some(node) = workspace.file_tree.find_mut(&path) {
                    node.is_expanded = !node.is_expanded;
                }
            }
        }

        if file_tree_close_requested {
            self.handle_close_workspace();
        }

        if let Some(width) = file_tree_new_width {
            if let Some(workspace) = self.state.workspace_mut() {
                workspace.file_tree_width = width;
            }
        }

        // Handle context menu actions
        if let Some(action) = file_tree_context_action {
            self.handle_file_tree_context_action(action);
        }

        // ═══════════════════════════════════════════════════════════════════
        // Live Pipeline Panel (Bottom panel for JSON/YAML command piping)
        // ═══════════════════════════════════════════════════════════════════
        // Only show if:
        // 1. Pipeline feature is enabled globally
        // 2. Not in Zen Mode (hide for distraction-free writing)
        // 3. Active tab is JSON/YAML and has pipeline panel visible
        let show_pipeline = self.state.settings.pipeline_enabled
            && !zen_mode
            && self.state.active_tab().map(|t| t.supports_pipeline() && t.pipeline_visible()).unwrap_or(false);

        if show_pipeline {
            let panel_height = self.pipeline_panel.height();
            egui::TopBottomPanel::bottom("pipeline_panel")
                .resizable(false) // We handle resize ourselves
                .exact_height(panel_height)
                .show(ctx, |ui| {
                    // Custom resize handle at the top of the panel
                    let resize_response = ui.allocate_response(
                        egui::vec2(ui.available_width(), 6.0),
                        egui::Sense::drag(),
                    );
                    
                    // Draw resize handle (thin line)
                    let handle_rect = resize_response.rect;
                    let handle_color = if resize_response.hovered() || resize_response.dragged() {
                        if is_dark {
                            egui::Color32::from_rgb(100, 100, 120)
                        } else {
                            egui::Color32::from_rgb(160, 160, 180)
                        }
                    } else {
                        if is_dark {
                            egui::Color32::from_rgb(60, 60, 70)
                        } else {
                            egui::Color32::from_rgb(200, 200, 210)
                        }
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(handle_rect.center(), egui::vec2(60.0, 3.0)),
                        2.0,
                        handle_color,
                    );
                    
                    // Change cursor on hover
                    if resize_response.hovered() || resize_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                    
                    // Handle drag to resize (dragging up = increase height, dragging down = decrease)
                    if resize_response.dragged() {
                        let delta = -resize_response.drag_delta().y; // Negative because up = bigger
                        let new_height = (panel_height + delta).clamp(100.0, 500.0);
                        if (new_height - panel_height).abs() > 0.5 {
                            self.pipeline_panel.set_height(new_height);
                            self.state.settings.pipeline_panel_height = new_height;
                            self.state.mark_settings_dirty();
                        }
                    }

                    // Get working directory from tab's file path or workspace
                    let working_dir = self.state.active_tab()
                        .and_then(|t| t.path.as_ref())
                        .and_then(|p| p.parent())
                        .map(|p| p.to_path_buf())
                        .or_else(|| self.state.workspace.as_ref().map(|w| w.root_path.clone()));

                    // Get content and tab state
                    let content = self.state.active_tab().map(|t| t.content.clone()).unwrap_or_default();

                    if let Some(tab) = self.state.active_tab_mut() {
                        let output = self.pipeline_panel.show(
                            ui,
                            &mut tab.pipeline_state,
                            &content,
                            working_dir,
                            is_dark,
                        );

                        // Handle panel close
                        if output.closed {
                            // Tab's pipeline_visible is already set to false by the panel
                        }
                    }

                    // Save recent commands if they changed
                    let recent_cmds = self.pipeline_panel.get_recent_commands_vec();
                    if recent_cmds != self.state.settings.pipeline_recent_commands {
                        self.state.settings.pipeline_recent_commands = recent_cmds;
                        self.state.mark_settings_dirty();
                    }
                });
        }

        // Central panel for editor content
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar - uses custom wrapping layout for multi-line support
            // Hidden in Zen Mode for distraction-free editing
            let mut tab_to_close: Option<usize> = None;
            
            if !zen_mode {

            // Collect tab info first to avoid borrow issues
            let tab_count = self.state.tab_count();
            let active_index = self.state.active_tab_index();
            let tab_titles: Vec<(usize, String, bool)> = (0..tab_count)
                .filter_map(|i| {
                    self.state
                        .tab(i)
                        .map(|tab| (i, tab.title(), i == active_index))
                })
                .collect();

            // Custom wrapping tab bar
            let available_width = ui.available_width();
            let tab_height = 24.0;
            let tab_spacing = 4.0;
            let close_btn_width = 18.0;
            let tab_padding = 16.0; // horizontal padding inside tab
            let min_text_width = 60.0;

            // Pre-calculate tab widths using actual text measurement
            // This ensures consistent sizing between layout and render passes
            let tab_widths: Vec<f32> = tab_titles
                .iter()
                .map(|(_, title, _)| {
                    let text_galley = ui.fonts(|f| {
                        f.layout_no_wrap(
                            title.clone(),
                            egui::FontId::default(),
                            egui::Color32::WHITE, // color doesn't affect measurement
                        )
                    });
                    let text_width = text_galley.size().x.max(min_text_width);
                    text_width + close_btn_width + tab_padding
                })
                .collect();

            // Calculate tab positions for layout
            let mut current_x = 0.0;
            let mut current_row = 0;
            let mut tab_positions: Vec<(f32, usize)> = Vec::new(); // (x position, row)

            for tab_width in &tab_widths {
                // Check if we need to wrap to next row
                if current_x + tab_width > available_width && current_x > 0.0 {
                    current_x = 0.0;
                    current_row += 1;
                }

                tab_positions.push((current_x, current_row));
                current_x += tab_width + tab_spacing;
            }

            // Add position for the + button
            let plus_btn_width = 24.0;
            if current_x + plus_btn_width > available_width && current_x > 0.0 {
                current_row += 1;
            }
            let total_rows = current_row + 1;
            let total_height = total_rows as f32 * (tab_height + 2.0);

            // Allocate space for all tab rows
            let (tab_bar_rect, _) = ui.allocate_exact_size(
                egui::vec2(available_width, total_height),
                egui::Sense::hover(),
            );

            // Render tabs
            let is_dark = ui.visuals().dark_mode;
            let selected_bg = ui.visuals().selection.bg_fill;
            let hover_bg = if is_dark {
                egui::Color32::from_rgb(60, 60, 70)
            } else {
                egui::Color32::from_rgb(220, 220, 230)
            };
            let text_color = ui.visuals().text_color();

            for (idx, (((tab_idx, title, selected), (x_pos, row)), tab_width)) in
                tab_titles.iter().zip(tab_positions.iter()).zip(tab_widths.iter()).enumerate()
            {
                // Use pre-calculated tab width for consistency
                let tab_width = *tab_width;

                let tab_rect = egui::Rect::from_min_size(
                    tab_bar_rect.min + egui::vec2(*x_pos, *row as f32 * (tab_height + 2.0)),
                    egui::vec2(tab_width, tab_height),
                );

                // Tab interaction
                let tab_response = ui.interact(
                    tab_rect,
                    egui::Id::new("tab").with(idx),
                    egui::Sense::click(),
                );

                // Draw tab background
                if *selected {
                    ui.painter().rect_filled(tab_rect, 4.0, selected_bg);
                } else if tab_response.hovered() {
                    ui.painter().rect_filled(tab_rect, 4.0, hover_bg);
                }

                // Draw tab title - use available width minus close button and padding
                let title_available_width = tab_width - close_btn_width - tab_padding;
                let title_rect = egui::Rect::from_min_size(
                    tab_rect.min + egui::vec2(8.0, 4.0),
                    egui::vec2(title_available_width, tab_height - 8.0),
                );
                ui.painter().text(
                    title_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    title,
                    egui::FontId::default(),
                    text_color,
                );

                // Draw close button
                let close_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        tab_rect.right() - close_btn_width - 4.0,
                        tab_rect.top() + 4.0,
                    ),
                    egui::vec2(close_btn_width, tab_height - 8.0),
                );
                let close_response = ui.interact(
                    close_rect,
                    egui::Id::new("tab_close").with(idx),
                    egui::Sense::click(),
                );

                let close_color = if close_response.hovered() {
                    egui::Color32::from_rgb(220, 80, 80)
                } else {
                    text_color
                };
                ui.painter().text(
                    close_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "×",
                    egui::FontId::default(),
                    close_color,
                );

                // Handle interactions
                if tab_response.clicked() && !close_response.hovered() {
                    self.state.set_active_tab(*tab_idx);
                }
                if close_response.clicked() {
                    tab_to_close = Some(*tab_idx);
                }
                if close_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                } else if tab_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }

            // Draw + button - use pre-calculated tab widths for consistency
            let plus_x = if tab_positions.is_empty() || tab_widths.is_empty() {
                0.0
            } else {
                let last_pos = tab_positions.last().unwrap();
                let last_width = *tab_widths.last().unwrap();

                if last_pos.0 + last_width + tab_spacing + plus_btn_width > available_width {
                    0.0 // Wrap to next row
                } else {
                    last_pos.0 + last_width + tab_spacing
                }
            };
            let plus_row = if tab_positions.is_empty() {
                0
            } else if plus_x == 0.0 && !tab_positions.is_empty() {
                tab_positions.last().unwrap().1 + 1
            } else {
                tab_positions.last().unwrap().1
            };

            let plus_rect = egui::Rect::from_min_size(
                tab_bar_rect.min + egui::vec2(plus_x, plus_row as f32 * (tab_height + 2.0)),
                egui::vec2(plus_btn_width, tab_height),
            );
            let plus_response = ui.interact(
                plus_rect,
                egui::Id::new("new_tab_btn"),
                egui::Sense::click(),
            );

            if plus_response.hovered() {
                ui.painter().rect_filled(plus_rect, 4.0, hover_bg);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            ui.painter().text(
                plus_rect.center(),
                egui::Align2::CENTER_CENTER,
                "+",
                egui::FontId::default(),
                text_color,
            );
            if plus_response.clicked() {
                self.state.new_tab();
            }
            plus_response.on_hover_text(t!("tooltip.new_tab").to_string());

            // Handle tab close action
            if let Some(index) = tab_to_close {
                // Get tab_id before closing for viewer state cleanup
                let tab_id = self.state.tabs().get(index).map(|t| t.id);
                self.state.close_tab(index);
                if let Some(id) = tab_id {
                    self.cleanup_tab_state(id, Some(ui.ctx()));
                }
            }

            // Draw a visible separator line between tabs and editor
            // Uses stronger contrast than default egui separator for accessibility
            ui.add_space(2.0);
            {
                let separator_color = if is_dark {
                    egui::Color32::from_rgb(60, 60, 60)
                } else {
                    egui::Color32::from_rgb(160, 160, 160) // ~3.2:1 contrast on white
                };
                let rect = ui.available_rect_before_wrap();
                let y = rect.min.y;
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0, separator_color),
                );
            }
            ui.add_space(3.0);
            } // End of tab bar (hidden in Zen Mode)

            // Editor widget - extract settings values to avoid borrow conflicts
            let font_size = self.state.settings.font_size;
            let font_family = self.state.settings.font_family.clone();
            let word_wrap = self.state.settings.word_wrap;
            let theme = self.state.settings.theme;
            let show_line_numbers = self.state.settings.show_line_numbers;

            // Get theme colors for line number styling
            let theme_colors = ThemeColors::from_theme(theme, ui.visuals());

            // Prepare search highlights if find panel is open
            let search_highlights = if self.state.ui.show_find_replace
                && !self.state.ui.find_state.matches.is_empty()
            {
                let highlights = SearchHighlights {
                    matches: self.state.ui.find_state.matches.clone(),
                    current_match: self.state.ui.find_state.current_match,
                    scroll_to_match: self.state.ui.scroll_to_match,
                };
                // Clear scroll flag after using it
                self.state.ui.scroll_to_match = false;
                Some(highlights)
            } else {
                None
            };

            // Extract pending scroll request before mutable borrow
            let scroll_to_line = self.pending_scroll_to_line.take();

            // Get tab metadata before mutable borrow
            let tab_info = self.state.active_tab().map(|t| {
                (
                    t.id,
                    t.view_mode,
                    t.path.as_ref().and_then(|p| get_structured_file_type(p)),
                    t.path.as_ref().and_then(|p| get_tabular_file_type(p)),
                    t.transient_highlight_range(),
                )
            });

            if let Some((tab_id, view_mode, structured_type, tabular_type, transient_hl)) = tab_info {
                match view_mode {
                    ViewMode::Raw => {
                        // Raw mode: use the plain EditorWidget with optional minimap
                        let zen_max_column_width = self.state.settings.zen_max_column_width;
                        let max_line_width = self.state.settings.max_line_width;

                        // Capture scroll offset before mutable borrow for scroll detection
                        let prev_scroll_offset = self.state.active_tab().map(|t| t.scroll_offset).unwrap_or(0.0);

                        // Get folding settings (before mutable borrow)
                        let folding_enabled = self.state.settings.folding_enabled;
                        let show_fold_indicators = self.state.settings.folding_show_indicators && folding_enabled;
                        let fold_headings = self.state.settings.fold_headings;
                        let fold_code_blocks = self.state.settings.fold_code_blocks;
                        let fold_lists = self.state.settings.fold_lists;
                        let fold_indentation = self.state.settings.fold_indentation;

                        // Get bracket matching setting
                        let highlight_matching_pairs = self.state.settings.highlight_matching_pairs;

                        // Get syntax highlighting settings
                        let syntax_highlighting_enabled = self.state.settings.syntax_highlighting_enabled;
                        let syntax_theme = if self.state.settings.syntax_theme.is_empty() {
                            None
                        } else {
                            Some(self.state.settings.syntax_theme.clone())
                        };

                        // Get minimap settings (hidden in Zen Mode)
                        let minimap_enabled = self.state.settings.minimap_enabled && !zen_mode;
                        let minimap_width = self.state.settings.minimap_width;
                        let minimap_mode = self.state.settings.minimap_mode;

                        // Check if file is markdown (for auto mode minimap selection)
                        // Check extension directly to avoid any caching issues
                        let is_markdown_file = self.state.active_tab()
                            .map(|tab| {
                                match &tab.path {
                                    Some(path) => {
                                        // Check extension directly
                                        let ext_result = path.extension()
                                            .and_then(|e| e.to_str())
                                            .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                                            .unwrap_or(false); // No extension = not markdown
                                        trace!(
                                            "Minimap file type check: path={:?}, ext={:?}, is_markdown={}",
                                            path.file_name(),
                                            path.extension(),
                                            ext_result
                                        );
                                        ext_result
                                    }
                                    None => {
                                        trace!("Minimap file type check: unsaved file, defaulting to markdown");
                                        true // Unsaved files default to markdown
                                    }
                                }
                            })
                            .unwrap_or(true);

                        // Determine whether to use semantic minimap based on mode setting
                        let use_semantic_minimap = minimap_mode.use_semantic(is_markdown_file);

                        // Get tab data needed for minimap before mutable borrow
                        // For semantic: structure-based minimap with headings
                        // For pixel: code overview minimap
                        let semantic_minimap_data = if minimap_enabled && use_semantic_minimap {
                            self.state.active_tab().map(|t| {
                                // Extract outline for semantic minimap
                                let outline = crate::editor::extract_outline_for_file(
                                    &t.content,
                                    t.path.as_deref(),
                                );
                                let total_lines = t.content.lines().count();
                                (
                                    outline,
                                    t.scroll_offset,
                                    t.content_height,
                                    t.raw_line_height,
                                    t.cursor_position.0 + 1, // Convert 0-indexed to 1-indexed line
                                    total_lines,
                                )
                            })
                        } else {
                            None
                        };

                        let pixel_minimap_data = if minimap_enabled && !use_semantic_minimap {
                            self.state.active_tab().map(|t| {
                                (
                                    t.content.clone(),
                                    t.scroll_offset,
                                    t.viewport_height,
                                    t.content_height,
                                    t.raw_line_height,
                                )
                            })
                        } else {
                            None
                        };

                        // Get search matches for pixel minimap visualization
                        let minimap_search_matches: Vec<(usize, usize)> = if minimap_enabled && !use_semantic_minimap {
                            self.state.ui.find_state.matches.clone()
                        } else {
                            Vec::new()
                        };
                        let minimap_current_match = self.state.ui.find_state.current_match;

                        // Track minimap scroll request
                        let mut minimap_nav_request: Option<HeadingNavRequest> = None;
                        let mut minimap_scroll_to_offset: Option<f32> = None;

                        // Clone tab path before mutable borrow for syntax highlighting
                        let tab_path_for_syntax = self.state.active_tab().and_then(|t| t.path.clone());

                        if let Some(tab) = self.state.active_tab_mut() {
                            // Update folds if dirty
                            if folding_enabled && tab.folds_dirty() {
                                tab.update_folds(
                                    fold_headings,
                                    fold_code_blocks,
                                    fold_lists,
                                    fold_indentation,
                                );
                            }

                            // Calculate layout for editor and minimap
                            let total_rect = ui.available_rect_before_wrap();
                            let editor_width = if minimap_enabled {
                                total_rect.width() - minimap_width
                            } else {
                                total_rect.width()
                            };

                            let editor_rect = egui::Rect::from_min_size(
                                total_rect.min,
                                egui::vec2(editor_width, total_rect.height()),
                            );
                            let minimap_rect = if minimap_enabled {
                                Some(egui::Rect::from_min_size(
                                    egui::pos2(total_rect.min.x + editor_width, total_rect.min.y),
                                    egui::vec2(minimap_width, total_rect.height()),
                                ))
                            } else {
                                None
                            };

                            // Allocate the total area
                            ui.allocate_rect(total_rect, egui::Sense::hover());

                            // Show editor in its region
                            let mut editor_ui = ui.child_ui(editor_rect, egui::Layout::top_down(egui::Align::LEFT), None);

                            let mut editor = EditorWidget::new(tab)
                                .font_size(font_size)
                                .font_family(font_family.clone())
                                .word_wrap(word_wrap)
                                .show_line_numbers(show_line_numbers && !zen_mode) // Hide line numbers in Zen Mode
                                .show_fold_indicators(show_fold_indicators && !zen_mode) // Hide in Zen Mode
                                .theme_colors(theme_colors.clone())
                                .id(egui::Id::new("main_editor_raw"))
                                .scroll_to_line(scroll_to_line)
                                .zen_mode(zen_mode, zen_max_column_width)
                                .max_line_width(max_line_width) // Apply when not in Zen Mode
                                .transient_highlight(transient_hl)
                                .highlight_matching_pairs(highlight_matching_pairs)
                                .syntax_highlighting(syntax_highlighting_enabled, tab_path_for_syntax.clone(), is_dark)
                                .syntax_theme(syntax_theme.clone());

                            // Add search highlights if available
                            if let Some(highlights) = search_highlights.clone() {
                                editor = editor.search_highlights(highlights);
                            }

                            let editor_output = editor.show(&mut editor_ui);

                            // Handle fold toggle click
                            if let Some(fold_line) = editor_output.fold_toggle_line {
                                tab.toggle_fold_at_line(fold_line);
                            }

                            // Handle transient highlight expiry
                            if tab.has_transient_highlight() {
                                // Clear on edit
                                if editor_output.changed {
                                    tab.on_edit_event();
                                    debug!("Cleared transient highlight due to edit");
                                }
                                // Clear on scroll (after the initial programmatic scroll)
                                else if (tab.scroll_offset - prev_scroll_offset).abs() > 1.0 {
                                    tab.on_scroll_event();
                                    // Note: on_scroll_event handles the guard for initial scroll
                                }
                                // Clear on any mouse click in the editor
                                else if ui.input(|i| i.pointer.any_click()) {
                                    tab.on_click_event();
                                    debug!("Cleared transient highlight due to click");
                                }
                            }

                            if editor_output.changed {
                                debug!("Content modified in raw editor");
                                // Mark folds as dirty when content changes
                                if folding_enabled {
                                    tab.mark_folds_dirty();
                                }
                            }

                            // Handle Ctrl+Click to add cursor
                            if let Some(click_pos) = editor_output.ctrl_click_pos {
                                tab.add_cursor(click_pos);
                                debug!(
                                    "{}+Click: added cursor at position {}, now {} cursor(s)",
                                    modifier_symbol(),
                                    click_pos,
                                    tab.cursor_count()
                                );
                            }

                            // Show minimap if enabled
                            if let Some(minimap_rect) = minimap_rect {
                                let mut minimap_ui = ui.child_ui(minimap_rect, egui::Layout::top_down(egui::Align::LEFT), None);

                                // Use semantic minimap for markdown files
                                if let Some((outline, scroll_offset, content_height, line_height, current_line, total_lines)) = semantic_minimap_data {
                                    let semantic_minimap = SemanticMinimap::new(&outline.items)
                                        .width(minimap_width)
                                        .scroll_offset(scroll_offset)
                                        .content_height(content_height)
                                        .line_height(line_height)
                                        .current_line(Some(current_line))
                                        .total_lines(total_lines)
                                        .theme_colors(theme_colors.clone());

                                    let minimap_output = semantic_minimap.show(&mut minimap_ui);

                                    // Handle semantic minimap navigation with text matching
                                    if let Some(target_line) = minimap_output.scroll_to_line {
                                        minimap_nav_request = Some(HeadingNavRequest {
                                            line: target_line,
                                            char_offset: minimap_output.scroll_to_char,
                                            title: minimap_output.scroll_to_title,
                                            level: minimap_output.scroll_to_level,
                                        });
                                    }
                                }
                                // Use pixel minimap for non-markdown files
                                else if let Some((content, scroll_offset, viewport_height, content_height, line_height)) = pixel_minimap_data {
                                    let mut minimap = Minimap::new(&content)
                                        .width(minimap_width)
                                        .scroll_offset(scroll_offset)
                                        .viewport_height(viewport_height)
                                        .content_height(content_height)
                                        .line_height(line_height)
                                        .theme_colors(theme_colors.clone());

                                    // Add search highlights to pixel minimap
                                    if !minimap_search_matches.is_empty() {
                                        minimap = minimap
                                            .search_highlights(&minimap_search_matches)
                                            .current_match(minimap_current_match);
                                    }

                                    let minimap_output = minimap.show(&mut minimap_ui);

                                    // Handle pixel minimap navigation
                                    if let Some(target_offset) = minimap_output.scroll_to_offset {
                                        minimap_scroll_to_offset = Some(target_offset);
                                    }
                                }
                            }
                        }

                        // Apply minimap navigation request (after mutable borrow ends)
                        if let Some(nav) = minimap_nav_request {
                            self.navigate_to_heading(nav);
                            ui.ctx().request_repaint();
                        }
                        if let Some(scroll_offset) = minimap_scroll_to_offset {
                            if let Some(tab) = self.state.active_tab_mut() {
                                tab.pending_scroll_offset = Some(scroll_offset);
                                ui.ctx().request_repaint();
                            }
                        }
                    }
                    ViewMode::Split => {
                        // Split view: raw editor on left, rendered preview on right
                        // Not available for structured files
                        
                        if structured_type.is_some() {
                            // Structured (JSON/YAML/TOML) files don't support split view,
                            // switch to Raw mode. CSV/TSV files DO support split view.
                            if let Some(tab) = self.state.active_tab_mut() {
                                tab.view_mode = ViewMode::Raw;
                            }
                        } else {
                            // Get split ratio before mutable borrow
                            let split_ratio = self.state.active_tab().map(|t| t.split_ratio).unwrap_or(0.5);
                            let available_width = ui.available_width();
                            let _available_height = ui.available_height(); // For reference (using rect-based layout)
                            let splitter_width = 8.0; // Width of the draggable splitter area

                            // Get Zen Mode settings
                            let zen_max_column_width = self.state.settings.zen_max_column_width;

                            // Get minimap settings (hidden in Zen Mode for distraction-free editing)
                            let minimap_enabled = self.state.settings.minimap_enabled && !zen_mode;
                            let minimap_width = self.state.settings.minimap_width;
                            let minimap_mode = self.state.settings.minimap_mode;
                            let effective_minimap_width = if minimap_enabled { minimap_width } else { 0.0 };

                            // Calculate widths: left pane gets split_ratio of (total - splitter - minimap)
                            let content_width = available_width - splitter_width - effective_minimap_width;
                            let left_width = content_width * split_ratio;
                            let right_width = content_width * (1.0 - split_ratio);

                            // Get folding settings (fold indicators hidden in Zen Mode)
                            let folding_enabled = self.state.settings.folding_enabled;
                            let show_fold_indicators = self.state.settings.folding_show_indicators && folding_enabled && !zen_mode;
                            let fold_headings = self.state.settings.fold_headings;
                            let fold_code_blocks = self.state.settings.fold_code_blocks;
                            let fold_lists = self.state.settings.fold_lists;
                            let fold_indentation = self.state.settings.fold_indentation;

                            // Get bracket matching setting
                            let highlight_matching_pairs = self.state.settings.highlight_matching_pairs;

                            // Get syntax highlighting settings
                            let syntax_highlighting_enabled = self.state.settings.syntax_highlighting_enabled;
                            let syntax_theme = if self.state.settings.syntax_theme.is_empty() {
                                None
                            } else {
                                Some(self.state.settings.syntax_theme.clone())
                            };

                            // Get line width setting
                            let max_line_width = self.state.settings.max_line_width;

                            // Get paragraph indent setting (CJK typography)
                            let paragraph_indent = self.state.settings.paragraph_indent;

                            // Get path for syntax highlighting
                            let tab_path_for_syntax = self.state.active_tab().and_then(|t| t.path.clone());

                            // Check if file is markdown (for auto mode minimap selection)
                            let is_markdown_file_split = self.state.active_tab()
                                .map(|tab| {
                                    match &tab.path {
                                        Some(path) => {
                                            path.extension()
                                                .and_then(|e| e.to_str())
                                                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                                                .unwrap_or(false)
                                        }
                                        None => true, // Unsaved files default to markdown
                                    }
                                })
                                .unwrap_or(true);

                            // Determine whether to use semantic minimap based on mode setting
                            let use_semantic_minimap_split = minimap_mode.use_semantic(is_markdown_file_split);

                            // Get tab data for semantic minimap (when using semantic mode)
                            let semantic_minimap_data_split = if minimap_enabled && use_semantic_minimap_split {
                                self.state.active_tab().map(|t| {
                                    let outline = crate::editor::extract_outline_for_file(
                                        &t.content,
                                        t.path.as_deref(),
                                    );
                                    let total_lines = t.content.lines().count();
                                    (
                                        outline,
                                        t.scroll_offset,
                                        t.content_height,
                                        t.raw_line_height,
                                        t.cursor_position.0 + 1, // Convert 0-indexed to 1-indexed line
                                        total_lines,
                                    )
                                })
                            } else {
                                None
                            };

                            // Get tab data for pixel minimap (when using pixel mode)
                            let pixel_minimap_data_split = if minimap_enabled && !use_semantic_minimap_split {
                                self.state.active_tab().map(|t| {
                                    (
                                        t.content.clone(),
                                        t.scroll_offset,
                                        t.viewport_height,
                                        t.content_height,
                                        t.raw_line_height,
                                    )
                                })
                            } else {
                                None
                            };

                            // Track minimap navigation request
                            let mut minimap_nav_request: Option<HeadingNavRequest> = None;

                            // Calculate explicit rectangles for split view layout
                            // Layout: [Editor] [Minimap] [Splitter] [Preview]
                            let total_rect = ui.available_rect_before_wrap();
                            let left_rect = egui::Rect::from_min_size(
                                total_rect.min,
                                egui::vec2(left_width, total_rect.height()),
                            );
                            let minimap_rect = if minimap_enabled {
                                Some(egui::Rect::from_min_size(
                                    egui::pos2(total_rect.min.x + left_width, total_rect.min.y),
                                    egui::vec2(minimap_width, total_rect.height()),
                                ))
                            } else {
                                None
                            };
                            let splitter_rect = egui::Rect::from_min_size(
                                egui::pos2(total_rect.min.x + left_width + effective_minimap_width, total_rect.min.y),
                                egui::vec2(splitter_width, total_rect.height()),
                            );
                            let right_rect = egui::Rect::from_min_size(
                                egui::pos2(total_rect.min.x + left_width + effective_minimap_width + splitter_width, total_rect.min.y),
                                egui::vec2(right_width, total_rect.height()),
                            );

                            // Allocate the entire area so egui knows we're using it
                            ui.allocate_rect(total_rect, egui::Sense::hover());

                            // ═══════════════════════════════════════════════════════════════
                            // Left pane: Raw editor
                            // ═══════════════════════════════════════════════════════════════
                            let mut left_ui = ui.child_ui_with_id_source(left_rect, egui::Layout::top_down(egui::Align::LEFT), "split_left_pane", None);
                            if let Some(tab) = self.state.active_tab_mut() {
                                // Update folds if dirty
                                if folding_enabled && tab.folds_dirty() {
                                    tab.update_folds(
                                        fold_headings,
                                        fold_code_blocks,
                                        fold_lists,
                                        fold_indentation,
                                    );
                                }

                                let mut editor = EditorWidget::new(tab)
                                    .font_size(font_size)
                                    .font_family(font_family.clone())
                                    .word_wrap(word_wrap)
                                    .show_line_numbers(show_line_numbers && !zen_mode) // Hide in Zen Mode
                                    .show_fold_indicators(show_fold_indicators)
                                    .theme_colors(theme_colors.clone())
                                    .id(egui::Id::new("split_editor_raw"))
                                    .scroll_to_line(scroll_to_line)
                                    .max_line_width(max_line_width)
                                    .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                    .transient_highlight(transient_hl)
                                    .highlight_matching_pairs(highlight_matching_pairs)
                                    .syntax_highlighting(syntax_highlighting_enabled, tab_path_for_syntax.clone(), is_dark)
                                    .syntax_theme(syntax_theme.clone());

                                // Add search highlights if available
                                if let Some(highlights) = search_highlights.clone() {
                                    editor = editor.search_highlights(highlights);
                                }

                                let editor_output = editor.show(&mut left_ui);

                                // Handle fold toggle click
                                if let Some(fold_line) = editor_output.fold_toggle_line {
                                    tab.toggle_fold_at_line(fold_line);
                                }

                                // Handle transient highlight expiry
                                if tab.has_transient_highlight() {
                                    if editor_output.changed {
                                        tab.on_edit_event();
                                    } else if left_ui.input(|i| i.pointer.any_click()) {
                                        tab.on_click_event();
                                    }
                                }

                                if editor_output.changed {
                                    if folding_enabled {
                                        tab.mark_folds_dirty();
                                    }
                                }
                            }

                            // ═══════════════════════════════════════════════════════════════
                            // Minimap (between editor and splitter)
                            // Uses semantic minimap for markdown, pixel minimap for others
                            // ═══════════════════════════════════════════════════════════════
                            if let Some(mm_rect) = minimap_rect {
                                let mut minimap_ui = ui.child_ui(mm_rect, egui::Layout::top_down(egui::Align::LEFT), None);

                                // Semantic minimap for markdown files
                                if let Some((outline, scroll_offset, content_height, line_height, current_line, total_lines)) = semantic_minimap_data_split {
                                    let semantic_minimap = SemanticMinimap::new(&outline.items)
                                        .width(minimap_width)
                                        .scroll_offset(scroll_offset)
                                        .content_height(content_height)
                                        .line_height(line_height)
                                        .current_line(Some(current_line))
                                        .total_lines(total_lines)
                                        .theme_colors(theme_colors.clone());

                                    let minimap_output = semantic_minimap.show(&mut minimap_ui);

                                    // Handle semantic minimap navigation with text matching
                                    if let Some(target_line) = minimap_output.scroll_to_line {
                                        minimap_nav_request = Some(HeadingNavRequest {
                                            line: target_line,
                                            char_offset: minimap_output.scroll_to_char,
                                            title: minimap_output.scroll_to_title,
                                            level: minimap_output.scroll_to_level,
                                        });
                                    }
                                }
                                // Pixel minimap for non-markdown files
                                else if let Some((content, scroll_offset, viewport_height, content_height, line_height)) = pixel_minimap_data_split {
                                    let minimap = Minimap::new(&content)
                                        .width(minimap_width)
                                        .scroll_offset(scroll_offset)
                                        .viewport_height(viewport_height)
                                        .content_height(content_height)
                                        .line_height(line_height)
                                        .theme_colors(theme_colors.clone());

                                    let minimap_output = minimap.show(&mut minimap_ui);

                                    // Handle pixel minimap scroll
                                    if let Some(offset) = minimap_output.scroll_to_offset {
                                        if let Some(tab) = self.state.active_tab_mut() {
                                            tab.pending_scroll_offset = Some(offset);
                                        }
                                        ui.ctx().request_repaint();
                                    }
                                }
                            }

                            // Apply minimap navigation request
                            if let Some(nav) = minimap_nav_request {
                                self.navigate_to_heading(nav);
                                ui.ctx().request_repaint();
                            }

                            // ═══════════════════════════════════════════════════════════════
                            // Splitter (draggable)
                            // ═══════════════════════════════════════════════════════════════
                            let splitter_response = ui.interact(splitter_rect, egui::Id::new("split_splitter"), egui::Sense::click_and_drag());

                            // Draw splitter visual
                            let is_dark = ui.visuals().dark_mode;
                            let splitter_color = if splitter_response.hovered() || splitter_response.dragged() {
                                if is_dark {
                                    egui::Color32::from_rgb(100, 100, 120)
                                } else {
                                    egui::Color32::from_rgb(140, 140, 160)
                                }
                            } else if is_dark {
                                egui::Color32::from_rgb(60, 60, 70)
                            } else {
                                egui::Color32::from_rgb(180, 180, 190)
                            };

                            ui.painter().rect_filled(splitter_rect, 0.0, splitter_color);

                            // Draw grip lines in the center
                            let grip_color = if is_dark {
                                egui::Color32::from_rgb(120, 120, 140)
                            } else {
                                egui::Color32::from_rgb(100, 100, 120)
                            };
                            let center_x = splitter_rect.center().x;
                            let center_y = splitter_rect.center().y;
                            for i in -2..=2 {
                                let y = center_y + i as f32 * 6.0;
                                ui.painter().line_segment(
                                    [egui::pos2(center_x - 2.0, y), egui::pos2(center_x + 2.0, y)],
                                    egui::Stroke::new(1.0, grip_color),
                                );
                            }

                            // Handle drag to resize
                            // Calculate ratio based on content_width (excluding minimap and splitter)
                            if splitter_response.dragged() {
                                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                    // The draggable area is content_width, and minimap is between editor and splitter
                                    // So we need to calculate ratio of (pointer - left - minimap) / content_width
                                    let drag_pos = pointer_pos.x - total_rect.left();
                                    // If minimap is enabled, the left pane ends at the minimap
                                    // The ratio should be based on how much of content_width is on the left
                                    let new_ratio = (drag_pos / (content_width + effective_minimap_width + splitter_width))
                                        .clamp(0.15, 0.85);
                                    if let Some(tab) = self.state.active_tab_mut() {
                                        tab.set_split_ratio(new_ratio);
                                    }
                                }
                            }

                            // Set resize cursor
                            if splitter_response.hovered() || splitter_response.dragged() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            }

                            // ═══════════════════════════════════════════════════════════════
                            // Right pane: Rendered preview (fully editable)
                            // ═══════════════════════════════════════════════════════════════
                            let mut right_ui = ui.child_ui_with_id_source(right_rect, egui::Layout::top_down(egui::Align::LEFT), "split_right_pane", None);
                            
                            // Check if this is a CSV/TSV file for the right pane
                            if let Some(file_type) = tabular_type {
                                // Tabular file: use the CsvViewer (read-only table view)
                                let csv_state = self.csv_viewer_states.entry(tab_id).or_default();
                                let rainbow_columns = self.state.settings.csv_rainbow_columns;

                                if let Some(tab) = self.state.active_tab_mut() {
                                    let _output =
                                        CsvViewer::new(&tab.content, file_type, csv_state)
                                            .font_size(font_size)
                                            .rainbow_columns(rainbow_columns)
                                            .show(&mut right_ui);
                                }
                            } else {
                                // Rendered pane - fully editable like the main Rendered mode
                                // Edits here modify tab.content directly, with proper undo/redo support
                                if let Some(tab) = self.state.active_tab_mut() {
                                    // Capture content and cursor before editing for undo support
                                    let content_before = tab.content.clone();
                                    let cursor_before = tab.cursors.primary().head;

                                    let editor_output = MarkdownEditor::new(&mut tab.content)
                                        .mode(EditorMode::Rendered)
                                        .font_size(font_size)
                                        .font_family(font_family.clone())
                                        .word_wrap(word_wrap)
                                        .theme(theme)
                                        .max_line_width(max_line_width)
                                        .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                        .paragraph_indent(paragraph_indent) // CJK paragraph indentation
                                        .id(egui::Id::new("split_preview_rendered"))
                                        .show(&mut right_ui);

                                    if editor_output.changed {
                                        // Record edit for undo/redo support
                                        tab.record_edit(content_before, cursor_before);
                                        // Mark content as edited for auto-save scheduling
                                        tab.mark_content_edited();
                                        debug!("Content modified in split rendered pane, recorded for undo");
                                    }

                                    // Don't update cursor_position in Split mode - the raw editor (left pane)
                                    // already maintains it via sync_cursor_from_primary(). Overwriting it here
                                    // would break line operations (delete line, move line) when editing the raw pane.
                                    // cursor_position is only needed for Rendered-only mode.

                                    // Update selection from focused element (for formatting toolbar)
                                    if let Some(focused) = editor_output.focused_element {
                                        if let Some((sel_start, sel_end)) = focused.selection {
                                            if sel_start != sel_end {
                                                let abs_start = focused.start_char + sel_start;
                                                let abs_end = focused.start_char + sel_end;
                                                tab.selection = Some((abs_start, abs_end));
                                            } else {
                                                tab.selection = None;
                                            }
                                        } else {
                                            tab.selection = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ViewMode::Rendered => {
                        // Check if this is a tabular file (CSV, TSV)
                        if let Some(file_type) = tabular_type {
                            // Tabular file: use the CsvViewer (read-only table view)
                            let csv_state = self.csv_viewer_states.entry(tab_id).or_default();
                            let rainbow_columns = self.state.settings.csv_rainbow_columns;

                            if let Some(tab) = self.state.active_tab_mut() {
                                let output =
                                    CsvViewer::new(&tab.content, file_type, csv_state)
                                        .font_size(font_size)
                                        .rainbow_columns(rainbow_columns)
                                        .show(ui);

                                // Update scroll offset for sync scrolling
                                tab.scroll_offset = output.scroll_offset;
                            }
                        } else if let Some(file_type) = structured_type {
                            // Structured file (JSON, YAML, TOML): use the TreeViewer
                            // Note: For structured files, the outline panel shows statistics
                            // rather than navigation, so scroll_to_line is not used here.
                            let tree_state = self.tree_viewer_states.entry(tab_id).or_default();

                            if let Some(tab) = self.state.active_tab_mut() {
                                // Capture content and cursor before editing for undo support
                                let content_before = tab.content.clone();
                                let cursor_before = tab.cursors.primary().head;

                                let output =
                                    TreeViewer::new(&mut tab.content, file_type, tree_state)
                                        .font_size(font_size)
                                        .show(ui);

                                if output.changed {
                                    // Record edit for undo/redo support
                                    tab.record_edit(content_before, cursor_before);
                                    // Mark content as edited for auto-save scheduling
                                    tab.mark_content_edited();
                                    debug!("Content modified in tree viewer, recorded for undo");
                                }

                                // Update scroll offset for sync scrolling
                                tab.scroll_offset = output.scroll_offset;
                            }
                        } else {
                            // Markdown file: use the WYSIWYG MarkdownEditor
                            // Capture settings before mutable borrow
                            let max_line_width = self.state.settings.max_line_width;
                            let zen_max_column_width = self.state.settings.zen_max_column_width;
                            let paragraph_indent = self.state.settings.paragraph_indent;

                            if let Some(tab) = self.state.active_tab_mut() {
                                // Capture content and cursor before editing for undo support
                                let content_before = tab.content.clone();
                                let cursor_before = tab.cursors.primary().head;
                                
                                // Handle scroll sync: check for pending scroll ratio or offset
                                let pending_offset = tab.pending_scroll_offset.take();
                                let pending_ratio = tab.pending_scroll_ratio.take();

                                let editor_output = MarkdownEditor::new(&mut tab.content)
                                    .mode(EditorMode::Rendered)
                                    .font_size(font_size)
                                    .font_family(font_family.clone())
                                    .word_wrap(word_wrap)
                                    .theme(theme)
                                    .max_line_width(max_line_width) // Apply line width limit
                                    .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                    .paragraph_indent(paragraph_indent) // CJK paragraph indentation
                                    .id(egui::Id::new("main_editor_rendered"))
                                    .scroll_to_line(scroll_to_line)
                                    .pending_scroll_offset(pending_offset)
                                    .show(ui);

                                if editor_output.changed {
                                    // Record edit for undo/redo support
                                    tab.record_edit(content_before, cursor_before);
                                    // Mark content as edited for auto-save scheduling
                                    tab.mark_content_edited();
                                    debug!("Content modified in rendered editor, recorded for undo");
                                }

                                // Update cursor position from rendered editor
                                tab.cursor_position = editor_output.cursor_position;

                                // Update scroll metrics for sync scrolling
                                tab.scroll_offset = editor_output.scroll_offset;
                                tab.content_height = editor_output.content_height;
                                tab.viewport_height = editor_output.viewport_height;
                                
                                // Store line mappings for scroll sync (source_line → rendered_y)
                                tab.rendered_line_mappings = editor_output.line_mappings
                                    .iter()
                                    .map(|m| (m.start_line, m.end_line, m.rendered_y))
                                    .collect();
                                
                                // Handle pending scroll to line: convert to offset using FRESH line mappings
                                // This provides accurate content-based sync using interpolation
                                if let Some(target_line) = tab.pending_scroll_to_line.take() {
                                    if let Some(rendered_y) = Self::find_rendered_y_for_line_interpolated(
                                        &tab.rendered_line_mappings,
                                        target_line,
                                        editor_output.content_height,
                                    ) {
                                        tab.pending_scroll_offset = Some(rendered_y);
                                        debug!(
                                            "Converted line {} to rendered offset {:.1} (interpolated, {} mappings)",
                                            target_line, rendered_y, tab.rendered_line_mappings.len()
                                        );
                                        ui.ctx().request_repaint();
                                    } else {
                                        debug!(
                                            "No mapping for line {} ({} mappings), falling back to ratio",
                                            target_line, tab.rendered_line_mappings.len()
                                        );
                                        // Fallback: estimate based on line ratio
                                        let total_lines = tab.content.lines().count().max(1);
                                        let line_ratio = (target_line as f32 / total_lines as f32).clamp(0.0, 1.0);
                                        let max_scroll = (editor_output.content_height - editor_output.viewport_height).max(0.0);
                                        tab.pending_scroll_offset = Some(line_ratio * max_scroll);
                                        ui.ctx().request_repaint();
                                    }
                                }
                                
                                // Handle pending scroll ratio: convert to offset now that we have content_height
                                if let Some(ratio) = pending_ratio {
                                    let max_scroll = (editor_output.content_height - editor_output.viewport_height).max(0.0);
                                    if max_scroll > 0.0 {
                                        let target_offset = ratio * max_scroll;
                                        tab.pending_scroll_offset = Some(target_offset);
                                        debug!(
                                            "Converted scroll ratio {:.3} to offset {:.1} (content_height={}, viewport_height={})",
                                            ratio, target_offset, editor_output.content_height, editor_output.viewport_height
                                        );
                                        // Request repaint to apply the offset on next frame
                                        ui.ctx().request_repaint();
                                    }
                                }

                                // Update selection from focused element (for rendered mode formatting)
                                if let Some(focused) = editor_output.focused_element {
                                    // Only update selection if there's an actual text selection within the element
                                    if let Some((sel_start, sel_end)) = focused.selection {
                                        if sel_start != sel_end {
                                            // Actual selection within the focused element
                                            let abs_start = focused.start_char + sel_start;
                                            let abs_end = focused.start_char + sel_end;
                                            tab.selection = Some((abs_start, abs_end));
                                        } else {
                                            // Just cursor, no selection
                                            tab.selection = None;
                                        }
                                    } else {
                                        // No selection info
                                        tab.selection = None;
                                    }
                                } else {
                                    // No focused element
                                    tab.selection = None;
                                }
                            }
                        }
                    }
                }
            }
        });

        // Render dialogs
        self.render_dialogs(ctx);

        // ═══════════════════════════════════════════════════════════════════
        // Quick File Switcher Overlay (Ctrl+P)
        // ═══════════════════════════════════════════════════════════════════
        if self.quick_switcher.is_open() {
            if let Some(workspace) = &self.state.workspace {
                let all_files = workspace.all_files();
                let recent_files = &workspace.recent_files;

                let output = self.quick_switcher.show(
                    ctx,
                    &all_files,
                    recent_files,
                    &workspace.root_path,
                    is_dark,
                );

                // Handle file selection
                if let Some(file_path) = output.selected_file {
                    match self.state.open_file(file_path.clone()) {
                        Ok(_) => {
                            debug!("Opened file from quick switcher: {}", file_path.display());
                            // Add to workspace recent files
                            if let Some(workspace) = self.state.workspace_mut() {
                                workspace.add_recent_file(file_path);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to open file: {}", e);
                            self.state
                                .show_error(format!("Failed to open file:\n{}", e));
                        }
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // File Operation Dialog (New File, Rename, Delete, etc.)
        // ═══════════════════════════════════════════════════════════════════
        if let Some(mut dialog) = self.file_operation_dialog.take() {
            let result = dialog.show(ctx, is_dark);

            match result {
                FileOperationResult::None => {
                    // Dialog still open, put it back
                    self.file_operation_dialog = Some(dialog);
                }
                FileOperationResult::Cancelled => {
                    // Dialog was cancelled, do nothing
                    debug!("File operation dialog cancelled");
                }
                FileOperationResult::CreateFile(path) => {
                    self.handle_create_file(path);
                }
                FileOperationResult::CreateFolder(path) => {
                    self.handle_create_folder(path);
                }
                FileOperationResult::Rename { old, new } => {
                    self.handle_rename_file(old, new);
                }
                FileOperationResult::Delete(path) => {
                    self.handle_delete_file(path, Some(ctx));
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // Go to Line Dialog (Ctrl+G)
        // ═══════════════════════════════════════════════════════════════════
        if let Some(mut dialog) = self.state.ui.go_to_line_dialog.take() {
            let result = dialog.show(ctx, is_dark);

            match result {
                GoToLineResult::None => {
                    // Dialog still open, put it back
                    self.state.ui.go_to_line_dialog = Some(dialog);
                }
                GoToLineResult::Cancelled => {
                    // Dialog was cancelled, do nothing
                    debug!("Go to Line dialog cancelled");
                }
                GoToLineResult::GoToLine(target_line) => {
                    // Navigate to the specified line
                    self.handle_go_to_line(target_line);
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // Search in Files Panel (Ctrl+Shift+F)
        // ═══════════════════════════════════════════════════════════════════
        if self.search_panel.is_open() {
            if let Some(workspace) = &self.state.workspace {
                let workspace_root = workspace.root_path.clone();
                let hidden_patterns = workspace.hidden_patterns.clone();
                let all_files = workspace.all_files();

                let output = self.search_panel.show(ctx, &workspace_root, is_dark);

                // Trigger search when requested
                if output.should_search {
                    self.search_panel.search(&all_files, &hidden_patterns);
                }

                // Handle navigation to file
                if let Some(target) = output.navigate_to {
                    self.handle_search_navigation(target);
                }
            }
        }

        // Return deferred format action to be handled after editor has captured selection
        deferred_format_action
    }

    /// Handle the "File > Open" action.
    ///
    /// Opens a native file dialog allowing multiple file selection and loads
    /// each selected file into a new tab.
    fn handle_open_file(&mut self) {
        // Get the last open directory from recent files, if available
        let initial_dir = self
            .state
            .settings
            .recent_files
            .first()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        // Open the native file dialog (supports multiple selection)
        let paths = open_multiple_files_dialog(initial_dir.as_ref());

        if paths.is_empty() {
            debug!("File dialog cancelled");
            return;
        }

        let file_count = paths.len();
        let mut success_count = 0;
        let mut last_error: Option<String> = None;

        for path in paths {
            info!("Opening file: {}", path.display());
            match self.state.open_file(path.clone()) {
                Ok(tab_index) => {
                    success_count += 1;
                    // Check for auto-save recovery
                    self.check_auto_save_recovery(tab_index);
                }
                Err(e) => {
                    warn!("Failed to open file {}: {}", path.display(), e);
                    last_error = Some(format!("Failed to open {}:\n{}", path.display(), e));
                }
            }
        }

        // Show toast for multiple files opened
        if file_count > 1 && success_count > 0 {
            let time = self.get_app_time();
            self.state
                .show_toast(format!("Opened {} files", success_count), time, 2.0);
        }

        // Show error if any file failed to open
        if let Some(error) = last_error {
            self.state.show_error(error);
        }
    }

    /// Handle the "File > Save" action.
    ///
    /// Saves the current document to its existing file path.
    /// If the document has no path, triggers "Save As" instead.
    fn handle_save_file(&mut self) {
        // Check if the active tab has a path
        let has_path = self
            .state
            .active_tab()
            .map(|t| t.path.is_some())
            .unwrap_or(false);

        if has_path {
            // Save to existing path
            let path_display = self
                .state
                .active_tab()
                .and_then(|t| t.path.as_ref())
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            // Get tab ID before save for cleanup
            let tab_id = self.state.active_tab().map(|t| t.id);

            match self.state.save_active_tab() {
                Ok(_) => {
                    debug!("File saved successfully");
                    let time = self.get_app_time();
                    self.state
                        .show_toast(format!("Saved: {}", path_display), time, 3.0);
                    
                    // Clean up auto-save temp file after successful manual save
                    if let Some(id) = tab_id {
                        self.cleanup_auto_save_for_tab(id);
                    }
                    
                    // Trigger git status refresh after successful save
                    self.request_git_refresh();
                }
                Err(e) => {
                    warn!("Failed to save file: {}", e);
                    self.state
                        .show_error(format!("Failed to save file:\n{}", e));
                }
            }
        } else {
            // No path set, trigger Save As
            self.handle_save_as_file();
        }
    }

    /// Handle the "File > Save As" action.
    ///
    /// Opens a native save dialog and saves the document to the selected location.
    fn handle_save_as_file(&mut self) {
        // Get initial directory from current file or recent files
        let initial_dir = self
            .state
            .active_tab()
            .and_then(|t| t.path.as_ref())
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .or_else(|| {
                self.state
                    .settings
                    .recent_files
                    .first()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            });

        // Get default filename from current tab
        let default_name = self
            .state
            .active_tab()
            .and_then(|t| t.path.as_ref())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "untitled.md".to_string());

        // Open the native save dialog
        if let Some(path) = save_file_dialog(initial_dir.as_ref(), Some(&default_name)) {
            info!("Saving file as: {}", path.display());
            
            // Get old path and tab ID before save for cleanup
            let old_path = self.state.active_tab().and_then(|t| t.path.clone());
            let tab_id = self.state.active_tab().map(|t| t.id);

            match self.state.save_active_tab_as(path.clone()) {
                Ok(_) => {
                    let time = self.get_app_time();
                    self.state
                        .show_toast(format!("Saved: {}", path.display()), time, 3.0);
                    
                    // Clean up auto-save temp files after successful manual save
                    // (both old path and new path, in case they differ)
                    if let Some(id) = tab_id {
                        use crate::config::delete_auto_save;
                        // Clean up old path's auto-save
                        delete_auto_save(id, old_path.as_ref());
                        // Clean up new path's auto-save (in case it exists)
                        delete_auto_save(id, Some(&path));
                        debug!("Cleaned up auto-save temp files for tab {}", id);
                    }
                    
                    // Trigger git status refresh after successful save
                    self.request_git_refresh();
                }
                Err(e) => {
                    warn!("Failed to save file: {}", e);
                    self.state
                        .show_error(format!("Failed to save file:\n{}", e));
                }
            }
        } else {
            debug!("Save dialog cancelled");
        }
    }

    /// Handle the "File > Open Workspace" action.
    ///
    /// Opens a native folder dialog and switches to workspace mode.
    fn handle_open_workspace(&mut self) {
        use crate::files::dialogs::open_folder_dialog;

        // Get initial directory from recent workspaces or recent files
        let initial_dir = self
            .state
            .settings
            .recent_workspaces
            .first()
            .cloned()
            .or_else(|| {
                self.state
                    .settings
                    .recent_files
                    .first()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            });

        // Open the native folder dialog
        if let Some(folder_path) = open_folder_dialog(initial_dir.as_ref()) {
            info!("Opening workspace: {}", folder_path.display());
            match self.state.open_workspace(folder_path.clone()) {
                Ok(_) => {
                    let time = self.get_app_time();
                    let folder_name = folder_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("folder");
                    self.state
                        .show_toast(format!("Opened workspace: {}", folder_name), time, 2.5);
                    
                    // Immediately save session to persist the workspace path
                    self.force_session_save();
                }
                Err(e) => {
                    warn!("Failed to open workspace: {}", e);
                    self.state
                        .show_error(format!("Failed to open workspace:\n{}", e));
                }
            }
        } else {
            debug!("Open workspace dialog cancelled");
        }
    }

    /// Handle closing the current workspace.
    ///
    /// Returns to single-file mode and hides workspace UI.
    fn handle_close_workspace(&mut self) {
        if self.state.is_workspace_mode() {
            self.state.close_workspace();
            let time = self.get_app_time();
            self.state.show_toast("Workspace closed", time, 2.0);
            
            // Immediately save session to persist the mode change
            self.force_session_save();
        }
    }
    
    /// Force an immediate session save (bypasses throttling).
    ///
    /// Use this after important state changes like opening/closing workspaces
    /// to ensure the change is persisted immediately.
    fn force_session_save(&mut self) {
        use crate::config::save_crash_recovery_state;

        let workspace_info = if let Some(root) = self.state.workspace_root() {
            format!("Workspace({})", root.display())
        } else {
            "SingleFile".to_string()
        };
        debug!(
            "Force session save requested: app_mode={}",
            workspace_info
        );

        let mut session_state = self.state.capture_session_state();
        session_state.clean_shutdown = false; // This is a crash recovery snapshot
        self.inject_csv_delimiters(&mut session_state);

        if save_crash_recovery_state(&session_state) {
            self.session_save_throttle.record_save();
            debug!(
                "Forced session save completed successfully: app_mode={}",
                workspace_info
            );
        } else {
            warn!(
                "Failed to force session save: app_mode={}",
                workspace_info
            );
        }
    }

    /// Handle toggling the file tree panel visibility.
    fn handle_toggle_file_tree(&mut self) {
        if self.state.is_workspace_mode() {
            self.state.toggle_file_tree();
            let time = self.get_app_time();
            let msg = if self.state.should_show_file_tree() {
                "File tree shown"
            } else {
                "File tree hidden"
            };
            self.state.show_toast(msg, time, 1.5);
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state
                .show_toast("Open a folder first (📁 button)", time, 2.0);
        }
    }

    /// Handle opening the quick file switcher.
    fn handle_quick_open(&mut self) {
        if self.state.is_workspace_mode() {
            self.quick_switcher.toggle();
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state
                .show_toast("Open a folder first to use quick open", time, 2.0);
        }
    }

    /// Handle opening the search in files panel.
    fn handle_search_in_files(&mut self) {
        if self.state.is_workspace_mode() {
            self.search_panel.toggle();
            // Trigger search if panel is now open
            if self.search_panel.is_open() {
                if let Some(workspace) = &self.state.workspace {
                    let files = workspace.all_files();
                    self.search_panel.search(&files, &workspace.hidden_patterns);
                }
            }
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state
                .show_toast("Open a folder first to use search in files", time, 2.0);
        }
    }

    /// Handle navigation from a search-in-files result click.
    ///
    /// This opens the file (if not already open), scrolls to the match location,
    /// applies a transient highlight, and switches to Raw mode if necessary.
    fn handle_search_navigation(&mut self, target: SearchNavigationTarget) {
        let file_path = target.path.clone();

        // Open the file (or switch to existing tab)
        match self.state.open_file(file_path.clone()) {
            Ok(_) => {
                debug!(
                    "Opened file from search: {} at line {}, char offset {}",
                    file_path.display(),
                    target.line_number,
                    target.char_offset
                );

                // Get the active tab and apply navigation
                if let Some(tab) = self.state.active_tab_mut() {
                    // Switch to Raw mode if currently in Rendered mode
                    // (search results are based on raw text positions)
                    if tab.view_mode == ViewMode::Rendered {
                        tab.view_mode = ViewMode::Raw;
                        debug!("Switched to Raw mode for search navigation");
                    }

                    // Clear any existing transient highlight from previous navigations
                    tab.clear_transient_highlight();

                    // Set the transient highlight for the matched text
                    let highlight_end = target.char_offset + target.match_len;
                    tab.set_transient_highlight(target.char_offset, highlight_end);

                    // Set cursor position to the match location
                    tab.set_cursor(target.char_offset);

                    // Schedule scroll to the target line (editor will handle this)
                    self.pending_scroll_to_line = Some(target.line_number);

                    debug!(
                        "Set transient highlight at {}..{} and scroll to line {}",
                        target.char_offset, highlight_end, target.line_number
                    );
                }

                // Add to workspace recent files
                if let Some(workspace) = self.state.workspace_mut() {
                    workspace.add_recent_file(file_path);
                }
            }
            Err(e) => {
                warn!("Failed to open file from search: {}", e);
                self.state
                    .show_error(format!("Failed to open file:\n{}", e));
            }
        }
    }

    /// Handle file watcher events from the workspace.
    fn handle_file_watcher_events(&mut self) {
        use crate::workspaces::WorkspaceEvent;

        // Poll for new events
        self.state.poll_file_watcher();

        // Process any pending events
        let events = self.state.take_file_events();
        if events.is_empty() {
            return;
        }

        let mut need_tree_refresh = false;
        let mut modified_files: Vec<std::path::PathBuf> = Vec::new();

        for event in events {
            match event {
                WorkspaceEvent::FileCreated(path) => {
                    debug!("File created: {}", path.display());
                    need_tree_refresh = true;
                }
                WorkspaceEvent::FileDeleted(path) => {
                    debug!("File deleted: {}", path.display());
                    need_tree_refresh = true;

                    // Check if this file is open in a tab and mark it
                    for tab in self.state.tabs() {
                        if tab.path.as_ref() == Some(&path) {
                            // File was deleted externally - we could show a warning
                            // For now, just log it
                            warn!("Open file was deleted: {}", path.display());
                        }
                    }
                }
                WorkspaceEvent::FileModified(path) => {
                    debug!("File modified: {}", path.display());
                    // Check if this file is open in a tab
                    for tab in self.state.tabs() {
                        if tab.path.as_ref() == Some(&path) {
                            modified_files.push(path.clone());
                            break;
                        }
                    }
                }
                WorkspaceEvent::FileRenamed(old_path, new_path) => {
                    debug!(
                        "File renamed: {} -> {}",
                        old_path.display(),
                        new_path.display()
                    );
                    need_tree_refresh = true;
                }
                WorkspaceEvent::Error(msg) => {
                    warn!("File watcher error: {}", msg);
                }
            }
        }

        // Refresh file tree if needed
        if need_tree_refresh {
            self.state.refresh_workspace();
            // Also request git refresh since files changed
            self.request_git_refresh();
        }

        // Show toast for modified files
        if !modified_files.is_empty() {
            let time = self.get_app_time();
            let msg = if modified_files.len() == 1 {
                format!(
                    "File changed externally: {}",
                    modified_files[0]
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                )
            } else {
                format!("{} files changed externally", modified_files.len())
            };
            self.state.show_toast(msg, time, 3.0);
        }
    }

    /// Handle automatic Git status refresh.
    ///
    /// This method manages:
    /// - Refresh on window focus gained
    /// - Periodic refresh every 10 seconds when a workspace is open
    /// - Debounced refresh requests (e.g., after file save)
    fn handle_git_auto_refresh(&mut self, ctx: &egui::Context) {
        // Get window focus state
        let is_focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));

        // Update focus state and detect focus gained
        self.git_auto_refresh.update_focus(is_focused);

        // Check if git service is active (workspace with git repo)
        let git_active = self.state.git_service.is_open();

        // Tick the auto-refresh manager
        if self.git_auto_refresh.tick(git_active) {
            // Perform the actual refresh
            self.state.git_service.refresh_status();
            self.git_auto_refresh.mark_refreshed();
            trace!("Git status auto-refreshed");
        }
    }

    /// Request a Git status refresh.
    ///
    /// This triggers a debounced refresh - multiple rapid calls will be batched
    /// into a single refresh after a short delay (500ms).
    fn request_git_refresh(&mut self) {
        if self.state.git_service.is_open() {
            self.git_auto_refresh.request_refresh();
        }
    }

    /// Check if a file path has a supported image extension.
    fn is_supported_image(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp"
                )
            })
            .unwrap_or(false)
    }

    /// Get the assets directory for storing dropped images.
    ///
    /// Priority:
    /// 1. Relative to the current document's directory (if document is saved)
    /// 2. Workspace root (if in workspace mode)
    /// 3. Current working directory as fallback
    fn get_assets_dir(&self) -> std::path::PathBuf {
        // Try to get the current document's directory
        if let Some(tab) = self.state.active_tab() {
            if let Some(doc_path) = &tab.path {
                if let Some(parent) = doc_path.parent() {
                    return parent.join("assets");
                }
            }
        }

        // Fall back to workspace root
        if let Some(workspace_root) = self.state.workspace_root() {
            return workspace_root.join("assets");
        }

        // Last resort: current directory
        std::path::PathBuf::from("assets")
    }

    /// Generate a unique filename for a dropped image using timestamp.
    ///
    /// Format: YYYYMMDD-HHMMSS-originalname.ext
    fn generate_unique_image_filename(original_path: &std::path::Path) -> String {
        use std::time::SystemTime;

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                // Convert to local time components
                let secs = d.as_secs();
                // Simple timestamp format: YYYYMMDD-HHMMSS
                // Note: This uses UTC, but that's fine for uniqueness
                let days = secs / 86400;
                let time_of_day = secs % 86400;
                let hours = time_of_day / 3600;
                let minutes = (time_of_day % 3600) / 60;
                let seconds = time_of_day % 60;

                // Approximate year/month/day calculation (not accounting for leap years perfectly)
                let years_since_1970 = days / 365;
                let year = 1970 + years_since_1970;
                let remaining_days = days % 365;
                let month = (remaining_days / 30) + 1;
                let day = (remaining_days % 30) + 1;

                format!(
                    "{:04}{:02}{:02}-{:02}{:02}{:02}",
                    year, month, day, hours, minutes, seconds
                )
            })
            .unwrap_or_else(|_| "unknown".to_string());

        let original_name = original_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");

        let extension = original_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png");

        format!("{}-{}.{}", timestamp, original_name, extension)
    }

    /// Handle a dropped image file by copying it to assets and inserting markdown link.
    fn handle_dropped_image(&mut self, image_path: &std::path::Path) -> Result<(), String> {
        // Get the assets directory
        let assets_dir = self.get_assets_dir();

        // Create assets directory if it doesn't exist
        if !assets_dir.exists() {
            std::fs::create_dir_all(&assets_dir).map_err(|e| {
                format!(
                    "Failed to create assets directory '{}': {}",
                    assets_dir.display(),
                    e
                )
            })?;
            info!("Created assets directory: {}", assets_dir.display());
        }

        // Generate unique filename
        let new_filename = Self::generate_unique_image_filename(image_path);
        let dest_path = assets_dir.join(&new_filename);

        // Copy the image file
        std::fs::copy(image_path, &dest_path).map_err(|e| {
            format!(
                "Failed to copy image to '{}': {}",
                dest_path.display(),
                e
            )
        })?;
        info!(
            "Copied dropped image to: {} (from {})",
            dest_path.display(),
            image_path.display()
        );

        // Insert markdown link at cursor position in the active tab
        if let Some(tab) = self.state.active_tab_mut() {
            // Helper to convert char position to byte position
            let char_to_byte = |text: &str, char_idx: usize| -> usize {
                text.char_indices()
                    .nth(char_idx)
                    .map(|(byte_idx, _)| byte_idx)
                    .unwrap_or(text.len())
            };

            // Save state for undo
            let old_content = tab.content.clone();
            let old_cursor = tab.cursors.primary().head;

            // Get cursor position
            let cursor_char_pos = tab.cursors.primary().head;
            let cursor_byte = char_to_byte(&tab.content, cursor_char_pos);

            // Build markdown image link with relative path
            let markdown_link = format!("![](assets/{})", new_filename);
            let link_len = markdown_link.chars().count();

            // Insert at cursor position
            tab.content.insert_str(cursor_byte, &markdown_link);

            // Position cursor after the inserted link
            let new_cursor_pos = cursor_char_pos + link_len;
            tab.pending_cursor_restore = Some(new_cursor_pos);
            tab.cursors
                .set_single(crate::state::Selection::cursor(new_cursor_pos));
            tab.sync_cursor_from_primary();

            // Record for undo
            tab.record_edit(old_content, old_cursor);

            debug!(
                "Inserted image link '{}' at position {}",
                markdown_link, cursor_char_pos
            );
        }

        Ok(())
    }

    /// Handle files/folders dropped onto the application window.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if dropped_files.is_empty() {
            return;
        }

        // Categorize dropped items
        let mut folders: Vec<std::path::PathBuf> = Vec::new();
        let mut images: Vec<std::path::PathBuf> = Vec::new();
        let mut documents: Vec<std::path::PathBuf> = Vec::new();

        for path in dropped_files {
            if path.is_dir() {
                folders.push(path);
            } else if path.is_file() {
                if Self::is_supported_image(&path) {
                    images.push(path);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext.to_lowercase().as_str(),
                        "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "txt" | "csv" | "tsv" | "json" | "yaml" | "yml" | "toml"
                    ) {
                        documents.push(path);
                    }
                }
            }
        }

        // Priority 1: If a folder was dropped, open it as a workspace
        if let Some(folder) = folders.into_iter().next() {
            info!("Opening dropped folder as workspace: {}", folder.display());
            match self.state.open_workspace(folder.clone()) {
                Ok(_) => {
                    let time = self.get_app_time();
                    let folder_name = folder
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("folder");
                    self.state
                        .show_toast(format!("Opened workspace: {}", folder_name), time, 2.5);

                    // Immediately save session to persist the workspace path
                    self.force_session_save();
                }
                Err(e) => {
                    warn!("Failed to open workspace: {}", e);
                    self.state
                        .show_error(format!("Failed to open workspace:\n{}", e));
                }
            }
            return; // Prioritize folder over files
        }

        // Priority 2: Handle images (copy to assets and insert markdown links)
        let mut images_inserted = 0;
        for image_path in images {
            match self.handle_dropped_image(&image_path) {
                Ok(_) => {
                    images_inserted += 1;
                }
                Err(e) => {
                    warn!("Failed to handle dropped image: {}", e);
                    self.state.show_error(format!("Failed to add image:\n{}", e));
                }
            }
        }

        if images_inserted > 0 {
            let time = self.get_app_time();
            let msg = if images_inserted == 1 {
                "Image added to assets".to_string()
            } else {
                format!("{} images added to assets", images_inserted)
            };
            self.state.show_toast(msg, time, 2.5);
        }

        // Priority 3: Open document files in tabs
        for file in documents {
            match self.state.open_file(file.clone()) {
                Ok(_) => {
                    debug!("Opened dropped file: {}", file.display());
                    // Add to workspace recent files if in workspace mode
                    if let Some(workspace) = self.state.workspace_mut() {
                        workspace.add_recent_file(file);
                    }
                }
                Err(e) => {
                    warn!("Failed to open dropped file: {}", e);
                }
            }
        }
    }

    /// Handle file tree context menu actions.
    fn handle_file_tree_context_action(&mut self, action: FileTreeContextAction) {
        match action {
            FileTreeContextAction::NewFile(parent_path) => {
                self.file_operation_dialog = Some(FileOperationDialog::new_file(parent_path));
            }
            FileTreeContextAction::NewFolder(parent_path) => {
                self.file_operation_dialog = Some(FileOperationDialog::new_folder(parent_path));
            }
            FileTreeContextAction::Rename(path) => {
                self.file_operation_dialog = Some(FileOperationDialog::rename(path));
            }
            FileTreeContextAction::Delete(path) => {
                self.file_operation_dialog = Some(FileOperationDialog::delete(path));
            }
            FileTreeContextAction::RevealInExplorer(path) => {
                // Open the file's parent folder in the system file explorer
                let folder = if path.is_dir() {
                    path.clone()
                } else {
                    path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
                };

                if let Err(e) = open::that(&folder) {
                    warn!("Failed to reveal in explorer: {}", e);
                    self.state
                        .show_error(format!("Failed to open explorer:\n{}", e));
                } else {
                    debug!("Revealed in explorer: {}", folder.display());
                }
            }
            FileTreeContextAction::Refresh => {
                self.state.refresh_workspace();
                let time = self.get_app_time();
                self.state.show_toast("File tree refreshed", time, 1.5);
            }
        }
    }

    /// Handle creating a new file.
    fn handle_create_file(&mut self, path: std::path::PathBuf) {
        use std::fs::File;
        use std::io::Write;

        // Create the file with default markdown content
        let default_content = format!(
            "# {}\n\n",
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
        );

        match File::create(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(default_content.as_bytes()) {
                    warn!("Failed to write to new file: {}", e);
                    self.state
                        .show_error(format!("Failed to write file:\n{}", e));
                    return;
                }

                info!("Created new file: {}", path.display());
                let time = self.get_app_time();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                self.state
                    .show_toast(format!("Created: {}", name), time, 2.0);

                // Refresh file tree
                self.state.refresh_workspace();

                // Open the new file in a tab
                if let Err(e) = self.state.open_file(path.clone()) {
                    warn!("Failed to open new file: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to create file: {}", e);
                self.state
                    .show_error(format!("Failed to create file:\n{}", e));
            }
        }
    }

    /// Handle creating a new folder.
    fn handle_create_folder(&mut self, path: std::path::PathBuf) {
        match std::fs::create_dir(&path) {
            Ok(_) => {
                info!("Created new folder: {}", path.display());
                let time = self.get_app_time();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder");
                self.state
                    .show_toast(format!("Created: {}", name), time, 2.0);

                // Refresh file tree
                self.state.refresh_workspace();
            }
            Err(e) => {
                warn!("Failed to create folder: {}", e);
                self.state
                    .show_error(format!("Failed to create folder:\n{}", e));
            }
        }
    }

    /// Handle renaming a file or folder.
    fn handle_rename_file(&mut self, old_path: std::path::PathBuf, new_path: std::path::PathBuf) {
        match std::fs::rename(&old_path, &new_path) {
            Ok(_) => {
                info!("Renamed: {} -> {}", old_path.display(), new_path.display());
                let time = self.get_app_time();
                let new_name = new_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("item");
                self.state
                    .show_toast(format!("Renamed to: {}", new_name), time, 2.0);

                // Update any open tabs with the old path
                for i in 0..self.state.tab_count() {
                    if let Some(tab) = self.state.tab_mut(i) {
                        if tab.path.as_ref() == Some(&old_path) {
                            tab.path = Some(new_path.clone());
                            break;
                        }
                    }
                }

                // Refresh file tree
                self.state.refresh_workspace();
            }
            Err(e) => {
                warn!("Failed to rename: {}", e);
                self.state.show_error(format!("Failed to rename:\n{}", e));
            }
        }
    }

    /// Handle deleting a file or folder.
    ///
    /// # Parameters
    /// - `path` - Path to the file or folder to delete
    /// - `ctx` - Optional egui Context for cleaning up tab state memory
    fn handle_delete_file(&mut self, path: std::path::PathBuf, ctx: Option<&egui::Context>) {
        let is_dir = path.is_dir();
        let result = if is_dir {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };

        match result {
            Ok(_) => {
                info!("Deleted: {}", path.display());
                let time = self.get_app_time();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("item");
                self.state
                    .show_toast(format!("Deleted: {}", name), time, 2.0);

                // Close any tabs with this path
                // Collect both index and tab_id for cleanup after closing
                let tabs_to_close: Vec<(usize, usize)> = self
                    .state
                    .tabs()
                    .iter()
                    .enumerate()
                    .filter(|(_, tab)| {
                        if let Some(tab_path) = &tab.path {
                            tab_path == &path || tab_path.starts_with(&path)
                        } else {
                            false
                        }
                    })
                    .map(|(i, tab)| (i, tab.id))
                    .collect();

                // Close tabs in reverse order to maintain indices
                for &(index, tab_id) in tabs_to_close.iter().rev() {
                    self.state.close_tab(index);
                    self.cleanup_tab_state(tab_id, ctx);
                }

                // Refresh file tree
                self.state.refresh_workspace();
            }
            Err(e) => {
                warn!("Failed to delete: {}", e);
                self.state.show_error(format!("Failed to delete:\n{}", e));
            }
        }
    }

    /// Consume undo/redo keyboard events BEFORE rendering.
    ///
    /// This MUST be called before render_ui() to prevent egui's TextEdit from
    /// processing Ctrl+Z/Y with its built-in undo functionality. TextEdit has
    /// internal undo that would conflict with our custom undo system.
    ///
    /// By consuming these keys before the TextEdit is rendered, we ensure only
    /// our undo system handles the events.
    fn consume_undo_redo_keys(&mut self, ctx: &egui::Context) {
        let consumed_action: Option<bool> = ctx.input_mut(|i| {
            // Cmd+Shift+Z (macOS) / Ctrl+Shift+Z (Win/Linux): Redo (check first since it's more specific)
            if i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z) {
                debug!("Keyboard shortcut: {}+Shift+Z (Redo) - consumed before render", modifier_symbol());
                return Some(false); // false = redo
            }
            // Cmd+Z (macOS) / Ctrl+Z (Win/Linux): Undo
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z) {
                debug!("Keyboard shortcut: {}+Z (Undo) - consumed before render", modifier_symbol());
                return Some(true); // true = undo
            }
            // Cmd+Y (macOS) / Ctrl+Y (Win/Linux): Redo
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y) {
                debug!("Keyboard shortcut: {}+Y (Redo) - consumed before render", modifier_symbol());
                return Some(false); // false = redo
            }
            None
        });
        
        // If undo/redo was consumed, handle it
        if let Some(is_undo) = consumed_action {
            if is_undo {
                self.handle_undo();
            } else {
                self.handle_redo();
            }
        }
    }

    /// Filter out Event::Cut when nothing is selected to prevent egui bug.
    ///
    /// egui's TextEdit has a bug where Ctrl+X with no selection cuts the entire
    /// document. This happens because eframe generates Event::Cut events which
    /// TextEdit processes. We filter out these events when there's no selection.
    fn filter_cut_event_if_no_selection(&mut self, ctx: &egui::Context) {
        // Check if there's a selection in the active tab
        let has_selection = self.state.active_tab()
            .map(|tab| tab.cursors.primary().is_selection())
            .unwrap_or(false);
        
        // If no selection, filter out Event::Cut to prevent egui from cutting everything
        if !has_selection {
            ctx.input_mut(|i| {
                let had_cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut));
                i.events.retain(|e| !matches!(e, egui::Event::Cut));
                if had_cut {
                    debug!("Event::Cut filtered out - no selection");
                }
            });
        }
    }

    /// Consume Alt+Arrow keys BEFORE render to prevent TextEdit from processing them.
    /// This must be called before the editor widget is rendered.
    /// Returns the direction to move (-1 for up, 1 for down) if a move was requested.
    fn consume_move_line_keys(&mut self, ctx: &egui::Context) -> Option<isize> {
        ctx.input_mut(|i| {
            // Alt+Up: Move line up
            if i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp) {
                debug!("Keyboard shortcut: Alt+Up (Move Line Up) - consumed before render");
                return Some(-1);
            }
            // Alt+Down: Move line down
            if i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown) {
                debug!("Keyboard shortcut: Alt+Down (Move Line Down) - consumed before render");
                return Some(1);
            }
            None
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Smart Paste for Links and Images
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if a string looks like a URL.
    ///
    /// Returns true for strings starting with common URL schemes:
    /// - `http://` or `https://`
    /// - Other schemes like `ftp://`, `file://`, etc.
    fn is_url(s: &str) -> bool {
        let s = s.trim();
        if s.is_empty() {
            return false;
        }

        // Check for common URL schemes
        if s.starts_with("http://") || s.starts_with("https://") {
            return true;
        }

        // Check for other valid URL schemes (alphanumeric + some chars, followed by ://)
        // Examples: ftp://, file://, mailto:, data:
        if let Some(colon_pos) = s.find(':') {
            let scheme = &s[..colon_pos];
            // Scheme must be alphanumeric or contain +, -, .
            // and must be followed by //
            if !scheme.is_empty()
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
                && scheme.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            {
                // Check for :// pattern
                if s.len() > colon_pos + 2 && &s[colon_pos..colon_pos + 3] == "://" {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a URL points to an image based on file extension.
    ///
    /// Checks for common image extensions: .png, .jpg, .jpeg, .gif, .webp, .svg, .bmp
    /// The check is case-insensitive and handles URLs with query strings.
    fn is_image_url(s: &str) -> bool {
        if !Self::is_url(s) {
            return false;
        }

        let s = s.trim();

        // Remove query string and fragment for extension check
        let path = s.split('?').next().unwrap_or(s);
        let path = path.split('#').next().unwrap_or(path);

        // Get the extension (case-insensitive)
        let path_lower = path.to_lowercase();
        
        path_lower.ends_with(".png")
            || path_lower.ends_with(".jpg")
            || path_lower.ends_with(".jpeg")
            || path_lower.ends_with(".gif")
            || path_lower.ends_with(".webp")
            || path_lower.ends_with(".svg")
            || path_lower.ends_with(".bmp")
            || path_lower.ends_with(".ico")
            || path_lower.ends_with(".tiff")
            || path_lower.ends_with(".tif")
    }

    /// Consume paste events BEFORE render to implement smart paste behavior.
    ///
    /// Smart paste transforms paste behavior based on context:
    /// - Pasting a URL with text selected: Creates markdown link `[selected](url)`
    /// - Pasting an image URL with no selection: Creates markdown image `![](url)`
    /// - Otherwise: Normal paste behavior
    ///
    /// Returns true if a paste event was consumed and handled with smart behavior.
    fn consume_smart_paste(&mut self, ctx: &egui::Context) -> bool {
        let Some(tab) = self.state.active_tab_mut() else {
            return false;
        };

        // Get cursor/selection info upfront
        let primary = tab.cursors.primary();
        let cursor_char_pos = primary.head;
        let has_selection = primary.is_selection();
        let selection_range = if has_selection { Some(primary.range()) } else { None };
        let content = tab.content.clone();

        // Helper to convert char position to byte position
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        // Scan for paste events
        #[derive(Debug)]
        enum SmartPasteAction {
            /// Create markdown link: [selected_text](url)
            CreateLink { url: String, selected_text: String },
            /// Create markdown image: ![](url)
            CreateImage { url: String },
        }

        let action: Option<(usize, SmartPasteAction)> = ctx.input(|input| {
            for (idx, event) in input.events.iter().enumerate() {
                if let egui::Event::Paste(pasted_text) = event {
                    let trimmed = pasted_text.trim();

                    // Case 1: URL pasted with text selected → create markdown link
                    if has_selection && Self::is_url(trimmed) {
                        let (start_char, end_char) = selection_range.unwrap();
                        let start_byte = char_to_byte(&content, start_char);
                        let end_byte = char_to_byte(&content, end_char);
                        let selected_text = content[start_byte..end_byte].to_string();

                        return Some((idx, SmartPasteAction::CreateLink {
                            url: trimmed.to_string(),
                            selected_text,
                        }));
                    }

                    // Case 2: Image URL pasted with no selection → create markdown image
                    if !has_selection && Self::is_image_url(trimmed) {
                        return Some((idx, SmartPasteAction::CreateImage {
                            url: trimmed.to_string(),
                        }));
                    }

                    // Case 3: Regular URL with no selection → let normal paste handle it
                    // Case 4: Non-URL paste → let normal paste handle it
                }
            }
            None
        });

        // If we found an action, consume the event and apply it
        if let Some((event_idx, action)) = action {
            // Remove the paste event to prevent TextEdit from handling it
            ctx.input_mut(|input| {
                input.events.remove(event_idx);
            });

            // Get mutable access to tab again
            let tab = self.state.active_tab_mut().unwrap();
            let old_content = tab.content.clone();
            let old_cursor = tab.cursors.primary().head;

            match action {
                SmartPasteAction::CreateLink { url, selected_text } => {
                    let (start_char, end_char) = selection_range.unwrap();
                    let start_byte = char_to_byte(&tab.content, start_char);
                    let end_byte = char_to_byte(&tab.content, end_char);

                    // Build markdown link: [selected_text](url)
                    let link = format!("[{}]({})", selected_text, url);
                    let link_len = link.chars().count();

                    // Replace selection with link
                    tab.content.replace_range(start_byte..end_byte, &link);

                    // Position cursor after the link
                    let new_cursor_pos = start_char + link_len;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(crate::state::Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    // Record for undo
                    tab.record_edit(old_content, old_cursor);

                    debug!(
                        "Smart paste: Created link [{}]({}) at position {}",
                        selected_text, url, start_char
                    );
                }
                SmartPasteAction::CreateImage { url } => {
                    let cursor_byte = char_to_byte(&tab.content, cursor_char_pos);

                    // Build markdown image: ![](url)
                    let image = format!("![]({})", url);
                    let image_len = image.chars().count();

                    // Insert at cursor position
                    tab.content.insert_str(cursor_byte, &image);

                    // Position cursor after the image
                    let new_cursor_pos = cursor_char_pos + image_len;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(crate::state::Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    // Record for undo
                    tab.record_edit(old_content, old_cursor);

                    debug!(
                        "Smart paste: Created image ![](url) with url='{}' at position {}",
                        url, cursor_char_pos
                    );
                }
            }

            return true;
        }

        false
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Auto-close Brackets & Quotes
    // ─────────────────────────────────────────────────────────────────────────

    /// Get the closing character for an opener, if it's a valid opener.
    fn get_closing_bracket(opener: char) -> Option<char> {
        match opener {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            '`' => Some('`'),
            _ => None,
        }
    }

    /// Check if a character is a closing bracket/quote.
    fn is_closing_bracket(ch: char) -> bool {
        matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
    }

    /// Handle auto-close brackets BEFORE render.
    ///
    /// This handles two cases that require consuming input events before TextEdit:
    /// 1. Skip-over: When typing a closer and the next character is the same closer,
    ///    move cursor forward instead of inserting a duplicate.
    /// 2. Selection wrapping: When typing an opener with text selected,
    ///    wrap the selection with the bracket pair.
    ///
    /// Returns true if an event was consumed and handled.
    fn handle_auto_close_pre_render(&mut self, ctx: &egui::Context) -> bool {
        if !self.state.settings.auto_close_brackets {
            return false;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return false;
        };

        // Get cursor info upfront to avoid borrow issues
        let primary = tab.cursors.primary();
        let cursor_char_pos = primary.head;
        let has_selection = primary.is_selection();
        let selection_range = if has_selection { Some(primary.range()) } else { None };

        // Get content for analysis
        let content = tab.content.clone();

        // Helper to convert char position to byte position
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        // First, check input events to determine what action to take (if any)
        #[derive(Debug)]
        enum AutoCloseAction {
            WrapSelection { opener: char, closer: char },
            SkipOver { closer: char },
        }

        let action: Option<(usize, AutoCloseAction)> = ctx.input(|input| {
            for (idx, event) in input.events.iter().enumerate() {
                if let egui::Event::Text(text) = event {
                    // Only handle single-character input
                    if text.chars().count() != 1 {
                        continue;
                    }

                    let ch = text.chars().next().unwrap();

                    // Case 1: Selection wrapping with opener
                    if has_selection {
                        if let Some(closer) = Self::get_closing_bracket(ch) {
                            return Some((idx, AutoCloseAction::WrapSelection { opener: ch, closer }));
                        }
                    }

                    // Case 2: Skip-over for closing brackets
                    if !has_selection && Self::is_closing_bracket(ch) {
                        // Check if the next character is the same closer
                        let cursor_byte = char_to_byte(&content, cursor_char_pos);
                        let next_char = content[cursor_byte..].chars().next();

                        if next_char == Some(ch) {
                            return Some((idx, AutoCloseAction::SkipOver { closer: ch }));
                        }
                    }
                }
            }
            None
        });

        // If we found an action, consume the event and apply it
        if let Some((event_idx, action)) = action {
            // Remove the event first
            ctx.input_mut(|input| {
                input.events.remove(event_idx);
            });

            // Get mutable tab reference again
            let tab = self.state.active_tab_mut().unwrap();

            match action {
                AutoCloseAction::WrapSelection { opener, closer } => {
                    let (start_char, end_char) = selection_range.unwrap();
                    let start_byte = char_to_byte(&tab.content, start_char);
                    let end_byte = char_to_byte(&tab.content, end_char);

                    // Get selected text
                    let selected_text = tab.content[start_byte..end_byte].to_string();
                    let selected_len = selected_text.chars().count();

                    // Save for undo
                    let old_content = tab.content.clone();
                    let old_cursor = cursor_char_pos;

                    // Build wrapped text: opener + selected + closer
                    let wrapped = format!("{}{}{}", opener, selected_text, closer);

                    // Replace selection with wrapped text
                    tab.content.replace_range(start_byte..end_byte, &wrapped);

                    // Position cursor after the closing bracket
                    let new_cursor_pos = start_char + 1 + selected_len + 1;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    // Record for undo
                    tab.record_edit(old_content, old_cursor);

                    debug!("Auto-close: Wrapped selection '{}' with {}...{}", 
                           selected_text, opener, closer);
                }
                AutoCloseAction::SkipOver { closer } => {
                    // Just move cursor forward, don't insert
                    let new_cursor_pos = cursor_char_pos + 1;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    debug!("Auto-close: Skip-over for '{}'", closer);
                }
            }

            return true;
        }

        false
    }

    /// Handle auto-close brackets AFTER render.
    ///
    /// This handles auto-pair insertion: When an opener was just typed (no selection),
    /// insert the closing bracket immediately after and position cursor between them.
    ///
    /// This runs after TextEdit has processed input, so we detect what was just typed
    /// by comparing the current state with the pre-render snapshot.
    fn handle_auto_close_post_render(
        &mut self,
        pre_render_content: &str,
        _pre_render_cursor: usize,
    ) {
        if !self.state.settings.auto_close_brackets {
            return;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Check if exactly one character was inserted at the cursor position
        let content_len_diff = tab.content.chars().count() as isize
            - pre_render_content.chars().count() as isize;
        
        if content_len_diff != 1 {
            return; // Not a single character insertion
        }

        // Get current cursor position (should be after the just-typed character)
        let cursor_char_pos = tab.cursors.primary().head;
        
        // The just-typed character is at cursor_pos - 1
        if cursor_char_pos == 0 {
            return;
        }

        // Helper to convert char position to byte position
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        let prev_char_byte = char_to_byte(&tab.content, cursor_char_pos - 1);
        let cursor_byte = char_to_byte(&tab.content, cursor_char_pos);
        
        let just_typed = tab.content[prev_char_byte..cursor_byte].chars().next();
        
        if let Some(opener) = just_typed {
            if let Some(closer) = Self::get_closing_bracket(opener) {
                // For quotes, check context to avoid unwanted auto-close
                // Don't auto-close if the character before the opener is alphanumeric
                // (e.g., don't auto-close after typing can't -> can't')
                if matches!(opener, '"' | '\'' | '`') {
                    if cursor_char_pos >= 2 {
                        let prev_prev_byte = char_to_byte(&tab.content, cursor_char_pos - 2);
                        let prev_char = tab.content[prev_prev_byte..prev_char_byte].chars().next();
                        if let Some(c) = prev_char {
                            if c.is_alphanumeric() {
                                return; // Don't auto-close after alphanumeric
                            }
                        }
                    }
                }

                // Insert the closing bracket at cursor position
                tab.content.insert(cursor_byte, closer);

                // Keep cursor between the brackets (position hasn't changed)
                // TextEdit will update, but we want cursor to stay where it is
                tab.pending_cursor_restore = Some(cursor_char_pos);

                debug!("Auto-close: Inserted '{}' after '{}'", closer, opener);
            }
        }
    }

    /// Handle keyboard shortcuts.
    ///
    /// Processes global keyboard shortcuts:
    /// - Ctrl+S: Save current file
    /// - Ctrl+Shift+S: Save As
    /// - Ctrl+O: Open file
    /// - Ctrl+N: New file
    /// - Ctrl+T: New tab
    /// - Ctrl+W: Close current tab
    /// - Ctrl+Tab: Next tab
    /// - Ctrl+Shift+Tab: Previous tab
    ///
    /// Note: Undo/Redo (Ctrl+Z/Y) are handled separately in consume_undo_redo_keys()
    /// which must be called BEFORE render to prevent TextEdit from processing them.
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Get keyboard shortcuts configuration
        let shortcuts = self.state.settings.keyboard_shortcuts.clone();

        ctx.input(|i| {
            // Helper macro to check if a shortcut matches
            macro_rules! check_shortcut {
                ($cmd:expr, $action:expr) => {
                    if shortcuts.get($cmd).matches(i) {
                        debug!("Keyboard shortcut: {} ({})", shortcuts.get($cmd).display_string(), $cmd.display_name());
                        return Some($action);
                    }
                };
            }

            // Check shortcuts in order (more specific shortcuts first)
            // File operations
            check_shortcut!(ShortcutCommand::SaveAs, KeyboardAction::SaveAs);
            check_shortcut!(ShortcutCommand::Save, KeyboardAction::Save);
            check_shortcut!(ShortcutCommand::Open, KeyboardAction::Open);
            check_shortcut!(ShortcutCommand::New, KeyboardAction::New);
            check_shortcut!(ShortcutCommand::NewTab, KeyboardAction::NewTab);
            check_shortcut!(ShortcutCommand::CloseTab, KeyboardAction::CloseTab);

            // Navigation - check more specific shortcuts first
            check_shortcut!(ShortcutCommand::PrevTab, KeyboardAction::PrevTab);
            check_shortcut!(ShortcutCommand::NextTab, KeyboardAction::NextTab);
            check_shortcut!(ShortcutCommand::GoToLine, KeyboardAction::GoToLine);
            check_shortcut!(ShortcutCommand::QuickOpen, KeyboardAction::QuickOpen);

            // View
            check_shortcut!(ShortcutCommand::ToggleViewMode, KeyboardAction::ToggleViewMode);
            check_shortcut!(ShortcutCommand::CycleTheme, KeyboardAction::CycleTheme);
            check_shortcut!(ShortcutCommand::ToggleZenMode, KeyboardAction::ToggleZenMode);
            check_shortcut!(ShortcutCommand::ToggleFullscreen, KeyboardAction::ToggleFullscreen);
            check_shortcut!(ShortcutCommand::ToggleOutline, KeyboardAction::ToggleOutline);
            check_shortcut!(ShortcutCommand::ToggleFileTree, KeyboardAction::ToggleFileTree);
            check_shortcut!(ShortcutCommand::TogglePipeline, KeyboardAction::TogglePipeline);

            // Edit - note: Undo/Redo handled separately, MoveLineUp/Down handled separately
            check_shortcut!(ShortcutCommand::DeleteLine, KeyboardAction::DeleteLine);
            check_shortcut!(ShortcutCommand::DuplicateLine, KeyboardAction::DuplicateLine);
            check_shortcut!(ShortcutCommand::SelectNextOccurrence, KeyboardAction::SelectNextOccurrence);

            // Search
            check_shortcut!(ShortcutCommand::SearchInFiles, KeyboardAction::SearchInFiles);
            check_shortcut!(ShortcutCommand::FindReplace, KeyboardAction::OpenFindReplace);
            check_shortcut!(ShortcutCommand::Find, KeyboardAction::OpenFind);
            check_shortcut!(ShortcutCommand::FindNext, KeyboardAction::FindNext);
            check_shortcut!(ShortcutCommand::FindPrev, KeyboardAction::FindPrev);

            // Formatting - check more specific (Shift) shortcuts first
            check_shortcut!(ShortcutCommand::FormatBulletList, KeyboardAction::Format(MarkdownFormatCommand::BulletList));
            check_shortcut!(ShortcutCommand::FormatNumberedList, KeyboardAction::Format(MarkdownFormatCommand::NumberedList));
            check_shortcut!(ShortcutCommand::FormatCodeBlock, KeyboardAction::Format(MarkdownFormatCommand::CodeBlock));
            check_shortcut!(ShortcutCommand::FormatImage, KeyboardAction::Format(MarkdownFormatCommand::Image));
            check_shortcut!(ShortcutCommand::FormatBold, KeyboardAction::Format(MarkdownFormatCommand::Bold));
            check_shortcut!(ShortcutCommand::FormatItalic, KeyboardAction::Format(MarkdownFormatCommand::Italic));
            check_shortcut!(ShortcutCommand::FormatLink, KeyboardAction::Format(MarkdownFormatCommand::Link));
            check_shortcut!(ShortcutCommand::FormatBlockquote, KeyboardAction::Format(MarkdownFormatCommand::Blockquote));
            check_shortcut!(ShortcutCommand::FormatInlineCode, KeyboardAction::Format(MarkdownFormatCommand::InlineCode));
            check_shortcut!(ShortcutCommand::FormatHeading1, KeyboardAction::Format(MarkdownFormatCommand::Heading(1)));
            check_shortcut!(ShortcutCommand::FormatHeading2, KeyboardAction::Format(MarkdownFormatCommand::Heading(2)));
            check_shortcut!(ShortcutCommand::FormatHeading3, KeyboardAction::Format(MarkdownFormatCommand::Heading(3)));
            check_shortcut!(ShortcutCommand::FormatHeading4, KeyboardAction::Format(MarkdownFormatCommand::Heading(4)));
            check_shortcut!(ShortcutCommand::FormatHeading5, KeyboardAction::Format(MarkdownFormatCommand::Heading(5)));
            check_shortcut!(ShortcutCommand::FormatHeading6, KeyboardAction::Format(MarkdownFormatCommand::Heading(6)));

            // Folding
            check_shortcut!(ShortcutCommand::FoldAll, KeyboardAction::FoldAll);
            check_shortcut!(ShortcutCommand::UnfoldAll, KeyboardAction::UnfoldAll);
            check_shortcut!(ShortcutCommand::ToggleFoldAtCursor, KeyboardAction::ToggleFoldAtCursor);

            // Other
            check_shortcut!(ShortcutCommand::OpenSettings, KeyboardAction::OpenSettings);
            check_shortcut!(ShortcutCommand::OpenAbout, KeyboardAction::OpenAbout);
            check_shortcut!(ShortcutCommand::ExportHtml, KeyboardAction::ExportHtml);
            check_shortcut!(ShortcutCommand::InsertToc, KeyboardAction::InsertToc);

            // Escape: Exit multi-cursor mode or close find panel (always hardcoded)
            if i.key_pressed(egui::Key::Escape) {
                debug!("Keyboard shortcut: Escape");
                return Some(KeyboardAction::ExitMultiCursor);
            }

            None
        })
        .map(|action| match action {
            KeyboardAction::Save => self.handle_save_file(),
            KeyboardAction::SaveAs => self.handle_save_as_file(),
            KeyboardAction::Open => self.handle_open_file(),
            KeyboardAction::New => {
                self.state.new_tab();
            }
            KeyboardAction::NewTab => {
                self.state.new_tab();
            }
            KeyboardAction::CloseTab => {
                self.handle_close_current_tab(ctx);
            }
            KeyboardAction::NextTab => {
                self.handle_next_tab();
            }
            KeyboardAction::PrevTab => {
                self.handle_prev_tab();
            }
            KeyboardAction::ToggleViewMode => {
                self.handle_toggle_view_mode();
            }
            KeyboardAction::CycleTheme => {
                self.handle_cycle_theme(ctx);
            }
            KeyboardAction::OpenSettings => {
                self.state.toggle_settings();
            }
            KeyboardAction::OpenAbout => {
                self.state.toggle_about();
            }
            KeyboardAction::OpenFind => {
                self.handle_open_find(false);
            }
            KeyboardAction::OpenFindReplace => {
                self.handle_open_find(true);
            }
            KeyboardAction::FindNext => {
                self.handle_find_next();
            }
            KeyboardAction::FindPrev => {
                self.handle_find_prev();
            }
            KeyboardAction::Format(cmd) => {
                self.handle_format_command(cmd);
            }
            KeyboardAction::ToggleOutline => {
                self.handle_toggle_outline();
            }
            KeyboardAction::ToggleFileTree => {
                self.handle_toggle_file_tree();
            }
            KeyboardAction::QuickOpen => {
                self.handle_quick_open();
            }
            KeyboardAction::SearchInFiles => {
                self.handle_search_in_files();
            }
            KeyboardAction::ExportHtml => {
                self.handle_export_html(ctx);
            }
            KeyboardAction::SelectNextOccurrence => {
                self.handle_select_next_occurrence();
            }
            KeyboardAction::ExitMultiCursor => {
                // Priority order for Escape key:
                // 1. Exit fullscreen mode if active
                // 2. Exit multi-cursor mode if active
                // 3. Close find/replace panel
                let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if is_fullscreen {
                    // Exit fullscreen mode
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    let time = self.get_app_time();
                    self.state.show_toast("Exited fullscreen mode", time, 1.5);
                    info!("Exited fullscreen mode via Escape");
                } else if let Some(tab) = self.state.active_tab_mut() {
                    if tab.has_multiple_cursors() {
                        debug!("Exiting multi-cursor mode");
                        tab.exit_multi_cursor_mode();
                    } else if self.state.ui.show_find_replace {
                        self.state.ui.show_find_replace = false;
                    }
                } else if self.state.ui.show_find_replace {
                    self.state.ui.show_find_replace = false;
                }
            }
            KeyboardAction::ToggleZenMode => {
                self.handle_toggle_zen_mode();
            }
            KeyboardAction::ToggleFullscreen => {
                self.handle_toggle_fullscreen(ctx);
            }
            KeyboardAction::FoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.fold_all();
                        debug!("Folded all regions");
                    }
                }
            }
            KeyboardAction::UnfoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.unfold_all();
                        debug!("Unfolded all regions");
                    }
                }
            }
            KeyboardAction::ToggleFoldAtCursor => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        // Convert cursor position to line number (0-indexed)
                        let cursor_line = tab.cursor_position.0;
                        tab.toggle_fold_at_line(cursor_line);
                    }
                }
            }
            KeyboardAction::TogglePipeline => {
                self.handle_toggle_pipeline();
            }
            KeyboardAction::GoToLine => {
                self.handle_open_go_to_line();
            }
            KeyboardAction::DuplicateLine => {
                self.handle_duplicate_line();
            }
            KeyboardAction::DeleteLine => {
                self.handle_delete_line();
            }
            KeyboardAction::InsertToc => {
                self.handle_insert_toc();
            }
        });
    }

    /// Handle closing the current tab (with unsaved prompt if needed).
    fn handle_close_current_tab(&mut self, ctx: &egui::Context) {
        let index = self.state.active_tab_index();
        // Get tab_id before closing for viewer state cleanup
        let tab_id = self.state.tabs().get(index).map(|t| t.id);
        self.state.close_tab(index);
        if let Some(id) = tab_id {
            self.cleanup_tab_state(id, Some(ctx));
        }
    }

    /// Switch to the next tab (cycles to first if at end).
    fn handle_next_tab(&mut self) {
        let count = self.state.tab_count();
        if count > 1 {
            let current = self.state.active_tab_index();
            let next = (current + 1) % count;
            self.state.set_active_tab(next);
        }
    }

    /// Switch to the previous tab (cycles to last if at beginning).
    fn handle_prev_tab(&mut self) {
        let count = self.state.tab_count();
        if count > 1 {
            let current = self.state.active_tab_index();
            let prev = if current == 0 { count - 1 } else { current - 1 };
            self.state.set_active_tab(prev);
        }
    }

    /// Toggle view modes for the active tab.
    ///
    /// For markdown files: cycles Raw → Split → Rendered → Raw
    /// For structured files (JSON, YAML, TOML): cycles Raw ↔ Rendered (no Split mode)
    ///
    /// When sync scrolling is enabled, this calculates the corresponding scroll
    /// position in the target mode using line-to-position mapping for accuracy.
    fn handle_toggle_view_mode(&mut self) {
        // Get sync scroll setting and file type before mutable borrow
        let sync_enabled = self.state.settings.sync_scroll_enabled;
        let file_type = self.state.active_tab()
            .and_then(|t| t.path.as_ref())
            .map(|p| FileType::from_path(p))
            .unwrap_or(FileType::Unknown);
        // Structured (JSON/YAML/TOML) files don't support Split mode
        // CSV/TSV files DO support split mode (raw text + table view)
        let skip_split_mode = file_type.is_structured();

        // Track if we need to set App-level pending_scroll_to_line for Raw mode
        let mut raw_mode_scroll_to_line: Option<usize> = None;

        if let Some(tab) = self.state.active_tab_mut() {
            let old_mode = tab.view_mode;
            let current_scroll = tab.scroll_offset;
            let line_mappings = tab.rendered_line_mappings.clone();

            // Debug: log the current state before toggle
            debug!(
                "Toggle view mode: old_mode={:?}, current_scroll={}, sync_enabled={}, mappings_count={}, skip_split={}",
                old_mode, current_scroll, sync_enabled, line_mappings.len(), skip_split_mode
            );

            // Toggle the view mode
            let new_mode = tab.toggle_view_mode();
            
            // For structured/tabular files, skip Split mode (not supported)
            let new_mode = if skip_split_mode && new_mode == ViewMode::Split {
                tab.toggle_view_mode() // Toggle again to skip Split
            } else {
                new_mode
            };
            
            debug!("View mode toggled to: {:?} for tab {}", new_mode, tab.id);

            // Handle sync scrolling when switching modes
            // Note: Split mode shows both panes, so scroll sync is handled in real-time
            if sync_enabled && new_mode != ViewMode::Split && old_mode != ViewMode::Split {
                let content_height = tab.content_height;
                let viewport_height = tab.viewport_height;
                let max_scroll = (content_height - viewport_height).max(0.0);
                
                // Check if we're at boundaries (within 5px tolerance)
                let at_top = current_scroll < 5.0;
                let at_bottom = max_scroll > 0.0 && (max_scroll - current_scroll) < 5.0;
                
                if at_top {
                    // At top - stay at top
                    tab.pending_scroll_offset = Some(0.0);
                    debug!("Sync scroll: at top, staying at top");
                } else if at_bottom {
                    // At bottom - use ratio to stay at bottom
                    tab.pending_scroll_ratio = Some(1.0);
                    debug!("Sync scroll: at bottom, using ratio=1.0");
                } else {
                    // In the middle - use line-based mapping for content preservation
                    match (old_mode, new_mode) {
                        (ViewMode::Raw, ViewMode::Rendered) => {
                            // Calculate which line is at the top of viewport
                            let line_height = tab.raw_line_height;
                            let topmost_line = if line_height > 0.0 {
                                ((current_scroll / line_height) as usize).saturating_add(1)
                            } else {
                                1
                            };
                            
                            // Store for line-based lookup after render (Rendered mode uses tab field)
                            tab.pending_scroll_to_line = Some(topmost_line);
                            debug!(
                                "Sync scroll Raw→Rendered: scroll={} / line_height={:.1} → line {}",
                                current_scroll, line_height, topmost_line
                            );
                        }
                        (ViewMode::Rendered, ViewMode::Raw) => {
                            // Find which line is at current scroll position using mappings
                            if let Some(source_line) = Self::find_source_line_for_rendered_y_interpolated(
                                &line_mappings,
                                current_scroll,
                                content_height,
                            ) {
                                // Raw mode EditorWidget uses App-level pending_scroll_to_line
                                // (not tab field), so we store it for setting after borrow ends.
                                // source_line is 1-indexed from the mapping.
                                raw_mode_scroll_to_line = Some(source_line);
                                debug!(
                                    "Sync scroll Rendered→Raw: scroll={} → line {} (will use App-level pending_scroll_to_line)",
                                    current_scroll, source_line
                                );
                            } else {
                                // Fallback to percentage if no mappings
                                let scroll_ratio = if max_scroll > 0.0 {
                                    (current_scroll / max_scroll).clamp(0.0, 1.0)
                                } else {
                                    0.0
                                };
                                tab.pending_scroll_ratio = Some(scroll_ratio);
                                debug!(
                                    "Sync scroll Rendered→Raw: no mappings, using ratio={:.3}",
                                    scroll_ratio
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Mark settings dirty to save per-tab view mode on exit
            self.state.mark_settings_dirty();
        }

        // Set App-level pending_scroll_to_line AFTER releasing mutable borrow.
        // This is used by Raw mode EditorWidget which reads from self.pending_scroll_to_line.
        if let Some(line) = raw_mode_scroll_to_line {
            self.pending_scroll_to_line = Some(line);
        }
    }
    
    /// Find the rendered Y position for a given source line using interpolated line mappings.
    /// This provides sub-element precision by interpolating within elements.
    fn find_rendered_y_for_line_interpolated(
        mappings: &[(usize, usize, f32)],
        line: usize,
        content_height: f32,
    ) -> Option<f32> {
        if mappings.is_empty() {
            return None;
        }
        
        // Find the element containing this line
        for (i, (start, end, y)) in mappings.iter().enumerate() {
            if line >= *start && line <= *end {
                // Found the element - now interpolate within it
                let element_height = if i + 1 < mappings.len() {
                    mappings[i + 1].2 - y  // Next element's Y - this element's Y
                } else {
                    (content_height - y).max(20.0)  // Last element - use remaining height
                };
                
                // Calculate progress within the element (0.0 to 1.0)
                let line_span = (*end - *start + 1) as f32;
                let progress = if line_span > 1.0 {
                    (line - *start) as f32 / line_span
                } else {
                    0.0
                };
                
                return Some(y + progress * element_height);
            }
        }
        
        // Line is beyond all mappings - return end position
        if let Some((_, _, y)) = mappings.last() {
            return Some(*y);
        }
        
        None
    }

    /// Find the source line for a given rendered Y position using interpolated line mappings.
    fn find_source_line_for_rendered_y_interpolated(
        mappings: &[(usize, usize, f32)],
        rendered_y: f32,
        content_height: f32,
    ) -> Option<usize> {
        if mappings.is_empty() {
            return None;
        }
        
        // Find the element at this Y position
        for (i, (start, end, y)) in mappings.iter().enumerate() {
            let next_y = if i + 1 < mappings.len() {
                mappings[i + 1].2
            } else {
                content_height
            };
            
            if rendered_y >= *y && rendered_y < next_y {
                // Found the element - interpolate to find the line
                let element_height = next_y - y;
                let progress = if element_height > 0.0 {
                    (rendered_y - y) / element_height
                } else {
                    0.0
                };
                
                let line_span = (*end - *start + 1) as f32;
                let line = *start + (progress * line_span) as usize;
                return Some(line.min(*end));
            }
        }
        
        // Beyond all mappings - return last line
        if let Some((_, end, _)) = mappings.last() {
            return Some(*end);
        }
        
        None
    }

    /// Set the application theme and apply it immediately.
    #[allow(dead_code)]
    fn handle_set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme_manager.set_theme(theme);
        self.theme_manager.apply(ctx);

        // Save preference to settings
        self.state.settings.theme = theme;
        self.state.mark_settings_dirty();

        info!("Theme changed to: {:?}", theme);
    }

    /// Cycle through available themes (Light -> Dark -> System).
    fn handle_cycle_theme(&mut self, ctx: &egui::Context) {
        let new_theme = self.theme_manager.cycle();
        self.theme_manager.apply(ctx);

        // Save preference to settings
        self.state.settings.theme = new_theme;
        self.state.mark_settings_dirty();

        info!("Theme cycled to: {:?}", new_theme);
    }

    /// Handle the Undo action (Ctrl+Z).
    ///
    /// Restores the previous content state from the undo stack.
    /// Preserves scroll position, focus, and cursor position across the undo operation.
    fn handle_undo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_undo() {
                let undo_count = tab.undo_count();
                // Preserve scroll position before undo
                let current_scroll = tab.scroll_offset;
                // Perform undo - returns the cursor position from the undo entry
                if let Some(restored_cursor) = tab.undo() {
                    // Restore scroll position via pending_scroll_offset
                    tab.pending_scroll_offset = Some(current_scroll);
                    // Request focus to be restored after content_version change
                    tab.needs_focus = true;
                    // Restore cursor to the position from the undo entry (clamped to content length)
                    let new_len = tab.content.len();
                    tab.pending_cursor_restore = Some(restored_cursor.min(new_len));
                    let time = self.get_app_time();
                    self.state.show_toast(
                        format!("Undo ({} remaining)", undo_count.saturating_sub(1)),
                        time,
                        1.5,
                    );
                    debug!("Undo performed, {} entries remaining", undo_count - 1);
                }
            } else {
                let time = self.get_app_time();
                self.state.show_toast("Nothing to undo", time, 1.5);
                debug!("Undo requested but stack is empty");
            }
        }
    }

    /// Handle the Redo action (Ctrl+Y or Ctrl+Shift+Z).
    ///
    /// Restores the next content state from the redo stack.
    /// Preserves scroll position, focus, and cursor position across the redo operation.
    fn handle_redo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_redo() {
                let redo_count = tab.redo_count();
                // Preserve scroll position before redo
                let current_scroll = tab.scroll_offset;
                // Perform redo - returns the cursor position from the redo entry
                if let Some(restored_cursor) = tab.redo() {
                    // Restore scroll position via pending_scroll_offset
                    tab.pending_scroll_offset = Some(current_scroll);
                    // Request focus to be restored after content_version change
                    tab.needs_focus = true;
                    // Restore cursor to the position from the redo entry (clamped to content length)
                    let new_len = tab.content.len();
                    tab.pending_cursor_restore = Some(restored_cursor.min(new_len));
                    let time = self.get_app_time();
                    self.state.show_toast(
                        format!("Redo ({} remaining)", redo_count.saturating_sub(1)),
                        time,
                        1.5,
                    );
                    debug!("Redo performed, {} entries remaining", redo_count - 1);
                }
            } else {
                let time = self.get_app_time();
                self.state.show_toast("Nothing to redo", time, 1.5);
                debug!("Redo requested but stack is empty");
            }
        }
    }

    /// Handle a markdown formatting command.
    ///
    /// Applies the formatting to the current selection in the active editor.
    fn handle_format_command(&mut self, cmd: MarkdownFormatCommand) {
        if let Some(tab) = self.state.active_tab_mut() {
            let content = tab.content.clone();

            // Use actual selection if available, otherwise use cursor position
            let selection = if let Some((start, end)) = tab.selection {
                Some((start, end))
            } else {
                // Fall back to cursor position (no selection = insertion point)
                let cursor_pos = tab.cursor_position;
                let char_index = line_col_to_char_index(&content, cursor_pos.0, cursor_pos.1);
                Some((char_index, char_index))
            };

            // Apply formatting
            let result = apply_raw_format(&content, selection, cmd);

            // Update content through tab to maintain undo history
            tab.set_content(result.text.clone());

            // Update cursor position and clear selection
            if let Some((sel_start, sel_end)) = result.selection {
                // There's a new selection to set
                let (line, col) = char_index_to_line_col(&result.text, sel_end);
                tab.cursor_position = (line, col);
                tab.selection = Some((sel_start, sel_end));
            } else {
                // Just move cursor to result position
                let (line, col) = char_index_to_line_col(&result.text, result.cursor);
                tab.cursor_position = (line, col);
                tab.selection = None;
            }

            debug!(
                "Applied formatting: {:?}, applied={}, selection={:?}",
                cmd, result.applied, tab.selection
            );
        }
    }

    /// Handle Table of Contents insertion/update.
    ///
    /// Finds an existing TOC block and updates it, or inserts a new one at the cursor.
    fn handle_insert_toc(&mut self) {
        // Check file type first (immutable borrow)
        let is_markdown = self
            .state
            .active_tab()
            .map(|t| t.file_type().is_markdown())
            .unwrap_or(false);

        if !is_markdown {
            let time = self.get_app_time();
            self.state
                .show_toast("TOC only available for Markdown files", time, 2.0);
            return;
        }

        // Get the data needed for TOC generation (immutable borrow)
        let (content, cursor_pos) = {
            let tab = match self.state.active_tab() {
                Some(t) => t,
                None => return,
            };
            (tab.content.clone(), tab.cursor_position)
        };

        // Get cursor position as character index for insertion point
        let cursor_char_index = line_col_to_char_index(&content, cursor_pos.0, cursor_pos.1);

        // Generate and insert/update TOC
        let options = TocOptions::default();
        let result = insert_or_update_toc(&content, cursor_char_index, &options);

        // Update content through tab to maintain undo history (mutable borrow)
        if let Some(tab) = self.state.active_tab_mut() {
            tab.set_content(result.text.clone());

            // Update cursor position to after the TOC
            let (line, col) = char_index_to_line_col(&result.text, result.cursor);
            tab.cursor_position = (line, col);
            tab.selection = None;
        }

        // Show feedback
        let time = self.get_app_time();
        let msg = if result.was_update {
            format!("TOC updated ({} headings)", result.heading_count)
        } else if result.heading_count > 0 {
            format!("TOC inserted ({} headings)", result.heading_count)
        } else {
            "TOC inserted (no headings found)".to_string()
        };
        self.state.show_toast(&msg, time, 2.0);

        debug!(
            "TOC {}: {} headings",
            if result.was_update { "updated" } else { "inserted" },
            result.heading_count
        );
    }

    /// Toggle the outline panel visibility.
    fn handle_toggle_outline(&mut self) {
        self.state.settings.outline_enabled = !self.state.settings.outline_enabled;
        self.state.mark_settings_dirty();

        let time = self.get_app_time();
        if self.state.settings.outline_enabled {
            self.state.show_toast("Outline panel shown", time, 1.5);
        } else {
            self.state.show_toast("Outline panel hidden", time, 1.5);
        }

        debug!(
            "Outline panel toggled: {}",
            self.state.settings.outline_enabled
        );
    }

    /// Toggle Zen Mode (distraction-free writing).
    fn handle_toggle_zen_mode(&mut self) {
        self.state.toggle_zen_mode();
        self.state.mark_settings_dirty();

        let time = self.get_app_time();
        if self.state.is_zen_mode() {
            self.state.show_toast("Zen Mode enabled", time, 1.5);
            info!("Zen Mode enabled");
        } else {
            self.state.show_toast("Zen Mode disabled", time, 1.5);
            info!("Zen Mode disabled");
        }
    }

    /// Toggle OS-level fullscreen mode.
    ///
    /// This is different from Zen Mode - fullscreen hides the taskbar/dock
    /// and makes the window cover the entire screen.
    fn handle_toggle_fullscreen(&mut self, ctx: &egui::Context) {
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        let new_fullscreen = !is_fullscreen;

        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(new_fullscreen));

        let time = self.get_app_time();
        if new_fullscreen {
            self.state.show_toast("Fullscreen mode (F10 or Esc to exit)", time, 2.0);
            info!("Entered fullscreen mode");
        } else {
            self.state.show_toast("Exited fullscreen mode", time, 1.5);
            info!("Exited fullscreen mode");
        }
    }

    /// Toggle the Live Pipeline panel for the active tab (JSON/YAML only).
    fn handle_toggle_pipeline(&mut self) {
        // Check if pipeline feature is enabled
        if !self.state.settings.pipeline_enabled {
            let time = self.get_app_time();
            self.state.show_toast("Pipeline feature is disabled", time, 2.0);
            return;
        }

        // Check if we're in Zen Mode (pipeline hidden in Zen Mode)
        if self.state.is_zen_mode() {
            let time = self.get_app_time();
            self.state.show_toast("Pipeline panel hidden in Zen Mode", time, 2.0);
            return;
        }

        // Check if file type supports pipeline before getting mutable borrow
        let supports = self.state.active_tab().map(|t| t.supports_pipeline()).unwrap_or(false);
        if !supports {
            let file_type_name = self.state.active_tab()
                .map(|t| t.file_type().display_name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let time = self.get_app_time();
            self.state.show_toast(
                &format!("Pipeline only available for JSON/YAML (current: {})", file_type_name),
                time,
                2.5,
            );
            return;
        }

        // Toggle the pipeline panel and get the result
        let (is_visible, tab_id) = {
            if let Some(tab) = self.state.active_tab_mut() {
                tab.toggle_pipeline_panel();
                (tab.pipeline_visible(), tab.id)
            } else {
                return;
            }
        };

        // Show toast after the mutable borrow is released
        let time = self.get_app_time();
        if is_visible {
            self.state.show_toast("Pipeline panel opened", time, 1.5);
            info!("Pipeline panel opened for tab {}", tab_id);
        } else {
            self.state.show_toast("Pipeline panel closed", time, 1.5);
            info!("Pipeline panel closed for tab {}", tab_id);
        }
    }

    /// Handle opening the Go to Line dialog.
    fn handle_open_go_to_line(&mut self) {
        // Get current line and max line from active tab
        let Some(tab) = self.state.active_tab() else {
            return;
        };

        // Calculate current line (1-indexed) from cursor position
        let current_line = tab.cursor_position.0 + 1;

        // Calculate total line count
        let max_line = tab.content.lines().count().max(1);

        // Open the Go to Line dialog
        self.state.ui.go_to_line_dialog =
            Some(crate::ui::GoToLineDialog::new(current_line, max_line));
    }

    /// Handle navigating to a specific line number.
    fn handle_go_to_line(&mut self, target_line: usize) {
        // Get the active tab
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Calculate the character index for the start of the target line
        // target_line is 1-indexed, we need 0-indexed for content iteration
        let line_index = target_line.saturating_sub(1);
        let mut char_index = 0;
        let mut current_line = 0;

        for (idx, ch) in tab.content.char_indices() {
            if current_line == line_index {
                char_index = tab.content[..idx].chars().count();
                break;
            }
            if ch == '\n' {
                current_line += 1;
            }
        }

        // If we didn't find the line (end of file), go to last character
        if current_line < line_index {
            char_index = tab.content.chars().count();
        }

        // Update cursor position to the start of the target line
        tab.cursors
            .set_single(crate::state::Selection::cursor(char_index));
        tab.sync_cursor_from_primary();

        // Use the existing scroll_to_line mechanism to center the line in viewport
        // This is already handled by EditorWidget when pending_scroll_to_line is set
        self.pending_scroll_to_line = Some(target_line);

        debug!("Go to Line: navigating to line {} (char index {})", target_line, char_index);
    }

    /// Handle duplicating the current line or selection.
    ///
    /// - If no selection: duplicates the entire current line (including newline)
    /// - If selection: duplicates the selected text immediately after the selection
    fn handle_duplicate_line(&mut self) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        let primary = tab.cursors.primary();
        let has_selection = primary.is_selection();

        // Helper to convert character index to byte index
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        if has_selection {
            // Duplicate selection: insert selected text at the end of selection
            let (start_char, end_char) = primary.range();

            // Convert character indices to byte indices
            let start_byte = char_to_byte(&tab.content, start_char);
            let end_byte = char_to_byte(&tab.content, end_char);

            let selected_text = tab.content[start_byte..end_byte].to_string();
            let selected_char_len = selected_text.chars().count();

            // Insert the selected text at the end of the selection
            tab.content.insert_str(end_byte, &selected_text);

            // Set new selection to cover the duplicated text (using character indices)
            let new_start = end_char;
            let new_end = end_char + selected_char_len;
            tab.cursors
                .set_single(crate::state::Selection::new(new_start, new_end));
        } else {
            // No selection: duplicate entire current line
            let cursor_char_pos = primary.head;
            let cursor_byte_pos = char_to_byte(&tab.content, cursor_char_pos);

            // Find the start of the current line (byte position)
            let line_start_byte = tab.content[..cursor_byte_pos]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);

            // Find the end of the current line (byte position)
            let line_end_byte = tab.content[cursor_byte_pos..]
                .find('\n')
                .map(|i| cursor_byte_pos + i)
                .unwrap_or(tab.content.len());

            // Get the line content (without the newline)
            let line_content = tab.content[line_start_byte..line_end_byte].to_string();

            // Build the text to insert: newline + line content
            let insert_text = format!("\n{}", line_content);

            // Insert after the current line (at line_end_byte position)
            tab.content.insert_str(line_end_byte, &insert_text);

            // Keep cursor on the original line at the same relative position
            // (cursor position doesn't change since we inserted after it)
        }

        tab.sync_cursor_from_primary();

        // Record the edit for undo support
        tab.record_edit(old_content, old_cursor);

        debug!("Duplicate line/selection: has_selection={}", has_selection);
    }

    /// Handle moving line(s) up or down.
    ///
    /// `direction`: -1 for up, 1 for down
    fn handle_move_line(&mut self, direction: isize) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        // Get cursor position - cursor_position gives (line, column) directly
        let (current_line_num, cursor_col) = tab.cursor_position;
        let total_lines = tab.content.matches('\n').count() + 1;

        // Check boundaries
        if direction < 0 && current_line_num == 0 {
            return; // Can't move up from first line
        }
        if direction > 0 && current_line_num >= total_lines - 1 {
            return; // Can't move down from last line
        }

        // Split into lines for manipulation
        let lines: Vec<&str> = tab.content.split('\n').collect();
        let mut new_lines = lines.clone();

        // Perform the swap
        if direction < 0 {
            // Moving up: swap with line above
            new_lines.swap(current_line_num, current_line_num - 1);
        } else {
            // Moving down: swap with line below
            new_lines.swap(current_line_num, current_line_num + 1);
        }

        // Build new content
        let new_content = new_lines.join("\n");

        // Calculate new cursor position
        // The cursor should be on the same line content, which has moved
        let new_line_num = if direction < 0 {
            current_line_num - 1
        } else {
            current_line_num + 1
        };

        // Find byte offset of the new line position
        let mut new_line_start = 0usize;
        for (i, line) in new_lines.iter().enumerate() {
            if i == new_line_num {
                break;
            }
            new_line_start += line.len() + 1; // +1 for newline
        }

        // Calculate new cursor byte position (line start + column, clamped to line length)
        let new_line_len = new_lines.get(new_line_num).map(|l| l.len()).unwrap_or(0);
        let new_cursor_byte = new_line_start + cursor_col.min(new_line_len);

        // Convert byte position to character position
        let new_cursor_char = new_content[..new_cursor_byte].chars().count();

        debug!(
            "Move line: new_line_num={}, new_line_start={}, new_cursor_byte={}, new_cursor_char={}",
            new_line_num, new_line_start, new_cursor_byte, new_cursor_char
        );

        // Apply changes
        tab.content = new_content;
        
        // Use pending_cursor_restore to ensure the cursor position is applied
        // This is necessary because egui's TextEdit has its own cursor state
        // that would otherwise override our changes on the next frame
        tab.pending_cursor_restore = Some(new_cursor_char);
        
        // Also update internal state for consistency
        tab.cursors.set_single(crate::state::Selection::cursor(new_cursor_char));
        tab.sync_cursor_from_primary();

        // Record for undo
        tab.record_edit(old_content, old_cursor);

        debug!("Move line: direction={}, line {} -> {}", direction, current_line_num, new_line_num);
    }

    /// Handle deleting the current line.
    ///
    /// Operates in Raw or Split view mode (both have raw editor). Removes the current line entirely,
    /// placing the cursor at the same column on the next line (or previous if at end).
    fn handle_delete_line(&mut self) {
        // Only operate in Raw or Split view mode (both have raw editor)
        let view_mode = self.state.active_tab()
            .map(|t| t.view_mode)
            .unwrap_or(ViewMode::Raw);

        if view_mode == ViewMode::Rendered {
            debug!("Delete line: skipping, Rendered mode has no raw editor");
            return;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        // Get cursor position - cursor_position gives (line, column) directly
        let (current_line_num, cursor_col) = tab.cursor_position;
        let total_lines = tab.content.matches('\n').count() + 1;

        // Can't delete if document is empty or has only one empty line
        if tab.content.is_empty() {
            debug!("Delete line: skipping, document is empty");
            return;
        }

        // Split into lines for manipulation
        let lines: Vec<&str> = tab.content.split('\n').collect();
        let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len().saturating_sub(1));

        // Remove the current line
        for (i, line) in lines.iter().enumerate() {
            if i != current_line_num {
                new_lines.push(line);
            }
        }

        // Build new content
        let new_content = if new_lines.is_empty() {
            // If we deleted the last line, result is empty
            String::new()
        } else {
            new_lines.join("\n")
        };

        // Calculate new cursor position
        // Stay on same line number if possible, or move to previous line if we were on last line
        let new_line_num = if current_line_num >= new_lines.len() {
            new_lines.len().saturating_sub(1)
        } else {
            current_line_num
        };

        // Find byte offset of the new line position
        let mut new_line_start = 0usize;
        for (i, line) in new_lines.iter().enumerate() {
            if i == new_line_num {
                break;
            }
            new_line_start += line.len() + 1; // +1 for newline
        }

        // Calculate new cursor byte position (line start + column, clamped to line length)
        let new_line_len = new_lines.get(new_line_num).map(|l| l.len()).unwrap_or(0);
        let new_cursor_byte = new_line_start + cursor_col.min(new_line_len);

        // Convert byte position to character position
        let new_cursor_char = if new_content.is_empty() {
            0
        } else {
            new_content[..new_cursor_byte.min(new_content.len())].chars().count()
        };

        debug!(
            "Delete line: line={}, total_lines={}, new_line_num={}, new_cursor_char={}",
            current_line_num, total_lines, new_line_num, new_cursor_char
        );

        // Apply changes
        tab.content = new_content;

        // Use pending_cursor_restore to ensure the cursor position is applied
        tab.pending_cursor_restore = Some(new_cursor_char);

        // Also update internal state for consistency
        tab.cursors.set_single(crate::state::Selection::cursor(new_cursor_char));
        tab.sync_cursor_from_primary();

        // Record for undo
        tab.record_edit(old_content, old_cursor);

        debug!("Delete line: deleted line {} (total was {})", current_line_num, total_lines);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Export Handlers
    // ─────────────────────────────────────────────────────────────────────────

    /// Handle exporting the current document as HTML file.
    fn handle_export_html(&mut self, ctx: &egui::Context) {
        // Get the active tab content
        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast("No document to export", time, 2.0);
            return;
        };

        let content = tab.content.clone();
        let source_path = tab.path.clone();

        // Determine initial directory and default filename
        let initial_dir = source_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .or_else(|| self.state.settings.last_export_directory.clone())
            .or_else(|| {
                self.state
                    .settings
                    .recent_files
                    .first()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            });

        let default_name = source_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| format!("{}.html", s))
            .unwrap_or_else(|| "exported.html".to_string());

        // Get current theme colors
        let theme_colors = self.theme_manager.colors(ctx);

        // Open save dialog for HTML
        let filter = rfd::FileDialog::new()
            .add_filter("HTML Files", &["html", "htm"])
            .set_file_name(&default_name);

        let filter = if let Some(dir) = initial_dir.as_ref() {
            filter.set_directory(dir)
        } else {
            filter
        };

        if let Some(path) = filter.save_file() {
            // Get document title
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Exported Document");

            // Generate HTML with paragraph indentation setting
            match generate_html_document(&content, Some(title), &theme_colors, true, self.state.settings.paragraph_indent) {
                Ok(html) => {
                    // Write to file
                    match std::fs::write(&path, html) {
                        Ok(()) => {
                            info!("Exported HTML to: {}", path.display());

                            // Update last export directory
                            if let Some(parent) = path.parent() {
                                self.state.settings.last_export_directory =
                                    Some(parent.to_path_buf());
                                self.state.mark_settings_dirty();
                            }

                            let time = self.get_app_time();
                            self.state.show_toast(
                                format!("Exported to {}", path.display()),
                                time,
                                2.5,
                            );

                            // Optionally open the file
                            if self.state.settings.open_after_export {
                                if let Err(e) = open::that(&path) {
                                    warn!("Failed to open exported file: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to write HTML file: {}", e);
                            let time = self.get_app_time();
                            self.state
                                .show_toast(format!("Export failed: {}", e), time, 3.0);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to generate HTML: {}", e);
                    let time = self.get_app_time();
                    self.state
                        .show_toast(format!("Export failed: {}", e), time, 3.0);
                }
            }
        }
    }

    /// Handle copying the current document as HTML to clipboard.
    fn handle_copy_as_html(&mut self) {
        // Get the active tab content
        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast("No document to copy", time, 2.0);
            return;
        };

        let content = tab.content.clone();

        // Copy HTML to clipboard
        match copy_html_to_clipboard(&content) {
            Ok(()) => {
                info!("Copied HTML to clipboard");
                let time = self.get_app_time();
                self.state.show_toast("HTML copied to clipboard", time, 2.0);
            }
            Err(e) => {
                warn!("Failed to copy HTML to clipboard: {}", e);
                let time = self.get_app_time();
                self.state
                    .show_toast(format!("Copy failed: {}", e), time, 3.0);
            }
        }
    }

    /// Handle formatting/pretty-printing a structured data document (JSON/YAML/TOML).
    fn handle_format_structured_document(&mut self) {
        use crate::markdown::tree_viewer::{parse_structured_content, serialize_tree};

        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast("No document to format", time, 2.0);
            return;
        };

        let file_type = tab.file_type();
        if !file_type.is_structured() {
            let time = self.get_app_time();
            self.state
                .show_toast("Not a structured data file", time, 2.0);
            return;
        }

        let content = tab.content.clone();

        // Convert FileType to StructuredFileType
        let structured_type = match file_type {
            FileType::Json => crate::markdown::tree_viewer::StructuredFileType::Json,
            FileType::Yaml => crate::markdown::tree_viewer::StructuredFileType::Yaml,
            FileType::Toml => crate::markdown::tree_viewer::StructuredFileType::Toml,
            _ => return,
        };

        // Parse and reserialize to format
        match parse_structured_content(&content, structured_type) {
            Ok(tree) => {
                match serialize_tree(&tree, structured_type) {
                    Ok(formatted) => {
                        // Update the tab content
                        if let Some(tab) = self.state.active_tab_mut() {
                            let old_content = tab.content.clone();
                            let old_cursor = tab.cursors.primary().head;
                            tab.content = formatted;
                            tab.record_edit(old_content, old_cursor);
                        }
                        let time = self.get_app_time();
                        self.state.show_toast("Document formatted", time, 2.0);
                        info!("Formatted {} document", file_type.display_name());
                    }
                    Err(e) => {
                        let time = self.get_app_time();
                        self.state
                            .show_toast(format!("Format failed: {}", e), time, 3.0);
                        warn!("Failed to serialize {}: {}", file_type.display_name(), e);
                    }
                }
            }
            Err(e) => {
                let time = self.get_app_time();
                self.state
                    .show_toast(format!("Parse error: {}", e), time, 3.0);
                warn!(
                    "Failed to parse {} for formatting: {}",
                    file_type.display_name(),
                    e
                );
            }
        }
    }

    /// Handle validating the syntax of a structured data document (JSON/YAML/TOML).
    fn handle_validate_structured_syntax(&mut self) {
        use crate::markdown::tree_viewer::parse_structured_content;

        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast("No document to validate", time, 2.0);
            return;
        };

        let file_type = tab.file_type();
        if !file_type.is_structured() {
            let time = self.get_app_time();
            self.state
                .show_toast("Not a structured data file", time, 2.0);
            return;
        }

        let content = tab.content.clone();

        // Convert FileType to StructuredFileType
        let structured_type = match file_type {
            FileType::Json => crate::markdown::tree_viewer::StructuredFileType::Json,
            FileType::Yaml => crate::markdown::tree_viewer::StructuredFileType::Yaml,
            FileType::Toml => crate::markdown::tree_viewer::StructuredFileType::Toml,
            _ => return,
        };

        // Try to parse to validate
        match parse_structured_content(&content, structured_type) {
            Ok(_) => {
                let time = self.get_app_time();
                self.state.show_toast(
                    format!("✓ Valid {} syntax", file_type.display_name()),
                    time,
                    2.0,
                );
                info!("{} document is valid", file_type.display_name());
            }
            Err(e) => {
                let time = self.get_app_time();
                self.state.show_toast(format!("✗ {}", e), time, 4.0);
                warn!("{} validation failed: {}", file_type.display_name(), e);
            }
        }
    }

    /// Update the cached outline if the document content has changed.
    fn update_outline_if_needed(&mut self) {
        if let Some(tab) = self.state.active_tab() {
            // Calculate a simple hash of the content and path
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            tab.content.hash(&mut hasher);
            tab.path.hash(&mut hasher); // Include path in hash for file type changes
            let content_hash = hasher.finish();

            // Only regenerate if content or path changed
            if content_hash != self.last_outline_content_hash {
                // Use file-type aware outline extraction
                self.cached_outline = extract_outline_for_file(&tab.content, tab.path.as_deref());

                // Calculate document stats for markdown files
                if matches!(self.cached_outline.outline_type, OutlineType::Markdown) {
                    self.cached_doc_stats = Some(DocumentStats::from_text(&tab.content));
                } else {
                    self.cached_doc_stats = None;
                }

                self.last_outline_content_hash = content_hash;
            }
        } else {
            // No active tab, clear outline and stats
            if !self.cached_outline.is_empty() {
                self.cached_outline = DocumentOutline::new();
                self.cached_doc_stats = None;
                self.last_outline_content_hash = 0;
            }
        }
    }

    /// Navigate to a heading with text-based search and transient highlighting.
    ///
    /// This provides more precise navigation than line-based scrolling by:
    /// 1. Searching for the exact heading text in the document
    /// 2. Applying transient highlight to make the heading visible
    /// 3. Positioning the cursor at the heading
    fn navigate_to_heading(&mut self, nav: HeadingNavRequest) {
        // Find the line range using the line number from OutlineItem
        // This is the most reliable approach since line numbers are always correct
        let found_range = if let Some(tab) = self.state.active_tab() {
            let content = &tab.content;
            
            // Find the byte range for the target line (nav.line is 1-indexed)
            Self::find_line_byte_range(content, nav.line)
        } else {
            None
        };

        // Apply navigation and calculate target line (1-indexed for scroll)
        let target_line_1indexed = if let Some(tab) = self.state.active_tab_mut() {
            if let Some((char_start, char_end)) = found_range {
                // Set transient highlight for the heading line (expects char offsets)
                tab.set_transient_highlight(char_start, char_end);
                
                // Calculate line and column from character offset (0-indexed)
                let (target_line, _) = Self::offset_to_line_col(&tab.content, char_start);
                tab.cursor_position = (target_line, 0);
                
                let line_1indexed = target_line + 1;
                debug!(
                    "Navigated to heading at char offset {}-{}, line {}",
                    char_start, char_end, line_1indexed
                );
                Some(line_1indexed)
            } else {
                // Fall back to basic line navigation using nav.line (already 1-indexed)
                tab.cursor_position = (nav.line.saturating_sub(1), 0);
                debug!("Navigated to heading via fallback, line {}", nav.line);
                Some(nav.line)
            }
        } else {
            None
        };

        // Set pending scroll AFTER releasing the mutable borrow.
        // Use App-level pending_scroll_to_line so EditorWidget calculates
        // scroll offset with fresh line height from ui.fonts().
        // This is more accurate than using potentially stale raw_line_height.
        if let Some(line) = target_line_1indexed {
            self.pending_scroll_to_line = Some(line);
        }
    }

    /// Find a heading near a specific line (for fuzzy matching).
    /// Returns character offsets (not byte offsets) for use with egui.
    fn find_heading_near_line(
        content: &str,
        title: &str,
        level: u8,
        expected_line: usize,
    ) -> Option<(usize, usize)> {
        let hashes = "#".repeat(level as usize);
        let mut current_line: usize = 1;
        let mut char_offset: usize = 0; // Track character offset, not byte offset

        for line in content.lines() {
            // Check if we're near the expected line (within 5 lines)
            let diff = if current_line > expected_line {
                current_line - expected_line
            } else {
                expected_line - current_line
            };
            
            if diff <= 5 {
                // Check if this line is a heading of the right level
                if line.starts_with(&hashes) && !line.starts_with(&format!("{}#", hashes)) {
                    // Extract heading text after the hashes
                    let heading_text = line[hashes.len()..].trim();
                    // Case-insensitive comparison
                    if heading_text.eq_ignore_ascii_case(title) {
                        let start = char_offset;
                        let end = char_offset + line.chars().count();
                        return Some((start, end));
                    }
                }
            }
            
            // Add character count of this line plus 1 for newline
            char_offset += line.chars().count() + 1;
            current_line += 1;
            
            // Stop searching too far past the expected line
            if current_line > expected_line + 10 {
                break;
            }
        }
        None
    }

    /// Find the BYTE range (start, end) for a specific line number.
    /// 
    /// # Arguments
    /// * `content` - The text content
    /// * `line_num` - The line number (1-indexed)
    /// 
    /// # Returns
    /// The byte offset range (start, end) for that line, or None if line doesn't exist.
    /// Note: Returns BYTE offsets because set_transient_highlight expects bytes.
    fn find_line_byte_range(content: &str, line_num: usize) -> Option<(usize, usize)> {
        if line_num == 0 {
            return None;
        }
        
        let target_idx = line_num - 1; // Convert to 0-indexed
        
        // Simple approach: find the byte position by scanning the actual bytes
        let bytes = content.as_bytes();
        let mut line_start = 0;
        let mut current_line = 0;
        
        for (i, &byte) in bytes.iter().enumerate() {
            if current_line == target_idx {
                // Found the start of our target line, now find its end
                let mut line_end = i;
                for j in i..bytes.len() {
                    if bytes[j] == b'\n' {
                        // Don't include \r if present
                        line_end = if j > 0 && bytes[j - 1] == b'\r' { j - 1 } else { j };
                        break;
                    }
                    line_end = j + 1;
                }
                return Some((i, line_end));
            }
            
            if byte == b'\n' {
                current_line += 1;
                line_start = i + 1;
            }
        }
        
        // Handle last line (no trailing newline)
        if current_line == target_idx {
            return Some((line_start, bytes.len()));
        }
        
        None
    }

    /// Convert a byte offset to character offset.
    /// 
    /// This is needed because `String::find()` returns byte offsets, but egui's
    /// text system (CCursor) uses character offsets. For ASCII text they're the same,
    /// but for UTF-8 content with multi-byte characters they differ.
    fn byte_to_char_offset(content: &str, byte_offset: usize) -> usize {
        // Count characters up to the byte offset
        content[..byte_offset.min(content.len())]
            .chars()
            .count()
    }

    /// Convert a character offset to (line, column) - 0-indexed.
    /// 
    /// NOTE: This expects a CHARACTER offset, not a byte offset.
    /// Use `byte_to_char_offset()` first if you have a byte offset from `String::find()`.
    fn offset_to_line_col(content: &str, char_offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        
        for (i, ch) in content.chars().enumerate() {
            if i >= char_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        
        (line, col)
    }

    /// Get the current formatting state for the active editor.
    ///
    /// Returns None if no editor is active.
    fn get_formatting_state(&self) -> Option<FormattingState> {
        let tab = self.state.active_tab()?;
        let content = &tab.content;
        let cursor_pos = tab.cursor_position;

        // Convert line/col to character index
        let char_index = line_col_to_char_index(content, cursor_pos.0, cursor_pos.1);

        Some(detect_raw_formatting_state(content, char_index))
    }

    /// Handle opening the find panel.
    ///
    /// Opens the find panel, optionally in replace mode.
    fn handle_open_find(&mut self, replace_mode: bool) {
        self.state.ui.show_find_replace = true;
        self.state.ui.find_state.is_replace_mode = replace_mode;
        self.find_replace_panel.request_focus();

        // Trigger initial search if there's already a search term
        if !self.state.ui.find_state.search_term.is_empty() {
            if let Some(tab) = self.state.active_tab() {
                let content = tab.content.clone();
                let count = self.state.ui.find_state.find_matches(&content);
                if count > 0 {
                    self.state.ui.scroll_to_match = true;
                }
            }
        }

        debug!("Find panel opened, replace_mode: {}", replace_mode);
    }

    /// Handle find next match action.
    fn handle_find_next(&mut self) {
        if !self.state.ui.show_find_replace {
            return;
        }

        if let Some(idx) = self.state.ui.find_state.next_match() {
            self.state.ui.scroll_to_match = true;
            debug!("Find next: moved to match {}", idx + 1);
        }
    }

    /// Handle find previous match action.
    fn handle_find_prev(&mut self) {
        if !self.state.ui.show_find_replace {
            return;
        }

        if let Some(idx) = self.state.ui.find_state.prev_match() {
            self.state.ui.scroll_to_match = true;
            debug!("Find prev: moved to match {}", idx + 1);
        }
    }

    /// Handle Ctrl+D: Select next occurrence of current word/selection.
    ///
    /// VS Code-style behavior:
    /// - If no selection: select the word under cursor
    /// - If selection exists: find next occurrence and add cursor there
    fn handle_select_next_occurrence(&mut self) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Get the text to search for
        let search_text = match tab.get_primary_selection_text() {
            Some(text) if !text.is_empty() => text,
            _ => {
                // No word at cursor, try to select word under cursor first
                let primary_pos = tab.cursors.primary().head;
                if let Some((start, end)) = tab.word_range_at_position(primary_pos) {
                    // Select the word under cursor
                    tab.set_selection(start, end);
                    debug!("Selected word at cursor: {}..{}", start, end);
                }
                return;
            }
        };

        // Get the last selection's end position to search from
        let search_from = {
            let selections = tab.cursors.selections();
            // Find the rightmost selection to search after
            selections
                .iter()
                .map(|s| s.end())
                .max()
                .unwrap_or(0)
        };

        // Find next occurrence that doesn't overlap with existing selections
        if let Some((start, end)) = tab.find_next_occurrence(&search_text, search_from) {
            // Check if this occurrence is already selected
            let already_selected = tab.cursors.selections().iter().any(|s| {
                s.start() == start && s.end() == end
            });

            if !already_selected {
                // Add new selection
                tab.add_selection(start, end);
                debug!(
                    "Added selection at {}..{}, now {} cursor(s)",
                    start,
                    end,
                    tab.cursor_count()
                );
            } else {
                debug!("All occurrences already selected");
            }
        } else {
            debug!("No more occurrences found for '{}'", search_text);
        }
    }

    /// Handle replace current match action.
    fn handle_replace_current(&mut self) {
        if let Some(tab) = self.state.active_tab() {
            let content = tab.content.clone();
            if let Some(new_content) = self.state.ui.find_state.replace_current(&content) {
                // Apply replacement through tab to maintain undo history
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.set_content(new_content.clone());
                }

                // Re-search to update matches
                self.state.ui.find_state.find_matches(&new_content);

                let time = self.get_app_time();
                self.state.show_toast("Replaced", time, 1.5);
                debug!("Replaced current match");
            }
        }
    }

    /// Handle replace all matches action.
    fn handle_replace_all(&mut self) {
        if let Some(tab) = self.state.active_tab() {
            let content = tab.content.clone();
            let match_count = self.state.ui.find_state.match_count();

            if match_count > 0 {
                let new_content = self.state.ui.find_state.replace_all(&content);

                // Apply replacement through tab to maintain undo history
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.set_content(new_content.clone());
                }

                // Re-search (will find 0 matches after replace all)
                self.state.ui.find_state.find_matches(&new_content);

                let time = self.get_app_time();
                self.state.show_toast(
                    format!(
                        "Replaced {} occurrence{}",
                        match_count,
                        if match_count == 1 { "" } else { "s" }
                    ),
                    time,
                    2.0,
                );
                debug!("Replaced all {} matches", match_count);
            }
        }
    }

    /// Handle actions triggered from the ribbon UI.
    ///
    /// Maps ribbon actions to their corresponding handler methods.
    fn handle_ribbon_action(&mut self, action: RibbonAction, ctx: &egui::Context) {
        match action {
            // File operations
            RibbonAction::New => {
                debug!("Ribbon: New file");
                self.state.new_tab();
            }
            RibbonAction::Open => {
                debug!("Ribbon: Open file");
                self.handle_open_file();
            }
            RibbonAction::OpenWorkspace => {
                debug!("Ribbon: Open workspace");
                self.handle_open_workspace();
            }
            RibbonAction::CloseWorkspace => {
                debug!("Ribbon: Close workspace");
                self.handle_close_workspace();
            }

            // Workspace operations (only available in workspace mode)
            RibbonAction::SearchInFiles => {
                debug!("Ribbon: Search in Files");
                self.handle_search_in_files();
            }
            RibbonAction::QuickFileSwitcher => {
                debug!("Ribbon: Quick File Switcher");
                self.handle_quick_open();
            }

            RibbonAction::Save => {
                debug!("Ribbon: Save file");
                self.handle_save_file();
            }
            RibbonAction::SaveAs => {
                debug!("Ribbon: Save As");
                self.handle_save_as_file();
            }
            RibbonAction::ToggleAutoSave => {
                debug!("Ribbon: Toggle Auto-Save");
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.toggle_auto_save();
                    info!("Auto-save {} for tab {}", 
                        if tab.auto_save_enabled { "enabled" } else { "disabled" },
                        tab.id
                    );
                }
            }

            // Edit operations
            RibbonAction::Undo => {
                debug!("Ribbon: Undo");
                self.handle_undo();
            }
            RibbonAction::Redo => {
                debug!("Ribbon: Redo");
                self.handle_redo();
            }

            // View operations
            RibbonAction::ToggleViewMode => {
                debug!("Ribbon: Toggle view mode");
                self.handle_toggle_view_mode();
            }
            RibbonAction::ToggleLineNumbers => {
                debug!("Ribbon: Toggle line numbers");
                self.state.settings.show_line_numbers = !self.state.settings.show_line_numbers;
                self.state.mark_settings_dirty();
            }
            RibbonAction::ToggleSyncScroll => {
                debug!("Ribbon: Toggle sync scroll");
                self.state.settings.sync_scroll_enabled = !self.state.settings.sync_scroll_enabled;
                self.state.mark_settings_dirty();

                // Show toast message
                let msg = if self.state.settings.sync_scroll_enabled {
                    "Sync scrolling enabled"
                } else {
                    "Sync scrolling disabled"
                };
                let app_time = self.get_app_time();
                self.state.show_toast(msg, app_time, 2.0);
            }

            // Tools
            RibbonAction::FindReplace => {
                debug!("Ribbon: Find/Replace");
                self.handle_open_find(false);
            }
            RibbonAction::ToggleOutline => {
                debug!("Ribbon: Toggle Outline");
                self.handle_toggle_outline();
            }

            // Settings
            RibbonAction::CycleTheme => {
                debug!("Ribbon: Cycle theme");
                self.handle_cycle_theme(ctx);
            }
            RibbonAction::OpenSettings => {
                debug!("Ribbon: Open settings");
                self.state.toggle_settings();
            }

            // Ribbon control
            RibbonAction::ToggleCollapse => {
                debug!("Ribbon: Toggle collapse");
                self.ribbon.toggle_collapsed();
            }

            // Zen Mode
            RibbonAction::ToggleZenMode => {
                debug!("Ribbon: Toggle Zen Mode");
                self.handle_toggle_zen_mode();
            }

            // Live Pipeline
            RibbonAction::TogglePipeline => {
                debug!("Ribbon: Toggle Pipeline");
                self.handle_toggle_pipeline();
            }

            // Export operations (Markdown)
            RibbonAction::ExportHtml => {
                debug!("Ribbon: Export HTML");
                self.handle_export_html(ctx);
            }
            RibbonAction::CopyAsHtml => {
                debug!("Ribbon: Copy as HTML");
                self.handle_copy_as_html();
            }

            // Structured data operations (JSON/YAML/TOML)
            RibbonAction::FormatDocument => {
                debug!("Ribbon: Format Document");
                self.handle_format_structured_document();
            }
            RibbonAction::ValidateSyntax => {
                debug!("Ribbon: Validate Syntax");
                self.handle_validate_structured_syntax();
            }

            // Markdown formatting operations
            RibbonAction::Format(cmd) => {
                debug!("Ribbon: Format {:?}", cmd);
                self.handle_format_command(cmd);
            }

            // Markdown document operations
            RibbonAction::InsertToc => {
                debug!("Ribbon: Insert/Update TOC");
                self.handle_insert_toc();
            }
        }
    }

    /// Render dialog windows.
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        // Confirmation dialog for unsaved changes
        if self.state.ui.show_confirm_dialog {
            egui::Window::new(t!("dialog.unsaved_changes.title").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(&self.state.ui.confirm_dialog_message);
                    ui.separator();
                    ui.horizontal(|ui| {
                        // Check if this is a tab close action (vs exit)
                        let is_tab_close = matches!(
                            self.state.ui.pending_action,
                            Some(PendingAction::CloseTab(_))
                        );
                        let is_exit = self.state.ui.pending_action == Some(PendingAction::Exit);

                        // Extract tab_id for cleanup if this is a CloseTab action
                        let tab_id_to_cleanup = if let Some(PendingAction::CloseTab(index)) =
                            self.state.ui.pending_action
                        {
                            self.state.tabs().get(index).map(|t| t.id)
                        } else {
                            None
                        };

                        // "Save" button - save then proceed with action
                        if ui.button(t!("dialog.unsaved_changes.save").to_string()).clicked() {
                            if is_tab_close {
                                // Save the tab first
                                if let Some(PendingAction::CloseTab(index)) =
                                    self.state.ui.pending_action
                                {
                                    // Switch to that tab to save it
                                    self.state.set_active_tab(index);
                                }
                                self.handle_save_file();
                                // If save succeeded (tab is no longer modified), close it
                                if let Some(PendingAction::CloseTab(index)) =
                                    self.state.ui.pending_action
                                {
                                    if !self
                                        .state
                                        .tab(index)
                                        .map(|t| t.is_modified())
                                        .unwrap_or(true)
                                    {
                                        self.state.handle_confirmed_action();
                                        // Clean up viewer state after tab is closed
                                        if let Some(id) = tab_id_to_cleanup {
                                            self.cleanup_tab_state(id, Some(ui.ctx()));
                                        }
                                    } else {
                                        // Save was cancelled or failed, cancel the close
                                        self.state.cancel_pending_action();
                                    }
                                }
                            } else if is_exit {
                                // Save all modified tabs before exit
                                self.handle_save_file();
                                if !self.state.has_unsaved_changes() {
                                    self.state.handle_confirmed_action();
                                    self.should_exit = true;
                                }
                            }
                        }

                        // "Discard" button - proceed without saving
                        if ui.button(t!("dialog.unsaved_changes.dont_save").to_string()).clicked() {
                            self.state.handle_confirmed_action();
                            // Clean up viewer state after tab is closed
                            if let Some(id) = tab_id_to_cleanup {
                                self.cleanup_tab_state(id, Some(ui.ctx()));
                            }
                            if is_exit {
                                self.should_exit = true;
                            }
                        }

                        // "Cancel" button - abort the action
                        if ui.button(t!("dialog.confirm.cancel").to_string()).clicked() {
                            self.state.cancel_pending_action();
                        }
                    });
                });
        }

        // Error modal
        if self.state.ui.show_error_modal {
            egui::Window::new(t!("common.error").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("⚠").size(24.0));
                    ui.label(&self.state.ui.error_message);
                    ui.separator();
                    if ui.button(t!("common.ok").to_string()).clicked() {
                        self.state.dismiss_error();
                    }
                });
        }

        // About/Help panel
        if self.state.ui.show_about {
            let is_dark = ctx.style().visuals.dark_mode;
            let output = self.about_panel.show(ctx, is_dark);

            if output.close_requested {
                self.state.ui.show_about = false;
            }
        }

        // Settings panel
        if self.state.ui.show_settings {
            let is_dark = ctx.style().visuals.dark_mode;
            
            // Capture font settings before showing panel
            let prev_font_family = self.state.settings.font_family.clone();
            let prev_cjk_preference = self.state.settings.cjk_font_preference;
            
            let output = self
                .settings_panel
                .show(ctx, &mut self.state.settings, is_dark);

            if output.changed {
                // Apply theme changes immediately
                self.theme_manager.set_theme(self.state.settings.theme);
                self.theme_manager.apply(ctx);
                self.state.mark_settings_dirty();
                
                // Reload fonts if font settings changed
                let font_changed = prev_font_family != self.state.settings.font_family
                    || prev_cjk_preference != self.state.settings.cjk_font_preference;
                
                if font_changed {
                    let custom_font = self.state.settings.font_family.custom_name().map(|s| s.to_string());
                    fonts::reload_fonts(
                        ctx,
                        custom_font.as_deref(),
                        self.state.settings.cjk_font_preference,
                    );
                    info!("Font settings changed, reloaded fonts");
                }
            }

            if output.reset_requested {
                // Reset to defaults
                let default_settings = Settings::default();
                self.state.settings = default_settings;
                self.theme_manager.set_theme(self.state.settings.theme);
                self.theme_manager.apply(ctx);
                self.state.mark_settings_dirty();
                
                // Reload fonts with defaults
                fonts::reload_fonts(ctx, None, CjkFontPreference::Auto);

                let time = self.get_app_time();
                self.state
                    .show_toast("Settings reset to defaults", time, 2.0);
            }

            if output.close_requested {
                self.state.ui.show_settings = false;
            }
        }

        // Find/Replace panel
        if self.state.ui.show_find_replace {
            let is_dark = ctx.style().visuals.dark_mode;
            let output = self
                .find_replace_panel
                .show(ctx, &mut self.state.ui.find_state, is_dark);

            // Handle search changes - re-search when term or options change
            if output.search_changed {
                if let Some(tab) = self.state.active_tab() {
                    let content = tab.content.clone();
                    let match_count = self.state.ui.find_state.find_matches(&content);
                    if match_count > 0 {
                        self.state.ui.scroll_to_match = true;
                    }
                    debug!("Search changed, found {} matches", match_count);
                }
            }

            // Handle navigation
            if output.next_requested {
                self.handle_find_next();
            }

            if output.prev_requested {
                self.handle_find_prev();
            }

            // Handle replace actions
            if output.replace_requested {
                self.handle_replace_current();
            }

            if output.replace_all_requested {
                self.handle_replace_all();
            }

            // Handle close
            if output.close_requested {
                self.state.ui.show_find_replace = false;
            }
        }
    }
}

impl eframe::App for FerriteApp {
    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle window resize for borderless window (must be early, before UI)
        // This detects mouse near edges, changes cursor, and initiates resize
        handle_window_resize(ctx, &mut self.window_resize_state);

        // Apply theme if needed (handles System theme changes)
        self.theme_manager.apply_if_needed(ctx);

        // Pre-warm font atlas if needed (deferred from font setup because
        // ctx.fonts() is not available until after first Context::run())
        fonts::check_and_prewarm_if_needed(ctx);

        // Track user interaction for idle detection
        // This updates the last interaction time when any user input is detected,
        // which is used to determine the appropriate repaint interval
        if self.had_user_input(ctx) {
            self.update_interaction_time();
        }

        // Lazy load CJK fonts when CJK content is detected (for faster startup)
        // Only loads the specific fonts needed for detected scripts (Korean/Japanese/Chinese)
        // This is much more memory efficient than loading all CJK fonts at once
        if let Some(tab) = self.state.active_tab() {
            if fonts::needs_cjk(&tab.content) {
                self.load_cjk_fonts_for_content(ctx, &tab.content);
            }
        }

        // Update toast message (clear if expired)
        let current_time = self.get_app_time();
        self.state.update_toast(current_time);

        // Update window title only if it changed (avoid viewport commands every frame)
        let title = self.window_title();
        if title != self.last_window_title {
            self.last_window_title = title.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }

        // Track window size/position changes for persistence
        self.update_window_state(ctx);

        // Handle drag-drop of files and folders
        self.handle_dropped_files(ctx);

        // Poll file watcher for workspace changes
        self.handle_file_watcher_events();

        // Handle automatic Git status refresh (focus, periodic, debounced)
        self.handle_git_auto_refresh(ctx);

        // Periodic session save for crash recovery
        self.update_session_recovery();

        // Process auto-save for tabs that need it
        self.process_auto_saves();

        // Show recovery dialog if we had a crash with unsaved changes
        self.show_recovery_dialog_if_needed(ctx);

        // Show auto-save recovery dialog if there's a pending recovery
        self.show_auto_save_recovery_dialog(ctx);

        // Handle close request from window
        if ctx.input(|i| i.viewport().close_requested()) && !self.handle_close_request() {
            // Cancel the close request - we need to show a confirmation dialog
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // IMPORTANT: Consume undo/redo keys BEFORE rendering to prevent egui's TextEdit
        // built-in undo from processing them. Must happen before render_ui().
        self.consume_undo_redo_keys(ctx);
        
        // IMPORTANT: Filter out Event::Cut when nothing is selected to prevent egui bug
        // where Ctrl+X cuts the entire document instead of doing nothing.
        self.filter_cut_event_if_no_selection(ctx);
        
        // IMPORTANT: Consume Alt+Arrow keys BEFORE rendering to prevent egui's TextEdit
        // from processing the arrow keys and moving the cursor before we can handle the move.
        // We save the direction and handle the move AFTER render so cursor updates stick.
        let move_line_direction = self.consume_move_line_keys(ctx);

        // IMPORTANT: Handle smart paste BEFORE rendering to intercept paste events
        // and transform them into markdown links/images when appropriate.
        self.consume_smart_paste(ctx);

        // Capture pre-render state for auto-close bracket detection
        let (pre_render_content, pre_render_cursor) = self.state.active_tab()
            .map(|tab| (tab.content.clone(), tab.cursors.primary().head))
            .unwrap_or_default();

        // IMPORTANT: Handle auto-close skip-over and selection wrapping BEFORE render
        // This consumes events that would otherwise be processed by TextEdit
        let auto_close_handled = self.handle_auto_close_pre_render(ctx);

        // Render the main UI (this updates editor selection)
        let deferred_format = self.render_ui(ctx);
        
        // Handle auto-close pair insertion AFTER render (if not already handled pre-render)
        if !auto_close_handled {
            self.handle_auto_close_post_render(&pre_render_content, pre_render_cursor);
        }

        // Handle move line AFTER render so cursor position updates are preserved
        if let Some(direction) = move_line_direction {
            self.handle_move_line(direction);
        }

        // Handle keyboard shortcuts AFTER render so selection is up-to-date
        // Note: Undo/redo is handled separately above, before render
        self.handle_keyboard_shortcuts(ctx);

        // Handle deferred format action from ribbon AFTER render so selection is up-to-date
        if let Some(cmd) = deferred_format {
            debug!("Applying deferred format command from ribbon: {:?}", cmd);
            self.handle_format_command(cmd);
        }

        // Try to expand snippets if enabled
        // This checks if a space/tab was just typed and expands any trigger word
        if self.state.settings.snippets_enabled && self.state.tab_count() > 0 {
            let tab_index = self.state.active_tab_index();
            self.try_expand_snippet(tab_index);
        }

        // Apply resize cursor at end of frame (to override any UI cursor settings)
        self.window_resize_state.apply_cursor(ctx);

        // Request exit if confirmed
        if self.should_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // ═══════════════════════════════════════════════════════════════════════
        // Frame Rate Diagnostics (Debug Only)
        // ═══════════════════════════════════════════════════════════════════════
        #[cfg(debug_assertions)]
        {
            self.frame_count += 1;
            let elapsed = self.last_fps_log.elapsed();
            if elapsed.as_secs() >= 5 {
                let fps = self.frame_count as f64 / elapsed.as_secs_f64();
                let needs_continuous = self.needs_continuous_repaint();
                let idle_secs = self.last_interaction_time.elapsed().as_secs_f32();
                let interval_ms = self.get_idle_repaint_interval().as_millis();
                
                // Log repaint causes to identify what's triggering repaints
                let repaint_causes = ctx.repaint_causes();
                let causes_str = if repaint_causes.is_empty() {
                    "none".to_string()
                } else {
                    repaint_causes.iter()
                        .take(3)  // Limit to first 3 causes
                        .map(|c| format!("{:?}", c))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                
                log::debug!(
                    "[REPAINT_DEBUG] FPS: {:.1}, continuous: {}, idle: {:.1}s, interval: {}ms, frames: {}, causes: {}",
                    fps, needs_continuous, idle_secs, interval_ms, self.frame_count, causes_str
                );
                self.frame_count = 0;
                self.last_fps_log = std::time::Instant::now();
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // Idle Repaint Optimization
        // ═══════════════════════════════════════════════════════════════════════
        // Schedule delayed repaints when idle to reduce CPU usage.
        // This is particularly important on macOS Intel where continuous repaints
        // can cause high CPU usage even when the app is idle.
        //
        // Uses tiered idle intervals:
        // - Active (ongoing animations/dialogs): Continuous repaint
        // - Light idle (recent interaction): 100ms (~10 FPS)
        // - Deep idle (no interaction for 2+ seconds): 500ms (~2 FPS)
        //
        // This significantly reduces CPU usage (from ~10% to <1%) when truly idle
        // while maintaining responsiveness during use.
        if !self.needs_continuous_repaint() {
            let interval = self.get_idle_repaint_interval();
            ctx.request_repaint_after(interval);
        }
    }

    /// Called when the application is about to close.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        use crate::config::{
            clear_all_recovery_data, remove_lock_file, save_session_state,
        };

        info!("Application exiting");

        // Capture and save session state for next startup
        let mut session_state = self.state.capture_session_state();
        session_state.mark_clean_shutdown();
        self.inject_csv_delimiters(&mut session_state);

        if save_session_state(&session_state) {
            info!("Session state saved for next startup");
            // Clear recovery data since we had a clean shutdown
            clear_all_recovery_data();
        } else {
            warn!("Failed to save session state");
        }

        // Remove lock file to indicate clean shutdown
        remove_lock_file();

        // Save settings
        self.state.shutdown();
    }

    /// Save persistent state.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        debug!("Saving application state");
        self.state.save_settings_if_dirty();
    }

    /// Whether to persist state.
    fn persist_egui_memory(&self) -> bool {
        true
    }

    /// Auto-save interval in seconds.
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a character index to line and column (0-indexed).
fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut current_index = 0;

    for ch in text.chars() {
        if current_index >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_index += 1;
    }

    (line, col)
}

/// Convert line and column (0-indexed) to a character index.
fn line_col_to_char_index(text: &str, target_line: usize, target_col: usize) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;
    let mut char_index = 0;

    for ch in text.chars() {
        if current_line == target_line && current_col == target_col {
            return char_index;
        }
        if ch == '\n' {
            if current_line == target_line {
                // Target column is beyond line end, return end of line
                return char_index;
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
        char_index += 1;
    }

    // Return end of text if target position is beyond text
    char_index
}
