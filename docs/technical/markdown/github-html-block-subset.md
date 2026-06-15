# GitHub HTML Block Subset (Phase 1)

## Overview

Safe block-level GitHub HTML in the rendered view: aligned `<div>`, collapsible `<details>/<summary>`, and `<br>` line breaks. Unsafe tags remain passthrough indicators; inline HTML tags are Phase 2 (task 19).

## Key Files

- `src/markdown/parser.rs` — `process_github_html_blocks()`, AST nodes, sanitization helpers
- `src/markdown/editor.rs` — `render_aligned_div`, `render_details`, `LineBreak` spacing
- `src/markdown/widgets.rs` — Round-trip serialization to source HTML

## Supported Syntax (Phase 1)

```markdown
<div align="center">

Centered paragraph

</div>

<details>
<summary>Click to expand</summary>

Hidden content

</details>

<details open>
<summary>Expanded by default</summary>

Visible content

</details>

<br>

Line one<br>Line two
```

| Construct | Behaviour |
|-----------|-----------|
| `<div align="left\|center\|right">` | Children laid out with matching horizontal alignment |
| `<details>` / `<summary>` | Collapsible block; **closed by default** unless `open` attribute present |
| `<br>` (block or inline) | Hard line break (`LineBreak` AST node) |

## Parser Pipeline

After comrak conversion and block-level fixes, `parse_markdown` calls `process_github_html_blocks(&mut converted_root)` before wikilink/video extraction.

1. **`transform_html_block_siblings`** — walks direct children at each tree level
2. **Unsafe check** — `is_unsafe_html()` rejects `<script>`, `<style>`, `<iframe>`, and `on*` event-handler attributes; those stay as `HtmlBlock` passthrough
3. **Standalone `<br>`** — block-level `HtmlBlock` → `LineBreak`
4. **`<div align>`** — opening `HtmlBlock` + inner siblings + closing `</div>` `HtmlBlock` → `AlignedDiv { alignment }`; **single-line** `<div align="…">…</div>` in one `HtmlBlock` → `coalesce_single_block_div()` (re-parses inner markdown)
5. **`<details>`** — opening tag (with embedded `<summary>`) + inner + `</details>` → `Details { summary, open }`
6. **Inline `<br>`** — inside paragraphs, `HtmlInline` → `LineBreak` via `convert_inline_br_in_paragraph`

### AST Nodes

```rust
AlignedDiv { alignment: TableAlignment }  // Left | Center | Right
Details { summary: String, open: bool }   // open from HTML attribute only
LineBreak                                 // existing node; also used for <br>
```

Closing tags are separate comrak `HtmlBlock` siblings; `find_closing_html_block` scans forward for `</div>` or `</details>`.

## Renderer

### Aligned div

`render_aligned_div` wraps children in `with_block_text_align()` so nested paragraphs inherit `TableAlignment`. Block layout uses `top_down(LEFT)` for center (text alignment is per-widget); right blocks use `top_down(RIGHT)`. Formatted and inline HTML paragraphs inside aligned divs go through `with_block_align_widget()` so center/right shrink-wrap the painted galley or inline row — **do not** set `LayoutJob.halign` for center (causes split-view bleed past the pane divider).

### Details

`render_details` mirrors callout collapse UX: arrow + summary row, click toggles via persisted egui id `(details_render, start_line, end_line)`. Initial collapsed state is `!open` (HTML `open` attribute only — not markdown-driven). Summary text may contain inline HTML/markdown via `render_details_summary()`.

### Unrecognized HTML

`render_unrecognized_html_block` shows a small `«HTML»` indicator (HTML comments are silently skipped).

## Serialization

`widgets.rs` reconstructs:

- `<div align="…">…</div>`
- `<details>` / `<details open>` with `<summary>…</summary>`
- `LineBreak` → markdown hard break (`  \n`) in inline context

## Tests

Parser tests in `parser.rs` (`GitHub HTML Phase 1 Tests`):

- `test_html_div_align_center`, `test_html_div_align_left_and_right`, `test_html_single_line_div_align`
- `test_html_details_closed_by_default`, `test_html_details_open_attribute`
- `test_html_standalone_br_block`, `test_html_inline_br_in_paragraph`
- `test_unsafe_script_html_stays_passthrough`, `test_unsafe_iframe_html_stays_passthrough`
- `test_github_html_fixture_parses_without_panic` — full [`test_md/test_github_html.md`](../../../test_md/test_github_html.md) fixture

## Out of Scope

- **Phase 2 inline tags** — documented in [github-html-subset.md](./github-html-subset.md) (not repeated here)
- **Phase 3 (v0.3.2):** nested HTML, HTML tables
