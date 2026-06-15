# Rendered Edit Session — Formatted Paragraphs & List Items

Formatted paragraphs and list items (inline `**bold**`, `*italic*`, `` `code` ``, links, wikilinks, images, hard line breaks) use [`RenderedEditSession`](../../../src/markdown/rendered_session.rs) for click-to-edit state, buffer ownership, and one-click cross-block switching — the same model as [headings](./rendered-edit-session-headings.md) and [plain paragraphs / simple list items](./rendered-edit-session-paragraphs-lists.md).

**PRD:** [Rendered Edit Session](../../../ai-workflow/prds/prd-rendered-edit-session.md)  
**Foundation:** [Core types](./rendered-edit-session-core.md), [Widget identity](./rendered-widget-identity.md), [Click-to-Edit Formatting (legacy)](./click-to-edit-formatting.md)

## Display ↔ edit toggle

Each formatted block has two modes, gated by `BlockEditState::formatted_editing`:

| `formatted_editing` | Render | Input |
|---------------------|--------|-------|
| `false` (display) | Single galley via [`FormattedBlockLayout`](./rendered-edit-session-formatted-layout.md) (`paint_formatted_block_display`) | Click → enter edit mode (see below); link/wikilink clicks consumed by nested widgets are skipped |
| `true` (edit) | `TextEdit::multiline` bound to `state.text` (raw markdown), shared layouter | Enter / Escape / lost focus exit (see below) |

## Click-to-edit cursor mapping

When the user clicks the styled display:

1. `enter_formatted_edit_on_display_click` reads `pointer.interact_pos()`.
2. `formatted_block_layout(block)` retrieves the galley stored during paint (same frame).
3. `FormattedBlockLayout::raw_cursor_at` runs `galley.cursor_from_pos` then `map_displayed_to_raw` on the session buffer.
4. The raw position is queued in `PendingActivation { cursor_char_index, request_focus: true }`.
5. `session.switch_to_ui` activates the block (commits any previous block via `commit_session_block`) and surrenders the previous focus.
6. `state.formatted_editing = true`.
7. `mark_session_active_clicked_if_clicked` records the click against the active-clicked flag so the end-of-frame `session_dismiss_if_clicked_outside` does not immediately close the block we just activated. The TextEdit for the new active block only renders on the next frame, so no widget response can set the flag for it on the click frame.

On the next frame, the edit branch renders the TextEdit, applies the activation (focus + cursor placement), and the caret lands where the user clicked.

> **Implementation note** — `session_active_clicked_key` returns a process-global `Id::new("ferrite_rendered_session_active_clicked")` so writes from any depth (deeply nested `ui.horizontal/vertical` scopes) land on the same key the outer dismiss reads. An earlier `ui.id()`-derived key silently mismatched between writers and the dismiss reader; the formatted display-click path was the only one without a TextEdit able to recover focus on the next frame, so it was the visible regression.

See [Galley cursor positioning](../editor/galley-cursor-positioning.md) for the display→raw mapping details.

## Edit-mode exits

| Trigger | Path | Active block | `formatted_editing` |
|---------|------|--------------|---------------------|
| Enter (paragraph: no `Shift`; list item: any modifier) | `close_active_ui(SaveIfDirty)` | cleared | `false` (via `close_active`) |
| Escape | `discard_active` reloads from source via `reload_formatted_block_from_source`, caller clears `active` + surrenders focus | cleared | `false` (via `discard_active`) |
| Click another session-backed block | that block's `switch_to_ui` runs `commit_session_block` for the previous block | switches to new | previous reset to `false` (via `close_active`) |
| Click outside any block | `session_dismiss_if_clicked_outside` → `close_active_ui(SaveIfDirty)` | cleared | `false` |
| Focus lost (Tab cycle, programmatic surrender) | `close_active_ui(SaveIfDirty)` | cleared | `false` |
| Source-epoch bump (raw edit, reload) | `load_for_epoch` calls `invalidate_buffers` | cleared | full state reset |

Rendered commits do **not** bump `Tab::source_epoch` — see [`rendered-edit-session-core.md`](./rendered-edit-session-core.md).

## Block identity

| Variant | `BlockRef` | Widget id key |
|---------|------------|---------------|
| Formatted paragraph (`render_paragraph`) | `FormattedParagraph { line, structural: false }` | `formatted_paragraph` + line + `text_edit` |
| Formatted paragraph (`render_paragraph_with_structural_keys`) | `FormattedParagraph { line, structural: true }` | `formatted_paragraph_sk` + line + `text_edit` |
| Formatted list item (`render_list_item`) | `FormattedListItem { line, item, structural: false }` | `formatted_list_item` + line + item + `text_edit` |
| Formatted list item (`render_list_item_with_structural_keys`) | `FormattedListItem { line, item, structural: true }` | `formatted_list_item_sk` + line + item + `text_edit` |

Widget ids match the pre-migration `StickyFocus::Formatted*` scheme so any persisted egui temp memory (focus, scroll, IME state) remains valid across the migration.

## Cold init & reload

| When | Source |
|------|--------|
| First render of block | `extract_paragraph_content(source, start_line, end_line)` (paragraph) or `extract_list_item_content(source, start_line)` (list item — strips marker) |
| Escape / discard | `reload_formatted_block_from_source` re-runs the extraction above |
| `source_epoch` mismatch | `invalidate_buffers()` clears all blocks; next render re-cold-inits |

Cold init seeds via `on_text_changed` then immediately clears `dirty` so the buffer isn't treated as a pending edit.

List item buffers strip embedded newlines on every edit and at commit time — `update_source_range` preserves the original list marker via `extract_line_prefix`.

## Implementation map

| Concern | Location |
|---------|----------|
| Edit-mode TextEdit + Enter/Escape/lost-focus | `render_session_formatted_edit_text` in `src/markdown/editor.rs` |
| Display-area click → edit | `enter_formatted_edit_on_display_click` in `src/markdown/editor.rs` |
| Cold seed helper | `ensure_formatted_block_initialized` |
| Per-variant render | `render_paragraph`, `render_paragraph_with_structural_keys`, `render_list_item`, `render_list_item_with_structural_keys` |
| Commit | `commit_session_block` handles `FormattedParagraph` (same as `Paragraph`) and `FormattedListItem` (same as `ListItem`, newline-stripped) |
| Reload on discard | `reload_formatted_block_from_source` |
| Display paint + hit-test | `FormattedBlockLayout`, `paint_formatted_block_display`, `layout_for_formatted_click`, `layout_wrap_width` in `rendered_session.rs` — see [formatted layout](./rendered-edit-session-formatted-layout.md) |
| Display→raw cursor | `FormattedBlockLayout::raw_cursor_at` → `map_displayed_to_raw` |
| Click-away dismiss | `session_dismiss_if_clicked_outside` (shared with plain blocks) |

## Removed (this migration)

- `FormattedItemEditState` struct + per-item `egui::memory` storage (`formatted_paragraph[_sk]/edit_state`, `formatted_list_item[_sk]/edit_state`)
- `FormattedItemEditState` entry in `cleanup_rendered_editor_memory` (formatted state now lives in the per-tab `RenderedEditSession`)
- `rendered_focus::formatted_exit_should_save` (Phase 0 blur hotfix) + all of its tests
- `rendered_focus::StickyFocus::FormattedParagraph` / `FormattedListItem` variants
- `rendered_focus` defer/switch helpers used only by formatted paths: `request_switch`, `after_text_edit`, `should_activate_display_click`, `should_defer_commit`, `should_flush_deferred_commit`, `focus_loss_should_commit`, `defer_commit_id`, `DeferCommitState`
- `commit_session_if_active` interim bridge — superseded by `switch_to_ui`'s built-in close-previous behaviour
- `prepare_switch` / `note_switch` are now private (table cells go through `request` / `set_active` / `restore_switch_focus` only)

## Manual verification

1. Document with: `# Title`, formatted paragraph (`A **bold** word.`), bullet list with a formatted item (`- *italic* item`), plain paragraph, plain list item.
2. Click each block; type to edit; single-click switch between every pair (formatted ↔ formatted, formatted ↔ heading, formatted ↔ plain).
3. Escape in a formatted block reverts the buffer to raw source.
4. Switch to raw mode, edit a formatted block's source line, switch back — buffers re-seed from updated source.
5. Verify caret lands at the clicked character in raw markdown (test on `A **bold** word.` — click after "bold" should place caret just before the closing `**`).

## Tests

- `cargo test rendered_session::` — includes `formatted_editing_flag_resets_on_close`, `formatted_switch_commits_previous_and_resets_editing_flag`, `formatted_discard_reload_resets_flags_via_reload_fn`, `formatted_widget_ids_remain_stable_with_legacy_keys`.
- Existing commit-path tests (`test_update_source_range_preserves_bullet_list`, etc.) cover formatted commits via the shared `commit_session_block` arms.
