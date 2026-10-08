# Inline Mermaid Fit-to-Pane Scaling

Wide inline Mermaid diagrams in the preview pane overflow behind a horizontal scrollbar when rendered at native layout size. Users expect diagrams to shrink to fit the pane width, matching mermaid.live behaviour ([GitHub #165a](https://github.com/OlaProeis/Ferrite/issues/165)).

Inline fit-to-pane measures each diagram's natural layout size, computes a width-only scale factor, and paints through a static `set_transform_layer` sublayer when the diagram is wider than the pane. Scaling is **paint-time only** — layout and cache keys stay tied to pane width.

Do **not** wrap inline fit in `egui::Scene`. `Scene::show` replaces the clip rect (diagrams paint over the status bar) and `register_pan_and_zoom` applies wheel delta as pan (one-tick scroll jitter). The interactive popup viewer still uses `Scene`.

## Scale model

| Symbol | Meaning |
|--------|---------|
| `pane_width` | Available width inside the Mermaid block content area |
| `natural_size` | Unscaled diagram bounds from `measure_mermaid_diagram` |
| `fit_scale` | `(pane_width / natural_width).min(1.0)` via `inline_fit_scale` |
| `MIN_FIT_SCALE` | `0.25` — below this threshold, fall back to horizontal scroll |

**Formula:**

```text
fit_scale = (pane_width / max(natural_width, ε)).min(1.0)
```

where `ε = f32::EPSILON` guards against division by zero.

**Never upscales:** diagrams narrower than or equal to the pane render at natural size (`fit_scale = 1.0`).

**Floor at 0.25 → scroll fallback:** when `fit_scale < MIN_FIT_SCALE`, the block does **not** render at 0.25× — it falls back to native size inside `ScrollArea::horizontal()` so text never becomes illegibly small. The effective scale range for transform-layer fit is `MIN_FIT_SCALE ≤ fit_scale < 1.0`.

**Height:** uniform aspect — display height is `natural_size.y * fit_scale` so following content flows naturally (preserves issue #129 `auto_shrink([false, true])` intent).

## Implementation

### Transform-layer paint (not Scene)

`show_inline_mermaid_diagram` in `src/markdown/widgets.rs` allocates `pane_width × (natural.y * fit_scale)`, then:

1. Capture `parent_clip = ui.clip_rect()` **before** creating the layer.
2. Build `to_global = translate(outer_rect.min) * scale(fit_scale)`.
3. `set_sublayer` + `new_child` with `max_rect` at natural size.
4. Clip in local space: `set_clip_rect(to_global.inverse() * (parent_clip ∩ outer_rect))` and `shrink_clip_rect(to_global.inverse() * parent_clip)`.
5. `ctx.set_transform_layer(layer_id, to_global)` — no `register_pan_and_zoom`.

Scrolled-out diagrams paint nothing outside the preview. Wheel over a fit diagram scrolls the document, not the layer.

The popup open gesture and expand button overlay use `ui.interact` on the diagram rect **after** the layer; they remain functional in fit mode.

### Rendering paths

- **`render_validated_mermaid`** — measures **once** with `EguiTextMeasurer::new(ui)` when fit mode is on, decides fit vs scroll fallback, passes `natural_size` into `show_inline_mermaid_diagram`.
- **`show_inline_mermaid_diagram`** — three branches:
  1. **Transform-layer fit** (`MIN_FIT_SCALE ≤ fit_scale < 1.0`): scaled sublayer as above. Renders via `render_mermaid_diagram(..., Some(pane_width))`.
  2. **Natural fit** (`fit_scale ≥ 1.0`, measurable): direct render with `set_max_width(pane_width)`.
  3. **Scroll fallback**: horizontal scroll area wrapping native render (`natural_size` is `None`).

### Measurement

`measure_mermaid_diagram(source, font_size, available_width, text_measurer: &dyn TextMeasurer) -> Option<Vec2>` in `src/markdown/mermaid/mod.rs`:

- Callers with a `Ui` must pass `EguiTextMeasurer` so sequence/CJK/title metrics match render. Tests may pass `EstimatedTextMeasurer`.
- **Flowchart/graph** — parse + layout via global cache (`CacheKey`: source hash, font size, width); size from `flowchart_diagram_size`. Cache insert is flagged `estimated` only when `text_measurer.is_estimated()`.
- **Sequence, pie, gantt, gitgraph** — type-specific natural-size helpers or fixed estimates.
- **Other types** — `None` → scroll fallback until a measurer exists.
- **Frontmatter title** — adds title height from the same measurer.

When the render path upgrades an estimated flowchart cache entry to a real-font layout, it `request_repaint()` so the next measure/paint uses the real size.

`insert_flowchart` skips LRU eviction when the key already exists (`contains_key`).

### Shared helpers

In `src/markdown/mermaid/mod.rs`:

- `inline_fit_scale`, `MIN_FIT_SCALE` — inline width-only fit
- `scene_fit_scale(viewport, scene_rect)` — uniform fit for interactive popup viewer
- `MIN_SCENE_ZOOM`, `MAX_SCENE_ZOOM` — popup zoom clamps

## Toggle

Each block defaults to **Fit width**. Users can switch to **Native size (scroll)** via a hover-revealed header toggle.

| Mode | Behaviour |
|------|-----------|
| **Fit width** (default) | Transform-layer fit when measurable and `fit_scale ≥ MIN_FIT_SCALE` |
| **Native size** | Always `ScrollArea::horizontal()` at native layout size |

Widget id mixes `rendered_editor_id` + start line + source hash. `MermaidBlockData` / last-good source stay at `(editor_id, start_line)` so typing does not drop the last successful render. Fit-mode bool is `block_id.with("fit_mode")` (default `true`). No cross-session persistence.

`render_validated_mermaid` sets:

```text
scroll_fallback = !fit_mode || natural_size.is_none() || fit_scale < MIN_FIT_SCALE
```

Full toggle UI details: [`inline-fit-native-toggle.md`](./inline-fit-native-toggle.md).

## Cache interaction

Blake3 cache keys on `(source, font_size, available_width)` — unchanged by inline fit.

Inline fit passes the same `pane_width` to both `measure_mermaid_diagram` and `render_mermaid_diagram` so layout matches measurement. Scale is applied only when painting on the transform layer.

See also [`mermaid-caching.md`](./mermaid-caching.md).

## Popup

The popup viewer (`src/ui/mermaid_popup.rs`) keeps its interactive `Scene` (`scene_fit_scale`, `MIN_SCENE_ZOOM`, `MAX_SCENE_ZOOM`). Open from an inline diagram passes `MermaidBlockOutput.layout_width` (the inline pane width) so flowchart layout matches the preview.

## Tests

Unit tests in `src/markdown/mermaid/mod.rs`:

- `test_inline_fit_scale_never_upscales`
- `test_inline_fit_scale_shrinks_wide_diagrams`
- `test_measure_flowchart_returns_positive_size`

Cache: `insert_existing_key_does_not_evict_other_entry` in `cache.rs`.

Manual QA on `test_md/test_mermaid_issue_165.md`: scroll a fit diagram half out of view → nothing painted outside the preview; wheel over it → no jitter; CJK sequence fills pane width; popup width matches the inline pane.

## Related

- [`inline-fit-native-toggle.md`](./inline-fit-native-toggle.md) — per-diagram Fit/Native toggle UI
- [`mermaid-caching.md`](./mermaid-caching.md) — flowchart cache keys
- [`dense-flowchart-layout.md`](./dense-flowchart-layout.md) — forward-edge lane clamp
- [`sequence-autonumber-rendering.md`](./sequence-autonumber-rendering.md) — badge placement used in fit diagrams
