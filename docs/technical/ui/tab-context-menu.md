# Tab Context Menu

**Status:** Implemented (Editor UX Polish wave, task 12).

## Summary

Right-click on a tab in the tab strip opens a context menu with New Tab, Close Tab, and (when the tab has a file path) Copy Path and Reveal in Explorer. Labels are localized, rows highlight on hover, and the popup sizes to content.

## Wiring

| Layer | Location |
|-------|----------|
| Menu trigger | `central_panel.rs` — sets `state.ui.tab_context_menu` on tab right-click; renders `egui::Area` popup at cursor position |
| Action metadata | `ActionRegistry::actions_for(ActionContext::Tab { has_file_path })` in `src/ui/action_registry.rs` |
| Row rendering | `render_action_menu_with_shortcuts()` — label + optional right-aligned shortcut hint; hover fill on background layer |
| Side effects | `central_panel.rs` — dispatches `ContextActionId` to `new_tab()`, close tab, copy path, reveal in explorer |

## i18n

Locale keys under top-level `tab:`:

- `tab.new_tab` — "New Tab"
- `tab.close` — "Close Tab"
- `tab.reveal_in_explorer` — existing key

Copy Path reuses `tree_viewer.copy_path`. Empty locale strings fall back to English via `localized_label()`.

## Hover and sizing

Menu rows use the same pattern as the command palette: `ui.interact` on the row rect, then paint `widgets.hovered.weak_bg_fill` to an `Order::Background` layer so text stays readable. Fixed `set_min_width` on the popup frame and per-row layout was removed so width follows the widest label + shortcut pair.

## Related docs

- [Raw Editor Context Menu](./raw-editor-context-menu.md) — other context menus in the same wave
