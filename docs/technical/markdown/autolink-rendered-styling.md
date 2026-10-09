# Autolink styling in rendered markdown (ISS-26)

## Problem

Comrak parses bare URLs and `<https://…>` as `Link` nodes when `MarkdownOptions::autolink` is true. Many paragraphs still paint through the **formatted-block** path (`build_inline_markdown_layout_job` → `parse_inline_markdown`), which only styled `[text](url)` and wikilinks. Autolinks in source stayed plain body text (no link color or underline).

AST-driven `render_link` / `RenderedLinkWidget` already styled links when a paragraph used the inline-widget path (e.g. images). Mixed and autolink-only formatted paragraphs needed the layouter fix.

## Solution

### Inline layouter (`src/markdown/widgets.rs`)

`parse_inline_markdown` now recognizes:

- Angle-bracket autolinks: `<https://example.com>`, `<mailto:user@host>`
- Bare URI autolinks: `http://`, `https://`, `www.`
- Bare email autolinks: `user@domain.tld`

Each match calls `append_link_span` (same `link_color` and underline as `[text](url)`).

`map_displayed_to_raw` skips `<` / `>` for angle autolinks so click-to-edit caret mapping stays aligned with the painted galley.

`collect_inline_display_spans` builds test-friendly `(text, is_link)` slices from layout job sections.

### Parser metadata (`src/markdown/parser.rs`)

- `MarkdownLinkKind`: `Inline`, `AutolinkUri`, `AutolinkEmail`
- `classify_markdown_link(url, display_text)` after parse (`finalize_link_kinds`)
- Used by `RenderedLinkState::from_link_kind` so email (`mailto:` URL vs display) and `www.` → `http://www.` pairs count as autolinks for safe edit (no `[text](url)` injection)

### Editor (`src/markdown/editor.rs`)

`render_link` passes `MarkdownLinkKind` into `RenderedLinkState::from_link_kind` when the AST inline path runs.

## Out of scope

HTML export / bat theme autolink styling (follow-up elsewhere).

## Tests

- `parser`: bare and angle autolinks → `Link` + `AutolinkUri`; `[label](url)` → `Inline`
- `widgets`: `collect_inline_display_spans` for bare URL, angle URL, and markdown link
- `editor`: AST inline spans mark `Link` children as links

## Related

- [`editable-links.md`](editable-links.md) — hover edit popup; autolink-safe URL-only edits
- [`table-inline-formatting.md`](table-inline-formatting.md) — same `parse_inline_markdown` stack in table cells
