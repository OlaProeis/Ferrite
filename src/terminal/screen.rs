//! Terminal screen buffer for terminal emulation.
//!
//! This module provides a character grid with color and attribute support,
//! cursor tracking, and scrollback buffer for terminal history.

use std::collections::VecDeque;
use unicode_width::UnicodeWidthChar;

/// ANSI color representation supporting 16, 256, and true color modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Default terminal color (foreground or background)
    Default,
    /// Standard 16-color palette (0-15)
    Indexed(u8),
    /// 24-bit RGB color
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

impl Color {
    /// Convert to egui color with theme awareness.
    pub fn to_egui(&self, is_foreground: bool, ansi_colors: &[eframe::egui::Color32; 16], default_fg: eframe::egui::Color32, _default_bg: eframe::egui::Color32) -> eframe::egui::Color32 {
        match self {
            Color::Default => {
                if is_foreground {
                    default_fg
                } else {
                    eframe::egui::Color32::TRANSPARENT
                }
            }
            Color::Indexed(idx) => Self::indexed_to_rgb(*idx, ansi_colors),
            Color::Rgb(r, g, b) => eframe::egui::Color32::from_rgb(*r, *g, *b),
        }
    }

    /// Convert 256-color index to RGB.
    fn indexed_to_rgb(idx: u8, ansi_colors: &[eframe::egui::Color32; 16]) -> eframe::egui::Color32 {
        if idx < 16 {
            return ansi_colors[idx as usize];
        }

        // 216 color cube (16-231)
        if idx < 232 {
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let to_component = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
            return eframe::egui::Color32::from_rgb(
                to_component(r),
                to_component(g),
                to_component(b),
            );
        }

        // Grayscale (232-255)
        let gray = 8 + (idx - 232) * 10;
        eframe::egui::Color32::from_rgb(gray, gray, gray)
    }
}

/// Cell attributes for text styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAttributes {
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Underlined text
    pub underline: bool,
    /// Strikethrough text
    pub strikethrough: bool,
    /// Dim/faint text
    pub dim: bool,
    /// Reverse video (swap fg/bg)
    pub reverse: bool,
    /// Hidden/invisible text
    pub hidden: bool,
    /// Blinking text (not typically rendered)
    pub blink: bool,
}

impl CellAttributes {
    /// Create attributes with all flags disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all attributes to default.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The character in this cell (space for empty)
    pub character: char,
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Text attributes
    pub attrs: CellAttributes,
    /// This cell contains a double-width (CJK) character
    pub wide: bool,
    /// This cell is the trailing half of a double-width character
    pub wide_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttributes::default(),
            wide: false,
            wide_continuation: false,
        }
    }
}

impl Cell {
    /// Create a new cell with the given character.
    pub fn new(ch: char) -> Self {
        Self {
            character: ch,
            ..Default::default()
        }
    }

    /// Create a cell with character, colors, and attributes.
    pub fn with_style(ch: char, fg: Color, bg: Color, attrs: CellAttributes) -> Self {
        Self {
            character: ch,
            fg,
            bg,
            attrs,
            ..Default::default()
        }
    }

    /// Reset the cell to empty.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Cursor position in the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPosition {
    /// Column (0-indexed)
    pub col: u16,
    /// Row (0-indexed)
    pub row: u16,
}

/// Terminal screen buffer with character grid and scrollback.
pub struct TerminalScreen {
    /// Current visible screen content (rows x cols)
    cells: Vec<Vec<Cell>>,
    /// Scrollback buffer (older lines at front)
    scrollback: VecDeque<Vec<Cell>>,
    /// Maximum scrollback lines
    max_scrollback: usize,
    /// Screen dimensions
    cols: u16,
    rows: u16,
    /// Current cursor position
    cursor: CursorPosition,
    /// Saved cursor position (for save/restore)
    saved_cursor: Option<CursorPosition>,
    /// Current text attributes for new characters
    current_attrs: CellAttributes,
    /// Current foreground color
    current_fg: Color,
    /// Current background color
    current_bg: Color,
    /// Whether cursor is visible
    cursor_visible: bool,
    /// Scroll region top (inclusive, 0-indexed)
    scroll_top: u16,
    /// Scroll region bottom (inclusive, 0-indexed)
    scroll_bottom: u16,
    /// Origin mode (cursor relative to scroll region)
    origin_mode: bool,
    /// Auto-wrap mode
    auto_wrap: bool,
    /// Pending wrap (cursor at end of line, next char wraps)
    pending_wrap: bool,
    /// Text selection (start, end) as absolute coordinates ((col, row), (col, row))
    selection: Option<((usize, usize), (usize, usize))>,
}

impl TerminalScreen {
    /// Create a new terminal screen with the given dimensions and scrollback limit.
    pub fn new(cols: u16, rows: u16, max_scrollback: usize) -> Self {
        let cells = vec![vec![Cell::default(); cols as usize]; rows as usize];
        Self {
            cells,
            scrollback: VecDeque::new(),
            max_scrollback,
            cols,
            rows,
            cursor: CursorPosition::default(),
            saved_cursor: None,
            current_attrs: CellAttributes::default(),
            current_fg: Color::Default,
            current_bg: Color::Default,
            cursor_visible: true,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            origin_mode: false,
            auto_wrap: true,
            pending_wrap: false,
            selection: None,
        }
    }

    /// Get the terminal size (cols, rows).
    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    /// Check if the cursor is visible.
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Set cursor visibility.
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    /// Get a reference to the screen cells.
    pub fn cells(&self) -> &Vec<Vec<Cell>> {
        &self.cells
    }

    /// Get a reference to the scrollback buffer.
    pub fn scrollback(&self) -> &VecDeque<Vec<Cell>> {
        &self.scrollback
    }

    /// Get the number of scrollback lines.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Resize the terminal screen.
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        // Resize each row to new column count
        for row in &mut self.cells {
            row.resize(new_cols as usize, Cell::default());
        }

        // Add or remove rows
        if new_rows > self.rows {
            // Add rows at bottom
            for _ in self.rows..new_rows {
                self.cells.push(vec![Cell::default(); new_cols as usize]);
            }
        } else if new_rows < self.rows {
            // Remove rows from bottom (or move to scrollback)
            while self.cells.len() > new_rows as usize {
                self.cells.pop();
            }
        }

        self.cols = new_cols;
        self.rows = new_rows;
        self.scroll_bottom = new_rows.saturating_sub(1);

        // Clamp cursor position
        self.cursor.col = self.cursor.col.min(new_cols.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(new_rows.saturating_sub(1));
    }

    /// Put a character at the current cursor position.
    pub fn put_char(&mut self, ch: char) {
        let char_width = ch.width().unwrap_or(1).max(1);

        // Handle pending wrap
        if self.pending_wrap && self.auto_wrap {
            self.cursor.col = 0;
            self.cursor.row += 1;
            if self.cursor.row > self.scroll_bottom {
                self.scroll_up(1);
                self.cursor.row = self.scroll_bottom;
            }
            self.pending_wrap = false;
        }

        // Wide char at last column: not enough room, wrap early
        if char_width == 2 && self.cursor.col + 1 >= self.cols {
            if self.auto_wrap {
                let row = self.cursor.row as usize;
                let col = self.cursor.col as usize;
                if row < self.rows as usize && col < self.cols as usize {
                    self.cells[row][col].clear();
                }
                self.cursor.col = 0;
                self.cursor.row += 1;
                if self.cursor.row > self.scroll_bottom {
                    self.scroll_up(1);
                    self.cursor.row = self.scroll_bottom;
                }
            } else {
                return;
            }
        }

        // Place the character
        if self.cursor.row < self.rows && self.cursor.col < self.cols {
            let row = self.cursor.row as usize;
            let col = self.cursor.col as usize;

            // Overwriting the trailing half of a wide char — clear the leading half
            if col > 0 && self.cells[row][col].wide_continuation {
                self.cells[row][col - 1].clear();
            }

            // Overwriting the leading half of a wide char — clear the trailing half
            if self.cells[row][col].wide && col + 1 < self.cols as usize {
                self.cells[row][col + 1].clear();
            }

            self.cells[row][col] = Cell::with_style(
                ch,
                self.current_fg,
                self.current_bg,
                self.current_attrs,
            );

            if char_width == 2 {
                self.cells[row][col].wide = true;

                if col + 1 < self.cols as usize {
                    // Clear any wide pair the continuation cell was part of
                    if self.cells[row][col + 1].wide && col + 2 < self.cols as usize {
                        self.cells[row][col + 2].clear();
                    }
                    self.cells[row][col + 1] = Cell::default();
                    self.cells[row][col + 1].wide_continuation = true;
                    // Inherit background color for visual continuity
                    self.cells[row][col + 1].fg = self.current_fg;
                    self.cells[row][col + 1].bg = self.current_bg;
                }
            }
        }

        // Advance cursor
        if char_width == 2 {
            let new_col = self.cursor.col + 2;
            if new_col < self.cols {
                self.cursor.col = new_col;
            } else {
                self.cursor.col = self.cols.saturating_sub(1);
                if self.auto_wrap {
                    self.pending_wrap = true;
                }
            }
        } else if self.cursor.col < self.cols.saturating_sub(1) {
            self.cursor.col += 1;
        } else if self.auto_wrap {
            self.pending_wrap = true;
        }
    }

    /// Move cursor to a specific position (1-indexed, as per ANSI).
    pub fn move_cursor(&mut self, row: u16, col: u16) {
        self.pending_wrap = false;
        self.cursor.row = row.saturating_sub(1).min(self.rows.saturating_sub(1));
        self.cursor.col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
    }

    /// Move cursor relative to current position.
    pub fn move_cursor_relative(&mut self, row_delta: i16, col_delta: i16) {
        self.pending_wrap = false;
        let new_row = (self.cursor.row as i16 + row_delta).max(0) as u16;
        let new_col = (self.cursor.col as i16 + col_delta).max(0) as u16;
        self.cursor.row = new_row.min(self.rows.saturating_sub(1));
        self.cursor.col = new_col.min(self.cols.saturating_sub(1));
    }

    /// Move cursor up by n rows.
    pub fn cursor_up(&mut self, n: u16) {
        self.pending_wrap = false;
        self.cursor.row = self.cursor.row.saturating_sub(n).max(self.scroll_top);
    }

    /// Move cursor down by n rows.
    pub fn cursor_down(&mut self, n: u16) {
        self.pending_wrap = false;
        self.cursor.row = (self.cursor.row + n).min(self.scroll_bottom);
    }

    /// Move cursor forward (right) by n columns.
    pub fn cursor_forward(&mut self, n: u16) {
        self.pending_wrap = false;
        self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
    }

    /// Move cursor backward (left) by n columns.
    pub fn cursor_backward(&mut self, n: u16) {
        self.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Carriage return (move to column 0).
    pub fn carriage_return(&mut self) {
        self.pending_wrap = false;
        self.cursor.col = 0;
    }

    /// Line feed (move down one row, scroll if needed).
    pub fn line_feed(&mut self) {
        self.pending_wrap = false;
        if self.cursor.row >= self.scroll_bottom {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
    }

    /// Backspace (move cursor left, don't delete).
    pub fn backspace(&mut self) {
        self.pending_wrap = false;
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    /// Tab (move to next tab stop, typically every 8 columns).
    pub fn tab(&mut self) {
        self.pending_wrap = false;
        let next_tab = ((self.cursor.col / 8) + 1) * 8;
        self.cursor.col = next_tab.min(self.cols.saturating_sub(1));
    }

    /// Scroll the screen up by n lines (content moves up).
    pub fn scroll_up(&mut self, n: u16) {
        for _ in 0..n {
            // Move top line to scrollback
            if self.scroll_top == 0 {
                let top_row = self.cells[0].clone();
                self.scrollback.push_back(top_row);
                while self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }

            // Shift lines up in scroll region
            let top = self.scroll_top as usize;
            let bottom = self.scroll_bottom as usize;
            for row in top..bottom {
                self.cells[row] = self.cells[row + 1].clone();
            }

            // Clear bottom line
            self.cells[bottom] = vec![Cell::default(); self.cols as usize];
        }
    }

    /// Scroll the screen down by n lines (content moves down).
    pub fn scroll_down(&mut self, n: u16) {
        for _ in 0..n {
            let top = self.scroll_top as usize;
            let bottom = self.scroll_bottom as usize;

            // Shift lines down in scroll region
            for row in (top + 1..=bottom).rev() {
                self.cells[row] = self.cells[row - 1].clone();
            }

            // Clear top line
            self.cells[top] = vec![Cell::default(); self.cols as usize];
        }
    }

    /// Erase from cursor to end of line.
    pub fn erase_to_end_of_line(&mut self) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            for c in col..self.cols as usize {
                if c < self.cells[row].len() {
                    self.cells[row][c].clear();
                }
            }
        }
    }

    /// Erase from start of line to cursor.
    pub fn erase_to_start_of_line(&mut self) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            for c in 0..=col {
                if c < self.cells[row].len() {
                    self.cells[row][c].clear();
                }
            }
        }
    }

    /// Erase entire line.
    pub fn erase_line(&mut self) {
        let row = self.cursor.row as usize;
        if row < self.cells.len() {
            for cell in &mut self.cells[row] {
                cell.clear();
            }
        }
    }

    /// Erase from cursor to end of screen.
    pub fn erase_to_end_of_screen(&mut self) {
        self.erase_to_end_of_line();
        for row in (self.cursor.row + 1) as usize..self.rows as usize {
            if row < self.cells.len() {
                for cell in &mut self.cells[row] {
                    cell.clear();
                }
            }
        }
    }

    /// Erase from start of screen to cursor.
    pub fn erase_to_start_of_screen(&mut self) {
        self.erase_to_start_of_line();
        for row in 0..self.cursor.row as usize {
            if row < self.cells.len() {
                for cell in &mut self.cells[row] {
                    cell.clear();
                }
            }
        }
    }

    /// Erase entire screen.
    pub fn erase_screen(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                cell.clear();
            }
        }
    }

    /// Insert n blank lines at cursor row.
    pub fn insert_lines(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let bottom = self.scroll_bottom as usize;

        for _ in 0..n {
            if row <= bottom {
                // Shift lines down
                for r in (row + 1..=bottom).rev() {
                    self.cells[r] = self.cells[r - 1].clone();
                }
                self.cells[row] = vec![Cell::default(); self.cols as usize];
            }
        }
    }

    /// Delete n lines at cursor row.
    pub fn delete_lines(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let bottom = self.scroll_bottom as usize;

        for _ in 0..n {
            if row <= bottom {
                // Shift lines up
                for r in row..bottom {
                    self.cells[r] = self.cells[r + 1].clone();
                }
                self.cells[bottom] = vec![Cell::default(); self.cols as usize];
            }
        }
    }

    /// Insert n blank characters at cursor position.
    pub fn insert_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            for _ in 0..n {
                if col < self.cells[row].len() {
                    self.cells[row].pop();
                    self.cells[row].insert(col, Cell::default());
                }
            }
        }
    }

    /// Delete n characters at cursor position.
    pub fn delete_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            for _ in 0..n {
                if col < self.cells[row].len() {
                    self.cells[row].remove(col);
                    self.cells[row].push(Cell::default());
                }
            }
        }
    }

    /// Save cursor position.
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    /// Restore cursor position.
    pub fn restore_cursor(&mut self) {
        if let Some(pos) = self.saved_cursor {
            self.cursor = pos;
            self.pending_wrap = false;
        }
    }

    /// Set the scroll region.
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let top = top.saturating_sub(1);
        let bottom = bottom.saturating_sub(1).min(self.rows.saturating_sub(1));
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
    }

    /// Reset scroll region to full screen.
    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
    }

    /// Set SGR (Select Graphic Rendition) attribute.
    pub fn set_attr(&mut self, attr: u16) {
        match attr {
            0 => {
                // Reset all attributes
                self.current_attrs = CellAttributes::default();
                self.current_fg = Color::Default;
                self.current_bg = Color::Default;
            }
            1 => self.current_attrs.bold = true,
            2 => self.current_attrs.dim = true,
            3 => self.current_attrs.italic = true,
            4 => self.current_attrs.underline = true,
            5 | 6 => self.current_attrs.blink = true,
            7 => self.current_attrs.reverse = true,
            8 => self.current_attrs.hidden = true,
            9 => self.current_attrs.strikethrough = true,
            21 => self.current_attrs.bold = false,
            22 => {
                self.current_attrs.bold = false;
                self.current_attrs.dim = false;
            }
            23 => self.current_attrs.italic = false,
            24 => self.current_attrs.underline = false,
            25 => self.current_attrs.blink = false,
            27 => self.current_attrs.reverse = false,
            28 => self.current_attrs.hidden = false,
            29 => self.current_attrs.strikethrough = false,
            // Foreground colors
            30..=37 => self.current_fg = Color::Indexed((attr - 30) as u8),
            38 => {} // Extended foreground (handled separately)
            39 => self.current_fg = Color::Default,
            // Background colors
            40..=47 => self.current_bg = Color::Indexed((attr - 40) as u8),
            48 => {} // Extended background (handled separately)
            49 => self.current_bg = Color::Default,
            // Bright foreground colors
            90..=97 => self.current_fg = Color::Indexed((attr - 90 + 8) as u8),
            // Bright background colors
            100..=107 => self.current_bg = Color::Indexed((attr - 100 + 8) as u8),
            _ => {}
        }
    }

    /// Set foreground color (for extended color sequences).
    pub fn set_fg(&mut self, color: Color) {
        self.current_fg = color;
    }

    /// Set background color (for extended color sequences).
    pub fn set_bg(&mut self, color: Color) {
        self.current_bg = color;
    }

    /// Get a line from scrollback (0 = oldest line).
    pub fn scrollback_line(&self, index: usize) -> Option<&Vec<Cell>> {
        self.scrollback.get(index)
    }

    /// Clear the scrollback buffer.
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
    }

    /// Set the text selection.
    pub fn set_selection(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.selection = Some((start, end));
    }

    /// Clear the text selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Get the text selection.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection
    }

    /// Snap a column to the leading cell of a wide character if it lands on a continuation cell.
    /// `abs_row` is an absolute row index (scrollback + screen).
    pub fn snap_wide_char(&self, col: usize, abs_row: usize) -> usize {
        let scrollback_len = self.scrollback.len();
        let row = if abs_row < scrollback_len {
            self.scrollback.get(abs_row)
        } else {
            self.cells.get(abs_row.saturating_sub(scrollback_len))
        };
        if let Some(row) = row {
            if col < row.len() && row[col].wide_continuation {
                return col.saturating_sub(1);
            }
        }
        col
    }

    /// Get the selected text.
    pub fn get_selected_text(&self) -> Option<String> {
        let (start, end) = self.selection?;
        
        // Normalize coordinates (start should be before end)
        let (start, end) = if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
            (start, end)
        } else {
            (end, start)
        };

        let mut text = String::new();
        let scrollback_len = self.scrollback_len();

        for row_idx in start.1..=end.1 {
            let row = if row_idx < scrollback_len {
                self.scrollback.get(row_idx)
            } else {
                self.cells.get(row_idx - scrollback_len)
            };

            if let Some(row) = row {
                let start_col = if row_idx == start.1 { start.0 } else { 0 };
                let end_col = if row_idx == end.1 { end.0.min(row.len().saturating_sub(1)) } else { row.len().saturating_sub(1) };
                
                if start_col <= end_col && start_col < row.len() {
                    for cell in &row[start_col..=end_col] {
                        if cell.wide_continuation {
                            continue;
                        }
                        text.push(cell.character);
                    }
                }
                
                if row_idx < end.1 {
                    text.push('\n');
                }
            }
        }
        
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Get the text content of a specific row.
    pub fn get_row_text(&self, row: usize) -> String {
        if row < self.cells.len() {
            self.cells[row].iter()
                .filter(|c| !c.wide_continuation)
                .map(|c| c.character)
                .collect::<String>()
                .trim_end()
                .to_string()
        } else {
            String::new()
        }
    }

    /// Get the text content of the row where the cursor is.
    pub fn get_cursor_line_text(&self) -> String {
        self.get_row_text(self.cursor.row as usize)
    }

    /// Check if any visible row contains the given text (case-insensitive).
    pub fn screen_contains(&self, needle: &str) -> bool {
        let needle_lower = needle.to_lowercase();
        for row in &self.cells {
            let line: String = row.iter()
                .filter(|c| !c.wide_continuation)
                .map(|c| c.character)
                .collect();
            if line.to_lowercase().contains(&needle_lower) {
                return true;
            }
        }
        false
    }

    /// Get all visible screen content as a single string.
    pub fn get_visible_text(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .filter(|c| !c.wide_continuation)
                    .map(|c| c.character)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Reset the terminal to initial state.
    pub fn reset(&mut self) {
        self.cells = vec![vec![Cell::default(); self.cols as usize]; self.rows as usize];
        self.cursor = CursorPosition::default();
        self.saved_cursor = None;
        self.current_attrs = CellAttributes::default();
        self.current_fg = Color::Default;
        self.current_bg = Color::Default;
        self.cursor_visible = true;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.auto_wrap = true;
        self.pending_wrap = false;
        self.selection = None;
    }

    /// Export the terminal content as HTML.
    pub fn export_html(&self, ansi_colors: &[eframe::egui::Color32; 16], default_fg: eframe::egui::Color32, default_bg: eframe::egui::Color32) -> String {
        let mut html = String::from("<pre style=\"font-family: monospace; line-height: 1.2; background-color: ");
        
        let bg_hex = format!("#{:02x}{:02x}{:02x}", default_bg.r(), default_bg.g(), default_bg.b());
        let fg_hex = format!("#{:02x}{:02x}{:02x}", default_fg.r(), default_fg.g(), default_fg.b());
        
        html.push_str(&bg_hex);
        html.push_str("; color: ");
        html.push_str(&fg_hex);
        html.push_str("\">\n");

        let process_line = |row: &Vec<Cell>, html: &mut String| {
            let mut current_fg = Color::Default;
            let mut current_bg = Color::Default;
            let mut current_attrs = CellAttributes::default();
            let mut span_open = false;

            for cell in row {
                if cell.wide_continuation {
                    continue;
                }

                let style_changed = cell.fg != current_fg || cell.bg != current_bg || cell.attrs != current_attrs;
                
                if style_changed {
                    if span_open {
                        html.push_str("</span>");
                        span_open = false;
                    }

                    // Only open span if style is not default
                    if cell.fg != Color::Default || cell.bg != Color::Default || cell.attrs != CellAttributes::default() {
                        html.push_str("<span style=\"");
                        
                        // FG
                        let fg = cell.fg.to_egui(true, ansi_colors, default_fg, default_bg);
                        if fg != default_fg {
                            html.push_str(&format!("color: #{:02x}{:02x}{:02x}; ", fg.r(), fg.g(), fg.b()));
                        }

                        // BG
                        let bg = cell.bg.to_egui(false, ansi_colors, default_fg, default_bg);
                        if bg != eframe::egui::Color32::TRANSPARENT && bg != default_bg {
                            html.push_str(&format!("background-color: #{:02x}{:02x}{:02x}; ", bg.r(), bg.g(), bg.b()));
                        }

                        if cell.attrs.bold { html.push_str("font-weight: bold; "); }
                        if cell.attrs.italic { html.push_str("font-style: italic; "); }
                        if cell.attrs.underline { html.push_str("text-decoration: underline; "); }
                        
                        html.push_str("\">");
                        span_open = true;
                    }

                    current_fg = cell.fg;
                    current_bg = cell.bg;
                    current_attrs = cell.attrs;
                }

                match cell.character {
                    '<' => html.push_str("&lt;"),
                    '>' => html.push_str("&gt;"),
                    '&' => html.push_str("&amp;"),
                    '"' => html.push_str("&quot;"),
                    c => html.push(c),
                }
            }

            if span_open {
                html.push_str("</span>");
            }
            html.push('\n');
        };

        for row in &self.scrollback {
            process_line(row, &mut html);
        }
        for row in &self.cells {
            process_line(row, &mut html);
        }

        html.push_str("</pre>");
        html
    }
}
