# Text Editor Widget

## Overview

The EditorWidget provides the main text editing interface for Ferrite. It wraps egui's `TextEdit::multiline` widget with additional functionality for cursor position tracking, scroll persistence, and integration with the application state.

## Key Files

- `src/editor/mod.rs` - Module declaration and exports
- `src/editor/widget.rs` - EditorWidget implementation with builder pattern and line numbers
- `src/editor/line_numbers.rs` - Line counting utilities
- `src/editor/stats.rs` - Text statistics (word/char/line counting)

## Implementation Details

### EditorWidget Structure

The widget uses a builder pattern for flexible configuration:

```rust
EditorWidget::new(tab)
    .font_size(14.0)
    .word_wrap(true)
    .show_line_numbers(true)
    .theme_colors(theme_colors)
    .id(egui::Id::new("main_editor"))
    .show(ui);
```

### Core Features

| Feature | Implementation |
|---------|----------------|
| Text editing | egui `TextEdit::multiline` handles input natively |
| Cursor movement | Arrow keys, Home/End, Page Up/Down via egui |
| Text selection | Mouse drag and Shift+Arrow supported natively |
| Clipboard | Ctrl+C/X/V handled by egui |
| Scrolling | Wrapped in `ScrollArea` with offset persistence |
| Line numbers | Optional gutter with sync scrolling (see [Line Numbers](./line-numbers.md)) |

### Cursor Position Tracking

The widget converts egui's character-based cursor index to `(line, column)` coordinates:

```rust
fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    
    for (i, ch) in text.chars().enumerate() {
        if i >= char_index { break; }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}
```

### Integration with Tab State

The editor integrates directly with the `Tab` struct:

- **Content**: Mutates `tab.content` directly via TextEdit
- **Cursor**: Updates `tab.cursor_position` as `(line, col)`
- **Scroll**: Persists `tab.scroll_offset` from ScrollArea

### EditorOutput

The `show()` method returns useful information:

```rust
pub struct EditorOutput {
    pub response: Response,      // egui interaction response
    pub changed: bool,           // Whether content was modified
    pub cursor_position: (usize, usize), // Current (line, col)
}
```

## Dependencies Used

- `egui` - TextEdit::multiline, ScrollArea, FontId
- `eframe` - Re-exports egui types

## Usage

In `app.rs`, the editor is rendered in the central panel:

```rust
let font_size = self.state.settings.font_size;
let word_wrap = self.state.settings.word_wrap;

if let Some(tab) = self.state.active_tab_mut() {
    let editor_output = EditorWidget::new(tab)
        .font_size(font_size)
        .word_wrap(word_wrap)
        .id(egui::Id::new("main_editor"))
        .show(ui);
}
```

## Tests

Run editor-specific tests:

```bash
cargo test editor::widget::tests
```

Test coverage includes:
- `test_char_index_to_line_col_*` - Cursor position conversion
- `test_line_col_to_char_index_*` - Reverse conversion
- `test_roundtrip_conversion` - Bidirectional consistency

## Related Documentation

- [Line Numbers](./line-numbers.md) - Line number gutter implementation
- [Line Number Alignment](./line-number-alignment.md) - Technical fix for alignment drift
- [Text Statistics](./text-statistics.md) - Word/character counting in status bar
- [WYSIWYG Editor](./wysiwyg-editor.md) - Rendered view mode using MarkdownEditor
