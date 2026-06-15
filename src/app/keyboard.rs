//! Keyboard shortcut handling for the Ferrite application.
//!
//! This module detects keyboard shortcuts and dispatches them to the
//! appropriate handler methods.

use super::types::KeyboardAction;
use super::FerriteApp;
use crate::config::ShortcutCommand;
use crate::markdown::MarkdownFormatCommand;
use eframe::egui;
use log::{debug, info};
use rust_i18n::t;

impl FerriteApp {
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Skip ALL keyboard shortcuts if terminal has focus
        // Terminal handles its own keyboard input
        if self.terminal_panel_state.terminal_has_focus {
            return;
        }

        // Get keyboard shortcuts configuration
        let shortcuts = self.state.settings.keyboard_shortcuts.clone();

        ctx.input(|i| {
            // Helper macro to check if a shortcut matches
            macro_rules! check_shortcut {
                ($cmd:expr, $action:expr) => {
                    if shortcuts.get($cmd).matches(i) {
                        debug!(
                            "Keyboard shortcut: {} ({})",
                            shortcuts.get($cmd).display_string(),
                            $cmd.display_name()
                        );
                        return Some($action);
                    }
                };
            }

            // Check shortcuts in order (more specific shortcuts first)
            // File operations
            check_shortcut!(ShortcutCommand::SaveAs, KeyboardAction::SaveAs);
            check_shortcut!(ShortcutCommand::Save, KeyboardAction::Save);
            check_shortcut!(ShortcutCommand::Open, KeyboardAction::Open);
            check_shortcut!(ShortcutCommand::New, KeyboardAction::New);
            check_shortcut!(ShortcutCommand::NewTab, KeyboardAction::NewTab);
            check_shortcut!(ShortcutCommand::NewWindow, KeyboardAction::NewWindow);

            // Navigation - check more specific shortcuts first
            // Skip file tab navigation if terminal has focus (terminal handles its own tab switching)
            if !self.terminal_panel_state.terminal_has_focus {
                check_shortcut!(ShortcutCommand::PrevTab, KeyboardAction::PrevTab);
                check_shortcut!(ShortcutCommand::NextTab, KeyboardAction::NextTab);
            }

            // Close tab - skip if terminal has focus (Ctrl+W is used for word deletion in terminal)
            if !self.terminal_panel_state.terminal_has_focus {
                check_shortcut!(ShortcutCommand::CloseTab, KeyboardAction::CloseTab);
            }
            check_shortcut!(ShortcutCommand::GoToLine, KeyboardAction::GoToLine);
            check_shortcut!(ShortcutCommand::QuickOpen, KeyboardAction::QuickOpen);

            // View
            check_shortcut!(
                ShortcutCommand::ToggleViewMode,
                KeyboardAction::ToggleViewMode
            );
            check_shortcut!(ShortcutCommand::CycleTheme, KeyboardAction::CycleTheme);
            check_shortcut!(
                ShortcutCommand::ToggleZenMode,
                KeyboardAction::ToggleZenMode
            );
            check_shortcut!(
                ShortcutCommand::ToggleWordWrap,
                KeyboardAction::ToggleWordWrap
            );
            check_shortcut!(
                ShortcutCommand::ToggleFullscreen,
                KeyboardAction::ToggleFullscreen
            );
            check_shortcut!(
                ShortcutCommand::ToggleOutline,
                KeyboardAction::ToggleOutline
            );
            check_shortcut!(
                ShortcutCommand::ToggleFileTree,
                KeyboardAction::ToggleFileTree
            );
            check_shortcut!(
                ShortcutCommand::TogglePipeline,
                KeyboardAction::TogglePipeline
            );
            check_shortcut!(
                ShortcutCommand::ToggleTerminal,
                KeyboardAction::ToggleTerminal
            );
            check_shortcut!(
                ShortcutCommand::ToggleProductivityHub,
                KeyboardAction::ToggleProductivityHub
            );
            check_shortcut!(
                ShortcutCommand::ToggleFrontmatter,
                KeyboardAction::ToggleFrontmatter
            );
            check_shortcut!(ShortcutCommand::ZoomIn, KeyboardAction::ZoomIn);
            check_shortcut!(ShortcutCommand::ZoomOut, KeyboardAction::ZoomOut);
            check_shortcut!(ShortcutCommand::ResetZoom, KeyboardAction::ResetZoom);

            // Edit - note: Undo/Redo handled separately, MoveLineUp/Down handled separately
            check_shortcut!(ShortcutCommand::DeleteLine, KeyboardAction::DeleteLine);
            check_shortcut!(
                ShortcutCommand::DuplicateLine,
                KeyboardAction::DuplicateLine
            );
            check_shortcut!(
                ShortcutCommand::SelectNextOccurrence,
                KeyboardAction::SelectNextOccurrence
            );

            // Search
            check_shortcut!(
                ShortcutCommand::SearchInFiles,
                KeyboardAction::SearchInFiles
            );
            check_shortcut!(
                ShortcutCommand::FindReplace,
                KeyboardAction::OpenFindReplace
            );
            check_shortcut!(ShortcutCommand::Find, KeyboardAction::OpenFind);
            check_shortcut!(ShortcutCommand::FindNext, KeyboardAction::FindNext);
            check_shortcut!(ShortcutCommand::FindPrev, KeyboardAction::FindPrev);

            // Formatting - check more specific (Shift) shortcuts first
            check_shortcut!(
                ShortcutCommand::FormatBulletList,
                KeyboardAction::Format(MarkdownFormatCommand::BulletList)
            );
            check_shortcut!(
                ShortcutCommand::FormatNumberedList,
                KeyboardAction::Format(MarkdownFormatCommand::NumberedList)
            );
            check_shortcut!(
                ShortcutCommand::FormatCodeBlock,
                KeyboardAction::Format(MarkdownFormatCommand::CodeBlock)
            );
            check_shortcut!(
                ShortcutCommand::FormatImage,
                KeyboardAction::Format(MarkdownFormatCommand::Image)
            );
            check_shortcut!(
                ShortcutCommand::FormatBold,
                KeyboardAction::Format(MarkdownFormatCommand::Bold)
            );
            check_shortcut!(
                ShortcutCommand::FormatItalic,
                KeyboardAction::Format(MarkdownFormatCommand::Italic)
            );
            check_shortcut!(
                ShortcutCommand::FormatLink,
                KeyboardAction::Format(MarkdownFormatCommand::Link)
            );
            check_shortcut!(
                ShortcutCommand::FormatBlockquote,
                KeyboardAction::Format(MarkdownFormatCommand::Blockquote)
            );
            check_shortcut!(
                ShortcutCommand::FormatInlineCode,
                KeyboardAction::Format(MarkdownFormatCommand::InlineCode)
            );
            // Skip markdown heading shortcuts if terminal has focus (terminal uses Ctrl+1-9 for tab selection)
            if !self.terminal_panel_state.terminal_has_focus {
                check_shortcut!(
                    ShortcutCommand::FormatHeading1,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(1))
                );
                check_shortcut!(
                    ShortcutCommand::FormatHeading2,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(2))
                );
                check_shortcut!(
                    ShortcutCommand::FormatHeading3,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(3))
                );
                check_shortcut!(
                    ShortcutCommand::FormatHeading4,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(4))
                );
                check_shortcut!(
                    ShortcutCommand::FormatHeading5,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(5))
                );
                check_shortcut!(
                    ShortcutCommand::FormatHeading6,
                    KeyboardAction::Format(MarkdownFormatCommand::Heading(6))
                );
            }

            // Folding
            check_shortcut!(ShortcutCommand::FoldAll, KeyboardAction::FoldAll);
            check_shortcut!(ShortcutCommand::UnfoldAll, KeyboardAction::UnfoldAll);
            check_shortcut!(
                ShortcutCommand::ToggleFoldAtCursor,
                KeyboardAction::ToggleFoldAtCursor
            );

            // Other
            check_shortcut!(ShortcutCommand::OpenSettings, KeyboardAction::OpenSettings);
            check_shortcut!(ShortcutCommand::OpenAbout, KeyboardAction::OpenAbout);
            check_shortcut!(ShortcutCommand::ExportHtml, KeyboardAction::ExportHtml);
            check_shortcut!(ShortcutCommand::ExportPdf, KeyboardAction::ExportPdf);
            check_shortcut!(ShortcutCommand::PrintPreview, KeyboardAction::PrintPreview);
            check_shortcut!(ShortcutCommand::InsertToc, KeyboardAction::InsertToc);

            // Escape: Exit multi-cursor mode or close find panel (always hardcoded)
            if i.key_pressed(egui::Key::Escape) {
                debug!("Keyboard shortcut: Escape");
                return Some(KeyboardAction::ExitMultiCursor);
            }

            // F3/Shift+F3: Find next/prev (hardcoded fallback in case shortcut system misses it)
            // This ensures F3 works even when TextEdit in find panel has focus
            if i.key_pressed(egui::Key::F3) {
                if i.modifiers.shift {
                    debug!("Keyboard shortcut: Shift+F3 (Find Previous - hardcoded)");
                    return Some(KeyboardAction::FindPrev);
                } else {
                    debug!("Keyboard shortcut: F3 (Find Next - hardcoded)");
                    return Some(KeyboardAction::FindNext);
                }
            }

            None
        })
        .map(|action| match action {
            KeyboardAction::Save => self.handle_save_file(),
            KeyboardAction::SaveAs => self.handle_save_as_file(),
            KeyboardAction::Open => self.handle_open_file(),
            KeyboardAction::New => {
                self.state.new_tab();
            }
            KeyboardAction::NewTab => {
                self.state.new_tab();
            }
            KeyboardAction::NewWindow => {
                self.handle_new_window(ctx);
            }
            KeyboardAction::CloseTab => {
                self.handle_close_current_tab(ctx);
            }
            KeyboardAction::NextTab => {
                self.handle_next_tab();
            }
            KeyboardAction::PrevTab => {
                self.handle_prev_tab();
            }
            KeyboardAction::ToggleViewMode => {
                self.handle_toggle_view_mode();
            }
            KeyboardAction::CycleTheme => {
                self.handle_cycle_theme(ctx);
            }
            KeyboardAction::OpenSettings => {
                self.state.toggle_settings();
            }
            KeyboardAction::OpenAbout => {
                self.state.toggle_about();
            }
            KeyboardAction::OpenFind => {
                self.handle_open_find(false);
            }
            KeyboardAction::OpenFindReplace => {
                self.handle_open_find(true);
            }
            KeyboardAction::FindNext => {
                self.handle_find_next();
            }
            KeyboardAction::FindPrev => {
                self.handle_find_prev();
            }
            KeyboardAction::Format(cmd) => {
                self.handle_format_command(ctx, cmd);
            }
            KeyboardAction::ToggleOutline => {
                self.handle_toggle_outline();
            }
            KeyboardAction::ToggleFileTree => {
                self.handle_toggle_file_tree();
            }
            KeyboardAction::QuickOpen => {
                self.handle_quick_open();
            }
            KeyboardAction::SearchInFiles => {
                self.handle_search_in_files();
            }
            KeyboardAction::ExportHtml => {
                self.handle_open_html_export_dialog();
            }
            KeyboardAction::ExportPdf => {
                self.handle_open_pdf_export_dialog();
            }
            KeyboardAction::PrintPreview => {
                self.handle_print_preview(ctx);
            }
            KeyboardAction::SelectNextOccurrence => {
                self.handle_select_next_occurrence();
            }
            KeyboardAction::ExitMultiCursor => {
                // Priority order for Escape key:
                // 1. Exit fullscreen mode if active
                // 2. Exit multi-cursor mode if active
                // 3. Close find/replace panel
                let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if is_fullscreen {
                    // Exit fullscreen mode
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    let time = self.get_app_time();
                    self.state.show_toast(
                        t!("notification.fullscreen_exit").to_string(),
                        time,
                        1.5,
                    );
                    info!("Exited fullscreen mode via Escape");
                } else if let Some(tab) = self.state.active_tab_mut() {
                    if tab.has_multiple_cursors() {
                        debug!("Exiting multi-cursor mode");
                        tab.exit_multi_cursor_mode();
                    } else if self.state.ui.show_find_replace {
                        self.state.ui.show_find_replace = false;
                    }
                } else if self.state.ui.show_find_replace {
                    self.state.ui.show_find_replace = false;
                }
            }
            KeyboardAction::ToggleZenMode => {
                self.handle_toggle_zen_mode();
            }
            KeyboardAction::ToggleWordWrap => {
                self.handle_toggle_word_wrap();
            }
            KeyboardAction::ToggleFullscreen => {
                self.handle_toggle_fullscreen(ctx);
            }
            KeyboardAction::FoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.fold_all();
                        debug!("Folded all regions");
                    }
                }
            }
            KeyboardAction::UnfoldAll => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.unfold_all();
                        debug!("Unfolded all regions");
                    }
                }
            }
            KeyboardAction::ToggleFoldAtCursor => {
                if self.state.settings.folding_enabled {
                    if let Some(tab) = self.state.active_tab_mut() {
                        // Convert cursor position to line number (0-indexed)
                        let cursor_line = tab.cursor_position.0;
                        tab.toggle_fold_at_line(cursor_line);
                    }
                }
            }
            KeyboardAction::TogglePipeline => {
                self.handle_toggle_pipeline();
            }
            KeyboardAction::ToggleTerminal => {
                self.handle_toggle_terminal();
            }
            KeyboardAction::ToggleProductivityHub => {
                if self.state.settings.productivity_panel_docked {
                    // When docked, toggle the outline panel and switch to Productivity tab
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
            KeyboardAction::GoToLine => {
                self.handle_open_go_to_line();
            }
            KeyboardAction::DuplicateLine => {
                self.handle_duplicate_line();
            }
            KeyboardAction::DeleteLine => {
                self.handle_delete_line();
            }
            KeyboardAction::InsertToc => {
                self.handle_insert_toc();
            }
            KeyboardAction::ToggleFrontmatter => {
                if !self.state.settings.outline_enabled {
                    self.state.settings.outline_enabled = true;
                }
                self.outline_panel
                    .set_active_tab(crate::ui::OutlinePanelTab::Frontmatter);
                self.state.mark_settings_dirty();
            }
            KeyboardAction::ZoomIn => {
                egui::gui_zoom::zoom_in(ctx);
            }
            KeyboardAction::ZoomOut => {
                egui::gui_zoom::zoom_out(ctx);
            }
            KeyboardAction::ResetZoom => {
                ctx.set_zoom_factor(1.0);
            }
        });
    }
}
