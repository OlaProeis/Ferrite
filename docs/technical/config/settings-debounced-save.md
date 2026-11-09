# Debounced settings persistence (SI-4)

## Problem

Settings toggles (for example **Allow multiple instances**) called `AppState::mark_settings_dirty` but `config.json` was only written on application exit, eframe’s 30s autosave, or other code paths that called `save_settings_if_dirty` immediately. A toggle alone did not reach disk until a later unrelated save.

## Behavior

1. **`mark_settings_dirty`** sets the dirty flag, calls `note_user_settings_edit`, and records a timestamp on `SettingsSaveDebounce` (500 ms window, `SETTINGS_SAVE_DEBOUNCE_MS`).
2. Each frame, **`FerriteApp`** calls **`AppState::poll_settings_save_debounce`** with elapsed ms from app start; when the debounce elapsed, **`save_settings_if_dirty`** runs (same merge of open tabs into `last_open_tabs` as before).
3. Closing the Settings tab (Esc / close request) calls **`flush_settings_on_panel_close`** so pending toggles persist immediately, without waiting for debounce.
4. After a successful save, the debounce pending flag is cleared (also cleared inside **`save_settings_if_dirty`** on success).

Recent-file and save-as paths that set `settings_dirty` and call **`save_settings_if_dirty`** immediately are unchanged.

## Key symbols

| Location | Role |
|----------|------|
| `src/config/settings.rs` | `SettingsSaveDebounce`, `try_debounced_settings_flush`, `flush_settings_on_panel_close`, `settings_mono_ms`, `Settings::set_allow_multiple_instances` |
| `src/state.rs` | `settings_save_debounce` field, `poll_settings_save_debounce`, `flush_settings_on_panel_close` |
| `src/app/mod.rs` | Per-frame `poll_settings_save_debounce` |
| `src/app/central_panel.rs` | `request_repaint_after(SETTINGS_SAVE_DEBOUNCE_MS)` on settings change; flush on `close_requested` |
| `src/ui/settings/files.rs` | Allow-multiple-instances checkbox uses `set_allow_multiple_instances` |

## Tests

Unit tests in `src/config/settings.rs` (`allow_multiple_instances_debounce_flush_persists_to_config`, `settings_panel_close_flushes_dirty_config_immediately`) use `with_temp_data_dir` and the debounce helpers with an advanced millisecond clock.
