# Table Editing Test Cases

## Simple 2x2 Table

| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |

Test:
- Click on a cell and type multiple characters
- Focus should persist (no need to re-click after each keystroke)

## 3x3 Table with Navigation

| Name    | Age | City     |
|---------|-----|----------|
| Alice   | 25  | New York |
| Bob     | 30  | London   |

Test keyboard navigation:
- Tab: moves to next cell (right, then wraps to next row)
- Shift+Tab: moves to previous cell (left, then wraps to previous row)
- Enter: moves to next row (same column)
- Escape: exits editing mode (unfocuses all cells)

## Wide Table (5 columns)

| Column 1 | Column 2 | Column 3 | Column 4 | Column 5 |
|----------|----------|----------|----------|----------|
| A1       | B1       | C1       | D1       | E1       |
| A2       | B2       | C2       | D2       | E2       |
| A3       | B3       | C3       | D3       | E3       |

Test:
- Tab through all cells in sequence
- Enter should move down within the same column

## Table with Alignment

| Left | Center | Right |
|----|:-----|:----|
| Text | Text   | Text  |
| More | More   | More  |

## Long Content Table

| Description                        | Value |
|------------------------------------|-------|
| A longer piece of text in a cell   | 100   |
| Another longer description here    | 200   |