# Settings search-first redesign — stable widgets and recents

The Settings tab is search-first: a query bar, category chips, and an overview of recently-changed plus Essentials. Changing a setting used to move its row into Recently Changed on the next frame, which shifted egui auto-ids and dropped slider drags, closed the color picker, and stole text-field focus.

## Stable widget ids

Every registry entry renders inside a salted scope so search results, chips, Essentials, and recents share the same widget ids:

```rust
ui.scope_builder(
    egui::UiBuilder::new().id_salt(("setting", entry.id)),
    |ui| { /* entry.render */ },
);
```

Do not rely on widget tree order for Settings controls.

## Frozen overview order

`SettingsPanel.overview_recent_ids` snapshots `recently_changed_settings` when the overview first becomes visible (no query, no chip). That display list is reused until the Settings tab is left (`end_frame` when `!shown_this_frame`) or a query/chip is applied.

`registry::note_recently_changed` still writes the persisted list immediately. Shortcut rebinds (`KEYBOARD_SHORTCUTS_ID`) are excluded so the shortcut editor never jumps to the top of Recently Changed.

`Settings::sanitize()` calls `sanitize_recently_changed`: drop ids not in the registry, then truncate to `MAX_RECENTLY_CHANGED` (8). One constant for persist, display, and sanitize.

## Ctrl+F

`consume_key(COMMAND, F)` is skipped while `key_capture` is set (so Ctrl+F can be rebound). Otherwise it is consumed only when the search field has focus or no widget has focus — a focused integrated terminal (or any other widget) keeps Ctrl+F.

## Registry notes

- `about.info` is search-only (`!browsable()`); it does not appear on the Appearance chip or in chip match counts.
- `files.show_welcome` (`show_welcome_on_empty_launch`) is featured.
- Clear `key_capture` / `conflict_warning` whenever the keyboard entry is not rendered this frame.
- Locale: `settings.clear_search` for the search × tooltip. Do not reintroduce `settings.keyboard.search_hint`.

## Key files

- `src/ui/settings/mod.rs` — `render_entry`, overview snapshot, Ctrl+F gate, `end_frame`
- `src/ui/settings/registry.rs` — `note_recently_changed`, `sanitize_recently_changed`, `MAX_RECENTLY_CHANGED`, `KEYBOARD_SHORTCUTS_ID`
- `src/config/settings.rs` — `sanitize()` calls `sanitize_recently_changed`
- `src/ui/settings/keyboard.rs`, `src/ui/settings/files.rs`

## Tests

- `test_sanitize_recently_changed_prunes_unknown_and_truncates` in `src/config/settings.rs`
- `test_registry_ids_unique`, `test_files_show_welcome_exists_and_featured`, `test_note_recently_changed_skips_keyboard_shortcuts`
- `test_settings_locale_keys_exist_and_search_hint_removed`

## Related

- Older sidebar-tab description: [`settings-panel.md`](settings-panel.md)
