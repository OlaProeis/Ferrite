//! HTML Export Generation
//!
//! Themed standalone HTML via comrak, custom syntax highlighting (syntect / two-face),
//! optional Mermaid flowchart SVG, and image/link post-processing.

// Allow dead code for export API surface
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use comrak::{markdown_to_html_with_plugins, Options, Plugins};
use regex::Regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::Color as SyntectColor;
use syntect::html::{append_highlighted_html_for_styled_line, IncludeBackground};
use syntect::util::LinesWithEndings;

use crate::config::ParagraphIndent;
use crate::export::flowchart_svg::try_flowchart_svg_snippet;
use crate::export::html_options::HtmlExportOptions;
use crate::markdown::mermaid::FlowchartColors;
use crate::markdown::syntax::get_highlighter;
use crate::markdown::toc::{extract_toc_headings, TocOptions};
use crate::markdown::{detect_mermaid_diagram_type, MermaidDiagramType};
use crate::theme::ThemeColors;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum HtmlExportError {
    IoError(std::io::Error),
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
// Theme resolution for export
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum HtmlThemeResolution {
    Single(ThemeColors),
    Auto {
        light: ThemeColors,
        dark: ThemeColors,
    },
}

/// Resolve editor / accent into export theme colors.
pub fn resolve_html_theme_for_export(
    choice: crate::export::HtmlExportThemeChoice,
    theme_mgr: &crate::theme::ThemeManager,
    accent_rgb: [u8; 3],
    ctx: &eframe::egui::Context,
) -> HtmlThemeResolution {
    let accent = eframe::egui::Color32::from_rgb(accent_rgb[0], accent_rgb[1], accent_rgb[2]);
    match choice {
        crate::export::HtmlExportThemeChoice::FollowEditor => {
            HtmlThemeResolution::Single(theme_mgr.colors(ctx))
        }
        crate::export::HtmlExportThemeChoice::Light => {
            let mut c = ThemeColors::light();
            c.apply_user_accent(accent);
            HtmlThemeResolution::Single(c)
        }
        crate::export::HtmlExportThemeChoice::Dark => {
            let mut c = ThemeColors::dark();
            c.apply_user_accent(accent);
            HtmlThemeResolution::Single(c)
        }
        crate::export::HtmlExportThemeChoice::Auto => {
            let mut light = ThemeColors::light();
            light.apply_user_accent(accent);
            let mut dark = ThemeColors::dark();
            dark.apply_user_accent(accent);
            HtmlThemeResolution::Auto { light, dark }
        }
    }
}

pub fn syntax_dark_mode_for_export(
    choice: crate::export::HtmlExportThemeChoice,
    theme_mgr: &crate::theme::ThemeManager,
    ctx: &eframe::egui::Context,
) -> bool {
    match choice {
        crate::export::HtmlExportThemeChoice::FollowEditor => theme_mgr.is_dark(ctx),
        crate::export::HtmlExportThemeChoice::Light => false,
        crate::export::HtmlExportThemeChoice::Dark => true,
        crate::export::HtmlExportThemeChoice::Auto => ctx.global_style().visuals.dark_mode,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Comrak syntax adapter (inline syntect HTML)
// ─────────────────────────────────────────────────────────────────────────────

struct FerriteHtmlHighlighter<'a> {
    theme_name: &'a str,
    dark_mode: bool,
}

impl SyntaxHighlighterAdapter for FerriteHtmlHighlighter<'_> {
    fn write_highlighted(
        &self,
        output: &mut dyn Write,
        lang: Option<&str>,
        code: &str,
    ) -> io::Result<()> {
        let hi = get_highlighter();
        let theme = hi.get_theme_by_name_or_mode(self.theme_name, self.dark_mode);
        let lang = lang.unwrap_or("text");
        let syntax = hi
            .find_syntax_for_language(lang)
            .unwrap_or_else(|| hi.syntax_set().find_syntax_plain_text());
        let mut hl = HighlightLines::new(syntax, theme);
        let bg = theme.settings.background.unwrap_or_else(|| SyntectColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        });
        for line in LinesWithEndings::from(code) {
            match hl.highlight_line(line, hi.syntax_set()) {
                Ok(regions) => {
                    let mut line_html = String::new();
                    append_highlighted_html_for_styled_line(
                        &regions[..],
                        IncludeBackground::IfDifferent(bg),
                        &mut line_html,
                    )
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    output.write_all(line_html.as_bytes())?;
                }
                Err(_) => {
                    for b in line.as_bytes() {
                        output.write_all(&[*b])?;
                    }
                }
            }
        }
        Ok(())
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<String, String>,
    ) -> io::Result<()> {
        let hi = get_highlighter();
        let theme = hi.get_theme_by_name_or_mode(self.theme_name, self.dark_mode);
        let colour = theme.settings.background.unwrap_or_else(|| SyntectColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        });
        let bg_style = format!(
            "background-color:#{:02x}{:02x}{:02x};",
            colour.r, colour.g, colour.b
        );
        let mut pairs: Vec<(String, String)> = attributes.into_iter().collect();
        let mut merged = bg_style;
        if let Some(pos) = pairs.iter().position(|(k, _)| k == "style") {
            let (_, existing) = pairs.swap_remove(pos);
            merged = format!("{existing}{merged}");
        }
        pairs.push(("style".to_string(), merged));
        comrak::html::write_opening_tag(
            output,
            "pre",
            pairs.iter().map(|(a, b)| (a.as_str(), b.as_str())),
        )
    }

    fn write_code_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<String, String>,
    ) -> io::Result<()> {
        comrak::html::write_opening_tag(
            output,
            "code",
            attributes.iter().map(|(a, b)| (a.as_str(), b.as_str())),
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mermaid fences
// ─────────────────────────────────────────────────────────────────────────────

static MERMAID_FENCE: OnceLock<Regex> = OnceLock::new();

fn extract_mermaid_fences(markdown: &str) -> (String, Vec<String>) {
    let re = MERMAID_FENCE.get_or_init(|| {
        Regex::new(r"(?s)```mermaid[ \t]*\r?\n(.*?)```").expect("mermaid fence regex")
    });
    let mut out = String::new();
    let mut blocks: Vec<String> = Vec::new();
    let mut last = 0usize;
    for cap in re.captures_iter(markdown) {
        let whole = cap.get(0).unwrap();
        out.push_str(&markdown[last..whole.start()]);
        let idx = blocks.len();
        blocks.push(cap.get(1).unwrap().as_str().to_string());
        out.push_str(&format!("\n\n<!--FERRITE-MERMAID-BLOCK:{idx}-->\n\n"));
        last = whole.end();
    }
    out.push_str(&markdown[last..]);
    (out, blocks)
}

fn inject_mermaid_exports(
    html: &str,
    blocks: &[String],
    diagram_w: f32,
    flowchart_colors: &FlowchartColors,
    theme_name: &str,
    dark: bool,
) -> String {
    let mut out = html.to_string();
    for (i, src) in blocks.iter().enumerate() {
        let needle = format!("<!--FERRITE-MERMAID-BLOCK:{i}-->");
        let body = if detect_mermaid_diagram_type(src) == MermaidDiagramType::Flowchart {
            try_flowchart_svg_snippet(src, flowchart_colors, diagram_w)
                .map(|svg| {
                    format!(r#"<figure class="ferrite-mermaid ferrite-mermaid-svg">{svg}</figure>"#)
                })
                .unwrap_or_else(|| mermaid_fallback_figure(src, theme_name, dark))
        } else {
            mermaid_fallback_figure(src, theme_name, dark)
        };
        out = out.replace(&needle, &body);
    }
    out
}

fn mermaid_fallback_figure(source: &str, theme_name: &str, dark: bool) -> String {
    let inner = highlighted_code_to_html_spans(source, "mermaid", theme_name, dark);
    format!(
        r#"<figure class="ferrite-mermaid ferrite-mermaid-fallback"><figcaption class="mermaid-caption">Mermaid diagram</figcaption><pre class="mermaid-source"><code>{inner}</code></pre></figure>"#
    )
}

fn highlighted_code_to_html_spans(code: &str, lang: &str, theme_name: &str, dark: bool) -> String {
    use crate::markdown::syntax::highlight_code_with_theme;
    let lines = highlight_code_with_theme(code, lang, theme_name, dark);
    let mut s = String::new();
    for line in &lines {
        for seg in &line.segments {
            let color = color32_to_css(seg.foreground);
            let mut style = format!("color:{color}");
            if seg.bold {
                style.push_str(";font-weight:600");
            }
            if seg.italic {
                style.push_str(";font-style:italic");
            }
            if seg.underline {
                style.push_str(";text-decoration:underline");
            }
            s.push_str(&format!(
                r#"<span style="{style}">{}</span>"#,
                html_escape(&seg.text)
            ));
        }
    }
    s
}

fn flowchart_colors_for_export(
    resolution: &HtmlThemeResolution,
    syntax_dark: bool,
) -> FlowchartColors {
    match resolution {
        HtmlThemeResolution::Single(c) => {
            if c.is_dark() {
                FlowchartColors::dark()
            } else {
                FlowchartColors::light()
            }
        }
        HtmlThemeResolution::Auto { .. } => {
            if syntax_dark {
                FlowchartColors::dark()
            } else {
                FlowchartColors::light()
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Images & links
// ─────────────────────────────────────────────────────────────────────────────

fn guess_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ref e) if e == "png" => "image/png",
        Some(ref e) if e == "jpg" || e == "jpeg" => "image/jpeg",
        Some(ref e) if e == "gif" => "image/gif",
        Some(ref e) if e == "webp" => "image/webp",
        Some(ref e) if e == "bmp" => "image/bmp",
        Some(ref e) if e == "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn base64_std_encode(data: &[u8]) -> String {
    const SET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = chunk.len();
        let triple = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        out.push(SET[((triple >> 18) & 63) as usize] as char);
        out.push(SET[((triple >> 12) & 63) as usize] as char);
        if n > 1 {
            out.push(SET[((triple >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if n > 2 {
            out.push(SET[(triple & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

use crate::export::options::ImageHandling;

fn postprocess_images(
    html: &str,
    base_dir: Option<&Path>,
    embed: bool,
    image_handling: ImageHandling,
) -> Result<String, HtmlExportError> {
    let re = Regex::new(r#"(<img\b[^>]*\bsrc=")([^"]+)(")"#)
        .map_err(|e| HtmlExportError::ConversionError(e.to_string()))?;
    let mut buf = String::with_capacity(html.len() + 256);
    let mut last = 0usize;
    for cap in re.captures_iter(html) {
        let whole = cap.get(0).unwrap();
        buf.push_str(&html[last..whole.start()]);
        let pfx = cap.get(1).unwrap().as_str();
        let src = cap.get(2).unwrap().as_str();
        let suf = cap.get(3).unwrap().as_str();
        let new_src = if embed {
            if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:")
            {
                src.to_string()
            } else {
                let path = match base_dir {
                    Some(b) => b.join(src),
                    None => PathBuf::from(src),
                };
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let mime = guess_mime(&path);
                        let b64 = base64_std_encode(&bytes);
                        format!("data:{mime};base64,{b64}")
                    }
                    Err(_) => src.to_string(),
                }
            }
        } else if src.starts_with("http://")
            || src.starts_with("https://")
            || src.starts_with("data:")
            || src.starts_with('#')
        {
            src.to_string()
        } else {
            let path = match base_dir {
                Some(b) => b.join(src),
                None => PathBuf::from(src),
            };
            match image_handling {
                ImageHandling::AbsolutePaths => path
                    .canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                ImageHandling::RelativePaths | ImageHandling::EmbedBase64 => {
                    path.to_string_lossy().replace('\\', "/")
                }
            }
        };
        buf.push_str(pfx);
        buf.push_str(&new_src);
        buf.push_str(suf);
        last = whole.end();
    }
    buf.push_str(&html[last..]);
    Ok(buf)
}

fn postprocess_links(html: &str, base_dir: Option<&Path>) -> Result<String, HtmlExportError> {
    let re = Regex::new(r#"(<a\b[^>]*\bhref=")([^"]+)(")"#)
        .map_err(|e| HtmlExportError::ConversionError(e.to_string()))?;
    let mut buf = String::with_capacity(html.len());
    let mut last = 0usize;
    for cap in re.captures_iter(html) {
        let whole = cap.get(0).unwrap();
        buf.push_str(&html[last..whole.start()]);
        let pfx = cap.get(1).unwrap().as_str();
        let href = cap.get(2).unwrap().as_str();
        let suf = cap.get(3).unwrap().as_str();
        let new_href =
            if href.contains("://") || href.starts_with('#') || href.starts_with("mailto:") {
                href.to_string()
            } else if let Some(b) = base_dir {
                b.join(href).to_string_lossy().replace('\\', "/")
            } else {
                href.to_string()
            };
        buf.push_str(pfx);
        buf.push_str(&new_href);
        buf.push_str(suf);
        last = whole.end();
    }
    buf.push_str(&html[last..]);
    Ok(buf)
}

fn build_toc_html(markdown: &str) -> String {
    let opts = TocOptions::default();
    let headings = extract_toc_headings(markdown, &opts);
    if headings.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        r#"<nav class="markdown-toc" aria-label="Table of contents"><h2 class="toc-title">Contents</h2><ul>"#,
    );
    for h in headings {
        let esc_anchor = html_escape(&h.anchor);
        let esc_text = html_escape(&h.text);
        s.push_str(&format!(
            "<li class=\"toc-level-{0}\"><a href=\"#{1}\">{2}</a></li>",
            h.level, esc_anchor, esc_text,
        ));
    }
    s.push_str("</ul></nav>");
    s
}

fn extract_frontmatter_comment_raw(md: &str) -> Option<String> {
    let md = md.strip_prefix('\u{feff}').unwrap_or(md);
    if !md.starts_with("---") {
        return None;
    }
    let rest = md.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some(rest[..end].to_string())
}

fn build_header_comment(opts: &HtmlExportOptions, source_path: Option<&Path>) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = source_path
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    format!(
        "<!-- ferrite-html-export: theme={:?} self_contained={} path={} ts={ts} -->\n",
        opts.theme, opts.self_contained, path
    )
}

fn theme_rules_inner(colors: &ThemeColors) -> String {
    format!(
        r#"
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
.markdown-body table tbody tr:nth-child(even) td {{
    background-color: {table_alt};
}}
.markdown-toc {{
    background-color: {panel};
    border: 1px solid {border_subtle};
}}
.markdown-toc .toc-title {{
    color: {heading};
}}
"#,
        bg = color32_to_css(colors.base.background),
        text = color32_to_css(colors.text.primary),
        heading = color32_to_css(colors.editor.heading),
        border = color32_to_css(colors.base.border),
        border_subtle = color32_to_css(colors.base.border_subtle),
        panel = color32_to_css(colors.base.background_secondary),
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
        table_alt = if colors.is_dark() {
            "rgba(255,255,255,0.04)".to_string()
        } else {
            "rgba(0,0,0,0.03)".to_string()
        },
    )
}

fn generate_theme_styles(resolution: &HtmlThemeResolution) -> String {
    match resolution {
        HtmlThemeResolution::Single(colors) => {
            let scheme = if colors.is_dark() { "dark" } else { "light" };
            format!(
                ":root {{ color-scheme: {scheme}; }}\n{}",
                theme_rules_inner(colors)
            )
        }
        HtmlThemeResolution::Auto { light, dark } => {
            format!(
                r#":root {{ color-scheme: light dark; }}
@media (prefers-color-scheme: light) {{
  :root {{ color-scheme: light; }}
  {light_rules}
}}
@media (prefers-color-scheme: dark) {{
  :root {{ color-scheme: dark; }}
  {dark_rules}
}}"#,
                light_rules = theme_rules_inner(light),
                dark_rules = theme_rules_inner(dark),
            )
        }
    }
}

/// Optional Ferrite-facing syntax CSS (spans from ThemeColors) when not using inline-only blocks.
fn syntax_css_from_palette(colors: &ThemeColors) -> String {
    format!(
        r#"
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

const BASE_CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; }
body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.6;
}
.markdown-body {
    max-width: 900px;
    margin: 0 auto;
    padding: 32px 24px;
}
.markdown-body h1, .markdown-body h2, .markdown-body h3,
.markdown-body h4, .markdown-body h5, .markdown-body h6 {
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
.markdown-body p { margin-top: 0; margin-bottom: 16px; }
.markdown-body a { text-decoration: none; }
.markdown-body a:hover { text-decoration: underline; }
.markdown-body ul, .markdown-body ol {
    margin-top: 0;
    margin-bottom: 16px;
    padding-left: 2em;
}
.markdown-body li { margin-bottom: 4px; }
.markdown-body li + li { margin-top: 4px; }
.markdown-body ul.contains-task-list { list-style-type: none; padding-left: 0; }
.markdown-body .task-list-item { padding-left: 1.5em; position: relative; }
.markdown-body .task-list-item input[type="checkbox"] {
    position: absolute; left: 0; top: 0.3em;
}
.markdown-body blockquote {
    margin: 0 0 16px 0; padding: 0 1em; border-left: 4px solid;
}
.markdown-body blockquote > :first-child { margin-top: 0; }
.markdown-body blockquote > :last-child { margin-bottom: 0; }
.markdown-body code {
    font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', monospace;
    font-size: 0.9em;
    padding: 0.2em 0.4em;
    border-radius: 4px;
}
.markdown-body pre {
    margin-top: 0; margin-bottom: 16px;
    padding: 16px; overflow: auto; border-radius: 6px; line-height: 1.45;
}
.markdown-body pre code {
    padding: 0; background: transparent; border-radius: 0; font-size: 0.875em;
}
.markdown-body table { border-collapse: collapse; width: 100%; margin-bottom: 16px; }
.markdown-body th, .markdown-body td { padding: 8px 12px; border: 1px solid; }
.markdown-body th { font-weight: 600; text-align: left; }
.markdown-body hr { height: 2px; margin: 24px 0; border: none; }
.markdown-body img { max-width: 100%; height: auto; border-radius: 4px; }
.markdown-body strong { font-weight: 600; }
.markdown-body em { font-style: italic; }
.markdown-body del { text-decoration: line-through; }
.ferrite-mermaid { margin: 1rem 0; }
.ferrite-mermaid svg { max-width: 100%; height: auto; display: block; }
.ferrite-mermaid-fallback .mermaid-caption { font-size: 0.85rem; color: var(--muted, #666); margin-bottom: 0.5rem; }
.markdown-toc { margin-bottom: 2rem; padding: 1rem 1.25rem; border-radius: 8px; }
.markdown-toc .toc-title { margin-top: 0; font-size: 1.1rem; }
.markdown-toc ul { list-style: none; padding-left: 0; margin: 0; }
.markdown-toc li { margin: 0.35rem 0; }
.markdown-toc .toc-level-2 { padding-left: 1rem; }
.markdown-toc .toc-level-3 { padding-left: 2rem; }
.markdown-toc .toc-level-4 { padding-left: 3rem; }
.markdown-toc .toc-level-5 { padding-left: 4rem; }
.markdown-toc .toc-level-6 { padding-left: 5rem; }
"#;

fn generate_paragraph_indent_css(indent: ParagraphIndent) -> String {
    if let Some(em_value) = indent.to_css() {
        format!(
            r#".markdown-body > p {{ text-indent: {em}; }}"#,
            em = em_value
        )
    } else {
        String::new()
    }
}

fn color32_to_css(color: eframe::egui::Color32) -> String {
    format!(
        "rgba({},{},{},{})",
        color.r(),
        color.g(),
        color.b(),
        color.a() as f32 / 255.0
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Full HTML export used by the Save dialog.
pub fn generate_html_document_export(
    markdown: &str,
    title: Option<&str>,
    resolution: HtmlThemeResolution,
    paragraph_indent: ParagraphIndent,
    syntax_theme_name: &str,
    syntax_dark_mode: bool,
    options: &HtmlExportOptions,
    source_path: Option<&Path>,
) -> Result<String, HtmlExportError> {
    let header_comment = build_header_comment(options, source_path);

    let fm_note = if options.include_html_comments {
        extract_frontmatter_comment_raw(markdown)
            .map(|fm| format!("<!-- ferrite:frontmatter\n{}\n-->\n\n", html_escape(&fm)))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let (md2, mermaids) = extract_mermaid_fences(markdown);

    let mut comrak_opts = Options::default();
    comrak_opts.extension.strikethrough = true;
    comrak_opts.extension.table = true;
    comrak_opts.extension.autolink = true;
    comrak_opts.extension.tasklist = true;
    comrak_opts.extension.footnotes = true;
    comrak_opts.extension.header_ids = Some(String::new());
    comrak_opts.extension.front_matter_delimiter = Some("---".to_string());
    comrak_opts.render.unsafe_ = true;
    comrak_opts.render.sourcepos = options.include_html_comments;

    let adapter = FerriteHtmlHighlighter {
        theme_name: syntax_theme_name,
        dark_mode: syntax_dark_mode,
    };
    let mut plugins = Plugins::default();
    if options.include_syntax_highlighting {
        plugins.render.codefence_syntax_highlighter = Some(&adapter);
    }

    let mut body = markdown_to_html_with_plugins(&md2, &comrak_opts, &plugins);

    let fc = flowchart_colors_for_export(&resolution, syntax_dark_mode);
    body = inject_mermaid_exports(
        &body,
        &mermaids,
        820.0,
        &fc,
        syntax_theme_name,
        syntax_dark_mode,
    );

    let base = options.resolved_link_base(source_path.and_then(|p| p.parent()));
    let base_ref = base.as_deref();

    let img_handling = if options.self_contained {
        ImageHandling::EmbedBase64
    } else {
        options.image_handling
    };
    body = postprocess_images(&body, base_ref, options.self_contained, img_handling)?;
    if !options.self_contained {
        body = postprocess_links(&body, base_ref)?;
    }

    let toc = if options.include_outline {
        build_toc_html(markdown)
    } else {
        String::new()
    };

    let theme_css = generate_theme_styles(&resolution);
    let extra_syntax = if options.include_syntax_highlighting {
        String::new()
    } else {
        match &resolution {
            HtmlThemeResolution::Single(c) => syntax_css_from_palette(c),
            HtmlThemeResolution::Auto { light, .. } => syntax_css_from_palette(light),
        }
    };

    let indent_css = generate_paragraph_indent_css(paragraph_indent);
    let custom = options
        .custom_css
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("");

    let doc_title = title.unwrap_or("Exported Document");
    let use_theme = options.use_theme_colors;
    let style_block = if use_theme {
        format!("{BASE_CSS}\n{theme_css}\n{extra_syntax}\n{indent_css}\n{custom}")
    } else {
        format!("{BASE_CSS}\n{indent_css}\n{custom}")
    };

    let title_block = if options.include_title {
        format!(
            r#"<header class="doc-title"><h1>{}</h1></header>"#,
            html_escape(doc_title)
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Ferrite">
    <title>{title}</title>
    <style>
{styles}
    </style>
</head>
<body>
{comments}{fm}{article_open}
    {title_el}{toc}
{body}
{article_close}
</body>
</html>"#,
        title = html_escape(doc_title),
        styles = style_block,
        comments = header_comment,
        fm = fm_note,
        article_open = r#"<article class="markdown-body">"#,
        title_el = title_block,
        toc = toc,
        body = body,
        article_close = r#"</article>"#,
    );

    Ok(html)
}

/// Backwards-compatible helper: single resolved palette, syntax on, title on, self-contained by palette.
pub fn generate_html_document(
    markdown: &str,
    title: Option<&str>,
    theme_colors: &ThemeColors,
    include_syntax_css: bool,
    paragraph_indent: ParagraphIndent,
) -> Result<String, HtmlExportError> {
    let syn_dark = theme_colors.is_dark();
    let syn_name = if syn_dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    };
    let opts = HtmlExportOptions {
        include_syntax_highlighting: include_syntax_css,
        use_theme_colors: true,
        include_title: true,
        self_contained: true,
        ..Default::default()
    };
    generate_html_document_export(
        markdown,
        title,
        HtmlThemeResolution::Single(theme_colors.clone()),
        paragraph_indent,
        syn_name,
        syn_dark,
        &opts,
        None,
    )
}

pub fn generate_html_fragment(markdown: &str) -> Result<String, HtmlExportError> {
    let mut comrak_opts = Options::default();
    comrak_opts.extension.strikethrough = true;
    comrak_opts.extension.table = true;
    comrak_opts.extension.autolink = true;
    comrak_opts.extension.tasklist = true;
    comrak_opts.extension.footnotes = true;
    comrak_opts.extension.header_ids = Some(String::new());
    comrak_opts.render.unsafe_ = true;
    Ok(comrak::markdown_to_html(markdown, &comrak_opts))
}

pub fn export_to_html_file(
    source_path: &Path,
    output_path: &Path,
    theme_colors: &ThemeColors,
    paragraph_indent: ParagraphIndent,
) -> Result<(), HtmlExportError> {
    let markdown = std::fs::read_to_string(source_path)?;
    let title = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Document");
    let html =
        generate_html_document(&markdown, Some(title), theme_colors, true, paragraph_indent)?;
    std::fs::write(output_path, html)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_export_keeps_dollar_math_as_text() {
        let html = generate_html_fragment("Energy $E = mc^2$ is rest energy.").unwrap();
        assert!(
            html.contains("$E = mc^2$"),
            "export must keep the LaTeX source, got {html}"
        );
        assert!(
            !html.contains("<math"),
            "export must not emit MathML, got {html}"
        );
    }

    #[test]
    fn test_markdown_body() {
        let md = "# Hello\n\nWorld";
        let mut o = Options::default();
        o.extension.header_ids = Some(String::new());
        o.render.unsafe_ = true;
        let h = comrak::markdown_to_html(md, &o);
        assert!(h.contains("<h1"));
    }

    #[test]
    fn test_generate_html_document_smoke() {
        let md = "# Test\n\n```rust\nlet x=1;\n```\n";
        let c = ThemeColors::light();
        let html = generate_html_document(md, Some("T"), &c, true, ParagraphIndent::Off).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(
            html.contains("<pre") && html.contains("<code"),
            "expected fenced code in HTML (len {})\n{}",
            html.len(),
            html
        );
    }

    #[test]
    fn test_color32_to_css() {
        let c = eframe::egui::Color32::from_rgb(1, 2, 3);
        assert!(color32_to_css(c).contains("1"));
    }
}
