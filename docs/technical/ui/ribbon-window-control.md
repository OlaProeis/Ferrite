# Ribbon New Window Control

**Status:** Implemented (Editor UX Polish wave, task 10).

## Summary

**New Window** is exposed from a single ribbon icon in the right-aligned cluster — not from the title bar or a left-group dropdown. Keyboard (**Ctrl+Shift+N**), command palette, and secondary-window flows are unchanged.

## Layout

In `src/ui/ribbon.rs`, the right-to-left block pins controls to the trailing edge:

| Position (left → right) | Control | Notes |
|-------------------------|---------|--------|
| Left of Terminal | Export ComboBox | Markdown tabs only |
| Between Export and Terminal | New Window | `APP_WINDOW` phosphor icon, compact `icon_button` |
| Rightmost | Terminal | `TERMINAL_WINDOW` icon |

Zen mode hides the ribbon entirely (`!zen_mode` in `src/app/mod.rs`); this control is not shown in zen mode.

## Wiring

| Layer | Location |
|-------|----------|
| UI | `icon_button(APP_WINDOW, …)` → `RibbonAction::NewWindow` |
| Handler | `FerriteApp::handle_ribbon_action` → `handle_new_window()` (`src/app/windows.rs`) |
| i18n tooltip | `t!("menu.window.new_window")` + platform modifier via `modifier_symbol()` |
| Shortcut | `ShortcutCommand::NewWindow` — unchanged (`src/app/keyboard.rs`) |

The title bar no longer renders a Window menu (`src/app/title_bar.rs`).

## Related docs

- [Multi-Window Implementation](../platform/multi-window-implementation.md) — viewport lifecycle and entry points
- [Ribbon UI](./ribbon-ui.md) — general ribbon architecture
