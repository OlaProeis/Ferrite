//! Productivity panel data models and persistence for Ferrite
//!
//! This module provides the core data structures for the productivity hub:
//! - Task management with markdown parsing
//! - Pomodoro timer state machine
//! - AutoSave helper for debounced writes
//! - Workspace-scoped persistence functions

use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Task Management
// ─────────────────────────────────────────────────────────────────────────────

/// A task item parsed from markdown checkbox syntax.
///
/// Supports:
/// - `- [ ] Task text` - Unchecked task
/// - `- [x] Task text` - Checked task
/// - `- [ ] ! Important` - Priority 1
/// - `- [ ] !! Urgent` - Priority 2
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub completed: bool,
    pub text: String,
    pub priority: u8, // 0=none, 1=!, 2=!!
}

impl Task {
    /// Parse a task from markdown checkbox syntax.
    ///
    /// Returns `None` if the line is not a valid task format.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(Task::from_markdown("- [ ] Buy milk").unwrap().text, "Buy milk");
    /// assert_eq!(Task::from_markdown("- [x] Done").unwrap().completed, true);
    /// assert_eq!(Task::from_markdown("- [ ] !! Urgent").unwrap().priority, 2);
    /// ```
    pub fn from_markdown(line: &str) -> Option<Self> {
        let trimmed = line.trim();

        // Must start with "- [ ]" or "- [x]"
        if !trimmed.starts_with("- [") {
            return None;
        }

        // Extract checkbox state
        let completed = if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            true
        } else if trimmed.starts_with("- [ ]") {
            false
        } else {
            return None;
        };

        // Extract text after checkbox
        let after_checkbox = if completed {
            trimmed
                .strip_prefix("- [x]")
                .or_else(|| trimmed.strip_prefix("- [X]"))?
        } else {
            trimmed.strip_prefix("- [ ]")?
        };

        let text = after_checkbox.trim();

        // Extract priority
        let (priority, text) = if let Some(rest) = text.strip_prefix("!! ") {
            (2, rest.to_string())
        } else if let Some(rest) = text.strip_prefix("! ") {
            (1, rest.to_string())
        } else {
            (0, text.to_string())
        };

        Some(Task {
            completed,
            text,
            priority,
        })
    }

    /// Render this task back to markdown checkbox syntax.
    #[cfg(test)]
    pub fn to_markdown(&self) -> String {
        let checkbox = if self.completed { "[x]" } else { "[ ]" };
        let priority = match self.priority {
            2 => "!! ",
            1 => "! ",
            _ => "",
        };
        format!("- {} {}{}", checkbox, priority, self.text)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pomodoro Timer
// ─────────────────────────────────────────────────────────────────────────────

/// Pomodoro timer state machine.
///
/// Uses `std::time::Instant` for timing to avoid issues with system clock changes.
#[derive(Clone, Debug)]
pub struct PomodoroTimer {
    state: TimerState,
    work_duration_secs: u64,  // Default: 25 * 60
    break_duration_secs: u64, // Default: 5 * 60
    completed_cycles: usize,
}

/// Internal timer state.
#[derive(Clone, Debug)]
enum TimerState {
    Idle,
    Work { started: Instant },
    Break { started: Instant },
}

impl PomodoroTimer {
    /// Create a new timer with default durations (25min work, 5min break).
    pub fn new() -> Self {
        Self {
            state: TimerState::Idle,
            work_duration_secs: 25 * 60,
            break_duration_secs: 5 * 60,
            completed_cycles: 0,
        }
    }

    /// Start a work session.
    pub fn start_work(&mut self) {
        self.state = TimerState::Work {
            started: Instant::now(),
        };
    }

    /// Start a break session.
    pub fn start_break(&mut self) {
        self.state = TimerState::Break {
            started: Instant::now(),
        };
    }

    /// Stop the timer.
    pub fn stop(&mut self) {
        self.state = TimerState::Idle;
    }

    /// Increment the completed cycles counter.
    pub fn increment_cycle(&mut self) {
        self.completed_cycles += 1;
    }

    /// Get the number of completed cycles.
    pub fn cycles(&self) -> usize {
        self.completed_cycles
    }

    /// Get remaining time in current session.
    ///
    /// Returns `None` if timer is idle.
    pub fn remaining(&self) -> Option<Duration> {
        match &self.state {
            TimerState::Idle => None,
            TimerState::Work { started } | TimerState::Break { started } => {
                let elapsed = started.elapsed();
                let total = Duration::from_secs(if matches!(self.state, TimerState::Work { .. }) {
                    self.work_duration_secs
                } else {
                    self.break_duration_secs
                });
                total.checked_sub(elapsed).or(Some(Duration::from_secs(0)))
            }
        }
    }

    /// Check if the timer has reached zero.
    pub fn is_complete(&self) -> bool {
        matches!(self.remaining(), Some(d) if d.as_secs() == 0)
    }

    /// Format remaining time as "MM:SS".
    pub fn format_remaining(&self) -> String {
        if let Some(remaining) = self.remaining() {
            let total_secs = remaining.as_secs();
            let minutes = total_secs / 60;
            let seconds = total_secs % 60;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            "00:00".to_string()
        }
    }

    /// Check if currently in a work session.
    pub fn is_work(&self) -> bool {
        matches!(self.state, TimerState::Work { .. })
    }

    /// Check if currently in a break session.
    pub fn is_break(&self) -> bool {
        matches!(self.state, TimerState::Break { .. })
    }

    /// Check if timer is active (work or break).
    pub fn is_active(&self) -> bool {
        !matches!(self.state, TimerState::Idle)
    }
}

impl Default for PomodoroTimer {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AutoSave Helper
// ─────────────────────────────────────────────────────────────────────────────

/// Debounced auto-save helper.
///
/// Prevents excessive file writes by debouncing edits.
pub struct AutoSave {
    last_edit: Instant,
    debounce_duration: Duration,
    pending_content: Option<String>,
}

impl AutoSave {
    /// Create a new auto-save helper with the given debounce duration in milliseconds.
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            last_edit: Instant::now(),
            debounce_duration: Duration::from_millis(debounce_ms),
            pending_content: None,
        }
    }

    /// Mark content as edited, resetting the debounce timer.
    pub fn mark_edited(&mut self, content: String) {
        self.last_edit = Instant::now();
        self.pending_content = Some(content);
    }

    /// Check if enough time has passed to save.
    pub fn should_save(&self) -> bool {
        self.pending_content.is_some() && self.last_edit.elapsed() >= self.debounce_duration
    }

    /// Take the pending content (consuming it).
    pub fn take_pending(&mut self) -> Option<String> {
        self.pending_content.take()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Save tasks to .ferrite/tasks.json in workspace root.
///
/// Uses atomic write pattern (write to .bak, then rename).
pub fn save_tasks(workspace_root: &Path, tasks: &[Task]) -> std::io::Result<()> {
    let ferrite_dir = workspace_root.join(".ferrite");
    std::fs::create_dir_all(&ferrite_dir)?;

    let tasks_path = ferrite_dir.join("tasks.json");
    let backup_path = ferrite_dir.join("tasks.json.bak");

    let json = serde_json::to_string_pretty(tasks)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Atomic write: backup first, then rename
    std::fs::write(&backup_path, &json)?;
    std::fs::rename(&backup_path, &tasks_path)?;

    Ok(())
}

/// Load tasks from .ferrite/tasks.json in workspace root.
///
/// Returns empty Vec if file doesn't exist or is invalid.
/// If JSON is corrupted, creates a backup and returns empty Vec.
pub fn load_tasks(workspace_root: &Path) -> Vec<Task> {
    let tasks_path = workspace_root.join(".ferrite").join("tasks.json");

    if !tasks_path.exists() {
        return Vec::new();
    }

    match std::fs::read_to_string(&tasks_path) {
        Ok(contents) => {
            match serde_json::from_str(&contents) {
                Ok(tasks) => tasks,
                Err(e) => {
                    log::warn!("Failed to parse tasks.json, creating backup: {}", e);
                    // Create backup of corrupted file
                    let backup = tasks_path.with_extension("json.corrupted");
                    let _ = std::fs::rename(&tasks_path, &backup);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to read tasks.json: {}", e);
            Vec::new()
        }
    }
}

/// Save note content to .ferrite/notes/{name}.txt
pub fn save_note(workspace_root: &Path, name: &str, content: &str) -> std::io::Result<()> {
    let notes_dir = workspace_root.join(".ferrite").join("notes");
    std::fs::create_dir_all(&notes_dir)?;

    // Sanitize name to prevent path traversal
    let safe_name = name.replace(['/', '\\'], "_").replace("..", "_");
    let note_path = notes_dir.join(format!("{}.txt", safe_name));
    let backup_path = notes_dir.join(format!("{}.txt.bak", safe_name));

    // Atomic write
    std::fs::write(&backup_path, content)?;
    std::fs::rename(&backup_path, &note_path)?;

    Ok(())
}

/// Load note content from .ferrite/notes/{name}.txt
pub fn load_note(workspace_root: &Path, name: &str) -> String {
    let safe_name = name.replace(['/', '\\'], "_").replace("..", "_");
    let note_path = workspace_root
        .join(".ferrite")
        .join("notes")
        .join(format!("{}.txt", safe_name));

    std::fs::read_to_string(&note_path).unwrap_or_default()
}

/// Delete a note from .ferrite/notes/{name}.txt
pub fn delete_note(workspace_root: &Path, name: &str) -> std::io::Result<()> {
    let safe_name = name.replace(['/', '\\'], "_").replace("..", "_");
    let note_path = workspace_root
        .join(".ferrite")
        .join("notes")
        .join(format!("{}.txt", safe_name));

    if note_path.exists() {
        std::fs::remove_file(&note_path)?;
    }

    Ok(())
}

/// Rename a note from old_name to new_name in .ferrite/notes/
pub fn rename_note(workspace_root: &Path, old_name: &str, new_name: &str) -> std::io::Result<()> {
    let safe_old = old_name.replace(['/', '\\'], "_").replace("..", "_");
    let safe_new = new_name.replace(['/', '\\'], "_").replace("..", "_");

    if safe_new.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            t!("productivity.notes.empty_name").to_string(),
        ));
    }

    let notes_dir = workspace_root.join(".ferrite").join("notes");
    let old_path = notes_dir.join(format!("{}.txt", safe_old));
    let new_path = notes_dir.join(format!("{}.txt", safe_new));

    if new_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            t!("productivity.notes.duplicate_name").to_string(),
        ));
    }

    if old_path.exists() {
        std::fs::rename(&old_path, &new_path)?;
    }

    Ok(())
}

/// List available notes in workspace
pub fn list_notes(workspace_root: &Path) -> Vec<String> {
    let notes_dir = workspace_root.join(".ferrite").join("notes");

    if !notes_dir.exists() {
        return vec!["default".to_string()];
    }

    let mut notes: Vec<String> = std::fs::read_dir(&notes_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();
                    if path.extension()? == "txt" {
                        path.file_stem()?.to_str().map(String::from)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    if notes.is_empty() {
        notes.push("default".to_string());
    }

    notes.sort();
    notes
}

// ─────────────────────────────────────────────────────────────────────────────
// Productivity Panel UI Component
// ─────────────────────────────────────────────────────────────────────────────

/// State for the productivity hub panel.
pub struct ProductivityPanel {
    /// Current workspace root (needed for persistence)
    workspace_root: Option<std::path::PathBuf>,

    /// Task list
    tasks: Vec<Task>,

    /// New task input text
    new_task_input: String,

    /// Pomodoro timer
    timer: PomodoroTimer,

    /// Notes content
    notes_content: String,

    /// Current note name
    current_note: String,

    /// Available notes list
    available_notes: Vec<String>,

    /// Auto-save helper for notes
    auto_save: AutoSave,

    /// Flag to indicate tasks need saving
    tasks_dirty: bool,

    /// Whether we're currently editing a note name (rename mode)
    renaming_note: bool,

    /// Buffer for the new note name during rename
    rename_buffer: String,

    /// Whether a note delete confirmation is pending
    delete_confirming: bool,

    /// Flag set when the user clicks "Dock" in the floating window
    dock_requested: bool,
}

impl ProductivityPanel {
    /// Create a new productivity panel.
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            tasks: Vec::new(),
            new_task_input: String::new(),
            timer: PomodoroTimer::new(),
            notes_content: String::new(),
            current_note: "default".to_string(),
            available_notes: vec!["default".to_string()],
            auto_save: AutoSave::new(1000),
            tasks_dirty: false,
            renaming_note: false,
            rename_buffer: String::new(),
            delete_confirming: false,
            dock_requested: false,
        }
    }

    /// Check if the user requested to dock the panel (and consume the flag).
    pub fn take_dock_request(&mut self) -> bool {
        let requested = self.dock_requested;
        self.dock_requested = false;
        requested
    }

    /// Set the workspace root and load data.
    pub fn set_workspace(&mut self, workspace_root: Option<std::path::PathBuf>) {
        if self.workspace_root != workspace_root {
            // Save current workspace data before switching
            self.save_all();

            self.workspace_root = workspace_root.clone();

            // Load data for new workspace
            if let Some(ref root) = workspace_root {
                self.tasks = load_tasks(root);
                self.available_notes = list_notes(root);
                self.current_note = self
                    .available_notes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                self.notes_content = load_note(root, &self.current_note);
            } else {
                // No workspace - reset to defaults
                self.tasks = Vec::new();
                self.notes_content = String::new();
                self.available_notes = vec!["default".to_string()];
                self.current_note = "default".to_string();
            }

            self.tasks_dirty = false;
        }
    }

    /// Save all pending data.
    pub fn save_all(&mut self) {
        if let Some(ref root) = self.workspace_root {
            // Save tasks if dirty
            if self.tasks_dirty {
                if let Err(e) = save_tasks(root, &self.tasks) {
                    log::warn!("Failed to save tasks: {}", e);
                }
                self.tasks_dirty = false;
            }

            // Save notes if pending
            if let Some(content) = self.auto_save.take_pending() {
                if let Err(e) = save_note(root, &self.current_note, &content) {
                    log::warn!("Failed to save note: {}", e);
                }
            }
        }
    }

    /// Add a new task from the input field.
    fn add_task(&mut self) {
        let input = self.new_task_input.trim();
        if input.is_empty() {
            return;
        }

        // Limit task text to 500 characters
        let text = if input.len() > 500 {
            format!("{}...", &input[..497])
        } else {
            input.to_string()
        };

        // If input already has markdown syntax, parse it
        if let Some(mut task) = Task::from_markdown(&text) {
            // Re-apply length limit to task text if needed
            if task.text.len() > 500 {
                task.text = format!("{}...", &task.text[..497]);
            }
            self.tasks.push(task);
        } else {
            // Otherwise create a simple unchecked task
            self.tasks.push(Task {
                completed: false,
                text,
                priority: 0,
            });
        }

        self.new_task_input.clear();
        self.tasks_dirty = true;
    }

    /// Delete a task by index.
    fn delete_task(&mut self, index: usize) {
        if index < self.tasks.len() {
            self.tasks.remove(index);
            self.tasks_dirty = true;
        }
    }

    /// Render the productivity panel content inline (for docked mode in outline panel).
    ///
    /// Returns true if a repaint is needed (timer active).
    ///
    /// Each subsection (Tasks / Pomodoro / Notes) is wrapped in a themed
    /// "card" frame so the panel reads as cohesive UI rather than a stack of
    /// loose widgets, matching the visual language used elsewhere in the app.
    pub fn show_content(
        &mut self,
        ui: &mut eframe::egui::Ui,
        ctx: &eframe::egui::Context,
        ferrite_accent: eframe::egui::Color32,
    ) -> bool {
        use crate::ui::phosphor_icons::{
            phosphor_rich_text, CARET_DOWN, CARET_UP, CHECK, COFFEE, LIST_CHECKS, NOTE_PENCIL,
            PENCIL, PLAY, PLUS, STOP, TIMER, TRASH, X,
        };
        use eframe::egui::{
            Align, Button, Color32, ComboBox, CornerRadius, Frame, Key, Label, Layout, Margin,
            RichText, ScrollArea, Stroke, TextEdit, Vec2,
        };

        let mut needs_repaint = false;

        let visuals = ui.visuals().clone();
        let is_dark = visuals.dark_mode;
        let header_color = visuals.text_color();
        let muted_color = visuals.weak_text_color();

        let card_bg = if is_dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, 8)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 5)
        };
        let card_border = if is_dark {
            Color32::from_rgb(60, 62, 70)
        } else {
            Color32::from_rgb(218, 222, 228)
        };
        let accent_color = ferrite_accent;
        let success_color = if is_dark {
            Color32::from_rgb(75, 210, 100)
        } else {
            Color32::from_rgb(40, 167, 69)
        };
        let warn_color = if is_dark {
            Color32::from_rgb(255, 195, 80)
        } else {
            Color32::from_rgb(190, 130, 0)
        };
        let danger_color = if is_dark {
            Color32::from_rgb(255, 110, 120)
        } else {
            Color32::from_rgb(220, 53, 69)
        };
        // Alternate row fill: use theme surfaces so striping matches the rest of the app.
        // Dark mode: darken alternate rows (card already lifts with a light overlay).
        // Light mode: faint_bg matches settings panels and other subtle stripes.
        let row_alt_bg = if is_dark {
            visuals.panel_fill
        } else {
            visuals.faint_bg_color
        };
        let completed_task_color = visuals.widgets.inactive.fg_stroke.color;

        // Workspace hint banner: only shown when persistence is unavailable
        if self.workspace_root.is_none() {
            Frame::new()
                .fill(if is_dark {
                    Color32::from_rgba_unmultiplied(255, 200, 80, 18)
                } else {
                    Color32::from_rgba_unmultiplied(255, 200, 80, 36)
                })
                .stroke(Stroke::new(1.0, warn_color))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("ⓘ").size(13.0).strong().color(warn_color));
                        ui.add(
                            Label::new(
                                RichText::new(t!("productivity.workspace_hint").to_string())
                                    .size(11.0)
                                    .color(header_color),
                            )
                            .wrap(),
                        );
                    });
                });
            ui.add_space(8.0);
        }

        // ── TASKS CARD ──────────────────────────────────────────────────
        let completed = self.tasks.iter().filter(|t| t.completed).count();
        let total = self.tasks.len();
        let mut delete_idx: Option<usize> = None;
        let mut move_up_idx: Option<usize> = None;
        let mut move_down_idx: Option<usize> = None;

        Frame::new()
            .fill(card_bg)
            .stroke(Stroke::new(1.0, card_border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                // Section header
                ui.horizontal(|ui| {
                    ui.label(
                        phosphor_rich_text(LIST_CHECKS, 13.0)
                            .strong()
                            .color(header_color),
                    );
                    ui.label(
                        RichText::new(t!("productivity.tasks.title").to_string())
                            .size(13.0)
                            .strong()
                            .color(header_color),
                    );
                    if total > 0 {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let badge_color = if completed == total {
                                success_color
                            } else {
                                muted_color
                            };
                            ui.label(
                                RichText::new(format!("{}/{}", completed, total))
                                    .size(11.0)
                                    .strong()
                                    .color(badge_color),
                            );
                        });
                    }
                });

                ui.add_space(6.0);

                // Input row: text field + Add button
                ui.horizontal(|ui| {
                    let avail = ui.available_width();
                    let response = ui.add(
                        TextEdit::singleline(&mut self.new_task_input)
                            .hint_text(t!("productivity.tasks.input_hint").to_string())
                            .desired_width(avail - 56.0),
                    );
                    let row_h = response.rect.height().max(22.0);
                    let add_clicked = ui
                        .add_sized(
                            [50.0, row_h],
                            Button::new(
                                RichText::new(t!("productivity.tasks.add").to_string())
                                    .size(11.0)
                                    .color(accent_color),
                            ),
                        )
                        .clicked();
                    if add_clicked
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        self.add_task();
                    }
                });

                ui.add(
                    Label::new(
                        RichText::new(t!("productivity.tasks.tip").to_string())
                            .size(10.0)
                            .italics()
                            .color(muted_color),
                    )
                    .wrap(),
                );

                ui.add_space(6.0);

                // Task list
                ScrollArea::vertical()
                    .id_salt("tasks_scroll")
                    .max_height(220.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if self.tasks.is_empty() {
                            ui.add_space(10.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(t!("productivity.tasks.empty").to_string())
                                        .size(11.0)
                                        .italics()
                                        .color(muted_color),
                                );
                                ui.label(
                                    RichText::new(t!("productivity.tasks.add_first").to_string())
                                        .size(10.0)
                                        .color(muted_color),
                                );
                            });
                            ui.add_space(10.0);
                        } else {
                            let tasks_len = self.tasks.len();
                            for (i, task) in self.tasks.iter_mut().enumerate() {
                                let row_bg = if i % 2 == 1 {
                                    row_alt_bg
                                } else {
                                    Color32::TRANSPARENT
                                };
                                Frame::new()
                                    .fill(row_bg)
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(4, 2))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Reorder controls
                                            ui.add_enabled_ui(i > 0, |ui| {
                                                if ui
                                                    .add(
                                                        Button::new(
                                                            phosphor_rich_text(CARET_UP, 9.0)
                                                                .color(muted_color),
                                                        )
                                                        .frame(false)
                                                        .min_size(Vec2::new(14.0, 16.0)),
                                                    )
                                                    .on_hover_text(
                                                        t!("productivity.tasks.move_up")
                                                            .to_string(),
                                                    )
                                                    .clicked()
                                                {
                                                    move_up_idx = Some(i);
                                                }
                                            });
                                            ui.add_enabled_ui(i < tasks_len - 1, |ui| {
                                                if ui
                                                    .add(
                                                        Button::new(
                                                            phosphor_rich_text(CARET_DOWN, 9.0)
                                                                .color(muted_color),
                                                        )
                                                        .frame(false)
                                                        .min_size(Vec2::new(14.0, 16.0)),
                                                    )
                                                    .on_hover_text(
                                                        t!("productivity.tasks.move_down")
                                                            .to_string(),
                                                    )
                                                    .clicked()
                                                {
                                                    move_down_idx = Some(i);
                                                }
                                            });

                                            if ui.checkbox(&mut task.completed, "").changed() {
                                                self.tasks_dirty = true;
                                            }

                                            // Priority chip
                                            match task.priority {
                                                2 => {
                                                    Self::draw_priority_chip(ui, "!!", danger_color)
                                                }
                                                1 => Self::draw_priority_chip(ui, "!", warn_color),
                                                _ => {}
                                            }

                                            // The task text is user input and can be arbitrarily
                                            // long. We must give the label a fixed maximum width
                                            // and let it truncate, otherwise its natural width
                                            // would push the row -> card -> SidePanel wider than
                                            // the user's chosen width (egui stores the content's
                                            // `min_rect` in `PanelState`, so any overflow gets
                                            // baked in for the next frame).
                                            let text = if task.completed {
                                                RichText::new(&task.text)
                                                    .size(12.0)
                                                    .strikethrough()
                                                    .color(completed_task_color)
                                            } else {
                                                RichText::new(&task.text)
                                                    .size(12.0)
                                                    .color(header_color)
                                            };

                                            // Reserve room for the trailing delete button so the
                                            // label has an explicit upper bound to truncate to.
                                            let label_w = (ui.available_width() - 22.0).max(20.0);
                                            ui.add_sized(
                                                [label_w, 18.0],
                                                Label::new(text).truncate(),
                                            );

                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add(
                                                            Button::new(
                                                                phosphor_rich_text(X, 10.0)
                                                                    .color(muted_color),
                                                            )
                                                            .frame(false)
                                                            .min_size(Vec2::new(16.0, 16.0)),
                                                        )
                                                        .on_hover_text(
                                                            t!("productivity.tasks.delete")
                                                                .to_string(),
                                                        )
                                                        .clicked()
                                                    {
                                                        delete_idx = Some(i);
                                                    }
                                                },
                                            );
                                        });
                                    });
                            }
                        }
                    });
            });

        // Apply pending mutations after the borrow on self.tasks ends
        if let Some(i) = move_up_idx {
            if i > 0 {
                self.tasks.swap(i, i - 1);
                self.tasks_dirty = true;
            }
        }
        if let Some(i) = move_down_idx {
            if i + 1 < self.tasks.len() {
                self.tasks.swap(i, i + 1);
                self.tasks_dirty = true;
            }
        }
        if let Some(idx) = delete_idx {
            self.delete_task(idx);
        }

        ui.add_space(8.0);

        // ── POMODORO CARD ───────────────────────────────────────────────
        Frame::new()
            .fill(card_bg)
            .stroke(Stroke::new(1.0, card_border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(phosphor_rich_text(TIMER, 13.0).strong().color(header_color));
                    ui.label(
                        RichText::new(t!("productivity.pomodoro.title").to_string())
                            .size(13.0)
                            .strong()
                            .color(header_color),
                    );
                    if self.timer.cycles() > 0 {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(
                                    t!("productivity.pomodoro.cycles", count = self.timer.cycles())
                                        .to_string(),
                                )
                                .size(11.0)
                                .color(muted_color),
                            );
                        });
                    }
                });

                ui.add_space(6.0);

                // Big centered timer + status label
                let (time_text, status_label, status_color) = if self.timer.is_work() {
                    (
                        self.timer.format_remaining(),
                        t!("productivity.pomodoro_status.work").to_string(),
                        accent_color,
                    )
                } else if self.timer.is_break() {
                    (
                        self.timer.format_remaining(),
                        t!("productivity.pomodoro_status.break_label").to_string(),
                        success_color,
                    )
                } else {
                    (
                        "25:00".to_string(),
                        t!("productivity.pomodoro.ready").to_string(),
                        muted_color,
                    )
                };

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(time_text)
                            .size(34.0)
                            .strong()
                            .monospace()
                            .color(if self.timer.is_active() {
                                status_color
                            } else {
                                header_color
                            }),
                    );
                    ui.label(RichText::new(status_label).size(11.0).color(status_color));
                });

                ui.add_space(8.0);

                // Action buttons (icon + label rendered inside each button)
                ui.horizontal(|ui| {
                    if self.timer.is_active() {
                        if ui
                            .add_sized(
                                [ui.available_width(), 26.0],
                                Button::new(Self::pomodoro_button_label(
                                    STOP,
                                    &t!("productivity.pomodoro.stop").to_string(),
                                    danger_color,
                                )),
                            )
                            .clicked()
                        {
                            self.timer.stop();
                        }

                        ctx.request_repaint_after(Duration::from_secs(1));
                        needs_repaint = true;

                        if self.timer.is_complete() {
                            crate::terminal::play_notification(None);
                            if self.timer.is_work() {
                                self.timer.increment_cycle();
                                self.timer.start_break();
                            } else {
                                self.timer.stop();
                            }
                        }
                    } else {
                        let half_w = (ui.available_width() - 6.0) / 2.0;
                        if ui
                            .add_sized(
                                [half_w, 26.0],
                                Button::new(Self::pomodoro_button_label(
                                    PLAY,
                                    &t!("productivity.pomodoro.start_work").to_string(),
                                    accent_color,
                                )),
                            )
                            .clicked()
                        {
                            self.timer.start_work();
                        }
                        if ui
                            .add_sized(
                                [half_w, 26.0],
                                Button::new(Self::pomodoro_button_label(
                                    COFFEE,
                                    &t!("productivity.pomodoro.start_break").to_string(),
                                    success_color,
                                )),
                            )
                            .clicked()
                        {
                            self.timer.start_break();
                        }
                    }
                });
            });

        ui.add_space(8.0);

        // ── NOTES CARD ──────────────────────────────────────────────────
        Frame::new()
            .fill(card_bg)
            .stroke(Stroke::new(1.0, card_border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        phosphor_rich_text(NOTE_PENCIL, 13.0)
                            .strong()
                            .color(header_color),
                    );
                    ui.label(
                        RichText::new(t!("productivity.notes.title").to_string())
                            .size(13.0)
                            .strong()
                            .color(header_color),
                    );
                });

                ui.add_space(6.0);

                if self.available_notes.len() > 1 || self.workspace_root.is_some() {
                    if self.renaming_note {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(t!("productivity.notes.name_label").to_string())
                                    .size(11.0)
                                    .color(muted_color),
                            );
                            let response = ui.add(
                                TextEdit::singleline(&mut self.rename_buffer)
                                    .desired_width(ui.available_width() - 60.0),
                            );

                            if ui
                                .small_button(phosphor_rich_text(CHECK, 12.0).color(success_color))
                                .on_hover_text(t!("productivity.notes.confirm_rename").to_string())
                                .clicked()
                                || (response.lost_focus()
                                    && ui.input(|i| i.key_pressed(Key::Enter)))
                            {
                                let new_name = self.rename_buffer.trim().to_string();
                                if !new_name.is_empty() && new_name != self.current_note {
                                    if let Some(ref root) = self.workspace_root {
                                        let _ = save_note(
                                            root,
                                            &self.current_note,
                                            &self.notes_content,
                                        );
                                        if let Err(e) =
                                            rename_note(root, &self.current_note, &new_name)
                                        {
                                            log::warn!("Failed to rename note: {}", e);
                                        } else {
                                            if let Some(pos) = self
                                                .available_notes
                                                .iter()
                                                .position(|n| n == &self.current_note)
                                            {
                                                self.available_notes[pos] = new_name.clone();
                                            }
                                            self.current_note = new_name;
                                        }
                                    }
                                }
                                self.renaming_note = false;
                            }

                            if ui
                                .small_button(phosphor_rich_text(X, 12.0).color(muted_color))
                                .on_hover_text(t!("productivity.notes.cancel_rename").to_string())
                                .clicked()
                            {
                                self.renaming_note = false;
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let combo_w = (ui.available_width() - 90.0).max(80.0);
                            ComboBox::from_id_salt("note_selector")
                                .selected_text(RichText::new(&self.current_note).size(11.0))
                                .width(combo_w)
                                .show_ui(ui, |ui| {
                                    for note in &self.available_notes.clone() {
                                        if ui
                                            .selectable_label(self.current_note == *note, note)
                                            .clicked()
                                        {
                                            if let Some(ref root) = self.workspace_root {
                                                if self.auto_save.take_pending().is_some()
                                                    || !self.notes_content.is_empty()
                                                {
                                                    let _ = save_note(
                                                        root,
                                                        &self.current_note,
                                                        &self.notes_content,
                                                    );
                                                }
                                                self.current_note = note.clone();
                                                self.notes_content =
                                                    load_note(root, &self.current_note);
                                            }
                                            self.renaming_note = false;
                                            self.delete_confirming = false;
                                        }
                                    }
                                });

                            // Inline icon actions
                            if ui
                                .add(
                                    Button::new(phosphor_rich_text(PLUS, 11.0).color(accent_color))
                                        .frame(false)
                                        .min_size(Vec2::new(20.0, 20.0)),
                                )
                                .on_hover_text(t!("productivity.notes.new_note").to_string())
                                .clicked()
                            {
                                let new_name = format!("note_{}", self.available_notes.len() + 1);
                                self.available_notes.push(new_name.clone());
                                if let Some(ref root) = self.workspace_root {
                                    let _ =
                                        save_note(root, &self.current_note, &self.notes_content);
                                }
                                self.current_note = new_name;
                                self.notes_content = String::new();
                                self.renaming_note = false;
                                self.delete_confirming = false;
                            }

                            if ui
                                .add(
                                    Button::new(
                                        phosphor_rich_text(PENCIL, 11.0).color(muted_color),
                                    )
                                    .frame(false)
                                    .min_size(Vec2::new(20.0, 20.0)),
                                )
                                .on_hover_text(t!("productivity.notes.rename_note").to_string())
                                .clicked()
                            {
                                self.rename_buffer = self.current_note.clone();
                                self.renaming_note = true;
                                self.delete_confirming = false;
                            }

                            if self.available_notes.len() > 1 {
                                if self.delete_confirming {
                                    if ui
                                        .add(
                                            Button::new(
                                                phosphor_rich_text(CHECK, 11.0).color(danger_color),
                                            )
                                            .min_size(Vec2::new(20.0, 20.0)),
                                        )
                                        .on_hover_text(
                                            t!("productivity.notes.confirm_delete").to_string(),
                                        )
                                        .clicked()
                                    {
                                        if let Some(ref root) = self.workspace_root {
                                            let _ = delete_note(root, &self.current_note);
                                            self.available_notes
                                                .retain(|n| n != &self.current_note);
                                            self.current_note = self
                                                .available_notes
                                                .first()
                                                .cloned()
                                                .unwrap_or_else(|| "default".to_string());
                                            self.notes_content =
                                                load_note(root, &self.current_note);
                                        }
                                        self.delete_confirming = false;
                                    }
                                } else if ui
                                    .add(
                                        Button::new(
                                            phosphor_rich_text(TRASH, 11.0).color(muted_color),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(20.0, 20.0)),
                                    )
                                    .on_hover_text(t!("productivity.notes.delete_note").to_string())
                                    .clicked()
                                {
                                    self.delete_confirming = true;
                                    self.renaming_note = false;
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                }

                // Bound the textarea to the currently available width.
                // Using `f32::INFINITY` here causes the host (Window or
                // SidePanel) to keep growing because `Resize` snaps its
                // desired size to the content size each frame.
                let avail_w = ui.available_width();
                let response = ui.add(
                    TextEdit::multiline(&mut self.notes_content)
                        .desired_rows(8)
                        .hint_text(t!("productivity.notes.input_hint").to_string())
                        .desired_width(avail_w),
                );

                if response.changed() {
                    self.auto_save.mark_edited(self.notes_content.clone());
                }

                if self.auto_save.should_save() {
                    if let (Some(ref root), Some(content)) =
                        (&self.workspace_root, self.auto_save.take_pending())
                    {
                        if let Err(e) = save_note(root, &self.current_note, &content) {
                            log::warn!("Failed to auto-save note: {}", e);
                        }
                    }
                }
            });

        // Persist task changes (debounced by frame rate)
        if self.tasks_dirty {
            if let Some(ref root) = self.workspace_root {
                if let Err(e) = save_tasks(root, &self.tasks) {
                    log::warn!("Failed to save tasks: {}", e);
                }
                self.tasks_dirty = false;
            }
        }

        needs_repaint
    }

    /// Build button label text with a Phosphor icon glyph followed by caption text.
    fn pomodoro_button_label(icon: &str, label: &str, color: eframe::egui::Color32) -> eframe::egui::text::LayoutJob {
        use crate::ui::icons::phosphor_font;
        use eframe::egui::text::{LayoutJob, TextFormat};
        use eframe::egui::FontId;

        let mut job = LayoutJob::default();
        job.append(
            icon,
            0.0,
            TextFormat {
                font_id: phosphor_font(11.0),
                color,
                ..Default::default()
            },
        );
        job.append(
            &format!(" {label}"),
            0.0,
            TextFormat {
                font_id: FontId::proportional(11.0),
                color,
                ..Default::default()
            },
        );
        job
    }

    /// Draw a small colored chip used for task priority indicators.
    fn draw_priority_chip(ui: &mut eframe::egui::Ui, label: &str, color: eframe::egui::Color32) {
        use eframe::egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke};
        let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 32);
        Frame::new()
            .fill(bg)
            .stroke(Stroke::new(1.0, color))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin::symmetric(4, 1))
            .show(ui, |ui| {
                ui.label(RichText::new(label).size(9.0).strong().color(color));
            });
    }

    /// Render the productivity panel as a floating window (detached mode).
    ///
    /// `dock_width` is the current width of the docked outline sidebar; the
    /// floating window opens at that width on the *first* detach so the
    /// transition from docked to floating doesn't cause a visual jump. After
    /// that, egui persists the user's manual resize.
    ///
    /// Returns true if the panel requested a repaint (timer active).
    ///
    /// Closing the window with the title-bar `X` re-docks the panel into the
    /// outline sidebar instead of hiding it entirely. This mirrors the explicit
    /// `Dock` button so the panel never becomes inaccessible after detaching.
    pub fn show(
        &mut self,
        ctx: &eframe::egui::Context,
        visible: &mut bool,
        dock_width: f32,
        ferrite_accent: eframe::egui::Color32,
    ) -> bool {
        use eframe::egui::{self, Layout, RichText, Vec2};

        let was_visible = *visible;
        let mut needs_repaint = false;

        let _is_dark = ctx.global_style().visuals.dark_mode;
        let muted_color = ctx.global_style().visuals.weak_text_color();

        let viewport = crate::ui::window::viewport_window_rect(ctx);
        let initial_w = dock_width.clamp(220.0, viewport.width() - 32.0);
        let max_w = (viewport.width() - 16.0).max(220.0);
        let max_h = (viewport.height() - 48.0).max(200.0);
        let default_h = 420.0_f32.min(max_h);

        egui::Window::new(t!("productivity.title").to_string())
            .id(egui::Id::new("productivity_hub_floating"))
            .open(visible)
            .fade_in(false)
            .fade_out(false)
            .default_size([initial_w, default_h])
            .min_size([220.0, 200.0])
            .max_size([max_w, max_h])
            .resizable(true)
            .show(ctx, |ui| {
                // Action bar with the explicit Dock button. The window's `X`
                // achieves the same result via the dock-on-close logic below.
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let dock_btn = ui.add_sized(
                            Vec2::new(64.0, 22.0),
                            egui::Button::new(
                                RichText::new(format!("⤵ {}", t!("productivity.notes.dock")))
                                    .size(11.0)
                                    .color(ferrite_accent),
                            ),
                        );
                        if dock_btn
                            .on_hover_text(t!("productivity.notes.dock_tooltip").to_string())
                            .clicked()
                        {
                            self.dock_requested = true;
                        }
                        ui.label(
                            RichText::new(t!("productivity.notes.close_hint").to_string())
                                .size(10.0)
                                .italics()
                                .color(muted_color),
                        );
                    });
                });

                ui.add_space(2.0);

                // Clip content to the window's inner rect so wide widgets cannot
                // force `Resize` to expand back out (felt like an animated snap).
                let avail_w = ui.available_width();
                let avail_h = ui.available_height();
                let (content_rect, _) =
                    ui.allocate_exact_size(Vec2::new(avail_w, avail_h), egui::Sense::hover());
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child_ui.set_clip_rect(content_rect);
                child_ui.set_max_width(avail_w);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(&mut child_ui, |ui| {
                        ui.set_max_width(avail_w);
                        needs_repaint = self.show_content(ui, ctx, ferrite_accent);
                    });
            });

        // The window was just closed via the title-bar X. Treat this as a dock
        // request so the panel re-attaches to the outline sidebar instead of
        // becoming unreachable until the user invokes the shortcut again.
        if was_visible && !*visible {
            self.save_all();
            self.dock_requested = true;
        }

        needs_repaint
    }
}

impl Default for ProductivityPanel {
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
    use std::time::Duration;

    // Task parsing tests
    #[test]
    fn test_task_from_markdown_unchecked() {
        let task = Task::from_markdown("- [ ] Buy milk").unwrap();
        assert!(!task.completed);
        assert_eq!(task.text, "Buy milk");
        assert_eq!(task.priority, 0);
    }

    #[test]
    fn test_task_from_markdown_checked() {
        let task = Task::from_markdown("- [x] Done task").unwrap();
        assert!(task.completed);
        assert_eq!(task.text, "Done task");
    }

    #[test]
    fn test_task_from_markdown_priority_high() {
        let task = Task::from_markdown("- [ ] !! Urgent").unwrap();
        assert_eq!(task.priority, 2);
        assert_eq!(task.text, "Urgent");
    }

    #[test]
    fn test_task_from_markdown_priority_medium() {
        let task = Task::from_markdown("- [ ] ! Important").unwrap();
        assert_eq!(task.priority, 1);
        assert_eq!(task.text, "Important");
    }

    #[test]
    fn test_task_from_markdown_invalid() {
        assert!(Task::from_markdown("Not a task").is_none());
        assert!(Task::from_markdown("- Regular list item").is_none());
        assert!(Task::from_markdown("[ ] Missing dash").is_none());
    }

    #[test]
    fn test_task_to_markdown() {
        let task = Task {
            completed: false,
            text: "Test".to_string(),
            priority: 0,
        };
        assert_eq!(task.to_markdown(), "- [ ] Test");

        let task = Task {
            completed: true,
            text: "Done".to_string(),
            priority: 0,
        };
        assert_eq!(task.to_markdown(), "- [x] Done");

        let task = Task {
            completed: false,
            text: "Urgent".to_string(),
            priority: 2,
        };
        assert_eq!(task.to_markdown(), "- [ ] !! Urgent");
    }

    // Pomodoro timer tests
    #[test]
    fn test_timer_initial_state() {
        let timer = PomodoroTimer::new();
        assert!(!timer.is_active());
        assert!(timer.remaining().is_none());
    }

    #[test]
    fn test_timer_work_session() {
        let mut timer = PomodoroTimer::new();
        timer.start_work();

        assert!(timer.is_active());
        assert!(timer.is_work());
        assert!(!timer.is_break());

        // Should have ~25 minutes remaining (allow small tolerance)
        let remaining = timer.remaining().unwrap();
        assert!(remaining.as_secs() > 24 * 60);
        assert!(remaining.as_secs() <= 25 * 60);
    }

    #[test]
    fn test_timer_format() {
        let mut timer = PomodoroTimer::new();
        timer.start_work();

        let formatted = timer.format_remaining();
        // Should be like "24:59" or "25:00"
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 5);
    }

    #[test]
    fn test_timer_stop() {
        let mut timer = PomodoroTimer::new();
        timer.start_work();
        assert!(timer.is_active());

        timer.stop();
        assert!(!timer.is_active());
    }

    // AutoSave tests
    #[test]
    fn test_autosave_initial() {
        let autosave = AutoSave::new(1000);
        assert!(!autosave.should_save());
    }

    #[test]
    fn test_autosave_mark_edited() {
        let mut autosave = AutoSave::new(10); // 10ms for testing
        autosave.mark_edited("test content".to_string());

        // Immediately after edit, should not save (debounce)
        // Note: This might pass due to timing, so we just check pending exists
        assert!(autosave.pending_content.is_some());
    }

    #[test]
    fn test_autosave_take_pending() {
        let mut autosave = AutoSave::new(1000);
        autosave.mark_edited("content".to_string());

        // Manually trigger the save check
        autosave.last_edit = std::time::Instant::now() - Duration::from_secs(2);

        assert!(autosave.should_save());
        let content = autosave.take_pending();
        assert_eq!(content, Some("content".to_string()));
        assert!(autosave.pending_content.is_none());
    }
}
