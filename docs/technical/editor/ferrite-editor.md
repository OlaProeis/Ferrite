# FerriteEditor Widget

## Overview

`FerriteEditor` is a custom text editor widget that integrates Phase 1 modules for high-performance text editing with virtual scrolling and efficient rendering.

## Key Files

The Ferrite editor is organized in a modular subfolder structure at `src/editor/ferrite/`:

| File | Description |
|------|-------------|
| `src/editor/ferrite/mod.rs` | Module exports and re-exports |
| `src/editor/ferrite/editor.rs` | Main FerriteEditor widget |
| `src/editor/ferrite/buffer.rs` | TextBuffer - rope-based text storage |
| `src/editor/ferrite/history.rs` | EditHistory types + diff (`Tab` owns the stack) |
| `src/editor/ferrite/view.rs` | ViewState - viewport tracking |
| `src/editor/ferrite/line_cache.rs` | LineCache - galley caching |
| `src/editor/ferrite/cursor.rs` | Cursor - position tracking |
| `src/editor/ferrite/input/mod.rs` | InputHandler - event dispatch |
| `src/editor/ferrite/input/keyboard.rs` | Keyboard input handling |
| `src/editor/ferrite/input/mouse.rs` | Mouse input handling |
| `src/editor/ferrite/rendering/mod.rs` | Rendering coordinator |
| `src/editor/ferrite/rendering/gutter.rs` | Line number gutter rendering |
| `src/editor/ferrite/rendering/text.rs` | Text galley rendering |
| `src/editor/ferrite/rendering/cursor.rs` | Cursor/caret rendering |

## Architecture

```
src/editor/ferrite/
├── mod.rs                    # Re-exports: FerriteEditor, TextBuffer, etc.
├── editor.rs                 # FerriteEditor struct + ui() coordinator
├── buffer.rs                 # TextBuffer - rope-based content (ropey)
├── history.rs                # EditHistory module (owned by Tab, not FerriteEditor)
├── view.rs                   # ViewState - viewport tracking
├── line_cache.rs             # LineCache - LRU galley cache (200 entries)
├── cursor.rs                 # Cursor - line/column position
├── input/
│   ├── mod.rs                # InputHandler dispatch + InputResult
│   ├── keyboard.rs           # Key events (arrows, backspace, etc.)
│   └── mouse.rs              # Mouse wheel scrolling
└── rendering/
    ├── mod.rs                # Rendering coordinator
    ├── gutter.rs             # Line numbers (right-aligned)
    ├── text.rs               # Text galley rendering
    └── cursor.rs             # Cursor/caret drawing
```

**Component Relationships:**
```
FerriteEditor
├── TextBuffer      - Rope-based content storage (ropey)
├── (undo via Tab)  - Rope edits synced to tab.content → tab.edit_history
├── ViewState       - Viewport tracking, visible line range
├── LineCache       - LRU galley cache (200 entries)
├── Cursor          - Simple line/column position
├── input/          - Input event processing (keyboard + mouse)
├── rendering/      - Visual rendering (gutter, text, cursor)
└── ui()            - egui widget method (coordinator)
```

## Struct Definition

```rust
pub struct FerriteEditor {
    buffer: TextBuffer,           // Text content (undo recorded on Tab after sync)
    view: ViewState,              // Viewport state
    line_cache: LineCache,        // Galley cache
    selection: Selection,         // Current selection (anchor + head)
    font_size: f32,               // Rendering font size
    content_dirty: bool,          // Cache invalidation flag
    wrap_enabled: bool,           // Word wrap toggle
    max_wrap_width: Option<f32>,  // Maximum wrap width
    // Multi-click and drag tracking
    last_click_time: Option<Instant>,
    click_count: u32,
    last_click_pos: Option<Cursor>,
    drag_start_cursor: Option<Cursor>,
    // Syntax highlighting (Phase 2)
    syntax_enabled: bool,         // Whether syntax highlighting is on
    syntax_language: Option<String>, // Language identifier (e.g., "rust")
    syntax_dark_mode: bool,       // Dark/light theme selection
    syntax_theme_hash: u64,       // Cache invalidation on theme change
}

pub struct Cursor {
    pub line: usize,          // 0-indexed line
    pub column: usize,        // 0-indexed column (char-based)
}

pub struct Selection {
    pub anchor: Cursor,       // Fixed point of selection
    pub head: Cursor,         // Moving point (follows cursor)
}
```

**Selection vs Cursor:**
- `Cursor` is a single position (line, column)
- `Selection` has two cursors: `anchor` (where selection started) and `head` (current cursor)
- When `anchor == head`, selection is "collapsed" (just a cursor, no range)
- `selection.head` is the logical cursor position

## Key Methods

| Method | Description |
|--------|-------------|
| `new()` | Create empty editor |
| `from_string(content)` | Create with initial content |
| `ui(&mut self, ctx, ui) -> Response` | Main egui widget method |
| `cursor()` | Get current cursor position (`selection.head`) |
| `set_cursor(cursor)` | Set cursor (clamped to valid range) |
| `selection()` | Get current selection |
| `set_selection(selection)` | Set selection |
| `has_selection()` | Check if there's a range selection |
| `selected_text()` | Get text within selection range |
| `delete_selection()` | Delete selected text, collapse to anchor |
| `select_all()` | Select entire document |
| `set_font_size(size)` | Set font size (8.0-72.0) |
| `enable_wrap()` / `disable_wrap()` | Toggle word wrap |
| `mark_dirty()` | Invalidate cache for next render |

## Rendering Pipeline (`ui()`)

1. **Cache Check**: Invalidate `LineCache` if `content_dirty`
2. **Layout**: Calculate gutter width, text area
3. **Visible Range**: Get lines from `ViewState::get_visible_line_range()`
4. **Gutter Rendering**: Draw line numbers (right-aligned)
5. **Text Rendering**: Draw visible lines using cached galleys
6. **Cursor Rendering**: Draw cursor at current position

## Usage

```rust
use ferrite::editor::{FerriteEditor, Cursor};

// Create editor
let mut editor = FerriteEditor::from_string("Hello\nWorld");

// Set cursor
editor.set_cursor(Cursor::new(1, 3)); // Line 1, column 3

// In egui update loop
egui::CentralPanel::default().show(ctx, |ui| {
    let response = editor.ui(ctx, ui);
    // Handle response (clicks, etc.)
});
```

## EditorWidget Integration

FerriteEditor is the default editor in `EditorWidget`.

**Build:**
```bash
cargo build
```

**Integration details (`src/editor/widget.rs`):**
- `EditorWidget::show()` renders the `FerriteEditor` widget
- Each tab gets its own FerriteEditor instance stored in egui's memory

**Content synchronization:**
- Tab.content → FerriteEditor buffer (automatic on content change)
- FerriteEditor buffer → Tab.content (Phase 2, after keyboard input)
- Cursor position is preserved across sync operations

**FerriteEditor storage:**
- Editors are stored in egui's memory keyed by tab ID
- Content hashes track external changes for re-sync
- Editors persist across frames (not recreated each render)

**Large file handling:**
- Files > 5MB trigger "large file mode"
- Some features may be disabled for performance

## Implementation Status

**✅ Phase 1 Complete - Core Editor:**
- Virtual scrolling (only visible lines rendered)
- Line number gutter with right-aligned numbers
- Galley caching (LRU, 200 entries)
- Cursor display (vertical line with blinking)
- Horizontal scroll support for long lines
- Feature flag integration with EditorWidget
- Tab/AppState content synchronization

**✅ Phase 2 Complete - Full Feature Parity:**
- **Word wrap** with dynamic line heights and visual row navigation
- **Full keyboard input handling** (arrows, Home/End, Page Up/Down, Ctrl+arrows)
- **Text selection** (click-drag, shift+arrow, double/triple-click)
- **Clipboard support** (Ctrl+A/C/X/V)
- **Syntax highlighting** (per-line caching, theme-aware, viewport-optimized)
- **Search highlights** (matches highlighted, current match distinct, capped at 1000)
- **Bracket matching** (windowed search ±100 lines, theme-aware colors)
- **Find & Replace** integration (via EditorWidget configuration)

**✅ Phase 3 Complete - Advanced Features (v0.2.6):**
- **Undo/redo** - Ctrl+Z/Y with operation-based history and grouping
- **Multi-cursor editing** - Ctrl+Click to add cursors, simultaneous typing/deletion
- **Code folding** - Gutter indicators, click-to-toggle, content hiding
- **IME support** - CJK input composition with proper cursor positioning
- **Selection rendering** - Semi-transparent (~40% alpha) with readable text
- **Cursor improvements** - Blinking (500ms), theme-aware color, auto-focus

**Rendering Constants:**
```rust
const DEFAULT_FONT_SIZE: f32 = 14.0;  // Font size in points
const FIXED_LINE_HEIGHT: f32 = 20.0;  // Line spacing (non-wrapped mode)
const GUTTER_CHARS: usize = 5;        // Line number width (99999)
const GUTTER_PADDING: f32 = 8.0;      // Space between gutter and text
const LARGE_FILE_THRESHOLD: usize = 5 * 1024 * 1024;  // 5MB
const MAX_DISPLAYED_MATCHES: usize = 1000;  // Search match display cap
```

**Performance Characteristics:**
- Viewport rendering: Only ~20-30 lines rendered (visible + 5 overscan)
- Large file support: 100k+ lines with O(log n) access
- Cache efficiency: 200 galleys cached, LRU eviction
- Memory: ~80MB for 80MB file (was 460MB+ with egui TextEdit)
- Bracket matching: O(window) complexity, cursor ±100 lines max
- Search highlights: Pre-computed line numbers, capped display

**⏳ Future Enhancements:**
- Column selection mode (Alt+Shift+drag)
- Ctrl+D for "select next occurrence"
- Improved undo/redo that tracks multi-cursor state

## Input Handling

The input handling is split into modular submodules under `src/editor/ferrite/input/`:
- `mod.rs` - `InputHandler` dispatch and `InputResult` enum
- `keyboard.rs` - Keyboard event processing (arrows, backspace, etc.)
- `mouse.rs` - Mouse wheel scrolling

**Raw Enter and list markers (ISS-49 / STAT-3):** In Raw view, **Enter** always inserts a plain newline (respecting the tab’s `LineEnding` for the inserted break). Ferrite does **not** auto-continue `- `, `* `, `1. `, or task-list markers on Enter. List continuation on Enter exists only in the Rendered/WYSIWYG path via `split_list_item` in `src/markdown/ast_ops.rs`. The Stats panel still counts list lines via `DocumentStats::is_list_item`. v0.3.1 keeps Raw Notepad++-style (plain text); the GrokBot STAT-3 row is closed as a checklist expectation, not a product bug.

**Keyboard Operations:**

| Operation | Keys | Description |
|-----------|------|-------------|
| Character insertion | Any printable | Insert at cursor (deletes selection first) |
| Newline | Enter | Insert newline (deletes selection first) |
| Backspace | Backspace | Delete selection, or char before cursor |
| Delete | Delete | Delete selection, or char after cursor |
| Move left | ← | Move cursor left (collapses selection) |
| Move right | → | Move cursor right (collapses selection) |
| Move up | ↑ | Move to previous line (collapses selection) |
| Move down | ↓ | Move to next line (collapses selection) |
| Extend left | Shift+← | Extend selection left |
| Extend right | Shift+→ | Extend selection right |
| Extend up | Shift+↑ | Extend selection up |
| Extend down | Shift+↓ | Extend selection down |
| Word left | Ctrl+← | Move to start of previous word |
| Word right | Ctrl+→ | Move to start of next word |
| Line start | Home | Move to start of line |
| Line end | End | Move to end of line |
| Doc start | Ctrl+Home | Move to start of document |
| Doc end | Ctrl+End | Move to end of document |
| Page up | PageUp | Move cursor up by viewport height |
| Page down | PageDown | Move cursor down by viewport height |
| Select all | Ctrl+A | Select entire document |
| Copy | Ctrl+C | Copy selection to clipboard |
| Cut | Ctrl+X | Cut selection to clipboard |
| Paste | Ctrl+V | Paste from clipboard (replaces selection) |

**Mouse Operations:**

| Operation | Action | Description |
|-----------|--------|-------------|
| Click | Single click | Position cursor at click location |
| Shift+Click | Shift + click | Extend selection to click location |
| Double-click | Double click | Select word at cursor |
| Triple-click | Triple click | Select entire line |
| Click-drag | Click and drag | Select text range |

**Click Position Calculation:**
- Uses `galley.cursor_from_pos()` for wrapped text (accurate to character)
- Uses binary search with midpoint rounding for non-wrapped text
- Focus is requested on click for keyboard input

**InputResult Enum:**
```rust
pub enum InputResult {
    NoChange,      // No modification
    CursorMoved,   // Cursor moved, text unchanged
    TextChanged,   // Text was modified
    ViewScrolled,  // Mouse wheel scrolled view
}
```

## Selection & Clipboard Implementation

### Selection Architecture

Selection uses two cursors: `anchor` (start) and `head` (end/current):

```rust
pub struct Selection {
    pub anchor: Cursor,  // Fixed point where selection started
    pub head: Cursor,    // Moving point (follows user actions)
}

impl Selection {
    pub fn collapsed(cursor: Cursor) -> Self;  // No selection, just cursor
    pub fn is_range(&self) -> bool;            // anchor != head
    pub fn ordered(&self) -> (Cursor, Cursor); // (start, end) in document order
    pub fn with_head(self, new_head: Cursor) -> Self;  // Keep anchor, change head
}
```

### Clipboard Operations

Clipboard uses egui's event and output systems:

```rust
// Handle Copy/Cut events (preferred - generated by OS/egui)
match event {
    egui::Event::Copy => {
        if self.selection.is_range() {
            let text = self.selected_text();
            ui.output_mut(|o| o.copied_text = text);
        }
    }
    egui::Event::Cut => {
        if self.selection.is_range() {
            let text = self.selected_text();
            ui.output_mut(|o| o.copied_text = text);
            self.delete_selection();
        }
    }
    egui::Event::Paste(text) => {
        self.delete_selection();
        insert_text(&mut self.buffer, &mut cursor, text);
    }
    _ => {}
}
```

**Important**: egui generates `Event::Copy` and `Event::Cut` events for clipboard operations, NOT `Key::C`/`Key::X` events. Handle both for cross-platform compatibility.

### Important Implementation Details

**1. Drag Anchor Capture (Critical Fix)**

egui's `drag_started()` fires **after** the mouse has moved (to distinguish from clicks). This means `interact_pointer_pos()` at that moment is NOT the original click position.

**Solution:** Capture position on initial press using `is_pointer_button_down_on()`:

```rust
// Capture position when button first goes down (before egui decides if it's a drag)
if response.is_pointer_button_down_on() && self.drag_start_cursor.is_none() {
    if let Some(pos) = response.interact_pointer_pos() {
        self.drag_start_cursor = Some(self.pos_to_cursor(pos, ...));
    }
}

// When drag starts, use the stored position as anchor
if response.drag_started() {
    if let Some(anchor) = self.drag_start_cursor {
        self.selection = Selection::collapsed(anchor);
    }
}

// Clear when button released
if !response.is_pointer_button_down_on() {
    self.drag_start_cursor = None;
}
```

**2. Content Dirty vs Wrap Info (Flickering Fix)**

When `content_dirty` is set, do NOT clear `wrap_info`. Clearing it causes y-position calculations to alternate between methods on consecutive frames, creating flickering.

```rust
// CORRECT: Only invalidate line cache
if self.content_dirty {
    self.line_cache.invalidate();
    // DON'T clear wrap_info here - causes flickering!
    self.content_dirty = false;
}
```

**3. Selection Rendering Order**

Selection background must be rendered BEFORE text galleys:

```rust
// In ui() method:
self.render_selection(painter, ...);  // Background first
// ... then render text galleys
render_cursor(painter, ...);          // Cursor last
```

**Usage in FerriteEditor:**
```rust
// In ui() method, when focused:
for event in &events {
    let result = InputHandler::handle_event(
        event,
        &mut self.buffer,
        &mut self.cursor,
        &mut self.view,
    );
    
    match result {
        InputResult::TextChanged => {
            self.content_dirty = true;
            self.view.ensure_line_visible(self.cursor.line, total_lines);
        }
        InputResult::CursorMoved => {
            self.view.ensure_line_visible(self.cursor.line, total_lines);
        }
        InputResult::NoChange => {}
    }
}
```

## Syntax Highlighting

FerriteEditor supports per-line syntax highlighting with efficient caching.

### Configuration

```rust
// Configure syntax highlighting
editor.configure_syntax(
    enabled: true,
    language: Some("rust".to_string()),
    dark_mode: true,
);

// Or individual setters
editor.set_syntax_enabled(true);
editor.set_syntax_language(Some("rust".to_string()));
editor.set_syntax_dark_mode(true);
```

### Implementation

1. **Per-line highlighting**: Only visible lines are syntax-highlighted, integrating with virtual scrolling
2. **Cache key includes theme**: `CacheKey::new_highlighted()` hashes content + font + color + `syntax_theme_hash`
3. **Uses existing syntax module**: `crate::markdown::syntax::highlight_code()` provides the highlighting
4. **HighlightedSegment**: Simplified struct for cache (`text`, `color`)

### Integration with EditorWidget

`EditorWidget` automatically configures syntax highlighting based on:
- `syntax_highlighting` setting (enabled/disabled)
- `file_path` (language detection via `language_from_path()`)
- `is_dark_mode` (theme selection)

```rust
// In widget.rs show_with_ferrite_editor():
let syntax_language = if self.syntax_highlighting {
    self.file_path.as_ref().and_then(|p| language_from_path(p))
} else {
    None
};
editor.configure_syntax(
    self.syntax_highlighting && syntax_language.is_some(),
    syntax_language,
    self.is_dark_mode,
);
```

## Search Highlights

FerriteEditor supports search match highlighting with a distinct current match indicator.

### Configuration

```rust
// Set search matches from find panel
editor.set_search_matches(
    matches: Vec<(usize, usize)>,  // (start_byte, end_byte) pairs
    current_match: usize,          // Index of current match
    scroll_to_match: bool,         // Whether to scroll to current match
);

// Clear all search matches
editor.clear_search_matches();
```

### Implementation Details

1. **Pre-computed line numbers**: Matches store pre-computed line numbers (`SearchMatch.line`) for O(1) scroll-to-match
2. **Display cap**: Only first 1000 matches are rendered (matches VS Code behavior)
3. **Distinct current match**: Current match has brighter highlight color
4. **Viewport-aware**: Only visible matches are rendered
5. **Theme-aware**: Colors adapt to dark/light mode

### Integration with EditorWidget

```rust
// In widget.rs show_with_ferrite_editor():
if let Some(ref highlights) = self.search_highlights {
    editor.set_search_matches(
        highlights.matches.clone(),
        highlights.current_match,
        highlights.scroll_to_match,
    );
} else {
    editor.clear_search_matches();
}
```

## Bracket Matching

FerriteEditor highlights matching brackets when the cursor is adjacent to one.

### Configuration

```rust
// Enable/disable bracket matching
editor.set_bracket_matching_enabled(true);

// Set custom colors (optional - uses theme defaults if None)
editor.set_bracket_colors(Some((
    bg_color: Color32,     // Background fill
    border_color: Color32, // Border stroke
)));
```

### Implementation Details

1. **Windowed search**: Only searches cursor ±100 lines (O(window), not O(N))
2. **Uses existing matcher**: Leverages `crate::editor::matching::DelimiterMatcher`
3. **Supported delimiters**: `()`, `[]`, `{}`, `<>`, markdown emphasis `**`, `__`
4. **Theme-aware**: Colors from `theme_colors.ui.matching_bracket_bg/border`

### Rendering

Both the cursor-adjacent bracket and its matching pair are highlighted:
- Background fill for visibility
- Border stroke for clarity

## Dependencies

- `egui` - GUI framework
- `ropey` - Rope data structure (via TextBuffer)
- `syntect` / `two-face` - Syntax highlighting (via `crate::markdown::syntax`)

## Related Documentation

- [TextBuffer](./text-buffer.md) - Rope-based text storage
- [EditHistory](./edit-history.md) - Undo/redo system
- [ViewState](./view-state.md) - Viewport tracking
- [LineCache](./line-cache.md) - Galley caching
- [Custom Editor Widget Plan](../planning/custom-editor-widget-plan.md) - Full roadmap
