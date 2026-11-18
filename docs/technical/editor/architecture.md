# Ferrite Editor Architecture

## Overview

This document defines the architectural principles and constraints for the Ferrite editor component. **All editor features must follow these guidelines** to ensure consistent performance across file sizes from 1KB to 100MB+.

## Core Principles

### 1. Single Source of Truth

**The Rope (`TextBuffer`) is the only authoritative source of content.**

```
┌─────────────────────────────────────────────────────────────┐
│                     Current (WRONG)                         │
├─────────────────────────────────────────────────────────────┤
│  Tab.content: String                         (file size)    │
│  pending_undo_snapshot: String               (file size)    │
│  FerriteEditor.buffer: Rope                  (~1.2× file)   │
│                                                             │
│  Measured 2026-11-12: 82MB CSV, Raw view, ~50 → ~370MB      │
│  working set. Rendered view does not free the rope.        │
│  v0.3.2: step 1 drops the snapshot; step 2 keeps the rope. │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                     Target (CORRECT)                        │
├─────────────────────────────────────────────────────────────┤
│  FerriteEditor.buffer: Rope (80-100MB)  ← Single source     │
│       ↓ on-demand (save only)                               │
│  String conversion                                          │
│                                                             │
│  Total: ~100MB for an 80MB file (rope overhead only)        │
└─────────────────────────────────────────────────────────────┘
```

**Rules:**
- `Tab.content: String` should be removed or made optional (only for non-Ferrite mode)
- All content reads go through `TextBuffer` methods
- String conversion (`to_string()`) only happens on:
  - File save
  - Export operations
  - Clipboard copy of entire document
- **NEVER** call `buffer.to_string()` in per-frame code

### 2. Complexity Tiers

Every operation must fall into one of these tiers:

| Tier | Complexity | When Allowed | Examples |
|------|------------|--------------|----------|
| **O(1)** | Constant | Always, any context | `line_count()`, `is_dirty()`, cursor position |
| **O(log N)** | Logarithmic | Always, any context | Rope index lookups, `get_line(idx)` |
| **O(visible)** | Proportional to viewport | Per-frame rendering | Syntax highlighting, line rendering |
| **O(window)** | Small window around cursor | Per-frame, cursor-dependent | Bracket matching (±100 lines) |
| **O(N)** | Linear in file size | User-initiated only | Find all, save, export |

**Per-frame operations MUST be O(1), O(log N), O(visible), or O(window).**

**O(N) operations are ONLY allowed when:**
- User explicitly triggers them (Find All, Save, Export)
- They run in a background thread with progress indication
- They can be cancelled

### 3. Viewport-Aware Processing

All rendering features must only process visible content plus a small buffer.

```rust
// WRONG: Process entire file
for line_idx in 0..total_lines {
    render_line(line_idx);
}

// CORRECT: Process only visible lines
let (start, end) = view.get_visible_line_range(total_lines);
for line_idx in start..end {
    render_line(line_idx);
}
```

**Viewport buffer sizes:**
- Rendering: visible lines only
- Syntax highlighting: visible lines + 10 lines buffer (for scroll smoothness)
- Bracket matching: cursor line ± 100 lines (configurable)
- Auto-complete: cursor line ± 50 lines for context

### 4. No Per-Frame Allocations

Per-frame code (60fps) must not allocate memory proportional to file size.

```rust
// WRONG: 80MB allocation every frame
let content = self.buffer.to_string();
let matcher = DelimiterMatcher::new(&content);

// CORRECT: Work with rope slices or bounded windows
let cursor_line = self.cursor.line;
let search_start = cursor_line.saturating_sub(100);
let search_end = (cursor_line + 100).min(total_lines);
// Only extract the window we need
```

**Allowed per-frame allocations:**
- Fixed-size buffers (e.g., 64KB scratch buffer)
- Visible line content (typically < 10KB)
- Small Vec for visible matches (capped at 1000 items)

---

## Module Architecture

### Modular `impl` Pattern

FerriteEditor uses Rust's ability to split `impl` blocks across files. The struct is defined in `editor.rs`, but methods are distributed across logical modules:

```
src/editor/ferrite/
├── editor.rs       # struct FerriteEditor { ... } + ui() + core methods
├── selection.rs    # impl FerriteEditor { render_selection(), select_all(), ... }
├── highlights.rs   # impl FerriteEditor { render_search_highlights(), ... }
├── find_replace.rs # impl FerriteEditor { replace_current_match(), ... }
├── mouse.rs        # impl FerriteEditor { pos_to_cursor(), ... }
└── search.rs       # impl FerriteEditor { search_matches(), set_search_matches(), ... }
```

**Benefits:**
- Each file stays focused and <500 lines
- Related functionality grouped together
- Tests live with their implementations
- Clear extension points for new features

**Pattern:**
```rust
// In selection.rs
use super::editor::FerriteEditor;

impl FerriteEditor {
    pub fn select_all(&mut self) { ... }
    pub(crate) fn render_selection(&self, ...) { ... }
}
```

Fields must be `pub(crate)` for sibling modules to access them.

### TextBuffer (Rope Wrapper)

The `TextBuffer` struct wraps `ropey::Rope` and provides efficient access methods.

```rust
impl TextBuffer {
    // O(1) operations
    pub fn line_count(&self) -> usize;
    pub fn len_chars(&self) -> usize;
    pub fn len_bytes(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    
    // O(log N) operations - safe for per-frame use
    pub fn get_line(&self, idx: usize) -> Option<Cow<str>>;
    pub fn char_to_line(&self, char_idx: usize) -> usize;
    pub fn line_to_char(&self, line_idx: usize) -> usize;
    pub fn byte_to_line(&self, byte_idx: usize) -> usize;
    pub fn byte_to_char(&self, byte_idx: usize) -> usize;
    
    // O(window) operations - use with bounded range
    pub fn slice(&self, start_char: usize, end_char: usize) -> RopeSlice;
    pub fn lines_in_range(&self, start_line: usize, end_line: usize) -> impl Iterator;
    
    // O(N) operations - user-initiated only, NEVER per-frame
    pub fn to_string(&self) -> String;  // Mark with #[doc(hidden)] or rename
}
```

### ViewState (Viewport Management)

Manages what's visible and provides viewport bounds to other systems.

```rust
impl ViewState {
    // Returns (start_line, end_line) for the current viewport
    pub fn get_visible_line_range(&self, total_lines: usize) -> (usize, usize);
    
    // Returns extended range for features that need lookahead
    pub fn get_extended_range(&self, total_lines: usize, buffer: usize) -> (usize, usize);
    
    // Scroll management
    pub fn scroll_to_line(&mut self, line: usize);
    pub fn ensure_line_visible(&mut self, line: usize, total_lines: usize);
}
```

### Feature Modules

Each feature module must document its complexity and follow these patterns:

#### Syntax Highlighting
```rust
// Per-frame: O(visible)
pub fn highlight_visible_lines(
    buffer: &TextBuffer,
    visible_range: (usize, usize),
    language: &str,
) -> Vec<HighlightedLine>;
```

#### Bracket Matching
```rust
// Per-frame: O(window) where window = cursor ± MAX_SEARCH_DISTANCE
const MAX_SEARCH_DISTANCE: usize = 100; // lines

pub fn find_matching_bracket(
    buffer: &TextBuffer,
    cursor_line: usize,
    cursor_col: usize,
    total_lines: usize,
) -> Option<BracketPair> {
    let search_start = cursor_line.saturating_sub(MAX_SEARCH_DISTANCE);
    let search_end = (cursor_line + MAX_SEARCH_DISTANCE).min(total_lines);
    // Only search within this window
}
```

#### Search/Find
```rust
// User-initiated: O(N) but runs async with progress
pub async fn find_all(
    buffer: &TextBuffer,
    pattern: &str,
    progress: impl Fn(f32),
) -> Vec<SearchMatch>;

// Per-frame rendering: O(visible) - only render visible matches
pub fn render_visible_matches(
    matches: &[SearchMatch],
    visible_range: (usize, usize),
) -> Vec<HighlightRect>;
```

---

## Data Flow

### Content Lifecycle

```
File Open:
  disk → bytes → encoding detection → String → Rope
                                      ↓
                                   (discard String after Rope creation)

Editing:
  keypress → Rope.insert/delete → mark dirty → UI updates from Rope

File Save:
  Rope.to_string() → encoding → bytes → disk
  (only allocation point for full content)
```

### Tab ↔ Editor Relationship

**Current (problematic):**
```
Tab {
    content: String,        // 80MB - redundant
    cursor_position: (usize, usize),
    ...
}

FerriteEditor {
    buffer: TextBuffer,     // 80MB - authoritative
    cursor: Cursor,
    ...
}

// Sync happens every frame - expensive
```

**Target:**
```
Tab {
    editor_id: usize,       // Reference to editor in storage
    // No content field!
    file_path: Option<PathBuf>,
    is_modified: bool,
    ...
}

FerriteEditorStorage {
    editors: HashMap<usize, FerriteEditor>,
}

FerriteEditor {
    buffer: TextBuffer,     // Single source of truth
    cursor: Cursor,
    view: ViewState,
    ...
}

// Tab only stores metadata, editor owns content
```

---

## Memory Budget

### Per-File Memory

| Component | Size | Notes |
|-----------|------|-------|
| Rope content | 1.0-1.2x file size | Rope has ~10-20% overhead |
| Rope metadata | ~50 bytes per line | Tree nodes |
| Line cache | Fixed 1000 entries | LRU cache for galleys |
| Search matches | Capped at 1000 | ~24 bytes each |
| Undo history | Configurable | Reduced for large files |

**Target (v0.3.2 step 2):** one rope, about 100–130MB of buffer for an 80MB file, on top of the idle process. Not 1× the file, and not the ~370MB working set measured in Raw on 2026-11-12. See [`prd-v0.3.2-large-file-memory-step1.md`](../../ai-workflow/prds/prd-v0.3.2-large-file-memory-step1.md) and [`prd-v0.3.2-large-file-memory-step2.md`](../../ai-workflow/prds/prd-v0.3.2-large-file-memory-step2.md).

### Per-Frame Budget

| Operation | Max Allocation | Notes |
|-----------|----------------|-------|
| Line rendering | 100KB | Visible lines only |
| Syntax segments | 50KB | Visible lines only |
| Search highlights | 24KB | 1000 matches × 24 bytes |
| Bracket matching | 1KB | Single pair |
| **Total per frame** | **<500KB** | |

---

## Migration Plan

### Phase 1: Audit Current Code ✅
- [x] List all places that call `buffer.to_string()` or `tab.content`
- [x] Classify each as O(1), O(visible), O(N)
- [x] Identify per-frame O(N) violations

### Phase 2: Fix Per-Frame Violations ✅
- [x] Bracket matching: implement windowed search (cursor ±100 lines)
- [x] Content sync uses `is_content_dirty()` flag, not string comparison
- [x] Syntax highlighting: viewport-aware, per-line caching
- [x] Search highlights: capped at 1000 matches

### Phase 3: Remove Content Duplication ✅ (v0.2.6)
- [x] `Tab.content` still exists but is synced lazily (not per-frame)
- [x] Save/export get content from `Tab.content` (synced from editor buffer)
- [x] FerriteEditor.buffer is the authoritative source during editing
- [ ] *Future:* Make `Tab.content` optional for full memory optimization

### Phase 4: Validate (v0.2.6, RAM row withdrawn)
- [x] Test with 1MB, 10MB, 50MB, 80MB files
- [ ] RAM usage: ~80MB for 80MB file (1x file size) — **not true.** Withdrawn 2026-11-12. Raw view of `test_md/very_large_test_data.csv` (82MB) sat near 370MB working set. The 1× row was marked done in error.
- [x] Frame time: Smooth 60fps scrolling
- [x] See `docs/archive/release-qa/v0.2.6-manual-test-suite.md` for test coverage

---

## Feature Implementation Checklist

When implementing any new editor feature, verify:

- [ ] **Complexity documented**: What tier is this operation?
- [ ] **Viewport-aware**: Does it only process visible content?
- [ ] **No per-frame O(N)**: Does the render path avoid full-file operations?
- [ ] **Bounded allocations**: Are allocations capped or fixed-size?
- [ ] **Rope-native**: Does it use rope methods, not `to_string()`?
- [ ] **Tested at scale**: Verified with 50MB+ file?

---

## Anti-Patterns to Avoid

### 1. Full Content Conversion
```rust
// NEVER in per-frame code
let text = self.buffer.to_string();
```

### 2. Unbounded Iteration
```rust
// WRONG
for line in 0..self.buffer.line_count() {
    process(line);
}

// CORRECT
let (start, end) = self.view.get_visible_line_range(total);
for line in start..end {
    process(line);
}
```

### 3. Per-Frame Cloning
```rust
// WRONG
let content_copy = tab.content.clone(); // 80MB clone at 60fps = 4.8GB/s

// CORRECT
let is_modified = editor.is_dirty(); // O(1) flag check
```

### 4. Disabling Features Instead of Fixing
```rust
// WRONG approach (what we've been doing)
if is_large_file {
    return; // Just disable the feature
}

// CORRECT approach
let window = get_bounded_window(cursor, MAX_DISTANCE);
process_within_window(window); // Works for any file size
```

---

## Related Documents

- [TextBuffer Implementation](text-buffer.md)
- [ViewState Implementation](view-state.md)
- [Line Cache Strategy](line-cache.md)
- [Large File Performance Fixes](large-file-performance.md) (legacy, to be superseded)
