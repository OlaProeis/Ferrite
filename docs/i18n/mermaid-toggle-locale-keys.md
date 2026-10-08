# Mermaid Inline Fit Toggle — Locale Keys

English base strings for the hover-revealed Fit/Native header toggle on inline Mermaid blocks.

## Keys (`locales/en.yaml`)

| Key | Value | Shown when |
|-----|-------|------------|
| `mermaid.fit_width` | Fit width | Native/scroll mode is active (click to enable fit-to-pane) |
| `mermaid.native_size` | Native size | Fit mode is active (click to show diagram at native width) |

Keys live in the `mermaid:` section alongside `badge`, `empty`, and error strings.

## Usage

`MermaidBlock::show` in `src/markdown/widgets.rs` passes the tooltip via `t!("mermaid.fit_width")` / `t!("mermaid.native_size")` on the fit toggle button.

## Scope

v0.3.1 adds these keys to `locales/en.yaml` only. Translating other locale files is out of scope for this addendum.

## Related

- `docs/technical/mermaid/inline-fit-native-toggle.md` — toggle behaviour, icons, and rendering integration
