//! Vim modal editing support for FerriteEditor.
//!
//! Provides Normal/Insert/Visual mode keybindings with mode indicator.
//! Activated via the `vim_mode` setting. When disabled, the editor
//! uses standard (non-modal) keybindings.

use egui::Key;

use super::buffer::TextBuffer;
use super::cursor::{Cursor, Selection};
use super::input::{InputHandler, InputResult};
use super::view::ViewState;

/// Active Vim editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl VimMode {
    /// Display label for the status bar indicator.
    pub fn label(&self) -> &'static str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::VisualLine => "V-LINE",
        }
    }
}

/// Pending operator in Normal mode (e.g. `d` waiting for a motion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOperator {
    Delete,
    Yank,
    Change,
}

/// Persistent Vim state across frames.
#[derive(Debug, Clone)]
pub struct VimState {
    pub mode: VimMode,
    /// Yank register (clipboard internal to vim).
    pub yank_register: String,
    /// Whether the yanked content is line-wise (dd/yy/p behaviour).
    pub yank_linewise: bool,
    /// Pending operator waiting for a motion key.
    pending_op: Option<PendingOperator>,
    /// Numeric repeat prefix (e.g. `3j` = move down 3 lines).
    repeat_count: Option<usize>,
}

impl Default for VimState {
    fn default() -> Self {
        Self {
            mode: VimMode::Normal,
            yank_register: String::new(),
            yank_linewise: false,
            pending_op: None,
            repeat_count: None,
        }
    }
}

/// Result of processing a Vim key event.
pub enum VimKeyResult {
    /// The key was consumed and produced a buffer/cursor change.
    Handled(InputResult),
    /// The key should be forwarded to normal (Insert-mode) input handling.
    Passthrough,
    /// The key was consumed but produced no visible change (e.g. mode switch feedback).
    Consumed,
}

impl VimState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns effective repeat count (at least 1), then resets it.
    fn take_count(&mut self) -> usize {
        self.repeat_count.take().unwrap_or(1)
    }

    /// Process a key event in the current Vim mode.
    ///
    /// Returns how the event should be handled by the caller.
    pub fn handle_key(
        &mut self,
        key: Key,
        modifiers: &egui::Modifiers,
        buffer: &mut TextBuffer,
        selection: &mut Selection,
        view: &mut ViewState,
    ) -> VimKeyResult {
        match self.mode {
            VimMode::Insert => self.handle_insert_mode(key, modifiers),
            VimMode::Normal => self.handle_normal_mode(key, modifiers, buffer, selection, view),
            VimMode::Visual | VimMode::VisualLine => {
                self.handle_visual_mode(key, modifiers, buffer, selection, view)
            }
        }
    }

    /// Process text input (character insertion).
    /// Returns true if the text should be inserted (Insert mode),
    /// false if it should be suppressed (Normal/Visual mode).
    pub fn should_insert_text(&self) -> bool {
        self.mode == VimMode::Insert
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert Mode
    // ─────────────────────────────────────────────────────────────────────────

    fn handle_insert_mode(&mut self, key: Key, _modifiers: &egui::Modifiers) -> VimKeyResult {
        if key == Key::Escape {
            self.mode = VimMode::Normal;
            self.pending_op = None;
            self.repeat_count = None;
            return VimKeyResult::Consumed;
        }
        VimKeyResult::Passthrough
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Normal Mode
    // ─────────────────────────────────────────────────────────────────────────

    fn handle_normal_mode(
        &mut self,
        key: Key,
        modifiers: &egui::Modifiers,
        buffer: &mut TextBuffer,
        selection: &mut Selection,
        _view: &mut ViewState,
    ) -> VimKeyResult {
        // Numeric prefix accumulation (1-9 start, 0 only extends)
        if let Some(digit) = key_to_digit(key) {
            if digit != 0 || self.repeat_count.is_some() {
                let current = self.repeat_count.unwrap_or(0);
                self.repeat_count = Some(current * 10 + digit);
                return VimKeyResult::Consumed;
            }
        }

        let count = self.take_count();

        // Handle pending operator + motion
        if let Some(op) = self.pending_op.take() {
            return self.handle_operator_motion(op, key, count, buffer, selection);
        }

        match key {
            // ── Mode switches ────────────────────────────────────────────
            Key::I if !modifiers.shift => {
                self.mode = VimMode::Insert;
                VimKeyResult::Consumed
            }
            Key::I if modifiers.shift => {
                // I (shift) → insert at line start
                selection.head.column = 0;
                *selection = Selection::collapsed(selection.head);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::A if !modifiers.shift => {
                // a → append after cursor
                let line_len = InputHandler::line_length(buffer, selection.head.line);
                if selection.head.column < line_len {
                    selection.head.column += 1;
                    *selection = Selection::collapsed(selection.head);
                }
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::A if modifiers.shift => {
                // A → append at end of line
                selection.head.column = InputHandler::line_length(buffer, selection.head.line);
                *selection = Selection::collapsed(selection.head);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::O if !modifiers.shift => {
                // o → open line below
                let line_len = InputHandler::line_length(buffer, selection.head.line);
                let mut cursor = Cursor {
                    line: selection.head.line,
                    column: line_len,
                };
                let pos = InputHandler::cursor_to_char_pos(buffer, &cursor);
                buffer.insert(pos, "\n");
                cursor.line += 1;
                cursor.column = 0;
                *selection = Selection::collapsed(cursor);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            Key::O if modifiers.shift => {
                // O → open line above
                let mut cursor = Cursor {
                    line: selection.head.line,
                    column: 0,
                };
                let pos = InputHandler::cursor_to_char_pos(buffer, &cursor);
                buffer.insert(pos, "\n");
                cursor.column = 0;
                *selection = Selection::collapsed(cursor);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            Key::V if !modifiers.shift => {
                // v → visual mode (character-wise)
                self.mode = VimMode::Visual;
                // Anchor at current position
                *selection = Selection {
                    anchor: selection.head,
                    head: selection.head,
                };
                VimKeyResult::Consumed
            }
            Key::V if modifiers.shift => {
                // V → visual line mode
                self.mode = VimMode::VisualLine;
                let line = selection.head.line;
                let line_end = InputHandler::line_length(buffer, line);
                *selection = Selection {
                    anchor: Cursor { line, column: 0 },
                    head: Cursor {
                        line,
                        column: line_end,
                    },
                };
                VimKeyResult::Consumed
            }

            // ── Motion keys ──────────────────────────────────────────────
            Key::H => {
                for _ in 0..count {
                    if selection.head.column > 0 {
                        selection.head.column -= 1;
                    }
                }
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::J => {
                let last_line = buffer.line_count().saturating_sub(1);
                for _ in 0..count {
                    if selection.head.line < last_line {
                        selection.head.line += 1;
                    }
                }
                clamp_column(buffer, &mut selection.head);
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::K => {
                for _ in 0..count {
                    if selection.head.line > 0 {
                        selection.head.line -= 1;
                    }
                }
                clamp_column(buffer, &mut selection.head);
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::L => {
                let line_len = InputHandler::line_length(buffer, selection.head.line);
                for _ in 0..count {
                    if selection.head.column < line_len {
                        selection.head.column += 1;
                    }
                }
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::W => {
                // w → word forward
                for _ in 0..count {
                    move_word_forward(buffer, &mut selection.head);
                }
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::B => {
                // b → word backward
                for _ in 0..count {
                    move_word_backward(buffer, &mut selection.head);
                }
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }

            // 0 → beginning of line (only when no repeat count pending)
            Key::Num0 => {
                selection.head.column = 0;
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }

            // $ → end of line (egui doesn't have a $ key, use End)
            Key::End => {
                selection.head.column = InputHandler::line_length(buffer, selection.head.line);
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }

            // G → go to last line / gg → go to first line
            Key::G if modifiers.shift => {
                let last_line = buffer.line_count().saturating_sub(1);
                selection.head.line = last_line;
                clamp_column(buffer, &mut selection.head);
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::CursorMoved)
            }

            // ── Operators ────────────────────────────────────────────────
            Key::D => {
                if modifiers.shift {
                    // D → delete to end of line
                    let line_len = InputHandler::line_length(buffer, selection.head.line);
                    if selection.head.column < line_len {
                        let start = InputHandler::cursor_to_char_pos(buffer, &selection.head);
                        let end_cursor = Cursor {
                            line: selection.head.line,
                            column: line_len,
                        };
                        let end = InputHandler::cursor_to_char_pos(buffer, &end_cursor);
                        self.yank_register = buffer.slice(start, end);
                        self.yank_linewise = false;
                        buffer.remove(start, end - start);
                    }
                    VimKeyResult::Handled(InputResult::TextChanged)
                } else {
                    self.pending_op = Some(PendingOperator::Delete);
                    self.repeat_count = if count > 1 { Some(count) } else { None };
                    VimKeyResult::Consumed
                }
            }
            Key::Y => {
                if modifiers.shift {
                    // Y → yank to end of line
                    let line_len = InputHandler::line_length(buffer, selection.head.line);
                    let start = InputHandler::cursor_to_char_pos(buffer, &selection.head);
                    let end_cursor = Cursor {
                        line: selection.head.line,
                        column: line_len,
                    };
                    let end = InputHandler::cursor_to_char_pos(buffer, &end_cursor);
                    self.yank_register = buffer.slice(start, end);
                    self.yank_linewise = false;
                    VimKeyResult::Consumed
                } else {
                    self.pending_op = Some(PendingOperator::Yank);
                    self.repeat_count = if count > 1 { Some(count) } else { None };
                    VimKeyResult::Consumed
                }
            }
            Key::C => {
                if !modifiers.ctrl && !modifiers.command {
                    self.pending_op = Some(PendingOperator::Change);
                    self.repeat_count = if count > 1 { Some(count) } else { None };
                    VimKeyResult::Consumed
                } else {
                    VimKeyResult::Passthrough
                }
            }

            // ── Put (paste) ──────────────────────────────────────────────
            Key::P if !modifiers.shift => {
                if !self.yank_register.is_empty() {
                    let result = self.put_after(buffer, selection);
                    return VimKeyResult::Handled(result);
                }
                VimKeyResult::Consumed
            }
            Key::P if modifiers.shift => {
                if !self.yank_register.is_empty() {
                    let result = self.put_before(buffer, selection);
                    return VimKeyResult::Handled(result);
                }
                VimKeyResult::Consumed
            }

            // ── Undo/Redo (passthrough to normal Ctrl+Z/Y handling) ──────
            Key::U => VimKeyResult::Consumed, // TODO: wire to undo

            // ── x → delete char under cursor ─────────────────────────────
            Key::X if !modifiers.shift => {
                for _ in 0..count {
                    let line_len = InputHandler::line_length(buffer, selection.head.line);
                    if selection.head.column < line_len {
                        let pos = InputHandler::cursor_to_char_pos(buffer, &selection.head);
                        self.yank_register = buffer.slice(pos, pos + 1);
                        self.yank_linewise = false;
                        buffer.remove(pos, 1);
                    }
                }
                clamp_column(buffer, &mut selection.head);
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Handled(InputResult::TextChanged)
            }

            // ── Escape resets pending state ──────────────────────────────
            Key::Escape => {
                self.pending_op = None;
                self.repeat_count = None;
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Consumed
            }

            _ => VimKeyResult::Consumed,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Operator + Motion (dd, yy, cc, dw, etc.)
    // ─────────────────────────────────────────────────────────────────────────

    fn handle_operator_motion(
        &mut self,
        op: PendingOperator,
        key: Key,
        count: usize,
        buffer: &mut TextBuffer,
        selection: &mut Selection,
    ) -> VimKeyResult {
        match (op, key) {
            // dd → delete line(s)
            (PendingOperator::Delete, Key::D) => {
                self.delete_lines(count, buffer, selection);
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            // yy → yank line(s)
            (PendingOperator::Yank, Key::Y) => {
                self.yank_lines(count, buffer, selection);
                VimKeyResult::Consumed
            }
            // cc → change line(s)
            (PendingOperator::Change, Key::C) => {
                self.delete_lines(count, buffer, selection);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            // dw → delete word
            (PendingOperator::Delete, Key::W) => {
                for _ in 0..count {
                    self.delete_word_forward(buffer, selection);
                }
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            // cw → change word
            (PendingOperator::Change, Key::W) => {
                for _ in 0..count {
                    self.delete_word_forward(buffer, selection);
                }
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            _ => {
                // Unknown motion — discard the operator
                VimKeyResult::Consumed
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Visual Mode
    // ─────────────────────────────────────────────────────────────────────────

    fn handle_visual_mode(
        &mut self,
        key: Key,
        modifiers: &egui::Modifiers,
        buffer: &mut TextBuffer,
        selection: &mut Selection,
        _view: &mut ViewState,
    ) -> VimKeyResult {
        match key {
            Key::Escape => {
                self.mode = VimMode::Normal;
                *selection = Selection::collapsed(selection.head);
                VimKeyResult::Consumed
            }
            // Motion extends selection
            Key::H => {
                if selection.head.column > 0 {
                    selection.head.column -= 1;
                }
                if self.mode == VimMode::VisualLine {
                    expand_to_full_lines(buffer, selection);
                }
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::J => {
                let last_line = buffer.line_count().saturating_sub(1);
                if selection.head.line < last_line {
                    selection.head.line += 1;
                    clamp_column(buffer, &mut selection.head);
                }
                if self.mode == VimMode::VisualLine {
                    expand_to_full_lines(buffer, selection);
                }
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::K => {
                if selection.head.line > 0 {
                    selection.head.line -= 1;
                    clamp_column(buffer, &mut selection.head);
                }
                if self.mode == VimMode::VisualLine {
                    expand_to_full_lines(buffer, selection);
                }
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            Key::L => {
                let line_len = InputHandler::line_length(buffer, selection.head.line);
                if selection.head.column < line_len {
                    selection.head.column += 1;
                }
                if self.mode == VimMode::VisualLine {
                    expand_to_full_lines(buffer, selection);
                }
                VimKeyResult::Handled(InputResult::CursorMoved)
            }
            // d → delete selection
            Key::D | Key::X => {
                self.delete_selection(buffer, selection);
                self.mode = VimMode::Normal;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            // y → yank selection
            Key::Y => {
                self.yank_selection(buffer, selection);
                self.mode = VimMode::Normal;
                *selection = Selection::collapsed(selection.start_pos());
                VimKeyResult::Consumed
            }
            // c → change (delete + insert mode)
            Key::C if !modifiers.ctrl && !modifiers.command => {
                self.delete_selection(buffer, selection);
                self.mode = VimMode::Insert;
                VimKeyResult::Handled(InputResult::TextChanged)
            }
            _ => VimKeyResult::Consumed,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line operations
    // ─────────────────────────────────────────────────────────────────────────

    fn delete_lines(&mut self, count: usize, buffer: &mut TextBuffer, selection: &mut Selection) {
        let start_line = selection.head.line;
        let last_line = buffer.line_count().saturating_sub(1);
        let end_line = (start_line + count - 1).min(last_line);

        // Build yanked text
        let mut yanked = String::new();
        for line_idx in start_line..=end_line {
            if let Some(line_text) = buffer.get_line(line_idx) {
                yanked.push_str(line_text.trim_end_matches('\n'));
            }
            if line_idx < end_line {
                yanked.push('\n');
            }
        }
        self.yank_register = yanked;
        self.yank_linewise = true;

        // Calculate char range to delete
        let start_char = buffer.line_to_char(start_line);
        let end_char = if end_line + 1 < buffer.line_count() {
            buffer.line_to_char(end_line + 1)
        } else if start_line > 0 {
            // Deleting last line(s): include preceding newline
            let buf_len = buffer.len();
            let remove_from = buffer.line_to_char(start_line);
            let _ = remove_from;
            buf_len
        } else {
            buffer.len()
        };

        let remove_start = if end_line + 1 >= buffer.line_count() && start_line > 0 {
            // Remove preceding newline too (we're deleting through the end)
            let prev_line_end = buffer.line_to_char(start_line);
            prev_line_end.saturating_sub(1)
        } else {
            start_char
        };

        if end_char > remove_start {
            buffer.remove(remove_start, end_char - remove_start);
        }

        // Position cursor
        let cursor_line = start_line.min(buffer.line_count().saturating_sub(1));
        let cursor = Cursor {
            line: cursor_line,
            column: 0,
        };
        *selection = Selection::collapsed(cursor);
    }

    fn yank_lines(&mut self, count: usize, buffer: &TextBuffer, selection: &Selection) {
        let start_line = selection.head.line;
        let last_line = buffer.line_count().saturating_sub(1);
        let end_line = (start_line + count - 1).min(last_line);

        let mut yanked = String::new();
        for line_idx in start_line..=end_line {
            if let Some(line_text) = buffer.get_line(line_idx) {
                yanked.push_str(line_text.trim_end_matches('\n'));
            }
            if line_idx < end_line {
                yanked.push('\n');
            }
        }
        self.yank_register = yanked;
        self.yank_linewise = true;
    }

    fn delete_word_forward(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) {
        let start = selection.head;
        let mut end = start;
        move_word_forward(buffer, &mut end);
        let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
        let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
        if end_pos > start_pos {
            self.yank_register = buffer.slice(start_pos, end_pos);
            self.yank_linewise = false;
            buffer.remove(start_pos, end_pos - start_pos);
        }
        *selection = Selection::collapsed(start);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Selection operations (Visual mode)
    // ─────────────────────────────────────────────────────────────────────────

    fn delete_selection(&mut self, buffer: &mut TextBuffer, selection: &mut Selection) {
        let (start, end) = selection.ordered();
        let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
        let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
        if end_pos > start_pos {
            self.yank_register = buffer.slice(start_pos, end_pos);
            self.yank_linewise = self.mode == VimMode::VisualLine;
            buffer.remove(start_pos, end_pos - start_pos);
        }
        *selection = Selection::collapsed(start);
    }

    fn yank_selection(&mut self, buffer: &TextBuffer, selection: &Selection) {
        let (start, end) = selection.ordered();
        let start_pos = InputHandler::cursor_to_char_pos(buffer, &start);
        let end_pos = InputHandler::cursor_to_char_pos(buffer, &end);
        if end_pos > start_pos {
            self.yank_register = buffer.slice(start_pos, end_pos);
            self.yank_linewise = self.mode == VimMode::VisualLine;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Put (paste) operations
    // ─────────────────────────────────────────────────────────────────────────

    fn put_after(&self, buffer: &mut TextBuffer, selection: &mut Selection) -> InputResult {
        if self.yank_linewise {
            let line = selection.head.line;
            let last_line = buffer.line_count().saturating_sub(1);
            let insert_pos = if line >= last_line {
                // At last line: append newline + text at end
                let end_col = InputHandler::line_length(buffer, line);
                let pos = InputHandler::cursor_to_char_pos(
                    buffer,
                    &Cursor {
                        line,
                        column: end_col,
                    },
                );
                let text = format!("\n{}", self.yank_register);
                buffer.insert(pos, &text);
                selection.head.line = line + 1;
                selection.head.column = 0;
                *selection = Selection::collapsed(selection.head);
                return InputResult::TextChanged;
            } else {
                let next_line = Cursor {
                    line: line + 1,
                    column: 0,
                };
                InputHandler::cursor_to_char_pos(buffer, &next_line)
            };
            let text = format!("{}\n", self.yank_register);
            buffer.insert(insert_pos, &text);
            selection.head.line = line + 1;
            selection.head.column = 0;
            *selection = Selection::collapsed(selection.head);
        } else {
            let line_len = InputHandler::line_length(buffer, selection.head.line);
            if selection.head.column < line_len {
                selection.head.column += 1;
            }
            let pos = InputHandler::cursor_to_char_pos(buffer, &selection.head);
            buffer.insert(pos, &self.yank_register);
            *selection = Selection::collapsed(selection.head);
        }
        InputResult::TextChanged
    }

    fn put_before(&self, buffer: &mut TextBuffer, selection: &mut Selection) -> InputResult {
        if self.yank_linewise {
            let line = selection.head.line;
            let insert_cursor = Cursor { line, column: 0 };
            let pos = InputHandler::cursor_to_char_pos(buffer, &insert_cursor);
            let text = format!("{}\n", self.yank_register);
            buffer.insert(pos, &text);
            selection.head.line = line;
            selection.head.column = 0;
            *selection = Selection::collapsed(selection.head);
        } else {
            let pos = InputHandler::cursor_to_char_pos(buffer, &selection.head);
            buffer.insert(pos, &self.yank_register);
            *selection = Selection::collapsed(selection.head);
        }
        InputResult::TextChanged
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

fn key_to_digit(key: Key) -> Option<usize> {
    match key {
        Key::Num0 => Some(0),
        Key::Num1 => Some(1),
        Key::Num2 => Some(2),
        Key::Num3 => Some(3),
        Key::Num4 => Some(4),
        Key::Num5 => Some(5),
        Key::Num6 => Some(6),
        Key::Num7 => Some(7),
        Key::Num8 => Some(8),
        Key::Num9 => Some(9),
        _ => None,
    }
}

fn clamp_column(buffer: &TextBuffer, cursor: &mut Cursor) {
    let line_len = InputHandler::line_length(buffer, cursor.line);
    if cursor.column > line_len {
        cursor.column = line_len;
    }
}

fn move_word_forward(buffer: &TextBuffer, cursor: &mut Cursor) {
    let line_count = buffer.line_count();
    let line_len = InputHandler::line_length(buffer, cursor.line);

    if cursor.column >= line_len {
        if cursor.line + 1 < line_count {
            cursor.line += 1;
            cursor.column = 0;
        }
        return;
    }

    let line_text = buffer.line(cursor.line);
    let chars: Vec<char> = line_text.chars().collect();
    let mut col = cursor.column;

    // Skip current word characters
    while col < chars.len() && !chars[col].is_whitespace() {
        col += 1;
    }
    // Skip whitespace
    while col < chars.len() && chars[col].is_whitespace() {
        col += 1;
    }

    cursor.column = col;
}

fn move_word_backward(buffer: &TextBuffer, cursor: &mut Cursor) {
    if cursor.column == 0 {
        if cursor.line > 0 {
            cursor.line -= 1;
            cursor.column = InputHandler::line_length(buffer, cursor.line);
        }
        return;
    }

    let line_text = buffer.line(cursor.line);
    let chars: Vec<char> = line_text.chars().collect();
    let mut col = cursor.column;

    // Skip whitespace backward
    while col > 0 && chars[col - 1].is_whitespace() {
        col -= 1;
    }
    // Skip word characters backward
    while col > 0 && !chars[col - 1].is_whitespace() {
        col -= 1;
    }

    cursor.column = col;
}

fn expand_to_full_lines(buffer: &TextBuffer, selection: &mut Selection) {
    let (start, end) = selection.ordered();
    let start_col = 0;
    let end_col = InputHandler::line_length(buffer, end.line);
    selection.anchor = if selection.anchor.line <= selection.head.line {
        Cursor {
            line: start.line,
            column: start_col,
        }
    } else {
        Cursor {
            line: start.line,
            column: end_col,
        }
    };
    selection.head = if selection.head.line >= selection.anchor.line {
        Cursor {
            line: end.line,
            column: end_col,
        }
    } else {
        Cursor {
            line: end.line,
            column: start_col,
        }
    };
}
