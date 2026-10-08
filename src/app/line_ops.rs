//! Line operations for the Ferrite application.
//!
//! This module contains handlers for go-to-line, duplicate line,
//! move line up/down, and delete line operations.
//!
//! Line operations rejoin the buffer with the tab's stored [`LineEnding`]. Files that
//! contain mixed CRLF and LF are opened using the dominant ending (#174); edits here
//! normalize the whole buffer to that ending rather than preserving per-line EOL style.

use super::FerriteApp;
use crate::config::ViewMode;
use crate::state::LineEnding;
use crate::string_utils::{rope_char_index_at_line_start, rope_line_count};
use log::{debug, warn};

/// Char index of `(line_num, char_col)` within `lines` joined by an EOL of
/// `eol_chars` characters, clamping the column to the target line's char length.
///
/// Works entirely in characters: `tab.cursor_position.1` is a char column, so
/// byte math would land mid-codepoint on multi-byte lines and panic on slicing.
fn cursor_char_index(lines: &[&str], line_num: usize, char_col: usize, eol_chars: usize) -> usize {
    let mut idx = 0usize;
    for line in lines.iter().take(line_num) {
        idx += line.chars().count() + eol_chars;
    }
    let line_char_len = lines.get(line_num).map(|l| l.chars().count()).unwrap_or(0);
    idx + char_col.min(line_char_len)
}

impl FerriteApp {
    /// Handle opening the Go to Line dialog.
    pub(crate) fn handle_open_go_to_line(&mut self) {
        // Get current line and max line from active tab
        let Some(tab) = self.state.active_tab() else {
            return;
        };

        // Calculate current line (1-indexed) from cursor position
        let current_line = tab.cursor_position.0 + 1;

        // Calculate total line count (ropey line breaks, matches raw editor)
        let max_line = rope_line_count(&tab.content).max(1);

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
        // target_line is 1-indexed
        let line_index = target_line.saturating_sub(1);
        let char_index = rope_char_index_at_line_start(&tab.content, line_index);

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
        let ending = tab.line_ending;
        let eol_len = ending.byte_len();

        // Split into lines for manipulation (preserves trailing-empty segment)
        let lines = LineEnding::split_lines(&tab.content);

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

        let new_content = ending.join_lines(&new_lines);

        // Calculate new cursor position on the duplicated line (one line down, same column)
        let new_line_num = current_line_num + 1;
        let new_cursor_char = cursor_char_index(&new_lines, new_line_num, cursor_col, eol_len)
            .min(new_content.chars().count());

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
        let ending = tab.line_ending;
        let eol_len = ending.byte_len();
        let total_lines = LineEnding::split_lines(&tab.content).len().max(1);

        // Check boundaries
        if direction < 0 && current_line_num == 0 {
            return; // Can't move up from first line
        }
        if direction > 0 && current_line_num >= total_lines - 1 {
            return; // Can't move down from last line
        }

        // Split into lines for manipulation
        let lines = LineEnding::split_lines(&tab.content);
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
        let new_content = ending.join_lines(&new_lines);

        // Calculate new cursor position
        // The cursor should be on the same line content, which has moved
        let new_line_num = if direction < 0 {
            current_line_num - 1
        } else {
            current_line_num + 1
        };

        // Cursor lands on the moved line at the same (clamped) char column.
        let new_cursor_char = cursor_char_index(&new_lines, new_line_num, cursor_col, eol_len)
            .min(new_content.chars().count());

        debug!(
            "Move line: new_line_num={}, new_cursor_char={}",
            new_line_num, new_cursor_char
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
        let ending = tab.line_ending;
        let eol_len = ending.byte_len();
        let total_lines = LineEnding::split_lines(&tab.content).len().max(1);

        // Can't delete if document is empty or has only one empty line
        if tab.content.is_empty() {
            debug!("Delete line: skipping, document is empty");
            return;
        }

        // Split into lines for manipulation
        let lines = LineEnding::split_lines(&tab.content);
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
            ending.join_lines(&new_lines)
        };

        // Calculate new cursor position
        // Stay on same line number if possible, or move to previous line if we were on last line
        let new_line_num = if current_line_num >= new_lines.len() {
            new_lines.len().saturating_sub(1)
        } else {
            current_line_num
        };

        // Cursor stays at the same (clamped) char column on the surviving line.
        let new_cursor_char = if new_content.is_empty() {
            0
        } else {
            cursor_char_index(&new_lines, new_line_num, cursor_col, eol_len)
                .min(new_content.chars().count())
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

#[cfg(test)]
mod tests {
    use super::cursor_char_index;
    use crate::string_utils::{rope_char_index_at_line_start, rope_line_count};

    #[test]
    fn go_to_line_char_index_matches_ropey_breaks() {
        let text = format!("line0{LS}line1{FF}line2", LS = '\u{2028}', FF = '\u{000c}');
        assert_eq!(rope_line_count(&text), 3);
        assert_eq!(rope_char_index_at_line_start(&text, 2), 12);
    }

    #[test]
    fn cursor_char_index_multibyte_lines() {
        // "日本語" is 3 chars / 9 bytes — byte math would overshoot into the
        // next line or land mid-codepoint (regression: duplicate/move/delete
        // line panic on multi-byte content).
        let lines = vec!["日本語", "ééé", "ascii"];
        // Line 1, col 2 → 3 chars + 1 EOL + 2 = 6.
        assert_eq!(cursor_char_index(&lines, 1, 2, 1), 6);
        // Column clamps to the line's char length (3), not its byte length (6).
        assert_eq!(cursor_char_index(&lines, 1, 99, 1), 7);
        // CRLF: EOL counts 2 chars.
        assert_eq!(cursor_char_index(&lines, 1, 0, 2), 5);
        // Line index past the end clamps to document end.
        assert_eq!(cursor_char_index(&lines, 0, 1, 1), 1);
    }
}
