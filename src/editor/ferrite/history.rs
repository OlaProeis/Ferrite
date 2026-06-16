//! Unified operation-based undo/redo system.
//!
//! This module is the **single undo system** for the entire application.
//! `EditHistory` is owned by each `Tab` in `state.rs` and handles undo for
//! both raw (FerriteEditor) and rendered (MarkdownEditor) modes.
//!
//! Edits are recorded as discrete operations (insert/delete) rather than full
//! state snapshots, making memory usage proportional to edit size, not file size.
//!
//! # Recording flow
//!
//! Non-raw modes use diff-based recording:
//! 1. Before editor `show()`: `Tab::prepare_undo_snapshot_hashed()` clones content
//!    only when the blake3 hash changes (avoids per-frame allocation).
//! 2. After editor `show()`: if changed, `Tab::record_external_edit_from_snapshot()` computes
//!    a minimal diff via [`compute_edit_ops`] and pushes the resulting operations.
//!
//! Raw mode relies on FerriteEditor's own undo — no central-panel snapshot.
//!
//! # Features
//! - Operation-based undo/redo (not state snapshots)
//! - Memory-efficient: stores only changed text, not full content
//! - Atomic batches: each `record_operations` call is one undo step (replace = delete+insert together)
//! - Works on both `TextBuffer` (rope) and plain `String`

#[cfg(test)]
use crate::editor::ferrite::buffer::TextBuffer;

/// Represents a single edit operation that can be undone or redone.
///
/// Each operation stores enough information to both apply and reverse itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOperation {
    /// Text was inserted at a position.
    ///
    /// To undo: delete `text.len()` characters starting at `pos`.
    /// To redo: insert `text` at `pos`.
    Insert {
        /// The character position where text was inserted.
        pos: usize,
        /// The text that was inserted.
        text: String,
    },
    /// Text was deleted from a position.
    ///
    /// To undo: insert `text` at `pos`.
    /// To redo: delete `text.len()` characters starting at `pos`.
    Delete {
        /// The character position where text was deleted.
        pos: usize,
        /// The text that was deleted (stored for undo).
        text: String,
    },
}

impl EditOperation {
    /// Returns the inverse operation (for undo).
    ///
    /// - Insert becomes Delete
    /// - Delete becomes Insert
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Insert { pos, text } => Self::Delete {
                pos: *pos,
                text: text.clone(),
            },
            Self::Delete { pos, text } => Self::Insert {
                pos: *pos,
                text: text.clone(),
            },
        }
    }

    /// Applies this operation to the given TextBuffer (used in tests).
    #[cfg(test)]
    pub fn apply(&self, buffer: &mut TextBuffer) {
        match self {
            Self::Insert { pos, text } => {
                buffer.insert(*pos, text);
            }
            Self::Delete { pos, text } => {
                buffer.remove(*pos, text.chars().count());
            }
        }
    }

    /// Applies this operation to a plain `String` using char-indexed positions.
    pub fn apply_to_string(&self, s: &mut String) {
        match self {
            Self::Insert { pos, text } => {
                let byte_pos = char_pos_to_byte_pos(s, *pos);
                s.insert_str(byte_pos, text);
            }
            Self::Delete { pos, text } => {
                let byte_start = char_pos_to_byte_pos(s, *pos);
                let byte_end = char_pos_to_byte_pos(s, *pos + text.chars().count());
                s.drain(byte_start..byte_end);
            }
        }
    }
}

/// Converts a char index to a byte offset within a string.
fn char_pos_to_byte_pos(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

/// Computes the minimal edit operations to transform `old` into `new`.
///
/// Uses prefix/suffix matching to find the changed region, then emits
/// a Delete (if text was removed) and/or an Insert (if text was added).
/// Positions are char-indexed, not byte-indexed.
pub fn compute_edit_ops(old: &str, new: &str) -> Vec<EditOperation> {
    if old == new {
        return Vec::new();
    }

    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();

    let prefix = old_chars
        .iter()
        .zip(&new_chars)
        .take_while(|(a, b)| a == b)
        .count();

    let max_suffix = old_chars.len().min(new_chars.len()).saturating_sub(prefix);
    let suffix = old_chars
        .iter()
        .rev()
        .zip(new_chars.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();

    let del_end = old_chars.len() - suffix;
    let ins_end = new_chars.len() - suffix;

    let mut ops = Vec::new();
    if del_end > prefix {
        let deleted: String = old_chars[prefix..del_end].iter().collect();
        ops.push(EditOperation::Delete {
            pos: prefix,
            text: deleted,
        });
    }
    if ins_end > prefix {
        let inserted: String = new_chars[prefix..ins_end].iter().collect();
        ops.push(EditOperation::Insert {
            pos: prefix,
            text: inserted,
        });
    }
    ops
}

/// A group of operations that should be undone/redone together.
///
/// All operations in a group are undone/redone together (one user-visible edit step).
#[derive(Debug, Clone)]
struct OperationGroup {
    /// The operations in this group, in order of execution.
    operations: Vec<EditOperation>,
}

impl OperationGroup {
    fn from_operations(operations: Vec<EditOperation>) -> Self {
        Self { operations }
    }

    /// Applies the inverse of all operations in reverse order (for undo).
    #[cfg(test)]
    fn undo(&self, buffer: &mut TextBuffer) {
        for op in self.operations.iter().rev() {
            op.inverse().apply(buffer);
        }
    }

    /// Applies all operations in order (for redo).
    #[cfg(test)]
    fn redo(&self, buffer: &mut TextBuffer) {
        for op in &self.operations {
            op.apply(buffer);
        }
    }

    /// Applies the inverse of all operations in reverse order to a `String`.
    fn undo_string(&self, s: &mut String) {
        for op in self.operations.iter().rev() {
            op.inverse().apply_to_string(s);
        }
    }

    /// Applies all operations in order to a `String`.
    fn redo_string(&self, s: &mut String) {
        for op in &self.operations {
            op.apply_to_string(s);
        }
    }
}

/// Manages undo/redo history for text editing operations.
///
/// `EditHistory` maintains two stacks:
/// - `undo_stack`: Operations that can be undone
/// - `redo_stack`: Operations that have been undone and can be redone
///
/// # Operation Grouping
///
/// Each call to [`EditHistory::record_operations`] (or [`EditHistory::record_operation`])
/// pushes one undo group. A diff that deletes and inserts at the same position stays in
/// that single group. Separate frames or edit commits produce separate undo steps.
///
/// # Memory Efficiency
///
/// Unlike snapshot-based undo systems, `EditHistory` stores only the operations themselves,
/// not full copies of the document. This makes it suitable for large files.
#[derive(Debug, Clone)]
pub struct EditHistory {
    /// Stack of operation groups that can be undone.
    undo_stack: Vec<OperationGroup>,
    /// Stack of operation groups that can be redone.
    redo_stack: Vec<OperationGroup>,
    /// Maximum number of undo groups to keep. Operations are tiny so this can be large.
    max_groups: usize,
}

/// Default maximum undo groups (operation-based entries are small).
const DEFAULT_MAX_GROUPS: usize = 500;

impl Default for EditHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // EditHistory API for alternate undo recording paths
impl EditHistory {
    /// Creates a new empty `EditHistory`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let history = EditHistory::new();
    /// assert!(!history.can_undo());
    /// assert!(!history.can_redo());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_groups: DEFAULT_MAX_GROUPS,
        }
    }

    /// Creates a new `EditHistory` with a custom max undo group limit.
    #[must_use]
    pub fn with_max_groups(max_groups: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_groups,
        }
    }

    /// Records a single edit operation as its own undo group.
    ///
    /// Prefer [`Self::record_operations`] when a change produces multiple ops (e.g. replace).
    /// Recording clears the redo stack.
    ///
    /// # Arguments
    /// * `op` - The operation to record
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut history = EditHistory::new();
    ///
    /// // Record an insert
    /// history.record_operation(EditOperation::Insert {
    ///     pos: 0,
    ///     text: "Hello".to_string(),
    /// });
    ///
    /// assert!(history.can_undo());
    /// ```
    pub fn record_operation(&mut self, op: EditOperation) {
        self.record_operations(vec![op]);
    }

    /// Records a batch of edit operations as a single undo group.
    ///
    /// Used by the diff-based recording path: one edit frame produces 0–2 operations
    /// (delete and/or insert) that must undo together. Each call is a separate undo step;
    /// rapid typing creates one step per dirty frame, not one step per typing session.
    pub fn record_operations(&mut self, ops: Vec<EditOperation>) {
        if ops.is_empty() {
            return;
        }
        self.undo_stack.push(OperationGroup::from_operations(ops));

        while self.undo_stack.len() > self.max_groups {
            self.undo_stack.remove(0);
        }

        self.redo_stack.clear();
    }

    /// Undoes the last operation group.
    ///
    /// Applies the inverse of all operations in the last group (in reverse order)
    /// and moves the group to the redo stack.
    ///
    /// # Arguments
    /// * `buffer` - The text buffer to apply the undo to
    ///
    /// # Returns
    /// `Some(char_pos)` with the cursor position to restore to if an operation was undone,
    /// `None` if the undo stack was empty.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut buffer = TextBuffer::from_string("Hello World");
    /// let mut history = EditHistory::new();
    ///
    /// // Record a delete operation
    /// history.record_operation(EditOperation::Delete {
    ///     pos: 5,
    ///     text: " World".to_string(),
    /// });
    /// buffer.remove(5, 6);
    ///
    /// // Undo restores the deleted text
    /// let cursor_pos = history.undo(&mut buffer);
    /// assert_eq!(buffer.to_string(), "Hello World");
    /// assert!(cursor_pos.is_some());
    /// ```
    #[cfg(test)]
    pub fn undo(&mut self, buffer: &mut TextBuffer) -> Option<usize> {
        if let Some(group) = self.undo_stack.pop() {
            let cursor_pos = group.operations.first().map(|op| match op {
                EditOperation::Insert { pos, .. } => *pos,
                EditOperation::Delete { pos, text } => *pos + text.chars().count(),
            });

            group.undo(buffer);
            self.redo_stack.push(group);
            cursor_pos
        } else {
            None
        }
    }

    /// Redoes the last undone operation group.
    ///
    /// Reapplies all operations in the last undone group (in order)
    /// and moves the group back to the undo stack.
    ///
    /// # Arguments
    /// * `buffer` - The text buffer to apply the redo to
    ///
    /// # Returns
    /// `Some(char_pos)` with the cursor position to restore to if an operation was redone,
    /// `None` if the redo stack was empty.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut buffer = TextBuffer::from_string("Hello");
    /// let mut history = EditHistory::new();
    ///
    /// // Insert and record
    /// buffer.insert(5, " World");
    /// history.record_operation(EditOperation::Insert {
    ///     pos: 5,
    ///     text: " World".to_string(),
    /// });
    ///
    /// // Undo
    /// history.undo(&mut buffer);
    /// assert_eq!(buffer.to_string(), "Hello");
    ///
    /// // Redo
    /// let cursor_pos = history.redo(&mut buffer);
    /// assert_eq!(buffer.to_string(), "Hello World");
    /// assert!(cursor_pos.is_some());
    /// ```
    #[cfg(test)]
    pub fn redo(&mut self, buffer: &mut TextBuffer) -> Option<usize> {
        if let Some(group) = self.redo_stack.pop() {
            let cursor_pos = group.operations.last().map(|op| match op {
                EditOperation::Insert { pos, text } => *pos + text.chars().count(),
                EditOperation::Delete { pos, .. } => *pos,
            });

            group.redo(buffer);
            self.undo_stack.push(group);
            cursor_pos
        } else {
            None
        }
    }

    /// Undoes the last operation group, applying changes to a plain `String`.
    ///
    /// Returns `Some(char_pos)` with the cursor position to restore,
    /// or `None` if the undo stack was empty.
    pub fn undo_string(&mut self, s: &mut String) -> Option<usize> {
        if let Some(group) = self.undo_stack.pop() {
            let cursor_pos = group.operations.first().map(|op| match op {
                EditOperation::Insert { pos, .. } => *pos,
                EditOperation::Delete { pos, text } => *pos + text.chars().count(),
            });

            group.undo_string(s);
            self.redo_stack.push(group);
            cursor_pos
        } else {
            None
        }
    }

    /// Redoes the last undone operation group, applying changes to a plain `String`.
    ///
    /// Returns `Some(char_pos)` with the cursor position to restore,
    /// or `None` if the redo stack was empty.
    pub fn redo_string(&mut self, s: &mut String) -> Option<usize> {
        if let Some(group) = self.redo_stack.pop() {
            let cursor_pos = group.operations.last().map(|op| match op {
                EditOperation::Insert { pos, text } => *pos + text.chars().count(),
                EditOperation::Delete { pos, .. } => *pos,
            });

            group.redo_string(s);
            self.undo_stack.push(group);
            cursor_pos
        } else {
            None
        }
    }

    /// Returns `true` if there are operations that can be undone.
    ///
    /// # Example
    /// ```rust,ignore
    /// let history = EditHistory::new();
    /// assert!(!history.can_undo());
    /// ```
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there are operations that can be redone.
    ///
    /// # Example
    /// ```rust,ignore
    /// let history = EditHistory::new();
    /// assert!(!history.can_redo());
    /// ```
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears all undo and redo history.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns the number of operation groups in the undo stack.
    ///
    /// This is useful for debugging and testing.
    #[must_use]
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of operation groups in the redo stack.
    ///
    /// This is useful for debugging and testing.
    #[must_use]
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// No-op retained for API compatibility.
    ///
    /// Each [`record_operations`] call already creates its own undo group.
    pub fn break_group(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_history() {
        let history = EditHistory::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_count(), 0);
        assert_eq!(history.redo_count(), 0);
    }

    #[test]
    fn test_default_history() {
        let history = EditHistory::default();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_record_operation() {
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "Hello".to_string(),
        });

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_count(), 1);
    }

    #[test]
    fn test_simple_undo() {
        let mut buffer = TextBuffer::from_string("Hello");
        let mut history = EditHistory::new();

        // Insert " World"
        buffer.insert(5, " World");
        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        });
        assert_eq!(buffer.to_string(), "Hello World");

        // Undo
        let cursor_pos = history.undo(&mut buffer);
        assert!(cursor_pos.is_some());
        assert_eq!(cursor_pos.unwrap(), 5); // Cursor at start of undone insert
        assert_eq!(buffer.to_string(), "Hello");
        assert!(history.can_redo());
    }

    #[test]
    fn test_simple_redo() {
        let mut buffer = TextBuffer::from_string("Hello");
        let mut history = EditHistory::new();

        // Insert and undo
        buffer.insert(5, " World");
        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        });
        history.undo(&mut buffer);

        // Redo
        let cursor_pos = history.redo(&mut buffer);
        assert!(cursor_pos.is_some());
        assert_eq!(cursor_pos.unwrap(), 11); // Cursor at end of redone insert (5 + 6 chars)
        assert_eq!(buffer.to_string(), "Hello World");
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_delete_undo_redo() {
        let mut buffer = TextBuffer::from_string("Hello World");
        let mut history = EditHistory::new();

        // Delete " World"
        let deleted_text = " World";
        history.record_operation(EditOperation::Delete {
            pos: 5,
            text: deleted_text.to_string(),
        });
        buffer.remove(5, deleted_text.len());
        assert_eq!(buffer.to_string(), "Hello");

        // Undo should restore
        let cursor_pos = history.undo(&mut buffer);
        assert!(cursor_pos.is_some());
        assert_eq!(cursor_pos.unwrap(), 11); // Cursor at end of restored text (5 + 6 chars)
        assert_eq!(buffer.to_string(), "Hello World");

        // Redo should delete again
        let cursor_pos = history.redo(&mut buffer);
        assert!(cursor_pos.is_some());
        assert_eq!(cursor_pos.unwrap(), 5); // Cursor at delete position
        assert_eq!(buffer.to_string(), "Hello");
    }

    #[test]
    fn test_new_operation_clears_redo() {
        let mut buffer = TextBuffer::from_string("Hello");
        let mut history = EditHistory::new();

        // Insert, undo, then insert something new
        buffer.insert(5, " World");
        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        });

        history.undo(&mut buffer);
        assert!(history.can_redo());

        // New operation should clear redo stack
        buffer.insert(5, "!");
        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: "!".to_string(),
        });
        assert!(!history.can_redo());
    }

    #[test]
    fn test_empty_undo_redo() {
        let mut buffer = TextBuffer::from_string("Hello");
        let mut history = EditHistory::new();

        // Undo with empty stack
        assert!(history.undo(&mut buffer).is_none());
        assert_eq!(buffer.to_string(), "Hello");

        // Redo with empty stack
        assert!(history.redo(&mut buffer).is_none());
        assert_eq!(buffer.to_string(), "Hello");
    }

    #[test]
    fn test_clear() {
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "test".to_string(),
        });
        assert!(history.can_undo());

        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_count(), 0);
        assert_eq!(history.redo_count(), 0);
    }

    #[test]
    fn test_operation_inverse() {
        let insert = EditOperation::Insert {
            pos: 5,
            text: "test".to_string(),
        };
        let delete = EditOperation::Delete {
            pos: 5,
            text: "test".to_string(),
        };

        assert_eq!(insert.inverse(), delete);
        assert_eq!(delete.inverse(), insert);
    }

    #[test]
    fn test_operation_apply() {
        let mut buffer = TextBuffer::from_string("Hello");

        // Test insert apply
        let insert = EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        };
        insert.apply(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello World");

        // Test delete apply
        let delete = EditOperation::Delete {
            pos: 5,
            text: " World".to_string(),
        };
        delete.apply(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello");
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut buffer = TextBuffer::new();
        let mut history = EditHistory::new();

        // Perform multiple operations with breaks between them
        buffer.insert(0, "A");
        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "A".to_string(),
        });
        history.break_group();

        buffer.insert(1, "B");
        history.record_operation(EditOperation::Insert {
            pos: 1,
            text: "B".to_string(),
        });
        history.break_group();

        buffer.insert(2, "C");
        history.record_operation(EditOperation::Insert {
            pos: 2,
            text: "C".to_string(),
        });

        assert_eq!(buffer.to_string(), "ABC");
        assert_eq!(history.undo_count(), 3);

        // Undo all
        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "AB");

        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "A");

        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");

        // Redo all
        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "A");

        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "AB");

        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "ABC");
    }

    #[test]
    fn test_separate_record_operation_calls_are_separate_groups() {
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "A".to_string(),
        });
        history.record_operation(EditOperation::Insert {
            pos: 1,
            text: "B".to_string(),
        });
        history.record_operation(EditOperation::Insert {
            pos: 2,
            text: "C".to_string(),
        });

        assert_eq!(history.undo_count(), 3);
    }

    #[test]
    fn test_break_group() {
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "A".to_string(),
        });
        history.break_group();

        history.record_operation(EditOperation::Insert {
            pos: 1,
            text: "B".to_string(),
        });

        // Should be in separate groups
        assert_eq!(history.undo_count(), 2);
    }

    #[test]
    fn test_record_operations_atomic_batch() {
        let mut history = EditHistory::new();

        history.record_operations(vec![
            EditOperation::Delete {
                pos: 6,
                text: "World".to_string(),
            },
            EditOperation::Insert {
                pos: 6,
                text: "Rust!".to_string(),
            },
        ]);

        assert_eq!(history.undo_count(), 1);

        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "X".to_string(),
        });
        assert_eq!(history.undo_count(), 2);
    }

    #[test]
    fn test_incremental_undo_steps() {
        let mut buffer = TextBuffer::new();
        let mut history = EditHistory::new();

        buffer.insert(0, "H");
        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "H".to_string(),
        });
        buffer.insert(1, "i");
        history.record_operation(EditOperation::Insert {
            pos: 1,
            text: "i".to_string(),
        });

        assert_eq!(buffer.to_string(), "Hi");
        assert_eq!(history.undo_count(), 2);

        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "H");

        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");
    }

    #[test]
    fn test_unicode_operations() {
        let mut buffer = TextBuffer::new();
        let mut history = EditHistory::new();

        // Insert unicode text
        buffer.insert(0, "こんにちは");
        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "こんにちは".to_string(),
        });

        assert_eq!(buffer.to_string(), "こんにちは");

        // Undo
        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");

        // Redo
        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "こんにちは");
    }

    #[test]
    fn test_emoji_operations() {
        let mut buffer = TextBuffer::new();
        let mut history = EditHistory::new();

        // Insert emoji
        buffer.insert(0, "Hello 🌍 World");
        history.record_operation(EditOperation::Insert {
            pos: 0,
            text: "Hello 🌍 World".to_string(),
        });

        // Undo
        history.undo(&mut buffer);
        assert_eq!(buffer.to_string(), "");

        // Redo
        history.redo(&mut buffer);
        assert_eq!(buffer.to_string(), "Hello 🌍 World");
    }

    /// Test multiple insert/delete/undo/redo sequences
    #[test]
    fn test_extensive_operations() {
        let mut buffer = TextBuffer::from_string("Initial content");
        let mut history = EditHistory::new();
        let original = buffer.to_string();

        // Perform 100 operations
        for i in 0..100 {
            history.break_group(); // Force separate groups for testing

            if i % 2 == 0 {
                // Insert
                let text = format!("[{i}]");
                let pos = buffer.len().min(i % (buffer.len() + 1));
                buffer.insert(pos, &text);
                history.record_operation(EditOperation::Insert {
                    pos,
                    text: text.clone(),
                });
            } else {
                // Delete (if there's content)
                if buffer.len() > 5 {
                    let pos = i % (buffer.len().saturating_sub(3));
                    let len = (i % 3).max(1).min(buffer.len() - pos);
                    // Get the text to be deleted
                    let deleted: String = buffer.rope().slice(pos..pos + len).to_string();
                    history.record_operation(EditOperation::Delete { pos, text: deleted });
                    buffer.remove(pos, len);
                }
            }
        }

        // Undo all
        while history.can_undo() {
            history.undo(&mut buffer);
        }

        // After undoing all, should be back to original
        assert_eq!(buffer.to_string(), original);

        // Redo all
        while history.can_redo() {
            history.redo(&mut buffer);
        }

        // Undo all again to verify cycle
        while history.can_undo() {
            history.undo(&mut buffer);
        }

        assert_eq!(buffer.to_string(), original);
    }

    #[test]
    fn test_compute_edit_ops_insert() {
        let ops = compute_edit_ops("Hello", "Hello World");
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            EditOperation::Insert {
                pos: 5,
                text: " World".to_string(),
            }
        );
    }

    #[test]
    fn test_compute_edit_ops_delete() {
        let ops = compute_edit_ops("Hello World", "Hello");
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            EditOperation::Delete {
                pos: 5,
                text: " World".to_string(),
            }
        );
    }

    #[test]
    fn test_compute_edit_ops_replace() {
        let ops = compute_edit_ops("Hello World", "Hello Rust!");
        assert_eq!(ops.len(), 2);
        assert_eq!(
            ops[0],
            EditOperation::Delete {
                pos: 6,
                text: "World".to_string(),
            }
        );
        assert_eq!(
            ops[1],
            EditOperation::Insert {
                pos: 6,
                text: "Rust!".to_string(),
            }
        );
    }

    #[test]
    fn test_compute_edit_ops_no_change() {
        let ops = compute_edit_ops("same", "same");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_compute_edit_ops_unicode() {
        let ops = compute_edit_ops("Hello", "Hello こんにちは");
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            EditOperation::Insert {
                pos: 5,
                text: " こんにちは".to_string(),
            }
        );
    }

    #[test]
    fn test_undo_string() {
        let mut s = "Hello World".to_string();
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        });

        let cursor = history.undo_string(&mut s);
        assert_eq!(s, "Hello");
        assert_eq!(cursor, Some(5));
    }

    #[test]
    fn test_redo_string() {
        let mut s = "Hello World".to_string();
        let mut history = EditHistory::new();

        history.record_operation(EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        });

        history.undo_string(&mut s);
        assert_eq!(s, "Hello");

        let cursor = history.redo_string(&mut s);
        assert_eq!(s, "Hello World");
        assert_eq!(cursor, Some(11));
    }

    #[test]
    fn test_apply_to_string_insert() {
        let mut s = "Hello".to_string();
        EditOperation::Insert {
            pos: 5,
            text: " World".to_string(),
        }
        .apply_to_string(&mut s);
        assert_eq!(s, "Hello World");
    }

    #[test]
    fn test_apply_to_string_delete() {
        let mut s = "Hello World".to_string();
        EditOperation::Delete {
            pos: 5,
            text: " World".to_string(),
        }
        .apply_to_string(&mut s);
        assert_eq!(s, "Hello");
    }

    #[test]
    fn test_apply_to_string_unicode() {
        let mut s = "こんにちは".to_string();
        EditOperation::Insert {
            pos: 5,
            text: "世界".to_string(),
        }
        .apply_to_string(&mut s);
        assert_eq!(s, "こんにちは世界");

        EditOperation::Delete {
            pos: 5,
            text: "世界".to_string(),
        }
        .apply_to_string(&mut s);
        assert_eq!(s, "こんにちは");
    }

    #[test]
    fn test_roundtrip_diff_undo() {
        let old = "# Hello\n\nWorld";
        let new = "# Hello\n\nRust World";
        let ops = compute_edit_ops(old, new);

        let mut s = new.to_string();
        let mut history = EditHistory::new();
        history.record_operations(ops);

        history.undo_string(&mut s);
        assert_eq!(s, old);

        history.redo_string(&mut s);
        assert_eq!(s, new);
    }

    #[test]
    fn test_max_groups_cap() {
        let mut history = EditHistory::with_max_groups(3);
        for i in 0..5 {
            history.break_group();
            history.record_operation(EditOperation::Insert {
                pos: i,
                text: format!("{i}"),
            });
        }
        assert_eq!(history.undo_count(), 3);
    }

    /// Performance test with 1MB buffer
    #[test]
    #[ignore] // Slow test, run explicitly
    fn test_large_buffer_performance() {
        // Create 1MB buffer
        let line = "This is a test line for performance testing.\n";
        let line_count = (1024 * 1024) / line.len();
        let content: String = (0..line_count).map(|_| line).collect();

        let mut buffer = TextBuffer::from_string(&content);
        let mut history = EditHistory::new();
        let original_len = buffer.len();

        // Perform 100 operations
        for i in 0..100 {
            history.break_group();

            let pos = i * 100 % original_len;
            let text = format!("INSERT_{i}");
            buffer.insert(pos, &text);
            history.record_operation(EditOperation::Insert {
                pos,
                text: text.clone(),
            });
        }

        // Verify undo count
        assert_eq!(history.undo_count(), 100);

        // Undo all
        while history.can_undo() {
            history.undo(&mut buffer);
        }

        // Buffer should be back to original size
        assert_eq!(buffer.len(), original_len);
    }
}
