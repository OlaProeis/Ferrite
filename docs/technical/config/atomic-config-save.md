# Atomic config save and last-good backup

## Problem

`save_config` used to write the new JSON to `config.json.bak` and rename that file onto `config.json`. The `.bak` was the new payload, not the previous file. A session that started from defaults after a failed full load could then replace a good backup with those defaults.

## API

| Symbol | Location | Role |
|--------|----------|------|
| `save_config` | `src/config/persistence.rs` | Write `config.json.tmp`, copy the readable current file to `config.json.bak`, rename tmp onto `config.json` |
| `note_user_settings_edit` | `src/config/persistence.rs` | Sticky process flag: the user changed settings |
| `AppState::mark_settings_dirty` / `update_settings` | `src/state.rs` | Call `note_user_settings_edit` |
| Failed-full-load flag | `src/config/persistence.rs` | Set on syntax-invalid rename and on load errors that return all-default settings |

## Behavior

1. Serialize to `config.json.tmp`.
2. If `config.json` exists and is readable, copy those bytes to `config.json.bak` (previous contents). A missing file skips the copy. An unreadable file skips the copy and still continues the save.
3. Rename `config.json.tmp` onto `config.json`. If the destination already exists (Windows), remove it and rename again.
4. If the copy to `.bak` or the final rename fails, delete the temp file and return `Error::ConfigSave`.

**Failed-load guard.** Syntax-invalid JSON (`ConfigLoadOutcome::CorruptFileRenamed`) and any `load_config_outcome` error that falls back to `Settings::default()` set a process-wide failed-full-load flag. Partial field recovery (`from_json_tolerant`) does not. The flag stays set when a later load sees a missing file (the corrupt file was already renamed). While it is set and `note_user_settings_edit` has not run, step 2 is skipped so defaults are not copied over `config.json.bak`.

`mark_settings_dirty` and `update_settings` record a user edit. Recent-file updates, tab-info upserts, and `save_settings` (which forces dirty) do not, so an exit save after a failed load still preserves `.bak`.

## Out of scope

- Toast when a corrupt file was renamed
- Rewriting `config.json` during the corrupt-load path itself
