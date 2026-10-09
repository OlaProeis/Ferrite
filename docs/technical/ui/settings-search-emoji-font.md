# Settings search emoji and epaint replacement glyphs (ISS-29)

## Problem

On Linux QA (Noto Color Emoji installed), pasting emoji (e.g. 🙂) into the Settings search field accepted text but showed **blank glyphs**. Every launch also logged:

`epaint … Failed to find replacement characters '◻' or '?'. Will use empty glyph.`

Document emoji in Raw/Rendered already used the OS emoji fallback from [#168](emoji-font-fallback.md); the Settings `TextEdit` and Phosphor icon family needed separate handling.

## Root cause

1. **Phosphor family** — Registered as icon font only. egui builds a per-family **replacement glyph** from `◻` then `?` when the family is first used. Phosphor has neither, so epaint warns and uses an invisible replacement glyph for that family.

2. **Settings search** — Default `TextEdit` uses `FontFamily::Proportional` with OS **color** emoji in the chain. Color emoji fonts can expose glyphs that do not rasterize reliably in egui text inputs, while still “winning” fallback resolution.

## Behaviour

- **Phosphor** (`FontFamily::Name("phosphor")`): icon font first, then the full **Proportional** fallback chain (Inter, JetBrains, OS emoji, CJK, etc.). Icons unchanged; missing-glyph and replacement resolution use text fonts.

- **Settings input** (`ferrite-settings-input`): dedicated family for UI text fields — Inter → JetBrains → **bundled Noto Emoji Regular** (B&W, from `epaint_default_fonts`) → remainder of Proportional (including OS color emoji). Bundled Noto is preferred before OS color for legible monochrome glyphs in `TextEdit`.

- Settings search uses `fonts::settings_input_font(ui)` on the search `TextEdit` (`src/ui/settings/mod.rs`).

Editor/document Proportional order is unchanged (OS color emoji before CJK); only the settings-input family inserts bundled Noto ahead of the copied proportional tail.

## Implementation

| Location | Role |
|----------|------|
| `src/fonts.rs` | `register_builtin_emoji_font`, `register_settings_input_font_family`, `register_phosphor_icon_font` fallback extend, `settings_input_font` |
| `src/ui/settings/mod.rs` | `.font(settings_input_font(ui))` on settings search |
| `Cargo.toml` | Direct `epaint_default_fonts = "0.34"` for `NOTO_EMOJI_REGULAR` bytes |

Constants: `FONT_NOTO_EMOJI_BUILTIN` (`NotoEmoji-Regular`), `FONT_SETTINGS_INPUT` (`ferrite-settings-input`).

## Tests

- `fonts::tests::test_phosphor_family_includes_proportional_fallbacks`
- `fonts::tests::test_settings_input_family_includes_builtin_emoji_before_os_emoji`

## Manual verification

Launch app, open Settings, paste 🙂 into search — glyph visible, no epaint replacement-character WARN on startup (Phosphor family init).

## Related

- [`emoji-font-fallback.md`](emoji-font-fallback.md) — OS emoji on Proportional/Monospace for document text (#168)
- [`settings-search-redesign.md`](settings-search-redesign.md) — search-first Settings shell
