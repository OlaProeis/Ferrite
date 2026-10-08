# Document Outline

## Overview

The outline panel builds a navigable table of contents from the active document. Heading extraction lives in `src/editor/outline.rs` via `extract_outline()` — a regex-based pass over raw source text, independent of the CommonMark preview parser (`src/markdown/parser.rs`).

ATX headings (`#` … `######`) are parsed in `parse_atx_heading()`. The display `title` on each `OutlineItem` is cleaned for readability; navigation fields (`line`, `char_offset`) always point at the raw source line.

## Heading title cleaning

Production path: `parse_atx_heading()` → `clean_heading_title()`.

1. **Backslash unescape** — CommonMark escapes are resolved before inline formatting is stripped (#166).
2. **Inline strip** — `strip_inline_formatting()` removes emphasis, code spans, links, images, etc.

See [outline-heading-unescape.md](./outline-heading-unescape.md) for the mask → strip → restore pipeline, examples, and tests.

## Backslash unescape rules

The outline extractor resolves CommonMark backslash escapes in heading text so titles match what the preview renders.

| Input pattern | Result |
|---------------|--------|
| `\` + ASCII punctuation | Punctuation only (backslash dropped) |
| `\` + non-punctuation | Backslash preserved |

Escapable ASCII punctuation matches CommonMark: `!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~`

Examples:

| Source heading text | Outline title |
|---------------------|---------------|
| `1\.3\.1 Title` | `1.3.1 Title` |
| `\*literal asterisks\*` | `*literal asterisks*` |
| `backslash before letter\n` | `backslash before letter\n` |

## Interaction with `strip_inline_formatting`

Unescape runs **before** the inline strip. Escaped punctuation is turned into literal characters first, so it is not treated as markdown syntax during stripping.

Without this ordering, `\*literal asterisks\*` would become `*literal asterisks*` after unescape and then lose the asterisks as emphasis delimiters. The mask → strip → restore pipeline in `clean_heading_title()` avoids that by hiding escaped characters from the strip step until restore.

## Key functions

| Function | Role |
|----------|------|
| `extract_outline()` | Entry: scan lines, build `Outline` |
| `parse_atx_heading()` | Parse `#` prefix; produce cleaned title |
| `clean_heading_title()` | Mask escapes → strip inline → restore escapes |
| `strip_inline_formatting()` | Remove inline markdown from display text |

## Related

- Issue [#166](https://github.com/OlaProeis/Ferrite/issues/166) — outline titles did not unescape backslashes while preview did.
- [outline-heading-unescape.md](./outline-heading-unescape.md) — detailed pipeline, escapable set, and unit tests.
