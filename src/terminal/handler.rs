//! VTE event handler for terminal emulation.
//!
//! This module implements the `vte::Perform` trait to handle ANSI escape
//! sequences and update the terminal screen buffer.

use super::screen::{Color, TerminalScreen};

/// Handler for VTE parser events that updates a terminal screen.
pub struct TerminalHandler<'a> {
    /// Reference to the terminal screen to update
    screen: &'a mut TerminalScreen,
    /// Pending title from OSC sequence
    pending_title: Option<String>,
}

impl<'a> TerminalHandler<'a> {
    /// Create a new terminal handler for the given screen.
    pub fn new(screen: &'a mut TerminalScreen) -> Self {
        Self {
            screen,
            pending_title: None,
        }
    }

    /// Take the pending title if set by an OSC sequence.
    pub fn take_title(&mut self) -> Option<String> {
        self.pending_title.take()
    }
}

impl<'a> vte::Perform for TerminalHandler<'a> {
    /// Handle printable characters.
    fn print(&mut self, c: char) {
        self.screen.put_char(c);
    }

    /// Handle C0/C1 control characters.
    fn execute(&mut self, byte: u8) {
        match byte {
            // Bell (BEL)
            0x07 => {
                // Could trigger a visual/audio bell
            }
            // Backspace (BS)
            0x08 => self.screen.backspace(),
            // Horizontal Tab (HT)
            0x09 => self.screen.tab(),
            // Line Feed (LF), Vertical Tab (VT), Form Feed (FF)
            0x0A | 0x0B | 0x0C => self.screen.line_feed(),
            // Carriage Return (CR)
            0x0D => self.screen.carriage_return(),
            // Shift Out (SO) - switch to G1 character set
            0x0E => {}
            // Shift In (SI) - switch to G0 character set
            0x0F => {}
            _ => {}
        }
    }

    /// Handle device control strings.
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // DCS sequences - not commonly used
    }

    /// Handle data within device control strings.
    fn put(&mut self, _byte: u8) {
        // DCS data - not commonly used
    }

    /// Handle end of device control string.
    fn unhook(&mut self) {
        // DCS end - not commonly used
    }

    /// Handle operating system commands (OSC).
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let _ = bell_terminated;
        if params.is_empty() {
            return;
        }

        // Parse the first parameter as the OSC command number
        let cmd = match std::str::from_utf8(params[0]) {
            Ok(s) => s.parse::<u8>().unwrap_or(255),
            Err(_) => return,
        };

        match cmd {
            // Set window title and icon name
            0 | 2 => {
                if params.len() > 1 {
                    if let Ok(title) = std::str::from_utf8(params[1]) {
                        self.pending_title = Some(title.to_string());
                    }
                }
            }
            // Set icon name only
            1 => {}
            // Set color palette (4;index;color)
            4 => {}
            // Hyperlink (8;;url)
            8 => {}
            _ => {}
        }
    }

    /// Handle CSI (Control Sequence Introducer) commands.
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params: Vec<u16> = params.iter().map(|p| p[0]).collect();

        // Check for intermediate characters that modify behavior
        let has_question = intermediates.contains(&b'?');
        let has_greater = intermediates.contains(&b'>');
        let has_space = intermediates.contains(&b' ');
        let _ = (has_greater, has_space);

        match action {
            // Cursor Up (CUU)
            'A' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_up(n);
            }
            // Cursor Down (CUD)
            'B' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_down(n);
            }
            // Cursor Forward (CUF)
            'C' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_forward(n);
            }
            // Cursor Back (CUB)
            'D' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_backward(n);
            }
            // Cursor Next Line (CNL)
            'E' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_down(n);
                self.screen.carriage_return();
            }
            // Cursor Previous Line (CPL)
            'F' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_up(n);
                self.screen.carriage_return();
            }
            // Cursor Horizontal Absolute (CHA)
            'G' => {
                let col = params.first().copied().unwrap_or(1);
                let (_, row) = {
                    let cursor = self.screen.cursor();
                    (cursor.col, cursor.row)
                };
                self.screen.move_cursor(row + 1, col);
            }
            // Cursor Position (CUP) / Horizontal and Vertical Position (HVP)
            'H' | 'f' => {
                let row = params.first().copied().unwrap_or(1);
                let col = params.get(1).copied().unwrap_or(1);
                self.screen.move_cursor(row, col);
            }
            // Erase in Display (ED)
            'J' => {
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.screen.erase_to_end_of_screen(),
                    1 => self.screen.erase_to_start_of_screen(),
                    2 | 3 => {
                        self.screen.erase_screen();
                        if mode == 3 {
                            self.screen.clear_scrollback();
                        }
                    }
                    _ => {}
                }
            }
            // Erase in Line (EL)
            'K' => {
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => self.screen.erase_to_end_of_line(),
                    1 => self.screen.erase_to_start_of_line(),
                    2 => self.screen.erase_line(),
                    _ => {}
                }
            }
            // Insert Lines (IL)
            'L' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.insert_lines(n);
            }
            // Delete Lines (DL)
            'M' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.delete_lines(n);
            }
            // Insert Characters (ICH)
            '@' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.insert_chars(n);
            }
            // Delete Characters (DCH)
            'P' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.delete_chars(n);
            }
            // Scroll Up (SU)
            'S' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.scroll_up(n);
            }
            // Scroll Down (SD)
            'T' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.scroll_down(n);
            }
            // Erase Characters (ECH)
            'X' => {
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let cursor = self.screen.cursor();
                let row = cursor.row as usize;
                let _col = cursor.col as usize;
                let cells = self.screen.cells();
                if row < cells.len() {
                    // We need to clear n characters starting at cursor
                    // This requires mutable access, so we'll do it via the screen
                }
                // For now, erase to end of line if n is large
                let _ = n;
                self.screen.erase_to_end_of_line();
            }
            // Cursor Vertical Absolute (VPA)
            'd' => {
                let row = params.first().copied().unwrap_or(1);
                let col = self.screen.cursor().col + 1;
                self.screen.move_cursor(row, col);
            }
            // Horizontal Position Relative (HPR)
            'a' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_forward(n);
            }
            // Vertical Position Relative (VPR)
            'e' => {
                let n = params.first().copied().unwrap_or(1).max(1);
                self.screen.cursor_down(n);
            }
            // Tab Clear (TBC)
            'g' => {
                // Tab stop management - not implemented
            }
            // Set Mode (SM) / Reset Mode (RM)
            'h' | 'l' => {
                let set = action == 'h';
                if has_question {
                    // DEC private modes
                    for param in &params {
                        match *param {
                            // Cursor Keys Mode (DECCKM)
                            1 => {}
                            // 132 Column Mode (DECCOLM)
                            3 => {}
                            // Origin Mode (DECOM)
                            6 => {}
                            // Auto-wrap Mode (DECAWM)
                            7 => {}
                            // Cursor Visibility (DECTCEM)
                            25 => self.screen.set_cursor_visible(set),
                            // Alternate Screen Buffer
                            47 | 1047 | 1049 => {
                                // Alternate screen buffer - simplified handling
                                if set {
                                    self.screen.erase_screen();
                                }
                            }
                            // Bracketed Paste Mode
                            2004 => {}
                            _ => {}
                        }
                    }
                } else {
                    // ANSI modes
                    for param in &params {
                        match *param {
                            // Insert Mode (IRM)
                            4 => {}
                            // Automatic Newline (LNM)
                            20 => {}
                            _ => {}
                        }
                    }
                }
            }
            // Select Graphic Rendition (SGR)
            'm' => {
                if params.is_empty() {
                    self.screen.set_attr(0);
                    return;
                }

                let mut iter = params.iter().peekable();
                while let Some(&param) = iter.next() {
                    match param {
                        // Extended foreground color
                        38 => {
                            if let Some(&&color_type) = iter.peek() {
                                iter.next();
                                match color_type {
                                    // 256 color
                                    5 => {
                                        if let Some(&&idx) = iter.peek() {
                                            iter.next();
                                            self.screen.set_fg(Color::Indexed(idx as u8));
                                        }
                                    }
                                    // True color
                                    2 => {
                                        let r = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        let g = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        let b = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        self.screen.set_fg(Color::Rgb(r, g, b));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // Extended background color
                        48 => {
                            if let Some(&&color_type) = iter.peek() {
                                iter.next();
                                match color_type {
                                    // 256 color
                                    5 => {
                                        if let Some(&&idx) = iter.peek() {
                                            iter.next();
                                            self.screen.set_bg(Color::Indexed(idx as u8));
                                        }
                                    }
                                    // True color
                                    2 => {
                                        let r = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        let g = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        let b = iter.next().map(|&v| v as u8).unwrap_or(0);
                                        self.screen.set_bg(Color::Rgb(r, g, b));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => self.screen.set_attr(param),
                    }
                }
            }
            // Device Status Report (DSR)
            'n' => {
                // Would need to send response back to PTY
            }
            // Set Scroll Region (DECSTBM)
            'r' => {
                let top = params.first().copied().unwrap_or(1);
                let bottom = params.get(1).copied().unwrap_or(self.screen.size().1);
                self.screen.set_scroll_region(top, bottom);
                self.screen.move_cursor(1, 1);
            }
            // Save Cursor Position (DECSC)
            's' => {
                if !has_question {
                    self.screen.save_cursor();
                }
            }
            // Restore Cursor Position (DECRC)
            'u' => {
                self.screen.restore_cursor();
            }
            // Soft Terminal Reset (DECSTR)
            'p' => {
                if intermediates.contains(&b'!') {
                    self.screen.reset();
                }
            }
            _ => {
                // Unhandled CSI sequence
                log::trace!(
                    "Unhandled CSI: {:?} {:?} {:?}",
                    params,
                    intermediates,
                    action
                );
            }
        }
    }

    /// Handle escape sequences.
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            // Reset (RIS)
            ([], b'c') => self.screen.reset(),
            // Save Cursor (DECSC)
            ([], b'7') => self.screen.save_cursor(),
            // Restore Cursor (DECRC)
            ([], b'8') => self.screen.restore_cursor(),
            // Reverse Index (RI) - move up one line, scroll down if at top
            ([], b'M') => {
                let cursor = self.screen.cursor();
                if cursor.row == 0 {
                    self.screen.scroll_down(1);
                } else {
                    self.screen.cursor_up(1);
                }
            }
            // Next Line (NEL)
            ([], b'E') => {
                self.screen.carriage_return();
                self.screen.line_feed();
            }
            // Horizontal Tab Set (HTS)
            ([], b'H') => {
                // Set tab stop at current position
            }
            // Character Set Designations
            (b"(", _) | (b")", _) | (b"*", _) | (b"+", _) => {
                // Character set selection - not commonly needed
            }
            _ => {
                log::trace!("Unhandled ESC: {:?} {:02x}", intermediates, byte);
            }
        }
    }
}
