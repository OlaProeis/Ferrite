# Tab / Shift+Tab indent selection (#177)

Manual fixture for Raw-editor block indent/outdent. Open in **Raw** view.

## Block 1 — plain lines (select all three, press Tab, then Shift+Tab)

alpha line one
beta line two
gamma line three

## Block 2 — already-indented lines (Shift+Tab should outdent one step at a time)

    indented four spaces
        indented eight spaces
	indented with a hard tab

## Block 3 — mixed content (indent must apply per line, one step each)

- list item one
- list item two
  - nested item

## Block 4 — short lines / edge cases

x

(line above has a single char; line below is empty — outdent at column 0 must be a no-op)

## Block 5 — multi-byte characters (no panics, correct columns)

§ section start
ß sharp s
中文行首
émigré

## What to check

- [ ] Select 3 lines in Block 1 → Tab indents all three one step; Shift+Tab restores exactly.
- [ ] Settings → Use spaces ON, tab size 4 → indent inserts 4 spaces; OFF → inserts `\t`.
- [ ] Block 2: Shift+Tab removes one indent step per press; stops at column 0.
- [ ] Empty caret (no selection) → Tab inserts one indent step; focus stays in the editor.
- [ ] Multi-cursor (Ctrl+Click on several lines) → Tab indents at every cursor.
- [ ] One Ctrl+Z undoes an entire block indent in one step.
- [ ] Block 5: indent/outdent on multi-byte lines — no crash, no garbled characters.
- [ ] Rendered-view list Tab (indent list item) still works — raw indent must not break it.
