# Undo/Redo System

## Overview

Ferrite implements a per-tab undo/redo system that tracks content changes and allows users to navigate through their editing history using keyboard shortcuts.

## Architecture

### Per-Tab State

Each `Tab` maintains its own independent undo/redo history:

```rust
struct Tab {
    // ... other fields
    undo_stack: Vec<String>,     // Stack of previous content states
    redo_stack: Vec<String>,     // Stack of undone states for redo
    max_undo_size: usize,        // Maximum history size (default: 100)
}
```

### Storage Model

The system uses **full content snapshots** rather than incremental deltas:
- **Pros**: Simple, reliable, works with any edit type
- **Cons**: Higher memory usage for large documents
- **Trade-off**: For typical markdown documents (<100KB), memory impact is minimal

### Maximum History

The `max_undo_size` limits the undo stack to 100 entries by default. When exceeded, the oldest entries are removed (FIFO).

## Implementation Details

### Recording Edits

Because egui's `TextEdit` modifies content directly (bypassing `Tab::set_content()`), a separate method records edits:

```rust
impl Tab {
    /// Record an edit after TextEdit modifies content directly.
    /// Call with the OLD content (before the edit).
    pub fn record_edit(&mut self, old_content: String) {
        if old_content != self.content {
            self.undo_stack.push(old_content);
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();  // New edits invalidate redo
        }
    }
}
```

### EditorWidget Integration

The `EditorWidget` captures content before showing `TextEdit`, then records if changed:

```rust
// Before TextEdit
let original_content = self.tab.content.clone();

// Show TextEdit (may modify content)
let text_output = text_edit.show(ui);

// After TextEdit - record for undo if changed
if self.tab.content != original_content {
    self.tab.record_edit(original_content);
}
```

### Content Version for External Changes

egui's `TextEdit` maintains internal state keyed by widget ID. When content changes externally (via undo/redo), the `TextEdit` doesn't automatically detect this. The solution is a `content_version` counter:

```rust
struct Tab {
    // ... other fields
    content_version: u64,  // Incremented on undo/redo
}
```

The `EditorWidget` includes this version in its ID:

```rust
let base_id = self.id.unwrap_or_else(|| ui.id().with("editor"));
let id = base_id.with(self.tab.content_version());
```

When `undo()` or `redo()` is called, the version increments, causing egui to treat it as a new widget and re-read the content from the source string.

### Undo/Redo Operations

```rust
impl Tab {
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = previous;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            true
        } else {
            false
        }
    }
}
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo last edit |
| `Ctrl+Y` | Redo undone edit |
| `Ctrl+Shift+Z` | Redo undone edit (alternative) |

## User Feedback

The system provides visual feedback via toast notifications:
- **Successful undo**: "Undo (N remaining)" where N is remaining undo count
- **Successful redo**: "Redo (N remaining)" where N is remaining redo count
- **Empty stack**: "Nothing to undo" or "Nothing to redo"

Toast messages display for 1.5 seconds.

## Behavior Notes

### Redo Stack Clearing

The redo stack is **cleared** whenever a new edit is made. This is standard behavior - you cannot redo after making new changes:

```
Initial: "Hello"
Edit:    "Hello World"  → undo_stack: ["Hello"]
Undo:    "Hello"        → redo_stack: ["Hello World"]
Edit:    "Hello!"       → redo_stack: CLEARED
```

### Tab Independence

Each tab maintains completely independent undo/redo history:
- Switching tabs preserves each tab's history
- Closing a tab discards its history
- Opening a file starts with empty history

### Save Interaction

Saving a file does **not** clear the undo history. You can still undo after saving.

## Testing

Unit tests cover:
- Basic undo/redo operations (`test_tab_undo_redo`)
- `record_edit` for external modifications (`test_tab_record_edit`)
- Redo clearing on new edit (`test_tab_undo_clears_redo_on_edit`, `test_tab_record_edit_clears_redo`)
- Stack count tracking (`test_tab_undo_redo_counts`)
- Maximum size enforcement (`test_tab_max_undo_size`)
- No-op for unchanged content (`test_tab_record_edit_no_change`)

## Future Considerations

Potential improvements for future versions:
1. **Delta-based storage**: Store diffs instead of full content for memory efficiency
2. **Word-boundary grouping**: Group character-by-character edits into word-level undo entries
3. **UI buttons**: Add undo/redo buttons to toolbar (planned for Ribbon UI - Task 41)
4. **Persistent undo**: Save undo history to disk for session restoration
