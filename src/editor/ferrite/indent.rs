//! Tab / Shift+Tab indent and outdent helpers for the raw Ferrite editor (#177).

/// One indent step as configured (spaces or tab character).
#[must_use]
pub fn indent_string(use_spaces: bool, tab_size: u8) -> String {
    if use_spaces {
        " ".repeat(tab_size as usize)
    } else {
        "\t".to_string()
    }
}

/// Prepends one indent step to `line`, preserving trailing `\r`/`\n`.
#[cfg(test)]
#[must_use]
pub fn indent_line(line: &str, use_spaces: bool, tab_size: u8) -> String {
    let stripped = line.trim_end_matches(['\r', '\n']);
    let suffix = &line[stripped.len()..];
    format!(
        "{}{}{}",
        indent_string(use_spaces, tab_size),
        stripped,
        suffix
    )
}

/// Removes one outdent step from the start of `line`.
///
/// Returns `(new_line, removed_char_count)`. When there is no removable prefix,
/// returns the original line and `0`.
#[must_use]
pub fn outdent_line(line: &str, use_spaces: bool, tab_size: u8) -> (String, usize) {
    let stripped = line.trim_end_matches(['\r', '\n']);
    let suffix = &line[stripped.len()..];

    if stripped.is_empty() {
        return (line.to_string(), 0);
    }

    if stripped.starts_with('\t') {
        let body: String = stripped.chars().skip(1).collect();
        return (format!("{}{}", body, suffix), 1);
    }

    if !use_spaces {
        return (line.to_string(), 0);
    }

    let leading_spaces = stripped.chars().take_while(|&c| c == ' ').count();
    if leading_spaces == 0 {
        return (line.to_string(), 0);
    }

    let remove = leading_spaces.min(tab_size as usize);
    let body: String = stripped.chars().skip(remove).collect();
    (format!("{}{}", body, suffix), remove)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_string_spaces() {
        assert_eq!(indent_string(true, 4), "    ");
    }

    #[test]
    fn test_indent_string_tabs() {
        assert_eq!(indent_string(false, 4), "\t");
    }

    #[test]
    fn test_indent_line_preserves_newline() {
        assert_eq!(indent_line("hello\n", true, 2), "  hello\n");
    }

    #[test]
    fn test_outdent_line_at_column_zero_noop() {
        let (line, removed) = outdent_line("hello", true, 4);
        assert_eq!(removed, 0);
        assert_eq!(line, "hello");
    }

    #[test]
    fn test_outdent_line_spaces() {
        let (line, removed) = outdent_line("    hello", true, 4);
        assert_eq!(removed, 4);
        assert_eq!(line, "hello");
    }

    #[test]
    fn test_outdent_line_spaces_partial() {
        let (line, removed) = outdent_line("  hello", true, 4);
        assert_eq!(removed, 2);
        assert_eq!(line, "hello");
    }

    #[test]
    fn test_outdent_line_tab() {
        let (line, removed) = outdent_line("\tworld", false, 4);
        assert_eq!(removed, 1);
        assert_eq!(line, "world");
    }

    #[test]
    fn test_outdent_line_leading_tab_with_spaces_setting_ind2() {
        let (line, removed) = outdent_line("\tfoo", true, 4);
        assert_eq!(removed, 1);
        assert_eq!(line, "foo");

        let (line, removed) = outdent_line("\t\tfoo", true, 4);
        assert_eq!(removed, 1);
        assert_eq!(line, "\tfoo");
    }

    #[test]
    fn test_multi_line_block_outdent_mixed_tabs_and_spaces() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("    four\n        eight\n\t\ttab");
        editor.set_tab_settings(true, 4);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(2, 8)));
        editor.handle_tab_key(true);
        assert_eq!(editor.buffer().to_string(), "four\n    eight\n\ttab");
    }

    #[test]
    fn test_block_outdent_shifts_caret_on_tab_indented_line() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("\tindented");
        editor.set_tab_settings(true, 4);
        editor.set_selection(Selection::new(Cursor::new(0, 5), Cursor::new(0, 5)));
        editor.handle_tab_key(true);
        assert_eq!(editor.buffer().to_string(), "indented");
        assert_eq!(editor.primary_selection().head, Cursor::new(0, 4));
    }

    #[test]
    fn test_multi_line_block_indent() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("alpha\nbeta\ngamma");
        editor.set_tab_settings(true, 2);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(2, 5)));
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "  alpha\n  beta\n  gamma");
    }

    #[test]
    fn test_multi_line_block_outdent() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("  one\n  two\n  three");
        editor.set_tab_settings(true, 2);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(2, 7)));
        editor.handle_tab_key(true);
        assert_eq!(editor.buffer().to_string(), "one\ntwo\nthree");
    }

    #[test]
    fn test_empty_caret_inserts_spaces() {
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("hello");
        editor.set_tab_settings(true, 4);
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "    hello");
    }

    #[test]
    fn test_empty_caret_inserts_tab_character() {
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("hello");
        editor.set_tab_settings(false, 4);
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "\thello");
    }

    #[test]
    fn test_shift_tab_empty_caret_at_column_zero_noop() {
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("hello\nworld");
        editor.set_tab_settings(true, 4);
        editor.handle_tab_key(true);
        assert_eq!(editor.buffer().to_string(), "hello\nworld");
    }

    #[test]
    fn test_multi_cursor_block_indent() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("aa\nbb\ncc\ndd");
        editor.set_tab_settings(true, 2);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(1, 2)));
        editor.add_selection(Selection::new(Cursor::new(2, 0), Cursor::new(3, 2)));
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "  aa\n  bb\n  cc\n  dd");
    }

    #[test]
    fn test_mixed_multiline_and_single_line_range_tab_no_corruption() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        // Multi-line selection (block indent, lines 0-1) mixed with a
        // single-line range on line 3. Every non-empty range indents its
        // line(s); the selected text is never replaced (QA ISS-67).
        let mut editor = FerriteEditor::from_string("aa\nbb\ncc\ndddd");
        editor.set_tab_settings(true, 2);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(1, 2)));
        editor.add_selection(Selection::new(Cursor::new(3, 1), Cursor::new(3, 3)));
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "  aa\n  bb\ncc\n  dddd");
    }

    /// QA ISS-67 (T1-20): a whole-line single-line selection plus a Ctrl+click
    /// caret mid-line. Tab must indent the selected line (not replace its text
    /// with spaces) and insert one step at the caret; one undo restores all.
    #[test]
    fn test_single_line_selection_tab_indents_line_instead_of_replacing() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("alpha line one\nbeta\n中文行首");
        editor.set_tab_settings(true, 4);
        // `alpha line one` fully selected (anchor 0, head 14) + caret after 中文行.
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(0, 14)));
        editor.add_selection(Selection::new(Cursor::new(2, 3), Cursor::new(2, 3)));
        editor.handle_tab_key(false);
        assert_eq!(
            editor.buffer().to_string(),
            "    alpha line one\nbeta\n中文行    首"
        );
        // Selection still covers the same text, shifted by the indent.
        let sel = editor.primary_selection();
        let (start, end) = sel.ordered();
        assert_eq!((start, end), (Cursor::new(0, 4), Cursor::new(0, 18)));

        // Shift+Tab on the same selection reverses the line indent.
        editor.handle_tab_key(true);
        assert!(editor.buffer().to_string().starts_with("alpha line one\n"));
    }

    /// A selection inside a single line (a word) still indents the whole line.
    #[test]
    fn test_partial_single_line_selection_tab_indents_line() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        let mut editor = FerriteEditor::from_string("hello world");
        editor.set_tab_settings(false, 4);
        editor.set_selection(Selection::new(Cursor::new(0, 6), Cursor::new(0, 11)));
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "\thello world");
    }

    #[test]
    fn test_caret_on_block_indented_line_inserts_at_shifted_column() {
        use super::super::cursor::{Cursor, Selection};
        use super::super::editor::FerriteEditor;

        // Collapsed caret between 'x' and 'y' on a line that the block pass
        // also indents: the caret column must shift with the block indent so
        // the caret insert lands between 'x' and 'y', not inside the new
        // leading whitespace.
        let mut editor = FerriteEditor::from_string("aa\nxy");
        editor.set_tab_settings(true, 2);
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(1, 0)));
        editor.add_selection(Selection::new(Cursor::new(1, 1), Cursor::new(1, 1)));
        editor.handle_tab_key(false);
        assert_eq!(editor.buffer().to_string(), "  aa\n  x  y");
    }
}
