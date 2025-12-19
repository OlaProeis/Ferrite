# Line Number Alignment

This document explains the line number alignment implementation in the editor widget.

## Problem

Line numbers in the editor were drifting out of sync with actual text lines. As you scrolled down, the line numbers became increasingly misaligned with their corresponding text lines.

### Root Causes

**Original Issue (v1):** Line positions calculated using multiplication:
```rust
let y_pos = gutter_rect.top() + (line_num as f32 * line_height);
```

Problems:
1. **Font metrics mismatch** - `row_height()` didn't match TextEdit's internal rendering
2. **Word wrap** - Multiple visual rows per logical line weren't handled
3. **Floating-point drift** - `line_num * line_height` accumulated errors

**Secondary Issue (v2):** Using offset from gutter rect:
```rust
let y_offset = row_y - galley_pos.y;
let text_pos = egui::pos2(gutter_rect.right() - 12.0, gutter_rect.top() + y_offset);
```

Problem: `gutter_rect.top()` ≠ `galley_pos.y` due to TextEdit's internal padding/margins.

## Solution

The fix uses **absolute screen coordinates** from the galley rows:

```rust
let text_output = text_edit.show(ui);
let galley = &text_output.galley;
let galley_pos = text_output.galley_pos;

for row in galley.rows.iter() {
    // Calculate absolute Y position (screen coordinates)
    let row_y = galley_pos.y + row.min_y();
    
    // Draw line number at EXACT same Y as the text row
    let text_pos = egui::pos2(
        gutter_rect.right() - 12.0,  // X in gutter area
        row_y,                        // Absolute Y from galley (NOT offset!)
    );
    
    painter.text(text_pos, Align2::RIGHT_TOP, line_num, font_id, color);
}
```

### Why This Works

- `galley_pos.y + row.min_y()` gives the **exact screen Y coordinate** where text renders
- Using this directly for line numbers ensures **perfect pixel alignment**
- No offset calculations means no accumulated errors
- Both gutter and TextEdit share the same coordinate system inside `ScrollArea`

### Key Implementation Details

1. **Reserve gutter space first** - Allocate space for line numbers before TextEdit
2. **Render TextEdit** - Show the editor and capture its `TextEditOutput`
3. **Use absolute positions** - Draw line numbers at `row_y` directly (not offset from gutter)
4. **Track logical lines** - Use `row.ends_with_newline` to handle word wrap

### Word Wrap Handling

With word wrap enabled, one logical line can span multiple visual rows:

```rust
let mut logical_line = 0;
let mut line_number_drawn_for_line = false;

for row in galley.rows.iter() {
    let row_y = galley_pos.y + row.min_y();
    
    // Draw line number only once per logical line (first row of wrapped line)
    if !line_number_drawn_for_line {
        let text_pos = egui::pos2(gutter_rect.right() - 12.0, row_y);
        painter.text(text_pos, Align2::RIGHT_TOP, logical_line + 1, font_id, color);
        line_number_drawn_for_line = true;
    }
    
    // Increment logical line when row ends with newline
    if row.ends_with_newline {
        logical_line += 1;
        line_number_drawn_for_line = false;
    }
}
```

## Architecture

```
ScrollArea (vertical)
└── horizontal_top
    ├── Gutter rect (allocated first, drawn after TextEdit)
    │   ├── Background fill
    │   ├── Separator line
    │   └── Line numbers at absolute row_y positions
    └── TextEdit (multiline)
        └── Returns TextEditOutput with:
            ├── galley: Arc<Galley> (rows with min_y positions)
            └── galley_pos: Pos2 (where galley renders)
```

## Files

- `src/editor/widget.rs` - Main implementation in `EditorWidget::show()`
- `src/editor/line_numbers.rs` - Helper functions for gutter width calculation

## Common Pitfalls

| Approach | Problem |
|----------|---------|
| `line_num * line_height` | Floating-point drift, font mismatch |
| `gutter_rect.top() + offset` | Gutter top ≠ galley top (TextEdit padding) |
| **`row_y` directly** ✓ | Correct - uses exact galley coordinates |

## Related

- [egui TextEditOutput](https://docs.rs/egui/latest/egui/widgets/text_edit/struct.TextEditOutput.html) - Contains `galley` and `galley_pos`
- [epaint Galley](https://docs.rs/epaint/latest/epaint/text/struct.Galley.html) - Contains `rows` with `min_y()` positions
- [epaint Row](https://docs.rs/epaint/latest/epaint/text/struct.Row.html) - Has `min_y()` and `ends_with_newline`
