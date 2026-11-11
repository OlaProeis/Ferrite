# Rendered Edit — Source Range Replacement

When a rendered-mode block commits its session buffer to `tab.content`, paragraph and list-item paths replace a **line span** in the markdown source via `update_source_range` in `src/markdown/editor.rs`.

**Related:** [Plain paragraphs & lists](./rendered-edit-session-paragraphs-lists.md), [Container prefix model](./rendered-edit-container-prefix.md), [Rendered edit session overview](./rendered-edit-session.md)

## Problem

The span a commit replaces must be the lines the block occupies **now**. The AST is parsed at frame start and rendered commits do not bump `source_epoch` (RS-7), so after the first write of a buffer that gained or lost a line (Shift+Enter soft break), the AST `end_line` is stale for every later write of the same open block — and the live write-through writes every frame.

Two earlier bugs came from guessing the span:

| Guess | Result |
|-------|--------|
| AST `end_line` only | Buffer grew to two lines, one line replaced each frame → duplicated lines (`test\ntest2\ntest2`) |
| `max(ast_end_line, start_line + buffer_lines - 1)` | A grown buffer overwrote the lines **below** the block: Shift+Enter in a paragraph deleted the blank line and then the next paragraph (0.3.1 data loss) |

## Model

`update_source_range(source, start_line, end_line, text) -> rows` replaces **exactly** `start_line..=end_line` and returns the number of lines it wrote. A longer text inserts lines, a shorter one removes them; `end_line == start_line - 1` inserts before `start_line` (a block that currently occupies no line).

The block remembers what it wrote:

| Piece | Role |
|-------|------|
| `BlockEditState::source_rows` | Lines the buffer occupied after its last write, while the block stays open. `None` = never written or closed since → use the AST span |
| `session_block_source_end_line` | `start_line + source_rows - 1` when set, else `ast_end_line_for_session_block` |
| `note_block_written` | Stores `source_rows` after a write and updates the `EditState` node's `end_line` for the rest of the frame |
| `close_active` / `discard_active` / `reset_session_block_if_locked` | Clear `source_rows` (the next open starts from a fresh AST span) |
| `apply_rendered_session_live_frame` | Saves the session when a write changed `source_rows` |
| `rendered_session_pending_text` | Writes a **clone** of the block state into a content copy, so a preview never changes the session |

Commit callbacks take `&mut BlockEditState` for this reason.

Row rules (shared by paragraphs and list items; see [container prefix model](./rendered-edit-container-prefix.md)): the first row gets the first line's container prefix; a continuation row that replaces an existing line keeps that line's own indent and quote markers; rows beyond the old block get `continuation_prefix`.

All line indices are **1-based inclusive**; arithmetic uses `saturating_add` / `saturating_sub`.

## Blocks below a commit that changes the line count

Session buffers (`BlockRef`) are keyed by source line. When a commit adds or removes lines without an epoch bump, every block below moves to a different line and may land on a key that still holds another block's closed buffer.

| Block kind | Handling |
|------------|----------|
| Plain paragraph / list item | Inactive, non-dirty buffer is replaced from source every frame (`cold_text` comparison) |
| Formatted paragraph / list item | Same, in `ensure_formatted_block_initialized` |
| Heading | Heading buffers may legitimately differ from the node's plain text, so they cannot be compared. `RenderedEditSession::note_source_line_breaks` (called once per frame from `MarkdownEditor::show`) drops closed heading buffers when the document's line-break count changes |
| Table | **Not handled.** `TableData` is cached in egui memory under `table` + start line and is not refreshed within an epoch; a table that lands on a line previously used by another table shows that table's data until the epoch changes |

## Tests

Unit tests in `src/markdown/editor.rs`: `test_update_source_range_replaces_exactly_the_given_lines`, `test_update_source_range_grow_stale_single_line_end`, `test_update_source_range_shrink_multi_line_block`, `test_paragraph_soft_newline_commit_roundtrip`, `live_write_through_replaces_only_the_lines_it_wrote`.

Headless end-to-end (`RenderedTabHarness`: real `MarkdownEditor::show` + `finish_rendered_content_frame` frames with pointer and key events): `paragraph_enter_commits_without_breaking_the_line`, `paragraph_soft_break_keeps_the_following_paragraphs`, `soft_breaks_do_not_swap_the_text_of_blocks_below`.

Regression: `cargo test rendered_session::` (RS-1…RS-7).

## Related

- Session flush before view/tab/save/close — [`rendered-edit-flush.md`](./rendered-edit-flush.md).
- Enter/newline semantics in plain paragraphs — [paragraphs & lists](./rendered-edit-session-paragraphs-lists.md).
