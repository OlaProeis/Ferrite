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
use eframe::egui::{self, Color32, Response, RichText, Ui, Vec2};

/// Height of the ribbon in expanded state.
const RIBBON_HEIGHT_EXPANDED: f32 = 40.0;

/// Height of the ribbon in collapsed state.
const RIBBON_HEIGHT_COLLAPSED: f32 = 28.0;

/// Size of icon buttons.
const ICON_BUTTON_SIZE: Vec2 = Vec2::new(32.0, 28.0);

/// Actions that can be triggered from the ribbon.
#[derive(Debug, Clone, Copy, PartialEq)]
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

    // Settings (kept for keyboard shortcut handling, but removed from ribbon)
    /// Cycle through themes
    CycleTheme,
    /// Open settings panel (placeholder)
    OpenSettings,

    // Zen Mode (kept for keyboard shortcut handling, but removed from ribbon)
    /// Toggle Zen Mode (distraction-free writing)
    ToggleZenMode,

    // Ribbon control
    /// Toggle ribbon collapsed state
    ToggleCollapse,
}

/// Ribbon UI state and rendering.
#[derive(Debug, Clone)]
pub struct Ribbon {
    /// Whether the ribbon is in collapsed mode (icon-only).
    collapsed: bool,
}

impl Default for Ribbon {
    fn default() -> Self {
        Self::new()
    }
}

impl Ribbon {
    /// Create a new ribbon instance.
    pub fn new() -> Self {
        Self { collapsed: false }
    }

    /// Check if the ribbon is collapsed.
    #[allow(dead_code)]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Toggle the collapsed state.
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Get the current ribbon height.
    pub fn height(&self) -> f32 {
        if self.collapsed {
            RIBBON_HEIGHT_COLLAPSED
        } else {
            RIBBON_HEIGHT_EXPANDED
        }
    }

    /// Render the ribbon and return any triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `theme_colors` - Current theme colors for styling
    /// * `view_mode` - Current view mode (Raw/Rendered) - kept for compatibility
    /// * `show_line_numbers` - Whether line numbers are currently visible - kept for compatibility
    /// * `can_undo` - Whether undo is available
    /// * `can_redo` - Whether redo is available
    /// * `can_save` - Whether save is available (file has path and is modified)
    /// * `has_editor` - Whether an editor is currently active
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
        &mut self,
        ui: &mut Ui,
        theme_colors: &ThemeColors,
        _view_mode: ViewMode,
        _show_line_numbers: bool,
        can_undo: bool,
        can_redo: bool,
        _can_save: bool,
        has_editor: bool,
        formatting_state: Option<&FormattingState>,
        outline_enabled: bool,
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
            ui.set_height(self.height());
            ui.spacing_mut().item_spacing.x = 2.0;

            // Collapse/Expand toggle
            let collapse_icon = if self.collapsed { "▶" } else { "◀" };
            let collapse_tooltip = if self.collapsed {
                "Expand ribbon"
            } else {
                "Collapse ribbon"
            };
            if icon_button(ui, collapse_icon, collapse_tooltip, true, is_dark).clicked() {
                action = Some(RibbonAction::ToggleCollapse);
            }

            ui.add_space(4.0);
            vertical_separator(ui, separator_color, self.height() - 8.0);
            ui.add_space(4.0);

            // ═══════════════════════════════════════════════════════════════════
            // File Group (Streamlined with Save Dropdown)
            // ═══════════════════════════════════════════════════════════════════
            if !self.collapsed {
                ui.label(
                    RichText::new("File")
                        .size(10.0)
                        .color(theme_colors.text.muted),
                );
            }

            // New file button
            if icon_button(ui, "📄", &format!("New ({}+N)", modifier_symbol()), true, is_dark).clicked() {
                action = Some(RibbonAction::New);
            }

            // Open file button
            if icon_button(ui, "📂", &format!("Open File ({}+O)", modifier_symbol()), true, is_dark).clicked() {
                action = Some(RibbonAction::Open);
            }

            // Open Workspace / Close Workspace button
            if is_workspace_mode {
                if icon_button(ui, "📁", "Close Workspace", true, is_dark).clicked() {
                    action = Some(RibbonAction::CloseWorkspace);
                }
            } else if icon_button(ui, "📁", &format!("Open Folder ({}+Shift+O)", modifier_symbol()), true, is_dark).clicked()
            {
                action = Some(RibbonAction::OpenWorkspace);
            }

            // Workspace-only buttons: Search in Files and Quick File Switcher
            if is_workspace_mode {
                if icon_button(ui, "🔎", &format!("Search in Files ({}+Shift+F)", modifier_symbol()), true, is_dark).clicked()
                {
                    action = Some(RibbonAction::SearchInFiles);
                }

                if icon_button(ui, "⚡", &format!("Quick File Switcher ({}+P)", modifier_symbol()), true, is_dark).clicked() {
                    action = Some(RibbonAction::QuickFileSwitcher);
                }
            }

            // Save Dropdown - replaces separate Save and SaveAs buttons
            // Note: ComboBox adds its own dropdown arrow, so we don't add ▾ manually
            egui::ComboBox::from_id_source("save_dropdown")
                .selected_text(RichText::new("💾").size(14.0))
                .width(40.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(false, "💾 Save")
                        .on_hover_text(format!("Save ({}+S)", modifier_symbol()))
                        .clicked()
                    {
                        action = Some(RibbonAction::Save);
                    }
                    if ui
                        .selectable_label(false, "📥 Save As...")
                        .on_hover_text(format!("Save As ({}+Shift+S)", modifier_symbol()))
                        .clicked()
                    {
                        action = Some(RibbonAction::SaveAs);
                    }
                });

            ui.add_space(4.0);
            vertical_separator(ui, separator_color, self.height() - 8.0);
            ui.add_space(4.0);

            // ═══════════════════════════════════════════════════════════════════
            // Edit Group
            // ═══════════════════════════════════════════════════════════════════
            if !self.collapsed {
                ui.label(
                    RichText::new("Edit")
                        .size(10.0)
                        .color(theme_colors.text.muted),
                );
            }

            if icon_button(ui, "↩", &format!("Undo ({}+Z)", modifier_symbol()), can_undo, is_dark).clicked() {
                action = Some(RibbonAction::Undo);
            }

            if icon_button(ui, "↪", &format!("Redo ({}+Y)", modifier_symbol()), can_redo, is_dark).clicked() {
                action = Some(RibbonAction::Redo);
            }

            ui.add_space(4.0);
            vertical_separator(ui, separator_color, self.height() - 8.0);
            ui.add_space(4.0);

            // ═══════════════════════════════════════════════════════════════════
            // Format Group (Adaptive based on file type)
            // ═══════════════════════════════════════════════════════════════════
            if file_type.is_markdown() {
                // Markdown formatting buttons
                if !self.collapsed {
                    ui.label(
                        RichText::new("Format")
                            .size(10.0)
                            .color(theme_colors.text.muted),
                    );
                }

                // Get formatting state for button highlighting
                let is_bold = formatting_state.map(|s| s.is_bold).unwrap_or(false);
                let is_italic = formatting_state.map(|s| s.is_italic).unwrap_or(false);
                let is_code = formatting_state.map(|s| s.is_inline_code).unwrap_or(false);

                // Bold button
                if format_button(
                    ui,
                    "B",
                    &MarkdownFormatCommand::Bold.tooltip(),
                    has_editor,
                    is_bold,
                    is_dark,
                    true,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::Bold));
                }

                // Italic button
                if format_button(
                    ui,
                    "I",
                    &MarkdownFormatCommand::Italic.tooltip(),
                    has_editor,
                    is_italic,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::Italic));
                }

                // Inline code button
                if format_button(
                    ui,
                    "<>",
                    &MarkdownFormatCommand::InlineCode.tooltip(),
                    has_editor,
                    is_code,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::InlineCode));
                }

                // Link button
                let is_link = formatting_state.map(|s| s.is_link).unwrap_or(false);
                if format_button(
                    ui,
                    "[~]",
                    &MarkdownFormatCommand::Link.tooltip(),
                    has_editor,
                    is_link,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::Link));
                }

                ui.add_space(2.0);

                // Heading dropdown/buttons (compact)
                let current_heading = formatting_state.and_then(|s| s.heading_level);
                let heading_label = current_heading
                    .map(|h| format!("H{}", h as u8))
                    .unwrap_or_else(|| "H".to_string());

                egui::ComboBox::from_id_source("heading_dropdown")
                    .selected_text(RichText::new(heading_label).size(12.0))
                    .width(40.0)
                    .show_ui(ui, |ui| {
                        for level in 1..=6u8 {
                            let is_selected =
                                current_heading.map(|h| h as u8 == level).unwrap_or(false);
                            let label = format!("Heading {}", level);
                            if ui
                                .selectable_label(is_selected, &label)
                                .on_hover_text(format!("{}+{}", modifier_symbol(), level))
                                .clicked()
                            {
                                action = Some(RibbonAction::Format(
                                    MarkdownFormatCommand::Heading(level),
                                ));
                            }
                        }
                    });

                ui.add_space(2.0);

                // List buttons
                let is_bullet = formatting_state.map(|s| s.is_bullet_list).unwrap_or(false);
                let is_numbered = formatting_state
                    .map(|s| s.is_numbered_list)
                    .unwrap_or(false);

                if format_button(
                    ui,
                    "-",
                    &MarkdownFormatCommand::BulletList.tooltip(),
                    has_editor,
                    is_bullet,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::BulletList));
                }

                if format_button(
                    ui,
                    "1.",
                    &MarkdownFormatCommand::NumberedList.tooltip(),
                    has_editor,
                    is_numbered,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::NumberedList));
                }

                // Blockquote button
                let is_quote = formatting_state.map(|s| s.is_blockquote).unwrap_or(false);
                if format_button(
                    ui,
                    ">",
                    &MarkdownFormatCommand::Blockquote.tooltip(),
                    has_editor,
                    is_quote,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::Blockquote));
                }

                // Code block button
                let is_code_block = formatting_state.map(|s| s.is_code_block).unwrap_or(false);
                if format_button(
                    ui,
                    "{}",
                    &MarkdownFormatCommand::CodeBlock.tooltip(),
                    has_editor,
                    is_code_block,
                    is_dark,
                    false,
                )
                .clicked()
                {
                    action = Some(RibbonAction::Format(MarkdownFormatCommand::CodeBlock));
                }

                ui.add_space(4.0);
                vertical_separator(ui, separator_color, self.height() - 8.0);
                ui.add_space(4.0);
            } else if file_type.is_structured() {
                // Structured data buttons (JSON/YAML/TOML)
                if !self.collapsed {
                    ui.label(
                        RichText::new(file_type.display_name())
                            .size(10.0)
                            .color(theme_colors.text.muted),
                    );
                }

                // Format/Pretty-print button
                if icon_button(
                    ui,
                    "✨",
                    "Format Document (Pretty-print)",
                    has_editor,
                    is_dark,
                )
                .clicked()
                {
                    action = Some(RibbonAction::FormatDocument);
                }

                // Validate button
                if icon_button(ui, "✓", "Validate Syntax", has_editor, is_dark).clicked() {
                    action = Some(RibbonAction::ValidateSyntax);
                }

                // Pipeline button (JSON/YAML only, not TOML)
                if matches!(file_type, FileType::Json | FileType::Yaml) {
                    if icon_button(
                        ui,
                        "⚡",
                        &format!("Live Pipeline ({}+Shift+L)\nPipe document through shell commands", modifier_symbol()),
                        has_editor && pipeline_enabled,
                        is_dark,
                    )
                    .clicked()
                    {
                        action = Some(RibbonAction::TogglePipeline);
                    }
                }

                ui.add_space(4.0);
                vertical_separator(ui, separator_color, self.height() - 8.0);
                ui.add_space(4.0);
            }
            // Note: View Group removed - controls moved to title bar

            // ═══════════════════════════════════════════════════════════════════
            // Tools Group
            // ═══════════════════════════════════════════════════════════════════
            if !self.collapsed {
                ui.label(
                    RichText::new("Tools")
                        .size(10.0)
                        .color(theme_colors.text.muted),
                );
            }

            // Find/Replace (universal)
            if icon_button(ui, "🔍", &format!("Find/Replace ({}+F)", modifier_symbol()), true, is_dark).clicked() {
                action = Some(RibbonAction::FindReplace);
            }

            // Outline toggle
            let outline_icon = if outline_enabled { "📑" } else { "📋" };
            let outline_tooltip: String = if file_type.is_markdown() {
                if outline_enabled {
                    format!("Hide Outline ({}+Shift+O)", modifier_symbol())
                } else {
                    format!("Show Outline ({}+Shift+O)", modifier_symbol())
                }
            } else if file_type.is_structured() {
                if outline_enabled {
                    "Hide Info Panel".to_string()
                } else {
                    "Show Info Panel".to_string()
                }
            } else {
                "Toggle Outline".to_string()
            };
            if icon_button(ui, outline_icon, &outline_tooltip, true, is_dark).clicked() {
                action = Some(RibbonAction::ToggleOutline);
            }

            ui.add_space(4.0);
            vertical_separator(ui, separator_color, self.height() - 8.0);
            ui.add_space(4.0);

            // ═══════════════════════════════════════════════════════════════════
            // Export Dropdown (Markdown only)
            // ═══════════════════════════════════════════════════════════════════
            if file_type.is_markdown() {
                // Note: ComboBox adds its own dropdown arrow, so we don't add ▾ manually
                let export_label = if self.collapsed { "🌐" } else { "Export" };
                egui::ComboBox::from_id_source("export_dropdown")
                    .selected_text(RichText::new(export_label).size(12.0))
                    .width(if self.collapsed { 40.0 } else { 65.0 })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(false, "🌐 Export as HTML")
                            .on_hover_text(format!("Export as HTML ({}+Shift+E)", modifier_symbol()))
                            .clicked()
                        {
                            action = Some(RibbonAction::ExportHtml);
                        }
                        if ui
                            .selectable_label(false, "📋 Copy as HTML")
                            .on_hover_text("Copy rendered HTML to clipboard")
                            .clicked()
                        {
                            action = Some(RibbonAction::CopyAsHtml);
                        }
                        ui.separator();
                        ui.add_enabled_ui(false, |ui| {
                            ui.selectable_label(false, "📄 Export as PDF")
                                .on_hover_text("Coming soon");
                        });
                    });
            }

            // Note: Settings Group removed - controls moved to title bar and Settings panel
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
            .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);
    }

    // Apply vertical offset for icons that render at wrong baseline
    let y_offset = match icon {
        "⚙" => 2.0,
        _ => 0.0,
    };

    let icon_pos = egui::pos2(btn.rect.center().x, btn.rect.center().y + y_offset);

    ui.painter().text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(16.0),
        text_color,
    );

    btn.on_hover_text(tooltip)
}

/// Render a format button with active state highlighting.
#[allow(clippy::too_many_arguments)]
fn format_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    active: bool,
    is_dark: bool,
    bold_text: bool,
) -> Response {
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

    let active_bg = if is_dark {
        Color32::from_rgb(70, 90, 120)
    } else {
        Color32::from_rgb(200, 220, 240)
    };

    let hover_bg = if is_dark {
        Color32::from_rgb(60, 60, 60)
    } else {
        Color32::from_rgb(220, 220, 220)
    };

    let mut text = RichText::new(icon).size(12.0).color(text_color);
    if bold_text {
        text = text.strong();
    }

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .frame(false)
            .min_size(Vec2::new(24.0, 22.0)),
    );

    if active && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::Rounding::same(3.0), active_bg);

        let font_id = if bold_text {
            egui::FontId::new(12.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(12.0)
        };
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            font_id,
            text_color,
        );
    } else if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::Rounding::same(3.0), hover_bg);

        let font_id = if bold_text {
            egui::FontId::new(12.0, egui::FontFamily::Name("Inter-Bold".into()))
        } else {
            egui::FontId::proportional(12.0)
        };
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            font_id,
            text_color,
        );
    }

    btn.on_hover_text(tooltip)
}

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
    fn test_ribbon_new() {
        let ribbon = Ribbon::new();
        assert!(!ribbon.is_collapsed());
    }

    #[test]
    fn test_ribbon_toggle_collapsed() {
        let mut ribbon = Ribbon::new();
        assert!(!ribbon.is_collapsed());

        ribbon.toggle_collapsed();
        assert!(ribbon.is_collapsed());

        ribbon.toggle_collapsed();
        assert!(!ribbon.is_collapsed());
    }

    #[test]
    fn test_ribbon_height() {
        let mut ribbon = Ribbon::new();

        assert_eq!(ribbon.height(), RIBBON_HEIGHT_EXPANDED);

        ribbon.toggle_collapsed();

        assert_eq!(ribbon.height(), RIBBON_HEIGHT_COLLAPSED);
    }

    #[test]
    fn test_ribbon_default() {
        let ribbon = Ribbon::default();
        assert!(!ribbon.is_collapsed());
    }

    #[test]
    fn test_ribbon_action_equality() {
        assert_eq!(RibbonAction::New, RibbonAction::New);
        assert_ne!(RibbonAction::New, RibbonAction::Open);
    }
}
