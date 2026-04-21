//! Navigation and panel toggle handlers for the Ferrite application.
//!
//! This module contains tab switching, view mode cycling, panel toggles
//! (outline, terminal, zen, pipeline), theme switching, undo/redo dispatch,
//! scroll interpolation, and heading navigation.

use super::FerriteApp;
use super::types::HeadingNavRequest;
use crate::config::{Theme, ViewMode};
use crate::editor::{extract_outline_for_file, DocumentOutline, DocumentStats, OutlineType};
use crate::state::{BacklinkIndex, FileType};
use eframe::egui;
use log::{debug, info};
use rust_i18n::t;
use std::path::{Path, PathBuf};

impl FerriteApp {

    /// Handle closing the current tab (with unsaved prompt if needed).
    pub(crate) fn handle_close_current_tab(&mut self, ctx: &egui::Context) {
        let index = self.state.active_tab_index();
        // Get tab_id before closing for viewer state cleanup
        let tab_id = self.state.tabs().get(index).map(|t| t.id);
        self.state.close_tab(index);
        if let Some(id) = tab_id {
            self.cleanup_tab_state(id, Some(ctx));
        }
    }

    /// Switch to the next tab (cycles to first if at end).
    pub(crate) fn handle_next_tab(&mut self) {
        let count = self.state.tab_count();
        if count > 1 {
            let current = self.state.active_tab_index();
            let next = (current + 1) % count;
            self.state.set_active_tab(next);
            self.pending_cjk_check = true;
        }
    }

    /// Switch to the previous tab (cycles to last if at beginning).
    pub(crate) fn handle_prev_tab(&mut self) {
        let count = self.state.tab_count();
        if count > 1 {
            let current = self.state.active_tab_index();
            let prev = if current == 0 { count - 1 } else { current - 1 };
            self.state.set_active_tab(prev);
            self.pending_cjk_check = true;
        }
    }

    /// Toggle view modes for the active tab.
    ///
    /// For markdown files: cycles Raw ΓåÆ Split ΓåÆ Rendered ΓåÆ Raw
    /// For structured files (JSON, YAML, TOML): cycles Raw Γåö Rendered (no Split mode)
    ///
    /// When sync scrolling is enabled, this calculates the corresponding scroll
    /// position in the target mode using line-to-position mapping for accuracy.
    pub(crate) fn handle_toggle_view_mode(&mut self) {
        // Get sync scroll setting and file type before mutable borrow
        let sync_enabled = self.state.settings.sync_scroll_enabled;
        let file_type = self.state.active_tab()
            .and_then(|t| t.path.as_ref())
            .map(|p| FileType::from_path(p))
            .unwrap_or(FileType::Unknown);
        // Structured (JSON/YAML/TOML) files don't support Split mode
        // CSV/TSV files DO support split mode (raw text + table view)
        let skip_split_mode = file_type.is_structured();

        // Track if we need to set App-level pending_scroll_to_line for Raw mode
        let mut raw_mode_scroll_to_line: Option<usize> = None;

        if let Some(tab) = self.state.active_tab_mut() {
            let old_mode = tab.view_mode;
            let current_scroll = tab.scroll_offset;
            let line_mappings = tab.rendered_line_mappings.clone();

            // Debug: log the current state before toggle
            debug!(
                "Toggle view mode: old_mode={:?}, current_scroll={}, sync_enabled={}, mappings_count={}, skip_split={}",
                old_mode, current_scroll, sync_enabled, line_mappings.len(), skip_split_mode
            );

            // Toggle the view mode
            let new_mode = tab.toggle_view_mode();
            
            // For structured/tabular files, skip Split mode (not supported)
            let new_mode = if skip_split_mode && new_mode == ViewMode::Split {
                tab.toggle_view_mode() // Toggle again to skip Split
            } else {
                new_mode
            };
            
            debug!("View mode toggled to: {:?} for tab {}", new_mode, tab.id);

            // Handle sync scrolling when switching modes
            // Note: Split mode shows both panes, so scroll sync is handled in real-time
            if sync_enabled && new_mode != ViewMode::Split && old_mode != ViewMode::Split {
                let content_height = tab.content_height;
                let viewport_height = tab.viewport_height;
                let max_scroll = (content_height - viewport_height).max(0.0);
                
                // Check if we're at boundaries (within 5px tolerance)
                let at_top = current_scroll < 5.0;
                let at_bottom = max_scroll > 0.0 && (max_scroll - current_scroll) < 5.0;
                
                if at_top {
                    // At top - stay at top
                    tab.pending_scroll_offset = Some(0.0);
                    debug!("Sync scroll: at top, staying at top");
                } else if at_bottom {
                    // At bottom - use ratio to stay at bottom
                    tab.pending_scroll_ratio = Some(1.0);
                    debug!("Sync scroll: at bottom, using ratio=1.0");
                } else {
                    // In the middle - use line-based mapping for content preservation
                    match (old_mode, new_mode) {
                        (ViewMode::Raw, ViewMode::Rendered) => {
                            // Calculate which line is at the top of viewport
                            let line_height = tab.raw_line_height;
                            let topmost_line = if line_height > 0.0 {
                                ((current_scroll / line_height) as usize).saturating_add(1)
                            } else {
                                1
                            };
                            
                            // Store for line-based lookup after render (Rendered mode uses tab field)
                            tab.pending_scroll_to_line = Some(topmost_line);
                            debug!(
                                "Sync scroll RawΓåÆRendered: scroll={} / line_height={:.1} ΓåÆ line {}",
                                current_scroll, line_height, topmost_line
                            );
                        }
                        (ViewMode::Rendered, ViewMode::Raw) => {
                            // Find which line is at current scroll position using mappings
                            if let Some(source_line) = Self::find_source_line_for_rendered_y_interpolated(
                                &line_mappings,
                                current_scroll,
                                content_height,
                            ) {
                                // Raw mode EditorWidget uses App-level pending_scroll_to_line
                                // (not tab field), so we store it for setting after borrow ends.
                                // source_line is 1-indexed from the mapping.
                                raw_mode_scroll_to_line = Some(source_line);
                                debug!(
                                    "Sync scroll RenderedΓåÆRaw: scroll={} ΓåÆ line {} (will use App-level pending_scroll_to_line)",
                                    current_scroll, source_line
                                );
                            } else {
                                // Fallback to percentage if no mappings
                                let scroll_ratio = if max_scroll > 0.0 {
                                    (current_scroll / max_scroll).clamp(0.0, 1.0)
                                } else {
                                    0.0
                                };
                                tab.pending_scroll_ratio = Some(scroll_ratio);
                                debug!(
                                    "Sync scroll RenderedΓåÆRaw: no mappings, using ratio={:.3}",
                                    scroll_ratio
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Mark settings dirty to save per-tab view mode on exit
            self.state.mark_settings_dirty();
        }

        // Set App-level pending_scroll_to_line AFTER releasing mutable borrow.
        // This is used by Raw mode EditorWidget which reads from self.pending_scroll_to_line.
        if let Some(line) = raw_mode_scroll_to_line {
            self.pending_scroll_to_line = Some(line);
        }
    }
    
    /// Find the rendered Y position for a given source line using interpolated line mappings.
    /// This provides sub-element precision by interpolating within elements.
    pub(crate) fn find_rendered_y_for_line_interpolated(
        mappings: &[(usize, usize, f32)],
        line: usize,
        content_height: f32,
    ) -> Option<f32> {
        if mappings.is_empty() {
            return None;
        }
        
        // Find the element containing this line
        for (i, (start, end, y)) in mappings.iter().enumerate() {
            if line >= *start && line <= *end {
                // Found the element - now interpolate within it
                let element_height = if i + 1 < mappings.len() {
                    mappings[i + 1].2 - y  // Next element's Y - this element's Y
                } else {
                    (content_height - y).max(20.0)  // Last element - use remaining height
                };
                
                // Calculate progress within the element (0.0 to 1.0)
                let line_span = (*end - *start + 1) as f32;
                let progress = if line_span > 1.0 {
                    (line - *start) as f32 / line_span
                } else {
                    0.0
                };
                
                return Some(y + progress * element_height);
            }
        }
        
        // Line is beyond all mappings - return end position
        if let Some((_, _, y)) = mappings.last() {
            return Some(*y);
        }
        
        None
    }

    /// Find the source line for a given rendered Y position using interpolated line mappings.
    pub(crate) fn find_source_line_for_rendered_y_interpolated(
        mappings: &[(usize, usize, f32)],
        rendered_y: f32,
        content_height: f32,
    ) -> Option<usize> {
        if mappings.is_empty() {
            return None;
        }
        
        // Find the element at this Y position
        for (i, (start, end, y)) in mappings.iter().enumerate() {
            let next_y = if i + 1 < mappings.len() {
                mappings[i + 1].2
            } else {
                content_height
            };
            
            if rendered_y >= *y && rendered_y < next_y {
                // Found the element - interpolate to find the line
                let element_height = next_y - y;
                let progress = if element_height > 0.0 {
                    (rendered_y - y) / element_height
                } else {
                    0.0
                };
                
                let line_span = (*end - *start + 1) as f32;
                let line = *start + (progress * line_span) as usize;
                return Some(line.min(*end));
            }
        }
        
        // Beyond all mappings - return last line
        if let Some((_, end, _)) = mappings.last() {
            return Some(*end);
        }
        
        None
    }

    /// Set the application theme and apply it immediately.
    #[allow(dead_code)]
    pub(crate) fn handle_set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
        self.theme_manager.set_theme(theme);
        self.theme_manager.apply(ctx);

        // Save preference to settings
        self.state.settings.theme = theme;
        self.state.mark_settings_dirty();

        info!("Theme changed to: {:?}", theme);
    }

    /// Cycle through available themes (Light -> Dark -> System).
    pub(crate) fn handle_cycle_theme(&mut self, ctx: &egui::Context) {
        let new_theme = self.theme_manager.cycle();
        self.theme_manager.apply(ctx);

        // Save preference to settings
        self.state.settings.theme = new_theme;
        self.state.mark_settings_dirty();

        info!("Theme cycled to: {:?}", new_theme);
    }

    /// Handle the Undo action (Ctrl+Z).
    ///
    /// Restores the previous content state from the undo stack.
    /// Preserves scroll position, focus, and cursor position across the undo operation.
    pub(crate) fn handle_undo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_undo() {
                let undo_count = tab.undo_count();
                // Preserve scroll position before undo
                let current_scroll = tab.scroll_offset;
                // Perform undo - returns the cursor position from the undo entry
                if let Some(restored_cursor) = tab.undo() {
                    tab.pending_scroll_offset = Some(current_scroll);
                    tab.needs_focus = true;
                    let char_count = tab.content.chars().count();
                    tab.pending_cursor_restore = Some(restored_cursor.min(char_count));
                    let time = self.get_app_time();
                    self.state.show_toast(
                        t!("notification.undo", remaining = undo_count.saturating_sub(1)).to_string(),
                        time,
                        1.5,
                    );
                    debug!("Undo performed, {} entries remaining", undo_count - 1);
                }
            } else {
                let time = self.get_app_time();
                self.state.show_toast(t!("notification.nothing_to_undo").to_string(), time, 1.5);
                debug!("Undo requested but stack is empty");
            }
        }
    }

    /// Handle the Redo action (Ctrl+Y or Ctrl+Shift+Z).
    ///
    /// Restores the next content state from the redo stack.
    /// Preserves scroll position, focus, and cursor position across the redo operation.
    pub(crate) fn handle_redo(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if tab.can_redo() {
                let redo_count = tab.redo_count();
                // Preserve scroll position before redo
                let current_scroll = tab.scroll_offset;
                // Perform redo - returns the cursor position from the redo entry
                if let Some(restored_cursor) = tab.redo() {
                    tab.pending_scroll_offset = Some(current_scroll);
                    tab.needs_focus = true;
                    let char_count = tab.content.chars().count();
                    tab.pending_cursor_restore = Some(restored_cursor.min(char_count));
                    let time = self.get_app_time();
                    self.state.show_toast(
                        t!("notification.redo", remaining = redo_count.saturating_sub(1)).to_string(),
                        time,
                        1.5,
                    );
                    debug!("Redo performed, {} entries remaining", redo_count - 1);
                }
            } else {
                let time = self.get_app_time();
                self.state.show_toast(t!("notification.nothing_to_redo").to_string(), time, 1.5);
                debug!("Redo requested but stack is empty");
            }
        }
    }

    // Panel toggle handlers
    /// Toggle the outline panel visibility.
    pub(crate) fn handle_toggle_outline(&mut self) {
        self.state.settings.outline_enabled = !self.state.settings.outline_enabled;
        self.state.mark_settings_dirty();

        let time = self.get_app_time();
        if self.state.settings.outline_enabled {
            self.state.show_toast(t!("notification.outline_shown").to_string(), time, 1.5);
        } else {
            self.state.show_toast(t!("notification.outline_hidden").to_string(), time, 1.5);
        }

        debug!(
            "Outline panel toggled: {}",
            self.state.settings.outline_enabled
        );
    }

    /// Toggle the terminal panel visibility.
    pub(crate) fn handle_toggle_terminal(&mut self) {
        // Set working directory from workspace or current file's directory
        let working_dir = self.state.workspace.as_ref()
            .map(|w| w.root_path.clone())
            .or_else(|| {
                self.state.active_tab()
                    .and_then(|t| t.path.as_ref())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            });

        // Check if working directory changed - reset layout state if so
        if self.terminal_panel_state.working_dir != working_dir {
            self.terminal_panel_state.reset_workspace_layout_state();
        }
        self.terminal_panel_state.working_dir = working_dir;

        self.terminal_panel_state.toggle();

        // Try to auto-load workspace layout when panel opens (if enabled and not yet loaded)
        if self.terminal_panel_state.is_visible() && self.state.settings.terminal_auto_load_layout {
            if self.terminal_panel_state.try_load_workspace_layout() {
                let time = self.get_app_time();
                self.state.show_toast(t!("notification.loaded_terminal_layout").to_string(), time, 2.0);
            }
        }

        let time = self.get_app_time();
        if self.terminal_panel_state.is_visible() {
            self.state.show_toast(t!("notification.terminal_shown").to_string(), time, 1.5);
        } else {
            // Auto-save when hiding the panel (if enabled)
            if self.state.settings.terminal_auto_save_layout {
                self.terminal_panel_state.save_workspace_layout();
            }
            self.state.show_toast(t!("notification.terminal_hidden").to_string(), time, 1.5);
        }

        debug!(
            "Terminal panel toggled: {}",
            self.terminal_panel_state.is_visible()
        );
    }

    /// Ensure echo worker is spawned (lazy initialization).
    ///
    /// This is called before rendering panels that need the worker.
    /// The worker spawns only when the AI panel is first shown, not on app startup.
    #[cfg(feature = "async-workers")]
    pub(crate) fn ensure_echo_worker(&mut self) {
        if self.echo_worker.is_none() && self.state.settings.ai_panel_visible {
            info!("Spawning echo worker (lazy initialization)");
            self.echo_worker = Some(WorkerHandle::spawn(echo_worker));

            // Process ready signal
            if let Some(worker) = &self.echo_worker {
                match worker.response_rx.try_recv() {
                    Ok(WorkerResponse::Ready) => info!("Echo worker ready"),
                    _ => {}
                }
            }
        }
    }

    /// Toggle Zen Mode (distraction-free writing).
    pub(crate) fn handle_toggle_zen_mode(&mut self) {
        self.state.toggle_zen_mode();
        self.state.mark_settings_dirty();

        let time = self.get_app_time();
        if self.state.is_zen_mode() {
            self.state.show_toast(t!("notification.zen_enabled").to_string(), time, 1.5);
            info!("Zen Mode enabled");
        } else {
            self.state.show_toast(t!("notification.zen_disabled").to_string(), time, 1.5);
            info!("Zen Mode disabled");
        }
    }

    /// Toggle OS-level fullscreen mode.
    ///
    /// This is different from Zen Mode - fullscreen hides the taskbar/dock
    /// and makes the window cover the entire screen.
    pub(crate) fn handle_toggle_fullscreen(&mut self, ctx: &egui::Context) {
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        let new_fullscreen = !is_fullscreen;

        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(new_fullscreen));

        let time = self.get_app_time();
        if new_fullscreen {
            self.state.show_toast(t!("notification.fullscreen_enter").to_string(), time, 2.0);
            info!("Entered fullscreen mode");
        } else {
            self.state.show_toast(t!("notification.fullscreen_exit").to_string(), time, 1.5);
            info!("Exited fullscreen mode");
        }
    }

    /// Toggle the Live Pipeline panel for the active tab (JSON/YAML only).
    pub(crate) fn handle_toggle_pipeline(&mut self) {
        // Check if pipeline feature is enabled
        if !self.state.settings.pipeline_enabled {
            let time = self.get_app_time();
            self.state.show_toast(t!("notification.pipeline_disabled").to_string(), time, 2.0);
            return;
        }

        // Check if we're in Zen Mode (pipeline hidden in Zen Mode)
        if self.state.is_zen_mode() {
            let time = self.get_app_time();
            self.state.show_toast(t!("notification.pipeline_zen").to_string(), time, 2.0);
            return;
        }

        // Check if file type supports pipeline before getting mutable borrow
        let supports = self.state.active_tab().map(|t| t.supports_pipeline()).unwrap_or(false);
        if !supports {
            let file_type_name = self.state.active_tab()
                .map(|t| t.file_type().display_name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let time = self.get_app_time();
            self.state.show_toast(
                t!("notification.pipeline_unsupported", file_type = file_type_name).to_string(),
                time,
                2.5,
            );
            return;
        }

        // Toggle the pipeline panel and get the result
        let (is_visible, tab_id) = {
            if let Some(tab) = self.state.active_tab_mut() {
                tab.toggle_pipeline_panel();
                (tab.pipeline_visible(), tab.id)
            } else {
                return;
            }
        };

        // Show toast after the mutable borrow is released
        let time = self.get_app_time();
        if is_visible {
            self.state.show_toast(t!("notification.pipeline_opened").to_string(), time, 1.5);
            info!("Pipeline panel opened for tab {}", tab_id);
        } else {
            self.state.show_toast(t!("notification.pipeline_closed").to_string(), time, 1.5);
            info!("Pipeline panel closed for tab {}", tab_id);
        }
    }

    /// Navigate to a heading with text-based search and transient highlighting.
    ///
    /// This provides more precise navigation than line-based scrolling by:
    /// 1. Searching for the exact heading text in the document
    /// 2. Applying transient highlight to make the heading visible
    /// 3. Positioning the cursor at the heading
    pub(crate) fn navigate_to_heading(&mut self, nav: HeadingNavRequest) {
        // Find the byte range for the target line, then convert to character offsets
        // for the transient highlight (which expects char offsets, not byte offsets).
        let char_range = if let Some(tab) = self.state.active_tab() {
            let content = &tab.content;
            
            // Find the byte range for the target line (nav.line is 1-indexed)
            if let Some((byte_start, byte_end)) = super::helpers::find_line_byte_range(content, nav.line) {
                // IMPORTANT: Convert byte offsets to character offsets!
                // set_transient_highlight expects char offsets, but find_line_byte_range
                // returns byte offsets. For UTF-8 text with multi-byte characters
                // (emojis, non-ASCII), bytes != chars and using bytes causes the
                // highlight and cursor to land on the wrong line.
                let char_start = super::helpers::byte_to_char_offset(content, byte_start);
                let char_end = super::helpers::byte_to_char_offset(content, byte_end);
                Some((char_start, char_end))
            } else {
                None
            }
        } else {
            None
        };

        // Apply navigation using nav.line directly (already correct, 1-indexed)
        // We don't need to recalculate the line from char_offset since OutlineItem
        // already has the correct line number from outline extraction.
        if let Some(tab) = self.state.active_tab_mut() {
            if let Some((char_start, char_end)) = char_range {
                // Set transient highlight for the heading line
                tab.set_transient_highlight(char_start, char_end);
            }
            
            // Set cursor position using nav.line directly (convert to 0-indexed)
            // This is more reliable than recalculating from byte/char offsets.
            tab.cursor_position = (nav.line.saturating_sub(1), 0);
            // Prevent EditorWidget from overwriting this position
            tab.skip_cursor_sync = true;
            
            debug!(
                "Navigated to heading '{}' at line {} (char range: {:?})",
                nav.title.as_deref().unwrap_or("unknown"),
                nav.line,
                char_range
            );
        }

        // Set pending scroll AFTER releasing the mutable borrow.
        // Use App-level pending_scroll_to_line so EditorWidget calculates
        // scroll offset with fresh line height from ui.fonts().
        self.pending_scroll_to_line = Some(nav.line);
    }

    /// Find a heading near a specific line (for fuzzy matching).
    /// Returns character offsets (not byte offsets) for use with egui.
    pub(crate) fn find_heading_near_line(
        content: &str,
        title: &str,
        level: u8,
        expected_line: usize,
    ) -> Option<(usize, usize)> {
        let hashes = "#".repeat(level as usize);
        let mut current_line: usize = 1;
        let mut char_offset: usize = 0; // Track character offset, not byte offset

        for line in content.lines() {
            // Check if we're near the expected line (within 5 lines)
            let diff = if current_line > expected_line {
                current_line - expected_line
            } else {
                expected_line - current_line
            };
            
            if diff <= 5 {
                // Check if this line is a heading of the right level
                if line.starts_with(&hashes) && !line.starts_with(&format!("{}#", hashes)) {
                    // Extract heading text after the hashes
                    let heading_text = line[hashes.len()..].trim();
                    // Case-insensitive comparison
                    if heading_text.eq_ignore_ascii_case(title) {
                        let start = char_offset;
                        let end = char_offset + line.chars().count();
                        return Some((start, end));
                    }
                }
            }
            
            // Add character count of this line plus 1 for newline
            char_offset += line.chars().count() + 1;
            current_line += 1;
            
            // Stop searching too far past the expected line
            if current_line > expected_line + 10 {
                break;
            }
        }
        None
    }

    /// Update the cached outline if the document content has changed.
    pub(crate) fn update_outline_if_needed(&mut self) {
        if let Some(tab) = self.state.active_tab() {
            // PERFORMANCE: Use content_version (O(1)) instead of hashing (O(n))
            // content_version is incremented whenever content changes
            let tab_id = tab.id;
            let content_version = tab.content_version();
            let path_hash = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                tab.path.hash(&mut hasher);
                hasher.finish()
            };
            
            // Combine tab_id, content_version, and path_hash for change detection
            // This is O(1) instead of O(n) for content hashing
            let change_key = (tab_id as u64)
                .wrapping_mul(31)
                .wrapping_add(content_version)
                .wrapping_mul(31)
                .wrapping_add(path_hash);

            // Only regenerate if content or path changed
            if change_key != self.last_outline_content_hash {
                // Use file-type aware outline extraction
                self.cached_outline = extract_outline_for_file(&tab.content, tab.path.as_deref());

                // Calculate document stats for markdown files
                if matches!(self.cached_outline.outline_type, OutlineType::Markdown) {
                    self.cached_doc_stats = Some(DocumentStats::from_text(&tab.content));
                } else {
                    self.cached_doc_stats = None;
                }

                self.last_outline_content_hash = change_key;
            }
        } else {
            // No active tab, clear outline and stats
            if !self.cached_outline.is_empty() {
                self.cached_outline = DocumentOutline::new();
                self.cached_doc_stats = None;
                self.last_outline_content_hash = 0;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Backlink Index
    // ─────────────────────────────────────────────────────────────────────────

    /// Refresh backlinks for the currently active file.
    ///
    /// Strategy:
    /// - For workspaces with ≤50 files: scan on demand each time
    /// - For workspaces with >50 files: build full index once, then use cached lookups
    /// - For single-file mode: scan files in the current directory
    pub(crate) fn refresh_backlinks(&mut self) {
        let current_filename = self
            .state
            .active_tab()
            .and_then(|tab| tab.path.as_ref())
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());

        let current_path = self
            .state
            .active_tab()
            .and_then(|tab| tab.path.clone());

        let Some(filename) = current_filename else {
            // No file open or unsaved — clear backlinks
            self.backlinks_panel.clear();
            return;
        };

        // Check if backlinks are already cached for this file
        if self.backlinks_panel.cached_for_file() == Some(filename.as_str())
            && !self.backlinks_need_refresh
        {
            return;
        }

        if let Some(workspace) = &self.state.workspace {
            // Walk the workspace directory to find all markdown files.
            // We use walkdir instead of workspace.all_files() because the file tree
            // uses shallow/lazy loading — subdirectories may not be scanned yet
            // (e.g., on session restore), which would miss backlink sources.
            let root = workspace.root_path.clone();
            let hidden = workspace.hidden_patterns.clone();
            let all_md_files = collect_markdown_files(&root, &hidden);
            let file_count = all_md_files.len();

            if file_count <= 50 {
                // Small workspace: scan on demand
                let backlinks = BacklinkIndex::scan_on_demand(
                    &filename,
                    &all_md_files,
                    current_path.as_deref(),
                );
                debug!(
                    "Backlinks (on-demand scan): {} backlinks for '{}'",
                    backlinks.len(),
                    filename
                );
                self.backlinks_panel
                    .set_backlinks(Some(filename), backlinks);
            } else {
                // Large workspace: use cached index
                if !self.state.backlink_index.is_built
                    || self.state.backlink_index.file_count != file_count
                {
                    debug!(
                        "Building backlink index for {} files...",
                        file_count
                    );
                    self.state.backlink_index.build_from_files(&all_md_files);
                }

                let mut backlinks = self.state.backlink_index.get_backlinks(&filename);
                // Filter out self-references
                if let Some(ref cp) = current_path {
                    backlinks.retain(|e| &e.source_path != cp);
                }
                debug!(
                    "Backlinks (cached index): {} backlinks for '{}'",
                    backlinks.len(),
                    filename
                );
                self.backlinks_panel
                    .set_backlinks(Some(filename), backlinks);
            }
        } else {
            // Single-file mode: scan markdown files in the current file's directory
            if let Some(ref path) = current_path {
                if let Some(parent) = path.parent() {
                    let dir_files: Vec<std::path::PathBuf> = std::fs::read_dir(parent)
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.is_file())
                        .collect();

                    let backlinks = BacklinkIndex::scan_on_demand(
                        &filename,
                        &dir_files,
                        Some(path),
                    );
                    debug!(
                        "Backlinks (directory scan): {} backlinks for '{}'",
                        backlinks.len(),
                        filename
                    );
                    self.backlinks_panel
                        .set_backlinks(Some(filename), backlinks);
                } else {
                    self.backlinks_panel.set_backlinks(Some(filename), vec![]);
                }
            } else {
                self.backlinks_panel.clear();
            }
        }
    }
}

/// Walk the workspace directory recursively to collect all markdown files,
/// skipping hidden directories (same patterns as the file tree).
///
/// This is independent of the lazy file tree, so it works even when
/// subdirectories haven't been expanded yet (e.g., right after session restore).
fn collect_markdown_files(root: &Path, hidden_patterns: &[String]) -> Vec<PathBuf> {
    use walkdir::WalkDir;

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            // Skip hidden dot-directories/files
            if name.starts_with('.') {
                return false;
            }
            // Skip directories matching hidden patterns
            if entry.file_type().is_dir() {
                for pattern in hidden_patterns {
                    if name == pattern.as_str() {
                        return false;
                    }
                }
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            if !e.file_type().is_file() {
                return false;
            }
            match e.path().extension().and_then(|ext| ext.to_str()) {
                Some(ext) => ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"),
                None => false,
            }
        })
        .map(|e| e.into_path())
        .collect()
}
