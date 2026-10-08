# Terminal font picker and Nerd Font fallback (#185)

## Problem

The integrated terminal painted with `FontId::monospace` (JetBrains Mono only) and had no path to Nerd Font private-use glyphs. Powerlevel10k-style prompts showed empty boxes.

## Behaviour

- **Settings → Terminal → Terminal Font** (`terminal.font_family`): default **JetBrains Mono**, or a system face via the shared `custom_font_picker`.
- `Settings.terminal_font_family: Option<String>` — `None` (serde default) uses JetBrains Mono. Empty / whitespace sanitizes to `None`.
- The terminal widget uses named family `ferrite-terminal` (`FONT_TERMINAL`): `[custom? , JetBrainsMono] + monospace fallbacks`.
- Cell metrics stay `'M'`-based. A proportional pick keeps the grid aligned by that width (glyphs may look uneven).
- **Lazy Nerd Font fallback:** PTY output containing private-use / Powerline characters sets `NERD_FONT_REQUESTED`. After `poll_all`, `ensure_nerd_font_loaded` tries to load a system Nerd Font once.
- On success, `FONT_NERD` is inserted on `FontFamily::Monospace` and `ferrite-terminal` **after** `FONT_EMOJI`. Rebuild uses `bump_font_generation` + `schedule_prewarm` and keeps the current CJK / complex-script preferences.
- Missing Nerd Font: no toast, one debug log, glyphs stay as boxes. Ferrite does not bundle a Nerd Font.
- Cell paint is clipped to `cell_rect.expand2(vec2(0.0, 1.0))` so wide Powerline glyphs stay in-grid.

## PUA detection

`is_pua_char` / `needs_nerd_font`:

| Range | Role |
|-------|------|
| U+E000–U+F8FF | BMP private use |
| U+F0000–U+FFFFD | Supplementary PUA-A |
| U+E0A0–U+E0D4 | Powerline (also inside BMP PUA; listed explicitly) |

## Load order

1. `Symbols Nerd Font Mono`
2. `Symbols Nerd Font`
3. `SymbolsNerdFontMono-Regular`
4. `list_system_fonts()` names containing `Nerd Font` or ending ` NF` / ` NFM`, Mono preferred

First successful `load_system_font_by_name` wins. Bytes are leaked once and reused on later rebuilds (`NERD_FONT_LOADED`).

## Out of scope

- Bundling any font
- Ligature support
- Changing cell-width logic in `screen.rs`

## Tests

- `fonts::tests::test_is_pua_char_range_boundaries` — each range edge; `'a'` rejected
- `test_needs_nerd_font` — `"\u{e0b0}"` true, `"abc"` false
- `test_nerd_font_excluded_when_not_loaded` / `test_nerd_font_included_after_emoji_when_loaded` — inject bytes via the test hook
- `test_font_selection_from_settings` — `FontSelection.terminal` from `terminal_font_family`

Manual (Windows with a Nerd Font): print U+E0B0, U+F015, U+E702 in the integrated PowerShell — one glyph per cell. Without a Nerd Font: boxes, no crash.

## Key files

- `src/fonts.rs` — `FONT_TERMINAL`, `needs_nerd_font`, `ensure_nerd_font_loaded`, `register_nerd_font_fallback`
- `src/terminal/widget.rs` — `font_family` + cell clip
- `src/terminal/mod.rs` — `Terminal::poll` sets `NERD_FONT_REQUESTED`
- `src/ui/terminal_panel.rs` — family on the widget; `ensure_nerd_font_loaded` after `poll_all`
- `src/config/settings.rs` — `terminal_font_family`
- `src/ui/settings/terminal.rs` — `render_font_family`

## Related

- Custom-font registry / `FontSelection`: [`rendered-raw-fonts.md`](../ui/rendered-raw-fonts.md)
- Emoji fallback order: [`emoji-font-fallback.md`](../ui/emoji-font-fallback.md)
