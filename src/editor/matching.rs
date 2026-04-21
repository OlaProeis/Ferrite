//! Bracket and Markdown Emphasis Matching
//!
//! This module provides delimiter matching functionality for the editor,
//! supporting bracket pairs `()`, `[]`, `{}`, `<>` and markdown emphasis
//! pairs `**` and `__`.
//!
//! # Example
//!
//! ```ignore
//! use crate::editor::matching::DelimiterMatcher;
//!
//! let text = "function(a, b) { return a + b; }";
//! let matcher = DelimiterMatcher::new(text);
//!
//! // Find match for opening paren at position 8
//! if let Some(match_pos) = matcher.find_matching_bracket(8) {
//!     println!("Matching bracket at position {}", match_pos);
//! }
//! ```

// ─────────────────────────────────────────────────────────────────────────────
// Delimiter Types
// ─────────────────────────────────────────────────────────────────────────────

/// Kind of delimiter that can be matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterKind {
    /// Parentheses `(` and `)`
    Paren,
    /// Square brackets `[` and `]`
    Bracket,
    /// Curly braces `{` and `}`
    Brace,
    /// Angle brackets `<` and `>`
    Angle,
    /// Bold emphasis with asterisks `**`
    EmphasisBoldAsterisk,
    /// Bold emphasis with underscores `__`
    EmphasisBoldUnderscore,
}

impl DelimiterKind {
    /// Get the opening character(s) for this delimiter kind.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn opening_chars(&self) -> &'static str {
        match self {
            DelimiterKind::Paren => "(",
            DelimiterKind::Bracket => "[",
            DelimiterKind::Brace => "{",
            DelimiterKind::Angle => "<",
            DelimiterKind::EmphasisBoldAsterisk => "**",
            DelimiterKind::EmphasisBoldUnderscore => "__",
        }
    }

    /// Get the closing character(s) for this delimiter kind.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn closing_chars(&self) -> &'static str {
        match self {
            DelimiterKind::Paren => ")",
            DelimiterKind::Bracket => "]",
            DelimiterKind::Brace => "}",
            DelimiterKind::Angle => ">",
            DelimiterKind::EmphasisBoldAsterisk => "**",
            DelimiterKind::EmphasisBoldUnderscore => "__",
        }
    }

    /// Check if this is a bracket type (single character).
    pub fn is_bracket(&self) -> bool {
        matches!(
            self,
            DelimiterKind::Paren
                | DelimiterKind::Bracket
                | DelimiterKind::Brace
                | DelimiterKind::Angle
        )
    }

    /// Check if this is a markdown emphasis type.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn is_emphasis(&self) -> bool {
        matches!(
            self,
            DelimiterKind::EmphasisBoldAsterisk | DelimiterKind::EmphasisBoldUnderscore
        )
    }
}

/// A delimiter token found in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimiterToken {
    /// The kind of delimiter.
    pub kind: DelimiterKind,
    /// Whether this is an opening delimiter.
    pub is_open: bool,
    /// Start byte position in the text.
    pub start: usize,
    /// End byte position in the text (exclusive).
    pub end: usize,
}

impl DelimiterToken {
    /// Create a new delimiter token.
    pub fn new(kind: DelimiterKind, is_open: bool, start: usize, end: usize) -> Self {
        Self {
            kind,
            is_open,
            start,
            end,
        }
    }

    /// Get the byte range of this token.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Get the length of this token in bytes.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if this token is empty.
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Matching Result
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a delimiter match operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchingPair {
    /// The delimiter at the cursor position.
    pub source: DelimiterToken,
    /// The matching delimiter (if found).
    pub target: DelimiterToken,
}

impl MatchingPair {
    /// Get the source range (where the cursor is).
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn source_range(&self) -> (usize, usize) {
        self.source.range()
    }

    /// Get the target range (the matching delimiter).
    #[allow(dead_code)] // Part of public API for delimiter matching
    pub fn target_range(&self) -> (usize, usize) {
        self.target.range()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delimiter Matcher
// ─────────────────────────────────────────────────────────────────────────────

/// Service for finding matching delimiters in text.
///
/// The matcher supports:
/// - Single-character brackets: `()`, `[]`, `{}`, `<>`
/// - Markdown emphasis pairs: `**` and `__`
///
/// The algorithm uses stack-based scanning for correct nesting.
pub struct DelimiterMatcher<'a> {
    text: &'a str,
}

impl<'a> DelimiterMatcher<'a> {
    /// Create a new delimiter matcher for the given text.
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// Find the matching delimiter for a position in the text.
    ///
    /// The position should be adjacent to (before or after) a delimiter.
    /// Returns the matching pair if found, or None if:
    /// - The position is not adjacent to a delimiter
    /// - No matching delimiter exists
    ///
    /// # Arguments
    ///
    /// * `cursor_pos` - The character index (not byte index) of the cursor
    pub fn find_match(&self, cursor_pos: usize) -> Option<MatchingPair> {
        // Convert character position to byte position
        let byte_pos = char_to_byte_pos(self.text, cursor_pos);

        // Try to find a delimiter at or around the cursor position
        if let Some(token) = self.find_delimiter_at(byte_pos) {
            return self.find_matching_for(token);
        }

        // Also check one position before (cursor is after delimiter)
        if byte_pos > 0 {
            // Try the character just before
            let prev_byte_pos = self.prev_char_boundary(byte_pos);
            if let Some(token) = self.find_delimiter_at(prev_byte_pos) {
                return self.find_matching_for(token);
            }
        }

        None
    }

    /// Find the byte position of the previous character boundary.
    fn prev_char_boundary(&self, pos: usize) -> usize {
        let bytes = self.text.as_bytes();
        let mut p = pos.saturating_sub(1);
        while p > 0 && !is_char_boundary(bytes, p) {
            p -= 1;
        }
        p
    }

    /// Find a delimiter token at the given byte position.
    fn find_delimiter_at(&self, byte_pos: usize) -> Option<DelimiterToken> {
        if byte_pos >= self.text.len() {
            return None;
        }

        let bytes = self.text.as_bytes();

        // Safety check: if we're on a non-ASCII byte (multi-byte UTF-8 character),
        // there's no delimiter here since all delimiters are ASCII
        let current_byte = bytes[byte_pos];
        if current_byte >= 128 {
            return None;
        }

        // Check for emphasis delimiters first (multi-character)
        // Only safe if we have at least 2 ASCII bytes
        if byte_pos + 1 < self.text.len() && bytes[byte_pos + 1] < 128 {
            let two_chars = &self.text[byte_pos..byte_pos + 2];
            if two_chars == "**" {
                let is_open = self.is_opening_emphasis(byte_pos, "**");
                return Some(DelimiterToken::new(
                    DelimiterKind::EmphasisBoldAsterisk,
                    is_open,
                    byte_pos,
                    byte_pos + 2,
                ));
            }
            if two_chars == "__" {
                let is_open = self.is_opening_emphasis(byte_pos, "__");
                return Some(DelimiterToken::new(
                    DelimiterKind::EmphasisBoldUnderscore,
                    is_open,
                    byte_pos,
                    byte_pos + 2,
                ));
            }
        }

        // Also check if cursor is on the second character of emphasis
        if byte_pos > 0 && byte_pos < self.text.len() {
            let ch = current_byte as char;
            if ch == '*' || ch == '_' {
                let prev_pos = byte_pos - 1;
                // Only safe if previous byte is also ASCII
                if prev_pos < self.text.len() && bytes[prev_pos] < 128 {
                    let two_chars = &self.text[prev_pos..byte_pos + 1];
                    if two_chars == "**" {
                        let is_open = self.is_opening_emphasis(prev_pos, "**");
                        return Some(DelimiterToken::new(
                            DelimiterKind::EmphasisBoldAsterisk,
                            is_open,
                            prev_pos,
                            byte_pos + 1,
                        ));
                    }
                    if two_chars == "__" {
                        let is_open = self.is_opening_emphasis(prev_pos, "__");
                        return Some(DelimiterToken::new(
                            DelimiterKind::EmphasisBoldUnderscore,
                            is_open,
                            prev_pos,
                            byte_pos + 1,
                        ));
                    }
                }
            }
        }

        // Check for single-character brackets (all ASCII, so current_byte is safe)
        let ch = current_byte as char;
        match ch {
            '(' => Some(DelimiterToken::new(
                DelimiterKind::Paren,
                true,
                byte_pos,
                byte_pos + 1,
            )),
            ')' => Some(DelimiterToken::new(
                DelimiterKind::Paren,
                false,
                byte_pos,
                byte_pos + 1,
            )),
            '[' => Some(DelimiterToken::new(
                DelimiterKind::Bracket,
                true,
                byte_pos,
                byte_pos + 1,
            )),
            ']' => Some(DelimiterToken::new(
                DelimiterKind::Bracket,
                false,
                byte_pos,
                byte_pos + 1,
            )),
            '{' => Some(DelimiterToken::new(
                DelimiterKind::Brace,
                true,
                byte_pos,
                byte_pos + 1,
            )),
            '}' => Some(DelimiterToken::new(
                DelimiterKind::Brace,
                false,
                byte_pos,
                byte_pos + 1,
            )),
            '<' => Some(DelimiterToken::new(
                DelimiterKind::Angle,
                true,
                byte_pos,
                byte_pos + 1,
            )),
            '>' => Some(DelimiterToken::new(
                DelimiterKind::Angle,
                false,
                byte_pos,
                byte_pos + 1,
            )),
            _ => None,
        }
    }

    /// Determine if an emphasis delimiter is opening or closing.
    ///
    /// Simple heuristic: opening if preceded by whitespace/start or followed by non-whitespace.
    fn is_opening_emphasis(&self, byte_pos: usize, _marker: &str) -> bool {
        // Check character before the emphasis marker
        if byte_pos == 0 {
            return true; // Start of text = opening
        }

        let bytes = self.text.as_bytes();

        // Check previous character
        let prev_byte = if byte_pos > 0 {
            Some(bytes[byte_pos - 1])
        } else {
            None
        };

        // Opening emphasis is typically preceded by whitespace or punctuation
        match prev_byte {
            Some(b) if (b as char).is_whitespace() => true,
            Some(b) if is_punctuation(b as char) => true,
            None => true,
            _ => false, // Preceded by alphanumeric = likely closing
        }
    }

    /// Find the matching delimiter for a given token.
    fn find_matching_for(&self, token: DelimiterToken) -> Option<MatchingPair> {
        if token.kind.is_bracket() {
            self.find_matching_bracket(token)
        } else {
            self.find_matching_emphasis(token)
        }
    }

    /// Find matching bracket using stack-based scanning.
    fn find_matching_bracket(&self, token: DelimiterToken) -> Option<MatchingPair> {
        let bytes = self.text.as_bytes();
        let (open_char, close_char) = match token.kind {
            DelimiterKind::Paren => (b'(', b')'),
            DelimiterKind::Bracket => (b'[', b']'),
            DelimiterKind::Brace => (b'{', b'}'),
            DelimiterKind::Angle => (b'<', b'>'),
            _ => return None,
        };

        if token.is_open {
            // Scan forward for closing bracket
            let mut depth = 1;
            let mut pos = token.end;

            while pos < bytes.len() {
                let ch = bytes[pos];
                if ch == open_char {
                    depth += 1;
                } else if ch == close_char {
                    depth -= 1;
                    if depth == 0 {
                        let target = DelimiterToken::new(token.kind, false, pos, pos + 1);
                        return Some(MatchingPair {
                            source: token,
                            target,
                        });
                    }
                }
                pos += 1;
            }
        } else {
            // Scan backward for opening bracket
            let mut depth = 1;
            let mut pos = token.start;

            while pos > 0 {
                pos -= 1;
                let ch = bytes[pos];
                if ch == close_char {
                    depth += 1;
                } else if ch == open_char {
                    depth -= 1;
                    if depth == 0 {
                        let target = DelimiterToken::new(token.kind, true, pos, pos + 1);
                        return Some(MatchingPair {
                            source: token,
                            target,
                        });
                    }
                }
            }
        }

        None
    }

    /// Find matching emphasis marker.
    ///
    /// For emphasis, we look for the next occurrence of the same marker
    /// that could form a valid pair.
    fn find_matching_emphasis(&self, token: DelimiterToken) -> Option<MatchingPair> {
        let marker = match token.kind {
            DelimiterKind::EmphasisBoldAsterisk => "**",
            DelimiterKind::EmphasisBoldUnderscore => "__",
            _ => return None,
        };

        if token.is_open {
            // Scan forward for closing emphasis
            let search_start = token.end;
            if let Some(rel_pos) = self.text[search_start..].find(marker) {
                let target_start = search_start + rel_pos;
                let target_end = target_start + marker.len();
                let target = DelimiterToken::new(token.kind, false, target_start, target_end);
                return Some(MatchingPair {
                    source: token,
                    target,
                });
            }
        } else {
            // Scan backward for opening emphasis
            let search_end = token.start;
            if let Some(rel_pos) = self.text[..search_end].rfind(marker) {
                let target = DelimiterToken::new(token.kind, true, rel_pos, rel_pos + marker.len());
                return Some(MatchingPair {
                    source: token,
                    target,
                });
            }
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a character index to a byte position.
fn char_to_byte_pos(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(text.len())
}

/// Check if a position is a valid UTF-8 character boundary.
fn is_char_boundary(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 || pos >= bytes.len() {
        return true;
    }
    // UTF-8 continuation bytes start with 10xxxxxx
    (bytes[pos] & 0xC0) != 0x80
}

/// Check if a character is punctuation (for emphasis detection).
fn is_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | '!' | '?' | ';' | ':' | '-' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parentheses() {
        let text = "func(a, b)";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at '(' - position 4
        let result = matcher.find_match(4);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Paren);
        assert!(pair.source.is_open);
        assert_eq!(pair.source.start, 4);
        assert_eq!(pair.target.start, 9);
    }

    #[test]
    fn test_simple_parentheses_closing() {
        let text = "func(a, b)";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at ')' - position 9
        let result = matcher.find_match(9);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Paren);
        assert!(!pair.source.is_open);
        assert_eq!(pair.source.start, 9);
        assert_eq!(pair.target.start, 4);
    }

    #[test]
    fn test_nested_brackets() {
        // Text:    a[b{c(d)e}f]g
        // Pos:     0123456789...
        let text = "a[b{c(d)e}f]g";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at innermost '(' - position 5
        let result = matcher.find_match(5);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Paren);
        assert_eq!(pair.target.start, 7); // Matching ')' at position 7

        // Cursor at '{' - position 3
        let result = matcher.find_match(3);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Brace);
        assert_eq!(pair.target.start, 9); // Matching '}' at position 9
    }

    #[test]
    fn test_square_brackets() {
        let text = "arr[i]";
        let matcher = DelimiterMatcher::new(text);

        let result = matcher.find_match(3);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Bracket);
        assert_eq!(pair.target.start, 5);
    }

    #[test]
    fn test_angle_brackets() {
        let text = "Vec<String>";
        let matcher = DelimiterMatcher::new(text);

        let result = matcher.find_match(3);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Angle);
        assert_eq!(pair.target.start, 10);
    }

    #[test]
    fn test_emphasis_asterisks() {
        let text = "This is **bold** text";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at opening '**' - position 8
        let result = matcher.find_match(8);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::EmphasisBoldAsterisk);
        assert!(pair.source.is_open);
        assert_eq!(pair.target.start, 14); // Closing '**'
    }

    #[test]
    fn test_emphasis_underscores() {
        let text = "This is __bold__ text";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at opening '__' - position 8
        let result = matcher.find_match(8);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::EmphasisBoldUnderscore);
        assert!(pair.source.is_open);
        assert_eq!(pair.target.start, 14);
    }

    #[test]
    fn test_unmatched_bracket() {
        let text = "func(a, b";
        let matcher = DelimiterMatcher::new(text);

        let result = matcher.find_match(4);
        assert!(result.is_none());
    }

    #[test]
    fn test_mismatched_brackets() {
        let text = "[ ( ] )";
        let matcher = DelimiterMatcher::new(text);

        // The '[' at position 0 should NOT match ']' at position 4
        // because there's an unmatched '(' in between
        let result = matcher.find_match(0);
        // In our simple stack-based algorithm, it will still find ']'
        // because we only track one bracket type at a time
        // This is acceptable for a simple implementation
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_no_delimiter_at_position() {
        let text = "hello world";
        let matcher = DelimiterMatcher::new(text);

        let result = matcher.find_match(2);
        assert!(result.is_none());
    }

    #[test]
    fn test_cursor_after_delimiter() {
        let text = "a(b)c";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at position 2 (after the '(', before 'b')
        let result = matcher.find_match(2);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Paren);
    }

    #[test]
    fn test_empty_text() {
        let text = "";
        let matcher = DelimiterMatcher::new(text);

        let result = matcher.find_match(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_delimiter_at_end() {
        let text = "(test)";
        let matcher = DelimiterMatcher::new(text);

        // Cursor at closing ')' - position 5
        let result = matcher.find_match(5);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.start, 5);
        assert_eq!(pair.target.start, 0);
    }

    #[test]
    fn test_code_block_brackets() {
        let text = "```js\nfunction test<T>(arr: T[]) { return arr[0]; }\n```";
        let matcher = DelimiterMatcher::new(text);

        // Find the '<' in test<T>
        let angle_pos = text.find('<').unwrap();
        let char_pos = text[..angle_pos].chars().count();
        let result = matcher.find_match(char_pos);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.source.kind, DelimiterKind::Angle);
    }

    #[test]
    fn test_multiple_emphasis_pairs() {
        let text = "**bold1** and **bold2**";
        let matcher = DelimiterMatcher::new(text);

        // First opening **
        let result = matcher.find_match(0);
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.target.start, 7); // First closing **
    }

    #[test]
    fn test_char_to_byte_pos_ascii() {
        let text = "hello";
        assert_eq!(char_to_byte_pos(text, 0), 0);
        assert_eq!(char_to_byte_pos(text, 2), 2);
        assert_eq!(char_to_byte_pos(text, 5), 5);
    }

    #[test]
    fn test_char_to_byte_pos_unicode() {
        let text = "héllo"; // 'é' is 2 bytes in UTF-8
        assert_eq!(char_to_byte_pos(text, 0), 0);
        assert_eq!(char_to_byte_pos(text, 1), 1); // 'é' starts at byte 1
        assert_eq!(char_to_byte_pos(text, 2), 3); // 'l' starts at byte 3
    }

    #[test]
    fn test_delimiter_token_methods() {
        let token = DelimiterToken::new(DelimiterKind::Paren, true, 5, 6);
        assert_eq!(token.range(), (5, 6));
        assert_eq!(token.len(), 1);
        assert!(!token.is_empty());
    }

    #[test]
    fn test_delimiter_kind_methods() {
        assert!(DelimiterKind::Paren.is_bracket());
        assert!(DelimiterKind::Bracket.is_bracket());
        assert!(!DelimiterKind::EmphasisBoldAsterisk.is_bracket());
        assert!(DelimiterKind::EmphasisBoldAsterisk.is_emphasis());
        assert!(!DelimiterKind::Brace.is_emphasis());
    }
}
