# Preview code block undo (#167 / ISS-6 / ISS-40)

Rendered and split preview fenced code blocks (`EditableCodeBlock`) record **one undo step** per edit session (Edit → type → Done or editor focus-out), not one step per keystroke.

**Modules:** `src/markdown/widgets.rs` (`EditableCodeBlock`, session start), `src/markdown/rendered_session.rs` (`mark_codeblock_edit_session_start`), `src/markdown/editor.rs` (`render_code_block`, `finish_rendered_content_frame`), `src/markdown/rendered_commit_undo.rs`, `src/state.rs` (`Tab::apply_rendered_commit_undo_entries`, `break_undo_group`)

## Problem

Code blocks wrote into `tab.content` on every frame while the TextEdit was dirty. `central_panel` then called `record_edit_from_snapshot()` whenever `MarkdownEditorOutput.changed`, producing many undo groups for a single logical edit (QA log: 15 `Content modified in split rendered pane` lines for one block).

## Behaviour

| Phase | Source (`tab.content`) | Undo |
|-------|------------------------|------|
| Edit clicked | Unchanged | `mark_codeblock_edit_session_start` → `Tab::break_undo_group` in `finish_rendered_content_frame` + `mark_break_before_next_commit` for the commit |
| Keystrokes in session | Unchanged (draft in `CodeBlockData`) | None; `finish_rendered_content_frame` skips `record_edit_from_snapshot` while `live.code_pending` |
| Done or TextEdit `lost_focus` | `update_code_block` via `record_source_commit` | One queued commit → `apply_rendered_commit_undo_entries`; then `mark_break_after_commit` → `break_undo_group` again |

`CodeBlockOutput.changed` is true only on commit boundaries when `CodeBlockData::is_modified()`.

## Frame finish (`finish_rendered_content_frame`)

1. `MarkdownEditor::show` → `rendered_commit_undo::begin_frame`
2. After `show`: consume code-block session tab break → apply queued commits → live write-through / fallback snapshot
3. `undo_recorded` (from `had_commits`) plus `live.code_pending` gate fallback `record_edit_from_snapshot`
4. One `log::debug` (`Content modified in split rendered pane, recorded for undo`) only when commit undo entries were applied — not from `central_panel` each frame

Structural edits and checkboxes still use the frame-start snapshot + fallback `record_edit_from_snapshot` when `changed && !undo_recorded && !code_pending`.

## Split view note

The raw pane does not see code-block typing until commit; only the preview widget shows the in-memory draft.

## Related

- [TextEdit identity & focus](./preview-code-block-textedit-identity.md) — stable scoped ids and draft retention (ISS-40 part 2)
- [Rendered session undo](./rendered-edit-session-undo.md) — same `record_source_commit` queue for WYSIWYG blocks
- [Undo/redo](../editor/undo-redo.md)

## Tests

```bash
cargo test issue_167
cargo test rendered_commit_undo::
cargo test code_block_session
```
