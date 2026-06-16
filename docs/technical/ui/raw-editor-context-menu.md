# Raw Editor Context Menu

**Status:** Implemented (Editor UX Polish wave, task 11).

## Summary

The raw `FerriteEditor` (main editor and split-view raw pane) exposes a right-click context menu with Copy, Cut, Paste, Select All, and Undo. Rendered-view and CSV/table menus are unchanged.

## Wiring

| Layer | Location |
|-------|----------|
| Menu UI | `show_raw_editor_context_menu()` in `src/editor/widget.rs` — attached via `response.context_menu()` after `editor.ui()` |
| Copy / Cut | `FerriteEditor::copy_selection_to_clipboard`, `cut_selection_to_clipboard` (`src/editor/ferrite/editor.rs`) — `selected_text()` + `ui.copy_text()`; cut also calls `delete_selection()` |
| Paste | `arboard::Clipboard::get_text()` → `FerriteEditor::paste_text()` → `insert_text_at_all_cursors()` |
| Select All | `FerriteEditor::select_all()` (`src/editor/ferrite/selection.rs`) |
| Undo | Sets `EditorOutput.request_undo`; `central_panel.rs` calls `handle_undo()` (app-level tab history, not editor-internal history) |

Both main raw editor and split raw pane use `EditorWidget::show()`, so the menu applies to both without duplicate wiring.

## Behaviour

- **Copy / Cut** — disabled when `!editor.has_any_selection()` (`add_enabled` on menu buttons).
- **Undo** — disabled when `!tab.can_undo()`.
- **Labels** — `t!("shortcuts.edit.copy")`, `cut`, `paste`, `select_all`, `undo`.
- Content edits from paste/cut sync back to `Tab` via the existing `EditorWidget` dirty-path (`record_external_edit_from_snapshot`).

## Related docs

- [Ribbon Window Control](./ribbon-window-control.md) — other UI polish in the same wave
