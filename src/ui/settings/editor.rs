//! Editor setting render functions (toggles, indentation, folding, snippets,
//! code execution, LSP).

use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;

use crate::config::{HeaderSpacing, MaxLineWidth, MinimapMode, Settings};

use super::registry::SettingsCtx;
use super::SettingsPanel;

/// Word wrap toggle.
pub(super) fn render_word_wrap(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(&mut ctx.settings.word_wrap, t!("settings.editor.word_wrap"))
        .on_hover_text(t!("settings.editor.word_wrap_tooltip"))
        .changed()
}

/// Line numbers toggle.
pub(super) fn render_line_numbers(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.show_line_numbers,
        t!("settings.editor.show_line_numbers"),
    )
    .on_hover_text(t!("settings.editor.line_numbers_tooltip"))
    .changed()
}

/// Minimap toggle plus mode selector when enabled.
pub(super) fn render_minimap(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    if ui
        .checkbox(
            &mut settings.minimap_enabled,
            t!("settings.editor.show_minimap"),
        )
        .on_hover_text(t!("settings.editor.minimap_tooltip"))
        .changed()
    {
        changed = true;
    }

    if settings.minimap_enabled {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.minimap_mode")).small());
            ui.add_space(8.0);
            for mode in MinimapMode::all() {
                let label = match mode {
                    MinimapMode::Auto => t!("settings.editor.minimap_mode_auto").to_string(),
                    MinimapMode::Semantic => {
                        t!("settings.editor.minimap_mode_semantic").to_string()
                    }
                    MinimapMode::Pixel => t!("settings.editor.minimap_mode_pixel").to_string(),
                };
                let desc = match mode {
                    MinimapMode::Auto => t!("settings.editor.minimap_mode_auto_desc").to_string(),
                    MinimapMode::Semantic => {
                        t!("settings.editor.minimap_mode_semantic_desc").to_string()
                    }
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

    changed
}

/// Matching bracket highlighting toggle.
pub(super) fn render_highlight_brackets(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.highlight_matching_pairs,
        t!("settings.editor.highlight_brackets"),
    )
    .on_hover_text(t!("settings.editor.brackets_tooltip"))
    .changed()
}

/// Auto-close brackets toggle.
pub(super) fn render_auto_close_brackets(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.auto_close_brackets,
        t!("settings.editor.auto_close_brackets"),
    )
    .on_hover_text(t!("settings.editor.auto_close_tooltip"))
    .changed()
}

/// Syntax highlighting toggle plus default language combo when enabled.
pub(super) fn render_syntax_highlighting(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    if ui
        .checkbox(
            &mut settings.syntax_highlighting_enabled,
            t!("settings.editor.syntax_highlighting"),
        )
        .on_hover_text(t!("settings.editor.syntax_tooltip"))
        .changed()
    {
        changed = true;
    }

    // Default language for syntax highlighting (only when enabled)
    if settings.syntax_highlighting_enabled {
        ui.add_space(8.0);
        ui.label(RichText::new(t!("settings.editor.default_language")).strong());
        ui.label(
            RichText::new(t!("settings.editor.default_language_hint"))
                .weak()
                .small(),
        );
        ui.add_space(4.0);

        let languages = crate::markdown::syntax::get_available_languages();
        let current_display = if settings.default_syntax_language.is_empty() {
            t!("settings.editor.default_language_auto").to_string()
        } else {
            languages
                .iter()
                .find(|(id, _)| id == &settings.default_syntax_language)
                .map(|(_, display)| display.clone())
                .unwrap_or_else(|| settings.default_syntax_language.clone())
        };

        egui::ComboBox::from_id_salt("default_language_combo")
            .selected_text(&current_display)
            .width(200.0)
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        // Auto option (empty string = no default)
                        if ui
                            .selectable_label(
                                settings.default_syntax_language.is_empty(),
                                t!("settings.editor.default_language_auto"),
                            )
                            .clicked()
                        {
                            settings.default_syntax_language = String::new();
                            changed = true;
                        }

                        ui.separator();

                        for (lang_id, display_name) in &languages {
                            if ui
                                .selectable_label(
                                    &settings.default_syntax_language == lang_id,
                                    display_name,
                                )
                                .clicked()
                            {
                                settings.default_syntax_language = lang_id.clone();
                                changed = true;
                            }
                        }
                    });
            });
    }

    changed
}

/// Use spaces instead of tabs toggle.
pub(super) fn render_use_spaces(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.use_spaces,
        t!("settings.editor.use_spaces"),
    )
    .on_hover_text(t!("settings.editor.use_spaces_tooltip"))
    .changed()
}

/// Vim mode toggle.
pub(super) fn render_vim_mode(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(&mut ctx.settings.vim_mode, t!("settings.editor.vim_mode"))
        .on_hover_text(t!("settings.editor.vim_mode_tooltip"))
        .changed()
}

/// Linux middle-click primary-selection paste toggle.
#[cfg(target_os = "linux")]
pub(super) fn render_middle_click_paste(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(
        &mut ctx.settings.middle_click_paste,
        t!("settings.editor.middle_click_paste"),
    )
    .on_hover_text(t!("settings.editor.middle_click_paste_tooltip"))
    .changed()
}

/// Strict line breaks toggle.
pub(super) fn render_strict_line_breaks(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    ui.checkbox(&mut ctx.settings.strict_line_breaks, "Strict Line Breaks")
        .on_hover_text("Treat single newlines as hard line breaks in rendered view")
        .changed()
}

/// LSP master toggle plus per-server binary overrides.
pub(super) fn render_lsp(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let workspace_root = ctx.workspace_root;
    let settings = &mut *ctx.settings;
    let mut changed = false;

    if ui
        .checkbox(&mut settings.lsp_enabled, "LSP (Language Servers)")
        .on_hover_text(
            "Auto-detect and start language servers for code intelligence (requires servers on PATH)",
        )
        .changed()
    {
        changed = true;
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Language servers").strong());
    ui.label(
        RichText::new(
            "Optional path to the server binary per detected server. Leave empty to use PATH.",
        )
        .weak()
        .small(),
    );
    ui.add_space(6.0);

    let mut server_keys: Vec<String> = workspace_root
        .map(|root| {
            crate::lsp::detect_servers_for_workspace(root)
                .into_iter()
                .map(|(k, _)| k)
                .collect()
        })
        .unwrap_or_default();
    for k in settings.lsp_server_overrides.keys() {
        if !server_keys.contains(k) {
            server_keys.push(k.clone());
        }
    }
    server_keys.sort();
    server_keys.dedup();

    if workspace_root.is_none() {
        ui.label(
            RichText::new("Open a folder workspace to detect language servers for this project.")
                .italics()
                .weak(),
        );
    } else if server_keys.is_empty() {
        ui.label(
            RichText::new(
                "No language servers detected for this workspace (add code files or set overrides below).",
            )
            .italics()
            .weak(),
        );
    }

    for key in &server_keys {
        let key = key.clone();
        ui.horizontal(|ui| {
            ui.label(format!("{}:", key));
            let mut path = settings
                .lsp_server_overrides
                .get(&key)
                .cloned()
                .unwrap_or_default();
            let desired = (ui.available_width() - 72.0).clamp(120.0, 420.0);
            let te = egui::TextEdit::singleline(&mut path)
                .desired_width(desired)
                .hint_text("default from PATH");
            if ui.add(te).changed() {
                if path.trim().is_empty() {
                    settings.lsp_server_overrides.remove(&key);
                } else {
                    settings.lsp_server_overrides.insert(key, path);
                }
                changed = true;
            }
        });
    }

    changed
}

/// Markdown code execution group (master toggle, per-language allows,
/// timeout, inline output).
pub(super) fn render_code_execution(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.label(RichText::new(t!("settings.editor.code_execution_heading")).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t!("settings.editor.code_execution_warning"))
            .small()
            .color(ui.visuals().warn_fg_color),
    );
    ui.add_space(8.0);

    let prev_master = settings.enable_code_execution;
    if ui
        .checkbox(
            &mut settings.enable_code_execution,
            t!("settings.editor.code_execution_enable"),
        )
        .on_hover_text(t!("settings.editor.code_execution_enable_tooltip"))
        .changed()
    {
        changed = true;
        if settings.enable_code_execution && !prev_master {
            settings.allow_shell = true;
            settings.allow_python = true;
            settings.code_execution_consent_acknowledged = true;
        }
    }

    ui.add_space(4.0);
    let exec_on = settings.enable_code_execution;

    ui.horizontal(|ui| {
        ui.add_enabled_ui(exec_on, |ui| {
            if ui
                .checkbox(
                    &mut settings.allow_shell,
                    t!("settings.editor.code_execution_allow_shell"),
                )
                .on_hover_text(t!("settings.editor.code_execution_allow_shell_tooltip"))
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(
                    &mut settings.allow_python,
                    t!("settings.editor.code_execution_allow_python"),
                )
                .on_hover_text(t!("settings.editor.code_execution_allow_python_tooltip"))
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_enabled_ui(exec_on, |ui| {
            ui.label(format!(
                "{}: ",
                t!("settings.editor.code_execution_timeout")
            ));
            if ui
                .add(
                    egui::Slider::new(
                        &mut settings.code_execution_timeout_secs,
                        Settings::MIN_CODE_EXECUTION_TIMEOUT_SECS
                            ..=Settings::MAX_CODE_EXECUTION_TIMEOUT_SECS,
                    )
                    .suffix(format!(
                        " {}",
                        t!("settings.editor.code_execution_seconds_suffix")
                    )),
                )
                .on_hover_text(t!("settings.editor.code_execution_timeout_tooltip"))
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(6.0);
    ui.add_enabled_ui(exec_on, |ui| {
        if ui
            .checkbox(
                &mut settings.code_execution_show_inline_output,
                t!("settings.editor.code_execution_show_inline_output"),
            )
            .on_hover_text(t!(
                "settings.editor.code_execution_show_inline_output_tooltip"
            ))
            .changed()
        {
            changed = true;
        }
    });

    changed
}

/// Tab size slider with presets.
pub(super) fn render_tab_size(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

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

    changed
}

/// Maximum line width combo with custom pixel value.
pub(super) fn render_max_line_width(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.max_line_width")).strong());
        ui.add_space(8.0);

        let current_display = settings.max_line_width.display_name();
        egui::ComboBox::from_id_salt("max_line_width_combo")
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
                if ui
                    .selectable_label(is_custom, custom_label.to_string())
                    .clicked()
                    && !is_custom
                {
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
                    .range(
                        Settings::MIN_CUSTOM_LINE_WIDTH as f32
                            ..=Settings::MAX_CUSTOM_LINE_WIDTH as f32,
                    )
                    .suffix("px"),
            );
            if drag.changed() {
                *px = px_value as u32;
                changed = true;
            }
        }
    });

    changed
}

/// Code folding master toggle plus per-kind options.
pub(super) fn render_code_folding(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.code_folding")).strong());
        ui.add_space(8.0);
        if ui
            .checkbox(
                &mut settings.folding_enabled,
                t!("settings.editor.enable_folding"),
            )
            .on_hover_text(t!("settings.editor.folding_tooltip"))
            .changed()
        {
            changed = true;
        }
    });

    if settings.folding_enabled {
        ui.add_space(4.0);
        ui.indent("fold_options", |ui| {
            egui::Grid::new("fold_options_grid")
                .num_columns(2)
                .spacing([24.0, 6.0])
                .min_col_width(180.0)
                .show(ui, |ui| {
                    if ui
                        .checkbox(
                            &mut settings.folding_show_indicators,
                            t!("settings.editor.show_fold_indicators"),
                        )
                        .on_hover_text(t!("settings.editor.fold_indicators_tooltip"))
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .checkbox(
                            &mut settings.fold_headings,
                            t!("settings.editor.fold_headings"),
                        )
                        .on_hover_text(t!("settings.editor.fold_headings_tooltip"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    if ui
                        .checkbox(
                            &mut settings.fold_code_blocks,
                            t!("settings.editor.fold_code_blocks"),
                        )
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

                    if ui
                        .checkbox(
                            &mut settings.fold_indentation,
                            t!("settings.editor.fold_indentation"),
                        )
                        .on_hover_text(t!("settings.editor.fold_indentation_tooltip"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();
                });
        });
    }

    changed
}

/// Snippets toggle with built-in snippet reference.
pub(super) fn render_snippets(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.snippets")).strong());
        ui.add_space(8.0);
        if ui
            .checkbox(
                &mut settings.snippets_enabled,
                t!("settings.editor.enable_snippets"),
            )
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
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t!("settings.editor.snippet_date"))
                        .code()
                        .small(),
                );
                ui.add_space(16.0);
                ui.label(
                    RichText::new(t!("settings.editor.snippet_time"))
                        .code()
                        .small(),
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t!("settings.editor.snippet_datetime"))
                        .code()
                        .small(),
                );
                ui.add_space(16.0);
                ui.label(
                    RichText::new(t!("settings.editor.snippet_now"))
                        .code()
                        .small(),
                );
            });
        });
    }

    changed
}

/// CJK paragraph indentation combo with custom em value.
pub(super) fn render_paragraph_indent(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.paragraph_indent")).strong());
        ui.add_space(8.0);

        use crate::config::ParagraphIndent;
        let current_display = settings.paragraph_indent.display_name();
        egui::ComboBox::from_id_salt("paragraph_indent_combo")
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
                if ui
                    .selectable_label(
                        is_custom,
                        format!(
                            "{} - {}",
                            custom_label,
                            t!("settings.editor.paragraph_indent_custom_desc")
                        ),
                    )
                    .clicked()
                    && !is_custom
                {
                    settings.paragraph_indent = ParagraphIndent::Custom(20);
                    changed = true;
                }
            });

        // Show inline numeric input when custom is selected
        if let crate::config::ParagraphIndent::Custom(tenths) = &mut settings.paragraph_indent {
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

    ui.label(
        RichText::new(t!("settings.editor.paragraph_indent_hint"))
            .weak()
            .small(),
    );

    changed
}

/// Header spacing (markdown rendering) combo.
pub(super) fn render_header_spacing(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("settings.editor.header_spacing")).strong());
        ui.add_space(8.0);

        let current_display = settings.header_spacing.display_name();
        egui::ComboBox::from_id_salt("header_spacing_combo")
            .selected_text(current_display)
            .width(100.0)
            .show_ui(ui, |ui| {
                for preset in HeaderSpacing::presets() {
                    let label = format!("{} - {}", preset.display_name(), preset.description());
                    if ui
                        .selectable_value(&mut settings.header_spacing, *preset, label)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
    });

    ui.label(
        RichText::new(t!("settings.editor.header_spacing_hint"))
            .weak()
            .small(),
    );

    changed
}

/// Spellcheck master toggle, language, extra dictionary folder, ignore flags.
#[cfg(feature = "spellcheck")]
pub(super) fn render_spellcheck(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;

    if ui
        .checkbox(
            &mut settings.spellcheck_enabled,
            t!("settings.editor.spellcheck.enabled"),
        )
        .on_hover_text(t!("settings.editor.spellcheck.enabled_tooltip"))
        .changed()
    {
        changed = true;
    }

    ui.add_space(6.0);
    ui.add_enabled_ui(settings.spellcheck_enabled, |ui| {
        ui.horizontal(|ui| {
            ui.label(t!("settings.editor.spellcheck.language").to_string());
            let mut langs = crate::spellcheck::discover_dictionary_langs(
                settings.spellcheck_dictionary_dir.as_deref(),
            );
            if !settings.spellcheck_language.is_empty()
                && !langs.iter().any(|l| l == &settings.spellcheck_language)
            {
                langs.push(settings.spellcheck_language.clone());
            }
            let current = settings.spellcheck_language.clone();
            egui::ComboBox::from_id_salt("spellcheck_language_combo")
                .selected_text(&current)
                .width(160.0)
                .show_ui(ui, |ui| {
                    for lang in &langs {
                        if ui
                            .selectable_label(settings.spellcheck_language == *lang, lang)
                            .clicked()
                        {
                            settings.spellcheck_language = lang.clone();
                            changed = true;
                        }
                    }
                });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(t!("settings.editor.spellcheck.dictionary_dir").to_string());
            let mut dir_text = settings
                .spellcheck_dictionary_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let desired = (ui.available_width() - 160.0).clamp(120.0, 360.0);
            if ui
                .add(
                    egui::TextEdit::singleline(&mut dir_text)
                        .desired_width(desired)
                        .hint_text(""),
                )
                .changed()
            {
                settings.spellcheck_dictionary_dir = if dir_text.trim().is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(dir_text.trim()))
                };
                changed = true;
            }
            if ui
                .button(t!("settings.editor.spellcheck.browse").to_string())
                .clicked()
            {
                let initial = settings.spellcheck_dictionary_dir.clone();
                if let crate::files::dialogs::DialogResult::Success(path) =
                    crate::files::dialogs::open_folder_dialog(initial.as_ref())
                {
                    settings.spellcheck_dictionary_dir = Some(path);
                    changed = true;
                }
            }
            if settings.spellcheck_dictionary_dir.is_some()
                && ui
                    .button(t!("settings.editor.spellcheck.clear_dir").to_string())
                    .clicked()
            {
                settings.spellcheck_dictionary_dir = None;
                changed = true;
            }
        });

        ui.add_space(4.0);
        if ui
            .checkbox(
                &mut settings.spellcheck_ignore_all_caps,
                t!("settings.editor.spellcheck.ignore_all_caps"),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut settings.spellcheck_ignore_words_with_digits,
                t!("settings.editor.spellcheck.ignore_words_with_digits"),
            )
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);
        let user_path = crate::config::get_config_dir()
            .map(|d| crate::spellcheck::user_words_path(&d))
            .unwrap_or_else(|_| {
                std::env::temp_dir()
                    .join("spellcheck")
                    .join("user-words.txt")
            });
        let word_count = crate::spellcheck::load_user_words(&user_path).len();
        ui.horizontal(|ui| {
            ui.label(
                t!(
                    "settings.editor.spellcheck.personal_dictionary",
                    count = word_count
                )
                .to_string(),
            );
            if ui
                .button(t!("settings.editor.spellcheck.open_file").to_string())
                .clicked()
            {
                if let Some(parent) = user_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if !user_path.exists() {
                    let _ = std::fs::write(&user_path, "");
                }
                let _ = open::that(&user_path);
            }
        });
    });

    changed
}
