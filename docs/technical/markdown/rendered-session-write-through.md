# Rendered Session — Per-Frame Write-Through

Paragraph, list, heading, and table-cell edits update `Tab::content` on the frame the buffer changes. Fenced code blocks stay in `CodeBlockData` until Done or focus-out, but the tab is unsaved as soon as the buffer differs from source.

**Related:** [Session flush and save commit](./rendered-edit-flush.md), [Preview code block undo](./preview-code-block-undo.md)

## Frame update

`finish_rendered_content_frame` (`src/markdown/editor.rs`) runs after `MarkdownEditor::show` in both Rendered mode and the Split preview pane (`src/app/central_panel.rs`).

| Kind | What happens |
|------|----------------|
| **Active** dirty paragraph, list, formatted variant, or heading in `RenderedEditSession` (not table cells) | `write_session_block_to_source` into `tab.content`. Inactive dirty blocks wait for `close_active(SaveIfDirty)` / flush — see [scope-and-seed](./rendered-session-scope-and-seed.md) §3 |
| Table with persisted `TableEditState.content_modified` | `update_table_in_source` from `TableData::to_markdown` |
| Code block with `is_editing && is_modified()` | No content write. `Tab::set_rendered_pending_dirty(true)` |

`Tab::is_modified` treats `rendered_pending_dirty` as modified even when `content` still matches the saved original. `AppState::has_unsaved_changes` uses that through `should_prompt_to_save`. `mark_saved` clears the flag.

The Split raw pane reads `tab.content`. A write-through bumps `content_version` (via the undo record below) and does **not** bump `source_epoch`, so the raw editor resyncs next frame and rendered widget ids stay stable.

## Undo

Write-through calls `Tab::record_rendered_live_write_through`, which records with typing-run coalescing (`record_edit_from_snapshot_inner`, no epoch bump). It does not enqueue `record_source_commit` per keystroke. Code-block commits still use one `record_source_commit` on Done or focus-out.

## Tests

- `paragraph_session_one_char_writes_through_and_marks_unsaved`
- `table_cell_session_one_char_writes_through_and_marks_unsaved`
- `code_block_session_one_char_marks_unsaved_without_writing_until_commit`
