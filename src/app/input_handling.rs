//! Input handling for the Ferrite application.
//!
//! This module contains pre-render input consumption: undo/redo key interception,
//! move-line key consumption, smart paste, and auto-close bracket handling.

use super::helpers::modifier_symbol;
use super::FerriteApp;
use crate::state::Selection;
use eframe::egui;
use log::{debug, warn};

impl FerriteApp {
    /// Consume undo/redo keyboard events BEFORE rendering.
    ///
    /// This MUST be called before render_ui() to prevent egui's TextEdit from
    /// processing Ctrl+Z/Y with its built-in undo functionality. TextEdit has
    /// internal undo that would conflict with our custom undo system.
    ///
    /// By consuming these keys before the TextEdit is rendered, we ensure only
    /// our undo system handles the events.
    pub(crate) fn consume_undo_redo_keys(&mut self, ctx: &egui::Context) {
        // Skip if terminal has focus - let terminal handle all keyboard input
        if self.terminal_panel_state.terminal_has_focus {
            return;
        }

        let consumed_action: Option<bool> = ctx.input_mut(|i| {
            // Cmd+Shift+Z (macOS) / Ctrl+Shift+Z (Win/Linux): Redo (check first since it's more specific)
            if i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            ) {
                debug!(
                    "Keyboard shortcut: {}+Shift+Z (Redo) - consumed before render",
                    modifier_symbol()
                );
                return Some(false); // false = redo
            }
            // Cmd+Z (macOS) / Ctrl+Z (Win/Linux): Undo
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z) {
                debug!(
                    "Keyboard shortcut: {}+Z (Undo) - consumed before render",
                    modifier_symbol()
                );
                return Some(true); // true = undo
            }
            // Cmd+Y (macOS) / Ctrl+Y (Win/Linux): Redo
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y) {
                debug!(
                    "Keyboard shortcut: {}+Y (Redo) - consumed before render",
                    modifier_symbol()
                );
                return Some(false); // false = redo
            }
            None
        });

        // If undo/redo was consumed, handle it
        if let Some(is_undo) = consumed_action {
            if is_undo {
                self.handle_undo();
            } else {
                self.handle_redo();
            }
        }
    }

    /// Filter out Event::Cut when nothing is selected to prevent egui bug.
    ///
    /// egui's TextEdit has a bug where Ctrl+X with no selection cuts the entire
    /// document. This happens because eframe generates Event::Cut events which
    /// TextEdit processes. We filter out these events when there's no selection.
    ///
    /// Skips handling when the terminal has focus so the terminal widget can
    /// process clipboard shortcuts directly.
    pub(crate) fn filter_cut_event_if_no_selection(&mut self, ctx: &egui::Context) {
        // Skip editor cut/copy/paste handling when terminal has focus
        if self.terminal_panel_state.terminal_has_focus
            && self.terminal_panel_state.renaming_index.is_none()
        {
            return;
        }

        // Check if there's a selection in the active tab
        let has_selection = self
            .state
            .active_tab()
            .map(|tab| tab.cursors.primary().is_selection())
            .unwrap_or(false);

        // If no selection, filter out Event::Cut to prevent egui from cutting everything
        if !has_selection {
            ctx.input_mut(|i| {
                let had_cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut));
                i.events.retain(|e| !matches!(e, egui::Event::Cut));
                if had_cut {
                    debug!("Event::Cut filtered out - no selection");
                }
            });
        }
    }

    /// Consume the command palette shortcut (default Alt+Space) BEFORE render.
    ///
    /// On Windows, Alt+Space opens the system window menu. On some Linux WMs it
    /// opens the window actions menu. By consuming the key pre-render we prevent
    /// both the OS handler and egui TextEdit from seeing it.
    pub(crate) fn consume_command_palette_key(&mut self, ctx: &egui::Context) {
        if self.terminal_panel_state.terminal_has_focus {
            return;
        }

        let binding = self
            .state
            .settings
            .keyboard_shortcuts
            .get(crate::config::ShortcutCommand::CommandPalette);
        let egui_mods = binding.modifiers.to_egui();
        let egui_key = binding.key.to_egui();

        let consumed = ctx.input_mut(|i| i.consume_key(egui_mods, egui_key));
        if consumed {
            debug!("Command palette shortcut consumed before render");
            self.command_palette.toggle();
        }
    }

    /// Consume Alt+Arrow keys BEFORE render to prevent TextEdit from processing them.
    /// This must be called before the editor widget is rendered.
    /// Returns the direction to move (-1 for up, 1 for down) if a move was requested.
    pub(crate) fn consume_move_line_keys(&mut self, ctx: &egui::Context) -> Option<isize> {
        // Skip if terminal has focus - let terminal handle its own input
        if self.terminal_panel_state.terminal_has_focus {
            return None;
        }

        ctx.input_mut(|i| {
            // Alt+Up: Move line up
            if i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp) {
                debug!("Keyboard shortcut: Alt+Up (Move Line Up) - consumed before render");
                return Some(-1);
            }
            // Alt+Down: Move line down
            if i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown) {
                debug!("Keyboard shortcut: Alt+Down (Move Line Down) - consumed before render");
                return Some(1);
            }
            None
        })
    }

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Smart Paste for Links and Images
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

    /// Check if a string looks like a URL.
    ///
    /// Returns true for strings starting with common URL schemes:
    /// - `http://` or `https://`
    /// - Other schemes like `ftp://`, `file://`, etc.
    pub(crate) fn is_url(s: &str) -> bool {
        let s = s.trim();
        if s.is_empty() {
            return false;
        }

        // Check for common URL schemes
        if s.starts_with("http://") || s.starts_with("https://") {
            return true;
        }

        // Check for other valid URL schemes (alphanumeric + some chars, followed by ://)
        // Examples: ftp://, file://, mailto:, data:
        if let Some(colon_pos) = s.find(':') {
            let scheme = &s[..colon_pos];
            // Scheme must be alphanumeric or contain +, -, .
            // and must be followed by //
            if !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
                && scheme
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_alphabetic())
                    .unwrap_or(false)
            {
                // Check for :// pattern
                if s.len() > colon_pos + 2 && &s[colon_pos..colon_pos + 3] == "://" {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a URL points to an image based on file extension.
    ///
    /// Checks for common image extensions: .png, .jpg, .jpeg, .gif, .webp, .svg, .bmp
    /// The check is case-insensitive and handles URLs with query strings.
    pub(crate) fn is_image_url(s: &str) -> bool {
        if !Self::is_url(s) {
            return false;
        }

        let s = s.trim();

        // Remove query string and fragment for extension check
        let path = s.split('?').next().unwrap_or(s);
        let path = path.split('#').next().unwrap_or(path);

        // Get the extension (case-insensitive)
        let path_lower = path.to_lowercase();

        path_lower.ends_with(".png")
            || path_lower.ends_with(".jpg")
            || path_lower.ends_with(".jpeg")
            || path_lower.ends_with(".gif")
            || path_lower.ends_with(".webp")
            || path_lower.ends_with(".svg")
            || path_lower.ends_with(".bmp")
            || path_lower.ends_with(".ico")
            || path_lower.ends_with(".tiff")
            || path_lower.ends_with(".tif")
    }

    /// Consume paste events BEFORE render to implement smart paste behavior.
    ///
    /// Smart paste transforms paste behavior based on context:
    /// - Pasting a URL with text selected: Creates markdown link `[selected](url)`
    /// - Pasting an image URL with no selection: Creates markdown image `![](url)`
    /// - Otherwise: Normal paste behavior
    ///
    /// Uses FerriteEditor's selection state (which is authoritative) rather than
    /// tab.cursors which may be stale.
    ///
    /// Returns true if a paste event was consumed and handled with smart behavior.
    pub(crate) fn consume_smart_paste(&mut self, ctx: &egui::Context) -> bool {
        use crate::editor::get_ferrite_editor_mut;

        if self.terminal_panel_state.terminal_has_focus
            && self.terminal_panel_state.renaming_index.is_none()
        {
            return false;
        }

        let Some(tab) = self.state.active_tab() else {
            return false;
        };
        let tab_id = tab.id;
        let content = tab.content.clone();

        // Query FerriteEditor for authoritative selection state
        // This is the actual selection visible in the editor, not the potentially stale tab.cursors
        let editor_state: Option<(bool, String, usize, usize)> =
            get_ferrite_editor_mut(ctx, tab_id, |editor| {
                let has_sel = editor.has_selection();
                let selected_text = if has_sel {
                    editor.selected_text()
                } else {
                    String::new()
                };
                let cursor = editor.cursor();
                (has_sel, selected_text, cursor.line, cursor.column)
            });

        let (has_selection, selected_text_from_editor, cursor_line, cursor_col) = match editor_state
        {
            Some(state) => state,
            None => {
                // No FerriteEditor available - fall back to tab state
                let tab = self.state.active_tab().unwrap();
                (
                    false,
                    String::new(),
                    tab.cursor_position.0,
                    tab.cursor_position.1,
                )
            }
        };

        // Calculate cursor byte position from line/col
        let lines: Vec<&str> = content.split('\n').collect();
        let mut cursor_byte_pos = 0usize;
        for (i, line) in lines.iter().enumerate() {
            if i == cursor_line {
                cursor_byte_pos += cursor_col.min(line.len());
                break;
            }
            cursor_byte_pos += line.len() + 1;
        }
        cursor_byte_pos = cursor_byte_pos.min(content.len());

        // Scan for paste events
        #[derive(Debug)]
        enum SmartPasteAction {
            /// Create markdown link: [selected_text](url)
            CreateLink { url: String, selected_text: String },
            /// Create markdown image: ![](url)
            CreateImage { url: String },
        }

        let selected_text_clone = selected_text_from_editor.clone();
        let action: Option<(usize, SmartPasteAction)> = ctx.input(|input| {
            for (idx, event) in input.events.iter().enumerate() {
                if let egui::Event::Paste(pasted_text) = event {
                    let trimmed = pasted_text.trim();

                    // Case 1: URL pasted with text selected -> create markdown link
                    if has_selection && !selected_text_clone.is_empty() && Self::is_url(trimmed) {
                        return Some((
                            idx,
                            SmartPasteAction::CreateLink {
                                url: trimmed.to_string(),
                                selected_text: selected_text_clone.clone(),
                            },
                        ));
                    }

                    // Case 2: Image URL pasted with no selection -> create markdown image
                    if !has_selection && Self::is_image_url(trimmed) {
                        return Some((
                            idx,
                            SmartPasteAction::CreateImage {
                                url: trimmed.to_string(),
                            },
                        ));
                    }

                    // Case 3: Regular URL with no selection -> let normal paste handle it
                    // Case 4: Non-URL paste -> let normal paste handle it
                }
            }
            None
        });

        // If we found an action, consume the event and apply it
        if let Some((event_idx, action)) = action {
            // Remove the paste event to prevent FerriteEditor from handling it
            ctx.input_mut(|input| {
                if event_idx < input.events.len() {
                    input.events.remove(event_idx);
                }
            });

            // Get mutable access to tab
            let tab = self.state.active_tab_mut().unwrap();
            let old_content = tab.content.clone();
            let old_cursor = tab.cursors.primary().head;

            match action {
                SmartPasteAction::CreateLink { url, selected_text } => {
                    // Find the selected text near the cursor position in content
                    let sel_bytes = selected_text.len();
                    let search_start = cursor_byte_pos.saturating_sub(sel_bytes + 20);
                    let search_end = (cursor_byte_pos + sel_bytes + 20).min(content.len());
                    let search_region = &content[search_start..search_end];

                    if let Some(found_offset) = search_region.find(&selected_text) {
                        let start_byte = search_start + found_offset;
                        let end_byte = start_byte + sel_bytes;

                        // Build markdown link: [selected_text](url)
                        let link = format!("[{}]({})", selected_text, url);
                        let link_len = link.chars().count();

                        // Replace selection with link
                        tab.content.replace_range(start_byte..end_byte, &link);

                        // Position cursor after the link
                        let start_char = tab.content[..start_byte].chars().count();
                        let new_cursor_pos = start_char + link_len;
                        tab.pending_cursor_restore = Some(new_cursor_pos);
                        tab.cursors
                            .set_single(crate::state::Selection::cursor(new_cursor_pos));
                        tab.sync_cursor_from_primary();

                        // Record for undo
                        tab.record_edit(old_content, old_cursor);

                        debug!(
                            "Smart paste: Created link [{}]({}) at byte {}",
                            selected_text, url, start_byte
                        );
                    } else {
                        warn!(
                            "Smart paste: Could not find selected text '{}' near cursor",
                            selected_text
                        );
                    }
                }
                SmartPasteAction::CreateImage { url } => {
                    // Build markdown image: ![](url)
                    let image = format!("![]({})", url);
                    let image_len = image.chars().count();

                    // Insert at cursor byte position
                    tab.content.insert_str(cursor_byte_pos, &image);

                    // Position cursor after the image
                    let cursor_char_pos = tab.content[..cursor_byte_pos].chars().count();
                    let new_cursor_pos = cursor_char_pos + image_len;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors
                        .set_single(crate::state::Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    // Record for undo
                    tab.record_edit(old_content, old_cursor);

                    debug!(
                        "Smart paste: Created image ![](url) with url='{}' at line {} col {}",
                        url, cursor_line, cursor_col
                    );
                }
            }

            return true;
        }

        false
    }

    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    // Auto-close Brackets & Quotes
    // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

    /// Get the closing character for an opener, if it's a valid opener.
    pub(crate) fn get_closing_bracket(opener: char) -> Option<char> {
        match opener {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            '`' => Some('`'),
            _ => None,
        }
    }

    /// Check if a character is a closing bracket/quote.
    pub(crate) fn is_closing_bracket(ch: char) -> bool {
        matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
    }

    /// Handle auto-close brackets BEFORE render.
    ///
    /// This handles two cases that require consuming input events before TextEdit:
    /// 1. Skip-over: When typing a closer and the next character is the same closer,
    ///    move cursor forward instead of inserting a duplicate.
    /// 2. Selection wrapping: When typing an opener with text selected,
    ///    wrap the selection with the bracket pair.
    ///
    /// Returns true if an event was consumed and handled.
    pub(crate) fn handle_auto_close_pre_render(&mut self, ctx: &egui::Context) -> bool {
        // Skip if terminal has focus - let terminal handle its own input
        if self.terminal_panel_state.terminal_has_focus {
            return false;
        }

        if !self.state.settings.auto_close_brackets {
            return false;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return false;
        };

        // Get cursor info upfront to avoid borrow issues
        let primary = tab.cursors.primary();
        let cursor_char_pos = primary.head;
        let has_selection = primary.is_selection();
        let selection_range = if has_selection {
            Some(primary.range())
        } else {
            None
        };

        // Get content for analysis
        let content = tab.content.clone();

        // Helper to convert char position to byte position
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        // First, check input events to determine what action to take (if any)
        #[derive(Debug)]
        enum AutoCloseAction {
            WrapSelection { opener: char, closer: char },
            SkipOver { closer: char },
        }

        let action: Option<(usize, AutoCloseAction)> = ctx.input(|input| {
            for (idx, event) in input.events.iter().enumerate() {
                if let egui::Event::Text(text) = event {
                    // Only handle single-character input
                    if text.chars().count() != 1 {
                        continue;
                    }

                    let ch = text.chars().next().unwrap();

                    // Case 1: Selection wrapping with opener
                    if has_selection {
                        if let Some(closer) = Self::get_closing_bracket(ch) {
                            return Some((
                                idx,
                                AutoCloseAction::WrapSelection { opener: ch, closer },
                            ));
                        }
                    }

                    // Case 2: Skip-over for closing brackets
                    if !has_selection && Self::is_closing_bracket(ch) {
                        // Check if the next character is the same closer
                        let cursor_byte = char_to_byte(&content, cursor_char_pos);
                        let next_char = content[cursor_byte..].chars().next();

                        if next_char == Some(ch) {
                            return Some((idx, AutoCloseAction::SkipOver { closer: ch }));
                        }
                    }
                }
            }
            None
        });

        // If we found an action, consume the event and apply it
        if let Some((event_idx, action)) = action {
            // Remove the event first
            ctx.input_mut(|input| {
                input.events.remove(event_idx);
            });

            // Get mutable tab reference again
            let tab = self.state.active_tab_mut().unwrap();

            match action {
                AutoCloseAction::WrapSelection { opener, closer } => {
                    let (start_char, end_char) = selection_range.unwrap();
                    let start_byte = char_to_byte(&tab.content, start_char);
                    let end_byte = char_to_byte(&tab.content, end_char);

                    // Get selected text
                    let selected_text = tab.content[start_byte..end_byte].to_string();
                    let selected_len = selected_text.chars().count();

                    // Save for undo
                    let old_content = tab.content.clone();
                    let old_cursor = cursor_char_pos;

                    // Build wrapped text: opener + selected + closer
                    let wrapped = format!("{}{}{}", opener, selected_text, closer);

                    // Replace selection with wrapped text
                    tab.content.replace_range(start_byte..end_byte, &wrapped);

                    // Position cursor after the closing bracket
                    let new_cursor_pos = start_char + 1 + selected_len + 1;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    // Record for undo
                    tab.record_edit(old_content, old_cursor);

                    debug!(
                        "Auto-close: Wrapped selection '{}' with {}...{}",
                        selected_text, opener, closer
                    );
                }
                AutoCloseAction::SkipOver { closer } => {
                    // Just move cursor forward, don't insert
                    let new_cursor_pos = cursor_char_pos + 1;
                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors.set_single(Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    debug!("Auto-close: Skip-over for '{}'", closer);
                }
            }

            return true;
        }

        false
    }

    /// Handle auto-close brackets AFTER render.
    ///
    /// This handles auto-pair insertion: When an opener was just typed (no selection),
    /// insert the closing bracket immediately after and position cursor between them.
    ///
    /// This runs after TextEdit has processed input, so we detect what was just typed
    /// by comparing the current state with the pre-render snapshot.
    pub(crate) fn handle_auto_close_post_render(
        &mut self,
        pre_render_content: &str,
        _pre_render_cursor: usize,
    ) {
        if !self.state.settings.auto_close_brackets {
            return;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Check if exactly one character was inserted at the cursor position
        let content_len_diff =
            tab.content.chars().count() as isize - pre_render_content.chars().count() as isize;

        if content_len_diff != 1 {
            return; // Not a single character insertion
        }

        // Get current cursor position (should be after the just-typed character)
        let cursor_char_pos = tab.cursors.primary().head;

        // The just-typed character is at cursor_pos - 1
        if cursor_char_pos == 0 {
            return;
        }

        // Helper to convert char position to byte position
        let char_to_byte = |text: &str, char_idx: usize| -> usize {
            text.char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(text.len())
        };

        let prev_char_byte = char_to_byte(&tab.content, cursor_char_pos - 1);
        let cursor_byte = char_to_byte(&tab.content, cursor_char_pos);

        let just_typed = tab.content[prev_char_byte..cursor_byte].chars().next();

        if let Some(opener) = just_typed {
            if let Some(closer) = Self::get_closing_bracket(opener) {
                // For quotes, check context to avoid unwanted auto-close
                // Don't auto-close if the character before the opener is alphanumeric
                // (e.g., don't auto-close after typing can't -> can't')
                if matches!(opener, '"' | '\'' | '`') {
                    if cursor_char_pos >= 2 {
                        let prev_prev_byte = char_to_byte(&tab.content, cursor_char_pos - 2);
                        let prev_char = tab.content[prev_prev_byte..prev_char_byte].chars().next();
                        if let Some(c) = prev_char {
                            if c.is_alphanumeric() {
                                return; // Don't auto-close after alphanumeric
                            }
                        }
                    }
                }

                // Insert the closing bracket at cursor position
                tab.content.insert(cursor_byte, closer);

                // Keep cursor between the brackets (position hasn't changed)
                // TextEdit will update, but we want cursor to stay where it is
                tab.pending_cursor_restore = Some(cursor_char_pos);

                debug!("Auto-close: Inserted '{}' after '{}'", closer, opener);
            }
        }
    }
}
