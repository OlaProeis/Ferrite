# Settings Keyboard chip live search count (ISS-15)

During live search, category chips show how many settings match the query. The Keyboard section is one registry entry (`keyboard.shortcuts`) but many shortcut **rows**, so its chip must count rows—not registry entries.

## Behaviour

- **Keyboard chip:** `keyboard::matching_shortcut_row_count(query, &settings.keyboard_shortcuts)`.
- **Other chips:** unchanged—`registry::all_entries()` filtered by `section`, `browsable()`, and `matches(query)`.

Row counting uses the same filter gate as the shortcut list in `render_keyboard_shortcuts`:

1. Trim and lowercase the query.
2. Apply row filtering only when `keyboard::any_command_matches` is true (otherwise the keyboard entry may match via label/keywords and the list shows all rows; the chip count is **0**).
3. Each row matches if the filter substring appears in the localized command name (`shortcut_command_name`), the category label, or the current binding’s `display_string()`.

Shared helpers live in `src/ui/settings/keyboard.rs`: `shortcut_row_filter_query`, `shortcut_row_matches_filter`, `matching_shortcut_row_count`.

## Key files

- `src/ui/settings/mod.rs` — chip label when `searching`
- `src/ui/settings/keyboard.rs` — row filter, count, unit tests

## Tests

- `matching_shortcut_row_count_zoom_matches_three_rows`
- `matching_shortcut_row_count_unrelated_query_is_zero`
- `matching_shortcut_row_count_tab_matches_multiple_rows`

## Related

- [`settings-search-redesign.md`](settings-search-redesign.md) — search-first shell and chip overview
