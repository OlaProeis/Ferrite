# Smart Paste for Links and Images

## Overview

Intelligent paste behavior that creates markdown links/images from URLs and saves OS clipboard image bytes under `./assets/`.

## Features

### 1. Link Creation with Selection
When text is selected and a URL is pasted:
- Select "Click here", paste `https://example.com`
- Result: `[Click here](https://example.com)`
- Cursor lands after the closing parenthesis

### 2. Image Insertion without Selection
When an image URL is pasted with no selection:
- Paste `https://example.com/pic.png`
- Result: `![](https://example.com/pic.png)`
- Cursor lands after the inserted image markdown
- **Note:** This only inserts markdown pointing at the URL — Ferrite does **not** download remote images.

### 3. Regular URL Paste
When a regular URL (non-image) is pasted with no selection:
- Normal paste behavior (URL inserted as-is)

### 4. Non-URL Paste
When non-URL text is pasted:
- Normal paste behavior

### 5. Clipboard image bytes (#164)
When the OS clipboard holds image pixels (screenshot, Copy image) and paste is requested without a competing non-empty text paste:
- Bytes are encoded as PNG under `./assets/` (same location rules as drag-drop)
- Markdown `![](assets/…)` is inserted at the caret
- Undo removes the markdown insert (file on disk remains)

See [`clipboard-image-paste.md`](clipboard-image-paste.md) for preview-lock / Raw / Split policy and helpers.

## Implementation

### Key Files
- `src/app/input_handling.rs` — `consume_smart_paste`, `try_consume_clipboard_image_paste`, `is_url`, `is_image_url`
- `src/app/file_ops.rs` — assets helpers and clipboard/drag-drop save + markdown insert
- `src/editor/widget.rs` — raw editor context-menu Paste (text, then image)

### Helper Functions

```rust
/// Check if a string looks like a URL
fn is_url(s: &str) -> bool

/// Check if a URL points to an image based on file extension
fn is_image_url(s: &str) -> bool
```

### URL Detection
URLs are detected by checking for:
- `http://` or `https://` prefix
- General scheme pattern: `[a-zA-Z][a-zA-Z0-9+.-]*://`

### Image URL Detection
Image URLs are detected by checking file extensions (case-insensitive):
- `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg`, `.bmp`, `.ico`, `.tiff`, `.tif`
- Query strings and fragments are stripped before extension check

### Architecture

The implementation uses a pre-render event consumption pattern:

**Pre-render Phase** (`consume_smart_paste`):
1. **Early gate** — return immediately unless the frame has `Event::Paste` or Ctrl/Cmd+V (`events_indicate_paste`); avoids cloning the document every frame.
2. Try clipboard image bytes first (`try_consume_clipboard_image_paste`).
3. Query FerriteEditor for authoritative selection + caret (line/col and selection char range via `cursor_to_char_pos`).
4. Scan egui input events for `Event::Paste(text)`.
5. If URL + selection → create markdown link, consume event.
6. If image URL + no selection → create markdown image URL syntax, consume event.
7. Otherwise → let normal paste proceed.

This approach:
- Intercepts paste events before TextEdit / FerriteEditor processes them
- Only consumes events when smart behavior applies
- Falls through to default paste for normal text cases
- Image-only clipboards often produce no `Event::Paste`; the paste shortcut is detected explicitly

### UTF-8-safe link replacement

URL-over-selection no longer searches a ±20 **byte** window around the caret (that slice could start/end mid-codepoint and abort in release builds).

Instead:
1. Read the primary selection’s **char-index** range from FerriteEditor (`primary_selection().ordered()` → `cursor_to_char_pos`).
2. Convert start/end chars to byte offsets with `char_index_to_byte_index` (`string_utils.rs`).
3. Replace exactly that byte range via `replace_selection_with_link` (no `str::find` window).

Helpers live at the top of `src/app/input_handling.rs`: `replace_selection_with_link`, `cursor_byte_from_line_col`, `events_indicate_paste`.

Image URL insertion (no selection) uses `cursor_byte_from_line_col` for the same char-column → byte mapping.

### Auto-close bracket gate

`handle_auto_close_pre_render` clones tab content only when `events_indicate_auto_close` sees a relevant single-char `Event::Text` (opener with selection, or closer for skip-over). No per-frame clone on idle frames.

### Integration Points
The `consume_smart_paste` function is called in `update()`:
```rust
// IMPORTANT: Handle smart paste BEFORE rendering to intercept paste events
// and transform them into markdown links/images when appropriate.
self.consume_smart_paste(ctx);
```

## Testing

| Scenario | Expected Behavior |
|----------|-------------------|
| Select "Click here", paste URL | `[Click here](https://example.com)` |
| Select word beside CJK text, paste URL | Link created, no crash (char-index replace) |
| No selection, paste image URL | `![](https://example.com/pic.png)` (no fetch) |
| No selection, paste regular URL | Plain URL inserted |
| Paste non-URL text | Normal paste behavior |
| Paste OS clipboard image | File under `assets/` + `![](assets/…)` |
| Undo after smart / clipboard image paste | Restores buffer (markdown removed) |
| URLs with query strings | Detection still works |
| URLs with fragments | Detection still works |

### Supported Image Extensions (URL paste)
- PNG: `.png`
- JPEG: `.jpg`, `.jpeg`
- GIF: `.gif`
- WebP: `.webp`
- SVG: `.svg`
- BMP: `.bmp`
- ICO: `.ico`
- TIFF: `.tiff`, `.tif`

### Edge Cases
- URLs with query strings: `https://example.com/pic.png?v=1` → detects as image
- URLs with fragments: `https://example.com/pic.png#section` → detects as image
- Mixed case extensions: `https://example.com/pic.PNG` → detects as image
- Non-standard schemes: `ftp://`, `file://` → detected as URLs
- Mixed-script paste after `:` must not panic (`str::get` for `://` check)

## Undo Behavior
- Smart link creation: Undo removes the entire link and restores selection
- Smart image URL insertion: Undo removes the image markdown
- Clipboard image paste: Undo removes the inserted `![](assets/…)` (disk file kept)
- Operations are recorded as single edits for clean undo

## Future Enhancements
- Auto-fetch image titles for alt text
- Smart paste for other URL types (YouTube → embedded video placeholder)
- Configurable behavior per URL pattern
