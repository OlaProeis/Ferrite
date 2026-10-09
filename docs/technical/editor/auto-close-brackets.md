# Auto-close Brackets & Quotes

## Overview

Task 4 implements automatic insertion of closing brackets and quotes while typing, with selection wrapping and skip-over behavior.

## Features

### 1. Auto-pair Insertion
When typing an opener character without a selection, automatically insert the closing pair:
- `(` → `()`
- `[` → `[]`
- `{` → `{}`
- `"` → `""`
- `'` → `''`
- `` ` `` → ``` `` ```

Cursor is positioned between the pair.

### 2. Selection Wrapping
When text is selected and an opener is typed, wrap the selection:
- Select "hello", type `(` → `(hello)`
- Select "world", type `"` → `"world"`

Cursor moves to after the closing bracket.

### 3. Skip-over Behavior
When typing a closer and the next character is the same closer, move cursor forward instead of inserting a duplicate:
- `text|)` + type `)` → `text)|` (skipped over)

### 4. Smart Quote Handling
Quotes are not auto-closed after alphanumeric characters to avoid interference with contractions:
- `can't` won't trigger auto-close after the apostrophe

### 5. Type-through and fence / quote-run guards
`FerriteApp::should_insert_auto_closer` decides whether post-render (and the raw editor) may insert a closer. It refuses when:

- The character after the caret is already the closer (type-through). Typing that closer still skip-overs.
- The previous character is the same `` ` ``, `"`, or `'` (building a fence or a quote run). Typing `` ```mermaid `` therefore inserts one closer on the first backtick; the second type-throughs; later backticks are plain characters. At most one `Auto-close: Inserted` log per opener.
- The current line already has an odd count of that quote before the opener, so this character closes an existing span.

The raw editor calls the same predicate from `FerriteEditor::auto_close_pair_allowed`. Pre-render leaves the key as a normal insert when the predicate says no.

## Implementation

### Settings
- `Settings.auto_close_brackets: bool` - Default: `true`
- UI toggle in Settings → Editor → "Auto-close Brackets & Quotes"

### Key Files
- `src/config/settings.rs` - Setting definition
- `src/ui/settings.rs` - Settings UI checkbox
- `src/app.rs` - Core auto-close logic

### Architecture

The implementation uses a two-phase approach:

**Pre-render Phase** (`handle_auto_close_pre_render`):
1. Scan egui input events for bracket/quote characters
2. Handle skip-over: Consume closer events when next char matches
3. Handle selection wrapping: Consume opener events, modify content manually

**Post-render Phase** (`handle_auto_close_post_render` in `src/app/input_handling.rs`):
1. Detect if a single opener was just typed (`content` char-count grew by 1)
2. Insert the closer at `tab.cursors.primary().head`
3. Set `tab.pending_cursor_restore` to that char index (between opener and closer)

**Next raw frame** (`EditorWidget::show_inner` in `src/editor/widget.rs`):
1. After content sync and outline `scroll_to_line`, take `pending_cursor_restore`
2. Clamp to the char count, `set_cursor` on the tab and `FerriteEditor`
3. After paint, if the tab has a single cursor, write the editor selection back onto `tab.cursors` so the next post-render insert uses the caret that was just shown

`pending_cursor_restore` is also set by undo/redo, line ops, and skip-over/wrap. The widget is the consumer for all of them.

### Helper Functions
```rust
fn get_closing_bracket(opener: char) -> Option<char>
fn is_closing_bracket(ch: char) -> bool
fn should_insert_auto_closer(content: &str, cursor_after_opener: usize, opener: char) -> bool
```

## Testing

| Scenario | Expected |
|----------|----------|
| Type `(` with no selection | `()` inserted, cursor between |
| Type `(` with "hello" selected | `(hello)` with cursor after `)` |
| Cursor before `)`, type `)` | Cursor jumps over `)` |
| Disable setting, type `(` | Only `(` inserted |
| Type `'` after "can" | Only `'` inserted (apostrophe) |
| Type `` ```mermaid `` | One closer on the first backtick; text stays `` ```mermaid `` |
| Char-by-char `` ```mermaid\nflowchart TD\n `` (Raw line guard and Split full buffer) | Content and caret match the typed text; exactly three backticks |
| Type `(` then `)` | `()` with caret between after `(`, then caret after `)` with length still 2 |
| Undo after auto-close | Restores to before |

Regression coverage lives in `src/app/input_handling.rs` (`mermaid_fence_char_by_char_raw_and_split_keep_one_closer`, `paren_auto_close_places_caret_inside_and_type_through_skips`). The Raw simulator applies `should_insert_auto_closer` to the current line only, matching `FerriteEditor::auto_close_pair_allowed`. The Split simulator uses the full buffer, matching post-render auto-close.

## Undo Behavior

- Selection wrapping: Undo removes both brackets
- Auto-pair insertion: TextEdit's internal undo may handle this differently
- Skip-over: No undo needed (just cursor movement)

## Future Enhancements

- Multi-char pairs: `**`, `__` for markdown emphasis
- Triple backticks for code blocks
- Configurable pair mapping
