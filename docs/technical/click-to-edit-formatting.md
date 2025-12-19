# Click-to-Edit for Formatted Content

## Overview

Implements a hybrid editing approach for list items and paragraphs that contain inline markdown formatting (bold, italic, code, links). Instead of trying to edit styled text directly (which would require complex rich text editing), this feature uses a click-to-edit pattern: display formatted text normally, switch to raw markdown editing on click.

## Problem Solved

In WYSIWYG mode, content with inline formatting like `**bold**` or `*italic*` was rendered as styled labels (non-editable). Simple text worked fine, but formatted content was read-only.

## Solution: Hybrid Click-to-Edit

| Mode | Behavior |
|------|----------|
| **Display** | Shows formatted text with proper styling (bold, italic, code, etc.) |
| **Edit** | On click, switches to TextEdit showing raw markdown syntax |
| **Save** | On blur, Enter, or clicking elsewhere - saves and returns to display mode |
| **Cancel** | Press Escape to discard changes and return to display mode |

## Key Files

- `src/markdown/editor.rs` - Contains all implementation:
  - `FormattedItemEditState` struct - Tracks editing state per item
  - `extract_list_item_content()` - Gets raw content from list item source
  - `extract_paragraph_content()` - Gets raw content from paragraph source
  - `render_list_item()` - Hybrid rendering for formatted list items
  - `render_paragraph()` - Hybrid rendering for formatted paragraphs

## Implementation Details

### State Structure

```rust
struct FormattedItemEditState {
    editing: bool,       // Currently in edit mode?
    edit_text: String,   // Raw markdown being edited
    needs_focus: bool,   // Request focus on next frame?
}
```

### Edit State Storage

Uses egui's temporary memory to persist state per-item across frames:

```rust
let formatted_item_id = ui.id().with("formatted_list_item").with(node.start_line);

let mut item_edit_state = ui.memory_mut(|mem| {
    mem.data
        .get_temp_mut_or_insert_with(formatted_item_id.with("edit_state"), FormattedItemEditState::default)
        .clone()
});
```

### Edit Mode Flow

1. **Enter Edit Mode** (on click):
   - Set `editing = true` and `needs_focus = true`
   - Extract raw markdown from source using line numbers
   - Store in `edit_text`

2. **During Edit Mode**:
   - Display `TextEdit::singleline` with raw markdown
   - Request focus on first frame (when `needs_focus` is true)
   - Save state every frame so edits persist

3. **Exit Edit Mode**:
   - **Enter key**: Save changes, exit
   - **Escape key**: Discard changes, exit
   - **Focus lost** (click elsewhere): Save changes, exit

### Content Extraction

For list items, strips the marker prefix:
```rust
fn extract_list_item_content(source: &str, start_line: usize) -> String {
    let line = lines[start_line - 1];
    let (_, content) = extract_line_prefix(line);  // Removes "- " or "1. " etc.
    content.to_string()
}
```

For paragraphs, extracts full line range:
```rust
fn extract_paragraph_content(source: &str, start_line: usize, end_line: usize) -> String {
    lines[(start_line - 1)..end].join("\n")
}
```

### Detection of Formatted Content

Content is considered "formatted" if it contains any of:
- `MarkdownNodeType::Strong` (bold)
- `MarkdownNodeType::Emphasis` (italic)
- `MarkdownNodeType::Strikethrough`
- `MarkdownNodeType::Link`
- `MarkdownNodeType::Code` (inline code)

## Usage

1. Open a markdown file in **Rendered view** mode
2. Click on any formatted list item or paragraph
3. Edit the raw markdown (e.g., `**bold** text`)
4. Click away or press Enter to save

## UX Indicators

- **Hover cursor**: Shows text cursor (`CursorIcon::Text`) when hovering over clickable formatted content
- **Instant focus**: Text field receives focus immediately on click

## Limitations

This is a temporary solution. Future improvements could include:
- True WYSIWYG editing using egui's `LayoutJob` for styled TextEdit
- Inline formatting buttons/shortcuts while editing
- Visual indicator for edit mode (subtle background change)

## Tests

Manual testing with `test-formatted-list.md`:
- Bold text in list items and paragraphs
- Italic text in list items and paragraphs
- Inline code in list items and paragraphs
- Links in list items and paragraphs
- Mixed formatting

## Dependencies

- egui 0.28 - UI framework, memory system for state persistence
- Existing markdown parser and AST structures

---

*Created: December 18, 2025*
*Related: HANDOVER-list-editing-bug.md*
