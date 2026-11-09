//! Per-character simple case folding for literal find/replace.
//!
//! Each input scalar maps to exactly one folded scalar so byte offsets in the
//! original UTF-8 string stay aligned with character indices. This fixes Turkish
//! **İ** (U+0130) matching ASCII `i`, which full-string `str::to_lowercase` mishandles.
//!
//! **Not supported:** German **ß** is not folded to `ss` (and `SS` is not folded to ß).

use std::ops::Range;

/// Latin capital letter I with dot above (U+0130).
const LATIN_CAPITAL_I_DOT: char = '\u{0130}';

/// Simple case fold for one Unicode scalar (one output char per input char).
#[must_use]
pub fn fold_char(c: char) -> char {
    if c == LATIN_CAPITAL_I_DOT {
        'i'
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// All non-overlapping literal matches of `needle` in `haystack`, case-insensitive.
///
/// Ranges are byte indices into `haystack` (start inclusive, end exclusive).
pub fn find_all_case_insensitive(haystack: &str, needle: &str) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }

    let needle_folded: Vec<char> = needle.chars().map(fold_char).collect();
    let haystack_chars: Vec<(usize, char)> = haystack
        .char_indices()
        .map(|(byte_idx, c)| (byte_idx, fold_char(c)))
        .collect();

    if haystack_chars.len() < needle_folded.len() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    let last_start = haystack_chars.len().saturating_sub(needle_folded.len());

    while start <= last_start {
        if haystack_chars[start..start + needle_folded.len()]
            .iter()
            .map(|(_, fc)| *fc)
            .eq(needle_folded.iter().copied())
        {
            let byte_start = haystack_chars[start].0;
            let byte_end = haystack_chars
                .get(start + needle_folded.len())
                .map(|(idx, _)| *idx)
                .unwrap_or(haystack.len());
            ranges.push(byte_start..byte_end);
            start += needle_folded.len();
        } else {
            start += 1;
        }
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istanbul_query_finds_three_variants() {
        let haystack = "İstanbul Istanbul istanbul";
        let needle = "istanbul";
        let ranges = find_all_case_insensitive(haystack, needle);
        assert_eq!(ranges.len(), 3);
        for r in &ranges {
            let slice = &haystack[r.start..r.end];
            assert!(
                slice == "İstanbul" || slice == "Istanbul" || slice == "istanbul",
                "unexpected slice: {slice:?}"
            );
        }
    }

    #[test]
    fn eszett_does_not_match_ss() {
        assert!(find_all_case_insensitive("straße", "strasse").is_empty());
        assert!(find_all_case_insensitive("STRASSE", "straße").is_empty());
        assert!(find_all_case_insensitive("abc", "ABC").len() == 1);
    }

    #[test]
    fn ascii_case_insensitive_unchanged() {
        let haystack = "Hello HELLO hello";
        let ranges = find_all_case_insensitive(haystack, "hello");
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0], 0..5);
        assert_eq!(ranges[1], 6..11);
        assert_eq!(ranges[2], 12..17);
    }

    #[test]
    fn fold_char_istanbul_dot_i() {
        assert_eq!(fold_char('İ'), 'i');
        assert_eq!(fold_char('I'), 'i');
    }
}
