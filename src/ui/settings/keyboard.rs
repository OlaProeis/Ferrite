//! Keyboard shortcuts setting render function and command-name search support.

use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;

use crate::config::{KeyBinding, KeyCode, KeyModifiers, KeyboardShortcuts, ShortcutCommand};
use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::WARNING;

use super::registry::SettingsCtx;
use super::{KeyCaptureState, SettingsPanel};

/// Get localized shortcut command name.
pub(super) fn shortcut_command_name(cmd: &ShortcutCommand) -> String {
    match cmd {
        // File operations
        ShortcutCommand::Save => t!("shortcuts.commands.save").to_string(),
        ShortcutCommand::SaveAs => t!("shortcuts.commands.save_as").to_string(),
        ShortcutCommand::Open => t!("shortcuts.commands.open").to_string(),
        ShortcutCommand::New => t!("shortcuts.commands.new").to_string(),
        ShortcutCommand::NewTab => t!("shortcuts.commands.new_tab").to_string(),
        ShortcutCommand::CloseTab => t!("shortcuts.commands.close_tab").to_string(),
        ShortcutCommand::Reload => t!("shortcuts.commands.reload").to_string(),
        // Navigation
        ShortcutCommand::NextTab => t!("shortcuts.commands.next_tab").to_string(),
        ShortcutCommand::PrevTab => t!("shortcuts.commands.prev_tab").to_string(),
        ShortcutCommand::GoToLine => t!("shortcuts.commands.go_to_line").to_string(),
        ShortcutCommand::QuickOpen => t!("shortcuts.commands.quick_open").to_string(),
        // View
        ShortcutCommand::ToggleViewMode => t!("shortcuts.commands.toggle_view_mode").to_string(),
        ShortcutCommand::CycleTheme => t!("shortcuts.commands.cycle_theme").to_string(),
        ShortcutCommand::ToggleZenMode => t!("shortcuts.commands.toggle_zen_mode").to_string(),
        ShortcutCommand::ToggleWordWrap => t!("shortcuts.commands.toggle_word_wrap").to_string(),
        ShortcutCommand::TogglePreviewLock => {
            t!("shortcuts.commands.toggle_preview_lock").to_string()
        }
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
        ShortcutCommand::SelectNextOccurrence => {
            t!("shortcuts.commands.select_next_occurrence").to_string()
        }
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
        ShortcutCommand::ExportPdf => t!("shortcuts.commands.export_pdf").to_string(),
        ShortcutCommand::PrintPreview => t!("shortcuts.commands.print_preview").to_string(),
        ShortcutCommand::InsertToc => t!("shortcuts.commands.insert_toc").to_string(),
        ShortcutCommand::ToggleTerminal => cmd.display_name().to_string(),
        ShortcutCommand::ToggleProductivityHub => cmd.display_name().to_string(),
        ShortcutCommand::ToggleFrontmatter => "Toggle Frontmatter Panel".to_string(),
        ShortcutCommand::ZoomIn => "Zoom In".to_string(),
        ShortcutCommand::ZoomOut => "Zoom Out".to_string(),
        ShortcutCommand::ResetZoom => "Reset Zoom".to_string(),
        ShortcutCommand::CommandPalette => "Command Palette".to_string(),
        ShortcutCommand::OpenWorkspace => "Open Workspace".to_string(),
        ShortcutCommand::CloseWorkspace => "Close Workspace".to_string(),
        ShortcutCommand::NewWindow => t!("menu.window.new_window").to_string(),
    }
}

/// Whether any shortcut command name or category matches the lowercase query.
/// Used so the main settings search can surface individual bindings.
pub(super) fn any_command_matches(query: &str) -> bool {
    KeyboardShortcuts::commands_by_category()
        .iter()
        .any(|(category, commands)| {
            category.to_lowercase().contains(query)
                || commands
                    .iter()
                    .any(|cmd| shortcut_command_name(cmd).to_lowercase().contains(query))
        })
}

/// Keyboard shortcuts editor: reset-all, key capture, and the bindings list.
///
/// When the main settings search query matches command names, the list is
/// filtered to those commands; otherwise all bindings are shown.
pub(super) fn render_keyboard_shortcuts(
    panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let settings = &mut *ctx.settings;
    let mut changed = false;
    panel.keyboard_rendered_this_frame = true;

    // Reset all button
    ui.horizontal(|ui| {
        if ui
            .button(t!("settings.keyboard.reset_all").to_string())
            .on_hover_text(t!("settings.keyboard.reset_all_tooltip"))
            .clicked()
        {
            settings.keyboard_shortcuts.reset_all();
            panel.conflict_warning = None;
            changed = true;
        }
    });

    ui.add_space(8.0);

    // Show conflict warning if any
    if let Some((cmd, msg)) = &panel.conflict_warning {
        let warn_color = ui.visuals().warn_fg_color;
        ui.horizontal(|ui| {
            ui.label(phosphor_rich_text(WARNING, 14.0).color(warn_color));
            ui.label(
                RichText::new(format!("{}: {}", shortcut_command_name(cmd), msg)).color(warn_color),
            );
        });
        ui.add_space(4.0);
    }

    // Key capture modal - clone state for use in closure
    let mut cancel_capture = false;
    let mut apply_capture: Option<(ShortcutCommand, KeyBinding)> = None;

    if let Some(capture) = &panel.key_capture {
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
            ui.label(
                RichText::new(format!(
                    "{} \"{}\"...",
                    t!("settings.keyboard.press_key"),
                    cmd_name
                ))
                .strong(),
            );
            ui.add_space(4.0);

            ui.label(RichText::new(&display).monospace().size(16.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button(t!("settings.keyboard.cancel")).clicked() {
                    cancel_capture = true;
                }
                if has_key && ui.button(t!("settings.keyboard.apply")).clicked() {
                    if let Some(key) = capture_key {
                        apply_capture = Some((capture_cmd, KeyBinding::new(capture_mods, key)));
                    }
                }
            });
        });
        ui.add_space(8.0);
    }

    // Handle deferred capture actions
    if cancel_capture {
        panel.key_capture = None;
    }
    if let Some((cmd, binding)) = apply_capture {
        // Check for conflicts
        if let Some(conflict_cmd) = settings
            .keyboard_shortcuts
            .find_conflict(&binding, Some(cmd))
        {
            panel.conflict_warning = Some((
                cmd,
                format!(
                    "{} \"{}\"",
                    t!("settings.keyboard.conflict_with"),
                    shortcut_command_name(&conflict_cmd)
                ),
            ));
        } else {
            settings.keyboard_shortcuts.set(cmd, binding);
            panel.conflict_warning = None;
            changed = true;
        }
        panel.key_capture = None;
    }

    // Capture keyboard input when in capture mode
    let mut escape_pressed = false;
    let mut new_modifiers: Option<KeyModifiers> = None;
    let mut new_key: Option<KeyCode> = None;

    // Check if we already have a key captured (to latch modifiers)
    let key_already_captured = panel
        .key_capture
        .as_ref()
        .map(|c| c.key.is_some())
        .unwrap_or(false);

    if panel.key_capture.is_some() {
        ui.input(|i| {
            // Only update modifiers if no key captured yet (latch them once key is pressed)
            if !key_already_captured {
                new_modifiers = Some(KeyModifiers::from_egui(&i.modifiers));
            }

            // Check for key press
            for event in &i.events {
                if let egui::Event::Key {
                    key, pressed: true, ..
                } = event
                {
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
        panel.key_capture = None;
    } else if let Some(capture) = &mut panel.key_capture {
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

    // The main settings search doubles as the shortcut filter: only filter
    // rows when the query actually matches command names (otherwise the entry
    // matched by its own label/keywords and the full list should show).
    let query = panel.search_query.trim().to_lowercase();
    let filter_lower = if !query.is_empty() && any_command_matches(&query) {
        query
    } else {
        String::new()
    };

    for (category, commands) in KeyboardShortcuts::commands_by_category() {
        // Filter commands by search term
        let filtered_commands: Vec<_> = commands
            .iter()
            .filter(|cmd| {
                if filter_lower.is_empty() {
                    return true;
                }
                shortcut_command_name(cmd)
                    .to_lowercase()
                    .contains(&filter_lower)
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
            let is_capturing = panel
                .key_capture
                .as_ref()
                .map(|c| c.command == *cmd)
                .unwrap_or(false);

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
                    if is_custom
                        && ui
                            .small_button("↺")
                            .on_hover_text(t!("settings.keyboard.reset_default"))
                            .clicked()
                    {
                        settings.keyboard_shortcuts.reset(*cmd);
                        changed = true;
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
                    if btn.clicked() && panel.key_capture.is_none() {
                        panel.key_capture = Some(KeyCaptureState {
                            command: *cmd,
                            modifiers: KeyModifiers::none(),
                            key: None,
                        });
                        panel.conflict_warning = None;
                    }
                    if btn.hovered() && !is_capturing {
                        btn.on_hover_text(t!("settings.keyboard.click_to_change"));
                    }
                });
            });
        }
    }

    changed
}
