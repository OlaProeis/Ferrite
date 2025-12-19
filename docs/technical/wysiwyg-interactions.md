# WYSIWYG Markdown Editor Interactions

This document describes the word processor-like keyboard interactions implemented in the WYSIWYG (Rendered) mode of the markdown editor.

## Overview

The WYSIWYG mode renders markdown content as editable widgets while maintaining synchronization with the underlying markdown source. Unlike raw mode where text is edited directly, rendered mode provides semantic editing where structural operations (like creating new paragraphs or list items) are handled through special keyboard shortcuts.

## Keyboard Interactions

### Enter Key Behavior

| Context | Behavior |
|---------|----------|
| **Paragraph** | Splits the paragraph at cursor position into two separate paragraphs |
| **List Item (non-empty)** | Splits the list item at cursor, creating a new item after the current one |
| **List Item (empty)** | Exits the list and creates a new paragraph below |
| **Heading** | Creates a new empty paragraph below the heading (does not split heading text) |

### Backspace Key Behavior

| Context | Behavior |
|---------|----------|
| **List Item at position 0** | Merges with previous list item, or converts to paragraph if first item |
| **First list item** | Converts the list item to a regular paragraph |

### Tab Key Behavior

| Context | Behavior |
|---------|----------|
| **List Item** | Indents the item by 2 spaces (creates nested list) |

### Shift+Tab Behavior

| Context | Behavior |
|---------|----------|
| **Nested List Item** | Outdents the item by 2 spaces (promotes to parent level) |
| **Top-level List Item** | No operation (cannot outdent further) |

## Implementation Architecture

### Module: `src/markdown/ast_ops.rs`

This module contains the core AST manipulation functions:

- **`split_paragraph()`** - Splits a paragraph at cursor position into two
- **`split_list_item()`** - Splits a list item and inserts a new item
- **`exit_list_to_paragraph()`** - Converts empty list item to paragraph
- **`heading_enter()`** - Inserts paragraph after heading
- **`merge_with_previous_list_item()`** - Merges list item with previous or converts to paragraph
- **`indent_list_item()`** - Increases indentation by 2 spaces
- **`outdent_list_item()`** - Decreases indentation by 2 spaces

### Key Types

```rust
/// Edit context passed to AST operations
pub struct EditContext {
    pub node_type: EditNodeType,    // Type of node being edited
    pub start_line: usize,          // Line number (1-indexed)
    pub end_line: usize,
    pub cursor_offset: usize,       // Character offset within text
    pub text: String,               // Current text content
    pub list_type: Option<ListType>,
    pub list_item_index: Option<usize>,
    pub nesting_depth: usize,       // 0 = top level
}

/// Result of a structural edit operation
pub struct StructuralEdit {
    pub new_source: String,         // Modified markdown source
    pub cursor_position: CursorPosition,
    pub performed: bool,            // Whether edit was applied
}
```

### Key Event Handling

Structural key events are detected in `editor.rs` using egui's input system:

```rust
fn check_structural_keys(ui: &Ui, cursor_at_start: bool) -> StructuralKeyAction {
    ui.input(|i| {
        if i.key_pressed(Key::Enter) && !i.modifiers.shift { ... }
        if i.key_pressed(Key::Backspace) && cursor_at_start { ... }
        if i.key_pressed(Key::Tab) && !i.modifiers.shift { ... }
        if i.key_pressed(Key::Tab) && i.modifiers.shift { ... }
    })
}
```

## Design Decisions

### Enter in Heading Always Creates Paragraph

When pressing Enter in a heading, the heading text is not split. Instead, a new paragraph is always created below. This matches common word processor behavior where headings are single-line elements.

### Tab in Lists Creates Nested Structure

Tab and Shift+Tab modify the markdown indentation directly (2 spaces per level), which creates nested list structure when the markdown is re-parsed.

### Backspace at List Start

When backspace is pressed at the start of a list item:
1. If there's a previous item in the list, merge with it
2. If it's the first item, convert to a paragraph

This provides intuitive "join" behavior similar to word processors.

### Source Synchronization

After any structural edit:
1. The new markdown source is generated from the modified AST
2. The editor content is updated with the new source
3. The document is re-parsed on the next frame to reflect changes

## Examples

### Splitting a Paragraph

Before (cursor at |):
```markdown
Hello| world
```

After pressing Enter:
```markdown
Hello

world
```

### Creating New List Item

Before (cursor at |):
```markdown
- First| item
- Second item
```

After pressing Enter:
```markdown
- First
- item
- Second item
```

### Exiting a List

Before (empty item):
```markdown
- First item
- |
- Third item
```

After pressing Enter:
```markdown
- First item

|
- Third item
```

### Indenting a List Item

Before:
```markdown
- First item
- Second item|
```

After pressing Tab:
```markdown
- First item
  - Second item|
```

## Limitations and Future Work

1. **Inline formatting elements**: Paragraphs with inline elements (bold, italic, links) currently don't support paragraph splitting
2. **Cursor positioning**: After structural edits, cursor positioning is approximate
3. **Undo/Redo**: Structural edits integrate with the existing undo system but may create multiple undo steps
4. **Complex nested structures**: Very deeply nested lists may have edge cases in indent/outdent behavior

## Testing

Unit tests for AST operations are in `src/markdown/ast_ops.rs`:
- `test_split_paragraph_*` - Paragraph splitting tests
- `test_split_list_item_*` - List item splitting tests  
- `test_exit_list_*` - List exit behavior tests
- `test_indent_*` / `test_outdent_*` - Indent/outdent tests
- `test_merge_*` - List item merge tests
- `test_heading_enter` - Heading behavior tests

Run tests with:
```bash
cargo test ast_ops
```
