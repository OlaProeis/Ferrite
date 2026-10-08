# HTML Entity Decoding in Rendered/Split

Common named and numeric HTML entities in inline markdown render as Unicode glyphs in Rendered and Split preview instead of raw `&amp;` text or `«HTML»` placeholders ([#173](https://github.com/OlaProeis/Ferrite/issues/173)). Arbitrary HTML tags are **not** enabled — only entity decoding on safe inline fragments.

## Key files

| File | Role |
|------|------|
| `src/markdown/html_entities.rs` | `decode_html_entities()`, `is_entity_or_plain_text()` |
| `src/markdown/parser.rs` | `transform_inline_html_siblings()` converts entity-only `HtmlInline` → `Text` |
| `src/markdown/editor.rs` | `render_inline_node()` decodes entity-only `HtmlInline` as normal text |

Manual fixture: [`test_md/test_html_entities.md`](../../../test_md/test_html_entities.md).

## Supported entities

**Named** (minimum bar): `&amp;` `&lt;` `&gt;` `&quot;` `&nbsp;` `&rarr;` `&larr;` `&reg;` `&copy;` `&mdash;` `&ndash;` `&hellip;`

**Numeric**: `&#…;` (decimal) and `&#x…;` (hex) for the same code points.

**Unknown named** entities (e.g. `&foo;`) remain **literal** — soft-fail, no panic.

## Pipeline

Comrak emits entity references as `HtmlInline` siblings inside paragraphs. After GitHub HTML inline processing (`process_github_html_inline`):

1. If `is_entity_or_plain_text(html)` — fragment contains no `<` (no tag markup) — decode and replace the node with `MarkdownNodeType::Text`.
2. Otherwise the node stays `HtmlInline` (unrecognized tags, nested HTML, unsafe content).

At render time, any remaining entity-only `HtmlInline` is decoded in `render_inline_node`; tag-bearing fragments still show `«HTML»`.

**Plain paragraph pitfall:** After parse-time decode, entity-only lines look like plain `Text` and would otherwise use the source-backed `TextEdit` (raw `&amp;`). Those slices are detected via `contains_html_entities` and routed through the formatted-block display path, which decodes in `flush_plain` / `build_inline_markdown_layout_job`. Click-to-edit still edits raw source.

**Caret mapping:** `map_displayed_to_raw` in `widgets.rs` collapses `&amp;` → one displayed glyph for caret math, but **skips** entity collapsing while `in_code_span` or `in_link_text` so `` `&amp;` `` and `[&amp;](url)` map to the raw `&` offset.

Example: `&amp; &rarr; &reg;` → glyphs `& → ®`.

## Tests

| Location | Coverage |
|----------|----------|
| `src/markdown/html_entities.rs` | Named/numeric decode, mixed entities, unknown literal, `is_entity_or_plain_text` |
| `src/markdown/parser.rs` | `test_html_entities_decode_in_paragraph`, `test_html_entities_fixture_parses_without_panic` |

## Related

- GitHub HTML subset (tags, sanitization): [`github-html-subset.md`](./github-html-subset.md)
- Out of scope: GitHub HTML Phase 3, remote images, Mermaid
