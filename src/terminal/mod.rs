//! Terminal emulation module for Ferrite
//!
//! This module provides an integrated terminal emulator using:
//! - `portable-pty` for cross-platform pseudo-terminal handling
//! - `vte` for ANSI escape sequence parsing
//!
//! The terminal supports:
//! - Full ANSI color support (16, 256, and true color)
//! - Scrollback buffer
//! - Multiple terminal instances (tabs)
//! - Cross-platform shell spawning (cmd/PowerShell on Windows, bash/zsh on Unix)

mod handler;
mod layout;
mod pty;
mod screen;
mod sound;
mod theme;
mod widget;

pub use handler::TerminalHandler;
pub use layout::{Direction, MoveDirection, TerminalLayout};
pub use pty::{ShellType, TerminalPty};
pub use screen::TerminalScreen;
pub use sound::{play_notification, SoundNotifier};
pub use theme::TerminalTheme;
pub use widget::TerminalWidget;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalStatus {
    Idle,
    Running,
    Building,
    Testing,
    Error,
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Terminal instance that combines PTY, screen buffer, and VTE parser.
pub struct Terminal {
    /// Pseudo-terminal for shell communication
    pty: TerminalPty,
    /// Screen buffer for terminal content
    screen: Arc<Mutex<TerminalScreen>>,
    /// VTE parser for ANSI escape sequences
    parser: vte::Parser,
    /// Terminal title (from OSC sequences)
    title: String,
    /// Unique ID for this terminal
    id: usize,
    /// Shell type used to launch this terminal
    shell_type: ShellType,
    /// Initial working directory
    working_dir: Option<std::path::PathBuf>,
    /// Whether the terminal is active/running
    running: bool,
    /// Whether the terminal appears to be waiting for input (e.g. prompt detected)
    is_waiting_for_input: bool,
    /// Time of last output received
    last_output_time: std::time::Instant,
    /// Current git branch (if any)
    git_branch: Option<String>,
    /// Current activity status
    status: TerminalStatus,
    /// Whether the last command encountered an error
    last_command_failed: bool,
    /// Time when the current command started (if running)
    command_start_time: Option<std::time::Instant>,
    /// Current foreground process name
    foreground_process: Option<String>,
    /// Buffer for tracking typed command line (before Enter)
    command_line_buffer: String,
    /// Compiled regexes for custom prompt detection
    compiled_prompt_regexes: Vec<regex::Regex>,
    /// File path being watched for changes (auto-rerun)
    watched_path: Option<std::path::PathBuf>,
    /// Command to run when watched file changes
    watch_command: Option<String>,
    /// Receiver for status updates
    status_rx: Receiver<(Option<String>, Option<String>)>,
    /// Sender to stop status thread
    _stop_tx: Sender<()>,
}

impl Terminal {
    /// Create a new terminal instance with the given ID, shell type, optional working directory, and scrollback limit.
    pub fn new(
        id: usize,
        cols: u16,
        rows: u16,
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
        max_scrollback: usize,
    ) -> Result<Self, String> {
        let screen = Arc::new(Mutex::new(TerminalScreen::new(cols, rows, max_scrollback)));
        let pty = TerminalPty::new(cols, rows, shell_type, working_dir.clone())?;

        // Create channels for status updates
        let (status_tx, status_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();

        let pid = pty.pid();
        let wd = working_dir.clone();

        thread::spawn(move || {
            loop {
                // Check stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // 1. Check Git
                let mut git_branch = None;
                if let Some(dir) = &wd {
                    #[cfg(target_os = "windows")]
                    use std::os::windows::process::CommandExt;
                    use std::process::Command;

                    let mut cmd = Command::new("git");
                    cmd.args(&["branch", "--show-current"]).current_dir(dir);
                    #[cfg(target_os = "windows")]
                    cmd.creation_flags(0x08000000);

                    if let Ok(output) = cmd.output() {
                        if output.status.success() {
                            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if !branch.is_empty() {
                                git_branch = Some(branch);
                            }
                        }
                    }
                }

                // 2. Check Process (Windows)
                let mut fg_process = None;
                #[cfg(target_os = "windows")]
                if pid > 0 {
                    use std::os::windows::process::CommandExt;
                    use std::process::Command;
                    let mut cmd = Command::new("powershell.exe");
                    cmd.args(&[
                        "-NoProfile",
                        "-Command",
                        &format!("Get-CimInstance Win32_Process -Filter 'ParentProcessId = {}' | Select-Object -First 1 -ExpandProperty Name", pid)
                    ]);
                    cmd.creation_flags(0x08000000);
                    if let Ok(output) = cmd.output() {
                        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !name.is_empty() && name != "powershell.exe" && name != "cmd.exe" {
                            let clean_name = name.strip_suffix(".exe").unwrap_or(&name).to_string();
                            fg_process = Some(clean_name);
                        }
                    }
                }

                // Send update
                if status_tx.send((git_branch, fg_process)).is_err() {
                    break;
                }

                thread::sleep(std::time::Duration::from_secs(2));
            }
        });

        Ok(Self {
            pty,
            screen,
            parser: vte::Parser::new(),
            title: format!("Terminal {}", id),
            id,
            shell_type,
            working_dir,
            running: true,
            is_waiting_for_input: false,
            last_output_time: std::time::Instant::now(),
            git_branch: None,
            status: TerminalStatus::Idle,
            last_command_failed: false,
            command_start_time: None,
            foreground_process: None,
            command_line_buffer: String::new(),
            compiled_prompt_regexes: Vec::new(),
            watched_path: None,
            watch_command: None,
            status_rx,
            _stop_tx: stop_tx,
        })
    }

    /// Set the path and command to watch for auto-rerun.
    pub fn set_watch(&mut self, path: Option<std::path::PathBuf>, command: Option<String>) {
        self.watched_path = path;
        self.watch_command = command;
    }

    /// Get the path being watched.
    pub fn watched_path(&self) -> Option<&std::path::PathBuf> {
        self.watched_path.as_ref()
    }

    /// Handle file change event.
    pub fn on_file_changed(&mut self, path: &std::path::Path) {
        if let (Some(watched), Some(command)) = (&self.watched_path, &self.watch_command) {
            // Check if path is same or inside watched directory
            if path.starts_with(watched) {
                if self.is_waiting_for_input {
                    let cmd = command.clone();
                    self.write_str(&cmd);
                    self.write_str("\n");
                }
            }
        }
    }

    /// Update custom prompt patterns.
    pub fn update_prompt_patterns(&mut self, patterns: &[String]) {
        self.compiled_prompt_regexes.clear();
        for p in patterns {
            if let Ok(re) = regex::Regex::new(p) {
                self.compiled_prompt_regexes.push(re);
            }
        }
    }

    /// Get the terminal ID.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the terminal title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the terminal title.
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Check if the terminal is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get mutable access to the screen buffer.
    pub fn screen(&self) -> &Arc<Mutex<TerminalScreen>> {
        &self.screen
    }

    /// Process input from the user (keyboard).
    pub fn write_input(&mut self, data: &[u8]) {
        // Track command line for process detection
        for &byte in data {
            match byte {
                0x0D | 0x0A => {
                    // Enter pressed - extract command name and set as foreground process
                    if !self.command_line_buffer.is_empty() {
                        let cmd = self.extract_command_name(&self.command_line_buffer.clone());
                        if let Some(name) = cmd {
                            self.foreground_process = Some(name);
                        }
                        self.command_line_buffer.clear();
                    }
                }
                0x7F | 0x08 => {
                    // Backspace - remove last character
                    self.command_line_buffer.pop();
                }
                0x03 => {
                    // Ctrl+C - clear buffer
                    self.command_line_buffer.clear();
                }
                0x15 => {
                    // Ctrl+U - clear line
                    self.command_line_buffer.clear();
                }
                0x17 => {
                    // Ctrl+W - delete word (simplified: just clear)
                    let trimmed = self.command_line_buffer.trim_end();
                    if let Some(pos) = trimmed.rfind(|c: char| c.is_whitespace()) {
                        self.command_line_buffer.truncate(pos + 1);
                    } else {
                        self.command_line_buffer.clear();
                    }
                }
                b if b >= 0x20 && b < 0x7F => {
                    // Printable ASCII
                    self.command_line_buffer.push(byte as char);
                }
                _ => {}
            }
        }

        if let Err(e) = self.pty.write(data) {
            log::warn!("Failed to write to terminal: {}", e);
        }
    }

    /// Write a string to the terminal.
    pub fn write_str(&mut self, s: &str) {
        self.write_input(s.as_bytes());
    }

    /// Extract the command name from a command line.
    /// Handles common patterns like "npm run dev", "cargo build", "claude", etc.
    fn extract_command_name(&self, cmd_line: &str) -> Option<String> {
        let trimmed = cmd_line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Split by whitespace and get first token
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let first = parts[0];

        // Extract base command name (remove path if present)
        let base_name = if first.contains('/') || first.contains('\\') {
            first.rsplit(['/', '\\']).next().unwrap_or(first)
        } else {
            first
        };

        // Remove .exe extension on Windows
        let name = base_name.strip_suffix(".exe").unwrap_or(base_name);

        Some(name.to_string())
    }

    /// Read and process output from the PTY.
    /// Returns true if new data was processed.
    pub fn poll(&mut self) -> bool {
        let mut processed = false;

        // Read available data from PTY
        match self.pty.read() {
            Ok(Some(data)) => {
                self.last_output_time = std::time::Instant::now();

                // Detect activity status from raw output
                let s = String::from_utf8_lossy(&data);
                if s.contains("Compiling") || s.contains("Building") {
                    self.status = TerminalStatus::Building;
                } else if s.contains("Running tests") || s.contains("test result:") {
                    self.status = TerminalStatus::Testing;
                } else if s.contains("error:") || s.contains("Error:") || s.contains("FAILED") {
                    self.status = TerminalStatus::Error;
                    self.last_command_failed = true;
                }

                // Parse through VTE first (updates screen)
                let mut screen = self.screen.lock().unwrap();
                let mut handler = TerminalHandler::new(&mut screen);

                for byte in data {
                    self.parser.advance(&mut handler, byte);
                }

                // Check for title updates
                if let Some(title) = handler.take_title() {
                    self.title = title;
                }

                // Get screen info for prompt detection
                let cursor_line = screen.get_cursor_line_text();
                let has_esc_to_interrupt = screen.screen_contains("esc to interrupt");
                drop(screen);

                // Detect prompt from screen content (after VTE parsing)
                // Also pass the terminal title for Claude-specific detection
                self.detect_prompt_from_screen_info(
                    &cursor_line,
                    has_esc_to_interrupt,
                    &self.title.clone(),
                );

                // Update status based on prompt state
                if !self.is_waiting_for_input
                    && self.status != TerminalStatus::Building
                    && self.status != TerminalStatus::Testing
                    && self.status != TerminalStatus::Error
                {
                    self.status = TerminalStatus::Running;
                } else if self.is_waiting_for_input && self.status != TerminalStatus::Error {
                    self.status = TerminalStatus::Idle;
                }

                processed = true;
            }
            Ok(None) => {
                // No data available
            }
            Err(e) => {
                log::debug!("PTY read error (may be closed): {}", e);
                self.running = false;
            }
        }

        // Check if process is still running
        if !self.pty.is_running() {
            self.running = false;
        }

        // Poll status updates from background thread
        while let Ok((branch, process)) = self.status_rx.try_recv() {
            if branch.is_some() {
                self.git_branch = branch;
            }
            if process.is_some() {
                self.foreground_process = process;
            }
        }

        processed
    }

    /// Detect if the screen shows a prompt waiting for input.
    /// This checks the actual rendered screen content, not raw PTY data.
    ///
    /// For Claude Code specifically:
    /// - "esc to interrupt" visible = Claude is actively working (NOT waiting)
    /// - Title contains loading dots (⠋⠙⠹ etc) = Claude is working
    /// - Otherwise + cursor ends with ">" = waiting for input
    fn detect_prompt_from_screen_info(
        &mut self,
        cursor_line: &str,
        has_esc_to_interrupt: bool,
        title: &str,
    ) {
        let trimmed = cursor_line.trim();

        // Check if title contains spinner/loading characters (Claude working indicator)
        // Claude Code uses braille spinner characters: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
        let title_has_spinner = title.chars().any(|c| {
            matches!(
                c,
                '⠋' | '⠙'
                    | '⠹'
                    | '⠸'
                    | '⠼'
                    | '⠴'
                    | '⠦'
                    | '⠧'
                    | '⠇'
                    | '⠏'
                    | '◐'
                    | '◓'
                    | '◑'
                    | '◒'
            )
        });

        // Claude Code specific detection:
        // - "esc to interrupt" visible = Claude is actively working (NOT waiting)
        // - Title has spinner = Claude is working
        if has_esc_to_interrupt || title_has_spinner {
            // Claude Code is actively working
            if self.is_waiting_for_input {
                self.is_waiting_for_input = false;
                self.command_start_time = Some(std::time::Instant::now());
            }
            return;
        }

        // Check custom regexes
        let mut matched = false;
        for re in &self.compiled_prompt_regexes {
            if re.is_match(trimmed) {
                matched = true;
                break;
            }
        }

        // Check standard prompt characters at end of line
        // Common prompts: >, $, %, #
        if !matched {
            if trimmed.ends_with('>')
                || trimmed.ends_with('$')
                || trimmed.ends_with('%')
                || trimmed.ends_with('#')
            {
                matched = true;
            }
        }

        // Update waiting state
        if matched {
            if !self.is_waiting_for_input {
                self.is_waiting_for_input = true;
                self.command_start_time = None;
                // Clear foreground process when prompt returns
                self.foreground_process = None;
            }
        }
        // Note: We no longer aggressively set is_waiting_for_input = false
        // because that was causing false negatives with Claude Code.
        // The "esc to interrupt" check above handles the "not waiting" case.
    }

    /// Check if the terminal is waiting for input.
    pub fn is_waiting_for_input(&self) -> bool {
        self.is_waiting_for_input
    }

    /// Get current git branch.
    pub fn git_branch(&self) -> Option<&str> {
        self.git_branch.as_deref()
    }

    /// Get current foreground process name.
    pub fn foreground_process(&self) -> Option<&str> {
        self.foreground_process.as_deref()
    }

    /// Check if the current foreground process appears to be Claude Code.
    pub fn is_claude_code(&self) -> bool {
        if let Some(process) = &self.foreground_process {
            let lower = process.to_lowercase();
            // Check for various Claude Code indicators
            lower.contains("claude") || lower == "node" || lower == "npx"
        } else {
            false
        }
    }

    /// Get current activity status.
    pub fn status(&self) -> TerminalStatus {
        self.status
    }

    /// Check if the current command has been running for a long time (> 30s).
    pub fn is_long_running(&self) -> bool {
        if let Some(start) = self.command_start_time {
            start.elapsed().as_secs() > 30
        } else {
            false
        }
    }

    /// Export terminal content as HTML using the given theme.
    pub fn export_html(&self, theme: &TerminalTheme) -> String {
        let screen = self.screen.lock().unwrap();
        screen.export_html(&theme.ansi_colors, theme.foreground, theme.background)
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if let Err(e) = self.pty.resize(cols, rows) {
            log::warn!("Failed to resize PTY: {}", e);
        }

        let mut screen = self.screen.lock().unwrap();
        screen.resize(cols, rows);
    }

    /// Get the current terminal size (cols, rows).
    pub fn size(&self) -> (u16, u16) {
        let screen = self.screen.lock().unwrap();
        screen.size()
    }
}

/// Metadata for saving/loading a terminal instance.
#[derive(Serialize, Deserialize, Clone)]
pub struct SavedTerminal {
    pub shell: ShellType,
    pub cwd: Option<std::path::PathBuf>,
    pub title: String,
}

/// Structure for saving a complete tab layout.
#[derive(Serialize, Deserialize, Clone)]
pub struct SavedLayout {
    pub name: String,
    pub layout: TerminalLayout,
    pub terminals: HashMap<usize, SavedTerminal>,
}

/// Structure for saving a floating window.
#[derive(Serialize, Deserialize)]
pub struct SavedFloatingWindow {
    pub layout: SavedLayout,
    pub title: String,
    pub position: Option<(f32, f32)>,
    pub size: (f32, f32),
}

/// Structure for saving the entire workspace (tabs + floating windows).
#[derive(Serialize, Deserialize)]
pub struct SavedWorkspace {
    pub name: String,
    pub tabs: Vec<SavedLayout>,
    pub floating_windows: Vec<SavedFloatingWindow>,
    pub active_tab_index: usize,
}

/// Manager for multiple terminal instances.
pub struct TerminalManager {
    /// All terminal instances (by ID)
    terminals: HashMap<usize, Terminal>,
    /// Layouts for each tab
    tabs: Vec<TerminalLayout>,
    /// Index of the active tab
    active_tab_index: usize,
    /// ID of the currently focused terminal
    focused_terminal_id: Option<usize>,
    /// Counter for generating unique terminal IDs
    next_id: usize,
    /// Default terminal size
    default_cols: u16,
    default_rows: u16,
    /// Default scrollback lines
    default_scrollback: usize,
    /// Currently configured prompt patterns
    current_prompt_patterns: Vec<String>,
    /// Saved command macros
    macros: HashMap<String, String>,
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager {
    /// Create a new terminal manager.
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            tabs: Vec::new(),
            active_tab_index: 0,
            focused_terminal_id: None,
            next_id: 1,
            default_cols: 80,
            default_rows: 24,
            default_scrollback: 10000,
            current_prompt_patterns: Vec::new(),
            macros: HashMap::new(),
        }
    }

    /// Set macros.
    pub fn set_macros(&mut self, macros: HashMap<String, String>) {
        self.macros = macros;
    }

    /// Get all macros.
    pub fn macros(&self) -> &HashMap<String, String> {
        &self.macros
    }

    /// Play a macro on the active terminal.
    pub fn play_macro(&mut self, name: &str) -> Result<(), String> {
        if let Some(command) = self.macros.get(name).cloned() {
            if let Some(terminal) = self.active_terminal_mut() {
                terminal.write_str(&command);
                terminal.write_str("\n");
                Ok(())
            } else {
                Err("No active terminal".to_string())
            }
        } else {
            Err(format!("Macro '{}' not found", name))
        }
    }

    /// Notify terminals of a file change.
    pub fn on_file_changed(&mut self, path: &std::path::Path) {
        for terminal in self.terminals.values_mut() {
            terminal.on_file_changed(path);
        }
    }

    /// Set prompt patterns for all terminals.
    pub fn set_prompt_patterns(&mut self, patterns: Vec<String>) {
        if self.current_prompt_patterns == patterns {
            return;
        }
        self.current_prompt_patterns = patterns;
        for terminal in self.terminals.values_mut() {
            terminal.update_prompt_patterns(&self.current_prompt_patterns);
        }
    }

    /// Set the default terminal size.
    pub fn set_default_size(&mut self, cols: u16, rows: u16) {
        self.default_cols = cols;
        self.default_rows = rows;
    }

    /// Set the default scrollback limit.
    pub fn set_default_scrollback(&mut self, lines: usize) {
        self.default_scrollback = lines;
    }

    /// Create a new terminal and return its index.
    /// If working_dir is provided, the terminal will start in that directory.
    pub fn create_terminal(
        &mut self,
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<usize, String> {
        let id = self.next_id;
        self.next_id += 1;

        let mut terminal = Terminal::new(
            id,
            self.default_cols,
            self.default_rows,
            shell_type,
            working_dir,
            self.default_scrollback,
        )?;
        terminal.update_prompt_patterns(&self.current_prompt_patterns);
        self.terminals.insert(id, terminal);

        self.tabs.push(TerminalLayout::Terminal(id));

        let index = self.tabs.len() - 1;
        self.active_tab_index = index;
        self.focused_terminal_id = Some(id);

        log::info!(
            "Created terminal {} (tab {}) with shell type {:?}",
            id,
            index,
            shell_type
        );
        Ok(index)
    }

    /// Get the active terminal.
    pub fn active_terminal(&self) -> Option<&Terminal> {
        self.focused_terminal_id
            .and_then(|id| self.terminals.get(&id))
    }

    /// Get mutable access to the active terminal.
    pub fn active_terminal_mut(&mut self) -> Option<&mut Terminal> {
        self.focused_terminal_id
            .and_then(|id| self.terminals.get_mut(&id))
    }

    /// Get a terminal by tab index (returns the primary/first terminal in the tab).
    pub fn terminal(&self, index: usize) -> Option<&Terminal> {
        self.tabs.get(index).and_then(|layout| {
            let id = layout.first_leaf();
            self.terminals.get(&id)
        })
    }

    /// Get mutable access to a terminal by tab index.
    pub fn terminal_mut(&mut self, index: usize) -> Option<&mut Terminal> {
        if let Some(layout) = self.tabs.get(index) {
            let id = layout.first_leaf();
            self.terminals.get_mut(&id)
        } else {
            None
        }
    }

    /// Get mutable access to a terminal by ID.
    pub fn terminal_mut_by_id(&mut self, id: usize) -> Option<&mut Terminal> {
        self.terminals.get_mut(&id)
    }

    /// Set the focused terminal ID.
    pub fn set_focused_terminal(&mut self, id: usize) {
        if self.terminals.contains_key(&id) {
            self.focused_terminal_id = Some(id);
        }
    }

    /// Get the focused terminal ID.
    pub fn focused_terminal_id(&self) -> Option<usize> {
        self.focused_terminal_id
    }

    /// Get an immutable reference to the focused terminal.
    pub fn focused_terminal(&self) -> Option<&Terminal> {
        self.focused_terminal_id
            .and_then(|id| self.terminals.get(&id))
    }

    /// Set the active terminal tab by index.
    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            // Update focused terminal to the first one in this tab
            // In the future, we could remember the last focused terminal per tab
            if let Some(layout) = self.tabs.get(index) {
                self.focused_terminal_id = Some(layout.first_leaf());
            }
        }
    }

    /// Close a terminal tab by index.
    pub fn close_terminal(&mut self, index: usize) {
        if index < self.tabs.len() {
            let layout = self.tabs.remove(index);

            // Cleanup all terminals in this layout
            for id in layout.collect_leaves() {
                self.terminals.remove(&id);
            }

            // Adjust active index if needed
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }

            // Update focus
            if let Some(layout) = self.tabs.get(self.active_tab_index) {
                self.focused_terminal_id = Some(layout.first_leaf());
            } else {
                self.focused_terminal_id = None;
            }
        }
    }

    /// Remove a tab (e.g. for floating) without closing the terminals.
    pub fn remove_tab(&mut self, index: usize) -> Option<TerminalLayout> {
        if index < self.tabs.len() {
            let layout = self.tabs.remove(index);

            // Adjust active index
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }

            // Update focus
            if let Some(layout) = self.tabs.get(self.active_tab_index) {
                self.focused_terminal_id = Some(layout.first_leaf());
            } else {
                self.focused_terminal_id = None;
            }

            Some(layout)
        } else {
            None
        }
    }

    /// Add a tab (e.g. re-docking).
    pub fn add_tab(&mut self, layout: TerminalLayout) {
        self.tabs.push(layout);
        // Switch to new tab
        self.active_tab_index = self.tabs.len() - 1;
        // Focus first leaf
        if let Some(layout) = self.tabs.get(self.active_tab_index) {
            self.focused_terminal_id = Some(layout.first_leaf());
        }
    }

    /// Get the number of terminal tabs.
    pub fn terminal_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get the active terminal tab index.
    pub fn active_index(&self) -> usize {
        self.active_tab_index
    }

    /// Get the layout of the active tab.
    pub fn active_tab_layout(&self) -> Option<&TerminalLayout> {
        self.tabs.get(self.active_tab_index)
    }

    /// Get the layout of the active tab (mutable).
    pub fn active_tab_layout_mut(&mut self) -> Option<&mut TerminalLayout> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Check if there are any terminals.
    pub fn has_terminals(&self) -> bool {
        !self.tabs.is_empty()
    }

    /// Get IDs and running status of all terminals.
    /// Returns pairs of (terminal_id, is_running, title).
    pub fn terminal_statuses(&self) -> Vec<(usize, bool, String)> {
        self.terminals
            .iter()
            .map(|(&id, t)| (id, t.is_running(), t.title().to_string()))
            .collect()
    }

    /// Poll all terminals for new data.
    /// Returns true if any terminal had new data.
    pub fn poll_all(&mut self) -> bool {
        let mut any_data = false;
        for terminal in self.terminals.values_mut() {
            if terminal.poll() {
                any_data = true;
            }
        }
        any_data
    }

    /// Get terminal titles for tab display.
    /// Returns: (index, title, git_branch, status, long_running, is_active, is_waiting_for_input)
    pub fn terminal_titles(
        &self,
    ) -> Vec<(
        usize,
        String,
        Option<String>,
        TerminalStatus,
        bool,
        bool,
        bool,
    )> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, layout)| {
                let id = layout.first_leaf();
                let terminal = self.terminals.get(&id);
                let title = terminal
                    .map(|t| t.title().to_string())
                    .unwrap_or_else(|| "Terminal".to_string());
                let branch = terminal.and_then(|t| t.git_branch().map(|s| s.to_string()));
                let status = terminal.map(|t| t.status()).unwrap_or(TerminalStatus::Idle);
                let long_running = terminal.map(|t| t.is_long_running()).unwrap_or(false);
                let is_waiting = terminal.map(|t| t.is_waiting_for_input()).unwrap_or(false);
                (
                    i,
                    title,
                    branch,
                    status,
                    long_running,
                    i == self.active_tab_index,
                    is_waiting,
                )
            })
            .collect()
    }

    /// Resize all terminals to a new size.
    pub fn resize_all(&mut self, cols: u16, rows: u16) {
        self.default_cols = cols;
        self.default_rows = rows;

        for terminal in self.terminals.values_mut() {
            terminal.resize(cols, rows);
        }
    }

    /// Split the current pane.
    pub fn split_pane(
        &mut self,
        direction: layout::Direction,
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<(), String> {
        if let Some(layout) = self.tabs.get_mut(self.active_tab_index) {
            if let Some(target_id) = self.focused_terminal_id {
                let id = self.next_id;
                self.next_id += 1;

                let terminal = Terminal::new(
                    id,
                    self.default_cols,
                    self.default_rows,
                    shell_type,
                    working_dir,
                    self.default_scrollback,
                )?;
                self.terminals.insert(id, terminal);

                if layout.split(target_id, id, direction) {
                    self.focused_terminal_id = Some(id);
                    log::info!(
                        "Split pane for terminal {} -> new terminal {}",
                        target_id,
                        id
                    );
                    Ok(())
                } else {
                    Err("Failed to find target terminal in layout".to_string())
                }
            } else {
                Err("No focused terminal".to_string())
            }
        } else {
            Err("No active tab".to_string())
        }
    }

    /// Close the focused pane.
    pub fn close_focused_pane(&mut self) {
        if let Some(id) = self.focused_terminal_id {
            if let Some(layout) = self.tabs.get_mut(self.active_tab_index) {
                // If it's the only terminal in the tab, close the tab
                let is_root = match layout {
                    TerminalLayout::Terminal(tid) => *tid == id,
                    _ => false,
                };

                if is_root {
                    self.close_terminal(self.active_tab_index);
                } else {
                    layout.remove_id(id);
                    self.terminals.remove(&id);
                    // Update focus
                    if let Some(layout) = self.tabs.get(self.active_tab_index) {
                        self.focused_terminal_id = Some(layout.first_leaf());
                    }
                }
            }
        }
    }

    /// Move focus in the given direction.
    pub fn move_focus(&mut self, direction: layout::MoveDirection) {
        if let Some(id) = self.focused_terminal_id {
            if let Some(layout) = self.tabs.get(self.active_tab_index) {
                if let Some(new_id) = layout.navigate(id, direction) {
                    self.focused_terminal_id = Some(new_id);
                }
            }
        }
    }

    /// Create a new tab with a grid layout.
    pub fn create_grid_layout(
        &mut self,
        rows: usize,
        cols: usize,
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<usize, String> {
        let start_id = self.next_id;
        let (layout, next_id) = TerminalLayout::grid(rows, cols, start_id);

        // Create actual terminals
        for id in start_id..next_id {
            let terminal = Terminal::new(
                id,
                self.default_cols,
                self.default_rows,
                shell_type,
                working_dir.clone(),
                self.default_scrollback,
            )?;
            self.terminals.insert(id, terminal);
        }

        self.next_id = next_id;
        self.tabs.push(layout.clone());

        let index = self.tabs.len() - 1;
        self.active_tab_index = index;
        self.focused_terminal_id = Some(layout.first_leaf());

        Ok(index)
    }

    /// Swap two tabs by index.
    pub fn swap_tabs(&mut self, a: usize, b: usize) {
        if a < self.tabs.len() && b < self.tabs.len() && a != b {
            self.tabs.swap(a, b);

            // Follow active tab
            if self.active_tab_index == a {
                self.active_tab_index = b;
            } else if self.active_tab_index == b {
                self.active_tab_index = a;
            }
        }
    }

    /// Merge a tab into the active tab as a split with the focused terminal.
    /// Used for drag-to-split operations.
    /// Returns true if successful.
    pub fn merge_tab_as_split(
        &mut self,
        tab_idx: usize,
        direction: layout::Direction,
        insert_before: bool,
    ) -> bool {
        // Don't allow merging active tab into itself
        if tab_idx == self.active_tab_index {
            return false;
        }

        // Get the focused terminal in active tab
        let target_id = match self.focused_terminal_id {
            Some(id) => id,
            None => return false,
        };

        // Remove the source tab
        let layout = match self.remove_tab(tab_idx) {
            Some(l) => l,
            None => return false,
        };

        // Get the active tab's layout and insert the split
        if let Some(active_layout) = self.tabs.get_mut(self.active_tab_index) {
            if active_layout.split_with_layout(target_id, layout, direction, insert_before) {
                log::info!("Merged tab {} as split into active tab", tab_idx);
                return true;
            } else {
                log::warn!("Failed to split_with_layout - target not found");
            }
        }

        false
    }

    /// Get all tab layouts.
    pub fn tabs(&self) -> &Vec<TerminalLayout> {
        &self.tabs
    }

    /// Save a specific layout node.
    pub fn save_layout(&self, layout: &TerminalLayout, name: String) -> SavedLayout {
        let mut terminals = HashMap::new();

        for id in layout.collect_leaves() {
            if let Some(t) = self.terminals.get(&id) {
                terminals.insert(
                    id,
                    SavedTerminal {
                        shell: t.shell_type,
                        cwd: t.working_dir.clone(),
                        title: t.title.clone(),
                    },
                );
            }
        }

        SavedLayout {
            name,
            layout: layout.clone(),
            terminals,
        }
    }

    /// Save the current active tab's layout to a serializable structure.
    pub fn save_active_layout(&self, name: String) -> Option<SavedLayout> {
        let layout = self.tabs.get(self.active_tab_index)?;
        Some(self.save_layout(layout, name))
    }

    /// Helper to create a terminal from saved config.
    fn create_terminal_from_config(&mut self, config: SavedTerminal) -> Result<usize, String> {
        let new_id = self.next_id;
        self.next_id += 1;

        let mut terminal = Terminal::new(
            new_id,
            self.default_cols,
            self.default_rows,
            config.shell,
            config.cwd,
            self.default_scrollback,
        )?;
        terminal.set_title(config.title);
        terminal.update_prompt_patterns(&self.current_prompt_patterns);
        self.terminals.insert(new_id, terminal);

        Ok(new_id)
    }

    /// Load a layout from a saved structure.
    pub fn load_layout(&mut self, saved: SavedLayout) -> Result<(), String> {
        let mut id_map = HashMap::new();

        // 1. Create terminals
        for (old_id, config) in saved.terminals {
            let new_id = self.create_terminal_from_config(config)?;
            id_map.insert(old_id, new_id);
        }

        // 2. Map layout tree
        let mut new_layout = saved.layout;
        self.remap_layout_ids(&mut new_layout, &id_map);

        // 3. Add tab
        self.add_tab(new_layout);
        Ok(())
    }

    /// Load a full workspace.
    pub fn load_workspace(
        &mut self,
        saved: SavedWorkspace,
    ) -> Result<Vec<(TerminalLayout, String, Option<(f32, f32)>, (f32, f32))>, String> {
        // Load tabs
        self.terminals.clear();
        self.tabs.clear();
        self.next_id = 1;

        // 1. Load tabs
        for saved_layout in saved.tabs {
            self.load_layout(saved_layout)?;
        }
        self.active_tab_index = saved
            .active_tab_index
            .min(self.tabs.len().saturating_sub(1));
        if let Some(layout) = self.tabs.get(self.active_tab_index) {
            self.focused_terminal_id = Some(layout.first_leaf());
        }

        // 2. Process floating windows (return them for UI to create Viewports)
        let mut floating_windows = Vec::new();
        for fw in saved.floating_windows {
            // Create terminals for this FW
            let mut id_map = HashMap::new();
            for (old_id, config) in fw.layout.terminals {
                let new_id = self.create_terminal_from_config(config)?;
                id_map.insert(old_id, new_id);
            }

            let mut new_layout = fw.layout.layout;
            self.remap_layout_ids(&mut new_layout, &id_map);

            floating_windows.push((new_layout, fw.title, fw.position, fw.size));
        }

        Ok(floating_windows)
    }

    fn remap_layout_ids(&self, layout: &mut TerminalLayout, map: &HashMap<usize, usize>) {
        match layout {
            TerminalLayout::Terminal(id) => {
                if let Some(&new_id) = map.get(id) {
                    *id = new_id;
                }
            }
            TerminalLayout::Horizontal { splits, .. } | TerminalLayout::Vertical { splits, .. } => {
                for s in splits {
                    self.remap_layout_ids(s, map);
                }
            }
        }
    }
}

/// Information about a connected monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Detect connected monitors using PowerShell on Windows.
pub fn detect_monitors() -> Vec<MonitorInfo> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        let mut cmd = Command::new("powershell.exe");
        cmd.args(&[
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::AllScreens | Select-Object -Property DeviceName, @{Name='X';Expression={$_.Bounds.X}}, @{Name='Y';Expression={$_.Bounds.Y}}, @{Name='Width';Expression={$_.Bounds.Width}}, @{Name='Height';Expression={$_.Bounds.Height}} | ConvertTo-Json"
        ]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let json = String::from_utf8_lossy(&output.stdout);
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                    let mut monitors = Vec::new();
                    let items = if val.is_array() {
                        val.as_array().unwrap().clone()
                    } else if val.is_object() {
                        vec![val]
                    } else {
                        vec![]
                    };

                    for item in items {
                        let name = item["DeviceName"].as_str().unwrap_or("Unknown").to_string();
                        let x = item["X"].as_f64().unwrap_or(0.0) as f32;
                        let y = item["Y"].as_f64().unwrap_or(0.0) as f32;
                        let w = item["Width"].as_f64().unwrap_or(1920.0) as f32;
                        let h = item["Height"].as_f64().unwrap_or(1080.0) as f32;
                        monitors.push(MonitorInfo {
                            name,
                            x,
                            y,
                            width: w,
                            height: h,
                        });
                    }
                    if !monitors.is_empty() {
                        return monitors;
                    }
                }
            }
        }
    }

    // Fallback for primary monitor
    vec![MonitorInfo {
        name: "Primary".to_string(),
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    }]
}
