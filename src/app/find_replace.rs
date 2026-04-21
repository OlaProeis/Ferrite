//! Find and replace coordination for the Ferrite application.
//!
//! This module contains handlers for opening find/replace panel,
//! navigating matches, selecting occurrences, and replacing text.

use super::FerriteApp;
use eframe::egui;
use log::debug;
use rust_i18n::t;

impl FerriteApp {
    /// Handle opening the find panel.
    ///
    /// Opens the find panel, optionally in replace mode.
    pub(crate) fn handle_open_find(&mut self, replace_mode: bool) {
        self.state.ui.show_find_replace = true;
        self.state.ui.find_state.is_replace_mode = replace_mode;
        self.find_replace_panel.request_focus();

        // Trigger initial search if there's already a search term
        if !self.state.ui.find_state.search_term.is_empty() {
            // Clone content to avoid borrow conflict with find_state
            // This is only called when opening find panel, not on every keystroke
            let content = self.state.active_tab().map(|t| t.content.clone());
            if let Some(content) = content {
                let count = self.state.ui.find_state.find_matches(&content);
                if count > 0 {
                    self.state.ui.scroll_to_match = true;
                }
            }
        }

        debug!("Find panel opened, replace_mode: {}", replace_mode);
    }

    /// Handle find next match action.
    ///
    /// Works when find panel is open OR when there are existing matches from a previous search.
    /// This allows F3 to cycle through matches even after closing the find panel.
    pub(crate) fn handle_find_next(&mut self) {
        // Allow navigation if panel is open OR if there are matches from previous search
        if !self.state.ui.show_find_replace && self.state.ui.find_state.matches.is_empty() {
            return;
        }

        if let Some(idx) = self.state.ui.find_state.next_match() {
            self.state.ui.scroll_to_match = true;
            debug!("Find next: moved to match {}", idx + 1);
        }
    }

    /// Handle find previous match action.
    ///
    /// Works when find panel is open OR when there are existing matches from a previous search.
    /// This allows Shift+F3 to cycle through matches even after closing the find panel.
    pub(crate) fn handle_find_prev(&mut self) {
        // Allow navigation if panel is open OR if there are matches from previous search
        if !self.state.ui.show_find_replace && self.state.ui.find_state.matches.is_empty() {
            return;
        }

        if let Some(idx) = self.state.ui.find_state.prev_match() {
            self.state.ui.scroll_to_match = true;
            debug!("Find prev: moved to match {}", idx + 1);
        }
    }

    /// Handle Ctrl+D: Select next occurrence of current word/selection.
    ///
    /// VS Code-style behavior:
    /// - If no selection: select the word under cursor
    /// - If selection exists: find next occurrence and add cursor there
    pub(crate) fn handle_select_next_occurrence(&mut self) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Get the text to search for
        let search_text = match tab.get_primary_selection_text() {
            Some(text) if !text.is_empty() => text,
            _ => {
                // No word at cursor, try to select word under cursor first
                let primary_pos = tab.cursors.primary().head;
                if let Some((start, end)) = tab.word_range_at_position(primary_pos) {
                    // Select the word under cursor
                    tab.set_selection(start, end);
                    debug!("Selected word at cursor: {}..{}", start, end);
                }
                return;
            }
        };

        // Get the last selection's end position to search from
        let search_from = {
            let selections = tab.cursors.selections();
            // Find the rightmost selection to search after
            selections.iter().map(|s| s.end()).max().unwrap_or(0)
        };

        // Find next occurrence that doesn't overlap with existing selections
        if let Some((start, end)) = tab.find_next_occurrence(&search_text, search_from) {
            // Check if this occurrence is already selected
            let already_selected = tab
                .cursors
                .selections()
                .iter()
                .any(|s| s.start() == start && s.end() == end);

            if !already_selected {
                // Add new selection
                tab.add_selection(start, end);
                debug!(
                    "Added selection at {}..{}, now {} cursor(s)",
                    start,
                    end,
                    tab.cursor_count()
                );
            } else {
                debug!("All occurrences already selected");
            }
        } else {
            debug!("No more occurrences found for '{}'", search_text);
        }
    }

    /// Handle replace current match action.
    ///
    /// This uses the FerriteEditor's rope-based replace for better performance
    /// with large files.
    pub(crate) fn handle_replace_current(&mut self, ctx: &egui::Context) {
        use crate::editor::get_ferrite_editor_mut;

        let replacement = self.state.ui.find_state.replace_term.clone();

        // Get the active tab ID for FerriteEditor lookup
        let tab_id = self.state.active_tab().map(|t| t.id);

        if let Some(tab_id) = tab_id {
            // Use FerriteEditor's replace (efficient for large files)
            let replaced = get_ferrite_editor_mut(ctx, tab_id, |editor| {
                editor.replace_current_match(&replacement)
            });

            if replaced.unwrap_or(false) {
                // The editor content changed - sync back to Tab and re-search
                // to update match positions and count
                let new_content =
                    get_ferrite_editor_mut(ctx, tab_id, |editor| editor.buffer().to_string())
                        .unwrap_or_default();

                // Update Tab content
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.content = new_content.clone();
                }

                // Re-search to update match positions
                self.state.ui.find_state.find_matches(&new_content);

                let time = self.get_app_time();
                self.state
                    .show_toast(t!("notification.replaced").to_string(), time, 1.5);
                debug!("Replaced current match");
            }
        }
    }

    /// Handle replace all matches action.
    ///
    /// This uses the FerriteEditor's efficient batch replace for
    /// O(matches * log N) performance.
    pub(crate) fn handle_replace_all(&mut self, ctx: &egui::Context) {
        use crate::editor::get_ferrite_editor_mut;

        let replacement = self.state.ui.find_state.replace_term.clone();
        let match_count = self.state.ui.find_state.match_count();

        if match_count == 0 {
            return;
        }

        // Get the active tab ID for FerriteEditor lookup
        let tab_id = self.state.active_tab().map(|t| t.id);

        if let Some(tab_id) = tab_id {
            // Use FerriteEditor's batch replace (efficient for large files)
            let replaced_count = get_ferrite_editor_mut(ctx, tab_id, |editor| {
                editor.replace_all_matches(&replacement)
            });

            if let Some(count) = replaced_count {
                if count > 0 {
                    // The editor content changed - sync back to Tab
                    let new_content =
                        get_ferrite_editor_mut(ctx, tab_id, |editor| editor.buffer().to_string())
                            .unwrap_or_default();

                    // Update Tab content
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.content = new_content.clone();
                    }

                    // Re-search (will find 0 matches since all were replaced)
                    self.state.ui.find_state.find_matches(&new_content);

                    let time = self.get_app_time();
                    self.state.show_toast(
                        t!(
                            "notification.replaced_count",
                            count = count,
                            suffix = if count == 1 { "" } else { "s" }
                        )
                        .to_string(),
                        time,
                        2.0,
                    );
                    debug!("Replaced all {} matches", count);
                }
            }
        }
    }
}
