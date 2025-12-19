//! User settings and preferences for Ferrite
//!
//! This module defines the `Settings` struct that holds all user-configurable
//! options, with serde support for JSON persistence.

// Allow dead code - this module contains complete API with methods for UI display
// labels and settings that may not all be used yet but provide consistent API
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Theme Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Available color themes for the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
    System,
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Family Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Available font families for the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorFont {
    /// Inter - Modern, clean UI font (default)
    #[default]
    Inter,
    /// JetBrains Mono - Monospace font, good for code-heavy documents
    JetBrainsMono,
}

impl EditorFont {
    /// Get the display name for the font.
    pub fn display_name(&self) -> &'static str {
        match self {
            EditorFont::Inter => "Inter",
            EditorFont::JetBrainsMono => "JetBrains Mono",
        }
    }

    /// Get a description of the font.
    pub fn description(&self) -> &'static str {
        match self {
            EditorFont::Inter => "Modern, clean proportional font",
            EditorFont::JetBrainsMono => "Monospace font for code",
        }
    }

    /// Get all available fonts.
    pub fn all() -> &'static [EditorFont] {
        &[EditorFont::Inter, EditorFont::JetBrainsMono]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// View Mode Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Editor view modes for markdown editing.
///
/// Two modes are available:
/// - `Raw`: Plain markdown text editing using a standard text editor
/// - `Rendered`: WYSIWYG editing with rendered markdown elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Raw markdown text editing (plain TextEdit)
    #[default]
    Raw,
    /// WYSIWYG rendered editing (MarkdownEditor)
    Rendered,
}

impl ViewMode {
    /// Toggle between Raw and Rendered modes.
    pub fn toggle(&self) -> Self {
        match self {
            ViewMode::Raw => ViewMode::Rendered,
            ViewMode::Rendered => ViewMode::Raw,
        }
    }

    /// Get a display label for the mode.
    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::Raw => "Raw",
            ViewMode::Rendered => "Rendered",
        }
    }

    /// Get an icon/symbol for the mode.
    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            ViewMode::Raw => "📝",
            ViewMode::Rendered => "👁",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Outline Panel Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Which side of the editor the outline panel should appear on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutlinePanelSide {
    /// Outline panel on the left side
    Left,
    /// Outline panel on the right side (default)
    #[default]
    Right,
}

impl OutlinePanelSide {
    /// Toggle between left and right.
    #[allow(dead_code)]
    pub fn toggle(&self) -> Self {
        match self {
            OutlinePanelSide::Left => OutlinePanelSide::Right,
            OutlinePanelSide::Right => OutlinePanelSide::Left,
        }
    }

    /// Get display label.
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            OutlinePanelSide::Left => "Left",
            OutlinePanelSide::Right => "Right",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Window Size Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Window dimensions and position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    /// Window width in pixels
    pub width: f32,
    /// Window height in pixels
    pub height: f32,
    /// Window X position (optional, for restoring position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    /// Window Y position (optional, for restoring position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    /// Whether the window was maximized
    #[serde(default)]
    pub maximized: bool,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tab Information
// ─────────────────────────────────────────────────────────────────────────────

/// Information about an open tab for session restoration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    /// Path to the file (None for unsaved/new files)
    pub path: Option<PathBuf>,
    /// Whether this tab has unsaved changes (used for recovery)
    #[serde(default)]
    pub modified: bool,
    /// Cursor position (line, column)
    #[serde(default)]
    pub cursor_position: (usize, usize),
    /// Scroll position
    #[serde(default)]
    pub scroll_offset: f32,
    /// View mode for this tab (raw or rendered)
    #[serde(default)]
    pub view_mode: ViewMode,
}

impl Default for TabInfo {
    fn default() -> Self {
        Self {
            path: None,
            modified: false,
            cursor_position: (0, 0),
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw, // New documents default to raw mode
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Settings Struct
// ─────────────────────────────────────────────────────────────────────────────

/// User preferences and application settings.
///
/// This struct is serialized to JSON and persisted to the user's config directory.
/// All fields have sensible defaults via the `Default` trait and `#[serde(default)]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // ─────────────────────────────────────────────────────────────────────────
    // Appearance
    // ─────────────────────────────────────────────────────────────────────────
    /// Color theme (light, dark, or system)
    pub theme: Theme,

    /// Editor view mode (editor only, preview only, or split view)
    pub view_mode: ViewMode,

    /// Whether to show line numbers in the editor
    pub show_line_numbers: bool,

    /// Font size for the editor (in points)
    pub font_size: f32,

    /// Font family for the editor
    pub font_family: EditorFont,

    // ─────────────────────────────────────────────────────────────────────────
    // Editor Behavior
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to enable word wrap
    pub word_wrap: bool,

    /// Tab size (number of spaces)
    pub tab_size: u8,

    /// Whether to use spaces instead of tabs
    pub use_spaces: bool,

    /// Whether to auto-save files
    pub auto_save: bool,

    /// Auto-save interval in seconds (if auto_save is enabled)
    pub auto_save_interval_secs: u32,

    // ─────────────────────────────────────────────────────────────────────────
    // Session & History
    // ─────────────────────────────────────────────────────────────────────────
    /// Recently opened files (most recent first)
    pub recent_files: Vec<PathBuf>,

    /// Maximum number of recent files to remember
    pub max_recent_files: usize,

    /// Last open tabs for session restoration
    pub last_open_tabs: Vec<TabInfo>,

    /// Index of the active tab (for session restoration)
    pub active_tab_index: usize,

    // ─────────────────────────────────────────────────────────────────────────
    // Window State
    // ─────────────────────────────────────────────────────────────────────────
    /// Window size and position
    pub window_size: WindowSize,

    /// Split ratio for the editor/preview panes (0.0 to 1.0)
    pub split_ratio: f32,

    // ─────────────────────────────────────────────────────────────────────────
    // Syntax Highlighting
    // ─────────────────────────────────────────────────────────────────────────
    /// Syntax highlighting theme name
    pub syntax_theme: String,

    // ─────────────────────────────────────────────────────────────────────────
    // Outline Panel
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether the outline panel is visible
    pub outline_enabled: bool,

    /// Which side of the editor the outline panel appears on
    pub outline_side: OutlinePanelSide,

    /// Width of the outline panel in pixels
    pub outline_width: f32,

    // ─────────────────────────────────────────────────────────────────────────
    // Sync Scrolling
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether synchronized scrolling between Raw and Rendered views is enabled
    pub sync_scroll_enabled: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Export Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Last directory used for HTML export
    pub last_export_directory: Option<std::path::PathBuf>,

    /// Whether to open exported files after export
    pub open_after_export: bool,

    /// Whether to embed images as base64 in exports
    pub export_embed_images: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Workspace Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Recently opened workspaces (folders), most recent first
    pub recent_workspaces: Vec<PathBuf>,

    /// Maximum number of recent workspaces to remember
    pub max_recent_workspaces: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Appearance
            theme: Theme::default(),
            view_mode: ViewMode::default(),
            show_line_numbers: true,
            font_size: 14.0,
            font_family: EditorFont::default(),

            // Editor Behavior
            word_wrap: true,
            tab_size: 4,
            use_spaces: true,
            auto_save: false,
            auto_save_interval_secs: 60,

            // Session & History
            recent_files: Vec::new(),
            max_recent_files: 10,
            last_open_tabs: Vec::new(),
            active_tab_index: 0,

            // Window State
            window_size: WindowSize::default(),
            split_ratio: 0.5,

            // Syntax Highlighting
            syntax_theme: String::from("base16-ocean.dark"),

            // Outline Panel
            outline_enabled: false, // Hidden by default
            outline_side: OutlinePanelSide::default(),
            outline_width: 200.0,

            // Sync Scrolling
            sync_scroll_enabled: true, // Enabled by default

            // Export Settings
            last_export_directory: None,
            open_after_export: false,
            export_embed_images: true, // Standalone files by default

            // Workspace Settings
            recent_workspaces: Vec::new(),
            max_recent_workspaces: 10,
        }
    }
}

impl Settings {
    /// Add a file to the recent files list.
    ///
    /// If the file already exists in the list, it's moved to the front.
    /// The list is trimmed to `max_recent_files`.
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);
        // Add to front
        self.recent_files.insert(0, path);
        // Trim to max
        self.recent_files.truncate(self.max_recent_files);
    }

    /// Add a workspace (folder) to the recent workspaces list.
    ///
    /// If the workspace already exists in the list, it's moved to the front.
    /// The list is trimmed to `max_recent_workspaces`.
    pub fn add_recent_workspace(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_workspaces.retain(|p| p != &path);
        // Add to front
        self.recent_workspaces.insert(0, path);
        // Trim to max
        self.recent_workspaces.truncate(self.max_recent_workspaces);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation Constants and Sanitization
    // ─────────────────────────────────────────────────────────────────────────

    /// Minimum allowed font size.
    pub const MIN_FONT_SIZE: f32 = 8.0;
    /// Maximum allowed font size.
    pub const MAX_FONT_SIZE: f32 = 72.0;
    /// Minimum allowed tab size.
    pub const MIN_TAB_SIZE: u8 = 1;
    /// Maximum allowed tab size.
    pub const MAX_TAB_SIZE: u8 = 8;
    /// Minimum window dimension.
    pub const MIN_WINDOW_SIZE: f32 = 200.0;
    /// Maximum window dimension.
    pub const MAX_WINDOW_SIZE: f32 = 10000.0;
    /// Minimum outline panel width.
    pub const MIN_OUTLINE_WIDTH: f32 = 120.0;
    /// Maximum outline panel width.
    pub const MAX_OUTLINE_WIDTH: f32 = 500.0;

    /// Sanitize settings by clamping values to valid ranges.
    ///
    /// This is useful after loading settings from a file that might have
    /// been manually edited with invalid values.
    pub fn sanitize(&mut self) {
        // Clamp font size
        self.font_size = self
            .font_size
            .clamp(Self::MIN_FONT_SIZE, Self::MAX_FONT_SIZE);

        // Clamp tab size
        self.tab_size = self.tab_size.clamp(Self::MIN_TAB_SIZE, Self::MAX_TAB_SIZE);

        // Clamp window size
        self.window_size.width = self
            .window_size
            .width
            .clamp(Self::MIN_WINDOW_SIZE, Self::MAX_WINDOW_SIZE);
        self.window_size.height = self
            .window_size
            .height
            .clamp(Self::MIN_WINDOW_SIZE, Self::MAX_WINDOW_SIZE);

        // Clamp split ratio
        self.split_ratio = self.split_ratio.clamp(0.0, 1.0);

        // Ensure max_recent_files is reasonable
        if self.max_recent_files == 0 {
            self.max_recent_files = 10;
        } else if self.max_recent_files > 100 {
            self.max_recent_files = 100;
        }

        // Trim recent files to max
        self.recent_files.truncate(self.max_recent_files);

        // Ensure auto-save interval is reasonable
        if self.auto_save && self.auto_save_interval_secs < 5 {
            self.auto_save_interval_secs = 5;
        }

        // Ensure active_tab_index is valid
        if !self.last_open_tabs.is_empty() && self.active_tab_index >= self.last_open_tabs.len() {
            self.active_tab_index = self.last_open_tabs.len() - 1;
        }

        // Clamp outline width
        self.outline_width = self
            .outline_width
            .clamp(Self::MIN_OUTLINE_WIDTH, Self::MAX_OUTLINE_WIDTH);
    }

    /// Load settings and sanitize them to ensure validity.
    ///
    /// This is a convenience method that deserializes and then sanitizes.
    pub fn from_json_sanitized(json: &str) -> Result<Self, serde_json::Error> {
        let mut settings: Self = serde_json::from_str(json)?;
        settings.sanitize();
        Ok(settings)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();

        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.view_mode, ViewMode::Raw);
        assert!(settings.show_line_numbers);
        assert_eq!(settings.font_size, 14.0);
        assert!(settings.recent_files.is_empty());
        assert_eq!(settings.max_recent_files, 10);
        assert_eq!(settings.window_size.width, 1200.0);
        assert_eq!(settings.window_size.height, 800.0);
        assert_eq!(settings.split_ratio, 0.5);
    }

    #[test]
    fn test_add_recent_file() {
        let mut settings = Settings::default();
        settings.max_recent_files = 3;

        settings.add_recent_file(PathBuf::from("/file1.md"));
        settings.add_recent_file(PathBuf::from("/file2.md"));
        settings.add_recent_file(PathBuf::from("/file3.md"));

        assert_eq!(settings.recent_files.len(), 3);
        assert_eq!(settings.recent_files[0], PathBuf::from("/file3.md"));
        assert_eq!(settings.recent_files[2], PathBuf::from("/file1.md"));

        // Add existing file - should move to front
        settings.add_recent_file(PathBuf::from("/file1.md"));
        assert_eq!(settings.recent_files[0], PathBuf::from("/file1.md"));
        assert_eq!(settings.recent_files.len(), 3);

        // Add new file - should trim oldest
        settings.add_recent_file(PathBuf::from("/file4.md"));
        assert_eq!(settings.recent_files.len(), 3);
        assert_eq!(settings.recent_files[0], PathBuf::from("/file4.md"));
        assert!(!settings.recent_files.contains(&PathBuf::from("/file2.md")));
    }

    #[test]
    fn test_theme_serialization() {
        assert_eq!(serde_json::to_string(&Theme::Light).unwrap(), "\"light\"");
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(serde_json::to_string(&Theme::System).unwrap(), "\"system\"");
    }

    #[test]
    fn test_theme_deserialization() {
        assert_eq!(
            serde_json::from_str::<Theme>("\"light\"").unwrap(),
            Theme::Light
        );
        assert_eq!(
            serde_json::from_str::<Theme>("\"dark\"").unwrap(),
            Theme::Dark
        );
        assert_eq!(
            serde_json::from_str::<Theme>("\"system\"").unwrap(),
            Theme::System
        );
    }

    #[test]
    fn test_view_mode_serialization() {
        assert_eq!(serde_json::to_string(&ViewMode::Raw).unwrap(), "\"raw\"");
        assert_eq!(
            serde_json::to_string(&ViewMode::Rendered).unwrap(),
            "\"rendered\""
        );
    }

    #[test]
    fn test_view_mode_toggle() {
        assert_eq!(ViewMode::Raw.toggle(), ViewMode::Rendered);
        assert_eq!(ViewMode::Rendered.toggle(), ViewMode::Raw);
    }

    #[test]
    fn test_view_mode_labels() {
        assert_eq!(ViewMode::Raw.label(), "Raw");
        assert_eq!(ViewMode::Rendered.label(), "Rendered");
        assert_eq!(ViewMode::Raw.icon(), "📝");
        assert_eq!(ViewMode::Rendered.icon(), "👁");
    }

    #[test]
    fn test_settings_serialization_roundtrip() {
        let original = Settings::default();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_settings_deserialize_with_defaults() {
        // Minimal JSON - should fill in defaults
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();

        assert_eq!(settings.theme, Theme::Dark);
        // All other fields should have defaults
        assert_eq!(settings.view_mode, ViewMode::Raw);
        assert!(settings.show_line_numbers);
        assert_eq!(settings.font_size, 14.0);
    }

    #[test]
    fn test_settings_deserialize_empty_json() {
        // Empty JSON object - should use all defaults
        let json = "{}";
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_window_size_default() {
        let size = WindowSize::default();
        assert_eq!(size.width, 1200.0);
        assert_eq!(size.height, 800.0);
        assert!(size.x.is_none());
        assert!(size.y.is_none());
        assert!(!size.maximized);
    }

    #[test]
    fn test_tab_info_default() {
        let tab = TabInfo::default();
        assert!(tab.path.is_none());
        assert!(!tab.modified);
        assert_eq!(tab.cursor_position, (0, 0));
        assert_eq!(tab.scroll_offset, 0.0);
    }

    #[test]
    fn test_tab_info_serialization() {
        let tab = TabInfo {
            path: Some(PathBuf::from("/test.md")),
            modified: true,
            cursor_position: (10, 5),
            scroll_offset: 100.0,
            view_mode: ViewMode::Rendered,
        };

        let json = serde_json::to_string(&tab).unwrap();
        let deserialized: TabInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(tab, deserialized);
    }

    #[test]
    fn test_tab_info_default_view_mode() {
        let tab = TabInfo::default();
        assert_eq!(tab.view_mode, ViewMode::Raw); // Default to raw mode
    }

    #[test]
    fn test_tab_info_backward_compatibility() {
        // Old JSON without view_mode field should default to Raw
        let json = r#"{"path": "/test.md", "modified": false, "cursor_position": [0, 0], "scroll_offset": 0.0}"#;
        let tab: TabInfo = serde_json::from_str(json).unwrap();
        assert_eq!(tab.view_mode, ViewMode::Raw);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sanitization tests
    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn test_sanitize_font_size() {
        let mut settings = Settings::default();
        settings.font_size = 4.0;
        settings.sanitize();
        assert_eq!(settings.font_size, Settings::MIN_FONT_SIZE);

        settings.font_size = 100.0;
        settings.sanitize();
        assert_eq!(settings.font_size, Settings::MAX_FONT_SIZE);
    }

    #[test]
    fn test_sanitize_tab_size() {
        let mut settings = Settings::default();
        settings.tab_size = 0;
        settings.sanitize();
        assert_eq!(settings.tab_size, Settings::MIN_TAB_SIZE);

        settings.tab_size = 20;
        settings.sanitize();
        assert_eq!(settings.tab_size, Settings::MAX_TAB_SIZE);
    }

    #[test]
    fn test_sanitize_split_ratio() {
        let mut settings = Settings::default();
        settings.split_ratio = -0.5;
        settings.sanitize();
        assert_eq!(settings.split_ratio, 0.0);

        settings.split_ratio = 1.5;
        settings.sanitize();
        assert_eq!(settings.split_ratio, 1.0);
    }

    #[test]
    fn test_sanitize_recent_files() {
        let mut settings = Settings::default();
        settings.max_recent_files = 2;
        settings.recent_files = vec![
            PathBuf::from("/file1.md"),
            PathBuf::from("/file2.md"),
            PathBuf::from("/file3.md"),
        ];
        settings.sanitize();
        assert_eq!(settings.recent_files.len(), 2);
    }

    #[test]
    fn test_sanitize_active_tab_index() {
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo::default()];
        settings.active_tab_index = 5;
        settings.sanitize();
        assert_eq!(settings.active_tab_index, 0);
    }

    #[test]
    fn test_from_json_sanitized() {
        let json = r#"{"font_size": 4.0, "split_ratio": 2.0}"#;
        let settings = Settings::from_json_sanitized(json).unwrap();
        assert_eq!(settings.font_size, Settings::MIN_FONT_SIZE);
        assert_eq!(settings.split_ratio, 1.0);
    }
}
