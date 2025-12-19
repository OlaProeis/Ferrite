# Find and Replace System

## Overview

The Find and Replace feature provides comprehensive search and replace functionality for Ferrite. It includes real-time incremental search, match highlighting, keyboard navigation, and integration with the undo/redo system.

## Architecture

### Components

```
src/editor/find_replace.rs
├── FindState          - Search state management
├── FindReplacePanel   - Floating UI panel
└── FindReplacePanelOutput - Panel action output
```

### Data Flow

```
User Input → FindReplacePanel → FindReplacePanelOutput → app.rs handlers
                    ↓
               FindState.find_matches(content)
                    ↓
               Vec<(start, end)> matches
                    ↓
               Navigation / Replace operations
```

## FindState

`FindState` manages all search state:

```rust
pub struct FindState {
    pub search_term: String,        // Current search term
    pub replace_term: String,       // Replacement text
    pub case_sensitive: bool,       // Case-sensitive matching
    pub whole_word: bool,           // Match whole words only
    pub use_regex: bool,            // Use regex matching
    pub current_match: usize,       // Current match index
    pub matches: Vec<(usize, usize)>, // Match positions (start, end)
    pub is_replace_mode: bool,      // Find-only vs Replace mode
}
```

### Key Methods

| Method | Description |
|--------|-------------|
| `find_matches(&mut self, text: &str) -> usize` | Find all matches, returns count |
| `next_match() -> Option<usize>` | Move to next match (wraps) |
| `prev_match() -> Option<usize>` | Move to previous match (wraps) |
| `current_match_position() -> Option<(usize, usize)>` | Get current match bounds |
| `replace_current(text: &str) -> Option<String>` | Replace current match |
| `replace_all(text: &str) -> String` | Replace all matches |

## Search Modes

### Literal Search (Default)
- Standard substring matching
- Case-insensitive by default
- Supports whole-word matching with word boundaries

### Regex Search
- Full regex pattern support via the `regex` crate
- Case-insensitive flag (`(?i)`) applied automatically when needed
- Word boundaries (`\b`) applied for whole-word regex matching
- Invalid patterns gracefully return 0 matches

### Search Options

| Option | Icon | Description |
|--------|------|-------------|
| Case Sensitive | `Aa` | Match exact case |
| Whole Word | `W` | Match complete words only |
| Use Regex | `.*` | Enable regex pattern matching |

## FindReplacePanel

A floating UI panel that provides the search interface:

### UI Elements
- **Search input**: Real-time search with incremental results
- **Replace input**: Visible only in replace mode
- **Match counter**: Shows "{current} of {total}"
- **Toggle buttons**: Case, Whole Word, Regex
- **Navigation buttons**: Previous (◀) / Next (▶)
- **Replace buttons**: Replace / Replace All

### Panel Modes
1. **Find Mode** (Ctrl+F): Search-only functionality
2. **Replace Mode** (Ctrl+H): Search with replace options

## Keyboard Shortcuts

| Shortcut | Action | Context |
|----------|--------|---------|
| `Ctrl+F` | Open find panel | Global |
| `Ctrl+H` | Open find/replace panel | Global |
| `F3` | Find next match | When panel open |
| `Shift+F3` | Find previous match | When panel open |
| `Enter` | Find next match | In search input |
| `Escape` | Close panel | When panel open |

## Integration Points

### app.rs
Handles keyboard shortcuts and panel output:
```rust
// Opening the panel
fn handle_open_find(&mut self, replace_mode: bool)

// Navigation
fn handle_find_next(&mut self)
fn handle_find_prev(&mut self)

// Replace operations (with undo support)
fn handle_replace_current(&mut self)
fn handle_replace_all(&mut self)
```

### state.rs
`FindState` is stored in `UiState`:
```rust
pub struct UiState {
    pub show_find_replace: bool,
    pub find_state: FindState,
    // ...
}
```

### Ribbon Integration
The Tools section 🔍 button opens the find panel:
```rust
RibbonAction::FindReplace => {
    self.handle_open_find(false);
}
```

## Undo/Redo Integration

Replace operations integrate with the existing undo system:

```rust
fn handle_replace_current(&mut self) {
    if let Some(new_content) = find_state.replace_current(&content) {
        // tab.set_content() automatically:
        // - Pushes current state to undo stack
        // - Clears redo stack
        tab.set_content(new_content);
    }
}
```

## Implementation Details

### Whole Word Matching
Uses word boundary detection:
```rust
let is_start_boundary = match_start == 0
    || !text[..match_start]
        .chars()
        .last()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
```

### Case-Insensitive Search
- Literal: Both text and search term lowercased
- Regex: `(?i)` flag prepended to pattern

### Current Match Clamping
After re-searching (e.g., options changed), `current_match` is clamped:
```rust
if !self.matches.is_empty() && self.current_match >= self.matches.len() {
    self.current_match = 0;
}
```

## Testing

33 tests cover:
- Basic matching (literal, case, whole word, regex)
- Navigation (next/prev, wrapping)
- Replace operations (current, all)
- Edge cases (empty, unicode, multiline, invalid regex)
- Panel state management

Run tests:
```bash
cargo test find_replace
```

## Match Highlighting

When the find panel is open and matches are found, the editor displays:

### Visual Indicators
- **Current match**: Bright yellow highlight (more opaque)
- **Other matches**: Pale yellow highlight (semi-transparent)

### Auto-Scroll
When navigating between matches (F3/Shift+F3), the editor automatically scrolls to show the current match, positioning it roughly 1/3 from the top of the viewport.

### Implementation
The `SearchHighlights` struct in `widget.rs` carries:
```rust
pub struct SearchHighlights {
    pub matches: Vec<(usize, usize)>,  // All match positions
    pub current_match: usize,           // Index of current match
    pub scroll_to_match: bool,          // Whether to auto-scroll
}
```

Highlights are drawn using the galley's cursor positioning:
1. Convert byte positions to `CCursor`
2. Get `RCursor` with row and column information
3. Calculate screen rectangles using `row.x_offset()`
4. Draw filled rectangles behind text

## Future Enhancements

Potential improvements for future versions:
1. **Selection-based search** - search for selected text
2. **Search history** - recent search terms dropdown
3. **Find in files** - search across multiple open tabs
4. **Preserve case** replacement option
5. **Count-only mode** for large documents
6. **Match highlighting in Rendered mode** - currently only Raw mode is supported
