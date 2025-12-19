# Editable Code Blocks

## Overview

The editable code blocks feature (Task 44) enables WYSIWYG-style editing of markdown code blocks in rendered mode. Users can edit code content in-place, select a programming language from a dropdown, and have their changes automatically synchronized back to the markdown source.

## Features

### View Mode
- **Syntax Highlighting**: Code is displayed with syntax highlighting using syntect
- **Copy Button**: One-click copy to clipboard (📋 icon)
- **Edit Button**: Click ✎ to enter edit mode
- **Language Label**: Displays current language (or "Code" for plain text)
- **Click-to-Edit**: Click anywhere in the code area to start editing

### Edit Mode
- **Language Dropdown**: Select from 30+ supported languages
- **Monospace TextEdit**: Full code editing with monospace font
- **Auto-Exit**: Clicking outside the code block exits edit mode
- **Done Button**: Click ✓ to explicitly finish editing
- **Live Markdown Sync**: Changes are immediately reflected in the markdown source

## Supported Languages

The following languages are supported with syntax highlighting:

| Language | Code Fence ID |
|----------|---------------|
| Plain Text | (empty) |
| Rust | `rust`, `rs` |
| Python | `python`, `py` |
| JavaScript | `javascript`, `js` |
| TypeScript | `typescript`, `ts` |
| Go | `go`, `golang` |
| Java | `java` |
| C | `c` |
| C++ | `cpp`, `c++` |
| C# | `csharp`, `cs`, `c#` |
| HTML | `html` |
| CSS | `css` |
| JSON | `json` |
| YAML | `yaml`, `yml` |
| TOML | `toml` |
| Markdown | `markdown`, `md` |
| Bash | `bash`, `sh`, `shell` |
| SQL | `sql` |
| Ruby | `ruby`, `rb` |
| PHP | `php` |
| Swift | `swift` |
| Kotlin | `kotlin`, `kt` |
| Scala | `scala` |
| Lua | `lua` |
| Perl | `perl`, `pl` |
| R | `r` |
| Haskell | `haskell`, `hs` |
| Elixir | `elixir`, `ex` |
| Clojure | `clojure`, `clj` |
| XML | `xml` |
| Dockerfile | `dockerfile`, `docker` |
| Makefile | `makefile`, `make` |
| Diff | `diff`, `patch` |

## Architecture

### Module Structure

```
src/markdown/
├── widgets.rs         # EditableCodeBlock widget, CodeBlockData, SUPPORTED_LANGUAGES
├── editor.rs          # render_code_block() uses EditableCodeBlock
└── syntax.rs          # Syntax highlighting via syntect (unchanged)
```

### Key Types

#### `CodeBlockData`
Stores the state of a code block:
```rust
pub struct CodeBlockData {
    pub code: String,           // The code content
    pub language: String,       // Language identifier (e.g., "rust")
    pub is_editing: bool,       // Whether in edit mode
    original_language: String,  // For change detection
    original_code: String,      // For change detection
}
```

#### `CodeBlockOutput`
Returned by the widget after rendering:
```rust
pub struct CodeBlockOutput {
    pub changed: bool,           // Any change occurred
    pub language_changed: bool,  // Language specifically changed
    pub markdown: String,        // New markdown representation
    pub code: String,            // Current code content
    pub language: String,        // Current language
}
```

#### `EditableCodeBlock<'a>`
The widget struct with builder pattern:
```rust
EditableCodeBlock::new(&mut code_data)
    .font_size(14.0)
    .dark_mode(true)
    .colors(widget_colors)
    .id(unique_id)
    .show(ui)
```

### State Management

Code block state is stored in egui's memory system to persist across frames:
```rust
// Store in memory
let mut code_data = ui.memory_mut(|mem| {
    mem.data
        .get_temp_mut_or_insert_with(code_block_id.with("data"), || {
            CodeBlockData::new(literal, language)
        })
        .clone()
});

// Update after widget renders
ui.memory_mut(|mem| {
    mem.data.insert_temp(code_block_id.with("data"), code_data);
});
```

### Markdown Synchronization

When code or language changes, the `update_code_block()` function in `editor.rs` updates the source:
```rust
fn update_code_block(
    source: &mut String,
    start_line: usize,
    end_line: usize,
    language: &str,
    new_content: &str,
)
```

This regenerates the fenced code block syntax:
```markdown
```language
code content here
```
```

## Usage Example

### In Editor (editor.rs)

```rust
fn render_code_block(ui: &mut Ui, ...) {
    // Get or create code block data from memory
    let mut code_data = ui.memory_mut(|mem| {
        mem.data.get_temp_mut_or_insert_with(id, || {
            CodeBlockData::new(literal, language)
        }).clone()
    });

    // Render the widget
    let output = EditableCodeBlock::new(&mut code_data)
        .font_size(font_size)
        .dark_mode(dark_mode)
        .show(ui);

    // Handle changes
    if output.changed {
        update_code_block(source, start_line, end_line, &output.language, &output.code);
    }
}
```

### Standalone Widget

```rust
let mut data = CodeBlockData::new("fn main() {}", "rust");

let output = EditableCodeBlock::new(&mut data)
    .font_size(14.0)
    .dark_mode(ui.visuals().dark_mode)
    .show(ui);

if output.changed {
    println!("New markdown:\n{}", output.markdown);
}
```

## User Interface

### Buttons
- **Edit/Done** - Toggle between view and edit mode
- **Copy** - Copy code content to clipboard

### Edit Mode Behavior
- Click "Edit" button or click in the code area to enter edit mode
- Use the language dropdown to change syntax highlighting
- Click "Done" button or click outside the code block to exit edit mode
- The dropdown can be used without accidentally exiting edit mode

## Undo/Redo Integration

Changes made through the code block editor are synchronized through `Tab.set_content()` which automatically records undo history. This ensures:
- Ctrl+Z reverts code changes
- Ctrl+Y re-applies code changes
- Language changes are also undoable

## Theme Support

The widget adapts to light/dark mode:
- **Dark mode**: Dark background (`#23272e`), light text
- **Light mode**: Light background (`#e9ecef`), dark text
- Syntax highlighting colors adjust based on mode (via syntect themes)

## Testing

### Unit Tests (in widgets.rs)

```rust
#[test]
fn test_code_block_data_new() { ... }
#[test]
fn test_code_block_data_modification_detection() { ... }
#[test]
fn test_code_block_to_markdown_with_language() { ... }
#[test]
fn test_language_display_name() { ... }
#[test]
fn test_normalize_language() { ... }
#[test]
fn test_supported_languages_contains_common() { ... }
```

### Manual Testing Checklist

1. ✅ Click fenced code block → edit mode with TextEdit + language dropdown
2. ✅ Edit text, click outside → exit mode, syntax refreshes, markdown updates
3. ✅ Change language dropdown → syntax highlights with new language
4. ✅ Copy button copies latest content without entering edit mode
5. ✅ Raw/Rendered mode switches preserve content
6. ✅ Empty code blocks work without panics
7. ✅ Large code blocks perform reasonably

## Related Files

- `src/markdown/widgets.rs` - EditableCodeBlock widget implementation
- `src/markdown/editor.rs` - Integration with WYSIWYG editor
- `src/markdown/syntax.rs` - Syntax highlighting (syntect)
- `src/markdown/parser.rs` - Markdown AST parsing

## Related Tasks

- **Task 19**: Markdown parser (provides CodeBlock AST nodes)
- **Task 21**: WYSIWYG editor (provides editing infrastructure)
- **Task 23**: Syntax highlighting (provides highlight_code function)
- **Task 45**: Code block copy button (will use existing copy functionality)
