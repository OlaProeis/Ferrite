//! Line operations for the Ferrite application.
//!
//! This module contains handlers for go-to-line, duplicate line,
//! move line up/down, and delete line operations.

use super::FerriteApp;
use crate::config::ViewMode;
use log::{debug, warn};

impl FerriteApp {
    /// Handle opening the Go to Line dialog.
    pub(crate) fn handle_open_go_to_line(&mut self) {
        // Get current line and max line from active tab
        let Some(tab) = self.state.active_tab() else {
            return;
        };

        // Calculate current line (1-indexed) from cursor position
        let current_line = tab.cursor_position.0 + 1;

        // Calculate total line count
        let max_line = tab.content.lines().count().max(1);

        // Open the Go to Line dialog
        self.state.ui.go_to_line_dialog =
            Some(crate::ui::GoToLineDialog::new(current_line, max_line));
    }

    /// Handle navigating to a specific line number.
    pub(crate) fn handle_go_to_line(&mut self, target_line: usize) {
        // Get the active tab
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Calculate the character index for the start of the target line
        // target_line is 1-indexed, we need 0-indexed for content iteration
        let line_index = target_line.saturating_sub(1);
        let mut char_index = 0;
        let mut current_line = 0;

        for (idx, ch) in tab.content.char_indices() {
            if current_line == line_index {
                char_index = tab.content[..idx].chars().count();
                break;
            }
            if ch == '\n' {
                current_line += 1;
            }
        }

        // If we didn't find the line (end of file), go to last character
        if current_line < line_index {
            char_index = tab.content.chars().count();
        }

        // Update cursor position to the start of the target line
        tab.cursors
            .set_single(crate::state::Selection::cursor(char_index));
        tab.sync_cursor_from_primary();

        // Use the existing scroll_to_line mechanism to center the line in viewport
        // This is already handled by EditorWidget when pending_scroll_to_line is set
        self.pending_scroll_to_line = Some(target_line);

        debug!(
            "Go to Line: navigating to line {} (char index {})",
            target_line, char_index
        );
    }

    /// Handle duplicating the current line or selection.
    ///
    /// - If no selection: duplicates the entire current line below it
    /// - If selection: duplicates the selected text immediately after the selection
    ///
    /// Uses `cursor_position` (line, col) which is reliably synced from
    /// FerriteEditor, rather than `tab.cursors` which may be stale.
    pub(crate) fn handle_duplicate_line(&mut self) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        // Use cursor_position (line, col) which is reliably synced from FerriteEditor
        let (current_line_num, cursor_col) = tab.cursor_position;

        // Split into lines for manipulation
        let lines: Vec<&str> = tab.content.split('\n').collect();

        // Bounds check
        if current_line_num >= lines.len() {
            warn!(
                "Duplicate line: cursor line {} out of range (total {})",
                current_line_num,
                lines.len()
            );
            return;
        }

        // Get the current line content
        let line_content = lines[current_line_num];

        // Build new content with the duplicated line inserted after the current line
        let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len() + 1);
        let line_content_owned = line_content.to_string();
        for (i, line) in lines.iter().enumerate() {
            new_lines.push(line);
            if i == current_line_num {
                new_lines.push(&line_content_owned);
            }
        }

        let new_content = new_lines.join("\n");

        // Calculate new cursor position on the duplicated line (one line down, same column)
        let new_line_num = current_line_num + 1;
        let mut new_line_start = 0usize;
        for (i, line) in new_lines.iter().enumerate() {
            if i == new_line_num {
                break;
            }
            new_line_start += line.len() + 1; // +1 for newline
        }

        // Clamp column to new line length
        let new_line_len = new_lines.get(new_line_num).map(|l| l.len()).unwrap_or(0);
        let new_cursor_byte = new_line_start + cursor_col.min(new_line_len);
        let new_cursor_char = new_content[..new_cursor_byte.min(new_content.len())]
            .chars()
            .count();

        // Apply changes
        tab.content = new_content;

        // Use pending_cursor_restore to ensure the cursor position is applied
        tab.pending_cursor_restore = Some(new_cursor_char);

        // Also update internal state for consistency
        tab.cursors
            .set_single(crate::state::Selection::cursor(new_cursor_char));
        tab.sync_cursor_from_primary();

        // Record the edit for undo support
        tab.record_edit(old_content, old_cursor);

        debug!(
            "Duplicate line: line {} duplicated, cursor moved to line {} col {}",
            current_line_num, new_line_num, cursor_col
        );
    }

    /// Handle moving line(s) up or down.
    ///
    /// `direction`: -1 for up, 1 for down
    pub(crate) fn handle_move_line(&mut self, direction: isize) {
        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        // Get cursor position - cursor_position gives (line, column) directly
        let (current_line_num, cursor_col) = tab.cursor_position;
        let total_lines = tab.content.matches('\n').count() + 1;

        // Check boundaries
        if direction < 0 && current_line_num == 0 {
            return; // Can't move up from first line
        }
        if direction > 0 && current_line_num >= total_lines - 1 {
            return; // Can't move down from last line
        }

        // Split into lines for manipulation
        let lines: Vec<&str> = tab.content.split('\n').collect();
        let mut new_lines = lines.clone();

        // Perform the swap
        if direction < 0 {
            // Moving up: swap with line above
            new_lines.swap(current_line_num, current_line_num - 1);
        } else {
            // Moving down: swap with line below
            new_lines.swap(current_line_num, current_line_num + 1);
        }

        // Build new content
        let new_content = new_lines.join("\n");

        // Calculate new cursor position
        // The cursor should be on the same line content, which has moved
        let new_line_num = if direction < 0 {
            current_line_num - 1
        } else {
            current_line_num + 1
        };

        // Find byte offset of the new line position
        let mut new_line_start = 0usize;
        for (i, line) in new_lines.iter().enumerate() {
            if i == new_line_num {
                break;
            }
            new_line_start += line.len() + 1; // +1 for newline
        }

        // Calculate new cursor byte position (line start + column, clamped to line length)
        let new_line_len = new_lines.get(new_line_num).map(|l| l.len()).unwrap_or(0);
        let new_cursor_byte = new_line_start + cursor_col.min(new_line_len);

        // Convert byte position to character position
        let new_cursor_char = new_content[..new_cursor_byte].chars().count();

        debug!(
            "Move line: new_line_num={}, new_line_start={}, new_cursor_byte={}, new_cursor_char={}",
            new_line_num, new_line_start, new_cursor_byte, new_cursor_char
        );

        // Apply changes
        tab.content = new_content;

        // Use pending_cursor_restore to ensure the cursor position is applied
        // This is necessary because egui's TextEdit has its own cursor state
        // that would otherwise override our changes on the next frame
        tab.pending_cursor_restore = Some(new_cursor_char);

        // Also update internal state for consistency
        tab.cursors
            .set_single(crate::state::Selection::cursor(new_cursor_char));
        tab.sync_cursor_from_primary();

        // Record for undo
        tab.record_edit(old_content, old_cursor);

        debug!(
            "Move line: direction={}, line {} -> {}",
            direction, current_line_num, new_line_num
        );
    }

    /// Handle deleting the current line.
    ///
    /// Operates in Raw or Split view mode (both have raw editor). Removes the current line entirely,
    /// placing the cursor at the same column on the next line (or previous if at end).
    pub(crate) fn handle_delete_line(&mut self) {
        // Only operate in Raw or Split view mode (both have raw editor)
        let view_mode = self
            .state
            .active_tab()
            .map(|t| t.view_mode)
            .unwrap_or(ViewMode::Raw);

        if view_mode == ViewMode::Rendered {
            debug!("Delete line: skipping, Rendered mode has no raw editor");
            return;
        }

        let Some(tab) = self.state.active_tab_mut() else {
            return;
        };

        // Save state for undo
        let old_content = tab.content.clone();
        let old_cursor = tab.cursors.primary().head;

        // Get cursor position - cursor_position gives (line, column) directly
        let (current_line_num, cursor_col) = tab.cursor_position;
        let total_lines = tab.content.matches('\n').count() + 1;

        // Can't delete if document is empty or has only one empty line
        if tab.content.is_empty() {
            debug!("Delete line: skipping, document is empty");
            return;
        }

        // Split into lines for manipulation
        let lines: Vec<&str> = tab.content.split('\n').collect();
        let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len().saturating_sub(1));

        // Remove the current line
        for (i, line) in lines.iter().enumerate() {
            if i != current_line_num {
                new_lines.push(line);
            }
        }

        // Build new content
        let new_content = if new_lines.is_empty() {
            // If we deleted the last line, result is empty
            String::new()
        } else {
            new_lines.join("\n")
        };

        // Calculate new cursor position
        // Stay on same line number if possible, or move to previous line if we were on last line
        let new_line_num = if current_line_num >= new_lines.len() {
            new_lines.len().saturating_sub(1)
        } else {
            current_line_num
        };

        // Find byte offset of the new line position
        let mut new_line_start = 0usize;
        for (i, line) in new_lines.iter().enumerate() {
            if i == new_line_num {
                break;
            }
            new_line_start += line.len() + 1; // +1 for newline
        }

        // Calculate new cursor byte position (line start + column, clamped to line length)
        let new_line_len = new_lines.get(new_line_num).map(|l| l.len()).unwrap_or(0);
        let new_cursor_byte = new_line_start + cursor_col.min(new_line_len);

        // Convert byte position to character position
        let new_cursor_char = if new_content.is_empty() {
            0
        } else {
            new_content[..new_cursor_byte.min(new_content.len())]
                .chars()
                .count()
        };

        debug!(
            "Delete line: line={}, total_lines={}, new_line_num={}, new_cursor_char={}",
            current_line_num, total_lines, new_line_num, new_cursor_char
        );

        // Apply changes
        tab.content = new_content;

        // Use pending_cursor_restore to ensure the cursor position is applied
        tab.pending_cursor_restore = Some(new_cursor_char);

        // Also update internal state for consistency
        tab.cursors
            .set_single(crate::state::Selection::cursor(new_cursor_char));
        tab.sync_cursor_from_primary();

        // Record for undo
        tab.record_edit(old_content, old_cursor);

        debug!(
            "Delete line: deleted line {} (total was {})",
            current_line_num, total_lines
        );
    }
}
