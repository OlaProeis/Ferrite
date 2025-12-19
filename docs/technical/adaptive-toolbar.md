# Adaptive Toolbar

## Overview

The adaptive toolbar system dynamically shows/hides ribbon buttons based on the active file's type. Markdown files display formatting buttons, while structured data files (JSON, YAML, TOML) show format and validate buttons. Universal buttons like Save, Undo, and Find remain visible for all file types.

## Key Files

- `src/state.rs` - `FileType` enum and `Tab::file_type()` method
- `src/ui/ribbon.rs` - Conditional button rendering based on file type
- `src/app.rs` - Passes file type to ribbon, handles structured data actions

## Implementation Details

### FileType Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Markdown,
    Json,
    Yaml,
    Toml,
    Unknown,
}
```

The enum provides helper methods:
- `from_path(path: &Path)` - Detect type from file path
- `from_extension(ext: &str)` - Detect type from extension string
- `is_markdown()` - Check if Markdown type
- `is_structured()` - Check if JSON, YAML, or TOML
- `display_name()` - Human-readable name for UI

### Tab Integration

Each `Tab` caches its file type to avoid re-computation:

```rust
pub struct Tab {
    // ... other fields ...
    file_type: FileType,
}

impl Tab {
    pub fn file_type(&self) -> FileType {
        self.file_type
    }
    
    pub fn set_path(&mut self, path: PathBuf) {
        self.file_type = FileType::from_path(&path);
        self.path = Some(path);
    }
}
```

### Ribbon Conditional Rendering

The `Ribbon::show()` method accepts a `file_type` parameter and uses it for conditional rendering:

```rust
// Format Group - adapts based on file type
if file_type.is_markdown() {
    // Show Bold, Italic, H1-H3, List, Quote, Code, Link buttons
} else if file_type.is_structured() {
    // Show Format (✨) and Validate (✓) buttons
}

// View Group - view toggle for markdown AND structured files
if file_type.is_markdown() || file_type.is_structured() {
    // Show view mode toggle (📝/👁)
}

// Tools Group - outline for all, sync scroll only for markdown
// Show outline toggle for all types (shows stats for structured files)
if file_type.is_markdown() {
    // Also show sync scroll toggle
}

// Export Group - only for markdown
if file_type.is_markdown() {
    // Show HTML export and Copy as HTML buttons
}
```

### Structured Data Actions

Two new `RibbonAction` variants handle structured data operations:

- `RibbonAction::FormatDocument` - Pretty-prints JSON/YAML/TOML
- `RibbonAction::ValidateSyntax` - Validates syntax and shows toast

Handler implementations in `app.rs`:

```rust
fn handle_format_structured_document(&mut self) {
    // Parse content, serialize with formatting, update tab
}

fn handle_validate_structured_syntax(&mut self) {
    // Parse content, show success/error toast
}
```

## Toolbar Button Layout

### Markdown Files (.md)

| Group | Buttons |
|-------|---------|
| Format | Bold, Italic, H1, H2, H3, List, Quote, Code, Link |
| View | Raw/Rendered (📝/👁), Line Numbers, Sync Scroll |
| Tools | Find (🔍), Outline (📋/📑) |
| Export | HTML Export, Copy as HTML |

### Structured Files (.json, .yaml, .yml, .toml)

| Group | Buttons |
|-------|---------|
| Format | Format Document (✨), Validate (✓) |
| View | Raw/Tree (📝/👁), Line Numbers |
| Tools | Find (🔍), Info Panel (📋/📑) |

### Universal Buttons (All Files)

| Group | Buttons |
|-------|---------|
| File | New, Open, Save, Save As |
| Edit | Undo, Redo |

## View Mode Behavior

- **Markdown**: Raw shows text editor, Rendered shows WYSIWYG editor
- **Structured**: Raw shows text editor, Rendered shows TreeViewer widget
- **Unknown**: Only Raw mode available

The view mode toggle (📝/👁) shows contextual tooltips:
- Markdown: "Switch to Rendered View" / "Switch to Raw Editor"
- Structured: "Switch to Tree View" / "Switch to Raw Editor"

## Outline Panel Adaptation

The outline panel (📋/📑 button) adapts its content:
- **Markdown**: Shows document headings for navigation
- **Structured**: Shows document statistics (key count, nesting depth, etc.)

## Dependencies Used

- No new dependencies - uses existing `state.rs` patterns and `tree_viewer.rs` parsing functions

## Usage

File type detection is automatic:
1. Open or create a file with any extension
2. Toolbar automatically adapts to show relevant buttons
3. Switch tabs to see toolbar update for different file types

## Tests

Run file type tests:
```bash
cargo test file_type
```

Key test cases:
- `test_file_type_from_extension` - Extension detection
- `test_file_type_from_path` - Full path detection
- `test_file_type_helpers` - `is_markdown()`, `is_structured()` methods
- `test_tab_file_type_detection` - Tab integration
- `test_tab_set_path_updates_file_type` - Path change updates type

## Future Improvements

- Add more file types (XML, HTML, CSS, etc.)
- Custom toolbar profiles per file type
- User-configurable button visibility
- Animated transitions when toolbar changes
