//! Terminal panel UI component.
//!
//! This module provides a bottom panel with an integrated terminal emulator,
//! supporting multiple terminal tabs, split panes, and floating windows.

use crate::terminal::{
    MoveDirection, SoundNotifier, TerminalLayout, TerminalManager, TerminalWidget,
};
use eframe::egui::{self, Color32, Id, Ui};
use rust_i18n::t;

/// Represents which drop zone is being hovered during a drag operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    /// Left edge - horizontal split, new terminal on left
    Left,
    /// Right edge - horizontal split, new terminal on right
    Right,
    /// Top edge - vertical split, new terminal on top
    Top,
    /// Bottom edge - vertical split, new terminal on bottom
    Bottom,
    /// Center - merge/add to existing tab group
    Center,
}

/// Output from the terminal panel.
#[derive(Debug, Default)]
pub struct TerminalPanelOutput {
    /// Whether the panel was closed by the user
    pub closed: bool,
    /// Whether the panel visibility was toggled
    pub toggled: bool,
}

/// Per-terminal view state.
#[derive(Debug, Default, Clone)]
pub struct PerTerminalState {
    pub scroll_offset: usize,
    pub last_scrollback_len: usize,
    /// Previous waiting-for-input state (for detecting transitions)
    pub was_waiting: bool,
}

/// A floating terminal window.
pub struct FloatingWindow {
    pub id: egui::ViewportId,
    pub layout: TerminalLayout,
    pub title: String,
    pub pos: Option<egui::Pos2>,
    pub size: egui::Vec2,
    pub first_frame: bool,
}

/// Terminal panel state that persists across frames.
pub struct TerminalPanelState {
    /// Terminal manager handling all terminal instances
    pub manager: TerminalManager,
    /// Whether the terminal panel is visible
    pub visible: bool,
    /// Panel height in pixels
    pub height: f32,
    /// Whether a terminal has been initialized
    pub initialized: bool,
    /// Per-terminal view state (scroll offset, etc.)
    pub terminal_states: std::collections::HashMap<usize, PerTerminalState>,
    /// Floating terminal windows
    pub floating_windows: Vec<FloatingWindow>,
    /// Working directory for new terminals (workspace root or current file's directory)
    pub working_dir: Option<std::path::PathBuf>,
    /// Index of terminal being renamed (None if not renaming)
    pub renaming_index: Option<usize>,
    /// Temporary name buffer during rename
    pub rename_buffer: String,
    /// Whether terminal currently has keyboard focus (for shortcuts)
    pub terminal_has_focus: bool,
    /// Index of terminal pending close (waiting for confirmation)
    pub pending_close_index: Option<usize>,
    /// ID of the currently maximized terminal (if any)
    pub maximized_terminal_id: Option<usize>,
    /// Request to close a tab
    pub close_tab_request: Option<usize>,
    /// Request to pop out a tab
    pub pop_out_request: Option<(usize, Option<egui::Pos2>)>,
    /// Request to swap tabs
    pub swap_tab_request: Option<(usize, usize)>,
    /// Pending drop action from drag-to-split/merge (tab_idx, zone)
    pub pending_drop_action: Option<(usize, DropZone)>,
    /// Current index when cycling through saved layouts (for Ctrl+Shift+L)
    pub saved_layout_cycle_index: usize,
    /// Cached list of saved layout paths (refreshed on cycle)
    saved_layout_paths: Vec<std::path::PathBuf>,
    /// Whether workspace layout has been loaded for current working_dir
    pub workspace_layout_loaded: bool,
    /// Last time the layout was auto-saved (for debouncing)
    pub last_auto_save: Option<std::time::Instant>,
    /// Sound notifier for prompt detection
    pub sound_notifier: SoundNotifier,
    /// Pending error message to display as toast (consumed by app after show())
    pub pending_error: Option<String>,
    /// IDs of terminals that have been reported as exited (to avoid duplicate toasts)
    exited_terminal_ids: std::collections::HashSet<usize>,
}

impl Default for TerminalPanelState {
    fn default() -> Self {
        Self {
            manager: TerminalManager::new(),
            visible: false,
            height: 300.0,
            initialized: false,
            terminal_states: std::collections::HashMap::new(),
            floating_windows: Vec::new(),
            working_dir: None,
            renaming_index: None,
            rename_buffer: String::new(),
            terminal_has_focus: false,
            pending_close_index: None,
            maximized_terminal_id: None,
            close_tab_request: None,
            pop_out_request: None,
            swap_tab_request: None,
            pending_drop_action: None,
            saved_layout_cycle_index: 0,
            saved_layout_paths: Vec::new(),
            workspace_layout_loaded: false,
            last_auto_save: None,
            sound_notifier: SoundNotifier::new(),
            pending_error: None,
            exited_terminal_ids: std::collections::HashSet::new(),
        }
    }
}

impl TerminalPanelState {
    /// Create a new terminal panel state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the panel is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Toggle panel visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible && !self.initialized {
            self.initialize();
        }
    }

    /// Show the panel.
    pub fn show(&mut self) {
        self.visible = true;
        if !self.initialized {
            self.initialize();
        }
    }

    /// Hide the panel.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Take the pending error message (if any), clearing it.
    /// Call this after show() to display errors via toast.
    pub fn take_error(&mut self) -> Option<String> {
        self.pending_error.take()
    }

    /// Set an error message with a user-friendly recovery hint.
    fn report_error(&mut self, operation: &str, error: &str) {
        let hint = Self::recovery_hint(error);
        let user_msg = t!("terminal.error.format", operation = operation, hint = hint).to_string();
        log::error!("{}: {}", operation, error);
        self.pending_error = Some(user_msg);
    }

    /// Generate a user-friendly recovery hint based on the error message.
    fn recovery_hint(error: &str) -> &'static str {
        let error_lower = error.to_lowercase();
        if error_lower.contains("spawn")
            || error_lower.contains("not found")
            || error_lower.contains("no such file")
        {
            "Check that the shell is installed and the path is correct"
        } else if error_lower.contains("pty") || error_lower.contains("pseudo") {
            "PTY allocation failed. Try closing other terminals or restarting the app"
        } else if error_lower.contains("permission") || error_lower.contains("access denied") {
            "Permission denied. Check that you have access to run the shell"
        } else if error_lower.contains("writer") || error_lower.contains("reader") {
            "Terminal I/O failed. Try closing and reopening the terminal"
        } else if error_lower.contains("layout") || error_lower.contains("split") {
            "Failed to modify terminal layout"
        } else {
            "An unexpected error occurred. Try restarting the terminal"
        }
    }

    /// Check for terminals that have exited and return their info for notification.
    /// Call this periodically (e.g., after poll_all) to detect dead terminals.
    pub fn check_exited_terminals(&mut self) -> Vec<String> {
        let mut exited = Vec::new();

        for (id, is_running, title) in self.manager.terminal_statuses() {
            if !is_running && !self.exited_terminal_ids.contains(&id) {
                self.exited_terminal_ids.insert(id);
                exited.push(t!("terminal.process_exited", title = title).to_string());
            }
        }
        exited
    }

    /// Initialize the first terminal if needed.
    /// First tries to load workspace layout from .ferrite/terminal-layout.json,
    /// otherwise creates a new default terminal.
    fn initialize(&mut self) {
        if !self.initialized {
            // Try to load workspace layout first
            if self.try_load_workspace_layout() {
                log::info!("Terminal panel initialized from workspace layout");
                return;
            }

            // No workspace layout found, create default terminal
            use crate::terminal::ShellType;
            match self
                .manager
                .create_terminal(ShellType::Default, self.working_dir.clone())
            {
                Ok(_) => {
                    self.initialized = true;
                    self.terminal_has_focus = true;
                    log::info!("Terminal initialized in {:?}", self.working_dir);
                }
                Err(e) => {
                    self.report_error("Failed to start terminal", &e);
                }
            }
        }
    }

    /// Set the panel height.
    pub fn set_height(&mut self, height: f32) {
        self.height = height.clamp(100.0, 3000.0);
    }

    /// Get the panel height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Get the directory where terminal layouts are saved.
    fn get_layouts_dir() -> Option<std::path::PathBuf> {
        // Try to get the config directory and add a "layouts" subdirectory
        if let Ok(config_dir) = crate::config::get_config_dir() {
            Some(config_dir.join("terminal_layouts"))
        } else {
            None
        }
    }

    /// Refresh the list of saved layout paths.
    fn refresh_saved_layouts(&mut self) {
        self.saved_layout_paths.clear();

        if let Some(layouts_dir) = Self::get_layouts_dir() {
            if layouts_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&layouts_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "json") {
                            self.saved_layout_paths.push(path);
                        }
                    }
                    // Sort alphabetically for consistent cycling
                    self.saved_layout_paths.sort();
                }
            }
        }

        log::debug!(
            "Found {} saved terminal layouts",
            self.saved_layout_paths.len()
        );
    }

    /// Cycle to the next saved layout. Returns true if a layout was loaded.
    pub fn cycle_to_next_layout(&mut self) -> bool {
        // Refresh the list of saved layouts
        self.refresh_saved_layouts();

        if self.saved_layout_paths.is_empty() {
            log::info!("No saved terminal layouts found to cycle through");
            return false;
        }

        // Wrap around the index
        self.saved_layout_cycle_index =
            self.saved_layout_cycle_index % self.saved_layout_paths.len();

        let layout_path = self.saved_layout_paths[self.saved_layout_cycle_index].clone();

        // Try to load the layout
        match std::fs::read_to_string(&layout_path) {
            Ok(json) => {
                match serde_json::from_str::<crate::terminal::SavedLayout>(&json) {
                    Ok(saved) => {
                        let layout_name = saved.name.clone();
                        if let Err(e) = self.manager.load_layout(saved) {
                            log::error!("Failed to load layout '{}': {}", layout_path.display(), e);
                            // Move to next layout for next attempt
                            self.saved_layout_cycle_index =
                                (self.saved_layout_cycle_index + 1) % self.saved_layout_paths.len();
                            return false;
                        }
                        log::info!(
                            "Loaded terminal layout '{}' from {}",
                            layout_name,
                            layout_path.display()
                        );
                        self.terminal_has_focus = true;

                        // Advance index for next cycle
                        self.saved_layout_cycle_index =
                            (self.saved_layout_cycle_index + 1) % self.saved_layout_paths.len();
                        return true;
                    }
                    Err(e) => {
                        log::error!("Failed to parse layout '{}': {}", layout_path.display(), e);
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to read layout file '{}': {}",
                    layout_path.display(),
                    e
                );
            }
        }

        // Move to next layout for next attempt
        self.saved_layout_cycle_index =
            (self.saved_layout_cycle_index + 1) % self.saved_layout_paths.len();
        false
    }

    /// Get the workspace layout file path (.ferrite/terminal-layout.json)
    pub fn get_workspace_layout_path(&self) -> Option<std::path::PathBuf> {
        self.working_dir
            .as_ref()
            .map(|dir| dir.join(".ferrite").join("terminal-layout.json"))
    }

    /// Try to load workspace layout from .ferrite/terminal-layout.json
    /// Returns true if layout was loaded successfully
    pub fn try_load_workspace_layout(&mut self) -> bool {
        if self.workspace_layout_loaded {
            return false;
        }

        let layout_path = match self.get_workspace_layout_path() {
            Some(p) => p,
            None => return false,
        };

        if !layout_path.exists() {
            log::debug!("No workspace layout found at {:?}", layout_path);
            self.workspace_layout_loaded = true; // Mark as "checked" even if not found
            return false;
        }

        match std::fs::read_to_string(&layout_path) {
            Ok(json) => {
                match serde_json::from_str::<crate::terminal::SavedWorkspace>(&json) {
                    Ok(workspace) => {
                        match self.manager.load_workspace(workspace) {
                            Ok(floating_windows_data) => {
                                // Recreate floating windows from saved data
                                self.floating_windows.clear();
                                for (layout, title, pos, size) in floating_windows_data {
                                    let leaf = layout.first_leaf();
                                    let id = egui::ViewportId::from_hash_of(
                                        egui::Id::new("floating_term").with(leaf),
                                    );
                                    let pos = pos.map(|(x, y)| egui::pos2(x, y));
                                    let size = egui::vec2(size.0, size.1);
                                    self.floating_windows.push(FloatingWindow {
                                        id,
                                        layout,
                                        title,
                                        pos,
                                        size,
                                        first_frame: true,
                                    });
                                }
                                self.initialized = true;
                                self.workspace_layout_loaded = true;
                                self.terminal_has_focus = true;
                                log::info!("Loaded workspace layout from {:?}", layout_path);
                                return true;
                            }
                            Err(e) => {
                                log::error!("Failed to load workspace layout: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse workspace layout JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read workspace layout file: {}", e);
            }
        }

        self.workspace_layout_loaded = true; // Mark as checked to prevent repeated attempts
        false
    }

    /// Save current workspace layout to .ferrite/terminal-layout.json
    pub fn save_workspace_layout(&mut self) -> bool {
        let layout_path = match self.get_workspace_layout_path() {
            Some(p) => p,
            None => return false,
        };

        // Don't save if no terminals exist
        if !self.manager.has_terminals() && self.floating_windows.is_empty() {
            return false;
        }

        // Create .ferrite directory if it doesn't exist
        if let Some(parent) = layout_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::error!("Failed to create .ferrite directory: {}", e);
                return false;
            }
        }

        // Build workspace structure
        let tabs: Vec<_> = self
            .manager
            .tabs()
            .iter()
            .enumerate()
            .map(|(i, layout)| self.manager.save_layout(layout, format!("Tab {}", i + 1)))
            .collect();

        let floating_windows: Vec<_> = self
            .floating_windows
            .iter()
            .map(|fw| crate::terminal::SavedFloatingWindow {
                layout: self.manager.save_layout(&fw.layout, fw.title.clone()),
                title: fw.title.clone(),
                position: fw.pos.map(|p| (p.x, p.y)),
                size: (fw.size.x, fw.size.y),
            })
            .collect();

        let workspace = crate::terminal::SavedWorkspace {
            name: t!("terminal.workspace.default_name").to_string(),
            tabs,
            floating_windows,
            active_tab_index: self.manager.active_index(),
        };

        match serde_json::to_string_pretty(&workspace) {
            Ok(json) => match std::fs::write(&layout_path, json) {
                Ok(_) => {
                    self.last_auto_save = Some(std::time::Instant::now());
                    log::info!("Saved workspace layout to {:?}", layout_path);
                    return true;
                }
                Err(e) => {
                    log::error!("Failed to write workspace layout: {}", e);
                }
            },
            Err(e) => {
                log::error!("Failed to serialize workspace layout: {}", e);
            }
        }

        false
    }

    /// Auto-save workspace layout if enough time has passed (debounced)
    pub fn maybe_auto_save_layout(&mut self, settings: &crate::config::Settings) {
        if !settings.terminal_auto_save_layout {
            return;
        }

        // Debounce: only save if 5 seconds have passed since last save
        let should_save = match self.last_auto_save {
            Some(last) => last.elapsed().as_secs() >= 5,
            None => true,
        };

        if should_save && self.initialized {
            self.save_workspace_layout();
        }
    }

    /// Reset workspace layout loaded flag (call when working_dir changes)
    pub fn reset_workspace_layout_state(&mut self) {
        self.workspace_layout_loaded = false;
    }
}

/// Terminal panel UI component.
pub struct TerminalPanel {
    /// Unique ID for the panel
    #[allow(dead_code)]
    id: Id,
}

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalPanel {
    /// Create a new terminal panel.
    pub fn new() -> Self {
        Self {
            id: Id::new("terminal_panel"),
        }
    }

    fn render_recursive(
        &self,
        ui: &mut Ui,
        layout: &mut TerminalLayout,
        manager: &mut TerminalManager,
        terminal_states: &mut std::collections::HashMap<usize, PerTerminalState>,
        terminal_has_focus: &mut bool,
        renaming_index: &Option<usize>,
        settings: &crate::config::Settings,
        theme: &crate::terminal::TerminalTheme,
    ) -> bool {
        match layout {
            crate::terminal::TerminalLayout::Terminal(id) => {
                let is_renaming = renaming_index == &Some(*id);
                let is_focused = manager.focused_terminal_id() == Some(*id);

                let (screen_arc, is_waiting) =
                    if let Some(terminal) = manager.terminal_mut_by_id(*id) {
                        (terminal.screen().clone(), terminal.is_waiting_for_input())
                    } else {
                        ui.label(t!("terminal.not_found").to_string());
                        return false;
                    };

                let mut term_state = terminal_states.entry(*id).or_default().clone();

                {
                    let screen = screen_arc.lock().unwrap();
                    let scrollback_len = screen.scrollback_len();
                    if scrollback_len > term_state.last_scrollback_len {
                        term_state.scroll_offset = 0;
                        term_state.last_scrollback_len = scrollback_len;
                    }
                }

                // Detect transition from running to waiting for input (focus-on-detect)
                let transition_to_waiting = !term_state.was_waiting && is_waiting;
                if settings.terminal_focus_on_detect && transition_to_waiting {
                    // Auto-focus this terminal when it starts waiting for input
                    manager.set_focused_terminal(*id);
                    *terminal_has_focus = true;
                    log::debug!("Auto-focusing terminal {} on input detection", id);
                }

                let widget = TerminalWidget::new(&screen_arc)
                    .font_size(settings.terminal_font_size)
                    .focused(*terminal_has_focus && is_focused && !is_renaming)
                    .scroll_offset(term_state.scroll_offset)
                    .copy_on_select(settings.terminal_copy_on_select)
                    .is_waiting(is_waiting)
                    .breathing_color(settings.terminal_breathing_color)
                    .theme(theme.clone())
                    .opacity(settings.terminal_opacity);

                let widget_output = widget.show(ui);

                if let Some(new_offset) = widget_output.new_scroll_offset {
                    term_state.scroll_offset = new_offset;
                }

                // Update was_waiting state for next frame
                term_state.was_waiting = is_waiting;
                terminal_states.insert(*id, term_state);

                if widget_output.user_interacted {
                    manager.set_focused_terminal(*id);
                    *terminal_has_focus = true;
                }

                if !widget_output.input.is_empty() {
                    if let Some(terminal) = manager.terminal_mut_by_id(*id) {
                        terminal.write_input(&widget_output.input);
                    }
                }

                if let Some((cols, rows)) = widget_output.new_size {
                    if let Some(terminal) = manager.terminal_mut_by_id(*id) {
                        terminal.resize(cols, rows);
                    }
                }

                widget_output.user_interacted
            }
            crate::terminal::TerminalLayout::Horizontal { splits, weights } => {
                let mut interacted = false;
                let available_width = ui.available_width();
                let total_weight: f32 = weights.iter().sum();
                let num_splits = splits.len();

                // Calculate all rects upfront using indices
                let mut cumulative_x = 0.0;
                let mut rects = Vec::with_capacity(num_splits);
                for i in 0..num_splits {
                    let width = available_width * weights[i] / total_weight;
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.min_rect().left() + cumulative_x, ui.min_rect().top()),
                        egui::vec2(width, ui.available_height()),
                    );
                    rects.push(rect);
                    cumulative_x += width;
                }

                // Handle dividers (separate loop to avoid borrow issues)
                for i in 1..num_splits {
                    let divider_rect = egui::Rect::from_min_max(
                        egui::pos2(rects[i].left() - 2.0, rects[i].top()),
                        egui::pos2(rects[i].left() + 2.0, rects[i].bottom()),
                    );
                    let divider_response = ui.interact(
                        divider_rect,
                        egui::Id::new("h_divider").with(i),
                        egui::Sense::drag(),
                    );
                    if divider_response.dragged() {
                        let delta = divider_response.drag_delta().x;
                        let delta_ratio = delta / available_width;
                        weights[i - 1] = (weights[i - 1] + delta_ratio * total_weight).max(0.1);
                        weights[i] = (weights[i] - delta_ratio * total_weight).max(0.1);
                    }
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }

                // Render children
                for i in 0..num_splits {
                    let mut child_ui = ui.child_ui(
                        rects[i],
                        egui::Layout::left_to_right(egui::Align::Min),
                        None,
                    );
                    if self.render_recursive(
                        &mut child_ui,
                        &mut splits[i],
                        manager,
                        terminal_states,
                        terminal_has_focus,
                        renaming_index,
                        settings,
                        theme,
                    ) {
                        interacted = true;
                    }
                }
                interacted
            }
            crate::terminal::TerminalLayout::Vertical { splits, weights } => {
                let mut interacted = false;
                let available_height = ui.available_height();
                let total_weight: f32 = weights.iter().sum();
                let num_splits = splits.len();

                // Calculate all rects upfront using indices
                let mut cumulative_y = 0.0;
                let mut rects = Vec::with_capacity(num_splits);
                for i in 0..num_splits {
                    let height = available_height * weights[i] / total_weight;
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.min_rect().left(), ui.min_rect().top() + cumulative_y),
                        egui::vec2(ui.available_width(), height),
                    );
                    rects.push(rect);
                    cumulative_y += height;
                }

                // Handle dividers (separate loop to avoid borrow issues)
                for i in 1..num_splits {
                    let divider_rect = egui::Rect::from_min_max(
                        egui::pos2(rects[i].left(), rects[i].top() - 2.0),
                        egui::pos2(rects[i].right(), rects[i].top() + 2.0),
                    );
                    let divider_response = ui.interact(
                        divider_rect,
                        egui::Id::new("v_divider").with(i),
                        egui::Sense::drag(),
                    );
                    if divider_response.dragged() {
                        let delta = divider_response.drag_delta().y;
                        let delta_ratio = delta / available_height;
                        weights[i - 1] = (weights[i - 1] + delta_ratio * total_weight).max(0.1);
                        weights[i] = (weights[i] - delta_ratio * total_weight).max(0.1);
                    }
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }
                }

                // Render children
                for i in 0..num_splits {
                    let mut child_ui =
                        ui.child_ui(rects[i], egui::Layout::top_down(egui::Align::Min), None);
                    if self.render_recursive(
                        &mut child_ui,
                        &mut splits[i],
                        manager,
                        terminal_states,
                        terminal_has_focus,
                        renaming_index,
                        settings,
                        theme,
                    ) {
                        interacted = true;
                    }
                }
                interacted
            }
        }
    }

    fn handle_shortcuts(
        &self,
        ui: &mut Ui,
        manager: &mut TerminalManager,
        terminal_has_focus: bool,
        maximized_terminal_id: &mut Option<usize>,
        pop_out_request: &mut Option<(usize, Option<egui::Pos2>)>,
        _settings: &crate::config::Settings,
        floating_windows: &[FloatingWindow],
    ) {
        if !terminal_has_focus {
            return;
        }

        let docked_count = manager.terminal_count();
        let total_count = docked_count + floating_windows.len();

        // Ctrl+Tab / Ctrl+Shift+Tab to cycle through ALL terminals (docked + floating)
        let ctrl_tab_pressed = ui.input(|i| i.key_pressed(egui::Key::Tab) && i.modifiers.command);
        if ctrl_tab_pressed && total_count > 1 {
            let current_id = ui.ctx().viewport_id();
            let mut current_global_idx = 0;

            if current_id == egui::ViewportId::ROOT {
                current_global_idx = manager.active_index();
            } else {
                for (i, fw) in floating_windows.iter().enumerate() {
                    if fw.id == current_id {
                        current_global_idx = docked_count + i;
                        break;
                    }
                }
            }

            let next_global = if ui.input(|i| i.modifiers.shift) {
                if current_global_idx == 0 {
                    total_count - 1
                } else {
                    current_global_idx - 1
                }
            } else {
                (current_global_idx + 1) % total_count
            };

            if next_global < docked_count {
                manager.set_active(next_global);
                ui.ctx()
                    .send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
            } else {
                let fw_idx = next_global - docked_count;
                if let Some(fw) = floating_windows.get(fw_idx) {
                    ui.ctx()
                        .send_viewport_cmd_to(fw.id, egui::ViewportCommand::Focus);
                }
            }

            ui.ctx().input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Tab);
            });
        }

        // Ctrl+1-9 to jump to specific terminal (including floating)
        for i in 1..=9 {
            let key = match i {
                1 => egui::Key::Num1,
                2 => egui::Key::Num2,
                3 => egui::Key::Num3,
                4 => egui::Key::Num4,
                5 => egui::Key::Num5,
                6 => egui::Key::Num6,
                7 => egui::Key::Num7,
                8 => egui::Key::Num8,
                9 => egui::Key::Num9,
                _ => continue,
            };
            if ui.input(|input| input.key_pressed(key) && input.modifiers.command) {
                let idx = i - 1;
                if idx < docked_count {
                    manager.set_active(idx);
                    ui.ctx()
                        .send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                } else if idx < total_count {
                    let fw_idx = idx - docked_count;
                    if let Some(fw) = floating_windows.get(fw_idx) {
                        ui.ctx()
                            .send_viewport_cmd_to(fw.id, egui::ViewportCommand::Focus);
                    }
                }
                ui.ctx().input_mut(|input| {
                    input.consume_key(egui::Modifiers::COMMAND, key);
                    input.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, key);
                });
            }
        }

        // Ctrl+L to clear terminal
        if ui.input(|i| i.key_pressed(egui::Key::L) && i.modifiers.command) {
            if let Some(terminal) = manager.active_terminal_mut() {
                terminal.write_input(&[12]);
            }
            ui.ctx().input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::L);
            });
        }

        // Ctrl+F4 to close active terminal/pane
        if ui.input(|i| i.key_pressed(egui::Key::F4) && i.modifiers.ctrl) {
            manager.close_focused_pane();
        }

        // Ctrl+W to close focused pane
        if ui.input(|i| i.key_pressed(egui::Key::W) && i.modifiers.command) {
            manager.close_focused_pane();
            ui.ctx().input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W);
            });
        }

        // Ctrl+Shift+M to toggle maximize
        if ui.input(|i| i.key_pressed(egui::Key::M) && i.modifiers.command && i.modifiers.shift) {
            if maximized_terminal_id.is_some() {
                *maximized_terminal_id = None;
            } else if let Some(active_id) = manager.focused_terminal_id() {
                *maximized_terminal_id = Some(active_id);
            }
            ui.ctx().input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::M,
                );
            });
        }

        // Ctrl+Shift+P to Pop Out active terminal
        if ui.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.command && i.modifiers.shift) {
            if maximized_terminal_id.is_none() {
                *pop_out_request = Some((manager.active_index(), None));
            }
            ui.ctx().input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::P,
                );
            });
        }

        // Ctrl+Alt+Home to focus Main Window
        if ui.input(|i| i.key_pressed(egui::Key::Home) && i.modifiers.command && i.modifiers.alt) {
            ui.ctx()
                .send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
            ui.ctx().input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::ALT,
                    egui::Key::Home,
                );
            });
        }

        // Focus navigation with Ctrl+Arrow
        let move_dir = ui.input(|i| {
            if i.modifiers.command && !i.modifiers.shift {
                if i.key_pressed(egui::Key::ArrowLeft) {
                    Some(MoveDirection::Left)
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    Some(MoveDirection::Right)
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    Some(MoveDirection::Up)
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    Some(MoveDirection::Down)
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(dir) = move_dir {
            manager.move_focus(dir);
            ui.ctx().input_mut(|i| {
                let key = match dir {
                    MoveDirection::Left => egui::Key::ArrowLeft,
                    MoveDirection::Right => egui::Key::ArrowRight,
                    MoveDirection::Up => egui::Key::ArrowUp,
                    MoveDirection::Down => egui::Key::ArrowDown,
                };
                i.consume_key(egui::Modifiers::COMMAND, key);
            });
        }

        // Ctrl+Shift+F1-F4 to move to monitor
        for i in 1..=4 {
            let key = match i {
                1 => egui::Key::F1,
                2 => egui::Key::F2,
                3 => egui::Key::F3,
                4 => egui::Key::F4,
                _ => continue,
            };
            if ui.input(|input| {
                input.key_pressed(key) && input.modifiers.command && input.modifiers.shift
            }) {
                let monitors = crate::terminal::detect_monitors();
                if let Some(m) = monitors.get(i - 1) {
                    let pos = egui::pos2(m.x + 100.0, m.y + 100.0);
                    *pop_out_request = Some((manager.active_index(), Some(pos)));
                }
                ui.ctx().input_mut(|i| {
                    i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, key);
                });
            }
        }
    }

    /// Render visual drop zones when a tab is being dragged.
    /// Returns (hovered_zone, dropped_tab_idx) - dropped_tab_idx is Some if a drop just occurred.
    fn render_drop_zones(
        &self,
        ui: &mut Ui,
        content_rect: egui::Rect,
    ) -> (Option<DropZone>, Option<usize>) {
        // Check if a tab is being dragged - only show drop zones when there's a terminal tab payload
        let payload = egui::DragAndDrop::payload::<usize>(ui.ctx());
        let has_tab_payload = payload.is_some();

        if !has_tab_payload {
            return (None, None);
        }

        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let mut hovered_zone: Option<DropZone> = None;

        // Colors for drop zones
        let split_color = Color32::from_rgba_unmultiplied(70, 130, 220, 77); // Blue ~30% opacity
        let split_hover_color = Color32::from_rgba_unmultiplied(70, 130, 220, 128); // Blue ~50% opacity
        let merge_color = Color32::from_rgba_unmultiplied(70, 180, 100, 77); // Green ~30% opacity
        let merge_hover_color = Color32::from_rgba_unmultiplied(70, 180, 100, 128); // Green ~50% opacity

        // Zone dimensions - edge zones take up 20% of width/height
        let edge_ratio = 0.20;
        let edge_width = content_rect.width() * edge_ratio;
        let edge_height = content_rect.height() * edge_ratio;

        // Calculate zone rectangles
        let left_zone = egui::Rect::from_min_max(
            content_rect.left_top(),
            egui::pos2(content_rect.left() + edge_width, content_rect.bottom()),
        );

        let right_zone = egui::Rect::from_min_max(
            egui::pos2(content_rect.right() - edge_width, content_rect.top()),
            content_rect.right_bottom(),
        );

        let top_zone = egui::Rect::from_min_max(
            egui::pos2(content_rect.left() + edge_width, content_rect.top()),
            egui::pos2(
                content_rect.right() - edge_width,
                content_rect.top() + edge_height,
            ),
        );

        let bottom_zone = egui::Rect::from_min_max(
            egui::pos2(
                content_rect.left() + edge_width,
                content_rect.bottom() - edge_height,
            ),
            egui::pos2(content_rect.right() - edge_width, content_rect.bottom()),
        );

        let center_zone = egui::Rect::from_min_max(
            egui::pos2(
                content_rect.left() + edge_width,
                content_rect.top() + edge_height,
            ),
            egui::pos2(
                content_rect.right() - edge_width,
                content_rect.bottom() - edge_height,
            ),
        );

        // Check which zone is hovered
        if let Some(pos) = pointer_pos {
            if left_zone.contains(pos) {
                hovered_zone = Some(DropZone::Left);
            } else if right_zone.contains(pos) {
                hovered_zone = Some(DropZone::Right);
            } else if top_zone.contains(pos) {
                hovered_zone = Some(DropZone::Top);
            } else if bottom_zone.contains(pos) {
                hovered_zone = Some(DropZone::Bottom);
            } else if center_zone.contains(pos) {
                hovered_zone = Some(DropZone::Center);
            }
        }

        let painter = ui.painter();

        // Draw left zone (split horizontal - new terminal left)
        let left_color = if hovered_zone == Some(DropZone::Left) {
            split_hover_color
        } else {
            split_color
        };
        painter.rect_filled(left_zone, 4.0, left_color);
        if hovered_zone == Some(DropZone::Left) {
            painter.rect_stroke(
                left_zone,
                4.0,
                egui::Stroke::new(2.0, Color32::from_rgb(70, 130, 220)),
            );
        }
        // Draw icon/indicator
        let left_center = left_zone.center();
        painter.text(
            left_center,
            egui::Align2::CENTER_CENTER,
            "◀",
            egui::FontId::proportional(20.0),
            Color32::WHITE,
        );

        // Draw right zone (split horizontal - new terminal right)
        let right_color = if hovered_zone == Some(DropZone::Right) {
            split_hover_color
        } else {
            split_color
        };
        painter.rect_filled(right_zone, 4.0, right_color);
        if hovered_zone == Some(DropZone::Right) {
            painter.rect_stroke(
                right_zone,
                4.0,
                egui::Stroke::new(2.0, Color32::from_rgb(70, 130, 220)),
            );
        }
        let right_center = right_zone.center();
        painter.text(
            right_center,
            egui::Align2::CENTER_CENTER,
            "▶",
            egui::FontId::proportional(20.0),
            Color32::WHITE,
        );

        // Draw top zone (split vertical - new terminal top)
        let top_color = if hovered_zone == Some(DropZone::Top) {
            split_hover_color
        } else {
            split_color
        };
        painter.rect_filled(top_zone, 4.0, top_color);
        if hovered_zone == Some(DropZone::Top) {
            painter.rect_stroke(
                top_zone,
                4.0,
                egui::Stroke::new(2.0, Color32::from_rgb(70, 130, 220)),
            );
        }
        let top_center = top_zone.center();
        painter.text(
            top_center,
            egui::Align2::CENTER_CENTER,
            "▲",
            egui::FontId::proportional(20.0),
            Color32::WHITE,
        );

        // Draw bottom zone (split vertical - new terminal bottom)
        let bottom_color = if hovered_zone == Some(DropZone::Bottom) {
            split_hover_color
        } else {
            split_color
        };
        painter.rect_filled(bottom_zone, 4.0, bottom_color);
        if hovered_zone == Some(DropZone::Bottom) {
            painter.rect_stroke(
                bottom_zone,
                4.0,
                egui::Stroke::new(2.0, Color32::from_rgb(70, 130, 220)),
            );
        }
        let bottom_center = bottom_zone.center();
        painter.text(
            bottom_center,
            egui::Align2::CENTER_CENTER,
            "▼",
            egui::FontId::proportional(20.0),
            Color32::WHITE,
        );

        // Draw center zone (merge - add to tab group)
        let center_color = if hovered_zone == Some(DropZone::Center) {
            merge_hover_color
        } else {
            merge_color
        };
        painter.rect_filled(center_zone, 4.0, center_color);
        if hovered_zone == Some(DropZone::Center) {
            painter.rect_stroke(
                center_zone,
                4.0,
                egui::Stroke::new(2.0, Color32::from_rgb(70, 180, 100)),
            );
        }
        let center_center = center_zone.center();
        painter.text(
            center_center,
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(24.0),
            Color32::WHITE,
        );

        // Check if drag was just released (drop occurred)
        let dropped_tab = if hovered_zone.is_some() && ui.input(|i| i.pointer.any_released()) {
            // Get the payload before it's cleared
            payload.map(|p| *p)
        } else {
            None
        };

        (hovered_zone, dropped_tab)
    }

    /// Show the terminal panel.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        state: &mut TerminalPanelState,
        settings: &crate::config::Settings,
        is_dark: bool,
    ) -> TerminalPanelOutput {
        let mut output = TerminalPanelOutput::default();

        // Update settings
        state
            .manager
            .set_default_scrollback(settings.terminal_scrollback_lines);
        state
            .manager
            .set_prompt_patterns(settings.terminal_prompt_patterns.clone());
        state.manager.set_macros(settings.terminal_macros.clone());
        state.sound_notifier.update_settings(
            settings.terminal_sound_enabled,
            settings.terminal_sound_file.clone(),
        );

        // Poll all terminals for new data
        state.manager.poll_all();

        // Check for sound notification on focused terminal
        if let Some(terminal) = state.manager.focused_terminal() {
            state
                .sound_notifier
                .check_and_notify(terminal.is_waiting_for_input());
        }

        // Resolve theme
        let theme_name = &settings.terminal_theme_name;
        let theme = crate::terminal::TerminalTheme::from_name(theme_name).unwrap_or_else(|| {
            if is_dark {
                crate::terminal::TerminalTheme::ferrite_dark()
            } else {
                crate::terminal::TerminalTheme::ferrite_light()
            }
        });

        // Get theme colors
        let bg_color = if is_dark {
            Color32::from_rgb(30, 30, 30)
        } else {
            Color32::from_rgb(245, 245, 245)
        };
        let border_color = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(200, 200, 200)
        };
        let tab_bg = if is_dark {
            Color32::from_rgb(40, 40, 40)
        } else {
            Color32::from_rgb(235, 235, 235)
        };
        let tab_active_bg = if is_dark {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(255, 255, 255)
        };
        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(30, 30, 30)
        };

        // Draw panel background
        let panel_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(panel_rect, 0.0, bg_color);

        // Draw top border
        ui.painter().line_segment(
            [
                egui::pos2(panel_rect.left(), panel_rect.top()),
                egui::pos2(panel_rect.right(), panel_rect.top()),
            ],
            egui::Stroke::new(1.0, border_color),
        );

        ui.vertical(|ui| {
            // Handle Ctrl+Shift+L to cycle through saved layouts
            // This needs full mutable access to state, so handle it before the partial borrow
            if state.terminal_has_focus {
                if ui.input(|i| {
                    i.key_pressed(egui::Key::L) && i.modifiers.command && i.modifiers.shift
                }) {
                    state.cycle_to_next_layout();
                    ui.ctx().input_mut(|i| {
                        i.consume_key(
                            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                            egui::Key::L,
                        );
                    });
                }
            }

            // Handle shortcuts in this viewport
            let TerminalPanelState {
                ref mut manager,
                ref mut terminal_has_focus,
                ref mut pop_out_request,
                ref mut maximized_terminal_id,
                ref floating_windows,
                ..
            } = *state;

            self.handle_shortcuts(
                ui,
                manager,
                *terminal_has_focus,
                maximized_terminal_id,
                pop_out_request,
                settings,
                floating_windows,
            );

            // Tab bar and controls
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                // Terminal tabs
                let titles = state.manager.terminal_titles();

                for (idx, title, git_branch, status, long_running, is_active, is_waiting) in &titles
                {
                    ui.horizontal(|ui| {
                        // Show text input if this tab is being renamed
                        if state.renaming_index == Some(*idx) {
                            let text_edit = egui::TextEdit::singleline(&mut state.rename_buffer)
                                .desired_width(120.0)
                                .font(egui::TextStyle::Body);

                            let text_response = ui.add(text_edit);
                            text_response.request_focus();

                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if text_response.lost_focus() || enter_pressed {
                                if !state.rename_buffer.trim().is_empty() {
                                    if let Some(terminal) = state.manager.terminal_mut(*idx) {
                                        terminal.set_title(state.rename_buffer.clone());
                                    }
                                }
                                state.renaming_index = None;
                                state.rename_buffer.clear();
                                if enter_pressed {
                                    ui.ctx().input_mut(|i| {
                                        i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                                    });
                                }
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                state.renaming_index = None;
                                state.rename_buffer.clear();
                                ui.ctx().input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                                });
                            }
                        } else {
                            // Tab button with manual drag support (preserves clicks)
                            let inner_response = ui.horizontal(|ui| {
                                let status_icon = match status {
                                    crate::terminal::TerminalStatus::Idle => "",
                                    crate::terminal::TerminalStatus::Running => "▶ ",
                                    crate::terminal::TerminalStatus::Building => "⚙ ",
                                    crate::terminal::TerminalStatus::Testing => "🧪 ",
                                    crate::terminal::TerminalStatus::Error => "⚠ ",
                                };
                                let badge = if *long_running { " ⏳" } else { "" };

                                let btn_text = if let Some(branch) = git_branch {
                                    format!(
                                        "{}{}{}  {}{}",
                                        status_icon,
                                        title,
                                        if status_icon.is_empty() { "" } else { " " },
                                        branch,
                                        badge
                                    )
                                } else {
                                    format!("{}{}{}", status_icon, title, badge)
                                };

                                // Number badge
                                let badge_text = format!("{}", idx + 1);
                                let badge_bg = if *is_active {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().widgets.noninteractive.bg_fill
                                };

                                let badge_response = ui.add(egui::Label::new(
                                    egui::RichText::new(badge_text)
                                        .size(10.0)
                                        .color(Color32::WHITE)
                                        .background_color(badge_bg),
                                ));

                                // Draw attention indicator dot when terminal is waiting for input and not active
                                if *is_waiting && !*is_active {
                                    let dot_radius = 4.0;
                                    let dot_center = egui::pos2(
                                        badge_response.rect.right() + dot_radius + 2.0,
                                        badge_response.rect.center().y,
                                    );
                                    ui.painter().circle_filled(
                                        dot_center,
                                        dot_radius,
                                        settings.terminal_breathing_color,
                                    );
                                    ui.add_space(dot_radius * 2.0 + 2.0);
                                } else {
                                    ui.add_space(4.0);
                                }

                                // Determine title color based on terminal state
                                let title_color = if *is_waiting {
                                    // Waiting for input - use breathing color (pulsing blue/green)
                                    let time = ui.ctx().input(|i| i.time);
                                    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
                                    let base = settings.terminal_breathing_color;
                                    Color32::from_rgba_unmultiplied(
                                        base.r(),
                                        base.g(),
                                        base.b(),
                                        (180.0 + pulse * 75.0) as u8,
                                    )
                                } else if *status != crate::terminal::TerminalStatus::Idle {
                                    // Working/Building/Testing - animated orange/yellow
                                    let time = ui.ctx().input(|i| i.time);
                                    let pulse = ((time * 4.0).sin() * 0.5 + 0.5) as f32;
                                    Color32::from_rgb(
                                        255,
                                        (180.0 + pulse * 75.0) as u8,
                                        (50.0 + pulse * 50.0) as u8,
                                    )
                                } else {
                                    text_color
                                };

                                // Request repaint for animation
                                if *is_waiting || *status != crate::terminal::TerminalStatus::Idle {
                                    ui.ctx().request_repaint();
                                }

                                let btn = egui::Button::new(
                                    egui::RichText::new(btn_text).size(12.0).color(title_color),
                                )
                                .sense(egui::Sense::click_and_drag())
                                .fill(if *is_active { tab_active_bg } else { tab_bg })
                                .stroke(egui::Stroke::new(1.0, border_color))
                                .rounding(egui::Rounding::same(4.0));

                                ui.add(btn)
                            });
                            let tab_response = inner_response.inner;

                            // Set drag payload only when actually dragging (not on click)
                            if tab_response.dragged() {
                                egui::DragAndDrop::set_payload(ui.ctx(), *idx);
                            }

                            // Handle drag to float (if dropped outside panel)
                            if tab_response.drag_stopped() {
                                if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
                                    if !panel_rect.contains(pointer_pos) {
                                        state.pop_out_request = Some((*idx, Some(pointer_pos)));
                                    }
                                }
                            }

                            // Check if another tab is being dragged over this one
                            if let Some(source_idx) = egui::DragAndDrop::payload::<usize>(ui.ctx())
                            {
                                if *source_idx != *idx && tab_response.hovered() {
                                    ui.painter().rect_stroke(
                                        tab_response.rect,
                                        4.0,
                                        egui::Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                                    );
                                    if ui.input(|i| i.pointer.any_released()) {
                                        state.swap_tab_request = Some((*source_idx, *idx));
                                    }
                                }
                            }

                            if tab_response.clicked() {
                                state.manager.set_active(*idx);
                                state.maximized_terminal_id = None;
                            }

                            if tab_response.double_clicked() {
                                state.renaming_index = Some(*idx);
                                state.rename_buffer = title.to_string();
                            }

                            if tab_response.middle_clicked() {
                                state.close_tab_request = Some(*idx);
                            }

                            tab_response.context_menu(|ui: &mut Ui| {
                                if ui
                                    .button(t!("terminal.new_terminal_here").to_string())
                                    .clicked()
                                {
                                    use crate::terminal::ShellType;
                                    match state.manager.create_terminal(
                                        ShellType::Default,
                                        state.working_dir.clone(),
                                    ) {
                                        Ok(id) => {
                                            if !settings.terminal_startup_command.is_empty() {
                                                if let Some(term) = state.manager.terminal_mut(id) {
                                                    term.write_str(
                                                        &settings.terminal_startup_command,
                                                    );
                                                    term.write_str("\n");
                                                }
                                            }
                                            state.terminal_has_focus = true;
                                        }
                                        Err(e) => {
                                            state.report_error("Failed to create terminal", &e)
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(t!("terminal.maximize_active_pane").to_string())
                                    .clicked()
                                {
                                    if let Some(active_id) = state.manager.focused_terminal_id() {
                                        state.maximized_terminal_id = Some(active_id);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button(t!("terminal.pop_out").to_string()).clicked() {
                                    state.pop_out_request = Some((*idx, None));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(t!("terminal.split_horizontal").to_string())
                                    .clicked()
                                {
                                    use crate::terminal::{Direction, ShellType};
                                    if let Err(e) = state.manager.split_pane(
                                        Direction::Horizontal,
                                        ShellType::Default,
                                        state.working_dir.clone(),
                                    ) {
                                        state.report_error("Failed to split pane", &e);
                                    } else {
                                        state.terminal_has_focus = true;
                                    }
                                    ui.close_menu();
                                }
                                if ui
                                    .button(t!("terminal.split_vertical").to_string())
                                    .clicked()
                                {
                                    use crate::terminal::{Direction, ShellType};
                                    if let Err(e) = state.manager.split_pane(
                                        Direction::Vertical,
                                        ShellType::Default,
                                        state.working_dir.clone(),
                                    ) {
                                        state.report_error("Failed to split pane", &e);
                                    } else {
                                        state.terminal_has_focus = true;
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button(t!("terminal.rename").to_string()).clicked() {
                                    state.renaming_index = Some(*idx);
                                    state.rename_buffer = title.to_string();
                                    ui.close_menu();
                                }
                                if ui.button(t!("terminal.close").to_string()).clicked() {
                                    state.close_tab_request = Some(*idx);
                                    ui.close_menu();
                                }
                                if ui.button(t!("terminal.close_pane").to_string()).clicked() {
                                    state.manager.close_focused_pane();
                                    ui.close_menu();
                                }
                                if ui.button(t!("terminal.close_others").to_string()).clicked() {
                                    for i in (0..state.manager.terminal_count()).rev() {
                                        if i != *idx {
                                            state.manager.close_terminal(i);
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(t!("terminal.scatter_to_windows").to_string())
                                    .clicked()
                                {
                                    let monitors = crate::terminal::detect_monitors();
                                    let active_idx = *idx;
                                    let count = state.manager.terminal_count();
                                    let mut to_pop = Vec::new();

                                    for i in 0..count {
                                        if i != active_idx {
                                            to_pop.push(i);
                                        }
                                    }
                                    to_pop.sort_by(|a, b| b.cmp(a));

                                    for (i, target_idx) in to_pop.iter().enumerate() {
                                        if let Some(layout) = state.manager.remove_tab(*target_idx)
                                        {
                                            let leaf = layout.first_leaf();
                                            let id = egui::ViewportId::from_hash_of(
                                                egui::Id::new("floating_term").with(leaf),
                                            );
                                            let title = state
                                                .manager
                                                .terminal_mut_by_id(leaf)
                                                .map(|t| t.title().to_string())
                                                .unwrap_or_else(|| {
                                                    t!("terminal.fallback_title").to_string()
                                                });

                                            // Choose monitor (round-robin)
                                            let monitor_idx = (i + 1) % monitors.len();
                                            let m = &monitors[monitor_idx];

                                            let sub_idx = (i + 1) / monitors.len();
                                            let cascade_offset = sub_idx as f32 * 40.0;

                                            let pos = Some(egui::pos2(
                                                m.x + cascade_offset
                                                    + (m.width - 800.0).max(0.0) / 2.0,
                                                m.y + cascade_offset
                                                    + (m.height - 600.0).max(0.0) / 2.0,
                                            ));

                                            state.floating_windows.push(FloatingWindow {
                                                id,
                                                layout,
                                                title,
                                                pos,
                                                size: egui::vec2(800.0, 600.0),
                                                first_frame: true,
                                            });
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                ui.menu_button(t!("terminal.menu.watch_mode").to_string(), |ui| {
                                    if let Some(terminal) = state.manager.terminal_mut(*idx) {
                                        if let Some(path) = terminal.watched_path() {
                                            ui.label(
                                                t!(
                                                    "terminal.watching",
                                                    path = path.display().to_string()
                                                )
                                                .to_string(),
                                            );
                                            if ui
                                                .button(t!("terminal.stop_watching").to_string())
                                                .clicked()
                                            {
                                                terminal.set_watch(None, None);
                                                ui.close_menu();
                                            }
                                        } else {
                                            if ui
                                                .button(
                                                    t!("terminal.watch_workspace_root").to_string(),
                                                )
                                                .clicked()
                                            {
                                                if let Some(root) = state.working_dir.clone() {
                                                    let cmd = if !settings
                                                        .terminal_startup_command
                                                        .is_empty()
                                                    {
                                                        settings.terminal_startup_command.clone()
                                                    } else {
                                                        "cargo build".to_string()
                                                    };
                                                    terminal.set_watch(Some(root), Some(cmd));
                                                }
                                                ui.close_menu();
                                            }
                                        }
                                    }
                                });
                                if ui.button(t!("terminal.export_html").to_string()).clicked() {
                                    if let Some(terminal) = state.manager.terminal(*idx) {
                                        let html = terminal.export_html(&theme);
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("HTML", &["html"])
                                            .set_file_name("terminal_output.html")
                                            .save_file()
                                        {
                                            if let Err(e) = std::fs::write(path, html) {
                                                log::error!("Failed to save HTML: {}", e);
                                            }
                                        }
                                    }
                                    ui.close_menu();
                                }
                            });

                            if *is_active || tab_response.hovered() {
                                let close_response = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("×").size(14.0).color(text_color),
                                    )
                                    .frame(false)
                                    .min_size(egui::vec2(16.0, 16.0)),
                                );
                                if close_response.clicked() {
                                    state.close_tab_request = Some(*idx);
                                }
                            }
                        }
                    });

                    ui.add_space(4.0);
                }

                if let Some((from, to)) = state.swap_tab_request.take() {
                    state.manager.swap_tabs(from, to);
                }

                if let Some((idx, pos)) = state.pop_out_request.take() {
                    if let Some(layout) = state.manager.remove_tab(idx) {
                        let leaf = layout.first_leaf();
                        let id = egui::ViewportId::from_hash_of(
                            egui::Id::new("floating_term").with(leaf),
                        );
                        let title = state
                            .manager
                            .terminal_mut_by_id(leaf)
                            .map(|t| t.title().to_string())
                            .unwrap_or_else(|| t!("terminal.fallback_title").to_string());

                        state.floating_windows.push(FloatingWindow {
                            id,
                            layout,
                            title,
                            pos,
                            size: egui::vec2(800.0, 600.0),
                            first_frame: true,
                        });
                    }
                }

                // Handle drag-to-split/merge drop actions
                if let Some((tab_idx, zone)) = state.pending_drop_action.take() {
                    use crate::terminal::Direction;
                    match zone {
                        DropZone::Left => {
                            state
                                .manager
                                .merge_tab_as_split(tab_idx, Direction::Horizontal, true);
                        }
                        DropZone::Right => {
                            state
                                .manager
                                .merge_tab_as_split(tab_idx, Direction::Horizontal, false);
                        }
                        DropZone::Top => {
                            state
                                .manager
                                .merge_tab_as_split(tab_idx, Direction::Vertical, true);
                        }
                        DropZone::Bottom => {
                            state
                                .manager
                                .merge_tab_as_split(tab_idx, Direction::Vertical, false);
                        }
                        DropZone::Center => {
                            // Merge: just swap tabs to bring it next to active (simple for now)
                            // Could be enhanced to add to same tab group in future
                            let active = state.manager.active_index();
                            if tab_idx != active {
                                state.manager.swap_tabs(tab_idx, active);
                            }
                        }
                    }
                }

                // New terminal button
                let new_btn = ui.add(
                    egui::Button::new(egui::RichText::new("+").size(14.0).color(text_color))
                        .fill(tab_bg)
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .rounding(egui::Rounding::same(4.0))
                        .min_size(egui::vec2(24.0, 24.0)),
                );

                // Handle new terminal button
                use crate::terminal::ShellType;

                // Left-click: show menu to choose shell type
                if new_btn.clicked() {
                    // Create default PowerShell terminal on left click
                    match state
                        .manager
                        .create_terminal(ShellType::PowerShell, state.working_dir.clone())
                    {
                        Ok(id) => {
                            if !settings.terminal_startup_command.is_empty() {
                                if let Some(term) = state.manager.terminal_mut(id) {
                                    term.write_str(&settings.terminal_startup_command);
                                    term.write_str("\n");
                                }
                            }
                            state.terminal_has_focus = true; // Auto-focus new terminal
                        }
                        Err(e) => state.report_error("Failed to create terminal", &e),
                    }
                }

                // Right-click: show menu with all options
                new_btn.context_menu(|ui| {
                    if ui
                        .button(t!("terminal.shell.powershell").to_string())
                        .clicked()
                    {
                        match state
                            .manager
                            .create_terminal(ShellType::PowerShell, state.working_dir.clone())
                        {
                            Ok(id) => {
                                if !settings.terminal_startup_command.is_empty() {
                                    if let Some(term) = state.manager.terminal_mut(id) {
                                        term.write_str(&settings.terminal_startup_command);
                                        term.write_str("\n");
                                    }
                                }
                                state.terminal_has_focus = true;
                            }
                            Err(e) => {
                                state.report_error("Failed to create PowerShell terminal", &e)
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button(t!("terminal.shell.cmd").to_string()).clicked() {
                        match state
                            .manager
                            .create_terminal(ShellType::Cmd, state.working_dir.clone())
                        {
                            Ok(id) => {
                                if !settings.terminal_startup_command.is_empty() {
                                    if let Some(term) = state.manager.terminal_mut(id) {
                                        term.write_str(&settings.terminal_startup_command);
                                        term.write_str("\n");
                                    }
                                }
                                state.terminal_has_focus = true;
                            }
                            Err(e) => state.report_error("Failed to create CMD terminal", &e),
                        }
                        ui.close_menu();
                    }
                    if ui.button(t!("terminal.shell.wsl").to_string()).clicked() {
                        match state
                            .manager
                            .create_terminal(ShellType::Wsl, state.working_dir.clone())
                        {
                            Ok(id) => {
                                if !settings.terminal_startup_command.is_empty() {
                                    if let Some(term) = state.manager.terminal_mut(id) {
                                        term.write_str(&settings.terminal_startup_command);
                                        term.write_str("\n");
                                    }
                                }
                                state.terminal_has_focus = true;
                            }
                            Err(e) => state.report_error("Failed to create WSL terminal", &e),
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button(t!("terminal.menu.layouts").to_string(), |ui| {
                        if ui
                            .button(t!("terminal.layout.columns_2").to_string())
                            .clicked()
                        {
                            if let Err(e) = state.manager.create_grid_layout(
                                1,
                                2,
                                ShellType::Default,
                                state.working_dir.clone(),
                            ) {
                                state.report_error("Failed to create terminal layout", &e);
                            } else {
                                state.terminal_has_focus = true;
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button(t!("terminal.layout.rows_2").to_string())
                            .clicked()
                        {
                            if let Err(e) = state.manager.create_grid_layout(
                                2,
                                1,
                                ShellType::Default,
                                state.working_dir.clone(),
                            ) {
                                state.report_error("Failed to create terminal layout", &e);
                            } else {
                                state.terminal_has_focus = true;
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button(t!("terminal.layout.grid_2x2").to_string())
                            .clicked()
                        {
                            if let Err(e) = state.manager.create_grid_layout(
                                2,
                                2,
                                ShellType::Default,
                                state.working_dir.clone(),
                            ) {
                                state.report_error("Failed to create terminal layout", &e);
                            } else {
                                state.terminal_has_focus = true;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(t!("terminal.layout.save_layout").to_string())
                            .clicked()
                        {
                            if let Some(saved) = state
                                .manager
                                .save_active_layout(t!("terminal.layout.custom_layout").to_string())
                            {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        &t!("terminal.filter.terminal_layout").to_string(),
                                        &["json"],
                                    )
                                    .set_file_name("layout.json")
                                    .save_file()
                                {
                                    if let Ok(json) = serde_json::to_string_pretty(&saved) {
                                        let _ = std::fs::write(path, json);
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button(t!("terminal.layout.load_layout").to_string())
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    &t!("terminal.filter.terminal_layout").to_string(),
                                    &["json"],
                                )
                                .pick_file()
                            {
                                if let Ok(json) = std::fs::read_to_string(path) {
                                    if let Ok(saved) =
                                        serde_json::from_str::<crate::terminal::SavedLayout>(&json)
                                    {
                                        if let Err(e) = state.manager.load_layout(saved) {
                                            state
                                                .report_error("Failed to load terminal layout", &e);
                                        } else {
                                            state.terminal_has_focus = true;
                                        }
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button(t!("terminal.menu.workspaces").to_string(), |ui| {
                            if ui
                                .button(t!("terminal.workspace.scatter_all").to_string())
                                .clicked()
                            {
                                let active_idx = state.manager.active_index();
                                let count = state.manager.terminal_count();
                                let mut to_pop = Vec::new();

                                for i in 0..count {
                                    if i != active_idx {
                                        to_pop.push(i);
                                    }
                                }
                                to_pop.sort_by(|a, b| b.cmp(a));

                                for (i, idx) in to_pop.iter().enumerate() {
                                    if let Some(layout) = state.manager.remove_tab(*idx) {
                                        let leaf = layout.first_leaf();
                                        let id = egui::ViewportId::from_hash_of(
                                            egui::Id::new("floating_term").with(leaf),
                                        );
                                        let title = state
                                            .manager
                                            .terminal_mut_by_id(leaf)
                                            .map(|t| t.title().to_string())
                                            .unwrap_or_else(|| {
                                                t!("terminal.fallback_title").to_string()
                                            });

                                        let offset = (i as f32 + 1.0) * 40.0;
                                        let pos = Some(egui::pos2(100.0 + offset, 100.0 + offset));

                                        state.floating_windows.push(FloatingWindow {
                                            id,
                                            layout,
                                            title,
                                            pos,
                                            size: egui::vec2(800.0, 600.0),
                                            first_frame: true,
                                        });
                                    }
                                }
                                ui.close_menu();
                            }

                            // Save to .ferrite/terminal-layout.json in workspace root
                            let workspace_layout_path = state.get_workspace_layout_path();
                            let can_save_workspace = workspace_layout_path.is_some();

                            if ui
                                .add_enabled(
                                    can_save_workspace,
                                    egui::Button::new(
                                        t!("terminal.layout.save_workspace_layout").to_string(),
                                    ),
                                )
                                .clicked()
                            {
                                if state.save_workspace_layout() {
                                    log::info!(
                                        "Saved workspace layout to .ferrite/terminal-layout.json"
                                    );
                                }
                                ui.close_menu();
                            }
                            if !can_save_workspace {
                                ui.label(t!("terminal.no_workspace_root").to_string());
                            }

                            ui.separator();

                            if ui
                                .button(t!("terminal.workspace.save").to_string())
                                .clicked()
                            {
                                let tabs: Vec<_> = state
                                    .manager
                                    .tabs()
                                    .iter()
                                    .enumerate()
                                    .map(|(i, layout)| {
                                        state.manager.save_layout(
                                            layout,
                                            t!("terminal.tab_name", index = i + 1).to_string(),
                                        )
                                    })
                                    .collect();

                                let floating_windows: Vec<_> = state
                                    .floating_windows
                                    .iter()
                                    .map(|fw| crate::terminal::SavedFloatingWindow {
                                        layout: state
                                            .manager
                                            .save_layout(&fw.layout, fw.title.clone()),
                                        title: fw.title.clone(),
                                        position: fw.pos.map(|p| (p.x, p.y)),
                                        size: (fw.size.x, fw.size.y),
                                    })
                                    .collect();

                                let workspace = crate::terminal::SavedWorkspace {
                                    name: t!("terminal.workspace.default_name").to_string(),
                                    tabs,
                                    floating_windows,
                                    active_tab_index: state.manager.active_index(),
                                };

                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        &t!("terminal.filter.ferrite_workspace").to_string(),
                                        &["json"],
                                    )
                                    .set_file_name("workspace.json")
                                    .save_file()
                                {
                                    if let Ok(json) = serde_json::to_string_pretty(&workspace) {
                                        let _ = std::fs::write(path, json);
                                    }
                                }
                                ui.close_menu();
                            }

                            if ui
                                .button(t!("terminal.workspace.load").to_string())
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        &t!("terminal.filter.ferrite_workspace").to_string(),
                                        &["json"],
                                    )
                                    .pick_file()
                                {
                                    if let Ok(json) = std::fs::read_to_string(path) {
                                        if let Ok(workspace) =
                                            serde_json::from_str::<crate::terminal::SavedWorkspace>(
                                                &json,
                                            )
                                        {
                                            match state.manager.load_workspace(workspace) {
                                                Ok(fws) => {
                                                    // Recreate floating windows
                                                    state.floating_windows.clear();
                                                    for (layout, title, pos, size) in fws {
                                                        let leaf = layout.first_leaf();
                                                        let id = egui::ViewportId::from_hash_of(
                                                            egui::Id::new("floating_term")
                                                                .with(leaf),
                                                        );
                                                        let pos =
                                                            pos.map(|(x, y)| egui::pos2(x, y));
                                                        let size = egui::vec2(size.0, size.1);
                                                        state.floating_windows.push(
                                                            FloatingWindow {
                                                                id,
                                                                layout,
                                                                title,
                                                                pos,
                                                                size,
                                                                first_frame: true,
                                                            },
                                                        );
                                                    }
                                                    state.terminal_has_focus = true;
                                                }
                                                Err(e) => state.report_error(
                                                    "Failed to load workspace layout",
                                                    &e,
                                                ),
                                            }
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        });
                        ui.menu_button(t!("terminal.menu.macros").to_string(), |ui| {
                            if settings.terminal_macros.is_empty() {
                                ui.label(t!("terminal.no_macros").to_string());
                            } else {
                                let mut names: Vec<_> = settings.terminal_macros.keys().collect();
                                names.sort();
                                for name in names {
                                    if ui.button(name).clicked() {
                                        let _ = state.manager.play_macro(name);
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                    });
                });

                // Handle tab close request
                if let Some(idx) = state.close_tab_request.take() {
                    if let Some(terminal) = state.manager.terminal(idx) {
                        if terminal.is_running() {
                            state.pending_close_index = Some(idx);
                        } else {
                            state.manager.close_terminal(idx);
                        }
                    }
                }

                // Spacer
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close panel button
                    let close_btn = ui.add(
                        egui::Button::new(egui::RichText::new("×").size(16.0).color(text_color))
                            .frame(false)
                            .min_size(egui::vec2(24.0, 24.0)),
                    );

                    if close_btn
                        .clone()
                        .on_hover_text(t!("terminal.close_panel_tooltip").to_string())
                        .clicked()
                    {
                        output.closed = true;
                        state.hide();
                    }

                    ui.add_space(8.0);
                });
            });

            ui.add_space(4.0);

            // Determine what layout to render
            // If maximized, we fake a single-terminal layout for rendering
            let mut effective_layout = if let Some(max_id) = state.maximized_terminal_id {
                // Verify terminal exists
                if state.manager.terminal(max_id).is_some() {
                    Some(crate::terminal::TerminalLayout::Terminal(max_id))
                } else {
                    state.maximized_terminal_id = None;
                    state.manager.active_tab_layout().cloned()
                }
            } else {
                state.manager.active_tab_layout().cloned()
            };

            let terminal_was_clicked = if let Some(ref mut layout) = effective_layout {
                // Split borrow of state
                let TerminalPanelState {
                    manager,
                    terminal_states,
                    terminal_has_focus,
                    renaming_index,
                    ..
                } = state;

                // Create a child UI for the content area to ensure max_rect()
                // inside render_recursive doesn't include the tab bar.
                let content_rect = ui.available_rect_before_wrap();
                let mut content_ui =
                    ui.child_ui(content_rect, egui::Layout::top_down(egui::Align::Min), None);

                let interacted = self.render_recursive(
                    &mut content_ui,
                    layout,
                    manager,
                    terminal_states,
                    terminal_has_focus,
                    renaming_index,
                    settings,
                    &theme,
                );

                // Render drop zones overlay when dragging a tab
                let (hovered_zone, dropped_tab) = self.render_drop_zones(ui, content_rect);

                // Handle drop action
                if let (Some(zone), Some(tab_idx)) = (hovered_zone, dropped_tab) {
                    state.pending_drop_action = Some((tab_idx, zone));
                }

                // If maximized, draw Restore button overlay
                if state.maximized_terminal_id.is_some() {
                    let btn_size = egui::vec2(90.0, 24.0);
                    // Position top-right, slightly offset
                    let btn_pos = content_rect.right_top() + egui::vec2(-btn_size.x - 8.0, 8.0);
                    let btn_rect = egui::Rect::from_min_size(btn_pos, btn_size);

                    ui.allocate_ui_at_rect(btn_rect, |ui| {
                        ui.visuals_mut().widgets.inactive.weak_bg_fill =
                            Color32::from_rgba_premultiplied(0, 0, 0, 200);
                        if ui
                            .add(
                                egui::Button::new(t!("terminal.restore").to_string())
                                    .fill(Color32::from_rgba_premultiplied(50, 50, 50, 230)),
                            )
                            .clicked()
                        {
                            state.maximized_terminal_id = None;
                        }
                    });
                }

                interacted
            } else {
                // No terminal - show placeholder
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(&t!("terminal.no_terminal").to_string())
                            .color(text_color)
                            .size(14.0),
                    );
                });
                false
            };

            // Write back layout changes if any (weights, etc.), ONLY if not maximized
            // (changes made during maximized view are temporary/irrelevant to the tree)
            if state.maximized_terminal_id.is_none() {
                if let Some(layout) = effective_layout {
                    if let Some(mgr_layout) = state.manager.active_tab_layout_mut() {
                        *mgr_layout = layout;
                    }
                }
            }

            // Clear terminal focus if user clicked somewhere else (outside terminal)
            // But only if they clicked in the panel area (not tabs or buttons)
            if !terminal_was_clicked && ui.input(|i| i.pointer.any_click()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    // Check if click was in the terminal area
                    let terminal_area = ui.available_rect_before_wrap();
                    if !terminal_area.contains(pos) {
                        log::info!("Click outside terminal area, removing focus");
                        state.terminal_has_focus = false;
                    }
                }
            }
        });

        // Close confirmation dialog
        if let Some(idx) = state.pending_close_index {
            let mut close_dialog = true; // Stay open by default
            let mut confirm_close = false;

            egui::Window::new(t!("terminal.close_terminal_title").to_string())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(t!("terminal.running_process_warning").to_string());
                    ui.label(t!("terminal.terminate_warning").to_string());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(t!("terminal.close_terminal").to_string())
                            .clicked()
                        {
                            confirm_close = true;
                            close_dialog = false;
                        }
                        if ui.button(t!("terminal.cancel").to_string()).clicked() {
                            close_dialog = false;
                        }
                    });
                });

            if confirm_close {
                state.manager.close_terminal(idx);
                state.pending_close_index = None;
            } else if !close_dialog {
                state.pending_close_index = None;
            }
        }

        // Render floating windows (Phase 4)
        let floating_windows = std::mem::take(&mut state.floating_windows);
        let mut re_dock = Vec::new();
        let mut kept_windows = Vec::new();

        let docked_count = state.manager.terminal_count();

        for (i, mut win) in floating_windows.into_iter().enumerate() {
            let id = win.id;
            let global_idx = docked_count + i + 1;
            let display_title = format!("#{} {}", global_idx, win.title);
            let mut open = true;

            let pos_cell = std::rc::Rc::new(std::cell::Cell::new(win.pos));
            let size_cell = std::rc::Rc::new(std::cell::Cell::new(win.size));
            let pos_clone = pos_cell.clone();
            let size_clone = size_cell.clone();

            let mut builder = egui::ViewportBuilder::default().with_title(&display_title);

            // Only force position/size on first frame (creation/load)
            if win.first_frame {
                if let Some(pos) = win.pos {
                    builder = builder.with_position(pos);
                }
                builder = builder.with_inner_size(win.size);
            }

            ui.ctx()
                .show_viewport_immediate(id, builder, |ctx, _class| {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }

                    // Capture current state
                    if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                        pos_clone.set(Some(outer.min));
                    }
                    size_clone.set(ctx.screen_rect().size());

                    egui::CentralPanel::default().show(ctx, |ui| {
                        // Handle shortcuts in this viewport
                        let TerminalPanelState {
                            ref mut manager,
                            ref mut terminal_has_focus,
                            ref mut pop_out_request,
                            ref mut maximized_terminal_id,
                            ..
                        } = *state;

                        self.handle_shortcuts(
                            ui,
                            manager,
                            *terminal_has_focus,
                            maximized_terminal_id,
                            pop_out_request,
                            settings,
                            &kept_windows, // Use already processed windows
                        );

                        // Resolve theme
                        let theme_name = &settings.terminal_theme_name;
                        let theme = crate::terminal::TerminalTheme::from_name(theme_name)
                            .unwrap_or_else(|| {
                                if is_dark {
                                    crate::terminal::TerminalTheme::ferrite_dark()
                                } else {
                                    crate::terminal::TerminalTheme::ferrite_light()
                                }
                            });

                        // Split borrow of state
                        let TerminalPanelState {
                            manager,
                            terminal_states,
                            terminal_has_focus,
                            renaming_index,
                            ..
                        } = state;

                        let rect = ui.available_rect_before_wrap();
                        let mut child_ui =
                            ui.child_ui(rect, egui::Layout::top_down(egui::Align::Min), None);

                        self.render_recursive(
                            &mut child_ui,
                            &mut win.layout,
                            manager,
                            terminal_states,
                            terminal_has_focus,
                            renaming_index,
                            settings,
                            &theme,
                        );
                    });
                });

            win.pos = pos_cell.get();
            win.size = size_cell.get();
            win.first_frame = false;

            if open {
                kept_windows.push(win);
            } else {
                re_dock.push(win.layout);
            }
        }

        state.floating_windows = kept_windows;
        for layout in re_dock {
            state.manager.add_tab(layout);
        }

        output
    }
}
