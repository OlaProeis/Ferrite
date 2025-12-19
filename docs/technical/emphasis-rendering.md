# Emphasis Rendering in WYSIWYG Mode

## Overview

Ferrite supports bold, italic, and strikethrough text formatting in the WYSIWYG (Rendered) view mode. This document describes how emphasis markdown is parsed and rendered, including support for nested/combined formatting.

## Supported Syntax

### Basic Emphasis

| Markdown | Rendered | Style |
|----------|----------|-------|
| `*italic*` | *italic* | Italic |
| `_italic_` | _italic_ | Italic |
| `**bold**` | **bold** | Bold |
| `__bold__` | __bold__ | Bold |
| `~~strikethrough~~` | ~~strikethrough~~ | Strikethrough |

### Nested/Combined Emphasis

| Markdown | Rendered | Styles Applied |
|----------|----------|----------------|
| `***bold italic***` | ***bold italic*** | Bold + Italic |
| `___bold italic___` | ___bold italic___ | Bold + Italic |
| `**_bold italic_**` | **_bold italic_** | Bold + Italic |
| `__*bold italic*__` | __*bold italic*__ | Bold + Italic |
| `*__bold italic__*` | *__bold italic__* | Bold + Italic |
| `_**bold italic**_` | _**bold italic**_ | Bold + Italic |
| `~~**bold strikethrough**~~` | ~~**bold**~~ | Bold + Strikethrough |
| `~~*italic strikethrough*~~` | ~~*italic*~~ | Italic + Strikethrough |
| `~~***all three***~~` | ~~***all***~~ | Bold + Italic + Strikethrough |

## Implementation

### Architecture

The emphasis rendering is implemented in `src/markdown/editor.rs` using a style accumulation pattern:

```
Parser (comrak) → AST with nested nodes → Renderer with TextStyle accumulator → egui RichText
```

### Key Components

#### 1. TextStyle Accumulator

```rust
struct TextStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
}
```

The `TextStyle` struct accumulates formatting as the renderer traverses nested AST nodes. When entering a `Strong` node, bold is added; when entering `Emphasis`, italic is added; etc.

#### 2. Style Propagation

When rendering nested emphasis nodes:

1. The renderer starts with an empty `TextStyle`
2. For each formatting node (Strong, Emphasis, Strikethrough), the corresponding flag is set
3. The accumulated style is passed to child nodes
4. When a `Text` node is reached, all accumulated styles are applied via `style.apply(rich_text)`

#### 3. RichText Application

The `TextStyle::apply()` method chains egui's `RichText` styling methods:

```rust
fn apply(&self, mut text: RichText) -> RichText {
    if self.bold { text = text.strong(); }
    if self.italic { text = text.italics(); }
    if self.strikethrough { text = text.strikethrough(); }
    text
}
```

### AST Structure Example

For `***bold italic***`, comrak produces:

```
Paragraph
└── Strong
    └── Emphasis
        └── Text("bold italic")
```

Or alternatively (depending on marker order):

```
Paragraph
└── Emphasis
    └── Strong
        └── Text("bold italic")
```

Both structures render identically because the TextStyle accumulator collects all formatting regardless of nesting order.

## Files Modified

- `src/markdown/editor.rs` - Main WYSIWYG rendering logic
  - Added `TextStyle` struct for style accumulation
  - Updated `render_inline_node()` to propagate styles
  - Updated `render_styled_inline()` for top-level emphasis nodes
  - Removed simple `render_strong()` and `render_emphasis()` functions

- `src/markdown/parser.rs` - Added tests for nested emphasis parsing

## Testing

### Unit Tests Added

- `test_text_style_default` - Default style has no formatting
- `test_text_style_with_bold` - Bold flag works
- `test_text_style_with_italic` - Italic flag works
- `test_text_style_with_strikethrough` - Strikethrough flag works
- `test_text_style_bold_and_italic` - Combined styles work
- `test_text_style_all_combined` - All three styles combined
- `test_text_style_chaining_order_independent` - Order doesn't matter
- `test_text_style_apply_no_style` - Apply with empty style
- `test_text_style_apply_with_styles` - Apply with combined styles

### Parser Tests Added

- `test_parse_bold_italic_triple_asterisk` - `***text***` parsing
- `test_parse_bold_inside_italic` - `_**text**_` parsing
- `test_parse_italic_inside_bold` - `**_text_**` parsing
- `test_parse_mixed_emphasis_in_sentence` - Multiple emphasis types in one paragraph
- `test_parse_underscore_emphasis` - Underscore syntax variants
- `test_parse_strikethrough_with_bold` - Combined strikethrough and bold

## Limitations

1. **Inline code** (`code`) does not inherit text styles - it uses monospace font with code background
2. **Links** do not inherit text styles - they maintain their distinctive link appearance
3. **Very deep nesting** (4+ levels) is technically supported but unusual in practice

## Round-Trip Behavior

When switching between Raw and Rendered modes:
- The markdown source is preserved exactly as written
- Emphasis markers (`*`, `_`, `~`) are not modified during editing
- Visual styling in Rendered mode reflects the source accurately
