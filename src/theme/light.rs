//! Light Theme Configuration
//!
//! This module provides the light theme for Ferrite.
//! It converts the `ThemeColors::light()` palette into egui's `Visuals`
//! for consistent UI styling.

// Allow dead code - exports are available for future use
#![allow(dead_code)]

//! # Design Principles
//!
//! - High contrast for readability
//! - Soft backgrounds to reduce eye strain
//! - Professional appearance suitable for extended use
//! - Accessible color choices (WCAG AA compliant)

use eframe::egui::{self, Color32, Rounding, Stroke, Visuals};

use super::{ThemeColors, ThemeSpacing};

/// Create egui Visuals configured for the light theme.
///
/// This converts our custom `ThemeColors::light()` palette into egui's
/// native `Visuals` structure for consistent UI styling.
///
/// # Example
///
/// ```ignore
/// use crate::theme::light::create_light_visuals;
///
/// let ctx = &egui::Context::default();
/// ctx.set_visuals(create_light_visuals());
/// ```
pub fn create_light_visuals() -> Visuals {
    let colors = ThemeColors::light();
    let spacing = ThemeSpacing::default();

    let mut visuals = Visuals::light();

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
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: Color32::from_black_alpha(25),
    };
    visuals.window_stroke = Stroke::new(1.0, colors.base.border);

    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(30),
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
    visuals.dark_mode = false;

    visuals
}

/// Get the light theme colors.
///
/// This is a convenience re-export of `ThemeColors::light()`.
pub fn colors() -> ThemeColors {
    ThemeColors::light()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_visuals_is_light_mode() {
        let visuals = create_light_visuals();
        assert!(!visuals.dark_mode);
    }

    #[test]
    fn test_light_visuals_has_light_background() {
        let visuals = create_light_visuals();
        // Light theme should have bright panel fill
        assert!(visuals.panel_fill.r() > 200);
        assert!(visuals.panel_fill.g() > 200);
        assert!(visuals.panel_fill.b() > 200);
    }

    #[test]
    fn test_light_colors_available() {
        let colors = colors();
        assert!(!colors.is_dark());
    }

    #[test]
    fn test_light_visuals_selection_visible() {
        let visuals = create_light_visuals();
        // Selection should be visually distinct
        assert_ne!(visuals.selection.bg_fill, visuals.panel_fill);
    }

    #[test]
    fn test_light_visuals_text_contrast() {
        let visuals = create_light_visuals();
        let colors = colors();

        // Text stroke should be dark for contrast on light background
        assert!(visuals.widgets.noninteractive.fg_stroke.color.r() < 100);

        // Verify we're using our theme colors
        assert_eq!(
            visuals.widgets.noninteractive.fg_stroke.color,
            colors.text.primary
        );
    }
}
