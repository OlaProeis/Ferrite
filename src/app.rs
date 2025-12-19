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

use crate::config::{Settings, Theme, ViewMode, WindowSize};
use crate::editor::{
    extract_outline_for_file, DocumentOutline, EditorWidget, FindReplacePanel, SearchHighlights,
    TextStats,
};
use crate::export::{copy_html_to_clipboard, generate_html_document};
use crate::files::dialogs::{open_multiple_files_dialog, save_file_dialog};
use crate::fonts;
use crate::markdown::{
    apply_raw_format, detect_raw_formatting_state, get_structured_file_type, EditorMode,
    FormattingState, MarkdownEditor, MarkdownFormatCommand, TreeViewer, TreeViewerState,
};
use crate::preview::{ScrollOrigin, SyncScrollState};
use crate::state::{AppState, FileType, PendingAction};
use crate::theme::{ThemeColors, ThemeManager};
use crate::ui::{
    handle_window_resize, AboutPanel, FileOperationDialog, FileOperationResult,
    FileTreeContextAction, FileTreePanel, OutlinePanel, QuickSwitcher, Ribbon, RibbonAction,
    SearchPanel, SettingsPanel, WindowResizeState,
};
use eframe::egui;
use log::{debug, info, warn};
use std::collections::HashMap;

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
    /// Undo (Ctrl+Z)
    Undo,
    /// Redo (Ctrl+Y or Ctrl+Shift+Z)
    Redo,
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
    /// Close find panel (Escape)
    CloseFindPanel,
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
    /// Cached document outline (updated when content changes)
    cached_outline: DocumentOutline,
    /// Hash of the last content used to generate outline (for change detection)
    last_outline_content_hash: u64,
    /// Pending scroll-to-line request from outline navigation (1-indexed)
    pending_scroll_to_line: Option<usize>,
    /// Tree viewer states per tab (keyed by tab ID)
    tree_viewer_states: HashMap<usize, TreeViewerState>,
    /// Sync scroll states per tab (keyed by tab ID)
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
}

impl FerriteApp {
    /// Create a new FerriteApp instance.
    ///
    /// This initializes the application state from the config file and applies
    /// the saved theme preference.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing Ferrite");

        // Set up custom fonts with proper bold/italic variants
        fonts::setup_fonts(&cc.egui_ctx);

        let state = AppState::new();

        // Initialize theme manager with saved theme preference
        let mut theme_manager = ThemeManager::new(state.settings.theme);

        // Apply initial theme to egui context
        theme_manager.apply(&cc.egui_ctx);
        info!("Applied initial theme: {:?}", state.settings.theme);

        // Initialize outline panel with saved settings
        let outline_panel = OutlinePanel::new()
            .with_width(state.settings.outline_width)
            .with_side(state.settings.outline_side);

        Self {
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
            cached_outline: DocumentOutline::new(),
            last_outline_content_hash: 0,
            pending_scroll_to_line: None,
            tree_viewer_states: HashMap::new(),
            sync_scroll_states: HashMap::new(),
            should_exit: false,
            last_window_size: None,
            last_window_pos: None,
            start_time: std::time::Instant::now(),
            previous_view_mode: None,
            window_resize_state: WindowResizeState::new(),
        }
    }

    /// Get elapsed time since app start in seconds.
    fn get_app_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
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

    /// Render the main UI content.
    /// Returns a deferred format command if one was requested from the ribbon.
    fn render_ui(&mut self, ctx: &egui::Context) -> Option<MarkdownFormatCommand> {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_dark = ctx.style().visuals.dark_mode;

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

                // Title bar row
                ui.horizontal(|ui| {
                    ui.add_space(8.0);

                    // App icon/logo placeholder
                    ui.label(egui::RichText::new("📝").size(14.0));

                    ui.add_space(8.0);

                    // Window title (dynamically generated)
                    let title = self.window_title();
                    ui.label(egui::RichText::new(title).size(12.0).color(text_color));

                    // Fill remaining space with draggable area
                    let drag_rect = ui.available_rect_before_wrap();
                    let drag_response = ui.allocate_rect(drag_rect, egui::Sense::click_and_drag());

                    // Handle double-click to maximize/restore
                    if drag_response.double_clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // Handle drag to move window (but not if we're in a resize zone)
                    // The resize handling runs before UI rendering and sets the resize state
                    let is_in_resize = self.window_resize_state.current_direction().is_some()
                        || self.window_resize_state.is_resizing();
                    if drag_response.dragged() && !is_in_resize {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

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
                        close_btn.on_hover_text("Close");

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
                        min_btn.on_hover_text("Minimize");
                    });
                });

                ui.add_space(2.0);
            });

        // Ribbon panel (below title bar)
        let ribbon_action = {
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
                    );
                });
            action
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

        // Bottom panel for status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left side: File path (clickable for recent files popup)
                let path_display = if let Some(tab) = self.state.active_tab() {
                    tab.path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Untitled".to_string())
                } else {
                    "No file open".to_string()
                };

                // Make the file path a clickable button that opens the recent files popup
                let has_recent_files = !self.state.settings.recent_files.is_empty();
                let popup_id = ui.make_persistent_id("recent_files_popup");

                let button_response = ui.add(
                    egui::Button::new(&path_display)
                        .frame(false)
                        .sense(if has_recent_files {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        })
                );

                if has_recent_files {
                    button_response.clone().on_hover_text("Click for recent files\nShift+Click to open in background");
                }

                // Toggle popup on click
                let just_opened = if button_response.clicked() && has_recent_files {
                    self.state.ui.show_recent_files_popup = !self.state.ui.show_recent_files_popup;
                    self.state.ui.show_recent_files_popup // true if we just opened it
                } else {
                    false
                };

                // Show recent files popup
                if self.state.ui.show_recent_files_popup && has_recent_files {
                    let popup_response = egui::Area::new(popup_id)
                        .order(egui::Order::Foreground)
                        .fixed_pos(button_response.rect.left_bottom())
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.set_min_width(300.0);
                                ui.label(egui::RichText::new("Recent Files").strong());
                                ui.separator();

                                // Show up to 5 recent files
                                let recent_files: Vec<_> = self.state.settings.recent_files
                                    .iter()
                                    .take(5)
                                    .cloned()
                                    .collect();

                                let mut file_to_open: Option<(std::path::PathBuf, bool)> = None;

                                for path in &recent_files {
                                    let file_name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown");
                                    let parent_dir = path
                                        .parent()
                                        .and_then(|p| p.to_str())
                                        .unwrap_or("");

                                    // Use theme-aware colors for file names
                                    let file_name_color = if is_dark {
                                        egui::Color32::from_rgb(220, 220, 220) // Light text for dark mode
                                    } else {
                                        egui::Color32::from_rgb(30, 30, 30) // Dark text for light mode
                                    };

                                    let item_response = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(file_name).strong().color(file_name_color)
                                        )
                                        .frame(false)
                                        .min_size(egui::vec2(ui.available_width(), 0.0))
                                    );

                                    // Show path on hover
                                    item_response.clone().on_hover_text(format!(
                                        "{}\n\nClick: Open with focus\nShift+Click: Open in background",
                                        path.display()
                                    ));

                                    // Show parent directory in smaller text with theme-aware color
                                    if !parent_dir.is_empty() {
                                        let secondary_color = if is_dark {
                                            egui::Color32::from_rgb(160, 160, 160) // Light gray for dark mode
                                        } else {
                                            egui::Color32::from_rgb(80, 80, 80) // Dark gray for light mode
                                        };
                                        ui.label(egui::RichText::new(parent_dir).small().color(secondary_color));
                                    }

                                    ui.add_space(4.0);

                                    if item_response.clicked() {
                                        // Check if shift is held for background open
                                        let shift_held = ui.input(|i| i.modifiers.shift);
                                        file_to_open = Some((path.clone(), !shift_held));
                                    }
                                }

                                file_to_open
                            })
                        });

                    // Handle file opening after UI is done
                    if let Some((path, focus)) = popup_response.inner.inner {
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
                        .on_hover_text("About / Help (F1)")
                        .clicked()
                    {
                        self.state.toggle_about();
                    }

                    if let Some(tab) = self.state.active_tab() {
                        ui.separator();

                        // Cursor position
                        let (line, col) = tab.cursor_position;
                        ui.label(format!("Ln {}, Col {}", line + 1, col + 1));

                        ui.separator();

                        // Encoding (Rust strings are always UTF-8)
                        ui.label("UTF-8");

                        ui.separator();

                        // Text statistics
                        let stats = TextStats::from_text(&tab.content);
                        ui.label(stats.format_compact());
                    }
                });
            });
        });

        // ═══════════════════════════════════════════════════════════════════
        // Outline Panel (if enabled)
        // ═══════════════════════════════════════════════════════════════════
        let mut outline_scroll_to_line: Option<usize> = None;
        let mut outline_toggled_id: Option<String> = None;
        let mut outline_new_width: Option<f32> = None;
        let mut outline_close_requested = false;

        if self.state.settings.outline_enabled {
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
            let outline_output = self.outline_panel.show(ctx, &self.cached_outline, is_dark);

            // Capture output for processing after render
            outline_scroll_to_line = outline_output.scroll_to_line;
            outline_toggled_id = outline_output.toggled_id;
            outline_new_width = outline_output.new_width;
            outline_close_requested = outline_output.close_requested;
        }

        // Handle outline panel interactions
        if let Some(line) = outline_scroll_to_line {
            // Store the scroll request - will be processed when editor renders
            self.pending_scroll_to_line = Some(line);
            // Also update cursor position so it stays at the target line
            self.scroll_to_line(line);
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
        // File Tree Panel (workspace mode only)
        // ═══════════════════════════════════════════════════════════════════
        let mut file_tree_file_clicked: Option<std::path::PathBuf> = None;
        let mut file_tree_path_toggled: Option<std::path::PathBuf> = None;
        let mut file_tree_close_requested = false;
        let mut file_tree_new_width: Option<f32> = None;
        let mut file_tree_context_action: Option<FileTreeContextAction> = None;

        if self.state.should_show_file_tree() {
            if let Some(workspace) = &self.state.workspace {
                let workspace_name = workspace
                    .root_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Workspace");

                let output =
                    self.file_tree_panel
                        .show(ctx, &workspace.file_tree, workspace_name, is_dark);

                file_tree_file_clicked = output.file_clicked;
                file_tree_path_toggled = output.path_toggled;
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

        // Central panel for editor content
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar - uses custom wrapping layout for multi-line support
            let mut tab_to_close: Option<usize> = None;

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

            // Calculate tab widths and layout
            let mut current_x = 0.0;
            let mut current_row = 0;
            let mut tab_positions: Vec<(f32, usize)> = Vec::new(); // (x position, row)

            for (_, title, _) in &tab_titles {
                // Estimate tab width: title + close button + padding
                let text_width = ui
                    .fonts(|f| {
                        f.glyph_width(&egui::FontId::default(), 'M') * title.len() as f32 * 0.6
                    })
                    .max(60.0);
                let tab_width = text_width + close_btn_width + 16.0; // padding

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

            for (idx, ((tab_idx, title, selected), (x_pos, row))) in
                tab_titles.iter().zip(tab_positions.iter()).enumerate()
            {
                // Calculate tab dimensions
                let text_width = ui
                    .fonts(|f| {
                        f.glyph_width(&egui::FontId::default(), 'M') * title.len() as f32 * 0.6
                    })
                    .max(60.0);
                let tab_width = text_width + close_btn_width + 16.0;

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

                // Draw tab title
                let title_rect = egui::Rect::from_min_size(
                    tab_rect.min + egui::vec2(8.0, 4.0),
                    egui::vec2(text_width, tab_height - 8.0),
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

            // Draw + button
            let plus_x = if tab_positions.is_empty() {
                0.0
            } else {
                let last_pos = tab_positions.last().unwrap();
                let last_title = &tab_titles.last().unwrap().1;
                let last_width = ui
                    .fonts(|f| {
                        f.glyph_width(&egui::FontId::default(), 'M') * last_title.len() as f32 * 0.6
                    })
                    .max(60.0)
                    + close_btn_width
                    + 16.0;

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
            plus_response.on_hover_text("New tab");

            // Handle tab close action
            if let Some(index) = tab_to_close {
                self.state.close_tab(index);
            }

            ui.separator();

            // Editor widget - extract settings values to avoid borrow conflicts
            let font_size = self.state.settings.font_size;
            let font_family = self.state.settings.font_family;
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
                )
            });

            if let Some((tab_id, view_mode, structured_type)) = tab_info {
                match view_mode {
                    ViewMode::Raw => {
                        // Raw mode: use the plain EditorWidget
                        if let Some(tab) = self.state.active_tab_mut() {
                            let mut editor = EditorWidget::new(tab)
                                .font_size(font_size)
                                .font_family(font_family)
                                .word_wrap(word_wrap)
                                .show_line_numbers(show_line_numbers)
                                .theme_colors(theme_colors.clone())
                                .id(egui::Id::new("main_editor_raw"))
                                .scroll_to_line(scroll_to_line);

                            // Add search highlights if available
                            if let Some(highlights) = search_highlights.clone() {
                                editor = editor.search_highlights(highlights);
                            }

                            let editor_output = editor.show(ui);

                            if editor_output.changed {
                                debug!("Content modified in raw editor");
                            }
                        }
                    }
                    ViewMode::Rendered => {
                        // Check if this is a structured file (JSON, YAML, TOML)
                        if let Some(file_type) = structured_type {
                            // Structured file: use the TreeViewer
                            // Note: For structured files, the outline panel shows statistics
                            // rather than navigation, so scroll_to_line is not used here.
                            let tree_state = self.tree_viewer_states.entry(tab_id).or_default();

                            if let Some(tab) = self.state.active_tab_mut() {
                                let output =
                                    TreeViewer::new(&mut tab.content, file_type, tree_state)
                                        .font_size(font_size)
                                        .show(ui);

                                if output.changed {
                                    debug!("Content modified in tree viewer");
                                }

                                // Update scroll offset for sync scrolling
                                tab.scroll_offset = output.scroll_offset;
                            }
                        } else {
                            // Markdown file: use the WYSIWYG MarkdownEditor
                            if let Some(tab) = self.state.active_tab_mut() {
                                let editor_output = MarkdownEditor::new(&mut tab.content)
                                    .mode(EditorMode::Rendered)
                                    .font_size(font_size)
                                    .font_family(font_family)
                                    .word_wrap(word_wrap)
                                    .theme(theme)
                                    .id(egui::Id::new("main_editor_rendered"))
                                    .scroll_to_line(scroll_to_line)
                                    .show(ui);

                                if editor_output.changed {
                                    // Content is already modified through the mutable reference
                                    debug!("Content modified in rendered editor");
                                }

                                // Update cursor position from rendered editor
                                tab.cursor_position = editor_output.cursor_position;

                                // Update scroll offset for sync scrolling
                                let old_scroll = tab.scroll_offset;
                                tab.scroll_offset = editor_output.scroll_offset;
                                if (old_scroll - editor_output.scroll_offset).abs() > 1.0 {
                                    debug!(
                                        "MarkdownEditor scroll: {} → {}",
                                        old_scroll, editor_output.scroll_offset
                                    );
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
                    self.handle_delete_file(path);
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
                if let Some((file_path, line_number)) = output.navigate_to {
                    match self.state.open_file(file_path.clone()) {
                        Ok(_) => {
                            debug!(
                                "Opened file from search: {} at line {}",
                                file_path.display(),
                                line_number
                            );
                            // TODO(enhancement): Navigate to line_number in the editor
                            // This would scroll the editor to the specific line from search results
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
                Ok(_) => {
                    success_count += 1;
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

            match self.state.save_active_tab() {
                Ok(_) => {
                    debug!("File saved successfully");
                    let time = self.get_app_time();
                    self.state
                        .show_toast(format!("Saved: {}", path_display), time, 3.0);
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
            match self.state.save_active_tab_as(path.clone()) {
                Ok(_) => {
                    let time = self.get_app_time();
                    self.state
                        .show_toast(format!("Saved: {}", path.display()), time, 3.0);
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

        // Check if any dropped item is a directory
        let mut folders: Vec<std::path::PathBuf> = Vec::new();
        let mut files: Vec<std::path::PathBuf> = Vec::new();

        for path in dropped_files {
            if path.is_dir() {
                folders.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }

        // If a folder was dropped, open it as a workspace
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
                }
                Err(e) => {
                    warn!("Failed to open workspace: {}", e);
                    self.state
                        .show_error(format!("Failed to open workspace:\n{}", e));
                }
            }
            return; // Prioritize folder over files
        }

        // If files were dropped, open them in tabs
        for file in files {
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                // Only open markdown files
                if matches!(
                    ext.to_lowercase().as_str(),
                    "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "txt"
                ) {
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
    fn handle_delete_file(&mut self, path: std::path::PathBuf) {
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
                let tabs_to_close: Vec<usize> = self
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
                    .map(|(i, _)| i)
                    .collect();

                // Close tabs in reverse order to maintain indices
                for &index in tabs_to_close.iter().rev() {
                    self.state.close_tab(index);
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
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+Shift+S: Save As (check first since it's more specific)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                debug!("Keyboard shortcut: Ctrl+Shift+S (Save As)");
                return Some(KeyboardAction::SaveAs);
            }

            // Ctrl+E: Toggle View Mode
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::E) {
                debug!("Keyboard shortcut: Ctrl+E (Toggle View Mode)");
                return Some(KeyboardAction::ToggleViewMode);
            }

            // Ctrl+Shift+T: Cycle Theme
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                debug!("Keyboard shortcut: Ctrl+Shift+T (Cycle Theme)");
                return Some(KeyboardAction::CycleTheme);
            }

            // Ctrl+Shift+Tab: Previous tab
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                debug!("Keyboard shortcut: Ctrl+Shift+Tab (Previous Tab)");
                return Some(KeyboardAction::PrevTab);
            }

            // Ctrl+Shift+Z: Redo (check before Ctrl+Z since it's more specific)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z) {
                debug!("Keyboard shortcut: Ctrl+Shift+Z (Redo)");
                return Some(KeyboardAction::Redo);
            }

            // Ctrl+Z: Undo
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z) {
                debug!("Keyboard shortcut: Ctrl+Z (Undo)");
                return Some(KeyboardAction::Undo);
            }

            // Ctrl+Y: Redo
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Y) {
                debug!("Keyboard shortcut: Ctrl+Y (Redo)");
                return Some(KeyboardAction::Redo);
            }

            // Ctrl+S: Save
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
                debug!("Keyboard shortcut: Ctrl+S (Save)");
                return Some(KeyboardAction::Save);
            }

            // Ctrl+O: Open
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                debug!("Keyboard shortcut: Ctrl+O (Open)");
                return Some(KeyboardAction::Open);
            }

            // Ctrl+N: New file
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                debug!("Keyboard shortcut: Ctrl+N (New)");
                return Some(KeyboardAction::New);
            }

            // Ctrl+T: New tab
            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                debug!("Keyboard shortcut: Ctrl+T (New Tab)");
                return Some(KeyboardAction::NewTab);
            }

            // Ctrl+W: Close current tab
            if i.modifiers.ctrl && i.key_pressed(egui::Key::W) {
                debug!("Keyboard shortcut: Ctrl+W (Close Tab)");
                return Some(KeyboardAction::CloseTab);
            }

            // Ctrl+Tab: Next tab
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                debug!("Keyboard shortcut: Ctrl+Tab (Next Tab)");
                return Some(KeyboardAction::NextTab);
            }

            // Ctrl+,: Open settings
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
                debug!("Keyboard shortcut: Ctrl+, (Open Settings)");
                return Some(KeyboardAction::OpenSettings);
            }

            // Ctrl+F: Open find panel
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::F) {
                debug!("Keyboard shortcut: Ctrl+F (Open Find)");
                return Some(KeyboardAction::OpenFind);
            }

            // Ctrl+H: Open find and replace panel
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                debug!("Keyboard shortcut: Ctrl+H (Open Find/Replace)");
                return Some(KeyboardAction::OpenFindReplace);
            }

            // F1: Open About/Help panel
            if i.key_pressed(egui::Key::F1) {
                debug!("Keyboard shortcut: F1 (Open About)");
                return Some(KeyboardAction::OpenAbout);
            }

            // F3: Find next (only when find panel is open)
            if i.key_pressed(egui::Key::F3) && !i.modifiers.shift {
                debug!("Keyboard shortcut: F3 (Find Next)");
                return Some(KeyboardAction::FindNext);
            }

            // Shift+F3: Find previous (only when find panel is open)
            if i.key_pressed(egui::Key::F3) && i.modifiers.shift {
                debug!("Keyboard shortcut: Shift+F3 (Find Previous)");
                return Some(KeyboardAction::FindPrev);
            }

            // ═══════════════════════════════════════════════════════════════════
            // Formatting shortcuts (editor-scoped)
            // ═══════════════════════════════════════════════════════════════════

            // Ctrl+Shift+B: Bullet list
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::B) {
                debug!("Keyboard shortcut: Ctrl+Shift+B (Bullet List)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::BulletList));
            }

            // Ctrl+Shift+N: Numbered list
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::N) {
                debug!("Keyboard shortcut: Ctrl+Shift+N (Numbered List)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::NumberedList));
            }

            // Ctrl+Shift+O: Toggle outline panel
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::O) {
                debug!("Keyboard shortcut: Ctrl+Shift+O (Toggle Outline)");
                return Some(KeyboardAction::ToggleOutline);
            }

            // Ctrl+B: Toggle file tree panel (when in workspace mode)
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::B) {
                debug!("Keyboard shortcut: Ctrl+B (Toggle File Tree)");
                return Some(KeyboardAction::ToggleFileTree);
            }

            // Ctrl+P: Quick file switcher (workspace mode only)
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::P) {
                debug!("Keyboard shortcut: Ctrl+P (Quick Open)");
                return Some(KeyboardAction::QuickOpen);
            }

            // Ctrl+Shift+F: Search in files (workspace mode only)
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                debug!("Keyboard shortcut: Ctrl+Shift+F (Search in Files)");
                return Some(KeyboardAction::SearchInFiles);
            }

            // Ctrl+Shift+E: Export as HTML
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::E) {
                debug!("Keyboard shortcut: Ctrl+Shift+E (Export HTML)");
                return Some(KeyboardAction::ExportHtml);
            }

            // Ctrl+Shift+C: Code block
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::C) {
                debug!("Keyboard shortcut: Ctrl+Shift+C (Code Block)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::CodeBlock));
            }

            // Ctrl+Shift+K: Image
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::K) {
                debug!("Keyboard shortcut: Ctrl+Shift+K (Image)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::Image));
            }

            // Ctrl+B: Bold (must check after Ctrl+Shift+B)
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::B) {
                debug!("Keyboard shortcut: Ctrl+B (Bold)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::Bold));
            }

            // Ctrl+I: Italic
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::I) {
                debug!("Keyboard shortcut: Ctrl+I (Italic)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::Italic));
            }

            // Ctrl+K: Link (must check after Ctrl+Shift+K)
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::K) {
                debug!("Keyboard shortcut: Ctrl+K (Link)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::Link));
            }

            // Ctrl+Q: Blockquote
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Q) {
                debug!("Keyboard shortcut: Ctrl+Q (Blockquote)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::Blockquote));
            }

            // Ctrl+`: Inline code
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Backtick) {
                debug!("Keyboard shortcut: Ctrl+` (Inline Code)");
                return Some(KeyboardAction::Format(MarkdownFormatCommand::InlineCode));
            }

            // Ctrl+1-6: Headings
            if i.modifiers.ctrl && !i.modifiers.shift {
                if i.key_pressed(egui::Key::Num1) {
                    debug!("Keyboard shortcut: Ctrl+1 (Heading 1)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(1)));
                }
                if i.key_pressed(egui::Key::Num2) {
                    debug!("Keyboard shortcut: Ctrl+2 (Heading 2)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(2)));
                }
                if i.key_pressed(egui::Key::Num3) {
                    debug!("Keyboard shortcut: Ctrl+3 (Heading 3)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(3)));
                }
                if i.key_pressed(egui::Key::Num4) {
                    debug!("Keyboard shortcut: Ctrl+4 (Heading 4)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(4)));
                }
                if i.key_pressed(egui::Key::Num5) {
                    debug!("Keyboard shortcut: Ctrl+5 (Heading 5)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(5)));
                }
                if i.key_pressed(egui::Key::Num6) {
                    debug!("Keyboard shortcut: Ctrl+6 (Heading 6)");
                    return Some(KeyboardAction::Format(MarkdownFormatCommand::Heading(6)));
                }
            }

            // Escape: Close find panel (if open)
            if i.key_pressed(egui::Key::Escape) {
                debug!("Keyboard shortcut: Escape (Close Find Panel)");
                return Some(KeyboardAction::CloseFindPanel);
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
                self.handle_close_current_tab();
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
            KeyboardAction::Undo => {
                self.handle_undo();
            }
            KeyboardAction::Redo => {
                self.handle_redo();
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
            KeyboardAction::CloseFindPanel => {
                if self.state.ui.show_find_replace {
                    self.state.ui.show_find_replace = false;
                }
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
        });
    }

    /// Handle closing the current tab (with unsaved prompt if needed).
    fn handle_close_current_tab(&mut self) {
        let index = self.state.active_tab_index();
        self.state.close_tab(index);
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

    /// Toggle between Raw and Rendered view modes for the active tab.
    ///
    /// When sync scrolling is enabled, this calculates the corresponding scroll
    /// position in the target mode based on the current scroll position.
    fn handle_toggle_view_mode(&mut self) {
        // Get sync scroll setting before mutable borrow
        let sync_enabled = self.state.settings.sync_scroll_enabled;

        if let Some(tab) = self.state.active_tab_mut() {
            let old_mode = tab.view_mode;
            let tab_id = tab.id;
            let current_scroll = tab.scroll_offset;
            let content = tab.content.clone();

            // Debug: log the current state before toggle
            debug!(
                "Toggle view mode: old_mode={:?}, current_scroll={}, sync_enabled={}",
                old_mode, current_scroll, sync_enabled
            );

            // Toggle the view mode
            let new_mode = tab.toggle_view_mode();
            debug!("View mode toggled to: {:?} for tab {}", new_mode, tab.id);

            // Handle sync scrolling when switching modes
            if sync_enabled {
                let sync_state = self.sync_scroll_states.entry(tab_id).or_default();

                // Update source metadata for proportional fallback
                let line_count = content.lines().count().max(1);
                sync_state.set_source_metadata(line_count, line_count as f32 * 20.0); // Approximate

                // Calculate target scroll position based on mode switch
                match (old_mode, new_mode) {
                    (ViewMode::Raw, ViewMode::Rendered) => {
                        // Going from Raw to Rendered
                        sync_state.update_raw_offset(current_scroll);
                        sync_state.mark_scroll(ScrollOrigin::Raw);

                        // Estimate line height (will be refined when editor renders)
                        let estimated_line_height = 20.0; // Approximate
                        let source_line =
                            sync_state.raw_offset_to_line(current_scroll, estimated_line_height);
                        let target_offset = sync_state.line_to_rendered_offset(source_line);

                        // Set pending scroll target for rendered view
                        self.pending_scroll_to_line = Some(source_line);

                        debug!(
                            "Sync scroll Raw→Rendered: offset {} → line {} → target offset {}",
                            current_scroll, source_line, target_offset
                        );
                    }
                    (ViewMode::Rendered, ViewMode::Raw) => {
                        // Going from Rendered to Raw
                        sync_state.update_rendered_offset(current_scroll);
                        sync_state.mark_scroll(ScrollOrigin::Rendered);

                        // For Rendered→Raw, use proportional estimation based on scroll position
                        // Since rendered content may have variable heights (headings, code blocks),
                        // we estimate using the same approach as Raw: offset / estimated_line_height
                        let estimated_line_height = 20.0;
                        let source_line = ((current_scroll / estimated_line_height) as usize)
                            .saturating_add(1)
                            .max(1);

                        // Set pending scroll target for raw view
                        self.pending_scroll_to_line = Some(source_line);

                        debug!(
                            "Sync scroll Rendered→Raw: offset {} → line {} (line_count={}, scroll_offset_was={})",
                            current_scroll, source_line, line_count, current_scroll
                        );
                    }
                    _ => {}
                }
            }

            // Mark settings dirty to save per-tab view mode on exit
            self.state.mark_settings_dirty();
        }
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
    fn handle_undo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_undo() {
                let undo_count = tab.undo_count();
                if tab.undo() {
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
    fn handle_redo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_redo() {
                let redo_count = tab.redo_count();
                if tab.redo() {
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

            // Generate HTML
            match generate_html_document(&content, Some(title), &theme_colors, true) {
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
                            tab.content = formatted;
                            tab.record_edit(old_content);
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
                self.last_outline_content_hash = content_hash;
            }
        } else {
            // No active tab, clear outline
            if !self.cached_outline.is_empty() {
                self.cached_outline = DocumentOutline::new();
                self.last_outline_content_hash = 0;
            }
        }
    }

    /// Scroll the editor to a specific line (1-indexed).
    fn scroll_to_line(&mut self, line: usize) {
        if let Some(tab) = self.state.active_tab_mut() {
            // Calculate character offset for the start of the line
            let content = &tab.content;
            let mut char_offset = 0;
            let mut current_line = 1;

            for (idx, ch) in content.chars().enumerate() {
                if current_line == line {
                    char_offset = idx;
                    break;
                }
                if ch == '\n' {
                    current_line += 1;
                }
            }

            // Update cursor position to the start of the line
            tab.cursor_position = (line.saturating_sub(1), 0);

            debug!("Scrolling to line {} (char offset {})", line, char_offset);
        }
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
        }
    }

    /// Render dialog windows.
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        // Confirmation dialog for unsaved changes
        if self.state.ui.show_confirm_dialog {
            egui::Window::new("Unsaved Changes")
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

                        // "Save" button - save then proceed with action
                        if ui.button("Save").clicked() {
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
                        if ui.button("Discard").clicked() {
                            self.state.handle_confirmed_action();
                            if is_exit {
                                self.should_exit = true;
                            }
                        }

                        // "Cancel" button - abort the action
                        if ui.button("Cancel").clicked() {
                            self.state.cancel_pending_action();
                        }
                    });
                });
        }

        // Error modal
        if self.state.ui.show_error_modal {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("⚠").size(24.0));
                    ui.label(&self.state.ui.error_message);
                    ui.separator();
                    if ui.button("OK").clicked() {
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
            let output = self
                .settings_panel
                .show(ctx, &mut self.state.settings, is_dark);

            if output.changed {
                // Apply theme changes immediately
                self.theme_manager.set_theme(self.state.settings.theme);
                self.theme_manager.apply(ctx);
                self.state.mark_settings_dirty();
            }

            if output.reset_requested {
                // Reset to defaults
                let default_settings = Settings::default();
                self.state.settings = default_settings;
                self.theme_manager.set_theme(self.state.settings.theme);
                self.theme_manager.apply(ctx);
                self.state.mark_settings_dirty();

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

        // Update toast message (clear if expired)
        let current_time = self.get_app_time();
        self.state.update_toast(current_time);

        // Update window title if it changed
        let title = self.window_title();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // Track window size/position changes for persistence
        self.update_window_state(ctx);

        // Handle drag-drop of files and folders
        self.handle_dropped_files(ctx);

        // Poll file watcher for workspace changes
        self.handle_file_watcher_events();

        // Handle close request from window
        if ctx.input(|i| i.viewport().close_requested()) && !self.handle_close_request() {
            // Cancel the close request - we need to show a confirmation dialog
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // Render the main UI (this updates editor selection)
        let deferred_format = self.render_ui(ctx);

        // Handle keyboard shortcuts AFTER render so selection is up-to-date
        self.handle_keyboard_shortcuts(ctx);

        // Handle deferred format action from ribbon AFTER render so selection is up-to-date
        if let Some(cmd) = deferred_format {
            debug!("Applying deferred format command from ribbon: {:?}", cmd);
            self.handle_format_command(cmd);
        }

        // Request exit if confirmed
        if self.should_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Called when the application is about to close.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application exiting");
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
