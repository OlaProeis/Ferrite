# Rendered Edit Session — Formatted Block Shared Layout

Formatted paragraphs and list items in **display mode** paint and hit-test from one shared egui galley per block, keyed by [`BlockRef`](../../../src/markdown/rendered_session.rs). This fixes imprecise click-to-edit caret placement when inline formatting (bold, italic, links) or word wrap caused paint and pointer mapping to diverge.

**Related:** [Formatted blocks overview](./rendered-edit-session-formatted.md), [Galley cursor positioning](../editor/galley-cursor-positioning.md)

---

## Problem

Before this refactor, display mode painted formatted blocks with a galley built from raw markdown (`build_inline_markdown_layout_job`), but click-to-edit mapped pointer position using `node.text_content()` and a separate single-font galley (`compute_displayed_cursor_index`). Those sources disagree on:

- **Inline spans** — styled widths differ from plain-text approximation.
- **Links** — painted galley shows link label text only; `text_content()` flattens AST children differently.
- **Word wrap** — re-layout at a different width than paint time maps clicks on wrapped lines to the wrong visual row.

Result: clicking mid-word in bold/italic/link text, or on the second line of a wrapped paragraph, often placed the caret at the wrong raw offset.

---

## Solution: `FormattedBlockLayout`

Defined in `src/markdown/rendered_session.rs`:

| Piece | Role |
|-------|------|
| `FormattedBlockLayout` | Builds one `Arc<egui::Galley>` from raw session buffer via `build_inline_markdown_layout_job` |
| `paint()` | Allocates exact galley size and paints it |
| `displayed_cursor_at()` | `galley.cursor_from_pos(local_pos)` on the painted galley |
| `raw_cursor_at()` | displayed index → raw markdown via `map_displayed_to_raw` (`widgets.rs`) |
| `paint_formatted_block_display()` | Persist wrap width, build layout, store in egui temp memory, paint |
| `formatted_block_layout()` | Retrieve same-frame stored layout |
| `layout_for_formatted_click()` | Resolve galley for clicks — reuse stored layout or rebuild at persisted paint width |

Storage id: `block.widget_id(ui).with("formatted_display_layout")`.

---

## Persisted wrap width (`layout_wrap_width`)

`BlockEditState::layout_wrap_width` records the wrap width used at the last paint for each block.

| Function | When |
|----------|------|
| `persist_block_layout_wrap_width()` | Called from `paint_formatted_block_display` every paint (clamps to ≥ 1px) |
| `layout_for_formatted_click()` | Click handler: prefer same-frame stored layout when text and width match; otherwise rebuild at **persisted paint-time width** |

This keeps hit-testing aligned with paint after window resize: the current viewport width may differ, but clicks re-layout at the width from the most recent paint, not an ad-hoc estimate.

`invalidate_buffers()` clears all block state (including `layout_wrap_width`) on `source_epoch` bump — blocks must be repainted before click mapping works.

---

## Render flow (display mode)

```
paint_formatted_block_display(session, block, raw_text, wrap_width, …)
  → persist_block_layout_wrap_width(session, block, wrap_width)
  → FormattedBlockLayout::build → store_formatted_block_layout
  → layout.paint → Response (galley rect)

ui.interact(display_response.rect, …, Sense::click())
  → enter_formatted_edit_on_display_click
      → layout_for_formatted_click(session, block, params)
          → reuse stored layout OR rebuild at session.blocks[block].layout_wrap_width
      → layout.raw_cursor_at(click_pos, rect, leading_indent)
      → session.switch_to_ui + formatted_editing = true
```

All four formatted block render sites use this path:

- `render_paragraph` / `render_paragraph_with_structural_keys`
- `render_list_item` / `render_list_item_with_structural_keys`

Headings and plain text blocks still use `compute_displayed_cursor_index` (single-font galley).

---

## Link click handling

`enter_formatted_edit_on_display_click` still bails when `link_click_consumed_this_frame` is set. In galley display mode, links are styled text (not separate link widgets); the flag is only relevant if a nested interactive widget consumed the click first.

---

## Tests

```bash
cargo test rs2_
cargo test layout_for_formatted_click
cargo test persist_block_layout
cargo test formatted_block_layout
```

| Test | Coverage |
|------|----------|
| `formatted_block_layout_store_roundtrip` | egui temp storage roundtrip |
| `formatted_block_layout_raw_cursor_skips_markers` | `map_displayed_to_raw` through `**` markers |
| `rs2_wrapped_paragraph_click_second_visual_line` | Second visual line of wrapped paragraph |
| `rs2_wrapped_paragraph_two_wrap_widths_map_correctly` | Narrow (72px) vs wide (480px) wrap widths |
| `rs2_link_paragraph_cursor_maps_inside_link` | Click on link label maps inside `[...]` |
| `persist_block_layout_wrap_width_roundtrip` | Persistence + 1px minimum clamp |
| `layout_for_formatted_click_rebuilds_at_persisted_paint_width` | Stale frame layout after resize → rebuild at paint width |

Manual RS-2 cases: [`v0.3.0-regression-matrix.md`](../platform/v0.3.0-regression-matrix.md) §3.12 (RS-2a, RS-2b).

## Manual verification

1. Rendered view, formatted paragraph: `A **bold** word with a [link](url).`
2. Click mid-word in bold, italic, and link label text; type — caret should match click.
3. Long paragraph that wraps: click the **second visual line** — caret lands on that line, not the first.
4. Resize window, repeat step 3 — behavior stays correct after re-layout.
5. Compare several positions against raw-mode offsets after entering edit.
