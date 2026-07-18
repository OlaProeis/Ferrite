//! Ribbon UI Component for Ferrite
//!
//! This module implements a modern ribbon-style interface with icon-based controls
//! organized into logical groups, replacing the traditional menu bar.
//!
//! Design C: Streamlined ribbon with dropdowns, view controls moved to title bar.

use crate::app::modifier_symbol;
use crate::config::ViewMode;
use crate::markdown::formatting::{FormattingState, MarkdownFormatCommand};
use crate::state::FileType;
use crate::theme::ThemeColors;
use crate::ui::icons::{phosphor_font, phosphor_rich_text, RIBBON_ICON_SIZE};
use eframe::egui::{self, Color32, Response, RichText, Ui, Vec2};
use egui_phosphor::regular::{
    CHECK, CLIPBOARD, EXPORT, FILE_MAGNIFYING_GLASS, FILE_PDF, FILE_PLUS, FILE_TEXT, FLOPPY_DISK,
    FOLDERS, FOLDER_SIMPLE_MINUS, GLOBE, LIGHTNING, MAGNIFYING_GLASS, PRINTER, SPARKLE,
    TERMINAL_WINDOW,
};
use rust_i18n::t;

/// Height of the ribbon toolbar.
const RIBBON_HEIGHT: f32 = 28.0;

/// Size of icon buttons.
const ICON_BUTTON_SIZE: Vec2 = Vec2::new(32.0, 28.0);

/// Actions that can be triggered from the ribbon.
///
/// Some variants are defined for keyboard shortcut compatibility but are not
/// directly triggered from the ribbon UI. These are marked with comments.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // Some variants reserved for keyboard shortcuts
pub enum RibbonAction {
    // File operations
    /// Create a new file/tab
    New,
    /// Open file dialog
    Open,
    /// Open folder/workspace dialog
    OpenWorkspace,
    /// Close current workspace (return to single-file mode)
    CloseWorkspace,
    /// Save current file
    Save,
    /// Save As dialog
    SaveAs,
    /// Toggle auto-save for current document (kept for keyboard shortcut handling)
    ToggleAutoSave,

    // Workspace operations (only visible in workspace mode)
    /// Search in files across workspace (Ctrl+Shift+F)
    SearchInFiles,
    /// Quick file switcher / file palette (Ctrl+P)
    QuickFileSwitcher,

    // Edit operations
    /// Undo last change
    Undo,
    /// Redo last undone change
    Redo,

    // Formatting operations (Markdown)
    /// Apply a markdown formatting command
    Format(MarkdownFormatCommand),

    // Markdown document operations
    /// Insert or update Table of Contents
    InsertToc,

    // Structured data operations (JSON/YAML/TOML)
    /// Format/pretty-print the structured data document
    FormatDocument,
    /// Validate syntax of the structured data document
    ValidateSyntax,
    /// Toggle Live Pipeline panel (JSON/YAML only)
    TogglePipeline,

    // View operations (kept for keyboard shortcut handling, but removed from ribbon)
    /// Toggle between Raw and Rendered view
    ToggleViewMode,
    /// Toggle line numbers visibility
    ToggleLineNumbers,
    /// Toggle sync scrolling between Raw and Rendered views
    ToggleSyncScroll,

    // Tools
    /// Open Find/Replace dialog (placeholder)
    FindReplace,
    /// Toggle outline panel
    ToggleOutline,

    // Export operations
    /// Export current document as HTML file
    ExportHtml,
    /// Copy rendered HTML to clipboard
    CopyAsHtml,
    /// Export current document as a PDF file (opens the options dialog).
    ExportPdf,
    /// Print preview via in-app PDF viewer (same renderer as Export PDF).
    PrintPreview,

    // Settings (kept for keyboard shortcut handling, but removed from ribbon)
    /// Cycle through themes
    CycleTheme,
    /// Open settings panel (placeholder)
    OpenSettings,

    // Zen Mode (kept for keyboard shortcut handling, but removed from ribbon)
    /// Toggle Zen Mode (distraction-free writing)
    ToggleZenMode,

    // Terminal
    /// Toggle terminal panel visibility
    ToggleTerminal,

    // Productivity
    /// Toggle productivity hub visibility
    ToggleProductivity,

    // Frontmatter
    /// Toggle frontmatter editing panel
    ToggleFrontmatter,
}

/// Ribbon UI (icon-only toolbar).
#[derive(Debug, Clone, Copy, Default)]
pub struct Ribbon;

impl Ribbon {
    /// Create a new ribbon instance.
    pub fn new() -> Self {
        Self
    }

    /// Get the ribbon height.
    pub fn height(&self) -> f32 {
        RIBBON_HEIGHT
    }

    /// Render the ribbon and return any triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `theme_colors` - Current theme colors for styling
    /// * `view_mode` - Current view mode (Raw/Rendered) - kept for compatibility
    /// * `show_line_numbers` - Whether line numbers are currently visible - kept for compatibility
    /// * `can_save` - Whether save is available (file has path and is modified)
    /// * `has_editor` - Whether an editor is currently active
    /// * `new_file_enabled` - Whether the New File ribbon button can be used
    /// * `formatting_state` - Current formatting state at cursor (for button highlighting)
    /// * `outline_enabled` - Whether outline panel is currently visible
    /// * `sync_scroll_enabled` - Whether sync scrolling is enabled - kept for compatibility
    /// * `is_workspace_mode` - Whether app is in workspace mode
    /// * `file_type` - Current file type for adaptive toolbar
    /// * `zen_mode_enabled` - Whether Zen Mode is currently enabled - kept for compatibility
    /// * `auto_save_enabled` - Whether auto-save is enabled for current tab - kept for compatibility
    /// * `pipeline_enabled` - Whether pipeline panel is enabled
    ///
    /// # Returns
    ///
    /// Optional action triggered by user interaction
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &self,
        ui: &mut Ui,
        theme_colors: &ThemeColors,
        _view_mode: ViewMode,
        _show_line_numbers: bool,
        _can_save: bool,
        has_editor: bool,
        new_file_enabled: bool,
        _formatting_state: Option<&FormattingState>,
        _outline_enabled: bool,
        _sync_scroll_enabled: bool,
        is_workspace_mode: bool,
        file_type: FileType,
        _zen_mode_enabled: bool,
        _auto_save_enabled: bool,
        pipeline_enabled: bool,
    ) -> Option<RibbonAction> {
        let mut action: Option<RibbonAction> = None;
        let is_dark = theme_colors.is_dark();

        // Colors for the ribbon
        let ribbon_bg = if is_dark {
            Color32::from_rgb(40, 40, 40)
        } else {
            Color32::from_rgb(248, 248, 248)
        };

        // Separator color for ribbon dividers
        let separator_color = if is_dark {
            Color32::from_rgb(70, 70, 70)
        } else {
            Color32::from_rgb(165, 165, 165)
        };

        // Set ribbon background
        ui.painter()
            .rect_filled(ui.available_rect_before_wrap(), 0.0, ribbon_bg);

        ui.horizontal(|ui| {
            // Expand to full panel width so right-aligned controls stay visible.
            let ribbon_width = ui.available_width();
            ui.set_min_width(ribbon_width);
            ui.set_height(self.height());
            ui.spacing_mut().item_spacing.x = 2.0;

            // ═══════════════════════════════════════════════════════════════════
            // Left: File, workspace, save, format, find
            // ═══════════════════════════════════════════════════════════════════
            // New file button
            if icon_button(
                ui,
                FILE_PLUS,
                &format!("New ({}+N)", modifier_symbol()),
                new_file_enabled,
                is_dark,
            )
            .clicked()
            {
                action = Some(RibbonAction::New);
            }

            // Open file button
            if icon_button(
                ui,
                FILE_TEXT,
                &format!("Open File ({}+O)", modifier_symbol()),
                true,
                is_dark,
            )
            .clicked()
            {
                action = Some(RibbonAction::Open);
            }

            // Open Workspace / Close Workspace button
            if is_workspace_mode {
                if icon_button(ui, FOLDER_SIMPLE_MINUS, "Close Workspace", true, is_dark).clicked()
                {
                    action = Some(RibbonAction::CloseWorkspace);
                }
            } else if icon_button(
                ui,
                FOLDERS,
                &format!("Open Folder ({}+Shift+O)", modifier_symbol()),
                true,
                is_dark,
            )
            .clicked()
            {
                action = Some(RibbonAction::OpenWorkspace);
            }

            // Workspace-only buttons: Search in Files and Quick File Switcher
            if is_workspace_mode {
                if icon_button(
                    ui,
                    FILE_MAGNIFYING_GLASS,
                    &format!("Search in Files ({}+Shift+F)", modifier_symbol()),
                    true,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::SearchInFiles);
                }

                if icon_button(
                    ui,
                    LIGHTNING,
                    &format!("Quick File Switcher ({}+P)", modifier_symbol()),
                    true,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::QuickFileSwitcher);
                }
            }

            // Save Dropdown - replaces separate Save and SaveAs buttons
            egui::ComboBox::from_id_salt("save_dropdown")
                .selected_text(phosphor_rich_text(FLOPPY_DISK, 14.0))
                .width(40.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(false, t!("menu.file.save"))
                        .on_hover_text(format!("Save ({}+S)", modifier_symbol()))
                        .clicked()
                    {
                        action = Some(RibbonAction::Save);
                    }
                    if ui
                        .selectable_label(false, format!("{}...", t!("menu.file.save_as")))
                        .on_hover_text(format!("Save As ({}+Shift+S)", modifier_symbol()))
                        .clicked()
                    {
                        action = Some(RibbonAction::SaveAs);
                    }
                });

            // Format Group (Structured data only)
            if file_type.is_structured() {
                ui.add_space(4.0);
                vertical_separator(ui, separator_color, self.height() - 8.0);
                ui.add_space(4.0);

                if icon_button(
                    ui,
                    SPARKLE,
                    &t!("ribbon.format_document").to_string(),
                    has_editor,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::FormatDocument);
                }

                if icon_button(
                    ui,
                    CHECK,
                    &t!("ribbon.validate_syntax").to_string(),
                    has_editor,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::ValidateSyntax);
                }

                if matches!(file_type, FileType::Json | FileType::Yaml) {
                    if icon_button(
                        ui,
                        LIGHTNING,
                        &format!("{} ({}+Shift+L)", t!("ribbon.pipeline"), modifier_symbol()),
                        has_editor && pipeline_enabled,
                        is_dark,
                    )
                    .clicked()
                    {
                        action = Some(RibbonAction::TogglePipeline);
                    }
                }
            }

            // Find/Replace (universal)
            ui.add_space(4.0);
            vertical_separator(ui, separator_color, self.height() - 8.0);
            ui.add_space(4.0);

            if icon_button(
                ui,
                MAGNIFYING_GLASS,
                &format!("Find/Replace ({}+F)", modifier_symbol()),
                true,
                is_dark,
            )
            .clicked()
            {
                action = Some(RibbonAction::FindReplace);
            }

            // ═══════════════════════════════════════════════════════════════════
            // Right: Export + Terminal (pinned to the right edge)
            // ═══════════════════════════════════════════════════════════════════
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Terminal — rightmost (added first in RTL layout)
                if icon_button(
                    ui,
                    TERMINAL_WINDOW,
                    &format!("Toggle Terminal ({}+`)", modifier_symbol()),
                    true,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::ToggleTerminal);
                }

                if file_type.is_markdown() {
                    ui.add_space(4.0);
                    vertical_separator(ui, separator_color, self.height() - 8.0);
                    ui.add_space(4.0);

                    egui::ComboBox::from_id_salt("export_dropdown")
                        .selected_text(phosphor_rich_text(EXPORT, 14.0))
                        .width(40.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{} {}", GLOBE, t!("menu.file.export_html")),
                                )
                                .on_hover_text(format!(
                                    "Export as HTML ({}+Shift+E)",
                                    modifier_symbol()
                                ))
                                .clicked()
                            {
                                action = Some(RibbonAction::ExportHtml);
                            }
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{} {}", CLIPBOARD, t!("menu.file.export_clipboard")),
                                )
                                .on_hover_text(t!("ribbon.copy_html_tooltip").to_string())
                                .clicked()
                            {
                                action = Some(RibbonAction::CopyAsHtml);
                            }
                            ui.separator();
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{} {}", FILE_PDF, t!("ribbon.export_pdf")),
                                )
                                .on_hover_text(format!(
                                    "Export as PDF ({}+Shift+P)",
                                    modifier_symbol()
                                ))
                                .clicked()
                            {
                                action = Some(RibbonAction::ExportPdf);
                            }
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{} {}", PRINTER, t!("ribbon.print_preview")),
                                )
                                .on_hover_text(format!(
                                    "Print preview (+{}+Alt+P)",
                                    modifier_symbol()
                                ))
                                .clicked()
                            {
                                action = Some(RibbonAction::PrintPreview);
                            }
                        });
                }
            });
        });

        // Draw bottom border
        let rect = ui.min_rect();
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x, rect.max.y),
                egui::pos2(rect.max.x, rect.max.y),
            ],
            egui::Stroke::new(1.0, separator_color),
        );

        action
    }
}

/// Render an icon button with consistent styling.
fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str, enabled: bool, is_dark: bool) -> Response {
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
        egui::Button::new(RichText::new(" ").size(16.0))
            .frame(false)
            .min_size(ICON_BUTTON_SIZE),
    );

    if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::CornerRadius::same(3), hover_bg);
    }

    let icon_pos = egui::pos2(btn.rect.center().x, btn.rect.center().y);

    ui.painter().text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        icon,
        phosphor_font(RIBBON_ICON_SIZE),
        text_color,
    );

    btn.on_hover_text(tooltip)
}

// Note: format_button was removed - markdown formatting buttons moved to format_toolbar.rs

/// Draw a vertical separator line.
fn vertical_separator(ui: &mut Ui, color: Color32, height: f32) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(1.0, height), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(1.0, color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ribbon_height() {
        let ribbon = Ribbon::new();
        assert_eq!(ribbon.height(), RIBBON_HEIGHT);
    }

    #[test]
    fn test_ribbon_action_equality() {
        assert_eq!(RibbonAction::New, RibbonAction::New);
        assert_ne!(RibbonAction::New, RibbonAction::Open);
    }
}
