# Font System

## Overview

Custom font loading and selection system for Ferrite. Provides user-selectable fonts with proper support for bold, italic, and combined styling through explicit font family variants.

## Key Files

- `src/fonts.rs` - Font definitions, loading, and family selection
- `src/config/settings.rs` - `EditorFont` enum for user selection
- `src/ui/settings.rs` - Font selection UI in Appearance section
- `src/markdown/editor.rs` - Font usage in WYSIWYG rendered mode
- `src/editor/widget.rs` - Font usage in Raw editor mode
- `assets/fonts/` - Embedded font files (TTF)

## Implementation Details

### Why Custom Fonts?

egui's default `RichText::strong()` method relies on the system font having a bold variant. Many fonts don't properly support this, resulting in no visible bolding. By loading explicit font variants (Regular, Bold, Italic, BoldItalic), we ensure consistent styling across all systems.

### EditorFont Enum

User-selectable fonts defined in `settings.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorFont {
    #[default]
    Inter,          // Modern proportional font
    JetBrainsMono,  // Developer-friendly monospace
}

impl EditorFont {
    pub fn display_name(&self) -> &'static str;
    pub fn description(&self) -> &'static str;
    pub fn all() -> &'static [EditorFont];
}
```

### Font Loading

Fonts are embedded at compile time using `include_bytes!`:

```rust
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
const INTER_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-Italic.ttf");
const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-BoldItalic.ttf");
// ... JetBrains Mono variants similarly
```

Each variant is registered as a named font family:

| Font Family Name | Description |
|------------------|-------------|
| `Inter` | Regular weight |
| `Inter-Bold` | Bold weight |
| `Inter-Italic` | Italic style |
| `Inter-BoldItalic` | Bold + Italic |
| `JetBrains Mono` | Monospace regular |
| `JetBrains Mono-Bold` | Monospace bold |
| ... | etc. |

### Font Selection Function

The core function that maps styling flags to font families:

```rust
pub fn get_styled_font_family(
    bold: bool, 
    italic: bool, 
    editor_font: EditorFont
) -> FontFamily {
    match editor_font {
        EditorFont::Inter => match (bold, italic) {
            (true, true) => FontFamily::Name("Inter-BoldItalic".into()),
            (true, false) => FontFamily::Name("Inter-Bold".into()),
            (false, true) => FontFamily::Name("Inter-Italic".into()),
            (false, false) => FontFamily::Name("Inter".into()),
        },
        EditorFont::JetBrainsMono => match (bold, italic) {
            // ... similar pattern
        },
    }
}
```

### Integration Points

#### WYSIWYG Editor (`markdown/editor.rs`)

The `TextStyle` struct accumulates bold/italic flags during recursive rendering:

```rust
struct TextStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
}

impl TextStyle {
    fn apply(&self, text: RichText, font_size: f32, editor_font: EditorFont) -> RichText {
        let family = fonts::get_styled_font_family(self.bold, self.italic, editor_font);
        let mut styled = text.font(FontId::new(font_size, family));
        
        if self.strikethrough {
            styled = styled.strikethrough();
        }
        styled
    }
}
```

#### Raw Editor (`editor/widget.rs`)

Uses selected font for the TextEdit widget:

```rust
let font_family = fonts::get_styled_font_family(false, false, self.font_family);
TextEdit::multiline(content)
    .font(FontId::new(font_size, font_family))
```

#### Line Numbers

Line numbers always use monospace font for proper alignment:

```rust
let line_number_font_id = FontId::monospace(font_size);
```

### Settings UI

Font selection appears in Settings → Appearance:

```rust
ui.label("Font Family");
for font in EditorFont::all() {
    let selected = settings.font_family == *font;
    if ui.add(RadioButton::new(selected, font.display_name())).clicked() {
        settings.font_family = *font;
    }
    ui.label(font.description());
}
```

## Bundled Fonts

| Font | Type | License | Use Case |
|------|------|---------|----------|
| Inter | Proportional | SIL OFL 1.1 | Default, readable text |
| JetBrains Mono | Monospace | SIL OFL 1.1 | Code-like editing |

Font files are stored in `assets/fonts/` with their respective licenses.

## Adding New Fonts

1. Add TTF files to `assets/fonts/` (Regular, Bold, Italic, BoldItalic)
2. Add `include_bytes!` constants in `fonts.rs`
3. Add font family name constants
4. Register fonts in `create_font_definitions()`
5. Add variant to `EditorFont` enum
6. Update `get_styled_font_family()` match arms
7. Add display name and description

## Usage

```rust
use crate::config::EditorFont;
use crate::fonts;

// Get font family for bold text with Inter
let bold_family = fonts::get_styled_font_family(true, false, EditorFont::Inter);
let font_id = FontId::new(14.0, bold_family);

// Apply to RichText
let styled = RichText::new("Bold text").font(font_id);
```

## Tests

Run font system tests:

```bash
cargo test fonts::tests
```

Tests verify:
- Font definitions are created correctly
- Style combinations map to correct families
- Both Inter and JetBrains Mono variants work
