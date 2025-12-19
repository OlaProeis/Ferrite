# Tree Viewer for Structured Files

This document describes the JSON/YAML/TOML tree viewer implementation in Ferrite.

## Overview

The tree viewer provides a specialized rendered view for structured data files (`.json`, `.yaml`, `.yml`, `.toml`). When a file with one of these extensions is opened and the view mode is set to "Rendered", the editor displays an interactive collapsible tree structure instead of the standard markdown WYSIWYG view.

## Features

### File Type Detection

Files are automatically detected by extension:
- `.json` → JSON parser
- `.yaml`, `.yml` → YAML parser
- `.toml` → TOML parser

Detection happens at render time based on the file path associated with the active tab.

### Unified Tree Model

All formats are parsed into a unified `TreeNode` enum:

```rust
pub enum TreeNode {
    Null,           // null/nil values
    Bool(bool),     // true/false
    Integer(i64),   // whole numbers
    Float(f64),     // decimal numbers
    String(String), // text values
    Array(Vec<TreeNode>),           // lists
    Object(Vec<(String, TreeNode)>), // key-value maps
}
```

### Tree Rendering

- **Collapsible nodes**: Objects and arrays show ▼/▶ toggle buttons
- **Syntax coloring**:
  - Keys: Blue
  - Strings: Green
  - Numbers: Orange
  - Booleans: Purple
  - Null: Gray
  - Brackets: Light gray
- **Hierarchy display**: Proper indentation shows nesting levels
- **Item counts**: Arrays show `[N items]`, objects show `{...} (N keys)`

### Toolbar

The toolbar provides:
- **File type label**: Shows "JSON", "YAML", or "TOML"
- **Expand All**: Expands all collapsed nodes
- **Collapse All**: Collapses all expandable nodes
- **Raw View/Tree View**: Toggles between tree and raw text view

### Inline Editing

- **Double-click** on a leaf value to edit it
- **Enter** commits the change
- **Escape** or clicking away cancels editing
- Values are validated on commit:
  - `null`, `true`, `false` for primitives
  - Numbers parsed appropriately
  - Text treated as strings
- Error badge shown for invalid values

### Context Menu

Right-click on any node to access:
- **Copy Path**: Copies the JSONPath to the clipboard (e.g., `$.users[0].name`)

### Large File Handling

Files larger than 1MB show a warning banner:
- Option to dismiss the warning and continue
- Option to switch to raw view for better performance
- Large file warning is per-session (dismissed state is remembered)

### Error Handling

If parsing fails:
1. Error message displayed with details
2. Automatic fallback to raw text view
3. User can still view and edit the file in raw mode

## Implementation

### Key Files

- `src/markdown/tree_viewer.rs` - Main module containing:
  - `StructuredFileType` enum for file detection
  - `TreeNode` enum and parsers
  - `TreeViewerState` for widget state (expansion, editing)
  - `TreeViewer` widget
  - Serialization functions for saving changes

### Integration

The tree viewer is integrated into `src/app.rs`:

```rust
// In render_ui(), when in Rendered view mode:
if let Some(file_type) = get_structured_file_type(path) {
    // Use TreeViewer for structured files
    TreeViewer::new(&mut tab.content, file_type, tree_state)
        .font_size(font_size)
        .show(ui);
} else {
    // Use MarkdownEditor for markdown files
    MarkdownEditor::new(&mut tab.content)
        .mode(EditorMode::Rendered)
        .show(ui);
}
```

### State Management

Tree viewer state is stored per-tab using a `HashMap<usize, TreeViewerState>` in `FerriteApp`:
- Keyed by tab ID
- Preserves expansion state, editing state, etc.
- Cleaned up automatically when tabs are closed

### Dependencies

Added to `Cargo.toml`:
- `serde_yaml = "0.9"` - YAML parsing
- `toml = "0.8"` - TOML parsing
- `serde_json` - Already present for JSON parsing

## Usage

1. Open a `.json`, `.yaml`, `.yml`, or `.toml` file
2. Toggle to "Rendered" view mode (Ctrl+Shift+V)
3. Navigate the tree:
   - Click ▼/▶ to expand/collapse
   - Use toolbar for bulk expand/collapse
4. Edit values:
   - Double-click a value
   - Type new value
   - Press Enter to save
5. Copy paths:
   - Right-click any node
   - Select "Copy Path"

## Limitations

- TOML null values are not supported (TOML specification limitation)
- Very deeply nested structures may have display issues
- Arrays and objects cannot be edited inline (values only)
- Structural changes (add/remove keys) require raw editing

## Future Enhancements

Potential improvements for future versions:
- Add/remove keys and array items through UI
- Drag-and-drop reordering
- Search within tree
- Syntax-highlighted raw view
- Schema validation for JSON files
- Format conversion (JSON ↔ YAML ↔ TOML)
