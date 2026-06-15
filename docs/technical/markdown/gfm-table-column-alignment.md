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
| Render | `EditableTable::show()` | `table_alignment_to_egui()` → `LayoutJob.halign` |
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

1. **Display mode** — formatted galley (`build_inline_markdown_layout_job` / `build_cell_layout_job_with_base_bold`) sets `job.halign` before `layout_job`.
2. **Edit mode** — `TextEdit` custom layouter sets `job.halign` on the plain-text `LayoutJob`.
3. **Click-to-cursor** — `table_cell_raw_cursor_at_click()` uses the same halign when mapping pointer position to a raw caret index.

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

```bash
cargo test table_alignment
cargo test table
```

## Related documentation

- [Editable Tables](./editable-tables.md) — widget overview, deferred commits, keyboard navigation
- [Table Inline Formatting](./table-inline-formatting.md) — bold/italic/code inside cells
- [Rendered edit session (tables)](./rendered-edit-session-tables.md) — cross-block commit when leaving a table
