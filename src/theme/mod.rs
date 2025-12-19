//! Theme System for Ferrite
//!
//! This module provides a comprehensive theming system that defines colors,
//! fonts, and spacing for consistent UI styling across the application.

// Allow dead code - this module has comprehensive theme utilities for future use
#![allow(dead_code)]

//! # Architecture
//!
//! The theme system is built around the `ThemeColors` struct which contains
//! all color definitions needed for the UI. The existing `Theme` enum in
//! `config::settings` (Light/Dark/System) is used to select which palette
//! to use at runtime.
//!
//! # Usage
//!
//! ```ignore
//! use crate::theme::{ThemeColors, ThemeSpacing, ThemeFonts};
//! use crate::config::Theme;
//!
//! // Get colors for the current theme
//! let colors = ThemeColors::from_theme(Theme::Dark, &ctx.style().visuals);
//!
//! // Use in egui
//! ui.label(RichText::new("Hello").color(colors.text.primary));
//!
//! // Apply theme to egui context
//! let visuals = colors.to_visuals();
//! ctx.set_visuals(visuals);
//! ```
//!
//! # Theme Files
//!
//! - `light.rs` - Light theme configuration and egui Visuals
//! - `dark.rs` - Dark theme configuration and egui Visuals
//! - `colors.rs` - Color constants and utilities
//!
//! # Color Categories
//!
//! - **Base colors**: Background, foreground, borders
//! - **Text colors**: Primary, secondary, muted, link, code
//! - **Editor colors**: Headings, blockquotes, code blocks, lists
//! - **Syntax colors**: Keywords, strings, comments, etc.
//! - **UI colors**: Accent, success, warning, error

pub mod dark;
pub mod light;
pub mod manager;

pub use manager::ThemeManager;

use eframe::egui::Color32;

// ─────────────────────────────────────────────────────────────────────────────
// Theme Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Comprehensive theme colors for the entire application.
///
/// This struct consolidates all color definitions needed for consistent
/// UI theming, replacing the fragmented `EditorColors` and `WidgetColors`
/// with a unified system.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeColors {
    /// Base UI colors (backgrounds, borders)
    pub base: BaseColors,
    /// Text colors for various contexts
    pub text: TextColors,
    /// Colors for the markdown editor
    pub editor: EditorThemeColors,
    /// Syntax highlighting colors for code blocks
    pub syntax: SyntaxColors,
    /// UI feedback colors (success, warning, error)
    pub ui: UiColors,
}

impl ThemeColors {
    /// Create theme colors for the given theme variant.
    ///
    /// This is the primary way to get themed colors. It automatically
    /// selects the appropriate palette based on the theme setting.
    pub fn from_theme(theme: crate::config::Theme, visuals: &eframe::egui::Visuals) -> Self {
        match theme {
            crate::config::Theme::Dark => Self::dark(),
            crate::config::Theme::Light => Self::light(),
            crate::config::Theme::System => {
                if visuals.dark_mode {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }

    /// Get the light theme colors.
    pub fn light() -> Self {
        Self {
            base: BaseColors::light(),
            text: TextColors::light(),
            editor: EditorThemeColors::light(),
            syntax: SyntaxColors::light(),
            ui: UiColors::light(),
        }
    }

    /// Get the dark theme colors.
    pub fn dark() -> Self {
        Self {
            base: BaseColors::dark(),
            text: TextColors::dark(),
            editor: EditorThemeColors::dark(),
            syntax: SyntaxColors::dark(),
            ui: UiColors::dark(),
        }
    }

    /// Check if this is a dark theme (useful for conditional styling).
    pub fn is_dark(&self) -> bool {
        // Dark themes have darker backgrounds
        self.base.background.r() < 128
    }

    /// Convert theme colors to egui Visuals for UI styling.
    ///
    /// This is the primary method to apply the theme to egui. It creates
    /// a complete `Visuals` struct configured with the theme's colors.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::theme::ThemeColors;
    ///
    /// let colors = ThemeColors::dark();
    /// let visuals = colors.to_visuals();
    /// ctx.set_visuals(visuals);
    /// ```
    pub fn to_visuals(&self) -> eframe::egui::Visuals {
        if self.is_dark() {
            dark::create_dark_visuals()
        } else {
            light::create_light_visuals()
        }
    }

    /// Create visuals for the given theme variant.
    ///
    /// Convenience method that combines `from_theme` and `to_visuals`.
    pub fn visuals_for_theme(
        theme: crate::config::Theme,
        system_visuals: &eframe::egui::Visuals,
    ) -> eframe::egui::Visuals {
        Self::from_theme(theme, system_visuals).to_visuals()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Base Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Base UI colors for backgrounds and borders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseColors {
    /// Primary background color
    pub background: Color32,
    /// Secondary/elevated background (panels, cards)
    pub background_secondary: Color32,
    /// Tertiary background (inputs, code blocks)
    pub background_tertiary: Color32,
    /// Primary border color
    pub border: Color32,
    /// Subtle border color (dividers)
    pub border_subtle: Color32,
    /// Hover state background
    pub hover: Color32,
    /// Selected/active state background
    pub selected: Color32,
}

impl BaseColors {
    /// Light theme base colors.
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(255, 255, 255),
            background_secondary: Color32::from_rgb(250, 250, 250),
            background_tertiary: Color32::from_rgb(245, 245, 245),
            border: Color32::from_rgb(200, 200, 200),
            border_subtle: Color32::from_rgb(230, 230, 230),
            hover: Color32::from_rgb(240, 240, 240),
            selected: Color32::from_rgb(230, 240, 255),
        }
    }

    /// Dark theme base colors.
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(30, 30, 30),
            background_secondary: Color32::from_rgb(37, 37, 37),
            background_tertiary: Color32::from_rgb(45, 45, 45),
            border: Color32::from_rgb(60, 60, 60),
            border_subtle: Color32::from_rgb(50, 50, 50),
            hover: Color32::from_rgb(50, 50, 50),
            selected: Color32::from_rgb(40, 60, 80),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Text Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Text colors for various contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextColors {
    /// Primary text color (main content)
    pub primary: Color32,
    /// Secondary text color (descriptions, labels)
    pub secondary: Color32,
    /// Muted text color (hints, placeholders)
    pub muted: Color32,
    /// Disabled text color
    pub disabled: Color32,
    /// Link text color
    pub link: Color32,
    /// Code text color (inline code)
    pub code: Color32,
}

impl TextColors {
    /// Light theme text colors.
    pub fn light() -> Self {
        Self {
            primary: Color32::from_rgb(30, 30, 30),
            secondary: Color32::from_rgb(80, 80, 80),
            muted: Color32::from_rgb(120, 120, 120),
            disabled: Color32::from_rgb(160, 160, 160),
            link: Color32::from_rgb(0, 100, 180),
            code: Color32::from_rgb(80, 80, 80),
        }
    }

    /// Dark theme text colors.
    pub fn dark() -> Self {
        Self {
            primary: Color32::from_rgb(220, 220, 220),
            secondary: Color32::from_rgb(180, 180, 180),
            muted: Color32::from_rgb(140, 140, 140),
            disabled: Color32::from_rgb(100, 100, 100),
            link: Color32::from_rgb(100, 180, 255),
            code: Color32::from_rgb(200, 200, 150),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor Theme Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors specific to the markdown editor.
///
/// These colors are used for rendering markdown elements in both
/// raw and WYSIWYG (rendered) editing modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorThemeColors {
    /// Heading text color (H1-H6)
    pub heading: Color32,
    /// Block quote border color
    pub blockquote_border: Color32,
    /// Block quote text color
    pub blockquote_text: Color32,
    /// Code block background color
    pub code_block_bg: Color32,
    /// Code block border color
    pub code_block_border: Color32,
    /// Horizontal rule color
    pub horizontal_rule: Color32,
    /// List marker color (bullets, numbers)
    pub list_marker: Color32,
    /// Task checkbox color
    pub checkbox: Color32,
    /// Table border color
    pub table_border: Color32,
    /// Table header background
    pub table_header_bg: Color32,
}

impl EditorThemeColors {
    /// Light theme editor colors.
    pub fn light() -> Self {
        Self {
            heading: Color32::from_rgb(0, 100, 180),
            blockquote_border: Color32::from_rgb(200, 200, 200),
            blockquote_text: Color32::from_rgb(100, 100, 100),
            code_block_bg: Color32::from_rgb(233, 236, 239),
            code_block_border: Color32::from_rgb(195, 202, 210),
            horizontal_rule: Color32::from_rgb(200, 200, 200),
            list_marker: Color32::from_rgb(100, 100, 100),
            checkbox: Color32::from_rgb(0, 100, 180),
            table_border: Color32::from_rgb(200, 205, 210),
            table_header_bg: Color32::from_rgb(240, 242, 245),
        }
    }

    /// Dark theme editor colors.
    pub fn dark() -> Self {
        Self {
            heading: Color32::from_rgb(100, 180, 255),
            blockquote_border: Color32::from_rgb(80, 80, 80),
            blockquote_text: Color32::from_rgb(180, 180, 180),
            code_block_bg: Color32::from_rgb(35, 39, 46),
            code_block_border: Color32::from_rgb(55, 60, 68),
            horizontal_rule: Color32::from_rgb(80, 80, 80),
            list_marker: Color32::from_rgb(150, 150, 150),
            checkbox: Color32::from_rgb(100, 180, 255),
            table_border: Color32::from_rgb(60, 65, 75),
            table_header_bg: Color32::from_rgb(45, 50, 60),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Syntax Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for syntax highlighting in code blocks.
///
/// These colors are used when syntax highlighting is not available
/// or as fallback colors. The full syntax highlighting uses syntect
/// themes which have their own color definitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntaxColors {
    /// Keyword color (if, else, fn, let, etc.)
    pub keyword: Color32,
    /// String literal color
    pub string: Color32,
    /// Number literal color
    pub number: Color32,
    /// Comment color
    pub comment: Color32,
    /// Function name color
    pub function: Color32,
    /// Type/class name color
    pub type_name: Color32,
    /// Variable name color
    pub variable: Color32,
    /// Operator color (+, -, =, etc.)
    pub operator: Color32,
    /// Punctuation color (brackets, semicolons)
    pub punctuation: Color32,
}

impl SyntaxColors {
    /// Light theme syntax colors.
    pub fn light() -> Self {
        Self {
            keyword: Color32::from_rgb(175, 0, 175),       // Purple
            string: Color32::from_rgb(0, 128, 0),          // Green
            number: Color32::from_rgb(0, 128, 128),        // Teal
            comment: Color32::from_rgb(128, 128, 128),     // Gray
            function: Color32::from_rgb(0, 0, 175),        // Blue
            type_name: Color32::from_rgb(0, 100, 150),     // Dark cyan
            variable: Color32::from_rgb(50, 50, 50),       // Dark gray
            operator: Color32::from_rgb(80, 80, 80),       // Gray
            punctuation: Color32::from_rgb(100, 100, 100), // Medium gray
        }
    }

    /// Dark theme syntax colors.
    pub fn dark() -> Self {
        Self {
            keyword: Color32::from_rgb(198, 120, 221),   // Light purple
            string: Color32::from_rgb(152, 195, 121),    // Light green
            number: Color32::from_rgb(209, 154, 102),    // Orange
            comment: Color32::from_rgb(92, 99, 112),     // Gray
            function: Color32::from_rgb(97, 175, 239),   // Light blue
            type_name: Color32::from_rgb(229, 192, 123), // Yellow
            variable: Color32::from_rgb(224, 108, 117),  // Red/pink
            operator: Color32::from_rgb(171, 178, 191),  // Light gray
            punctuation: Color32::from_rgb(150, 150, 150), // Gray
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for UI feedback and interactive elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiColors {
    /// Primary accent color (buttons, active elements)
    pub accent: Color32,
    /// Accent color for hover state
    pub accent_hover: Color32,
    /// Success color (confirmations, positive actions)
    pub success: Color32,
    /// Warning color (cautions, alerts)
    pub warning: Color32,
    /// Error color (errors, destructive actions)
    pub error: Color32,
    /// Info color (informational messages)
    pub info: Color32,
}

impl UiColors {
    /// Light theme UI colors.
    pub fn light() -> Self {
        Self {
            accent: Color32::from_rgb(0, 120, 212),
            accent_hover: Color32::from_rgb(0, 100, 180),
            success: Color32::from_rgb(40, 167, 69),
            warning: Color32::from_rgb(255, 193, 7),
            error: Color32::from_rgb(220, 53, 69),
            info: Color32::from_rgb(23, 162, 184),
        }
    }

    /// Dark theme UI colors.
    pub fn dark() -> Self {
        Self {
            accent: Color32::from_rgb(100, 180, 255),
            accent_hover: Color32::from_rgb(130, 200, 255),
            success: Color32::from_rgb(75, 210, 100),
            warning: Color32::from_rgb(255, 210, 50),
            error: Color32::from_rgb(255, 100, 100),
            info: Color32::from_rgb(80, 200, 220),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme Spacing
// ─────────────────────────────────────────────────────────────────────────────

/// Spacing values for consistent layout.
///
/// These values define the standard spacing used throughout the UI
/// to maintain visual consistency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSpacing {
    /// Extra small spacing (2px)
    pub xs: f32,
    /// Small spacing (4px)
    pub sm: f32,
    /// Medium spacing (8px)
    pub md: f32,
    /// Large spacing (16px)
    pub lg: f32,
    /// Extra large spacing (24px)
    pub xl: f32,
    /// Double extra large spacing (32px)
    pub xxl: f32,
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            xl: 24.0,
            xxl: 32.0,
        }
    }
}

impl ThemeSpacing {
    /// Create the default spacing values.
    pub fn new() -> Self {
        Self::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_colors_light() {
        let colors = ThemeColors::light();

        // Light theme should have light background
        assert!(colors.base.background.r() > 200);
        assert!(!colors.is_dark());
    }

    #[test]
    fn test_theme_colors_dark() {
        let colors = ThemeColors::dark();

        // Dark theme should have dark background
        assert!(colors.base.background.r() < 50);
        assert!(colors.is_dark());
    }

    #[test]
    fn test_theme_colors_from_theme() {
        let dark_colors =
            ThemeColors::from_theme(crate::config::Theme::Dark, &eframe::egui::Visuals::dark());
        assert!(dark_colors.is_dark());

        let light_colors =
            ThemeColors::from_theme(crate::config::Theme::Light, &eframe::egui::Visuals::light());
        assert!(!light_colors.is_dark());
    }

    #[test]
    fn test_base_colors_light() {
        let colors = BaseColors::light();
        assert!(colors.background.r() > 200);
        assert!(colors.background_secondary.r() > 200);
    }

    #[test]
    fn test_base_colors_dark() {
        let colors = BaseColors::dark();
        assert!(colors.background.r() < 50);
        assert!(colors.background_secondary.r() < 50);
    }

    #[test]
    fn test_text_colors_contrast() {
        // Light theme: dark text on light background
        let light = TextColors::light();
        assert!(light.primary.r() < 50);

        // Dark theme: light text on dark background
        let dark = TextColors::dark();
        assert!(dark.primary.r() > 200);
    }

    #[test]
    fn test_editor_colors_heading_distinct() {
        // Headings should be visually distinct
        let light = EditorThemeColors::light();
        let dark = EditorThemeColors::dark();

        // Light theme heading should be different from standard text
        assert_ne!(light.heading, TextColors::light().primary);

        // Dark theme heading should be different from standard text
        assert_ne!(dark.heading, TextColors::dark().primary);
    }

    #[test]
    fn test_syntax_colors_variety() {
        let light = SyntaxColors::light();

        // All syntax colors should be distinct for readability
        assert_ne!(light.keyword, light.string);
        assert_ne!(light.string, light.comment);
        assert_ne!(light.function, light.type_name);
    }

    #[test]
    fn test_ui_colors_feedback() {
        let colors = UiColors::light();

        // Success should be greenish
        assert!(colors.success.g() > colors.success.r());

        // Error should be reddish
        assert!(colors.error.r() > colors.error.g());

        // Warning should be yellowish
        assert!(colors.warning.r() > 200 && colors.warning.g() > 150);
    }

    #[test]
    fn test_spacing_default() {
        let spacing = ThemeSpacing::default();

        assert_eq!(spacing.xs, 2.0);
        assert_eq!(spacing.sm, 4.0);
        assert_eq!(spacing.md, 8.0);
        assert_eq!(spacing.lg, 16.0);
        assert_eq!(spacing.xl, 24.0);
        assert_eq!(spacing.xxl, 32.0);
    }

    #[test]
    fn test_theme_colors_to_visuals_light() {
        let colors = ThemeColors::light();
        let visuals = colors.to_visuals();

        // Light theme visuals should not be dark mode
        assert!(!visuals.dark_mode);

        // Panel fill should match our theme's background
        assert_eq!(visuals.panel_fill, colors.base.background);
    }

    #[test]
    fn test_theme_colors_to_visuals_dark() {
        let colors = ThemeColors::dark();
        let visuals = colors.to_visuals();

        // Dark theme visuals should be dark mode
        assert!(visuals.dark_mode);

        // Panel fill should match our theme's background
        assert_eq!(visuals.panel_fill, colors.base.background);
    }

    #[test]
    fn test_visuals_for_theme_light() {
        let visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::Light,
            &eframe::egui::Visuals::light(),
        );
        assert!(!visuals.dark_mode);
    }

    #[test]
    fn test_visuals_for_theme_dark() {
        let visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::Dark,
            &eframe::egui::Visuals::dark(),
        );
        assert!(visuals.dark_mode);
    }

    #[test]
    fn test_visuals_for_theme_system() {
        // System theme follows the provided visuals
        let dark_visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::System,
            &eframe::egui::Visuals::dark(),
        );
        assert!(dark_visuals.dark_mode);

        let light_visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::System,
            &eframe::egui::Visuals::light(),
        );
        assert!(!light_visuals.dark_mode);
    }
}
