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
| Enter (no `Shift`) | `close_active_ui(SaveIfDirty)` — commit and exit | Same (any Enter; newlines stripped) |
| Shift+Enter | Soft line break in buffer; preserved on commit via `update_source_range` | Treated as Enter (lists strip newlines) |
| Escape | `discard_active` + `reload_plain_block_from_source`; clear `active` + surrender focus | Same |

Plain Enter must **not** insert a structural `\n` into the TextEdit buffer (that caused duplication when committed against a stale single-line AST span). `consume_plain_block_enter` runs **before** `TextEdit::show` on the active block so egui never records the newline; Shift+Enter is not consumed and inserts a soft break.

Multi-line soft-break commits rely on [source range replacement](./rendered-edit-source-range.md) (`block_replace_end_line`) for round-trip without duplication.

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
- **List item:** `extract_list_item_content(source, start_line)` — content without marker; newlines stripped on edit/commit.

## Implementation map

| Concern | Location |
|---------|----------|
| Shared TextEdit + Enter/Escape/lost-focus | `render_session_plain_text_block` in `src/markdown/editor.rs` |
| Enter consumption (before TextEdit) | `consume_plain_block_enter` in `src/markdown/editor.rs` |
| Paragraph render | `render_paragraph`, `render_paragraph_with_structural_keys` (simple branch only) |
| List item render | `render_list_item`, `render_list_item_with_structural_keys` (simple branch only) |
| Session load/save | `load_for_epoch` / `save_for_epoch` in `rendered_session.rs` |
| Commit | `commit_session_block`, `update_source_range` — list markers preserved via `extract_line_prefix`; span merge via `block_replace_end_line` — see [source range replacement](./rendered-edit-source-range.md) |
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
- Soft newline round-trip: `test_paragraph_soft_newline_commit_roundtrip` in `editor.rs`
