# CSV/TSV Table Viewer

## Overview

Ferrite v0.2.5 adds support for viewing CSV and TSV files as formatted tables. When opening `.csv` or `.tsv` files, the Rendered view mode displays the data as a scrollable table with fixed-width columns and header highlighting.

## Key Files

- `src/markdown/csv_viewer.rs` - Main CSV viewer module (parsing, widget, state, virtual scrolling)
- `src/state.rs` - FileType enum with Csv/Tsv variants and `is_tabular()` helper
- `src/app.rs` - Integration with view mode system

## Implementation Details

### File Type Detection

The `FileType` enum was extended with `Csv` and `Tsv` variants:

```rust
pub enum FileType {
    Markdown,
    Json,
    Yaml,
    Toml,
    Csv,   // .csv files
    Tsv,   // .tsv files
    Unknown,
}
```

The `is_tabular()` helper identifies tabular file types:

```rust
pub fn is_tabular(&self) -> bool {
    matches!(self, Self::Csv | Self::Tsv)
}
```

### CSV Parsing

Uses the `csv` crate (v1.3+) with flexible parsing:

- **Auto-delimiter detection**: Automatically detects comma, tab, semicolon, or pipe delimiters
- **Manual override**: Users can manually select delimiter via status bar dropdown
- **Header detection**: Automatically detects if first row is a header
- **Flexible columns**: Handles rows with varying column counts
- **Column width calculation**: Based on first 1,000 rows (sampled for performance), clamped to 50-300px

### Virtual Scrolling (Task 15)

For handling large files (10,000+ rows) efficiently:

- **No row limit**: Parses all rows (removed previous 10,000 row limit)
- **Viewport-based rendering**: Uses `ScrollArea::show_viewport()` to determine visible rows
- **Buffer rows**: Renders 5 extra rows above/below viewport for smooth scrolling
- **Memory efficient**: Only allocates UI elements for visible rows + buffer
- **Column width sampling**: Samples first 1,000 rows for width calculation to optimize large file loading

```rust
const VIRTUAL_SCROLL_BUFFER: usize = 5; // Extra rows rendered above/below viewport
```

### Table Viewer Widget

The `CsvViewer` widget provides:

- **Header row highlighting** (first row styled as bold, auto-detected or manual override)
- **Horizontal/vertical scrolling** via egui's `ScrollArea`
- **Virtual scrolling** for efficient large file handling
- **Cell tooltips** for truncated content (>50 chars)
- **Large file warning** (>1MB threshold)
- **Theme-aware colors** for light/dark modes
- **Rainbow column coloring** (optional, uses Oklch color space for perceptually even colors)

### View Mode Integration

CSV/TSV files follow the same pattern as structured files (JSON/YAML/TOML):

- **Raw mode**: Plain text editor (FerriteEditor) — always editable, including files ≥ 1 MB
- **Rendered mode**: Table viewer with inline cell editing for files **&lt; 1 MB**
- **Split mode**: Left pane raw editor, right pane table viewer (same editing rules as Rendered)

### Rendered Cell Editing (Task 15)

For files under **1 MB** (`LARGE_FILE_THRESHOLD` in `csv_viewer.rs`):

| Action | Behaviour |
|--------|-----------|
| **Click** | Select cell; requests table focus so arrow keys work immediately |
| **Double-click** | Enter inline edit mode on that cell (single-line `TextEdit` overlay) |
| **Enter** | Commit edit (or start editing the selected cell if not already editing) |
| **Escape** | Cancel edit without changing content |
| **Click away** | Commit edit when the text field loses focus |
| **Tab / Shift+Tab** | When not editing: move selection with row wrap. When editing: commit, move, and begin edit on the new cell (if editing enabled). Tab never escapes the table (`TextEdit::lock_focus(true)`); Shift+Tab is consumed before plain Tab (same egui pattern as GFM `EditableTable`) |
| **Arrow keys** | When not editing: move selection (clamped at table edges). When editing: commit current cell, move selection, exit edit mode |

**Commit path:** Updates the in-memory `CsvData` model, re-serializes all rows with `csv::Writer` (RFC 4180 quoting), writes to `Tab::content`, and sets `CsvViewerOutput.content_changed`. Navigation that leaves a dirty cell goes through `queue_cell_commit` → `apply_pending_cell_commit` so undo + content-change signaling in `central_panel.rs` stay correct.

**Preview lock:** Selection and keyboard navigation still work; inline editing is blocked when `CsvCellEditParams.cell_edit_enabled` is false.

**Shared navigation math:** `src/markdown/table_cell_nav.rs` provides pure `table_cell_next` / `table_cell_prev` (Tab wrap) and `table_cell_arrow` (arrow clamp). GFM `TableEditState` in `widgets.rs` uses the same helpers.

**Size gating (≥ 1 MB):** Rendered table is **read-only**. A persistent banner explains that cell editing is disabled and directs users to **Raw view** (`Ctrl+E`). Lazy row-index parsing is still used for scrolling performance.

**Not editable in Rendered mode:** Internal “Show Raw” preview inside the CSV widget (`csv.show_raw`) remains a non-interactive preview; use app-level Raw view for large-file text edits.

## Dependencies Used

- `csv = "1.3"` - CSV parsing with RFC 4180 compliance
- `palette = "0.7"` - Oklch color space for rainbow column colors

## Usage

1. Open any `.csv` or `.tsv` file
2. Press `Ctrl+E` to toggle between Raw and Rendered views
3. In Rendered view, the table displays with:
   - Header row at top (highlighted)
   - Scrollable content with virtual scrolling
   - Hover over truncated cells for full content tooltip
4. Use status bar dropdown to:
   - Change delimiter (auto-detect, comma, tab, semicolon, pipe)
   - Toggle header row on/off

## Test Files

Sample test files in `test_md/`:
- `test_data.csv` - Employee data with quoted fields and commas
- `test_data.tsv` - Product catalog with tab-delimited columns
- `large_test_data.csv` - 20,000 row test file for virtual scrolling
- `very_large_test_data.csv` - 500,000 row test file for benchmarking

## Performance Targets

- **10k+ row files**: Smooth 60fps scrolling with virtual scrolling
- **100MB files**: Opens in <2 seconds (parsing only, UI-ready)
- **Memory**: Proportional to file size (no truncation)
