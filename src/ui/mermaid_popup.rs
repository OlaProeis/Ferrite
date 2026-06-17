//! Ferrite-native Mermaid diagram popup overlay with zoom and pan.
//!
//! The popup is stored in egui memory so rendered Markdown can open a large
//! diagram viewer without adding app-level state plumbing.

use egui::{
    Color32, CornerRadius, DragPanButtons, FontId, Id, Order, Pos2, Rect, Scene, Sense, Stroke,
    Vec2,
};

use crate::markdown::mermaid::render_mermaid_diagram;
use crate::ui::phosphor_icons::{
    phosphor_rich_text, ARROWS_COUNTER_CLOCKWISE, CURSOR, HAND, MAGNIFYING_GLASS_MINUS,
    MAGNIFYING_GLASS_PLUS, X,
};

const POPUP_STATE_KEY: &str = "mermaid_popup_state";

/// Persistent state for the Mermaid diagram popup.
#[derive(Debug, Clone)]
pub struct MermaidPopupState {
    /// Whether the popup is currently visible.
    pub open: bool,
    /// The Mermaid source code to render.
    pub source: String,
    /// Screen position of the popup's top-left corner.
    pub position: Pos2,
    /// Zoom scale factor (1.0 = original size).
    pub zoom: f32,
    /// Whether the user is currently dragging the popup.
    pub dragging: bool,
    /// Whether dark mode is active.
    pub dark_mode: bool,
    /// Base font size for diagram rendering.
    pub font_size: f32,
    /// Unique ID to distinguish multiple diagrams.
    pub diagram_id: u64,
    /// egui input time when zoom last changed.
    pub zoom_changed_at: f64,
    /// Current scene rect for panning/zooming the rendered diagram.
    pub scene_rect: Rect,
    /// Current interaction mode for the diagram canvas.
    pub interaction_mode: MermaidInteractionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MermaidInteractionMode {
    Select,
    Hand,
}

const POPUP_MARGIN: f32 = 18.0;
const POPUP_MIN_W: f32 = 640.0;
const POPUP_MIN_H: f32 = 420.0;
const DEFAULT_SCENE_SIZE: Vec2 = Vec2::new(1200.0, 900.0);
const MIN_SCENE_ZOOM: f32 = 0.15;
const MAX_SCENE_ZOOM: f32 = 5.0;
const SCENE_ZOOM_STEP: f32 = 0.1;

impl MermaidPopupState {
    /// Request to open a popup for a diagram.
    pub fn request_open(
        ctx: &egui::Context,
        source: String,
        _anchor_pos: Pos2,
        dark_mode: bool,
        font_size: f32,
        diagram_id: u64,
    ) {
        let screen_rect = ctx.content_rect();
        let popup_rect = popup_rect_for_screen(screen_rect);
        let position = popup_rect.min;

        let state = Self {
            open: true,
            source,
            position,
            zoom: 1.0,
            dragging: false,
            dark_mode,
            font_size,
            diagram_id,
            zoom_changed_at: 0.0,
            scene_rect: Rect::from_min_size(Pos2::ZERO, DEFAULT_SCENE_SIZE),
            interaction_mode: MermaidInteractionMode::Hand,
        };

        ctx.memory_mut(|mem| {
            mem.data.insert_temp(Id::new(POPUP_STATE_KEY), state);
        });
    }

    /// Get the current popup state from egui memory.
    pub fn get(ctx: &egui::Context) -> Option<Self> {
        ctx.memory(|mem| mem.data.get_temp::<Self>(Id::new(POPUP_STATE_KEY)))
    }

    /// Returns true while a Mermaid popup is open.
    pub fn is_open(ctx: &egui::Context) -> bool {
        Self::get(ctx).is_some_and(|state| state.open)
    }

    /// Update the popup state in egui memory.
    pub fn set(&self, ctx: &egui::Context) {
        ctx.memory_mut(|mem| {
            mem.data.insert_temp(Id::new(POPUP_STATE_KEY), self.clone());
        });
    }

    /// Clear the popup state.
    pub fn close(ctx: &egui::Context) {
        ctx.memory_mut(|mem| {
            mem.data.remove::<Self>(Id::new(POPUP_STATE_KEY));
        });
    }

    /// Render the popup overlay if one is open.
    pub fn render_overlay(ctx: &egui::Context) {
        let state = Self::get(ctx);
        let Some(mut state) = state else {
            return;
        };

        if !state.open {
            Self::close(ctx);
            return;
        }

        let current_time = ctx.input(|i| i.time);
        let mut should_close = false;

        {
            let screen_rect = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                Order::Background,
                Id::new("mermaid_popup_bg"),
            ));
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(70));
        }

        let popup_id = Id::new("mermaid_popup_area").with(state.diagram_id);
        let screen_rect = ctx.content_rect();
        let popup_rect = popup_rect_for_screen(screen_rect);
        state.position = popup_rect.min;
        let popup_size = popup_rect.size();

        egui::Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(state.position)
            .interactable(true)
            .sense(Sense::hover())
            .show(ctx, |ui| {
                ui.set_min_size(popup_size);
                ui.set_max_size(popup_size);
                render_popup_content(ui, &mut state, &mut should_close, current_time);
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            should_close = true;
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }

        let popup_rect = Rect::from_min_size(state.position, popup_size);
        let outside_clicked = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !popup_rect.contains(pos))
                && !state.dragging
        });
        if outside_clicked {
            should_close = true;
        }

        if should_close {
            Self::close(ctx);
        } else {
            state.set(ctx);
        }

        let seconds_since_change = (current_time - state.zoom_changed_at).max(0.0);
        if seconds_since_change < 1.0 {
            let alpha = if seconds_since_change > 0.7 {
                ((1.0 - seconds_since_change) / 0.3 * 200.0) as u8
            } else {
                200
            };
            let zoom_text = format!("{:.0}%", state.zoom * 100.0);
            egui::Area::new(Id::new("mermaid_zoom_notification"))
                .order(Order::Tooltip)
                .fixed_pos(Pos2::new(
                    state.position.x + popup_size.x / 2.0 - 30.0,
                    state.position.y + 42.0,
                ))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_black_alpha(alpha))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&zoom_text)
                                    .font(FontId::proportional(13.0))
                                    .color(Color32::from_white_alpha(alpha)),
                            );
                        });
                });
            ctx.request_repaint();
        }
    }
}

fn popup_rect_for_screen(screen_rect: Rect) -> Rect {
    let margin = Vec2::splat(POPUP_MARGIN);
    let available = screen_rect.shrink2(margin);
    let size = Vec2::new(
        available.width().max(POPUP_MIN_W),
        available.height().max(POPUP_MIN_H),
    );
    Rect::from_min_size(available.min, size)
}

fn render_popup_content(
    ui: &mut egui::Ui,
    state: &mut MermaidPopupState,
    should_close: &mut bool,
    current_time: f64,
) {
    let icon_color = if state.dark_mode {
        Color32::from_rgb(180, 180, 190)
    } else {
        Color32::from_rgb(80, 80, 90)
    };

    egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .inner_margin(0)
        .show(ui, |ui| {
            let title_frame = egui::Frame::new()
                .fill(if state.dark_mode {
                    Color32::from_rgba_unmultiplied(28, 28, 34, 225)
                } else {
                    Color32::from_rgba_unmultiplied(246, 247, 250, 226)
                })
                .stroke(Stroke::new(
                    1.0,
                    if state.dark_mode {
                        Color32::from_rgba_unmultiplied(85, 85, 100, 180)
                    } else {
                        Color32::from_rgba_unmultiplied(170, 175, 190, 190)
                    },
                ))
                .inner_margin(egui::Margin::symmetric(12, 6))
                .corner_radius(CornerRadius::same(10))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 3],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(70),
                });

            title_frame.show(ui, |ui| {
                state.dragging = false;

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Mermaid Diagram")
                            .font(FontId::proportional(13.0))
                            .strong()
                            .color(if state.dark_mode {
                                Color32::from_rgb(200, 200, 210)
                            } else {
                                Color32::from_rgb(60, 60, 70)
                            }),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn = ui.add(
                            egui::Button::new(phosphor_rich_text(X, 14.0).color(icon_color))
                                .frame(false)
                                .small(),
                        );
                        if close_btn.clicked() {
                            *should_close = true;
                        }
                        close_btn.on_hover_text("Close popup (Esc)");

                        ui.add_space(4.0);

                        let reset_btn = ui.add(
                            egui::Button::new(
                                phosphor_rich_text(ARROWS_COUNTER_CLOCKWISE, 14.0)
                                    .color(icon_color),
                            )
                            .frame(false)
                            .small(),
                        );
                        if reset_btn.clicked() {
                            set_scene_zoom(&mut state.scene_rect, 1.0, None);
                            state.zoom = current_scene_zoom(state.scene_rect);
                            state.zoom_changed_at = current_time;
                            ui.ctx().request_repaint();
                        }
                        reset_btn.on_hover_text("Reset zoom to 100%");

                        ui.add_space(2.0);

                        let zoom_out_btn = ui.add(
                            egui::Button::new(
                                phosphor_rich_text(MAGNIFYING_GLASS_MINUS, 14.0).color(icon_color),
                            )
                            .frame(false)
                            .small(),
                        );
                        if zoom_out_btn.clicked() {
                            let new_zoom = (current_scene_zoom(state.scene_rect) - SCENE_ZOOM_STEP)
                                .clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM);
                            set_scene_zoom(&mut state.scene_rect, new_zoom, None);
                            state.zoom = current_scene_zoom(state.scene_rect);
                            state.zoom_changed_at = current_time;
                            ui.ctx().request_repaint();
                        }
                        zoom_out_btn.on_hover_text("Zoom out");

                        ui.add_space(2.0);

                        let zoom_in_btn = ui.add(
                            egui::Button::new(
                                phosphor_rich_text(MAGNIFYING_GLASS_PLUS, 14.0).color(icon_color),
                            )
                            .frame(false)
                            .small(),
                        );
                        if zoom_in_btn.clicked() {
                            let new_zoom = (current_scene_zoom(state.scene_rect) + SCENE_ZOOM_STEP)
                                .clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM);
                            set_scene_zoom(&mut state.scene_rect, new_zoom, None);
                            state.zoom = current_scene_zoom(state.scene_rect);
                            state.zoom_changed_at = current_time;
                            ui.ctx().request_repaint();
                        }
                        zoom_in_btn.on_hover_text("Zoom in");

                        ui.add_space(6.0);

                        let zoom_text = format!("{:.0}%", state.zoom * 100.0);
                        ui.label(
                            egui::RichText::new(&zoom_text)
                                .font(FontId::monospace(11.0))
                                .color(if state.dark_mode {
                                    Color32::from_rgb(150, 150, 165)
                                } else {
                                    Color32::from_rgb(90, 90, 100)
                                }),
                        );

                        ui.add_space(8.0);

                        if mode_button(
                            ui,
                            CURSOR,
                            icon_color,
                            state.interaction_mode == MermaidInteractionMode::Select,
                            "Select mode: drag to move, wheel zoom disabled",
                        )
                        .clicked()
                        {
                            state.interaction_mode = MermaidInteractionMode::Select;
                        }

                        if mode_button(
                            ui,
                            HAND,
                            icon_color,
                            state.interaction_mode == MermaidInteractionMode::Hand,
                            "Hand mode: drag to move, wheel zooms",
                        )
                        .clicked()
                        {
                            state.interaction_mode = MermaidInteractionMode::Hand;
                        }
                    });
                });
            });

            let content_frame = egui::Frame::new()
                .fill(if state.dark_mode {
                    Color32::from_rgba_unmultiplied(12, 14, 18, 190)
                } else {
                    Color32::from_rgba_unmultiplied(252, 253, 255, 214)
                })
                .stroke(Stroke::new(
                    1.0,
                    if state.dark_mode {
                        Color32::from_rgba_unmultiplied(100, 108, 125, 125)
                    } else {
                        Color32::from_rgba_unmultiplied(178, 186, 202, 150)
                    },
                ))
                .inner_margin(egui::Margin::symmetric(10, 10))
                .corner_radius(CornerRadius::same(12))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_black_alpha(75),
                });

            content_frame.show(ui, |ui| {
                let content_rect = ui.available_rect_before_wrap();
                let hover_pos = ui.input(|i| i.pointer.hover_pos());
                let pointer_over_content = hover_pos.is_some_and(|pos| content_rect.contains(pos));
                let wheel_delta = if pointer_over_content {
                    ui.input(|i| i.smooth_scroll_delta.y)
                } else {
                    0.0
                };
                let zoom_anchor = hover_pos
                    .filter(|pos| content_rect.contains(*pos))
                    .map(|pos| viewport_pos_to_scene_pos(content_rect, state.scene_rect, pos));

                if pointer_over_content && wheel_delta.abs() > f32::EPSILON {
                    ui.input_mut(|i| {
                        i.smooth_scroll_delta = Vec2::ZERO;
                        i.events
                            .retain(|e| !matches!(e, egui::Event::MouseWheel { .. }));
                    });
                }

                let scene_zoom_before = current_scene_zoom(state.scene_rect);
                let scene_scale = scene_fit_scale(content_rect, state.scene_rect);
                let scene_zoom_range = if state.interaction_mode == MermaidInteractionMode::Select {
                    scene_scale..=scene_scale
                } else {
                    MIN_SCENE_ZOOM..=MAX_SCENE_ZOOM
                };
                let scene = Scene::new()
                    .zoom_range(scene_zoom_range)
                    .sense(Sense::click_and_drag())
                    .drag_pan_buttons(DragPanButtons::PRIMARY);

                let scene_response = scene.show(ui, &mut state.scene_rect, |ui| {
                    render_mermaid_diagram(ui, &state.source, state.dark_mode, state.font_size);
                });

                if state.interaction_mode == MermaidInteractionMode::Hand
                    && pointer_over_content
                    && wheel_delta.abs() > f32::EPSILON
                {
                    let current_zoom = current_scene_zoom(state.scene_rect);
                    let target_zoom = if wheel_delta > 0.0 {
                        (current_zoom + SCENE_ZOOM_STEP).clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM)
                    } else {
                        (current_zoom - SCENE_ZOOM_STEP).clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM)
                    };
                    if (target_zoom - current_zoom).abs() > 0.001 {
                        set_scene_zoom(&mut state.scene_rect, target_zoom, zoom_anchor);
                        state.zoom = current_scene_zoom(state.scene_rect);
                        state.zoom_changed_at = current_time;
                        ui.ctx().request_repaint();
                    }
                } else if state.interaction_mode == MermaidInteractionMode::Select
                    && scene_response.response.changed()
                {
                    let derived_zoom = current_scene_zoom(state.scene_rect);
                    if (derived_zoom - scene_zoom_before).abs() > 0.001 {
                        set_scene_zoom(&mut state.scene_rect, scene_zoom_before, None);
                    }
                    state.zoom = scene_zoom_before;
                } else if scene_response.response.changed() {
                    let derived_zoom = current_scene_zoom(state.scene_rect);
                    if (derived_zoom - state.zoom).abs() > 0.001 {
                        state.zoom = derived_zoom;
                        state.zoom_changed_at = current_time;
                    }
                }

                if scene_response.response.hovered() || scene_response.response.dragged() {
                    let cursor_icon = match state.interaction_mode {
                        MermaidInteractionMode::Select => egui::CursorIcon::Default,
                        MermaidInteractionMode::Hand => {
                            if scene_response.response.dragged() {
                                egui::CursorIcon::Grabbing
                            } else {
                                egui::CursorIcon::Grab
                            }
                        }
                    };
                    ui.ctx().set_cursor_icon(cursor_icon);
                }
            });
        });
}

fn mode_button(
    ui: &mut egui::Ui,
    icon: &str,
    icon_color: Color32,
    selected: bool,
    hover_text: &str,
) -> egui::Response {
    ui.add(
        egui::Button::new(phosphor_rich_text(icon, 14.0).color(icon_color))
            .selected(selected)
            .frame(true)
            .min_size(Vec2::splat(24.0)),
    )
    .on_hover_text(hover_text)
}

fn current_scene_zoom(scene_rect: Rect) -> f32 {
    (DEFAULT_SCENE_SIZE.x / scene_rect.width().abs().max(f32::EPSILON))
        .clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM)
}

fn scene_fit_scale(viewport_rect: Rect, scene_rect: Rect) -> f32 {
    let scene_size = scene_rect.size();
    let scale_x = viewport_rect.width() / scene_size.x.abs().max(f32::EPSILON);
    let scale_y = viewport_rect.height() / scene_size.y.abs().max(f32::EPSILON);
    scale_x.min(scale_y).clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM)
}

fn set_scene_zoom(scene_rect: &mut Rect, zoom: f32, anchor_in_scene: Option<Pos2>) {
    let zoom = zoom.clamp(MIN_SCENE_ZOOM, MAX_SCENE_ZOOM);
    let old_size = scene_rect.size();
    let new_size = Vec2::new(DEFAULT_SCENE_SIZE.x / zoom, DEFAULT_SCENE_SIZE.y / zoom);

    let new_rect = if let Some(anchor) = anchor_in_scene {
        let rel_x = if old_size.x.abs() > f32::EPSILON {
            (anchor.x - scene_rect.min.x) / old_size.x
        } else {
            0.5
        };
        let rel_y = if old_size.y.abs() > f32::EPSILON {
            (anchor.y - scene_rect.min.y) / old_size.y
        } else {
            0.5
        };
        let min = Pos2::new(anchor.x - rel_x * new_size.x, anchor.y - rel_y * new_size.y);
        Rect::from_min_size(min, new_size)
    } else {
        Rect::from_center_size(scene_rect.center(), new_size)
    };

    *scene_rect = new_rect;
}

fn viewport_pos_to_scene_pos(viewport_rect: Rect, scene_rect: Rect, viewport_pos: Pos2) -> Pos2 {
    let rel_x = if viewport_rect.width().abs() > f32::EPSILON {
        ((viewport_pos.x - viewport_rect.min.x) / viewport_rect.width()).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let rel_y = if viewport_rect.height().abs() > f32::EPSILON {
        ((viewport_pos.y - viewport_rect.min.y) / viewport_rect.height()).clamp(0.0, 1.0)
    } else {
        0.5
    };

    Pos2::new(
        scene_rect.min.x + scene_rect.width() * rel_x,
        scene_rect.min.y + scene_rect.height() * rel_y,
    )
}
