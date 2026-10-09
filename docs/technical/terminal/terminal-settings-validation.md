# Terminal settings inline validation (ISS-19)

## Problem

Settings → Terminal exposed plain `TextEdit`s for startup command, custom prompt regex patterns, and optional notification sound path with no feedback when values were invalid. Invalid regex, missing sound files, and dubious commands were saved silently.

## Behaviour

Validation is **non-blocking**: settings still save; a small red hint appears under the field using `error_fg_color`.

| Field | Renderer | Checks |
|-------|----------|--------|
| Startup command | `render_startup_command` | Non-empty command that is only whitespace; unbalanced `'` / `"` (shell-style, `\` escapes inside double quotes) |
| Custom prompt patterns | `render_prompt_patterns` | Each non-empty line compiled with `regex::Regex::new`; first error shown as `Line N: …` |
| Custom sound file | `render_sound` (when sound enabled) | Path must exist; extension must be in `TERMINAL_SOUND_EXTENSIONS` |

Empty startup command and empty sound path are allowed (optional fields).

### Allowed sound extensions

`wav`, `wave`, `ogg`, `oga`, `mp3`, `aiff`, `aif`, `flac`, `m4a` — aligned with cross-platform players used in `src/terminal/sound.rs`.

## Code layout

- **Validators:** `src/ui/settings/terminal_validation.rs` — pure functions returning `Option` / small error enums.
- **UI:** `src/ui/settings/terminal.rs` — `show_validation_hint` after each `TextEdit`.
- **i18n:** `settings.terminal.validation_*` keys in `locales/en.yaml` (regex message passes through compile error text).

## Tests

```bash
cargo test terminal_validation --bin ferrite
```

Unit tests cover valid/invalid regex lines, missing sound file, bad extension (temp file), whitespace-only startup command, and unbalanced quotes.

## Related

- Prompt patterns are still applied at runtime in the terminal manager; invalid regex in saved config may fail there until the user fixes the pattern.
- Sound playback already falls back to system beep when the custom file is missing; the hint is UX-only.
