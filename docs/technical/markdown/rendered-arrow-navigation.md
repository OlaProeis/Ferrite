# Rendered Arrow Up/Down navigation (#170)

## Problem

In Rendered (and Split preview), ArrowUp / ArrowDown did not move between blocks, so the preview could not be used as a light editor. Raw / Split raw already handled arrows inside the text editor.

## WYSIWYG model

1. **Inside click-to-edit / session TextEdit:** ArrowUp/Down move the caret on visual rows (egui `TextEdit`).
2. **At the first visual row:** ArrowUp leaves to the previous navigable block (caret at end).
3. **At the last visual row:** ArrowDown leaves to the next navigable block (caret at start).
4. **Single-line blocks** (headings, short paragraphs) are always on the first/last row, so Up/Down move between blocks immediately.
5. **Preview lock:** the preview is read-only and has no caret, so there is no block cursor. ArrowUp/Down **scroll** the view by a fixed step (3 × font size per press, clamped to the document). Before QA ISS-76 the lock kept a hidden block selection: a click on a plain block set no anchor, so the first ArrowDown targeted the first block of the document and jumped the view to the top.
6. **Tables / CSV:** keep their own cell arrow handling — skip when `BlockRef::TableCell` is active.
7. **Raw / Split raw:** unchanged. Callers only consume keys when the rendered pane owns keyboard focus (or the pointer is over the preview with no conflicting focus).

## Navigable blocks

`collect_navigable_blocks` walks the AST once per frame (O(nodes), no layout) and lists session-backed blocks in document order:

- Headings
- Plain / formatted paragraphs
- List items (plain + formatted), including nested lists

`FormattedListItem.item` is the **0-based child index** (same as plain `ListItem`), not the 1-based ordered display number.

Code / mermaid / HR / video / tables are not in this list.

## Runtime wiring (`editor.rs`)

| Mechanism | Role |
|-----------|------|
| `queue_boundary_arrow_nav_if_needed` | Before `TextEdit::show`, if focused caret is on first/last visual row (galley row cache, else char-index fallback), consume Up/Down and queue leave |
| `store_arrow_nav_row_cache` | After paint, cache `(row_count, cursor_row)` for the next keypress |
| `apply_pending_arrow_navigation` | End of rendered frame (not called under preview lock): apply queued leave or container-level Up/Down; activate adjacent block; scroll via culling Y. Logs `Rendered arrow nav <dir>: line <from> -> line <to>` at debug level for each block move |
| `take_locked_preview_arrow_presses` + `locked_preview_arrow_scroll_target` | Preview lock only, after the scroll area: consume Up/Down and store the next scroll offset in the nav-button scroll target (applied next frame, logged as `Applied nav button scroll offset in rendered mode`) |
| `rendered_session_caret_position` / `rendered_caret_source_position` | Map the active block's caret to a 0-indexed source `(line, column)` for the status bar (see below) |
| `handoff_arrow_nav_after_edit_exit` | After Enter/Escape exit: remember last block + `request_focus(display_sense)` so Up/Down continue without pointer over preview |
| `arrow_nav_sel_key` | Last navigable block temp data scoped per `rendered_editor_id(tab.id)` — not a global id; cleared in `cleanup_rendered_editor_memory` on tab close |
| Split-safe ownership | Never steal keys when Raw/find has focus, or when the pointer is clearly over another pane |

Scroll-into-view uses `scroll_y_for_source_line` on viewport-culling `block_line_ranges` / `block_start_y` (no full-document layout per key).

## Status bar position (QA ISS-77)

In Rendered-only mode the status bar `Ln` / `Col` follow the caret of the block being edited. `MarkdownEditorOutput::cursor_position` is `Option<(line, column)>`, 0-indexed source coordinates:

- Buffer rows map 1:1 onto source lines from the block's start line.
- The column is shifted by what the edit buffer omits from that source line: the quote / list prefix, plus the ATX `#` marker on a heading's first row. Columns count chars.
- Headings with inline markup are approximate, because their buffer holds plain text.
- `None` when no block holds a caret, or the caret is in a table cell or code block (their text lives outside the session buffers). The caller then keeps `tab.cursor_position` as it was, so the status bar shows the last known position instead of resetting to `Ln 1, Col 1`.

Split mode is unchanged: the raw pane owns `tab.cursor_position` there.

## Counting presses (QA T1-17 / ISS-78)

Code fences are not navigable, so ArrowDown from the block above a fence lands on the block below it. A multi-row paragraph takes one press per visual row before the caret leaves it, and the row count depends on the window width. In `test_md/test_find_arrow_nav.md` the caret needs 15 block moves from `## Section 1` (line 8) to the multi-line paragraph (line 49), plus one press per extra wrapped row on the way (5 at the QA window size, so 20 presses). Verify with the `Rendered arrow nav` debug lines or by typing a marker; a single screenshot can miss the blinking caret.

## Key files

- `src/markdown/rendered_arrow_nav.rs` — pure helpers + unit tests
- `src/markdown/editor.rs` — queue / apply / handoff / TextEdit integration
- `src/markdown/rendered_session.rs` — `BlockRef`, session activate/commit

## Tests

Module `markdown::rendered_arrow_nav::tests`:

- `adjacent_*`, `char_fallback_boundaries`
- `collect_multi_paragraph_and_heading`, `collect_list_items_in_order`
- `collect_formatted_list_item_uses_child_index`
- `scroll_y_maps_contained_line`
- `qa_fixture_arrow_nav_steps_over_fenced_code_block` — block order of `test_find_arrow_nav.md`; the fence at lines 36–42 is skipped

Module `markdown::editor::tests`:

- `rendered_caret_source_position_*`, `rendered_session_caret_position_reads_active_block_text_edit_state` — status bar mapping
- `locked_preview_arrow_scroll_target_steps_and_clamps`, `locked_preview_arrow_keys_scroll_the_view` — preview-lock scrolling (headless frame loop)

## Manual QA

Checklist row **UX-12** (`UX-arrow-nav`) in [`archive/v0.3.1-test-checklist.md`](../platform/archive/v0.3.1-test-checklist.md) (merged into [`v0.3.1-total-test.md`](../platform/v0.3.1-total-test.md)).
