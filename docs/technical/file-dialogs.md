# File Dialogs and Operations

## Overview

The file operations module provides native file dialog integration and file I/O utilities for opening, reading, and writing files. It uses the `rfd` crate for cross-platform native file dialogs.

The application integrates these dialogs into the File menu and supports keyboard shortcuts for common operations.

## Key Files

- `src/files/mod.rs` - Module declaration and re-exports
- `src/files/dialogs.rs` - Native file dialog functions using rfd
- `src/files/operations.rs` - File I/O utilities with error handling

## Implementation Details

### File Dialog Functions

The `dialogs.rs` module provides three main dialog functions:

```rust
// Open single file (legacy, less commonly used)
pub fn open_file_dialog(initial_dir: Option<&PathBuf>) -> Option<PathBuf>

// Open multiple files (primary open method - supports Ctrl/Shift selection)
pub fn open_multiple_files_dialog(initial_dir: Option<&PathBuf>) -> Vec<PathBuf>

// Save file dialog
pub fn save_file_dialog(initial_dir: Option<&PathBuf>, default_name: Option<&str>) -> Option<PathBuf>
```

**Note**: The app primarily uses `open_multiple_files_dialog` for File > Open, allowing users to select multiple files at once.

All dialogs include filters for:
- Markdown Files (`.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`)
- Text Files (`.txt`, `.text`)
- All Files (`*`)

### FileDialogOptions Builder

For more control, use the `FileDialogOptions` builder:

```rust
let path = FileDialogOptions::new()
    .title("Open Markdown File")
    .initial_dir(PathBuf::from("/home/user/docs"))
    .open_file();
```

### File Operations

The `operations.rs` module provides utilities:

| Function | Purpose |
|----------|---------|
| `read_file_contents()` | Read file with error handling |
| `write_file_contents()` | Write file, creating parent dirs |
| `validate_file_path()` | Check path exists and is readable |
| `get_file_size()` | Get file size in bytes |
| `is_likely_text_file()` | Heuristic check for text vs binary |
| `ensure_parent_dir()` | Create parent directories if needed |

### Error Handling

File operations return `Result<T>` with these error types:
- `Error::FileNotFound` - File doesn't exist
- `Error::PermissionDenied` - Access denied
- `Error::FileRead` - Read failure with source error
- `Error::FileWrite` - Write failure with source error

## Dependencies Used

- `rfd = "0.14"` - Rusty File Dialogs for native OS dialogs
- `log` - Logging for debug/info/warn messages

## Menu Integration

The File menu in `app.rs` provides these operations:

| Menu Item | Shortcut | Handler | Description |
|-----------|----------|---------|-------------|
| New | Ctrl+N | `state.new_tab()` | Creates a new empty tab |
| Open... | Ctrl+O | `handle_open_file()` | Opens native file dialog |
| Save | Ctrl+S | `handle_save_file()` | Saves to existing path (or triggers Save As) |
| Save As... | Ctrl+Shift+S | `handle_save_as_file()` | Opens native save dialog |
| Exit | - | `request_exit()` | Exits with unsaved changes check |

### Keyboard Shortcuts

Shortcuts are handled in `handle_keyboard_shortcuts()` using egui's input system:

```rust
enum KeyboardAction {
    Save,      // Ctrl+S
    SaveAs,    // Ctrl+Shift+S
    Open,      // Ctrl+O
    New,       // Ctrl+N
}
```

The handler detects key presses in an input closure and returns an action enum to avoid borrow conflicts, then executes the action outside the closure.

## Usage

### Opening Files (Multiple Selection)

```rust
use crate::files::dialogs::open_multiple_files_dialog;

// In app.rs handle_open_file()
let paths = open_multiple_files_dialog(initial_dir.as_ref());

for path in paths {
    match self.state.open_file(path) {
        Ok(_) => success_count += 1,
        Err(e) => warn!("Failed to open: {}", e),
    }
}

// Show toast for multiple files
if file_count > 1 && success_count > 0 {
    self.state.show_toast(format!("Opened {} files", success_count), time, 2.0);
}
```

### Reading File Contents

```rust
use crate::files::operations::read_file_contents;

match read_file_contents(&path) {
    Ok(content) => println!("Content: {}", content),
    Err(Error::FileNotFound(_)) => println!("File not found"),
    Err(e) => println!("Error: {}", e),
}
```

### Saving a File

```rust
// Save to existing path (handle_save_file)
fn handle_save_file(&mut self) {
    let has_path = self.state.active_tab()
        .map(|t| t.path.is_some())
        .unwrap_or(false);

    if has_path {
        match self.state.save_active_tab() {
            Ok(_) => debug!("File saved"),
            Err(e) => self.state.set_status(format!("Error: {}", e)),
        }
    } else {
        self.handle_save_as_file(); // No path, trigger Save As
    }
}
```

### Save As

```rust
use crate::files::dialogs::save_file_dialog;

// Get initial directory and default filename
let initial_dir = /* from current file or recent files */;
let default_name = /* from tab path or "untitled.md" */;

if let Some(path) = save_file_dialog(initial_dir.as_ref(), Some(&default_name)) {
    match self.state.save_active_tab_as(path.clone()) {
        Ok(_) => self.state.set_status(format!("Saved: {}", path.display())),
        Err(e) => self.state.set_status(format!("Error: {}", e)),
    }
}
```

### Writing File Contents (Low-level)

```rust
use crate::files::operations::write_file_contents;

write_file_contents(&path, "# Hello World")?;
// Parent directories are created automatically
```

## Dirty State Tracking

The `Tab` struct tracks unsaved changes:

```rust
impl Tab {
    /// Check if content differs from saved version
    pub fn is_modified(&self) -> bool {
        self.content != self.original_content
    }

    /// Mark current content as saved
    pub fn mark_saved(&mut self) {
        self.original_content = self.content.clone();
    }

    /// Title shows asterisk for modified tabs
    pub fn title(&self) -> String {
        let name = /* filename or "Untitled" */;
        if self.is_modified() {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }
}
```

## Tests

Run file operation tests:

```bash
cargo test files::
```

Test coverage includes:
- Reading existing files
- Reading non-existent files (FileNotFound error)
- Writing files (creates parent directories)
- Path validation
- File size retrieval
- Text vs binary file detection
- FileDialogOptions builder pattern
