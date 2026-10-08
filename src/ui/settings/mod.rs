//! Settings Panel Component for Ferrite
//!
//! Registry-backed settings UI rendered inline in a special tab:
//! - Instant search across every setting (including keyboard shortcut names)
//! - Single-select category filter chips instead of sidebar tabs
//! - Default view shows recently-changed settings plus welcome-screen
//!   "Essentials"; both disappear as soon as a query or chip is active
//! - About (version, update check, links) lives behind a corner button

mod about;
mod appearance;
mod editor;
mod files;
mod keyboard;
pub(crate) mod registry;
mod terminal;

use crate::config::{KeyCode, KeyModifiers, Settings, ShortcutCommand};
use crate::terminal::MonitorInfo;
use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::{
    ARROWS_COUNTER_CLOCKWISE, CLOCK_COUNTER_CLOCKWISE, GEAR, INFO, MAGNIFYING_GLASS, X,
};
use crate::update::{UpdateCheckResult, UpdateState};
use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;
use std::sync::mpsc;

pub(crate) use registry::{sanitize_recently_changed, MAX_RECENTLY_CHANGED};
pub use registry::{SettingEntry, SettingsCtx};

/// Settings categories used by the filter chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Appearance,
    Editor,
    Files,
    Keyboard,
    Terminal,
}

impl SettingsSection {
    /// All sections in chip display order.
    pub fn all() -> [SettingsSection; 5] {
        [
            SettingsSection::Appearance,
            SettingsSection::Editor,
            SettingsSection::Files,
            SettingsSection::Keyboard,
            SettingsSection::Terminal,
        ]
    }

    /// Get the display label for the section.
    pub fn label(&self) -> String {
        match self {
            SettingsSection::Appearance => t!("settings.appearance.title"),
            SettingsSection::Editor => t!("settings.editor.title"),
            SettingsSection::Files => t!("settings.files.title"),
            SettingsSection::Keyboard => t!("settings.keyboard.title"),
            SettingsSection::Terminal => t!("settings.terminal.title"),
        }
        .to_string()
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        use crate::ui::phosphor_icons::{FILES, KEYBOARD, NOTE_PENCIL, PALETTE, TERMINAL_WINDOW};
        match self {
            SettingsSection::Appearance => PALETTE,
            SettingsSection::Editor => NOTE_PENCIL,
            SettingsSection::Files => FILES,
            SettingsSection::Keyboard => KEYBOARD,
            SettingsSection::Terminal => TERMINAL_WINDOW,
        }
    }
}

/// Result of showing the settings panel.
#[derive(Debug, Clone, Default)]
pub struct SettingsPanelOutput {
    /// Whether settings were modified.
    pub changed: bool,
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
#[derive(Debug)]
pub struct SettingsPanel {
    /// Live search query (instant filtering).
    search_query: String,
    /// Active category chip (`None` = All).
    active_filter: Option<SettingsSection>,
    /// Request focus on the search field next frame.
    focus_search: bool,
    /// Whether the About popup window is open.
    about_open: bool,
    /// State for capturing a new key binding (None if not capturing)
    key_capture: Option<KeyCaptureState>,
    /// Conflict warning message (if any)
    conflict_warning: Option<(ShortcutCommand, String)>,
    /// Cached monitor info
    cached_monitor_info: Option<Vec<MonitorInfo>>,
    /// Current update check state
    update_state: UpdateState,
    /// Receiver for background update check result
    update_check_rx: Option<mpsc::Receiver<UpdateCheckResult>>,
    /// Frozen recently-changed ids for the current overview visit.
    overview_recent_ids: Option<Vec<String>>,
    /// Id of the settings search field (previous frame), used for Ctrl+F gating.
    search_field_id: Option<egui::Id>,
    /// Whether `show_inline` ran this frame (cleared in [`Self::end_frame`]).
    shown_this_frame: bool,
    /// Whether the keyboard shortcuts entry rendered this frame.
    keyboard_rendered_this_frame: bool,
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
            search_query: String::new(),
            active_filter: None,
            focus_search: false,
            about_open: false,
            key_capture: None,
            conflict_warning: None,
            cached_monitor_info: None,
            update_state: UpdateState::default(),
            update_check_rx: None,
            overview_recent_ids: None,
            search_field_id: None,
            shown_this_frame: false,
            keyboard_rendered_this_frame: false,
        }
    }

    /// Called at the end of the app frame. Clears the overview snapshot and
    /// key-capture state when the Settings tab was not shown.
    pub fn end_frame(&mut self) {
        if !self.shown_this_frame {
            self.overview_recent_ids = None;
            self.key_capture = None;
            self.conflict_warning = None;
        }
        self.shown_this_frame = false;
        self.keyboard_rendered_this_frame = false;
    }

    /// Focus the Keyboard section and prefilter to a specific command.
    pub fn open_keyboard_section_for(&mut self, command: ShortcutCommand) {
        self.active_filter = Some(SettingsSection::Keyboard);
        self.search_query = keyboard::shortcut_command_name(&command);
        self.key_capture = None;
        self.conflict_warning = None;
    }

    /// Render the settings panel inline within a tab (not as a modal window).
    ///
    /// This is used when settings are displayed as a special tab in the main
    /// editor area, giving more screen real estate than the modal version.
    pub fn show_inline(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        _is_dark: bool,
        workspace_root: Option<&std::path::Path>,
    ) -> SettingsPanelOutput {
        let mut output = SettingsPanelOutput::default();
        self.shown_this_frame = true;
        self.keyboard_rendered_this_frame = false;

        // Ctrl+F focuses search, but never while capturing a shortcut and never
        // when another widget (e.g. integrated terminal) owns focus.
        if self.key_capture.is_none() {
            let focused = ui.ctx().memory(|m| m.focused());
            let search_focused = self.search_field_id.is_some_and(|id| focused == Some(id));
            let nothing_focused = focused.is_none();
            if (search_focused || nothing_focused)
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F))
            {
                self.focus_search = true;
            }
        }

        ui.add_space(8.0);

        // ── Header: title + corner buttons ─────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(phosphor_rich_text(GEAR, 18.0));
            ui.label(RichText::new(t!("settings.title")).size(18.0).strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if ui
                    .button(format!("{} {}", INFO, t!("settings.about.title")))
                    .on_hover_text(t!("settings.about_button_tooltip"))
                    .clicked()
                {
                    self.about_open = !self.about_open;
                }
                if ui
                    .button(format!(
                        "{} {}",
                        ARROWS_COUNTER_CLOCKWISE,
                        t!("settings.reset_all")
                    ))
                    .on_hover_text(t!("settings.reset_tooltip"))
                    .clicked()
                {
                    output.reset_requested = true;
                }
            });
        });

        ui.add_space(8.0);

        // ── Search bar ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(phosphor_rich_text(MAGNIFYING_GLASS, 16.0));
            let clear_width = 36.0;
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .id_salt("settings_search")
                    .hint_text(t!("settings.search_hint"))
                    .desired_width((ui.available_width() - clear_width).max(120.0)),
            );
            self.search_field_id = Some(response.id);
            if self.focus_search {
                response.request_focus();
                self.focus_search = false;
            }
            // Esc clears the query (TextEdit surrenders focus on Escape).
            if (response.has_focus() || response.lost_focus())
                && ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                self.search_query.clear();
            }
            if !self.search_query.is_empty()
                && ui
                    .small_button(phosphor_rich_text(X, 12.0))
                    .on_hover_text(t!("settings.clear_search"))
                    .clicked()
            {
                self.search_query.clear();
            }
        });

        ui.add_space(6.0);

        let query = self.search_query.trim().to_lowercase();
        let searching = !query.is_empty();
        let overview = !searching && self.active_filter.is_none();
        if !overview {
            self.overview_recent_ids = None;
        }

        // ── Category filter chips ───────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
            ui.add_space(4.0);
            let all_selected = self.active_filter.is_none();
            if ui
                .add(egui::Button::selectable(
                    all_selected,
                    RichText::new(t!("settings.filter_all")).size(13.0),
                ))
                .clicked()
            {
                self.active_filter = None;
            }
            for section in SettingsSection::all() {
                let selected = self.active_filter == Some(section);
                let mut label = format!("{} {}", section.icon(), section.label());
                if searching {
                    let count = registry::all_entries()
                        .iter()
                        .filter(|e| e.section == section && e.browsable() && e.matches(&query))
                        .count();
                    label = format!("{} ({})", label, count);
                }
                if ui
                    .add(egui::Button::selectable(
                        selected,
                        RichText::new(label).size(13.0),
                    ))
                    .clicked()
                {
                    // Clicking the active chip toggles back to All.
                    self.active_filter = if selected { None } else { Some(section) };
                    if self.active_filter != Some(SettingsSection::Keyboard) {
                        self.key_capture = None;
                        self.conflict_warning = None;
                    }
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();

        // ── Content ─────────────────────────────────────────────────────────
        let mut ctx = SettingsCtx {
            settings,
            workspace_root,
        };

        egui::ScrollArea::vertical()
            .id_salt("settings_content_scroll")
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 16,
                        top: 8,
                        bottom: 24,
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(760.0);

                        if searching {
                            self.show_search_results(ui, &mut ctx, &query, &mut output);
                        } else if let Some(section) = self.active_filter {
                            self.show_section(ui, &mut ctx, section, &mut output);
                        } else {
                            self.show_overview(ui, &mut ctx, &mut output);
                        }
                    });
            });

        // About popup window (overlay)
        about::about_window(self, ui.ctx());

        if !self.keyboard_rendered_this_frame {
            self.key_capture = None;
            self.conflict_warning = None;
        }

        output
    }

    /// Render one registry entry with spacing and trailing separator.
    /// Records recently-changed and updates output on change.
    fn render_entry(
        &mut self,
        ui: &mut Ui,
        ctx: &mut SettingsCtx<'_>,
        entry: &'static SettingEntry,
        show_section_tag: bool,
        output: &mut SettingsPanelOutput,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new().id_salt(("setting", entry.id)),
            |ui| {
                ui.add_space(8.0);
                if show_section_tag {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            entry.section.icon(),
                            entry.section.label()
                        ))
                        .small()
                        .weak(),
                    );
                    ui.add_space(2.0);
                }
                if (entry.render)(self, ui, ctx) {
                    registry::note_recently_changed(ctx.settings, entry.id);
                    output.changed = true;
                }
                ui.add_space(8.0);
                ui.separator();
            },
        );
    }

    /// Search results grouped in registry order, with section tags per row.
    fn show_search_results(
        &mut self,
        ui: &mut Ui,
        ctx: &mut SettingsCtx<'_>,
        query: &str,
        output: &mut SettingsPanelOutput,
    ) {
        let matches: Vec<&'static SettingEntry> = registry::all_entries()
            .iter()
            .filter(|e| {
                self.active_filter
                    .is_none_or(|section| e.section == section && e.browsable())
                    && e.matches(query)
            })
            .collect();

        if matches.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(t!("settings.search_no_results")).weak());
            });
            return;
        }

        for entry in matches {
            self.render_entry(ui, ctx, entry, true, output);
        }
    }

    /// All entries of one section, in registry order.
    fn show_section(
        &mut self,
        ui: &mut Ui,
        ctx: &mut SettingsCtx<'_>,
        section: SettingsSection,
        output: &mut SettingsPanelOutput,
    ) {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{} {}", section.icon(), section.label()))
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);

        let entries: Vec<&'static SettingEntry> = registry::all_entries()
            .iter()
            .filter(|e| e.section == section && e.browsable())
            .collect();
        for entry in entries {
            self.render_entry(ui, ctx, entry, false, output);
        }
    }

    /// Default view: recently-changed settings followed by the Essentials
    /// (welcome-screen) set.
    fn show_overview(
        &mut self,
        ui: &mut Ui,
        ctx: &mut SettingsCtx<'_>,
        output: &mut SettingsPanelOutput,
    ) {
        // Freeze display order for this overview visit. Persistence still
        // updates immediately via note_recently_changed.
        if self.overview_recent_ids.is_none() {
            self.overview_recent_ids = Some(
                ctx.settings
                    .recently_changed_settings
                    .iter()
                    .take(registry::MAX_RECENTLY_CHANGED)
                    .cloned()
                    .collect(),
            );
        }
        let recent_ids = self.overview_recent_ids.clone().unwrap_or_default();
        let recent_entries: Vec<&'static SettingEntry> = recent_ids
            .iter()
            .filter_map(|id| registry::entry_by_id(id))
            .filter(|e| e.browsable())
            .collect();

        if !recent_entries.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    CLOCK_COUNTER_CLOCKWISE,
                    t!("settings.recently_changed")
                ))
                .strong()
                .size(16.0),
            );
            ui.add_space(4.0);
            for entry in &recent_entries {
                self.render_entry(ui, ctx, entry, true, output);
            }
            ui.add_space(12.0);
        }

        ui.add_space(4.0);
        ui.label(RichText::new(t!("settings.essentials")).strong().size(16.0));
        ui.label(RichText::new(t!("settings.essentials_hint")).weak().small());
        ui.add_space(4.0);

        // Skip entries already rendered above (also avoids duplicate egui ids).
        let featured: Vec<&'static SettingEntry> = registry::all_entries()
            .iter()
            .filter(|e| e.featured && e.browsable() && !recent_ids.iter().any(|id| id == e.id))
            .collect();
        for entry in featured {
            self.render_entry(ui, ctx, entry, true, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_panel_new() {
        let panel = SettingsPanel::new();
        assert!(panel.search_query.is_empty());
        assert_eq!(panel.active_filter, None);
        assert!(!panel.about_open);
    }

    #[test]
    fn test_settings_panel_default() {
        let panel = SettingsPanel::default();
        assert_eq!(panel.active_filter, None);
    }

    #[test]
    fn test_settings_section_label() {
        assert_eq!(SettingsSection::Appearance.label(), "Appearance");
        assert_eq!(SettingsSection::Editor.label(), "Editor");
        assert_eq!(SettingsSection::Files.label(), "Files");
        assert_eq!(SettingsSection::Terminal.label(), "Terminal");
    }

    #[test]
    fn test_settings_section_icon() {
        assert_eq!(
            SettingsSection::Appearance.icon(),
            crate::ui::phosphor_icons::PALETTE
        );
        assert_eq!(
            SettingsSection::Editor.icon(),
            crate::ui::phosphor_icons::NOTE_PENCIL
        );
        assert_eq!(
            SettingsSection::Files.icon(),
            crate::ui::phosphor_icons::FILES
        );
        assert_eq!(
            SettingsSection::Terminal.icon(),
            crate::ui::phosphor_icons::TERMINAL_WINDOW
        );
    }

    #[test]
    fn test_settings_panel_output_default() {
        let output = SettingsPanelOutput::default();
        assert!(!output.changed);
        assert!(!output.reset_requested);
    }

    #[test]
    fn test_registry_ids_unique() {
        let mut ids: Vec<&str> = registry::all_entries().iter().map(|e| e.id).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate setting ids in registry");
    }

    #[test]
    fn test_registry_covers_all_sections() {
        for section in SettingsSection::all() {
            assert!(
                registry::all_entries()
                    .iter()
                    .any(|e| e.section == section && e.browsable()),
                "no browsable entries for section {:?}",
                section
            );
        }
    }

    #[test]
    fn test_registry_has_featured_entries() {
        assert!(registry::all_entries().iter().any(|e| e.featured));
    }

    #[test]
    fn test_files_show_welcome_exists_and_featured() {
        let entry = registry::entry_by_id("files.show_welcome").expect("files.show_welcome exists");
        assert!(entry.featured);
        assert_eq!(entry.section, SettingsSection::Files);
        assert!(entry.browsable());
    }

    #[test]
    fn test_search_matches_language_setting() {
        // "lang" must instantly surface the UI language setting (English locale).
        let entry = registry::entry_by_id("appearance.language").expect("entry exists");
        assert!(entry.matches("lang"));
    }

    #[test]
    fn test_search_matches_keyboard_command() {
        // Shortcut command names are searchable through the keyboard entry.
        let entry = registry::entry_by_id(registry::KEYBOARD_SHORTCUTS_ID).expect("entry exists");
        assert!(entry.matches("bold"));
    }

    #[test]
    fn test_about_entry_not_browsable() {
        let entry = registry::entry_by_id(registry::ABOUT_ID).expect("entry exists");
        assert!(!entry.browsable());
        assert!(entry.matches("version"));
    }

    #[test]
    fn test_note_recently_changed_caps_and_dedupes() {
        let mut settings = Settings::default();
        for i in 0..12 {
            registry::note_recently_changed(&mut settings, &format!("id{}", i));
        }
        assert_eq!(
            settings.recently_changed_settings.len(),
            registry::MAX_RECENTLY_CHANGED
        );
        assert_eq!(settings.recently_changed_settings[0], "id11");

        // Re-changing an existing id moves it to the front without duplicating.
        registry::note_recently_changed(&mut settings, "id7");
        assert_eq!(settings.recently_changed_settings[0], "id7");
        let count = settings
            .recently_changed_settings
            .iter()
            .filter(|s| *s == "id7")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_note_recently_changed_skips_keyboard_shortcuts() {
        let mut settings = Settings::default();
        registry::note_recently_changed(&mut settings, registry::KEYBOARD_SHORTCUTS_ID);
        assert!(settings.recently_changed_settings.is_empty());
    }

    #[test]
    fn test_open_keyboard_section_for_prefills_search() {
        let mut panel = SettingsPanel::new();
        panel.open_keyboard_section_for(ShortcutCommand::FormatBold);
        assert_eq!(panel.active_filter, Some(SettingsSection::Keyboard));
        assert!(!panel.search_query.is_empty());
    }

    #[test]
    fn test_settings_locale_keys_exist_and_search_hint_removed() {
        use std::collections::BTreeSet;

        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let settings_dir = manifest.join("src/ui/settings");
        let key_re = regex::Regex::new(r#"t!\(\s*"((?:settings)\.[a-z0-9_.]+)""#).unwrap();
        let mut referenced = BTreeSet::new();
        for entry in std::fs::read_dir(&settings_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for cap in key_re.captures_iter(&src) {
                referenced.insert(cap[1].to_string());
            }
        }
        assert!(
            referenced.contains("settings.clear_search"),
            "settings.clear_search must be referenced"
        );

        let en: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(manifest.join("locales/en.yaml")).unwrap(),
        )
        .unwrap();
        let mut defined = BTreeSet::new();
        flatten_yaml_keys(&en, "", &mut defined);
        let missing: Vec<_> = referenced
            .iter()
            .filter(|k| !defined.contains(*k))
            .cloned()
            .collect();
        assert!(
            missing.is_empty(),
            "settings.* keys used in src/ui/settings missing from locales/en.yaml: {missing:?}"
        );

        let locales_dir = manifest.join("locales");
        for entry in std::fs::read_dir(&locales_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(
                yaml.get("settings")
                    .and_then(|s| s.get("keyboard"))
                    .and_then(|k| k.get("search_hint"))
                    .is_none(),
                "{} still defines settings.keyboard.search_hint",
                path.display()
            );
        }
    }

    fn flatten_yaml_keys(
        value: &serde_yaml::Value,
        prefix: &str,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        let serde_yaml::Value::Mapping(map) = value else {
            if !prefix.is_empty() {
                out.insert(prefix.to_string());
            }
            return;
        };
        for (k, v) in map {
            let Some(key) = k.as_str() else {
                continue;
            };
            let path = if prefix.is_empty() {
                key.to_string()
            } else {
                format!("{prefix}.{key}")
            };
            if matches!(v, serde_yaml::Value::Mapping(_)) {
                flatten_yaml_keys(v, &path, out);
            } else {
                out.insert(path);
            }
        }
    }
}
