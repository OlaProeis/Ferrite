# Find scroll in Rendered / Split preview (#175)

## Problem

Ctrl+F in Rendered (and Split preview) showed match counts and highlights, but Enter / F3 did not advance the preview viewport to the current match. Raw mode already scrolled. Focusing Find also appended to the previous query instead of selecting it for replace-as-you-type.

## Behaviour

1. **Rendered / Split preview:** When `UiState.scroll_to_match` is set (find next/prev, Enter, F3, or a fresh search with hits), the preview scrolls so the current match is visible (~¼ viewport from the top).
2. **Highlights:** `search_highlights` + `current_match` stay in sync with `FindState`.
3. **Ctrl+F focus:** The search field focuses and **selects all** existing query text (`find_query_select_all_char_range`).
4. **Raw:** Unchanged — `EditorWidget` + `SearchHighlights.scroll_to_match` still scroll the raw editor.

## Match → scroll mapping

| Step | Where | What |
|------|--------|------|
| Byte → 1-indexed line | `search_match_source_line` / `source_byte_to_line_1indexed` in `src/markdown/editor.rs` | Current match start byte → source line |
| Prefer accurate Y | `central_panel.rs` + `find_rendered_y_for_line_interpolated` | Uses `tab.rendered_line_mappings` when present |
| Fallback | `MarkdownEditor::scroll_to_search_match` | Reuses outline-style `scroll_to_line` offset (`line × row_height − 0.25 × viewport`) |

Find scroll takes priority over sync pending offsets. After a find jump, sync `pending_ratio` must not rewrite `pending_scroll_offset` on the next frame (would undo the jump).

Split persists preview `rendered_line_mappings` / height metrics on the tab each frame so the next Find cycle can use interpolated Y.

## Select-all on focus

`FindReplacePanel` (on `request_focus` / Ctrl+F) calls `TextEdit::show`, then sets an egui `CCursorRange` over the full query via `find_query_select_all_char_range` and stores the text-edit state.

## Key files

- `src/markdown/editor.rs` — helpers, `scroll_to_search_match`, highlight paint
- `src/app/central_panel.rs` — Rendered/Split wiring; sync vs find priority
- `src/app/find_replace.rs` — sets `scroll_to_match` on next/prev/open
- `src/editor/find_replace.rs` — panel select-all + `FindState`
- `src/state.rs` — `UiState.scroll_to_match`

## Tests

- `test_source_byte_to_line_1indexed_*`, `test_search_match_source_line_*`
- `test_find_query_select_all_char_range`

## Manual QA

Checklist row **UX-11** (`UX-find-scroll`) in [`v0.3.1-test-checklist.md`](../platform/v0.3.1-test-checklist.md).

See also: [`find-replace.md`](find-replace.md) for panel/search options.
