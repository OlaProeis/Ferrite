//! Clipboard Operations for HTML Export
//!
//! This module provides cross-platform clipboard functionality for copying
//! HTML content to the system clipboard using the arboard crate.

// Allow dead code - this module includes fallback methods for future clipboard enhancements
// - enum_variant_names: Error variants follow standard naming convention
#![allow(dead_code)]
#![allow(clippy::enum_variant_names)]

use super::html::{generate_html_fragment, HtmlExportError};
use arboard::Clipboard;

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard Error
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during clipboard operations.
#[derive(Debug)]
pub enum ClipboardError {
    /// Failed to access clipboard
    AccessError(String),
    /// Failed to set clipboard content
    WriteError(String),
    /// HTML generation failed
    HtmlError(HtmlExportError),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::AccessError(msg) => write!(f, "Clipboard access error: {}", msg),
            ClipboardError::WriteError(msg) => write!(f, "Clipboard write error: {}", msg),
            ClipboardError::HtmlError(e) => write!(f, "HTML generation error: {}", e),
        }
    }
}

impl std::error::Error for ClipboardError {}

impl From<HtmlExportError> for ClipboardError {
    fn from(err: HtmlExportError) -> Self {
        ClipboardError::HtmlError(err)
    }
}

impl From<arboard::Error> for ClipboardError {
    fn from(err: arboard::Error) -> Self {
        ClipboardError::WriteError(err.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard Operations
// ─────────────────────────────────────────────────────────────────────────────

/// Copy HTML content to clipboard.
///
/// This function converts markdown to HTML and copies it to the clipboard.
/// On platforms that support HTML clipboard format, apps like email clients
/// and word processors can paste the formatted content.
///
/// # Arguments
///
/// * `markdown` - The markdown source text to convert and copy
///
/// # Returns
///
/// Ok(()) on success, or a ClipboardError on failure.
///
/// # Example
///
/// ```ignore
/// use crate::export::clipboard::copy_html_to_clipboard;
///
/// copy_html_to_clipboard("# Hello\n\n**Bold** text")?;
/// // User can now paste formatted content in email/word processor
/// ```
pub fn copy_html_to_clipboard(markdown: &str) -> Result<(), ClipboardError> {
    // Generate HTML fragment from markdown
    let html = generate_html_fragment(markdown)?;

    // Copy HTML to clipboard
    copy_text_to_clipboard(&html)
}

/// Copy plain text to clipboard.
///
/// Uses arboard for cross-platform clipboard support.
pub fn copy_text_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::AccessError(e.to_string()))?;

    clipboard
        .set_text(text)
        .map_err(|e| ClipboardError::WriteError(e.to_string()))?;

    Ok(())
}

/// Copy HTML with plain text fallback to clipboard.
///
/// This attempts to set both HTML and plain text on the clipboard,
/// allowing rich paste in supported apps while providing a fallback.
///
/// # Arguments
///
/// * `html` - The HTML content
/// * `plain_text` - Plain text fallback
///
/// # Returns
///
/// Ok(()) on success, or a ClipboardError on failure.
pub fn copy_html_with_fallback(html: &str, plain_text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new().map_err(|e| ClipboardError::AccessError(e.to_string()))?;

    // arboard supports setting HTML directly on some platforms
    // For now, we use the HTML content as text since full HTML clipboard
    // support varies by platform
    clipboard
        .set_html(html, Some(plain_text))
        .map_err(|e| ClipboardError::WriteError(e.to_string()))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_error_display() {
        let err = ClipboardError::AccessError("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_clipboard_error_write() {
        let err = ClipboardError::WriteError("write failed".to_string());
        assert!(err.to_string().contains("write failed"));
    }

    #[test]
    fn test_html_error_conversion() {
        let html_err = HtmlExportError::ConversionError("test".to_string());
        let clipboard_err: ClipboardError = html_err.into();

        match clipboard_err {
            ClipboardError::HtmlError(_) => {}
            _ => panic!("Expected HtmlError variant"),
        }
    }

    // Note: Actual clipboard tests require a display/clipboard context
    // which isn't typically available in CI environments.
}
