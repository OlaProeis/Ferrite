# Rendered Edit Session — Headings

Headings in rendered (WYSIWYG) mode use [`RenderedEditSession`](../../../src/markdown/rendered_session.rs) for buffer ownership, one-click block switching, and commit-on-switch — replacing legacy `rendered_focus` defer/commit hacks for this block type.

**PRD:** [Rendered Edit Session](../../../ai-workflow/prds/prd-rendered-edit-session.md)  
**Foundation:** [Core types](./rendered-edit-session-core.md), [Widget identity](./rendered-widget-identity.md)

## Behaviour

| Action | Result |
|--------|--------|
| Edit heading A, click heading B | A commits to source, B activates with one click |
| Click mid-word in heading | Cursor placed via galley mapping; stable while typing |
| Rendered heading edit commits | Source updated via `update_source_line`; **`source_epoch` unchanged** |
| Focus leaves heading (no other session block) | `close_active_ui(SaveIfDirty)` commits buffer |

## Implementation map

| Concern | Location |
|---------|----------|
| Session load/save per frame | `show_rendered_editor` — `rendered_session::load` / `save(ui, editor_id, …)` |
| Heading render | `render_heading` in `src/markdown/editor.rs` |
| Block identity | `BlockRef::Heading { line, structural }` — `structural` selects `heading_text` vs `heading_text_sk` widget id |
| TextEdit id | `block_ref.widget_id(ui)` under `push_id(editor_id)` + `push_id(source_epoch)` |
| Buffer | `BlockEditState.text` — cold init from `node.text_content()` |
| Activation | `session.switch_to_ui` + `PendingActivation { cursor_char_index, request_focus }` |
| Text edits | `session.on_text_changed` (marks dirty; no source write) |
| Commit | Callback: `format!("{prefix}{}", format_heading(…))` + `update_source_line`; level from Heading AST via `heading_level_for_commit` (fallback: `#` after container prefix). See [container prefix](./rendered-edit-container-prefix.md) |

## Activation flow

```
Click/focus on heading B while A is active
  → switch_to_ui(B, PendingActivation { … })
  → commit callback writes A buffer if dirty
  → surrender A egui focus
  → B buffer active; pending_activation applied after TextEdit show
```

Cross-widget click uses `heading_click_cursor` (wraps `compute_displayed_cursor_index`) so the caret lands at the click position.

## Removed (headings only)

- Temp egui buffers: `heading_edit_buffer`, `heading_sk_edit_buffer`, tracking ids
- `rendered_focus::after_text_edit` for headings
- `rendered_focus::focus_loss_should_commit` for headings
- Separate `render_heading_with_structural_keys` — unified `render_heading(..., structural: bool)`

Paragraphs, formatted blocks, list items, and tables all use the session — see linked docs below.

## Session threading

`RenderedEditSession` is loaded once in `show_rendered_editor` and passed as `&mut` through `render_node` and container helpers (blockquote, callout) so nested headings share the same session.

## Tests

- `rendered_session::` — state machine, widget id suffix stability
- `test_heading_level_from_source`, `test_rendered_heading_widget_id_*` in `editor.rs` tests module

## Manual verification

1. Document with `# Alpha` and `# Beta`
2. Edit Alpha, single-click Beta → Alpha saved, Beta focused
3. Click mid-word in a heading, type several seconds → cursor stays put
