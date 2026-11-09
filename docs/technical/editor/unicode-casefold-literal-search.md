# Unicode case folding for literal find/search (ISS-44)

## Problem

Full-string `str::to_lowercase()` and regex `(?i)` / `RegexBuilder::case_insensitive` do not match Turkish **İ** (U+0130) with ASCII `i` the way users expect (`İstanbul` vs query `istanbul`). A lowercase copy also misaligns byte offsets when folded scalars change UTF-8 length.

## Approach

`src/util/casefold.rs` provides per-scalar **simple** folding: one output `char` per input `char`, so character indices stay aligned and match ranges use **byte** indices from `char_indices` on the original string.

| API | Role |
|-----|------|
| `fold_char(c)` | `char::to_lowercase().next()`, except U+0130 → `'i'` |
| `find_all_case_insensitive(haystack, needle)` | Non-overlapping literal matches as `Range<usize>` in `haystack` |

**Not folded:** German **ß** is not treated as `ss` (and `SS` is not folded to ß). Document this limitation for users comparing to full Unicode case folding.

## Call sites

| Area | File | Behavior |
|------|------|----------|
| Editor Find/Replace | `src/editor/find_replace.rs` | Literal + case-insensitive uses `find_all_case_insensitive`; whole-word path compares folded scalars and keeps `advance_past_char_at` on rejection |
| Search in Files | `src/ui/search.rs` | Literal + case-insensitive uses casefold on each line; case-sensitive literal still uses escaped regex; **regex mode unchanged** |

Regex search mode still uses the `regex` crate’s case-insensitive flag (out of scope for ISS-44).

## Tests

- `util::casefold::tests::*` — İstanbul triple match, ASCII, ß vs SS
- `editor::find_replace::tests::test_find_matches_istanbul_case_insensitive`
- `ui::search::tests::test_search_istanbul_case_insensitive`

Related offset contract: [`search-in-files-offsets.md`](search-in-files-offsets.md).
