# Settings search — initial focus and first keystrokes (ISS-18)

GrokBot QA reported the first character(s) missing when typing immediately into Settings search or pressing the first chord in shortcut rebinding. Root causes in code (not only synthetic input):

1. **Search field** — `request_focus()` runs after the `TextEdit` is built, so same-frame `Event::Text` can arrive before focus lands.
2. **Shortcut capture** — Global `handle_keyboard_shortcuts` runs **after** UI render and could consume the first chord key; the search field could keep focus after clicking **Change binding**.

## Search focus on tab open

- `SettingsPanel.end_frame`: when Settings was not shown that frame, set `defer_search_focus`.
- First `show_inline` after opening the Settings tab sets `focus_search` (search-first UX).
- `open_keyboard_section_for` also sets `focus_search` after prefilling the query (cheat-sheet deep link).

## Same-frame text merge

When `focus_search` requests focus and the field did not already have focus, pending `Event::Text` in the current input is appended to `search_query` and removed from the event list (`merge_pending_text_events`). Ctrl+F uses the same path.

## Shortcut rebinding

- `SettingsPanel::is_capturing_keyboard_shortcut` — `FerriteApp::handle_keyboard_shortcuts` returns early while `key_capture` is set.
- Starting capture surrenders the current egui focus id so the search `TextEdit` does not keep keyboard focus.

## Key files

- `src/ui/settings/mod.rs` — `defer_search_focus`, `focus_search`, `merge_pending_text_events`
- `src/ui/settings/keyboard.rs` — capture start + event loop
- `src/app/keyboard.rs` — capture guard

## Tests

- `ui::settings::tests::test_end_frame_defers_search_focus_when_settings_not_shown`
- `ui::settings::tests::test_merge_pending_text_events_appends_and_removes_text_events`
- `ui::settings::tests::test_open_keyboard_section_for_prefills_search` — asserts `focus_search`

## Related

- Search-first shell: [`settings-search-redesign.md`](settings-search-redesign.md)
- Esc / Ctrl+,: [`settings-esc-open-shortcut.md`](settings-esc-open-shortcut.md)
