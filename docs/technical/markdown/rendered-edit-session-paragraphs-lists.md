# Rendered Edit Session — Plain Paragraphs & List Items

Plain (non-formatted) paragraphs and simple list items in rendered mode use [`RenderedEditSession`](../../../src/markdown/rendered_session.rs) for buffer ownership, one-click cross-block switching, and commit-on-switch — the same model as [headings](./rendered-edit-session-headings.md).

**PRD:** [Rendered Edit Session](../../../ai-workflow/prds/prd-rendered-edit-session.md)  
**Foundation:** [Core types](./rendered-edit-session-core.md), [Widget identity](./rendered-widget-identity.md)

## Behaviour

| Action | Result |
|--------|--------|
| Edit paragraph P, click list item L | P commits to source, L activates with one click |
| Edit heading H, click paragraph P | H commits, P activates (shared session) |
| Focus leaves active block (no other session block) | `close_active_ui(SaveIfDirty)` commits buffer |
| Click outside active widget (empty space, label, formatted display, culled gap) | End-of-frame dismiss when active block did not receive `response.clicked()` |
| Click formatted display while session block active | Formatted display click goes through `switch_to_ui`, which commits the previous active block automatically (no separate bridge) |
| Raw-mode edit bumps `source_epoch` | Session buffers cleared on next rendered frame |

## Enter / newline semantics (plain paragraphs & list items)

Same model as [formatted blocks](./rendered-edit-session-formatted.md) — **commit+exit on plain Enter**, not structural paragraph split.

| Trigger | Paragraph | List item |
|---------|-----------|-----------|
| Enter (no `Shift`) | `close_active_ui(SaveIfDirty)` — commit and exit | Same (any Enter; an item cannot gain a line) |
| Shift+Enter | Soft line break in buffer; preserved on commit via `update_source_range` | Treated as Enter (new line breaks are rejected) |
| Escape | `discard_active` + `reload_plain_block_from_source`; clear `active` + surrender focus | Same |

Plain Enter must **not** insert a `\n` into the TextEdit buffer. `take_block_commit_enter` runs **before** `TextEdit::show` on the focused active block — for plain and formatted blocks alike — and consumes the key so egui never records the newline; Shift+Enter in a paragraph is not consumed and inserts a soft break. (Checking `key_pressed(Enter)` after the TextEdit is too late: the multiline TextEdit has already inserted the line break.)

Multi-line soft-break commits rely on [source range replacement](./rendered-edit-source-range.md) (`BlockEditState::source_rows`) so a grown paragraph inserts lines instead of overwriting the blocks below it.

## Buffer resync

| When | Behaviour |
|------|-----------|
| First render of block | `on_text_changed(cold_text)` then `dirty = false` |
| Block inactive and not dirty, source changed | Buffer replaced from `cold_text` (`extract_paragraph_content` / `extract_list_item_content`) |
| Escape / discard | `reload_plain_block_from_source` |
| `source_epoch` mismatch | `invalidate_buffers()` — full session reset |

Formatted paragraphs/list items and table cells share this model — see [`rendered-edit-session-formatted.md`](./rendered-edit-session-formatted.md) and [`rendered-edit-session-tables.md`](./rendered-edit-session-tables.md).

## Block identity

| Block | `BlockRef` | Widget id key |
|-------|------------|---------------|
| Plain paragraph | `Paragraph { line }` | `para_text` + line |
| Simple list item | `ListItem { line, item }` | `list_item_text` + line (`item` is HashMap key only) |

Cold buffer init:

- **Paragraph:** `extract_paragraph_content(source, start_line, end_line)` — preserves trailing spaces vs AST round-trip.
- **List item:** `extract_list_item_content(source, start_line, end_line)` — the item paragraph's full line span, one buffer row per source line (see below).

## Hard-wrapped list items

A list item's paragraph may span several source lines (hanging indent or lazy continuation):

```markdown
- [ ] **B1. Reload.** Open a saved `.md`, back in
  Ferrite **Ctrl+Shift+R** → content updates.
```

The buffer for `ListItem` / `FormattedListItem` holds **every** line of that paragraph, joined with `\n`. Seeding only the first line hid the rest in Rendered view and, on commit, replaced the whole AST span with that one line — deleting the continuation lines from the file.

| Step | Behaviour |
|------|-----------|
| Seed | `extract_list_item_content`: row 0 without the list marker (`split_container_prefix`), later rows without their hanging indent / quote markers (`continuation_indent`) |
| Span | Paragraph node's `start_line..=end_line`. `list_item_paragraph_end_line` falls back to the start line when no paragraph node starts there, so a container span (item + nested lists) is never read as item text |
| Edit | The buffer keeps its seeded `\n`, but an edit cannot add one: `strip_inserted_newlines(before, after)` drops newlines from the inserted region only (Enter in the formatted TextEdit, pasted line breaks). Flag: `reject_new_newlines` |
| Commit | `list_item_commit_text` drops rows the edit emptied (a blank source line would split the item), then `update_source_range` writes the rows back; a row replacing an existing line keeps that line's own indent, so an unchanged item round-trips byte for byte |
| Status bar | `rendered_caret_source_position` adds the hanging indent on continuation rows |

## Other blocks inside a list item

A list item can hold more than its first paragraph:

````markdown
- first paragraph (the marker row)

  second paragraph

  ```
  code
  ```

  - nested list
````

| Concern | Behaviour |
|---------|-----------|
| Render | The marker row shows the first paragraph. `list_item_child_blocks` returns every other child in source order — paragraphs, headings, code blocks, quotes, tables, nested lists — and both list-item render paths draw them below the row. Nested lists go through `render_list*` at `indent_level + 1`; other blocks through `render_node*` inside `with_list_item_child_indent` (same left edge as a nested list's markers). Previously only nested lists were drawn and the rest of the item was missing from Rendered view |
| Item without a paragraph | Unchanged: the marker row shows the joined text of the item's children, and only nested lists follow |
| Identity | Child blocks are ordinary session blocks keyed by their own source line (`Paragraph { line }`, `Heading { line }`, …) |
| Paragraph / heading edit | The indent that keeps the block inside the item is its container prefix (`split_container_prefix`): not part of the buffer, written back on commit |
| Code block / table edit | `update_code_block` and `update_table_in_source` put the first line's `continuation_indent` in front of every line they write (`with_container_prefix`; blank lines get the prefix without trailing whitespace) — also for code blocks and tables inside a `>` quote |
| Arrow navigation | `rendered_arrow_nav::collect_list_items` walks the same children in the same order |

## Implementation map

| Concern | Location |
|---------|----------|
| Shared TextEdit + Enter/Escape/lost-focus | `render_session_plain_text_block` in `src/markdown/editor.rs` |
| Enter consumption (before TextEdit) | `take_block_commit_enter` in `src/markdown/editor.rs` |
| Paragraph render | `render_paragraph`, `render_paragraph_with_structural_keys` (simple branch only) |
| List item render | `render_list_item`, `render_list_item_with_structural_keys` (simple branch only) |
| Session load/save | `load_for_epoch` / `save_for_epoch` in `rendered_session.rs` |
| Commit | `commit_session_block`, `update_source_range` — list markers preserved via `extract_line_prefix`; exact span via `session_block_source_end_line` — see [source range replacement](./rendered-edit-source-range.md) |
| Reload on discard | `reload_plain_block_from_source` |
| Click-away dismiss | `session_dismiss_if_clicked_outside` — end of frame if active block did not receive `response.clicked()` |
| Formatted display click | Handled natively by `switch_to_ui` (see [`rendered-edit-session-formatted.md`](./rendered-edit-session-formatted.md)) |

## Epoch invalidation

Session state is stored in egui temp memory keyed by editor id. A companion key `rendered_edit_session_epoch` records the last seen `Tab::source_epoch`.

When `load_for_epoch` sees a mismatched epoch (raw edit, reload, external mutation), it calls `invalidate_buffers()` so all block buffers reload from source on next interaction.

Rendered commits do **not** bump `source_epoch` — only external/raw edits do.

## Removed (plain paragraph + simple list item only)

- Temp egui buffers: `para_edit_buffer`, `list_item_edit_buffer`, tracking ids
- `rendered_focus::after_text_edit` for these paths
- `rendered_focus::focus_loss_should_commit` for these paths
- Per-frame immediate source writes on `TextEdit::changed()` (non-structural paragraph path)

## Manual verification

1. Document: `# Title`, plain paragraph, bullet list with one simple item.
2. Edit each block; single-click switch Title → paragraph → list item → Title; confirm edits persist at leave time.
3. Switch to raw mode, change a line, return to rendered; confirm session buffers match updated source.
4. Single-line paragraph `test`: edit in rendered view, press **Enter** — commits and exits (no extra line). **Shift+Enter**, type `test2`, blur — Raw and Split show `test` / `test2` on two lines with no duplication.

## Tests

- `cargo test rendered_session::` — includes `load_for_epoch_invalidates_when_epoch_changes`, widget id parity for paragraph/list
- Existing list commit tests: `test_update_source_range_preserves_bullet_list` in `editor.rs`
- Soft newline round-trip: `test_paragraph_soft_newline_commit_roundtrip` in `editor.rs`; Enter / Shift+Enter end to end: `paragraph_enter_commits_without_breaking_the_line`, `paragraph_soft_break_keeps_the_following_paragraphs`
- Blocks inside list items in `editor.rs`: `list_item_shows_all_of_its_blocks_in_source_order`, `list_item_child_paragraphs_edit_in_place`, `indentation_is_container_prefix_not_editable_text`, `list_item_table_renders_and_edits_inside_the_item`, `nested_code_block_and_table_writes_keep_their_container_prefix`; in `rendered_arrow_nav.rs`: `collect_list_item_child_blocks_in_document_order`
- Hard-wrapped list items in `editor.rs`: `seed_session_block_text_list_item_includes_continuation_lines`, `roundtrip_wrapped_*` (task, ordered, nested, quoted, CRLF), `wrapped_list_item_commit_shrinks_and_drops_emptied_rows`, `strip_inserted_newlines_keeps_seeded_line_breaks`, and the headless end-to-end `wrapped_task_item_renders_every_line_and_survives_an_edit`
