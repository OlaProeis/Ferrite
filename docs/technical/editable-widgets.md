# Editable Markdown Widgets

## Overview

This module provides standalone editable widgets for markdown elements that synchronize changes back to the markdown source through the AST. These widgets can be used independently or integrated with the `MarkdownEditor`.

## Widgets

### EditableHeading

An editable heading widget (H1-H6) that renders with:
- Visual level indicator (# symbols)
- Scaled font size based on level
- Inline text editing
- Optional level controls (+/- buttons)

```rust
use crate::markdown::{EditableHeading, HeadingLevel, WidgetOutput};

let mut text = "My Heading".to_string();
let mut level = HeadingLevel::H1;

let output: WidgetOutput = EditableHeading::new(&mut text, &mut level)
    .font_size(14.0)
    .with_level_controls()  // Optional +/- buttons
    .show(ui);

if output.changed {
    // output.markdown contains "# My Heading"
}
```

**Font Sizes by Level:**
| Level | Scale Factor |
|-------|-------------|
| H1 | 1.8x |
| H2 | 1.5x |
| H3 | 1.3x |
| H4 | 1.15x |
| H5 | 1.05x |
| H6 | 1.0x |

### EditableParagraph

An editable paragraph widget with:
- Multi-line text editing
- Word wrap support
- Optional indentation

```rust
use crate::markdown::{EditableParagraph, WidgetOutput};

let mut text = "This is a paragraph.\nWith multiple lines.".to_string();

let output: WidgetOutput = EditableParagraph::new(&mut text)
    .font_size(14.0)
    .indent(1)  // Optional indentation level
    .show(ui);

if output.changed {
    // output.markdown contains the paragraph text
}
```

### EditableList

An editable list widget (ordered or unordered) with:
- Bullet (•) or numbered (1. 2. 3.) markers
- Inline editing of items
- Task list checkbox support
- Add/remove item controls (optional)

```rust
use crate::markdown::{EditableList, ListItem, ListType, WidgetOutput};

let mut items = vec![
    ListItem::new("First item"),
    ListItem::new("Second item"),
    ListItem::task("Task item", false),  // Unchecked task
];
let mut list_type = ListType::Bullet;

let output: WidgetOutput = EditableList::new(&mut items, &mut list_type)
    .font_size(14.0)
    .with_controls()  // Enable add/remove buttons
    .show(ui);

if output.changed {
    // output.markdown contains formatted list
}
```

**List Types:**
- `ListType::Bullet` - Unordered list with `-` markers
- `ListType::Ordered { start, delimiter }` - Numbered list

**ListItem Variants:**
- `ListItem::new(text)` - Regular list item
- `ListItem::task(text, checked)` - Task list item with checkbox

## WidgetOutput

All widgets return a `WidgetOutput`:

```rust
pub struct WidgetOutput {
    /// Whether the content was modified
    pub changed: bool,
    /// The new markdown text for this element
    pub markdown: String,
}
```

**Constructors:**
- `WidgetOutput::unchanged(markdown)` - No modifications
- `WidgetOutput::modified(markdown)` - Content was changed

## WidgetColors

Theme-aware color configuration:

```rust
pub struct WidgetColors {
    pub text: Color32,        // Primary text
    pub heading: Color32,     // Heading text
    pub code_bg: Color32,     // Code background
    pub list_marker: Color32, // Bullets/numbers
    pub muted: Color32,       // Dimmed text
}

// Create from theme
let colors = WidgetColors::from_theme(Theme::Dark, ui.visuals());
```

## AST-to-Markdown Serialization

The `serialize_node` function converts AST nodes back to markdown:

```rust
use crate::markdown::{parse_markdown, serialize_node};

let doc = parse_markdown("# Hello\n\nWorld")?;
let markdown = serialize_node(&doc.root);
// markdown == "# Hello\n\nWorld"
```

**Supported Elements:**
- Headings (H1-H6)
- Paragraphs with inline formatting
- Code blocks (fenced and indented)
- Block quotes
- Lists (ordered, unordered, task lists)
- Tables with alignment
- Horizontal rules
- Front matter
- Links and images
- Inline elements (bold, italic, code, strikethrough)

### Format Functions

Individual format functions for each element type:

```rust
use crate::markdown::{format_heading, format_paragraph, format_list};
use crate::markdown::{HeadingLevel, ListType, ListItem};

// Format heading
let md = format_heading("Hello", HeadingLevel::H2);
// "## Hello"

// Format paragraph
let md = format_paragraph("Some text");
// "Some text"

// Format list
let items = vec![ListItem::new("A"), ListItem::new("B")];
let md = format_list(&items, &ListType::Bullet);
// "- A\n- B"
```

## Integration with MarkdownEditor

The widgets are imported and available for use in `MarkdownEditor`:

```rust
use crate::markdown::{MarkdownEditor, EditorMode};
use crate::markdown::{EditableHeading, EditableParagraph, EditableList};

// MarkdownEditor uses these widgets internally in Rendered mode
let output = MarkdownEditor::new(&mut content)
    .mode(EditorMode::Rendered)
    .show(ui);
```

## Testing

The widgets module includes 14 unit tests:

```bash
cargo test markdown::widgets
```

Tests cover:
- Heading formatting and level changes
- List formatting (bullet, ordered, task)
- Widget output creation
- ListItem constructors
- Color theme generation

## File Structure

```
src/markdown/
├── mod.rs       # Module exports
├── parser.rs    # AST types (Task 19)
├── editor.rs    # MarkdownEditor (Task 20)
└── widgets.rs   # Editable widgets (Task 21)
```

## Related Documentation

- [Markdown Parser](markdown-parser.md) - AST types and parsing (Task 19)
- [WYSIWYG Editor](wysiwyg-editor.md) - MarkdownEditor widget (Task 20)
- [Settings Config](settings-config.md) - Theme settings
