# Markdown Parser Module

The markdown parser module (`src/markdown/`) provides markdown parsing and HTML rendering functionality using the [comrak](https://crates.io/crates/comrak) library, a CommonMark + GitHub Flavored Markdown (GFM) compatible parser written in Rust.

## Overview

This module wraps comrak's parsing functions to provide:

1. **Parse markdown to AST** - Convert raw markdown text into an Abstract Syntax Tree
2. **Render to HTML** - Convert markdown to HTML for preview/display
3. **Document analysis** - Extract headings, code blocks, links, and images

## Public API

### Core Functions

```rust
use crate::markdown::{parse_markdown, render_to_html, MarkdownDocument};

// Parse markdown to AST
let doc: MarkdownDocument = parse_markdown("# Hello\n\nWorld")?;

// Render markdown to HTML
let html: String = render_to_html("**bold** text")?;
```

### With Custom Options

```rust
use crate::markdown::{parse_markdown_with_options, render_to_html_with_options, MarkdownOptions};

// Use minimal CommonMark (no GFM extensions)
let options = MarkdownOptions::minimal();
let doc = parse_markdown_with_options(markdown, &options)?;

// Use GitHub Flavored Markdown (default)
let options = MarkdownOptions::gfm();
let html = render_to_html_with_options(markdown, &options)?;
```

## Types

### `MarkdownOptions`

Configuration for parsing and rendering:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tables` | `bool` | `true` | Enable GFM tables |
| `strikethrough` | `bool` | `true` | Enable ~~strikethrough~~ |
| `autolink` | `bool` | `true` | Auto-link URLs and emails |
| `tasklist` | `bool` | `true` | Enable task lists `- [ ]` |
| `superscript` | `bool` | `false` | Enable ^superscript^ |
| `footnotes` | `bool` | `true` | Enable footnotes |
| `description_lists` | `bool` | `false` | Enable description lists |
| `front_matter_delimiter` | `Option<String>` | `Some("---")` | YAML front matter |
| `safe_urls` | `bool` | `true` | Remove dangerous protocols |
| `header_ids` | `Option<String>` | `Some("")` | Generate heading IDs |

### `MarkdownDocument`

A parsed document containing the AST:

```rust
pub struct MarkdownDocument {
    pub root: MarkdownNode,        // Root AST node
    pub source: String,            // Original source text
    pub front_matter: Option<String>, // Extracted front matter
}
```

#### Document Methods

```rust
// Get all headings in document order
let headings: Vec<&MarkdownNode> = doc.headings();

// Get all code blocks
let code_blocks: Vec<&MarkdownNode> = doc.code_blocks();

// Get all links
let links: Vec<&MarkdownNode> = doc.links();

// Get all images
let images: Vec<&MarkdownNode> = doc.images();

// Check if document is empty
let empty: bool = doc.is_empty();
```

### `MarkdownNode`

An AST node with position information:

```rust
pub struct MarkdownNode {
    pub node_type: MarkdownNodeType,
    pub children: Vec<MarkdownNode>,
    pub start_line: usize,   // 1-indexed
    pub start_column: usize, // 1-indexed
    pub end_line: usize,
    pub end_column: usize,
}
```

#### Node Methods

```rust
// Get all text content from node and descendants
let text: String = node.text_content();

// Check node classification
let is_block: bool = node.is_block();
let is_inline: bool = node.is_inline();
```

### `MarkdownNodeType`

All supported node types:

**Block Elements:**
- `Document` - Root node
- `BlockQuote` - Block quote (`>`)
- `List { list_type, tight }` - Ordered/unordered list
- `Item` - List item
- `CodeBlock { language, info, literal }` - Fenced/indented code
- `HtmlBlock(String)` - Raw HTML block
- `Paragraph` - Paragraph
- `Heading { level, setext }` - H1-H6 heading
- `ThematicBreak` - Horizontal rule (`---`)
- `Table { alignments, num_columns }` - GFM table
- `TableRow { header }` - Table row
- `TableCell` - Table cell
- `FootnoteDefinition(String)` - Footnote definition
- `DescriptionList`, `DescriptionItem`, `DescriptionTerm`, `DescriptionDetails`
- `FrontMatter(String)` - YAML/TOML front matter

**Inline Elements:**
- `Text(String)` - Plain text
- `Code(String)` - Inline code
- `HtmlInline(String)` - Inline HTML
- `Emphasis` - Italic (`*text*`)
- `Strong` - Bold (`**text**`)
- `Strikethrough` - Strikethrough (`~~text~~`)
- `Superscript` - Superscript (`^text^`)
- `Link { url, title }` - Link
- `Image { url, title }` - Image
- `SoftBreak` - Soft line break
- `LineBreak` - Hard line break
- `TaskItem { checked }` - Task list marker
- `FootnoteReference(String)` - Footnote reference

### Supporting Types

```rust
// Heading levels H1-H6
pub enum HeadingLevel { H1, H2, H3, H4, H5, H6 }

// List types
pub enum ListType {
    Bullet,
    Ordered { start: u32, delimiter: char },
}

// Table alignment
pub enum TableAlignment { None, Left, Center, Right }
```

## Error Handling

The module uses the centralized error types from `error.rs`:

```rust
// Error::MarkdownParse - Parsing failures
Error::MarkdownParse { message: String }

// Error::MarkdownRender - Rendering failures  
Error::MarkdownRender { message: String }
```

Both are marked as recoverable errors (app can continue with defaults).

## Example: Full Document Analysis

```rust
use crate::markdown::{parse_markdown, MarkdownNodeType, HeadingLevel};

let markdown = r#"---
title: My Document
---

# Introduction

Some text with **bold** and *italic*.

## Code Example

```rust
fn main() {}
```

- Item 1
- Item 2
"#;

let doc = parse_markdown(markdown)?;

// Check for front matter
if let Some(fm) = &doc.front_matter {
    println!("Front matter: {}", fm);
}

// List all headings
for heading in doc.headings() {
    if let MarkdownNodeType::Heading { level, .. } = &heading.node_type {
        println!("H{}: {}", *level as u8, heading.text_content());
    }
}

// List code blocks with languages
for block in doc.code_blocks() {
    if let MarkdownNodeType::CodeBlock { language, literal, .. } = &block.node_type {
        println!("Code ({}): {} chars", language, literal.len());
    }
}
```

## Integration with WYSIWYG Editor

The AST structure is designed to support the WYSIWYG editor widget (Task 20-21):

1. **Parse** - Convert user's markdown to AST
2. **Render widgets** - Each AST node maps to an egui widget
3. **Edit** - User modifies widgets (headings, lists, code blocks)
4. **Serialize** - Convert modified AST back to markdown

The position information (`start_line`, `start_column`, etc.) enables:
- Cursor positioning in raw mode
- Syntax highlighting ranges
- Error location reporting

## Tests

The module includes comprehensive tests covering:

- Basic parsing (paragraphs, headings, lists)
- GFM extensions (tables, task lists, strikethrough)
- Code blocks (fenced, indented, with language)
- Inline elements (bold, italic, code, links, images)
- Block elements (blockquotes, horizontal rules)
- Front matter parsing
- HTML rendering
- Options configuration
- Error handling for malformed input
- Position tracking

Run tests:
```bash
cargo test markdown
```

## Dependencies

- `comrak` 0.22 - CommonMark + GFM parser (already in Cargo.toml)
