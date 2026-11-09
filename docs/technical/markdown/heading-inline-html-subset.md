# Inline HTML in headings (HTML-4)

## Overview

Rendered headings use the same GitHub inline HTML subset as paragraphs for `<br>`, `<kbd>`, `<sup>`, `<sub>`, and sized `<img>`. Plain ATX headings without those constructs keep the existing single-line `TextEdit` path.

Outline and folding labels use **plain text** (no raw tags, `<br>` collapsed to whitespace) so titles stay readable in side panels.

## Key files

| File | Role |
|------|------|
| `src/markdown/editor.rs` | `render_heading`: `paragraph_needs_ast_inline_widgets` → `render_inline_content` with heading color; session seed via `heading_plain_text` |
| `src/markdown/parser.rs` | `heading_plain_text`, `strip_github_inline_html_plain` |
| `src/markdown/widgets.rs` | `heading_inline_display_parts` (display text + hard-break flag for tests) |
| `src/editor/outline.rs` | `parse_atx_heading` applies `strip_github_inline_html_plain` after markdown strip |
| `src/editor/folding.rs` | Fold preview title uses `strip_github_inline_html_plain` |

Re-exports: `crate::markdown::{heading_plain_text, strip_github_inline_html_plain}`.

## Behaviour

| Context | `<br>` | `<kbd>Ctrl</kbd>` |
|---------|--------|-------------------|
| Rendered display | `LineBreak` → wrapped row break (`render_inline_content`) | `render_kbd_span` with inner text |
| Session / commit seed | Space in `heading_plain_text` (no U+FFFD in single-line editor) | Inner text preserved |
| Outline / folding | Stripped to plain words | Inner text only |

Unknown inline tags still follow the existing passthrough / `«HTML»` chip behaviour inside the AST inline path.

## Tests

- `cargo test --bin ferrite heading` — `heading_inline_*` in `widgets.rs`, parser plain-text tests, outline HTML title
- `cargo test --bin ferrite html` — existing GitHub HTML parser tests plus `test_html_heading_inline_*`

See also: [GitHub HTML subset (Phases 1–2)](./github-html-subset.md).
