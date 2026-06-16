//! Pure helpers for keyboard navigation between table cells (row, col indices).
//!
//! Used by GFM [`EditableTable`](super::widgets::EditableTable) and the CSV viewer.

/// Move right; wrap to the first column of the next row at the row edge.
pub fn table_cell_next(
    row: usize,
    col: usize,
    num_rows: usize,
    num_cols: usize,
) -> Option<(usize, usize)> {
    if col + 1 < num_cols {
        Some((row, col + 1))
    } else if row + 1 < num_rows {
        Some((row + 1, 0))
    } else {
        None
    }
}

/// Move left; wrap to the last column of the previous row at the row start.
pub fn table_cell_prev(row: usize, col: usize, num_cols: usize) -> Option<(usize, usize)> {
    if col > 0 {
        Some((row, col - 1))
    } else if row > 0 {
        Some((row - 1, num_cols.saturating_sub(1)))
    } else {
        None
    }
}

/// Move down one row in the same column; clamp at the last row.
pub fn table_cell_down(row: usize, col: usize, num_rows: usize) -> Option<(usize, usize)> {
    if row + 1 < num_rows {
        Some((row + 1, col))
    } else {
        None
    }
}

/// Move up one row in the same column; clamp at the first row.
pub fn table_cell_up(row: usize, col: usize) -> Option<(usize, usize)> {
    if row > 0 {
        Some((row - 1, col))
    } else {
        None
    }
}

/// Arrow-key delta navigation; clamps at table boundaries (no wrap).
pub fn table_cell_arrow(
    row: usize,
    col: usize,
    num_rows: usize,
    num_cols: usize,
    dr: i32,
    dc: i32,
) -> (usize, usize) {
    let col_count = num_cols.max(1);
    let row_count = num_rows.max(1);
    let mut r = row;
    let mut c = col;
    if dr != 0 {
        r = (r as i32 + dr).clamp(0, row_count as i32 - 1) as usize;
    }
    if dc != 0 {
        c = (c as i32 + dc).clamp(0, col_count as i32 - 1) as usize;
    }
    (r, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_cell_next_wraps_rows() {
        assert_eq!(table_cell_next(0, 0, 3, 3), Some((0, 1)));
        assert_eq!(table_cell_next(0, 2, 3, 3), Some((1, 0)));
        assert_eq!(table_cell_next(2, 2, 3, 3), None);
    }

    #[test]
    fn table_cell_prev_wraps_rows() {
        assert_eq!(table_cell_prev(0, 1, 3), Some((0, 0)));
        assert_eq!(table_cell_prev(1, 0, 3), Some((0, 2)));
        assert_eq!(table_cell_prev(0, 0, 3), None);
    }

    #[test]
    fn table_cell_vertical_moves_clamp() {
        assert_eq!(table_cell_down(0, 1, 3), Some((1, 1)));
        assert_eq!(table_cell_down(2, 1, 3), None);
        assert_eq!(table_cell_up(1, 1), Some((0, 1)));
        assert_eq!(table_cell_up(0, 1), None);
    }

    #[test]
    fn table_cell_arrow_clamps() {
        assert_eq!(table_cell_arrow(0, 0, 3, 3, 0, -1), (0, 0));
        assert_eq!(table_cell_arrow(0, 0, 3, 3, -1, 0), (0, 0));
        assert_eq!(table_cell_arrow(2, 2, 3, 3, 1, 0), (2, 2));
        assert_eq!(table_cell_arrow(2, 2, 3, 3, 0, 1), (2, 2));
        assert_eq!(table_cell_arrow(1, 1, 3, 3, 1, 1), (2, 2));
    }
}
