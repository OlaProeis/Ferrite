# Editable Tables Widget

## Overview

The `EditableTable` widget provides a fully interactive table editing experience in the WYSIWYG markdown editor. Users can edit cell content, add/remove rows and columns, and control column alignment - all while the widget automatically regenerates valid GitHub Flavored Markdown (GFM) table syntax.

## Features

- **Editable Cells**: Each cell renders as a `TextEdit` field for inline editing
- **Dynamic Cell Width**: Columns auto-size based on content (min 60px, max 400px)
- **Add/Remove Rows**: `➕ Add row` button and 🗑 delete buttons per row
- **Add/Remove Columns**: `➕` button to add columns, `×` buttons to delete columns
- **Automatic Markdown Generation**: All changes regenerate valid GFM table syntax
- **Theme-Aware Styling**: Adapts colors for dark and light modes
- **Persistent State**: Table data persists across frames using egui's memory

> **Note:** Column alignment controls are disabled in the UI but the data structure (`TableData.alignments`) supports them for future implementation.

## Architecture

### Core Types

#### `TableCellData`
```rust
pub struct TableCellData {
    pub text: String,
}
```
Represents a single cell's content.

#### `TableData`
```rust
pub struct TableData {
    pub rows: Vec<Vec<TableCellData>>,  // First row is header
    pub alignments: Vec<TableAlignment>,
    pub num_columns: usize,
}
```
Complete table state including rows, alignments, and column count.

#### `EditableTable`
```rust
pub struct EditableTable<'a> {
    data: &'a mut TableData,
    font_size: f32,
    colors: Option<WidgetColors>,
    show_controls: bool,
    show_alignment_controls: bool,
    id: Option<egui::Id>,
}
```
The egui widget that renders the interactive table.

### File Locations

- **Widget Implementation**: `src/markdown/widgets.rs`
  - `TableCellData`, `TableData`, `EditableTable` structs
  - Table manipulation methods (add/remove/insert row/column)
  - Markdown generation (`to_markdown()`)
  
- **Editor Integration**: `src/markdown/editor.rs`
  - `render_table()` function uses `EditableTable`
  - `update_table_in_source()` for source synchronization

## Usage

### Creating from Markdown AST Node

```rust
use crate::markdown::widgets::{EditableTable, TableData};

let mut table_data = TableData::from_node(&table_node);

let output = EditableTable::new(&mut table_data)
    .font_size(14.0)
    .with_controls(true)
    .with_alignment_controls(true)
    .show(ui);

if output.changed {
    // output.markdown contains regenerated table syntax
    update_source(source, &output.markdown);
}
```

### Creating Programmatically

```rust
let mut table = TableData::new(3, 2); // 3 columns, 2 rows
table.rows[0][0].text = "Header 1".to_string();
table.rows[0][1].text = "Header 2".to_string();
table.rows[0][2].text = "Header 3".to_string();
table.rows[1][0].text = "Data 1".to_string();
table.rows[1][1].text = "Data 2".to_string();
table.rows[1][2].text = "Data 3".to_string();

table.set_column_alignment(0, TableAlignment::Left);
table.set_column_alignment(1, TableAlignment::Center);
table.set_column_alignment(2, TableAlignment::Right);
```

## Markdown Generation

The `to_markdown()` method generates valid GFM table syntax:

### Input State
```rust
TableData {
    rows: [
        ["Left", "Center", "Right"],
        ["A", "B", "C"],
    ],
    alignments: [Left, Center, Right],
    num_columns: 3,
}
```

### Generated Markdown
```markdown
| Left   | Center | Right |
|:-------|:------:|------:|
| A      | B      | C     |
```

### Alignment Markers

| Alignment | Separator Format |
|-----------|------------------|
| None      | `---`            |
| Left      | `:---`           |
| Center    | `:---:`          |
| Right     | `---:`           |

## Table Manipulation Methods

### Row Operations

```rust
// Add row at end
table.add_row();

// Insert row at specific index
table.insert_row(1); // Insert after header

// Remove row (protects last row)
table.remove_row(2);
```

### Column Operations

```rust
// Add column at end
table.add_column();

// Insert column at specific index
table.insert_column(1);

// Remove column (protects last column)
table.remove_column(0);
```

### Alignment Control

```rust
// Set specific alignment
table.set_column_alignment(0, TableAlignment::Center);

// Cycle through: None → Left → Center → Right → None
table.cycle_column_alignment(0);
```

## UI Layout

```
┌─────────────────────────────────────────────────┐
│ Align: [⬅] [⬌] [➡]                              │  ← Alignment controls
├─────────────────────────────────────────────────┤
│ [🗑] │ Header 1 │ Header 2 │ Header 3 │ [➕]    │  ← Header row + add column
│ [🗑] │ Cell 1   │ Cell 2   │ Cell 3   │         │  ← Data row
│ [🗑] │ Cell 4   │ Cell 5   │ Cell 6   │         │  ← Data row
├─────────────────────────────────────────────────┤
│ [➕ Add row]                                     │  ← Add row button
├─────────────────────────────────────────────────┤
│ Del col: [×] [×] [×]                            │  ← Column delete buttons
└─────────────────────────────────────────────────┘
```

## Integration with Editor

The `render_table()` function in `editor.rs` integrates `EditableTable`:

1. **Creates unique ID** from table's source line position
2. **Stores `TableData`** in egui's frame-persistent memory
3. **Renders `EditableTable`** with appropriate styling
4. **Syncs changes** back to markdown source when modified

```rust
fn render_table(ui, node, source, edit_state, colors, font_size) {
    let table_id = ui.id().with("table").with(node.start_line);
    
    // Retrieve or create persistent table data
    let mut table_data = ui.memory_mut(|mem| {
        mem.data.get_temp_mut_or_insert_with(table_id, || {
            TableData::from_node(node)
        }).clone()
    });
    
    // Show widget
    let output = EditableTable::new(&mut table_data)
        .font_size(font_size)
        .show(ui);
    
    // Update source on change
    if output.changed {
        update_table_in_source(source, node.start_line, node.end_line, &output.markdown);
    }
}
```

## Tests

The widget includes comprehensive tests in `src/markdown/widgets.rs`:

- `test_table_cell_data_new` - Cell creation
- `test_table_data_new` - Table initialization
- `test_table_data_add_row` / `test_table_data_insert_row` - Row addition
- `test_table_data_remove_row` / `test_table_data_remove_row_protects_last` - Row removal
- `test_table_data_add_column` / `test_table_data_insert_column` - Column addition
- `test_table_data_remove_column` / `test_table_data_remove_column_protects_last` - Column removal
- `test_table_data_set_alignment` / `test_table_data_cycle_alignment` - Alignment control
- `test_table_data_to_markdown_basic` / `test_table_data_to_markdown_with_alignment` - Markdown generation
- `test_table_row_count` / `test_table_has_header` - Table properties

Run tests with:
```bash
cargo test table
```

## Future Enhancements

Potential improvements for future iterations:

1. **Column Resizing**: Drag-to-resize column widths
2. **Row Reordering**: Drag-and-drop to reorder rows
3. **Cell Formatting**: Support for bold/italic/code within cells
4. **Multi-select**: Select multiple cells for bulk operations
5. **Copy/Paste**: Clipboard support for table data
6. **Column Sorting**: Sort rows by column values
7. **Undo/Redo**: Table-specific undo stack

## Related Documentation

- [WYSIWYG Editor](./wysiwyg-editor.md) - Overall editor architecture
- [Editable Widgets](./editable-widgets.md) - Other editable widgets (headings, lists, etc.)
- [Markdown Parser](./markdown-parser.md) - AST structure for tables
