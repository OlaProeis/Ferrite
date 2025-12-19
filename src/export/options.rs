//! Export Options and Configuration
//!
//! This module defines the export options, format types, and configuration
//! for document export functionality.

// Allow dead code - this module provides complete export API with builder methods
// and format options that may be used for future export features (PDF, etc.)
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Export Format
// ─────────────────────────────────────────────────────────────────────────────

/// Supported export formats for documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Export as a standalone HTML file with embedded styles
    #[default]
    HtmlFile,
    /// Copy rendered HTML to clipboard
    ClipboardHtml,
}

impl ExportFormat {
    /// Get the display label for this format.
    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::HtmlFile => "HTML File",
            ExportFormat::ClipboardHtml => "Copy as HTML",
        }
    }

    /// Get the file extension for this format (if applicable).
    pub fn extension(&self) -> Option<&'static str> {
        match self {
            ExportFormat::HtmlFile => Some("html"),
            ExportFormat::ClipboardHtml => None,
        }
    }

    /// Get an icon for this format.
    pub fn icon(&self) -> &'static str {
        match self {
            ExportFormat::HtmlFile => "🌐",
            ExportFormat::ClipboardHtml => "📋",
        }
    }

    /// Get all available export formats.
    pub fn all() -> &'static [ExportFormat] {
        &[ExportFormat::HtmlFile, ExportFormat::ClipboardHtml]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Image Handling
// ─────────────────────────────────────────────────────────────────────────────

/// How to handle images in exported HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageHandling {
    /// Embed images as base64 data URIs (standalone, larger files)
    #[default]
    EmbedBase64,
    /// Keep relative paths (requires images to be available)
    RelativePaths,
    /// Convert to absolute paths
    AbsolutePaths,
}

impl ImageHandling {
    /// Get the display label for this option.
    pub fn label(&self) -> &'static str {
        match self {
            ImageHandling::EmbedBase64 => "Embed as Base64",
            ImageHandling::RelativePaths => "Relative Paths",
            ImageHandling::AbsolutePaths => "Absolute Paths",
        }
    }

    /// Get a description for this option.
    pub fn description(&self) -> &'static str {
        match self {
            ImageHandling::EmbedBase64 => "Images embedded in HTML (standalone, larger file)",
            ImageHandling::RelativePaths => "Keep image paths relative to document",
            ImageHandling::AbsolutePaths => "Convert to absolute file paths",
        }
    }

    /// Get all available image handling options.
    pub fn all() -> &'static [ImageHandling] {
        &[
            ImageHandling::EmbedBase64,
            ImageHandling::RelativePaths,
            ImageHandling::AbsolutePaths,
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Export Options
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration options for document export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    /// The export format to use
    pub format: ExportFormat,

    /// How to handle images in the export
    pub image_handling: ImageHandling,

    /// Whether to include the document title in the HTML
    pub include_title: bool,

    /// Whether to include syntax highlighting CSS
    pub include_syntax_highlighting: bool,

    /// Whether to use the current theme colors
    pub use_theme_colors: bool,

    /// Custom CSS to append (optional)
    pub custom_css: Option<String>,

    /// Last export directory (for remembering user preference)
    pub last_export_directory: Option<PathBuf>,

    /// Whether to open the exported file after export
    pub open_after_export: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::default(),
            image_handling: ImageHandling::default(),
            include_title: true,
            include_syntax_highlighting: true,
            use_theme_colors: true,
            custom_css: None,
            last_export_directory: None,
            open_after_export: false,
        }
    }
}

impl ExportOptions {
    /// Create options for HTML file export.
    pub fn html_file() -> Self {
        Self {
            format: ExportFormat::HtmlFile,
            ..Default::default()
        }
    }

    /// Create options for clipboard export.
    pub fn clipboard() -> Self {
        Self {
            format: ExportFormat::ClipboardHtml,
            // For clipboard, we typically want embedded images
            image_handling: ImageHandling::EmbedBase64,
            // Don't need title for clipboard fragments
            include_title: false,
            ..Default::default()
        }
    }

    /// Set the export directory.
    pub fn with_directory(mut self, dir: PathBuf) -> Self {
        self.last_export_directory = Some(dir);
        self
    }

    /// Set image handling mode.
    pub fn with_image_handling(mut self, handling: ImageHandling) -> Self {
        self.image_handling = handling;
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Export Settings (for persistence)
// ─────────────────────────────────────────────────────────────────────────────

/// Persistent export settings stored in user configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ExportSettings {
    /// Default export options
    pub default_options: ExportOptions,

    /// Last used export format
    pub last_format: ExportFormat,
}

impl ExportSettings {
    /// Get options for the last used format.
    pub fn last_options(&self) -> ExportOptions {
        let mut options = self.default_options.clone();
        options.format = self.last_format;
        options
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_default() {
        assert_eq!(ExportFormat::default(), ExportFormat::HtmlFile);
    }

    #[test]
    fn test_export_format_label() {
        assert_eq!(ExportFormat::HtmlFile.label(), "HTML File");
        assert_eq!(ExportFormat::ClipboardHtml.label(), "Copy as HTML");
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::HtmlFile.extension(), Some("html"));
        assert_eq!(ExportFormat::ClipboardHtml.extension(), None);
    }

    #[test]
    fn test_image_handling_default() {
        assert_eq!(ImageHandling::default(), ImageHandling::EmbedBase64);
    }

    #[test]
    fn test_export_options_default() {
        let options = ExportOptions::default();
        assert_eq!(options.format, ExportFormat::HtmlFile);
        assert!(options.include_title);
        assert!(options.include_syntax_highlighting);
        assert!(options.use_theme_colors);
    }

    #[test]
    fn test_export_options_clipboard() {
        let options = ExportOptions::clipboard();
        assert_eq!(options.format, ExportFormat::ClipboardHtml);
        assert!(!options.include_title);
    }

    #[test]
    fn test_export_options_serialization() {
        let options = ExportOptions::default();
        let json = serde_json::to_string(&options).unwrap();
        let deserialized: ExportOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(options, deserialized);
    }
}
