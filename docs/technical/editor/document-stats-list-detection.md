# Document Stats — List Item Detection

List item counting in the Stats panel uses `DocumentStats::is_list_item` in `src/editor/stats.rs`. This helper must never panic on valid UTF-8 input.

## Problem ([#171](https://github.com/OlaProeis/Ferrite/issues/171))

The previous implementation gated unordered-list detection on **byte length** (`trimmed.len() >= 2`) and then indexed characters with `.unwrap()`. A single multi-byte character (e.g. `§`, `ß`, `é`, CJK) can satisfy `len() >= 2` in bytes while exposing only **one** Unicode scalar value, so `chars().nth(1).unwrap()` panicked when typing those characters at line start.

Ordered-list detection had a similar hazard: byte-indexed `nth(dot_pos + 1).unwrap()` after `find('.')` could land on a non-char boundary or past the end of the string.

## Fix

Both branches now iterate with `trimmed.chars()` and use `Option` matching — no `.unwrap()` on character indices.

**Unordered** (`- `, `* `, `+ `):

```rust
if let (Some(first), Some(second)) = (chars.next(), chars.next()) {
    if (first == '-' || first == '*' || first == '+') && second == ' ' {
        return true;
    }
}
```

**Ordered** (`1. `, `2) `, …): walk digits (max 9), then require `.` or `)` followed by a space via `matches!(chars.next(), Some(' '))`.

## Regression tests

In `src/editor/stats.rs` (`#[cfg(test)]`):

| Test | Asserts |
|------|---------|
| `test_is_list_item_multibyte_no_panic` | `"§ foo"`, `"ß"`, `"中文"`, `"é item"` → `false`, no panic |
| `test_is_list_item_ascii_markers` | `- item`, `* item`, `1. item`, etc. still match; `-item` / `1.item` do not |
| `test_doc_stats_multibyte_line_start_no_panic` | `from_text` with multi-byte lines counts exactly one real list item |

## Related

- Panel overview: [`document-statistics.md`](../document-statistics.md)
- Implementation: `src/editor/stats.rs` (`is_list_item`, `from_text` line loop)
