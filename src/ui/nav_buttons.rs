//! Document Navigation Buttons
//!
//! This module provides subtle navigation buttons for jumping to the top, middle,
//! or bottom of a document. The buttons appear as a floating overlay in the
//! top-right corner of the editor area.
//!
//! # Usage
//!
//! ```ignore
//! let action = render_nav_buttons(ui, editor_rect, is_dark_mode);
//! match action {
//!     NavAction::Top => { /* scroll to top */ }
//!     NavAction::Middle => { /* scroll to middle */ }
//!     NavAction::Bottom => { /* scroll to bottom */ }
//!     NavAction::None => {}
//! }
//! ```

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Ui, Vec2};

/// Action requested by navigation button click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    /// No action (no button clicked)
    None,
    /// Jump to top of document
    Top,
    /// Jump to middle of document
    Middle,
    /// Jump to bottom of document
    Bottom,
}

/// Button size in pixels.
const BUTTON_SIZE: f32 = 24.0;

/// Spacing between buttons.
const BUTTON_SPACING: f32 = 2.0;

/// Margin from the editor edge.
const MARGIN: f32 = 8.0;

/// Alpha value when not hovered (semi-transparent).
const IDLE_ALPHA: u8 = 100;

/// Alpha value when hovered.
const HOVER_ALPHA: u8 = 220;

/// Renders navigation buttons overlay and returns any requested action.
///
/// The buttons appear in the top-right corner of the given `editor_rect`.
/// They are semi-transparent when idle and become more visible on hover.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `editor_rect` - The rectangle of the editor area (buttons positioned relative to this)
/// * `is_dark_mode` - Whether dark mode is active (affects button colors)
///
/// # Returns
/// A `NavAction` indicating which button was clicked, or `NavAction::None`.
pub fn render_nav_buttons(ui: &mut Ui, editor_rect: Rect, is_dark_mode: bool) -> NavAction {
    let mut action = NavAction::None;

    // Calculate button container position (top-right with margin)
    let container_pos = Pos2::new(
        editor_rect.max.x - BUTTON_SIZE - MARGIN,
        editor_rect.min.y + MARGIN,
    );

    // Check if mouse is near the button area to show/hide
    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    let container_rect = Rect::from_min_size(
        container_pos,
        Vec2::new(BUTTON_SIZE, BUTTON_SIZE * 3.0 + BUTTON_SPACING * 2.0),
    );

    // Expand the hover detection area slightly for better UX
    let hover_area = container_rect.expand(20.0);
    let is_near = mouse_pos.map_or(false, |pos| hover_area.contains(pos));

    // Only render buttons if mouse is near the area
    // This prevents visual clutter when not navigating
    if !is_near {
        return NavAction::None;
    }

    // Create an Area for the floating overlay
    let layer_id = egui::LayerId::new(egui::Order::Foreground, ui.id().with("nav_buttons"));

    ui.with_layer_id(layer_id, |ui| {
        // Position the buttons vertically
        // Using simple arrow characters that render in most fonts
        let button_positions = [
            (
                container_pos,
                "▲",
                "Jump to top (Ctrl+Home)",
                NavAction::Top,
            ),
            (
                Pos2::new(
                    container_pos.x,
                    container_pos.y + BUTTON_SIZE + BUTTON_SPACING,
                ),
                "●",
                "Jump to middle",
                NavAction::Middle,
            ),
            (
                Pos2::new(
                    container_pos.x,
                    container_pos.y + (BUTTON_SIZE + BUTTON_SPACING) * 2.0,
                ),
                "▼",
                "Jump to bottom (Ctrl+End)",
                NavAction::Bottom,
            ),
        ];

        for (pos, icon, tooltip, button_action) in button_positions {
            let button_rect = Rect::from_min_size(pos, Vec2::splat(BUTTON_SIZE));

            // Check if this specific button is hovered
            let button_hovered = mouse_pos.map_or(false, |mp| button_rect.contains(mp));

            // Determine colors based on hover state and theme
            let (bg_color, text_color) = get_button_colors(is_dark_mode, button_hovered);

            // Draw button background
            ui.painter().rect_filled(button_rect, 4.0, bg_color);

            // Draw button border on hover
            if button_hovered {
                let border_color = if is_dark_mode {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 60)
                } else {
                    Color32::from_rgba_unmultiplied(0, 0, 0, 40)
                };
                ui.painter()
                    .rect_stroke(button_rect, 4.0, egui::Stroke::new(1.0, border_color));
            }

            // Draw icon
            let text = RichText::new(icon).size(14.0).color(text_color);
            let galley = ui.painter().layout_no_wrap(
                text.text().to_string(),
                egui::FontId::proportional(14.0),
                text_color,
            );
            let text_pos = Pos2::new(
                button_rect.center().x - galley.size().x / 2.0,
                button_rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, text_color);

            // Handle interaction
            let response = ui.interact(button_rect, ui.id().with(icon), Sense::click());

            // Show tooltip
            response.clone().on_hover_text(tooltip);

            // Check for click
            if response.clicked() {
                action = button_action;
            }
        }
    });

    action
}

/// Returns (background_color, text_color) for a button based on theme and hover state.
fn get_button_colors(is_dark_mode: bool, hovered: bool) -> (Color32, Color32) {
    let alpha = if hovered { HOVER_ALPHA } else { IDLE_ALPHA };

    if is_dark_mode {
        let bg = Color32::from_rgba_unmultiplied(50, 50, 55, alpha);
        let text = Color32::from_rgba_unmultiplied(200, 200, 200, alpha + 30);
        (bg, text)
    } else {
        let bg = Color32::from_rgba_unmultiplied(240, 240, 240, alpha);
        let text = Color32::from_rgba_unmultiplied(60, 60, 60, alpha + 30);
        (bg, text)
    }
}
