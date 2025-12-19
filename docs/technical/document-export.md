# Document Export System

Technical documentation for the document export functionality in Ferrite.

## Overview

The export system allows users to export markdown documents to standalone HTML files with inlined theme CSS, and copy rendered HTML to the clipboard for pasting into other applications.

## Architecture

### Module Structure

```
src/export/
├── mod.rs          # Module exports and public API
├── options.rs      # Export configuration types
├── html.rs         # HTML document generation
└── clipboard.rs    # Clipboard operations
```

### Key Types

#### ExportFormat

```rust
pub enum ExportFormat {
    HtmlFile,       // Export as standalone HTML file
    ClipboardHtml,  // Copy rendered HTML to clipboard
}
```

#### ImageHandling

```rust
pub enum ImageHandling {
    EmbedBase64,    // Embed images as data URIs (standalone)
    RelativePaths,  // Keep relative paths to images
    AbsolutePaths,  // Convert to absolute file paths
}
```

#### ExportOptions

Configuration for document export:
- `format`: Export target format
- `image_handling`: How to process images
- `include_title`: Include document title in HTML
- `include_syntax_highlighting`: Include syntax CSS
- `use_theme_colors`: Apply current theme colors
- `custom_css`: Optional additional CSS
- `last_export_directory`: Remember last directory
- `open_after_export`: Open file after export

## HTML Generation

### Theme CSS Inlining

The `generate_html_document()` function creates standalone HTML with:

1. **Base CSS**: Typography, layout, markdown element styling
2. **Theme CSS**: Colors from the current theme (light/dark)
3. **Syntax CSS**: Code highlighting colors (optional)

### Color Conversion

Theme colors are converted to CSS using:

```rust
fn color32_to_css(color: Color32) -> String {
    format!("rgb({}, {}, {})", color.r(), color.g(), color.b())
}
```

### Generated HTML Structure

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Ferrite">
    <title>{title}</title>
    <style>
        /* Base CSS */
        /* Theme CSS */
        /* Syntax CSS */
    </style>
</head>
<body>
    <article class="markdown-body">
        {rendered_markdown}
    </article>
</body>
</html>
```

## Clipboard Integration

### arboard Crate

The export system uses `arboard` for cross-platform clipboard support:

```rust
use arboard::Clipboard;

pub fn copy_text_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
```

### HTML Clipboard Format

For applications that support HTML clipboard format:

```rust
pub fn copy_html_with_fallback(html: &str, plain_text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_html(html, Some(plain_text))?;
    Ok(())
}
```

## UI Integration

### Ribbon Buttons

Export actions added to the ribbon:

| Icon | Action | Description |
|------|--------|-------------|
| 🌐 | Export HTML | Save as HTML file |
| 📋 | Copy as HTML | Copy to clipboard |

### Keyboard Shortcuts

- **Ctrl+Shift+E**: Export as HTML (opens save dialog)

### RibbonAction Variants

```rust
pub enum RibbonAction {
    // ... other actions
    ExportHtml,    // Export current document as HTML file
    CopyAsHtml,    // Copy rendered HTML to clipboard
}
```

## Settings Persistence

Export preferences stored in `Settings`:

```rust
// Export Settings
pub last_export_directory: Option<PathBuf>,
pub open_after_export: bool,
pub export_embed_images: bool,
```

## Error Handling

### HtmlExportError

```rust
pub enum HtmlExportError {
    IoError(std::io::Error),        // File operations
    ConversionError(String),         // Markdown conversion
}
```

### ClipboardError

```rust
pub enum ClipboardError {
    AccessError(String),   // Can't access clipboard
    WriteError(String),    // Can't write to clipboard
    HtmlError(HtmlExportError),
}
```

## Handler Implementation

### Export HTML Handler

```rust
fn handle_export_html(&mut self, ctx: &egui::Context) {
    // 1. Get active tab content
    // 2. Determine initial directory and filename
    // 3. Get theme colors for CSS
    // 4. Open save dialog
    // 5. Generate HTML document
    // 6. Write to file
    // 7. Update settings and show toast
}
```

### Copy as HTML Handler

```rust
fn handle_copy_as_html(&mut self) {
    // 1. Get active tab content
    // 2. Generate HTML fragment
    // 3. Copy to clipboard
    // 4. Show toast notification
}
```

## Dependencies

### Cargo.toml

```toml
arboard = "3"  # Cross-platform clipboard support
```

## Usage Examples

### Exporting a Document

1. Open a markdown document
2. Click 🌐 in ribbon OR press Ctrl+Shift+E
3. Choose save location
4. HTML file created with current theme

### Copying to Clipboard

1. Open a markdown document
2. Click 📋 in ribbon
3. Paste in email client, CMS, or word processor
4. Formatted content appears

## Theme Consistency

Exported HTML matches in-app rendering:

- Background colors from `ThemeColors.base`
- Text colors from `ThemeColors.text`
- Heading colors from `ThemeColors.editor.heading`
- Code block styling from `ThemeColors.editor.code_block_*`
- Link colors from `ThemeColors.text.link`

## Future Enhancements

- PDF export via HTML-to-PDF conversion
- Image embedding (base64 data URIs)
- Custom CSS injection
- Template selection
- Batch export multiple documents
