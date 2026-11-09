# Diagnostic squiggles with word wrap (ISS-48)

LSP diagnostics and spellcheck both reach the raw editor as `DiagnosticEntry` values and are painted by `FerriteEditor::render_diagnostic_squiggles` in `src/editor/ferrite/highlights.rs`. Mermaid inline validation uses the same path (`severity: Hint`, `source: "spell"` or Mermaid messages).

## Problem

With word wrap on, line text is laid out with a **wrapped** `egui::Galley` (`LineCache::get_galley_wrapped` / syntax-highlighted wrap). Squiggles previously measured X with `layout_no_wrap` on prefix substrings. That matches non-wrapped paint only, so underlines appeared about one wrap-width to the right of the word (GrokBot SPL-7 / ISS-48).

## Fix

Positioning lives in `src/editor/ferrite/rendering/squiggle.rs`:

| API | Role |
|-----|------|
| `squiggle_segments_for_columns` | Char column range on one logical line → screen segments |
| `paint_squiggle_wave` | Wavy underline between `x_start` and `x_end` at `baseline_y` |

When wrap is active (`wrap_enabled && wrap_width > 0 && view.is_wrap_enabled()` — same effective wrap as the paint pass):

1. Build a wrapped galley for the line (plain `painter.layout`, same width as the editor).
2. Map `col_start` / `col_end` with `galley.layout_from_cursor` → row + column within row.
3. Use `row.x_offset(column)` for X (same approach as search range highlights in `highlights.rs` and the caret in `rendering/cursor.rs`).
4. Baseline Y is `line_top_y + row.rect().max.y - 2.0` per visual row.
5. Spans crossing a wrap boundary split into one segment per row.

Non-wrapped mode keeps prefix `layout_no_wrap` plus `horizontal_scroll`, with a single baseline from `line_height`.

`render_diagnostic_squiggles` loops diagnostics and visible logical lines; column ranges remain 0-indexed **char** columns from `DiagnosticEntry` (spell tokenizer and LSP agree on this for raw lines).

## Tests

Egui-context unit tests in `squiggle.rs` (narrow forced wrap width, word on second visual row, unwrapped single segment, multi-row span split):

```bash
cargo test --bin ferrite squiggle
```

## Related

- Spellcheck pipeline and 300-diagnostic cap: `docs/technical/editor/spellcheck.md`
- Mermaid raw squiggles: `docs/technical/mermaid/mermaid-inline-validation.md`
