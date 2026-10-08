# Search-in-Files offset contract (#178)

## Problem

Ctrl+Shift+F (Search in Files) crashed on the first multi-byte match because the literal-search loop advanced by one **byte** (`start = abs_pos + 1`) and could land inside a UTF-8 codepoint. The same path also:

- Built a `to_lowercase()` copy and treated its byte offsets as positions in the original line (caret landed past the match in non-ASCII documents).
- Advanced document offsets with `line.len() + 1`, which drifts one character per CRLF line (`\r\n` is two chars, not one).

`FindState` whole-word rejection used the same `+ 1` byte advance and panicked on accented terms such as `é`.

## Offset contract

`SearchMatch` carries two parallel offset systems:

| Field | Unit | Scope | Consumer |
|-------|------|-------|----------|
| `match_start`, `match_end` | **bytes** | `line_content` | Result-row highlight painting (`floor_char_boundary`) |
| `char_offset`, `match_len` | **characters** | Whole document | `handle_search_navigation` → `Tab::set_cursor` / `set_transient_highlight` |

Document character offset for a match:

```
char_offset = line_start_char_offset + line[..match_start].chars().count()
match_len   = line[match_start..match_end].chars().count()
```

Line advancement uses `content.split_inclusive('\n')` and `line_segment_without_eol()` so `\r\n` counts as **2** characters; bare `\n` counts as **1**.

## Literal search

Literal mode no longer allocates a lowercase copy. One `regex::RegexBuilder` is compiled per search with `regex::escape(&query)` and `case_insensitive(!case_sensitive)`, then `find_iter(line)` runs on the **original** line (same boundary-safe byte offsets as regex mode).

## Find / replace whole-word advance

When whole-word mode rejects a partial match, advance with `Self::advance_past_char_at` — add `text[byte_index..].chars().next().map_or(1, char::len_utf8)` instead of `+ 1`.

## Key files

- `src/ui/search.rs` — `SearchPanel::search`, `SearchMatch`, `line_segment_without_eol`
- `src/app/file_ops.rs` — `handle_search_navigation` (consumes `char_offset` / `match_len`)
- `src/editor/find_replace.rs` — `FindState::advance_past_char_at`, whole-word literal paths
- `src/app/helpers.rs` — `byte_to_char_offset` (other call sites; search-in-files uses char offsets directly)

## Tests

- `ui::search::tests::test_search_cjk_literal_no_panic`
- `ui::search::tests::test_search_unicode_case_insensitive_offsets`
- `ui::search::tests::test_search_istanbul_multibyte_offsets`
- `ui::search::tests::test_search_crlf_char_offset`
- `find_replace::tests::test_whole_word_accented_char_no_panic`

## Manual QA

Rows **15–16** in [`v0.3.1-minimal-release-test.md`](../platform/v0.3.1-minimal-release-test.md): CJK search-in-files does not crash; clicking a result places the caret on the match.
