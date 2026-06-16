//! Multi-window viewport lifecycle for document windows.

use super::FerriteApp;
use crate::state::{document_viewport_id, WindowId, PRIMARY_WINDOW_ID};
use crate::ui::{
    apply_window_chrome, consume_clicks_in_resize_zones, handle_window_resize, WindowResizeState,
};
use eframe::egui;
use log::debug;

impl FerriteApp {
    pub(crate) fn secondary_resize_state(&mut self, window_id: WindowId) -> &mut WindowResizeState {
        self.secondary_window_resize_states
            .entry(window_id)
            .or_default()
    }

    /// Render secondary document windows via child viewports.
    pub(crate) fn render_secondary_document_windows(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
    ) {
        let secondary_windows: Vec<(WindowId, egui::ViewportId, bool)> = self
            .state
            .windows
            .iter()
            .filter(|w| w.id != PRIMARY_WINDOW_ID)
            .map(|w| (w.id, w.viewport_id, w.first_frame))
            .collect();

        let mut closed_windows = Vec::new();
        let app_ptr = self as *mut FerriteApp;
        let frame_ptr = frame as *mut eframe::Frame;

        for (window_id, viewport_id, first_frame) in secondary_windows {
            let geometry = self
                .state
                .window_by_id(window_id)
                .map(|w| w.geometry.clone())
                .unwrap_or_default();

            let use_native_decorations = self.state.settings.native_window_decorations_enabled();
            let mut builder = apply_window_chrome(
                egui::ViewportBuilder::default().with_title(self.window_title_for(window_id)),
                use_native_decorations,
            );

            if first_frame {
                if let (Some(x), Some(y)) = (geometry.x, geometry.y) {
                    builder = builder.with_position(egui::pos2(x, y));
                }
                builder = builder.with_inner_size(egui::vec2(geometry.width, geometry.height));
            }

            let resize_ptr = self.secondary_resize_state(window_id) as *mut WindowResizeState;
            let mut open = true;

            ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, _class| {
                let app = unsafe { &mut *app_ptr };
                let frame = unsafe { &mut *frame_ptr };
                let resize_state = unsafe { &mut *resize_ptr };

                if !use_native_decorations {
                    handle_window_resize(child_ctx, resize_state);
                }

                if child_ctx.input(|i| i.viewport().focused.unwrap_or(false)) {
                    app.state.set_focused_window(window_id);
                }

                if child_ctx.input(|i| i.viewport().close_requested()) {
                    if app.handle_window_close_request(window_id, child_ctx) {
                        open = false;
                    } else {
                        child_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    }
                }

                if first_frame {
                    if let Some(w) = app.state.window_by_id_mut(window_id) {
                        w.first_frame = false;
                    }
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }

                let title = app.window_title_for(window_id);
                let last = app
                    .last_window_titles
                    .entry(window_id)
                    .or_insert_with(String::new);
                if *last != title {
                    *last = title.clone();
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                }

                app.state.working_window_id = window_id;
                app.update_window_geometry_for(window_id, child_ctx);

                if !use_native_decorations {
                    consume_clicks_in_resize_zones(child_ctx, resize_state);
                    resize_state.apply_cursor(child_ctx);
                }

                if open {
                    let panel_id =
                        egui::Id::new((child_ctx.viewport_id(), "document_window_root"));
                    let mut panel_ui = egui::Ui::new(
                        child_ctx.clone(),
                        panel_id,
                        egui::UiBuilder::new()
                            .layer_id(egui::LayerId::background())
                            .max_rect(child_ctx.content_rect()),
                    );
                    panel_ui.set_clip_rect(child_ctx.content_rect());
                    app.render_ui(&mut panel_ui, frame);

                    // Keyboard shortcuts and palette dispatch for this viewport
                    app.handle_keyboard_shortcuts(child_ctx);
                    if let Some(cmd) = app.pending_palette_command.take() {
                        app.dispatch_palette_command(child_ctx, cmd);
                    }
                }
            });

            if !open {
                closed_windows.push(window_id);
            }
        }

        for window_id in closed_windows {
            debug!("Secondary window {} closed", window_id);
            self.state.close_document_window(window_id);
            self.secondary_window_resize_states.remove(&window_id);
            self.last_window_titles.remove(&window_id);
        }

        // Subsequent UI (and next frame's update-phase dialogs) belongs to the
        // primary window again — e.g. video occluder rects are gated on this.
        self.state.working_window_id = PRIMARY_WINDOW_ID;
    }

    pub(crate) fn handle_window_close_request(
        &mut self,
        window_id: WindowId,
        ctx: &egui::Context,
    ) -> bool {
        if self.state.window_count() <= 1 {
            if self.handle_close_request(ctx) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return true;
            }
            return false;
        }
        self.flush_window_rendered_sessions(ctx, window_id);
        self.state.request_close_window(window_id)
    }

    pub(crate) fn window_title_for(&self, window_id: WindowId) -> String {
        const APP_NAME: &str = "Ferrite";
        let tab_title = self
            .state
            .window_by_id(window_id)
            .and_then(|w| w.tab_ids.get(w.active_tab_index))
            .and_then(|id| self.state.tab_by_id(*id))
            .map(|t| t.title())
            .unwrap_or_else(|| APP_NAME.to_string());

        if tab_title == APP_NAME {
            APP_NAME.to_string()
        } else {
            format!("{} - {}", tab_title, APP_NAME)
        }
    }

    pub(crate) fn update_window_geometry_for(
        &mut self,
        window_id: WindowId,
        ctx: &egui::Context,
    ) {
        ctx.input(|i| {
            if let Some(rect) = i.viewport().outer_rect {
                let size = rect.size();
                let pos = rect.min;
                let maximized = i.viewport().maximized.unwrap_or(false);
                if let Some(window) = self.state.window_by_id_mut(window_id) {
                    window.geometry.width = size.x;
                    window.geometry.height = size.y;
                    window.geometry.x = Some(pos.x);
                    window.geometry.y = Some(pos.y);
                    window.geometry.maximized = maximized;
                }
                if window_id == PRIMARY_WINDOW_ID {
                    self.last_window_size = Some(size);
                    self.last_window_pos = Some(pos);
                    self.state.settings.window_size = crate::config::WindowSize {
                        width: size.x,
                        height: size.y,
                        x: Some(pos.x),
                        y: Some(pos.y),
                        maximized,
                    };
                    self.state.mark_settings_dirty();
                }
            }
        });
    }

    pub(crate) fn focus_document_window(&self, ctx: &egui::Context, window_id: WindowId) {
        if let Some(window) = self.state.window_by_id(window_id) {
            ctx.send_viewport_cmd_to(window.viewport_id, egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd_to(
                window.viewport_id,
                egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Informational,
                ),
            );
        }
    }

    pub(crate) fn handle_new_window(&mut self, ctx: &egui::Context) {
        let window_id = self.state.new_document_window();
        let viewport_id = document_viewport_id(window_id);
        ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}
