# Word Wrap Support

## Overview

Word wrap support allows long lines to wrap to fit within the available editor width, improving readability without requiring horizontal scrolling. This is implemented in Phase 2 of the Ferrite editor development.

## User Toggle (#145)

Word wrap is a persisted global setting (`Settings::word_wrap`, default `true`). Users can toggle it from:

| Entry point | Location |
|-------------|----------|
| **Alt+Z** / **Option+Z** | `ShortcutCommand::ToggleWordWrap` → `handle_toggle_word_wrap()` in `src/app/navigation.rs` |
| **Command palette** | Same handler via `dispatch_palette_command` in `src/app/central_panel.rs` |
| **Settings → Editor** | Checkbox in `src/ui/settings.rs` (existing) |

The shortcut is rebindable in Settings → Keyboard. Listed in F1 Help (View) and the keyboard settings list.

### Immediate effect

`central_panel.rs` reads `settings.word_wrap` each frame and passes it to `EditorWidget::word_wrap()` and `MarkdownEditor::word_wrap()` for raw, rendered, and split panes.

### Very large files (100K+ lines)

Files at or above the uniform-height threshold (`ViewState::configure_for_file_size`, 100 000 lines) force-disable wrap in the editor regardless of the setting (`effective_wrap_enabled = wrap_enabled && !uses_uniform_heights()` in `FerriteEditor`).

When the user toggles wrap while the active tab is in uniform-height mode:

- **`settings.word_wrap` still flips** and persists (applies to other tabs and after switching away).
- A toast shows: *"Word wrap is disabled for very large files"* (`notification.word_wrap_large_file`).
- On normal-sized tabs, enabled/disabled toasts use `notification.word_wrap_enabled` / `word_wrap_disabled`.

No toolbar icon (Tier C, deferred).

## Architecture

### Key Components

1. **ViewState** (`src/editor/ferrite/view.rs`)
   - Tracks wrap width and wrap state
   - Stores per-line wrap information (`WrapInfo`)
   - Calculates cumulative heights for wrapped lines
   - Converts between logical lines/columns and visual rows

2. **LineCache** (`src/editor/ferrite/line_cache.rs`)
   - Caches wrapped galleys keyed by content, font, color, AND wrap width
   - Uses egui's `painter.layout()` with wrap width for wrapped text

3. **FerriteEditor** (`src/editor/ferrite/editor.rs`)
   - Manages wrap enabled state
   - Integrates wrap width from available text area
   - Renders wrapped lines with proper y-positioning

4. **Cursor Rendering** (`src/editor/ferrite/rendering/cursor.rs`)
   - Positions cursor within wrapped galleys
   - Calculates correct visual row for cursor

5. **Keyboard Navigation** (`src/editor/ferrite/input/keyboard.rs`)
   - Visual row-based up/down navigation when wrap is enabled
   - Maintains approximate column position across visual rows

## Data Structures

### WrapInfo

```rust
pub struct WrapInfo {
    /// Number of visual rows this logical line occupies
    pub visual_rows: usize,
    /// Total height of this line in pixels
    pub height: f32,
}
```

### ViewState Extensions

```rust
// New fields in ViewState
wrap_width: Option<f32>,           // None = no wrapping
wrap_info: Vec<WrapInfo>,          // Per-line wrap info
cumulative_heights: Vec<f32>,      // For fast y-offset lookup
total_content_height: f32,         // Cached total height
```

## Key Methods

### ViewState

| Method | Description |
|--------|-------------|
| `enable_wrap(width)` | Enable word wrap at specified width |
| `disable_wrap()` | Disable word wrap |
| `is_wrap_enabled()` | Check if wrap is enabled |
| `set_line_wrap_info(line, rows, height)` | Update wrap info for a line |
| `rebuild_height_cache(total_lines)` | Rebuild cumulative heights after wrap changes |
| `get_line_height(line)` | Get height of a line (wrapped or default) |
| `get_visual_rows(line)` | Get visual row count for a line |
| `get_line_y_offset(line)` | Get y-offset for a line (uses cumulative heights) |
| `total_content_height(total_lines)` | Get total document height |
| `logical_to_visual_row(line, col, chars_per_row)` | Convert logical position to visual row |
| `visual_row_to_logical(visual_row, total_lines)` | Convert visual row to logical position |

### LineCache

| Method | Description |
|--------|-------------|
| `get_galley_wrapped(content, painter, font, color, wrap_width)` | Get or create wrapped galley |

### FerriteEditor

| Method | Description |
|--------|-------------|
| `enable_wrap()` | Enable word wrap |
| `disable_wrap()` | Disable word wrap |
| `set_wrap_enabled(enabled)` | Set wrap state |
| `is_wrap_enabled()` | Check wrap state |

## Rendering Flow

1. Editor calculates available text area width
2. If wrap is enabled, calls `view.enable_wrap(text_area_width)`
3. For each visible line:
   - If wrap enabled: use `line_cache.get_galley_wrapped()` with wrap width
   - Update `view.set_line_wrap_info()` with galley's row count and height
   - Position line using cumulative heights from ViewState
4. After rendering visible lines, call `view.rebuild_height_cache()`

## Cursor Positioning in Wrapped Text

When word wrap is enabled, cursor positioning requires finding:
1. Which visual row the cursor column falls on
2. The x-offset within that visual row
3. The y-offset from the line's top (accounting for the visual row)

### Implementation

The cursor rendering module (`src/editor/ferrite/rendering/cursor.rs`) uses egui's built-in cursor positioning API for accurate results:

```rust
// Convert cursor column to egui's CCursor (character cursor)
let ccursor = egui::text::CCursor::new(cursor_col);

// Get the galley cursor which tracks position within wrapped text
let galley_cursor = galley.from_ccursor(ccursor);

// Get the cursor rectangle relative to galley origin
// CRITICAL: cursor_rect.min.y contains the Y offset from galley top,
// which accounts for which visual row the cursor is on
let cursor_rect = galley.pos_from_cursor(&galley_cursor);

// Final cursor position
let cursor_x = text_start_x + cursor_rect.min.x;  // X within visual row
let cursor_y = line_top_y + cursor_rect.min.y;    // Y accounts for wrapped rows
```

### Key Insight

The `galley.pos_from_cursor()` method returns a `Rect` where:
- `min.x` is the X offset within the current visual row
- `min.y` is the Y offset from the galley's top, automatically accounting for which visual row contains the cursor

For example, if a line wraps to 4 visual rows (each ~16px tall) and the cursor is on row 3:
- `cursor_rect.min.y` ≈ 48.0 (row index 3 × ~16px per row)
- Final `cursor_y = line_top_y + 48.0` correctly positions the cursor on row 3

### Why Not Manual Byte Offset Calculation

Previous implementations attempted manual byte offset iteration through galley rows, which was error-prone:
- Byte offset calculations don't always match egui's internal layout
- Edge cases with Unicode, combining characters, and ligatures
- Maintenance burden for complex logic

Using `galley.pos_from_cursor()` delegates to egui's battle-tested text layout engine.

### Public API

For selection rendering and other features that need cursor position without rendering:

```rust
pub fn get_cursor_position(
    painter: &egui::Painter,
    buffer: &TextBuffer,
    cursor: &Cursor,
    view: &ViewState,
    font_id: &FontId,
    text_start_x: f32,
    line_top_y: f32,
    wrap_width: f32,
) -> (f32, f32, f32)  // Returns (x, y, height)
```

## Visual Navigation

Up/down arrow keys move by visual row when wrap is enabled:
- If cursor is not on the first/last visual row of a line, it moves within the same logical line
- If cursor is on first visual row, up moves to the last visual row of the previous line
- If cursor is on last visual row, down moves to the first visual row of the next line

## Scrollbar Integration

Total document height for scrollbar sizing uses:
- Sum of all wrapped line heights (from cumulative_heights)
- Falls back to `total_lines * line_height` if no wrap info available

## Performance Considerations

1. **Galley Caching**: Wrapped galleys are cached by content + wrap width to avoid re-layout each frame
2. **Incremental Updates**: Only visible lines update their wrap info
3. **Cumulative Heights**: Pre-computed for O(1) y-offset lookup
4. **Cache Invalidation**: Cache is invalidated when:
   - Content changes
   - Wrap width changes
   - Wrap is enabled/disabled

## Testing

Word wrap tests cover:
- Enable/disable wrap
- Minimum wrap width enforcement
- Per-line wrap info tracking
- Cumulative height calculation
- Visual row conversion
- Logical to visual row mapping

See `src/editor/ferrite/view.rs` test module for comprehensive tests.

## Future Improvements

1. **Preferred Column Tracking**: Maintain x-position when moving between visual rows
2. **Word Break Points**: Integrate with selection to select whole words
3. **Zen Mode Integration**: Center wrapped content with configurable margins
4. **Soft vs Hard Wrap**: Option for soft (display-only) vs hard (insert newlines) wrap
