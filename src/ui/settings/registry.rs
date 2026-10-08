//! Declarative registry of every setting shown in the Settings panel.
//!
//! Each setting is described once as a [`SettingEntry`]: a stable id, the
//! section it belongs to, a localized label, extra English search keywords,
//! whether it appears in the default "Essentials" view, and a render function.
//! The panel shell (search, filter chips, recently-changed) operates purely on
//! this table, so adding a setting means adding one entry here plus its render
//! function in the section module.

use eframe::egui::Ui;
use rust_i18n::t;

use crate::config::Settings;

use super::{about, appearance, editor, files, keyboard, terminal, SettingsPanel, SettingsSection};

/// Maximum number of setting ids kept in the persisted recently-changed list.
pub const MAX_RECENTLY_CHANGED: usize = 8;

/// Shared mutable context passed to every setting render function.
pub struct SettingsCtx<'a> {
    /// Application settings being edited (live preview).
    pub settings: &'a mut Settings,
    /// Workspace root, used by LSP server detection.
    pub workspace_root: Option<&'a std::path::Path>,
}

/// One searchable, renderable setting (or cohesive group of sub-settings).
pub struct SettingEntry {
    /// Stable identifier, also persisted in the recently-changed list.
    pub id: &'static str,
    /// Section this entry belongs to (drives the filter chips).
    pub section: SettingsSection,
    /// Localized display label (used for search matching).
    pub label: fn() -> String,
    /// Extra lowercase English keywords matched against the search query.
    pub keywords: &'static [&'static str],
    /// Whether this entry shows in the default "Essentials" view.
    pub featured: bool,
    /// Renders the widgets; returns `true` if any value changed.
    pub render: fn(&mut SettingsPanel, &mut Ui, &mut SettingsCtx<'_>) -> bool,
}

impl SettingEntry {
    /// Whether this entry appears when browsing via chips or the Essentials
    /// view. Search-only entries (About) are reachable through search and the
    /// corner button instead.
    pub fn browsable(&self) -> bool {
        self.id != ABOUT_ID
    }

    /// Whether this entry matches a lowercase search query.
    pub fn matches(&self, query: &str) -> bool {
        if (self.label)().to_lowercase().contains(query) {
            return true;
        }
        if self.keywords.iter().any(|k| k.contains(query)) {
            return true;
        }
        if self.section.label().to_lowercase().contains(query) {
            return true;
        }
        // Keyboard entry also matches individual shortcut command names so
        // typing e.g. "bold" surfaces the Bold binding row.
        self.id == KEYBOARD_SHORTCUTS_ID && keyboard::any_command_matches(query)
    }
}

/// Id of the keyboard shortcuts entry (needs special search handling).
pub const KEYBOARD_SHORTCUTS_ID: &str = "keyboard.shortcuts";

/// Id of the search-only About entry.
pub const ABOUT_ID: &str = "about.info";

/// Look up an entry by its stable id.
pub fn entry_by_id(id: &str) -> Option<&'static SettingEntry> {
    all_entries().iter().find(|e| e.id == id)
}

/// Record a setting change in the persisted recently-changed list
/// (most recent first, deduplicated, capped).
///
/// Keyboard shortcut rebinds are excluded so the shortcut editor does not
/// jump into "Recently changed" and steal focus from the capture UI.
pub fn note_recently_changed(settings: &mut Settings, id: &str) {
    if id == KEYBOARD_SHORTCUTS_ID {
        return;
    }
    settings.recently_changed_settings.retain(|s| s != id);
    settings.recently_changed_settings.insert(0, id.to_string());
    settings
        .recently_changed_settings
        .truncate(MAX_RECENTLY_CHANGED);
}

/// Drop unknown ids and cap the persisted recently-changed list.
pub fn sanitize_recently_changed(settings: &mut Settings) {
    settings
        .recently_changed_settings
        .retain(|id| entry_by_id(id).is_some());
    settings
        .recently_changed_settings
        .truncate(MAX_RECENTLY_CHANGED);
}

/// The full ordered registry. Order within a section defines display order.
pub fn all_entries() -> &'static [SettingEntry] {
    ENTRIES
}

static ENTRIES: &[SettingEntry] = &[
    // ── Appearance ──────────────────────────────────────────────────────────
    SettingEntry {
        id: "appearance.theme",
        section: SettingsSection::Appearance,
        label: || t!("settings.general.theme").to_string(),
        keywords: &["theme", "dark", "light", "system", "mode", "color scheme"],
        featured: true,
        render: appearance::render_theme,
    },
    SettingEntry {
        id: "appearance.accent_color",
        section: SettingsSection::Appearance,
        label: || t!("settings.appearance.accent_color").to_string(),
        keywords: &["accent", "color", "highlight", "headings", "tint"],
        featured: true,
        render: appearance::render_accent_color,
    },
    SettingEntry {
        id: "appearance.language",
        section: SettingsSection::Appearance,
        label: || t!("settings.appearance.language").to_string(),
        keywords: &[
            "language",
            "locale",
            "translation",
            "english",
            "ui language",
        ],
        featured: true,
        render: appearance::render_language,
    },
    SettingEntry {
        id: "appearance.default_view_mode",
        section: SettingsSection::Appearance,
        label: || t!("settings.preview.default_view").to_string(),
        keywords: &[
            "view", "mode", "raw", "rendered", "split", "preview", "default",
        ],
        featured: true,
        render: appearance::render_default_view_mode,
    },
    SettingEntry {
        id: "appearance.font_family",
        section: SettingsSection::Appearance,
        label: || t!("settings.editor.font_family").to_string(),
        keywords: &[
            "font",
            "family",
            "typeface",
            "inter",
            "jetbrains",
            "custom font",
        ],
        featured: false,
        render: appearance::render_font_family,
    },
    SettingEntry {
        id: "appearance.rendered_font_family",
        section: SettingsSection::Appearance,
        label: || t!("settings.editor.rendered_font_family").to_string(),
        keywords: &["preview font", "rendered font", "proportional"],
        featured: false,
        render: appearance::render_rendered_font_family,
    },
    SettingEntry {
        id: "appearance.font_size",
        section: SettingsSection::Appearance,
        label: || t!("settings.editor.font_size").to_string(),
        keywords: &["font", "size", "text size", "zoom", "px", "points"],
        featured: true,
        render: appearance::render_font_size,
    },
    SettingEntry {
        id: "appearance.syntax_theme",
        section: SettingsSection::Appearance,
        label: || t!("settings.appearance.syntax_theme").to_string(),
        keywords: &["syntax", "theme", "code", "colors", "highlighting"],
        featured: false,
        render: appearance::render_syntax_theme,
    },
    SettingEntry {
        id: "appearance.system_title_bar",
        section: SettingsSection::Appearance,
        label: || t!("settings.appearance.system_title_bar").to_string(),
        keywords: &[
            "title bar",
            "window",
            "decorations",
            "chrome",
            "native",
            "frame",
        ],
        featured: false,
        render: appearance::render_system_title_bar,
    },
    SettingEntry {
        id: "appearance.complex_scripts",
        section: SettingsSection::Appearance,
        label: || t!("settings.editor.complex_scripts").to_string(),
        keywords: &[
            "scripts",
            "arabic",
            "bengali",
            "devanagari",
            "thai",
            "hebrew",
            "tamil",
            "georgian",
            "armenian",
            "ethiopic",
            "indic",
            "fonts",
        ],
        featured: false,
        render: appearance::render_complex_scripts,
    },
    SettingEntry {
        id: "appearance.cjk_preference",
        section: SettingsSection::Appearance,
        label: || t!("settings.editor.cjk_preference").to_string(),
        keywords: &[
            "cjk", "chinese", "japanese", "korean", "font", "glyph", "regional",
        ],
        featured: true,
        render: appearance::render_cjk_preference,
    },
    // ── Editor ──────────────────────────────────────────────────────────────
    SettingEntry {
        id: "editor.word_wrap",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.word_wrap").to_string(),
        keywords: &["wrap", "word", "lines", "horizontal scroll"],
        featured: true,
        render: editor::render_word_wrap,
    },
    SettingEntry {
        id: "editor.line_numbers",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.show_line_numbers").to_string(),
        keywords: &["line", "numbers", "gutter"],
        featured: true,
        render: editor::render_line_numbers,
    },
    SettingEntry {
        id: "editor.minimap",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.show_minimap").to_string(),
        keywords: &[
            "minimap",
            "overview",
            "map",
            "navigation",
            "semantic",
            "pixel",
        ],
        featured: true,
        render: editor::render_minimap,
    },
    SettingEntry {
        id: "editor.highlight_brackets",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.highlight_brackets").to_string(),
        keywords: &["brackets", "matching", "pairs", "highlight", "parentheses"],
        featured: true,
        render: editor::render_highlight_brackets,
    },
    SettingEntry {
        id: "editor.auto_close_brackets",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.auto_close_brackets").to_string(),
        keywords: &["brackets", "auto close", "pairs", "parentheses", "quotes"],
        featured: true,
        render: editor::render_auto_close_brackets,
    },
    SettingEntry {
        id: "editor.syntax_highlighting",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.syntax_highlighting").to_string(),
        keywords: &[
            "syntax",
            "highlighting",
            "code",
            "default language",
            "colors",
        ],
        featured: true,
        render: editor::render_syntax_highlighting,
    },
    SettingEntry {
        id: "editor.use_spaces",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.use_spaces").to_string(),
        keywords: &["spaces", "tabs", "indent", "soft tabs"],
        featured: true,
        render: editor::render_use_spaces,
    },
    SettingEntry {
        id: "editor.tab_size",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.tab_size").to_string(),
        keywords: &["tab", "size", "indent", "width", "spaces"],
        featured: false,
        render: editor::render_tab_size,
    },
    SettingEntry {
        id: "editor.vim_mode",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.vim_mode").to_string(),
        keywords: &["vim", "modal", "normal mode", "keybindings"],
        featured: false,
        render: editor::render_vim_mode,
    },
    #[cfg(feature = "spellcheck")]
    SettingEntry {
        id: "editor.spellcheck",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.spellcheck.enabled").to_string(),
        keywords: &["spell", "spelling", "dictionary", "typo", "squiggle"],
        featured: true,
        render: editor::render_spellcheck,
    },
    #[cfg(target_os = "linux")]
    SettingEntry {
        id: "editor.middle_click_paste",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.middle_click_paste").to_string(),
        keywords: &[
            "middle click",
            "paste",
            "primary selection",
            "x11",
            "wayland",
        ],
        featured: false,
        render: editor::render_middle_click_paste,
    },
    SettingEntry {
        id: "editor.strict_line_breaks",
        section: SettingsSection::Editor,
        label: || "Strict Line Breaks".to_string(),
        keywords: &[
            "line breaks",
            "newlines",
            "hard break",
            "markdown",
            "strict",
        ],
        featured: true,
        render: editor::render_strict_line_breaks,
    },
    SettingEntry {
        id: "editor.max_line_width",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.max_line_width").to_string(),
        keywords: &["line width", "maximum", "column", "measure", "readability"],
        featured: true,
        render: editor::render_max_line_width,
    },
    SettingEntry {
        id: "editor.lsp",
        section: SettingsSection::Editor,
        label: || "LSP (Language Servers)".to_string(),
        keywords: &[
            "lsp",
            "language server",
            "completion",
            "intelligence",
            "diagnostics",
            "binary",
        ],
        featured: false,
        render: editor::render_lsp,
    },
    SettingEntry {
        id: "editor.code_execution",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.code_execution_heading").to_string(),
        keywords: &[
            "code execution",
            "run",
            "shell",
            "python",
            "timeout",
            "runner",
            "execute",
        ],
        featured: false,
        render: editor::render_code_execution,
    },
    SettingEntry {
        id: "editor.code_folding",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.code_folding").to_string(),
        keywords: &[
            "fold",
            "folding",
            "collapse",
            "headings",
            "code blocks",
            "lists",
        ],
        featured: false,
        render: editor::render_code_folding,
    },
    SettingEntry {
        id: "editor.snippets",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.snippets").to_string(),
        keywords: &["snippets", "date", "time", "insert", "expansion"],
        featured: false,
        render: editor::render_snippets,
    },
    SettingEntry {
        id: "editor.paragraph_indent",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.paragraph_indent").to_string(),
        keywords: &["paragraph", "indent", "cjk", "first line"],
        featured: false,
        render: editor::render_paragraph_indent,
    },
    SettingEntry {
        id: "editor.header_spacing",
        section: SettingsSection::Editor,
        label: || t!("settings.editor.header_spacing").to_string(),
        keywords: &["header", "heading", "spacing", "margin", "rendering"],
        featured: false,
        render: editor::render_header_spacing,
    },
    // ── Files ───────────────────────────────────────────────────────────────
    SettingEntry {
        id: "files.restore_session",
        section: SettingsSection::Files,
        label: || t!("settings.general.restore_session").to_string(),
        keywords: &["session", "restore", "startup", "reopen", "tabs"],
        featured: false,
        render: files::render_restore_session,
    },
    SettingEntry {
        id: "files.show_welcome",
        section: SettingsSection::Files,
        label: || t!("settings.files.show_welcome").to_string(),
        keywords: &[
            "welcome",
            "launch",
            "startup",
            "empty",
            "first run",
            "blank tab",
        ],
        featured: true,
        render: files::render_show_welcome,
    },
    SettingEntry {
        id: "files.allow_multiple_instances",
        section: SettingsSection::Files,
        label: || t!("settings.files.allow_multiple_instances").to_string(),
        keywords: &[
            "instance",
            "instances",
            "multiple",
            "windows",
            "single instance",
            "new instance",
            "lock",
        ],
        featured: false,
        render: files::render_allow_multiple_instances,
    },
    SettingEntry {
        id: "files.quick_note_workflow",
        section: SettingsSection::Files,
        label: || t!("settings.files.quick_note_workflow").to_string(),
        keywords: &["quick note", "notepad", "scratch", "unsaved", "prompt"],
        featured: false,
        render: files::render_quick_note_workflow,
    },
    SettingEntry {
        id: "files.auto_save",
        section: SettingsSection::Files,
        label: || t!("settings.files.enable_auto_save").to_string(),
        keywords: &["autosave", "auto save", "save", "delay", "interval"],
        featured: true,
        render: files::render_auto_save,
    },
    SettingEntry {
        id: "files.recent_files",
        section: SettingsSection::Files,
        label: || t!("settings.files.recent_files").to_string(),
        keywords: &["recent", "files", "history", "clear", "remember"],
        featured: false,
        render: files::render_recent_files,
    },
    // ── Keyboard ────────────────────────────────────────────────────────────
    SettingEntry {
        id: KEYBOARD_SHORTCUTS_ID,
        section: SettingsSection::Keyboard,
        label: || t!("settings.keyboard.title").to_string(),
        keywords: &[
            "keyboard",
            "shortcuts",
            "hotkeys",
            "keys",
            "binding",
            "rebind",
        ],
        featured: false,
        render: keyboard::render_keyboard_shortcuts,
    },
    // ── Terminal ────────────────────────────────────────────────────────────
    SettingEntry {
        id: "terminal.enabled",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.enable").to_string(),
        keywords: &["terminal", "enable", "shell", "console"],
        featured: false,
        render: terminal::render_enabled,
    },
    SettingEntry {
        id: "terminal.font_size",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.font_size").to_string(),
        keywords: &["terminal", "font", "size"],
        featured: false,
        render: terminal::render_font_size,
    },
    SettingEntry {
        id: "terminal.font_family",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.font_family").to_string(),
        keywords: &[
            "terminal",
            "font",
            "family",
            "nerd",
            "powerline",
            "typeface",
            "mono",
        ],
        featured: false,
        render: terminal::render_font_family,
    },
    SettingEntry {
        id: "terminal.scrollback",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.scrollback").to_string(),
        keywords: &["terminal", "scrollback", "history", "lines", "buffer"],
        featured: false,
        render: terminal::render_scrollback,
    },
    SettingEntry {
        id: "terminal.copy_on_select",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.copy_selection").to_string(),
        keywords: &["terminal", "copy", "selection", "clipboard"],
        featured: false,
        render: terminal::render_copy_on_select,
    },
    SettingEntry {
        id: "terminal.theme",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.theme").to_string(),
        keywords: &["terminal", "theme", "colors"],
        featured: false,
        render: terminal::render_theme,
    },
    SettingEntry {
        id: "terminal.opacity",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.opacity").to_string(),
        keywords: &["terminal", "opacity", "transparency", "translucent"],
        featured: false,
        render: terminal::render_opacity,
    },
    SettingEntry {
        id: "terminal.startup_command",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.startup_command").to_string(),
        keywords: &["terminal", "startup", "command", "init", "launch"],
        featured: false,
        render: terminal::render_startup_command,
    },
    SettingEntry {
        id: "terminal.monitors",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.monitors").to_string(),
        keywords: &["terminal", "monitors", "displays", "screens"],
        featured: false,
        render: terminal::render_monitors,
    },
    SettingEntry {
        id: "terminal.breathing_color",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.breathing_color").to_string(),
        keywords: &["terminal", "breathing", "color", "glow", "animation"],
        featured: false,
        render: terminal::render_breathing_color,
    },
    SettingEntry {
        id: "terminal.prompt_patterns",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.prompt_patterns").to_string(),
        keywords: &["terminal", "prompt", "patterns", "regex", "detection"],
        featured: false,
        render: terminal::render_prompt_patterns,
    },
    SettingEntry {
        id: "terminal.auto_load_layout",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.auto_load_layout").to_string(),
        keywords: &["terminal", "layout", "auto load", "restore"],
        featured: false,
        render: terminal::render_auto_load_layout,
    },
    SettingEntry {
        id: "terminal.sound",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.sound_notification").to_string(),
        keywords: &[
            "terminal",
            "sound",
            "notification",
            "bell",
            "audio",
            "alert",
        ],
        featured: false,
        render: terminal::render_sound,
    },
    SettingEntry {
        id: "terminal.focus_on_detect",
        section: SettingsSection::Terminal,
        label: || t!("settings.terminal.auto_focus").to_string(),
        keywords: &["terminal", "focus", "prompt", "detect"],
        featured: false,
        render: terminal::render_focus_on_detect,
    },
    // ── About (search-only; also opens as popup from the corner button) ────
    SettingEntry {
        id: ABOUT_ID,
        section: SettingsSection::Appearance,
        label: || t!("settings.about.title").to_string(),
        keywords: &["about", "version", "update", "release", "license", "github"],
        featured: false,
        render: about::render_about_link,
    },
];
