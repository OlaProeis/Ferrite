# Inline display-to-raw spans (ISS-52)

## Problem

Formatted-block painting (`parse_inline_markdown`) drops delimiters and decodes HTML entities, so a caret in the visible string is not a byte index into the source line. Click-to-edit needs that mapping. A legacy char walker in `map_displayed_to_raw` only adjusted for entities and images; hidden `**` / link / code delimiters earlier on the line caused ~10-character raw caret drift on mixed-format lines.

## Solution

`src/markdown/widgets.rs`:

- `InlineDisplaySpan` carries `raw_range` (bytes into the source line), `display_range` (chars into concatenated visible text), `is_link`, and `hidden`.
- `parse_inline_display_spans` follows the same decisions as `parse_inline_markdown` and emits hidden zero-width spans for `***`, `**`, `*`, `~~`, backticks, `[` / `](url)`, angle-bracket autolink brackets, and wikilink wrappers. Raw ranges tile the line with no gaps.
- HTML entities (`&amp;` and the rest of `consume_leading_entity`) are one visible glyph whose `raw_range` covers the whole entity. Code spans and plain link text (no nested markup) stay literal.
- Table-cell images are one visible span (icon + alt) whose raw range is the whole `![alt](url)`.
- `display_to_raw(spans, display_idx)` returns the raw **byte** for a display char index. `build_display_to_raw_table(line)` lists every caret from 0 through the end of the display.
- `collect_inline_display_spans` returns the non-hidden spans (ISS-26 link tests).

### Click-to-edit caret mapping

- `map_displayed_to_raw_one_line` / `map_displayed_to_raw` call `parse_inline_display_spans` + `display_to_raw`, then convert the raw byte to a **char index** for `egui::CCursor` / session TextEdit.
- Multi-line session buffers (`\n` in the block text): walk raw lines with per-line span mapping; helpers `display_start_for_raw_line` and `raw_char_start_for_raw_line` offset into the full buffer.
- **Table cells:** `table_cell_raw_cursor_at_click` — galley `cursor_from_pos` → `map_displayed_to_raw` on `cell.text`.
- **Formatted paragraphs / list items:** `FormattedBlockLayout::raw_cursor_at` in `rendered_session.rs` — same galley as paint; single-line blocks use global display index → `map_displayed_to_raw`. When the buffer contains `\n`, resolve the galley row from click Y, pick the matching raw line, then apply the per-line table (see [formatted layout](./rendered-edit-session-formatted-layout.md)).

Modifier and middle-click on those links open the target before click-to-edit. See [formatted link open](./formatted-link-open.md).

## Tests

| Test | Coverage |
|------|----------|
| `inline_display_to_raw_maps_tail_entity_and_plain` | Tail after `**b** *i* \`c\` [l](u)`, `&amp; x`, plain identity |
| `map_displayed_to_raw_mixed_format_tail_matches_raw` | UX-8-style line — display index of `tail` → raw char index of `tail` |
| `map_displayed_to_raw_link_label_and_entity_outside_markup` | Link label + decoded entity outside markup |
| `map_displayed_to_raw_plain_line_is_identity` | No inline markup |
| `map_displayed_to_raw_entity_in_code_span_is_literal` / `_in_link_text_is_literal` | Entities inside `` ` `` / `[...]` stay literal |
| `map_displayed_to_raw_skips_table_cell_image_markdown` | `![alt](url)` fallback glyph |
| `formatted_block_layout_raw_cursor_*`, `rs2_link_paragraph_cursor_maps_inside_link` | Paragraph path via `raw_cursor_at` |
