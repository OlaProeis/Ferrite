//! Central panel rendering for the Ferrite application.
//!
//! This module renders the main editor content area including the tab bar,
//! editor widget (raw/rendered/split views), CSV viewer, tree viewer,
//! minimap, and navigation buttons.

use super::helpers::{char_index_to_line_col, get_formatting_state_for, modifier_symbol};
use super::types::{DeferredFormatAction, HeadingNavRequest};
use super::FerriteApp;
use crate::config::{ShortcutCommand, Theme, ViewMode};
use crate::editor::{
    cleanup_ferrite_editor, DocumentOutline, EditorWidget, FindReplacePanel, Minimap,
    SearchHighlights, SemanticMinimap,
};
use crate::markdown::{
    apply_raw_format, cleanup_rendered_editor_memory, get_structured_file_type,
    get_tabular_file_type, CsvViewer, CsvViewerState, EditorMode, FormattingState, MarkdownEditor,
    MarkdownFormatCommand, TreeViewer, TreeViewerState, WikilinkContext,
};
#[allow(unused_imports)]
use crate::preview::SyncScrollState;
use crate::state::{FileType, PdfViewerState, PendingAction, Selection, SpecialTabKind, TabKind};
use crate::theme::ThemeColors;
use crate::ui::{FileOperationResult, FormatToolbar, GoToLineResult, RibbonAction};
use eframe::egui;
use log::{debug, info, trace, warn};
use rust_i18n::t;
use std::collections::HashMap;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Image Viewer Texture Cache
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ImageViewerTexture {
    texture: Option<egui::TextureHandle>,
    width: u32,
    height: u32,
    error: Option<String>,
}

fn load_viewer_image(ctx: &egui::Context, path: &Path) -> Result<ImageViewerTexture, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {}", e))?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<egui::Color32> = rgba
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_image = egui::ColorImage {
        size: [width as usize, height as usize],
        pixels,
    };

    let texture_name = format!("img_viewer_{}", path.display());
    let texture = ctx.load_texture(&texture_name, color_image, egui::TextureOptions::LINEAR);

    Ok(ImageViewerTexture {
        texture: Some(texture),
        width,
        height,
        error: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// PDF Viewer Texture Cache
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct PdfPageTexture {
    texture: Option<egui::TextureHandle>,
    width: u32,
    height: u32,
    page_index: usize,
    zoom: f32,
    error: Option<String>,
}

fn render_pdf_page(
    ctx: &egui::Context,
    path: &Path,
    page_index: usize,
    zoom: f32,
) -> PdfPageTexture {
    use hayro::hayro_interpret::hayro_syntax::Pdf;
    use hayro::hayro_interpret::InterpreterSettings;
    use hayro::RenderSettings;

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return PdfPageTexture {
                texture: None,
                width: 0,
                height: 0,
                page_index,
                zoom,
                error: Some(format!("Failed to read file: {}", e)),
            }
        }
    };

    let pdf_data = std::sync::Arc::new(bytes);
    let pdf = match Pdf::new(pdf_data) {
        Ok(p) => p,
        Err(e) => {
            return PdfPageTexture {
                texture: None,
                width: 0,
                height: 0,
                page_index,
                zoom,
                error: Some(format!("Failed to parse PDF: {:?}", e)),
            }
        }
    };

    let pages = pdf.pages();
    if page_index >= pages.len() {
        return PdfPageTexture {
            texture: None,
            width: 0,
            height: 0,
            page_index,
            zoom,
            error: Some(format!(
                "Page {} out of range (total: {})",
                page_index + 1,
                pages.len()
            )),
        };
    }

    let page = &pages[page_index];
    let interpreter_settings = InterpreterSettings::default();
    let render_settings = RenderSettings {
        x_scale: zoom,
        y_scale: zoom,
        bg_color: {
            use hayro::vello_cpu::color::{AlphaColor, Srgb};
            AlphaColor::<Srgb>::new([1.0, 1.0, 1.0, 1.0])
        },
        ..Default::default()
    };

    let pixmap = hayro::render(page, &interpreter_settings, &render_settings);
    let width = pixmap.width() as u32;
    let height = pixmap.height() as u32;

    let rgba_data = pixmap.data_as_u8_slice();
    let pixels: Vec<egui::Color32> = rgba_data
        .chunks_exact(4)
        .map(|c| egui::Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
        .collect();

    let color_image = egui::ColorImage {
        size: [width as usize, height as usize],
        pixels,
    };

    let texture_name = format!("pdf_page_{}_{}_{:.2}", path.display(), page_index, zoom);
    let texture = ctx.load_texture(&texture_name, color_image, egui::TextureOptions::LINEAR);

    PdfPageTexture {
        texture: Some(texture),
        width,
        height,
        page_index,
        zoom,
        error: None,
    }
}

impl FerriteApp {
    /// Render the central panel containing tabs and editor content.
    ///
    /// Returns a deferred format action if one was requested.
    pub(crate) fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        is_dark: bool,
    ) -> Option<DeferredFormatAction> {
        let zen_mode = self.state.is_zen_mode();
        let mut deferred_format_action: Option<DeferredFormatAction> = None;
        let mut pending_wikilink_target: Option<String> = None;

        // Get the theme-appropriate fill color from the current visuals
        let fill_color = ctx.style().visuals.panel_fill;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::Margin::ZERO).fill(fill_color))
            .show(ctx, |ui| {
            // Tab bar - uses custom wrapping layout for multi-line support
            // Hidden in Zen Mode for distraction-free editing
            let mut tab_to_close: Option<usize> = None;
            let mut tab_swap: Option<(usize, usize)> = None;

            if !zen_mode {
                // Collect tab info first to avoid borrow issues
                let tab_count = self.state.tab_count();
                let active_index = self.state.active_tab_index();
                let tab_titles: Vec<(usize, String, bool)> = (0..tab_count)
                    .filter_map(|i| {
                        self.state.tab(i).map(|tab| (i, tab.title(), i == active_index))
                    })
                    .collect();

                // Custom wrapping tab bar
                let available_width = ui.available_width();
                let tab_height = 24.0;
                let tab_spacing = 4.0;
                let close_btn_width = 18.0;
                let tab_padding = 16.0; // horizontal padding inside tab
                let min_text_width = 60.0;

                // Pre-calculate tab widths using actual text measurement
                // This ensures consistent sizing between layout and render passes
                let tab_widths: Vec<f32> = tab_titles
                    .iter()
                    .map(|(_, title, _)| {
                        let text_galley = ui.fonts(|f| {
                            f.layout_no_wrap(
                                title.clone(),
                                egui::FontId::default(),
                                egui::Color32::WHITE // color doesn't affect measurement
                            )
                        });
                        let text_width = text_galley.size().x.max(min_text_width);
                        text_width + close_btn_width + tab_padding
                    })
                    .collect();

                // Calculate tab positions for layout
                let mut current_x = 0.0;
                let mut current_row = 0;
                let mut tab_positions: Vec<(f32, usize)> = Vec::new(); // (x position, row)

                for tab_width in &tab_widths {
                    // Check if we need to wrap to next row
                    if current_x + tab_width > available_width && current_x > 0.0 {
                        current_x = 0.0;
                        current_row += 1;
                    }

                    tab_positions.push((current_x, current_row));
                    current_x += tab_width + tab_spacing;
                }

                // Add position for the + button
                let plus_btn_width = 24.0;
                if current_x + plus_btn_width > available_width && current_x > 0.0 {
                    current_row += 1;
                }
                let total_rows = current_row + 1;
                let total_height = (total_rows as f32) * (tab_height + 2.0);

                // Allocate space for all tab rows
                let (tab_bar_rect, _) = ui.allocate_exact_size(
                    egui::vec2(available_width, total_height),
                    egui::Sense::hover()
                );

                // Render tabs
                let is_dark = ui.visuals().dark_mode;
                let selected_bg = ui.visuals().selection.bg_fill;
                let hover_bg = if is_dark {
                    egui::Color32::from_rgb(60, 60, 70)
                } else {
                    egui::Color32::from_rgb(220, 220, 230)
                };
                let text_color = ui.visuals().text_color();

                for (idx, (((tab_idx, title, selected), (x_pos, row)), tab_width)) in tab_titles
                    .iter()
                    .zip(tab_positions.iter())
                    .zip(tab_widths.iter())
                    .enumerate() {
                    // Use pre-calculated tab width for consistency
                    let tab_width = *tab_width;

                    let tab_rect = egui::Rect::from_min_size(
                        tab_bar_rect.min + egui::vec2(*x_pos, (*row as f32) * (tab_height + 2.0)),
                        egui::vec2(tab_width, tab_height)
                    );

                    // Tab interaction - support both click and drag for reordering
                    let tab_response = ui.interact(
                        tab_rect,
                        egui::Id::new("tab").with(idx),
                        egui::Sense::click_and_drag()
                    );

                    // Handle drag-and-drop for tab reordering
                    if tab_response.dragged() {
                        egui::DragAndDrop::set_payload(ui.ctx(), *tab_idx);
                        // Show drag cursor
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }

                    // Check if another tab is being dropped on this one
                    let mut is_drop_target = false;
                    if tab_response.hovered() && ui.ctx().input(|i| i.pointer.any_released()) {
                        if
                            let Some(dragged_tab_idx) = egui::DragAndDrop::payload::<usize>(
                                ui.ctx()
                            )
                        {
                            let dragged_idx = *dragged_tab_idx;
                            if dragged_idx != *tab_idx {
                                tab_swap = Some((dragged_idx, *tab_idx));
                            }
                        }
                    }
                    if tab_response.hovered() {
                        if let Some(_) = egui::DragAndDrop::payload::<usize>(ui.ctx()) {
                            is_drop_target = true;
                        }
                    }

                    // Draw tab background
                    if is_drop_target {
                        // Show drop indicator
                        let indicator_color = if is_dark {
                            egui::Color32::from_rgb(80, 120, 200)
                        } else {
                            egui::Color32::from_rgb(100, 150, 230)
                        };
                        ui.painter().rect_filled(tab_rect, 4.0, indicator_color);
                    } else if *selected {
                        ui.painter().rect_filled(tab_rect, 4.0, selected_bg);
                    } else if tab_response.hovered() {
                        ui.painter().rect_filled(tab_rect, 4.0, hover_bg);
                    }

                    // Draw tab title - use available width minus close button and padding
                    let title_available_width = tab_width - close_btn_width - tab_padding;
                    let title_rect = egui::Rect::from_min_size(
                        tab_rect.min + egui::vec2(8.0, 4.0),
                        egui::vec2(title_available_width, tab_height - 8.0)
                    );
                    ui.painter().text(
                        title_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        title,
                        egui::FontId::default(),
                        text_color
                    );

                    // Draw close button
                    let close_rect = egui::Rect::from_min_size(
                        egui::pos2(tab_rect.right() - close_btn_width - 4.0, tab_rect.top() + 4.0),
                        egui::vec2(close_btn_width, tab_height - 8.0)
                    );
                    let close_response = ui.interact(
                        close_rect,
                        egui::Id::new("tab_close").with(idx),
                        egui::Sense::click()
                    );

                    let close_color = if close_response.hovered() {
                        egui::Color32::from_rgb(220, 80, 80)
                    } else {
                        text_color
                    };
                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::default(),
                        close_color
                    );

                    // Handle interactions
                    if tab_response.clicked() && !close_response.hovered() {
                        self.state.set_active_tab(*tab_idx);
                        self.pending_cjk_check = true;
                    }
                    if close_response.clicked() || tab_response.middle_clicked() {
                        tab_to_close = Some(*tab_idx);
                    }
                    if close_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    } else if tab_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }

                // Draw + button - use pre-calculated tab widths for consistency
                let plus_x = if tab_positions.is_empty() || tab_widths.is_empty() {
                    0.0
                } else {
                    let last_pos = tab_positions.last().unwrap();
                    let last_width = *tab_widths.last().unwrap();

                    if last_pos.0 + last_width + tab_spacing + plus_btn_width > available_width {
                        0.0 // Wrap to next row
                    } else {
                        last_pos.0 + last_width + tab_spacing
                    }
                };
                let plus_row = if tab_positions.is_empty() {
                    0
                } else if plus_x == 0.0 && !tab_positions.is_empty() {
                    tab_positions.last().unwrap().1 + 1
                } else {
                    tab_positions.last().unwrap().1
                };

                let plus_rect = egui::Rect::from_min_size(
                    tab_bar_rect.min + egui::vec2(plus_x, (plus_row as f32) * (tab_height + 2.0)),
                    egui::vec2(plus_btn_width, tab_height)
                );
                let plus_response = ui.interact(
                    plus_rect,
                    egui::Id::new("new_tab_btn"),
                    egui::Sense::click()
                );

                if plus_response.hovered() {
                    ui.painter().rect_filled(plus_rect, 4.0, hover_bg);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.painter().text(
                    plus_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::default(),
                    text_color
                );
                if plus_response.clicked() {
                    self.state.new_tab();
                }
                plus_response.on_hover_text(t!("tooltip.new_tab").to_string());

                // Handle tab swap (drag-and-drop reorder)
                if let Some((from_idx, to_idx)) = tab_swap {
                    if self.state.swap_tabs(from_idx, to_idx) {
                        debug!("Reordered tabs: {} <-> {}", from_idx, to_idx);
                    }
                }

                // Handle tab close action
                if let Some(index) = tab_to_close {
                    // Get tab_id before closing for viewer state cleanup
                    let tab_id = self.state
                        .tabs()
                        .get(index)
                        .map(|t| t.id);
                    self.state.close_tab(index);
                    if let Some(id) = tab_id {
                        self.cleanup_tab_state(id, Some(ui.ctx()));
                    }
                }

                // Draw a visible separator line between tabs and editor
                // Uses stronger contrast than default egui separator for accessibility
                ui.add_space(2.0);
                {
                    let separator_color = if is_dark {
                        egui::Color32::from_rgb(60, 60, 60)
                    } else {
                        egui::Color32::from_rgb(160, 160, 160) // ~3.2:1 contrast on white
                    };
                    let rect = ui.available_rect_before_wrap();
                    let y = rect.min.y;
                    ui.painter().line_segment(
                        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                        egui::Stroke::new(1.0, separator_color)
                    );
                }
                ui.add_space(3.0);
            } // End of tab bar (hidden in Zen Mode)

            // Check if active tab is a special tab (settings, about, etc.)
            // If so, render the special tab content instead of the editor
            let active_tab_kind = self.state
                .active_tab()
                .map(|t| t.kind.clone())
                .unwrap_or(TabKind::Document);

            // Check if the active tab is loading or has a load error
            let active_tab_content = self.state
                .active_tab()
                .map(|t| t.tab_content.clone());

            if let TabKind::Special(special_kind) = active_tab_kind {
                self.render_special_tab_content(ui, special_kind);
            } else if matches!(active_tab_kind, TabKind::ImageViewer(_)) {
                self.render_image_viewer_tab(ui, ctx);
            } else if matches!(active_tab_kind, TabKind::PdfViewer(_)) {
                self.render_pdf_viewer_tab(ui, ctx);
            } else if let Some(crate::state::TabContent::Loading(ref progress)) = active_tab_content {
                self.render_loading_tab(ui, progress);
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            } else if let Some(crate::state::TabContent::Error(ref error)) = active_tab_content {
                Self::render_load_error_tab(ui, error);
            } else {
                // Editor widget - extract settings values to avoid borrow conflicts
                let font_size = self.state.settings.font_size;
                let font_family = self.state.settings.font_family.clone();
                let word_wrap = self.state.settings.word_wrap;
                let theme = self.state.settings.theme;
                let show_line_numbers = self.state.settings.show_line_numbers;
                let auto_close_brackets = self.state.settings.auto_close_brackets;
                let vim_mode = self.state.settings.vim_mode;

                // Get theme colors for line number styling
                let theme_colors = ThemeColors::from_theme(theme, ui.visuals());

                // Prepare search highlights if find panel is open
                let search_highlights = if
                    self.state.ui.show_find_replace &&
                    !self.state.ui.find_state.matches.is_empty()
                {
                    let highlights = SearchHighlights {
                        matches: self.state.ui.find_state.matches.clone(),
                        current_match: self.state.ui.find_state.current_match,
                        scroll_to_match: self.state.ui.scroll_to_match,
                    };
                    // Clear scroll flag after using it
                    self.state.ui.scroll_to_match = false;
                    Some(highlights)
                } else {
                    None
                };

                // Extract pending scroll request before mutable borrow
                let scroll_to_line = self.pending_scroll_to_line.take();

                // Get tab metadata before mutable borrow
                let tab_info = self.state
                    .active_tab()
                    .map(|t| {
                        (
                            t.id,
                            t.view_mode,
                            t.path.as_ref().and_then(|p| get_structured_file_type(p)),
                            t.path.as_ref().and_then(|p| get_tabular_file_type(p)),
                            t.transient_highlight_range(),
                        )
                    });

                if
                    let Some((tab_id, view_mode, structured_type, tabular_type, transient_hl)) =
                        tab_info
                {
                    match view_mode {
                        ViewMode::Raw => {
                            // Raw mode: use the plain EditorWidget with optional minimap
                            let zen_max_column_width = self.state.settings.zen_max_column_width;
                            let max_line_width = self.state.settings.max_line_width;

                            // Capture scroll offset before mutable borrow for scroll detection
                            let prev_scroll_offset = self.state
                                .active_tab()
                                .map(|t| t.scroll_offset)
                                .unwrap_or(0.0);

                            // Get folding settings (before mutable borrow)
                            let folding_enabled = self.state.settings.folding_enabled;
                            let show_fold_indicators =
                                self.state.settings.folding_show_indicators && folding_enabled;
                            let fold_headings = self.state.settings.fold_headings;
                            let fold_code_blocks = self.state.settings.fold_code_blocks;
                            let fold_lists = self.state.settings.fold_lists;
                            let fold_indentation = self.state.settings.fold_indentation;

                            // Get bracket matching setting
                            let highlight_matching_pairs =
                                self.state.settings.highlight_matching_pairs;

                            // Get syntax highlighting settings
                            let syntax_highlighting_enabled =
                                self.state.settings.syntax_highlighting_enabled;
                            let syntax_theme = if self.state.settings.syntax_theme.is_empty() {
                                None
                            } else {
                                Some(self.state.settings.syntax_theme.clone())
                            };
                            let default_syntax_language =
                                self.state.settings.default_syntax_language.clone();

                            // Get minimap settings (hidden in Zen Mode)
                            // Disable minimap for large files to avoid per-frame content iteration
                            let is_tab_large_file = self.state
                                .active_tab()
                                .map(|t| t.is_large_file())
                                .unwrap_or(false);
                            let minimap_enabled =
                                self.state.settings.minimap_enabled &&
                                !zen_mode &&
                                !is_tab_large_file;
                            let minimap_width = self.state.settings.minimap_width;
                            let minimap_mode = self.state.settings.minimap_mode;

                            // Check if file is markdown (for auto mode minimap selection)
                            // Check extension directly to avoid any caching issues
                            let is_markdown_file = self.state
                                .active_tab()
                                .map(|tab| {
                                    match &tab.path {
                                        Some(path) => {
                                            // Check extension directly
                                            let ext_result = path
                                                .extension()
                                                .and_then(|e| e.to_str())
                                                .map(
                                                    |ext|
                                                        ext.eq_ignore_ascii_case("md") ||
                                                        ext.eq_ignore_ascii_case("markdown")
                                                )
                                                .unwrap_or(false); // No extension = not markdown
                                            trace!(
                                                "Minimap file type check: path={:?}, ext={:?}, is_markdown={}",
                                                path.file_name(),
                                                path.extension(),
                                                ext_result
                                            );
                                            ext_result
                                        }
                                        None => {
                                            trace!(
                                                "Minimap file type check: unsaved file, defaulting to markdown"
                                            );
                                            true // Unsaved files default to markdown
                                        }
                                    }
                                })
                                .unwrap_or(true);

                            // Determine whether to use semantic minimap based on mode setting
                            let use_semantic_minimap = minimap_mode.use_semantic(is_markdown_file);

                            // Get tab data needed for minimap before mutable borrow
                            // For semantic: structure-based minimap with headings
                            // For pixel: code overview minimap
                            let semantic_minimap_data = if minimap_enabled && use_semantic_minimap {
                                self.state.active_tab().map(|t| {
                                    // Extract outline for semantic minimap
                                    let outline = crate::editor::extract_outline_for_file(
                                        &t.content,
                                        t.path.as_deref()
                                    );
                                    let total_lines = t.content.lines().count();
                                    (
                                        outline,
                                        t.scroll_offset,
                                        t.content_height,
                                        t.raw_line_height,
                                        t.cursor_position.0 + 1, // Convert 0-indexed to 1-indexed line
                                        total_lines,
                                    )
                                })
                            } else {
                                None
                            };

                            let pixel_minimap_data = if minimap_enabled && !use_semantic_minimap {
                                self.state
                                    .active_tab()
                                    .map(|t| {
                                        (
                                            t.content.clone(),
                                            t.scroll_offset,
                                            t.viewport_height,
                                            t.content_height,
                                            t.raw_line_height,
                                        )
                                    })
                            } else {
                                None
                            };

                            // Get search matches for pixel minimap visualization
                            let minimap_search_matches: Vec<(usize, usize)> = if
                                minimap_enabled &&
                                !use_semantic_minimap
                            {
                                self.state.ui.find_state.matches.clone()
                            } else {
                                Vec::new()
                            };
                            let minimap_current_match = self.state.ui.find_state.current_match;

                            // Track minimap scroll request
                            let mut minimap_nav_request: Option<HeadingNavRequest> = None;
                            let mut minimap_scroll_to_offset: Option<f32> = None;
                            let mut ime_text_for_font_loading: Option<String> = None;

                            // Clone tab path before mutable borrow for syntax highlighting
                            let tab_path_for_syntax = self.state
                                .active_tab()
                                .and_then(|t| t.path.clone());

                            // Collect diagnostics for the active tab's file (cloned to avoid borrow conflict)
                            let tab_diagnostics: Vec<crate::lsp::state::DiagnosticEntry> =
                                tab_path_for_syntax
                                    .as_ref()
                                    .and_then(|p| self.state.diagnostics.get(p))
                                    .map(|d| d.to_vec())
                                    .unwrap_or_default();

                            // Raw mode: FerriteEditor owns undo via EditHistory
                            // — no central-panel snapshot needed.

                            // Format toolbar state (markdown files only, hidden in Zen Mode)
                            let show_format_toolbar = is_markdown_file && !zen_mode;
                            let format_toolbar_expanded = self.state.settings.format_toolbar_visible;
                            let raw_formatting_state = if show_format_toolbar {
                                self.state.active_tab().map(|tab| {
                                    get_formatting_state_for(
                                        &tab.content,
                                        tab.cursor_position.0,
                                        tab.cursor_position.1,
                                    )
                                })
                            } else {
                                None
                            };
                            let mut format_bar_toggled = false;
                            let mut format_bar_action: Option<RibbonAction> = None;
                            let mut vim_label_for_status: Option<&'static str> = None;
                            let mut content_changed_in_editor = false;

                            if let Some(tab) = self.state.active_tab_mut() {
                                // Update folds if dirty
                                if folding_enabled && tab.folds_dirty() {
                                    tab.update_folds(
                                        fold_headings,
                                        fold_code_blocks,
                                        fold_lists,
                                        fold_indentation
                                    );
                                }

                                // Calculate format toolbar height
                                let format_bar_height = if show_format_toolbar {
                                    if format_toolbar_expanded { 32.0 } else { 18.0 }
                                } else {
                                    0.0
                                };

                                // Calculate layout for editor and minimap
                                let total_rect = ui.available_rect_before_wrap();

                                // Reserve space for format toolbar at the bottom
                                let content_rect = egui::Rect::from_min_max(
                                    total_rect.min,
                                    egui::pos2(total_rect.max.x, total_rect.max.y - format_bar_height),
                                );
                                let format_bar_rect = if show_format_toolbar {
                                    Some(egui::Rect::from_min_max(
                                        egui::pos2(total_rect.min.x, total_rect.max.y - format_bar_height),
                                        total_rect.max,
                                    ))
                                } else {
                                    None
                                };

                                let editor_width = if minimap_enabled {
                                    content_rect.width() - minimap_width
                                } else {
                                    content_rect.width()
                                };

                                let editor_rect = egui::Rect::from_min_size(
                                    content_rect.min,
                                    egui::vec2(editor_width, content_rect.height())
                                );
                                let minimap_rect = if minimap_enabled {
                                    Some(
                                        egui::Rect::from_min_size(
                                            egui::pos2(
                                                content_rect.min.x + editor_width,
                                                content_rect.min.y
                                            ),
                                            egui::vec2(minimap_width, content_rect.height())
                                        )
                                    )
                                } else {
                                    None
                                };

                                // Allocate the total area
                                ui.allocate_rect(total_rect, egui::Sense::hover());

                                // Show editor in its region
                                let mut editor_ui = ui.child_ui(
                                    editor_rect,
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    None
                                );

                                let editor_widget_id = egui::Id::new("main_editor_raw").with(tab.id);
                                 tab.prepare_undo_snapshot_hashed();
                                 let mut editor = EditorWidget::new(tab)
                                    .font_size(font_size)
                                    .font_family(font_family.clone())
                                    .word_wrap(word_wrap)
                                    .show_line_numbers(show_line_numbers && !zen_mode) // Hide line numbers in Zen Mode
                                    .show_fold_indicators(show_fold_indicators && !zen_mode) // Hide in Zen Mode
                                    .theme_colors(theme_colors.clone())
                                    .id(editor_widget_id)
                                    .scroll_to_line(scroll_to_line)
                                    .zen_mode(zen_mode, zen_max_column_width)
                                    .max_line_width(max_line_width) // Apply when not in Zen Mode
                                    .transient_highlight(transient_hl)
                                    .highlight_matching_pairs(highlight_matching_pairs)
                                    .syntax_highlighting(
                                        syntax_highlighting_enabled,
                                        tab_path_for_syntax.clone(),
                                        is_dark
                                    )
                                    .default_syntax_language(default_syntax_language.clone())
                                    .syntax_theme(syntax_theme.clone())
                                    .auto_close_brackets(auto_close_brackets)
                                    .vim_mode(vim_mode)
                                    .diagnostics(tab_diagnostics.clone());

                                // Add search highlights if available
                                if let Some(highlights) = search_highlights.clone() {
                                    editor = editor.search_highlights(highlights);
                                }

                                let editor_output = editor.show(&mut editor_ui);

                                vim_label_for_status = editor_output.vim_mode_label;

                                // NOTE: Fold toggle is handled internally by FerriteEditor and synced
                                // back to Tab in widget.rs. We just need to check if a fold was toggled
                                // for any post-processing (currently none needed).
                                if editor_output.fold_toggle_line.is_some() {
                                    // Fold state already synced from FerriteEditor to Tab in widget.rs
                                    log::debug!(
                                        "Fold toggled at line {:?}",
                                        editor_output.fold_toggle_line
                                    );
                                }

                                // Handle transient highlight expiry
                                if tab.has_transient_highlight() {
                                    // Clear on edit
                                    if editor_output.changed {
                                        tab.on_edit_event();
                                        debug!("Cleared transient highlight due to edit");
                                    } else if
                                        // Clear on scroll (after the initial programmatic scroll)
                                        (tab.scroll_offset - prev_scroll_offset).abs() > 1.0
                                    {
                                        tab.on_scroll_event();
                                        // Note: on_scroll_event handles the guard for initial scroll
                                    } else if
                                        // Clear on any mouse click in the editor
                                        ui.input(|i| i.pointer.any_click())
                                    {
                                        tab.on_click_event();
                                        debug!("Cleared transient highlight due to click");
                                    }
                                }

                                if editor_output.changed {
                                    tab.record_edit_from_snapshot();
                                    tab.mark_content_edited();
                                    debug!("Content modified in raw editor, recorded for undo");
                                    if folding_enabled {
                                        tab.mark_folds_dirty();
                                    }
                                    content_changed_in_editor = true;
                                }

                                // Capture IME committed text for font loading (processed after tab borrow ends)
                                if editor_output.ime_committed_text.is_some() {
                                    ime_text_for_font_loading =
                                        editor_output.ime_committed_text.clone();
                                }

                                // Handle Ctrl+Click to add cursor
                                if let Some(click_pos) = editor_output.ctrl_click_pos {
                                    tab.add_cursor(click_pos);
                                    debug!(
                                        "{}+Click: added cursor at position {}, now {} cursor(s)",
                                        modifier_symbol(),
                                        click_pos,
                                        tab.cursor_count()
                                    );
                                }

                                // Show minimap if enabled
                                if let Some(minimap_rect) = minimap_rect {
                                    let mut minimap_ui = ui.child_ui(
                                        minimap_rect,
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        None
                                    );

                                    // Use semantic minimap for markdown files
                                    if
                                        let Some(
                                            (
                                                outline,
                                                scroll_offset,
                                                content_height,
                                                line_height,
                                                current_line,
                                                total_lines,
                                            ),
                                        ) = semantic_minimap_data
                                    {
                                        let semantic_minimap = SemanticMinimap::new(&outline.items)
                                            .width(minimap_width)
                                            .scroll_offset(scroll_offset)
                                            .content_height(content_height)
                                            .line_height(line_height)
                                            .current_line(Some(current_line))
                                            .total_lines(total_lines)
                                            .theme_colors(theme_colors.clone());

                                        let minimap_output = semantic_minimap.show(&mut minimap_ui);

                                        // Handle semantic minimap navigation with text matching
                                        if let Some(target_line) = minimap_output.scroll_to_line {
                                            minimap_nav_request = Some(HeadingNavRequest {
                                                line: target_line,
                                                char_offset: minimap_output.scroll_to_char,
                                                title: minimap_output.scroll_to_title,
                                                level: minimap_output.scroll_to_level,
                                            });
                                        }
                                    } else if
                                        // Use pixel minimap for non-markdown files
                                        let Some(
                                            (
                                                content,
                                                scroll_offset,
                                                viewport_height,
                                                content_height,
                                                line_height,
                                            ),
                                        ) = pixel_minimap_data
                                    {
                                        let mut minimap = Minimap::new(&content)
                                            .width(minimap_width)
                                            .scroll_offset(scroll_offset)
                                            .viewport_height(viewport_height)
                                            .content_height(content_height)
                                            .line_height(line_height)
                                            .theme_colors(theme_colors.clone());

                                        // Add search highlights to pixel minimap
                                        if !minimap_search_matches.is_empty() {
                                            minimap = minimap
                                                .search_highlights(&minimap_search_matches)
                                                .current_match(minimap_current_match);
                                        }

                                        let minimap_output = minimap.show(&mut minimap_ui);

                                        // Handle pixel minimap navigation
                                        if
                                            let Some(target_offset) =
                                                minimap_output.scroll_to_offset
                                        {
                                            minimap_scroll_to_offset = Some(target_offset);
                                        }
                                    }
                                }

                                // Format toolbar at bottom of raw editor
                                if let Some(bar_rect) = format_bar_rect {
                                    let mut bar_ui = ui.child_ui(
                                        bar_rect,
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        None,
                                    );
                                    let toolbar_output = FormatToolbar::show(
                                        &mut bar_ui,
                                        format_toolbar_expanded,
                                        raw_formatting_state.as_ref(),
                                        true,
                                        is_dark,
                                    );
                                    if toolbar_output.toggle_visibility {
                                        format_bar_toggled = true;
                                    }
                                    if toolbar_output.action.is_some() {
                                        format_bar_action = toolbar_output.action;
                                    }
                                }
                            }

                            // Update Vim mode indicator (after tab borrow ends)
                            self.state.ui.vim_mode_indicator = vim_label_for_status;

                            // Recompute search matches when content changes while find panel is open.
                            // Byte positions in find_state.matches become stale after buffer edits.
                            if content_changed_in_editor
                                && self.state.ui.show_find_replace
                                && !self.state.ui.find_state.search_term.is_empty()
                            {
                                if let Some(content) = self.state.active_tab().map(|t| t.content.clone()) {
                                    self.state.ui.find_state.find_matches(&content);
                                }
                            }

                            // Handle format toolbar toggle (after mutable borrow ends)
                            if format_bar_toggled {
                                self.state.settings.format_toolbar_visible = !self.state.settings.format_toolbar_visible;
                                self.state.mark_settings_dirty();
                            }

                            // Handle format toolbar actions
                            // IMPORTANT: Use deferred format action with pre-captured selection
                            // to ensure formatting is applied correctly even if button click steals focus
                            if let Some(action) = format_bar_action {
                                match action {
                                    RibbonAction::Format(cmd) => {
                                        // Capture selection now before any focus changes
                                        use crate::editor::get_ferrite_editor_mut;
                                        let tab_id = self.state.active_tab().map(|t| t.id);
                                        let selection = tab_id.and_then(|id| {
                                            get_ferrite_editor_mut(ctx, id, |editor| {
                                                let sel = editor.selection();
                                                let (start, end) = sel.ordered();
                                                let line_count = editor.buffer().line_count();
                                                let start_line = start.line.min(line_count.saturating_sub(1));
                                                let end_line = end.line.min(line_count.saturating_sub(1));
                                                let start_line_char = editor.buffer().try_line_to_char(start_line).unwrap_or(0);
                                                let end_line_char = editor.buffer().try_line_to_char(end_line).unwrap_or(0);
                                                let start_char = start_line_char + start.column;
                                                let end_char = end_line_char + end.column;
                                                (start_char, end_char)
                                            })
                                        });
                                        deferred_format_action = Some(DeferredFormatAction { cmd, selection });
                                    }
                                    RibbonAction::InsertToc => {
                                        self.handle_insert_toc();
                                    }
                                    _ => {}
                                }
                            }

                            // Apply minimap navigation request (after mutable borrow ends)
                            if let Some(nav) = minimap_nav_request {
                                self.navigate_to_heading(nav);
                                ui.ctx().request_repaint();
                            }
                            if let Some(scroll_offset) = minimap_scroll_to_offset {
                                if let Some(tab) = self.state.active_tab_mut() {
                                    tab.pending_scroll_offset = Some(scroll_offset);
                                    ui.ctx().request_repaint();
                                }
                            }

                            // Load CJK / complex script fonts if IME committed text needs them
                            if let Some(ref ime_text) = ime_text_for_font_loading {
                                let _ = self.load_cjk_fonts_for_content(ctx, ime_text);
                                let _ = self.load_complex_script_fonts_for_content(ctx, ime_text);
                            }
                        }
                        ViewMode::Split => {
                            // Split view: raw editor on left, rendered preview on right
                            // Not available for structured files

                            if structured_type.is_some() {
                                // Structured (JSON/YAML/TOML) files don't support split view,
                                // switch to Raw mode. CSV/TSV files DO support split view.
                                if let Some(tab) = self.state.active_tab_mut() {
                                    tab.view_mode = ViewMode::Raw;
                                }
                            } else {
                                // Get split ratio before mutable borrow
                                let split_ratio = self.state
                                    .active_tab()
                                    .map(|t| t.split_ratio)
                                    .unwrap_or(0.5);
                                let available_width = ui.available_width();
                                let _available_height = ui.available_height(); // For reference (using rect-based layout)
                                let splitter_width = 8.0; // Width of the draggable splitter area

                                // Get Zen Mode settings
                                let zen_max_column_width = self.state.settings.zen_max_column_width;

                                // Get minimap settings (hidden in Zen Mode for distraction-free editing)
                                // Disable minimap for large files to avoid per-frame content iteration
                                let is_tab_large_file = self.state
                                    .active_tab()
                                    .map(|t| t.is_large_file())
                                    .unwrap_or(false);
                                let minimap_enabled =
                                    self.state.settings.minimap_enabled &&
                                    !zen_mode &&
                                    !is_tab_large_file;
                                let minimap_width = self.state.settings.minimap_width;
                                let minimap_mode = self.state.settings.minimap_mode;
                                let effective_minimap_width = if minimap_enabled {
                                    minimap_width
                                } else {
                                    0.0
                                };

                                // Calculate widths: left pane gets split_ratio of (total - splitter - minimap)
                                let content_width =
                                    available_width - splitter_width - effective_minimap_width;
                                let left_width = content_width * split_ratio;
                                let right_width = content_width * (1.0 - split_ratio);

                                // Get folding settings (fold indicators hidden in Zen Mode)
                                let folding_enabled = self.state.settings.folding_enabled;
                                let show_fold_indicators =
                                    self.state.settings.folding_show_indicators &&
                                    folding_enabled &&
                                    !zen_mode;
                                let fold_headings = self.state.settings.fold_headings;
                                let fold_code_blocks = self.state.settings.fold_code_blocks;
                                let fold_lists = self.state.settings.fold_lists;
                                let fold_indentation = self.state.settings.fold_indentation;

                                // Get bracket matching setting
                                let highlight_matching_pairs =
                                    self.state.settings.highlight_matching_pairs;

                                // Get syntax highlighting settings
                                let syntax_highlighting_enabled =
                                    self.state.settings.syntax_highlighting_enabled;
                                let syntax_theme = if self.state.settings.syntax_theme.is_empty() {
                                    None
                                } else {
                                    Some(self.state.settings.syntax_theme.clone())
                                };
                                let default_syntax_language =
                                    self.state.settings.default_syntax_language.clone();

                                // Get line width setting
                                let max_line_width = self.state.settings.max_line_width;

                                // Get paragraph indent setting (CJK typography)
                                let paragraph_indent = self.state.settings.paragraph_indent;

                                // Get header spacing setting (Markdown rendering)
                                let header_spacing = self.state.settings.header_spacing;

                                // Get path for syntax highlighting
                                let tab_path_for_syntax = self.state
                                    .active_tab()
                                    .and_then(|t| t.path.clone());

                                // Collect diagnostics for the active tab's file
                                let tab_diagnostics: Vec<crate::lsp::state::DiagnosticEntry> =
                                    tab_path_for_syntax
                                        .as_ref()
                                        .and_then(|p| self.state.diagnostics.get(p))
                                        .map(|d| d.to_vec())
                                        .unwrap_or_default();

                                // Check if file is markdown (for auto mode minimap selection)
                                let is_markdown_file_split = self.state
                                    .active_tab()
                                    .map(|tab| {
                                        match &tab.path {
                                            Some(path) => {
                                                path.extension()
                                                    .and_then(|e| e.to_str())
                                                    .map(
                                                        |ext|
                                                            ext.eq_ignore_ascii_case("md") ||
                                                            ext.eq_ignore_ascii_case("markdown")
                                                    )
                                                    .unwrap_or(false)
                                            }
                                            None => true, // Unsaved files default to markdown
                                        }
                                    })
                                    .unwrap_or(true);

                                // Determine whether to use semantic minimap based on mode setting
                                let use_semantic_minimap_split =
                                    minimap_mode.use_semantic(is_markdown_file_split);

                                // Get tab data for semantic minimap (when using semantic mode)
                                let semantic_minimap_data_split = if
                                    minimap_enabled &&
                                    use_semantic_minimap_split
                                {
                                    self.state.active_tab().map(|t| {
                                        let outline = crate::editor::extract_outline_for_file(
                                            &t.content,
                                            t.path.as_deref()
                                        );
                                        let total_lines = t.content.lines().count();
                                        (
                                            outline,
                                            t.scroll_offset,
                                            t.content_height,
                                            t.raw_line_height,
                                            t.cursor_position.0 + 1, // Convert 0-indexed to 1-indexed line
                                            total_lines,
                                        )
                                    })
                                } else {
                                    None
                                };

                                // Get tab data for pixel minimap (when using pixel mode)
                                let pixel_minimap_data_split = if
                                    minimap_enabled &&
                                    !use_semantic_minimap_split
                                {
                                    self.state
                                        .active_tab()
                                        .map(|t| {
                                            (
                                                t.content.clone(),
                                                t.scroll_offset,
                                                t.viewport_height,
                                                t.content_height,
                                                t.raw_line_height,
                                            )
                                        })
                                } else {
                                    None
                                };

                                // Track minimap navigation request
                                let mut minimap_nav_request: Option<HeadingNavRequest> = None;
                                let mut ime_text_for_font_loading_split: Option<String> = None;

                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Sync Scroll Setup (DISABLED - deferred to v0.3.0)
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Feature disabled until v0.3.0 - ignore settings value
                                let sync_scroll_enabled = false; // was: self.state.settings.sync_scroll_enabled

                                // Get or create sync scroll state for this tab
                                // Use longer debounce to prevent jitter (200ms instead of 16ms)
                                let sync_state = self.sync_scroll_states
                                    .entry(tab_id)
                                    .or_insert_with(|| {
                                        let mut state = SyncScrollState::new();
                                        // Disable smooth scrolling to reduce feedback loops
                                        state.set_enabled(sync_scroll_enabled);
                                        state
                                    });
                                sync_state.set_enabled(sync_scroll_enabled);

                                // Get pending scroll offsets for each pane (from previous frame's sync)
                                let pending_editor_scroll = if sync_scroll_enabled {
                                    sync_state.get_animated_raw_offset()
                                } else {
                                    None
                                };
                                // For preview, read and clear tab.pending_scroll_offset (set by sync code)
                                let pending_preview_scroll = if sync_scroll_enabled {
                                    self.state
                                        .active_tab_mut()
                                        .and_then(|t| t.pending_scroll_offset.take())
                                } else {
                                    None
                                };

                                let mut split_vim_label: Option<&'static str> = None;
                                let mut split_content_changed = false;

                                // Track scroll outputs from both panes
                                let mut editor_scroll_offset: Option<f32> = None;
                                let mut editor_content_height: Option<f32> = None;
                                let mut editor_first_visible_line: Option<usize> = None;
                                let mut editor_line_height: Option<f32> = None;
                                let mut editor_viewport_height: Option<f32> = None;
                                let mut preview_scroll_offset: Option<f32> = None;
                                let mut preview_content_height: Option<f32> = None;
                                let mut preview_viewport_height: Option<f32> = None;
                                let mut preview_line_mappings: Vec<crate::markdown::LineMapping> =
                                    Vec::new();

                                // Split left pane is Raw (FerriteEditor) — no
                                // central-panel undo snapshot needed for it.

                                // Format toolbar state for split view (markdown only)
                                let show_format_toolbar_split = is_markdown_file_split && !zen_mode;
                                let format_toolbar_expanded_split = self.state.settings.format_toolbar_visible;
                                let split_formatting_state = if show_format_toolbar_split {
                                    self.state.active_tab().map(|tab| {
                                        get_formatting_state_for(
                                            &tab.content,
                                            tab.cursor_position.0,
                                            tab.cursor_position.1,
                                        )
                                    })
                                } else {
                                    None
                                };
                                let mut format_bar_toggled_split = false;
                                let mut format_bar_action_split: Option<RibbonAction> = None;

                                // Calculate format toolbar height for split view
                                let format_bar_height_split = if show_format_toolbar_split {
                                    if format_toolbar_expanded_split { 32.0 } else { 18.0 }
                                } else {
                                    0.0
                                };

                                // Calculate explicit rectangles for split view layout
                                // Layout: [Editor] [Minimap] [Splitter] [Preview]
                                //         [Format Toolbar (bottom of left pane)]
                                let total_rect = ui.available_rect_before_wrap();
                                let left_editor_height = total_rect.height() - format_bar_height_split;
                                let left_rect = egui::Rect::from_min_size(
                                    total_rect.min,
                                    egui::vec2(left_width, left_editor_height)
                                );
                                let split_format_bar_rect = if show_format_toolbar_split {
                                    Some(egui::Rect::from_min_size(
                                        egui::pos2(total_rect.min.x, total_rect.min.y + left_editor_height),
                                        egui::vec2(left_width, format_bar_height_split),
                                    ))
                                } else {
                                    None
                                };
                                let minimap_rect = if minimap_enabled {
                                    Some(
                                        egui::Rect::from_min_size(
                                            egui::pos2(
                                                total_rect.min.x + left_width,
                                                total_rect.min.y
                                            ),
                                            egui::vec2(minimap_width, total_rect.height())
                                        )
                                    )
                                } else {
                                    None
                                };
                                let splitter_rect = egui::Rect::from_min_size(
                                    egui::pos2(
                                        total_rect.min.x + left_width + effective_minimap_width,
                                        total_rect.min.y
                                    ),
                                    egui::vec2(splitter_width, total_rect.height())
                                );
                                let right_rect = egui::Rect::from_min_size(
                                    egui::pos2(
                                        total_rect.min.x +
                                            left_width +
                                            effective_minimap_width +
                                            splitter_width,
                                        total_rect.min.y
                                    ),
                                    egui::vec2(right_width, total_rect.height())
                                );

                                // Allocate the entire area so egui knows we're using it
                                ui.allocate_rect(total_rect, egui::Sense::hover());

                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Left pane: Raw editor
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                let mut left_ui = ui.child_ui_with_id_source(
                                    left_rect,
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    "split_left_pane",
                                    None
                                );
                                if let Some(tab) = self.state.active_tab_mut() {
                                    // Update folds if dirty
                                    if folding_enabled && tab.folds_dirty() {
                                        tab.update_folds(
                                            fold_headings,
                                            fold_code_blocks,
                                            fold_lists,
                                            fold_indentation
                                        );
                                    }

                                    let editor_widget_id = egui::Id::new("split_editor_raw").with(tab.id);
                                     tab.prepare_undo_snapshot_hashed();
                                     let mut editor = EditorWidget::new(tab)
                                        .font_size(font_size)
                                        .font_family(font_family.clone())
                                        .word_wrap(word_wrap)
                                        .show_line_numbers(show_line_numbers && !zen_mode) // Hide in Zen Mode
                                        .show_fold_indicators(show_fold_indicators)
                                        .theme_colors(theme_colors.clone())
                                        .id(editor_widget_id)
                                        .scroll_to_line(scroll_to_line)
                                        .max_line_width(max_line_width)
                                        .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                        .transient_highlight(transient_hl)
                                        .highlight_matching_pairs(highlight_matching_pairs)
                                        .syntax_highlighting(
                                            syntax_highlighting_enabled,
                                            tab_path_for_syntax.clone(),
                                            is_dark
                                        )
                                        .default_syntax_language(default_syntax_language.clone())
                                        .syntax_theme(syntax_theme.clone())
                                        .auto_close_brackets(auto_close_brackets)
                                        .vim_mode(vim_mode)
                                        .diagnostics(tab_diagnostics.clone())
                                        .pending_sync_scroll_offset(pending_editor_scroll);

                                    // Add search highlights if available
                                    if let Some(highlights) = search_highlights.clone() {
                                        editor = editor.search_highlights(highlights);
                                    }

                                    let editor_output = editor.show(&mut left_ui);

                                    split_vim_label = editor_output.vim_mode_label;

                                    // Capture scroll metrics for sync scrolling
                                    editor_scroll_offset = Some(editor_output.scroll_offset);
                                    editor_content_height = Some(editor_output.content_height);
                                    editor_first_visible_line = Some(
                                        editor_output.first_visible_line
                                    );
                                    editor_line_height = Some(editor_output.line_height);
                                    editor_viewport_height = Some(editor_output.viewport_height);

                                    // NOTE: Fold toggle is handled internally by FerriteEditor and synced
                                    // back to Tab in widget.rs. We just need to check if a fold was toggled
                                    // for any post-processing (currently none needed).
                                    if editor_output.fold_toggle_line.is_some() {
                                        // Fold state already synced from FerriteEditor to Tab in widget.rs
                                        log::debug!(
                                            "Fold toggled at line {:?}",
                                            editor_output.fold_toggle_line
                                        );
                                    }

                                    // Handle transient highlight expiry
                                    if tab.has_transient_highlight() {
                                        if editor_output.changed {
                                            tab.on_edit_event();
                                        } else if left_ui.input(|i| i.pointer.any_click()) {
                                            tab.on_click_event();
                                        }
                                    }

                                    if editor_output.changed {
                                        // FerriteEditor records its own undo ops.
                                        if folding_enabled {
                                            tab.mark_folds_dirty();
                                        }
                                        split_content_changed = true;
                                    }

                                    // Capture IME committed text for font loading (processed after tab borrow ends)
                                    ime_text_for_font_loading_split =
                                        editor_output.ime_committed_text.clone();
                                }


                                // Update Vim mode indicator (after tab borrow ends)
                                self.state.ui.vim_mode_indicator = split_vim_label;


                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Minimap (between editor and splitter)
                                // Uses semantic minimap for markdown, pixel minimap for others
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                if let Some(mm_rect) = minimap_rect {
                                    let mut minimap_ui = ui.child_ui(
                                        mm_rect,
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        None
                                    );

                                    // Semantic minimap for markdown files
                                    if
                                        let Some(
                                            (
                                                outline,
                                                scroll_offset,
                                                content_height,
                                                line_height,
                                                current_line,
                                                total_lines,
                                            ),
                                        ) = semantic_minimap_data_split
                                    {
                                        let semantic_minimap = SemanticMinimap::new(&outline.items)
                                            .width(minimap_width)
                                            .scroll_offset(scroll_offset)
                                            .content_height(content_height)
                                            .line_height(line_height)
                                            .current_line(Some(current_line))
                                            .total_lines(total_lines)
                                            .theme_colors(theme_colors.clone());

                                        let minimap_output = semantic_minimap.show(&mut minimap_ui);

                                        // Handle semantic minimap navigation with text matching
                                        if let Some(target_line) = minimap_output.scroll_to_line {
                                            minimap_nav_request = Some(HeadingNavRequest {
                                                line: target_line,
                                                char_offset: minimap_output.scroll_to_char,
                                                title: minimap_output.scroll_to_title,
                                                level: minimap_output.scroll_to_level,
                                            });
                                        }
                                    } else if
                                        // Pixel minimap for non-markdown files
                                        let Some(
                                            (
                                                content,
                                                scroll_offset,
                                                viewport_height,
                                                content_height,
                                                line_height,
                                            ),
                                        ) = pixel_minimap_data_split
                                    {
                                        let minimap = Minimap::new(&content)
                                            .width(minimap_width)
                                            .scroll_offset(scroll_offset)
                                            .viewport_height(viewport_height)
                                            .content_height(content_height)
                                            .line_height(line_height)
                                            .theme_colors(theme_colors.clone());

                                        let minimap_output = minimap.show(&mut minimap_ui);

                                        // Handle pixel minimap scroll
                                        if let Some(offset) = minimap_output.scroll_to_offset {
                                            if let Some(tab) = self.state.active_tab_mut() {
                                                tab.pending_scroll_offset = Some(offset);
                                            }
                                            ui.ctx().request_repaint();
                                        }
                                    }
                                }

                                // Apply minimap navigation request
                                if let Some(nav) = minimap_nav_request {
                                    self.navigate_to_heading(nav);
                                    ui.ctx().request_repaint();
                                }

                                // Format toolbar at bottom of left (raw) pane in split view
                                if let Some(bar_rect) = split_format_bar_rect {
                                    let mut bar_ui = ui.child_ui(
                                        bar_rect,
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        None,
                                    );
                                    let toolbar_output = FormatToolbar::show(
                                        &mut bar_ui,
                                        format_toolbar_expanded_split,
                                        split_formatting_state.as_ref(),
                                        true,
                                        is_dark,
                                    );
                                    if toolbar_output.toggle_visibility {
                                        format_bar_toggled_split = true;
                                    }
                                    if toolbar_output.action.is_some() {
                                        format_bar_action_split = toolbar_output.action;
                                    }
                                }

                                // Handle format toolbar toggle/actions (split view)
                                // IMPORTANT: Use deferred format action with pre-captured selection
                                // to ensure formatting is applied correctly even if button click steals focus
                                if format_bar_toggled_split {
                                    self.state.settings.format_toolbar_visible = !self.state.settings.format_toolbar_visible;
                                    self.state.mark_settings_dirty();
                                }
                                if let Some(action) = format_bar_action_split {
                                    match action {
                                        RibbonAction::Format(cmd) => {
                                            // Capture selection now before any focus changes
                                            use crate::editor::get_ferrite_editor_mut;
                                            let tab_id = self.state.active_tab().map(|t| t.id);
                                            let selection = tab_id.and_then(|id| {
                                                get_ferrite_editor_mut(ctx, id, |editor| {
                                                    let sel = editor.selection();
                                                    let (start, end) = sel.ordered();
                                                    let line_count = editor.buffer().line_count();
                                                    let start_line = start.line.min(line_count.saturating_sub(1));
                                                    let end_line = end.line.min(line_count.saturating_sub(1));
                                                    let start_line_char = editor.buffer().try_line_to_char(start_line).unwrap_or(0);
                                                    let end_line_char = editor.buffer().try_line_to_char(end_line).unwrap_or(0);
                                                    let start_char = start_line_char + start.column;
                                                    let end_char = end_line_char + end.column;
                                                    (start_char, end_char)
                                                })
                                            });
                                            deferred_format_action = Some(DeferredFormatAction { cmd, selection });
                                        }
                                        RibbonAction::InsertToc => {
                                            self.handle_insert_toc();
                                        }
                                        _ => {}
                                    }
                                }

                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Splitter (draggable)
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                let splitter_response = ui.interact(
                                    splitter_rect,
                                    egui::Id::new("split_splitter"),
                                    egui::Sense::click_and_drag()
                                );

                                // Draw splitter visual
                                let is_dark = ui.visuals().dark_mode;
                                let splitter_color = if
                                    splitter_response.hovered() ||
                                    splitter_response.dragged()
                                {
                                    if is_dark {
                                        egui::Color32::from_rgb(100, 100, 120)
                                    } else {
                                        egui::Color32::from_rgb(140, 140, 160)
                                    }
                                } else if is_dark {
                                    egui::Color32::from_rgb(60, 60, 70)
                                } else {
                                    egui::Color32::from_rgb(180, 180, 190)
                                };

                                ui.painter().rect_filled(splitter_rect, 0.0, splitter_color);

                                // Draw grip lines in the center
                                let grip_color = if is_dark {
                                    egui::Color32::from_rgb(120, 120, 140)
                                } else {
                                    egui::Color32::from_rgb(100, 100, 120)
                                };
                                let center_x = splitter_rect.center().x;
                                let center_y = splitter_rect.center().y;
                                for i in -2..=2 {
                                    let y = center_y + (i as f32) * 6.0;
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(center_x - 2.0, y),
                                            egui::pos2(center_x + 2.0, y),
                                        ],
                                        egui::Stroke::new(1.0, grip_color)
                                    );
                                }

                                // Handle drag to resize
                                // Calculate ratio based on content_width (excluding minimap and splitter)
                                if splitter_response.dragged() {
                                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                        // The draggable area is content_width, and minimap is between editor and splitter
                                        // So we need to calculate ratio of (pointer - left - minimap) / content_width
                                        let drag_pos = pointer_pos.x - total_rect.left();
                                        // If minimap is enabled, the left pane ends at the minimap
                                        // The ratio should be based on how much of content_width is on the left
                                        let new_ratio = (
                                            drag_pos /
                                            (content_width +
                                                effective_minimap_width +
                                                splitter_width)
                                        ).clamp(0.15, 0.85);
                                        if let Some(tab) = self.state.active_tab_mut() {
                                            tab.set_split_ratio(new_ratio);
                                        }
                                    }
                                }

                                // Set resize cursor
                                if splitter_response.hovered() || splitter_response.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }

                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Right pane: Rendered preview (fully editable)
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                let mut right_ui = ui.child_ui_with_id_source(
                                    right_rect,
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    "split_right_pane",
                                    None
                                );

                                // Check if this is a CSV/TSV file for the right pane
                                if let Some(file_type) = tabular_type {
                                    // Tabular file: use the CsvViewer (read-only table view)
                                    let csv_state = self.csv_viewer_states
                                        .entry(tab_id)
                                        .or_default();
                                    let rainbow_columns = self.state.settings.csv_rainbow_columns;

                                    if let Some(tab) = self.state.active_tab_mut() {
                                        let _output = CsvViewer::new(
                                            &tab.content,
                                            file_type,
                                            csv_state
                                        )
                                            .font_size(font_size)
                                            .rainbow_columns(rainbow_columns)
                                            .show(&mut right_ui);
                                    }
                                } else {
                                    // Rendered pane - fully editable like the main Rendered mode
                                    // Edits here modify tab.content directly, with proper undo/redo support
                                    // Collect workspace root before mutable borrow
                                    let ws_root = self.state.workspace_root().cloned();
                                    let render_web_images = self.state.settings.render_web_images;
                                    if let Some(tab) = self.state.active_tab_mut() {

                                        // Build wikilink context from current file and workspace
                                        let wl_ctx = WikilinkContext {
                                            current_dir: tab.path
                                                .as_ref()
                                                .and_then(|p| p.parent().map(|d| d.to_path_buf())),
                                            workspace_root: ws_root.clone(),
                                        };

                                        let mut md_editor = MarkdownEditor::new(&mut tab.content)
                                            .mode(EditorMode::Rendered)
                                            .font_size(font_size)
                                            .font_family(font_family.clone())
                                            .word_wrap(word_wrap)
                                            .theme(theme)
                                            .max_line_width(max_line_width)
                                            .zen_mode(zen_mode, zen_max_column_width)
                                            .paragraph_indent(paragraph_indent)
                                            .header_spacing(header_spacing)
                                            .wikilink_context(wl_ctx)
                                            .render_web_images(render_web_images)
                                            .id(egui::Id::new("split_preview_rendered").with(tab.id))
                                            .pending_scroll_offset(pending_preview_scroll);
                                        if let Some(ref sh) = search_highlights {
                                            md_editor = md_editor.search_highlights(
                                                sh.matches.clone(),
                                                sh.current_match,
                                            );
                                        }
                                        let md_editor_output = md_editor.show(&mut right_ui);

                                        // Capture scroll metrics for sync scrolling
                                        preview_scroll_offset = Some(
                                            md_editor_output.scroll_offset
                                        );
                                        preview_content_height = Some(
                                            md_editor_output.content_height
                                        );
                                        preview_viewport_height = Some(
                                            md_editor_output.viewport_height
                                        );
                                        preview_line_mappings =
                                            md_editor_output.line_mappings.clone();

                                        if md_editor_output.changed {
                                            split_content_changed = true;
                                            debug!(
                                                "Content modified in split rendered pane, recorded for undo"
                                            );
                                        }

                                        // Don't update cursor_position in Split mode - the raw editor (left pane)
                                        // already maintains it via sync_cursor_from_primary(). Overwriting it here
                                        // would break line operations (delete line, move line) when editing the raw pane.
                                        // cursor_position is only needed for Rendered-only mode.

                                        // Update selection from focused element (for formatting toolbar)
                                        if let Some(focused) = md_editor_output.focused_element {
                                            if let Some((sel_start, sel_end)) = focused.selection {
                                                if sel_start != sel_end {
                                                    let abs_start = focused.start_char + sel_start;
                                                    let abs_end = focused.start_char + sel_end;
                                                    tab.selection = Some((abs_start, abs_end));
                                                } else {
                                                    tab.selection = None;
                                                }
                                            } else {
                                                tab.selection = None;
                                            }
                                        }

                                        // Handle wikilink navigation
                                        if let Some(target) = md_editor_output.wikilink_clicked {
                                            pending_wikilink_target = Some(target);
                                        }
                                    }
                                }

                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // Bidirectional Scroll Sync (after both panes render)
                                // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
                                // DEBOUNCED sync: Only sync AFTER scrolling stops to avoid
                                // fighting egui's scroll physics. Track scroll state and sync
                                // once when user stops scrolling.
                                //
                                // Strategy:
                                // 1. While scrolling: track which pane is being scrolled
                                // 2. After scroll stops (~100ms): do a single sync jump
                                // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                // Viewport-Based Scroll Sync (Task 36)
                                // Uses binary search + interpolation for smooth, accurate sync
                                // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                if sync_scroll_enabled && !preview_line_mappings.is_empty() {
                                    // Get scroll delta to detect active scrolling
                                    let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                                    let is_scrolling = scroll_delta.y.abs() > 0.5;

                                    // Determine which pane the mouse is over
                                    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
                                    let editor_area = egui::Rect::from_min_max(
                                        left_rect.min,
                                        egui::pos2(splitter_rect.min.x, left_rect.max.y)
                                    );
                                    let mouse_over_editor = mouse_pos
                                        .map(|p| editor_area.contains(p))
                                        .unwrap_or(false);
                                    let mouse_over_preview = mouse_pos
                                        .map(|p| right_rect.contains(p))
                                        .unwrap_or(false);

                                    // Get sync state for this tab
                                    let sync_state = self.sync_scroll_states
                                        .entry(tab_id)
                                        .or_insert_with(SyncScrollState::new);

                                    if is_scrolling {
                                        // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                        // Active scrolling: record origin and offset, sync to other pane
                                        // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                        if mouse_over_editor {
                                            if let Some(ed_offset) = editor_scroll_offset {
                                                // Record that editor is the scroll source
                                                sync_state.mark_scroll(
                                                    crate::preview::ScrollOrigin::Raw
                                                );
                                                sync_state.update_raw_offset(ed_offset);

                                                // Sync editor ΓåÆ preview (user scrolling editor)
                                                if let Some(first_line) = editor_first_visible_line {
                                                    let source_line = first_line.saturating_add(1);
                                                    let target_y =
                                                        SyncScrollState::source_line_to_preview_y(
                                                            source_line,
                                                            &preview_line_mappings
                                                        );

                                                    if let Some(tab) = self.state.active_tab_mut() {
                                                        tab.pending_scroll_offset = Some(target_y);
                                                    }
                                                }
                                            }
                                        } else if mouse_over_preview {
                                            if let Some(pv_offset) = preview_scroll_offset {
                                                // Record that preview is the scroll source
                                                sync_state.mark_scroll(
                                                    crate::preview::ScrollOrigin::Rendered
                                                );
                                                sync_state.update_rendered_offset(pv_offset);

                                                // Sync preview ΓåÆ editor (user scrolling preview)
                                                if let Some(ed_line_height) = editor_line_height {
                                                    let source_line =
                                                        SyncScrollState::preview_y_to_source_line(
                                                            pv_offset,
                                                            &preview_line_mappings
                                                        );
                                                    let editor_line = source_line.saturating_sub(1);
                                                    let target_offset =
                                                        (editor_line as f32) * ed_line_height;

                                                    sync_state.set_raw_target(target_offset);
                                                }
                                            }
                                        }
                                    } else {
                                        // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                        // Not scrolling: clear origin after debounce to allow next sync
                                        // ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
                                        sync_state.clear_origin();
                                    }
                                }

                                 // Record any changes from either pane for undo
                                 if split_content_changed {
                                     if let Some(tab) = self.state.active_tab_mut() {
                                         tab.record_edit_from_snapshot();
                                         tab.mark_content_edited();
                                         debug!("Split mode edits recorded for undo");
                                     }
                                 }

                                 // Recompute search matches when content changes in either split pane
                                if split_content_changed
                                    && self.state.ui.show_find_replace
                                    && !self.state.ui.find_state.search_term.is_empty()
                                {
                                    if let Some(content) = self.state.active_tab().map(|t| t.content.clone()) {
                                        self.state.ui.find_state.find_matches(&content);
                                    }
                                }

                                // Load CJK / complex script fonts if IME committed text needs them
                                if let Some(ref ime_text) = ime_text_for_font_loading_split {
                                    let _ = self.load_cjk_fonts_for_content(ctx, ime_text);
                                    let _ = self.load_complex_script_fonts_for_content(ctx, ime_text);
                                }
                            }
                        }
                        ViewMode::Rendered => {
                            // Check if this is a tabular file (CSV, TSV)
                            if let Some(file_type) = tabular_type {
                                // Tabular file: use the CsvViewer (read-only table view)
                                let csv_state = self.csv_viewer_states.entry(tab_id).or_default();
                                let rainbow_columns = self.state.settings.csv_rainbow_columns;

                                if let Some(tab) = self.state.active_tab_mut() {
                                    let output = CsvViewer::new(&tab.content, file_type, csv_state)
                                        .font_size(font_size)
                                        .rainbow_columns(rainbow_columns)
                                        .show(ui);

                                    // Update scroll offset for sync scrolling
                                    tab.scroll_offset = output.scroll_offset;
                                }
                            } else if let Some(file_type) = structured_type {
                                // Structured file (JSON, YAML, TOML): use the TreeViewer
                                // Note: For structured files, the outline panel shows statistics
                                // rather than navigation, so scroll_to_line is not used here.
                                let tree_state = self.tree_viewer_states.entry(tab_id).or_default();

                                if let Some(tab) = self.state.active_tab_mut() {
                                    tab.prepare_undo_snapshot_hashed();

                                    let output = TreeViewer::new(
                                        &mut tab.content,
                                        file_type,
                                        tree_state
                                    )
                                        .font_size(font_size)
                                        .show(ui);

                                    if output.changed {
                                        tab.record_edit_from_snapshot();
                                        tab.mark_content_edited();
                                        debug!(
                                            "Content modified in tree viewer, recorded for undo"
                                        );
                                    }

                                    // Update scroll offset for sync scrolling
                                    tab.scroll_offset = output.scroll_offset;
                                }
                            } else {
                                // Markdown file: use the WYSIWYG MarkdownEditor
                                // Capture settings before mutable borrow
                                let max_line_width = self.state.settings.max_line_width;
                                let zen_max_column_width = self.state.settings.zen_max_column_width;
                                let paragraph_indent = self.state.settings.paragraph_indent;
                                let header_spacing = self.state.settings.header_spacing;

                                // Collect workspace root before mutable borrow
                                let ws_root = self.state.workspace_root().cloned();
                                let render_web_images = self.state.settings.render_web_images;
                                let mut rendered_content_changed = false;
                                if let Some(tab) = self.state.active_tab_mut() {
                                    tab.prepare_undo_snapshot_hashed();

                                    // Handle scroll sync: check for pending scroll ratio or offset
                                    let pending_offset = tab.pending_scroll_offset.take();
                                    let pending_ratio = tab.pending_scroll_ratio.take();

                                    // Build wikilink context from current file and workspace
                                    let wl_ctx = WikilinkContext {
                                        current_dir: tab.path
                                            .as_ref()
                                            .and_then(|p| p.parent().map(|d| d.to_path_buf())),
                                        workspace_root: ws_root,
                                    };

                                    let mut md_editor = MarkdownEditor::new(&mut tab.content)
                                        .mode(EditorMode::Rendered)
                                        .font_size(font_size)
                                        .font_family(font_family.clone())
                                        .word_wrap(word_wrap)
                                        .theme(theme)
                                        .max_line_width(max_line_width)
                                        .zen_mode(zen_mode, zen_max_column_width)
                                        .paragraph_indent(paragraph_indent)
                                        .header_spacing(header_spacing)
                                        .wikilink_context(wl_ctx)
                                        .render_web_images(render_web_images)
                                        .id(egui::Id::new("main_editor_rendered").with(tab.id))
                                        .scroll_to_line(scroll_to_line)
                                        .pending_scroll_offset(pending_offset);
                                    if let Some(ref sh) = search_highlights {
                                        md_editor = md_editor.search_highlights(
                                            sh.matches.clone(),
                                            sh.current_match,
                                        );
                                    }
                                    let editor_output = md_editor.show(ui);

                                    if editor_output.changed {
                                        tab.record_edit_from_snapshot();
                                        tab.mark_content_edited();
                                        rendered_content_changed = true;
                                        debug!(
                                            "Content modified in rendered editor, recorded for undo"
                                        );
                                    }

                                    // Update cursor position from rendered editor
                                    tab.cursor_position = editor_output.cursor_position;

                                    // Update scroll metrics for sync scrolling
                                    tab.scroll_offset = editor_output.scroll_offset;
                                    tab.content_height = editor_output.content_height;
                                    tab.viewport_height = editor_output.viewport_height;

                                    // Store line mappings for scroll sync (source_line ΓåÆ rendered_y)
                                    tab.rendered_line_mappings = editor_output.line_mappings
                                        .iter()
                                        .map(|m| (m.start_line, m.end_line, m.rendered_y))
                                        .collect();

                                    // Handle pending scroll to line: convert to offset using FRESH line mappings
                                    // This provides accurate content-based sync using interpolation
                                    if let Some(target_line) = tab.pending_scroll_to_line.take() {
                                        if
                                            let Some(rendered_y) =
                                                Self::find_rendered_y_for_line_interpolated(
                                                    &tab.rendered_line_mappings,
                                                    target_line,
                                                    editor_output.content_height
                                                )
                                        {
                                            tab.pending_scroll_offset = Some(rendered_y);
                                            debug!(
                                                "Converted line {} to rendered offset {:.1} (interpolated, {} mappings)",
                                                target_line,
                                                rendered_y,
                                                tab.rendered_line_mappings.len()
                                            );
                                            ui.ctx().request_repaint();
                                        } else {
                                            debug!(
                                                "No mapping for line {} ({} mappings), falling back to ratio",
                                                target_line,
                                                tab.rendered_line_mappings.len()
                                            );
                                            // Fallback: estimate based on line ratio
                                            let total_lines = tab.content.lines().count().max(1);
                                            let line_ratio = (
                                                (target_line as f32) / (total_lines as f32)
                                            ).clamp(0.0, 1.0);
                                            let max_scroll = (
                                                editor_output.content_height -
                                                editor_output.viewport_height
                                            ).max(0.0);
                                            tab.pending_scroll_offset = Some(
                                                line_ratio * max_scroll
                                            );
                                            ui.ctx().request_repaint();
                                        }
                                    }

                                    // Handle pending scroll ratio: convert to offset now that we have content_height
                                    if let Some(ratio) = pending_ratio {
                                        let max_scroll = (
                                            editor_output.content_height -
                                            editor_output.viewport_height
                                        ).max(0.0);
                                        if max_scroll > 0.0 {
                                            let target_offset = ratio * max_scroll;
                                            tab.pending_scroll_offset = Some(target_offset);
                                            debug!(
                                                "Converted scroll ratio {:.3} to offset {:.1} (content_height={}, viewport_height={})",
                                                ratio,
                                                target_offset,
                                                editor_output.content_height,
                                                editor_output.viewport_height
                                            );
                                            // Request repaint to apply the offset on next frame
                                            ui.ctx().request_repaint();
                                        }
                                    }

                                    // Update selection from focused element (for rendered mode formatting)
                                    if let Some(focused) = editor_output.focused_element {
                                        // Only update selection if there's an actual text selection within the element
                                        if let Some((sel_start, sel_end)) = focused.selection {
                                            if sel_start != sel_end {
                                                // Actual selection within the focused element
                                                let abs_start = focused.start_char + sel_start;
                                                let abs_end = focused.start_char + sel_end;
                                                tab.selection = Some((abs_start, abs_end));
                                            } else {
                                                // Just cursor, no selection
                                                tab.selection = None;
                                            }
                                        } else {
                                            // No selection info
                                            tab.selection = None;
                                        }
                                    } else {
                                        // No focused element
                                        tab.selection = None;
                                    }

                                    // Handle wikilink navigation
                                    if let Some(target) = editor_output.wikilink_clicked {
                                        pending_wikilink_target = Some(target);
                                    }
                                }

                                // Recompute search matches when content changes in rendered editor
                                if rendered_content_changed
                                    && self.state.ui.show_find_replace
                                    && !self.state.ui.find_state.search_term.is_empty()
                                {
                                    if let Some(content) = self.state.active_tab().map(|t| t.content.clone()) {
                                        self.state.ui.find_state.find_matches(&content);
                                    }
                                }
                            }
                        }
                    }
                }
            } // End of else block (document tab rendering)
        });

        // Render dialogs
        self.render_dialogs(ctx);

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Quick File Switcher Overlay (Ctrl+P)
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        if self.quick_switcher.is_open() {
            if let Some(workspace) = &self.state.workspace {
                let all_files = workspace.all_files();
                let recent_files = &workspace.recent_files;

                let output = self.quick_switcher.show(
                    ctx,
                    &all_files,
                    recent_files,
                    &workspace.root_path,
                    is_dark,
                );

                // Handle file selection
                if let Some(file_path) = output.selected_file {
                    let time = self.get_app_time();
                    match self.open_file_smart(file_path.clone(), true, Some(time)) {
                        Ok(_) => {
                            self.pending_cjk_check = true;
                            debug!("Opened file from quick switcher: {}", file_path.display());
                            if let Some(workspace) = self.state.workspace_mut() {
                                workspace.add_recent_file(file_path);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to open file: {}", e);
                            self.state
                                .show_error(format!("Failed to open file:\n{}", e));
                        }
                    }
                }
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Command Palette Overlay (Alt+Space)
        // Commands are NOT dispatched here during render — they are stored in
        // `pending_palette_command` and executed after render in update().
        if self.command_palette.is_open() {
            let shortcuts = self.state.settings.keyboard_shortcuts.clone();
            let palette_output = self.command_palette.show(ctx, &shortcuts, is_dark);

            if let Some(cmd) = palette_output.selected_command {
                self.command_palette.record_recent(cmd);
                self.state.settings.command_palette_recent = self
                    .command_palette
                    .recent_commands()
                    .iter()
                    .copied()
                    .collect();
                self.state.mark_settings_dirty();
                self.pending_palette_command = Some(cmd);
            }
        }

        // File Operation Dialog (New File, Rename, Delete, etc.)
        if let Some(mut dialog) = self.file_operation_dialog.take() {
            let result = dialog.show(ctx, is_dark);

            match result {
                FileOperationResult::None => {
                    // Dialog still open, put it back
                    self.file_operation_dialog = Some(dialog);
                }
                FileOperationResult::Cancelled => {
                    // Dialog was cancelled, do nothing
                    debug!("File operation dialog cancelled");
                }
                FileOperationResult::CreateFile(path) => {
                    self.handle_create_file(path);
                }
                FileOperationResult::CreateFolder(path) => {
                    self.handle_create_folder(path);
                }
                FileOperationResult::Rename { old, new } => {
                    self.handle_rename_file(old, new);
                }
                FileOperationResult::Delete(path) => {
                    self.handle_delete_file(path, Some(ctx));
                }
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Go to Line Dialog (Ctrl+G)
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        if let Some(mut dialog) = self.state.ui.go_to_line_dialog.take() {
            let result = dialog.show(ctx, is_dark);

            match result {
                GoToLineResult::None => {
                    // Dialog still open, put it back
                    self.state.ui.go_to_line_dialog = Some(dialog);
                }
                GoToLineResult::Cancelled => {
                    // Dialog was cancelled, do nothing
                    debug!("Go to Line dialog cancelled");
                }
                GoToLineResult::GoToLine(target_line) => {
                    // Navigate to the specified line
                    self.handle_go_to_line(target_line);
                }
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // Search in Files Panel (Ctrl+Shift+F)
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        if self.search_panel.is_open() {
            if let Some(workspace) = &self.state.workspace {
                let workspace_root = workspace.root_path.clone();
                let hidden_patterns = workspace.hidden_patterns.clone();
                let all_files = workspace.all_files();

                let output = self.search_panel.show(ctx, &workspace_root, is_dark);

                // Trigger search when requested
                if output.should_search {
                    self.search_panel.search(&all_files, &hidden_patterns);
                }

                // Handle navigation to file
                if let Some(target) = output.navigate_to {
                    self.handle_search_navigation(target);
                }
            }
        }

        // Handle wikilink navigation (deferred until after UI rendering completes)
        if let Some(target) = pending_wikilink_target {
            self.navigate_wikilink(&target);
        }

        // Return deferred format action to be handled after editor has captured selection
        deferred_format_action
    }

    /// Render the content for a special (non-editable) tab.
    ///
    /// This renders settings, about/help, or other special panel content
    /// directly in the central editor area instead of the document editor.
    fn render_special_tab_content(&mut self, ui: &mut egui::Ui, kind: SpecialTabKind) {
        match kind {
            SpecialTabKind::Settings => {
                let is_dark = ui.visuals().dark_mode;
                let prev_font_family = self.state.settings.font_family.clone();
                let prev_cjk_preference = self.state.settings.cjk_font_preference;
                let prev_language = self.state.settings.language;

                let workspace_for_settings = self.state.workspace_root().map(|p| p.to_path_buf());
                let output = self.settings_panel.show_inline(
                    ui,
                    &mut self.state.settings,
                    is_dark,
                    workspace_for_settings.as_deref(),
                );

                if output.changed {
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty();

                    let font_changed = prev_font_family != self.state.settings.font_family
                        || prev_cjk_preference != self.state.settings.cjk_font_preference;

                    if font_changed {
                        let custom_font = self
                            .state
                            .settings
                            .font_family
                            .custom_name()
                            .map(|s| s.to_string());
                        let load_err = crate::fonts::reload_fonts(
                            ui.ctx(),
                            custom_font.as_deref(),
                            self.state.settings.cjk_font_preference,
                            Some(&self.state.settings.complex_script_font_preferences),
                        );
                        if let Some(reason) = load_err {
                            self.state.settings.font_family = crate::config::EditorFont::default();
                            self.state.mark_settings_dirty();
                            let time = self.get_app_time();
                            self.state.show_toast(
                                format!("Font failed to load: {reason}. Reverted to Inter."),
                                time,
                                5.0,
                            );
                            warn!("Custom font reverted to Inter: {}", reason);
                        } else {
                            info!("Font settings changed, reloaded fonts");
                        }
                    }

                    if prev_language != self.state.settings.language {
                        if let Some(cjk_pref) = self.state.settings.language.required_cjk_font() {
                            let custom_font = self
                                .state
                                .settings
                                .font_family
                                .custom_name()
                                .map(|s| s.to_string());
                            crate::fonts::preload_explicit_cjk_font_with_custom(
                                ui.ctx(),
                                cjk_pref,
                                custom_font.as_deref(),
                            );
                            info!(
                                "Loaded CJK fonts for language: {:?}",
                                self.state.settings.language
                            );
                        }
                    }
                }

                if output.reset_requested {
                    let default_settings = crate::config::Settings::default();
                    self.state.settings = default_settings;
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty();

                    let _ = crate::fonts::reload_fonts(
                        ui.ctx(),
                        None,
                        crate::config::CjkFontPreference::Auto,
                        None,
                    );

                    let time = self.get_app_time();
                    self.state
                        .show_toast(t!("notification.settings_reset").to_string(), time, 2.0);
                }
            }
            SpecialTabKind::About => {
                let is_dark = ui.visuals().dark_mode;
                self.about_panel.show_inline(ui, is_dark);
            }
            SpecialTabKind::Welcome => {
                let prev_language = self.state.settings.language;

                let changed = {
                    let settings = &mut self.state.settings;
                    self.welcome_panel.show_inline(ui, settings)
                };

                if changed {
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty(); // so it persists

                    // Load CJK fonts when switching to a CJK language so UI
                    // labels rendered via i18n don't show as squares.
                    if prev_language != self.state.settings.language {
                        if let Some(cjk_pref) = self.state.settings.language.required_cjk_font() {
                            let custom_font = self
                                .state
                                .settings
                                .font_family
                                .custom_name()
                                .map(|s| s.to_string());
                            crate::fonts::preload_explicit_cjk_font_with_custom(
                                ui.ctx(),
                                cjk_pref,
                                custom_font.as_deref(),
                            );
                            info!(
                                "Loaded CJK fonts for language: {:?}",
                                self.state.settings.language
                            );
                        }
                    }

                    ui.ctx().request_repaint();
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Image Viewer Tab
    // ─────────────────────────────────────────────────────────────────────────

    fn render_image_viewer_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let active_idx = self.state.active_tab_index();

        let (path, viewer_state) = {
            let tab = match self.state.tab(active_idx) {
                Some(t) => t,
                None => return,
            };
            let path = match tab.path.clone() {
                Some(p) => p,
                None => return,
            };
            let vs = match &tab.kind {
                TabKind::ImageViewer(vs) => vs.clone(),
                _ => return,
            };
            (path, vs)
        };

        let cache_id = egui::Id::new("image_viewer_texture").with(&path);
        let cached: Option<ImageViewerTexture> = ui.data(|d| d.get_temp(cache_id));

        let load_result = cached.unwrap_or_else(|| match load_viewer_image(ctx, &path) {
            Ok(tex) => {
                ui.data_mut(|d| d.insert_temp(cache_id, tex.clone()));
                tex
            }
            Err(msg) => {
                let failed = ImageViewerTexture {
                    texture: None,
                    width: 0,
                    height: 0,
                    error: Some(msg),
                };
                ui.data_mut(|d| d.insert_temp(cache_id, failed.clone()));
                failed
            }
        });

        // Update dimensions in tab state if loaded and not yet set
        if load_result.texture.is_some() {
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::ImageViewer(ref mut vs) = tab.kind {
                    if vs.dimensions.is_none() {
                        vs.dimensions = Some((load_result.width, load_result.height));
                    }
                }
            }
        }

        if let Some(ref error_msg) = load_result.error {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!("Failed to load image: {}", error_msg))
                        .color(ui.visuals().error_fg_color)
                        .size(16.0),
                );
            });
            return;
        }

        let texture = match load_result.texture {
            Some(ref t) => t,
            None => return,
        };

        let available = ui.available_size();
        let img_w = load_result.width as f32;
        let img_h = load_result.height as f32;

        // Read current zoom from tab
        let mut zoom = viewer_state.zoom;
        let fitted = viewer_state.fitted;

        // Fit-to-window on first render
        if !fitted && img_w > 0.0 && img_h > 0.0 {
            let scale_x = available.x / img_w;
            let scale_y = (available.y - 30.0) / img_h; // reserve space for metadata bar
            zoom = scale_x.min(scale_y).min(1.0); // don't upscale beyond 1:1
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::ImageViewer(ref mut vs) = tab.kind {
                    vs.zoom = zoom;
                    vs.fitted = true;
                }
            }
        }

        // Handle Ctrl+Scroll zoom via raw MouseWheel events
        // (smooth_scroll_delta is consumed by egui's global gui_zoom when Ctrl is held)
        let wheel_delta: Option<f32> = ui.input(|i| {
            if !i.modifiers.command {
                return None;
            }
            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    if delta.y.abs() > 0.01 {
                        return Some(delta.y);
                    }
                }
            }
            None
        });
        if let Some(delta) = wheel_delta {
            let zoom_factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
            zoom = (zoom * zoom_factor).clamp(0.1, 10.0);
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::ImageViewer(ref mut vs) = tab.kind {
                    vs.zoom = zoom;
                }
            }
            // Consume the events so egui's global zoom doesn't also fire
            ui.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(e, egui::Event::MouseWheel { modifiers, .. } if modifiers.command)
                });
            });
        }

        let display_w = img_w * zoom;
        let display_h = img_h * zoom;

        // Scrollable area for the image
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(available.y - 28.0)
            .show(ui, |ui| {
                let content_size = egui::vec2(display_w, display_h);

                // Center the image if smaller than available space
                let padding_x = ((ui.available_width() - content_size.x) / 2.0).max(0.0);
                let padding_y = ((ui.available_height() - content_size.y) / 2.0).max(0.0);

                if padding_y > 0.0 {
                    ui.add_space(padding_y);
                }

                ui.horizontal(|ui| {
                    if padding_x > 0.0 {
                        ui.add_space(padding_x);
                    }
                    let sized = egui::load::SizedTexture::new(
                        texture.id(),
                        egui::Vec2::new(display_w, display_h),
                    );
                    ui.add(egui::Image::from_texture(sized));
                });
            });

        // Metadata bar at the bottom
        let file_size = viewer_state.file_size;
        let format_label = &viewer_state.format_label;
        let dims_text = if let Some((w, h)) = viewer_state.dimensions {
            format!("{} x {}", w, h)
        } else {
            "Loading...".to_string()
        };
        let size_text = if file_size >= 1_048_576 {
            format!("{:.1} MB", file_size as f64 / 1_048_576.0)
        } else {
            format!("{:.1} KB", file_size as f64 / 1024.0)
        };
        let zoom_pct = (zoom * 100.0).round() as u32;

        ui.separator();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.label(
                egui::RichText::new(format!(
                    "{}  |  {}  |  {}  |  {}%",
                    dims_text, format_label, size_text, zoom_pct
                ))
                .small()
                .color(ui.visuals().text_color().gamma_multiply(0.7)),
            );
        });
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PDF Viewer Tab
    // ─────────────────────────────────────────────────────────────────────────

    fn render_pdf_viewer_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let active_idx = self.state.active_tab_index();

        let (path, viewer_state) = {
            let tab = match self.state.tab(active_idx) {
                Some(t) => t,
                None => return,
            };
            let path = match tab.path.clone() {
                Some(p) => p,
                None => return,
            };
            let vs = match &tab.kind {
                TabKind::PdfViewer(vs) => vs.clone(),
                _ => return,
            };
            (path, vs)
        };

        // Show error overlay if PDF failed to load
        if let Some(ref error_msg) = viewer_state.error {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!("Failed to load PDF: {}", error_msg))
                        .color(ui.visuals().error_fg_color)
                        .size(16.0),
                );
            });
            return;
        }

        if viewer_state.page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("PDF has no pages")
                        .color(ui.visuals().error_fg_color)
                        .size(16.0),
                );
            });
            return;
        }

        let current_page = viewer_state.current_page;
        let page_count = viewer_state.page_count;
        let mut zoom = viewer_state.zoom;
        let fitted = viewer_state.fitted;

        // Cache key includes page index and zoom for invalidation
        let cache_id = egui::Id::new("pdf_viewer_texture")
            .with(&path)
            .with(current_page)
            .with((zoom * 100.0) as u32);
        let cached: Option<PdfPageTexture> = ui.data(|d| d.get_temp(cache_id));

        let page_texture = cached.unwrap_or_else(|| {
            let tex = render_pdf_page(ctx, &path, current_page, zoom);
            ui.data_mut(|d| d.insert_temp(cache_id, tex.clone()));
            tex
        });

        if let Some(ref error_msg) = page_texture.error {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!("Failed to render page: {}", error_msg))
                        .color(ui.visuals().error_fg_color)
                        .size(16.0),
                );
            });
            return;
        }

        let texture = match page_texture.texture {
            Some(ref t) => t,
            None => return,
        };

        let available = ui.available_size();
        let img_w = page_texture.width as f32;
        let img_h = page_texture.height as f32;

        // Fit-to-window on first render: render at scale 1.0 then compute visual zoom
        if !fitted && img_w > 0.0 && img_h > 0.0 {
            let scale_x = available.x / img_w;
            let scale_y = (available.y - 60.0) / img_h;
            zoom = scale_x.min(scale_y).min(1.0);
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                    vs.zoom = zoom;
                    vs.fitted = true;
                }
            }
            // Re-render at the fitted zoom by clearing cache
            let new_cache_id = egui::Id::new("pdf_viewer_texture")
                .with(&path)
                .with(current_page)
                .with((zoom * 100.0) as u32);
            let tex = render_pdf_page(ctx, &path, current_page, zoom);
            ui.data_mut(|d| d.insert_temp(new_cache_id, tex));
            ctx.request_repaint();
            return;
        }

        // Handle Ctrl+Scroll zoom via raw MouseWheel events
        // (smooth_scroll_delta is consumed by egui's global gui_zoom when Ctrl is held)
        let wheel_delta: Option<f32> = ui.input(|i| {
            if !i.modifiers.command {
                return None;
            }
            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    if delta.y.abs() > 0.01 {
                        return Some(delta.y);
                    }
                }
            }
            None
        });
        if let Some(delta) = wheel_delta {
            let zoom_factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
            let new_zoom = (zoom * zoom_factor).clamp(0.5, 4.0);
            if (new_zoom - zoom).abs() > 0.001 {
                zoom = new_zoom;
                if let Some(tab) = self.state.tab_mut(active_idx) {
                    if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                        vs.zoom = zoom;
                    }
                }
                // Consume the events so egui's global zoom doesn't also fire
                ui.input_mut(|i| {
                    i.events.retain(|e| {
                        !matches!(e, egui::Event::MouseWheel { modifiers, .. } if modifiers.command)
                    });
                });
                ctx.request_repaint();
                return;
            }
        }

        // Handle keyboard navigation
        let (prev_pressed, next_pressed) = ui.input(|i| {
            let prev = i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp);
            let next = i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown);
            (prev, next)
        });

        if prev_pressed && current_page > 0 {
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                    vs.current_page = current_page - 1;
                    vs.fitted = false;
                }
            }
            ctx.request_repaint();
            return;
        }

        if next_pressed && current_page + 1 < page_count {
            if let Some(tab) = self.state.tab_mut(active_idx) {
                if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                    vs.current_page = current_page + 1;
                    vs.fitted = false;
                }
            }
            ctx.request_repaint();
            return;
        }

        let display_w = img_w;
        let display_h = img_h;

        // Scrollable area for the page
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(available.y - 36.0)
            .show(ui, |ui| {
                let content_size = egui::vec2(display_w, display_h);
                let padding_x = ((ui.available_width() - content_size.x) / 2.0).max(0.0);
                let padding_y = ((ui.available_height() - content_size.y) / 2.0).max(0.0);

                if padding_y > 0.0 {
                    ui.add_space(padding_y);
                }

                ui.horizontal(|ui| {
                    if padding_x > 0.0 {
                        ui.add_space(padding_x);
                    }
                    let sized = egui::load::SizedTexture::new(
                        texture.id(),
                        egui::Vec2::new(display_w, display_h),
                    );
                    ui.add(egui::Image::from_texture(sized));
                });
            });

        // Navigation + metadata bar at the bottom
        ui.separator();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            let prev_enabled = current_page > 0;
            if ui
                .add_enabled(prev_enabled, egui::Button::new("\u{25C0}").small())
                .clicked()
            {
                if let Some(tab) = self.state.tab_mut(active_idx) {
                    if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                        vs.current_page = current_page.saturating_sub(1);
                        vs.fitted = false;
                    }
                }
                ctx.request_repaint();
            }

            ui.label(egui::RichText::new(format!("{} / {}", current_page + 1, page_count)).small());

            let next_enabled = current_page + 1 < page_count;
            if ui
                .add_enabled(next_enabled, egui::Button::new("\u{25B6}").small())
                .clicked()
            {
                if let Some(tab) = self.state.tab_mut(active_idx) {
                    if let TabKind::PdfViewer(ref mut vs) = tab.kind {
                        vs.current_page = current_page + 1;
                        vs.fitted = false;
                    }
                }
                ctx.request_repaint();
            }

            ui.separator();

            let size_text = if viewer_state.file_size >= 1_048_576 {
                format!("{:.1} MB", viewer_state.file_size as f64 / 1_048_576.0)
            } else {
                format!("{:.1} KB", viewer_state.file_size as f64 / 1024.0)
            };
            let zoom_pct = (zoom * 100.0).round() as u32;

            ui.label(
                egui::RichText::new(format!("PDF  |  {}  |  {}%", size_text, zoom_pct))
                    .small()
                    .color(ui.visuals().text_color().gamma_multiply(0.7)),
            );
        });
    }

    // ─────────────────────────────────────────────────────────────────────
    // Loading / Error Tab Rendering
    // ─────────────────────────────────────────────────────────────────────

    /// Render a progress indicator for a tab whose file is still loading.
    fn render_loading_tab(&self, ui: &mut egui::Ui, progress: &crate::state::LoadingProgress) {
        let avail = ui.available_size();

        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(
                ui.min_rect().min + egui::vec2(0.0, avail.y * 0.35),
                egui::vec2(avail.x, avail.y * 0.3),
            ),
            |ui| {
                ui.vertical_centered(|ui| {
                    let file_name = progress
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file");

                    ui.add_space(8.0);
                    ui.spinner();
                    ui.add_space(12.0);

                    ui.label(
                        egui::RichText::new(format!("Loading {}", file_name))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    let fraction = progress.fraction();
                    let bar = egui::ProgressBar::new(fraction)
                        .text(format!(
                            "{:.1} / {:.1} MB  ({:.0}%)",
                            progress.mb_loaded(),
                            progress.mb_total(),
                            fraction * 100.0
                        ))
                        .desired_width(avail.x.min(400.0));
                    ui.add(bar);
                });
            },
        );
    }

    /// Render an error message for a tab whose file failed to load.
    fn render_load_error_tab(ui: &mut egui::Ui, error: &str) {
        let avail = ui.available_size();

        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(
                ui.min_rect().min + egui::vec2(0.0, avail.y * 0.35),
                egui::vec2(avail.x, avail.y * 0.3),
            ),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("\u{26A0}")
                            .size(32.0)
                            .color(egui::Color32::from_rgb(220, 80, 60)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Failed to load file")
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(error).color(ui.visuals().warn_fg_color));
                });
            },
        );
    }

    /// Dispatch a command selected from the command palette.
    ///
    /// Maps ShortcutCommand variants to the same handler methods used by
    /// the ribbon and keyboard shortcuts.
    pub(crate) fn dispatch_palette_command(&mut self, ctx: &egui::Context, cmd: ShortcutCommand) {
        use crate::markdown::MarkdownFormatCommand;

        debug!("Command palette: {}", cmd.display_name());

        match cmd {
            // File
            ShortcutCommand::Save => self.handle_save_file(),
            ShortcutCommand::SaveAs => self.handle_save_as_file(),
            ShortcutCommand::Open => self.handle_open_file(),
            ShortcutCommand::New | ShortcutCommand::NewTab => {
                self.state.new_tab();
            }
            ShortcutCommand::CloseTab => self.handle_close_current_tab(ctx),
            ShortcutCommand::OpenWorkspace => self.handle_open_workspace(),
            ShortcutCommand::CloseWorkspace => self.handle_close_workspace(),
            // Navigation
            ShortcutCommand::NextTab => self.handle_next_tab(),
            ShortcutCommand::PrevTab => self.handle_prev_tab(),
            ShortcutCommand::GoToLine => self.handle_open_go_to_line(),
            ShortcutCommand::QuickOpen => self.handle_quick_open(),
            // View
            ShortcutCommand::ToggleViewMode => self.handle_toggle_view_mode(),
            ShortcutCommand::CycleTheme => self.handle_cycle_theme(ctx),
            ShortcutCommand::ToggleZenMode => self.handle_toggle_zen_mode(),
            ShortcutCommand::ToggleFullscreen => self.handle_toggle_fullscreen(ctx),
            ShortcutCommand::ToggleOutline => self.handle_toggle_outline(),
            ShortcutCommand::ToggleFileTree => self.handle_toggle_file_tree(),
            ShortcutCommand::TogglePipeline => self.handle_toggle_pipeline(),
            ShortcutCommand::ToggleTerminal => self.handle_toggle_terminal(),
            ShortcutCommand::ToggleProductivityHub => {
                if self.state.settings.productivity_panel_docked {
                    if self.state.settings.outline_enabled
                        && self.outline_panel.active_tab()
                            == crate::ui::OutlinePanelTab::Productivity
                    {
                        self.state.settings.outline_enabled = false;
                    } else {
                        self.state.settings.outline_enabled = true;
                        self.outline_panel
                            .set_active_tab(crate::ui::OutlinePanelTab::Productivity);
                    }
                } else {
                    self.state.settings.productivity_panel_visible =
                        !self.state.settings.productivity_panel_visible;
                }
                self.state.mark_settings_dirty();
            }
            ShortcutCommand::ZoomIn => egui::gui_zoom::zoom_in(ctx),
            ShortcutCommand::ZoomOut => egui::gui_zoom::zoom_out(ctx),
            ShortcutCommand::ResetZoom => ctx.set_zoom_factor(1.0),
            // Edit
            ShortcutCommand::Undo => self.handle_undo(),
            ShortcutCommand::Redo => self.handle_redo(),
            ShortcutCommand::DeleteLine => self.handle_delete_line(),
            ShortcutCommand::DuplicateLine => self.handle_duplicate_line(),
            ShortcutCommand::MoveLineUp | ShortcutCommand::MoveLineDown => {
                // Move-line is handled via pre-render consumption; palette just records it
            }
            ShortcutCommand::SelectNextOccurrence => self.handle_select_next_occurrence(),
            // Search
            ShortcutCommand::Find => self.handle_open_find(false),
            ShortcutCommand::FindReplace => self.handle_open_find(true),
            ShortcutCommand::FindNext => self.handle_find_next(),
            ShortcutCommand::FindPrev => self.handle_find_prev(),
            ShortcutCommand::SearchInFiles => self.handle_search_in_files(),
            // Format
            ShortcutCommand::FormatBold => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Bold)
            }
            ShortcutCommand::FormatItalic => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Italic)
            }
            ShortcutCommand::FormatInlineCode => {
                self.handle_format_command(ctx, MarkdownFormatCommand::InlineCode)
            }
            ShortcutCommand::FormatCodeBlock => {
                self.handle_format_command(ctx, MarkdownFormatCommand::CodeBlock)
            }
            ShortcutCommand::FormatLink => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Link)
            }
            ShortcutCommand::FormatImage => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Image)
            }
            ShortcutCommand::FormatBlockquote => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Blockquote)
            }
            ShortcutCommand::FormatBulletList => {
                self.handle_format_command(ctx, MarkdownFormatCommand::BulletList)
            }
            ShortcutCommand::FormatNumberedList => {
                self.handle_format_command(ctx, MarkdownFormatCommand::NumberedList)
            }
            ShortcutCommand::FormatHeading1 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(1))
            }
            ShortcutCommand::FormatHeading2 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(2))
            }
            ShortcutCommand::FormatHeading3 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(3))
            }
            ShortcutCommand::FormatHeading4 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(4))
            }
            ShortcutCommand::FormatHeading5 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(5))
            }
            ShortcutCommand::FormatHeading6 => {
                self.handle_format_command(ctx, MarkdownFormatCommand::Heading(6))
            }
            // Folding
            ShortcutCommand::FoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.fold_all();
                    }
                }
            }
            ShortcutCommand::UnfoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.unfold_all();
                    }
                }
            }
            ShortcutCommand::ToggleFoldAtCursor => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor_line = tab.cursor_position.0;
                        tab.toggle_fold_at_line(cursor_line);
                    }
                }
            }
            // Other
            ShortcutCommand::CommandPalette => {} // Already open
            ShortcutCommand::OpenSettings => self.state.toggle_settings(),
            ShortcutCommand::OpenAbout => self.state.toggle_about(),
            ShortcutCommand::ExportHtml => self.handle_export_html(ctx),
            ShortcutCommand::InsertToc => self.handle_insert_toc(),
            ShortcutCommand::ToggleFrontmatter => {
                if !self.state.settings.outline_enabled {
                    self.state.settings.outline_enabled = true;
                }
                self.outline_panel
                    .set_active_tab(crate::ui::OutlinePanelTab::Frontmatter);
                self.state.mark_settings_dirty();
            }
        }
    }
}
