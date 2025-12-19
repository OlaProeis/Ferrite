# Text Statistics

This document describes the text statistics feature that displays real-time word, character, and line counts in the status bar.

## Overview

The text statistics feature provides instant feedback about document metrics as users type. Statistics are displayed in the status bar, updating in real-time whenever the content changes.

## Architecture

### Module Structure

```
src/editor/
├── mod.rs           # Exports TextStats
├── stats.rs         # TextStats struct and counting algorithms
├── line_numbers.rs  # Reused count_lines() function
└── widget.rs        # Editor widget
```

### TextStats Struct

The `TextStats` struct holds all document metrics:

```rust
pub struct TextStats {
    pub words: usize,              // Word count
    pub characters: usize,         // Characters including spaces
    pub characters_no_spaces: usize, // Characters excluding spaces
    pub lines: usize,              // Line count
    pub paragraphs: usize,         // Paragraph count
}
```

## Counting Algorithms

### Word Count

Words are defined as sequences of non-whitespace characters separated by whitespace:

```rust
text.split_whitespace().count()
```

Note: This means Chinese/Japanese text without spaces between words will be counted as fewer words than expected.

### Character Count

Two counts are provided:
- **With spaces**: Total character count including all whitespace
- **Without spaces**: Characters excluding whitespace

### Line Count

Uses the existing `count_lines()` function from `line_numbers.rs`:
- Empty text = 1 line
- Non-empty text = number of newline characters + 1

### Paragraph Count

Paragraphs are non-empty text blocks separated by blank lines:
- Consecutive non-blank lines are one paragraph
- One or more blank lines separate paragraphs

## Integration

### Status Bar Display

Statistics appear in the status bar alongside the cursor position:

```
Ready          150 words | 892 chars | 25 lines     Ln 1, Col 1
```

### Real-Time Updates

Statistics are recalculated on every frame for the active tab's content. The single-pass algorithm ensures minimal performance impact.

## API

### Creating Stats

```rust
use crate::editor::TextStats;

let stats = TextStats::from_text("Hello, World!");
println!("{}", stats.format_compact()); // "2 words | 13 chars | 1 lines"
```

### Formatting Options

- `format_compact()` - Concise: "150 words | 892 chars | 25 lines"
- `format_detailed()` - Full: "150 words | 892 chars (743 no spaces) | 25 lines | 5 paragraphs"

### Helper Functions

Standalone functions for individual metrics:

```rust
use crate::editor::stats::{count_words, count_characters, count_paragraphs};

let words = count_words("Hello World"); // 2
let chars = count_characters("Hello World"); // 11
let paragraphs = count_paragraphs("Para 1\n\nPara 2"); // 2
```

## Performance

The `TextStats::from_text()` method uses a single-pass algorithm that:
1. Iterates through each character once
2. Tracks word boundaries
3. Tracks paragraph boundaries
4. Counts all metrics simultaneously

This ensures O(n) time complexity where n is the text length.

## Testing

31 unit tests cover:
- Empty text handling
- Single word/line/paragraph cases
- Multiple paragraphs
- Unicode text
- Whitespace-only text
- Edge cases (trailing newlines, mixed whitespace)
- Formatting functions

Run tests:
```bash
cargo test stats
```

## Future Enhancements

Potential improvements:
- Reading time estimate (words per minute)
- Selection-based statistics
- CJK word segmentation for accurate Asian language word counts
- Sentence count
- Average word length
