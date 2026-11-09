# Shipped UI languages (ISS-56)

## Overview

The Settings → Appearance language dropdown lists every locale that has a `locales/*.yaml` file. Runtime switching uses `rust_i18n::set_locale` with `Language::locale_code()` (file stem, e.g. `nb_NO`).

## Shipped locales (10)

| `Language` variant | YAML stem | Config serde |
|--------------------|-----------|--------------|
| English | `en` | `en` |
| ChineseSimplified | `zh_Hans` | `zh-Hans` |
| Czech | `cs` | `cs` |
| German | `de` | `de` |
| Spanish | `es` | `es` |
| Estonian | `et` | `et` |
| Japanese | `ja` | `ja` |
| NorwegianBokmal | `nb_NO` | `nb-NO` |
| Portuguese | `pt` | `pt` |
| Russian | `ru` | `ru` |

## Code

- **`Language` enum and `Language::all()`** — `src/config/settings.rs`. `all()` returns a fixed slice and calls an exhaustive `match` helper so a new enum variant without updating the slice fails to compile.
- **`locale_yaml_stem()`** — Same as `locale_code()` today; used by tests to map variants to `locales/{stem}.yaml`.
- **`from_locale_code`** — Primary BCP-47 tag plus `nb` / `no` → Norwegian Bokmål.
- **Language selector** — `Language::all()` in `src/ui/settings/appearance.rs` and welcome flow.
- **Keyboard command labels** — `src/ui/settings/keyboard.rs` uses `t!("shortcuts.commands.*")` for word wrap, preview lock, terminal, productivity hub, and zoom commands (not hardcoded English).

## Locale keys (ISS-56)

These keys must exist in every shipped YAML file; Japanese and Simplified Chinese must use non-English values (regression-tested):

- `settings.appearance.system_title_bar`
- `settings.appearance.system_title_bar_restart`
- `shortcuts.commands.toggle_word_wrap`
- `shortcuts.commands.toggle_preview_lock`
- `shortcuts.commands.toggle_terminal`
- `shortcuts.commands.toggle_productivity_hub`
- `shortcuts.commands.zoom_in`
- `shortcuts.commands.reset_zoom`

English also defines `shortcuts.commands.zoom_out` for the keyboard settings panel.

## Tests

- `language_all_matches_locale_files` — `config::settings` tests: `Language::all().len()` equals `locales/*.yaml` count; each stem has a file.
- `locale_keys_exist_in_all_yaml` — `spellcheck::tests`: spellcheck keys plus ISS-56 keys in all YAML; ja / zh_Hans ISS-56 values must differ from English.

Targeted: `cargo test --bin ferrite locale_keys_exist_in_all_yaml language_all_matches_locale_files`.

## Adding a language

1. Add `locales/{stem}.yaml` with the full key set (copy structure from `en.yaml`).
2. Add a `Language` variant, serde rename, and update every `match` arm in `Language` impls plus the `all()` slice.
3. Extend tests if new mandatory keys are introduced.

See also `docs/technical/config/i18n.md` and `docs/adding-languages.md`.
