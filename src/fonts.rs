//! Font management for Ferrite
//!
//! This module handles loading custom fonts with proper bold/italic variants.
//! Fonts are embedded at compile time using `include_bytes!`.

// Allow dead code - includes utility functions for font styling that may be
// used for advanced text rendering features
#![allow(dead_code)]

use egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use log::{info, warn};
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// Font Data - Embedded at compile time
// ─────────────────────────────────────────────────────────────────────────────

// Inter font family (UI/proportional)
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
const INTER_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-Italic.ttf");
const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-BoldItalic.ttf");

// JetBrains Mono font family (code/monospace)
const JETBRAINS_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");
const JETBRAINS_ITALIC: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf");
const JETBRAINS_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf");

// ─────────────────────────────────────────────────────────────────────────────
// System Font Detection
// ─────────────────────────────────────────────────────────────────────────────

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use std::sync::Arc;

// NanumGothic bundled fallback removed per user request.
// We strictly rely on system fonts now.

/// Attempt to load a system CJK font dynamically.
///
/// Returns `Some(FontData)` if a suitable system font is found and loaded.
/// Returns `None` if no system font could be loaded, in which case the bundled fallback should be used.
fn load_system_cjk_font() -> Option<FontData> {
    let source = SystemSource::new();

    // Define fallback list per OS
    // Prioritize fonts that provide good Korean support
    #[cfg(target_os = "macos")]
    let families = ["Apple SD Gothic Neo", "PingFang SC", "Hiragino Sans"];

    #[cfg(target_os = "windows")]
    let families = ["Malgun Gothic", "Microsoft YaHei", "Yu Gothic"];

    #[cfg(target_os = "linux")]
    let families = [
        "Noto Sans CJK KR",
        "Noto Sans CJK SC",
        "NanumGothic",
        "UnDotum",
    ];

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let families = ["Noto Sans CJK KR"];

    for family in families {
        info!("Attempting to load system font: {}", family);
        if let Ok(handle) =
            source.select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new())
        {
            match handle {
                Handle::Path { path, .. } => {
                    info!("Found system font at: {:?}", path);
                    // Read file content
                    if let Ok(bytes) = std::fs::read(&path) {
                        return Some(FontData::from_owned(bytes));
                    }
                }
                Handle::Memory { bytes, .. } => {
                    info!("Found system font in memory ({} bytes)", bytes.len());
                    return Some(FontData::from_owned(bytes.to_vec()));
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, sometimes generic SansSerif resolves to Noto Sans CJK
        info!("Attempting to resolve generic SansSerif font");
        if let Ok(handle) = source.select_best_match(&[FamilyName::SansSerif], &Properties::new()) {
            if let Ok(font) = handle.load() {
                let name = font.postscript_name().unwrap_or_default();
                info!("Resolved SansSerif to: {}", name);
                // Only use if it looks like a CJK font or if we are desperate?
                // Actually, let's trust specific family names first.
                // We can try to load data from handle
                match handle {
                    Handle::Path { path, .. } => {
                        if let Ok(bytes) = std::fs::read(&path) {
                            return Some(FontData::from_owned(bytes));
                        }
                    }
                    Handle::Memory { bytes, .. } => {
                        return Some(FontData::from_owned(bytes.to_vec()));
                    }
                }
            }
        }
    }

    warn!("No system CJK font found. CJK characters may not render correctly.");
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Family Names
// ─────────────────────────────────────────────────────────────────────────────

/// Custom font family for Inter (proportional UI font)
pub const FONT_INTER: &str = "Inter";
/// Custom font family for Inter Bold
pub const FONT_INTER_BOLD: &str = "Inter-Bold";
/// Custom font family for Inter Italic
pub const FONT_INTER_ITALIC: &str = "Inter-Italic";
/// Custom font family for Inter Bold Italic
pub const FONT_INTER_BOLD_ITALIC: &str = "Inter-BoldItalic";

/// Custom font family for JetBrains Mono (monospace/code font)
pub const FONT_JETBRAINS: &str = "JetBrainsMono";
/// Custom font family for JetBrains Mono Bold
pub const FONT_JETBRAINS_BOLD: &str = "JetBrainsMono-Bold";
/// Custom font family for JetBrains Mono Italic
pub const FONT_JETBRAINS_ITALIC: &str = "JetBrainsMono-Italic";
/// Custom font family for JetBrains Mono Bold Italic
pub const FONT_JETBRAINS_BOLD_ITALIC: &str = "JetBrainsMono-BoldItalic";

/// Generic name for the loaded CJK font (Apple SD Gothic, Malgun Gothic, or Noto Sans CJK)
const FONT_CJK: &str = "CJK";

// ─────────────────────────────────────────────────────────────────────────────
// Font Loading
// ─────────────────────────────────────────────────────────────────────────────

/// Create font definitions with custom fonts loaded.
///
/// This sets up:
/// - Inter as the proportional (UI) font with bold/italic variants
/// - JetBrains Mono as the monospace (code) font with bold/italic variants
/// - Custom named font families for explicit bold/italic access
pub fn create_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    // Insert Inter font variants
    fonts
        .font_data
        .insert(FONT_INTER.to_owned(), FontData::from_static(INTER_REGULAR));
    fonts.font_data.insert(
        FONT_INTER_BOLD.to_owned(),
        FontData::from_static(INTER_BOLD),
    );
    fonts.font_data.insert(
        FONT_INTER_ITALIC.to_owned(),
        FontData::from_static(INTER_ITALIC),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD_ITALIC.to_owned(),
        FontData::from_static(INTER_BOLD_ITALIC),
    );

    // Insert JetBrains Mono font variants
    fonts.font_data.insert(
        FONT_JETBRAINS.to_owned(),
        FontData::from_static(JETBRAINS_REGULAR),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD.to_owned(),
        FontData::from_static(JETBRAINS_BOLD),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_ITALIC),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_BOLD_ITALIC),
    );

    // Insert CJK/Korean font (System preferred)
    if let Some(font_data) = load_system_cjk_font() {
        fonts.font_data.insert(FONT_CJK.to_owned(), font_data);
    } else {
        warn!("Skipping CJK font registration as no system font was found.");
    }

    // Set up Proportional font family (Inter with Noto Sans KR fallback for Korean)
    // Order matters: first font is primary, rest are fallbacks
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_INTER.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(FONT_CJK.to_owned());

    // Set up Monospace font family (JetBrains Mono with Noto Sans KR fallback)
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, FONT_JETBRAINS.to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_CJK.to_owned());

    // Get fallback fonts from default families for CJK/Korean support
    let proportional_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Create custom named font families for explicit style access
    // These allow us to directly select bold/italic fonts
    // Each family includes fallbacks for CJK character support

    // Inter variants with proportional fallbacks
    let mut inter_family = vec![FONT_INTER.to_owned()];
    inter_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER.into()), inter_family);

    let mut inter_bold_family = vec![FONT_INTER_BOLD.to_owned()];
    inter_bold_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER_BOLD.into()), inter_bold_family);

    let mut inter_italic_family = vec![FONT_INTER_ITALIC.to_owned()];
    inter_italic_family.extend(proportional_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_ITALIC.into()),
        inter_italic_family,
    );

    let mut inter_bold_italic_family = vec![FONT_INTER_BOLD_ITALIC.to_owned()];
    inter_bold_italic_family.extend(proportional_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_BOLD_ITALIC.into()),
        inter_bold_italic_family,
    );

    // JetBrains Mono variants with monospace fallbacks
    let mut jetbrains_family = vec![FONT_JETBRAINS.to_owned()];
    jetbrains_family.extend(monospace_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_JETBRAINS.into()), jetbrains_family);

    let mut jetbrains_bold_family = vec![FONT_JETBRAINS_BOLD.to_owned()];
    jetbrains_bold_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD.into()),
        jetbrains_bold_family,
    );

    let mut jetbrains_italic_family = vec![FONT_JETBRAINS_ITALIC.to_owned()];
    jetbrains_italic_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_ITALIC.into()),
        jetbrains_italic_family,
    );

    let mut jetbrains_bold_italic_family = vec![FONT_JETBRAINS_BOLD_ITALIC.to_owned()];
    jetbrains_bold_italic_family.extend(monospace_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into()),
        jetbrains_bold_italic_family,
    );

    info!("Loaded custom fonts: Inter, JetBrains Mono, CJK");

    fonts
}

/// Apply custom fonts to an egui context.
///
/// This should be called once during application initialization.
pub fn setup_fonts(ctx: &egui::Context) {
    let fonts = create_font_definitions();
    ctx.set_fonts(fonts);

    // Configure text styles with appropriate sizes
    let text_styles: BTreeMap<TextStyle, FontId> = [
        (
            TextStyle::Heading,
            FontId::new(24.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
    ]
    .into();

    ctx.style_mut(|style| {
        style.text_styles = text_styles.clone();
    });

    info!("Configured egui text styles");
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions for Getting Font Families
// ─────────────────────────────────────────────────────────────────────────────

use crate::config::EditorFont;

/// Get the appropriate font family for styled text based on editor font setting.
///
/// This returns the correct font variant based on bold/italic flags and the
/// user's selected editor font.
pub fn get_styled_font_family(bold: bool, italic: bool, editor_font: EditorFont) -> FontFamily {
    match editor_font {
        EditorFont::JetBrainsMono => match (bold, italic) {
            (true, true) => FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into()),
            (true, false) => FontFamily::Name(FONT_JETBRAINS_BOLD.into()),
            (false, true) => FontFamily::Name(FONT_JETBRAINS_ITALIC.into()),
            (false, false) => FontFamily::Name(FONT_JETBRAINS.into()),
        },
        EditorFont::Inter => match (bold, italic) {
            (true, true) => FontFamily::Name(FONT_INTER_BOLD_ITALIC.into()),
            (true, false) => FontFamily::Name(FONT_INTER_BOLD.into()),
            (false, true) => FontFamily::Name(FONT_INTER_ITALIC.into()),
            (false, false) => FontFamily::Name(FONT_INTER.into()),
        },
    }
}

/// Get the base font family for an editor font (regular weight, no style).
pub fn get_base_font_family(editor_font: EditorFont) -> FontFamily {
    match editor_font {
        EditorFont::Inter => FontFamily::Name(FONT_INTER.into()),
        EditorFont::JetBrainsMono => FontFamily::Name(FONT_JETBRAINS.into()),
    }
}

/// Create a FontId for styled text.
///
/// Convenience function that combines size with the appropriate styled font family.
pub fn styled_font_id(size: f32, bold: bool, italic: bool, editor_font: EditorFont) -> FontId {
    FontId::new(size, get_styled_font_family(bold, italic, editor_font))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_font_definitions() {
        let fonts = create_font_definitions();

        // Check that all font data is loaded
        assert!(fonts.font_data.contains_key(FONT_INTER));
        assert!(fonts.font_data.contains_key(FONT_INTER_BOLD));
        assert!(fonts.font_data.contains_key(FONT_INTER_ITALIC));
        assert!(fonts.font_data.contains_key(FONT_INTER_BOLD_ITALIC));

        assert!(fonts.font_data.contains_key(FONT_JETBRAINS));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_BOLD));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_ITALIC));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_BOLD_ITALIC));

        // Check that font families are set up
        assert!(fonts.families.contains_key(&FontFamily::Proportional));
        assert!(fonts.families.contains_key(&FontFamily::Monospace));
    }

    #[test]
    fn test_get_styled_font_family_inter() {
        // Inter variants
        assert_eq!(
            get_styled_font_family(false, false, EditorFont::Inter),
            FontFamily::Name(FONT_INTER.into())
        );
        assert_eq!(
            get_styled_font_family(true, false, EditorFont::Inter),
            FontFamily::Name(FONT_INTER_BOLD.into())
        );
        assert_eq!(
            get_styled_font_family(false, true, EditorFont::Inter),
            FontFamily::Name(FONT_INTER_ITALIC.into())
        );
        assert_eq!(
            get_styled_font_family(true, true, EditorFont::Inter),
            FontFamily::Name(FONT_INTER_BOLD_ITALIC.into())
        );
    }

    #[test]
    fn test_get_styled_font_family_jetbrains() {
        // JetBrains Mono variants
        assert_eq!(
            get_styled_font_family(false, false, EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS.into())
        );
        assert_eq!(
            get_styled_font_family(true, false, EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_BOLD.into())
        );
        assert_eq!(
            get_styled_font_family(false, true, EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_ITALIC.into())
        );
        assert_eq!(
            get_styled_font_family(true, true, EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into())
        );
    }

    #[test]
    fn test_styled_font_id() {
        let font_id = styled_font_id(16.0, true, false, EditorFont::Inter);
        assert_eq!(font_id.size, 16.0);
        assert_eq!(font_id.family, FontFamily::Name(FONT_INTER_BOLD.into()));
    }
}
