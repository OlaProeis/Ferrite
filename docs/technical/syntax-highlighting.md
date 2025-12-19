# Syntax Highlighting

This document describes the syntax highlighting system for fenced code blocks in Ferrite.

## Overview

The syntax highlighting feature uses **syntect** (v5.1) to provide syntax-aware coloring for code blocks in the rendered/WYSIWYG editor mode. This enhances code readability by applying theme-consistent colors to language constructs like keywords, strings, comments, and operators.

## Code Block UI Features

Code blocks in the WYSIWYG editor include:

- **Visible Background**: Light gray (`#E9ECEF`) in light mode, dark (`#23272E`) in dark mode
- **Border**: Subtle border with rounded corners (6px)
- **Header Row**: Language label (left) + Copy button (right)
- **Copy Button** (📋): One-click clipboard copy with tooltip
- **Separator**: Visual divider between header and code
- **Click-to-Edit**: Click anywhere in code to enter edit mode

## Architecture

### Module Structure

```
src/markdown/
├── syntax.rs      # Syntax highlighting module
├── editor.rs      # WYSIWYG editor (consumes highlighting)
└── mod.rs         # Module exports
```

### Key Components

#### `SyntaxHighlighter`

The main struct that manages syntax and theme sets:

```rust
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,    // Loaded syntax definitions
    theme_set: ThemeSet,      // Loaded color themes
}
```

**Key Methods:**
- `new()` - Creates highlighter with default syntaxes and themes
- `highlight_code(code, language, theme)` - Highlights code with specific theme
- `highlight_code_for_mode(code, language, dark_mode)` - Auto-selects theme based on mode
- `get_theme_for_mode(dark_mode)` - Returns appropriate theme for dark/light mode
- `find_syntax_for_language(language)` - Maps language identifier to syntax definition

#### `HighlightedLine` and `HighlightedSegment`

Represents highlighted output:

```rust
pub struct HighlightedLine {
    pub segments: Vec<HighlightedSegment>,
}

pub struct HighlightedSegment {
    pub text: String,
    pub foreground: Color32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}
```

#### Global Highlighter

A lazy-initialized global instance avoids repeated loading overhead:

```rust
static HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();

pub fn get_highlighter() -> &'static SyntaxHighlighter {
    HIGHLIGHTER.get_or_init(SyntaxHighlighter::new)
}
```

## Usage

### Basic Usage

```rust
use crate::markdown::syntax::{highlight_code, highlight_code_with_theme};

// Highlight with automatic theme selection
let lines = highlight_code("fn main() {}", "rust", true); // dark mode

// Highlight with specific theme
let lines = highlight_code_with_theme(
    "print('hello')",
    "python",
    "base16-ocean.dark",
    true
);
```

### In WYSIWYG Editor

The `render_code_block` function in `editor.rs` uses syntax highlighting:

```rust
fn render_code_block(ui, source, edit_state, colors, font_size, language, literal, node) {
    let dark_mode = colors.background.r() < 128;
    let highlighted_lines = highlight_code(&code, language, dark_mode);
    
    for line in &highlighted_lines {
        ui.horizontal(|ui| {
            for segment in &line.segments {
                ui.label(segment.to_rich_text(font_size));
            }
        });
    }
}
```

### Converting to egui RichText

```rust
let segment = HighlightedSegment {
    text: "let".to_string(),
    foreground: Color32::from_rgb(198, 120, 221),
    bold: true,
    italic: false,
    underline: false,
};

// Convert to egui RichText
let rich_text = segment.to_rich_text(14.0);
ui.label(rich_text);
```

## Supported Languages

The highlighter supports 50+ languages via syntect's default syntax set. Common languages are mapped through aliases:

| Alias(es) | Extension | Language |
|-----------|-----------|----------|
| rust, rs | rs | Rust |
| python, py | py | Python |
| javascript, js | js | JavaScript |
| typescript, ts | ts | TypeScript |
| cpp, c++, cxx | cpp | C++ |
| csharp, c#, cs | cs | C# |
| java | java | Java |
| go, golang | go | Go |
| ruby, rb | rb | Ruby |
| php | php | PHP |
| swift | swift | Swift |
| kotlin, kt | kt | Kotlin |
| html, htm | html | HTML |
| css | css | CSS |
| json | json | JSON |
| yaml, yml | yaml | YAML |
| toml | toml | TOML |
| sql | sql | SQL |
| shell, sh, bash, zsh | sh | Shell |
| markdown, md | md | Markdown |
| ... | ... | ... |

Unknown languages fall back to plain text display with the theme's default foreground color.

## Theme Integration

### Built-in Themes

| Theme Name | Mode | Description |
|------------|------|-------------|
| `base16-ocean.dark` | Dark | Default dark theme |
| `base16-eighties.dark` | Dark | Eighties-inspired dark |
| `InspiredGitHub` | Light | GitHub-inspired light |
| `Solarized (dark)` | Dark | Solarized dark |
| `Solarized (light)` | Light | Solarized light |

### Default Theme Selection

```rust
pub const DEFAULT_DARK_THEME: &str = "base16-ocean.dark";
pub const DEFAULT_LIGHT_THEME: &str = "InspiredGitHub";
```

### Theme Color Extraction

```rust
let highlighter = get_highlighter();
let theme = highlighter.get_theme_for_mode(true); // dark mode

// Get theme colors
let bg = highlighter.get_theme_background(theme); // Option<Color32>
let fg = highlighter.get_theme_foreground(theme); // Option<Color32>
```

## Performance Considerations

### Caching Strategy

1. **Global SyntaxSet/ThemeSet**: Loaded once, reused for all operations
2. **Lazy Initialization**: Sets only loaded on first use
3. **OnceLock**: Thread-safe initialization without mutex overhead

### Optimization Tips

- The global highlighter instance avoids repeated loading (~10-20ms per load)
- For large documents, consider caching highlighted output
- Line-by-line highlighting (`highlight_line`) is more memory-efficient than whole-document

## Color Conversion

Syntect uses its own Color type; conversion to egui's Color32:

```rust
pub fn syntect_to_egui_color(color: syntect::highlighting::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}
```

## Settings Integration

The syntax theme can be configured in `Settings`:

```rust
// In src/config/settings.rs
pub struct Settings {
    /// Syntax highlighting theme name
    pub syntax_theme: String,  // Default: "base16-ocean.dark"
}
```

Usage with settings:

```rust
let highlighter = get_highlighter();
let theme = highlighter.get_theme_by_name_or_mode(
    &settings.syntax_theme,
    dark_mode
);
```

## Testing

The module includes comprehensive tests:

```rust
#[test]
fn test_highlight_rust_code() {
    let highlighter = SyntaxHighlighter::new();
    let code = "fn main() {\n    println!(\"Hello\");\n}";
    let lines = highlighter.highlight_code_for_mode(code, "rust", true);
    
    assert_eq!(lines.len(), 3);
    assert!(!lines[0].segments.is_empty());
}

#[test]
fn test_language_aliases() {
    let highlighter = SyntaxHighlighter::new();
    let syntax1 = highlighter.find_syntax_for_language("rs");
    let syntax2 = highlighter.find_syntax_for_language("rust");
    
    assert_eq!(syntax1.unwrap().name, syntax2.unwrap().name);
}
```

## API Reference

### Public Functions

| Function | Description |
|----------|-------------|
| `get_highlighter()` | Get global highlighter instance |
| `highlight_code(code, lang, dark)` | Highlight with auto theme |
| `highlight_code_with_theme(code, lang, theme, dark)` | Highlight with specific theme |
| `syntect_to_egui_color(color)` | Convert syntect Color to egui Color32 |

### Public Types

| Type | Description |
|------|-------------|
| `SyntaxHighlighter` | Main highlighter struct |
| `HighlightedLine` | A line of highlighted segments |
| `HighlightedSegment` | A segment with color and style |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `DEFAULT_DARK_THEME` | `"base16-ocean.dark"` | Default dark theme |
| `DEFAULT_LIGHT_THEME` | `"InspiredGitHub"` | Default light theme |
| `FALLBACK_THEME` | `"base16-ocean.dark"` | Fallback if theme not found |
