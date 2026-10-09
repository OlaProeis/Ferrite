# Settings — Recently changed list (cap and exclusions)

The overview **Recently changed** block shows registry setting ids the user edited most recently. Persistence, display, and config sanitize share one cap and one exclusion rule (ISS-16).

## Cap

`registry::MAX_RECENTLY_CHANGED` is **8**. `note_recently_changed` truncates after each insert; `sanitize_recently_changed` (via `Settings::sanitize()`) drops unknown ids then truncates to the same limit. The overview snapshot uses `.take(MAX_RECENTLY_CHANGED)` when freezing `overview_recent_ids`.

## Shortcut rebinds excluded (option B)

Rebinding a keyboard shortcut does **not** add an entry to Recently changed. `note_recently_changed` returns immediately for `KEYBOARD_SHORTCUTS_ID` (`keyboard.shortcuts`) so the large shortcut table does not jump to the top of the overview and steal focus from chord capture.

Individual per-command ids (`shortcut:<Command>`) are **not** stored in recents; only normal registry `SettingEntry` ids appear.

## Manual QA

Checklist row **T2-25** in [`v0.3.1-minimal-release-test.md`](../platform/v0.3.1-minimal-release-test.md): change theme and font size, reopen Settings, confirm both appear (at most eight entries), restart and confirm persistence. Do **not** expect a shortcut rebind in this list.

## Tests

- `test_note_recently_changed_caps_and_dedupes` — length never exceeds `MAX_RECENTLY_CHANGED`
- `test_note_recently_changed_skips_keyboard_shortcuts`
- `test_sanitize_recently_changed_prunes_unknown_and_truncates` in `src/config/settings.rs`

## Related

- [`settings-search-redesign.md`](settings-search-redesign.md) — stable `id_salt`, frozen overview order, Ctrl+F
