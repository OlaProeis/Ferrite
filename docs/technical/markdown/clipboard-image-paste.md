# Clipboard image paste (#164)

Paste OS clipboard image bytes into a markdown document: save a PNG under the document’s `assets/` folder and insert `![](assets/…)` at the caret. Does **not** fetch remote `https://` images.

## Behaviour

| Context | Result |
|---------|--------|
| **Ctrl/Cmd+V** with image pixels on clipboard (no non-empty text paste) | Encode RGBA → PNG → `assets/` → insert markdown; single undo entry removes the insert |
| **Ctrl/Cmd+V** with non-empty `Event::Paste` text | Existing smart-paste / normal text paste wins (URLs stay links; no network fetch) |
| **Raw / Split** | Always inserts into the source buffer |
| **Rendered + preview lock** | No mutation (preview-targeted paste blocked) |
| **Rendered unlocked** | Inserts into document content |
| **Raw context menu → Paste** | Prefers text; if none, same assets + markdown path |
| **Special / image / PDF tabs** | No-op |

`assets/` writes use `assets_dir_for_write` (same as drag-drop): beside the saved document, else workspace root. Pathless tab + no workspace → toast `image_paste.save_document_first` and **no** CWD `assets/` write.

## Key code

| Piece | Location |
|-------|----------|
| Smart-paste + clipboard branch | `src/app/input_handling.rs` — `consume_smart_paste` → `try_consume_clipboard_image_paste` |
| Save / insert | `src/app/file_ops.rs` — `handle_clipboard_image_paste`, `insert_asset_image_markdown` |
| Shared helpers | `encode_rgba_to_png`, `write_image_bytes_to_assets`, `unique_asset_dest_path`, `save_clipboard_rgba_to_assets`, `assets_dir_for_write` |
| Raw context menu | `src/editor/widget.rs` — Paste action |
| Clipboard API | `arboard::Clipboard::get_image` (RGBA); encode via `image` crate PNG |

Drag-drop reuse: `handle_dropped_image` uses the same `ensure_assets_dir` / `insert_asset_image_markdown` helpers.

## Related docs

- URL / link smart paste: [`smart-paste.md`](smart-paste.md)
- Drag-drop + Rendered preview of `assets/` paths: [`local-image-assets.md`](local-image-assets.md)

## Tests

`src/app/file_ops.rs` module `image_assets_tests`:

- `encode_rgba_to_png_roundtrips_dimensions` / length mismatch
- `save_clipboard_rgba_writes_under_assets_and_markdown` (temp dir)
- `insert_markdown_at_line_col_inserts_and_places_cursor`
- `assets_dir_for_paths_prefers_document_parent`
