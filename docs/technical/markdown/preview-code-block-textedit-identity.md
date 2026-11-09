# Preview code block TextEdit identity & focus (ISS-40 part 2)

Rendered/Split fenced code blocks keep a typing draft in `CodeBlockData` until Done or blur. Part 2 fixes dropped or garbled keystrokes caused by unstable egui ids and per-frame resets from parsed source.

**Modules:** `src/markdown/rendered_session.rs` (`BlockRef::CodeBlock`, `widget_id_in_scope`), `src/markdown/editor.rs` (`render_code_block`, `sync_codeblock_data_from_parsed`), `src/markdown/widgets.rs` (`EditableCodeBlock`, `pending_text_edit_focus`)

## Widget id hierarchy

Inside `show_rendered_editor` (`push_id(editor_id)` + `push_id(source_epoch)`):

| Piece | Id |
|-------|-----|
| TextEdit | `BlockRef::CodeBlock { start_line, end_line }.widget_id(ui)` → `…codeblock_text…` |
| Chrome (scroll, lang combo, frame salt) | `ui.make_persistent_id(("codeblock_chrome", start_line, end_line))` |

Do **not** use global `egui::Id::new(("codeblock", line))` for the TextEdit — it collides across tabs/panes and can remap focus when layout changes.

See [rendered widget identity](./rendered-widget-identity.md) for the general `source_epoch` rules.

## Draft vs parsed literal

While `is_editing` or the TextEdit has focus, `sync_codeblock_data_from_parsed` **must not** replace `CodeBlockData` when `code != literal` (every keystroke diverges from committed fence text until commit). External/raw sync and preview-lock reset still reload from `literal`.

## Focus & session

- **Edit** sets `pending_text_edit_focus` → one `request_focus` on the next paint (not every frame).
- While the TextEdit has focus: stay in edit mode; `RenderedEditSession.active` may be `BlockRef::CodeBlock` (commits other session blocks on entry); container arrow-nav is suppressed like table cells.
- Session end: `log::debug` with reason (`blur`, `done_or_commit`, `preview_lock`, `reparse`).

## Related

- [Preview code block undo](./preview-code-block-undo.md) — one undo group per session
- [Rendered session write-through](./rendered-session-write-through.md) — code blocks stay pending until commit

## Tests

```bash
cargo test code_block_ten_keystroke
cargo test code_block_session
```
