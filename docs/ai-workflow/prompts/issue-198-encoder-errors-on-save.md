# Fix GitHub issue #198 (Ferrite 0.3.1)

Paste this whole file into a new chat. Fix only this issue. Do not commit unless asked.

## Issue

https://github.com/OlaProeis/Ferrite/issues/198

Save ignores encoder errors. Unmappable characters are written as `&#NNNN;`.

`Tab::encode_content` in `src/state.rs` (about line 2663) calls `encoding.encode(&self.content)` and drops `had_errors`. For encodings other than UTF-8, encoding_rs replaces a character it cannot store with an HTML numeric reference. Saving a Windows-1252 or Shift-JIS file after typing `→` or an emoji writes `&#8594;` (or similar) and reports success.

UTF-16 is encoded by hand in the same function and is not this bug.

## Repo

- Path: `G:\DEV\markDownNotepad`
- Branch: `0.3.1-experimental`
- The working tree already has unrelated edits. Do not revert them, do not stash them, and do not commit them.

## What to change

Any path that writes the file (`save_tab_by_id` around line 5532, and the other `encode_content` callers that write bytes to disk) must stop when `had_errors` is true. Do not write the `&#NNNN;` bytes.

Ask the user:

- Save as UTF-8: set the tab encoding to UTF-8, write UTF-8 bytes, then mark saved as the normal save does.
- Cancel: leave the buffer, the encoding, and the file untouched.

Do not reuse `show_confirm_dialog` or `PendingAction`. That dialog is the close-tab / exit prompt in `src/app/dialogs.rs`. Add a separate flag and a small window from `render_dialogs`, named `show_encoding_lossy_save_dialog`, so it does not collide with issue #197 (that chat may add `show_encoding_switch_dialog` in the same file).

UTF-8 and the manual UTF-16 paths have no lossy replacement. They should save as they do now, with no dialog.

New strings go through `t!`. Add the keys to `locales/en.yaml` and the other `locales/*.yaml` files. English text is fine in the locales you are not translating. Follow the `dialog.unsaved_changes` keys for shape.

If you change `encode_content`'s return type, update every caller. Tests that only want bytes can keep a helper that still returns `Vec<u8>`, but the save path must not use a helper that hides `had_errors`.

## Tests

- Encoding `→` as windows-1252 reports an error and does not produce a buffer you would write.
- The same string as UTF-8 does not report an error.
- A save of a temp windows-1252 file that contains `→` does not modify the file when the user cancels. Follow whatever temp-dir pattern the existing save tests use.

## Docs

Add a short note under `docs/technical/files/` and one bullet in `docs/index.md`. #198. Say that a lossy legacy-encoding save is refused, with UTF-8 or cancel.

## Leave alone

- Issue #197 is the status-bar picker discarding unsaved edits. Do not change `set_encoding` here.
- Do not comment on GitHub.
- Do not mark tasks done.
- Do not revert other dirty files in the tree.
