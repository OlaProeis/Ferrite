# Settings tab — Esc close and Ctrl+, open (ISS-12)

Settings is a **special tab** (`SpecialTabKind::Settings`), not the legacy modal controlled by `UiState.show_settings`. GrokBot reported Esc and Ctrl+, as no-ops; code review showed real wiring gaps (not only synthetic keyboard input).

## Ctrl+, (Open Settings)

Global shortcut dispatch (`KeyboardAction::OpenSettings`, ribbon `RibbonAction::OpenSettings`, command palette) must use the same entry point as the title-bar gear:

- `AppState::toggle_settings()` delegates to `open_settings_tab()` — open/focus the Settings tab, or close it if already active.
- Do **not** flip `UiState.show_settings`; that flag is unused after the v0.2.7 special-tab migration.

Default binding remains `ShortcutCommand::OpenSettings` → `KeyBinding::new(M::ctrl(), Comma)` in `KeyboardShortcuts::default_for`.

## Esc

| Context | Behavior |
|--------|----------|
| Settings search focused, query non-empty | Clear query only |
| Settings search focused, query empty | TextEdit blurs; further Esc when nothing else has focus closes the tab |
| No widget focus (overview, chips, etc.) | Close Settings tab |
| Another Settings text field focused | Esc handled by that widget; tab stays open |
| Shortcut rebind capture (`key_capture`) | Esc cancels capture (keyboard section) |
| Reset-all or About overlay open | Esc handled by those dialogs |

Implementation:

1. `SettingsPanel::show_inline` sets `SettingsPanelOutput.close_requested` when Esc is pressed and the focus gates above allow it; consumes the key so the global Escape handler does not run.
2. `render_special_tab_content` calls `force_close_tab` on `close_requested`.

Gates mirror Ctrl+F: skip while `key_capture`, `reset_confirm_open`, or `about_open` is active; treat only the search field id as the Settings search text focus target.

## Key files

- `src/state.rs` — `toggle_settings`, `open_settings_tab`
- `src/app/keyboard.rs` — `OpenSettings` → `toggle_settings`
- `src/ui/settings/mod.rs` — `close_requested`, Esc + search clear
- `src/app/central_panel.rs` — close tab on output

## Tests

- `state::tests::test_appstate_toggle_settings` — open/close via special tab kind
- `ui::settings::tests::test_settings_panel_output_default` — `close_requested` default false

## Related

- Search-first Settings UX: [`settings-search-redesign.md`](settings-search-redesign.md)
- Special tabs overview: [`special-tabs.md`](special-tabs.md) (modal `show_settings` is historical)
