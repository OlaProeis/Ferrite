//! Central panel rendering for the Ferrite application.
//!
//! This module renders the main editor content area including the tab bar,
//! editor widget (raw/rendered/split views), CSV viewer, tree viewer,
//! minimap, and navigation buttons.

use super::FerriteApp;
use super::types::{ DeferredFormatAction, HeadingNavRequest };
use super::helpers::{ char_index_to_line_col, get_formatting_state_for, modifier_symbol };
use crate::config::{ Theme, ViewMode };
use crate::editor::{
    cleanup_ferrite_editor,
    DocumentOutline,
    EditorWidget,
    FindReplacePanel,
    Minimap,
    SearchHighlights,
    SemanticMinimap,
};
use crate::markdown::{
    apply_raw_format,
    cleanup_rendered_editor_memory,
    get_structured_file_type,
    get_tabular_file_type,
    CsvViewer,
    CsvViewerState,
    EditorMode,
    FormattingState,
    MarkdownEditor,
    MarkdownFormatCommand,
    TreeViewer,
    TreeViewerState,
    WikilinkContext,
};
#[allow(unused_imports)]
use crate::preview::SyncScrollState;
use crate::state::{ FileType, PendingAction, Selection, SpecialTabKind, TabKind };
use crate::theme::ThemeColors;
use crate::ui::{ FileOperationResult, GoToLineResult };
use eframe::egui;
use log::{ debug, info, trace, warn };
use rust_i18n::t;
use std::collections::HashMap;

impl FerriteApp {
    /// Render the central panel containing tabs and editor content.
    ///
    /// Returns a deferred format action if one was requested.
    pub(crate) fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        is_dark: bool
    ) -> Option<DeferredFormatAction> {
        let zen_mode = self.state.is_zen_mode();
        let mut deferred_format_action: Option<DeferredFormatAction> = None;
        let mut pending_wikilink_target: Option<String> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
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
                    if close_response.clicked() {
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

            if let TabKind::Special(special_kind) = active_tab_kind {
                self.render_special_tab_content(ui, special_kind);
            } else {
                // Editor widget - extract settings values to avoid borrow conflicts
                let font_size = self.state.settings.font_size;
                let font_family = self.state.settings.font_family.clone();
                let word_wrap = self.state.settings.word_wrap;
                let theme = self.state.settings.theme;
                let show_line_numbers = self.state.settings.show_line_numbers;
                let auto_close_brackets = self.state.settings.auto_close_brackets;

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

                            // Capture content and cursor before editing for undo support
                            let (content_before, cursor_before) = self.state
                                .active_tab()
                                .map(|t| (t.content.clone(), t.cursors.primary().head))
                                .unwrap_or_default();

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

                                // Calculate layout for editor and minimap
                                let total_rect = ui.available_rect_before_wrap();
                                let editor_width = if minimap_enabled {
                                    total_rect.width() - minimap_width
                                } else {
                                    total_rect.width()
                                };

                                let editor_rect = egui::Rect::from_min_size(
                                    total_rect.min,
                                    egui::vec2(editor_width, total_rect.height())
                                );
                                let minimap_rect = if minimap_enabled {
                                    Some(
                                        egui::Rect::from_min_size(
                                            egui::pos2(
                                                total_rect.min.x + editor_width,
                                                total_rect.min.y
                                            ),
                                            egui::vec2(minimap_width, total_rect.height())
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

                                let mut editor = EditorWidget::new(tab)
                                    .font_size(font_size)
                                    .font_family(font_family.clone())
                                    .word_wrap(word_wrap)
                                    .show_line_numbers(show_line_numbers && !zen_mode) // Hide line numbers in Zen Mode
                                    .show_fold_indicators(show_fold_indicators && !zen_mode) // Hide in Zen Mode
                                    .theme_colors(theme_colors.clone())
                                    .id(egui::Id::new("main_editor_raw"))
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
                                    .syntax_theme(syntax_theme.clone())
                                    .auto_close_brackets(auto_close_brackets);

                                // Add search highlights if available
                                if let Some(highlights) = search_highlights.clone() {
                                    editor = editor.search_highlights(highlights);
                                }

                                let editor_output = editor.show(&mut editor_ui);

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
                                    debug!("Content modified in raw editor");
                                    // Record edit for undo/redo support
                                    tab.record_edit(content_before.clone(), cursor_before);
                                    // Mark folds as dirty when content changes
                                    if folding_enabled {
                                        tab.mark_folds_dirty();
                                    }
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

                            // Load CJK fonts if IME committed text contains CJK characters
                            if let Some(ref ime_text) = ime_text_for_font_loading {
                                let _ = self.load_cjk_fonts_for_content(ctx, ime_text);
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

                                // Get line width setting
                                let max_line_width = self.state.settings.max_line_width;

                                // Get paragraph indent setting (CJK typography)
                                let paragraph_indent = self.state.settings.paragraph_indent;

                                // Get path for syntax highlighting
                                let tab_path_for_syntax = self.state
                                    .active_tab()
                                    .and_then(|t| t.path.clone());

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

                                // Capture content and cursor before editing for undo support
                                let (content_before_split, cursor_before_split) = self.state
                                    .active_tab()
                                    .map(|t| (t.content.clone(), t.cursors.primary().head))
                                    .unwrap_or_default();

                                // Calculate explicit rectangles for split view layout
                                // Layout: [Editor] [Minimap] [Splitter] [Preview]
                                let total_rect = ui.available_rect_before_wrap();
                                let left_rect = egui::Rect::from_min_size(
                                    total_rect.min,
                                    egui::vec2(left_width, total_rect.height())
                                );
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

                                    let mut editor = EditorWidget::new(tab)
                                        .font_size(font_size)
                                        .font_family(font_family.clone())
                                        .word_wrap(word_wrap)
                                        .show_line_numbers(show_line_numbers && !zen_mode) // Hide in Zen Mode
                                        .show_fold_indicators(show_fold_indicators)
                                        .theme_colors(theme_colors.clone())
                                        .id(egui::Id::new("split_editor_raw"))
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
                                        .syntax_theme(syntax_theme.clone())
                                        .auto_close_brackets(auto_close_brackets)
                                        .pending_sync_scroll_offset(pending_editor_scroll);

                                    // Add search highlights if available
                                    if let Some(highlights) = search_highlights.clone() {
                                        editor = editor.search_highlights(highlights);
                                    }

                                    let editor_output = editor.show(&mut left_ui);

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
                                        // Record edit for undo/redo support
                                        tab.record_edit(
                                            content_before_split.clone(),
                                            cursor_before_split
                                        );
                                        if folding_enabled {
                                            tab.mark_folds_dirty();
                                        }
                                    }

                                    // Capture IME committed text for font loading (processed after tab borrow ends)
                                    ime_text_for_font_loading_split =
                                        editor_output.ime_committed_text.clone();
                                }

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
                                    if let Some(tab) = self.state.active_tab_mut() {
                                        // Capture content and cursor before editing for undo support
                                        let content_before = tab.content.clone();
                                        let cursor_before = tab.cursors.primary().head;

                                        // Build wikilink context from current file and workspace
                                        let wl_ctx = WikilinkContext {
                                            current_dir: tab.path
                                                .as_ref()
                                                .and_then(|p| p.parent().map(|d| d.to_path_buf())),
                                            workspace_root: ws_root.clone(),
                                        };

                                        let md_editor_output = MarkdownEditor::new(&mut tab.content)
                                            .mode(EditorMode::Rendered)
                                            .font_size(font_size)
                                            .font_family(font_family.clone())
                                            .word_wrap(word_wrap)
                                            .theme(theme)
                                            .max_line_width(max_line_width)
                                            .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                            .paragraph_indent(paragraph_indent) // CJK paragraph indentation
                                            .wikilink_context(wl_ctx)
                                            .id(egui::Id::new("split_preview_rendered"))
                                            .pending_scroll_offset(pending_preview_scroll)
                                            .show(&mut right_ui);

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
                                            // Record edit for undo/redo support
                                            tab.record_edit(content_before, cursor_before);
                                            // Mark content as edited for auto-save scheduling
                                            tab.mark_content_edited();
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

                                // Load CJK fonts if IME committed text contains CJK characters
                                if let Some(ref ime_text) = ime_text_for_font_loading_split {
                                    let _ = self.load_cjk_fonts_for_content(ctx, ime_text);
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
                                    // Capture content and cursor before editing for undo support
                                    let content_before = tab.content.clone();
                                    let cursor_before = tab.cursors.primary().head;

                                    let output = TreeViewer::new(
                                        &mut tab.content,
                                        file_type,
                                        tree_state
                                    )
                                        .font_size(font_size)
                                        .show(ui);

                                    if output.changed {
                                        // Record edit for undo/redo support
                                        tab.record_edit(content_before, cursor_before);
                                        // Mark content as edited for auto-save scheduling
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

                                // Collect workspace root before mutable borrow
                                let ws_root = self.state.workspace_root().cloned();
                                if let Some(tab) = self.state.active_tab_mut() {
                                    // Capture content and cursor before editing for undo support
                                    let content_before = tab.content.clone();
                                    let cursor_before = tab.cursors.primary().head;

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

                                    let editor_output = MarkdownEditor::new(&mut tab.content)
                                        .mode(EditorMode::Rendered)
                                        .font_size(font_size)
                                        .font_family(font_family.clone())
                                        .word_wrap(word_wrap)
                                        .theme(theme)
                                        .max_line_width(max_line_width) // Apply line width limit
                                        .zen_mode(zen_mode, zen_max_column_width) // Apply Zen Mode centering
                                        .paragraph_indent(paragraph_indent) // CJK paragraph indentation
                                        .wikilink_context(wl_ctx)
                                        .id(egui::Id::new("main_editor_rendered"))
                                        .scroll_to_line(scroll_to_line)
                                        .pending_scroll_offset(pending_offset)
                                        .show(ui);

                                    if editor_output.changed {
                                        // Record edit for undo/redo support
                                        tab.record_edit(content_before, cursor_before);
                                        // Mark content as edited for auto-save scheduling
                                        tab.mark_content_edited();
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
                    is_dark
                );

                // Handle file selection
                if let Some(file_path) = output.selected_file {
                    let time = self.get_app_time();
                    match self.state.open_file(file_path.clone(), Some(time)) {
                        Ok(_) => {
                            self.pending_cjk_check = true;
                            debug!("Opened file from quick switcher: {}", file_path.display());
                            // Add to workspace recent files
                            if let Some(workspace) = self.state.workspace_mut() {
                                workspace.add_recent_file(file_path);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to open file: {}", e);
                            self.state.show_error(format!("Failed to open file:\n{}", e));
                        }
                    }
                }
            }
        }

        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
        // File Operation Dialog (New File, Rename, Delete, etc.)
        // ΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉΓòÉ
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

                let output = self.settings_panel.show_inline(ui, &mut self.state.settings, is_dark);

                if output.changed {
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty();

                    let font_changed =
                        prev_font_family != self.state.settings.font_family ||
                        prev_cjk_preference != self.state.settings.cjk_font_preference;

                    if font_changed {
                        let custom_font = self.state.settings.font_family
                            .custom_name()
                            .map(|s| s.to_string());
                        crate::fonts::reload_fonts(
                            ui.ctx(),
                            custom_font.as_deref(),
                            self.state.settings.cjk_font_preference
                        );
                        info!("Font settings changed, reloaded fonts");
                    }
                }

                if output.reset_requested {
                    let default_settings = crate::config::Settings::default();
                    self.state.settings = default_settings;
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty();

                    crate::fonts::reload_fonts(
                        ui.ctx(),
                        None,
                        crate::config::CjkFontPreference::Auto
                    );

                    let time = self.get_app_time();
                    self.state.show_toast("Settings reset to defaults", time, 2.0);
                }
            }
            SpecialTabKind::About => {
                let is_dark = ui.visuals().dark_mode;
                self.about_panel.show_inline(ui, is_dark);
            }
            SpecialTabKind::Welcome => {
                let changed = {
                    let settings = &mut self.state.settings;
                    self.welcome_panel.show_inline(ui, settings)
                };

                if changed {
                    self.theme_manager.set_theme(self.state.settings.theme);
                    self.theme_manager.apply(ui.ctx());
                    self.state.mark_settings_dirty(); // so it persists
                    ui.ctx().request_repaint();
                }
            }
        }
    }
}
