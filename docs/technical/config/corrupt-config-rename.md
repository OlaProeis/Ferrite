# Syntax-invalid config rename

## Problem

When `config.json` is not JSON at all, overwriting it with defaults would destroy the user's file. The loader preserves the bytes, loads defaults, and leaves a path for a later toast.

## API

| Symbol | Location | Role |
|--------|----------|------|
| `ConfigLoadOutcome` | `src/config/persistence.rs` | `Loaded { settings, dropped_field_count }` or `CorruptFileRenamed { settings, path }` |
| `load_config_outcome` | `src/config/persistence.rs` | Load plus the rename signal |
| `load_config` | `src/config/persistence.rs` | Settings only (`into_settings`) |
| `take_pending_corrupt_config_rename` | `src/config/persistence.rs` | One-shot process slot for the renamed path |
| `AppState.corrupt_config_renamed_to` | `src/state.rs` | Set in `AppState::new`; drained by `show_pending_config_load_toasts` (see `config-load-toasts.md`) |

## Behavior

1. Missing file → `Settings::default_with_system_locale()`. Empty file → `Settings::default()`. Neither is renamed.
2. If `serde_json::from_str::<Value>` fails, rename `config.json` to `config.json.corrupt-YYYYMMDD-HHMMSS` (local time). A name collision advances the stamp by one second.
3. Keep the three newest `config.json.corrupt-*` names (lexicographic order matches the timestamp). Delete older copies.
4. Return `CorruptFileRenamed` with `Settings::default()`. Do not write a replacement `config.json` on this path.
5. Valid JSON still goes through `Settings::from_json_tolerant` (see `tolerant-settings-load.md`).

`main` calls `load_config()` before `AppState::new` calls `load_config_outcome()`. The first load records the renamed path in a process mutex so the second load can still fill `corrupt_config_renamed_to` after the original file is gone. `take_pending_corrupt_config_rename` clears that slot.

## Out of scope

- Save-time last-good backup (`atomic-config-save.md`)
- Per-field tolerant recovery (separate doc)
