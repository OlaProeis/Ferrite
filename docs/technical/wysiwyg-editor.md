# WYSIWYG Markdown Editor Widget

## Overview

The WYSIWYG (What You See Is What You Get) Markdown Editor is a widget that allows users to edit markdown content directly in a rendered view. Instead of editing raw markdown text, users interact with styled, semantic egui widgets that represent markdown elements.

## Architecture

### Module Structure

```
src/markdown/
├── mod.rs          # Module exports
├── parser.rs       # Markdown parsing (Task 19)
└── editor.rs       # WYSIWYG editor widget (Task 20)
```

### Key Components

1. **EditorMode** - Enum defining editing modes:
   - `Raw` - Plain text markdown editing
   - `Rendered` - WYSIWYG editing with styled widgets

2. **MarkdownEditor** - Main widget struct with builder pattern
3. **EditorColors** - Theme-aware color definitions
4. **EditState** - Tracks editable nodes and modifications
5. **MarkdownEditorOutput** - Result type containing response and change status

## Usage

### Basic Usage

```rust
use crate::markdown::{MarkdownEditor, EditorMode};

let mut content = "# Hello\n\nWorld".to_string();

let output = MarkdownEditor::new(&mut content)
    .mode(EditorMode::Rendered)
    .show(ui);

if output.changed {
    // Content was modified
}
```

### With Settings

```rust
let output = MarkdownEditor::new(&mut content)
    .with_settings(&settings)  // Apply font_size, word_wrap, theme
    .mode(EditorMode::Rendered)
    .show(ui);
```

### Builder Options

```rust
MarkdownEditor::new(&mut content)
    .mode(EditorMode::Rendered)  // Raw or Rendered
    .font_size(16.0)             // Base font size
    .word_wrap(true)             // Enable word wrapping
    .theme(Theme::Dark)          // Color theme
    .id(egui::Id::new("my_editor"))  // Custom ID
    .show(ui)
```

## Editing Modes

### Raw Mode

In raw mode, the editor displays a plain text editor for markdown source:

- Uses monospace font
- Full syntax visible (e.g., `# Heading`, `**bold**`)
- Standard text editing capabilities
- Cursor position tracking

### Rendered Mode

In rendered mode, markdown elements are displayed as styled widgets:

| Markdown Element | Widget Type | Features |
|------------------|-------------|----------|
| Headings (H1-H6) | `TextEdit::singleline` | Scaled font sizes, colored |
| Paragraphs | `TextEdit::multiline` | Full text editing |
| Code Blocks | `TextEdit::multiline` + Frame | Monospace, syntax-aware |
| Block Quotes | Bordered container | Visual quote styling |
| Lists (ul/ol) | Items with markers | Bullet/numbered rendering |
| Task Lists | Checkbox + Text | Toggle-able checkboxes |
| Tables | `egui::Grid` | Striped rows, headers |
| Horizontal Rules | Painted rect | Theme-aware color |
| Front Matter | Code display | Read-only YAML/TOML |

## Theming

The editor supports three themes:

1. **Light** - Light background with dark text
2. **Dark** - Dark background with light text  
3. **System** - Follows egui's `Visuals.dark_mode`

### Color Palette

```rust
pub struct EditorColors {
    pub background: Color32,     // Editor background
    pub text: Color32,           // Primary text
    pub heading: Color32,        // Heading text
    pub code_bg: Color32,        // Code block background
    pub code_text: Color32,      // Code text
    pub quote_border: Color32,   // Block quote border
    pub quote_text: Color32,     // Block quote text
    pub link: Color32,           // Link color
    pub hr: Color32,             // Horizontal rule
    pub list_marker: Color32,    // Bullets/numbers
    pub checkbox: Color32,       // Task list checkbox
}
```

## Source Synchronization

The WYSIWYG editor maintains synchronization between the visual representation and underlying markdown source:

### How It Works

1. **Parsing**: Content is parsed to AST using `parse_markdown()`
2. **Rendering**: Each AST node becomes an editable widget
3. **Edit Tracking**: `EditState` tracks modifications per node
4. **Reconstruction**: Modified nodes trigger source updates

### Edit Tracking

```rust
struct EditableNode {
    id: usize,          // Unique node identifier
    text: String,       // Current text content
    start_line: usize,  // Source position
    end_line: usize,    // Source position
    modified: bool,     // Change flag
}
```

### Source Update Functions

- `update_source_line()` - Update a single line
- `update_source_range()` - Update a range of lines
- `update_code_block()` - Reconstruct code block with fences
- `rebuild_markdown()` - Full document reconstruction

## Supported Markdown Elements

### Block Elements

- [x] Document root
- [x] Headings (H1-H6, ATX and Setext)
- [x] Paragraphs
- [x] Code blocks (fenced and indented)
- [x] Block quotes (including nested)
- [x] Lists (ordered and unordered)
- [x] Task lists (checkboxes)
- [x] Tables (with alignment)
- [x] Thematic breaks (horizontal rules)
- [x] Front matter (YAML/TOML)
- [x] HTML blocks (read-only display)

### Inline Elements

- [x] Text
- [x] Bold/Strong
- [x] Italic/Emphasis
- [x] Inline code
- [x] Links
- [x] Images
- [x] Strikethrough
- [x] Soft/Hard line breaks

## Integration with Tab System

The WYSIWYG editor is designed to integrate with the application's tab system:

```rust
// In the main UI loop
if let Some(tab) = state.active_tab_mut() {
    let output = MarkdownEditor::new(&mut tab.content)
        .with_settings(&state.settings)
        .mode(current_mode)
        .show(ui);
    
    // Track if tab was modified
    if output.changed {
        // Tab's is_modified() will reflect this
    }
}
```

## Design Decisions

### Single Editing Area

The WYSIWYG editor uses a **single editing area** that toggles between raw and rendered modes:

- No split view or side-by-side preview
- Mode toggle via keyboard shortcut (Ctrl+Shift+V) or UI button
- Simpler UX and implementation
- Better performance

### Best-Effort Structure Preservation

When editing in WYSIWYG mode:

- Markdown structure is preserved where possible
- Some edge cases may result in slightly different markdown
- The document always remains valid markdown

### Performance Considerations

- AST is rebuilt on each frame for simplicity
- For very large documents, consider caching or virtual scrolling
- Edit tracking is node-based for efficient change detection

## Testing

The editor includes 20 unit tests covering:

- Editor mode defaults and equality
- Theme color generation
- Edit state management
- Heading formatting
- Source line/range updates
- Character index conversion
- Builder pattern

Run tests with:

```bash
cargo test markdown::editor
```

## Future Enhancements

Potential improvements for future tasks:

1. **Syntax Highlighting** - Code blocks with language-specific colors
2. **Image Preview** - Inline image display
3. **Link Clickability** - Navigate links in rendered mode
4. **Table Editing** - Add/remove rows and columns
5. **Undo/Redo** - Per-widget undo history
6. **Virtual Scrolling** - Performance for large documents
7. **Cursor Sync** - Line-based cursor tracking in WYSIWYG mode

## Related Documentation

- [Markdown Parser](markdown-parser.md) - AST types and parsing (Task 19)
- [Editor Widget](editor-widget.md) - Raw text editor patterns
- [Settings Config](settings-config.md) - Theme and font settings
- [Tab System](tab-system.md) - Tab integration patterns
