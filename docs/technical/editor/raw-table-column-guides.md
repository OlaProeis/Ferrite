# Raw GFM table column guides (ISS-61)

Display-only vertical guides in the **Raw** markdown editor at pipe (`|`) boundaries. They help scan column alignment while editing table source; they never modify the buffer.

Rendered-view table alignment is separate — see [GFM Table Column Alignment](../markdown/gfm-table-column-alignment.md).

## Key files

| File | Role |
|------|------|
| `src/editor/ferrite/table_guides.rs` | Detection, per-row x math, `guide_segments`, cache, paint |
| `src/editor/ferrite/editor.rs` | Calls `render_table_guides` before the text loop; owns `TableGuideCache`; supplies syntax galleys via `LineCache` |
| `src/editor/ferrite/mod.rs` | Module export |

## When guides appear

- **Syntax language:** `md` or `markdown` only (`FerriteEditor::syntax_language`).
- **Word wrap:** disabled — guides are not drawn when wrap is on (including uniform-height large-file mode where wrap is force-disabled for layout). Status/tooltip copy: `TABLE_GUIDES_DISABLED_WRAP_REASON` (`"table guides off while word wrap is on"`) via `table_guides_disabled_reason(wrap_enabled)`.
- **Scope:** visible viewport; table blocks that start above the viewport extend backward so the full block is detected.

## Detection

`detect_table_ranges` finds contiguous GFM table blocks in the scan range:

1. Skips fenced code blocks (```).
2. Requires at least two consecutive `is_table_line` rows.
3. Qualifies when the second line is a delimiter row (`is_delimiter_row`) **or** three or more table lines (ragged tables).

## Pipe x-positions and paint

Draw order in `FerriteEditor::ui`: gutter → **table guides** → text galleys (guides sit behind text).

| Font | Strategy |
|------|----------|
| **Proportional** (e.g. Inter) | For each table row in the block: `pipe_char_columns` on that row’s text, then x from that row’s galley with `Galley::pos_from_cursor(CCursor::new(col)).min.x` (same cached galley path as line paint, including syntax highlight). Vertical segments span **only that row’s** y-range via `guide_segments` / `TableGuideRowLayout`. |
| **Monospace** | Header row pipe columns only; one continuous vertical segment per pipe from first to last table line (char-aligned; header-derived x is exact). |

Horizontal scroll matches non-wrapped text: `text_start_x + x - horizontal_scroll`.

Guide color: `weak_text_color` at ~30% alpha.

## Pure helpers (tests)

- `guide_segments(rows)` — builds `TableGuideSegment { x, y_top, y_bottom }` from precomputed per-row pipe x and heights.
- `plan_table_guides(wrap_enabled, rows)` — empty segments + `disabled_reason` when wrap is on.
- `pipe_x_from_galley` — pipe char indices → x from a galley.

## Cache

`TableGuideCache` keys `(start_line, blake3 hash of table lines)` and stores header `pipe_columns` for the **monospace** path. Proportional mode recomputes per-row x each frame from galleys; hash still drives invalidation on edits.

| Event | Behavior |
|-------|----------|
| Edit in table | Hash miss → recompute |
| Font / zoom change | Full cache clear |
| Full buffer dirty | Full cache clear |

## Out of scope

- Rendered-view tables
- Auto-formatting table source
- Guides with word wrap on

## Tests

Unit tests in `table_guides::tests` (detection, cache, `guide_segments`, wrap disabled reason):

```bash
cargo test --bin ferrite table_guides
```

## Manual verification

1. Open a `.md` file in **Raw** with word wrap off.
2. On a wide/ragged table, body-row pipes align with guides (proportional font).
3. Switch to a monospace editor font — continuous guides from header pipes.
4. Enable word wrap — guides off; reason string available for UI.
5. Fake table inside a fenced code block — no guides.
