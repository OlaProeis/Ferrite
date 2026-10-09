# EditHistory Module

## Overview

`EditHistory` (`src/editor/ferrite/history.rs`) is the **sole undo/redo engine** for the entire application. Each `Tab` in `state.rs` owns one instance. It stores discrete edit operations (insert/delete) rather than full content snapshots, making memory usage proportional to edit size, not file size.

FerriteEditor does **not** embed its own `EditHistory`; the rope buffer is edited in place and changes are diffed into `tab.edit_history` via `EditorWidget`.

## Architecture

### Key Types

```rust
pub enum EditOperation {
    Insert { pos: usize, text: String },
    Delete { pos: usize, text: String },
}

pub struct EditHistory {
    undo_stack: Vec<OperationGroup>,
    redo_stack: Vec<OperationGroup>,
    max_groups: usize,  // Default 500, large files 200
}
```

### Data Flow

```
Editor modifies tab.content (raw sync or rendered commit)
    → prepare_undo_snapshot_hashed() (baseline)
    → tab.record_edit_from_snapshot()
    → compute_edit_ops(old, new) → Vec<EditOperation>
    → edit_history.record_operations(ops)  // one undo group per call

Ctrl+Z
    → input_handling::consume_undo_redo_keys()
    → navigation::handle_undo()
    → tab.undo() → edit_history.undo_string(&mut tab.content)
    → content_version bumped
    → EditorWidget re-syncs FerriteEditor via set_content()
```

## Edit Operations

### Insert

Records text insertion at a char-indexed position. Undo: delete the text. Redo: re-insert it.

### Delete

Records text deletion, storing the removed text. Undo: re-insert the text. Redo: delete it again.

### apply_to_string

Operations can be applied to both `TextBuffer` (rope, tests) and plain `String`:

```rust
op.apply_to_string(&mut s);  // Used by Tab for undo/redo on tab.content
```

Char positions are converted to byte offsets internally via `char_pos_to_byte_pos()`.

## Diff Algorithm

`compute_edit_ops(old, new)` finds the minimal changed region using prefix/suffix matching:

```rust
pub fn compute_edit_ops(old: &str, new: &str) -> Vec<EditOperation> {
    // 1. Find common prefix (chars from start)
    // 2. Find common suffix (chars from end, excluding prefix)
    // 3. Emit Delete for old[prefix..old_len-suffix]
    // 4. Emit Insert for new[prefix..new_len-suffix]
}
```

Returns 0 ops (no change), 1 op (pure insert or delete), or 2 ops (replace = delete + insert in one `record_operations` call).

## Operation Grouping

**v0.3.0+:** Each `record_operations` / `record_operation` call appends **one** `OperationGroup`. Operations inside that call (e.g. delete + insert for a replace) undo together.

`record_operations` does not merge by time. Typing runs merge only through `record_with_meta` when the gap is under 700 ms. See [undo-coalescing.md](undo-coalescing.md).

`break_group()` closes the open typing run so the next record starts a new group.

## API Reference

### EditHistory

| Method | Description |
|--------|-------------|
| `new()` | Create with default 500-group cap |
| `with_max_groups(n)` | Create with custom group cap (200 for large files) |
| `record_operation(op)` | Record one op as its own group (delegates to `record_operations`) |
| `record_operations(ops)` | Record all ops as **one** atomic undo group (no coalesce) |
| `record_with_meta(ops, meta)` | Same, but may merge a single-character typing insert into the open group |
| `undo_string(s)` → `Option<usize>` | Undo on `&mut String`, returns cursor pos |
| `redo_string(s)` → `Option<usize>` | Redo on `&mut String`, returns cursor pos |
| `can_undo()` / `can_redo()` → `bool` | Check availability |
| `undo_count()` / `redo_count()` → `usize` | Stack sizes |
| `break_group()` | Close the open typing run |
| `clear()` | Clear all history |

### EditOperation

| Method | Description |
|--------|-------------|
| `inverse()` | Returns the reverse operation |
| `apply_to_string(s)` | Apply to a `String` (char-indexed) |

### Standalone Functions

| Function | Description |
|----------|-------------|
| `compute_edit_ops(old, new)` | Minimal diff between two strings |

## Memory Efficiency

| Scenario (4 MB file) | Snapshot System | Operation System |
|-----------------------|-----------------|-----------------|
| 100 char inserts      | 100 × 4 MB = 400 MB | 100 × ~20 B = ~2 KB |
| 50 line deletions     | 50 × 4 MB = 200 MB  | 50 × ~80 B = ~4 KB  |
| Mixed 100 operations  | ~300 MB              | ~5 KB                |

## Testing

```bash
cargo test history     # EditHistory unit tests
cargo test undo        # Tab-level undo integration tests
```

Tests cover: basic undo/redo, separate groups per `record_operation`, atomic `record_operations` batches, max group cap, roundtrip diff-undo, extensive sequences (100 ops), large buffer performance (1 MB).

## Related

- [Undo/Redo System](./undo-redo.md) — User-facing behavior and integration
- [Undo Hash Change Detection](./undo-hash-change-detection.md) — Snapshot elision
