# Undo typing-run coalescing

`EditHistory` (`src/editor/ferrite/history.rs`) can merge consecutive single-character inserts into one undo group. The constant is `UNDO_COALESCE_MS` (700), not a setting.

## API

Callers that want coalescing use `record_with_meta(ops, HistoryRecordMeta)`:

- `cursor_line` / `cursor_column` — caret **after** the edit (column is a character offset on that line)
- `timestamp_ms` — caller clock
- `kind` — `HistoryOpKind`

`record_operations` / `record_operation` still push one group per call and do not coalesce.

Helpers: `history_op_kind_for_insert_text`, `history_record_meta_for_ops` (refine editor hints against diff ops).

## Merge rule

A `Typing` insert joins the open group only when all of these hold:

- The batch is one `Insert` of a single character that is not `\n` or `\r`
- The last operation in the open group is that same kind of insert, and the new insert position is the previous position plus one character
- `cursor_line` matches and `cursor_column` is the previous column plus one
- `timestamp_ms - previous < 700` (equal timestamps, one frame, merge; a gap of 700 ms or more starts a new group)

## Breaks

`break_group()` clears the coalesce anchor. The next record starts a new group.

These kinds break before they record: `Newline`, `Delete`, `CursorJump`, `SelectionChange`, `Paste`. A `Typing` batch that is not a single non-newline character (paste-sized text, or delete+insert) also breaks. An empty `ops` list with a non-`Typing` kind only breaks.

`undo` / `redo` / `undo_string` / `redo_string` / `clear` call `break_group()` so a later keystroke does not append to a group that was just undone.

Redo replays the merged group as one step.

## Raw editor path (external snapshot)

Raw mode does not record inside `FerriteEditor`; each dirty frame syncs rope → `Tab.content` and records via snapshot diff.

| Step | Location |
|------|----------|
| Pre-edit baseline | `EditorWidget` → `Tab::prepare_undo_snapshot_hashed()` |
| Edit kind + monotonic time | `FerriteEditor::note_external_undo_edit` / `take_external_undo_recording` (per-frame insert/delete/paste/format/find-replace/Tab) |
| Record with coalescing | `Tab::record_external_edit_from_snapshot_coalesced` → `EditHistory::record_with_meta` |
| Non-coalesced external | `Tab::record_external_edit_from_snapshot` (tree viewer, etc.) → `record_operations` |

**Boundaries** (call `Tab::break_undo_group` / `EditHistory::break_group`):

- Editor focus lost (`EditorWidget` after `editor.ui`)
- Caret moved without a content change that frame (cursor jump)
- View mode toggle (`handle_toggle_view_mode`)
- Non-typing commands flagged on the editor (`break_external_undo_group` before format, find/replace, etc.)

Rendered paragraph/list session commits and tree-viewer diffs still use `record_operations` (one group per call). They do not time-merge keystrokes.

## Split preview commit

Fenced code blocks in Rendered and Split preview are not typed into `tab.content` each frame. `EditableCodeBlock` keeps the draft in `CodeBlockData` until Done or the TextEdit loses focus. That commit is one undo group for the whole session, including a pause longer than 700 ms and edits longer than one character.

| Phase | Source | Undo |
|-------|--------|------|
| Edit clicked | Unchanged | `mark_break_before_next_commit` so the next commit does not join an open typing run |
| Keystrokes | Draft only (`CodeBlockData`) | None |
| Done or `lost_focus` | `update_code_block` via `record_source_commit` | One queued commit |

`central_panel` applies the queue with `Tab::apply_rendered_commit_undo_entries` and sets `undo_recorded` so it skips a second per-frame `record_edit_from_snapshot`. The raw pane in Split view stays on the last committed source until that commit. See [preview code block undo](../../markdown/preview-code-block-undo.md).
