# Workspace/Folder Support

## Overview

Comprehensive folder/workspace management system that enables project-based editing with file tree navigation, context menus, file operations, quick file switching, file watching, and search-in-files functionality.

## Key Files

- `src/workspaces/mod.rs` - Core workspace types (`AppMode`, `Workspace`) and module re-exports
- `src/workspaces/file_tree.rs` - File tree data structure and directory scanning
- `src/workspaces/settings.rs` - Workspace-specific settings and persistence
- `src/workspaces/persistence.rs` - Workspace state persistence (expanded folders, recent files)
- `src/workspaces/watcher.rs` - File system watcher for detecting external changes
- `src/ui/file_tree.rs` - File tree sidebar panel UI
- `src/ui/quick_switcher.rs` - Quick file switcher overlay (Ctrl+P)
- `src/ui/search.rs` - Search in files panel (Ctrl+Shift+F)
- `src/ui/dialogs.rs` - File operation dialogs (New File, New Folder, Rename, Delete)
- `src/state.rs` - AppState integration for workspace mode
- `src/app.rs` - Main application integration

## Implementation Details

### Application Mode

The app operates in one of two modes:

```rust
pub enum AppMode {
    SingleFile,          // Traditional single-file editing
    Workspace {          // Folder-based project mode
        root: PathBuf,
        settings_path: PathBuf,
    },
}
```

### Workspace Struct

When a folder is opened, a `Workspace` instance is created:

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

### File Tree

Recursive data structure representing the directory hierarchy:

```rust
pub struct FileTreeNode {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileTreeNodeKind,
    pub expanded: bool,
}

pub enum FileTreeNodeKind {
    File,
    Directory { children: Vec<FileTreeNode> },
}
```

### File Watcher

Uses the `notify` crate to monitor filesystem changes:

```rust
pub enum WorkspaceEvent {
    FileCreated(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
    FileRenamed(PathBuf, PathBuf),
    Error(String),
}
```

Events are polled each frame and used to:
- Refresh the file tree when files are created/deleted
- Show toast notifications when open files are modified externally

### Quick File Switcher

Fuzzy search across all workspace files:
- Opens with **Ctrl+P**
- Uses `fuzzy-matcher` crate for scoring
- Prioritizes recently opened files
- Keyboard navigation with arrow keys

### Search in Files

Full-text search across workspace:
- Opens with **Ctrl+Shift+F**
- Supports plain text and regex
- Case-sensitive toggle
- Results grouped by file with highlighted matches
- Click result to open file

### File Operation Dialogs

Modal dialogs for file operations:
- **New File**: Creates file with default markdown content
- **New Folder**: Creates empty directory
- **Rename**: Renames file/folder, updates open tabs
- **Delete**: Confirmation dialog, closes affected tabs

## Dependencies Used

- `notify = "6"` - Cross-platform file system watching
- `fuzzy-matcher = "0.3"` - Fuzzy string matching for quick switcher
- `regex` - Regular expression support for search

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+B | Toggle file tree panel |
| Ctrl+P | Open quick file switcher |
| Ctrl+Shift+F | Open search in files |

## Ribbon Toolbar Buttons

When in workspace mode, additional buttons appear in the ribbon toolbar next to the Open Folder button:

- **🔎 Search in Files** - Opens the search panel (same as Ctrl+Shift+F)
- **⚡ Quick File Switcher** - Opens the file palette (same as Ctrl+P)

These buttons are only visible when a workspace/folder is open. The emoji icons are temporary placeholders for future SVG/PNG replacement.

## UI Components

### File Tree Panel

- Left sidebar showing folder structure
- Expand/collapse folders
- File icons based on extension
- Context menu (right-click):
  - New File
  - New Folder
  - Rename
  - Delete
  - Reveal in Explorer
  - Refresh

### Quick Switcher

- Modal overlay in center of screen
- Search input with fuzzy matching
- File list with icons and paths
- Keyboard navigation

### Search Panel

- Modal window with search input
- Regex and case-sensitive options
- Results list with match highlighting
- Click to navigate to file

## Persistence

Workspace state is saved to `.ferrite/` directory within the workspace root:

- `settings.json` - Workspace-specific settings
- `state.json` - UI state (expanded folders, recent files, panel width)

## Usage

### Opening a Workspace

1. Click "Open Folder" button in ribbon (or use Ctrl+Shift+O future shortcut)
2. Or drag a folder onto the application window

### File Operations

1. Right-click a file/folder in the file tree
2. Select operation from context menu
3. Complete dialog (if applicable)

### Searching Files

1. Press Ctrl+Shift+F
2. Enter search term
3. Press Enter to search
4. Click result to open file

## Tests

Run workspace-related tests:

```bash
cargo test workspaces::
cargo test ui::file_tree::
cargo test ui::quick_switcher::
cargo test ui::search::
cargo test ui::dialogs::
```
