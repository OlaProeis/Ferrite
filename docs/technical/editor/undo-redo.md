# Undo/Redo System

## Overview

Ferrite uses a **unified, operation-based** undo/redo system. A single `EditHistory` instance per tab records minimal diffs (insert/delete operations) rather than full content snapshots. Raw mode (`EditorWidget` / FerriteEditor), rendered mode (`MarkdownEditor`), tree viewer, and legacy toolbar paths all record into the same `tab.edit_history`.

## Architecture

### Per-Tab State

Each `Tab` owns one `EditHistory`:

```rust
struct Tab {
    edit_history: EditHistory,       // Operation-based undo/redo
    content_version: u64,            // Bumped on undo/redo for widget sync
    pending_undo_snapshot: Option<String>,  // Baseline for diff-based recording
    undo_content_hash: [u8; 32],   // Blake3 digest for snapshot elision
    pending_cursor_restore: Option<usize>,  // Cursor char index (set on undo; see limitations)
}
```

### Storage Model

Operations store only the changed text, not full document copies:

| Metric (4 MB file, 100 edits) | Old (snapshots) | New (operations) |
|-------------------------------|-----------------|-----------------|
| Memory usage                  | ~400 MB         | ~2 KB           |
| Per-edit cost                 | O(n) clone      | O(diff) ≈ small |

### Maximum History

Default: **500** undo groups. Large files (≥ 1 MB): **200** groups. Each group is tiny (a few bytes to a few KB of changed text), so even 500 groups use negligible memory.

## Recording Flow

### Diff-based path (raw + rendered + tree)

```rust
// Before editor.show():
tab.prepare_undo_snapshot_hashed();  // Clone only when content hash changed

// After editor.show() if content changed:
tab.record_edit_from_snapshot();  // Diff snapshot vs current → record_operations
```

`prepare_undo_snapshot_hashed()` avoids cloning every frame — see [Undo Hash Change Detection](./undo-hash-change-detection.md).

**Raw mode:** `EditorWidget::show` calls the same snapshot + diff path when FerriteEditor reports `is_content_dirty()` and syncs the rope into `tab.content`.

**Rendered / split preview / tree viewer:** `central_panel.rs` calls the same APIs when `MarkdownEditor` or `TreeViewer` reports `changed`.

### Diff Algorithm

`compute_edit_ops(old, new)` uses prefix/suffix matching to find the minimal changed region:

1. Find common prefix (char-by-char from start)
2. Find common suffix (char-by-char from end)
3. Emit Delete for removed text, Insert for added text

This is O(n) worst case but near-instant for typical single-point edits.

### Legacy Recording

Some code paths (formatting, line operations, paste helpers) still use:

```rust
let old_content = tab.content.clone();
tab.content = new_content;
tab.record_edit(old_content, cursor_pos);
```

This computes the same diff internally via `record_operations`.

## Undo/Redo Operations

```rust
impl Tab {
    pub fn undo(&mut self) -> Option<usize> {
        // Applies inverse ops to self.content via edit_history.undo_string()
        // Bumps content_version, refreshes undo snapshot/hash
        // Returns cursor char position from the first op in the group
    }

    pub fn redo(&mut self) -> Option<usize> {
        // Reapplies ops via edit_history.redo_string()
        // Returns cursor char position after the last op in the group
    }
}
```

### Content Version

`content_version` is bumped on every undo/redo and on each recorded edit. `EditorWidget` detects external changes (including undo) via content length/hash and calls `FerriteEditor::set_content()` to re-sync the rope buffer.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` (`Cmd+Z` on macOS) | Undo last edit group |
| `Ctrl+Y` | Redo undone edit group |
| `Ctrl+Shift+Z` | Redo (alternative) |

### Event Consumption

Keys are consumed in `consume_undo_redo_keys()` **before** UI rendering. This prevents egui `TextEdit` built-in undo (rendered blocks) from conflicting with the tab-level stack.

## Operation Grouping (v0.3.0+)

Each call to `EditHistory::record_operations` (or `record_operation`) pushes **one** undo group:

| Source | Typical undo step |
|--------|-------------------|
| Raw typing (per dirty frame) | One group per frame — usually one character, sometimes more if input batches in one frame |
| Replace in diff | Delete + insert in **one** group (atomic) |
| Rendered plain paragraph | Per frame while `TextEdit` reports `changed` |
| Rendered heading / list item | On **focus loss** (or Enter) — whole block edit session |
| Formatting / move line | One group per `record_edit` call |

`record_operations` still creates one group per call. Consecutive single-character inserts merge only when recorded with `record_with_meta` inside 700 ms on the same line. See [undo-coalescing.md](undo-coalescing.md).

`break_undo_group()` on `Tab` / `EditHistory::break_group()` closes that typing run.

## Behavior Notes

- **Redo stack clearing**: New edits clear the redo stack (standard behavior).
- **Tab independence**: Each tab has its own `EditHistory`. Closing a tab discards it.
- **Save interaction**: Saving does not clear undo history.
- **Cursor restoration**: Undo/redo set `pending_cursor_restore` (char index). FerriteEditor cursor sync from this field is not fully wired yet — scroll position is preserved via `pending_scroll_offset`.
- **Scroll preservation**: `handle_undo` / `handle_redo` in `navigation.rs` preserve scroll offset across operations.
- **Rendered undo UI**: After undo, egui edit buffers for in-flight blocks may be stale until focus changes; see rendered commit rules above.

## Testing

Tests in `state.rs` and `history.rs`:

```bash
cargo test undo
cargo test history
```

Tests cover: basic undo/redo, per-call grouping (no time merge), atomic replace batches, max group cap, unicode/emoji, roundtrip diff-undo, redo clearing, extensive operation sequences.

## Related Documentation

- [EditHistory Module](./edit-history.md) — API reference and implementation details
- [Undo Hash Change Detection](./undo-hash-change-detection.md) — Snapshot elision for large files
