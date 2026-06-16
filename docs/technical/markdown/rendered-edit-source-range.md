# Rendered Edit — Source Range Replacement

When a rendered-mode block commits its session buffer to `tab.content`, paragraph and list-item paths replace a **line span** in the markdown source via `update_source_range` in `src/markdown/editor.rs`.

**Related:** [Plain paragraphs & lists](./rendered-edit-session-paragraphs-lists.md), [Rendered edit session overview](./rendered-edit-session.md)

## Problem

Commit used `end_line` from the AST parsed at **frame start**. After Enter grows the session buffer to multiple lines, that `end_line` can still describe a single source line. `update_source_range` then:

1. Writes all lines from the committed buffer.
2. Keeps every source line **after** the stale `end_line`.

If line 2 already held content (or duplicated buffer text), the result was duplicated lines (e.g. `test\ntest2\ntest2`).

## Fix (option (a) — span merge, no epoch bump)

Rendered commits still do **not** bump `source_epoch` (RS-7). Span math is corrected instead of cold-reloading the session via `invalidate_buffers`.

| Function | Role |
|----------|------|
| `committed_block_line_count` | Line count from committed text (`lines()`; trailing newline ignored). |
| `block_replace_end_line` | `max(ast_end_line, start_line + committed_lines - 1)` — grow replaces trailing old lines; shrink still clears the full AST span. |
| `update_source_range` | Uses `effective_end = max(end_line, content_end)` when dropping trailing source lines. |
| `write_session_block_to_source` | Computes `end_line` via `block_replace_end_line` before `update_source_range`. |
| `mark_block_modified` | Updates `EditState` node `end_line` after commit for intra-frame consistency. |

All line indices are **1-based inclusive**; arithmetic uses `saturating_add` / `saturating_sub`.

## Tests

Unit tests in `src/markdown/editor.rs` (`test_update_source_range_grow_*`, `test_update_source_range_shrink_multi_line_block`, `test_block_replace_end_line_grow_and_shrink`).

Regression: `cargo test rendered_session::` (RS-1…RS-7).

## Related

- Session flush before view/tab/save/close — [`rendered-edit-flush.md`](./rendered-edit-flush.md).
- Enter/newline semantics in plain paragraphs — task 3 (see [paragraphs & lists](./rendered-edit-session-paragraphs-lists.md)).
