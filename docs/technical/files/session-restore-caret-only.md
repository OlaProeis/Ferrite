# Session restore: caret-only (ISS-36)

## Problem

Persisted session data can include a **selection range** (`SessionTabState.selection`) and the raw editor keeps **multi-click state** (`click_count`, last-click time/position). Rehydrating those on startup was a leading hypothesis for “whole buffer selected, first keystroke replaces content with a few characters” after restore.

## Policy

On restore, Ferrite carries over **caret position only**:

- `Tab.cursors` is a single collapsed cursor at the saved char index.
- Legacy `Tab.selection` stays `None`.
- Selection ranges in `session.json` are **ignored** at apply time (they may still be written on save for backward compatibility).

Scroll offset, view mode, preview lock, and file content paths are unchanged.

## Restore paths

| Path | Entry | Caret |
|------|--------|--------|
| Crash / `session.json` | `AppState::restore_from_session_result` | `Tab::apply_restored_caret_only(session_tab.cursor_char_index)` — does not read `session_tab.selection` |
| Settings `last_open_tabs` | `restore_session_tabs` → `Tab::from_tab_info_with_bytes` | Caret from `TabInfo.cursor_position` (line/col → char index); no selection in `TabInfo` |

Both paths set `Tab.pending_restore_editor_reset = true` so the first raw-editor frame clears FerriteEditor transient input state.

## Raw editor first frame

`EditorWidget::show_inner` consumes `pending_restore_editor_reset`:

1. `set_cursor` from `Tab.cursor_position`
2. `FerriteEditor::reset_transient_input_state()` — `click_count = 0`, clear last-click fields, collapse selection to caret

The ISS-36 diagnostic in `EditorWidget` (buffer shrink + pre-frame selection / `click_count` in the warn log) is unchanged.

## API reference

- `Tab::apply_restored_caret_only` — `src/state.rs`
- `Tab.pending_restore_editor_reset` — one-shot flag on `Tab`
- `FerriteEditor::reset_transient_input_state` — `src/editor/ferrite/editor.rs`

## Tests

- `state::tests::test_last_open_tabs_restore_drops_selection_keeps_caret`
- `state::tests::test_session_json_restore_drops_selection_keeps_caret`
- `editor::ferrite::editor::tests::test_reset_transient_input_state_after_triple_click`
