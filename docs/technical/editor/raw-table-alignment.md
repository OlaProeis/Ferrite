# Raw-Mode GFM Table Column Guides

## Overview

Display-only vertical guides in the raw markdown editor that align with GFM table pipe (`|`) boundaries. Guides help visually scan column alignment while editing table source; they never modify the buffer.

Rendered-view column alignment editing is separate — see [GFM Table Column Alignment](../markdown/gfm-table-column-alignment.md).

## Key Files

| File | Role |
|------|------|
| `src/editor/ferrite/table_guides.rs` | Table detection, pipe-column math, cache, paint |
| `src/editor/ferrite/editor.rs` | Calls `render_table_guides` before the text loop; owns `TableGuideCache` |
| `src/editor/ferrite/mod.rs` | Module export |

## When Guides Appear

- **Syntax language:** `md` or `markdown` only (via FerriteEditor `syntax_language`).
- **Word wrap:** disabled — wrapped lines break pipe-column alignment, so guides are skipped.
- **Scope:** visible viewport only; table blocks that start above the viewport are extended backward so guides span the full table height.

## Detection

`detect_table_ranges` scans the visible line range and finds contiguous GFM table blocks:

1. Skips lines inside fenced code blocks (```).
2. Requires at least two consecutive table lines (`is_table_line`).
3. Qualifies when the second line is a delimiter row (`is_delimiter_row`, e.g. `| --- | --- |`) **or** when three or more table lines are present (ragged tables).

Pipe positions for guides come from the **header row** (first line of the block) via `pipe_char_columns`.

## Rendering

Draw order in `FerriteEditor::ui`:

1. Gutter background
2. **Table column guides** (behind text)
3. Text galleys, selections, search highlights, cursors

X positions are computed from header-row pipe character columns using `Painter::layout_no_wrap` on the prefix up to each pipe. Horizontal scroll is applied the same way as non-wrapped text.

Guide color: `weak_text_color` at ~30% alpha.

## Cache

`TableGuideCache` stores pipe column indices keyed by `(start_line, blake3 hash of table lines)`.

| Event | Cache behavior |
|-------|----------------|
| Edit within a table | Content hash changes → cache miss → recompute |
| Font / zoom change | Full cache clear |
| `set_content` / full dirty invalidation | Full cache clear |

Per-frame cost is O(visible table blocks); no full-buffer scan.

## Out of Scope

- Column alignment editing (rendered view — task 20)
- Auto-formatting table source
- Guides when word wrap is on

## Tests

Unit tests in `table_guides::tests`:

- `is_table_line` / `is_delimiter_row`
- Aligned and ragged table range detection
- Code-fence exclusion
- Viewport backward extension
- Cache invalidation on content change

Run: `cargo test table_guides`

## Manual Verification

1. Open a `.md` file in **Raw** mode with word wrap off.
2. Confirm faint vertical lines at `|` columns over table rows only.
3. Edit a cell — guides should update without lag.
4. Enable word wrap — guides should disappear.
5. Put a fake table inside a fenced code block — no guides over it.
