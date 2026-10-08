# Linux primary-selection paste (#183)

X11/Wayland primary-selection semantics in the **raw** Ferrite editor: selecting text publishes it, and middle-click inserts it at the click position. egui has no primary-selection API; Ferrite talks to the OS through arboard’s Linux extension traits.

## Behaviour

| Action | Result |
|--------|--------|
| Range selection in raw editor | After 150 ms with no pointer button down, the selected text is published to the primary selection |
| Middle-click in raw editor (not a drag) | Inserts `get_primary()` at the click caret as one undo step |
| `Settings.middle_click_paste` off | Disables both publish and insertion |
| Windows / macOS | `set_primary` / `get_primary` are no-ops; the Settings row is not shown |

Middle-click on the **tab strip** still closes the tab. Terminal and rendered-view middle-click are unchanged (rendered paste is out of scope).

Vim: insertion runs only when `vim_state.should_insert_text()` is true (Insert mode). Clicks in the fold-indicator gutter do not paste.

## Platform API — `src/platform/primary_selection.rs`

| Function | Role |
|----------|------|
| `set_primary(text)` | Publish to `LinuxClipboardKind::Primary`. Empty text is ignored. |
| `get_primary()` | Read primary; `None` when empty or unavailable. |
| `primary_publish_decision(...)` | Pure debounce: `(selection, now, pointer_down, last_published, armed_at)` → `(publish, next_deadline)` |

Linux uses `SetExtLinux::clipboard` / `GetExtLinux::clipboard`. One `arboard::Clipboard` lives in a `thread_local!` so the object is not dropped after set (arboard serves the selection from that handle). Do **not** call `.wait()` — that blocks until another app takes the selection and would freeze the UI.

`Cargo.toml`: `arboard = { version = "3", features = ["wayland-data-control"] }` for native Wayland compositors that support `wlr-data-control`. Compositors without that protocol fall back to **XWayland**; publish/paste then work only in XWayland clients.

## Debounce

`PRIMARY_PUBLISH_DEBOUNCE` is 150 ms.

- A new char-offset range arms `now + 150ms` and does not publish yet.
- Elapsed deadline with a pointer button still down waits (same deadline).
- Elapsed deadline with the pointer up publishes once, then remembers the range.
- An identical already-published range does not re-publish.
- A collapsed selection (`None`) clears the deadline.

While armed, the editor calls `ctx.request_repaint_after` for the remaining time.

## Editor wiring — `src/editor/ferrite/editor.rs`

- `last_primary_selection` / `primary_publish_at` track the last published char range and the debounce deadline.
- Drag uses `drag_started_by(PointerButton::Primary)` / `dragged_by(PointerButton::Primary)` so middle-click does not start a selection drag.
- After the primary-click handler: `middle_clicked() && !dragged()` → `interact_pointer_pos` → skip fold gutter → `pos_to_cursor` → `get_primary` → `set_cursor` + `insert_text_at_all_cursors` + `request_focus`.
- `maybe_publish_primary_selection` runs only when `middle_click_paste` is on (Linux body; no-op stubs elsewhere).

`EditorWidget::middle_click_paste` is set from `Settings.middle_click_paste` in both Raw and Split raw panes (`central_panel.rs`).

## Settings

- `Settings.middle_click_paste: bool` — default `true` (`serde` `default_true`).
- Registry id `editor.middle_click_paste`; render + registry entry are `#[cfg(target_os = "linux")]`.

## Tests

- `primary_publish_decision` state machine in `src/platform/primary_selection.rs`
- `test_middle_click_insert_is_one_undo_step` — `set_cursor` + `paste_text` records one `EditHistory` group
- `test_middle_click_paste_defaults_true_when_absent`

## Manual QA (Linux X11 and/or Wayland)

1. Select text in Ferrite raw → middle-click in a terminal pastes it.
2. Select in a terminal → middle-click in Ferrite raw inserts at the click.
3. Middle-click a tab still closes it.
4. Setting off disables insertion (and publishing).
