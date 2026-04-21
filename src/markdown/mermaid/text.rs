//! Text measurement utilities for Mermaid diagram rendering.
//!
//! This module provides backend-agnostic text measurement capabilities,
//! supporting both egui-based rendering and fallback estimation for tests.

use egui::{Color32, FontId, Ui};

/// Result of measuring text dimensions.
#[derive(Debug, Clone, Copy)]
pub struct TextSize {
    pub width: f32,
    pub height: f32,
}

impl TextSize {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Trait for measuring text dimensions.
///
/// This enables backend-agnostic text measurement, supporting future
/// SVG/PNG backends when extracting to a standalone crate.
pub trait TextMeasurer {
    /// Measure the dimensions of text with the given font size.
    fn measure(&self, text: &str, font_size: f32) -> TextSize;

    /// Get the row height for a font at the given size.
    fn row_height(&self, font_size: f32) -> f32;

    /// Measure text with wrapping at max_width. Returns size of wrapped text.
    fn measure_wrapped(&self, text: &str, font_size: f32, max_width: f32) -> TextSize {
        let single_line = self.measure(text, font_size);
        if single_line.width <= max_width || max_width <= 0.0 {
            return single_line;
        }

        // Estimate wrapped height based on number of lines needed
        let lines_needed = (single_line.width / max_width).ceil();
        TextSize::new(max_width, single_line.height * lines_needed)
    }

    /// Truncate text to fit within max_width, adding ellipsis if needed.
    fn truncate_with_ellipsis(&self, text: &str, font_size: f32, max_width: f32) -> String {
        let size = self.measure(text, font_size);
        if size.width <= max_width || max_width <= 0.0 {
            return text.to_string();
        }

        let ellipsis = "…";
        let ellipsis_width = self.measure(ellipsis, font_size).width;
        let available_width = max_width - ellipsis_width;

        if available_width <= 0.0 {
            return ellipsis.to_string();
        }

        // Binary search for the right truncation point
        let chars: Vec<char> = text.chars().collect();
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let truncated: String = chars[..mid].iter().collect();
            let width = self.measure(&truncated, font_size).width;

            if width <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            let truncated: String = chars[..low].iter().collect();
            format!("{}{}", truncated, ellipsis)
        }
    }
}

/// Text measurer implementation using egui's font system.
pub struct EguiTextMeasurer<'a> {
    ui: &'a Ui,
}

impl<'a> EguiTextMeasurer<'a> {
    pub fn new(ui: &'a Ui) -> Self {
        Self { ui }
    }
}

impl TextMeasurer for EguiTextMeasurer<'_> {
    fn measure(&self, text: &str, font_size: f32) -> TextSize {
        let font_id = FontId::proportional(font_size);
        let galley = self
            .ui
            .fonts(|fonts| fonts.layout_no_wrap(text.to_string(), font_id, Color32::PLACEHOLDER));
        TextSize::new(galley.rect.width(), galley.rect.height())
    }

    fn row_height(&self, font_size: f32) -> f32 {
        let font_id = FontId::proportional(font_size);
        self.ui.fonts(|fonts| fonts.row_height(&font_id))
    }

    fn measure_wrapped(&self, text: &str, font_size: f32, max_width: f32) -> TextSize {
        if max_width <= 0.0 {
            return self.measure(text, font_size);
        }

        let font_id = FontId::proportional(font_size);
        let galley = self.ui.fonts(|fonts| {
            let layout_job = egui::text::LayoutJob::simple(
                text.to_string(),
                font_id,
                Color32::PLACEHOLDER,
                max_width,
            );
            fonts.layout_job(layout_job)
        });
        TextSize::new(galley.rect.width(), galley.rect.height())
    }
}

/// Fallback text measurer using character-based estimation.
/// Used when egui context is not available (e.g., in tests).
#[derive(Debug, Clone, Copy, Default)]
pub struct EstimatedTextMeasurer {
    /// Approximate width per character as a fraction of font size.
    char_width_factor: f32,
}

impl EstimatedTextMeasurer {
    pub fn new() -> Self {
        Self {
            char_width_factor: 0.55, // Slightly better than the old 0.6
        }
    }
}

impl TextMeasurer for EstimatedTextMeasurer {
    fn measure(&self, text: &str, font_size: f32) -> TextSize {
        let width = text.len() as f32 * font_size * self.char_width_factor;
        TextSize::new(width, font_size)
    }

    fn row_height(&self, font_size: f32) -> f32 {
        font_size * 1.2 // Standard line height
    }
}
