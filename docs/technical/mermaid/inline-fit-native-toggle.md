# Inline Mermaid Fit/Native Toggle

Each inline Mermaid block can switch between **Fit width** (default) and **Native size** (horizontal scroll). The choice is session-only and stored per block in egui temp memory.

## State

| Key | Type | Default | Location |
|-----|------|---------|----------|
| `block_id.with("fit_mode")` | `bool` | `true` (fit) | egui temp memory |

- `true` — fit-to-pane when measurable and above `MIN_FIT_SCALE` (see `inline-fit-to-pane.md`).
- `false` — native size inside `ScrollArea::horizontal()`.

No persistence across app restarts.

## UI

In `MermaidBlock::show` (`src/markdown/widgets.rs`):

- Hover-revealed icon button in the header bar (right side, before the Source toggle).
- Visible when the header or toggle itself was hovered on the previous frame (`header_hover_key`, `fit_toggle_hover_key`).
- Hidden while source view is open.
- Icons: `ARROWS_LEFT_RIGHT` (fit active) / `ARROWS_OUT` (native active).
- Styling: `ui.style().visuals.widgets.inactive`.
- Tooltips (i18n): `mermaid.native_size` when fit is on; `mermaid.fit_width` when native is on.

## Rendering integration

`render_validated_mermaid` reads `fit_mode` and sets:

```text
scroll_fallback = !fit_mode || natural_size.is_none() || fit_scale < MIN_FIT_SCALE
```

- **Fit (`fit_mode = true`, measurable, above threshold):** no horizontal scroll; `show_inline_mermaid_diagram(..., scroll_fallback: false)` may use locked-zoom `Scene`.
- **Native (`fit_mode = false`):** always wraps in `ScrollArea::horizontal()` at native size.

Popup open (diagram click / expand button) and source toggle are unchanged.

## i18n

`locales/en.yaml`:

- `mermaid.fit_width` — `"Fit width"`
- `mermaid.native_size` — `"Native size"`

## Related

- `docs/technical/mermaid/inline-fit-to-pane.md` — scale model and Scene fit path
