# Tab / Shift+Tab indent and outdent in Raw editor (#177)

## Behaviour

| Input | Selection | Result |
|-------|-----------|--------|
| **Tab** | Empty caret | Inserts one indent step at each cursor (`use_spaces` → `tab_size` spaces, else `\t`) |
| **Tab** | Any non-empty range (single- or multi-line) | Prepends one indent step to every line touched by the selection; the selected text is **never replaced** |
| **Shift+Tab** | Any non-empty range | Removes one outdent step from each line (no-op when line has no leading indent) |
| **Shift+Tab** | Empty caret | Outdents the line containing the caret (no-op at column 0) |
| **Multi-cursor** | Mixed | Ranges block-indent/outdent their lines; collapsed carets insert at the caret / line-outdent separately |

Outdent order per line (ISS-51 / IND-2): if the line starts with `\t`, remove exactly one tab character; otherwise remove up to `tab_size` leading spaces when `use_spaces` is on. Mixed selections (spaces on some lines, a hard tab on another) outdent each line once. Tab-insert mode (`use_spaces` off) still outdents leading `\t` only—lines indented with spaces are unchanged.

**Single-line ranges (QA ISS-67 / T1-20, round 3):** until round 3 only *multi-line* ranges were treated as block indents; a selection inside one line was handed to `insert_text_at_all_cursors`, which deleted the selected text and inserted whitespace (`alpha line one` became four spaces). `FerriteEditor::is_line_indent_range` now accepts every `Selection::is_range()`, matching VS Code / Sublime: Tab on a selected word indents the line. Collapsed carets are unchanged (a caret mid-line still inserts mid-line — `中文行|首` + Tab → `中文行    首`, by design).

Tab is consumed in the raw Ferrite editor so focus does not cycle to other widgets.

Rendered-mode list Tab / Shift+Tab is **not implemented** (`check_structural_keys` / `indent_list_item` exist in `markdown/editor.rs` + `ast_ops.rs` but are never wired; QA ISS-71, deferred to 0.3.2).

## Settings

- `settings.use_spaces` — soft tabs vs hard tab character
- `settings.tab_size` — spaces per indent step (1–8)

Wired each frame via `EditorWidget::tab_settings()` → `FerriteEditor::set_tab_settings()`.

## Implementation

### Helpers — `src/editor/ferrite/indent.rs`

- `indent_string(use_spaces, tab_size)` — one step as `String`
- `indent_line(line, …)` — prepend step, preserve trailing `\r`/`\n`
- `outdent_line(line, …)` — remove one leading step; returns `(new_line, removed_char_count)`

Leading `\t` is stripped first (one char), regardless of `use_spaces`. If there is no leading tab and `use_spaces` is true, outdent removes `min(leading_spaces, tab_size)` spaces. If `use_spaces` is false and there is no leading tab, the line is unchanged.

### Editor — `src/editor/ferrite/editor.rs`

- `handle_tab_key(shift)` — routes Tab / Shift+Tab for all selections
- `is_line_indent_range(selection)` — `selection.is_range()`; any non-empty range is a line indent (round-3 change)
- `block_indent_or_outdent(outdent)` — line-range block edit; adjusts anchor/head columns per line delta
- `outdent_lines_for_selection_indices(indices)` — line outdent for caret or single-line cursors

Block edits set `content_dirty` once per keypress so undo records one logical edit (existing Tab snapshot flow).

### Key handler

The `egui::Key::Tab` branch (~2851) calls `handle_tab_key(modifiers.shift)` instead of inserting a literal `\t`.

## Tests

Unit tests in `src/editor/ferrite/indent.rs`:

- Spaces vs tabs, outdent at column 0, partial soft-tab outdent
- IND-2: `\tfoo` → `foo`, `\t\tfoo` → `\tfoo`, leading tab with `use_spaces` true; mixed multi-line block outdent; caret column shift after tab outdent
- Multi-line block indent/outdent
- Empty caret insert (spaces and tab char)
- Multi-cursor block indent
- Shift+Tab no-op at column 0
- ISS-67 / T1-20: `test_single_line_selection_tab_indents_line_instead_of_replacing`, `test_partial_single_line_selection_tab_indents_line`; mixed multi-/single-line ranges both indent their lines

Manual: select three lines → Tab indents all; Shift+Tab restores; empty caret inserts per settings; Tab does not move focus away from the editor.

See also: [`find-replace.md`](find-replace.md), [`word-wrap.md`](word-wrap.md).
