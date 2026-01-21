//! Settings Panel Component for Ferrite
//!
//! This module implements a modal settings panel that allows users to configure
//! appearance, editor behavior, and file handling options with live preview.

use crate::config::{CjkFontPreference, EditorFont, KeyBinding, KeyboardShortcuts, KeyCode, KeyModifiers, Language, MaxLineWidth, MinimapMode, Settings, ShortcutCommand, Theme, ViewMode};
use crate::fonts;
use crate::markdown::syntax::get_available_themes;
use eframe::egui::{self, Color32, RichText, Ui};
use rust_i18n::{set_locale, t};

// ─────────────────────────────────────────────────────────────────────────────
// Localized Display Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Get localized font description
fn font_description(font: &EditorFont) -> String {
    match font {
        EditorFont::Inter => t!("settings.editor.font_inter_desc").to_string(),
        EditorFont::JetBrainsMono => t!("settings.editor.font_jetbrains_desc").to_string(),
        EditorFont::Custom(_) => t!("settings.editor.custom_font_desc").to_string(),
    }
}

/// Get localized CJK preference display name
fn cjk_display_name(pref: &CjkFontPreference) -> String {
    match pref {
        CjkFontPreference::Auto => t!("settings.editor.cjk_auto").to_string(),
        CjkFontPreference::Korean => t!("settings.editor.cjk_korean").to_string(),
        CjkFontPreference::SimplifiedChinese => t!("settings.editor.cjk_simplified_chinese").to_string(),
        CjkFontPreference::TraditionalChinese => t!("settings.editor.cjk_traditional_chinese").to_string(),
        CjkFontPreference::Japanese => t!("settings.editor.cjk_japanese").to_string(),
    }
}

/// Get localized CJK preference description
fn cjk_description(pref: &CjkFontPreference) -> String {
    match pref {
        CjkFontPreference::Auto => t!("settings.editor.cjk_auto_desc").to_string(),
        CjkFontPreference::Korean => t!("settings.editor.cjk_korean_desc").to_string(),
        CjkFontPreference::SimplifiedChinese => t!("settings.editor.cjk_simplified_chinese_desc").to_string(),
        CjkFontPreference::TraditionalChinese => t!("settings.editor.cjk_traditional_chinese_desc").to_string(),
        CjkFontPreference::Japanese => t!("settings.editor.cjk_japanese_desc").to_string(),
    }
}

/// Get localized view mode description
fn view_mode_description(mode: &ViewMode) -> String {
    match mode {
        ViewMode::Raw => t!("view_mode.raw_desc").to_string(),
        ViewMode::Rendered => t!("view_mode.rendered_desc").to_string(),
        ViewMode::Split => t!("view_mode.split_desc").to_string(),
    }
}

/// Get localized shortcut command name
fn shortcut_command_name(cmd: &ShortcutCommand) -> String {
    match cmd {
        // File operations
        ShortcutCommand::Save => t!("shortcuts.commands.save").to_string(),
        ShortcutCommand::SaveAs => t!("shortcuts.commands.save_as").to_string(),
        ShortcutCommand::Open => t!("shortcuts.commands.open").to_string(),
        ShortcutCommand::New => t!("shortcuts.commands.new").to_string(),
        ShortcutCommand::NewTab => t!("shortcuts.commands.new_tab").to_string(),
        ShortcutCommand::CloseTab => t!("shortcuts.commands.close_tab").to_string(),
        // Navigation
        ShortcutCommand::NextTab => t!("shortcuts.commands.next_tab").to_string(),
        ShortcutCommand::PrevTab => t!("shortcuts.commands.prev_tab").to_string(),
        ShortcutCommand::GoToLine => t!("shortcuts.commands.go_to_line").to_string(),
        ShortcutCommand::QuickOpen => t!("shortcuts.commands.quick_open").to_string(),
        // View
        ShortcutCommand::ToggleViewMode => t!("shortcuts.commands.toggle_view_mode").to_string(),
        ShortcutCommand::CycleTheme => t!("shortcuts.commands.cycle_theme").to_string(),
        ShortcutCommand::ToggleZenMode => t!("shortcuts.commands.toggle_zen_mode").to_string(),
        ShortcutCommand::ToggleFullscreen => t!("shortcuts.commands.toggle_fullscreen").to_string(),
        ShortcutCommand::ToggleOutline => t!("shortcuts.commands.toggle_outline").to_string(),
        ShortcutCommand::ToggleFileTree => t!("shortcuts.commands.toggle_file_tree").to_string(),
        ShortcutCommand::TogglePipeline => t!("shortcuts.commands.toggle_pipeline").to_string(),
        // Edit
        ShortcutCommand::Undo => t!("shortcuts.commands.undo").to_string(),
        ShortcutCommand::Redo => t!("shortcuts.commands.redo").to_string(),
        ShortcutCommand::DeleteLine => t!("shortcuts.commands.delete_line").to_string(),
        ShortcutCommand::DuplicateLine => t!("shortcuts.commands.duplicate_line").to_string(),
        ShortcutCommand::MoveLineUp => t!("shortcuts.commands.move_line_up").to_string(),
        ShortcutCommand::MoveLineDown => t!("shortcuts.commands.move_line_down").to_string(),
        ShortcutCommand::SelectNextOccurrence => t!("shortcuts.commands.select_next_occurrence").to_string(),
        // Search
        ShortcutCommand::Find => t!("shortcuts.commands.find").to_string(),
        ShortcutCommand::FindReplace => t!("shortcuts.commands.find_replace").to_string(),
        ShortcutCommand::FindNext => t!("shortcuts.commands.find_next").to_string(),
        ShortcutCommand::FindPrev => t!("shortcuts.commands.find_prev").to_string(),
        ShortcutCommand::SearchInFiles => t!("shortcuts.commands.search_in_files").to_string(),
        // Formatting
        ShortcutCommand::FormatBold => t!("shortcuts.commands.bold").to_string(),
        ShortcutCommand::FormatItalic => t!("shortcuts.commands.italic").to_string(),
        ShortcutCommand::FormatInlineCode => t!("shortcuts.commands.inline_code").to_string(),
        ShortcutCommand::FormatCodeBlock => t!("shortcuts.commands.code_block").to_string(),
        ShortcutCommand::FormatLink => t!("shortcuts.commands.link").to_string(),
        ShortcutCommand::FormatImage => t!("shortcuts.commands.image").to_string(),
        ShortcutCommand::FormatBlockquote => t!("shortcuts.commands.blockquote").to_string(),
        ShortcutCommand::FormatBulletList => t!("shortcuts.commands.bullet_list").to_string(),
        ShortcutCommand::FormatNumberedList => t!("shortcuts.commands.numbered_list").to_string(),
        ShortcutCommand::FormatHeading1 => t!("shortcuts.commands.heading_1").to_string(),
        ShortcutCommand::FormatHeading2 => t!("shortcuts.commands.heading_2").to_string(),
        ShortcutCommand::FormatHeading3 => t!("shortcuts.commands.heading_3").to_string(),
        ShortcutCommand::FormatHeading4 => t!("shortcuts.commands.heading_4").to_string(),
        ShortcutCommand::FormatHeading5 => t!("shortcuts.commands.heading_5").to_string(),
        ShortcutCommand::FormatHeading6 => t!("shortcuts.commands.heading_6").to_string(),
        // Folding
        ShortcutCommand::FoldAll => t!("shortcuts.commands.fold_all").to_string(),
        ShortcutCommand::UnfoldAll => t!("shortcuts.commands.unfold_all").to_string(),
        ShortcutCommand::ToggleFoldAtCursor => t!("shortcuts.commands.toggle_fold").to_string(),
        // Other
        ShortcutCommand::OpenSettings => t!("shortcuts.commands.open_settings").to_string(),
        ShortcutCommand::OpenAbout => t!("shortcuts.commands.open_about").to_string(),
        ShortcutCommand::ExportHtml => t!("shortcuts.commands.export_html").to_string(),
        ShortcutCommand::InsertToc => t!("shortcuts.commands.insert_toc").to_string(),
    }
}

/// Settings panel sections for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Appearance,
    Editor,
    Files,
    Keyboard,
}

impl SettingsSection {
    /// Get the display label for the section.
    pub fn label(&self) -> String {
        match self {
            SettingsSection::Appearance => t!("settings.appearance.title"),
            SettingsSection::Editor => t!("settings.editor.title"),
            SettingsSection::Files => t!("settings.files.title"),
            SettingsSection::Keyboard => t!("settings.keyboard.title"),
        }
        .to_string()
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "🎨",
            SettingsSection::Editor => "📝",
            SettingsSection::Files => "📁",
            SettingsSection::Keyboard => "⌨",
        }
    }
}

/// Result of showing the settings panel.
#[derive(Debug, Clone, Default)]
pub struct SettingsPanelOutput {
    /// Whether settings were modified.
    pub changed: bool,
    /// Whether the panel should be closed.
    pub close_requested: bool,
    /// Whether a reset to defaults was requested.
    pub reset_requested: bool,
}

/// State for capturing a new key binding.
#[derive(Debug, Clone)]
pub struct KeyCaptureState {
    /// Which command is being rebound
    pub command: ShortcutCommand,
    /// Captured modifiers so far
    pub modifiers: KeyModifiers,
    /// Captured key (if any)
    pub key: Option<KeyCode>,
}

/// Settings panel state and rendering.
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    /// Currently active settings section.
    active_section: SettingsSection,
    /// State for capturing a new key binding (None if not capturing)
    key_capture: Option<KeyCaptureState>,
    /// Filter text for keyboard shortcuts search
    keyboard_filter: String,
    /// Conflict warning message (if any)
    conflict_warning: Option<(ShortcutCommand, String)>,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    /// Create a new settings panel instance.
    pub fn new() -> Self {
        Self {
            active_section: SettingsSection::default(),
            key_capture: None,
            keyboard_filter: String::new(),
            conflict_warning: None,
        }
    }

    /// Show the settings panel as a modal window.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context
    /// * `settings` - The current settings (mutable for live preview)
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Output indicating what actions to take
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        is_dark: bool,
    ) -> SettingsPanelOutput {
        let mut output = SettingsPanelOutput::default();

        // Semi-transparent overlay
        let screen_rect = ctx.screen_rect();
        let overlay_color = if is_dark {
            Color32::from_rgba_unmultiplied(0, 0, 0, 180)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 120)
        };

        egui::Area::new(egui::Id::new("settings_overlay"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(screen_rect, 0.0, overlay_color);

                // Close on click outside
                if response.clicked() {
                    output.close_requested = true;
                }
            });

        // Settings modal window - fixed size for consistent layout across tabs
        const CONTENT_HEIGHT: f32 = 480.0;
        const CONTENT_WIDTH: f32 = 420.0;
        const SIDEBAR_WIDTH: f32 = 120.0;

        egui::Window::new(format!("⚙ {}", t!("settings.title")))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([SIDEBAR_WIDTH + CONTENT_WIDTH + 32.0, CONTENT_HEIGHT + 80.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // Handle escape key to close
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    output.close_requested = true;
                }

                ui.horizontal(|ui| {
                    // Left side: Section tabs (fixed width)
                    ui.vertical(|ui| {
                        ui.set_min_width(SIDEBAR_WIDTH);
                        ui.set_max_width(SIDEBAR_WIDTH);
                        ui.set_min_height(CONTENT_HEIGHT);

                        for section in [
                            SettingsSection::Appearance,
                            SettingsSection::Editor,
                            SettingsSection::Files,
                            SettingsSection::Keyboard,
                        ] {
                            let selected = self.active_section == section;
                            let text = format!("{} {}", section.icon(), section.label());

                            let btn = ui.add_sized(
                                [SIDEBAR_WIDTH - 8.0, 32.0],
                                egui::SelectableLabel::new(
                                    selected,
                                    RichText::new(text).size(14.0),
                                ),
                            );

                            if btn.clicked() {
                                self.active_section = section;
                                // Clear keyboard capture state when switching sections
                                if section != SettingsSection::Keyboard {
                                    self.key_capture = None;
                                    self.conflict_warning = None;
                                }
                            }
                        }
                    });

                    ui.separator();

                    // Right side: Section content (fixed size with scroll)
                    ui.vertical(|ui| {
                        ui.set_min_width(CONTENT_WIDTH);
                        ui.set_max_width(CONTENT_WIDTH);
                        ui.set_min_height(CONTENT_HEIGHT);
                        ui.set_max_height(CONTENT_HEIGHT);

                        egui::ScrollArea::vertical()
                            .id_source(format!("settings_scroll_{:?}", self.active_section))
                            .max_height(CONTENT_HEIGHT)
                            .show(ui, |ui| {
                                ui.set_min_width(CONTENT_WIDTH - 16.0); // Account for scrollbar

                                match self.active_section {
                                    SettingsSection::Appearance => {
                                        if self.show_appearance_section(ui, settings, is_dark) {
                                            output.changed = true;
                                        }
                                    }
                                    SettingsSection::Editor => {
                                        if self.show_editor_section(ui, settings) {
                                            output.changed = true;
                                        }
                                    }
                                    SettingsSection::Files => {
                                        if self.show_files_section(ui, settings) {
                                            output.changed = true;
                                        }
                                    }
                                    SettingsSection::Keyboard => {
                                        if self.show_keyboard_section(ui, settings) {
                                            output.changed = true;
                                        }
                                    }
                                }
                            });
                    });
                });

                ui.separator();

                // Bottom buttons
                ui.horizontal(|ui| {
                    // Reset button on the left
                    if ui
                        .button(format!("↺ {}", t!("settings.reset_all")))
                        .on_hover_text(t!("settings.reset_tooltip"))
                        .clicked()
                    {
                        output.reset_requested = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t!("dialog.confirm.close")).clicked() {
                            output.close_requested = true;
                        }
                        ui.label(
                            RichText::new(t!("settings.auto_save_hint"))
                                .small()
                                .weak(),
                        );
                    });
                });
            });

        output
    }

    /// Show the Appearance settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_appearance_section(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        _is_dark: bool,
    ) -> bool {
        let mut changed = false;

        ui.heading(t!("settings.appearance.title"));
        ui.add_space(8.0);

        // Theme selection
        ui.label(RichText::new(t!("settings.general.theme")).strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for theme in [Theme::Light, Theme::Dark, Theme::System] {
                let label = match theme {
                    Theme::Light => format!("☀ {}", t!("settings.general.theme_light")),
                    Theme::Dark => format!("🌙 {}", t!("settings.general.theme_dark")),
                    Theme::System => format!("💻 {}", t!("settings.general.theme_system")),
                };
                if ui
                    .selectable_value(&mut settings.theme, theme, label)
                    .changed()
                {
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);

        // Syntax highlighting theme
        ui.label(RichText::new(t!("settings.appearance.syntax_theme")).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new(t!("settings.appearance.syntax_theme_hint"))
                .weak()
                .small(),
        );
        ui.add_space(4.0);

        let themes = get_available_themes();
        let current_display = if settings.syntax_theme.is_empty() {
            t!("settings.appearance.syntax_theme_auto").to_string()
        } else {
            themes
                .iter()
                .find(|(name, _)| name == &settings.syntax_theme)
                .map(|(_, display)| display.clone())
                .unwrap_or_else(|| settings.syntax_theme.clone())
        };

        egui::ComboBox::from_id_source("syntax_theme_combo")
            .selected_text(&current_display)
            .width(200.0)
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        // Auto option (empty string = use dark/light default)
                        if ui
                            .selectable_label(
                                settings.syntax_theme.is_empty(),
                                t!("settings.appearance.syntax_theme_auto"),
                            )
                            .clicked()
                        {
                            settings.syntax_theme = String::new();
                            changed = true;
                        }

                        ui.separator();

                        // List all available themes
                        for (theme_name, display_name) in &themes {
                            if ui
                                .selectable_label(
                                    &settings.syntax_theme == theme_name,
                                    display_name,
                                )
                                .clicked()
                            {
                                settings.syntax_theme = theme_name.clone();
                                changed = true;
                            }
                        }
                    });
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font family selection
        ui.label(RichText::new(t!("settings.editor.font_family")).strong());
        ui.add_space(4.0);

        // Built-in fonts
        for font in EditorFont::builtin_fonts() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut settings.font_family, font.clone(), font.display_name())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(font_description(font)).weak().small());
            });
        }

        // Custom font option
        let is_custom = settings.font_family.is_custom();
        let custom_label = t!("settings.editor.custom_font");
        ui.horizontal(|ui| {
            if ui.selectable_label(is_custom, custom_label.to_string()).clicked() && !is_custom {
                // Switch to custom with a default system font
                let system_fonts = fonts::list_system_fonts();
                let default_font = system_fonts.first()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Arial".to_string());
                settings.font_family = EditorFont::Custom(default_font);
                changed = true;
            }
            ui.label(RichText::new(t!("settings.editor.custom_font_desc")).weak().small());
        });

        // Show system font picker when Custom is selected
        if let EditorFont::Custom(current_font) = &settings.font_family {
            // Clone the current font name to avoid borrow conflicts
            let current_font_name = current_font.clone();
            let system_fonts = fonts::list_system_fonts();
            let font_found = system_fonts.iter().any(|f| f == &current_font_name);
            
            ui.add_space(4.0);
            ui.indent("custom_font_picker", |ui| {
                ui.label(RichText::new(t!("settings.editor.select_system_font")).small());
                ui.add_space(2.0);
                
                egui::ComboBox::from_id_source("system_font_combo")
                    .selected_text(&current_font_name)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(200.0);
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for font_name in system_fonts {
                                    if ui.selectable_label(font_name == &current_font_name, font_name).clicked() {
                                        settings.font_family = EditorFont::Custom(font_name.to_string());
                                        changed = true;
                                    }
                                }
                            });
                    });
                
                // Font preview
                ui.add_space(4.0);
                ui.label(RichText::new(t!("settings.editor.font_preview")).small());
                ui.label(
                    RichText::new("The quick brown fox jumps over the lazy dog. 0123456789")
                        .size(14.0),
                );
                if !font_found {
                    ui.label(
                        RichText::new(t!("settings.editor.font_not_found"))
                            .color(Color32::RED)
                            .small(),
                    );
                }
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // CJK Font Preference
        ui.label(RichText::new(t!("settings.editor.cjk_preference")).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new(t!("settings.editor.cjk_preference_hint"))
                .weak()
                .small(),
        );
        ui.add_space(4.0);

        egui::ComboBox::from_id_source("cjk_preference_combo")
            .selected_text(cjk_display_name(&settings.cjk_font_preference))
            .show_ui(ui, |ui| {
                for pref in CjkFontPreference::all() {
                    let label = format!("{} - {}", cjk_display_name(pref), cjk_description(pref));
                    if ui
                        .selectable_value(&mut settings.cjk_font_preference, *pref, label)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font size slider
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.font_size")).strong());
            ui.add_space(8.0);
            ui.label(format!("{}px", settings.font_size as u32));
        });
        ui.add_space(4.0);

        let font_slider = ui.add(
            egui::Slider::new(
                &mut settings.font_size,
                Settings::MIN_FONT_SIZE..=Settings::MAX_FONT_SIZE,
            )
            .show_value(false)
            .step_by(1.0),
        );
        if font_slider.changed() {
            changed = true;
        }

        // Font size presets
        ui.horizontal(|ui| {
            for (label, size) in [
                (t!("settings.font_size.small"), 12.0),
                (t!("settings.font_size.medium"), 14.0),
                (t!("settings.font_size.large"), 18.0),
            ] {
                if ui.small_button(label).clicked() {
                    settings.font_size = size;
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Default View Mode selection
        ui.label(RichText::new(t!("settings.preview.default_view")).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new(t!("settings.default_view_hint"))
                .weak()
                .small(),
        );
        ui.add_space(4.0);

        for view_mode in ViewMode::all() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(
                        &mut settings.default_view_mode,
                        *view_mode,
                        format!("{} {}", view_mode.icon(), view_mode.label()),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(view_mode_description(view_mode)).weak().small());
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Language selection
        ui.label(RichText::new(t!("settings.appearance.language")).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new(t!("settings.appearance.language_hint"))
                .weak()
                .small(),
        );
        ui.add_space(4.0);

        let current_lang = settings.language;
        egui::ComboBox::from_id_source("language_combo")
            .selected_text(format!("🌐 {}", current_lang.native_name()))
            .show_ui(ui, |ui| {
                for lang in Language::all() {
                    if ui
                        .selectable_value(&mut settings.language, *lang, lang.native_name())
                        .changed()
                    {
                        // Apply language change immediately
                        set_locale(settings.language.locale_code());
                        changed = true;
                    }
                }
            });

        changed
    }

    /// Show the Editor settings section with two-column layout for toggles.
    ///
    /// Returns true if any setting was changed.
    fn show_editor_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        ui.heading(t!("settings.editor.title"));
        ui.add_space(8.0);

        // Two-column grid for basic toggles using egui::Grid for proper alignment
        egui::Grid::new("editor_toggles_grid")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .min_col_width(180.0)
            .show(ui, |ui| {
                // Row 1: Word Wrap | Show Line Numbers
                if ui
                    .checkbox(&mut settings.word_wrap, t!("settings.editor.word_wrap"))
                    .on_hover_text(t!("settings.editor.word_wrap_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut settings.show_line_numbers, t!("settings.editor.show_line_numbers"))
                    .on_hover_text(t!("settings.editor.line_numbers_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                // Row 2: Show Minimap | Sync Scroll
                if ui
                    .checkbox(&mut settings.minimap_enabled, t!("settings.editor.show_minimap"))
                    .on_hover_text(t!("settings.editor.minimap_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut settings.sync_scroll_enabled, t!("settings.preview.sync_scroll"))
                    .on_hover_text(t!("settings.editor.sync_scroll_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                // Row 3: Highlight Brackets | Auto-close Brackets
                if ui
                    .checkbox(&mut settings.highlight_matching_pairs, t!("settings.editor.highlight_brackets"))
                    .on_hover_text(t!("settings.editor.brackets_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut settings.auto_close_brackets, t!("settings.editor.auto_close_brackets"))
                    .on_hover_text(t!("settings.editor.auto_close_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                // Row 4: Syntax Highlighting | Use Spaces
                if ui
                    .checkbox(&mut settings.syntax_highlighting_enabled, t!("settings.editor.syntax_highlighting"))
                    .on_hover_text(t!("settings.editor.syntax_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut settings.use_spaces, t!("settings.editor.use_spaces"))
                    .on_hover_text(t!("settings.editor.use_spaces_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });

        // Minimap mode selector (only show if minimap is enabled) - full width
        if settings.minimap_enabled {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t!("settings.editor.minimap_mode")).small());
                ui.add_space(8.0);
                for mode in MinimapMode::all() {
                    let label = match mode {
                        MinimapMode::Auto => t!("settings.editor.minimap_mode_auto").to_string(),
                        MinimapMode::Semantic => t!("settings.editor.minimap_mode_semantic").to_string(),
                        MinimapMode::Pixel => t!("settings.editor.minimap_mode_pixel").to_string(),
                    };
                    let desc = match mode {
                        MinimapMode::Auto => t!("settings.editor.minimap_mode_auto_desc").to_string(),
                        MinimapMode::Semantic => t!("settings.editor.minimap_mode_semantic_desc").to_string(),
                        MinimapMode::Pixel => t!("settings.editor.minimap_mode_pixel_desc").to_string(),
                    };
                    if ui
                        .selectable_value(&mut settings.minimap_mode, *mode, &label)
                        .on_hover_text(&desc)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Tab size - compact horizontal layout
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.tab_size")).strong());
            ui.add_space(8.0);

            let mut tab_size_f32 = settings.tab_size as f32;
            let tab_slider = ui.add(
                egui::Slider::new(
                    &mut tab_size_f32,
                    Settings::MIN_TAB_SIZE as f32..=Settings::MAX_TAB_SIZE as f32,
                )
                .show_value(true)
                .suffix(format!(" {}", t!("settings.editor.spaces")))
                .step_by(1.0),
            );
            if tab_slider.changed() {
                settings.tab_size = tab_size_f32 as u8;
                changed = true;
            }

            ui.add_space(8.0);
            for size in [2u8, 4, 8] {
                if ui.small_button(format!("{}", size)).clicked() {
                    settings.tab_size = size;
                    changed = true;
                }
            }
        });

        ui.add_space(8.0);

        // Maximum Line Width - compact layout
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.max_line_width")).strong());
            ui.add_space(8.0);

            let current_display = settings.max_line_width.display_name();
            egui::ComboBox::from_id_source("max_line_width_combo")
                .selected_text(current_display)
                .width(140.0)
                .show_ui(ui, |ui| {
                    for preset in MaxLineWidth::presets() {
                        let label = format!("{} - {}", preset.display_name(), preset.description());
                        if ui
                            .selectable_value(&mut settings.max_line_width, *preset, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                    let is_custom = settings.max_line_width.is_custom();
                    let custom_label = t!("settings.editor.custom_width");
                    if ui.selectable_label(is_custom, custom_label.to_string()).clicked() && !is_custom {
                        settings.max_line_width = MaxLineWidth::Custom(800);
                        changed = true;
                    }
                });

            // Show inline numeric input when custom is selected
            if let MaxLineWidth::Custom(px) = &mut settings.max_line_width {
                let mut px_value = *px as f32;
                let drag = ui.add(
                    egui::DragValue::new(&mut px_value)
                        .speed(10.0)
                        .range(Settings::MIN_CUSTOM_LINE_WIDTH as f32..=Settings::MAX_CUSTOM_LINE_WIDTH as f32)
                        .suffix("px"),
                );
                if drag.changed() {
                    *px = px_value as u32;
                    changed = true;
                }
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Code Folding section - compact two-column when enabled
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.code_folding")).strong());
            ui.add_space(8.0);
            if ui
                .checkbox(&mut settings.folding_enabled, t!("settings.editor.enable_folding"))
                .on_hover_text(t!("settings.editor.folding_tooltip"))
                .changed()
            {
                changed = true;
            }
        });

        if settings.folding_enabled {
            ui.add_space(4.0);

            // Fold options in a grid
            ui.indent("fold_options", |ui| {
                egui::Grid::new("fold_options_grid")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .min_col_width(180.0)
                    .show(ui, |ui| {
                        // Row 1: Show indicators | Headings
                        if ui
                            .checkbox(&mut settings.folding_show_indicators, t!("settings.editor.show_fold_indicators"))
                            .on_hover_text(t!("settings.editor.fold_indicators_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .checkbox(&mut settings.fold_headings, t!("settings.editor.fold_headings"))
                            .on_hover_text(t!("settings.editor.fold_headings_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 2: Code Blocks | Lists
                        if ui
                            .checkbox(&mut settings.fold_code_blocks, t!("settings.editor.fold_code_blocks"))
                            .on_hover_text(t!("settings.editor.fold_code_blocks_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .checkbox(&mut settings.fold_lists, t!("settings.editor.fold_lists"))
                            .on_hover_text(t!("settings.editor.fold_lists_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 3: Indentation (single item)
                        if ui
                            .checkbox(&mut settings.fold_indentation, t!("settings.editor.fold_indentation"))
                            .on_hover_text(t!("settings.editor.fold_indentation_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();
                    });
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Snippets section - compact
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.snippets")).strong());
            ui.add_space(8.0);
            if ui
                .checkbox(&mut settings.snippets_enabled, t!("settings.editor.enable_snippets"))
                .on_hover_text(t!("settings.editor.snippets_tooltip"))
                .changed()
            {
                changed = true;
            }
        });

        if settings.snippets_enabled {
            ui.add_space(4.0);
            ui.indent("snippets_info", |ui| {
                ui.label(RichText::new(t!("settings.editor.builtin_snippets")).small());
                // Two-column snippet display
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t!("settings.editor.snippet_date")).code().small());
                    ui.add_space(16.0);
                    ui.label(RichText::new(t!("settings.editor.snippet_time")).code().small());
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t!("settings.editor.snippet_datetime")).code().small());
                    ui.add_space(16.0);
                    ui.label(RichText::new(t!("settings.editor.snippet_now")).code().small());
                });
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // CJK Paragraph Indentation - compact
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.paragraph_indent")).strong());
            ui.add_space(8.0);

            use crate::config::ParagraphIndent;
            let current_display = settings.paragraph_indent.display_name();
            egui::ComboBox::from_id_source("paragraph_indent_combo")
                .selected_text(current_display)
                .width(100.0)
                .show_ui(ui, |ui| {
                    for preset in ParagraphIndent::presets() {
                        let label = format!("{} - {}", preset.display_name(), preset.description());
                        if ui
                            .selectable_value(&mut settings.paragraph_indent, *preset, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                    let is_custom = settings.paragraph_indent.is_custom();
                    let custom_label = t!("settings.editor.paragraph_indent_custom");
                    if ui.selectable_label(is_custom, format!("{} - {}", custom_label, t!("settings.editor.paragraph_indent_custom_desc"))).clicked() && !is_custom {
                        settings.paragraph_indent = ParagraphIndent::Custom(20);
                        changed = true;
                    }
                });

            // Show inline numeric input when custom is selected
            if let ParagraphIndent::Custom(tenths) = &mut settings.paragraph_indent {
                let mut em_value = *tenths as f32 / 10.0;
                let drag = ui.add(
                    egui::DragValue::new(&mut em_value)
                        .speed(0.1)
                        .range(0.5..=5.0)
                        .suffix("em"),
                );
                if drag.changed() {
                    *tenths = (em_value * 10.0).round() as u8;
                    changed = true;
                }
            }
        });

        // Hint text for paragraph indent
        ui.label(
            RichText::new(t!("settings.editor.paragraph_indent_hint"))
                .weak()
                .small(),
        );

        changed
    }

    /// Show the Files settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_files_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        ui.heading(t!("settings.files.title"));
        ui.add_space(8.0);

        // Session restore toggle
        if ui
            .checkbox(&mut settings.restore_session, t!("settings.general.restore_session"))
            .on_hover_text(t!("settings.files.restore_session_tooltip"))
            .changed()
        {
            changed = true;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Auto-save toggle (default for new documents)
        if ui
            .checkbox(&mut settings.auto_save_enabled_default, t!("settings.files.enable_auto_save"))
            .on_hover_text(t!("settings.files.auto_save_tooltip"))
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Auto-save delay
        ui.horizontal(|ui| {
            ui.label(t!("settings.files.auto_save_delay"));
            ui.add_space(8.0);
            let secs = settings.auto_save_delay_ms / 1000;
            ui.label(t!("settings.files.seconds", count = secs));
        });
        ui.add_space(4.0);

        // Convert ms to seconds for slider display
        let mut delay_secs = (settings.auto_save_delay_ms / 1000) as f32;
        let delay_slider = ui.add(
            egui::Slider::new(&mut delay_secs, 5.0..=300.0)
                .show_value(false)
                .step_by(5.0),
        );
        if delay_slider.changed() {
            settings.auto_save_delay_ms = (delay_secs as u32) * 1000;
            changed = true;
        }

        // Delay presets
        ui.horizontal(|ui| {
            for (label, ms) in [("15s", 15000), ("30s", 30000), ("1m", 60000)] {
                if ui.small_button(label).clicked() {
                    settings.auto_save_delay_ms = ms;
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Recent files count
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.files.recent_files")).strong());
            ui.add_space(8.0);
            ui.label(t!("settings.files.remember_files", count = settings.max_recent_files));
        });
        ui.add_space(4.0);

        let mut recent_count_f32 = settings.max_recent_files as f32;
        let recent_slider = ui.add(
            egui::Slider::new(&mut recent_count_f32, 0.0..=20.0)
                .show_value(false)
                .step_by(1.0),
        );
        if recent_slider.changed() {
            settings.max_recent_files = recent_count_f32 as usize;
            changed = true;
        }

        ui.add_space(8.0);

        // Clear recent files button
        ui.horizontal(|ui| {
            if ui
                .button(t!("settings.files.clear_recent"))
                .on_hover_text(t!("settings.files.clear_recent_tooltip"))
                .clicked()
            {
                settings.recent_files.clear();
                changed = true;
            }

            if !settings.recent_files.is_empty() {
                ui.label(
                    RichText::new(t!("settings.files.files_count", count = settings.recent_files.len()))
                        .small()
                        .weak(),
                );
            }
        });

        changed
    }

    /// Show the Keyboard shortcuts settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_keyboard_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        ui.heading(t!("settings.keyboard.title"));
        ui.add_space(4.0);

        // Search/filter box
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.keyboard_filter)
                    .hint_text(t!("settings.keyboard.search_hint"))
                    .desired_width(200.0),
            );
            if !self.keyboard_filter.is_empty() {
                if ui.small_button("✕").clicked() {
                    self.keyboard_filter.clear();
                }
            }
        });

        ui.add_space(4.0);

        // Reset all button
        ui.horizontal(|ui| {
            if ui
                .button(format!("↺ {}", t!("settings.keyboard.reset_all")))
                .on_hover_text(t!("settings.keyboard.reset_all_tooltip"))
                .clicked()
            {
                settings.keyboard_shortcuts.reset_all();
                self.conflict_warning = None;
                changed = true;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Show conflict warning if any
        if let Some((cmd, msg)) = &self.conflict_warning {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").color(Color32::YELLOW));
                ui.label(RichText::new(format!("{}: {}", shortcut_command_name(cmd), msg)).color(Color32::YELLOW));
            });
            ui.add_space(4.0);
        }

        // Key capture modal - clone state for use in closure
        let mut cancel_capture = false;
        let mut apply_capture: Option<(ShortcutCommand, KeyBinding)> = None;

        if let Some(capture) = &self.key_capture {
            let cmd_name = shortcut_command_name(&capture.command);
            let current_mods = capture.modifiers.display_string();
            let current_key = capture.key.map(|k| k.display_string()).unwrap_or("");
            let has_key = capture.key.is_some();
            let capture_cmd = capture.command;
            let capture_mods = capture.modifiers;
            let capture_key = capture.key;

            let display = if current_mods.is_empty() && current_key.is_empty() {
                t!("settings.keyboard.waiting").to_string()
            } else if current_key.is_empty() {
                format!("{}+...", current_mods)
            } else if current_mods.is_empty() {
                current_key.to_string()
            } else {
                format!("{}+{}", current_mods, current_key)
            };

            ui.group(|ui| {
                ui.label(RichText::new(format!("{} \"{}\"...", t!("settings.keyboard.press_key"), cmd_name)).strong());
                ui.add_space(4.0);

                ui.label(RichText::new(&display).monospace().size(16.0));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("settings.keyboard.cancel")).clicked() {
                        cancel_capture = true;
                    }
                    if has_key {
                        if ui.button(t!("settings.keyboard.apply")).clicked() {
                            if let Some(key) = capture_key {
                                apply_capture = Some((capture_cmd, KeyBinding::new(capture_mods, key)));
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }

        // Handle deferred capture actions
        if cancel_capture {
            self.key_capture = None;
        }
        if let Some((cmd, binding)) = apply_capture {
            // Check for conflicts
            if let Some(conflict_cmd) = settings.keyboard_shortcuts.find_conflict(&binding, Some(cmd)) {
                self.conflict_warning = Some((
                    cmd,
                    format!("{} \"{}\"", t!("settings.keyboard.conflict_with"), shortcut_command_name(&conflict_cmd)),
                ));
            } else {
                settings.keyboard_shortcuts.set(cmd, binding);
                self.conflict_warning = None;
                changed = true;
            }
            self.key_capture = None;
        }

        // Capture keyboard input when in capture mode
        let mut escape_pressed = false;
        let mut new_modifiers: Option<KeyModifiers> = None;
        let mut new_key: Option<KeyCode> = None;

        // Check if we already have a key captured (to latch modifiers)
        let key_already_captured = self
            .key_capture
            .as_ref()
            .map(|c| c.key.is_some())
            .unwrap_or(false);

        if self.key_capture.is_some() {
            ui.input(|i| {
                // Only update modifiers if no key captured yet (latch them once key is pressed)
                if !key_already_captured {
                    new_modifiers = Some(KeyModifiers::from_egui(&i.modifiers));
                }

                // Check for key press
                for event in &i.events {
                    if let egui::Event::Key { key, pressed: true, .. } = event {
                        // Skip modifier-only keys
                        if matches!(key, egui::Key::Escape) {
                            escape_pressed = true;
                            return;
                        }
                        if let Some(key_code) = KeyCode::from_egui(*key) {
                            new_key = Some(key_code);
                            // Capture modifiers at the moment the key is pressed
                            new_modifiers = Some(KeyModifiers::from_egui(&i.modifiers));
                        }
                    }
                }
            });
        }

        // Apply captured input
        if escape_pressed {
            self.key_capture = None;
        } else if let Some(capture) = &mut self.key_capture {
            // Only update modifiers if no key captured yet, or if capturing new key with its modifiers
            if capture.key.is_none() {
                if let Some(mods) = new_modifiers {
                    capture.modifiers = mods;
                }
            }
            if let Some(key) = new_key {
                capture.key = Some(key);
                // Also latch the modifiers that came with the key press
                if let Some(mods) = new_modifiers {
                    capture.modifiers = mods;
                }
            }
        }

        // Scrollable area for shortcuts list
        let filter_lower = self.keyboard_filter.to_lowercase();

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for (category, commands) in KeyboardShortcuts::commands_by_category() {
                    // Filter commands by search term
                    let filtered_commands: Vec<_> = commands
                        .iter()
                        .filter(|cmd| {
                            if filter_lower.is_empty() {
                                return true;
                            }
                            shortcut_command_name(cmd).to_lowercase().contains(&filter_lower)
                                || category.to_lowercase().contains(&filter_lower)
                        })
                        .collect();

                    if filtered_commands.is_empty() {
                        continue;
                    }

                    // Category header
                    ui.add_space(4.0);
                    ui.label(RichText::new(category).strong().size(13.0));
                    ui.add_space(2.0);

                    for &cmd in &filtered_commands {
                        let binding = settings.keyboard_shortcuts.get(*cmd);
                        let is_custom = settings.keyboard_shortcuts.is_custom(*cmd);
                        let is_capturing = self.key_capture.as_ref().map(|c| c.command == *cmd).unwrap_or(false);

                        ui.horizontal(|ui| {
                            // Command name
                            let cmd_name = shortcut_command_name(cmd);
                            let name_text = if is_custom {
                                RichText::new(&cmd_name).italics()
                            } else {
                                RichText::new(&cmd_name)
                            };
                            ui.label(name_text);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Reset button (only if custom)
                                if is_custom {
                                    if ui.small_button("↺").on_hover_text(t!("settings.keyboard.reset_default")).clicked() {
                                        settings.keyboard_shortcuts.reset(*cmd);
                                        changed = true;
                                    }
                                }

                                // Binding button
                                let btn_text = if is_capturing {
                                    "...".to_string()
                                } else {
                                    binding.display_string()
                                };
                                let btn = ui.add(
                                    egui::Button::new(RichText::new(&btn_text).monospace())
                                        .min_size(egui::vec2(100.0, 0.0)),
                                );
                                if btn.clicked() && self.key_capture.is_none() {
                                    self.key_capture = Some(KeyCaptureState {
                                        command: *cmd,
                                        modifiers: KeyModifiers::none(),
                                        key: None,
                                    });
                                    self.conflict_warning = None;
                                }
                                if btn.hovered() && !is_capturing {
                                    btn.on_hover_text(t!("settings.keyboard.click_to_change"));
                                }
                            });
                        });
                    }
                }
            });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_panel_new() {
        let panel = SettingsPanel::new();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_default() {
        let panel = SettingsPanel::default();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_section_label() {
        assert_eq!(SettingsSection::Appearance.label(), "Appearance");
        assert_eq!(SettingsSection::Editor.label(), "Editor");
        assert_eq!(SettingsSection::Files.label(), "Files");
    }

    #[test]
    fn test_settings_section_icon() {
        assert_eq!(SettingsSection::Appearance.icon(), "🎨");
        assert_eq!(SettingsSection::Editor.icon(), "📝");
        assert_eq!(SettingsSection::Files.icon(), "📁");
    }

    #[test]
    fn test_settings_section_default() {
        let section = SettingsSection::default();
        assert_eq!(section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_output_default() {
        let output = SettingsPanelOutput::default();
        assert!(!output.changed);
        assert!(!output.close_requested);
        assert!(!output.reset_requested);
    }
}
