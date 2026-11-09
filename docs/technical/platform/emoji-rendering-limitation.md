# Colour emoji rendering limitation (ISS-30)

## Summary

Ferrite uses **egui / epaint**, which rasterises text through **outline fonts** (TTF/OTF outlines via `ab_glyph` / `skrifa`). It does **not** rasterise **colour bitmap** tables (CBDT/COLR, Apple bitmap strikes, etc.) from fonts such as **Noto Color Emoji** or **Segoe UI Emoji**.

Even when those OS fonts are registered as fallbacks ([#168](../ui/emoji-font-fallback.md)), document emoji typically appear **monochrome** (outline or bundled **Noto Emoji Regular**), not in full colour. This is a **documented platform limitation**, not a missing font install.

## What renders reliably

| Source | Role |
|--------|------|
| Inter / JetBrains Mono | Primary editor glyphs |
| Bundled **Noto Emoji Regular** (`FONT_NOTO_EMOJI_BUILTIN`) | Monochrome emoji in UI inputs (Settings search) and outline-capable emoji codepoints |
| OS emoji font (`FONT_EMOJI`) | Fallback on Proportional/Monospace; outline subset only |
| `test_md/test_emoji.md` | Manual fixture for Raw / Rendered / Split |

## Codepoints that often fail or degrade

| Class | Examples | Typical symptom |
|-------|----------|-----------------|
| **Variation selectors** | U+FE0F (emoji presentation), U+FE0E (text presentation) | Empty box or spurious “missing glyph” if not treated as zero-width |
| **ZWJ sequences** | 👨‍👩‍👧, 🏳️‍🌈 | May show component glyphs separately; must not crash |
| **Regional indicator pairs (flags)** | 🇳🇴 🇺🇸 | Often no single outline glyph; components may appear |
| **Some newer emoji** | e.g. on `test_emoji.md` line 9 | Blank slot (no tofu) when no outline exists in any fallback |

Colour presentation from the OS colour font is **out of scope** for v0.3.1.

## Ferrite mitigations (task 25)

### Zero-width variation selectors

epaint 0.34 treats many format characters as invisible but **not** U+FE0F. Ferrite:

1. **`[patch.crates-io] epaint`** — `vendor/epaint` adds U+FE0E and U+FE0F to `invisible_char` in `src/text/font.rs` so they render with zero advance.
2. **`src/fonts.rs`** — `ZERO_WIDTH_FONT_SETUP_CHARS` and `is_zero_width_font_setup_char` (FE0F, FE0E, U+200D) so atlas **pre-warm** never probes selectors in isolation (e.g. from `⚠️` or ZWJ lines).
3. **`missing_glyph_codepoints_for_prewarm`** — tests and diagnostics skip zero-width setup chars when checking `has_glyph`.

U+200D is already invisible in stock epaint; it is listed in Ferrite’s setup set so pre-warm stays explicit for ZWJ fixtures.

### Replacement glyph and pre-warm

epaint’s family replacement glyph is **◻** (U+25FB). Ferrite pre-warms **◻** via `COMMON_SYMBOLS` to reduce `Failed to find replacement characters '◻' or '?'. Will use empty glyph.` at startup.

`EMOJI_PREWARM_CHARS` includes the line-9 samples from `test_emoji.md` (😀😅🤔😴🥳😢😡) plus existing samples; zero-width code points in those strings are filtered during pre-warm.

## Future direction (separate PRD)

Full **colour emoji** would need an opt-in path outside outline rasterisation, for example:

- Per-glyph **texture atlas** from colour font strikes or a bundled colour source, or
- Platform text APIs composited into egui textures.

Not planned for v0.3.1 closeout.

## Tests

- `fonts::tests::test_zero_width_font_setup_includes_variation_selector`
- `fonts::tests::test_prewarm_does_not_report_missing_glyph_for_variation_selector`
- Manual: open `test_md/test_emoji.md` in Raw and Rendered; expect monochrome emoji, no empty box after ⚠ from U+FE0F alone

## Related

- [`emoji-font-fallback.md`](../ui/emoji-font-fallback.md) — OS emoji font registration (#168)
- [`settings-search-emoji-font.md`](../ui/settings-search-emoji-font.md) — Settings search bundled Noto Emoji (#ISS-29)
