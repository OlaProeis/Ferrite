//! Shared panic-free slicing helpers for Mermaid parsers.
//!
//! Half-typed or swapped brackets (`A}x{`, `subgraph a] [b`) must not panic when
//! `find` / `rfind` results are used as slice bounds.

/// Slice `text` into `(before, inner)` around the first `open` and last `close`.
///
/// Returns `None` when either delimiter is missing or when
/// `start + open.len() > end` (swapped / overlapping delimiters). Callers should
/// treat `None` the same as "delimiters absent" and fall through to the next
/// shape or plain-id path.
pub(crate) fn slice_between<'a>(
    text: &'a str,
    open: &str,
    close: &str,
) -> Option<(&'a str, &'a str)> {
    let start = text.find(open)?;
    let end = text.rfind(close)?;
    let inner_start = start + open.len();
    if inner_start <= end {
        Some((&text[..start], &text[inner_start..end]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::slice_between;

    #[test]
    fn ordered_delimiters_yield_before_and_inner() {
        assert_eq!(slice_between("A[Title]", "[", "]"), Some(("A", "Title")));
        assert_eq!(
            slice_between("id [title]", "[", "]"),
            Some(("id ", "title"))
        );
        assert_eq!(slice_between("A{{hex}}", "{{", "}}"), Some(("A", "hex")));
        assert_eq!(slice_between("[]", "[", "]"), Some(("", "")));
    }

    #[test]
    fn swapped_delimiters_return_none() {
        assert_eq!(slice_between("A}x{", "{", "}"), None);
        assert_eq!(slice_between("a] [b", "[", "]"), None);
        assert_eq!(slice_between("A}}x{{", "{{", "}}"), None);
        assert_eq!(slice_between(">>interface<<", "<<", ">>"), None);
    }

    #[test]
    fn missing_delimiter_returns_none() {
        assert_eq!(slice_between("A[Title", "[", "]"), None);
        assert_eq!(slice_between("ATitle]", "[", "]"), None);
        assert_eq!(slice_between("plain", "[", "]"), None);
    }
}
