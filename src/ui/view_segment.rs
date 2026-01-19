//! View Mode Segmented Control for Ferrite
//!
//! This module implements a modern segmented control widget for switching
//! between view modes (Raw, Split, Rendered) in the title bar.
//!
//! Note: This is an alternative view control design that isn't currently
//! integrated into the main UI. Kept for potential future use.
#![allow(dead_code)]

use crate::config::ViewMode;
use crate::app::modifier_symbol;
use crate::state::FileType;
use eframe::egui::{self, Color32, Response, RichText, Sense, Vec2};

/// Height of the segmented control.
const SEGMENT_HEIGHT: f32 = 22.0;

/// Width of each segment button.
const SEGMENT_WIDTH: f32 = 32.0;

/// Corner rounding for the control.
const CORNER_ROUNDING: f32 = 4.0;

/// Actions that can be triggered from the view mode segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewSegmentAction {
    /// Switch to Raw mode
    SetRaw,
    /// Switch to Split mode
    SetSplit,
    /// Switch to Rendered mode
    SetRendered,
}

/// View mode segmented control widget.
///
/// A three-button toggle control for switching between Raw, Split, and Rendered views.
/// File-type aware: disables Split for non-markdown structured files.
#[derive(Debug, Clone, Default)]
pub struct ViewModeSegment;

impl ViewModeSegment {
    /// Create a new view mode segment widget.
    pub fn new() -> Self {
        Self
    }

    /// Show the segmented control and return any triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `current_mode` - The currently active view mode
    /// * `file_type` - The current file type (affects which modes are available)
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Optional action if a segment was clicked
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        current_mode: ViewMode,
        file_type: FileType,
        is_dark: bool,
    ) -> Option<ViewSegmentAction> {
        let mut action: Option<ViewSegmentAction> = None;

        // Determine which modes are available based on file type
        let split_available = file_type.is_markdown();
        let rendered_available = file_type.is_markdown() || file_type.is_structured();

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(50, 50, 50)
        } else {
            Color32::from_rgb(220, 220, 220)
        };

        let selected_bg = if is_dark {
            Color32::from_rgb(80, 100, 140) // Blue-ish for dark mode
        } else {
            Color32::from_rgb(180, 200, 230) // Light blue for light mode
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(65, 65, 65)
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let disabled_text = if is_dark {
            Color32::from_rgb(100, 100, 100)
        } else {
            Color32::from_rgb(160, 160, 160)
        };

        // Calculate total width (3 segments with 1px borders between)
        let total_width = SEGMENT_WIDTH * 3.0 + 2.0; // 2px for internal borders
        let size = Vec2::new(total_width, SEGMENT_HEIGHT);

        // Allocate space for the entire control
        let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());

        // Draw background
        ui.painter()
            .rect_filled(rect, CORNER_ROUNDING, bg_color);

        // Define segment positions
        let segments = [
            (ViewMode::Raw, "📝", "Raw Editor", ViewSegmentAction::SetRaw, true),
            (ViewMode::Split, "||", "Split View", ViewSegmentAction::SetSplit, split_available),
            (ViewMode::Rendered, "👁", "Rendered View", ViewSegmentAction::SetRendered, rendered_available),
        ];

        let mut x_offset = rect.min.x;

        for (i, (mode, icon, tooltip, segment_action, enabled)) in segments.iter().enumerate() {
            let segment_rect = egui::Rect::from_min_size(
                egui::pos2(x_offset, rect.min.y),
                Vec2::new(SEGMENT_WIDTH, SEGMENT_HEIGHT),
            );

            let is_selected = current_mode == *mode;

            // Create clickable area
            let segment_response = ui.allocate_rect(segment_rect, Sense::click());

            // Draw segment background
            let rounding = match i {
                0 => egui::Rounding {
                    nw: CORNER_ROUNDING,
                    sw: CORNER_ROUNDING,
                    ne: 0.0,
                    se: 0.0,
                },
                2 => egui::Rounding {
                    nw: 0.0,
                    sw: 0.0,
                    ne: CORNER_ROUNDING,
                    se: CORNER_ROUNDING,
                },
                _ => egui::Rounding::ZERO,
            };

            if is_selected {
                ui.painter().rect_filled(segment_rect, rounding, selected_bg);
            } else if segment_response.hovered() && *enabled {
                ui.painter().rect_filled(segment_rect, rounding, hover_bg);
            }

            // Draw icon/text
            let icon_color = if *enabled { text_color } else { disabled_text };
            let font_size = if *icon == "||" { 10.0 } else { 12.0 }; // Smaller for ASCII split icon

            ui.painter().text(
                segment_rect.center(),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(font_size),
                icon_color,
            );

            // Handle click
            if segment_response.clicked() && *enabled && !is_selected {
                action = Some(*segment_action);
            }

            // Tooltip
            let tooltip_text = if *enabled {
                format!("{} ({}+E to cycle)", tooltip, modifier_symbol())
            } else {
                format!("{} (not available for this file type)", tooltip)
            };
            segment_response.on_hover_text(tooltip_text);

            // Draw separator between segments (except after last)
            if i < 2 {
                let sep_x = x_offset + SEGMENT_WIDTH;
                ui.painter().line_segment(
                    [
                        egui::pos2(sep_x, rect.min.y + 3.0),
                        egui::pos2(sep_x, rect.max.y - 3.0),
                    ],
                    egui::Stroke::new(1.0, if is_dark {
                        Color32::from_rgb(70, 70, 70)
                    } else {
                        Color32::from_rgb(190, 190, 190)
                    }),
                );
            }

            x_offset += SEGMENT_WIDTH + 1.0; // 1px for separator
        }

        action
    }
}

/// Title bar button for quick toggles (Settings, Zen Mode, etc.)
///
/// A compact button styled for the title bar area.
pub struct TitleBarButton;

impl TitleBarButton {
    /// Show a title bar toggle button.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `icon` - The icon/emoji to display
    /// * `tooltip` - Hover tooltip text
    /// * `is_active` - Whether the button is in active/on state
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// The button response for click detection
    pub fn show(
        ui: &mut egui::Ui,
        icon: &str,
        tooltip: &str,
        is_active: bool,
        is_dark: bool,
    ) -> Response {
        let size = Vec2::new(28.0, 24.0); // Slightly taller for better alignment
        
        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

        let active_bg = if is_dark {
            Color32::from_rgb(60, 90, 60) // Green-ish for active
        } else {
            Color32::from_rgb(200, 230, 200)
        };

        let btn = ui.add(
            egui::Button::new(RichText::new(" ").size(14.0)) // Match icon size
                .frame(false)
                .min_size(size),
        );

        // Draw background on hover or if active
        if is_active {
            ui.painter()
                .rect_filled(btn.rect, egui::Rounding::same(3.0), active_bg);
        } else if btn.hovered() {
            ui.painter()
                .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);
        }

        // Draw icon centered
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            text_color,
        );

        btn.on_hover_text(tooltip)
    }

    /// Show an auto-save indicator button with special styling.
    ///
    /// Green tint when enabled, muted when disabled.
    pub fn show_auto_save(
        ui: &mut egui::Ui,
        enabled: bool,
        is_dark: bool,
    ) -> Response {
        let size = Vec2::new(28.0, 24.0); // Match other title bar buttons
        
        let icon = if enabled { "⏱" } else { "⏸" };
        let tooltip = if enabled {
            "Auto-Save: ON (click to disable)"
        } else {
            "Auto-Save: OFF (click to enable)"
        };

        // Green tint for enabled, muted for disabled
        let text_color = if enabled {
            if is_dark {
                Color32::from_rgb(100, 200, 100) // Green
            } else {
                Color32::from_rgb(40, 140, 40)
            }
        } else if is_dark {
            Color32::from_rgb(120, 120, 120) // Muted
        } else {
            Color32::from_rgb(140, 140, 140)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

        let btn = ui.add(
            egui::Button::new(RichText::new(" ").size(14.0)) // Match icon size
                .frame(false)
                .min_size(size),
        );

        if btn.hovered() {
            ui.painter()
                .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);
        }

        // Draw icon centered with appropriate color
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            text_color,
        );

        btn.on_hover_text(tooltip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_segment_new() {
        let segment = ViewModeSegment::new();
        // Just verify it can be created
        let _ = segment;
    }

    #[test]
    fn test_view_segment_action_equality() {
        assert_eq!(ViewSegmentAction::SetRaw, ViewSegmentAction::SetRaw);
        assert_ne!(ViewSegmentAction::SetRaw, ViewSegmentAction::SetSplit);
    }

    #[test]
    fn test_view_mode_segment_default() {
        let segment = ViewModeSegment::default();
        let _ = segment;
    }
}
