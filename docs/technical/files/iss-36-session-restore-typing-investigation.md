# ISS-36 session restore typing investigation

SES-6 (one sighting) left a restored tab's buffer as the three characters just typed. This note records the evidence, the automated sequence that was run, the hardening already in tree, and why the QA row still needs a second sighting.

## Evidence

From the SES-6 log and screenshot (QA round 2, tab 71, `/tmp/ses1-a.md`):

- The editor was created with the file's content. Two heading fold regions were synced at creation, so the buffer was not empty when the widget came up.
- The recovery disk-anchor hash matched none of the trivial strings (`" W1"`, empty, and similar).
- `End` produced no caret move.
- Three typed characters at `(0,0)` left the buffer `" W1"`. Enter made it `" W1\n"` (status: 1 word, 4 characters, 2 lines).

That pattern matches a whole-buffer selection that was replaced by the typed text (or the same key events applied through a route that replaced the buffer). It does not match a restore path that loaded an empty or three-character file: the fold sync and the hash both say the real document was present first.

## What was tried

Manual SES-1→SES-6 on a running Ferrite process (Windows/Linux, `--log-level debug`, second window, 30-minute watch for `EditorWidget: buffer for tab N shrank`) is left to a human. See the task's manual verification. This pass scripted the same sequence in `restore_then_type_never_shrinks_buffer` (`cargo test --bin ferrite restore_then_type`):

1. Write an 8-line saved file.
2. Restore it.
3. Build a `FerriteEditor` from the restored buffer, apply the first-frame caret and `reset_transient_input_state`, move to end of the current line, insert `abc`.
4. Assert byte length is at least the original length plus 3, and that the original first line is still present.

Variations, all of which kept the file body:

| Variation | Result |
|-----------|--------|
| Legacy `settings.last_open_tabs` via `AppState::with_settings` | Disk text restored; typing appends |
| `capture_session_state` → `restore_from_session_result` (`session.json` shape), with a second window open | Same |
| Whole-buffer selection stored on the session tab before capture | `Tab.selection` is `None` after restore |
| Recovery blob `" W1"` keyed by the **runtime** tab id (not the session tab id) | Ignored; disk text restored |
| Vim mode on (insert) and off | Typing appends |
| `click_count = 3` planted before the first-frame reset | Reset clears it; selection stays collapsed |
| Word wrap on | Typing appends |

No `EditorWidget: buffer for tab N shrank` line is produced by this harness. It never drives `EditorWidget::show`, so it does not watch that log; it checks the buffer the widget would be given and the first-frame reset the widget applies.

## Hardening landed

These are already in the tree from the ISS-36 harden steps (they are not new edits in this investigation):

1. **Caret only.** `Tab::apply_restored_caret_only` on the session path and `Tab::from_tab_info` / `from_tab_info_with_bytes` on the legacy path. Persisted `SessionTabState.selection` is not copied onto the tab. `pending_restore_editor_reset` makes the first raw frame call `FerriteEditor::reset_transient_input_state` (`click_count = 0`, collapsed caret). See `technical/files/session-restore-caret-only.md`.
2. **Anomalous shrink undo.** A one-frame shrink of more than half of a buffer longer than 64 characters, with a collapsed pre-frame selection and a single `Text`/`Key` and no Cmd/Ctrl+A, is logged at error and recorded as its own undo groups so one Ctrl+Z restores the pre-frame text. The edit is not blocked. See `technical/editor/anomalous-buffer-shrink-undo.md`.
3. **Checkout slot.** `EditorWidget::show` takes the per-tab `Option<FerriteEditor>` instead of `mem::replace` with `FerriteEditor::new()`. A re-entrant `get_ferrite_editor_mut` sees `None`, not an empty placeholder. See `technical/editor/ferrite-editor-storage-checkout.md`.

## Outcome

The scripted sequence did **not** reproduce the buffer replacement. No new root cause was found, and no further product change was made beyond this regression test.

The regression test is `state::tests::restore_then_type_never_shrinks_buffer`. The editor half is `editor::type_three_chars_after_line_end`.

## Remaining hypotheses

Still consistent with the SES-6 observations, and not closed by the automated script:

- A whole-buffer selection created **after** the first raw frame (click handling, focus, or another widget) and still active when the three characters arrived. `End` moving nowhere fits a selection whose end is already at the document end, or a key that never reached the editor.
- The three characters arriving through a route that replaces buffer text without going through a collapsed caret (IME commit, a different egui `TextEdit`, or a sync that writes a short string back over `Tab.content`).
- Triple-click `click_count` surviving when the first frame is not the raw editor (`pending_restore_editor_reset` only runs from `EditorWidget::show_inner`).
- A recovery file that **does** match the session tab id and passes the identity checks, so the restored buffer is the short recovered text. SES-6's hash did not match the trivial strings, which argues against this for that sighting. A blob keyed only by the new runtime id is not consulted (`resolve_tab_content` looks up `session_tab.tab_id`).

The empty-placeholder checkout and "restore the saved selection" paths are no longer available.

## QA row

**Needs a second sighting.** The row stays open until a human run of SES-1→SES-6 (second window, recovery file present, `--log-level debug`) either shows `EditorWidget: buffer for tab N shrank` or a session long enough to treat the one-off as unreproduced. If it happens again, the anomalous-undo groups are the recovery path (one Ctrl+Z), and the error log's pre-frame selection, `click_count`, and input summary are the evidence to keep.
