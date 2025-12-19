//! HTML Export Generation
//!
//! This module generates complete HTML documents from markdown content,
//! with inlined theme CSS for standalone viewing.

// Allow dead code - this module contains complete export API with file export
// and error variants for future export enhancements
#![allow(dead_code)]

use crate::theme::ThemeColors;
use comrak::{markdown_to_html, Options};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Error Types
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during HTML export.
#[derive(Debug)]
pub enum HtmlExportError {
    /// Failed to read source file
    IoError(std::io::Error),
    /// Failed to convert markdown
    ConversionError(String),
}

impl std::fmt::Display for HtmlExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HtmlExportError::IoError(e) => write!(f, "IO error: {}", e),
            HtmlExportError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
        }
    }
}

impl std::error::Error for HtmlExportError {}

impl From<std::io::Error> for HtmlExportError {
    fn from(err: std::io::Error) -> Self {
        HtmlExportError::IoError(err)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML Generation
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a complete HTML document from markdown content.
///
/// # Arguments
///
/// * `markdown` - The markdown source text
/// * `title` - Optional document title
/// * `theme_colors` - Theme colors for styling
/// * `include_syntax_css` - Whether to include syntax highlighting CSS
///
/// # Returns
///
/// A complete HTML document as a string.
pub fn generate_html_document(
    markdown: &str,
    title: Option<&str>,
    theme_colors: &ThemeColors,
    include_syntax_css: bool,
) -> Result<String, HtmlExportError> {
    // Convert markdown to HTML body
    let html_body = markdown_to_html_body(markdown)?;

    // Generate CSS from theme
    let theme_css = generate_theme_css(theme_colors);

    // Generate syntax highlighting CSS if requested
    let syntax_css = if include_syntax_css {
        generate_syntax_css(theme_colors)
    } else {
        String::new()
    };

    // Build the complete HTML document
    let doc_title = title.unwrap_or("Exported Document");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Ferrite">
    <title>{title}</title>
    <style>
{base_css}

{theme_css}

{syntax_css}
    </style>
</head>
<body>
    <article class="markdown-body">
{body}
    </article>
</body>
</html>"#,
        title = html_escape(doc_title),
        base_css = BASE_CSS,
        theme_css = theme_css,
        syntax_css = syntax_css,
        body = html_body,
    );

    Ok(html)
}

/// Generate HTML fragment (no doctype, head, etc.) for clipboard.
///
/// # Arguments
///
/// * `markdown` - The markdown source text
///
/// # Returns
///
/// An HTML fragment suitable for pasting.
pub fn generate_html_fragment(markdown: &str) -> Result<String, HtmlExportError> {
    markdown_to_html_body(markdown)
}

/// Convert markdown to HTML body content.
fn markdown_to_html_body(markdown: &str) -> Result<String, HtmlExportError> {
    let mut options = Options::default();

    // Enable common extensions
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.header_ids = Some(String::new());

    // Render options
    options.render.unsafe_ = true; // Allow raw HTML

    let html = markdown_to_html(markdown, &options);
    Ok(html)
}

/// Export markdown file to HTML file.
///
/// # Arguments
///
/// * `source_path` - Path to the markdown file
/// * `output_path` - Path for the output HTML file
/// * `theme_colors` - Theme colors for styling
///
/// # Returns
///
/// Ok(()) on success, or an error.
pub fn export_to_html_file(
    source_path: &Path,
    output_path: &Path,
    theme_colors: &ThemeColors,
) -> Result<(), HtmlExportError> {
    let markdown = std::fs::read_to_string(source_path)?;

    let title = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Document");

    let html = generate_html_document(&markdown, Some(title), theme_colors, true)?;

    std::fs::write(output_path, html)?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// CSS Generation
// ─────────────────────────────────────────────────────────────────────────────

/// Base CSS for markdown rendering (layout, typography).
const BASE_CSS: &str = r#"
/* Reset and base styles */
*, *::before, *::after {
    box-sizing: border-box;
}

body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.6;
}

/* Article container */
.markdown-body {
    max-width: 900px;
    margin: 0 auto;
    padding: 32px 24px;
}

/* Headings */
.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
    margin-top: 24px;
    margin-bottom: 16px;
    font-weight: 600;
    line-height: 1.25;
}

.markdown-body h1 { font-size: 2em; border-bottom: 1px solid; padding-bottom: 0.3em; }
.markdown-body h2 { font-size: 1.5em; border-bottom: 1px solid; padding-bottom: 0.3em; }
.markdown-body h3 { font-size: 1.25em; }
.markdown-body h4 { font-size: 1em; }
.markdown-body h5 { font-size: 0.875em; }
.markdown-body h6 { font-size: 0.85em; }

/* Paragraphs */
.markdown-body p {
    margin-top: 0;
    margin-bottom: 16px;
}

/* Links */
.markdown-body a {
    text-decoration: none;
}

.markdown-body a:hover {
    text-decoration: underline;
}

/* Lists */
.markdown-body ul,
.markdown-body ol {
    margin-top: 0;
    margin-bottom: 16px;
    padding-left: 2em;
}

.markdown-body li {
    margin-bottom: 4px;
}

.markdown-body li + li {
    margin-top: 4px;
}

/* Task lists */
.markdown-body ul.contains-task-list {
    list-style-type: none;
    padding-left: 0;
}

.markdown-body .task-list-item {
    padding-left: 1.5em;
    position: relative;
}

.markdown-body .task-list-item input[type="checkbox"] {
    position: absolute;
    left: 0;
    top: 0.3em;
}

/* Blockquotes */
.markdown-body blockquote {
    margin: 0 0 16px 0;
    padding: 0 1em;
    border-left: 4px solid;
}

.markdown-body blockquote > :first-child {
    margin-top: 0;
}

.markdown-body blockquote > :last-child {
    margin-bottom: 0;
}

/* Code */
.markdown-body code {
    font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', monospace;
    font-size: 0.9em;
    padding: 0.2em 0.4em;
    border-radius: 4px;
}

.markdown-body pre {
    margin-top: 0;
    margin-bottom: 16px;
    padding: 16px;
    overflow: auto;
    border-radius: 6px;
    line-height: 1.45;
}

.markdown-body pre code {
    padding: 0;
    background: transparent;
    border-radius: 0;
    font-size: 0.875em;
}

/* Tables */
.markdown-body table {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: 16px;
}

.markdown-body th,
.markdown-body td {
    padding: 8px 12px;
    border: 1px solid;
}

.markdown-body th {
    font-weight: 600;
    text-align: left;
}

.markdown-body tr:nth-child(even) td {
    background-color: rgba(128, 128, 128, 0.05);
}

/* Horizontal rule */
.markdown-body hr {
    height: 2px;
    margin: 24px 0;
    border: none;
}

/* Images */
.markdown-body img {
    max-width: 100%;
    height: auto;
    border-radius: 4px;
}

/* Strong and emphasis */
.markdown-body strong {
    font-weight: 600;
}

.markdown-body em {
    font-style: italic;
}

/* Strikethrough */
.markdown-body del {
    text-decoration: line-through;
}
"#;

/// Generate theme-specific CSS from ThemeColors.
fn generate_theme_css(colors: &ThemeColors) -> String {
    let is_dark = colors.is_dark();

    format!(
        r#"
/* Theme colors */
:root {{
    color-scheme: {color_scheme};
}}

body {{
    background-color: {bg};
    color: {text};
}}

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {{
    color: {heading};
}}

.markdown-body h1,
.markdown-body h2 {{
    border-bottom-color: {border};
}}

.markdown-body a {{
    color: {link};
}}

.markdown-body blockquote {{
    color: {blockquote_text};
    border-left-color: {blockquote_border};
}}

.markdown-body code {{
    background-color: {code_bg};
    color: {code_text};
}}

.markdown-body pre {{
    background-color: {code_block_bg};
    border: 1px solid {code_block_border};
}}

.markdown-body th,
.markdown-body td {{
    border-color: {table_border};
}}

.markdown-body th {{
    background-color: {table_header_bg};
}}

.markdown-body hr {{
    background-color: {hr};
}}
"#,
        color_scheme = if is_dark { "dark" } else { "light" },
        bg = color32_to_css(colors.base.background),
        text = color32_to_css(colors.text.primary),
        heading = color32_to_css(colors.editor.heading),
        border = color32_to_css(colors.base.border),
        link = color32_to_css(colors.text.link),
        blockquote_text = color32_to_css(colors.editor.blockquote_text),
        blockquote_border = color32_to_css(colors.editor.blockquote_border),
        code_bg = color32_to_css(colors.base.background_tertiary),
        code_text = color32_to_css(colors.text.code),
        code_block_bg = color32_to_css(colors.editor.code_block_bg),
        code_block_border = color32_to_css(colors.editor.code_block_border),
        table_border = color32_to_css(colors.editor.table_border),
        table_header_bg = color32_to_css(colors.editor.table_header_bg),
        hr = color32_to_css(colors.editor.horizontal_rule),
    )
}

/// Generate syntax highlighting CSS.
fn generate_syntax_css(colors: &ThemeColors) -> String {
    format!(
        r#"
/* Syntax highlighting */
.markdown-body pre code .keyword {{ color: {keyword}; }}
.markdown-body pre code .string {{ color: {string}; }}
.markdown-body pre code .number {{ color: {number}; }}
.markdown-body pre code .comment {{ color: {comment}; font-style: italic; }}
.markdown-body pre code .function {{ color: {function}; }}
.markdown-body pre code .type {{ color: {type_name}; }}
.markdown-body pre code .variable {{ color: {variable}; }}
.markdown-body pre code .operator {{ color: {operator}; }}
.markdown-body pre code .punctuation {{ color: {punctuation}; }}
"#,
        keyword = color32_to_css(colors.syntax.keyword),
        string = color32_to_css(colors.syntax.string),
        number = color32_to_css(colors.syntax.number),
        comment = color32_to_css(colors.syntax.comment),
        function = color32_to_css(colors.syntax.function),
        type_name = color32_to_css(colors.syntax.type_name),
        variable = color32_to_css(colors.syntax.variable),
        operator = color32_to_css(colors.syntax.operator),
        punctuation = color32_to_css(colors.syntax.punctuation),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert Color32 to CSS color string.
fn color32_to_css(color: eframe::egui::Color32) -> String {
    format!("rgb({}, {}, {})", color.r(), color.g(), color.b())
}

/// HTML-escape a string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html_body() {
        let markdown = "# Hello\n\nWorld";
        let html = markdown_to_html_body(markdown).unwrap();

        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<p>"));
        assert!(html.contains("World"));
    }

    #[test]
    fn test_generate_html_document() {
        let markdown = "# Test\n\nParagraph text.";
        let colors = ThemeColors::light();
        let html = generate_html_document(markdown, Some("Test Doc"), &colors, true).unwrap();

        // Check document structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Test Doc</title>"));
        assert!(html.contains("<article class=\"markdown-body\">"));
        assert!(html.contains("</article>"));

        // Check content
        assert!(html.contains("<h1"));
        assert!(html.contains("Test"));
    }

    #[test]
    fn test_generate_html_fragment() {
        let markdown = "**Bold** and *italic*";
        let html = generate_html_fragment(markdown).unwrap();

        // Should be a fragment, not a full document
        assert!(!html.contains("<!DOCTYPE"));
        assert!(html.contains("<strong>"));
        assert!(html.contains("<em>"));
    }

    #[test]
    fn test_color32_to_css() {
        let color = eframe::egui::Color32::from_rgb(255, 128, 64);
        let css = color32_to_css(color);
        assert_eq!(css, "rgb(255, 128, 64)");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("Hello"), "Hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_theme_css_light() {
        let colors = ThemeColors::light();
        let css = generate_theme_css(&colors);

        assert!(css.contains("color-scheme: light"));
        assert!(css.contains("background-color:"));
    }

    #[test]
    fn test_theme_css_dark() {
        let colors = ThemeColors::dark();
        let css = generate_theme_css(&colors);

        assert!(css.contains("color-scheme: dark"));
    }
}
