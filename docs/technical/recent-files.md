# Recent Files Feature

## Overview

The Recent Files feature provides quick access to recently opened files directly from the status bar. Clicking the file path in the status bar opens a popup showing the 5 most recently accessed files.

## User Interface

### Status Bar File Path

- **Location**: Left side of the bottom status bar
- **Display**: Shows current file path (or "Untitled" / "No file open")
- **Clickable**: When recent files exist, clicking opens the popup

### Recent Files Popup

- **Triggered by**: Click on file path in status bar
- **Shows**: Up to 5 most recent files
- **Display format**: File name (bold) with parent directory below
- **Hover info**: Full path with usage instructions

## Interaction Modes

### Click (Open with Focus)

- Opens the file in a new tab
- Switches focus to the new tab immediately
- Default behavior for normal clicks

### Shift+Click (Open in Background)

- Opens the file in a new tab
- Does **not** switch focus (stays on current tab)
- Shows toast message: "Opened in background: filename"
- **Popup stays open** - allows opening multiple files in sequence
- Useful for batch-opening multiple files quickly

## Implementation

### Files Modified

- `src/state.rs` - Added `open_file_with_focus(path, focus: bool)` method
- `src/state.rs` - Added `show_recent_files_popup` to `UiState`
- `src/app.rs` - Status bar file path button and popup UI

### Key Components

#### `AppState::open_file_with_focus(path, focus)`

```rust
pub fn open_file_with_focus(&mut self, path: PathBuf, focus: bool) -> Result<usize, std::io::Error>
```

Opens a file with control over whether to switch focus:
- `focus: true` - Opens and switches to the new tab
- `focus: false` - Opens in background, current tab stays active
- Handles already-open files (returns existing tab index)
- Updates recent files list regardless of focus mode

#### `UiState::show_recent_files_popup`

Boolean flag controlling popup visibility. Toggled by:
- Click on status bar file path (toggles)
- Normal click on a file in the popup (closes)
- Shift+click on a file (stays open for batch opening)
- Clicking outside the popup (closes)

### Popup UI Structure

```
┌─────────────────────────────────┐
│ Recent Files (bold header)      │
├─────────────────────────────────┤
│ document.md (bold)              │
│ C:\Users\name\Documents         │
│                                 │
│ notes.md (bold)                 │
│ G:\Projects\MyProject           │
│ ...                             │
└─────────────────────────────────┘
```

## Data Storage

Recent files are stored in `Settings.recent_files`:
- Type: `Vec<PathBuf>`
- Maximum: 10 files (configurable via `max_recent_files`)
- Automatically updated on file open
- Persisted in config file

## Testing

Unit tests in `src/state.rs`:
- `test_open_file_with_focus_true` - Verifies focus behavior
- `test_open_file_with_focus_false` - Verifies background open
- `test_open_file_already_open_with_focus` - Handles duplicate opens
- `test_open_file_already_open_without_focus` - Background duplicate
- `test_open_file_updates_recent_files` - Recent files list updated

## Related Features

- **Settings persistence**: Recent files saved/loaded from config
- **Tab management**: Opens in new tabs, reuses existing tabs for same file
- **Toast messages**: Feedback for background opens
