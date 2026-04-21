//! Format Toolbar - Collapsible formatting bar at the bottom of the raw editor.
//!
//! Displays markdown formatting buttons (bold, italic, code, headings, lists, etc.)
//! at the bottom of the raw editor area. Only shown for markdown files in Raw or
//! Split view modes.
//!
//! When collapsed, shows a thin strip with an up-chevron to expand.
//! When expanded, shows the full formatting buttons with a down-chevron to collapse.

use crate::app::modifier_symbol;
use crate::markdown::formatting::{FormattingState, MarkdownFormatCommand};
use crate::ui::RibbonAction;
use eframe::egui::{self, Color32, RichText, Ui, Vec2};

/// Height of the format toolbar when expanded.
const TOOLBAR_HEIGHT_EXPANDED: f32 = 32.0;

/// Height of the format toolbar when collapsed (just the toggle strip).
const TOOLBAR_HEIGHT_COLLAPSED: f32 = 18.0;

/// Format toolbar component for the bottom of the raw editor.
pub struct FormatToolbar;

impl FormatToolbar {
    /// Render the format toolbar at the bottom of the editor area.
    ///
    /// Returns the height consumed and any triggered action.
    pub fn show(
        ui: &mut Ui,
        expanded: bool,
        formatting_state: Option<&FormattingState>,
        has_editor: bool,
        is_dark: bool,
    ) -> FormatToolbarOutput {
        let mut action: Option<RibbonAction> = None;
        let mut toggle_visibility = false;

        let bar_bg = if is_dark {
            Color32::from_rgb(35, 35, 35)
        } else {
            Color32::from_rgb(245, 245, 245)
        };

        let separator_color = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let chevron_color = if is_dark {
            Color32::from_rgb(140, 140, 140)
        } else {
            Color32::from_rgb(120, 120, 120)
        };

        if expanded {
            // Expanded: full toolbar with format buttons
            // Use full available rect to ensure we fill the entire allocated space
            let rect = ui.available_rect_before_wrap();
            let _response = ui.allocate_rect(rect, egui::Sense::hover());

            // Background
            ui.painter().rect_filled(rect, 0.0, bar_bg);

            // Top border
            ui.painter().line_segment(
                [rect.left_top(), rect.right_top()],
                egui::Stroke::new(1.0, separator_color),
            );

            // Render buttons inside the rect
            let mut button_ui = ui.child_ui(
                rect.shrink2(Vec2::new(4.0, 2.0)),
                egui::Layout::left_to_right(egui::Align::Center),
                None,
            );
            button_ui.spacing_mut().item_spacing.x = 2.0;

            // Get formatting state for button highlighting
            let is_bold = formatting_state.map(|s| s.is_bold).unwrap_or(false);
            let is_italic = formatting_state.map(|s| s.is_italic).unwrap_or(false);
            let is_code = formatting_state.map(|s| s.is_inline_code).unwrap_or(false);
            let is_link = formatting_state.map(|s| s.is_link).unwrap_or(false);

            // Bold
            if format_button(
                &mut button_ui,
                "B",
                &MarkdownFormatCommand::Bold.tooltip(),
                has_editor,
                is_bold,
                is_dark,
                true,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Bold));
            }

            // Italic
            if format_button(
                &mut button_ui,
                "I",
                &MarkdownFormatCommand::Italic.tooltip(),
                has_editor,
                is_italic,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Italic));
            }

            // Inline code
            if format_button(
                &mut button_ui,
                "<>",
                &MarkdownFormatCommand::InlineCode.tooltip(),
                has_editor,
                is_code,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::InlineCode));
            }

            // Link
            if format_button(
                &mut button_ui,
                "[~]",
                &MarkdownFormatCommand::Link.tooltip(),
                has_editor,
                is_link,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Link));
            }

            button_ui.add_space(4.0);
            toolbar_separator(
                &mut button_ui,
                separator_color,
                TOOLBAR_HEIGHT_EXPANDED - 12.0,
            );
            button_ui.add_space(4.0);

            // Heading dropdown
            let current_heading = formatting_state.and_then(|s| s.heading_level);
            let heading_label = current_heading
                .map(|h| format!("H{}", h as u8))
                .unwrap_or_else(|| "H".to_string());

            egui::ComboBox::from_id_source("format_bar_heading_dropdown")
                .selected_text(RichText::new(heading_label).size(11.0))
                .width(36.0)
                .show_ui(&mut button_ui, |ui| {
                    for level in 1..=6u8 {
                        let is_selected =
                            current_heading.map(|h| h as u8 == level).unwrap_or(false);
                        let label = format!("H{}", level);
                        if ui
                            .selectable_label(is_selected, &label)
                            .on_hover_text(format!("{}+{}", modifier_symbol(), level))
                            .clicked()
                        {
                            action =
                                Some(RibbonAction::Format(MarkdownFormatCommand::Heading(level)));
                        }
                    }
                });

            button_ui.add_space(4.0);
            toolbar_separator(
                &mut button_ui,
                separator_color,
                TOOLBAR_HEIGHT_EXPANDED - 12.0,
            );
            button_ui.add_space(4.0);

            // List buttons
            let is_bullet = formatting_state.map(|s| s.is_bullet_list).unwrap_or(false);
            let is_numbered = formatting_state
                .map(|s| s.is_numbered_list)
                .unwrap_or(false);

            if format_button(
                &mut button_ui,
                "−",
                &MarkdownFormatCommand::BulletList.tooltip(),
                has_editor,
                is_bullet,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::BulletList));
            }

            if format_button(
                &mut button_ui,
                "1.",
                &MarkdownFormatCommand::NumberedList.tooltip(),
                has_editor,
                is_numbered,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::NumberedList));
            }

            // Blockquote
            let is_quote = formatting_state.map(|s| s.is_blockquote).unwrap_or(false);
            if format_button(
                &mut button_ui,
                ">",
                &MarkdownFormatCommand::Blockquote.tooltip(),
                has_editor,
                is_quote,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Blockquote));
            }

            // Code block
            let is_code_block = formatting_state.map(|s| s.is_code_block).unwrap_or(false);
            if format_button(
                &mut button_ui,
                "{}",
                &MarkdownFormatCommand::CodeBlock.tooltip(),
                has_editor,
                is_code_block,
                is_dark,
                false,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::CodeBlock));
            }

            button_ui.add_space(4.0);
            toolbar_separator(
                &mut button_ui,
                separator_color,
                TOOLBAR_HEIGHT_EXPANDED - 12.0,
            );
            button_ui.add_space(4.0);

            // Table of Contents
            if toolbar_icon_button(
                &mut button_ui,
                "☰",
                &format!(
                    "Insert/Update Table of Contents ({}+Shift+U)",
                    modifier_symbol()
                ),
                has_editor,
                is_dark,
            )
            .clicked()
            {
                action = Some(RibbonAction::InsertToc);
            }

            // Collapse button (right-aligned)
            button_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = ui.add(
                    egui::Button::new(RichText::new("▼").size(10.0).color(chevron_color))
                        .frame(false)
                        .min_size(Vec2::new(20.0, 18.0)),
                );
                if btn.clicked() {
                    toggle_visibility = true;
                }
                btn.on_hover_text("Hide formatting toolbar");
            });

            FormatToolbarOutput {
                action,
                toggle_visibility,
            }
        } else {
            // Collapsed: thin strip with up-chevron
            // Use full available rect to ensure we fill the entire allocated space
            let rect = ui.available_rect_before_wrap();
            let response = ui.allocate_rect(rect, egui::Sense::click());

            // Subtle background
            let bg = if response.hovered() {
                if is_dark {
                    Color32::from_rgb(45, 45, 45)
                } else {
                    Color32::from_rgb(238, 238, 238)
                }
            } else {
                bar_bg
            };
            ui.painter().rect_filled(rect, 0.0, bg);

            // Top border
            ui.painter().line_segment(
                [rect.left_top(), rect.right_top()],
                egui::Stroke::new(1.0, separator_color),
            );

            // Centered chevron
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "▲ Format",
                egui::FontId::proportional(10.0),
                chevron_color,
            );

            if response.clicked() {
                toggle_visibility = true;
            }

            response.on_hover_text("Show formatting toolbar");

            FormatToolbarOutput {
                action,
                toggle_visibility,
            }
        }
    }
}

/// Output from the format toolbar.
pub struct FormatToolbarOutput {
    /// Action triggered by a button click.
    pub action: Option<RibbonAction>,
    /// Whether the user clicked the toggle button.
    pub toggle_visibility: bool,
}

/// Render a format button with active state highlighting.
fn format_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    active: bool,
    is_dark: bool,
    bold_text: bool,
) -> egui::Response {
    let text_color = if enabled {
        if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(50, 50, 50)
        }
    } else if is_dark {
        Color32::from_rgb(100, 100, 100)
    } else {
        Color32::from_rgb(160, 160, 160)
    };

    let active_bg = if is_dark {
        Color32::from_rgb(70, 90, 120)
    } else {
        Color32::from_rgb(200, 220, 240)
    };

    let hover_bg = if is_dark {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::from_rgb(220, 220, 220)
    };

    let mut text = RichText::new(icon).size(11.0).color(text_color);
    if bold_text {
        text = text.strong();
    }

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .frame(false)
            .min_size(Vec2::new(22.0, 20.0)),
    );

    if active && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::Rounding::same(3.0), active_bg);
        let font_id = if bold_text {
            egui::FontId::new(11.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(11.0)
        };
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            font_id,
            text_color,
        );
    } else if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);
        let font_id = if bold_text {
            egui::FontId::new(11.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(11.0)
        };
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            font_id,
            text_color,
        );
    }

    btn.on_hover_text(tooltip)
}

/// Small icon button for the toolbar.
fn toolbar_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    is_dark: bool,
) -> egui::Response {
    let text_color = if enabled {
        if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(50, 50, 50)
        }
    } else if is_dark {
        Color32::from_rgb(100, 100, 100)
    } else {
        Color32::from_rgb(160, 160, 160)
    };

    let hover_bg = if is_dark {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::from_rgb(220, 220, 220)
    };

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(" ").size(14.0))
            .frame(false)
            .min_size(Vec2::new(24.0, 20.0)),
    );

    if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);
    }

    ui.painter().text(
        btn.rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(14.0),
        text_color,
    );

    btn.on_hover_text(tooltip)
}

/// Draw a vertical separator line in the toolbar.
fn toolbar_separator(ui: &mut Ui, color: Color32, height: f32) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(1.0, height), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(1.0, color),
    );
}

/// Render the side panel toggle strip (shown when the outline panel is closed).
///
/// Returns true if the user clicked to open the side panel.
pub fn side_panel_toggle_strip(ctx: &egui::Context, is_dark: bool) -> bool {
    let mut clicked = false;

    let strip_width = 20.0;

    let bg = if is_dark {
        Color32::from_rgb(35, 35, 35)
    } else {
        Color32::from_rgb(245, 245, 245)
    };

    let separator_color = if is_dark {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::from_rgb(200, 200, 200)
    };

    let chevron_color = if is_dark {
        Color32::from_rgb(140, 140, 140)
    } else {
        Color32::from_rgb(120, 120, 120)
    };

    egui::SidePanel::right("side_panel_toggle_strip")
        .resizable(false)
        .exact_width(strip_width)
        .frame(
            egui::Frame::none()
                .fill(bg)
                .stroke(egui::Stroke::NONE)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ctx, |ui| {
            // Left border
            let panel_rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [panel_rect.left_top(), panel_rect.left_bottom()],
                egui::Stroke::new(1.0, separator_color),
            );

            // Clickable area for the whole strip
            let response = ui.allocate_rect(panel_rect, egui::Sense::click());

            // Hover effect
            if response.hovered() {
                let hover_bg = if is_dark {
                    Color32::from_rgb(50, 50, 50)
                } else {
                    Color32::from_rgb(230, 230, 230)
                };
                ui.painter().rect_filled(panel_rect, 0.0, hover_bg);
            }

            // Chevron pointing left (to indicate "open panel")
            ui.painter().text(
                panel_rect.center(),
                egui::Align2::CENTER_CENTER,
                "◀",
                egui::FontId::proportional(12.0),
                chevron_color,
            );

            if response.clicked() {
                clicked = true;
            }

            response.on_hover_text("Show side panel");
        });

    clicked
}
