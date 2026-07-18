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

mod central_panel;
pub(crate) mod commands;
mod dialogs;
mod export;
mod file_ops;
mod find_replace;
mod formatting;
mod helpers;
mod input_handling;
mod keyboard;
mod line_ops;
mod navigation;
mod platform;
mod status_bar;
mod title_bar;
mod types;

use crate::config::{apply_snippet, find_trigger_at_cursor, SnippetManager, ViewMode, WindowSize};
use crate::editor::{cleanup_ferrite_editor, DocumentOutline, DocumentStats, FindReplacePanel};
use crate::fonts;
use crate::markdown::{
    cleanup_rendered_editor_memory, delimiter_display_name, drain_code_execution_toasts,
    CsvViewerState, TreeViewerState,
};
pub use helpers::modifier_symbol;
use helpers::*;
use types::*;
// Note: SyncScrollState is available for future split-view sync scrolling
#[allow(unused_imports)]
use crate::preview::SyncScrollState;
use crate::state::{AppState, FileType, Selection};
use crate::theme::{ThemeColors, ThemeManager};
use crate::ui::{
    consume_clicks_in_resize_zones, handle_window_resize, load_app_logo_texture, AboutPanel,
    BacklinksPanel, CommandPalette, FileOperationDialog, FileTreeContextAction, FileTreePanel,
    FrontmatterPanel, OutlinePanel, ProductivityPanel, QuickSwitcher, Ribbon, RibbonAction,
    SearchPanel, SettingsPanel, TerminalPanel, TerminalPanelState, WelcomePanel, WindowResizeState,
};
use crate::vcs::GitAutoRefresh;

#[cfg(feature = "async-workers")]
use crate::workers::{echo_worker, WorkerCommand, WorkerHandle, WorkerResponse};

use eframe::egui;
use log::{debug, info, warn};
use rust_i18n::t;
use std::collections::HashMap;

#[derive(Debug, Default)]
struct RenderedChapterTracking {
    previous_scroll_offset: f32,
    previous_cursor_line: usize,
    initialized: bool,
    last_visible_heading_id: Option<String>,
}

/// Keyboard shortcut actions that need to be deferred.
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
    /// Welcome panel component
    welcome_panel: WelcomePanel,
    /// Find/replace panel component
    find_replace_panel: FindReplacePanel,
    /// Outline panel component
    outline_panel: OutlinePanel,
    /// Frontmatter editing panel (right side)
    frontmatter_panel: FrontmatterPanel,
    /// Backlinks panel component (rendered inside outline panel)
    backlinks_panel: BacklinksPanel,
    /// File tree panel component (for workspace mode)
    file_tree_panel: FileTreePanel,
    /// Quick file switcher (Ctrl+P) for workspace mode
    quick_switcher: QuickSwitcher,
    /// Command palette (Alt+Space) for quick command execution
    command_palette: CommandPalette,
    /// Pending command from palette to dispatch after render (avoids mid-render state mutation)
    pending_palette_command: Option<crate::config::ShortcutCommand>,
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
    /// Used for bidirectional scroll synchronization in split view
    sync_scroll_states: HashMap<usize, SyncScrollState>,
    /// Last heading seen in each tab's Rendered viewport.
    rendered_chapter_tracking: HashMap<usize, RenderedChapterTracking>,
    /// Track if we should exit (after confirmation)
    should_exit: bool,
    /// Last known window size (for detecting changes)
    last_window_size: Option<egui::Vec2>,
    /// Last known window position (for detecting changes)
    last_window_pos: Option<egui::Pos2>,
    /// Application start time for timing toast messages
    start_time: std::time::Instant,
    /// Previous active tab, view mode and viewer-layout option state.
    previous_view_mode: Option<(usize, ViewMode, bool)>,
    /// Effective editor size waiting to be committed after global GUI zoom returns to 100%.
    pending_editor_font_size: Option<f32>,
    /// Tracks previous LSP enabled state to detect toggle transitions.
    lsp_was_enabled: bool,
    /// Latest LSP status per server key (from `LspManagerEvent::StatusChanged`).
    lsp_status_by_server: std::collections::HashMap<String, crate::lsp::state::ServerStatus>,
    /// Workspace root last seen for LSP status (reset map when folder changes).
    lsp_status_workspace: Option<std::path::PathBuf>,
    /// Fingerprint of `lsp_server_overrides` to restart servers when paths change.
    lsp_overrides_fingerprint: u64,
    /// Paths already opened with `textDocument/didOpen` so we don't re-send.
    lsp_opened_docs: std::collections::HashSet<std::path::PathBuf>,
    /// Per-path LSP document version counters for `textDocument/didChange`.
    lsp_doc_versions: std::collections::HashMap<std::path::PathBuf, i32>,
    /// Per-path `last_edit_time` snapshot — only sync when it changes.
    lsp_last_edit_times: std::collections::HashMap<std::path::PathBuf, std::time::Instant>,
    /// Per-path debounce: when the last `didChange` was sent.
    lsp_last_change_sent: std::collections::HashMap<std::path::PathBuf, std::time::Instant>,
    /// Tab ID → (normalized_path, server_key) for tabs with an active didOpen.
    lsp_tab_server: std::collections::HashMap<usize, (std::path::PathBuf, String)>,
    /// Number of open documents per server key (for idle shutdown).
    lsp_open_doc_count: std::collections::HashMap<String, usize>,
    /// Instant when a server's doc count dropped to 0 (idle timer start).
    lsp_idle_since: std::collections::HashMap<String, std::time::Instant>,
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
    /// Paths from CLI / file association held until crash-recovery dialog is answered.
    pending_startup_paths: Vec<std::path::PathBuf>,
    /// Pending auto-save recovery info (for showing recovery dialog)
    pending_auto_save_recovery: Option<AutoSaveRecoveryInfo>,
    /// Snippet manager for text expansion
    snippet_manager: SnippetManager,
    /// Terminal panel component
    terminal_panel: TerminalPanel,
    /// Terminal panel state
    terminal_panel_state: TerminalPanelState,
    /// Productivity hub panel component
    productivity_panel: ProductivityPanel,
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
    /// Whether we've logged the one-shot window diagnostic on startup.
    window_diagnostic_logged: bool,
    /// Whether the platform-specific Alt+Space suppression has been applied.
    platform_init_done: bool,
    /// Flag indicating CJK font check is needed after a file was opened.
    /// Set when a file is opened during the UI render pass, checked at end of update().
    /// This ensures CJK fonts are loaded immediately rather than waiting for next frame.
    pending_cjk_check: bool,
    /// Last active tab index (for detecting tab switches to refresh backlinks)
    last_active_tab_for_backlinks: usize,
    /// Flag to trigger backlink refresh (set on file save or tab switch)
    backlinks_need_refresh: bool,
    /// Single-instance listener for receiving file paths from secondary instances
    instance_listener: Option<crate::single_instance::SingleInstanceListener>,
    /// Echo worker handle for async demo (lazy initialization)
    #[cfg(feature = "async-workers")]
    echo_worker: Option<WorkerHandle>,
    /// Input buffer for echo demo panel
    #[cfg(feature = "async-workers")]
    echo_demo_input: String,
    /// Receiver for background file loading messages (progress, completion, error).
    file_load_rx: std::sync::mpsc::Receiver<FileLoadMsg>,
    /// Sender cloned into each background loading thread.
    file_load_tx: std::sync::mpsc::Sender<FileLoadMsg>,
    /// Active background loading threads keyed by tab ID for cancellation on tab close.
    loading_tasks: HashMap<usize, std::thread::JoinHandle<()>>,
    /// Full-workspace file index for quick switcher and search-in-files.
    workspace_file_index: crate::workspaces::WorkspaceFileIndex,
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
        crate::diag::event_once("app_new", "FerriteApp::new started");
        crate::log_memory("App::new() start");

        // Set up custom fonts with lazy CJK loading for faster startup
        // CJK fonts will be loaded on-demand when CJK text is detected
        fonts::setup_fonts_lazy(&cc.egui_ctx);
        crate::log_memory("After fonts setup");

        // Set snappy/instant animations (default is ~83ms, we want instant)
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.animation_time = 0.0; // Instant - no animations

        // Push scrollbars a few pixels in from the panel edge so they don't
        // overlap with `SidePanel` resize hit zones. Without this, hovering
        // near the boundary between the central panel and a side panel makes
        // the cursor rapidly toggle between `ResizeHorizontal` and the
        // default arrow, which also flickers the scrollbar's hover state.
        // 6 px of outer margin is enough to clear the default
        // `interaction.resize_grab_radius_side` (5 px).
        style.spacing.scroll.bar_outer_margin = 6.0;
        // egui 0.34 adds scroll-edge fade gradients; disable to match pre-upgrade look.
        style.spacing.scroll.fade.strength = 0.0;

        cc.egui_ctx.set_global_style(style);

        // Configure egui options to reduce unnecessary repaints
        cc.egui_ctx.options_mut(|options| {
            // Disable repaint on widget change - this can cause constant repaints
            // when widgets move or IDs change
            options.repaint_on_widget_change = false;
        });

        // Load session state BEFORE creating lock file so that
        // check_and_clear_lock_file() detects the previous run's lock file,
        // not the one we're about to create for this session.
        crate::diag::trace("loading session state");
        let recovery_result = load_session_state();
        crate::diag::trace("session state loaded");

        // Now create lock file for this session - if we crash, the next
        // startup will find it and know recovery is needed.
        create_lock_file();
        let needs_recovery_dialog = recovery_result.is_crash_recovery
            && recovery_result
                .session
                .as_ref()
                .map(|s| s.has_unsaved_changes())
                .unwrap_or(false);

        crate::log_memory("Before AppState::new()");
        let mut state = AppState::new();
        crate::log_memory("After AppState::new()");

        // If we have a valid session to restore (but no crash with unsaved changes),
        // restore it silently - but only if restore_session is enabled in settings
        if !needs_recovery_dialog
            && recovery_result.session.is_some()
            && state.settings.restore_session
        {
            crate::diag::trace("restoring session tabs");
            if state.restore_from_session_result(&recovery_result) {
                info!("Session restored successfully");
            }
            crate::diag::trace("session restore done");
            crate::log_memory("After session restore");

            // Drop any `recovery/<id>.json` whose id is not in the freshly
            // restored tab set. Recovery files persist across launches but
            // tab ids reset every launch — without pruning, a leftover file
            // from a previous session can hijack an unrelated tab on a
            // future restore (silent data-loss hazard, see
            // `AppState::resolve_tab_content`).
            state.prune_stale_recovery_files();
        }

        // Initialize theme manager with saved theme preference
        let mut theme_manager = ThemeManager::new(state.settings.theme);
        theme_manager.sync_accent_rgb(state.settings.accent_color);

        // Apply initial theme to egui context
        theme_manager.apply(&cc.egui_ctx);
        info!("Applied initial theme: {:?}", state.settings.theme);

        // Reload fonts only if a CUSTOM font is specified (not for CJK preference alone)
        // CJK fonts are loaded lazily when CJK text is detected, not at startup.
        // This saves ~60-80 MB of RAM by not preloading all 4 CJK font files.
        let custom_font = state
            .settings
            .font_family
            .custom_name()
            .map(|s| s.to_string());
        if custom_font.is_some() {
            if let Some(reason) = fonts::reload_fonts(
                &cc.egui_ctx,
                custom_font.as_deref(),
                state.settings.cjk_font_preference,
                Some(&state.settings.complex_script_font_preferences),
            ) {
                warn!(
                    "Custom font failed at startup, reverting to Inter: {}",
                    reason
                );
                state.settings.font_family = crate::config::EditorFont::default();
                state.pending_toast =
                    Some(format!("Font failed to load: {reason}. Reverted to Inter."));
            } else {
                info!("Loaded custom font: {:?}", state.settings.font_family);
            }
        } else {
            // No custom font - check if we should preload CJK font
            // This loads only ONE CJK font (~20MB) based on preference or OS language
            if fonts::preload_explicit_cjk_font(&cc.egui_ctx, state.settings.cjk_font_preference) {
                // User has explicit CJK preference - preload that font so restored tabs render correctly
                info!(
                    "Preloaded CJK font for explicit preference: {:?}",
                    state.settings.cjk_font_preference
                );
            } else if fonts::preload_system_locale_cjk_font(
                &cc.egui_ctx,
                state.settings.cjk_font_preference,
            ) {
                // Auto mode - preload based on system locale detection
                info!("Preloaded CJK font for system locale");
            }
        }

        // If the UI language is CJK, ensure fonts are loaded regardless of the
        // checks above. This covers the case where a non-CJK-locale user chose
        // a CJK language — without this, the UI shows squares after restart.
        if let Some(lang_cjk) = state.settings.language.required_cjk_font() {
            if !fonts::are_cjk_fonts_loaded() {
                fonts::preload_explicit_cjk_font_with_custom(
                    &cc.egui_ctx,
                    lang_cjk,
                    custom_font.as_deref(),
                );
                info!(
                    "Preloaded CJK font for UI language {:?} at startup",
                    state.settings.language
                );
            }
        }
        crate::log_memory("After font configuration");

        // Initialize outline panel with saved settings
        let outline_panel = OutlinePanel::new()
            .with_width(state.settings.outline_width)
            .with_side(state.settings.outline_side);

        let frontmatter_panel = FrontmatterPanel::new();

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

        crate::log_memory("Before creating panels");

        // Create terminal panel components
        let terminal_panel = TerminalPanel::new();
        crate::log_memory("After TerminalPanel::new()");
        let terminal_panel_state = TerminalPanelState::new();
        crate::log_memory("After TerminalPanelState::new()");
        let productivity_panel = ProductivityPanel::new();
        crate::log_memory("After ProductivityPanel::new()");

        let lsp_overrides_fingerprint =
            crate::lsp::overrides_fingerprint(&state.settings.lsp_server_overrides);
        let lsp_was_enabled_init = state.settings.lsp_enabled;

        let (file_load_tx_init, file_load_rx_init) = std::sync::mpsc::channel();

        let mut app = Self {
            state,
            theme_manager,
            ribbon: Ribbon::new(),
            settings_panel: SettingsPanel::new(),
            about_panel: AboutPanel::new(),
            welcome_panel: WelcomePanel::new(),
            find_replace_panel: FindReplacePanel::new(),
            outline_panel,
            frontmatter_panel,
            backlinks_panel: BacklinksPanel::new(),
            file_tree_panel: FileTreePanel::new(),
            quick_switcher: QuickSwitcher::new(),
            command_palette: CommandPalette::new(),
            pending_palette_command: None,
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
            rendered_chapter_tracking: HashMap::new(),
            should_exit: false,
            last_window_size: None,
            last_window_pos: None,
            start_time: std::time::Instant::now(),
            previous_view_mode: None,
            pending_editor_font_size: None,
            lsp_was_enabled: lsp_was_enabled_init,
            lsp_status_by_server: std::collections::HashMap::new(),
            lsp_status_workspace: None,
            lsp_overrides_fingerprint,
            lsp_opened_docs: std::collections::HashSet::new(),
            lsp_doc_versions: std::collections::HashMap::new(),
            lsp_last_edit_times: std::collections::HashMap::new(),
            lsp_last_change_sent: std::collections::HashMap::new(),
            lsp_tab_server: std::collections::HashMap::new(),
            lsp_open_doc_count: std::collections::HashMap::new(),
            lsp_idle_since: std::collections::HashMap::new(),
            window_resize_state: WindowResizeState::new(),
            session_save_throttle: SessionSaveThrottle::default(),
            git_auto_refresh: GitAutoRefresh::new(),
            show_recovery_dialog,
            pending_recovery,
            pending_startup_paths: Vec::new(),
            pending_auto_save_recovery: None,
            snippet_manager,
            terminal_panel,
            terminal_panel_state,
            productivity_panel,
            #[cfg(debug_assertions)]
            frame_count: 0,
            #[cfg(debug_assertions)]
            last_fps_log: std::time::Instant::now(),
            last_interaction_time: std::time::Instant::now(),
            last_window_title: String::new(),
            app_logo_texture,
            window_diagnostic_logged: false,
            platform_init_done: false,
            pending_cjk_check: false,
            last_active_tab_for_backlinks: usize::MAX,
            backlinks_need_refresh: true,
            instance_listener: None,
            #[cfg(feature = "async-workers")]
            echo_worker: None, // Lazy - spawns when AI panel first shown
            #[cfg(feature = "async-workers")]
            echo_demo_input: String::new(),
            file_load_rx: file_load_rx_init,
            file_load_tx: file_load_tx_init,
            loading_tasks: HashMap::new(),
            workspace_file_index: crate::workspaces::WorkspaceFileIndex::new(),
        };

        // Restore CSV delimiter overrides from session if available
        if let Some(session) = session_for_csv {
            app.restore_csv_delimiters(&session);
        }

        // Restore command palette recent commands from settings
        let recent: std::collections::VecDeque<_> = app
            .state
            .settings
            .command_palette_recent
            .iter()
            .copied()
            .collect();
        app.command_palette.set_recent_commands(recent);

        crate::log_memory("App::new() complete");
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

        // Defer opening until the user chooses restore vs start fresh; restoring the
        // session clears all tabs and would drop files opened from a double-click.
        if self.show_recovery_dialog {
            info!(
                "Deferring {} startup path(s) until crash recovery dialog is answered",
                paths.len()
            );
            self.pending_startup_paths.extend(paths);
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
                let time = self.get_app_time();
                match self.open_file_smart(file_path.clone(), true, Some(time)) {
                    Ok(tab_idx) => {
                        if first_opened_tab_idx.is_none() {
                            first_opened_tab_idx = Some(tab_idx);
                        }
                        self.pending_cjk_check = true;
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

    /// Open the welcome tab on startup when no documents are open and the user
    /// has not disabled it (see `Settings::show_welcome_on_empty_launch`).
    pub fn open_welcome_on_startup(&mut self) {
        self.state.open_welcome_on_empty_launch();
    }
    /// Set the single-instance listener for receiving file paths from secondary instances.
    ///
    /// Called once during startup from `main()` after instance acquisition.
    pub fn set_instance_listener(
        &mut self,
        listener: crate::single_instance::SingleInstanceListener,
    ) {
        self.instance_listener = Some(listener);
    }

    /// Get elapsed time since app start in seconds.
    fn get_app_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Load CJK fonts on-demand for specific text content.
    ///
    /// This enables lazy CJK font loading - only the fonts needed for the detected
    /// scripts are loaded:
    /// - Korean text ΓåÆ loads only Korean font (~15-20MB)
    /// - Japanese text ΓåÆ loads only Japanese font (~15-20MB)
    /// - Chinese text ΓåÆ loads only Chinese font based on preference (~15-20MB)
    ///
    /// This is much more memory efficient than loading all CJK fonts at once.
    /// Returns `true` if any new fonts were loaded.
    fn load_cjk_fonts_for_content(&self, ctx: &egui::Context, content: &str) -> bool {
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
            Some(&self.state.settings.complex_script_font_preferences),
        )
    }

    /// Load complex script fonts on-demand for specific text content.
    ///
    /// Detects Arabic, Bengali, Devanagari, Thai, Hebrew, Tamil, etc. and loads
    /// only the necessary system fonts (~1-5MB each).
    fn load_complex_script_fonts_for_content(&self, ctx: &egui::Context, content: &str) -> bool {
        let custom_font = self
            .state
            .settings
            .font_family
            .custom_name()
            .map(|s| s.to_string());
        fonts::load_complex_script_fonts_for_text(
            content,
            ctx,
            custom_font.as_deref(),
            self.state.settings.cjk_font_preference,
            Some(&self.state.settings.complex_script_font_preferences),
        )
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

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Session Persistence (Crash Recovery)
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

    /// Update session recovery state - called every frame.
    ///
    /// This checks if enough time has passed since the last session save
    /// and saves a crash recovery snapshot if needed.
    fn update_session_recovery(&mut self) {
        use crate::config::save_crash_recovery_state;

        // Don't save crash recovery if we're exiting
        if self.should_exit {
            return;
        }

        // Mark session dirty if any document has unsaved edits (includes quick-note pathless tabs).
        if self.state.has_unsaved_changes() || self.state.any_modified_document_tab() {
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
                if let Some(tab) = self
                    .state
                    .tabs()
                    .iter()
                    .find(|t| t.path == session_tab.path)
                {
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
    /// - `navigation_parents` (linked-file Back navigation state)
    /// - `rendered_chapter_tracking` (Rendered-view chapter tracking state)
    /// - egui memory temp data (rendered editor widget states like
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
        self.state.ui.navigation_parents.remove(&tab_id);
        self.rendered_chapter_tracking.remove(&tab_id);

        // Drop the per-tab crash-recovery file. Recovery files are persistent
        // and keyed by `tab_id`, but tab ids reset every launch — leaving a
        // file behind can cause it to bleed into an unrelated tab on a future
        // restore (see `AppState::resolve_tab_content`). Always do this from
        // every close path so it runs whether the tab was discarded or saved.
        crate::config::delete_recovery_content(tab_id);

        // Cancel any in-progress background file loading for this tab.
        // Dropping the JoinHandle detaches the thread; it will notice the
        // channel receiver is gone when it tries to send the next message.
        self.loading_tasks.remove(&tab_id);

        // Send didClose and update doc count if this tab had an active LSP session
        if let Some((norm_path, server_key)) = self.lsp_tab_server.remove(&tab_id) {
            let still_open = self.state.tabs().iter().any(|t| {
                t.path
                    .as_ref()
                    .map(|p| crate::lsp::normalize_lsp_path(p) == norm_path)
                    .unwrap_or(false)
            });
            if !still_open {
                let uri = crate::lsp::path_to_uri(&norm_path);
                self.state.lsp.did_close(&server_key, uri);
                self.lsp_opened_docs.remove(&norm_path);
                self.lsp_doc_versions.remove(&norm_path);
                self.lsp_last_edit_times.remove(&norm_path);
                self.lsp_last_change_sent.remove(&norm_path);
                log::debug!("LSP didClose on tab close: {}", norm_path.display());
            }
            if let Some(count) = self.lsp_open_doc_count.get_mut(&server_key) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.lsp_idle_since
                        .insert(server_key.clone(), std::time::Instant::now());
                    log::debug!("LSP server {} idle (0 open docs)", server_key);
                }
            }
        }

        // Clean up egui temporary data for rendered editor widgets
        if let Some(ctx) = ctx {
            cleanup_rendered_editor_memory(ctx);
            cleanup_ferrite_editor(ctx, tab_id);
        }

        // Clear global caches that may hold large data from the closed tab.
        // The AST cache stores full parsed document trees keyed by content hash;
        // for large files this can retain tens of MB after the tab is gone.
        crate::markdown::cache::clear_ast_cache();
        crate::markdown::cache::clear_block_height_cache();

        // Clear find/replace matches — these can be very large for big files
        self.state.ui.find_state.clear();

        // After freeing large data, ask the allocator to return pages to the OS.
        // Without this, mimalloc keeps freed arenas mapped and RSS stays inflated.
        crate::purge_allocator_caches();
    }

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Snippet Expansion
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

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
            None => {
                return false;
            }
        };

        // Get cursor position (in characters)
        let cursor_char = tab.cursors.primary().head;

        // Convert to byte position for snippet detection
        let cursor_byte = tab
            .content
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
        if let Some(snippet_match) =
            find_trigger_at_cursor(content, before_trigger_byte, &self.snippet_manager)
        {
            // Apply the snippet expansion
            let (new_content, new_cursor_byte) = apply_snippet(content, &snippet_match);

            // Add back the trigger character (space/tab)
            let final_content = format!(
                "{}{}",
                &new_content[..new_cursor_byte],
                &content[before_trigger_byte..]
            );
            let final_cursor_byte = new_cursor_byte + 1; // +1 for the space/tab

            // Convert new cursor position back to character position
            let final_cursor_char = final_content[..final_cursor_byte.min(final_content.len())]
                .chars()
                .count();

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

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Idle Detection for CPU Optimization
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

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
        if self.state.ui.show_confirm_dialog || self.state.ui.show_error_modal {
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
            if i.smooth_scroll_delta() != egui::Vec2::ZERO {
                return true;
            }
            // Check for any events (key, mouse, paste, etc.)
            !i.events.is_empty()
        })
    }

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Auto-Save Processing
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

    /// Process auto-save for all tabs that need it.
    ///
    /// This is called every frame and checks each tab to see if auto-save
    /// should trigger based on idle time. Uses temp files to avoid
    /// overwriting the main file prematurely.
    fn process_auto_saves(&mut self) {
        use crate::config::save_auto_save_content;

        let delay_ms = self.state.settings.auto_save_delay_ms;
        let tab_count = self.state.tab_count();

        // Collect tabs that need auto-save (indices and info). The
        // `disk_content_hash` snapshot is captured at this moment so the
        // autosave file records the disk identity it was anchored to —
        // recovery uses this to detect external edits between sessions
        // (task 106 — hardened recovery).
        let mut tabs_to_save: Vec<(
            usize,
            usize,
            Option<std::path::PathBuf>,
            String,
            Option<u64>,
        )> = Vec::new();

        for i in 0..tab_count {
            if let Some(tab) = self.state.tab(i) {
                if tab.should_auto_save(delay_ms) {
                    tabs_to_save.push((
                        i,
                        tab.id,
                        tab.path.clone(),
                        tab.content.clone(),
                        tab.disk_content_hash(),
                    ));
                }
            }
        }

        // Process auto-saves
        for (index, tab_id, path, content, disk_hash) in tabs_to_save {
            // Save to temp file
            if save_auto_save_content(tab_id, path.as_ref(), &content, disk_hash) {
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
        if let Some((metadata, recovered_content)) = check_auto_save_recovery(tab_id, path.as_ref())
        {
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

        egui::Window::new(t!("recovery.auto_save.title").to_string())
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
                let saved_time =
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs(recovery_info.saved_at);
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
                    if ui
                        .button(egui::RichText::new(
                            t!("recovery.auto_save.restore").to_string(),
                        ))
                        .clicked()
                    {
                        should_restore = true;
                    }
                    if ui
                        .button(egui::RichText::new(
                            t!("recovery.auto_save.discard").to_string(),
                        ))
                        .clicked()
                    {
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
                    self.state.show_toast(
                        t!("notification.restored_auto_save").to_string(),
                        time,
                        3.0,
                    );
                    info!(
                        "Restored auto-save content for tab {}",
                        recovery_info.tab_id
                    );
                }
            }
            // Delete the auto-save file after restore
            delete_auto_save(recovery_info.tab_id, recovery_info.path.as_ref());
        } else if should_discard {
            // Delete the auto-save file
            delete_auto_save(recovery_info.tab_id, recovery_info.path.as_ref());
            let time = self.get_app_time();
            self.state.show_toast(
                t!("notification.auto_save_discarded").to_string(),
                time,
                2.0,
            );
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

        egui::Window::new(format!("{}?", t!("recovery.session.title")))
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
                            .button(t!("recovery.session.restore").to_string())
                            .on_hover_text(t!("recovery.session.restore_tooltip").to_string())
                            .clicked()
                        {
                            restore = true;
                        }

                        ui.add_space(8.0);

                        if ui
                            .button(t!("recovery.session.start_fresh").to_string())
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
                    self.state.show_toast(
                        t!("notification.session_restored").to_string(),
                        current_time,
                        3.0,
                    );
                    // Restore CSV delimiter overrides
                    if let Some(session) = session {
                        self.restore_csv_delimiters(&session);
                    }
                }
            }
            // Clear recovery data after successful restore
            clear_all_recovery_data();
            self.show_recovery_dialog = false;
            self.open_deferred_startup_paths();
        } else if discard {
            info!("User discarded crash recovery");
            clear_all_recovery_data();
            self.pending_recovery = None;
            self.show_recovery_dialog = false;
            self.open_deferred_startup_paths();
        }
    }

    /// Open paths deferred while the crash recovery dialog was shown.
    fn open_deferred_startup_paths(&mut self) {
        let paths = std::mem::take(&mut self.pending_startup_paths);
        if paths.is_empty() {
            return;
        }
        info!(
            "Opening {} deferred startup path(s) after crash recovery choice",
            paths.len()
        );
        self.open_initial_paths(paths);
    }

    /// Render the main UI content.
    /// Returns a deferred format action if one was requested from the ribbon.
    fn render_ui(&mut self, ui: &mut egui::Ui) -> Option<DeferredFormatAction> {
        let ctx = ui.ctx().clone();
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_dark = ctx.global_style().visuals.dark_mode;
        let zen_mode = self.state.is_zen_mode();

        // Title bar colors
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

        // Render custom title bar
        self.render_title_bar(
            ui,
            is_maximized,
            is_dark,
            zen_mode,
            title_bar_color,
            button_hover_color,
            close_hover_color,
            text_color,
        );

        // Activate the md-viewer-style layout when the active tab changes or
        // when its view mode changes to Rendered. Tracking the tab ID as well as
        // the mode also handles switching between two already-rendered tabs.
        let current_view_context = self.state.active_tab().map(|tab| {
            (
                tab.id,
                tab.view_mode,
                self.state.settings.show_viewer_layout_in_rendered_mode,
            )
        });

        if self.previous_view_mode != current_view_context {
            let previous_view_context = self.previous_view_mode;
            self.previous_view_mode = current_view_context;

            let should_activate = current_view_context
                .map(|(_, mode, enabled)| mode == ViewMode::Rendered && enabled)
                .unwrap_or(false);
            let should_deactivate = previous_view_context
                .map(|(_, mode, enabled)| mode == ViewMode::Rendered && enabled)
                .unwrap_or(false)
                && !should_activate;

            if should_activate {
                self.activate_viewer_layout_for_active_file();
            } else if should_deactivate {
                self.deactivate_viewer_layout();
            }
        }

        // View menu removed - Productivity Hub is now accessible via ribbon icon and outline panel tab

        // Ribbon panel (below view menu) - hidden in Zen Mode
        let ribbon_action = if !zen_mode {
            // Get state needed for ribbon
            let theme = self.state.settings.theme;
            let view_mode = self
                .state
                .active_tab()
                .map(|t| t.view_mode)
                .unwrap_or(ViewMode::Raw);
            let show_line_numbers = self.state.settings.show_line_numbers;
            let new_file_enabled = !(self.state.settings.disable_new_file_in_rendered_mode
                && view_mode == ViewMode::Rendered);
            let can_save = self
                .state
                .active_tab()
                .map(|t| t.path.is_some() && t.is_modified())
                .unwrap_or(false);

            let theme_colors = ThemeColors::from_theme(
                theme,
                &ctx.global_style().visuals,
                self.state.settings.ferrite_accent_rgb(),
            );

            let ribbon_bg = if is_dark {
                egui::Color32::from_rgb(40, 40, 40)
            } else {
                egui::Color32::from_rgb(248, 248, 248)
            };

            let mut action = None;
            egui::Panel::top("ribbon")
                .frame(
                    egui::Frame::new()
                        .fill(ribbon_bg)
                        .stroke(egui::Stroke::NONE)
                        .inner_margin(egui::Margin::symmetric(4, 4)),
                )
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    // Get formatting state for active editor
                    let formatting_state = self.state.active_tab().map(|tab| {
                        get_formatting_state_for(
                            &tab.content,
                            tab.cursor_position.0,
                            tab.cursor_position.1,
                        )
                    });

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
                        can_save,
                        self.state.active_tab().is_some(),
                        new_file_enabled,
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
        // IMPORTANT: Capture selection NOW, before the editor might lose focus
        let mut deferred_format_action = if let Some(action) = ribbon_action {
            match action {
                RibbonAction::Format(cmd) => {
                    // Capture the current selection state from FerriteEditor
                    use crate::editor::get_ferrite_editor_mut;
                    let tab_id = self.state.active_tab().map(|t| t.id);
                    let selection = tab_id.and_then(|id| {
                        get_ferrite_editor_mut(&ctx, id, |editor| {
                            let sel = editor.selection();
                            let (start, end) = sel.ordered();

                            // Convert character positions to byte positions for the formatting function
                            let content = editor.buffer().to_string();
                            let line_count = editor.buffer().line_count();

                            // Clamp lines to valid range to prevent panics
                            let start_line = start.line.min(line_count.saturating_sub(1));
                            let end_line = end.line.min(line_count.saturating_sub(1));

                            // Use try_line_to_char for safety
                            let start_line_char = editor
                                .buffer()
                                .try_line_to_char(start_line)
                                .unwrap_or(0);
                            let end_line_char = editor
                                .buffer()
                                .try_line_to_char(end_line)
                                .unwrap_or(0);

                            let start_char = start_line_char + start.column;
                            let end_char = end_line_char + end.column;

                            // Convert char indices to byte indices
                            let start_byte = crate::string_utils::char_index_to_byte_index(
                                &content,
                                start_char
                            );
                            let end_byte = crate::string_utils::char_index_to_byte_index(
                                &content,
                                end_char
                            );

                            // Get preview of selected text for debugging
                            let selected_preview: String = content
                                .chars()
                                .skip(start_char)
                                .take(end_char.saturating_sub(start_char).min(30))
                                .collect();

                            debug!(
                                "Capturing selection for format: cursor=({},{}) to ({},{}), chars={}..{}, bytes={}..{}, selected='{}'",
                                start.line,
                                start.column,
                                end.line,
                                end.column,
                                start_char,
                                end_char,
                                start_byte,
                                end_byte,
                                selected_preview
                            );
                            (start_byte, end_byte)
                        })
                    });
                    if selection.is_none() {
                        debug!(
                            "WARNING: No selection captured for format action {:?} - FerriteEditor may not be initialized yet",
                            cmd
                        );
                    } else {
                        debug!(
                            "Deferred format action: {:?}, selection={:?}",
                            cmd, selection
                        );
                    }
                    Some(DeferredFormatAction { cmd, selection })
                }
                other => {
                    self.handle_ribbon_action(other, &ctx);
                    None
                }
            }
        } else {
            None
        };

        // Status bar - hidden in Zen Mode
        if !zen_mode {
            let (rainbow_toggle, encoding_change) = self.render_status_bar(ui, is_dark);
            if rainbow_toggle {
                self.state.settings.csv_rainbow_columns = !self.state.settings.csv_rainbow_columns;
                self.state.mark_settings_dirty();
            }
            if let Some(encoding) = encoding_change {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.current_encoding = encoding;
                }
            }
        }

        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
        // Outline Panel (if enabled) - hidden in Zen Mode
        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
        let mut outline_nav_request: Option<HeadingNavRequest> = None;
        let mut outline_toggled_id: Option<String> = None;
        let mut outline_new_width: Option<f32> = None;
        let mut outline_close_requested = false;
        let mut outline_detach_productivity = false;

        // Track backlink navigation request from outline panel
        let mut backlink_navigate_to: Option<std::path::PathBuf> = None;

        // Side panel toggle strip (shown when outline panel is closed, hidden in Zen Mode)
        let blocks_resize_clicks = self.window_resize_state.blocks_widget_clicks();
        if !self.state.settings.outline_enabled && !zen_mode {
            if crate::ui::side_panel_toggle_strip(ui, is_dark, blocks_resize_clicks) {
                self.outline_panel.reset_default_view_state();
                self.state.settings.outline_enabled = true;
                self.state.mark_settings_dirty();
            }
        }

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

            // Detect tab switch and refresh backlinks
            let current_tab_idx = self.state.active_tab_index();
            if current_tab_idx != self.last_active_tab_for_backlinks {
                self.last_active_tab_for_backlinks = current_tab_idx;
                self.backlinks_need_refresh = true;
            }

            // Refresh backlinks if needed (tab switch or file save)
            if self.backlinks_need_refresh {
                self.refresh_backlinks();
                self.backlinks_need_refresh = false;
            }

            // Update frontmatter panel from current content (markdown files only)
            let is_markdown = self
                .state
                .active_tab()
                .map(|t| t.file_type() == FileType::Markdown)
                .unwrap_or(false);
            if is_markdown {
                if let Some(tab) = self.state.active_tab() {
                    let tab_id = tab.id;
                    let ver = tab.content_version();
                    let content_ref: &str = &tab.content;
                    self.frontmatter_panel
                        .update_from_content_versioned(content_ref, tab_id, ver);
                }
            }

            // Configure and render the outline panel
            let visible_rendered_chapter = if self.state.settings.mark_last_visible_chapter {
                self.update_rendered_chapter_marker()
            } else {
                None
            };
            self.outline_panel
                .set_side(self.state.settings.outline_side);
            self.outline_panel.set_current_section(current_section);
            self.outline_panel
                .set_visible_rendered_chapter(visible_rendered_chapter);
            let docked = self.state.settings.productivity_panel_docked;
            let outline_output = self.outline_panel.show(
                ui,
                &self.cached_outline,
                self.state.settings.show_chapters_tab,
                self.state.settings.chapters_default_view,
                self.state.settings.mark_last_visible_chapter,
                self.state.settings.show_code_in_chapters,
                self.state.settings.show_images_in_chapters,
                self.cached_doc_stats.as_ref(),
                is_dark,
                self.state.settings.ferrite_accent_rgb(),
                if docked {
                    Some(&mut self.productivity_panel)
                } else {
                    None
                },
                Some(&self.backlinks_panel),
                if is_markdown {
                    Some(&mut self.frontmatter_panel)
                } else {
                    None
                },
            );

            // Capture output for processing after render
            if let Some(line) = outline_output.scroll_to_line {
                outline_nav_request = Some(HeadingNavRequest {
                    line,
                    title: outline_output.scroll_to_title,
                });
            }
            outline_toggled_id = outline_output.toggled_id;
            outline_new_width = outline_output.new_width;
            outline_close_requested = outline_output.close_requested;
            outline_detach_productivity = outline_output.detach_productivity;
            backlink_navigate_to = outline_output.backlink_navigate_to;

            // Handle frontmatter edits from the outline panel's frontmatter tab
            if let Some(new_content) = outline_output.frontmatter_new_content {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.content = new_content;
                    tab.notify_external_content_change();
                }
            }

            // Handle repaint request from productivity panel (e.g. timer)
            if outline_output.needs_repaint {
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            }
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

        // Handle productivity panel detach request from outline panel
        if outline_detach_productivity {
            self.state.settings.productivity_panel_docked = false;
            self.state.settings.productivity_panel_visible = true;
            // Switch outline panel back to Outline tab
            self.outline_panel
                .set_active_tab(crate::ui::OutlinePanelTab::Outline);
            self.state.mark_settings_dirty();
        }

        // Handle backlink navigation (click on a backlink entry)
        if let Some(path) = backlink_navigate_to {
            info!("Navigating to backlink: {}", path.display());
            let time = self.get_app_time();
            match self.state.open_file(path.clone(), Some(time)) {
                Ok(tab_index) => {
                    self.pending_cjk_check = true;
                    self.check_auto_save_recovery(tab_index);
                    debug!("Opened backlink target in tab {}", tab_index);
                }
                Err(e) => {
                    warn!("Failed to open backlink target: {}", e);
                    let time = self.get_app_time();
                    self.state.show_toast(
                        format!("Could not open {}: {}", path.display(), e),
                        time,
                        3.0,
                    );
                }
            }
        }

        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
        // (Frontmatter is now rendered inside the outline panel's FM tab)

        // File Tree Panel (workspace mode only) - hidden in Zen Mode
        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
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
                    ui,
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
            let time = self.get_app_time();
            match self.open_file_smart(file_path.clone(), true, Some(time)) {
                Ok(_) => {
                    self.pending_cjk_check = true;
                    debug!("Opened file from tree: {}", file_path.display());
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

        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
        // Live Pipeline Panel (Bottom panel for JSON/YAML command piping)
        // Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰Î“Ã²Ã‰
        // Only show if:
        // 1. Pipeline feature is enabled globally
        // 2. Not in Zen Mode (hide for distraction-free writing)
        // 3. Active tab is JSON/YAML and has pipeline panel visible
        let show_pipeline = self.state.settings.pipeline_enabled
            && !zen_mode
            && self
                .state
                .active_tab()
                .map(|t| t.supports_pipeline() && t.pipeline_visible())
                .unwrap_or(false);

        if show_pipeline {
            let panel_height = self.pipeline_panel.height();
            egui::Panel::bottom("pipeline_panel")
                .resizable(false) // We handle resize ourselves
                .exact_size(panel_height)
                .show_inside(ui, |ui| {
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
                    let working_dir = self
                        .state
                        .active_tab()
                        .and_then(|t| t.path.as_ref())
                        .and_then(|p| p.parent())
                        .map(|p| p.to_path_buf())
                        .or_else(|| self.state.workspace.as_ref().map(|w| w.root_path.clone()));

                    // Get content and tab state
                    let content = self
                        .state
                        .active_tab()
                        .map(|t| t.content.clone())
                        .unwrap_or_default();

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

        // Terminal panel (bottom panel, shown when visible)
        // Similar to pipeline panel but for integrated terminal
        if self.terminal_panel_state.is_visible() && !zen_mode {
            let panel_height = self.terminal_panel_state.height;
            egui::Panel::bottom("terminal_panel")
                .resizable(false) // We handle resize ourselves
                .exact_size(panel_height)
                .show_inside(ui, |ui| {
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
                        let new_height = (panel_height + delta).clamp(100.0, 3000.0);
                        if (new_height - panel_height).abs() > 0.5 {
                            self.terminal_panel_state.height = new_height;
                            self.state.settings.terminal_panel_height = new_height;
                            self.state.mark_settings_dirty();
                        }
                    }

                    // Show the terminal panel UI
                    let output = self.terminal_panel.show(
                        ui,
                        &mut self.terminal_panel_state,
                        &self.state.settings,
                        is_dark,
                    );

                    // Handle panel close
                    if output.closed {
                        self.terminal_panel_state.visible = false;
                    }
                });

            // Handle terminal errors as toast notifications
            let time = ctx.input(|i| i.time);
            if let Some(error_msg) = self.terminal_panel_state.take_error() {
                self.state.show_toast(error_msg, time, 4.0);
            }

            // Check for exited terminal processes
            for msg in self.terminal_panel_state.check_exited_terminals() {
                log::info!("{}", msg);
                self.state.show_toast(msg, time, 3.0);
            }
        }

        // Echo Demo Panel (placeholder for AI Assistant)
        // This demonstrates async workers via lazy initialization
        #[cfg(feature = "async-workers")]
        if self.state.settings.ai_panel_visible {
            egui::Window::new("Echo Demo (AI Panel Placeholder)")
                .open(&mut self.state.settings.ai_panel_visible)
                .default_width(400.0)
                .default_height(300.0)
                .show(&ctx, |ui| {
                    ui.label(t!("echo.prompt").to_string());

                    // Input field with state
                    ui.add_space(8.0);

                    let text_edit = ui.text_edit_singleline(&mut self.echo_demo_input);

                    // Send message on Enter
                    if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Some(worker) = &self.echo_worker {
                            if !self.echo_demo_input.is_empty() {
                                let _ = worker
                                    .command_tx
                                    .send(WorkerCommand::Echo(self.echo_demo_input.clone()));
                                self.echo_demo_input.clear();
                            }
                        }
                    }

                    ui.separator();
                    ui.label(t!("echo.responses").to_string());

                    // Poll for responses (non-blocking)
                    if let Some(worker) = &self.echo_worker {
                        while let Ok(response) = worker.response_rx.try_recv() {
                            if let WorkerResponse::Echo(msg) = response {
                                ui.label(msg);
                            }
                        }
                    }

                    ui.separator();
                    ui.label(t!("echo.replacement_note").to_string());
                    ui.label(
                        "Demonstrates: lazy worker spawn, mpsc communication, non-blocking UI.",
                    );
                });
        }

        // Productivity Hub Panel (floating/detached mode only)
        if self.state.settings.productivity_panel_visible
            && !self.state.settings.productivity_panel_docked
        {
            // Pass the current sidebar width so the floating window opens at
            // the same size as the dock did (avoids a jarring resize on
            // first detach and gives `Resize` a sensible upper bound).
            let dock_width = self.state.settings.outline_width;
            let ferrite_accent_panel = self.state.settings.ferrite_accent_rgb();
            self.productivity_panel.show(
                &ctx,
                &mut self.state.settings.productivity_panel_visible,
                dock_width,
                ferrite_accent_panel,
            );

            // Check if user clicked "Dock" to re-attach to outline panel
            if self.productivity_panel.take_dock_request() {
                self.state.settings.productivity_panel_docked = true;
                self.state.settings.productivity_panel_visible = false;
                self.state.settings.outline_enabled = true;
                self.outline_panel
                    .set_active_tab(crate::ui::OutlinePanelTab::Productivity);
                self.state.mark_settings_dirty();
            }
        }

        // Central panel for editor content

        // Central panel for editor content
        let central_deferred = self.render_central_panel(ui, is_dark);
        if central_deferred.is_some() {
            deferred_format_action = central_deferred;
        }

        deferred_format_action
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
                    info!(
                        "Auto-save {} for tab {}",
                        if tab.auto_save_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
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

                if !self.state.settings.sync_scroll_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.clear_sync_pending_scroll();
                    }
                    if let Some(tab_id) = self.state.active_tab().map(|t| t.id) {
                        if let Some(state) = self.sync_scroll_states.get_mut(&tab_id) {
                            state.reset_on_disable();
                        }
                    }
                }

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
                self.handle_open_html_export_dialog();
            }
            RibbonAction::CopyAsHtml => {
                debug!("Ribbon: Copy as HTML");
                self.handle_copy_as_html();
            }
            RibbonAction::ExportPdf => {
                debug!("Ribbon: Export PDF");
                self.handle_open_pdf_export_dialog();
            }
            RibbonAction::PrintPreview => {
                debug!("Ribbon: Print preview");
                self.handle_print_preview(ctx);
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
                self.handle_format_command(ctx, cmd);
            }

            // Markdown document operations
            RibbonAction::InsertToc => {
                debug!("Ribbon: Insert/Update TOC");
                self.handle_insert_toc();
            }

            // Terminal
            RibbonAction::ToggleTerminal => {
                debug!("Ribbon: Toggle Terminal");
                self.handle_toggle_terminal();
            }
            RibbonAction::ToggleProductivity => {
                debug!("Ribbon: Toggle Productivity Hub");
                if self.state.settings.productivity_panel_docked {
                    // When docked, toggle the outline panel and switch to Productivity tab
                    if self.state.settings.outline_enabled
                        && self.outline_panel.active_tab()
                            == crate::ui::OutlinePanelTab::Productivity
                    {
                        // Already showing productivity tab - close the panel
                        self.state.settings.outline_enabled = false;
                    } else {
                        // Open outline panel and switch to Productivity tab
                        self.state.settings.outline_enabled = true;
                        self.outline_panel
                            .set_active_tab(crate::ui::OutlinePanelTab::Productivity);
                    }
                } else {
                    // When undocked, toggle the floating window
                    self.state.settings.productivity_panel_visible =
                        !self.state.settings.productivity_panel_visible;
                }
                self.state.mark_settings_dirty();
            }
            RibbonAction::ToggleFrontmatter => {
                debug!("Ribbon: Toggle Frontmatter tab in outline panel");
                if !self.state.settings.outline_enabled {
                    self.state.settings.outline_enabled = true;
                }
                self.outline_panel
                    .set_active_tab(crate::ui::OutlinePanelTab::Frontmatter);
                self.state.mark_settings_dirty();
            }
        }
    }

    /// Capture dynamic Ctrl+/Ctrl-/Ctrl+wheel zoom as the editor's configured
    /// font size when the carry-forward option is enabled.
    ///
    /// Applying the size is deferred by one frame so the current frame keeps
    /// the same apparent size while egui returns its global UI zoom to 100%.
    fn sync_dynamic_editor_font_size(&mut self, ctx: &egui::Context) {
        if let Some(font_size) = self.pending_editor_font_size.take() {
            let font_size = font_size.clamp(
                crate::config::Settings::MIN_FONT_SIZE,
                crate::config::Settings::MAX_FONT_SIZE,
            );
            if (self.state.settings.font_size - font_size).abs() > 0.01 {
                self.state.settings.font_size = font_size;
                self.state.mark_settings_dirty();
            }
        }

        if !self.state.settings.use_last_font_size_for_new_files {
            return;
        }

        let zoom_factor = ctx.zoom_factor();
        if (zoom_factor - 1.0).abs() <= 0.001 {
            return;
        }

        self.pending_editor_font_size = Some((self.state.settings.font_size * zoom_factor).clamp(
            crate::config::Settings::MIN_FONT_SIZE,
            crate::config::Settings::MAX_FONT_SIZE,
        ));
        ctx.set_zoom_factor(1.0);
    }

    /// Current apparent editor font size, including any live GUI zoom that has
    /// not yet been captured into settings.
    pub(crate) fn current_editor_font_size(&self, ctx: &egui::Context) -> f32 {
        self.pending_editor_font_size
            .unwrap_or(self.state.settings.font_size * ctx.zoom_factor())
            .clamp(
                crate::config::Settings::MIN_FONT_SIZE,
                crate::config::Settings::MAX_FONT_SIZE,
            )
    }
}

impl eframe::App for FerriteApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.sync_dynamic_editor_font_size(&ctx);

        // Consume command palette shortcut BEFORE render to suppress OS system menu
        // (Alt+Space on Windows/Linux) and prevent TextEdit from eating the key.
        self.consume_command_palette_key(&ctx);

        // IMPORTANT: Consume undo/redo keys BEFORE rendering to prevent egui's TextEdit
        // built-in undo from processing them. Must happen before render_ui().
        self.consume_undo_redo_keys(&ctx);

        // NOTE: Event::Cut filter removed - FerriteEditor handles cut correctly
        // The old filter checked Tab.cursors which isn't synced with FerriteEditor.selections
        // FerriteEditor's cut handler already checks has_selection() before cutting

        // IMPORTANT: Consume Alt+Arrow keys BEFORE rendering to prevent egui's TextEdit
        // from processing the arrow keys and moving the cursor before we can handle the move.
        // We save the direction and handle the move AFTER render so cursor updates stick.
        let move_line_direction = self.consume_move_line_keys(&ctx);

        // IMPORTANT: Handle smart paste BEFORE rendering to intercept paste events
        // and transform them into markdown links/images when appropriate.
        self.consume_smart_paste(&ctx);

        // PERFORMANCE: Only capture pre-render state for auto-close if enabled AND file is small
        // Cloning large content (5MB+) every frame is extremely expensive
        let is_large_file = self
            .state
            .active_tab()
            .map(|t| t.is_large_file())
            .unwrap_or(false);
        let auto_close_enabled = self.state.settings.auto_close_brackets && !is_large_file;

        let (pre_render_content, pre_render_cursor) = if auto_close_enabled {
            self.state
                .active_tab()
                .map(|tab| (tab.content.clone(), tab.cursors.primary().head))
                .unwrap_or_default()
        } else {
            (String::new(), 0)
        };

        // IMPORTANT: Handle auto-close skip-over and selection wrapping BEFORE render
        // This consumes events that would otherwise be processed by TextEdit
        let auto_close_handled = if auto_close_enabled {
            self.handle_auto_close_pre_render(&ctx)
        } else {
            false
        };

        // Ensure echo worker is spawned if AI panel is visible (lazy initialization)
        #[cfg(feature = "async-workers")]
        self.ensure_echo_worker();

        // Render the main UI (this updates editor selection)
        let deferred_format = self.render_ui(ui);

        for msg in drain_code_execution_toasts(&ctx) {
            self.state.show_toast(msg, self.get_app_time(), 5.0);
        }

        // Handle auto-close pair insertion AFTER render (if not already handled pre-render)
        if auto_close_enabled && !auto_close_handled {
            self.handle_auto_close_post_render(&pre_render_content, pre_render_cursor);
        }

        // Handle move line AFTER render so cursor position updates are preserved
        if let Some(direction) = move_line_direction {
            self.handle_move_line(direction);
        }

        // Handle keyboard shortcuts AFTER render so selection is up-to-date
        // Note: Undo/redo is handled separately above, before render
        self.handle_keyboard_shortcuts(&ctx);

        // Dispatch any pending command palette action AFTER render
        if let Some(cmd) = self.pending_palette_command.take() {
            self.dispatch_palette_command(&ctx, cmd);
        }

        // Handle deferred format action from ribbon with pre-captured selection
        if let Some(deferred) = deferred_format {
            debug!(
                "Applying deferred format command from ribbon: {:?} with selection {:?}",
                deferred.cmd, deferred.selection
            );
            self.handle_format_command_with_selection(&ctx, deferred.cmd, deferred.selection);
            // Restore focus to the editor after applying formatting
            // This ensures the editor keeps focus even though the button click stole it
            if let Some(tab) = self.state.active_tab_mut() {
                tab.needs_focus = true;
            }
        }

        // Try to expand snippets if enabled
        // This checks if a space/tab was just typed and expands any trigger word
        if self.state.settings.snippets_enabled && self.state.tab_count() > 0 {
            let tab_index = self.state.active_tab_index();
            self.try_expand_snippet(tab_index);
        }
    }

    /// Called each time the UI needs repainting.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::diag::next_frame();

        // Handle window resize for borderless window (must be early, before UI)
        // This detects mouse near edges, changes cursor, and initiates resize
        handle_window_resize(ctx, &mut self.window_resize_state);

        // One-shot window diagnostic for debugging borderless offset issues (GH #112)
        if !self.window_diagnostic_logged {
            self.window_diagnostic_logged = true;
            let viewport_rect = ctx.viewport_rect();
            let content_rect = ctx.content_rect();
            let ppp = ctx.pixels_per_point();
            let inner_rect = ctx.input(|i| i.viewport().inner_rect);
            let outer_rect = ctx.input(|i| i.viewport().outer_rect);
            info!(
                "Window diagnostic: viewport_rect={:?}, content_rect={:?}, pixels_per_point={:.2}, inner={:?}, outer={:?}",
                viewport_rect, content_rect, ppp, inner_rect, outer_rect
            );
        }

        // One-shot: install platform keyboard hook to suppress Alt+Space system menu
        if !self.platform_init_done {
            self.platform_init_done = true;
            platform::install_platform_hooks();
        }

        // Check if the platform hook intercepted Alt+Space this frame
        if platform::take_palette_toggle_from_hook() {
            self.command_palette.toggle();
        }

        // Apply theme if needed (handles System theme changes)
        self.theme_manager
            .sync_accent_rgb(self.state.settings.accent_color);
        self.theme_manager.apply_if_needed(ctx);

        // Pre-warm font atlas if needed (deferred from font setup because
        // ctx.fonts() is not available until after first Context::run())
        fonts::check_and_prewarm_if_needed(ctx);

        // Warm per-frame caches (is_modified) for all tabs so that
        // title() and is_modified() calls via &self are O(1) this frame.
        self.state.warm_tab_caches();

        // Track user interaction for idle detection
        // This updates the last interaction time when any user input is detected,
        // which is used to determine the appropriate repaint interval
        if self.had_user_input(ctx) {
            self.update_interaction_time();
        }

        // Lazy load CJK/complex-script fonts (cached via content_version — O(1) per frame)
        let (needs_cjk, needs_complex) = if let Some(tab) = self.state.active_tab_mut() {
            (tab.needs_cjk_cached(), tab.needs_complex_script_cached())
        } else {
            (false, false)
        };
        if needs_cjk {
            if let Some(tab) = self.state.active_tab() {
                let _ = self.load_cjk_fonts_for_content(ctx, &tab.content);
            }
        }
        if needs_complex {
            if let Some(tab) = self.state.active_tab() {
                let _ = self.load_complex_script_fonts_for_content(ctx, &tab.content);
            }
        }

        // Also ensure CJK fonts are loaded if the UI language requires them.
        // This catches the case where no CJK document is open but the user
        // selected a CJK language (e.g. Chinese) — all UI labels need the font.
        if !fonts::are_cjk_fonts_loaded() {
            if let Some(lang_cjk) = self.state.settings.language.required_cjk_font() {
                let custom_font = self
                    .state
                    .settings
                    .font_family
                    .custom_name()
                    .map(|s| s.to_string());
                fonts::preload_explicit_cjk_font_with_custom(ctx, lang_cjk, custom_font.as_deref());
                info!(
                    "Per-frame: loaded CJK font for UI language {:?}",
                    self.state.settings.language
                );
            }
        }

        // Display any deferred startup toast
        if let Some(msg) = self.state.pending_toast.take() {
            let t = self.get_app_time();
            self.state.show_toast(msg, t, 5.0);
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

        // Poll for file paths from secondary instances (single-instance protocol)
        self.handle_instance_paths(ctx);

        // Poll background file loading messages (progress, completion, error)
        self.poll_file_load_messages(ctx);

        // Poll file watcher for workspace changes
        self.handle_file_watcher_events();

        // Background workspace file index (quick switcher / search-in-files)
        self.sync_workspace_file_index();
        if self.workspace_file_index.poll() {
            ctx.request_repaint();
        } else if self.workspace_file_index.is_indexing() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Poll LSP manager for status changes and spawn failures
        self.handle_lsp_events(ctx);

        // Handle automatic Git status refresh (focus, periodic, debounced)
        self.handle_git_auto_refresh(ctx);

        // Periodic session save for crash recovery
        self.update_session_recovery();

        // Sync productivity panel with current workspace
        let workspace_root = self.state.workspace_root().cloned();
        self.productivity_panel.set_workspace(workspace_root);

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

        // Block widgets under resize grab zones while the resize cursor is active.
        consume_clicks_in_resize_zones(ctx, &self.window_resize_state);

        // Apply resize cursor at end of frame (to override any UI cursor settings)
        self.window_resize_state.apply_cursor(ctx);

        // Request exit if confirmed
        if self.should_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Frame Rate Diagnostics (Debug Only)
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        #[cfg(debug_assertions)]
        {
            self.frame_count += 1;
            let elapsed = self.last_fps_log.elapsed();
            if elapsed.as_secs() >= 5 {
                let fps = (self.frame_count as f64) / elapsed.as_secs_f64();
                let needs_continuous = self.needs_continuous_repaint();
                let idle_secs = self.last_interaction_time.elapsed().as_secs_f32();
                let interval_ms = self.get_idle_repaint_interval().as_millis();

                // Log repaint causes to identify what's triggering repaints
                let repaint_causes = ctx.repaint_causes();
                let causes_str = if repaint_causes.is_empty() {
                    "none".to_string()
                } else {
                    repaint_causes
                        .iter()
                        .take(3) // Limit to first 3 causes
                        .map(|c| format!("{:?}", c))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                log::debug!(
                    "[REPAINT_DEBUG] FPS: {:.1}, continuous: {}, idle: {:.1}s, interval: {}ms, frames: {}, causes: {}",
                    fps,
                    needs_continuous,
                    idle_secs,
                    interval_ms,
                    self.frame_count,
                    causes_str
                );
                self.frame_count = 0;
                self.last_fps_log = std::time::Instant::now();
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Deferred CJK Font Loading
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Check for CJK content in newly opened files and load fonts immediately.
        // This flag is set when a file is opened during the UI render pass (which
        // happens AFTER the regular CJK check at the start of update). By checking
        // here, we ensure CJK fonts are loaded before the next frame renders,
        // preventing the "tofu/boxes" issue on file open.
        if self.pending_cjk_check {
            self.pending_cjk_check = false;
            let (deferred_cjk, deferred_complex) = if let Some(tab) = self.state.active_tab_mut() {
                (tab.needs_cjk_cached(), tab.needs_complex_script_cached())
            } else {
                (false, false)
            };
            if deferred_cjk {
                log::debug!("Deferred CJK check: loading fonts for newly opened file");
                if let Some(tab) = self.state.active_tab() {
                    let _ = self.load_cjk_fonts_for_content(ctx, &tab.content);
                }
            }
            if deferred_complex {
                log::debug!("Deferred complex script check: loading fonts for newly opened file");
                if let Some(tab) = self.state.active_tab() {
                    let _ = self.load_complex_script_fonts_for_content(ctx, &tab.content);
                }
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Idle Repaint Optimization
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
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

        crate::diag::frame_end(100);
    }

    /// Called when the application is about to close.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        use crate::config::{
            clear_crash_recovery_snapshot_file, remove_lock_file, save_session_state,
        };

        info!("Application exiting");

        // Save productivity panel data
        self.productivity_panel.save_all();

        // Persist unsaved tab bodies; `session.json` only stores metadata.
        self.state.save_recovery_content();

        // Capture and save session state for next startup
        let mut session_state = self.state.capture_session_state();
        session_state.mark_clean_shutdown();
        self.inject_csv_delimiters(&mut session_state);

        if save_session_state(&session_state) {
            info!("Session state saved for next startup");
        } else {
            warn!("Failed to save session state");
        }
        // Drop crash snapshot only; keep `recovery/` so pathless tabs reload next launch.
        clear_crash_recovery_snapshot_file();

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
