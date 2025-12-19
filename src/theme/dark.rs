//! Dark Theme Configuration
//!
//! This module provides the dark theme for Ferrite.
//! It converts the `ThemeColors::dark()` palette into egui's `Visuals`
//! for consistent UI styling.

// Allow dead code - exports are available for future use
#![allow(dead_code)]

//! # Design Principles
//!
//! - Reduced eye strain for low-light environments
//! - Sufficient contrast for readability without being harsh
//! - Modern, professional dark appearance
//! - Accessible color choices (WCAG AA compliant)

use eframe::egui::{self, Color32, Rounding, Stroke, Visuals};

use super::{ThemeColors, ThemeSpacing};

/// Create egui Visuals configured for the dark theme.
///
/// This converts our custom `ThemeColors::dark()` palette into egui's
/// native `Visuals` structure for consistent UI styling.
///
/// # Example
///
/// ```ignore
/// use crate::theme::dark::create_dark_visuals;
///
/// let ctx = &egui::Context::default();
/// ctx.set_visuals(create_dark_visuals());
/// ```
pub fn create_dark_visuals() -> Visuals {
    let colors = ThemeColors::dark();
    let spacing = ThemeSpacing::default();

    let mut visuals = Visuals::dark();

    // ─────────────────────────────────────────────────────────────────────────
    // Window & Panel Background
    // ─────────────────────────────────────────────────────────────────────────
    visuals.panel_fill = colors.base.background;
    visuals.window_fill = colors.base.background;
    visuals.extreme_bg_color = colors.base.background_tertiary;
    visuals.faint_bg_color = colors.base.background_secondary;
    visuals.code_bg_color = colors.editor.code_block_bg;

    // ─────────────────────────────────────────────────────────────────────────
    // Text Colors
    // ─────────────────────────────────────────────────────────────────────────
    visuals.override_text_color = None; // Let widgets decide
    visuals.warn_fg_color = colors.ui.warning;
    visuals.error_fg_color = colors.ui.error;
    visuals.hyperlink_color = colors.text.link;

    // ─────────────────────────────────────────────────────────────────────────
    // Selection
    // ─────────────────────────────────────────────────────────────────────────
    visuals.selection.bg_fill = colors.base.selected;
    visuals.selection.stroke = Stroke::new(1.0, colors.ui.accent);

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Styling (Noninteractive)
    // ─────────────────────────────────────────────────────────────────────────
    visuals.widgets.noninteractive.bg_fill = colors.base.background_secondary;
    visuals.widgets.noninteractive.weak_bg_fill = colors.base.background_tertiary;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors.base.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.text.primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Styling (Inactive/Default)
    // ─────────────────────────────────────────────────────────────────────────
    visuals.widgets.inactive.bg_fill = colors.base.background_secondary;
    visuals.widgets.inactive.weak_bg_fill = colors.base.background_tertiary;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors.base.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.text.secondary);
    visuals.widgets.inactive.rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Styling (Hovered)
    // ─────────────────────────────────────────────────────────────────────────
    visuals.widgets.hovered.bg_fill = colors.base.hover;
    visuals.widgets.hovered.weak_bg_fill = colors.base.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors.ui.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, colors.text.primary);
    visuals.widgets.hovered.rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Styling (Active/Pressed)
    // ─────────────────────────────────────────────────────────────────────────
    visuals.widgets.active.bg_fill = colors.ui.accent;
    visuals.widgets.active.weak_bg_fill = colors.base.selected;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors.ui.accent_hover);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Widget Styling (Open/Expanded)
    // ─────────────────────────────────────────────────────────────────────────
    visuals.widgets.open.bg_fill = colors.base.selected;
    visuals.widgets.open.weak_bg_fill = colors.base.selected;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors.ui.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors.text.primary);
    visuals.widgets.open.rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Window & Popup Styling
    // ─────────────────────────────────────────────────────────────────────────
    visuals.window_rounding = Rounding::same(spacing.md);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(80),
    };
    visuals.window_stroke = Stroke::new(1.0, colors.base.border);

    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 20.0,
        spread: 0.0,
        color: Color32::from_black_alpha(100),
    };

    visuals.menu_rounding = Rounding::same(spacing.sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Miscellaneous
    // ─────────────────────────────────────────────────────────────────────────
    visuals.resize_corner_size = 12.0;
    visuals.clip_rect_margin = 3.0;
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = true;
    visuals.striped = true;
    visuals.slider_trailing_fill = true;
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

    // Dark mode flag
    visuals.dark_mode = true;

    visuals
}

/// Get the dark theme colors.
///
/// This is a convenience re-export of `ThemeColors::dark()`.
pub fn colors() -> ThemeColors {
    ThemeColors::dark()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_visuals_is_dark_mode() {
        let visuals = create_dark_visuals();
        assert!(visuals.dark_mode);
    }

    #[test]
    fn test_dark_visuals_has_dark_background() {
        let visuals = create_dark_visuals();
        // Dark theme should have dark panel fill
        assert!(visuals.panel_fill.r() < 50);
        assert!(visuals.panel_fill.g() < 50);
        assert!(visuals.panel_fill.b() < 50);
    }

    #[test]
    fn test_dark_colors_available() {
        let colors = colors();
        assert!(colors.is_dark());
    }

    #[test]
    fn test_dark_visuals_selection_visible() {
        let visuals = create_dark_visuals();
        // Selection should be visually distinct
        assert_ne!(visuals.selection.bg_fill, visuals.panel_fill);
    }

    #[test]
    fn test_dark_visuals_text_contrast() {
        let visuals = create_dark_visuals();
        let colors = colors();

        // Text stroke should be light for contrast on dark background
        assert!(visuals.widgets.noninteractive.fg_stroke.color.r() > 150);

        // Verify we're using our theme colors
        assert_eq!(
            visuals.widgets.noninteractive.fg_stroke.color,
            colors.text.primary
        );
    }

    #[test]
    fn test_dark_visuals_shadows_more_pronounced() {
        let dark_visuals = create_dark_visuals();
        let light_visuals = super::super::light::create_light_visuals();

        // Dark theme should have more pronounced shadows for depth
        assert!(dark_visuals.window_shadow.color.a() > light_visuals.window_shadow.color.a());
    }
}
