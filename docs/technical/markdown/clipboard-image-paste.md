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
| **Untitled tab, no workspace** (Ctrl/Cmd+V or context menu) | Save As dialog opens; once saved the image is pasted beside the new file. Cancel → toast `image_paste.save_document_first`, nothing written |
| **Special / image / PDF tabs** | No-op |

`assets/` writes use `assets_dir_for_write` (same as drag-drop): beside the saved document, else workspace root. Ferrite never writes a CWD `assets/`.

### Untitled tabs

A pathless tab with no workspace has nowhere to store the PNG, so `assets_dir_or_prompt_save` (`src/app/file_ops.rs`) runs the blocking Save As dialog first and continues with the paste when the tab has a path afterwards. If the dialog is cancelled or the save fails it toasts `image_paste.save_document_first` and returns `None`. Tabs that cannot be saved (special, viewer, loading) get the toast without a dialog.

The clipboard image is read **before** the dialog opens, so the pasted pixels are the ones on the clipboard when the user pressed paste.

The raw context menu lives in the editor widget, which cannot open dialogs. For this case it sets `EditorOutput::request_image_paste`; `central_panel` then calls `paste_clipboard_image_at_caret`, the same function Ctrl/Cmd+V uses. While that request is pending the clipboard's URL text is not pasted.

## Key code

| Piece | Location |
|-------|----------|
| Smart-paste + clipboard branch | `src/app/input_handling.rs` — `consume_smart_paste` → `try_consume_clipboard_image_paste` → `paste_clipboard_image_at_caret` |
| Save / insert | `src/app/file_ops.rs` — `handle_clipboard_image_paste`, `insert_asset_image_markdown` |
| Untitled Save As prompt | `src/app/file_ops.rs` — `assets_dir_or_prompt_save` |
| Shared helpers | `encode_rgba_to_png`, `write_image_bytes_to_assets`, `unique_asset_dest_path`, `save_clipboard_rgba_to_assets`, `assets_dir_for_write` |
| Raw context menu | `src/editor/widget.rs` — Paste action; `EditorOutput::request_image_paste` handoff for untitled tabs |
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
