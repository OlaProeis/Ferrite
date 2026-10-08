//! Input handling for the Ferrite application.
//!
//! This module contains pre-render input consumption: undo/redo key interception,
//! move-line key consumption, smart paste, and auto-close bracket handling.

use super::helpers::modifier_symbol;
use super::FerriteApp;
use crate::config::ViewMode;
use crate::state::Selection;
use crate::string_utils::{char_index_to_byte_index, rope_line_col_to_char_index};
use arboard::Clipboard;
use eframe::egui;
use log::{debug, warn};
use rust_i18n::t;

/// True when this frame may need smart-paste handling (paste event or Ctrl/Cmd+V).
fn events_indicate_paste(events: &[egui::Event]) -> bool {
    events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
        || events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key {
                    key: egui::Key::V,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if modifiers.command && !modifiers.shift
            )
        })
}

/// True when this frame may need pre-render auto-close bracket handling.
fn events_indicate_auto_close(events: &[egui::Event], has_selection: bool) -> bool {
    events.iter().any(|e| {
        let egui::Event::Text(text) = e else {
            return false;
        };
        if text.chars().count() != 1 {
            return false;
        }
        let ch = text.chars().next().unwrap();
        if has_selection {
            FerriteApp::get_closing_bracket(ch).is_some()
        } else {
            FerriteApp::is_closing_bracket(ch)
        }
    })
}

/// Map a (line, character-column) caret to a byte offset in `content`.
fn cursor_byte_from_line_col(content: &str, cursor_line: usize, cursor_col: usize) -> usize {
    let char_idx = rope_line_col_to_char_index(content, cursor_line, cursor_col);
    char_index_to_byte_index(content, char_idx)
}

/// Replace a char-index selection with a markdown link; returns new cursor char position.
fn replace_selection_with_link(
    content: &mut String,
    start_char: usize,
    end_char: usize,
    url: &str,
) -> usize {
    let start_byte = char_index_to_byte_index(content, start_char);
    let end_byte = char_index_to_byte_index(content, end_char);
    let selected_text = content[start_byte..end_byte].to_string();
    let link = format!("[{}]({})", selected_text, url);
    let link_len = link.chars().count();
    content.replace_range(start_byte..end_byte, &link);
    start_char + link_len
}

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
                // Check for :// pattern. Use `str::get` (not direct slicing) because the
                // bytes after `colon_pos` can be the start of a multi-byte UTF-8 codepoint
                // when the user pastes mixed-script text such as `Hebrew: שלום עולם`.
                // Direct byte-slicing would panic on non-char-boundary indices and abort
                // the process in release builds (`panic = "abort"`).
                if s.get(colon_pos..colon_pos + 3) == Some("://") {
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
    /// - Pasting OS clipboard image bytes: Saves under `./assets/` and inserts `![](assets/…)`
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

        if !ctx.input(|input| events_indicate_paste(&input.events)) {
            return false;
        }

        // Case 0: OS clipboard image pixels → `./assets/` + markdown.
        // Prefer this before URL smart-paste: browser "Copy image" often provides
        // both RGBA bytes and an https URL; local-first policy wins.
        if self.try_consume_clipboard_image_paste(ctx) {
            return true;
        }

        // Query FerriteEditor for authoritative selection state
        // This is the actual selection visible in the editor, not the potentially stale tab.cursors
        let editor_state: Option<(bool, String, usize, usize, Option<(usize, usize)>)> =
            get_ferrite_editor_mut(ctx, tab_id, |editor| {
                let has_sel = editor.has_selection();
                let selected_text = if has_sel {
                    editor.selected_text()
                } else {
                    String::new()
                };
                let cursor = editor.cursor();
                let selection_chars = if has_sel {
                    let sel = editor.primary_selection();
                    let (start, end) = sel.ordered();
                    Some((
                        editor.cursor_to_char_pos(start),
                        editor.cursor_to_char_pos(end),
                    ))
                } else {
                    None
                };
                (
                    has_sel,
                    selected_text,
                    cursor.line,
                    cursor.column,
                    selection_chars,
                )
            });

        let (
            has_selection,
            selected_text_from_editor,
            cursor_line,
            cursor_col,
            selection_char_range,
        ) = match editor_state {
            Some(state) => state,
            None => {
                // No FerriteEditor available - fall back to tab state
                let tab = self.state.active_tab().unwrap();
                (
                    false,
                    String::new(),
                    tab.cursor_position.0,
                    tab.cursor_position.1,
                    None,
                )
            }
        };

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
                    // (markdown link only — does not fetch remote image bytes)
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
                    let Some((start_char, end_char)) = selection_char_range else {
                        warn!(
                            "Smart paste: No selection char range for link paste of '{}'",
                            selected_text
                        );
                        return true;
                    };

                    let start_byte = char_index_to_byte_index(&tab.content, start_char);
                    let end_byte = char_index_to_byte_index(&tab.content, end_char);
                    if tab.content.get(start_byte..end_byte) != Some(selected_text.as_str()) {
                        warn!(
                            "Smart paste: Selection text mismatch at bytes {}..{}",
                            start_byte, end_byte
                        );
                        return true;
                    }

                    let new_cursor_pos =
                        replace_selection_with_link(&mut tab.content, start_char, end_char, &url);

                    tab.pending_cursor_restore = Some(new_cursor_pos);
                    tab.cursors
                        .set_single(crate::state::Selection::cursor(new_cursor_pos));
                    tab.sync_cursor_from_primary();

                    tab.record_edit(old_content, old_cursor);

                    debug!(
                        "Smart paste: Created link [{}]({}) at char {}..{}",
                        selected_text, url, start_char, end_char
                    );
                }
                SmartPasteAction::CreateImage { url } => {
                    // Build markdown image: ![](url)
                    let image = format!("![]({})", url);
                    let image_len = image.chars().count();

                    let cursor_byte_pos =
                        cursor_byte_from_line_col(&tab.content, cursor_line, cursor_col);

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

    /// When Ctrl/Cmd+V is pressed with image pixels on the clipboard, save under
    /// `./assets/` and insert markdown. Prefer raw / split as targets; skip when
    /// Rendered + preview lock would block preview-targeted mutation.
    ///
    /// Plain prose `Event::Paste` still wins over image bytes. URL paste text does
    /// **not** block image bytes (browser "Copy image" dual format).
    fn try_consume_clipboard_image_paste(&mut self, ctx: &egui::Context) -> bool {
        let Some(tab) = self.state.active_tab() else {
            return false;
        };

        if tab.is_special() || tab.is_image_viewer() || tab.is_pdf_viewer() {
            return false;
        }

        // Preview lock: do not mutate when paste would target locked rendered preview.
        // Raw and Split still accept paste into the source buffer.
        if tab.view_mode == ViewMode::Rendered && tab.is_preview_locked() {
            return false;
        }

        // Image-only clipboards often produce no Event::Paste — detect the paste
        // shortcut from the raw key event so we can exclude:
        // - Shift (Ctrl+Shift+V is "paste without formatting" / other bindings),
        // - key auto-repeat (holding Ctrl+V must not spam asset files).
        let paste_requested = ctx.input(|input| {
            input.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::V,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } if modifiers.command && !modifiers.shift
                )
            })
        });
        if !paste_requested {
            return false;
        }

        // Focus guard: when another widget owns keyboard focus (Find bar,
        // settings field, an active rendered-block TextEdit, a rename box…)
        // Ctrl+V belongs to that widget — pasting an image into the document
        // from there is never the user's intent. Allowed: no focus at all
        // (e.g. rendered preview) or the raw FerriteEditor itself.
        let tab_id = tab.id;
        if let Some(focused_id) = ctx.memory(|m| m.focused()) {
            let editor_owns_focus = crate::editor::get_ferrite_editor_mut(ctx, tab_id, |editor| {
                editor.last_widget_id == Some(focused_id)
            })
            .unwrap_or(false);
            if !editor_owns_focus {
                return false;
            }
        }

        // Prefer non-URL text paste (prose / code) over image bytes when both exist.
        let paste_text = ctx.input(|input| {
            input.events.iter().find_map(|e| match e {
                egui::Event::Paste(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                _ => None,
            })
        });
        if let Some(ref text) = paste_text {
            if !Self::is_url(text) {
                return false;
            }
        }

        // Sync FerriteEditor cursor into tab so insert lands at the visible caret.
        if let Some((line, col)) = crate::editor::get_ferrite_editor_mut(ctx, tab_id, |editor| {
            let c = editor.cursor();
            (c.line, c.column)
        }) {
            if let Some(tab) = self.state.active_tab_mut() {
                tab.cursor_position = (line, col);
            }
        }

        let image = match Clipboard::new().and_then(|mut cb| cb.get_image()) {
            Ok(img) => img,
            Err(_) => return false,
        };

        let width = image.width;
        let height = image.height;
        let rgba = image.bytes.as_ref();

        match self.handle_clipboard_image_paste(ctx, width, height, rgba) {
            Ok(None) => {
                ctx.input_mut(|input| {
                    let _ = input.consume_key(egui::Modifiers::COMMAND, egui::Key::V);
                    input.events.retain(|e| !matches!(e, egui::Event::Paste(_)));
                });
                true
            }
            Ok(Some(_)) => {
                // Consume paste shortcut / stray Paste events so the editor does not also paste.
                ctx.input_mut(|input| {
                    let _ = input.consume_key(egui::Modifiers::COMMAND, egui::Key::V);
                    input.events.retain(|e| !matches!(e, egui::Event::Paste(_)));
                });

                let time = self.get_app_time();
                self.state
                    .show_toast(t!("notification.image_added").to_string(), time, 2.5);
                debug!("Smart paste: Saved clipboard image under assets/");
                true
            }
            Err(e) => {
                warn!("Failed to paste clipboard image: {}", e);
                self.state
                    .show_error(t!("error.image_failed", error = e).to_string());
                // Still consume so we don't fall through to a no-op text paste.
                ctx.input_mut(|input| {
                    let _ = input.consume_key(egui::Modifiers::COMMAND, egui::Key::V);
                    input.events.retain(|e| !matches!(e, egui::Event::Paste(_)));
                });
                true
            }
        }
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

        if !ctx.input(|input| events_indicate_auto_close(&input.events, has_selection)) {
            return false;
        }

        // Get content for skip-over peek (only when a bracket key is present this frame)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_url_recognises_common_schemes() {
        assert!(FerriteApp::is_url("http://example.com"));
        assert!(FerriteApp::is_url("https://example.com/path?q=1#frag"));
        assert!(FerriteApp::is_url("ftp://files.example.com"));
        assert!(FerriteApp::is_url("file:///home/user/x.md"));
        assert!(FerriteApp::is_url("  https://example.com  ")); // trims
    }

    #[test]
    fn is_url_rejects_non_urls() {
        assert!(!FerriteApp::is_url(""));
        assert!(!FerriteApp::is_url("just plain text"));
        assert!(!FerriteApp::is_url("mailto:user@example.com")); // no `://`
        assert!(!FerriteApp::is_url("foo:bar")); // no `://`
        assert!(!FerriteApp::is_url("123://example.com")); // scheme must start alpha
    }

    /// Regression test for the v0.3.0 paste-crash:
    /// `is_url` on text whose first colon is followed by a multi-byte UTF-8
    /// codepoint (e.g. `Hebrew: שלום עולם`) used to panic with
    /// "byte index N is not a char boundary", aborting the process in a
    /// `panic = "abort"` release build (STATUS_STACK_BUFFER_OVERRUN).
    /// See `docs/technical/platform/v0.3.0-regression-matrix.md` Issue I-3.
    #[test]
    fn is_url_does_not_panic_on_mixed_script_text() {
        // Each of these previously panicked at `&s[colon_pos..colon_pos + 3]`
        // because the byte right after `:` was the start of a 2-/3-/4-byte UTF-8
        // codepoint, so `colon_pos + 3` landed mid-codepoint.
        assert!(!FerriteApp::is_url("Hebrew: שלום עולם")); // 2-byte char after `:`
        assert!(!FerriteApp::is_url("Bengali: আমি বাংলা")); // 3-byte char after `:`
        assert!(!FerriteApp::is_url("Hindi: नमस्ते दुनिया")); // 3-byte char after `:`
        assert!(!FerriteApp::is_url("Arabic: مرحبا بالعالم")); // 2-byte char after `:`
        assert!(!FerriteApp::is_url("note: 你好")); // 3-byte CJK after `:`
        assert!(!FerriteApp::is_url("emoji: 👨‍👩‍👧")); // 4-byte emoji after `:`
                                                   // Edge: single colon with nothing after
        assert!(!FerriteApp::is_url("a:"));
        // Edge: short scheme-like prefix that ends mid-multi-byte char
        assert!(!FerriteApp::is_url("a:ש"));
    }

    #[test]
    fn is_image_url_does_not_panic_on_mixed_script_text() {
        // is_image_url calls is_url first; the panic was reached via this path
        // (frame 21 in the v0.3.0-rc backtrace).
        assert!(!FerriteApp::is_image_url("Hebrew: שלום עולם"));
        assert!(!FerriteApp::is_image_url("emoji line 👨‍👩‍👧 here"));
    }

    #[test]
    fn is_image_url_recognises_image_extensions() {
        assert!(FerriteApp::is_image_url("https://example.com/photo.png"));
        assert!(FerriteApp::is_image_url("https://example.com/photo.JPG")); // case-insensitive
        assert!(FerriteApp::is_image_url("https://example.com/x.gif?v=2"));
        assert!(FerriteApp::is_image_url("https://example.com/x.webp#frag"));
        assert!(!FerriteApp::is_image_url("https://example.com/page.html"));
        assert!(!FerriteApp::is_image_url("not a url at all.png"));
    }

    #[test]
    fn replace_selection_with_link_multibyte_cjk_context() {
        let mut content = "日本語日本語日本語日本語 hello world".to_string();
        // 4×3 CJK chars + space = 13; "hello" occupies chars 13..18
        let new_cursor = replace_selection_with_link(&mut content, 13, 18, "https://x.y");
        assert_eq!(
            content,
            "日本語日本語日本語日本語 [hello](https://x.y) world"
        );
        assert_eq!(new_cursor, 13 + "[hello](https://x.y)".chars().count());
    }

    #[test]
    fn replace_selection_with_link_after_emoji() {
        let prefix = "👨‍👩‍👧 ";
        let mut content = format!("{}hello", prefix);
        let hello_start = prefix.chars().count();
        let hello_end = hello_start + "hello".chars().count();
        let new_cursor =
            replace_selection_with_link(&mut content, hello_start, hello_end, "https://x.y");
        assert_eq!(content, format!("{}[hello](https://x.y)", prefix));
        assert_eq!(
            new_cursor,
            hello_start + "[hello](https://x.y)".chars().count()
        );
    }

    #[test]
    fn replace_selection_with_link_ascii_regression() {
        let mut content = "see hello there".to_string();
        let new_cursor = replace_selection_with_link(&mut content, 4, 9, "https://example.com");
        assert_eq!(content, "see [hello](https://example.com) there");
        assert_eq!(
            new_cursor,
            4 + "[hello](https://example.com)".chars().count()
        );
    }

    #[test]
    fn events_indicate_paste_only_on_paste_or_ctrl_v() {
        use eframe::egui::{Event, Key, Modifiers};

        assert!(!events_indicate_paste(&[]));
        assert!(!events_indicate_paste(&[Event::Copy]));
        assert!(events_indicate_paste(&[Event::Paste("https://x.y".into())]));
        assert!(events_indicate_paste(&[Event::Key {
            key: Key::V,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }]));
        assert!(!events_indicate_paste(&[Event::Key {
            key: Key::V,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
        }]));
    }

    #[test]
    fn events_indicate_auto_close_only_on_bracket_keys() {
        use eframe::egui::Event;

        assert!(!events_indicate_auto_close(&[], false));
        assert!(!events_indicate_auto_close(
            &[Event::Text("a".into())],
            false
        ));
        assert!(events_indicate_auto_close(
            &[Event::Text(")".into())],
            false
        ));
        assert!(events_indicate_auto_close(&[Event::Text("(".into())], true));
        assert!(!events_indicate_auto_close(
            &[Event::Text("(".into())],
            false
        ));
    }
}
