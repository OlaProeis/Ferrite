# Line Number Display

## Overview

Line number display for the text editor, showing line numbers alongside editor content with toggleable visibility. The line numbers scroll synchronously with the editor content and use theme-appropriate muted colors.

## Key Files

- `src/editor/line_numbers.rs` - Line counting and gutter width calculation utilities
- `src/editor/widget.rs` - EditorWidget with integrated line number rendering
- `src/editor/mod.rs` - Module exports
- `src/config/settings.rs` - `show_line_numbers` setting
- `src/app.rs` - View menu toggle for line numbers

## Related Documentation

- [Line Number Alignment](./line-number-alignment.md) - Technical details on alignment fix

## Implementation Details

### Line Number Gutter

The line numbers are rendered **inside** the same `ScrollArea` as the text editor, ensuring perfect scroll synchronization:

```rust
ScrollArea::vertical().show(ui, |ui| {
    ui.horizontal_top(|ui| {
        // 1. Render line numbers gutter
        if show_line_numbers {
            // Draw background, border, and line numbers
        }
        
        // 2. Render text editor
        TextEdit::multiline(content).show(ui)
    })
});
```

### Dynamic Gutter Width

The gutter width automatically adjusts based on the number of digits needed:

```rust
pub fn calculate_gutter_width_for_lines(line_count: usize, font_size: f32) -> f32 {
    let digit_count = if line_count == 0 {
        1
    } else {
        (line_count as f32).log10().floor() as usize + 1
    };
    
    let char_width = font_size * 0.6;
    let content_width = char_width * digit_count as f32;
    
    (content_width + GUTTER_PADDING * 2.0 + GUTTER_RIGHT_MARGIN).max(MIN_GUTTER_WIDTH)
}
```

| Lines | Digits | Example Width |
|-------|--------|---------------|
| 1-9 | 1 | ~30px (minimum) |
| 10-99 | 2 | ~35px |
| 100-999 | 3 | ~42px |
| 1000+ | 4+ | Expands accordingly |

### Theme Integration

Line numbers use theme-appropriate colors:

```rust
let line_color = theme_colors.text.muted;      // Muted text for numbers
let bg_color = theme_colors.base.background_secondary;  // Subtle background
let border_color = theme_colors.base.border_subtle;     // Separator line
```

### Settings Toggle

The `show_line_numbers` boolean in `Settings` controls visibility:

```rust
// In Settings struct
pub show_line_numbers: bool,  // Default: true

// In EditorWidget
.show_line_numbers(settings.show_line_numbers)
```

Users can toggle via **View > Show Line Numbers** menu.

## Public API

### Functions

| Function | Description |
|----------|-------------|
| `count_lines(text: &str) -> usize` | Count lines in text (minimum 1 for empty) |
| `calculate_gutter_width_for_lines(line_count, font_size) -> f32` | Calculate gutter width needed |

### EditorWidget Methods

| Method | Description |
|--------|-------------|
| `.show_line_numbers(bool)` | Enable/disable line numbers |
| `.theme_colors(ThemeColors)` | Set colors for line number styling |

## Usage

### Basic Usage

```rust
EditorWidget::new(tab)
    .font_size(14.0)
    .show_line_numbers(true)
    .theme_colors(theme_colors)
    .show(ui);
```

### With Settings

```rust
EditorWidget::new(tab)
    .with_settings(&settings)  // Applies font_size, word_wrap, show_line_numbers
    .theme_colors(theme_colors)
    .show(ui);
```

## Tests

Run line number tests:

```bash
cargo test editor::line_numbers
```

### Test Coverage

- `test_count_lines_empty` - Empty text returns 1
- `test_count_lines_single_line` - Single line without newline
- `test_count_lines_multiple_lines` - Multiple lines
- `test_count_lines_trailing_newline` - Handles trailing newlines
- `test_count_lines_only_newlines` - Only newline characters
- `test_calculate_gutter_width_small` - 1-9 lines
- `test_calculate_gutter_width_medium` - 10-99 lines
- `test_calculate_gutter_width_large` - 100+ lines
- `test_calculate_gutter_width_scales_with_font` - Font size scaling
- `test_gutter_width_minimum` - Minimum width enforcement

## Visual Design

```
┌─────────────────────────────────────┐
│  1 │ # Heading                      │
│  2 │                                │
│  3 │ Some paragraph text here       │
│  4 │ that continues on this line.   │
│  5 │                                │
│  6 │ - List item 1                  │
│  7 │ - List item 2                  │
└─────────────────────────────────────┘
     ↑
     Subtle separator line
```

- **Background**: Slightly different from editor (background_secondary)
- **Numbers**: Right-aligned, muted color
- **Separator**: 1px subtle border between gutter and editor
- **Scroll**: Numbers and text scroll together as one unit

## Alignment Implementation

Line numbers are positioned using **absolute screen coordinates** from the TextEdit's galley rows to ensure perfect alignment:

```rust
for row in galley.rows.iter() {
    let row_y = galley_pos.y + row.min_y();  // Exact Y from galley
    let text_pos = egui::pos2(gutter_rect.right() - 12.0, row_y);
    painter.text(text_pos, Align2::RIGHT_TOP, line_num, font_id, color);
}
```

This approach:
- Eliminates drift from floating-point accumulation
- Handles word wrap correctly (one line number per logical line)
- Uses the same coordinate system as the text

See [Line Number Alignment](./line-number-alignment.md) for detailed technical documentation on the alignment fix.

