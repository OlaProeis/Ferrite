# Rendered Arrow Up/Down navigation (#170)

## Problem

In Rendered (and Split preview), ArrowUp / ArrowDown did not move between blocks, so the preview could not be used as a light editor. Raw / Split raw already handled arrows inside the text editor.

## WYSIWYG model

1. **Inside click-to-edit / session TextEdit:** ArrowUp/Down move the caret on visual rows (egui `TextEdit`).
2. **At the first visual row:** ArrowUp leaves to the previous navigable block (caret at end).
3. **At the last visual row:** ArrowDown leaves to the next navigable block (caret at start).
4. **Single-line blocks** (headings, short paragraphs) are always on the first/last row, so Up/Down move between blocks immediately.
5. **Preview lock:** navigation (focus target + scroll-into-view) still runs; edits stay gated by existing preview-lock paths. A click on a locked formatted display anchors keyboard nav without entering edit.
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
| `apply_pending_arrow_navigation` | End of rendered frame: apply queued leave or container-level Up/Down; activate adjacent block / focus display-sense under lock; scroll via culling Y |
| `handoff_arrow_nav_after_edit_exit` | After Enter/Escape exit: remember last block + `request_focus(display_sense)` so Up/Down continue without pointer over preview |
| `arrow_nav_sel_key` | Last navigable block temp data scoped per `rendered_editor_id(tab.id)` — not a global id; cleared in `cleanup_rendered_editor_memory` on tab close |
| Split-safe ownership | Never steal keys when Raw/find has focus, or when the pointer is clearly over another pane |

Scroll-into-view uses `scroll_y_for_source_line` on viewport-culling `block_line_ranges` / `block_start_y` (no full-document layout per key).

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

## Manual QA

Checklist row **UX-12** (`UX-arrow-nav`) in [`v0.3.1-test-checklist.md`](../platform/v0.3.1-test-checklist.md).
