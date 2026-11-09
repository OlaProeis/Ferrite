# Forwarded Open — Owning Viewport Raise (ISS-34)

When single-instance IPC opens a path that is **already open in another document window**, Ferrite must raise the **window that owns that tab**, not merely the last-focused window.

## Symptom (QA)

Linux SI-1 / ISS-34: with two Ferrite OS windows, forwarding a path to an existing tab in window A while window B is on top switched the tab in A but left B active. Logs showed `request_user_attention` on the front window and no effective raise of A. Single-window raises still worked (T2-38).

Part of the failure was Ferrite routing `ViewportCommand::Focus` to the last-focused viewport; xfwm4 focus-stealing prevention may still downgrade raises on X11 (manual re-check after the fix).

## Behaviour

After `open_file_with_focus` switches to an existing tab (`focus_tab_in_window`), `handle_instance_paths` / `open_os_paths_in_focused_window` call `focus_owning_window_for_open_path`:

1. Resolve owner via `find_tab_by_path`.
2. Set `working_window_id` and `focused_window_id`.
3. Set `tab.needs_focus`.
4. Send `ViewportCommand::Focus` and `RequestUserAttention(Informational)` to **`viewport_for_tab(tab_id)`** (primary → `ViewportId::ROOT`; secondary → `document_viewport_id(window_id)`).

For **secondary-instance IPC only**, on Unix (not macOS), schedule a **next-frame** `ViewportCommand::Focus` on that same viewport (`pending_forwarded_open_viewport` in `FerriteApp::update`), so winit’s `focus_window()` runs again after tab/window state settles.

New files still open in the last-focused window; only the post-open raise uses the owner when the path was already open anywhere.

## Key files

| File | Role |
|------|------|
| `src/app/mod.rs` | `viewport_for_tab`, `window_id_for_tab`; drain deferred focus after `handle_instance_paths` |
| `src/app/file_ops.rs` | `focus_owning_window_for_open_path`, `handle_instance_paths`, `open_os_paths_in_focused_window` |
| `src/app/windows.rs` | `focus_document_viewport` |
| `src/single_instance.rs` | IPC delivery (UI focus documented in module comment) |

## Tests

- `app::viewport_for_tab_tests::viewport_for_tab_resolves_primary_and_secondary_windows` (`src/app/mod.rs`)
- Existing `single_instance::tests::*`

## Related

- [single-instance.md](./single-instance.md)
- [multi-window-file-routing.md](./multi-window-file-routing.md)
