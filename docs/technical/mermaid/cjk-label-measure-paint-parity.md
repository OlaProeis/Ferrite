# CJK label measure vs paint parity (ISS-59)

GrokBot MMD-13 reported flowchart node boxes on FC-165a roughly **1.63–1.71×** the painted label width for pure-CJK labels (e.g. `堡垒机运维系统` 196 px box vs 116 px text). Task 17 checked whether `EguiTextMeasurer` diverges from the egui paint path before changing layout.

## Implementation

**Key file:** `src/markdown/mermaid/text.rs`

`EguiTextMeasurer` lays out through **`Ui::painter()`** — the same entry point node labels use when drawing:

- `measure`: `painter().layout_no_wrap(..., FontId::proportional(font_size), ...)`, size from `galley.size()`
- `measure_wrapped`: `painter().layout_job(LayoutJob::simple(...))`, same size fields

Flowchart nodes still paint with `FontId::proportional(font_size)` in `flowchart/render/nodes.rs`.

## Tests

**Module:** `src/markdown/mermaid/measure.rs` (included from `mermaid/mod.rs` under `#[cfg(test)]`).

```bash
cargo test --bin ferrite mermaid::measure
```

| Test | Purpose |
|------|---------|
| `test_measure_cjk_label_width_matches_painted_galley` | FC-165a CJK string; lazy CJK load + second frame; measurer width vs painter galley ≤ **1.1×** |
| `test_measure_latin_label_width_matches_painted_galley` | Latin control, same parity bar |
| `test_measure_flowchart_node_width_tracks_measurer_not_byte_estimate` | Single-node layout with `EguiTextMeasurer` vs `EstimatedTextMeasurer`; real layout follows measurer + padding; byte estimate stays much wider for pure CJK |

## Findings (task 17)

| Question | Result |
|----------|--------|
| Measurer vs painted galley (same `FontId::proportional`) | **Within 1.1×** after CJK fonts are loaded — no separate font family on the measure path |
| GrokBot box vs text ~1.6–1.7× | **Mostly layout chrome**, not glyph width error: node width uses measured text × **`text_width_factor` 1.15** plus **`node_padding.x` 24×2** (`flowchart/layout/mod.rs` / `graph.rs`). That yields ~**1.56×+** over raw galley width before any min-width floor |
| Headless / estimate path | `EstimatedTextMeasurer` still uses UTF-8 **byte length** × factor — heavily over-width for CJK. Render skips cache entries flagged `estimated` and re-layouts with `EguiTextMeasurer` (see `mermaid/mod.rs`, `cache.rs`) |

Tightening boxes to hug CJK labels like mermaid.live would be **layout policy** (padding / `text_width_factor`), not a measurer one-liner — out of scope for task 17.

## Related docs

- `mermaid-text-measurement.md` — trait, pre-measure pattern, safety multiplier
- `inline-fit-to-pane.md` — when to pass `EguiTextMeasurer` into `measure_mermaid_diagram`
- `prd-v0.3.1-mermaid-fidelity.md` §12 ISS-59 — QA hand-off and decision table
