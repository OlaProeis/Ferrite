# Application State Management

## Overview

The `state.rs` module provides comprehensive application state management for Ferrite. It defines the central `AppState` struct that manages all runtime data including open tabs, user settings, and UI state.

## Key Files

- `src/state.rs` - Main state module with all state types and logic

## Core Types

### `FileType` - File Type Detection

Enum for adaptive UI based on file extension:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Markdown,   // .md, .markdown
    Json,       // .json
    Yaml,       // .yaml, .yml
    Toml,       // .toml
    Unknown,    // Other extensions
}

impl FileType {
    pub fn from_path(path: &Path) -> Self;
    pub fn from_extension(ext: &str) -> Self;
    pub fn is_markdown(&self) -> bool;
    pub fn is_structured(&self) -> bool;  // JSON, YAML, or TOML
    pub fn display_name(&self) -> &'static str;
}
```

### `Tab` - Document State

Runtime state for an open document tab with full editing support.

```rust
pub struct Tab {
    pub id: usize,                    // Unique tab identifier
    pub path: Option<PathBuf>,        // File path (None = unsaved)
    pub content: String,              // Current content
    pub cursor_position: (usize, usize), // (line, column)
    pub scroll_offset: f32,           // Scroll position
    pub view_mode: ViewMode,          // Raw or Rendered mode (per-tab)
    pub file_type: FileType,          // Detected file type for adaptive UI
    // Private: undo/redo stacks, original content
}
```

**Key Methods:**
- `new(id)` / `with_file(id, path, content)` - Creation
- `from_tab_info(id, info, content)` - Restore from session
- `is_modified()` - Check for unsaved changes
- `title()` - Display name (with `*` for modified)
- `set_content(text)` - Update content (pushes to undo stack)
- `undo()` / `redo()` - History navigation
- `mark_saved()` - Reset modification tracking
- `to_tab_info()` - Convert to `TabInfo` for persistence
- `toggle_view_mode()` - Switch between Raw/Rendered
- `get_view_mode()` / `set_view_mode()` - View mode access

### `UiState` - UI Flags

Tracks visibility and state of UI elements.

```rust
pub struct UiState {
    pub show_settings: bool,
    pub show_file_dialog: bool,
    pub show_save_as_dialog: bool,
    pub show_about: bool,
    pub show_confirm_dialog: bool,
    pub confirm_dialog_message: String,
    pub pending_action: Option<PendingAction>,
    pub status_message: Option<String>,      // Deprecated, use toast
    pub show_find_replace: bool,
    pub search_query: String,
    pub replace_text: String,
    pub show_error_modal: bool,
    pub error_message: String,
    pub toast_message: Option<String>,       // Temporary notification
    pub toast_expires_at: Option<f64>,       // When toast should disappear
    pub show_recent_files_popup: bool,       // Recent files menu
    // Workspace UI state
    pub show_quick_switcher: bool,           // Quick file palette (Ctrl+P)
    pub show_search_panel: bool,             // Search in files (Ctrl+Shift+F)
}
```

### `PendingAction` - Confirmation Actions

Actions that may require user confirmation before execution.

```rust
pub enum PendingAction {
    CloseTab(usize),
    CloseAllTabs,
    Exit,
    OpenFile(PathBuf),
    NewDocument,
}
```

### `AppState` - Central State

The main application state container.

```rust
pub struct AppState {
    tabs: Vec<Tab>,
    active_tab_index: usize,
    pub settings: Settings,
    pub ui: UiState,
    pub mode: AppMode,           // Single file or workspace mode
    pub workspace: Option<Workspace>,  // Active workspace (if any)
    // Private: next_tab_id, settings_dirty, workspace_watcher
}
```

### `AppMode` - Application Mode

The application operates in one of two modes:

```rust
pub enum AppMode {
    SingleFile,              // Traditional single-file editing
    Workspace {              // Folder-based project mode
        root: PathBuf,
        settings_path: PathBuf,  // .ferrite/ directory
    },
}
```

### `Workspace` - Workspace State

When a folder is opened, workspace state is managed:

```rust
pub struct Workspace {
    pub root_path: PathBuf,
    pub file_tree: FileTreeNode,
    pub hidden_patterns: Vec<String>,
    pub recent_files: Vec<PathBuf>,
    pub settings: WorkspaceSettings,
    pub show_file_tree: bool,
    pub file_tree_width: f32,
}
```

See [Workspace Folder Support](./workspace-folder-support.md) for detailed workspace documentation.

## Implementation Details

### Initialization

`AppState::new()` performs:
1. Loads settings via `load_config()` (graceful fallback to defaults)
2. Creates initial empty tab
3. Initializes default UI state

```rust
let state = AppState::new();
// Or with custom settings for testing:
let state = AppState::with_settings(Settings::default());
```

### Tab Management

```rust
// Create tabs
let index = state.new_tab();
let index = state.open_file(path)?;

// Access tabs
let tab = state.active_tab();
let tab = state.active_tab_mut();
state.set_active_tab(1);

// Close tabs (respects unsaved changes)
state.close_tab(index);       // Shows confirmation if modified
state.force_close_tab(index); // Ignores modifications
```

### Undo/Redo System

Each tab maintains independent undo/redo stacks (100 levels):

```rust
if let Some(tab) = state.active_tab_mut() {
    tab.set_content("new text".to_string()); // Pushes to undo
    tab.undo();  // Restores previous
    tab.redo();  // Re-applies change
}
```

### Event Handling

Confirmation dialog workflow:

```rust
// Request exit (checks for unsaved changes)
if !state.request_exit() {
    // Confirmation dialog shown via state.ui.show_confirm_dialog
}

// After user confirms
state.handle_confirmed_action();

// Or cancels
state.cancel_pending_action();
```

### Settings Integration

```rust
// Update settings reactively
state.update_settings(|s| {
    s.theme = Theme::Dark;
    s.font_size = 16.0;
});

// Save to disk when dirty
state.save_settings_if_dirty();

// Shutdown (auto-saves)
state.shutdown();
```

## Dependencies Used

- `crate::config::Settings` - User preferences
- `crate::config::TabInfo` - Tab persistence data
- `crate::config::load_config` - Config loading
- `crate::config::save_config_silent` - Silent config saving
- `crate::error::Error` - Error types for file operations
- `log` - Logging (debug, info, warn)

## Usage

```rust
use crate::state::AppState;

fn main() {
    let mut state = AppState::new();
    
    // Open a file
    if let Ok(index) = state.open_file(PathBuf::from("doc.md")) {
        println!("Opened in tab {}", index);
    }
    
    // Edit content
    if let Some(tab) = state.active_tab_mut() {
        tab.set_content("# Hello World".to_string());
    }
    
    // Save
    state.save_active_tab().expect("Save failed");
    
    // Cleanup
    state.shutdown();
}
```

## Tests

Run state module tests:

```bash
cargo test state::tests
```

**Test Coverage:**
- Tab creation and modification tracking
- Title generation with modification indicator
- Undo/redo functionality
- AppState initialization
- Tab management (create, switch, close)
- Unsaved changes detection
- Settings updates
- UI state toggles
- Event handling (confirm/cancel)
- Pending action system

Total: 24 tests in `state::tests` module
