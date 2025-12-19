# Markdown Formatting Toolbar and Keyboard Shortcuts

## Overview

This feature implements a comprehensive markdown formatting system with toolbar integration and keyboard shortcuts. It provides formatting commands that work in both Raw and Rendered editor modes, with proper selection handling and formatting state reflection in the UI.

## Key Files

- `src/markdown/formatting.rs` - Core formatting command model and raw-mode formatting logic
- `src/ui/ribbon.rs` - Toolbar integration with formatting buttons
- `src/app.rs` - Keyboard shortcut handling and format command application
- `src/state.rs` - Selection tracking in Tab state
- `src/editor/widget.rs` - Selection capture from egui TextEdit
- `src/markdown/editor.rs` - Focus tracking for rendered mode

## Implementation Details

### Command Model

The `MarkdownFormatCommand` enum defines all supported formatting operations:

```rust
pub enum MarkdownFormatCommand {
    Bold,           // **text**
    Italic,         // *text*
    InlineCode,     // `code`
    Strikethrough,  // ~~text~~
    Link,           // [text](url)
    Image,          // ![alt](url)
    CodeBlock,      // ```code```
    Heading(u8),    // # to ######
    BulletList,     // - item
    NumberedList,   // 1. item
    Blockquote,     // > quote
}
```

### Formatting State Detection

The `FormattingState` struct tracks current formatting at cursor position for toolbar state reflection:

```rust
pub struct FormattingState {
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_inline_code: bool,
    pub heading_level: Option<HeadingLevel>,
    pub is_bullet_list: bool,
    pub is_numbered_list: bool,
    pub is_blockquote: bool,
    // ... other fields
}
```

### Selection Handling

**Raw Mode:**
- Selection is captured from egui's TextEdit `cursor_range`
- Stored in `Tab.selection` as `Option<(usize, usize)>`
- Keyboard shortcuts processed AFTER editor renders to ensure fresh selection

**Rendered Mode:**
- Focus tracking via `FocusedElement` struct
- Each editable widget (heading, paragraph, list item) reports focus and selection
- Selection converted to absolute character positions in source markdown

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+B | Bold |
| Ctrl+I | Italic |
| Ctrl+K | Link |
| Ctrl+Shift+K | Image |
| Ctrl+` | Inline Code |
| Ctrl+Shift+C | Code Block |
| Ctrl+1 to Ctrl+6 | Headings H1-H6 |
| Ctrl+Shift+B | Bullet List |
| Ctrl+Shift+N | Numbered List |
| Ctrl+Q | Blockquote |

### Timing Architecture

Critical for correct behavior:
1. `render_ui()` runs - editor widgets capture selection
2. Format actions from ribbon are **deferred** (returned from render_ui)
3. `handle_keyboard_shortcuts()` runs
4. Deferred ribbon format actions applied
5. All formatting uses fresh selection data

### Behavior Notes

- **No selection**: Inline formatting (bold, italic, code, link) does nothing - prevents placeholder insertion
- **With selection**: Selected text is wrapped with appropriate markers
- **Toggle behavior**: Re-applying same format to already-formatted text removes the formatting
- **Multi-line**: Block formats (list, blockquote, heading) affect all selected lines

## Dependencies Used

- `egui` - UI rendering and TextEdit selection handling
- Existing `comrak` AST - For detecting formatting state at cursor

## Usage

### Toolbar

The Ribbon UI contains a "Format" group with buttons:
- B (Bold), I (Italic), `<>` (Inline Code), `[~]` (Link)
- Heading dropdown (H1-H6)
- `-` (Bullet List), `1.` (Numbered List), `>` (Blockquote), `{}` (Code Block)

Buttons show active state when cursor is inside formatted text.

### API

```rust
// Apply formatting command
let result = apply_raw_format(&content, selection, MarkdownFormatCommand::Bold);

// Detect formatting at position
let state = detect_raw_formatting_state(&content, cursor_position);
```

## Tests

Run formatting tests:
```bash
cargo test markdown::formatting
```

Key test coverage:
- Bold/italic with and without selection
- Heading level changes and toggles
- List formatting (bullet, numbered)
- Blockquote toggle
- Code block wrapping
- Link/image formatting
- Formatting state detection

## Known Limitations

1. **Rendered Mode**: Selection tracking works for simple elements but complex inline-formatted content uses click-to-edit approach
2. **AST-based Transforms**: Raw mode uses text manipulation; rendered mode doesn't yet do full AST transforms
3. **Toggle Detection**: Some edge cases with nested formatting may not toggle correctly
