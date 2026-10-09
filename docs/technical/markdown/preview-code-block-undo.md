# Preview code block undo (#167 / ISS-6)

Rendered and split preview fenced code blocks (`EditableCodeBlock`) record **one undo step** per edit session (Edit → type → Done or editor focus-out), not one step per keystroke.

**Modules:** `src/markdown/widgets.rs` (`EditableCodeBlock`), `src/markdown/editor.rs` (`render_code_block`), `src/markdown/rendered_commit_undo.rs`, `src/app/central_panel.rs` (`apply_rendered_commit_undo_entries`)

## Problem

Code blocks wrote into `tab.content` on every frame while the TextEdit was dirty. `central_panel` then called `record_edit_from_snapshot()` whenever `MarkdownEditorOutput.changed`, producing many undo groups for a single logical edit (QA log: 15 `Content modified in split rendered pane` lines for one block).

## Behaviour

| Phase | Source (`tab.content`) | Undo |
|-------|------------------------|------|
| Edit clicked | Unchanged | `mark_break_before_next_commit` — next commit is a new group |
| Keystrokes in session | Unchanged (draft in `CodeBlockData`) | None |
| Done or TextEdit `lost_focus` | `update_code_block` via `record_source_commit` | One queued commit → `apply_rendered_commit_undo_entries` |

`CodeBlockOutput.changed` is true only on commit boundaries when `CodeBlockData::is_modified()`.

## Integration with `central_panel`

Same path as paragraph session commits:

1. `MarkdownEditor::show` → `rendered_commit_undo::begin_frame`
2. After `show`: `take_pending_commits` → `Tab::apply_rendered_commit_undo_entries`
3. `undo_recorded` set when the queue is non-empty — skip per-frame `record_edit_from_snapshot`

Structural edits and checkboxes still use the frame-start snapshot + fallback `record_edit_from_snapshot` when `changed && !undo_recorded`.

## Split view note

The raw pane does not see code-block typing until commit; only the preview widget shows the in-memory draft.

## Related

- [Rendered session undo](./rendered-edit-session-undo.md) — same `record_source_commit` queue for WYSIWYG blocks
- [Undo/redo](../editor/undo-redo.md)

## Tests

```bash
cargo test issue_167
cargo test rendered_commit_undo::
```
