//! File operations for the Ferrite application.
//!
//! This module contains handlers for file open, save, save-as, workspace
//! management, drag-and-drop, file tree context actions, file watcher events,
//! and git auto-refresh.

use super::types::FileLoadMsg;
use super::FerriteApp;
use crate::config::ViewMode;
use crate::files::dialogs::{
    open_folder_dialog, open_multiple_files_dialog, portal_install_instructions, save_file_dialog,
    DialogResult,
};
use crate::state::{is_binary_content, FileType};
use crate::ui::{FileOperationDialog, FileTreeContextAction, SearchNavigationTarget};
use eframe::egui;
use log::{debug, info, trace, warn};
use rust_i18n::t;
use std::path::{Path, PathBuf};

/// File size threshold (bytes) above which background thread loading is used.
/// Files below this threshold are loaded synchronously (fast enough for UI).
const BACKGROUND_LOAD_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB

/// Chunk size for background file reading (controls progress update granularity).
const LOAD_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

impl FerriteApp {
    /// Handle the "File > Open" action.
    ///
    /// Opens a native file dialog allowing multiple file selection and loads
    /// each selected file into a new tab.
    pub(crate) fn handle_open_file(&mut self) {
        // Get the last open directory from recent files, if available
        let initial_dir = self
            .state
            .settings
            .recent_files
            .first()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        // Open the native file dialog (supports multiple selection)
        let result = open_multiple_files_dialog(initial_dir.as_ref());

        // Handle dialog result, checking for portal failures
        let paths = match result {
            DialogResult::Success(p) => p,
            DialogResult::Cancelled => {
                debug!("File dialog cancelled");
                return;
            }
            DialogResult::Failed {
                is_portal_error,
                desktop_env,
            } => {
                if is_portal_error {
                    self.show_portal_error_dialog(desktop_env, "open");
                }
                return;
            }
        };

        let file_count = paths.len();
        let mut success_count = 0;
        let mut last_error: Option<String> = None;

        for path in paths {
            info!("Opening file: {}", path.display());
            let time = self.get_app_time();
            match self.open_file_smart(path.clone(), true, Some(time)) {
                Ok(tab_index) => {
                    success_count += 1;
                    self.pending_cjk_check = true;
                    // Check for auto-save recovery (skip for loading tabs)
                    if !self
                        .state
                        .tabs()
                        .get(tab_index)
                        .map(|t| t.is_loading())
                        .unwrap_or(false)
                    {
                        self.check_auto_save_recovery(tab_index);
                    }
                }
                Err(e) => {
                    warn!("Failed to open file {}: {}", path.display(), e);
                    last_error =
                        Some(t!("error.open_file_failed", error = e.to_string()).to_string());
                }
            }
        }

        // Show toast for multiple files opened
        if file_count > 1 && success_count > 0 {
            let time = self.get_app_time();
            self.state.show_toast(
                t!("notification.opened_files", count = success_count).to_string(),
                time,
                2.0,
            );
        }

        // Show error if any file failed to open
        if let Some(error) = last_error {
            self.state.show_error(error);
        }
    }

    /// Open a file, using background loading for large files.
    ///
    /// Files above `BACKGROUND_LOAD_THRESHOLD` are loaded in a background thread
    /// with progress updates. Smaller files use synchronous loading as before.
    /// Open a file, using background loading for large files (>5 MB).
    ///
    /// Files above `BACKGROUND_LOAD_THRESHOLD` are loaded in a background thread
    /// with progress updates. Smaller files use synchronous loading as before.
    pub(crate) fn open_file_smart(
        &mut self,
        path: PathBuf,
        focus: bool,
        app_time: Option<f64>,
    ) -> Result<usize, std::io::Error> {
        // Delegate to synchronous path for already-open, image, and PDF files
        if self.state.find_tab_by_path(&path).is_some()
            || FileType::from_path(&path).is_image()
            || FileType::from_path(&path).is_pdf()
        {
            return self.state.open_file_with_focus(path, focus, app_time);
        }

        let metadata = std::fs::metadata(&path)?;
        let file_size = metadata.len();

        if file_size >= BACKGROUND_LOAD_THRESHOLD {
            let (tab_index, tab_id) = self.state.open_file_loading(path.clone(), file_size, focus);
            self.spawn_file_loader(tab_id, path);

            if let Some(time) = app_time {
                let size_mb = file_size / (1024 * 1024);
                self.state.show_toast(
                    t!(
                        "notification.large_file_loading",
                        size = size_mb.to_string()
                    )
                    .to_string(),
                    time,
                    3.0,
                );
            }
            Ok(tab_index)
        } else {
            self.state.open_file_with_focus(path, focus, app_time)
        }
    }

    /// Spawn a background thread that reads a file in chunks, sending progress updates.
    fn spawn_file_loader(&mut self, tab_id: usize, path: PathBuf) {
        let tx = self.file_load_tx.clone();

        let handle = std::thread::spawn(move || {
            use std::io::Read;

            let file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(FileLoadMsg::Error {
                        tab_id,
                        error: format!("Failed to open: {}", e),
                    });
                    return;
                }
            };

            let total_size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let mut reader = std::io::BufReader::with_capacity(LOAD_CHUNK_SIZE, file);
            let mut bytes = Vec::with_capacity(total_size as usize);
            let mut buf = vec![0u8; LOAD_CHUNK_SIZE];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        bytes.extend_from_slice(&buf[..n]);
                        let _ = tx.send(FileLoadMsg::Progress {
                            tab_id,
                            bytes_loaded: bytes.len() as u64,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(FileLoadMsg::Error {
                            tab_id,
                            error: format!("Read error: {}", e),
                        });
                        return;
                    }
                }
            }

            // Check for binary content before sending
            if is_binary_content(&bytes) {
                let _ = tx.send(FileLoadMsg::Error {
                    tab_id,
                    error: "Binary file detected. Use a specialized tool to edit this file."
                        .to_string(),
                });
                return;
            }

            let _ = tx.send(FileLoadMsg::Complete { tab_id, bytes });
        });

        self.loading_tasks.insert(tab_id, handle);
    }

    /// Poll the file load channel and apply progress/completion/error messages.
    ///
    /// Called each frame from `update()`. Non-blocking: drains all pending messages.
    pub(crate) fn poll_file_load_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.file_load_rx.try_recv() {
            match msg {
                FileLoadMsg::Progress {
                    tab_id,
                    bytes_loaded,
                } => {
                    if let Some(tab) = self.state.tab_by_id_mut(tab_id) {
                        if let crate::state::TabContent::Loading(ref mut progress) = tab.tab_content
                        {
                            progress.bytes_loaded = bytes_loaded;
                        }
                    }
                    ctx.request_repaint();
                }
                FileLoadMsg::Complete { tab_id, bytes } => {
                    let auto_save = self.state.settings.auto_save_enabled_default;
                    let file_path = self.state.tab_by_id(tab_id).and_then(|t| t.path.clone());
                    let (view_mode, split_ratio) = file_path
                        .as_ref()
                        .map(|p| self.state.opening_view_prefs_for_path(p))
                        .unwrap_or((self.state.settings.default_view_mode, 0.5));

                    if let Some(tab) = self.state.tab_by_id_mut(tab_id) {
                        let file_size = bytes.len() as f64 / (1024.0 * 1024.0);
                        tab.finish_loading(bytes, auto_save, view_mode);
                        tab.split_ratio = split_ratio;

                        if tab.view_mode == crate::config::ViewMode::Split
                            && !tab.file_type().supports_split()
                        {
                            tab.view_mode = crate::config::ViewMode::Raw;
                        }

                        let time = self.get_app_time();
                        self.state.show_toast(
                            t!(
                                "notification.file_loaded",
                                size = format!("{:.1}", file_size)
                            )
                            .to_string(),
                            time,
                            2.0,
                        );
                    }

                    self.loading_tasks.remove(&tab_id);
                    self.pending_cjk_check = true;
                    ctx.request_repaint();
                }
                FileLoadMsg::Error { tab_id, error } => {
                    if let Some(tab) = self.state.tab_by_id_mut(tab_id) {
                        tab.fail_loading(error.clone());
                    }
                    self.loading_tasks.remove(&tab_id);

                    let time = self.get_app_time();
                    self.state.show_toast(
                        t!("notification.file_load_failed", error = error).to_string(),
                        time,
                        4.0,
                    );
                    ctx.request_repaint();
                }
            }
        }
    }

    /// Handle the "File > Save" action.
    ///
    /// Saves the current document to its existing file path.
    /// If the document has no path, triggers "Save As" instead.
    pub(crate) fn handle_save_file(&mut self) {
        // Special tabs (settings, about) cannot be saved
        if self
            .state
            .active_tab()
            .map(|t| t.is_special())
            .unwrap_or(false)
        {
            return;
        }

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
                    self.state.show_toast(
                        t!("notification.saved", path = path_display).to_string(),
                        time,
                        3.0,
                    );

                    // Clean up auto-save temp file after successful manual save
                    if let Some(id) = tab_id {
                        self.cleanup_auto_save_for_tab(id);
                    }

                    // Trigger git status refresh after successful save
                    self.request_git_refresh();

                    // Update backlink index incrementally for the saved file
                    if let Some(path) = self.state.active_tab().and_then(|t| t.path.clone()) {
                        if self.state.backlink_index.is_built {
                            self.state.backlink_index.update_file(&path);
                        }
                        self.backlinks_need_refresh = true;
                    }
                }
                Err(e) => {
                    warn!("Failed to save file: {}", e);
                    self.state
                        .show_error(t!("error.save_failed", error = e.to_string()).to_string());
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
    pub(crate) fn handle_save_as_file(&mut self) {
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
        let save_result = save_file_dialog(initial_dir.as_ref(), Some(&default_name));

        let path = match save_result {
            DialogResult::Success(p) => p,
            DialogResult::Cancelled => {
                debug!("Save dialog cancelled");
                return;
            }
            DialogResult::Failed {
                is_portal_error,
                desktop_env,
            } => {
                if is_portal_error {
                    self.show_portal_error_dialog(desktop_env, "save");
                }
                return;
            }
        };
        info!("Saving file as: {}", path.display());

        // Get old path and tab ID before save for cleanup
        let old_path = self.state.active_tab().and_then(|t| t.path.clone());
        let tab_id = self.state.active_tab().map(|t| t.id);

        match self.state.save_active_tab_as(path.clone()) {
            Ok(_) => {
                let time = self.get_app_time();
                self.state.show_toast(
                    t!("notification.saved", path = path.display().to_string()).to_string(),
                    time,
                    3.0,
                );

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

                // Update backlink index incrementally and refresh backlinks
                if self.state.backlink_index.is_built {
                    self.state.backlink_index.update_file(&path);
                }
                self.backlinks_need_refresh = true;
            }
            Err(e) => {
                warn!("Failed to save file: {}", e);
                self.state
                    .show_error(t!("error.save_failed", error = e.to_string()).to_string());
            }
        }
    }

    /// Handle the "File > Open Workspace" action.
    ///
    /// Opens a native folder dialog and switches to workspace mode.
    /// On Linux/Flatpak, rfd uses xdg-desktop-portal automatically, which
    /// grants the sandbox access to the user-selected directory.
    pub(crate) fn handle_open_workspace(&mut self) {
        use crate::files::dialogs::is_flatpak;

        // Get initial directory from recent workspaces or recent files.
        // resolve_initial_dir (inside open_folder_dialog) will fall back to
        // $HOME if this is None, which is critical for Flatpak's portal dialog.
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

        if is_flatpak() {
            debug!("Running in Flatpak sandbox, folder dialog will use xdg-desktop-portal");
        }

        // Open the native folder dialog
        let folder_result = open_folder_dialog(initial_dir.as_ref());

        let folder_path = match folder_result {
            DialogResult::Success(p) => p,
            DialogResult::Cancelled => {
                debug!("Open workspace dialog cancelled");
                return;
            }
            DialogResult::Failed {
                is_portal_error,
                desktop_env,
            } => {
                if is_portal_error {
                    self.show_portal_error_dialog(desktop_env, "open folder");
                }
                return;
            }
        };

        info!("Opening workspace: {}", folder_path.display());

        // Verify the folder is accessible (important for Flatpak portal paths)
        if !folder_path.is_dir() {
            warn!(
                "Selected path is not accessible as a directory: {}",
                folder_path.display()
            );
            if is_flatpak() {
                self.state.show_error(
                    "Could not access the selected folder. The Flatpak sandbox may not have \
                     permission to read this location. Try selecting a folder inside your home directory."
                        .to_string(),
                );
            } else {
                self.state.show_error(
                    t!(
                        "error.open_workspace_failed",
                        error = "Path is not a directory"
                    )
                    .to_string(),
                );
            }
            return;
        }

        match self.state.open_workspace(folder_path.clone()) {
            Ok(_) => {
                let time = self.get_app_time();
                let folder_name = folder_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder");
                self.state.show_toast(
                    t!("notification.opened_workspace", name = folder_name).to_string(),
                    time,
                    2.5,
                );

                // Auto-load terminal layout if enabled
                if self.state.settings.terminal_auto_load_layout {
                    let layout_path = folder_path.join("terminal_layout.json");
                    if layout_path.exists() {
                        if let Ok(json) = std::fs::read_to_string(layout_path) {
                            if let Ok(workspace) =
                                serde_json::from_str::<crate::terminal::SavedWorkspace>(&json)
                            {
                                match self.terminal_panel_state.manager.load_workspace(workspace) {
                                    Ok(fws) => {
                                        self.terminal_panel_state.floating_windows.clear();
                                        for (layout, title, pos, size) in fws {
                                            let leaf = layout.first_leaf();
                                            let id = egui::ViewportId::from_hash_of(
                                                egui::Id::new("floating_term").with(leaf),
                                            );
                                            self.terminal_panel_state.floating_windows.push(
                                                crate::ui::FloatingWindow {
                                                    id,
                                                    layout,
                                                    title,
                                                    pos: pos.map(|(x, y)| egui::pos2(x, y)),
                                                    size: egui::vec2(size.0, size.1),
                                                    first_frame: true,
                                                },
                                            );
                                        }
                                        info!("Auto-loaded terminal layout from workspace root");
                                    }
                                    Err(e) => warn!("Failed to auto-load terminal layout: {}", e),
                                }
                            }
                        }
                    }
                }

                // Immediately save session to persist the workspace path
                self.force_session_save();
            }
            Err(e) => {
                warn!("Failed to open workspace: {}", e);
                if is_flatpak() {
                    self.state.show_error(format!(
                        "Failed to open folder: {}. If running as Flatpak, ensure the folder \
                         was selected through the file dialog (portal access is required).",
                        e
                    ));
                } else {
                    self.state.show_error(
                        t!("error.open_workspace_failed", error = e.to_string()).to_string(),
                    );
                }
            }
        }
    }

    /// Handle closing the current workspace.
    ///
    /// Returns to single-file mode and hides workspace UI.
    pub(crate) fn handle_close_workspace(&mut self) {
        if self.state.is_workspace_mode() {
            self.state.close_workspace();
            self.state.backlink_index.clear();
            self.backlinks_panel.clear();
            self.backlinks_need_refresh = true;
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.workspace_closed").to_string(), time, 2.0);

            // Immediately save session to persist the mode change
            self.force_session_save();
        }
    }

    /// Poll LSP manager events, detect settings toggle transitions, and surface
    /// spawn failures as dismissible toast notifications.
    pub(crate) fn handle_lsp_events(&mut self, _ctx: &egui::Context) {
        // Reset all LSP tracking when the workspace folder changes.
        let current_ws = self.state.workspace_root().map(|p| p.to_path_buf());
        if current_ws != self.lsp_status_workspace {
            self.lsp_status_workspace = current_ws;
            self.lsp_status_by_server.clear();
            self.lsp_opened_docs.clear();
            self.lsp_doc_versions.clear();
            self.lsp_last_edit_times.clear();
            self.lsp_last_change_sent.clear();
            self.lsp_tab_server.clear();
            self.lsp_open_doc_count.clear();
            self.lsp_idle_since.clear();
        }

        // When override paths change, stop all servers so on-demand restart
        // picks up the new program paths.
        let fp = crate::lsp::overrides_fingerprint(&self.state.settings.lsp_server_overrides);
        if fp != self.lsp_overrides_fingerprint {
            self.lsp_overrides_fingerprint = fp;
            if self.state.settings.lsp_enabled {
                self.state.lsp.stop_all_servers();
                self.lsp_status_by_server.clear();
                self.lsp_opened_docs.clear();
                self.lsp_doc_versions.clear();
                self.lsp_last_edit_times.clear();
                self.lsp_last_change_sent.clear();
                self.lsp_tab_server.clear();
                self.lsp_open_doc_count.clear();
                self.lsp_idle_since.clear();
            }
        }

        let lsp_now = self.state.settings.lsp_enabled;
        if lsp_now != self.lsp_was_enabled {
            self.lsp_was_enabled = lsp_now;
            if !lsp_now {
                self.state.lsp.stop_all_servers();
                self.lsp_status_by_server.clear();
            }
            // When toggled on, sync_active_doc_to_lsp will start the
            // server on demand for the currently active tab.
        }

        let events = self.state.lsp.poll_events();
        for ev in events {
            match ev {
                crate::lsp::LspManagerEvent::StatusChanged { server_key, status } => {
                    log::debug!("LSP status: {} → {:?}", server_key, status);
                    self.lsp_status_by_server.insert(server_key, status);
                }
                crate::lsp::LspManagerEvent::SpawnFailed {
                    server_key: _,
                    program,
                    error,
                } => {
                    let hint = crate::lsp::install_hint(&program);
                    let msg = format!("LSP: {} not found ({}). {}", program, error, hint);
                    let time = self.get_app_time();
                    self.state.show_toast(msg, time, 6.0);
                }
                crate::lsp::LspManagerEvent::Diagnostics {
                    server_key: _,
                    path,
                    diagnostics,
                } => {
                    self.state.diagnostics.set(path, diagnostics);
                }
            }
        }

        // If LSP is off, keep the map empty so the status bar does not show stale rows.
        if !self.state.settings.lsp_enabled {
            self.lsp_status_by_server.clear();
            self.state.diagnostics.clear();
            self.lsp_opened_docs.clear();
            self.lsp_doc_versions.clear();
            self.lsp_last_edit_times.clear();
            self.lsp_last_change_sent.clear();
            self.lsp_tab_server.clear();
            self.lsp_open_doc_count.clear();
            self.lsp_idle_since.clear();
        } else {
            self.sync_active_doc_to_lsp();
            self.check_lsp_idle_shutdown();
        }
    }

    /// Minimum interval between `didChange` notifications (debounce).
    const LSP_DID_CHANGE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

    /// Idle timeout for shutting down servers with no open documents.
    const LSP_IDLE_SHUTDOWN: std::time::Duration = std::time::Duration::from_secs(30);

    /// Ensure the active tab's document is opened and up-to-date with the LSP server.
    ///
    /// On-demand: if no server is running for the active file's extension, starts one.
    /// Uses `Tab::last_edit_time` to detect edits without cloning content every frame,
    /// and debounces `didChange` notifications to avoid flooding the server.
    fn sync_active_doc_to_lsp(&mut self) {
        let tab = match self.state.active_tab() {
            Some(t) => t,
            None => return,
        };
        let tab_id = tab.id;
        let path = match &tab.path {
            Some(p) => p.clone(),
            None => return,
        };

        let spec = crate::lsp::detect_lsp_server_for_path(&path);
        let server_key = match &spec {
            Some(s) => s.program.clone(),
            None => return,
        };

        let status = self
            .lsp_status_by_server
            .get(&server_key)
            .cloned()
            .unwrap_or(crate::lsp::state::ServerStatus::Disconnected);

        match &status {
            crate::lsp::state::ServerStatus::Ready => {
                // Server ready — proceed to didOpen / didChange below
            }
            crate::lsp::state::ServerStatus::Disconnected => {
                // On-demand start: spawn the server now
                self.start_lsp_server_on_demand(&server_key, spec.unwrap(), &path);
                return;
            }
            crate::lsp::state::ServerStatus::Starting
            | crate::lsp::state::ServerStatus::Initializing => {
                // Server is coming up — wait
                return;
            }
            crate::lsp::state::ServerStatus::Error(_) => {
                // Failed previously — don't auto-retry (backoff handles crashes)
                return;
            }
        }

        let norm_path = crate::lsp::normalize_lsp_path(&path);
        let uri = crate::lsp::path_to_uri(&path);

        // First-time open: send didOpen with full content (only clone here)
        if !self.lsp_opened_docs.contains(&norm_path) {
            let content = tab.content.clone();
            let lang_id = crate::lsp::language_id_for_path(&path).to_string();
            let version = 1_i32;
            log::debug!(
                "LSP didOpen: {} → {} ({})",
                norm_path.display(),
                uri,
                lang_id
            );
            self.state
                .lsp
                .did_open(&server_key, uri, lang_id, version as i64, content);
            self.lsp_opened_docs.insert(norm_path.clone());
            self.lsp_doc_versions.insert(norm_path.clone(), version);
            self.lsp_tab_server
                .insert(tab_id, (norm_path.clone(), server_key.clone()));
            *self
                .lsp_open_doc_count
                .entry(server_key.clone())
                .or_insert(0) += 1;
            self.lsp_idle_since.remove(&server_key);
            if let Some(t) = tab.last_edit_time {
                self.lsp_last_edit_times.insert(norm_path.clone(), t);
            }
            self.lsp_last_change_sent
                .insert(norm_path, std::time::Instant::now());
            return;
        }

        // If the tab wasn't recorded yet (e.g. second tab opened same file), record it
        if !self.lsp_tab_server.contains_key(&tab_id) {
            self.lsp_tab_server
                .insert(tab_id, (norm_path.clone(), server_key.clone()));
        }

        // Check if the tab was edited since our last sync.
        let current_edit_time = tab.last_edit_time;
        let prev_edit_time = self.lsp_last_edit_times.get(&norm_path).copied();

        let needs_sync = match (current_edit_time, prev_edit_time) {
            (Some(cur), Some(prev)) => cur != prev,
            (Some(_), None) => true,
            (None, _) => false,
        };

        if !needs_sync {
            return;
        }

        // Debounce: don't send didChange more than once per interval
        let now = std::time::Instant::now();
        if let Some(last_sent) = self.lsp_last_change_sent.get(&norm_path) {
            if now.duration_since(*last_sent) < Self::LSP_DID_CHANGE_DEBOUNCE {
                return;
            }
        }

        // Content actually changed — clone and send
        let content = tab.content.clone();
        let version = self.lsp_doc_versions.get(&norm_path).copied().unwrap_or(0) + 1;
        log::debug!(
            "LSP didChange: {} v{} ({} bytes)",
            norm_path.display(),
            version,
            content.len()
        );
        self.state
            .lsp
            .did_change(&server_key, uri, version as i64, content);
        self.lsp_doc_versions.insert(norm_path.clone(), version);
        if let Some(t) = current_edit_time {
            self.lsp_last_edit_times.insert(norm_path.clone(), t);
        }
        self.lsp_last_change_sent.insert(norm_path, now);
    }

    /// Start an LSP server on demand for the given file, applying any user overrides.
    fn start_lsp_server_on_demand(
        &self,
        server_key: &str,
        mut spec: crate::lsp::detection::LspServerSpec,
        file_path: &std::path::Path,
    ) {
        if let Some(override_path) = self.state.settings.lsp_server_overrides.get(server_key) {
            let trimmed = override_path.trim();
            if !trimmed.is_empty() {
                spec.program = trimmed.to_string();
            }
        }
        let workspace_root = self
            .state
            .workspace_root()
            .cloned()
            .or_else(|| file_path.parent().map(|p| p.to_path_buf()));
        log::info!(
            "LSP on-demand start: {} (program={}) for {}",
            server_key,
            spec.program,
            file_path.display()
        );
        self.state
            .lsp
            .start_server(server_key.to_string(), spec, workspace_root);
    }

    /// Shut down LSP servers that have had no open documents for `LSP_IDLE_SHUTDOWN`.
    fn check_lsp_idle_shutdown(&mut self) {
        let now = std::time::Instant::now();
        let expired: Vec<String> = self
            .lsp_idle_since
            .iter()
            .filter(|(_, since)| now.duration_since(**since) >= Self::LSP_IDLE_SHUTDOWN)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            self.lsp_idle_since.remove(&key);
            self.lsp_open_doc_count.remove(&key);
            log::info!(
                "LSP idle shutdown: {} (no open documents for {}s)",
                key,
                Self::LSP_IDLE_SHUTDOWN.as_secs()
            );
            self.state.lsp.stop_server(&key);
            self.lsp_status_by_server
                .insert(key, crate::lsp::state::ServerStatus::Disconnected);
        }
    }

    /// Build status-bar text for LSP (compact) and optional hover detail.
    ///
    /// Only shows servers that have been started (on demand) rather than
    /// scanning the workspace for potential servers each frame.
    pub(crate) fn lsp_status_bar_text(&self) -> (String, String) {
        use crate::lsp::state::ServerStatus;

        if !self.state.settings.lsp_enabled {
            return (
                "LSP: Disabled".to_string(),
                "Language servers are disabled in Settings.".to_string(),
            );
        }

        if self.lsp_status_by_server.is_empty() {
            return (
                "LSP".to_string(),
                "No language servers running. Servers start on demand when you open a supported file.".to_string(),
            );
        }

        let mut keys: Vec<&String> = self.lsp_status_by_server.keys().collect();
        keys.sort();

        let mut parts = Vec::new();
        let mut detail = String::from("Language server status:\n");
        for key in &keys {
            let status = self
                .lsp_status_by_server
                .get(*key)
                .cloned()
                .unwrap_or(ServerStatus::Disconnected);
            let label = status.short_label();
            parts.push(format!("{}: {}", key, label));
            if let ServerStatus::Error(e) = &status {
                detail.push_str(&format!("• {} — {} ({})\n", key, label, e));
            } else {
                detail.push_str(&format!("• {} — {}\n", key, label));
            }
        }
        let summary = format!("LSP {}", parts.join(" · "));
        (summary, detail.trim_end().to_string())
    }

    /// Force an immediate session save (bypasses throttling).
    ///
    /// Use this after important state changes like opening/closing workspaces
    /// to ensure the change is persisted immediately.
    pub(crate) fn force_session_save(&mut self) {
        use crate::config::save_crash_recovery_state;

        let workspace_info = if let Some(root) = self.state.workspace_root() {
            format!("Workspace({})", root.display())
        } else {
            "SingleFile".to_string()
        };
        debug!("Force session save requested: app_mode={}", workspace_info);

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
            warn!("Failed to force session save: app_mode={}", workspace_info);
        }
    }

    /// Handle toggling the file tree panel visibility.
    pub(crate) fn handle_toggle_file_tree(&mut self) {
        if self.state.is_workspace_mode() {
            self.state.toggle_file_tree();
            let time = self.get_app_time();
            let msg = if self.state.should_show_file_tree() {
                t!("notification.file_tree_shown").to_string()
            } else {
                t!("notification.file_tree_hidden").to_string()
            };
            self.state.show_toast(msg, time, 1.5);
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.open_folder_first").to_string(), time, 2.0);
        }
    }

    /// Handle opening the quick file switcher.
    pub(crate) fn handle_quick_open(&mut self) {
        if self.state.is_workspace_mode() {
            self.quick_switcher.toggle();
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state.show_toast(
                t!("notification.open_folder_quick_open").to_string(),
                time,
                2.0,
            );
        }
    }

    /// Handle opening the search in files panel.
    pub(crate) fn handle_search_in_files(&mut self) {
        if self.state.is_workspace_mode() {
            self.search_panel.toggle();
            // Trigger search if panel is now open
            if self.search_panel.is_open() {
                if let Some(workspace) = &self.state.workspace {
                    let files = self.workspace_files_for_search(workspace);
                    self.search_panel.search(&files, &workspace.hidden_patterns);
                }
            }
        } else {
            // Not in workspace mode - show a hint
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.open_folder_search").to_string(), time, 2.0);
        }
    }

    /// Handle navigation from a search-in-files result click.
    ///
    /// This opens the file (if not already open), scrolls to the match location,
    /// applies a transient highlight, and switches to Raw mode if necessary.
    pub(crate) fn handle_search_navigation(&mut self, target: SearchNavigationTarget) {
        let file_path = target.path.clone();

        // Open the file (or switch to existing tab)
        let time = self.get_app_time();
        match self.state.open_file(file_path.clone(), Some(time)) {
            Ok(_) => {
                self.pending_cjk_check = true;
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
                    .show_error(t!("error.open_file_failed", error = e.to_string()).to_string());
            }
        }
    }

    /// Refresh the sidebar file tree and schedule a full file-index rebuild.
    pub(crate) fn refresh_workspace_tree(&mut self) {
        self.state.refresh_workspace();
        self.workspace_file_index.invalidate();
    }

    /// Keep the workspace file index in sync with the open folder.
    pub(crate) fn sync_workspace_file_index(&mut self) {
        if let Some(workspace) = self.state.workspace() {
            self.workspace_file_index
                .sync(&workspace.root_path, &workspace.hidden_patterns);
        } else {
            self.workspace_file_index.reset();
        }
    }

    /// Files for quick switcher / search: full index when available, tree fallback while starting.
    pub(crate) fn workspace_files_for_search(
        &self,
        workspace: &crate::workspaces::Workspace,
    ) -> Vec<PathBuf> {
        let indexed = self.workspace_file_index.files();
        if self.workspace_file_index.is_indexing() {
            if indexed.is_empty() {
                workspace.all_files()
            } else {
                indexed.to_vec()
            }
        } else if !indexed.is_empty() {
            indexed.to_vec()
        } else {
            workspace.all_files()
        }
    }

    /// Handle file watcher events from the workspace.
    pub(crate) fn handle_file_watcher_events(&mut self) {
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

                    // Notify terminal panel for watch mode
                    self.terminal_panel_state.manager.on_file_changed(&path);

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
            self.refresh_workspace_tree();
            // Also request git refresh since files changed
            self.request_git_refresh();
        }

        // Reload externally modified files that are open in tabs
        if !modified_files.is_empty() {
            let time = self.get_app_time();
            let mut reloaded_count = 0;
            let tab_count = self.state.tab_count();

            for path in &modified_files {
                // Read the updated content from disk
                match std::fs::read(path) {
                    Ok(bytes) => {
                        // Detect encoding and decode
                        let new_content = String::from_utf8(bytes.clone())
                            .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string());

                        // Find the tab with this path and reload if not modified by user
                        for idx in 0..tab_count {
                            let should_reload = self
                                .state
                                .tab(idx)
                                .map(|tab| tab.path.as_ref() == Some(path) && !tab.is_modified())
                                .unwrap_or(false);
                            let has_unsaved = self
                                .state
                                .tab(idx)
                                .map(|tab| tab.path.as_ref() == Some(path) && tab.is_modified())
                                .unwrap_or(false);

                            if should_reload {
                                if let Some(tab) = self.state.tab_mut(idx) {
                                    tab.content = new_content.clone();
                                    tab.notify_external_content_change();
                                    // Clamp cursor to new content length
                                    let max_chars = tab.content.chars().count();
                                    let current_cursor = tab.cursors.primary().head.min(max_chars);
                                    tab.pending_cursor_restore = Some(current_cursor);
                                    reloaded_count += 1;
                                    debug!("Reloaded externally modified file: {}", path.display());
                                }
                                break;
                            } else if has_unsaved {
                                warn!(
                                    "File modified externally but tab has unsaved changes: {}",
                                    path.display()
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to read externally modified file {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }

            // Show appropriate toast
            let msg = if reloaded_count > 0 {
                if reloaded_count == 1 {
                    let name = modified_files[0]
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    t!("notification.reloaded_single", name = name).to_string()
                } else {
                    t!("notification.reloaded_multiple", count = reloaded_count).to_string()
                }
            } else if modified_files.len() == 1 {
                let name = modified_files[0]
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                t!("notification.external_change_single", name = name).to_string()
            } else {
                t!(
                    "notification.external_change_multiple",
                    count = modified_files.len()
                )
                .to_string()
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
    pub(crate) fn handle_git_auto_refresh(&mut self, ctx: &egui::Context) {
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
    pub(crate) fn request_git_refresh(&mut self) {
        if self.state.git_service.is_open() {
            self.git_auto_refresh.request_refresh();
        }
    }

    /// Check if a file path has a supported image extension.
    pub(crate) fn is_supported_image(path: &std::path::Path) -> bool {
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
    pub(crate) fn get_assets_dir(&self) -> std::path::PathBuf {
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
    pub(crate) fn generate_unique_image_filename(original_path: &std::path::Path) -> String {
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
    pub(crate) fn handle_dropped_image(
        &mut self,
        image_path: &std::path::Path,
    ) -> Result<(), String> {
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
        std::fs::copy(image_path, &dest_path)
            .map_err(|e| format!("Failed to copy image to '{}': {}", dest_path.display(), e))?;
        info!(
            "Copied dropped image to: {} (from {})",
            dest_path.display(),
            image_path.display()
        );

        // Insert markdown link at cursor position in the active tab
        // Uses cursor_position (line, col) which is reliably synced from FerriteEditor,
        // rather than tab.cursors which may be stale.
        if let Some(tab) = self.state.active_tab_mut() {
            // Save state for undo
            let old_content = tab.content.clone();
            let old_cursor = tab.cursors.primary().head;

            // Use cursor_position (line, col) which is reliably synced from FerriteEditor
            let (cursor_line, cursor_col) = tab.cursor_position;

            // Calculate byte position from line/col
            let lines: Vec<&str> = tab.content.split('\n').collect();
            let mut cursor_byte = 0usize;
            for (i, line) in lines.iter().enumerate() {
                if i == cursor_line {
                    cursor_byte += cursor_col.min(line.len());
                    break;
                }
                cursor_byte += line.len() + 1; // +1 for newline
            }
            cursor_byte = cursor_byte.min(tab.content.len());

            // Build markdown image link with relative path
            let markdown_link = format!("![](assets/{})", new_filename);
            let link_len = markdown_link.chars().count();

            // Insert at cursor position
            tab.content.insert_str(cursor_byte, &markdown_link);

            // Position cursor after the inserted link
            let cursor_char_pos = tab.content[..cursor_byte].chars().count();
            let new_cursor_pos = cursor_char_pos + link_len;
            tab.pending_cursor_restore = Some(new_cursor_pos);
            tab.cursors
                .set_single(crate::state::Selection::cursor(new_cursor_pos));
            tab.sync_cursor_from_primary();

            // Record for undo
            tab.record_edit(old_content, old_cursor);

            debug!(
                "Inserted image link '{}' at line {} col {}",
                markdown_link, cursor_line, cursor_col
            );
        }

        Ok(())
    }

    /// Handle file paths received from secondary Ferrite instances.
    ///
    /// When the user double-clicks a file in the OS while Ferrite is already running,
    /// the second process forwards the path here via the single-instance TCP protocol.
    /// This opens the file as a new tab and brings the window to the front.
    pub(crate) fn handle_instance_paths(&mut self, ctx: &egui::Context) {
        let paths = match &self.instance_listener {
            Some(listener) => {
                // Ensure the background accept thread can wake us up immediately.
                // This is cheap (just an Arc clone check) when already set.
                listener.set_repaint_ctx(ctx.clone());
                listener.poll()
            }
            None => return,
        };

        if paths.is_empty() {
            return;
        }

        info!("Received {} path(s) from secondary instance", paths.len());

        // Keep the normal cross-platform focus path in place.
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));

        let time = self.get_app_time();
        let mut opened = 0;

        for path in paths {
            if path.is_dir() {
                // Open as workspace
                info!(
                    "Opening workspace from secondary instance: {}",
                    path.display()
                );
                match self.state.open_workspace(path.clone()) {
                    Ok(_) => {
                        let folder_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("folder");
                        self.state.show_toast(
                            t!("notification.opened_workspace", name = folder_name).to_string(),
                            time,
                            2.5,
                        );
                        self.force_session_save();
                    }
                    Err(e) => {
                        warn!("Failed to open workspace from secondary instance: {}", e);
                    }
                }
            } else if path.is_file() {
                match self.open_file_smart(path.clone(), true, Some(time)) {
                    Ok(tab_index) => {
                        self.pending_cjk_check = true;
                        if !self
                            .state
                            .tabs()
                            .get(tab_index)
                            .map(|t| t.is_loading())
                            .unwrap_or(false)
                        {
                            self.check_auto_save_recovery(tab_index);
                        }
                        opened += 1;
                        debug!("Opened file from secondary instance: {}", path.display());
                    }
                    Err(e) => {
                        warn!("Failed to open file from secondary instance: {}", e);
                    }
                }
            } else {
                warn!(
                    "Path from secondary instance does not exist: {}",
                    path.display()
                );
            }
        }

        if opened > 0 {
            let msg = if opened == 1 {
                t!("notification.opened_external_single").to_string()
            } else {
                t!("notification.opened_external_multiple", count = opened).to_string()
            };
            self.state.show_toast(msg, time, 2.5);
        }
    }

    /// Handle files/folders dropped onto the application window.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
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
                        "md" | "markdown"
                            | "mdown"
                            | "mkd"
                            | "mkdn"
                            | "txt"
                            | "csv"
                            | "tsv"
                            | "json"
                            | "yaml"
                            | "yml"
                            | "toml"
                    ) {
                        documents.push(path);
                    }
                }
            }
        }

        // Priority 1: If a folder was dropped, open it as a workspace
        if let Some(folder) = folders.into_iter().next() {
            info!("Opening dropped folder as workspace: {}", folder.display());
            let folder_path = folder.clone();
            match self.state.open_workspace(folder.clone()) {
                Ok(_) => {
                    let time = self.get_app_time();
                    let folder_name = folder
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("folder");
                    self.state.show_toast(
                        t!("notification.opened_workspace", name = folder_name).to_string(),
                        time,
                        2.5,
                    );

                    // Auto-load terminal layout if enabled
                    if self.state.settings.terminal_auto_load_layout {
                        let layout_path = folder_path.join("terminal_layout.json");
                        if layout_path.exists() {
                            if let Ok(json) = std::fs::read_to_string(layout_path) {
                                if let Ok(workspace) =
                                    serde_json::from_str::<crate::terminal::SavedWorkspace>(&json)
                                {
                                    match self
                                        .terminal_panel_state
                                        .manager
                                        .load_workspace(workspace)
                                    {
                                        Ok(fws) => {
                                            self.terminal_panel_state.floating_windows.clear();
                                            for (layout, title, pos, size) in fws {
                                                let leaf = layout.first_leaf();
                                                let id = egui::ViewportId::from_hash_of(
                                                    egui::Id::new("floating_term").with(leaf),
                                                );
                                                self.terminal_panel_state.floating_windows.push(
                                                    crate::ui::FloatingWindow {
                                                        id,
                                                        layout,
                                                        title,
                                                        pos: pos.map(|(x, y)| egui::pos2(x, y)),
                                                        size: egui::vec2(size.0, size.1),
                                                        first_frame: true,
                                                    },
                                                );
                                            }
                                            info!(
                                                "Auto-loaded terminal layout from workspace root"
                                            );
                                        }
                                        Err(e) => {
                                            warn!("Failed to auto-load terminal layout: {}", e)
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Immediately save session to persist the workspace path
                    self.force_session_save();
                }
                Err(e) => {
                    warn!("Failed to open workspace: {}", e);
                    self.state.show_error(
                        t!("error.open_workspace_failed", error = e.to_string()).to_string(),
                    );
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
                    self.state
                        .show_error(t!("error.image_failed", error = e.to_string()).to_string());
                }
            }
        }

        if images_inserted > 0 {
            let time = self.get_app_time();
            let msg = if images_inserted == 1 {
                t!("notification.image_added").to_string()
            } else {
                t!("notification.images_added", count = images_inserted).to_string()
            };
            self.state.show_toast(msg, time, 2.5);
        }

        // Priority 3: Open document files in tabs
        let time = self.get_app_time();
        for file in documents {
            match self.open_file_smart(file.clone(), true, Some(time)) {
                Ok(_) => {
                    self.pending_cjk_check = true;
                    debug!("Opened dropped file: {}", file.display());
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
    pub(crate) fn handle_file_tree_context_action(&mut self, action: FileTreeContextAction) {
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
                        .show_error(t!("error.explorer_failed", error = e.to_string()).to_string());
                } else {
                    debug!("Revealed in explorer: {}", folder.display());
                }
            }
            FileTreeContextAction::Refresh => {
                self.refresh_workspace_tree();
                let time = self.get_app_time();
                self.state.show_toast(
                    t!("notification.file_tree_refreshed").to_string(),
                    time,
                    1.5,
                );
            }
        }
    }

    /// Handle creating a new file.
    pub(crate) fn handle_create_file(&mut self, path: std::path::PathBuf) {
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
                        .show_error(t!("error.write_failed", error = e.to_string()).to_string());
                    return;
                }

                info!("Created new file: {}", path.display());
                let time = self.get_app_time();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                self.state.show_toast(
                    t!("notification.created", name = name).to_string(),
                    time,
                    2.0,
                );

                // Refresh file tree
                self.refresh_workspace_tree();

                // Open the new file in a tab
                let time = self.get_app_time();
                match self.state.open_file(path.clone(), Some(time)) {
                    Ok(_) => {
                        self.pending_cjk_check = true;
                    }
                    Err(e) => {
                        warn!("Failed to open new file: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create file: {}", e);
                self.state
                    .show_error(t!("error.create_file_failed", error = e.to_string()).to_string());
            }
        }
    }

    /// Handle creating a new folder.
    pub(crate) fn handle_create_folder(&mut self, path: std::path::PathBuf) {
        match std::fs::create_dir(&path) {
            Ok(_) => {
                info!("Created new folder: {}", path.display());
                let time = self.get_app_time();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder");
                self.state.show_toast(
                    t!("notification.created", name = name).to_string(),
                    time,
                    2.0,
                );

                // Refresh file tree
                self.refresh_workspace_tree();
            }
            Err(e) => {
                warn!("Failed to create folder: {}", e);
                self.state.show_error(
                    t!("error.create_folder_failed", error = e.to_string()).to_string(),
                );
            }
        }
    }

    /// Handle renaming a file or folder.
    pub(crate) fn handle_rename_file(
        &mut self,
        old_path: std::path::PathBuf,
        new_path: std::path::PathBuf,
    ) {
        match std::fs::rename(&old_path, &new_path) {
            Ok(_) => {
                info!("Renamed: {} -> {}", old_path.display(), new_path.display());
                let time = self.get_app_time();
                let new_name = new_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("item");
                self.state.show_toast(
                    t!("notification.renamed_to", name = new_name).to_string(),
                    time,
                    2.0,
                );

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
                self.refresh_workspace_tree();
            }
            Err(e) => {
                warn!("Failed to rename: {}", e);
                self.state
                    .show_error(t!("error.rename_failed", error = e.to_string()).to_string());
            }
        }
    }

    /// Handle deleting a file or folder.
    ///
    /// # Parameters
    /// - `path` - Path to the file or folder to delete
    /// - `ctx` - Optional egui Context for cleaning up tab state memory
    pub(crate) fn handle_delete_file(
        &mut self,
        path: std::path::PathBuf,
        ctx: Option<&egui::Context>,
    ) {
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
                self.state.show_toast(
                    t!("notification.deleted", name = name).to_string(),
                    time,
                    2.0,
                );

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
                self.refresh_workspace_tree();
            }
            Err(e) => {
                warn!("Failed to delete: {}", e);
                self.state
                    .show_error(t!("error.delete_failed", error = e.to_string()).to_string());
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Wikilink Navigation
    // ─────────────────────────────────────────────────────────────────────────

    /// Navigate to a wikilink target by resolving it to a file path and opening it.
    ///
    /// Resolution order:
    /// 1. Relative to the current file's directory (with and without `.md`)
    /// 2. Relative to the workspace root (if in workspace mode)
    /// 3. If not found, show an error toast
    pub(crate) fn navigate_wikilink(&mut self, target: &str) {
        let current_dir = self
            .state
            .active_tab()
            .and_then(|tab| tab.path.as_ref())
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        let workspace_root = self.state.workspace_root().cloned();

        // Build candidate paths
        let resolved =
            resolve_wikilink_target(target, current_dir.as_deref(), workspace_root.as_deref());

        match resolved {
            Some(path) => {
                info!("Wikilink [[{}]] resolved to: {}", target, path.display());
                let time = self.get_app_time();
                match self.state.open_file(path.clone(), Some(time)) {
                    Ok(tab_index) => {
                        self.pending_cjk_check = true;
                        self.check_auto_save_recovery(tab_index);
                        debug!("Opened wikilink target in tab {}", tab_index);
                    }
                    Err(e) => {
                        warn!("Failed to open wikilink target '{}': {}", target, e);
                        let time = self.get_app_time();
                        self.state.show_toast(
                            t!(
                                "notification.wikilink_open_failed",
                                target = target,
                                error = e.to_string()
                            )
                            .to_string(),
                            time,
                            3.0,
                        );
                    }
                }
            }
            None => {
                warn!("Wikilink target '{}' not found", target);
                let time = self.get_app_time();
                self.state.show_toast(
                    t!("notification.wikilink_not_found", target = target).to_string(),
                    time,
                    3.0,
                );
            }
        }
    }
}

impl FerriteApp {
    /// Show a portal error dialog with installation instructions for the user's distro.
    ///
    /// This is called when a file dialog fails on Linux desktops like Hyprland
    /// that require xdg-desktop-portal but don't have it properly configured.
    pub(crate) fn show_portal_error_dialog(
        &mut self,
        desktop_env: Option<String>,
        operation: &str,
    ) {
        let (cmd, packages) = portal_install_instructions(desktop_env.as_deref());
        let packages_str = packages.join(" ");
        let full_cmd = format!("{} {}", cmd, packages_str);

        let desktop_name = desktop_env.as_deref().unwrap_or("your Linux desktop");

        let message = format!(
            "File {operation} dialog failed.\n\n\
            {desktop_name} requires xdg-desktop-portal for file dialogs.\n\n\
            To fix this, install the portal packages:\n\n\
            {full_cmd}",
        );

        log::warn!(
            "Showing portal error dialog for {}: {}",
            desktop_name,
            message
        );
        self.state.show_portal_error(message, full_cmd);
    }
}

/// Resolve a wikilink target string to a file path.
///
/// Tries these candidates in order:
/// 1. `{current_dir}/{target}` (exact)
/// 2. `{current_dir}/{target}.md`
/// 3. `{workspace_root}/{target}` (exact)
/// 4. `{workspace_root}/{target}.md`
/// 5. Recursive search in workspace for `{target}.md` (same-folder-first, shortest path)
///
/// Returns the first existing path found, or `None`.
fn resolve_wikilink_target(
    target: &str,
    current_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> Option<PathBuf> {
    // Normalize the target: trim whitespace
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    // Helper: check exact path and path with .md extension
    let check_with_md = |dir: &Path| -> Option<PathBuf> {
        // Exact match first
        let exact = dir.join(target);
        if exact.is_file() {
            return Some(exact);
        }
        // With .md extension (only if target doesn't already end with .md)
        if !target.to_lowercase().ends_with(".md") {
            let with_md = dir.join(format!("{}.md", target));
            if with_md.is_file() {
                return Some(with_md);
            }
        }
        None
    };

    // 1. Relative to current file's directory
    if let Some(dir) = current_dir {
        if let Some(found) = check_with_md(dir) {
            return Some(found);
        }
    }

    // 2. Relative to workspace root
    if let Some(root) = workspace_root {
        if let Some(found) = check_with_md(root) {
            return Some(found);
        }

        // 3. Recursive search in workspace for matching file
        // Build the expected filename
        let filename_md = if target.to_lowercase().ends_with(".md") {
            target.to_string()
        } else {
            format!("{}.md", target)
        };
        let filename_lower = filename_md.to_lowercase();

        // Walk the workspace looking for the file
        let mut candidates: Vec<PathBuf> = Vec::new();
        collect_matching_files(root, &filename_lower, &mut candidates);

        if !candidates.is_empty() {
            // Tie-breaking: prefer same folder, then shortest path
            candidates.sort_by(|a, b| {
                let a_same_dir = current_dir.map_or(false, |d| a.parent() == Some(d));
                let b_same_dir = current_dir.map_or(false, |d| b.parent() == Some(d));
                // Same-folder first
                b_same_dir.cmp(&a_same_dir).then_with(|| {
                    // Shorter path wins
                    a.components().count().cmp(&b.components().count())
                })
            });
            return Some(candidates.into_iter().next().unwrap());
        }
    }

    None
}

/// Recursively collect files matching a given lowercase filename.
fn collect_matching_files(dir: &Path, filename_lower: &str, results: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden directories and common non-content dirs
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
            }
            collect_matching_files(&path, filename_lower, results);
        } else if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.to_lowercase() == filename_lower {
                    results.push(path);
                }
            }
        }
    }
}
