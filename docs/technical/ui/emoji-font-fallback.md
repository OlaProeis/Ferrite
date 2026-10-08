# Emoji font fallback for document text (#168)

## Problem

Inter and JetBrains Mono (Ferrite's bundled editor fonts) do not include Unicode emoji glyphs. Document text with emoji (`😀`, `✅`, ZWJ sequences) showed tofu or missing glyphs in Raw and Rendered views.

UI chrome uses **Phosphor** icons — this task covers **Unicode emoji in document content only**, not replacing toolbar/panel icons.

## Behaviour

At font init (startup, reload, CJK rebuild), Ferrite attempts to load the platform emoji font and register it as a fallback on both default families used by the editor and markdown preview:

| OS | Candidate fonts (first match wins) |
|----|-------------------------------------|
| Windows | Segoe UI Emoji |
| macOS | Apple Color Emoji |
| Linux | Noto Color Emoji, Noto Emoji |

If no candidate is installed, startup continues with no emoji fallback (no panic). Font bytes are validated once at first load; registration does not re-read disk on every rebuild.

Fallback order on **Proportional** and **Monospace**:

`Custom?` → Inter/JetBrains → **Emoji** → CJK → complex-script fonts

Named families (`Inter`, `JetBrainsMono`, styled variants, custom) inherit this chain via `proportional_fallbacks` / `monospace_fallbacks` snapshots in `create_font_definitions_*`.

## Implementation

### `src/fonts.rs`

- `FONT_EMOJI` (`"Emoji"`) — key in `font_data` and family chains
- `load_emoji_font_validated()` — font-kit `SystemSource`; `validate_emoji_font_bytes` allow-lists TTF/OTF/TTC (TTC uses `FontData.index = 0`)
- `cached_emoji_font()` — process-wide `OnceLock<Option<Arc<FontData>>>`; disk read + validation happen once
- `register_emoji_font_fallback()` — clones cached `Arc<FontData>` into `font_data` and appends to `FontFamily::Proportional` and `FontFamily::Monospace`
- Called from `create_font_definitions_with_settings` and `create_font_definitions_with_cjk_spec` (covers lazy init, settings reload, and lazy CJK rebuilds)
- `EMOJI_FONT_LOADED` atomic + `get_loaded_runtime_font_names()` for Stats runtime visibility
- Atlas prewarm includes sample emoji (`EMOJI_PREWARM_CHARS`) on proportional and monospace

TTC collections (e.g. Apple Color Emoji) use `FontData::index` 0; custom-font TTC rejection in `validate_font_bytes` does not apply to the emoji loader path.

### Out of scope

- Dual raw vs rendered font stacks ([#176](https://github.com/OlaProeis/Ferrite/issues/176))
- Bundling Noto Color Emoji when OS fonts are missing
- PDF export color emoji (see [`pdf-export.md`](../planning/pdf-export-pipeline.md))

## Tests

- `fonts::tests::test_emoji_fallback_registered_when_available` — when OS font loads, `Emoji` appears on both families before any CJK entry
- Manual: doc with `😀 ✅ ⚠️` in Raw and Rendered on Windows; CJK documents unchanged

## Related

- File tree context menus intentionally avoid emoji in chrome labels — see comment in `src/ui/file_tree.rs`
