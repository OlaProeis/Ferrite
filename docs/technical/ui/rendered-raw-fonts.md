# Separate Rendered and Raw fonts (#176)

## Problem

One `Settings.font_family` drove both the raw editor and the rendered preview, and Ferrite loaded only a single custom system font (`FONT_CUSTOM`). Users could not use JetBrains Mono in Raw and Inter (or another face) in Rendered.

## Behaviour

- **Raw editor** uses `Settings.font_family` (`EditorFont`: Inter, JetBrains Mono, or Custom).
- **Rendered / Split preview** uses `Settings.rendered_font_family: Option<EditorFont>`.
  - `None` (serde default) = same as the editor. Existing configs are unchanged.
  - `Some(font)` = independent pick (built-in or custom).
- Settings → Appearance: **Rendered view font** with a **Same as editor** toggle plus the same built-in / custom picker. Registry id `appearance.rendered_font_family` (`featured: false`).
- Empty `Custom("")` on the rendered field sanitizes to `None` (picker not finalized).

## Custom-font registry

Custom faces are keyed `"custom:<family name>"` (not a single `"Custom"` slot):

- Bytes leak **once per family** into `CUSTOM_FONT_BYTES: LazyLock<Mutex<HashMap<…>>>`; later rebuilds reuse the slice.
- Each loaded custom gets `FontFamily::Name("custom:<name>")` = `[font] + proportional_fallbacks` **after** emoji / CJK / complex-script fallbacks are attached.
- Only the **editor** custom (if any) is prepended to `FontFamily::Proportional` (UI look unchanged). A rendered-only custom is registered as a named family but does not become the UI face.
- `get_styled_font_family` / `get_base_font_family` `Custom(name)` → `Name("custom:{name}")`.
- `ttf_bytes_for_font_id_shaping` looks up `FontFamily::Name(n)` in the registry **first**, then Inter / JetBrains named faces.

## `FontSelection`

Rebuild functions take `FontSelection { editor, rendered, terminal: Option<String> }` instead of `custom_font: Option<&str>`. Fields are **custom family names**; built-ins are `None`.

`FontSelection::from_settings` maps `EditorFont::custom_name()`. **`terminal`** comes from `Settings.terminal_font_family` (see [`terminal-nerd-fonts.md`](../terminal/terminal-nerd-fonts.md)).

Failed loads: `revert_unloaded_custom_fonts` resets the editor to Inter and/or rendered to `None` for names in `LAST_CUSTOM_FONT_FAILED`.

## Builders

`create_font_definitions_with_settings` and `create_font_definitions_with_cjk_spec` are thin wrappers over one private `create_font_definitions_inner`. Lazy CJK / complex-script rebuilds keep every registered custom family.

## Wiring

| Surface | Font |
|---------|------|
| `EditorWidget` (Raw / Split left) | `settings.font_family` |
| `MarkdownEditor` (Rendered / Split right) | `rendered_font_family.unwrap_or(font_family)` |
| `MarkdownEditor::with_settings` | same rendered fallback |
| Settings change detection | compares both font fields + CJK preference |

## Out of scope

- Code-block / `FontFamily::Monospace` changes
- HarfRust shaping for the rendered view

Terminal font picker and Nerd Font fallback: [`terminal-nerd-fonts.md`](../terminal/terminal-nerd-fonts.md) (#185).

## Tests

- `fonts::tests::test_font_selection_from_settings` — `None` / `Some(Custom)` / built-in combinations; `terminal` from `terminal_font_family`
- `test_two_custom_font_families_registered` — `Name("custom:X")` and `Name("custom:Y")` when editor and rendered differ
- `test_proportional_fallback_order` — editor custom → Inter → JetBrains → emoji → CJK
- `test_builder_entry_points_same_families` — both public builders produce identical family maps
- `test_ttf_bytes_looks_up_custom_name_first`
- `config::settings::tests::test_rendered_font_family_defaults_none_and_sanitizes_pending`

## Key files

- `src/config/settings.rs` — `rendered_font_family`, Default, `sanitize`
- `src/fonts.rs` — `FontSelection`, registry, shared builder
- `src/app/central_panel.rs` — rendered font pass + change detection
- `src/ui/settings/appearance.rs` — `custom_font_picker`, `render_rendered_font_family`
- `src/ui/settings/registry.rs` — `appearance.rendered_font_family`

## Related

- Emoji fallback order: [`emoji-font-fallback.md`](emoji-font-fallback.md)
- Settings search / recents: [`settings-search-redesign.md`](settings-search-redesign.md)
