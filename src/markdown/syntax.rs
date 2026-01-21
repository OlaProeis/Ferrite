//! Syntax Highlighting Module
//!
//! This module integrates syntect for code block syntax highlighting
//! in the rendered/WYSIWYG markdown editor mode.
//!
//! # Features
//! - Loads and caches syntect SyntaxSet and ThemeSet
//! - Provides theme-aware syntax highlighting (dark/light)
//! - Highlights code blocks by language identifier
//! - Converts syntect styles to egui RichText
//! - Extended syntax support via two-face (PowerShell, TypeScript, etc.)
//!
//! # Example
//! ```ignore
//! use crate::markdown::syntax::{SyntaxHighlighter, highlight_code};
//!
//! let highlighter = SyntaxHighlighter::new();
//! let highlighted = highlighter.highlight_code("fn main() {}", "rust", true);
//! ```

// Allow dead code - this module has more features than currently used by the rendered editor
#![allow(dead_code)]

use eframe::egui::{Color32, FontId, RichText};
use log::{debug, trace, warn};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

// Re-export two-face for extended syntax support

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Default dark theme name from syntect's built-in themes
pub const DEFAULT_DARK_THEME: &str = "base16-ocean.dark";

/// Default light theme name from syntect's built-in themes
pub const DEFAULT_LIGHT_THEME: &str = "InspiredGitHub";

/// Fallback theme if the specified theme is not found
pub const FALLBACK_THEME: &str = "base16-ocean.dark";

// ─────────────────────────────────────────────────────────────────────────────
// Highlighted Segment
// ─────────────────────────────────────────────────────────────────────────────

/// A segment of highlighted text with its associated color.
#[derive(Debug, Clone)]
pub struct HighlightedSegment {
    /// The text content of this segment
    pub text: String,
    /// Foreground color for this segment
    pub foreground: Color32,
    /// Whether this segment should be bold
    pub bold: bool,
    /// Whether this segment should be italic
    pub italic: bool,
    /// Whether this segment should be underlined
    pub underline: bool,
}

impl HighlightedSegment {
    /// Create a new highlighted segment.
    pub fn new(text: String, foreground: Color32) -> Self {
        Self {
            text,
            foreground,
            bold: false,
            italic: false,
            underline: false,
        }
    }

    /// Convert this segment to egui RichText with the specified font size.
    pub fn to_rich_text(&self, font_size: f32) -> RichText {
        let mut rich_text = RichText::new(&self.text)
            .color(self.foreground)
            .font(FontId::monospace(font_size));

        if self.bold {
            rich_text = rich_text.strong();
        }
        if self.italic {
            rich_text = rich_text.italics();
        }
        if self.underline {
            rich_text = rich_text.underline();
        }

        rich_text
    }
}

/// A line of highlighted segments.
#[derive(Debug, Clone)]
pub struct HighlightedLine {
    /// The segments that make up this line
    pub segments: Vec<HighlightedSegment>,
}

impl HighlightedLine {
    /// Create a new highlighted line from segments.
    pub fn new(segments: Vec<HighlightedSegment>) -> Self {
        Self { segments }
    }

    /// Create an unhighlighted line with a single segment.
    pub fn plain(text: &str, color: Color32) -> Self {
        Self {
            segments: vec![HighlightedSegment::new(text.to_string(), color)],
        }
    }

    /// Check if this line is empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.segments.iter().all(|s| s.text.is_empty())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Syntax Highlighter
// ─────────────────────────────────────────────────────────────────────────────

/// Syntax highlighter that caches syntect sets for performance.
///
/// This struct holds the loaded SyntaxSet and ThemeSet, which are expensive
/// to load and should be reused across highlighting operations.
pub struct SyntaxHighlighter {
    /// Loaded syntax definitions
    syntax_set: SyntaxSet,
    /// Loaded color themes
    theme_set: ThemeSet,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default syntax and theme sets.
    ///
    /// This loads extended syntaxes and themes from two-face (which includes
    /// PowerShell, TypeScript, and many other languages beyond syntect's defaults)
    /// along with popular themes like Dracula, Nord, Catppuccin, Gruvbox, etc.
    ///
    /// The operation is relatively expensive, so the highlighter should be
    /// cached and reused.
    pub fn new() -> Self {
        debug!("Loading syntect syntax and theme sets (with two-face extras)");
        // Use two-face's extended syntax set which includes PowerShell, TypeScript, etc.
        let syntax_set = two_face::syntax::extra_newlines();
        // Use two-face's extended theme set which includes Dracula, Nord, Catppuccin, etc.
        // Convert to syntect's ThemeSet for compatibility
        let theme_set: ThemeSet = two_face::theme::extra().into();
        debug!(
            "Loaded {} syntaxes and {} themes",
            syntax_set.syntaxes().len(),
            theme_set.themes.len()
        );
        Self {
            syntax_set,
            theme_set,
        }
    }

    /// Get a reference to the syntax set.
    pub fn syntax_set(&self) -> &SyntaxSet {
        &self.syntax_set
    }

    /// Get a reference to the theme set.
    pub fn theme_set(&self) -> &ThemeSet {
        &self.theme_set
    }

    /// Get available theme names (unsorted).
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// Get available theme names sorted alphabetically.
    /// Returns a Vec of (theme_name, display_name) tuples.
    pub fn available_themes_sorted(&self) -> Vec<(String, String)> {
        let mut themes: Vec<_> = self
            .theme_set
            .themes
            .keys()
            .map(|name| {
                // Create a display name by prettifying the theme name
                let display = prettify_theme_name(name);
                (name.clone(), display)
            })
            .collect();
        // Sort by display name
        themes.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        themes
    }

    /// Get a theme by name, falling back to the default if not found.
    pub fn get_theme(&self, name: &str) -> &Theme {
        self.theme_set
            .themes
            .get(name)
            .or_else(|| self.theme_set.themes.get(FALLBACK_THEME))
            .expect("Fallback theme should always exist")
    }

    /// Get the appropriate theme for dark or light mode.
    pub fn get_theme_for_mode(&self, dark_mode: bool) -> &Theme {
        let theme_name = if dark_mode {
            DEFAULT_DARK_THEME
        } else {
            DEFAULT_LIGHT_THEME
        };
        self.get_theme(theme_name)
    }

    /// Get a theme by name from settings, with fallback based on dark mode.
    pub fn get_theme_by_name_or_mode(&self, theme_name: &str, dark_mode: bool) -> &Theme {
        if self.theme_set.themes.contains_key(theme_name) {
            self.get_theme(theme_name)
        } else {
            self.get_theme_for_mode(dark_mode)
        }
    }

    /// Highlight code with the specified language and theme.
    ///
    /// # Arguments
    /// * `code` - The source code to highlight
    /// * `language` - Language identifier (e.g., "rust", "python", "js")
    /// * `theme` - The syntect theme to use
    ///
    /// # Returns
    /// A vector of highlighted lines, or None if the language is not recognized.
    pub fn highlight_code(
        &self,
        code: &str,
        language: &str,
        theme: &Theme,
    ) -> Vec<HighlightedLine> {
        // Try to find syntax by language identifier
        let syntax = self.find_syntax_for_language(language);

        match syntax {
            Some(syntax_ref) => {
                let mut highlighter = HighlightLines::new(syntax_ref, theme);
                let mut lines = Vec::new();

                for line in LinesWithEndings::from(code) {
                    match highlighter.highlight_line(line, &self.syntax_set) {
                        Ok(ranges) => {
                            let segments = ranges
                                .into_iter()
                                .map(|(style, text)| style_to_segment(style, text))
                                .collect();
                            lines.push(HighlightedLine::new(segments));
                        }
                        Err(e) => {
                            warn!("Failed to highlight line: {}", e);
                            // Fall back to plain text for this line
                            let default_color = theme
                                .settings
                                .foreground
                                .map(syntect_to_egui_color)
                                .unwrap_or(Color32::GRAY);
                            lines.push(HighlightedLine::plain(line, default_color));
                        }
                    }
                }

                lines
            }
            None => {
                // Language not recognized, return plain text
                // Use trace level to avoid spam - this is expected for unlabeled code blocks
                trace!("No syntax found for language: {}", language);
                let default_color = theme
                    .settings
                    .foreground
                    .map(syntect_to_egui_color)
                    .unwrap_or(Color32::GRAY);

                // IMPORTANT: Use LinesWithEndings to preserve newlines!
                // Using code.lines() would strip newlines, causing text to collapse
                // to a single line when building the LayoutJob.
                LinesWithEndings::from(code)
                    .map(|line| HighlightedLine::plain(line, default_color))
                    .collect()
            }
        }
    }

    /// Highlight code for dark or light mode.
    pub fn highlight_code_for_mode(
        &self,
        code: &str,
        language: &str,
        dark_mode: bool,
    ) -> Vec<HighlightedLine> {
        let theme = self.get_theme_for_mode(dark_mode);
        self.highlight_code(code, language, theme)
    }

    /// Find syntax definition for a language identifier.
    ///
    /// Tries multiple strategies:
    /// 1. By extension (e.g., "rs" -> Rust)
    /// 2. By name (e.g., "Rust" -> Rust)
    /// 3. By first line (for shebangs)
    fn find_syntax_for_language(
        &self,
        language: &str,
    ) -> Option<&syntect::parsing::SyntaxReference> {
        if language.is_empty() {
            return None;
        }

        // Normalize the language identifier
        let lang_lower = language.to_lowercase();

        // Map common language aliases to extensions
        let extension = match lang_lower.as_str() {
            "rust" | "rs" => "rs",
            "python" | "py" => "py",
            "javascript" | "js" => "js",
            "typescript" | "ts" => "ts",
            "tsx" => "tsx",
            "jsx" => "jsx",
            "c" => "c",
            "cpp" | "c++" | "cxx" => "cpp",
            "csharp" | "c#" | "cs" => "cs",
            "java" => "java",
            "kotlin" | "kt" => "kt",
            "go" | "golang" => "go",
            "ruby" | "rb" => "rb",
            "php" => "php",
            "swift" => "swift",
            "scala" => "scala",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "sass" => "sass",
            "less" => "less",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "xml" => "xml",
            "markdown" | "md" => "md",
            "sql" => "sql",
            "shell" | "sh" | "bash" | "zsh" => "sh",
            "powershell" | "ps1" => "ps1",
            "dockerfile" | "docker" => "Dockerfile",
            "makefile" | "make" => "Makefile",
            "lua" => "lua",
            "perl" | "pl" => "pl",
            "r" => "r",
            "haskell" | "hs" => "hs",
            "elixir" | "ex" => "ex",
            "erlang" | "erl" => "erl",
            "clojure" | "clj" => "clj",
            "vim" => "vim",
            "diff" | "patch" => "diff",
            "ini" | "cfg" => "ini",
            "graphql" | "gql" => "graphql",
            other => other,
        };

        // Try by extension first
        if let Some(syntax) = self.syntax_set.find_syntax_by_extension(extension) {
            return Some(syntax);
        }

        // Try by name
        if let Some(syntax) = self.syntax_set.find_syntax_by_name(language) {
            return Some(syntax);
        }

        // Try case-insensitive name search
        self.syntax_set
            .syntaxes()
            .iter()
            .find(|&syntax| syntax.name.to_lowercase() == lang_lower)
            .map(|v| v as _)
    }

    /// Get the background color for a theme.
    pub fn get_theme_background(&self, theme: &Theme) -> Option<Color32> {
        theme.settings.background.map(syntect_to_egui_color)
    }

    /// Get the foreground color for a theme.
    pub fn get_theme_foreground(&self, theme: &Theme) -> Option<Color32> {
        theme.settings.foreground.map(syntect_to_egui_color)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert syntect Color to egui Color32.
pub fn syntect_to_egui_color(color: syntect::highlighting::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Convert syntect Style to HighlightedSegment.
fn style_to_segment(style: Style, text: &str) -> HighlightedSegment {
    let foreground = syntect_to_egui_color(style.foreground);

    HighlightedSegment {
        text: text.to_string(),
        foreground,
        bold: style
            .font_style
            .contains(syntect::highlighting::FontStyle::BOLD),
        italic: style
            .font_style
            .contains(syntect::highlighting::FontStyle::ITALIC),
        underline: style
            .font_style
            .contains(syntect::highlighting::FontStyle::UNDERLINE),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Global Highlighter Instance
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::OnceLock;

/// Global syntax highlighter instance.
///
/// This is lazily initialized on first access and reused for all highlighting
/// operations. Using a global instance avoids the cost of loading syntax and
/// theme sets multiple times.
static HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();

/// Get or create the global syntax highlighter.
pub fn get_highlighter() -> &'static SyntaxHighlighter {
    HIGHLIGHTER.get_or_init(SyntaxHighlighter::new)
}

/// Highlight code using the global highlighter.
///
/// This is a convenience function that uses the global highlighter instance.
///
/// # Arguments
/// * `code` - The source code to highlight
/// * `language` - Language identifier (e.g., "rust", "python")
/// * `dark_mode` - Whether to use dark mode theme
///
/// # Returns
/// A vector of highlighted lines.
pub fn highlight_code(code: &str, language: &str, dark_mode: bool) -> Vec<HighlightedLine> {
    get_highlighter().highlight_code_for_mode(code, language, dark_mode)
}

/// Highlight code with a specific theme name.
///
/// # Arguments
/// * `code` - The source code to highlight
/// * `language` - Language identifier
/// * `theme_name` - Name of the syntect theme to use
/// * `dark_mode` - Fallback mode if theme is not found
pub fn highlight_code_with_theme(
    code: &str,
    language: &str,
    theme_name: &str,
    dark_mode: bool,
) -> Vec<HighlightedLine> {
    let highlighter = get_highlighter();
    let theme = highlighter.get_theme_by_name_or_mode(theme_name, dark_mode);
    highlighter.highlight_code(code, language, theme)
}

/// Get available syntax highlighting themes sorted alphabetically.
/// Returns a Vec of (theme_name, display_name) tuples.
pub fn get_available_themes() -> Vec<(String, String)> {
    get_highlighter().available_themes_sorted()
}

/// Prettify a theme name for display in the UI.
fn prettify_theme_name(name: &str) -> String {
    // Special case mappings for better display names
    match name {
        "1337" => "1337 (Leet)".to_string(),
        "ansi" => "ANSI".to_string(),
        "base16" => "Base16".to_string(),
        "base16-256" => "Base16 256".to_string(),
        "Coldark-Cold" => "Coldark Light".to_string(),
        "Coldark-Dark" => "Coldark Dark".to_string(),
        "DarkNeon" => "Dark Neon".to_string(),
        "InspiredGitHub" => "Inspired GitHub".to_string(),
        "Monokai Extended" => "Monokai Extended".to_string(),
        "Monokai Extended Bright" => "Monokai Bright".to_string(),
        "Monokai Extended Light" => "Monokai Light".to_string(),
        "Monokai Extended Origin" => "Monokai Origin".to_string(),
        "OneHalfDark" => "One Half Dark".to_string(),
        "OneHalfLight" => "One Half Light".to_string(),
        "Sublime Snazzy" => "Sublime Snazzy".to_string(),
        "TwoDark" => "Two Dark".to_string(),
        "Visual Studio Dark+" => "VS Code Dark+".to_string(),
        "zenburn" => "Zenburn".to_string(),
        // Default: capitalize and replace separators
        _ => {
            name.split(|c| c == '-' || c == '_' || c == '.')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => {
                            first.to_uppercase().collect::<String>() + chars.as_str()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

/// Get the language identifier for a file path extension.
///
/// Returns the file extension as a language identifier that can be passed
/// to `highlight_code`. Returns None for unknown or unsupported extensions.
///
/// # Arguments
/// * `path` - The file path to check
///
/// # Returns
/// Some(language) if the file extension is recognized, None otherwise.
pub fn language_from_path(path: &std::path::Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    
    // Check if syntect can handle this extension
    let highlighter = get_highlighter();
    if highlighter.syntax_set().find_syntax_by_extension(&ext).is_some() {
        return Some(ext);
    }
    
    // Map common extensions that might not be found by direct lookup
    let mapped = match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "cpp" | "cxx" | "cc" | "hpp" => "cpp",
        "c" | "h" => "c",
        "cs" => "csharp",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "java" => "java",
        "scala" => "scala",
        "sh" | "bash" | "zsh" => "sh",
        "ps1" => "powershell",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "lua" => "lua",
        "pl" | "pm" => "perl",
        "r" => "r",
        "hs" => "haskell",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "clj" | "cljs" => "clojure",
        "vim" => "vim",
        "diff" | "patch" => "diff",
        "ini" | "cfg" => "ini",
        "cmake" => "cmake",
        "dockerfile" => "dockerfile",
        "makefile" | "mk" => "makefile",
        _ => return None,
    };
    
    Some(mapped.to_string())
}

/// Check if a file path has a syntax that can be highlighted.
///
/// # Arguments
/// * `path` - The file path to check
///
/// # Returns
/// true if the file can be syntax highlighted, false otherwise.
pub fn can_highlight_file(path: &std::path::Path) -> bool {
    language_from_path(path).is_some()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.syntax_set.syntaxes().is_empty());
        assert!(!highlighter.theme_set.themes.is_empty());
    }

    #[test]
    fn test_available_themes() {
        let highlighter = SyntaxHighlighter::new();
        let themes = highlighter.available_themes();
        assert!(themes.contains(&"base16-ocean.dark"));
        assert!(themes.contains(&"InspiredGitHub"));
    }

    #[test]
    fn test_get_theme_for_mode() {
        let highlighter = SyntaxHighlighter::new();

        let dark_theme = highlighter.get_theme_for_mode(true);
        assert!(dark_theme.name.is_some() || dark_theme.settings.background.is_some());

        let light_theme = highlighter.get_theme_for_mode(false);
        assert!(light_theme.name.is_some() || light_theme.settings.background.is_some());
    }

    #[test]
    fn test_highlight_rust_code() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlighter.highlight_code_for_mode(code, "rust", true);

        assert_eq!(lines.len(), 3);
        assert!(!lines[0].segments.is_empty());
    }

    #[test]
    fn test_highlight_python_code() {
        let highlighter = SyntaxHighlighter::new();
        let code = "def hello():\n    print('Hello')";
        let lines = highlighter.highlight_code_for_mode(code, "python", true);

        assert_eq!(lines.len(), 2);
        assert!(!lines[0].segments.is_empty());
    }

    #[test]
    fn test_highlight_unknown_language() {
        let highlighter = SyntaxHighlighter::new();
        let code = "some random text";
        let lines = highlighter.highlight_code_for_mode(code, "unknownlang123", true);

        // Should still return lines, just without syntax-specific highlighting
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].segments.is_empty());
    }

    #[test]
    fn test_fallback_highlighting_preserves_newlines() {
        // Regression test: when syntax is NOT found, the fallback highlighting
        // must preserve newlines. Previously, using code.lines() stripped them,
        // causing text to collapse to a single line in the editor.
        let highlighter = SyntaxHighlighter::new();
        
        // Use an unknown language to trigger the fallback path
        let code = "line one\nline two\nline three";
        let lines = highlighter.highlight_code_for_mode(code, "unknownlang123", true);
        
        assert_eq!(lines.len(), 3, "Should have 3 lines for 3 input lines");
        
        // Concatenate all segment text to verify newlines are preserved
        let mut full_text = String::new();
        for line in &lines {
            for segment in &line.segments {
                full_text.push_str(&segment.text);
            }
        }
        
        // The reconstructed text must match the original (with newlines)
        assert_eq!(full_text, code, "Highlighted text must preserve newlines");
        assert!(full_text.contains('\n'), "Output must contain newlines");
    }

    #[test]
    fn test_fallback_highlighting_with_trailing_newline() {
        // Test that trailing newlines are also preserved
        let highlighter = SyntaxHighlighter::new();
        
        let code = "line one\nline two\n";  // Has trailing newline
        let lines = highlighter.highlight_code_for_mode(code, "unknownlang123", true);
        
        // Should have 2 lines (LinesWithEndings includes the trailing newline with line two)
        assert_eq!(lines.len(), 2, "Should have 2 lines");
        
        let mut full_text = String::new();
        for line in &lines {
            for segment in &line.segments {
                full_text.push_str(&segment.text);
            }
        }
        
        assert_eq!(full_text, code, "Trailing newline must be preserved");
    }

    #[test]
    fn test_fallback_highlighting_crlf() {
        // Test Windows-style line endings (CRLF)
        let highlighter = SyntaxHighlighter::new();
        
        let code = "line one\r\nline two\r\nline three";
        let lines = highlighter.highlight_code_for_mode(code, "unknownlang123", true);
        
        let mut full_text = String::new();
        for line in &lines {
            for segment in &line.segments {
                full_text.push_str(&segment.text);
            }
        }
        
        assert_eq!(full_text, code, "CRLF line endings must be preserved");
    }

    #[test]
    fn test_powershell_syntax_available() {
        // Verify PowerShell syntax is available via two-face
        let highlighter = SyntaxHighlighter::new();
        
        // PowerShell should be found by extension
        let ps1_syntax = highlighter.find_syntax_for_language("ps1");
        assert!(ps1_syntax.is_some(), "PowerShell syntax should be found for 'ps1'");
        
        // Also check by name
        let powershell_syntax = highlighter.find_syntax_for_language("powershell");
        assert!(powershell_syntax.is_some(), "PowerShell syntax should be found for 'powershell'");
        
        // Verify they resolve to the same syntax
        assert_eq!(
            ps1_syntax.unwrap().name,
            powershell_syntax.unwrap().name,
            "ps1 and powershell should resolve to the same syntax"
        );
    }

    #[test]
    fn test_powershell_highlighting() {
        // Test actual PowerShell code highlighting
        let highlighter = SyntaxHighlighter::new();
        
        let code = "Write-Host \"Hello, World!\"\n$var = 123\n";
        let lines = highlighter.highlight_code_for_mode(code, "ps1", true);
        
        // Should have 2 lines (with newlines)
        assert_eq!(lines.len(), 2, "Should have 2 lines of PowerShell code");
        
        // Each line should have multiple colored segments (not just one plain segment)
        // This indicates actual syntax highlighting is happening
        let total_segments: usize = lines.iter().map(|l| l.segments.len()).sum();
        assert!(total_segments > 2, "PowerShell should have syntax-highlighted segments, got {}", total_segments);
        
        // Verify content is preserved
        let mut full_text = String::new();
        for line in &lines {
            for segment in &line.segments {
                full_text.push_str(&segment.text);
            }
        }
        assert_eq!(full_text, code, "Content must be preserved after highlighting");
    }

    #[test]
    fn test_highlight_empty_code() {
        let highlighter = SyntaxHighlighter::new();
        let lines = highlighter.highlight_code_for_mode("", "rust", true);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_language_aliases() {
        let highlighter = SyntaxHighlighter::new();

        // Test various language aliases
        let aliases = vec![
            ("rs", "rust"),
            ("py", "python"),
            ("js", "javascript"),
            ("ts", "typescript"),
            ("cpp", "c++"),
        ];

        for (alias, canonical) in aliases {
            let syntax1 = highlighter.find_syntax_for_language(alias);
            let syntax2 = highlighter.find_syntax_for_language(canonical);

            if syntax1.is_some() && syntax2.is_some() {
                assert_eq!(
                    syntax1.unwrap().name,
                    syntax2.unwrap().name,
                    "Alias {} should map to same syntax as {}",
                    alias,
                    canonical
                );
            }
        }
    }

    #[test]
    fn test_syntect_to_egui_color() {
        let syntect_color = syntect::highlighting::Color {
            r: 255,
            g: 128,
            b: 64,
            a: 255,
        };
        let egui_color = syntect_to_egui_color(syntect_color);

        assert_eq!(egui_color.r(), 255);
        assert_eq!(egui_color.g(), 128);
        assert_eq!(egui_color.b(), 64);
        assert_eq!(egui_color.a(), 255);
    }

    #[test]
    fn test_highlighted_segment_to_rich_text() {
        let segment = HighlightedSegment {
            text: "test".to_string(),
            foreground: Color32::RED,
            bold: true,
            italic: false,
            underline: false,
        };

        let rich_text = segment.to_rich_text(14.0);
        // RichText doesn't expose internal state, but we can verify it doesn't panic
        assert!(!rich_text.text().is_empty());
    }

    #[test]
    fn test_global_highlighter() {
        // Test that global highlighter works
        let lines = highlight_code("let x = 5;", "rust", true);
        assert!(!lines.is_empty());

        // Test that it returns the same instance
        let h1 = get_highlighter();
        let h2 = get_highlighter();
        assert!(std::ptr::eq(h1, h2));
    }

    #[test]
    fn test_highlight_with_theme() {
        let lines =
            highlight_code_with_theme("print('hello')", "python", "base16-ocean.dark", true);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_theme_colors() {
        let highlighter = SyntaxHighlighter::new();
        let theme = highlighter.get_theme_for_mode(true);

        // Dark theme should have a background color
        let bg = highlighter.get_theme_background(theme);
        assert!(bg.is_some());

        let fg = highlighter.get_theme_foreground(theme);
        assert!(fg.is_some());
    }
}
