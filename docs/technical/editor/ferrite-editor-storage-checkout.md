# FerriteEditor egui storage checkout (ISS-36)

## Problem

`EditorWidget::show` previously checked out the per-tab `FerriteEditor` with `std::mem::replace(editor, FerriteEditor::new())`. A re-entrant `get_ferrite_editor_mut` during the same frame could observe that **empty placeholder** instead of the real buffer — a named ISS-36 candidate.

## Model

Editors live in egui temp data as `FerriteEditorStorage`:

| Map value | Meaning |
|-----------|---------|
| `Some(FerriteEditor)` | Editor in storage; `get_ferrite_editor_mut` may borrow it |
| `None` (key present) | Checked out for the current `EditorWidget::show` frame |

Sync metadata (`content_hashes`, `content_lengths`, `content_versions`) stays keyed by tab id and is updated when the editor is put back.

## Checkout / put-back

- **Take:** `take_ferrite_editor_from_storage` — `Option::take()` from the slot; `debug_assert!` the slot was `Some` before take.
- **Return:** `put_ferrite_editor_in_storage` — `debug_assert!` slot is `None`, then `Some(editor)`.

`EditorWidget::show` takes at frame start and puts back after painting/input.

## External access

`get_ferrite_editor_mut(ctx, tab_id, f)`:

- Missing key → `None` (no editor yet).
- Slot `None` (checked out) → `None`; **`log::warn!` once per tab per frame** (`cumulative_frame_nr()` + `checked_out_warned_tabs`).
- Slot `Some` → runs `f` on the editor.

Callers that already handle `Option` (find/replace, formatting, central panel) behave as before when access is denied during show.

## Tab close

`cleanup_ferrite_editor` removes sync caches and the map entry. If the tab is closed while checked out (`Some(None)`), only the slot is dropped (the live editor is owned by the in-flight show frame).

## Code reference

- `FerriteEditorStorage`, `take_ferrite_editor_from_storage`, `put_ferrite_editor_in_storage` — `src/editor/widget.rs`
- `get_ferrite_editor_mut`, `cleanup_ferrite_editor` — `src/editor/widget.rs` (re-exported from `src/editor/mod.rs`)
- Unit test: `editor_slot_checkout_blocks_get_mut_until_put_back`
