# Rendered code block selection copy (CPY-2 / ISS-39)

Mouse-selection copy from a syntax-highlighted fenced block in Rendered/Split must match the fence body byte-for-byte. The header **Copy** button already copied `CodeBlockData::code`; partial selections were wrong when view mode used one `Label` per syntect segment.

## Root cause

egui’s label text selection can span multiple widgets. When each highlight token was its own `Label`, copy joined galley text with spaces between widgets (e.g. `tems. iter(). m` instead of `tems.iter().m`). See egui `LabelSelectionState::copy_text` append path when galleys are side-by-side.

## Fix

**Module:** `src/markdown/widgets.rs` (`EditableCodeBlock` view branch)

1. **`build_code_block_view_layout_job`** — One `LayoutJob` for the whole block: append every `highlight_code` segment with token colors. `job.text` must equal the raw fence body (debug-assert + unit test).
2. **`show_code_block_view`** — Single selectable `Label` over that galley so selection copy uses one continuous char index space.
3. **`selected_code_text(raw, char_range)`** — Maps UTF-8 **character** indices to a byte slice of `raw` (clamp end); used in tests and as the canonical mapping when char indices align with the layout job text.

The toolbar **Copy** control is unchanged (`ui.ctx().copy_text(self.data.code.clone())`).

## Tests

```bash
cargo test --bin ferrite selected_code_text
cargo test --bin ferrite code_block_view_layout_job_text_matches_source
```

Manual (minimal release checklist): select part of a rust line in `test_md/test_copy_and_split_undo.md` Rendered; Ctrl+C paste must match the source substring with no extra spaces at token boundaries.

## Related

- `docs/technical/markdown/editable-code-blocks.md` — widget overview
- `docs/technical/markdown/preview-code-block-undo.md` — edit session / undo (separate from copy)
