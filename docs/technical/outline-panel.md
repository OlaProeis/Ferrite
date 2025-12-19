# Document Outline Panel

## Overview

The Document Outline Panel provides a live-updating side panel that adapts to the current file type. It appears as a side panel that can be toggled on/off and positioned on either the left or right side of the editor.

The panel supports two modes:
- **Markdown mode** ("📑 Outline"): Shows heading hierarchy (H1-H6) with click-to-navigate
- **Statistics mode** ("📊 Statistics"): Shows JSON/YAML/TOML file statistics

File type is automatically detected based on file extension.

## Features

### Markdown Heading Extraction
- Parses ATX-style headings (`# Heading`, `## Heading`, etc.)
- Strips inline markdown formatting from heading text (bold, italic, code, links)
- Generates stable IDs for each heading using index and content hash
- Tracks line numbers and character offsets for navigation
- Summary shows heading count and estimated reading time

### Structured File Statistics (JSON/YAML/TOML)
- Automatically detects `.json`, `.yaml`, `.yml`, `.toml` files
- Shows **Structure** section:
  - Object count
  - Array count
  - Total keys
  - Maximum nesting depth
- Shows **Values** section:
  - Total value count
  - Strings, Numbers, Booleans, Nulls breakdown
  - Total array items
- Displays parse errors with helpful messages
- Color-coded statistics matching syntax highlighting theme

### Panel UI (Markdown Mode)
- Resizable side panel (120-400px width)
- Configurable position (left or right)
- Summary statistics: heading count and estimated read time
- Scrollable tree view with proper indentation by level
- Color-coded heading level indicators (H1-H6)
- Current section highlighting based on cursor position

### Panel UI (Statistics Mode)
- Shows format name (JSON, YAML, TOML)
- Structure and Values sections with labeled rows
- Color-coded value counts (strings in orange, numbers in green, etc.)

### Navigation (Markdown Only)
- Click any heading to scroll editor to that location (works in both Raw and Rendered modes)
- Double-click headings with children to collapse/expand
- Respects parent collapse state (hidden children stay hidden)
- Scroll position calculated to place target heading 1/3 from top of viewport

### Keyboard Shortcut
- `Ctrl+Shift+O`: Toggle outline panel visibility

## Architecture

### Module Structure

```
src/editor/outline.rs     - Heading extraction and OutlineItem model
src/ui/outline_panel.rs   - Outline panel egui widget
```

### Key Types

#### `OutlineItem`
```rust
pub struct OutlineItem {
    pub id: String,           // Stable ID (index + hash)
    pub level: u8,            // 1-6 for H1-H6
    pub title: String,        // Cleaned heading text
    pub line: usize,          // 1-indexed line number
    pub char_offset: usize,   // Character offset in document
    pub collapsed: bool,      // Collapse state for sections
}
```

#### `DocumentOutline`
```rust
pub struct DocumentOutline {
    pub items: Vec<OutlineItem>,      // Headings (markdown only)
    pub heading_count: usize,         // Total heading count
    pub estimated_read_time: u32,     // Minutes (200 words/min, markdown only)
    pub outline_type: OutlineType,    // Markdown or Structured with stats
}

pub enum OutlineType {
    Markdown,                    // Heading-based outline (H1-H6)
    Structured(StructuredStats), // Statistics for JSON/YAML/TOML
}

pub struct StructuredStats {
    pub total_keys: usize,       // Total number of keys
    pub array_count: usize,      // Number of arrays
    pub object_count: usize,     // Number of objects
    pub value_count: usize,      // Total leaf values
    pub max_depth: usize,        // Maximum nesting depth
    pub string_count: usize,     // String values
    pub number_count: usize,     // Integer + float values
    pub bool_count: usize,       // Boolean values
    pub null_count: usize,       // Null values
    pub total_array_items: usize,// Total items across all arrays
    pub format_name: String,     // "JSON", "YAML", or "TOML"
    pub parse_success: bool,     // Whether parsing succeeded
    pub parse_error: Option<String>, // Error message if failed
}
```

#### `OutlinePanel`
```rust
pub struct OutlinePanel {
    width: f32,                       // Panel width
    side: OutlinePanelSide,           // Left or Right
    current_section: Option<usize>,   // Highlighted heading
}
```

### Data Flow

1. **Content Change Detection**: App tracks content + path hash to detect changes
2. **File Type Detection**: Checks file extension for structured file types
3. **Outline Extraction**: 
   - Markdown files: `extract_outline()` parses headings into `OutlineItem` list
   - Structured files: `extract_structured_outline()` computes `StructuredStats`
   - Or use `extract_outline_for_file()` for automatic detection
4. **Current Section**: Uses cursor line to find active heading/key
5. **Rendering**: Panel renders with type-appropriate labels and colors
6. **Navigation**: Click events return scroll targets to app

## Settings

### Configuration (`settings.rs`)

```rust
// Whether the outline panel is visible
pub outline_enabled: bool,

// Which side of the editor (Left/Right)
pub outline_side: OutlinePanelSide,

// Panel width in pixels (120-500)
pub outline_width: f32,
```

### Persistence
- Settings saved to JSON config file
- Panel width persists after resize
- Visibility state persists across sessions

## Usage

### Toggle Panel
1. Click the outline button (📑) in the ribbon's Tools group
2. Or press `Ctrl+Shift+O`

### Navigate to Heading
1. Open outline panel
2. Click any heading in the list
3. Editor scrolls to that heading's line

### Collapse/Expand Sections
1. Double-click a heading that has children
2. Child headings hide/show accordingly

### Resize Panel
1. Drag the panel edge to resize
2. New width is saved automatically

## Implementation Notes

### Performance
- Outline only regenerates when content hash changes
- Uses efficient single-pass regex parsing
- Collapse state preserved per-document session

### Heading Detection
```rust
// ATX-style headings only
"# Heading 1"
"## Heading 2"
"### Heading 3"
// etc.

// Trailing # marks are stripped
"## Heading ##" -> "Heading"

// Inline formatting is stripped
"# **Bold** Heading" -> "Bold Heading"
"# `code` text" -> "code text"
```

### Color Coding

**Markdown headings** have distinct colors for quick identification:
- H1: Blue
- H2: Green
- H3: Orange
- H4: Purple
- H5: Gray
- H6: Light gray

**Structured file depths** use different bullet colors:
- Level 1 (•): Blue (top-level keys)
- Level 2 (◦): Green (nested keys)
- Level 3 (▪): Orange (deeper nesting)
- Level 4+ (▫): Gray (very deep nesting)

## Future Enhancements

Potential improvements:
- Support for Setext-style headings (`===` and `---` underlines)
- Skip headings inside code blocks
- Per-document collapse state persistence
- Drag-and-drop heading reordering
- Filter/search within outline
- Export outline as TOC markdown
- More structured file types (XML, INI, etc.)
