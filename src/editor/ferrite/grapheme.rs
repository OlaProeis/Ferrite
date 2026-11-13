//! Grapheme cluster boundary utilities for cursor navigation.
//!
//! Maps between char-column offsets and grapheme cluster boundaries so that
//! arrow keys, backspace, and delete operate on whole grapheme clusters
//! (e.g., emoji ZWJ sequences, Bengali conjuncts, Korean jamo).

use unicode_segmentation::UnicodeSegmentation;

/// Returns the char-column of the previous grapheme cluster boundary.
///
/// - If `char_col` is at a grapheme boundary, returns the start of the
///   preceding grapheme cluster.
/// - If `char_col` falls inside a grapheme cluster, returns the start of
///   that cluster (snapping backward).
/// - Returns 0 when already at or before the first grapheme.
pub fn prev_grapheme_boundary(line_text: &str, char_col: usize) -> usize {
    let mut prev_start = 0;
    let mut char_offset = 0;

    for grapheme in line_text.graphemes(true) {
        if char_offset >= char_col {
            return prev_start;
        }
        prev_start = char_offset;
        char_offset += grapheme.chars().count();
    }

    // char_col is at or past end — return the start of the last grapheme.
    prev_start
}

/// Returns the char-column just past the current grapheme cluster.
///
/// - If `char_col` is at a grapheme boundary, returns the end of that
///   grapheme cluster (= start of the next one).
/// - If `char_col` falls inside a grapheme cluster, returns the end of
///   that cluster.
/// - Returns the total char count when already at the last grapheme.
pub fn next_grapheme_boundary(line_text: &str, char_col: usize) -> usize {
    let mut char_offset = 0;

    for grapheme in line_text.graphemes(true) {
        let grapheme_end = char_offset + grapheme.chars().count();
        if char_col < grapheme_end {
            return grapheme_end;
        }
        char_offset = grapheme_end;
    }

    char_offset
}

/// Returns the line text stripped of trailing newline characters.
///
/// `TextBuffer::get_line()` includes the trailing `\n` (or `\r\n`).
/// Grapheme calculations must exclude those terminators.
pub fn line_text_stripped(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

/// True when `char_col` sits exactly on a grapheme cluster boundary.
///
/// A column that is already a boundary must be kept as-is. Snapping it would
/// move an ASCII caret one character backwards for no reason, because
/// `prev_grapheme_boundary` and `next_grapheme_boundary` both return the
/// *neighbouring* boundaries rather than the input.
pub fn is_grapheme_boundary(line_text: &str, char_col: usize) -> bool {
    let mut char_offset = 0;
    for grapheme in line_text.graphemes(true) {
        if char_offset == char_col {
            return true;
        }
        char_offset += grapheme.chars().count();
    }
    char_col == char_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Latin (ASCII) ──────────────────────────────────────────────────

    #[test]
    fn latin_prev_boundary_mid_word() {
        assert_eq!(prev_grapheme_boundary("Hello", 3), 2);
    }

    #[test]
    fn latin_prev_boundary_at_start() {
        assert_eq!(prev_grapheme_boundary("Hello", 0), 0);
    }

    #[test]
    fn latin_next_boundary_mid_word() {
        assert_eq!(next_grapheme_boundary("Hello", 2), 3);
    }

    #[test]
    fn latin_next_boundary_at_end() {
        assert_eq!(next_grapheme_boundary("Hello", 5), 5);
    }

    // ── Emoji ZWJ sequences ────────────────────────────────────────────

    #[test]
    fn emoji_zwj_family_is_single_cluster() {
        // 👨‍👩‍👧 = U+1F468 U+200D U+1F469 U+200D U+1F467 (5 chars, 1 grapheme)
        let s = "👨\u{200D}👩\u{200D}👧";
        let char_count = s.chars().count();
        assert_eq!(char_count, 5);

        // Moving right from 0 should jump to the end
        assert_eq!(next_grapheme_boundary(s, 0), 5);
        // Moving left from 5 should jump to 0
        assert_eq!(prev_grapheme_boundary(s, 5), 0);
    }

    #[test]
    fn emoji_zwj_mid_cluster_snaps() {
        let s = "👨\u{200D}👩\u{200D}👧";
        // Cursor inside the cluster at char 2 — prev should snap to 0
        assert_eq!(prev_grapheme_boundary(s, 2), 0);
        // Next should jump to end
        assert_eq!(next_grapheme_boundary(s, 2), 5);
    }

    #[test]
    fn emoji_with_surrounding_latin() {
        // "ab👨‍👩‍👧cd" — 'a'(0) 'b'(1) emoji(2..7) 'c'(7) 'd'(8)
        let s = "ab👨\u{200D}👩\u{200D}👧cd";
        assert_eq!(s.chars().count(), 9);

        assert_eq!(next_grapheme_boundary(s, 1), 2); // after 'b' → start of emoji
        assert_eq!(next_grapheme_boundary(s, 2), 7); // start of emoji → end of emoji
        assert_eq!(prev_grapheme_boundary(s, 7), 2); // 'c' → start of emoji
        assert_eq!(prev_grapheme_boundary(s, 2), 1); // start of emoji → 'b'
    }

    // ── Bengali conjuncts ──────────────────────────────────────────────

    #[test]
    fn bengali_conjunct_single_cluster() {
        // ক্ষ = ক + ্ + ষ (3 chars, 1 grapheme cluster)
        let s = "ক্ষ";
        let char_count = s.chars().count();
        assert_eq!(char_count, 3);

        assert_eq!(next_grapheme_boundary(s, 0), 3);
        assert_eq!(prev_grapheme_boundary(s, 3), 0);
    }

    // ── Korean jamo ────────────────────────────────────────────────────

    #[test]
    fn korean_precomposed_syllable() {
        // 한 = single precomposed syllable (1 char, 1 grapheme)
        let s = "한글";
        assert_eq!(s.chars().count(), 2);
        assert_eq!(next_grapheme_boundary(s, 0), 1);
        assert_eq!(next_grapheme_boundary(s, 1), 2);
    }

    #[test]
    fn korean_conjoining_jamo() {
        // ᄒ + ᅡ + ᆫ  (Hangul conjoining jamo sequence: 3 chars, 1 grapheme)
        let s = "\u{1112}\u{1161}\u{11AB}";
        let char_count = s.chars().count();
        assert_eq!(char_count, 3);

        assert_eq!(next_grapheme_boundary(s, 0), 3);
        assert_eq!(prev_grapheme_boundary(s, 3), 0);
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn empty_string() {
        assert_eq!(prev_grapheme_boundary("", 0), 0);
        assert_eq!(next_grapheme_boundary("", 0), 0);
    }

    #[test]
    fn single_char() {
        assert_eq!(prev_grapheme_boundary("a", 1), 0);
        assert_eq!(next_grapheme_boundary("a", 0), 1);
    }

    #[test]
    fn line_text_stripped_lf() {
        assert_eq!(line_text_stripped("Hello\n"), "Hello");
    }

    #[test]
    fn line_text_stripped_crlf() {
        assert_eq!(line_text_stripped("Hello\r\n"), "Hello");
    }

    #[test]
    fn line_text_stripped_no_newline() {
        assert_eq!(line_text_stripped("Hello"), "Hello");
    }

    // ── Combining diacritics ───────────────────────────────────────────

    #[test]
    fn combining_diacritics() {
        // "é" as e + combining acute = 2 chars, 1 grapheme
        let s = "e\u{0301}";
        assert_eq!(s.chars().count(), 2);
        assert_eq!(next_grapheme_boundary(s, 0), 2);
        assert_eq!(prev_grapheme_boundary(s, 2), 0);
    }
}
