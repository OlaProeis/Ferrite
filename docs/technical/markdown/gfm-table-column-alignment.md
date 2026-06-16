# GFM Table Column Alignment (Rendered View)

**Version**: v0.3.1 · Issue [#140](https://github.com/OlaProeis/Ferrite/issues/140)

## Overview

GitHub Flavored Markdown tables declare per-column alignment in the delimiter row (`:---`, `:---:`, `---:`). Ferrite parses these into `TableAlignment` on the AST table node and honors them in the rendered `EditableTable`: cell text is laid out left/center/right, toolbar buttons cycle alignment, and delimiter colons round-trip through edit commits.

Out of scope here: raw-mode column guides (task 27), CSV viewer alignment.

## Data flow

```
GFM source delimiter row
    → comrak Table.alignments (parser.rs)
    → MarkdownNodeType::Table { alignments, num_columns }
    → TableData::from_node() (widgets.rs)
    → EditableTable render + TableData::to_markdown() on commit
```

| Stage | Location | Notes |
|-------|----------|-------|
| Parse | `src/markdown/parser.rs` | `TableAlignment` enum; `From<ComrakTableAlignment>` |
| State | `TableData.alignments` | Resized to match `num_columns` on load |
| Render | `EditableTable::show()` | `build_table_cell_display_layout_job` + `table_cell_galley_paint_pos` |
| Serialize | `TableData::to_markdown()` | Delimiter row uses `:`, `:---:`, `---:` markers |
| Commit | `render_table()` in `editor.rs` | Alignment toolbar changes commit immediately (like add row/column) |

## Rendered alignment

`table_alignment_to_egui()` maps GFM alignment to egui `Align`:

| `TableAlignment` | egui `Align` | Delimiter |
|------------------|--------------|-----------|
| `None` | `LEFT` | `---` |
| `Left` | `LEFT` | `:---` |
| `Center` | `Center` | `:---:` |
| `Right` | `RIGHT` | `---:` |

Applied in three places per cell:

1. **Display mode** — `build_table_cell_display_layout_job` sets `job.halign` and `wrap.max_width = inner_w`, then paints via `table_cell_galley_paint_pos` (not raw `response.rect.min`).
2. **Edit mode** — `TextEdit` custom layouter sets `job.halign` on the plain-text `LayoutJob`.
3. **Click-to-cursor** — `table_cell_raw_cursor_at_click()` uses the same job builder and paint position as display mode.

### Display paint positioning

Direct `ui.painter().galley(response.rect.min, …)` ignores two offsets that egui widgets normally compensate:

| Issue | Fix |
|-------|-----|
| `LayoutJob.halign` Center/Right shifts glyph coordinates (`galley.rect.min` ≠ origin) | Subtract `galley.rect.min` when computing paint origin |
| `halign` aligns within content width, not the full cell | Add `table_cell_block_align_shift(cell_width, galley_width, alignment)` so short text centers/right-aligns inside `inner_w` |

```rust
// widgets.rs — shared by display paint and click-to-cursor
fn table_cell_galley_paint_pos(cell_rect, galley, alignment) -> Pos2 {
    let block_shift = table_cell_block_align_shift(cell_rect.width(), galley.size().x, alignment);
    cell_rect.min - galley.rect.min.to_vec2() + vec2(block_shift, 0.0)
}
```

Full-width wrapped cells get `block_shift = 0`; per-line center/right comes from `job.halign` within the galley. Left/`None` unchanged.

Column layout also uses `Layout::top_down(halign)` so cell chrome aligns with text.

## Toolbar

Alignment controls live in the `EditableTable` footer toolbar (Phosphor `TEXT_ALIGN_*` icons). Gated by `show_alignment_controls` (default `true`).

```rust
EditableTable::new(&mut table_data)
    .with_alignment_controls(true)  // editor.rs render_table()
    .show(ui);
```

Each column button cycles via `TableAction::CycleAlignment(col)` → `TableData::cycle_column_alignment()`:

`None → Left → Center → Right → None`

Toolbar clicks set `changed = true` and update source markdown immediately (not deferred like in-cell typing).

## Serialization and round-trip

`TableData::to_markdown()` writes the delimiter row from `alignments`, padding dashes to match column content width (minimum 3). Cell edits that commit on focus loss preserve alignment because `to_markdown()` always emits the current `alignments` vector.

`serialize_table()` in `widgets.rs` (AST → markdown for non-widget paths) uses fixed `:---` / `:---:` / `---:` markers.

## Tests

| Test | File | Covers |
|------|------|--------|
| `test_parse_table_with_alignment` | `parser.rs` | Parse delimiter row |
| `test_table_data_set_alignment` / `test_table_data_cycle_alignment` | `widgets.rs` | State API |
| `test_table_data_to_markdown_with_alignment` | `widgets.rs` | Delimiter markers in output |
| `test_table_alignment_roundtrip_after_edit` | `widgets.rs` | Edit + re-parse preserves alignments |
| `test_table_cell_block_align_shift` | `widgets.rs` | Block-level center/right shift math |

```bash
cargo test table_alignment
cargo test table
```

## Related documentation

- [Editable Tables](./editable-tables.md) — widget overview, deferred commits, keyboard navigation
- [Table Inline Formatting](./table-inline-formatting.md) — bold/italic/code inside cells
- [Rendered edit session (tables)](./rendered-edit-session-tables.md) — cross-block commit when leaving a table
