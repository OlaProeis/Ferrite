//! Cursor module for Ferrite editor.
//!
//! This module provides cursor position tracking and text selection for the text editor.
//!
//! # Selection Model
//!
//! A selection consists of two positions:
//! - **Anchor**: The fixed point where selection started (e.g., where user clicked/shift-started)
//! - **Head**: The moving point (e.g., where user dragged to or cursor currently is)
//!
//! The anchor and head can be in any order - anchor doesn't need to come before head.
//! Use `Selection::ordered()` to get (start, end) in document order.

/// Cursor position in the document.
///
/// Tracks a single insertion point using (line, column) coordinates.
/// Both line and column are zero-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    /// Zero-indexed line number.
    pub line: usize,
    /// Zero-indexed column (character offset within line).
    pub column: usize,
}

impl Cursor {
    /// Creates a new cursor at the given position.
    #[must_use]
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Creates a cursor at the start of the document.
    #[must_use]
    pub fn start() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

/// Text selection in the document.
///
/// A selection represents a range of text between two cursor positions.
/// The `anchor` is the fixed point where selection started, and `head` is
/// the moving point (where the cursor currently is).
///
/// # Examples
///
/// ```rust,ignore
/// // Create a selection from (0, 5) to (0, 10)
/// let sel = Selection::new(Cursor::new(0, 5), Cursor::new(0, 10));
/// assert!(sel.is_range());
///
/// // Create a collapsed selection (cursor with no selection)
/// let sel = Selection::collapsed(Cursor::new(0, 5));
/// assert!(!sel.is_range());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection {
    /// The anchor point - where the selection started (fixed).
    pub anchor: Cursor,
    /// The head point - where the cursor is (moves with navigation).
    pub head: Cursor,
}

impl Selection {
    /// Creates a new selection with the given anchor and head.
    #[must_use]
    pub fn new(anchor: Cursor, head: Cursor) -> Self {
        Self { anchor, head }
    }

    /// Creates a collapsed selection (cursor with no range) at the given position.
    #[must_use]
    pub fn collapsed(cursor: Cursor) -> Self {
        Self {
            anchor: cursor,
            head: cursor,
        }
    }

    /// Creates a collapsed selection at the start of the document.
    #[must_use]
    pub fn start() -> Self {
        Self::collapsed(Cursor::start())
    }

    /// Returns whether this selection has a non-empty range.
    ///
    /// Returns `false` if anchor equals head (collapsed/cursor-only).
    #[must_use]
    pub fn is_range(&self) -> bool {
        self.anchor != self.head
    }

    /// Returns whether this selection is collapsed (no range).
    #[must_use]
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    /// Returns the cursor position (head of the selection).
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.head
    }

    /// Returns (start, end) in document order.
    ///
    /// The start is always before or equal to the end.
    #[must_use]
    pub fn ordered(&self) -> (Cursor, Cursor) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Returns the start position (smaller of anchor/head).
    #[must_use]
    pub fn start_pos(&self) -> Cursor {
        self.ordered().0
    }

    /// Returns the end position (larger of anchor/head).
    #[must_use]
    pub fn end_pos(&self) -> Cursor {
        self.ordered().1
    }

    /// Collapses the selection to the head position.
    #[must_use]
    pub fn collapse_to_head(self) -> Self {
        Self::collapsed(self.head)
    }

    /// Collapses the selection to the anchor position.
    #[must_use]
    pub fn collapse_to_anchor(self) -> Self {
        Self::collapsed(self.anchor)
    }

    /// Collapses the selection to the start (smaller position).
    #[must_use]
    pub fn collapse_to_start(self) -> Self {
        Self::collapsed(self.start_pos())
    }

    /// Collapses the selection to the end (larger position).
    #[must_use]
    pub fn collapse_to_end(self) -> Self {
        Self::collapsed(self.end_pos())
    }

    /// Returns a new selection with the head moved to the given position.
    /// The anchor remains unchanged.
    #[must_use]
    pub fn with_head(self, head: Cursor) -> Self {
        Self {
            anchor: self.anchor,
            head,
        }
    }

    /// Returns a new selection with both anchor and head at the given position.
    #[must_use]
    pub fn with_cursor(cursor: Cursor) -> Self {
        Self::collapsed(cursor)
    }

    /// Extends the selection to the given head position.
    /// If currently collapsed, the current position becomes the anchor.
    #[must_use]
    pub fn extend_to(self, new_head: Cursor) -> Self {
        Self {
            anchor: self.anchor,
            head: new_head,
        }
    }

    /// Checks if a cursor position is within this selection.
    #[must_use]
    pub fn contains(&self, cursor: Cursor) -> bool {
        let (start, end) = self.ordered();
        cursor >= start && cursor < end
    }

    /// Checks if a line is touched by this selection.
    #[must_use]
    pub fn touches_line(&self, line: usize) -> bool {
        let (start, end) = self.ordered();
        line >= start.line && line <= end.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────────
    // Cursor Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cursor_new() {
        let cursor = Cursor::new(5, 10);
        assert_eq!(cursor.line, 5);
        assert_eq!(cursor.column, 10);
    }

    #[test]
    fn test_cursor_start() {
        let cursor = Cursor::start();
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_cursor_default() {
        let cursor = Cursor::default();
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn test_cursor_equality() {
        let cursor1 = Cursor::new(3, 7);
        let cursor2 = Cursor::new(3, 7);
        let cursor3 = Cursor::new(3, 8);

        assert_eq!(cursor1, cursor2);
        assert_ne!(cursor1, cursor3);
    }

    #[test]
    fn test_cursor_clone() {
        let cursor1 = Cursor::new(10, 20);
        let cursor2 = cursor1;
        assert_eq!(cursor1, cursor2);
    }

    #[test]
    fn test_cursor_ordering() {
        let a = Cursor::new(0, 5);
        let b = Cursor::new(0, 10);
        let c = Cursor::new(1, 0);
        let d = Cursor::new(1, 5);

        assert!(a < b);
        assert!(b < c);
        assert!(c < d);
        assert!(a < d);

        // Same position
        let e = Cursor::new(0, 5);
        assert!(a == e);
        assert!(!(a < e));
        assert!(!(a > e));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Selection Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_selection_collapsed() {
        let sel = Selection::collapsed(Cursor::new(5, 10));
        assert_eq!(sel.anchor, Cursor::new(5, 10));
        assert_eq!(sel.head, Cursor::new(5, 10));
        assert!(sel.is_collapsed());
        assert!(!sel.is_range());
    }

    #[test]
    fn test_selection_range() {
        let sel = Selection::new(Cursor::new(0, 5), Cursor::new(0, 10));
        assert!(!sel.is_collapsed());
        assert!(sel.is_range());
        assert_eq!(sel.cursor(), Cursor::new(0, 10)); // head is the cursor
    }

    #[test]
    fn test_selection_ordered_forward() {
        // Anchor before head
        let sel = Selection::new(Cursor::new(0, 5), Cursor::new(0, 10));
        let (start, end) = sel.ordered();
        assert_eq!(start, Cursor::new(0, 5));
        assert_eq!(end, Cursor::new(0, 10));
    }

    #[test]
    fn test_selection_ordered_backward() {
        // Head before anchor (selecting backwards)
        let sel = Selection::new(Cursor::new(0, 10), Cursor::new(0, 5));
        let (start, end) = sel.ordered();
        assert_eq!(start, Cursor::new(0, 5));
        assert_eq!(end, Cursor::new(0, 10));
    }

    #[test]
    fn test_selection_multiline_ordered() {
        // Selection spanning multiple lines, anchor on line 2
        let sel = Selection::new(Cursor::new(2, 5), Cursor::new(0, 3));
        let (start, end) = sel.ordered();
        assert_eq!(start, Cursor::new(0, 3));
        assert_eq!(end, Cursor::new(2, 5));
    }

    #[test]
    fn test_selection_start() {
        let sel = Selection::start();
        assert!(sel.is_collapsed());
        assert_eq!(sel.head, Cursor::start());
    }

    #[test]
    fn test_selection_collapse_operations() {
        let sel = Selection::new(Cursor::new(0, 5), Cursor::new(2, 10));

        // Collapse to head
        let collapsed_head = sel.collapse_to_head();
        assert_eq!(collapsed_head.anchor, Cursor::new(2, 10));
        assert_eq!(collapsed_head.head, Cursor::new(2, 10));

        // Collapse to anchor
        let collapsed_anchor = sel.collapse_to_anchor();
        assert_eq!(collapsed_anchor.anchor, Cursor::new(0, 5));
        assert_eq!(collapsed_anchor.head, Cursor::new(0, 5));

        // Collapse to start
        let collapsed_start = sel.collapse_to_start();
        assert_eq!(collapsed_start.head, Cursor::new(0, 5));

        // Collapse to end
        let collapsed_end = sel.collapse_to_end();
        assert_eq!(collapsed_end.head, Cursor::new(2, 10));
    }

    #[test]
    fn test_selection_with_head() {
        let sel = Selection::new(Cursor::new(0, 5), Cursor::new(0, 10));
        let extended = sel.with_head(Cursor::new(1, 3));

        assert_eq!(extended.anchor, Cursor::new(0, 5)); // anchor unchanged
        assert_eq!(extended.head, Cursor::new(1, 3)); // head moved
    }

    #[test]
    fn test_selection_extend_to() {
        let sel = Selection::collapsed(Cursor::new(0, 5));
        let extended = sel.extend_to(Cursor::new(0, 10));

        assert_eq!(extended.anchor, Cursor::new(0, 5));
        assert_eq!(extended.head, Cursor::new(0, 10));
        assert!(extended.is_range());
    }

    #[test]
    fn test_selection_contains() {
        let sel = Selection::new(Cursor::new(0, 5), Cursor::new(0, 10));

        // Inside
        assert!(sel.contains(Cursor::new(0, 5)));
        assert!(sel.contains(Cursor::new(0, 7)));
        assert!(sel.contains(Cursor::new(0, 9)));

        // Outside
        assert!(!sel.contains(Cursor::new(0, 4)));
        assert!(!sel.contains(Cursor::new(0, 10))); // end is exclusive
        assert!(!sel.contains(Cursor::new(1, 0)));
    }

    #[test]
    fn test_selection_contains_multiline() {
        let sel = Selection::new(Cursor::new(1, 5), Cursor::new(3, 10));

        // Line 1, column >= 5
        assert!(sel.contains(Cursor::new(1, 5)));
        assert!(sel.contains(Cursor::new(1, 100)));

        // Line 2, any column
        assert!(sel.contains(Cursor::new(2, 0)));
        assert!(sel.contains(Cursor::new(2, 50)));

        // Line 3, column < 10
        assert!(sel.contains(Cursor::new(3, 0)));
        assert!(sel.contains(Cursor::new(3, 9)));

        // Outside
        assert!(!sel.contains(Cursor::new(0, 0)));
        assert!(!sel.contains(Cursor::new(1, 4)));
        assert!(!sel.contains(Cursor::new(3, 10)));
        assert!(!sel.contains(Cursor::new(4, 0)));
    }

    #[test]
    fn test_selection_touches_line() {
        let sel = Selection::new(Cursor::new(2, 5), Cursor::new(4, 10));

        assert!(!sel.touches_line(0));
        assert!(!sel.touches_line(1));
        assert!(sel.touches_line(2));
        assert!(sel.touches_line(3));
        assert!(sel.touches_line(4));
        assert!(!sel.touches_line(5));
    }

    #[test]
    fn test_selection_default() {
        let sel = Selection::default();
        assert!(sel.is_collapsed());
        assert_eq!(sel.head, Cursor::default());
    }
}
