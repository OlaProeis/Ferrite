//! Title bar rendering for the Ferrite application.
//!
//! This module renders the custom title bar with window controls,
//! app icon, file title, auto-save indicator, view mode segment,
//! and minimize/maximize/close buttons.

use super::FerriteApp;
use crate::config::ViewMode;
use crate::state::FileType;
use crate::ui::phosphor_icons::{phosphor_font, ARROWS_IN, GEAR, NOTE_PENCIL};
use crate::ui::{TitleBarButton, ViewModeSegment, ViewSegmentAction};
use eframe::egui;
use log::debug;
use rust_i18n::t;

/// Hit area for minimize / maximize / close (≈20% smaller than the original 36×22).
const WINDOW_CHROME_BTN_SIZE: egui::Vec2 = egui::vec2(29.0, 18.0);
/// Scale factor for chrome icon geometry (matches button shrink).
const WINDOW_CHROME_ICON_SCALE: f32 = 0.8;

impl FerriteApp {
    /// Render the custom title bar panel.
    pub(crate) fn render_title_bar(
        &mut self,
        ui: &mut egui::Ui,
        is_maximized: bool,
        is_dark: bool,
        _zen_mode: bool,
        title_bar_color: egui::Color32,
        button_hover_color: egui::Color32,
        close_hover_color: egui::Color32,
        text_color: egui::Color32,
    ) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("title_bar")
            .frame(
                egui::Frame::new()
                    .fill(title_bar_color)
                    .stroke(egui::Stroke::NONE)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                // Remove spacing between elements
                ui.spacing_mut().item_spacing.y = 0.0;

                // Add top padding for title bar
                ui.add_space(5.0);

                // Get state needed for title bar controls
                let has_editor = self.state.active_tab().is_some();
                let is_special_tab = self
                    .state
                    .active_tab()
                    .map(|t| t.is_special())
                    .unwrap_or(false);
                let auto_save_enabled = self
                    .state
                    .active_tab()
                    .map(|t| t.auto_save_enabled)
                    .unwrap_or(false);
                let current_view_mode = self
                    .state
                    .active_tab()
                    .map(|t| t.view_mode)
                    .unwrap_or(ViewMode::Raw);
                let current_file_type = self
                    .state
                    .active_tab()
                    .map(|t| t.file_type())
                    .unwrap_or(FileType::Unknown);
                let zen_mode_active = self.state.is_zen_mode();

                // Track title bar actions
                let mut title_bar_toggle_auto_save = false;
                let mut title_bar_toggle_zen = false;
                let mut title_bar_open_settings = false;
                let mut title_bar_new_window = false;
                let mut title_bar_close_window: Option<crate::state::WindowId> = None;
                let mut title_bar_view_action: Option<ViewSegmentAction> = None;

                // Title bar row - set consistent height and center alignment
                let title_bar_height = 28.0;
                ui.set_height(title_bar_height);

                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // App icon/logo - display texture if available, fallback to emoji
                    if let Some(texture) = &self.app_logo_texture {
                        let logo_size = 18.0; // Match title bar height nicely
                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(egui::vec2(logo_size, logo_size)),
                        );
                    } else {
                        ui.label(egui::RichText::new(NOTE_PENCIL).font(phosphor_font(14.0)));
                    }

                    ui.add_space(4.0);

                    ui.menu_button(t!("menu.window.label"), |ui| {
                        if ui.button(t!("menu.window.new_window")).clicked() {
                            title_bar_new_window = true;
                            ui.close();
                        }
                    });

                    ui.add_space(4.0);

                    // Window title (dynamically generated) - use consistent sizing
                    // Offset text slightly upward to better align with icon center
                    let title = self.window_title_for(self.state.working_window_id);
                    ui.add(
                        egui::Label::new(egui::RichText::new(title).size(12.0).color(text_color))
                            .selectable(false),
                    );

                    // Auto-save indicator (after filename) - only show for document tabs
                    if has_editor && !is_special_tab {
                        ui.add_space(8.0);
                        if TitleBarButton::show_auto_save(ui, auto_save_enabled, is_dark).clicked()
                        {
                            title_bar_toggle_auto_save = true;
                        }
                    }

                    // Fill remaining space with draggable area, but EXCLUDE the button area
                    // on the right side to prevent drag response from consuming clicks
                    // intended for window control buttons. This fixes Linux hit-testing issues.
                    //
                    // Button area width calculation (right-to-left):
                    // - 12.0 spacing + Close(36) + Max(36) + Min(36) + Fullscreen(36) + 8.0 spacing = 164px
                    // - Settings(28) + 4.0 + Zen(28) + 4.0 = 64px
                    // - ViewModeSegment (3 x 26px) = 78px (or 2 x 26px = 52px for 2-mode)
                    // Total ~306px + extra margin for safety = 400px
                    const WINDOW_BUTTON_AREA_WIDTH: f32 = 400.0;

                    let available = ui.available_rect_before_wrap();
                    let drag_width = (available.width() - WINDOW_BUTTON_AREA_WIDTH).max(0.0);
                    let drag_rect = egui::Rect::from_min_size(
                        available.min,
                        egui::vec2(drag_width, available.height()),
                    );

                    // IMPORTANT: We use Sense::hover() and handle drag detection manually via
                    // raw input state. This is necessary because:
                    //
                    // 1. When StartDrag is sent, the window manager takes over the drag operation
                    // 2. egui doesn't receive the mouse release event (WM handles it)
                    // 3. egui's widget interaction state gets confused, thinking the widget
                    //    is still being interacted with
                    // 4. On the next click, drag_started() doesn't fire because egui thinks
                    //    we're continuing an existing interaction
                    //
                    // By using raw input state (primary_pressed), we bypass egui's widget-level
                    // tracking entirely and get reliable drag detection every time.
                    let drag_response = ui.allocate_rect(drag_rect, egui::Sense::hover());

                    // Get raw pointer state - this is always accurate regardless of widget state
                    let (primary_pressed, double_clicked, pointer_pos) = ctx.input(|i| {
                        (
                            i.pointer.primary_pressed(),
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary),
                            i.pointer.interact_pos(),
                        )
                    });

                    // Check if pointer is in the drag area
                    let pointer_in_drag_area = pointer_pos
                        .map(|pos| drag_rect.contains(pos))
                        .unwrap_or(false);

                    // Handle double-click to maximize/restore
                    if double_clicked && pointer_in_drag_area {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // Handle drag to move window (but not if we're in a resize zone)
                    //
                    // We use primary_pressed() which is only true on the FRAME the button
                    // is pressed down. This ensures StartDrag is sent exactly once per click,
                    // preventing the "mouse stuck" bug on Linux.
                    let is_in_resize = self.window_resize_state.current_direction().is_some()
                        || self.window_resize_state.is_resizing();

                    if primary_pressed && pointer_in_drag_area && !is_in_resize {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

                    // Still use the response for hover effects if needed
                    let _ = drag_response;

                    // Window control buttons (right-to-left)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // ── Close button (×) ──────────────────────────────────────────────
                        let close_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(WINDOW_CHROME_BTN_SIZE),
                        );
                        // Rounded hover background; white icon on hover, normal otherwise
                        let close_icon_color = if close_btn.hovered() {
                            ui.painter().rect_filled(
                                close_btn.rect,
                                egui::CornerRadius::same(4),
                                close_hover_color,
                            );
                            egui::Color32::WHITE
                        } else {
                            text_color
                        };
                        // Draw × as two crisp diagonal line segments
                        {
                            let c = close_btn.rect.center();
                            let d = 5.5_f32 * WINDOW_CHROME_ICON_SCALE;
                            let stroke = egui::Stroke::new(1.5, close_icon_color);
                            ui.painter().line_segment(
                                [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [egui::pos2(c.x + d, c.y - d), egui::pos2(c.x - d, c.y + d)],
                                stroke,
                            );
                        }
                        let window_id = self.state.working_window_id;
                        if close_btn.clicked() {
                            if self.state.window_count() <= 1 {
                                if self.state.request_exit() {
                                    self.should_exit = true;
                                }
                            } else if self.state.request_close_window(window_id) {
                                title_bar_close_window = Some(window_id);
                            }
                        }
                        close_btn.on_hover_text(t!("a11y.close_button").to_string());

                        // ── Maximize / Restore button ──────────────────────────────────────
                        let max_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(WINDOW_CHROME_BTN_SIZE),
                        );
                        if max_btn.hovered() {
                            ui.painter().rect_filled(
                                max_btn.rect,
                                egui::CornerRadius::same(4),
                                button_hover_color,
                            );
                        }
                        // Draw maximize/restore icon (always visible, even on hover)
                        let max_center = max_btn.rect.center();
                        let stroke = egui::Stroke::new(1.5, text_color);
                        if is_maximized {
                            // Restore icon: two overlapping rectangles
                            let size = 4.5 * WINDOW_CHROME_ICON_SCALE;
                            let offset = 2.0 * WINDOW_CHROME_ICON_SCALE;
                            // Back rectangle (offset up-right) - show top and right edges only
                            let back_min = egui::pos2(
                                max_center.x - size + offset,
                                max_center.y - size - offset,
                            );
                            let back_max = egui::pos2(
                                max_center.x + size + offset,
                                max_center.y + size - offset,
                            );
                            let back_top_right = egui::pos2(back_max.x, back_min.y);
                            ui.painter().line_segment(
                                [egui::pos2(back_min.x + size, back_min.y), back_top_right],
                                stroke,
                            );
                            ui.painter()
                                .line_segment([back_top_right, back_max], stroke);
                            ui.painter().line_segment(
                                [back_max, egui::pos2(back_min.x + size, back_max.y)],
                                stroke,
                            );
                            // Front rectangle (main)
                            let front_rect = egui::Rect::from_center_size(
                                egui::pos2(
                                    max_center.x - offset / 2.0,
                                    max_center.y + offset / 2.0,
                                ),
                                egui::vec2(size * 2.0, size * 2.0),
                            );
                            ui.painter().rect_stroke(
                                front_rect,
                                0.0,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                        } else {
                            // Maximize icon: rectangle with thicker top edge (suggests a window title bar)
                            let size = 5.5 * WINDOW_CHROME_ICON_SCALE;
                            let rect = egui::Rect::from_center_size(
                                max_center,
                                egui::vec2(size * 2.0, size * 2.0),
                            );
                            let top_stroke =
                                egui::Stroke::new(2.0 * WINDOW_CHROME_ICON_SCALE, text_color);
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.min.x, rect.min.y),
                                    egui::pos2(rect.max.x, rect.min.y),
                                ],
                                top_stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.max.x, rect.min.y),
                                    egui::pos2(rect.max.x, rect.max.y),
                                ],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.max.x, rect.max.y),
                                    egui::pos2(rect.min.x, rect.max.y),
                                ],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.min.x, rect.max.y),
                                    egui::pos2(rect.min.x, rect.min.y),
                                ],
                                stroke,
                            );
                        }
                        if max_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }
                        let max_tooltip = if is_maximized { "Restore" } else { "Maximize" };
                        max_btn.on_hover_text(max_tooltip);

                        // ── Minimize button (–) ────────────────────────────────────────────
                        let min_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(WINDOW_CHROME_BTN_SIZE),
                        );
                        if min_btn.hovered() {
                            ui.painter().rect_filled(
                                min_btn.rect,
                                egui::CornerRadius::same(4),
                                button_hover_color,
                            );
                        }
                        let center = min_btn.rect.center();
                        let min_half = 6.0 * WINDOW_CHROME_ICON_SCALE;
                        ui.painter().line_segment(
                            [
                                egui::pos2(center.x - min_half, center.y),
                                egui::pos2(center.x + min_half, center.y),
                            ],
                            egui::Stroke::new(1.5, text_color),
                        );
                        if min_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        min_btn.on_hover_text(t!("a11y.minimize_button").to_string());

                        // Fullscreen button - draw expand arrows icon
                        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        let fullscreen_btn = ui.add(
                            egui::Button::new(egui::RichText::new(" ").size(14.0))
                                .frame(false)
                                .min_size(egui::vec2(36.0, 22.0)),
                        );
                        if fullscreen_btn.hovered() || is_fullscreen {
                            ui.painter().rect_filled(
                                fullscreen_btn.rect,
                                egui::CornerRadius::same(4),
                                button_hover_color,
                            );
                        }
                        // Draw fullscreen icon using corner-bracket shapes.
                        //   Enter fullscreen: ⌜⌝⌞⌟  – L-brackets at outer corners, arms pointing in.
                        //   Exit  fullscreen: inverse – brackets facing inward, arms pointing out.
                        let cx = fullscreen_btn.rect.center().x;
                        let cy = fullscreen_btn.rect.center().y;
                        let d = 4.5_f32; // corner distance from center
                        let a = 2.5_f32; // arm length
                        let stroke = egui::Stroke::new(1.5, text_color);
                        if is_fullscreen {
                            // Compress icon: inward-facing brackets (vertex near center, arms outward)
                            // TL
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx - d + a, cy - d + a),
                                    egui::pos2(cx - d, cy - d + a),
                                ],
                                stroke,
                            ); // ← left
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx - d + a, cy - d + a),
                                    egui::pos2(cx - d + a, cy - d),
                                ],
                                stroke,
                            ); // ↑ up
                               // TR
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx + d - a, cy - d + a),
                                    egui::pos2(cx + d, cy - d + a),
                                ],
                                stroke,
                            ); // → right
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx + d - a, cy - d + a),
                                    egui::pos2(cx + d - a, cy - d),
                                ],
                                stroke,
                            ); // ↑ up
                               // BL
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx - d + a, cy + d - a),
                                    egui::pos2(cx - d, cy + d - a),
                                ],
                                stroke,
                            ); // ← left
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx - d + a, cy + d - a),
                                    egui::pos2(cx - d + a, cy + d),
                                ],
                                stroke,
                            ); // ↓ down
                               // BR
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx + d - a, cy + d - a),
                                    egui::pos2(cx + d, cy + d - a),
                                ],
                                stroke,
                            ); // → right
                            ui.painter().line_segment(
                                [
                                    egui::pos2(cx + d - a, cy + d - a),
                                    egui::pos2(cx + d - a, cy + d),
                                ],
                                stroke,
                            ); // ↓ down
                        } else {
                            // Expand icon: outer-facing brackets (vertex at corner, arms pointing in)
                            // TL
                            ui.painter().line_segment(
                                [egui::pos2(cx - d, cy - d), egui::pos2(cx - d + a, cy - d)],
                                stroke,
                            ); // → right
                            ui.painter().line_segment(
                                [egui::pos2(cx - d, cy - d), egui::pos2(cx - d, cy - d + a)],
                                stroke,
                            ); // ↓ down
                               // TR
                            ui.painter().line_segment(
                                [egui::pos2(cx + d, cy - d), egui::pos2(cx + d - a, cy - d)],
                                stroke,
                            ); // ← left
                            ui.painter().line_segment(
                                [egui::pos2(cx + d, cy - d), egui::pos2(cx + d, cy - d + a)],
                                stroke,
                            ); // ↓ down
                               // BL
                            ui.painter().line_segment(
                                [egui::pos2(cx - d, cy + d), egui::pos2(cx - d + a, cy + d)],
                                stroke,
                            ); // → right
                            ui.painter().line_segment(
                                [egui::pos2(cx - d, cy + d), egui::pos2(cx - d, cy + d - a)],
                                stroke,
                            ); // ↑ up
                               // BR
                            ui.painter().line_segment(
                                [egui::pos2(cx + d, cy + d), egui::pos2(cx + d - a, cy + d)],
                                stroke,
                            ); // ← left
                            ui.painter().line_segment(
                                [egui::pos2(cx + d, cy + d), egui::pos2(cx + d, cy + d - a)],
                                stroke,
                            ); // ↑ up
                        }
                        if fullscreen_btn.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                !is_fullscreen,
                            ));
                        }
                        let fs_tooltip = if is_fullscreen {
                            t!("tooltip.fullscreen_exit")
                        } else {
                            t!("tooltip.fullscreen_enter")
                        };
                        fullscreen_btn.on_hover_text(fs_tooltip.to_string());

                        ui.add_space(8.0);

                        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                        // Title Bar Controls (before window buttons, right-to-left)
                        // Settings ΓåÆ Zen Mode ΓåÆ View Mode Segment
                        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ

                        // Settings button
                        if TitleBarButton::show(
                            ui,
                            GEAR,
                            &t!("tooltip.settings").to_string(),
                            false,
                            is_dark,
                        )
                        .clicked()
                        {
                            title_bar_open_settings = true;
                        }

                        ui.add_space(4.0);

                        // Zen Mode toggle (ARROWS_IN — matches command palette)
                        let zen_tooltip = if zen_mode_active {
                            t!("zen.exit")
                        } else {
                            t!("zen.enter")
                        };
                        if TitleBarButton::show(
                            ui,
                            ARROWS_IN,
                            &format!("{} (F11)", zen_tooltip),
                            zen_mode_active,
                            is_dark,
                        )
                        .clicked()
                        {
                            title_bar_toggle_zen = true;
                        }

                        ui.add_space(4.0);

                        // View Mode segmented control (all document tabs)
                        if has_editor && !is_special_tab {
                            let segment = ViewModeSegment::new();
                            let ferrite_accent = self.state.settings.ferrite_accent_rgb();

                            if current_file_type.supports_split() {
                                // Markdown/tabular: 3-mode (Raw | Split | Rendered)
                                if let Some(action) = segment.show(
                                    ui,
                                    current_view_mode,
                                    current_file_type,
                                    is_dark,
                                    ferrite_accent,
                                ) {
                                    title_bar_view_action = Some(action);
                                }
                            } else {
                                // All other types: 2-mode (Raw | Rendered)
                                if let Some(action) = segment.show_two_mode(
                                    ui,
                                    current_view_mode,
                                    is_dark,
                                    ferrite_accent,
                                ) {
                                    title_bar_view_action = Some(action);
                                }
                            }
                        }
                    });
                });

                ui.add_space(2.0);

                // Handle title bar actions (deferred to avoid borrow conflicts)
                if title_bar_toggle_auto_save {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.toggle_auto_save();
                        debug!("Title bar: Toggle auto-save -> {}", tab.auto_save_enabled);
                    }
                }
                if title_bar_toggle_zen {
                    self.state.toggle_zen_mode();
                    debug!("Title bar: Toggle Zen Mode");
                }
                if title_bar_open_settings {
                    self.state.open_settings_tab();
                    debug!("Title bar: Open Settings tab");
                }
                if title_bar_new_window {
                    self.handle_new_window(&ctx);
                    debug!("Title bar: New Window");
                }
                if let Some(window_id) = title_bar_close_window {
                    self.state.close_document_window(window_id);
                    self.secondary_window_resize_states.remove(&window_id);
                    self.last_window_titles.remove(&window_id);
                    debug!("Title bar: Closed window {}", window_id);
                }
                if let Some(view_action) = title_bar_view_action {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let new_mode = match view_action {
                            ViewSegmentAction::SetRaw => ViewMode::Raw,
                            ViewSegmentAction::SetSplit => ViewMode::Split,
                            ViewSegmentAction::SetRendered => ViewMode::Rendered,
                        };
                        tab.view_mode = new_mode;
                        self.state.mark_settings_dirty();
                        debug!("Title bar: Set view mode to {:?}", new_mode);
                    }
                }
            });
    }
}
