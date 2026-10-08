//! Markdown-aware word tokenizer for spellcheck.
//!
//! Returns **char** offsets. Skips fenced/indented code, frontmatter, HTML
//! blocks, URLs, link destinations, wikilink targets, math, and similar
//! non-prose tokens. Only Latin-script words are emitted.

use crate::editor::DocumentStats;
use regex::Regex;
use std::sync::OnceLock;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

/// Multi-line skip state for a scanned window.
#[derive(Debug, Clone, Default)]
pub struct LineContext {
    /// Absolute 0-indexed line number of the *next* line passed to [`words_to_check`].
    pub line_index: usize,
    fence: Option<Fence>,
    in_frontmatter: bool,
    in_html_block: bool,
    in_list: bool,
}

#[derive(Debug, Clone, Copy)]
struct Fence {
    ch: char,
    len: usize,
}

impl LineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn starting_at(line_index: usize) -> Self {
        Self {
            line_index,
            ..Self::default()
        }
    }
}

/// Words on `line` that should be spell-checked, as `(char_start, char_end, word)`.
pub fn words_to_check<'a>(line: &'a str, ctx: &mut LineContext) -> Vec<(usize, usize, &'a str)> {
    words_to_check_opts(line, ctx, TokenizeOptions::default())
}

/// Same as [`words_to_check`] but honors ignore-flag settings.
pub fn words_to_check_opts<'a>(
    line: &'a str,
    ctx: &mut LineContext,
    opts: TokenizeOptions,
) -> Vec<(usize, usize, &'a str)> {
    let result = words_to_check_inner_opts(line, ctx, opts);
    ctx.line_index = ctx.line_index.saturating_add(1);
    result
}

/// Skip rules that settings can toggle. Defaults match the engine MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenizeOptions {
    pub ignore_all_caps: bool,
    pub ignore_words_with_digits: bool,
}

impl Default for TokenizeOptions {
    fn default() -> Self {
        Self {
            ignore_all_caps: true,
            ignore_words_with_digits: true,
        }
    }
}

fn words_to_check_inner_opts<'a>(
    line: &'a str,
    ctx: &mut LineContext,
    opts: TokenizeOptions,
) -> Vec<(usize, usize, &'a str)> {
    if ctx.fence.is_some() {
        if is_closing_fence(line, ctx.fence) {
            ctx.fence = None;
        }
        return Vec::new();
    }
    if let Some(fence) = parse_opening_fence(line) {
        ctx.fence = Some(fence);
        return Vec::new();
    }

    if ctx.in_frontmatter {
        if is_frontmatter_delim(line) {
            ctx.in_frontmatter = false;
        }
        return Vec::new();
    }
    if ctx.line_index == 0 && is_frontmatter_delim(line) {
        ctx.in_frontmatter = true;
        return Vec::new();
    }

    let html_open = is_html_block_open(line);
    let html_close = is_html_block_close(line);
    if ctx.in_html_block {
        if html_close {
            ctx.in_html_block = false;
        }
        return Vec::new();
    }
    if html_open && !html_close {
        ctx.in_html_block = true;
        return Vec::new();
    }

    let is_list = DocumentStats::is_list_item(line);
    if is_list {
        ctx.in_list = true;
    } else if is_blank_line(line) {
        // Keep list state across a blank line so indented continuations still match.
    } else if is_indented_code(line) && ctx.in_list {
        // List continuation — check the words.
    } else if is_indented_code(line) {
        ctx.in_list = false;
        return Vec::new();
    } else {
        ctx.in_list = false;
    }

    extract_words(line, opts)
}

fn extract_words(line: &str, opts: TokenizeOptions) -> Vec<(usize, usize, &str)> {
    let skip = collect_skip_ranges(line);
    let byte_to_char = byte_to_char_index(line);
    let mut out = Vec::new();

    for (byte_start, token) in line.split_word_bound_indices() {
        let byte_end = byte_start + token.len();
        if overlaps(byte_start, byte_end, &skip) {
            continue;
        }
        let stripped = strip_edge_punct(token);
        if stripped.is_empty() {
            continue;
        }
        if !first_alpha_is_latin(stripped) {
            continue;
        }
        if should_skip_word(stripped, opts) {
            continue;
        }
        let strip_prefix = token.len() - token.trim_start_matches(is_edge_punct).len();
        let inner_byte = byte_start + strip_prefix;
        let inner_end = inner_byte + stripped.len();
        if inner_end > line.len() || !line.is_char_boundary(inner_byte) || !line.is_char_boundary(inner_end)
        {
            continue;
        }
        let start = byte_to_char.get(inner_byte).copied().unwrap_or(0);
        let end = byte_to_char.get(inner_end).copied().unwrap_or(start);
        out.push((start, end, stripped));
    }
    out
}

fn should_skip_word(word: &str, opts: TokenizeOptions) -> bool {
    if word.contains('_') {
        return true;
    }
    if opts.ignore_words_with_digits && word.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    if opts.ignore_all_caps && is_all_caps(word) {
        return true;
    }
    if is_camel_or_pascal(word) {
        return true;
    }
    if is_hex_or_uuid(word) {
        return true;
    }
    false
}

fn is_all_caps(word: &str) -> bool {
    let mut n = 0usize;
    for c in word.chars() {
        if c.is_alphabetic() {
            if !c.is_uppercase() {
                return false;
            }
            n += 1;
        }
    }
    n >= 2
}

fn is_camel_or_pascal(word: &str) -> bool {
    let mut seen_lower = false;
    for c in word.chars() {
        if c.is_lowercase() {
            seen_lower = true;
        } else if c.is_uppercase() && seen_lower {
            return true;
        }
    }
    false
}

fn is_hex_or_uuid(word: &str) -> bool {
    let w = word.trim_matches(|c| c == '{' || c == '}');
    if w.len() == 36 {
        let mut parts = w.split('-');
        let expected = [8usize, 4, 4, 4, 12];
        for exp in expected {
            match parts.next() {
                Some(p) if p.len() == exp && p.chars().all(|c| c.is_ascii_hexdigit()) => {}
                _ => {
                    return false;
                }
            }
        }
        return parts.next().is_none();
    }
    if let Some(hex) = w
        .strip_prefix("0x")
        .or_else(|| w.strip_prefix("0X"))
    {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    w.len() >= 8 && w.chars().all(|c| c.is_ascii_hexdigit())
}

fn first_alpha_is_latin(word: &str) -> bool {
    word.chars()
        .find(|c| c.is_alphabetic())
        .map(|c| c.script() == Script::Latin)
        .unwrap_or(false)
}

fn is_edge_punct(c: char) -> bool {
    matches!(c, '\'' | '\u{2019}' | '-')
}

fn strip_edge_punct(word: &str) -> &str {
    word.trim_matches(is_edge_punct)
}

fn is_blank_line(line: &str) -> bool {
    line.trim().is_empty()
}

fn is_indented_code(line: &str) -> bool {
    line.starts_with('\t') || line.starts_with("    ")
}

fn is_frontmatter_delim(line: &str) -> bool {
    line.trim() == "---"
}

fn parse_opening_fence(line: &str) -> Option<Fence> {
    let indent = leading_space_count(line);
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let len = rest.chars().take_while(|&c| c == ch).count();
    if len < 3 {
        return None;
    }
    if ch == '`' {
        let after = &rest[len..];
        if after.contains('`') {
            return None;
        }
    }
    Some(Fence { ch, len })
}

fn is_closing_fence(line: &str, open: Option<Fence>) -> bool {
    let Some(open) = open else {
        return false;
    };
    let indent = leading_space_count(line);
    if indent > 3 {
        return false;
    }
    let rest = &line[indent..];
    if !rest.starts_with(open.ch) {
        return false;
    }
    let len = rest.chars().take_while(|&c| c == open.ch).count();
    if len < open.len {
        return false;
    }
    rest.chars().skip(len).all(char::is_whitespace)
}

fn leading_space_count(line: &str) -> usize {
    line.as_bytes().iter().take_while(|&&b| b == b' ').count()
}

fn is_html_block_open(line: &str) -> bool {
    let t = line.trim_start();
    let Some(after) = t.strip_prefix('<') else {
        return false;
    };
    if after.starts_with('/') {
        return false;
    }
    let name: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    HTML_BLOCK_TAGS.contains(&name.as_str())
}

fn is_html_block_close(line: &str) -> bool {
    line.contains("</")
}

const HTML_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "script",
    "section",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "ul",
];

fn collect_skip_ranges(line: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    mask_inline_code(line, &mut ranges);
    mask_math(line, &mut ranges);
    mask_wikilinks(line, &mut ranges);
    mask_link_destinations(line, &mut ranges);
    mask_reference_definition(line, &mut ranges);
    mask_regex(line, html_tag_re(), &mut ranges);
    mask_regex(line, html_entity_re(), &mut ranges);
    mask_regex(line, embed_re(), &mut ranges);
    mask_regex(line, url_re(), &mut ranges);
    mask_regex(line, email_re(), &mut ranges);
    mask_regex(line, mention_re(), &mut ranges);
    mask_regex(line, tag_re(), &mut ranges);
    ranges
}

fn mask_inline_code(line: &str, ranges: &mut Vec<(usize, usize)>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut n = 1;
            while i + n < bytes.len() && bytes[i + n] == b'`' {
                n += 1;
            }
            if let Some(close) = find_backtick_run(bytes, i + n, n) {
                ranges.push((i, close + n));
                i = close + n;
                continue;
            }
        }
        i += 1;
    }
}

fn find_backtick_run(bytes: &[u8], from: usize, n: usize) -> Option<usize> {
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut m = 1;
            while i + m < bytes.len() && bytes[i + m] == b'`' {
                m += 1;
            }
            if m == n {
                return Some(i);
            }
            i += m;
        } else {
            i += 1;
        }
    }
    None
}

fn mask_math(line: &str, ranges: &mut Vec<(usize, usize)>) {
    mask_delimited(line, "$$", "$$", ranges);
    let mut i = 0;
    while let Some(rel) = line[i..].find('$') {
        let start = i + rel;
        if line[start..].starts_with("$$") {
            i = start + 2;
            continue;
        }
        if ranges.iter().any(|&(a, b)| start >= a && start < b) {
            i = start + 1;
            continue;
        }
        if let Some(rel_end) = line[start + 1..].find('$') {
            let end = start + 1 + rel_end + 1;
            if !line[start + 1..end - 1].contains('\n') {
                ranges.push((start, end));
                i = end;
                continue;
            }
        }
        i = start + 1;
    }
}

fn mask_delimited(line: &str, open: &str, close: &str, ranges: &mut Vec<(usize, usize)>) {
    let mut i = 0;
    while let Some(rel) = line[i..].find(open) {
        let start = i + rel;
        let after = start + open.len();
        if after > line.len() {
            break;
        }
        if let Some(rel_end) = line[after..].find(close) {
            let end = after + rel_end + close.len();
            ranges.push((start, end));
            i = end;
        } else {
            break;
        }
    }
}

fn mask_wikilinks(line: &str, ranges: &mut Vec<(usize, usize)>) {
    let mut i = 0;
    while let Some(rel) = line[i..].find("[[") {
        let start = i + rel;
        let inner_at = start + 2;
        if let Some(rel_end) = line[inner_at..].find("]]") {
            let inner_end = inner_at + rel_end;
            let inner = &line[inner_at..inner_end];
            if let Some(pipe) = inner.find('|') {
                ranges.push((start, inner_at + pipe + 1));
                ranges.push((inner_end, inner_end + 2));
            } else {
                ranges.push((start, inner_end + 2));
            }
            i = inner_end + 2;
        } else {
            break;
        }
    }
}

fn mask_link_destinations(line: &str, ranges: &mut Vec<(usize, usize)>) {
    let mut i = 0;
    while let Some(rel) = line[i..].find(']') {
        let br = i + rel;
        let after = br + 1;
        if after >= line.len() {
            break;
        }
        let rest = &line[after..];
        if rest.starts_with('(') {
            if let Some(close) = find_matching_paren(line, after) {
                ranges.push((after, close + 1));
                i = close + 1;
                continue;
            }
        } else if rest.starts_with('[') {
            if let Some(rel_end) = rest[1..].find(']') {
                let end = after + 1 + rel_end + 1;
                ranges.push((after, end));
                i = end;
                continue;
            }
        }
        i = after;
    }
}

fn find_matching_paren(line: &str, open_at: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if open_at >= bytes.len() || bytes[open_at] != b'(' {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_at;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn mask_reference_definition(line: &str, ranges: &mut Vec<(usize, usize)>) {
    let indent = leading_space_count(line);
    if indent > 3 {
        return;
    }
    let rest = &line[indent..];
    if !rest.starts_with('[') {
        return;
    }
    if let Some(end_label) = rest.find("]:") {
        let dest_at = indent + end_label + 2;
        ranges.push((dest_at, line.len()));
    }
}

fn mask_regex(line: &str, re: &Regex, ranges: &mut Vec<(usize, usize)>) {
    for m in re.find_iter(line) {
        ranges.push((m.start(), m.end()));
    }
}

fn html_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]*>").expect("html tag regex"))
}

fn html_entity_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"&(?:[A-Za-z][A-Za-z0-9]*|#[0-9]+|#x[0-9a-fA-F]+);").expect("entity regex"))
}

fn embed_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{[^}]*\}\}").expect("embed regex"))
}

fn url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:https?://|www\.)[^\s<>()]+").expect("url regex")
    })
}

fn email_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").expect("email regex")
    })
}

fn mention_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@[A-Za-z][A-Za-z0-9_-]*").expect("mention regex"))
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?:^|[\s(\[{])(#[A-Za-z][A-Za-z0-9_-]*)").expect("tag regex"))
}

fn overlaps(start: usize, end: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(a, b)| start < b && end > a)
}

fn byte_to_char_index(line: &str) -> Vec<usize> {
    let mut map = vec![0usize; line.len() + 1];
    let mut ci = 0usize;
    for (bi, _) in line.char_indices() {
        map[bi] = ci;
        ci += 1;
    }
    map[line.len()] = ci;
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(text: &str) -> Vec<(usize, usize, String)> {
        let mut ctx = LineContext::new();
        let mut out = Vec::new();
        for line in text.lines() {
            for (a, b, w) in words_to_check(line, &mut ctx) {
                out.push((a, b, w.to_string()));
            }
        }
        out
    }

    fn words(text: &str) -> Vec<String> {
        collect(text).into_iter().map(|(_, _, w)| w).collect()
    }

    #[test]
    fn fenced_backticks_yield_no_tokens() {
        let text = "```\nspeling\n```\n";
        assert!(words(text).is_empty(), "{:?}", words(text));
    }

    #[test]
    fn fenced_tildes_yield_no_tokens() {
        let text = "~~~\nspeling\n~~~\n";
        assert!(words(text).is_empty(), "{:?}", words(text));
    }

    #[test]
    fn nested_fence_lengths() {
        let text = "````\n```\nspeling\n```\n````\nhello\n";
        assert_eq!(words(text), vec!["hello".to_string()]);
    }

    #[test]
    fn indented_code_vs_list_continuation() {
        let code = "    indented speling";
        assert!(words(code).is_empty(), "indented code: {:?}", words(code));

        let list = "- list item with speling";
        assert!(
            words(list).iter().any(|w| w == "speling"),
            "list: {:?}",
            words(list)
        );

        let cont = "- item\n    continuation with speling";
        assert!(
            words(cont).iter().any(|w| w == "speling"),
            "continuation: {:?}",
            words(cont)
        );
    }

    #[test]
    fn inline_code_skipped() {
        assert!(!words("see `speling` here").iter().any(|w| w == "speling"));
        assert!(words("see `speling` here").iter().any(|w| w == "see"));
        assert!(!words("see ``speling`` here").iter().any(|w| w == "speling"));
    }

    #[test]
    fn urls_and_emails_skipped() {
        assert!(words("visit https://exampel.com/speling now")
            .iter()
            .all(|w| w != "speling" && w != "exampel"));
        assert!(words("see www.exampel.com please")
            .iter()
            .all(|w| w != "exampel"));
        assert!(words("mail user@exampel.com please")
            .iter()
            .all(|w| w != "exampel" && w != "user"));
    }

    #[test]
    fn link_checks_text_not_destination() {
        let got = words("[speling](http://x)");
        assert_eq!(got, vec!["speling".to_string()]);
    }

    #[test]
    fn wikilinks_check_alias_only() {
        assert_eq!(
            words("[[target|speling]]"),
            vec!["speling".to_string()]
        );
        assert!(words("[[speling]]").is_empty());
    }

    #[test]
    fn html_tags_and_entities_skipped() {
        let got = words("<span>speling</span> &nbsp; &#39; hello");
        assert!(got.iter().any(|w| w == "speling"));
        assert!(got.iter().any(|w| w == "hello"));
        assert!(!got.iter().any(|w| w == "nbsp" || w == "span"));
    }

    #[test]
    fn frontmatter_skipped() {
        let text = "---\ntitle: speling\n---\nhello\n";
        assert_eq!(words(text), vec!["hello".to_string()]);
    }

    #[test]
    fn math_skipped() {
        let got = words("$speling$ and $$speling$$");
        assert!(
            !got.iter().any(|w| w == "speling"),
            "math must skip speling, got {got:?}"
        );
        assert_eq!(words("$x$ hello"), vec!["hello".to_string()]);
    }

    #[test]
    fn camel_allcaps_digits_skipped() {
        let got = words("Hello XMLHttpRequest USA utf8 foo_bar word");
        assert!(got.contains(&"Hello".to_string()));
        assert!(got.contains(&"word".to_string()));
        assert!(!got.iter().any(|w| {
            matches!(
                w.as_str(),
                "XMLHttpRequest" | "USA" | "utf8" | "foo_bar"
            )
        }));
    }

    #[test]
    fn apostrophe_kept_intact() {
        assert_eq!(words("don't"), vec!["don't".to_string()]);
    }

    #[test]
    fn cjk_and_emoji_only_yield_zero() {
        assert!(words("日本語").is_empty());
        assert!(words("😀🎉").is_empty());
    }

    #[test]
    fn mixed_cjk_char_offset() {
        let got = collect("日本語 speling");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].2, "speling");
        assert_eq!(got[0].0, 4, "char offset must be 4, got {:?}", got[0]);
    }

    #[test]
    fn mentions_and_tags_skipped() {
        assert!(!words("ask @speling about #speling now")
            .iter()
            .any(|w| w == "speling"));
        assert!(words("# Heading speling")
            .iter()
            .any(|w| w == "Heading" || w == "speling"));
    }
}
