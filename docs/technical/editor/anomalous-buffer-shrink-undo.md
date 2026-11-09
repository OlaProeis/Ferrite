# Anomalous buffer shrink undo protection (ISS-36)

## Problem

A single raw-editor frame can discard most of a non-trivial buffer (often whole-document selection replaced by one typed character). The ISS-36 diagnostic (`EditorWidget: buffer for tab N shrank …`) catches large one-frame shrinks; some cases look like accidental data loss rather than intentional select-all + type.

## Detection

After `FerriteEditor::ui`, when content is dirty and length dropped by more than half from a pre-frame buffer longer than 64 characters, `classify_buffer_shrink` in `src/editor/widget.rs` returns **Anomalous** when all of the following hold:

| Condition | Meaning |
|-----------|---------|
| Pre-frame selection collapsed | No range before input |
| Exactly one `Text` or `Key` event in the frame | Not multi-step edit |
| No Cmd/Ctrl+A in the frame | Deliberate select-all + type is **Normal** |
| Partial shrink (≤ half) | **Normal** |

Non-anomalous large shrinks still log at **warn** with the same context (selection, `click_count`, event summary). Anomalous shrinks log at **error**. The edit is never blocked or reverted automatically.

## Undo hardening

When classified as anomalous, tab sync skips `record_external_edit_from_snapshot_coalesced` and calls `Tab::record_anomalous_buffer_shrink_from_snapshot` instead. That uses `EditHistory::record_anomalous_buffer_shrink_groups`:

1. `break_group` — close any open typing run
2. `record_pre_frame_buffer_snapshot` — pre-edit buffer as its own undo group (isolates from coalescing)
3. `break_group`
4. `record_operations(compute_edit_ops(pre, post))` — shrink diff as a separate atomic group

One **Ctrl+Z** undoes the shrink group and restores the pre-frame text. `pending_undo_snapshot`, `content_version`, and `source_epoch` follow the usual external-edit path.

## API reference

- `classify_buffer_shrink`, `ShrinkVerdict` — `src/editor/widget.rs`
- `Tab::record_anomalous_buffer_shrink_from_snapshot` — `src/state.rs`
- `EditHistory::record_pre_frame_buffer_snapshot`, `record_anomalous_buffer_shrink_groups` — `src/editor/ferrite/history.rs`

## Related

- Caret-only session restore and first-frame input reset: `technical/files/session-restore-caret-only.md`
- Editor checkout slot (same ISS-36 workstream): `technical/editor/ferrite-editor-storage-checkout.md`

## Tests

- `editor::widget::tests::classify_buffer_shrink_*` (four cases)
- `editor::ferrite::history::tests::anomalous_shrink_path_single_undo_restores_pre_frame`
