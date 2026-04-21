//! Live Pipeline Panel for piping JSON/YAML through shell commands.
//!
//! This module implements a panel that allows users to pipe their current
//! JSON/YAML document content through configurable shell commands (like jq, yq)
//! and see live-updating output with recent command history.

// Allow dead code - includes API methods for future debounce and advanced features
#![allow(dead_code)]

use eframe::egui::{self, Color32, Key, RichText, ScrollArea, TextEdit, Ui};
use rust_i18n::t;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Default debounce delay in milliseconds.
const DEFAULT_DEBOUNCE_MS: u32 = 500;

/// Maximum output size in bytes (1 MB).
const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Maximum runtime in milliseconds (30 seconds).
const DEFAULT_MAX_RUNTIME_MS: u64 = 30000;

/// Maximum number of recent commands to keep.
const MAX_RECENT_COMMANDS: usize = 20;

/// Minimum panel height.
const MIN_PANEL_HEIGHT: f32 = 100.0;

/// Default panel height.
const DEFAULT_PANEL_HEIGHT: f32 = 200.0;

/// Maximum panel height.
const MAX_PANEL_HEIGHT: f32 = 500.0;

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Execution State
// ─────────────────────────────────────────────────────────────────────────────

/// Status of pipeline execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStatus {
    /// No command has been run yet.
    Idle,
    /// Command is currently executing.
    Running,
    /// Command completed successfully.
    Completed {
        /// Exit code from the process.
        exit_code: i32,
        /// Execution time in milliseconds.
        duration_ms: u64,
        /// Whether output was truncated.
        truncated: bool,
    },
    /// Command failed to execute.
    Error {
        /// Error message.
        message: String,
    },
    /// Command was cancelled.
    Cancelled,
    /// Command timed out.
    TimedOut,
}

impl Default for PipelineStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl PipelineStatus {
    /// Get a display string for the status.
    pub fn display(&self) -> &str {
        match self {
            Self::Idle => "Ready",
            Self::Running => "Running...",
            Self::Completed { .. } => "Completed",
            Self::Error { .. } => "Error",
            Self::Cancelled => "Cancelled",
            Self::TimedOut => "Timed Out",
        }
    }

    /// Get status color.
    pub fn color(&self, is_dark: bool) -> Color32 {
        match self {
            Self::Idle => {
                if is_dark {
                    Color32::from_rgb(150, 150, 150)
                } else {
                    Color32::from_rgb(100, 100, 100)
                }
            }
            Self::Running => Color32::from_rgb(59, 130, 246), // Blue
            Self::Completed { exit_code, .. } => {
                if *exit_code == 0 {
                    Color32::from_rgb(34, 197, 94) // Green
                } else {
                    Color32::from_rgb(249, 115, 22) // Orange
                }
            }
            Self::Error { .. } | Self::TimedOut => Color32::from_rgb(239, 68, 68), // Red
            Self::Cancelled => Color32::from_rgb(156, 163, 175),                   // Gray
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Result (from background thread)
// ─────────────────────────────────────────────────────────────────────────────

/// Result from a pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Stdout output.
    pub stdout: String,
    /// Stderr output.
    pub stderr: String,
    /// Exit code (None if process was killed or didn't exit normally).
    pub exit_code: Option<i32>,
    /// Execution duration.
    pub duration: Duration,
    /// Whether stdout was truncated.
    pub stdout_truncated: bool,
    /// Whether stderr was truncated.
    pub stderr_truncated: bool,
    /// Error message if execution failed.
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Panel Output
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the pipeline panel indicating user actions.
#[derive(Debug, Default)]
pub struct PipelinePanelOutput {
    /// Whether the panel was closed.
    pub closed: bool,
    /// New panel height if resized.
    pub new_height: Option<f32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-Tab Pipeline State
// ─────────────────────────────────────────────────────────────────────────────

/// Pipeline state stored per-tab.
#[derive(Debug, Clone, Default)]
pub struct TabPipelineState {
    /// Current command string.
    pub command: String,
    /// Last stdout output.
    pub stdout: String,
    /// Last stderr output.
    pub stderr: String,
    /// Current execution status.
    pub status: PipelineStatus,
    /// Whether the panel is visible for this tab.
    pub panel_visible: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Executor (handles background execution)
// ─────────────────────────────────────────────────────────────────────────────

/// Handle for cancelling a running pipeline.
pub struct PipelineHandle {
    /// Sender to signal cancellation.
    cancel_tx: Sender<()>,
}

impl PipelineHandle {
    /// Cancel the running pipeline.
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(());
    }
}

/// Execute a pipeline command in a background thread.
///
/// Returns a handle for cancellation and a receiver for the result.
pub fn execute_pipeline(
    command: String,
    input: String,
    working_dir: Option<std::path::PathBuf>,
    max_output_bytes: usize,
    max_runtime_ms: u64,
) -> (PipelineHandle, Receiver<PipelineResult>) {
    let (result_tx, result_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpsc::channel();

    thread::spawn(move || {
        let start = Instant::now();

        // Build the command based on platform
        #[cfg(windows)]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        };

        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        // Set working directory if provided
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Configure pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the process
        let mut child: Child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = result_tx.send(PipelineResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration: start.elapsed(),
                    stdout_truncated: false,
                    stderr_truncated: false,
                    error: Some(format!("Failed to spawn process: {}", e)),
                });
                return;
            }
        };

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            // stdin is dropped here, closing the pipe
        }

        // Read output with timeout and cancellation checking
        let timeout = Duration::from_millis(max_runtime_ms);
        let check_interval = Duration::from_millis(100);
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let mut stdout_truncated = false;
        let mut stderr_truncated = false;

        // We need to read stdout and stderr in a non-blocking way
        // For simplicity, we'll wait for the process to complete with timeout
        loop {
            // Check for cancellation
            if cancel_rx.try_recv().is_ok() {
                let _ = child.kill();
                let _ = result_tx.send(PipelineResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration: start.elapsed(),
                    stdout_truncated: false,
                    stderr_truncated: false,
                    error: Some("Cancelled".to_string()),
                });
                return;
            }

            // Check for timeout
            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = result_tx.send(PipelineResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration: start.elapsed(),
                    stdout_truncated: false,
                    stderr_truncated: false,
                    error: Some(format!("Process timed out after {}ms", max_runtime_ms)),
                });
                return;
            }

            // Check if process has exited
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Process has exited, read remaining output
                    break;
                }
                Ok(None) => {
                    // Process still running, wait a bit
                    thread::sleep(check_interval);
                }
                Err(e) => {
                    let _ = result_tx.send(PipelineResult {
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: None,
                        duration: start.elapsed(),
                        stdout_truncated: false,
                        stderr_truncated: false,
                        error: Some(format!("Error waiting for process: {}", e)),
                    });
                    return;
                }
            }
        }

        // Read stdout
        if let Some(mut stdout) = child.stdout.take() {
            let mut buf = vec![0u8; max_output_bytes + 1];
            match stdout.read(&mut buf) {
                Ok(n) => {
                    if n > max_output_bytes {
                        stdout_truncated = true;
                        stdout_buf = buf[..max_output_bytes].to_vec();
                    } else {
                        stdout_buf = buf[..n].to_vec();
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read stdout: {}", e);
                }
            }
        }

        // Read stderr
        if let Some(mut stderr) = child.stderr.take() {
            let mut buf = vec![0u8; max_output_bytes + 1];
            match stderr.read(&mut buf) {
                Ok(n) => {
                    if n > max_output_bytes {
                        stderr_truncated = true;
                        stderr_buf = buf[..max_output_bytes].to_vec();
                    } else {
                        stderr_buf = buf[..n].to_vec();
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read stderr: {}", e);
                }
            }
        }

        // Get exit status
        let exit_code = match child.wait() {
            Ok(status) => status.code(),
            Err(_) => None,
        };

        let _ = result_tx.send(PipelineResult {
            stdout: String::from_utf8_lossy(&stdout_buf).to_string(),
            stderr: String::from_utf8_lossy(&stderr_buf).to_string(),
            exit_code,
            duration: start.elapsed(),
            stdout_truncated,
            stderr_truncated,
            error: None,
        });
    });

    (PipelineHandle { cancel_tx }, result_rx)
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Panel
// ─────────────────────────────────────────────────────────────────────────────

/// The Live Pipeline panel widget.
pub struct PipelinePanel {
    /// Whether the panel is globally enabled (user can still toggle per-tab).
    is_enabled: bool,
    /// Panel height.
    height: f32,
    /// Recent commands history (shared across tabs).
    recent_commands: VecDeque<String>,
    /// Whether to show the recent commands dropdown.
    show_history_dropdown: bool,
    /// Current handle for running pipeline (for cancellation).
    current_handle: Option<PipelineHandle>,
    /// Receiver for pipeline results.
    result_receiver: Option<Receiver<PipelineResult>>,
    /// ID counter for unique widget IDs.
    id_counter: usize,
    /// Last command that was executed (for debounce tracking).
    last_executed_command: String,
    /// Last content hash that was executed (for debounce tracking).
    last_executed_content_hash: u64,
    /// Time of last keystroke (for debounce).
    last_keystroke: Option<Instant>,
    /// Debounce delay in milliseconds.
    debounce_ms: u32,
    /// Maximum output size in bytes.
    max_output_bytes: usize,
    /// Maximum runtime in milliseconds.
    max_runtime_ms: u64,
    /// Whether command input has focus.
    command_input_focused: bool,
}

impl Default for PipelinePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelinePanel {
    /// Create a new pipeline panel.
    pub fn new() -> Self {
        Self {
            is_enabled: true,
            height: DEFAULT_PANEL_HEIGHT,
            recent_commands: VecDeque::with_capacity(MAX_RECENT_COMMANDS),
            show_history_dropdown: false,
            current_handle: None,
            result_receiver: None,
            id_counter: 0,
            last_executed_command: String::new(),
            last_executed_content_hash: 0,
            last_keystroke: None,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_runtime_ms: DEFAULT_MAX_RUNTIME_MS,
            command_input_focused: false,
        }
    }

    /// Check if the panel is enabled.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Set whether the panel is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
    }

    /// Get the panel height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Set the panel height.
    pub fn set_height(&mut self, height: f32) {
        self.height = height.clamp(MIN_PANEL_HEIGHT, MAX_PANEL_HEIGHT);
    }

    /// Configure pipeline settings.
    pub fn configure(&mut self, debounce_ms: u32, max_output_bytes: usize, max_runtime_ms: u64) {
        self.debounce_ms = debounce_ms.max(100); // Minimum 100ms debounce
        self.max_output_bytes = max_output_bytes.max(1024); // Minimum 1KB
        self.max_runtime_ms = max_runtime_ms.max(1000); // Minimum 1 second
    }

    /// Get recent commands.
    pub fn recent_commands(&self) -> &VecDeque<String> {
        &self.recent_commands
    }

    /// Add a command to recent history.
    pub fn add_to_history(&mut self, command: &str) {
        if command.trim().is_empty() {
            return;
        }

        // Remove if already exists (deduplication)
        self.recent_commands.retain(|c| c != command);

        // Add to front
        self.recent_commands.push_front(command.to_string());

        // Trim to max size
        while self.recent_commands.len() > MAX_RECENT_COMMANDS {
            self.recent_commands.pop_back();
        }
    }

    /// Set recent commands (for loading from persistence).
    pub fn set_recent_commands(&mut self, commands: Vec<String>) {
        self.recent_commands.clear();
        for cmd in commands.into_iter().take(MAX_RECENT_COMMANDS) {
            self.recent_commands.push_back(cmd);
        }
    }

    /// Get recent commands as a Vec (for persistence).
    pub fn get_recent_commands_vec(&self) -> Vec<String> {
        self.recent_commands.iter().cloned().collect()
    }

    /// Cancel any running pipeline.
    pub fn cancel(&mut self) {
        if let Some(handle) = self.current_handle.take() {
            handle.cancel();
        }
        self.result_receiver = None;
    }

    /// Check if a pipeline is currently running.
    pub fn is_running(&self) -> bool {
        self.current_handle.is_some()
    }

    /// Poll for pipeline results.
    ///
    /// Returns Some(result) if a result is available, None otherwise.
    pub fn poll_result(&mut self) -> Option<PipelineResult> {
        if let Some(ref rx) = self.result_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.current_handle = None;
                    self.result_receiver = None;
                    Some(result)
                }
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.current_handle = None;
                    self.result_receiver = None;
                    None
                }
            }
        } else {
            None
        }
    }

    /// Execute a pipeline command.
    pub fn execute(&mut self, command: &str, input: &str, working_dir: Option<std::path::PathBuf>) {
        // Cancel any running pipeline
        self.cancel();

        // Don't execute empty commands
        if command.trim().is_empty() {
            return;
        }

        // Add to history
        self.add_to_history(command);

        // Execute in background
        let (handle, rx) = execute_pipeline(
            command.to_string(),
            input.to_string(),
            working_dir,
            self.max_output_bytes,
            self.max_runtime_ms,
        );

        self.current_handle = Some(handle);
        self.result_receiver = Some(rx);
        self.last_executed_command = command.to_string();
    }

    /// Check if execution should be triggered based on debounce.
    ///
    /// Returns true if enough time has passed since last keystroke.
    pub fn should_execute(&self) -> bool {
        if let Some(last) = self.last_keystroke {
            last.elapsed() >= Duration::from_millis(self.debounce_ms as u64)
        } else {
            false
        }
    }

    /// Record a keystroke for debounce tracking.
    pub fn record_keystroke(&mut self) {
        self.last_keystroke = Some(Instant::now());
    }

    /// Check if command or content has changed since last execution.
    pub fn has_changes(&self, command: &str, content_hash: u64) -> bool {
        command != self.last_executed_command || content_hash != self.last_executed_content_hash
    }

    /// Update the last executed content hash.
    pub fn set_content_hash(&mut self, hash: u64) {
        self.last_executed_content_hash = hash;
    }

    /// Generate a unique ID for widgets.
    fn next_id(&mut self) -> usize {
        self.id_counter += 1;
        self.id_counter
    }

    /// Render the pipeline panel.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `tab_state` - Mutable reference to the tab's pipeline state
    /// * `content` - Current document content (for piping to command)
    /// * `working_dir` - Optional working directory for command execution
    /// * `is_dark` - Whether using dark theme
    ///
    /// # Returns
    ///
    /// Output indicating any user actions.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tab_state: &mut TabPipelineState,
        content: &str,
        working_dir: Option<std::path::PathBuf>,
        is_dark: bool,
    ) -> PipelinePanelOutput {
        let mut output = PipelinePanelOutput::default();

        // Panel colors
        let panel_bg = if is_dark {
            Color32::from_rgb(30, 30, 30)
        } else {
            Color32::from_rgb(248, 248, 248)
        };

        let border_color = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(30, 30, 30)
        };

        let secondary_text = if is_dark {
            Color32::from_rgb(150, 150, 150)
        } else {
            Color32::from_rgb(100, 100, 100)
        };

        let error_color = Color32::from_rgb(239, 68, 68);
        let success_color = Color32::from_rgb(34, 197, 94);

        // Poll for results
        if let Some(result) = self.poll_result() {
            if let Some(error) = result.error {
                if error == "Cancelled" {
                    tab_state.status = PipelineStatus::Cancelled;
                } else if error.contains("timed out") {
                    tab_state.status = PipelineStatus::TimedOut;
                } else {
                    tab_state.status = PipelineStatus::Error { message: error };
                }
            } else {
                tab_state.stdout = result.stdout;
                tab_state.stderr = result.stderr;
                tab_state.status = PipelineStatus::Completed {
                    exit_code: result.exit_code.unwrap_or(-1),
                    duration_ms: result.duration.as_millis() as u64,
                    truncated: result.stdout_truncated || result.stderr_truncated,
                };
            }
        }

        // Update running status
        if self.is_running() && !matches!(tab_state.status, PipelineStatus::Running) {
            tab_state.status = PipelineStatus::Running;
        }

        // Main panel frame
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_min_height(self.height);

                // ─────────────────────────────────────────────────────────
                // Header row: Title, Status, Close button
                // ─────────────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t!("pipeline.title").to_string())
                            .strong()
                            .color(text_color),
                    );
                    ui.separator();

                    // Status indicator
                    let status_color = tab_state.status.color(is_dark);
                    ui.label(
                        RichText::new(tab_state.status.display())
                            .small()
                            .color(status_color),
                    );

                    // Show exit code and duration for completed
                    if let PipelineStatus::Completed {
                        exit_code,
                        duration_ms,
                        truncated,
                    } = &tab_state.status
                    {
                        ui.label(RichText::new(format!("Exit: {}", exit_code)).small().color(
                            if *exit_code == 0 {
                                success_color
                            } else {
                                error_color
                            },
                        ));
                        ui.label(
                            RichText::new(format!("{}ms", duration_ms))
                                .small()
                                .color(secondary_text),
                        );
                        if *truncated {
                            ui.label(
                                RichText::new(t!("pipeline.truncated").to_string())
                                    .small()
                                    .color(Color32::from_rgb(249, 115, 22)),
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Close button
                        if ui
                            .button(RichText::new("✕").color(secondary_text))
                            .on_hover_text(t!("pipeline.close_tooltip").to_string())
                            .clicked()
                        {
                            output.closed = true;
                            tab_state.panel_visible = false;
                        }

                        // Cancel button (only when running)
                        if self.is_running() {
                            if ui
                                .button(RichText::new("⏹").color(error_color))
                                .on_hover_text(t!("pipeline.cancel_tooltip").to_string())
                                .clicked()
                            {
                                self.cancel();
                                tab_state.status = PipelineStatus::Cancelled;
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                // ─────────────────────────────────────────────────────────
                // Command input row
                // ─────────────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(RichText::new("$").monospace().color(secondary_text));

                    // Use a stable ID so the TextEdit can maintain focus between frames
                    let response = ui.add_sized(
                        [ui.available_width() - 80.0, 20.0],
                        TextEdit::singleline(&mut tab_state.command)
                            .id(egui::Id::new("pipeline_command_input"))
                            .font(egui::TextStyle::Monospace)
                            .hint_text(t!("pipeline.command_placeholder").to_string()),
                    );

                    // Track focus state
                    if response.gained_focus() {
                        self.command_input_focused = true;
                    }
                    if response.lost_focus() {
                        self.command_input_focused = false;
                    }

                    // Track keystrokes for debounce
                    if response.changed() {
                        self.record_keystroke();
                    }

                    // Execute on Enter
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.execute(&tab_state.command, content, working_dir.clone());
                        tab_state.status = PipelineStatus::Running;
                    }

                    // History dropdown button
                    let history_btn = ui
                        .button("📜")
                        .on_hover_text(t!("pipeline.recent").to_string());
                    if history_btn.clicked() {
                        self.show_history_dropdown = !self.show_history_dropdown;
                    }

                    // Run button
                    let run_enabled = !tab_state.command.trim().is_empty() && !self.is_running();
                    if ui
                        .add_enabled(
                            run_enabled,
                            egui::Button::new(t!("pipeline.run").to_string()),
                        )
                        .on_hover_text(t!("pipeline.run_tooltip").to_string())
                        .clicked()
                    {
                        self.execute(&tab_state.command, content, working_dir.clone());
                        tab_state.status = PipelineStatus::Running;
                    }
                });

                // History dropdown
                if self.show_history_dropdown && !self.recent_commands.is_empty() {
                    ui.add_space(2.0);
                    egui::Frame::none()
                        .fill(if is_dark {
                            Color32::from_rgb(40, 40, 40)
                        } else {
                            Color32::from_rgb(255, 255, 255)
                        })
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                                let mut selected_cmd = None;
                                for cmd in self.recent_commands.iter() {
                                    if ui
                                        .selectable_label(
                                            false,
                                            RichText::new(cmd).monospace().color(text_color),
                                        )
                                        .clicked()
                                    {
                                        selected_cmd = Some(cmd.clone());
                                    }
                                }
                                if let Some(cmd) = selected_cmd {
                                    tab_state.command = cmd;
                                    self.show_history_dropdown = false;
                                }
                            });
                        });
                }

                ui.add_space(8.0);

                // ─────────────────────────────────────────────────────────
                // Output area (stdout/stderr)
                // ─────────────────────────────────────────────────────────
                let output_height = ui.available_height() - 4.0;

                // Decide layout based on whether we have stderr
                let has_stderr = !tab_state.stderr.is_empty();

                if has_stderr {
                    // Split view: stdout on left, stderr on right
                    ui.horizontal(|ui| {
                        let half_width = (ui.available_width() - 8.0) / 2.0;

                        // Stdout panel
                        egui::Frame::none()
                            .fill(if is_dark {
                                Color32::from_rgb(25, 25, 25)
                            } else {
                                Color32::from_rgb(255, 255, 255)
                            })
                            .stroke(egui::Stroke::new(1.0, border_color))
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.set_width(half_width);
                                ui.set_height(output_height);
                                ui.label(
                                    RichText::new(t!("pipeline.stdout").to_string())
                                        .small()
                                        .color(secondary_text),
                                );
                                ScrollArea::vertical()
                                    .id_source("stdout_scroll")
                                    .show(ui, |ui| {
                                        if tab_state.stdout.is_empty() {
                                            ui.label(
                                                RichText::new(t!("pipeline.no_output").to_string())
                                                    .italics()
                                                    .color(secondary_text),
                                            );
                                        } else {
                                            ui.add(
                                                TextEdit::multiline(&mut tab_state.stdout.as_str())
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(f32::INFINITY)
                                                    .interactive(true),
                                            );
                                        }
                                    });
                            });

                        ui.add_space(8.0);

                        // Stderr panel
                        egui::Frame::none()
                            .fill(if is_dark {
                                Color32::from_rgb(35, 25, 25)
                            } else {
                                Color32::from_rgb(255, 248, 248)
                            })
                            .stroke(egui::Stroke::new(1.0, error_color.linear_multiply(0.5)))
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.set_width(half_width);
                                ui.set_height(output_height);
                                ui.label(
                                    RichText::new(t!("pipeline.stderr").to_string())
                                        .small()
                                        .color(error_color),
                                );
                                ScrollArea::vertical()
                                    .id_source("stderr_scroll")
                                    .show(ui, |ui| {
                                        ui.add(
                                            TextEdit::multiline(&mut tab_state.stderr.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .text_color(error_color)
                                                .desired_width(f32::INFINITY)
                                                .interactive(true),
                                        );
                                    });
                            });
                    });
                } else {
                    // Single stdout panel (full width)
                    egui::Frame::none()
                        .fill(if is_dark {
                            Color32::from_rgb(25, 25, 25)
                        } else {
                            Color32::from_rgb(255, 255, 255)
                        })
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.set_min_height(output_height);
                            ScrollArea::vertical()
                                .id_source("output_scroll")
                                .show(ui, |ui| {
                                    if tab_state.stdout.is_empty() {
                                        // Show hint when no output
                                        match &tab_state.status {
                                            PipelineStatus::Idle => {
                                                ui.label(
                                                    RichText::new(t!("pipeline.hint").to_string())
                                                        .italics()
                                                        .color(secondary_text),
                                                );
                                            }
                                            PipelineStatus::Running => {
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label(
                                                        RichText::new(
                                                            t!("pipeline.running").to_string(),
                                                        )
                                                        .color(secondary_text),
                                                    );
                                                });
                                            }
                                            PipelineStatus::Completed { exit_code, .. }
                                                if *exit_code == 0 =>
                                            {
                                                ui.label(
                                                    RichText::new(
                                                        t!("pipeline.no_output_success")
                                                            .to_string(),
                                                    )
                                                    .italics()
                                                    .color(secondary_text),
                                                );
                                            }
                                            PipelineStatus::Error { message } => {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{}: {}",
                                                        t!("common.error"),
                                                        message
                                                    ))
                                                    .color(error_color),
                                                );
                                            }
                                            _ => {
                                                ui.label(
                                                    RichText::new(
                                                        t!("pipeline.no_output").to_string(),
                                                    )
                                                    .italics()
                                                    .color(secondary_text),
                                                );
                                            }
                                        }
                                    } else {
                                        ui.add(
                                            TextEdit::multiline(&mut tab_state.stdout.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .interactive(true),
                                        );
                                    }
                                });
                        });
                }
            });

        output
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_status_display() {
        assert_eq!(PipelineStatus::Idle.display(), "Ready");
        assert_eq!(PipelineStatus::Running.display(), "Running...");
        assert_eq!(
            PipelineStatus::Completed {
                exit_code: 0,
                duration_ms: 100,
                truncated: false
            }
            .display(),
            "Completed"
        );
    }

    #[test]
    fn test_recent_commands_history() {
        let mut panel = PipelinePanel::new();

        panel.add_to_history("jq '.'");
        panel.add_to_history("yq '.data'");
        panel.add_to_history("jq '.'"); // Duplicate, should move to front

        assert_eq!(panel.recent_commands.len(), 2);
        assert_eq!(panel.recent_commands[0], "jq '.'");
        assert_eq!(panel.recent_commands[1], "yq '.data'");
    }

    #[test]
    fn test_recent_commands_max_size() {
        let mut panel = PipelinePanel::new();

        for i in 0..30 {
            panel.add_to_history(&format!("cmd{}", i));
        }

        assert_eq!(panel.recent_commands.len(), MAX_RECENT_COMMANDS);
    }

    #[test]
    fn test_panel_height_clamping() {
        let mut panel = PipelinePanel::new();

        panel.set_height(50.0); // Below minimum
        assert_eq!(panel.height(), MIN_PANEL_HEIGHT);

        panel.set_height(1000.0); // Above maximum
        assert_eq!(panel.height(), MAX_PANEL_HEIGHT);

        panel.set_height(300.0); // Within range
        assert_eq!(panel.height(), 300.0);
    }

    #[test]
    fn test_tab_pipeline_state_default() {
        let state = TabPipelineState::default();
        assert!(state.command.is_empty());
        assert!(state.stdout.is_empty());
        assert!(state.stderr.is_empty());
        assert_eq!(state.status, PipelineStatus::Idle);
        assert!(!state.panel_visible);
    }
}
