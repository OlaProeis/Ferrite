# Config load toasts

## Problem

Users need to know when `config.json` was unreadable (renamed aside) or when tolerant load reset invalid fields, without repeating the message every frame.

## Locale keys

| Key | When |
|-----|------|
| `notification.config_unreadable_reset` | Syntax-invalid file renamed; `%{path}` is the backup filename |
| `notification.config_fields_invalid_reset` | ≥1 field dropped by tolerant load; `%{count}` is the number reset |

Defined in `locales/en.yaml` and translated locales per project convention (`de`, `ja`, `zh_Hans`, `es`, `et`; `ru`/`cs` empty placeholders; `pt`/`nb_NO` fall back to English).

## API

| Symbol | Location | Role |
|--------|----------|------|
| `ConfigLoadOutcome::Loaded { dropped_field_count, .. }` | `src/config/persistence.rs` | Carries tolerant recovery count on successful parse |
| `config_load_toast_kinds` | `src/config/persistence.rs` | Maps corrupt flag + count → `ConfigLoadToastKind` (tests) |
| `AppState.corrupt_config_renamed_to` | `src/state.rs` | Set in `AppState::new` from outcome or `take_pending_corrupt_config_rename` |
| `AppState.config_invalid_fields_reset` | `src/state.rs` | `Some(count)` when `dropped_field_count >= 1` at startup |
| `AppState::show_pending_config_load_toasts` | `src/state.rs` | `take()` both flags; `show_toast` with localized strings |
| `FerriteApp` frame drain | `src/app/mod.rs` | Calls `show_pending_config_load_toasts` before `pending_toast` (`CONFIG_LOAD_TOAST_SECS` = 30 s) |

## Behavior

1. **Corrupt JSON** — See `corrupt-config-rename.md`. After rename, `corrupt_config_renamed_to` holds the backup path (including when `main` loaded first and the file is already gone on the second load).
2. **Partial recovery** — See `tolerant-settings-load.md`. `dropped_field_count` from `load_config_outcome` becomes `config_invalid_fields_reset` in `AppState::new`.
3. **One-shot** — Each flag is cleared with `take()` when its toast is shown; later frames are no-ops.
4. Only one toast slot exists in UI; corrupt and partial signals do not occur on the same load path.
5. **Observability (round 3, QA ISS-72 / EXTRA-SET-9a/b).** GrokBot's re-run reported "no toast" but its screenshot was taken roughly a minute after launch, after the original 5 s toast had expired — the log showed the rename happened. The toast now lives 30 s (`CONFIG_LOAD_TOAST_SECS` in `src/app/mod.rs`) and `show_pending_config_load_toasts` emits `log::info!("Config load toast: …")` for both kinds, so a headless harness can confirm the toast fired from `ferrite.log` even when the screenshot misses it.

## Tests

- `config::persistence::tests::test_config_load_toast_kinds`
- `state::tests::test_pending_config_load_corrupt_toast_clears_flag`
- `state::tests::test_pending_config_load_fields_toast_clears_flag`
