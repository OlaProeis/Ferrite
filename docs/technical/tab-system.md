# Tab System

## Overview

The tab system provides multi-document editing support with unsaved changes tracking, visual indicators, and keyboard navigation. Tabs are managed directly within `AppState` rather than a separate TabManager, providing tight integration with application state.

## Key Files

| File | Purpose |
|------|---------|
| `src/state.rs` | `Tab` struct, tab management in `AppState`, `PendingAction` enum |
| `src/app.rs` | Tab bar UI rendering, close buttons, keyboard shortcuts |
| `src/config/settings.rs` | `TabInfo` struct for session persistence |

## Tab Data Structure

### Tab Struct (`src/state.rs`)

```rust
pub struct Tab {
    pub id: usize,                      // Unique identifier
    pub path: Option<PathBuf>,          // File path (None for untitled)
    pub content: String,                // Document content
    original_content: String,           // For dirty state detection
    pub cursor_position: (usize, usize), // Line, column (0-indexed)
    pub scroll_offset: f32,             // Scroll position
    undo_stack: Vec<String>,            // Undo history
    redo_stack: Vec<String>,            // Redo history
    max_undo_size: usize,               // Max undo entries (100)
}
```

### Key Methods

| Method | Description |
|--------|-------------|
| `Tab::new(id)` | Create empty tab |
| `Tab::with_file(id, path, content)` | Create tab from file |
| `Tab::is_modified()` | Check if content differs from original |
| `Tab::mark_saved()` | Update original_content after save |
| `Tab::title()` | Get display title with `*` if modified |
| `Tab::set_content(new)` | Set content with undo tracking |
| `Tab::undo() / redo()` | Undo/redo operations |
| `Tab::to_tab_info()` | Convert to TabInfo for persistence |

## Tab Management in AppState

### State Fields

```rust
pub struct AppState {
    tabs: Vec<Tab>,           // All open tabs
    active_tab_index: usize,  // Currently active tab
    next_tab_id: usize,       // Counter for unique IDs
    // ...
}
```

### Management Methods

| Method | Description |
|--------|-------------|
| `new_tab()` | Create empty tab, make active, return index |
| `open_file(path)` | Open file in new tab (or switch if already open) |
| `set_active_tab(index)` | Switch to tab by index |
| `close_tab(index)` | Close tab (prompts if unsaved changes) |
| `force_close_tab(index)` | Close tab ignoring unsaved changes |
| `tab_count()` | Get number of open tabs |
| `active_tab() / active_tab_mut()` | Get reference to active tab |
| `tab(index)` | Get tab by index |
| `has_unsaved_changes()` | Check if any tab is modified |

## Tab Bar UI (`src/app.rs`)

### Visual Features

- **Multi-line wrapping**: Tabs automatically flow to new rows when window is narrow
- **Dynamic height**: Tab bar expands/contracts based on number of rows needed
- **Active tab highlight**: Background color from `selection.bg_fill`
- **Hover highlight**: Subtle background on hover for inactive tabs
- **Unsaved indicator**: Asterisk (`*`) appended to title
- **Close button**: `×` on each tab with red hover effect
- **New tab button**: `+` at end of tab bar, wraps with tabs
- **Pointer cursor**: Shows clickable cursor on hover

### Multi-line Tab Bar Implementation

The tab bar uses a custom layout system that calculates tab positions and handles wrapping:

```rust
// 1. Pre-calculate tab widths and row positions
let mut tab_positions: Vec<(f32, usize)> = Vec::new(); // (x_pos, row)
let mut current_x = 0.0;
let mut current_row = 0;

for (_, title, _) in &tab_titles {
    let tab_width = calculate_tab_width(title);
    
    // Wrap to next row if needed
    if current_x + tab_width > available_width && current_x > 0.0 {
        current_x = 0.0;
        current_row += 1;
    }
    
    tab_positions.push((current_x, current_row));
    current_x += tab_width + spacing;
}

// 2. Allocate space for all rows
let total_height = (current_row + 1) * row_height;
let (tab_bar_rect, _) = ui.allocate_exact_size(vec2(width, total_height), Sense::hover());

// 3. Render each tab at its calculated position
for (idx, (tab_info, (x_pos, row))) in tabs.iter().zip(positions.iter()).enumerate() {
    let tab_rect = Rect::from_min_size(
        tab_bar_rect.min + vec2(*x_pos, *row as f32 * row_height),
        vec2(tab_width, row_height),
    );
    
    // Paint background (selected or hover)
    // Paint title text
    // Paint close button
    // Handle click interactions
}
```

### Key Implementation Details

- **Width calculation**: Tab width = title text width + close button + padding
- **Minimum width**: 60px to prevent overly narrow tabs
- **Row height**: 24px per row with 2px vertical spacing
- **Wrapping logic**: Tabs wrap when `current_x + tab_width > available_width`
- **Dynamic response**: Recalculates positions every frame for window resize support

## Unsaved Changes Dialog

### Three-Option Dialog

When closing a tab with unsaved changes, users see:

| Button | Action |
|--------|--------|
| **Save** | Save file (triggers Save As if no path), then close |
| **Discard** | Close without saving |
| **Cancel** | Abort close operation |

### PendingAction Enum

```rust
pub enum PendingAction {
    CloseTab(usize),    // Close specific tab
    CloseAllTabs,       // Close all tabs
    Exit,               // Exit application
    OpenFile(PathBuf),  // Open file
    NewDocument,        // Create new document
}
```

### Flow

1. `close_tab()` checks `is_modified()`
2. If modified, sets `show_confirm_dialog = true`
3. Stores `PendingAction::CloseTab(index)`
4. Dialog renders with Save/Discard/Cancel
5. User choice triggers appropriate handler

## Keyboard Shortcuts

| Shortcut | Action | Method |
|----------|--------|--------|
| Ctrl+T | New tab | `new_tab()` |
| Ctrl+W | Close tab | `close_tab(active_index)` |
| Ctrl+Tab | Next tab | `set_active_tab((current + 1) % count)` |
| Ctrl+Shift+Tab | Previous tab | `set_active_tab(current - 1)` with wrap |

## Session Persistence

### TabInfo Struct (`src/config/settings.rs`)

```rust
pub struct TabInfo {
    pub path: Option<PathBuf>,
    pub modified: bool,
    pub cursor_position: (usize, usize),
    pub scroll_offset: f32,
}
```

### Conversion Methods

- `Tab::to_tab_info()` - Convert runtime Tab to persistable TabInfo
- `Tab::from_tab_info()` - Restore Tab from saved TabInfo

## Tests

### Tab Tests (`src/state.rs`)

```bash
cargo test tab
```

| Test | Coverage |
|------|----------|
| `test_tab_new` | Empty tab creation |
| `test_tab_with_file` | File tab creation |
| `test_tab_modification_tracking` | Dirty state detection |
| `test_tab_title` | Title with/without asterisk |
| `test_tab_undo_redo` | Undo/redo operations |
| `test_tab_to_tab_info` | Persistence conversion |

### AppState Tab Tests

| Test | Coverage |
|------|----------|
| `test_appstate_new_tab` | Tab creation |
| `test_appstate_set_active_tab` | Tab switching |
| `test_appstate_force_close_tab` | Tab closing |
| `test_appstate_close_last_tab_creates_new` | Auto-create on empty |
| `test_appstate_has_unsaved_changes` | Dirty state checking |
| `test_appstate_handle_confirmed_close_tab` | Confirmation handling |

## Dependencies

- **egui** - Tab bar UI rendering
- **serde** - TabInfo serialization (via Settings)

## Related Documentation

- [App State](./app-state.md) - AppState overview
- [Keyboard Shortcuts](./keyboard-shortcuts.md) - Full shortcut reference
- [File Dialogs](./file-dialogs.md) - Save operations
