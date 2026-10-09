# Tolerant settings load (ISS-9)

## Problem

A single mistyped field in `config.json` (e.g. `"font_size": "abc"`) used to fail the entire `Settings` deserialize and could discard valid settings on the error path. Type errors should drop only the offending field(s) and keep the rest.

## API

| Symbol | Location | Role |
|--------|----------|------|
| `TolerantSettingsLoad` | `src/config/settings.rs` | `settings` + `dropped_field_count` (surfaced on `ConfigLoadOutcome::Loaded` and startup toast) |
| `Settings::from_json_tolerant` | `src/config/settings.rs` | Parse JSON text, tolerant field recovery, then `sanitize()` |
| `Settings::from_json_value_tolerant` | `src/config/settings.rs` | Same from an existing `serde_json::Value` |
| `Settings::from_json_sanitized` | `src/config/settings.rs` | Strict deserialize + sanitize (unchanged) |
| `load_config_internal` | `src/config/persistence.rs` | Valid JSON → `from_json_tolerant`. Syntax-invalid files are renamed aside (`corrupt-config-rename.md`), not parsed here. |

## Algorithm

1. Require a JSON **object** at the root; otherwise log WARN and use defaults.
2. Loop (bounded, max 512 drops): try `serde_json::from_value::<Settings>`.
3. On success: `sanitize()` and return.
4. On `serde_json::Error`: determine field path via `serde_error_path` (message prefix) or `find_config_field_to_drop` (trial removal that changes or clears the error).
5. Remove that path with `remove_config_value_at_path` (object keys and array indices), log WARN with path and raw value, increment `dropped_field_count`, retry.
6. If no path can be found or removal fails, fall back to defaults.

Missing fields still use serde `#[serde(default)]` on `Settings`. Unknown top-level keys remain ignored by serde as before.

## Related

- Non-JSON rename: `corrupt-config-rename.md`
- Startup toasts: `config-load-toasts.md`

## Tests

Unit tests in `src/config/settings.rs` and `src/config/persistence.rs` (`cargo test persistence`, `cargo test settings`).
