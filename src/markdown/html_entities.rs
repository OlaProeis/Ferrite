//! Decode common HTML named and numeric entities for inline markdown rendering.
//!
//! Covers a small safe subset used in GitHub-flavored markdown without enabling
//! arbitrary HTML. Unknown named entities are left literal.

use std::collections::HashMap;

/// Named entities supported for decode (minimum bar from #173).
fn named_entity_map() -> HashMap<&'static str, char> {
    HashMap::from([
        ("amp", '&'),
        ("lt", '<'),
        ("gt", '>'),
        ("quot", '"'),
        ("nbsp", '\u{00A0}'),
        ("rarr", '\u{2192}'),
        ("larr", '\u{2190}'),
        ("reg", '\u{00AE}'),
        ("copy", '\u{00A9}'),
        ("mdash", '\u{2014}'),
        ("ndash", '\u{2013}'),
        ("hellip", '\u{2026}'),
    ])
}

/// Whether an inline HTML fragment has no tag markup (plain text and/or entities only).
pub fn is_entity_or_plain_text(html: &str) -> bool {
    !html.contains('<')
}

/// True when `input` contains at least one entity that [`decode_html_entities`] rewrites.
///
/// Used by rendered plain-paragraph/list paths: those widgets paint raw source, so
/// entity-bearing slices must take the inline (AST) render path instead.
pub fn contains_html_entities(input: &str) -> bool {
    decode_html_entities(input) != input
}

/// If `input` starts with a recognized `&…;` / `&#…;` / `&#x…;` entity, return the
/// decoded character and the **char** length consumed in `input`.
pub fn consume_leading_entity(input: &str) -> Option<(char, usize)> {
    if !input.starts_with('&') {
        return None;
    }
    let bytes = input.as_bytes();
    let mut i = 1;
    if i >= bytes.len() {
        return None;
    }

    let map = named_entity_map();
    if bytes[i] == b'#' {
        i += 1;
        let hex = bytes.get(i) == Some(&b'x') || bytes.get(i) == Some(&b'X');
        if hex {
            i += 1;
        }
        let num_start = i;
        while i < bytes.len() && is_digit_for_base(bytes[i], hex) {
            i += 1;
        }
        if i > num_start && bytes.get(i) == Some(&b';') {
            let num_str = &input[num_start..i];
            let code = if hex {
                u32::from_str_radix(num_str, 16).ok()
            } else {
                num_str.parse::<u32>().ok()
            };
            if let Some(ch) = code.and_then(char::from_u32) {
                let end = i + 1;
                return Some((ch, input[..end].chars().count()));
            }
        }
        return None;
    }

    if bytes[i].is_ascii_alphabetic() {
        let name_start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if bytes.get(i) == Some(&b';') {
            let name = &input[name_start..i];
            if let Some(&ch) = map.get(name) {
                let end = i + 1;
                return Some((ch, input[..end].chars().count()));
            }
        }
    }
    None
}

/// Decode `&name;`, `&#…;`, and `&#x…;` entities in `input`.
///
/// Unknown named entities remain literal (`&foo;`). Malformed references copy the
/// `&` and continue scanning.
pub fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '&' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let remaining: String = chars[i..].iter().collect();
        if let Some((ch, consumed)) = consume_leading_entity(&remaining) {
            out.push(ch);
            i += consumed;
        } else {
            out.push('&');
            i += 1;
        }
    }
    out
}

fn is_digit_for_base(b: u8, hex: bool) -> bool {
    if hex {
        b.is_ascii_hexdigit()
    } else {
        b.is_ascii_digit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_named_entities() {
        assert_eq!(decode_html_entities("&amp;"), "&");
        assert_eq!(decode_html_entities("&lt;"), "<");
        assert_eq!(decode_html_entities("&gt;"), ">");
        assert_eq!(decode_html_entities("&quot;"), "\"");
        assert_eq!(decode_html_entities("&nbsp;"), "\u{00A0}");
        assert_eq!(decode_html_entities("&rarr;"), "\u{2192}");
        assert_eq!(decode_html_entities("&larr;"), "\u{2190}");
        assert_eq!(decode_html_entities("&reg;"), "\u{00AE}");
        assert_eq!(decode_html_entities("&copy;"), "\u{00A9}");
        assert_eq!(decode_html_entities("&mdash;"), "\u{2014}");
        assert_eq!(decode_html_entities("&ndash;"), "\u{2013}");
        assert_eq!(decode_html_entities("&hellip;"), "\u{2026}");
    }

    #[test]
    fn test_decode_numeric_entities() {
        assert_eq!(decode_html_entities("&#38;"), "&");
        assert_eq!(decode_html_entities("&#x26;"), "&");
        assert_eq!(decode_html_entities("&#8594;"), "\u{2192}");
        assert_eq!(decode_html_entities("&#x2192;"), "\u{2192}");
        assert_eq!(decode_html_entities("&#174;"), "\u{00AE}");
    }

    #[test]
    fn test_decode_mixed_entities_no_panic() {
        let decoded = decode_html_entities("&amp; &rarr; &reg;");
        assert_eq!(decoded, "& \u{2192} \u{00AE}");
    }

    #[test]
    fn test_unknown_entity_left_literal() {
        assert_eq!(decode_html_entities("&foo;"), "&foo;");
        assert_eq!(decode_html_entities("&amp"), "&amp");
    }

    #[test]
    fn test_is_entity_or_plain_text() {
        assert!(is_entity_or_plain_text("&amp;"));
        assert!(is_entity_or_plain_text("plain"));
        assert!(!is_entity_or_plain_text("<br>"));
        assert!(!is_entity_or_plain_text("<span>"));
    }

    #[test]
    fn test_contains_html_entities() {
        assert!(contains_html_entities("&amp; &rarr; &reg;"));
        assert!(contains_html_entities("a&#38;b"));
        assert!(!contains_html_entities("&foo;"));
        assert!(!contains_html_entities("plain & ampersand"));
    }

    #[test]
    fn test_consume_leading_entity() {
        assert_eq!(consume_leading_entity("&amp; more"), Some(('&', 5)));
        assert_eq!(consume_leading_entity("&#8594;"), Some(('\u{2192}', 7)));
        assert_eq!(consume_leading_entity("&foo;"), None);
        assert_eq!(consume_leading_entity("amp;"), None);
    }
}
