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

use crate::config::ShortcutCommand;
use crate::ui::phosphor_icons::{
    phosphor_font, CARET_DOWN, CARET_UP, CODE, CODE_BLOCK, LINK, LIST_BULLETS, TEXT_B, TEXT_ITALIC,
    X,
};
use eframe::egui::{
    self, Color32, Context, Pos2, Rect, RichText, ScrollArea, Sense, Shadow, Stroke, StrokeKind,
    Ui, Vec2,
};

/// Temp data key: when true, nav buttons are hidden (modal overlays are open).
fn overlay_blocks_nav_id() -> egui::Id {
    egui::Id::new("overlay_blocks_nav_buttons")
}

/// Hide document nav buttons while a modal overlay (quick switcher, command palette, etc.) is open.
pub fn set_overlay_blocks_nav_buttons(ctx: &Context, blocked: bool) {
    ctx.data_mut(|d| d.insert_temp(overlay_blocks_nav_id(), blocked));
}

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

    if ui
        .ctx()
        .data(|d| d.get_temp::<bool>(overlay_blocks_nav_id()).unwrap_or(false))
    {
        return NavAction::None;
    }

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

    // Middle layer: above editor content, below modal overlays (Foreground/Tooltip).
    let layer_id = egui::LayerId::new(egui::Order::Middle, ui.id().with("nav_buttons"));

    ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
        // Position the buttons vertically
        // Using simple arrow characters that render in most fonts
        let button_positions = [
            (
                container_pos,
                CARET_UP,
                true,
                "Jump to top (Ctrl+Home)",
                NavAction::Top,
            ),
            (
                Pos2::new(
                    container_pos.x,
                    container_pos.y + BUTTON_SIZE + BUTTON_SPACING,
                ),
                "●",
                false,
                "Jump to middle",
                NavAction::Middle,
            ),
            (
                Pos2::new(
                    container_pos.x,
                    container_pos.y + (BUTTON_SIZE + BUTTON_SPACING) * 2.0,
                ),
                CARET_DOWN,
                true,
                "Jump to bottom (Ctrl+End)",
                NavAction::Bottom,
            ),
        ];

        for (pos, icon, use_phosphor, tooltip, button_action) in button_positions {
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
                ui.painter().rect_stroke(
                    button_rect,
                    4.0,
                    egui::Stroke::new(1.0, border_color),
                    StrokeKind::Inside,
                );
            }

            // Draw icon
            let font_id = if use_phosphor {
                phosphor_font(14.0)
            } else {
                egui::FontId::proportional(14.0)
            };
            let galley = ui
                .painter()
                .layout_no_wrap(icon.to_string(), font_id, text_color);
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

/// Output from the Markdown quick reference overlay.
#[derive(Debug, Clone, Default)]
pub struct MarkdownCheatsheetOutput {
    /// If set, open Settings -> Keyboard and filter to the selected command.
    pub open_keyboard_shortcut: Option<ShortcutCommand>,
    /// Current expanded/collapsed visibility after this frame.
    pub expanded: bool,
}

/// Render a compact Markdown syntax and shortcut cheat sheet near a toolbar trigger.
pub fn render_markdown_cheatsheet(
    ui: &mut Ui,
    editor_rect: Rect,
    is_dark_mode: bool,
    trigger_rect: Option<Rect>,
    expanded: bool,
) -> MarkdownCheatsheetOutput {
    let mut output = MarkdownCheatsheetOutput {
        expanded,
        ..Default::default()
    };

    if ui
        .ctx()
        .data(|d| d.get_temp::<bool>(overlay_blocks_nav_id()).unwrap_or(false))
    {
        return output;
    }

    if editor_rect.width() < 180.0 || editor_rect.height() < 120.0 || !output.expanded {
        return output;
    }

    let width = editor_rect.width().clamp(240.0, 304.0);
    let height = (editor_rect.height() - 28.0).clamp(238.0, 330.0);
    let pos = markdown_cheatsheet_panel_pos(editor_rect, trigger_rect, output.expanded);
    let panel_rect = Rect::from_min_size(pos, Vec2::new(width, height));
    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    let hovered = mouse_pos.is_some_and(|p| panel_rect.expand(8.0).contains(p));

    let bg = panel_bg(is_dark_mode, hovered || output.expanded);
    let border = panel_border(is_dark_mode, hovered || output.expanded);
    let shadow = if hovered || output.expanded {
        if is_dark_mode {
            Shadow {
                offset: [0, 10],
                blur: 28,
                spread: 0,
                color: Color32::from_black_alpha(110),
            }
        } else {
            Shadow {
                offset: [0, 12],
                blur: 30,
                spread: 0,
                color: Color32::from_black_alpha(30),
            }
        }
    } else {
        Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if is_dark_mode { 70 } else { 18 }),
        }
    };

    egui::Area::new(egui::Id::new("markdown_cheatsheet_area"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(true)
        .show(ui.ctx(), |ui| {
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
            ui.set_clip_rect(rect.expand(2.0));
            ui.painter().add(shadow.as_shape(rect, 9));
            ui.painter().rect_filled(rect, 9.0, bg);
            ui.painter()
                .rect_stroke(rect, 9.0, Stroke::new(1.0, border), StrokeKind::Inside);

            let mut content_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(Vec2::new(6.0, 3.0)))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            let mut expanded = output.expanded;
            draw_cheatsheet_expanded(&mut content_ui, is_dark_mode, &mut expanded, &mut output);
            output.expanded = expanded;
        });

    let clicked_outside = ui.ctx().input(|i| {
        i.pointer.any_click()
            && i.pointer.interact_pos().is_some_and(|pos| {
                markdown_cheatsheet_should_dismiss_click(panel_rect, trigger_rect, pos)
            })
    });
    let escape_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
    if clicked_outside || escape_pressed {
        output.expanded = false;
    }

    output
}

fn draw_cheatsheet_expanded(
    ui: &mut Ui,
    is_dark: bool,
    expanded: &mut bool,
    output: &mut MarkdownCheatsheetOutput,
) {
    let text = text_color(is_dark, true);
    let muted = text_color(is_dark, false);
    let code_bg = if is_dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 18)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 14)
    };

    ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
    ui.spacing_mut().button_padding = Vec2::new(4.0, 2.0);
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.label(
            RichText::new("Markdown guide")
                .size(13.0)
                .strong()
                .color(text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_response = ui
                .add(
                    egui::Button::new(RichText::new(X).font(phosphor_font(11.0)).color(muted))
                        .frame(false)
                        .min_size(Vec2::new(24.0, 22.0)),
                )
                .on_hover_text("Collapse");
            if close_response.clicked() {
                *expanded = false;
            }
        });
    });

    let guide_url = "https://www.markdownguide.org/basic-syntax/";
    ui.hyperlink_to(guide_url, guide_url)
        .on_hover_text("Open Markdown Guide");

    ui.add_space(3.0);
    ui.separator();
    ui.add_space(3.0);

    ui.set_width(ui.available_width());
    ScrollArea::vertical()
        .id_salt("markdown_cheatsheet_rows")
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            cheat_row(
                ui,
                TEXT_B,
                "Bold",
                "**text**",
                "Ctrl+B",
                Some(ShortcutCommand::FormatBold),
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                TEXT_ITALIC,
                "Italic",
                "*text*",
                "Ctrl+I",
                Some(ShortcutCommand::FormatItalic),
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                CODE,
                "Inline code",
                "`code`",
                "Ctrl+Shift+`",
                Some(ShortcutCommand::FormatInlineCode),
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                LINK,
                "Link",
                "[text](url)",
                "Ctrl+K",
                Some(ShortcutCommand::FormatLink),
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                CODE_BLOCK,
                "Code block",
                "```lang",
                "Ctrl+Shift+C",
                Some(ShortcutCommand::FormatCodeBlock),
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                "$",
                "Math",
                "$$E=mc^2$$",
                "-",
                None,
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                "x2",
                "Superscript",
                "x^2",
                "-",
                None,
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui,
                ".*",
                "Regex escape",
                "\\d+ or \\*",
                "-",
                None,
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
            cheat_row(
                ui, "^", "Footnote", "[^1]", "-", None, text, muted, code_bg, expanded, output,
            );
            cheat_row(
                ui,
                LIST_BULLETS,
                "Task list",
                "- [x] item",
                "-",
                None,
                text,
                muted,
                code_bg,
                expanded,
                output,
            );
        });
}

#[allow(clippy::too_many_arguments)]
fn cheat_row(
    ui: &mut Ui,
    icon: &str,
    label: &str,
    syntax: &str,
    shortcut: &str,
    shortcut_command: Option<ShortcutCommand>,
    text: Color32,
    muted: Color32,
    code_bg: Color32,
    expanded: &mut bool,
    output: &mut MarkdownCheatsheetOutput,
) {
    let row_height = 28.0;
    let rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), row_height), Sense::hover())
        .0;
    let response = ui.interact(rect, ui.id().with(label), Sense::hover());

    if response.hovered() {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(4.0, 2.0)),
            6.0,
            code_bg.gamma_multiply(1.6),
        );
    }

    let icon_pos = Pos2::new(rect.left() + 13.0, rect.center().y);
    let icon_font = if icon == "x2" || icon == ".*" || icon == "^" || icon == "$" {
        egui::FontId::proportional(12.0)
    } else {
        phosphor_font(12.0)
    };
    ui.painter().text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        icon,
        icon_font,
        muted,
    );

    ui.painter().text(
        Pos2::new(rect.left() + 27.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.2),
        text,
    );

    let shortcut_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 82.0, rect.center().y - 9.0),
        Vec2::new(74.0, 18.0),
    );
    let shortcut_response = ui.interact(
        shortcut_rect,
        ui.id().with((label, "shortcut")),
        Sense::click(),
    );
    let shortcut_fill = if shortcut_response.hovered() {
        code_bg.gamma_multiply(1.45)
    } else {
        code_bg
    };
    ui.painter().rect_filled(shortcut_rect, 5.0, shortcut_fill);
    ui.painter().text(
        shortcut_rect.center(),
        egui::Align2::CENTER_CENTER,
        shortcut,
        egui::FontId::monospace(10.5),
        if shortcut_response.hovered() {
            text
        } else {
            muted
        },
    );

    if let Some(command) = shortcut_command {
        let response = shortcut_response
            .clone()
            .on_hover_text("Open Keyboard shortcuts")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            output.open_keyboard_shortcut = Some(command);
            output.expanded = false;
            *expanded = false;
        }
    }

    ui.painter().text(
        Pos2::new(rect.right() - 92.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        syntax,
        egui::FontId::monospace(10.8),
        muted,
    );
}

fn markdown_cheatsheet_should_dismiss_click(
    panel_rect: Rect,
    trigger_rect: Option<Rect>,
    click_pos: Pos2,
) -> bool {
    !panel_rect.expand(6.0).contains(click_pos)
        && !trigger_rect.is_some_and(|rect| rect.expand(6.0).contains(click_pos))
}

pub(crate) fn markdown_cheatsheet_panel_pos(
    editor_rect: Rect,
    trigger_rect: Option<Rect>,
    expanded: bool,
) -> Pos2 {
    let width = if expanded {
        editor_rect.width().clamp(240.0, 304.0)
    } else {
        138.0
    };
    let height = if expanded {
        (editor_rect.height() - 28.0).clamp(238.0, 330.0)
    } else {
        30.0
    };
    let margin = Vec2::new(12.0, 12.0);
    if let Some(trigger_rect) = trigger_rect {
        let x = (trigger_rect.center().x - width / 2.0).clamp(
            editor_rect.min.x + margin.x,
            editor_rect.max.x - width - margin.x,
        );
        let preferred_below = trigger_rect.max.y + 10.0;
        let y = if preferred_below + height <= editor_rect.max.y - margin.y {
            preferred_below
        } else {
            (trigger_rect.min.y - height - 10.0).max(editor_rect.min.y + margin.y)
        };
        Pos2::new(x, y)
    } else {
        Pos2::new(
            (editor_rect.max.x - width - margin.x).max(editor_rect.min.x + margin.x),
            (editor_rect.max.y - height - margin.y).max(editor_rect.min.y + margin.y),
        )
    }
}

fn panel_bg(is_dark: bool, active: bool) -> Color32 {
    match (is_dark, active) {
        (true, true) => Color32::from_rgba_unmultiplied(35, 37, 44, 238),
        (true, false) => Color32::from_rgba_unmultiplied(35, 37, 44, 200),
        (false, true) => Color32::from_rgba_unmultiplied(255, 255, 255, 242),
        (false, false) => Color32::from_rgba_unmultiplied(255, 255, 255, 210),
    }
}

fn panel_border(is_dark: bool, active: bool) -> Color32 {
    match (is_dark, active) {
        (true, true) => Color32::from_rgba_unmultiplied(255, 255, 255, 44),
        (true, false) => Color32::from_rgba_unmultiplied(255, 255, 255, 24),
        (false, true) => Color32::from_rgba_unmultiplied(0, 0, 0, 38),
        (false, false) => Color32::from_rgba_unmultiplied(0, 0, 0, 18),
    }
}

fn text_color(is_dark: bool, strong: bool) -> Color32 {
    match (is_dark, strong) {
        (true, true) => Color32::from_rgb(236, 237, 241),
        (true, false) => Color32::from_rgb(172, 176, 188),
        (false, true) => Color32::from_rgb(42, 45, 54),
        (false, false) => Color32::from_rgb(104, 110, 124),
    }
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
