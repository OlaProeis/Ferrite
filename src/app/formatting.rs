//! Formatting operations for the Ferrite application.
//!
//! This module contains markdown formatting commands, TOC insertion,
//! and structured document (JSON/YAML/TOML) format/validate.

use super::FerriteApp;
use super::helpers::{char_index_to_line_col, line_col_to_char_index};
use crate::markdown::{apply_raw_format, insert_or_update_toc, MarkdownFormatCommand, TocOptions};
use crate::state::FileType;
use eframe::egui;
use log::{debug, info, warn};
use rust_i18n::t;

impl FerriteApp {

    /// Handle a markdown formatting command.
    ///
    /// Applies the formatting to the current selection in the active editor.
    /// Uses FerriteEditor when available for better undo/redo integration,
    /// falling back to TabState-based formatting for legacy code paths.
    pub(crate) fn handle_format_command(&mut self, ctx: &egui::Context, cmd: MarkdownFormatCommand) {
        use crate::editor::get_ferrite_editor_mut;

        // Get tab_id before borrowing state mutably
        let tab_id = self.state.active_tab().map(|t| t.id);

        if let Some(tab_id) = tab_id {
            // Try to use FerriteEditor (preferred path)
            let ferrite_result = get_ferrite_editor_mut(ctx, tab_id, |editor| {
                let applied = editor.apply_markdown_format(cmd);
                if applied {
                    // Return new content and cursor state for syncing back to TabState
                    let content = editor.buffer().to_string();
                    let cursor = editor.cursor();
                    let selection = if editor.has_selection() {
                        let sel = editor.selection();
                        let (start, end) = sel.ordered();
                        // Convert cursors to char indices (with bounds checking)
                        let start_char = editor.buffer().try_line_to_char(start.line).unwrap_or(0) + start.column;
                        let end_char = editor.buffer().try_line_to_char(end.line).unwrap_or(0) + end.column;
                        Some((start_char, end_char))
                    } else {
                        None
                    };
                    Some((content, (cursor.line, cursor.column), selection))
                } else {
                    None
                }
            });

            // Sync result back to TabState if FerriteEditor was used
            if let Some(Some((content, cursor_pos, selection))) = ferrite_result {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.content = content;
                    tab.cursor_position = cursor_pos;
                    tab.selection = selection;
                    // Note: is_modified() automatically detects changes via content comparison
                    debug!(
                        "Applied formatting via FerriteEditor: {:?}, selection={:?}",
                        cmd, tab.selection
                    );
                }
                return;
            }
        }

        // Fallback: Use TabState-based formatting (legacy path)
        if let Some(tab) = self.state.active_tab_mut() {
            let content = tab.content.clone();

            // Use actual selection if available, otherwise use cursor position
            let selection = if let Some((start, end)) = tab.selection {
                Some((start, end))
            } else {
                // Fall back to cursor position (no selection = insertion point)
                let cursor_pos = tab.cursor_position;
                let char_index = line_col_to_char_index(&content, cursor_pos.0, cursor_pos.1);
                Some((char_index, char_index))
            };

            // Apply formatting
            let result = apply_raw_format(&content, selection, cmd);

            // Update content through tab to maintain undo history
            tab.set_content(result.text.clone());

            // Update cursor position and clear selection
            if let Some((sel_start, sel_end)) = result.selection {
                // There's a new selection to set
                let (line, col) = char_index_to_line_col(&result.text, sel_end);
                tab.cursor_position = (line, col);
                tab.selection = Some((sel_start, sel_end));
            } else {
                // Just move cursor to result position
                let (line, col) = char_index_to_line_col(&result.text, result.cursor);
                tab.cursor_position = (line, col);
                tab.selection = None;
            }

            debug!(
                "Applied formatting via TabState: {:?}, applied={}, selection={:?}",
                cmd, result.applied, tab.selection
            );
        }
    }


    /// Handle a markdown formatting command with a pre-captured selection.
    ///
    /// This variant is used when the selection was captured at button-click time
    /// to ensure formatting is applied to the correct text even if focus changed.
    pub(crate) fn handle_format_command_with_selection(
        &mut self,
        ctx: &egui::Context,
        cmd: MarkdownFormatCommand,
        captured_selection: Option<(usize, usize)>,
    ) {
        use crate::editor::get_ferrite_editor_mut;

        // Get tab_id before borrowing state mutably
        let tab_id = self.state.active_tab().map(|t| t.id);

        if let Some(tab_id) = tab_id {
            if let Some(selection) = captured_selection {
                debug!("Using captured selection {:?} for tab_id {}", selection, tab_id);
                // Use the pre-captured selection with FerriteEditor
                let ferrite_result = get_ferrite_editor_mut(ctx, tab_id, |editor| {
                    let applied = editor.apply_markdown_format_with_selection(cmd, selection);
                    if applied {
                        let content = editor.buffer().to_string();
                        let cursor = editor.cursor();
                        let new_selection = if editor.has_selection() {
                            let sel = editor.selection();
                            let (start, end) = sel.ordered();
                            // Use try_line_to_char for bounds safety
                            let start_char = editor.buffer().try_line_to_char(start.line).unwrap_or(0) + start.column;
                            let end_char = editor.buffer().try_line_to_char(end.line).unwrap_or(0) + end.column;
                            Some((start_char, end_char))
                        } else {
                            None
                        };
                        Some((content, (cursor.line, cursor.column), new_selection))
                    } else {
                        None
                    }
                });

                // Sync result back to TabState
                if let Some(Some((content, cursor_pos, selection))) = ferrite_result {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.content = content;
                        tab.cursor_position = cursor_pos;
                        tab.selection = selection;
                        debug!(
                            "Applied formatting via FerriteEditor with captured selection: {:?}",
                            cmd
                        );
                    }
                    return;
                }
            }
        }

        // Fallback: use the standard method (will read current selection)
        debug!("Fallback: using handle_format_command for {:?}", cmd);
        self.handle_format_command(ctx, cmd);
    }


    /// Handle Table of Contents insertion/update.
    ///
    /// Finds an existing TOC block and updates it, or inserts a new one at the cursor.
    pub(crate) fn handle_insert_toc(&mut self) {
        // Check file type first (immutable borrow)
        let is_markdown = self
            .state
            .active_tab()
            .map(|t| t.file_type().is_markdown())
            .unwrap_or(false);

        if !is_markdown {
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.toc_markdown_only").to_string(), time, 2.0);
            return;
        }

        // Get the data needed for TOC generation (immutable borrow)
        let (content, cursor_pos) = {
            let tab = match self.state.active_tab() {
                Some(t) => t,
                None => return,
            };
            (tab.content.clone(), tab.cursor_position)
        };

        // Get cursor position as character index for insertion point
        let cursor_char_index = line_col_to_char_index(&content, cursor_pos.0, cursor_pos.1);

        // Generate and insert/update TOC
        let options = TocOptions::default();
        let result = insert_or_update_toc(&content, cursor_char_index, &options);

        // Update content through tab to maintain undo history (mutable borrow)
        if let Some(tab) = self.state.active_tab_mut() {
            tab.set_content(result.text.clone());

            // Update cursor position to after the TOC
            let (line, col) = char_index_to_line_col(&result.text, result.cursor);
            tab.cursor_position = (line, col);
            tab.selection = None;
        }

        // Show feedback
        let time = self.get_app_time();
        let msg = if result.was_update {
            t!("notification.toc_updated", count = result.heading_count).to_string()
        } else if result.heading_count > 0 {
            t!("notification.toc_inserted", count = result.heading_count).to_string()
        } else {
            t!("notification.toc_inserted_empty").to_string()
        };
        self.state.show_toast(&msg, time, 2.0);

        debug!(
            "TOC {}: {} headings",
            if result.was_update { "updated" } else { "inserted" },
            result.heading_count
        );
    }


    /// Handle formatting/pretty-printing a structured data document (JSON/YAML/TOML).
    pub(crate) fn handle_format_structured_document(&mut self) {
        use crate::markdown::tree_viewer::{parse_structured_content, serialize_tree};

        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast(t!("notification.no_document_format").to_string(), time, 2.0);
            return;
        };

        let file_type = tab.file_type();
        if !file_type.is_structured() {
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.not_structured").to_string(), time, 2.0);
            return;
        }

        let content = tab.content.clone();

        // Convert FileType to StructuredFileType
        let structured_type = match file_type {
            FileType::Json => crate::markdown::tree_viewer::StructuredFileType::Json,
            FileType::Yaml => crate::markdown::tree_viewer::StructuredFileType::Yaml,
            FileType::Toml => crate::markdown::tree_viewer::StructuredFileType::Toml,
            _ => return,
        };

        // Parse and reserialize to format
        match parse_structured_content(&content, structured_type) {
            Ok(tree) => {
                match serialize_tree(&tree, structured_type) {
                    Ok(formatted) => {
                        // Update the tab content
                        if let Some(tab) = self.state.active_tab_mut() {
                            let old_content = tab.content.clone();
                            let old_cursor = tab.cursors.primary().head;
                            tab.content = formatted;
                            tab.record_edit(old_content, old_cursor);
                        }
                        let time = self.get_app_time();
                        self.state.show_toast(t!("notification.document_formatted").to_string(), time, 2.0);
                        info!("Formatted {} document", file_type.display_name());
                    }
                    Err(e) => {
                        let time = self.get_app_time();
                        self.state
                            .show_toast(t!("notification.format_failed", error = e.to_string()).to_string(), time, 3.0);
                        warn!("Failed to serialize {}: {}", file_type.display_name(), e);
                    }
                }
            }
            Err(e) => {
                let time = self.get_app_time();
                self.state
                    .show_toast(t!("notification.parse_error", error = e.to_string()).to_string(), time, 3.0);
                warn!(
                    "Failed to parse {} for formatting: {}",
                    file_type.display_name(),
                    e
                );
            }
        }
    }


    /// Handle validating the syntax of a structured data document (JSON/YAML/TOML).
    pub(crate) fn handle_validate_structured_syntax(&mut self) {
        use crate::markdown::tree_viewer::parse_structured_content;

        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state.show_toast(t!("notification.no_document_validate").to_string(), time, 2.0);
            return;
        };

        let file_type = tab.file_type();
        if !file_type.is_structured() {
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.not_structured").to_string(), time, 2.0);
            return;
        }

        let content = tab.content.clone();

        // Convert FileType to StructuredFileType
        let structured_type = match file_type {
            FileType::Json => crate::markdown::tree_viewer::StructuredFileType::Json,
            FileType::Yaml => crate::markdown::tree_viewer::StructuredFileType::Yaml,
            FileType::Toml => crate::markdown::tree_viewer::StructuredFileType::Toml,
            _ => return,
        };

        // Try to parse to validate
        match parse_structured_content(&content, structured_type) {
            Ok(_) => {
                let time = self.get_app_time();
                self.state.show_toast(
                    t!("notification.valid_syntax", file_type = file_type.display_name()).to_string(),
                    time,
                    2.0,
                );
                info!("{} document is valid", file_type.display_name());
            }
            Err(e) => {
                let time = self.get_app_time();
                self.state.show_toast(t!("notification.invalid_syntax", error = e.to_string()).to_string(), time, 4.0);
                warn!("{} validation failed: {}", file_type.display_name(), e);
            }
        }
    }
}