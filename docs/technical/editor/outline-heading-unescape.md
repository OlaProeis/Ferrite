# Outline Heading Backslash Unescape

## Overview

The document outline panel displays ATX heading titles with inline markdown stripped for readability. Headings that use CommonMark backslash escapes (e.g. `## 1\.3\.1 Title`) previously showed literal backslashes in the outline while the preview rendered `1.3.1 Title`. This behaviour is fixed in `src/editor/outline.rs` (#166).

## Pipeline

Heading display text is cleaned in `parse_atx_heading()` via `clean_heading_title()`:

1. **Mask** — `mask_backslash_escapes()` replaces each `\` + escapable ASCII punctuation pair with a private-use placeholder **`U+E000 + escape_index`** (not a single repeated char) and records the escaped character in order.
2. **Strip** — `strip_inline_formatting()` removes emphasis, code spans, links, etc. from the masked string.
3. **Restore** — `restore_backslash_escapes()` decodes each placeholder by its encoded index into `escaped_chars`, so formatting strip removing a middle placeholder cannot desync remaining restores.

Masking is required because a naive unescape-then-strip order would still remove `\*literal asterisks\*` as emphasis delimiters after unescaping to `*literal asterisks*`.

## Escapable characters

Matches the CommonMark ASCII punctuation set:

`!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~`

When `\` precedes a character outside this set, the backslash is kept (e.g. `\n` → `\n`).

## Navigation offsets unchanged

`OutlineItem::line` and `OutlineItem::char_offset` still reference the raw source line. Only the display `title` field is cleaned.

## Key functions

| Function | Role |
|----------|------|
| `clean_heading_title()` | Production entry: mask → strip → restore |
| `unescape_backslashes()` | Standalone unescape (mask + restore); unit-tested |
| `mask_backslash_escapes()` / `restore_backslash_escapes()` | Internal helpers |
| `is_backslash_escapable()` | CommonMark punctuation check |

## Examples

| Source heading text | Outline title |
|---------------------|---------------|
| `1\.3\.1 Title` | `1.3.1 Title` |
| `\*literal asterisks\*` | `*literal asterisks*` |
| `C:\\temp\\file` | `C:\temp\file` |
| `backslash before letter\n` | `backslash before letter\n` |
| `**Bold**` | `Bold` |

## Tests

Unit tests in `src/editor/outline.rs` (`#[cfg(test)]`):

- Integration via `extract_outline()` for each example above
- Direct `test_unescape_backslashes` for the unescape helper
- `restore_backslash_escapes_survives_stripped_placeholder` — gap in placeholder sequence still restores correct chars

Run: `cargo test -p ferrite -- outline`
