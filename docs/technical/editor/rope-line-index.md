# Rope-aligned line / column → character index

Go-to-line, clipboard image paste insert, smart-paste byte mapping, and line ops (duplicate / move / delete) must agree with **ropey**'s line-break rules — not `str::split('\n')` alone.

## Problem

Ropey treats several characters as line breaks (`\n`, `\r`, U+2028 line separator, U+2029 paragraph separator, vertical tab, form-feed, NEL). Code that split on `\n` only mis-counted lines and char indices for documents containing `\r`, U+2028, or form-feed, causing caret jumps and wrong paste/insert positions.

## API (`src/string_utils.rs`)

| Function | Role |
|----------|------|
| `rope_line_col_to_char_index(text, line, col)` | `(line, col)` → char index; `col` clamped to line content length |
| `rope_char_index_at_line_start(text, line)` | Char index at start of line `line` (0-based) |
| `rope_line_count(text)` | Line count per ropey |

`col` is a **character** column (matches `tab.cursor_position.1` and FerriteEditor).

## Call sites

| Area | File | Usage |
|------|------|-------|
| Smart-paste byte offset | `src/app/input_handling.rs` | `cursor_byte_from_line_col` |
| Image markdown insert | `src/app/file_ops.rs` | `insert_markdown_at_line_col` |
| Line duplicate / move / delete | `src/app/line_ops.rs` | `cursor_char_index` over split lines + EOL width |

Prefer `editor.cursor_char_index()` when a live FerriteEditor is available; fall back to `rope_line_col_to_char_index` when only `tab.cursor_position` is synced.

## Tests

- `string_utils::tests` — multibyte column clamp, U+2028 / form-feed line breaks
- `line_ops::tests::go_to_line_char_index_matches_ropey_breaks`
- `file_ops::image_assets_tests::insert_markdown_at_line_col_*`

## Related

- Raw buffer: `ropey::Rope` in FerriteEditor (`src/editor/ferrite/`)
- Line-ending preservation (CRLF dominant ending): [`line-ending-preservation.md`](../files/line-ending-preservation.md)
