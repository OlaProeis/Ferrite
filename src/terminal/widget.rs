//! Terminal widget for egui rendering.
//!
//! This module provides an egui widget that renders the terminal screen
//! buffer and handles keyboard input.

use super::screen::TerminalScreen;
use super::theme::TerminalTheme;
use arboard::Clipboard;
use eframe::egui::{self, Color32, FontId, Key, Modifiers, Rect, Sense, Ui, Vec2};
use std::sync::{Arc, Mutex};

/// Output from the terminal widget.
#[derive(Debug, Default)]
pub struct TerminalWidgetOutput {
    /// Keyboard input to send to the terminal (as bytes)
    pub input: Vec<u8>,
    /// Whether the widget has focus (for receiving keyboard input)
    pub has_focus: bool,
    /// Whether the user actually clicked/focused on the terminal (for shortcuts)
    pub user_interacted: bool,
    /// New size if terminal was resized (cols, rows)
    pub new_size: Option<(u16, u16)>,
    /// Text to copy to clipboard (if any)
    pub copy_text: Option<String>,
    /// Updated scroll offset after user scroll input
    pub new_scroll_offset: Option<usize>,
}

/// Widget for rendering a terminal in egui.
pub struct TerminalWidget<'a> {
    /// The terminal screen buffer to render
    screen: &'a Arc<Mutex<TerminalScreen>>,
    /// Font size in pixels
    font_size: f32,
    /// Whether the terminal is focused
    focused: bool,
    /// Scroll offset into scrollback (0 = current screen)
    scroll_offset: usize,
    /// Theme to use
    theme: TerminalTheme,
    /// Background opacity (0.0 - 1.0)
    opacity: f32,
    /// Whether to copy text to clipboard immediately on selection
    copy_on_select: bool,
    /// Whether the terminal is waiting for input (visual indicator)
    is_waiting: bool,
    /// Color for breathing animation
    breathing_color: Color32,
}

impl<'a> TerminalWidget<'a> {
    /// Create a new terminal widget.
    pub fn new(screen: &'a Arc<Mutex<TerminalScreen>>) -> Self {
        Self {
            screen,
            font_size: 14.0,
            focused: false,
            scroll_offset: 0,
            theme: TerminalTheme::default(),
            opacity: 1.0,
            copy_on_select: false,
            is_waiting: false,
            breathing_color: Color32::from_rgb(100, 149, 237),
        }
    }

    /// Set the font size.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set whether the terminal is focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set the scroll offset into scrollback.
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: TerminalTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the opacity.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set whether to copy text to clipboard immediately on selection.
    pub fn copy_on_select(mut self, enabled: bool) -> Self {
        self.copy_on_select = enabled;
        self
    }

    /// Set whether the terminal is waiting for input.
    pub fn is_waiting(mut self, waiting: bool) -> Self {
        self.is_waiting = waiting;
        self
    }

    /// Set the breathing animation color.
    pub fn breathing_color(mut self, color: Color32) -> Self {
        self.breathing_color = color;
        self
    }

    /// Calculate character dimensions for the monospace font.
    fn char_size(&self, ui: &Ui) -> Vec2 {
        let font_id = FontId::monospace(self.font_size);
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
        let line_height = self.font_size * 1.2;
        Vec2::new(char_width, line_height)
    }

    /// Show the terminal widget and return output.
    pub fn show(self, ui: &mut Ui) -> TerminalWidgetOutput {
        let mut output = TerminalWidgetOutput::default();

        let char_size = self.char_size(ui);
        // Lock screen once
        let mut screen = self.screen.lock().unwrap();
        let (cols, rows) = screen.size();
        let scrollback_len = screen.scrollback_len();

        // Allocate all available space for the terminal, reserving space for scrollbar
        // Use max_rect() to ensure we fill the entire child_ui allocated for this pane
        let total_rect = ui.max_rect();
        let scrollbar_width = 12.0;

        let terminal_rect = Rect::from_min_size(
            total_rect.min,
            egui::vec2(
                (total_rect.width() - scrollbar_width).max(0.0),
                total_rect.height(),
            ),
        );

        let scrollbar_rect = Rect::from_min_size(
            egui::pos2(total_rect.right() - scrollbar_width, total_rect.top()),
            egui::vec2(scrollbar_width, total_rect.height()),
        );

        // Allocate the terminal area
        let response = ui.allocate_rect(terminal_rect, Sense::click_and_drag());

        // Allocate the scrollbar area (so it doesn't overlap if we use allocate_ui later, though we draw manually)
        let scrollbar_response = ui.allocate_rect(scrollbar_rect, Sense::click_and_drag());

        // Request focus on click
        if response.clicked() {
            response.request_focus();
            output.user_interacted = true;
            // Clear selection on single click
            screen.clear_selection();
        } else {
            output.user_interacted = false;
        }

        // Request focus if we're supposed to be focused
        if self.focused && !response.has_focus() {
            response.request_focus();
        }

        output.has_focus = response.has_focus() || self.focused;

        // Check if size changed and calculate new terminal dimensions
        let rect_size = terminal_rect.size();
        let new_cols = (rect_size.x / char_size.x).floor() as u16;
        let new_rows = (rect_size.y / char_size.y).floor() as u16;
        if new_cols > 0 && new_rows > 0 && (new_cols != cols || new_rows != rows) {
            output.new_size = Some((new_cols.max(10), new_rows.max(2)));
        }

        let max_scroll_offset = scrollback_len;
        let mut scroll_offset = self.scroll_offset.min(max_scroll_offset);
        if scroll_offset != self.scroll_offset {
            output.new_scroll_offset = Some(scroll_offset);
        }

        // Calculate absolute start row of the visible area
        let total_lines = scrollback_len + rows as usize;
        let end_line = total_lines.saturating_sub(scroll_offset);
        let start_line = end_line.saturating_sub(rows as usize);

        // Helper to get absolute position from pointer pos
        let get_abs_pos = |pos: egui::Pos2| -> Option<(usize, usize)> {
            if !terminal_rect.contains(pos) {
                return None;
            }
            let rel_x = pos.x - terminal_rect.left();
            let rel_y = pos.y - terminal_rect.top();
            let col = (rel_x / char_size.x).floor() as usize;
            let row = (rel_y / char_size.y).floor() as usize;
            if col < cols as usize && row < rows as usize {
                Some((col, start_line + row))
            } else {
                None
            }
        };

        // Handle selection
        if response.drag_started() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(mut pos) = get_abs_pos(pointer_pos) {
                    pos.0 = screen.snap_wide_char(pos.0, pos.1);
                    screen.set_selection(pos, pos);
                }
            }
        } else if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let clamped_pos = terminal_rect.clamp(pointer_pos);
                let rel_x = (clamped_pos.x - terminal_rect.left()).max(0.0);
                let rel_y = (clamped_pos.y - terminal_rect.top()).max(0.0);
                let col =
                    ((rel_x / char_size.x).floor() as usize).min((cols as usize).saturating_sub(1));
                let row =
                    ((rel_y / char_size.y).floor() as usize).min((rows as usize).saturating_sub(1));

                let abs_row = start_line + row;
                let snapped_col = screen.snap_wide_char(col, abs_row);
                let abs_pos = (snapped_col, abs_row);

                if let Some(mut sel) = screen.selection() {
                    sel.1 = abs_pos;
                    screen.set_selection(sel.0, sel.1);
                } else {
                    screen.set_selection(abs_pos, abs_pos);
                }
            }
        }

        // Handle auto-copy on selection release
        if self.copy_on_select && response.drag_stopped() {
            if let Some(text) = screen.get_selected_text() {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
                output.copy_text = Some(text);
            }
        }

        // Handle scroll events on terminal rect
        let scroll_delta = ui.input(|i| {
            if i.smooth_scroll_delta.y.abs() > 0.0 {
                i.smooth_scroll_delta.y
            } else {
                i.raw_scroll_delta.y
            }
        });
        if scroll_delta.abs() > 0.1 && response.hovered() {
            // Scroll terminal (positive delta = scroll up, negative = scroll down)
            let mut lines = (scroll_delta / char_size.y).round() as isize;
            if lines == 0 {
                lines = if scroll_delta > 0.0 { 1 } else { -1 };
            }
            let max_offset = max_scroll_offset as isize;
            let new_offset = (scroll_offset as isize + lines).clamp(0, max_offset) as usize;
            if new_offset != scroll_offset {
                scroll_offset = new_offset;
                output.new_scroll_offset = Some(scroll_offset);
            }
        }

        // Scrollbar logic
        // Total range: 0 to max_scrollback_offset.
        // View size: rows.
        // Total items: max_scrollback_offset + rows.
        // Scrollbar represents the viewport position in the total history.
        // Top of scrollbar = oldest history (offset = max).
        // Bottom of scrollbar = newest output (offset = 0).

        let total_content_lines = scrollback_len + rows as usize;
        if total_content_lines > rows as usize {
            // Draw scrollbar background
            let scrollbar_bg = Color32::from_rgba_premultiplied(
                self.theme.background.r(),
                self.theme.background.g(),
                self.theme.background.b(),
                (self.opacity * 200.0) as u8,
            );
            ui.painter().rect_filled(scrollbar_rect, 0.0, scrollbar_bg);

            // Calculate handle
            // handle_height / track_height = viewport_height / total_height
            let viewport_ratio = (rows as f32 / total_content_lines as f32).clamp(0.05, 1.0);
            let handle_height = (scrollbar_rect.height() * viewport_ratio).max(20.0);

            // Position:
            // offset 0 (newest) -> bottom
            // offset max (oldest) -> top
            // wait, usually scrollbar top = 0 (top of file).
            // Here line 0 is top of scrollback.
            // Current viewport starts at `start_line`.
            // Ratio = start_line / (total - viewport)

            let max_start_line = total_content_lines.saturating_sub(rows as usize);
            let current_pos_ratio = if max_start_line > 0 {
                start_line as f32 / max_start_line as f32
            } else {
                1.0
            };

            let track_len = scrollbar_rect.height() - handle_height;
            let handle_y = scrollbar_rect.top() + (current_pos_ratio * track_len);

            let handle_rect = Rect::from_min_size(
                egui::pos2(scrollbar_rect.left(), handle_y),
                egui::vec2(scrollbar_width, handle_height),
            );

            // Draw handle
            let handle_color = if scrollbar_response.dragged() || scrollbar_response.hovered() {
                Color32::from_rgba_premultiplied(
                    self.theme.foreground.r(),
                    self.theme.foreground.g(),
                    self.theme.foreground.b(),
                    150,
                )
            } else {
                Color32::from_rgba_premultiplied(
                    self.theme.foreground.r(),
                    self.theme.foreground.g(),
                    self.theme.foreground.b(),
                    80,
                )
            };

            ui.painter().rect_filled(handle_rect, 6.0, handle_color);

            // Handle drag
            if scrollbar_response.dragged() {
                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    let rel_y = pointer_pos.y - scrollbar_rect.top() - (handle_height / 2.0);
                    let ratio = (rel_y / track_len).clamp(0.0, 1.0);

                    // Convert ratio back to start_line, then to offset
                    let new_start_line = (ratio * max_start_line as f32).round() as usize;
                    // start_line = (total - offset) - rows
                    // offset = total - rows - start_line
                    let new_offset = total_content_lines
                        .saturating_sub(rows as usize)
                        .saturating_sub(new_start_line);

                    if new_offset != scroll_offset {
                        scroll_offset = new_offset;
                        output.new_scroll_offset = Some(scroll_offset);
                    }
                }
            }
        }

        // Handle keyboard input
        if output.has_focus {
            self.handle_keyboard_input(ui, &mut screen, &mut output);
        } else {
            // Debug: log why we don't have focus
            if self.focused {
                // log::warn!("Terminal: self.focused=true but response.has_focus()=false");
            }
        }

        // Right-click to paste (instant paste, no menu)
        if response.secondary_clicked() {
            if let Ok(mut clipboard) = Clipboard::new() {
                if let Ok(text) = clipboard.get_text() {
                    if !text.is_empty() {
                        output.input.extend_from_slice(text.as_bytes());
                    }
                }
            }
        }

        // Render the terminal
        self.render_screen(ui, terminal_rect, &screen, char_size, scroll_offset);

        // Render cursor if focused
        if output.has_focus && screen.cursor_visible() && scroll_offset == 0 {
            self.render_cursor(ui, terminal_rect, &screen, char_size);
        }

        output
    }

    /// Handle keyboard input and convert to terminal bytes.
    fn handle_keyboard_input(
        &self,
        ui: &Ui,
        screen: &mut TerminalScreen,
        output: &mut TerminalWidgetOutput,
    ) {
        let ctx = ui.ctx();

        // 1. Gather abstract events (Copy/Cut/Paste)
        let mut copy_requested = false;
        let mut cut_requested = false;
        let mut paste_content = Vec::new();

        ctx.input_mut(|i| {
            let mut kept = Vec::with_capacity(i.events.len());
            for event in i.events.drain(..) {
                match event {
                    egui::Event::Copy => copy_requested = true,
                    egui::Event::Cut => cut_requested = true,
                    egui::Event::Paste(text) => paste_content.push(text),
                    _ => kept.push(event),
                }
            }
            i.events = kept;
        });

        // 2. Check for explicit key combinations that might trigger these actions
        // (In case egui didn't generate the abstract event, or to handle special terminal shortcuts)

        let mut sigint_requested = false;
        let mut word_erase_requested = false;

        // We need to consume specific keys to prevent them from generating duplicate Event::Text
        // or being handled by parent widgets.

        let (
            ctrl_c_pressed,
            ctrl_x_pressed,
            ctrl_v_pressed,
            ctrl_w_pressed,
            ctrl_shift_c_pressed,
            ctrl_shift_v_pressed,
        ) = ctx.input_mut(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            let shift = i.modifiers.shift;

            let c_pressed = i.key_pressed(Key::C);
            let x_pressed = i.key_pressed(Key::X);
            let v_pressed = i.key_pressed(Key::V);
            let w_pressed = i.key_pressed(Key::W);

            let ctrl_c = ctrl && !shift && c_pressed;
            let ctrl_x = ctrl && !shift && x_pressed;
            let ctrl_v = ctrl && !shift && v_pressed;
            let ctrl_w = ctrl && !shift && w_pressed;

            let ctrl_shift_c = ctrl && shift && c_pressed;
            let ctrl_shift_v = ctrl && shift && v_pressed;

            // Consume keys if matched
            if ctrl_c {
                i.consume_key(egui::Modifiers::COMMAND, Key::C);
            }
            if ctrl_x {
                i.consume_key(egui::Modifiers::COMMAND, Key::X);
            }
            if ctrl_v {
                i.consume_key(egui::Modifiers::COMMAND, Key::V);
            }
            if ctrl_w {
                i.consume_key(egui::Modifiers::COMMAND, Key::W);
            }
            if ctrl_shift_c {
                i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::C);
            }
            if ctrl_shift_v {
                i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::V);
            }

            (ctrl_c, ctrl_x, ctrl_v, ctrl_w, ctrl_shift_c, ctrl_shift_v)
        });

        // 3. Logic resolution

        // Paste
        if !paste_content.is_empty() || ctrl_shift_v_pressed || ctrl_v_pressed {
            // Prioritize explicit paste content, then clipboard fetch
            if !paste_content.is_empty() {
                for text in &paste_content {
                    if !text.is_empty() {
                        output.input.extend_from_slice(text.as_bytes());
                    }
                }
            } else {
                // Fetch from clipboard
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        if !text.is_empty() {
                            output.input.extend_from_slice(text.as_bytes());
                        }
                    }
                }
            }
        }

        // Copy / Cut / SIGINT
        let selection_text = screen.get_selected_text();

        // Ctrl+C handling:
        // - If Ctrl+Shift+C: Always Copy
        // - If Copy event (usually Ctrl+C): Copy if selection, else SIGINT
        // - If Ctrl+C key: Copy if selection, else SIGINT

        if ctrl_shift_c_pressed {
            if let Some(text) = &selection_text {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
                output.copy_text = Some(text.clone());
            }
        } else if copy_requested || ctrl_c_pressed {
            if let Some(text) = &selection_text {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
                output.copy_text = Some(text.clone());
            } else {
                sigint_requested = true;
            }
        }

        // Cut handling:
        // - If selection: Copy (terminals rarely do real cut)
        // - Else: Send Ctrl+X (0x18)
        if cut_requested || ctrl_x_pressed {
            if let Some(text) = &selection_text {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
            } else {
                output.input.push(0x18);
            }
        }

        // Ctrl+W handling
        if ctrl_w_pressed {
            word_erase_requested = true;
        }

        // Apply generated control signals
        if sigint_requested {
            output.input.push(0x03);
        }
        if word_erase_requested {
            output.input.push(0x17);
        }

        // 4. Handle remaining events (Text, Keys)
        // We must filter out handled events from the input stream to prevent bubbling
        ctx.input_mut(|i| {
            i.events.retain(|event| {
                match event {
                    egui::Event::Text(text) => {
                        // Filter out control chars we might have already handled
                        let bytes = text.as_bytes();
                        if bytes.len() == 1 {
                            let b = bytes[0];
                            if (sigint_requested && b == 0x03)
                                || (word_erase_requested && b == 0x17)
                                || ((cut_requested || ctrl_x_pressed) && b == 0x18)
                                || ((!paste_content.is_empty() || ctrl_v_pressed) && b == 0x16)
                            {
                                return false; // Consume/Filter
                            }
                        }

                        output.input.extend_from_slice(bytes);
                        false // Consume text events handled here
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        // Skip keys we explicitly consumed above
                        if matches!(key, Key::C | Key::X | Key::V | Key::W)
                            && (modifiers.ctrl || modifiers.command)
                        {
                            return false; // Already handled
                        }

                        if let Some(bytes) = self.key_to_bytes(*key, *modifiers) {
                            output.input.extend_from_slice(&bytes);
                            false // Consumed!
                        } else {
                            true // Keep for other handlers
                        }
                    }
                    _ => true,
                }
            });
        });
    }

    /// Convert a key press to terminal escape sequence bytes.
    fn key_to_bytes(&self, key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
        let ctrl = modifiers.ctrl || modifiers.command;
        let shift = modifiers.shift;
        let alt = modifiers.alt;

        // Control key combinations (Ctrl+A = 0x01, etc.)
        // NOTE: Ctrl+C and Ctrl+W are handled separately in handle_keyboard_input to consume events
        if ctrl && !alt && !shift {
            match key {
                Key::A => return Some(vec![0x01]),
                Key::B => return Some(vec![0x02]),
                // Key::C handled separately (SIGINT + event consumption)
                Key::D => return Some(vec![0x04]), // EOF
                Key::E => return Some(vec![0x05]),
                Key::F => return Some(vec![0x06]),
                Key::G => return Some(vec![0x07]),
                Key::H => return Some(vec![0x08]),
                Key::I => return Some(vec![0x09]),
                Key::J => return Some(vec![0x0A]),
                Key::K => return Some(vec![0x0B]),
                Key::L => return Some(vec![0x0C]), // Clear
                Key::M => return Some(vec![0x0D]),
                Key::N => return Some(vec![0x0E]),
                Key::O => return Some(vec![0x0F]),
                Key::P => return Some(vec![0x10]),
                Key::Q => return Some(vec![0x11]),
                Key::R => return Some(vec![0x12]),
                Key::S => return Some(vec![0x13]),
                Key::T => return Some(vec![0x14]),
                Key::U => return Some(vec![0x15]),
                Key::V => return Some(vec![0x16]),
                // Key::W handled separately (word deletion + event consumption)
                Key::X => return Some(vec![0x18]),
                Key::Y => return Some(vec![0x19]),
                Key::Z => return Some(vec![0x1A]), // SIGTSTP
                _ => {}
            }
        }

        // Special keys - Only handle if not modifiers are present or if explicitly handled with modifiers
        match key {
            Key::Enter => return Some(vec![0x0D]),
            Key::Tab => return Some(vec![0x09]),
            Key::Backspace => {
                if ctrl {
                    return Some(vec![0x17]); // Ctrl+Backspace -> Ctrl+W (delete word)
                } else if alt {
                    return Some(vec![0x1b, 0x7F]); // Alt+Backspace
                }
                return Some(vec![0x7F]);
            }
            Key::Delete => {
                if ctrl {
                    // Ctrl+Delete -> Alt+D (delete word forward)
                    return Some(vec![0x1b, 0x64]);
                }
                if alt {
                    return None;
                }
                return Some(b"\x1b[3~".to_vec());
            }
            Key::Escape => return Some(vec![0x1B]),
            Key::Insert => {
                if ctrl || alt {
                    return None;
                }
                return Some(b"\x1b[2~".to_vec());
            }
            Key::Home => {
                if ctrl || alt {
                    return None;
                }
                return Some(b"\x1b[H".to_vec());
            }
            Key::End => {
                if ctrl || alt {
                    return None;
                }
                return Some(b"\x1b[F".to_vec());
            }
            Key::PageUp => {
                if ctrl || alt {
                    return None;
                }
                return Some(b"\x1b[5~".to_vec());
            }
            Key::PageDown => {
                if ctrl || alt {
                    return None;
                }
                return Some(b"\x1b[6~".to_vec());
            }
            Key::ArrowUp => {
                if shift {
                    return Some(b"\x1b[1;2A".to_vec());
                } else if ctrl {
                    return Some(b"\x1b[1;5A".to_vec());
                } else if alt {
                    return Some(b"\x1b[1;3A".to_vec());
                }
                return Some(b"\x1b[A".to_vec());
            }
            Key::ArrowDown => {
                if shift {
                    return Some(b"\x1b[1;2B".to_vec());
                } else if ctrl {
                    return Some(b"\x1b[1;5B".to_vec());
                } else if alt {
                    return Some(b"\x1b[1;3B".to_vec());
                }
                return Some(b"\x1b[B".to_vec());
            }
            Key::ArrowRight => {
                if shift {
                    return Some(b"\x1b[1;2C".to_vec());
                } else if ctrl {
                    return Some(b"\x1b[1;5C".to_vec());
                } else if alt {
                    return Some(b"\x1b[1;3C".to_vec());
                }
                return Some(b"\x1b[C".to_vec());
            }
            Key::ArrowLeft => {
                if shift {
                    return Some(b"\x1b[1;2D".to_vec());
                } else if ctrl {
                    return Some(b"\x1b[1;5D".to_vec());
                } else if alt {
                    return Some(b"\x1b[1;3D".to_vec());
                }
                return Some(b"\x1b[D".to_vec());
            }
            Key::F1 => return Some(b"\x1bOP".to_vec()),
            Key::F2 => return Some(b"\x1bOQ".to_vec()),
            Key::F3 => return Some(b"\x1bOR".to_vec()),
            Key::F4 => return Some(b"\x1bOS".to_vec()),
            Key::F5 => return Some(b"\x1b[15~".to_vec()),
            Key::F6 => return Some(b"\x1b[17~".to_vec()),
            Key::F7 => return Some(b"\x1b[18~".to_vec()),
            Key::F8 => return Some(b"\x1b[19~".to_vec()),
            Key::F9 => return Some(b"\x1b[20~".to_vec()),
            Key::F10 => return Some(b"\x1b[21~".to_vec()),
            Key::F11 => return Some(b"\x1b[23~".to_vec()),
            Key::F12 => return Some(b"\x1b[24~".to_vec()),
            _ => {}
        }

        None
    }

    /// Render the terminal screen content.
    fn render_screen(
        &self,
        ui: &Ui,
        rect: Rect,
        screen: &TerminalScreen,
        char_size: Vec2,
        scroll_offset: usize,
    ) {
        let painter = ui.painter();

        // Draw background with opacity
        let mut bg_color = self.theme.background;
        if self.opacity < 1.0 {
            bg_color = Color32::from_rgba_premultiplied(
                bg_color.r(),
                bg_color.g(),
                bg_color.b(),
                (self.opacity * 255.0) as u8,
            );
        }
        painter.rect_filled(rect, 0.0, bg_color);

        // Draw breathing indicator if waiting
        if self.is_waiting {
            ui.ctx().request_repaint(); // Continuous repaint for animation
            let time = ui.input(|i| i.time);
            let alpha = (time * 3.0).sin().abs() * 0.4 + 0.1; // 0.1 to 0.5
            let color = Color32::from_rgba_unmultiplied(
                self.breathing_color.r(),
                self.breathing_color.g(),
                self.breathing_color.b(),
                (alpha * 255.0) as u8,
            );

            painter.rect_stroke(rect.expand(1.0), 0.0, egui::Stroke::new(2.0, color));
        }

        // Selection state
        let selection = screen.selection().map(|(start, end)| {
            if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
                (start, end)
            } else {
                (end, start)
            }
        });

        // Get cells to render
        let cells = screen.cells();
        let rows = screen.size().1 as usize;
        let scrollback_len = screen.scrollback_len();
        let max_scroll_offset = scrollback_len;
        let scroll_offset = scroll_offset.min(max_scroll_offset);
        let total_lines = scrollback_len + rows;
        let end = total_lines.saturating_sub(scroll_offset);
        let start = end.saturating_sub(rows);
        let font_id = FontId::monospace(self.font_size);

        // Render each cell
        for row_idx in 0..rows {
            let y = rect.top() + (row_idx as f32 * char_size.y);
            let line_index = start + row_idx;
            let row = if line_index < scrollback_len {
                screen.scrollback_line(line_index)
            } else {
                cells.get(line_index.saturating_sub(scrollback_len))
            };

            let Some(row) = row else {
                continue;
            };

            for (col_idx, cell) in row.iter().enumerate() {
                if cell.wide_continuation {
                    continue;
                }

                let x = rect.left() + (col_idx as f32 * char_size.x);
                let cell_w = if cell.wide {
                    char_size.x * 2.0
                } else {
                    char_size.x
                };
                let cell_rect =
                    Rect::from_min_size(egui::pos2(x, y), Vec2::new(cell_w, char_size.y));

                // Check if cell is selected (for wide chars, selected if either column is in range)
                let is_selected = if let Some((sel_start, sel_end)) = selection {
                    let col_end = if cell.wide { col_idx + 1 } else { col_idx };
                    if line_index > sel_start.1 && line_index < sel_end.1 {
                        true
                    } else if line_index == sel_start.1 && line_index == sel_end.1 {
                        col_end >= sel_start.0 && col_idx <= sel_end.0
                    } else if line_index == sel_start.1 {
                        col_end >= sel_start.0
                    } else if line_index == sel_end.1 {
                        col_idx <= sel_end.0
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Draw cell background
                let mut bg = cell.bg.to_egui(
                    false,
                    &self.theme.ansi_colors,
                    self.theme.foreground,
                    self.theme.background,
                );

                if is_selected {
                    bg = self.theme.selection_bg;
                }

                if bg != Color32::TRANSPARENT {
                    if bg == self.theme.background && self.opacity < 1.0 {
                        bg = bg_color;
                    }
                    painter.rect_filled(cell_rect, 0.0, bg);
                }

                // Draw character
                if cell.character != ' ' || is_selected {
                    let mut fg = cell.fg.to_egui(
                        true,
                        &self.theme.ansi_colors,
                        self.theme.foreground,
                        self.theme.background,
                    );

                    if cell.attrs.dim {
                        fg = Color32::from_rgba_unmultiplied(
                            fg.r(),
                            fg.g(),
                            fg.b(),
                            (fg.a() as f32 * 0.5) as u8,
                        );
                    }
                    if cell.attrs.reverse {
                        let temp_bg = cell.bg.to_egui(
                            false,
                            &self.theme.ansi_colors,
                            self.theme.foreground,
                            self.theme.background,
                        );
                        if temp_bg != Color32::TRANSPARENT {
                            fg = temp_bg;
                        } else {
                            fg = self.theme.background;
                        }
                        if !is_selected {
                            painter.rect_filled(
                                cell_rect,
                                0.0,
                                cell.fg.to_egui(
                                    true,
                                    &self.theme.ansi_colors,
                                    self.theme.foreground,
                                    self.theme.background,
                                ),
                            );
                        }
                    }
                    if cell.attrs.hidden {
                        fg = bg_color;
                    }

                    let font = if cell.attrs.bold {
                        FontId::monospace(self.font_size)
                    } else {
                        font_id.clone()
                    };

                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        cell.character,
                        font,
                        fg,
                    );

                    // Draw underline spanning full cell width
                    if cell.attrs.underline {
                        let underline_y = y + char_size.y - 2.0;
                        painter.line_segment(
                            [
                                egui::pos2(x, underline_y),
                                egui::pos2(x + cell_w, underline_y),
                            ],
                            egui::Stroke::new(1.0, fg),
                        );
                    }

                    // Draw strikethrough spanning full cell width
                    if cell.attrs.strikethrough {
                        let strike_y = y + char_size.y / 2.0;
                        painter.line_segment(
                            [egui::pos2(x, strike_y), egui::pos2(x + cell_w, strike_y)],
                            egui::Stroke::new(1.0, fg),
                        );
                    }
                }
            }
        }
    }

    /// Render the cursor.
    fn render_cursor(&self, ui: &Ui, rect: Rect, screen: &TerminalScreen, char_size: Vec2) {
        let cursor = screen.cursor();

        // Snap cursor to the leading cell if it lands on a continuation cell
        let mut col = cursor.col as usize;
        let cells = screen.cells();
        if let Some(row) = cells.get(cursor.row as usize) {
            if col < row.len() && row[col].wide_continuation && col > 0 {
                col -= 1;
            }
        }

        let x = rect.left() + (col as f32 * char_size.x);
        let y = rect.top() + (cursor.row as f32 * char_size.y);

        // Double-width cursor for wide characters
        let cursor_w = if let Some(row) = cells.get(cursor.row as usize) {
            if col < row.len() && row[col].wide {
                char_size.x * 2.0
            } else {
                char_size.x
            }
        } else {
            char_size.x
        };

        let cursor_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(cursor_w, char_size.y));

        let cursor_color = self.theme.cursor;
        ui.painter().rect_filled(cursor_rect, 0.0, cursor_color);
    }
}
