# GitHub HTML Subset (Phases 1–2)

## Overview

Ferrite renders a **safe GitHub-style HTML subset** in the rendered markdown view. Phase 1 covers block-level constructs; Phase 2 adds common inline tags and sized images. Everything outside this list remains a passthrough indicator (`«HTML»`) or raw `HtmlInline` text in the source.

**Out of scope (Phase 3, v0.3.2):** nested HTML wrappers, HTML tables, and additional tags/attributes.

## Key Files

| File | Role |
|------|------|
| `src/markdown/parser.rs` | `process_github_html_blocks()`, `process_github_html_inline()`, sanitization, AST coalescing |
| `src/markdown/editor.rs` | Render aligned div, details, kbd, sup/sub, sized images |
| `src/markdown/widgets.rs` | Round-trip serialization back to source HTML/markdown |

See also: [GitHub HTML block subset (Phase 1)](./github-html-block-subset.md) for block-level pipeline details.

## Sanitization

`is_unsafe_html()` rejects content that must never be rendered as structured HTML:

| Rule | Examples |
|------|----------|
| Blocked tags | `<script>`, `<style>`, `<iframe>` |
| Event handlers | Any `on*` attribute (`onclick=`, `onerror =`, …) |

Unsafe block HTML stays as `HtmlBlock` passthrough. Unsafe inline HTML stays as `HtmlInline` passthrough.

## Supported Tags — Phase 1 (Block)

| Tag / construct | Attributes | Behaviour |
|-----------------|------------|-----------|
| `<div align="…">` | `align="left\|center\|right"` | Children laid out with matching horizontal alignment |
| `<details>` / `<summary>` | `open` (optional) | Collapsible block; **closed by default** unless `open` present |
| `<br>` | — | Hard line break (`LineBreak` AST node) |

### AST nodes (Phase 1)

```rust
AlignedDiv { alignment: TableAlignment }  // Left | Center | Right
Details { summary: String, open: bool }
LineBreak                                 // also used for <br>
```

## Supported Tags — Phase 2 (Inline)

| Tag | Attributes | Behaviour |
|-----|------------|-----------|
| `<kbd>` | — | Boxed monospace key cap (GitHub-style) |
| `<sup>` | — | Smaller text, raised baseline |
| `<sub>` | — | Smaller text, lowered baseline |
| `<img>` | `src`, `alt`, `title`, `width`, `height` | Rendered via image pipeline; explicit `width`/`height` respected (scaled down if wider than viewport) |

### AST nodes (Phase 2)

```rust
Kbd                                       // container for inner inline content
Superscript                               // container (<sup> or ^text^ when enabled)
Subscript                                 // container
Image { url, title, width: Option<u32>, height: Option<u32> }
```

Comrak emits opening/closing `HtmlInline` siblings for `<kbd>`, `<sup>`, and `<sub>`; the parser coalesces them into container nodes. Standalone or inline `<img …>` tags become `Image` nodes with optional dimensions.

**Nested HTML inside wrappers is not coalesced** — e.g. `<kbd><b>Ctrl</b></kbd>` stays as passthrough `HtmlInline` nodes.

## Parser Pipeline

After comrak conversion and block-level fixes, `parse_markdown` runs:

1. `process_github_html_blocks()` — block `<div>`, `<details>`, `<br>`, block-level `<img>`
2. `process_github_html_inline()` — inline `<kbd>`, `<sup>`, `<sub>`, inline `<img>`
3. `extract_wikilinks()` / video embed extraction (unchanged)

## Renderer

| Construct | Implementation |
|-----------|----------------|
| `<kbd>` | `render_kbd_span()` — framed monospace box with border |
| `<sup>` / `<sub>` | `render_script_span()` — 75% font size, `Align::TOP` / `Align::BOTTOM` valign |
| Sized `<img>` | `render_image()` — uses HTML dimensions when set, then scales to available width |
| Inline HTML paragraphs | `paragraph_has_html_inline()` → `render_inline_content()` (AST path); avoids showing raw `<kbd>` / `<br>` source in WYSIWYG display |
| `<div align="center">` | `with_block_text_align()` + `with_block_align_widget()` — shrink-wrap formatted/inline rows and center/right via egui layout justify (not galley `halign`, which mis-paints in split view) |
| `<summary>` with HTML | `render_details_summary()` — mini `parse_markdown` + `render_inline_node` for bold/inline markup in the summary row |

Markdown `![alt](url)` images continue to use natural pixel dimensions (`width`/`height` = `None`).

## Serialization

`widgets.rs` reconstructs:

- Phase 1: `<div align="…">`, `<details>` / `<details open>`, hard breaks
- Phase 2: `<kbd>…</kbd>`, `<sup>…</sup>`, `<sub>…</sub>`
- Images with HTML dimensions → `<img src="…" width="…" height="…" alt="…">`; otherwise standard `![alt](url)` markdown

## Examples

```markdown
Press <kbd>Ctrl</kbd>+<kbd>S</kbd> to save.

Water is H<sub>2</sub>O; E=mc<sup>2</sup>.

<img src="assets/logo.png" width="120" height="40" alt="Logo">
```

## Explicitly Unsupported (Phase 3+)

- Nested HTML (wrappers inside wrappers, HTML emphasis inside `<kbd>`, etc.)
- HTML tables (`<table>`, `<tr>`, `<td>`, …)
- Arbitrary tags: `<span>`, `<a href>`, `<b>`, `<i>`, `<mark>`, etc.
- `<iframe>`, `<script>`, `<style>`, and event-handler attributes
- Remote/web images (existing image pipeline limitation — local paths only)

## Manual fixture

[`test_md/test_github_html.md`](../../../test_md/test_github_html.md) — rendered/split checklist for Phases 1–2 (alignment, details, `<br>`, kbd/sup/sub, sized images, unsafe passthrough, unsupported tags).

## Tests

Parser tests in `parser.rs`:

**Phase 1:** `test_html_div_align_*`, `test_html_single_line_div_align`, `test_html_details_*`, `test_html_*_br_*`, `test_unsafe_*`

**Phase 2:** `test_html_kbd_inline`, `test_html_kbd_chord`, `test_html_sup_inline`, `test_html_sub_inline`, `test_html_img_inline_dimensions`, `test_html_img_block_dimensions`, `test_html_nested_kbd_stays_passthrough`

**Integration:** `test_github_html_fixture_parses_without_panic` (loads the manual fixture)
