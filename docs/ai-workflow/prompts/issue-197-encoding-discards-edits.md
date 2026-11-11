# Fix GitHub issue #197 (Ferrite 0.3.1)

Paste this whole file into a new chat. Fix only this issue. Do not commit unless asked.

## Issue

https://github.com/OlaProeis/Ferrite/issues/197

Changing the encoding in the status bar drops unsaved edits with no warning.

`Tab::set_encoding` in `src/state.rs` (about line 2616) re-decodes `original_bytes` and assigns the result to both `content` and `original_content`. The status bar (`src/app/status_bar.rs`, about line 872) calls it as soon as you pick an encoding. Edits since the last save disappear, and because `original_content` was reset the tab looks clean.

## Repo

- Path: `G:\DEV\markDownNotepad`
- Branch: `0.3.1-experimental`
- The working tree already has unrelated edits. Do not revert them, do not stash them, and do not commit them.

## What to change

If the active tab is modified, do not call `set_encoding` yet. Ask first.

- Cancel: keep the edits and the current encoding.
- Discard: re-decode from `original_bytes` (today's `set_encoding` behaviour).
- Save, then switch: save the buffer with the encoding it already has, then re-decode the bytes you just wrote. If the save does not happen, do not switch.

A clean tab can still switch immediately, same as today.

Do not reuse `show_confirm_dialog` or `PendingAction`. That dialog is the close-tab / exit prompt in `src/app/dialogs.rs`. Add a separate flag and a small window from `render_dialogs`, for example `show_encoding_switch_dialog`, so it does not collide with issue #198 (that chat may add `show_encoding_lossy_save_dialog` in the same file).

New strings go through `t!`. Add the keys to `locales/en.yaml` and the other `locales/*.yaml` files. English text is fine in the locales you are not translating. Follow the `dialog.unsaved_changes` keys for shape.

`set_encoding` on a new unsaved document only changes the label used at save time (no `original_bytes`). That path should not pop the discard dialog.

## Tests

Unit-test the decision without opening the whole app if you can:

- A modified tab does not change `content` when the switch is cancelled.
- Discard re-decodes and the unsaved paragraph is gone.
- An unmodified tab still re-decodes straight away.

Look at existing `Tab` tests in `src/state.rs` and follow them.

## Docs

Add a short note under `docs/technical/files/` and one bullet in `docs/index.md`. #197. Say that a dirty tab asks before the status-bar encoding picker re-decodes from the last saved bytes.

## Leave alone

- Issue #198 is the lossy-save warning. Do not implement that here. If Save-then-switch calls the existing save path, that is enough.
- Do not comment on GitHub.
- Do not mark tasks done.
- Do not revert other dirty files in the tree.
