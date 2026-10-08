# Tab / Shift+Tab indent and outdent in Raw editor (#177)

## Behaviour

| Input | Selection | Result |
|-------|-----------|--------|
| **Tab** | Empty caret | Inserts one indent step at each cursor (`use_spaces` → `tab_size` spaces, else `\t`) |
| **Tab** | Multi-line range | Prepends one indent step to every line touched by the selection |
| **Shift+Tab** | Multi-line range | Removes one outdent step from each line (no-op when line has no leading indent) |
| **Shift+Tab** | Empty caret | Outdents the line containing the caret (no-op at column 0) |
| **Multi-cursor** | Mixed | Multi-line selections block-indent/outdent; single-line cursors insert or line-outdent separately |

Tab is consumed in the raw Ferrite editor so focus does not cycle to other widgets.

Rendered-mode list Tab / Shift+Tab (structural list indent) is unchanged (`markdown/editor.rs`).

## Settings

- `settings.use_spaces` — soft tabs vs hard tab character
- `settings.tab_size` — spaces per indent step (1–8)

Wired each frame via `EditorWidget::tab_settings()` → `FerriteEditor::set_tab_settings()`.

## Implementation

### Helpers — `src/editor/ferrite/indent.rs`

- `indent_string(use_spaces, tab_size)` — one step as `String`
- `indent_line(line, …)` — prepend step, preserve trailing `\r`/`\n`
- `outdent_line(line, …)` — remove one leading step; returns `(new_line, removed_char_count)`

Soft-tab outdent removes `min(leading_spaces, tab_size)` spaces. Hard-tab outdent removes a single leading `\t` only.

### Editor — `src/editor/ferrite/editor.rs`

- `handle_tab_key(shift)` — routes Tab / Shift+Tab for all selections
- `block_indent_or_outdent(outdent)` — multi-line block edit; adjusts anchor/head columns per line delta
- `outdent_lines_for_selection_indices(indices)` — line outdent for caret or single-line cursors

Block edits set `content_dirty` once per keypress so undo records one logical edit (existing Tab snapshot flow).

### Key handler

The `egui::Key::Tab` branch (~2851) calls `handle_tab_key(modifiers.shift)` instead of inserting a literal `\t`.

## Tests

Unit tests in `src/editor/ferrite/indent.rs`:

- Spaces vs tabs, outdent at column 0, partial soft-tab outdent
- Multi-line block indent/outdent
- Empty caret insert (spaces and tab char)
- Multi-cursor block indent
- Shift+Tab no-op at column 0

Manual: select three lines → Tab indents all; Shift+Tab restores; empty caret inserts per settings; Tab does not move focus away from the editor.

See also: [`find-replace.md`](find-replace.md), [`word-wrap.md`](word-wrap.md).
