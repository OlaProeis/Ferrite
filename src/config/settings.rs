//! User settings and preferences for Ferrite
//!
//! This module defines the `Settings` struct that holds all user-configurable
//! options, with serde support for JSON persistence.

// Allow dead code - this module contains complete API with methods for UI display
// labels and settings that may not all be used yet but provide consistent API
#![allow(dead_code)]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Keyboard Shortcut Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Modifier keys for keyboard shortcuts (serializable wrapper for egui::Modifiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyModifiers {
    /// Ctrl key (Command on macOS)
    #[serde(default)]
    pub ctrl: bool,
    /// Shift key
    #[serde(default)]
    pub shift: bool,
    /// Alt key (Option on macOS)
    #[serde(default)]
    pub alt: bool,
    /// Raw Ctrl key (physical Ctrl on all platforms, including macOS)
    /// This is used for shortcuts that need the actual Ctrl key, not Command on macOS
    #[serde(default)]
    pub raw_ctrl: bool,
}

impl KeyModifiers {
    /// Create modifiers with only Ctrl/Command
    pub const fn ctrl() -> Self {
        Self { ctrl: true, shift: false, alt: false, raw_ctrl: false }
    }

    /// Create modifiers with Ctrl+Shift
    pub const fn ctrl_shift() -> Self {
        Self { ctrl: true, shift: true, alt: false, raw_ctrl: false }
    }

    /// Create modifiers with only Alt
    pub const fn alt() -> Self {
        Self { ctrl: false, shift: false, alt: true, raw_ctrl: false }
    }

    /// Create modifiers with only Shift
    pub const fn shift() -> Self {
        Self { ctrl: false, shift: true, alt: false, raw_ctrl: false }
    }

    /// No modifiers
    pub const fn none() -> Self {
        Self { ctrl: false, shift: false, alt: false, raw_ctrl: false }
    }

    /// Create modifiers with only raw Ctrl (physical Ctrl key on all platforms)
    /// This is used for shortcuts that need the actual Ctrl key, not Command on macOS
    pub const fn raw_ctrl() -> Self {
        Self { ctrl: false, shift: false, alt: false, raw_ctrl: true }
    }

    /// Convert to egui::Modifiers for comparison
    pub fn to_egui(&self) -> egui::Modifiers {
        let mut mods = egui::Modifiers::NONE;
        if self.ctrl {
            mods = mods | egui::Modifiers::COMMAND;
        }
        if self.shift {
            mods = mods | egui::Modifiers::SHIFT;
        }
        if self.alt {
            mods = mods | egui::Modifiers::ALT;
        }
        // Note: raw_ctrl is handled specially in KeyBinding::matches()
        mods
    }

    /// Create from egui::Modifiers
    pub fn from_egui(mods: &egui::Modifiers) -> Self {
        Self {
            ctrl: mods.command,
            shift: mods.shift,
            alt: mods.alt,
            raw_ctrl: false, // raw_ctrl is only set explicitly, not from egui input
        }
    }

    /// Get display string for the modifiers
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();
        if self.raw_ctrl {
            parts.push("Ctrl"); // Always show "Ctrl" for raw_ctrl, even on macOS
        } else if self.ctrl {
            parts.push(if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" });
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push(if cfg!(target_os = "macos") { "Option" } else { "Alt" });
        }
        parts.join("+")
    }
}

/// Key codes for keyboard shortcuts (serializable wrapper).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Special keys
    Escape, Tab, Backspace, Enter, Space,
    ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    Home, End, PageUp, PageDown, Insert, Delete,
    // Punctuation
    Comma, Period, Semicolon, Colon,
    OpenBracket, CloseBracket,
    Backslash, Slash, Minus, Equals, Backtick,
}

impl KeyCode {
    /// Convert to egui::Key
    pub fn to_egui(&self) -> egui::Key {
        match self {
            // Letters
            KeyCode::A => egui::Key::A,
            KeyCode::B => egui::Key::B,
            KeyCode::C => egui::Key::C,
            KeyCode::D => egui::Key::D,
            KeyCode::E => egui::Key::E,
            KeyCode::F => egui::Key::F,
            KeyCode::G => egui::Key::G,
            KeyCode::H => egui::Key::H,
            KeyCode::I => egui::Key::I,
            KeyCode::J => egui::Key::J,
            KeyCode::K => egui::Key::K,
            KeyCode::L => egui::Key::L,
            KeyCode::M => egui::Key::M,
            KeyCode::N => egui::Key::N,
            KeyCode::O => egui::Key::O,
            KeyCode::P => egui::Key::P,
            KeyCode::Q => egui::Key::Q,
            KeyCode::R => egui::Key::R,
            KeyCode::S => egui::Key::S,
            KeyCode::T => egui::Key::T,
            KeyCode::U => egui::Key::U,
            KeyCode::V => egui::Key::V,
            KeyCode::W => egui::Key::W,
            KeyCode::X => egui::Key::X,
            KeyCode::Y => egui::Key::Y,
            KeyCode::Z => egui::Key::Z,
            // Numbers
            KeyCode::Num0 => egui::Key::Num0,
            KeyCode::Num1 => egui::Key::Num1,
            KeyCode::Num2 => egui::Key::Num2,
            KeyCode::Num3 => egui::Key::Num3,
            KeyCode::Num4 => egui::Key::Num4,
            KeyCode::Num5 => egui::Key::Num5,
            KeyCode::Num6 => egui::Key::Num6,
            KeyCode::Num7 => egui::Key::Num7,
            KeyCode::Num8 => egui::Key::Num8,
            KeyCode::Num9 => egui::Key::Num9,
            // Function keys
            KeyCode::F1 => egui::Key::F1,
            KeyCode::F2 => egui::Key::F2,
            KeyCode::F3 => egui::Key::F3,
            KeyCode::F4 => egui::Key::F4,
            KeyCode::F5 => egui::Key::F5,
            KeyCode::F6 => egui::Key::F6,
            KeyCode::F7 => egui::Key::F7,
            KeyCode::F8 => egui::Key::F8,
            KeyCode::F9 => egui::Key::F9,
            KeyCode::F10 => egui::Key::F10,
            KeyCode::F11 => egui::Key::F11,
            KeyCode::F12 => egui::Key::F12,
            // Special keys
            KeyCode::Escape => egui::Key::Escape,
            KeyCode::Tab => egui::Key::Tab,
            KeyCode::Backspace => egui::Key::Backspace,
            KeyCode::Enter => egui::Key::Enter,
            KeyCode::Space => egui::Key::Space,
            KeyCode::ArrowLeft => egui::Key::ArrowLeft,
            KeyCode::ArrowRight => egui::Key::ArrowRight,
            KeyCode::ArrowUp => egui::Key::ArrowUp,
            KeyCode::ArrowDown => egui::Key::ArrowDown,
            KeyCode::Home => egui::Key::Home,
            KeyCode::End => egui::Key::End,
            KeyCode::PageUp => egui::Key::PageUp,
            KeyCode::PageDown => egui::Key::PageDown,
            KeyCode::Insert => egui::Key::Insert,
            KeyCode::Delete => egui::Key::Delete,
            // Punctuation
            KeyCode::Comma => egui::Key::Comma,
            KeyCode::Period => egui::Key::Period,
            KeyCode::Semicolon => egui::Key::Semicolon,
            KeyCode::Colon => egui::Key::Colon,
            KeyCode::OpenBracket => egui::Key::OpenBracket,
            KeyCode::CloseBracket => egui::Key::CloseBracket,
            KeyCode::Backslash => egui::Key::Backslash,
            KeyCode::Slash => egui::Key::Slash,
            KeyCode::Minus => egui::Key::Minus,
            KeyCode::Equals => egui::Key::Equals,
            KeyCode::Backtick => egui::Key::Backtick,
        }
    }

    /// Try to create from egui::Key
    pub fn from_egui(key: egui::Key) -> Option<Self> {
        Some(match key {
            // Letters
            egui::Key::A => KeyCode::A,
            egui::Key::B => KeyCode::B,
            egui::Key::C => KeyCode::C,
            egui::Key::D => KeyCode::D,
            egui::Key::E => KeyCode::E,
            egui::Key::F => KeyCode::F,
            egui::Key::G => KeyCode::G,
            egui::Key::H => KeyCode::H,
            egui::Key::I => KeyCode::I,
            egui::Key::J => KeyCode::J,
            egui::Key::K => KeyCode::K,
            egui::Key::L => KeyCode::L,
            egui::Key::M => KeyCode::M,
            egui::Key::N => KeyCode::N,
            egui::Key::O => KeyCode::O,
            egui::Key::P => KeyCode::P,
            egui::Key::Q => KeyCode::Q,
            egui::Key::R => KeyCode::R,
            egui::Key::S => KeyCode::S,
            egui::Key::T => KeyCode::T,
            egui::Key::U => KeyCode::U,
            egui::Key::V => KeyCode::V,
            egui::Key::W => KeyCode::W,
            egui::Key::X => KeyCode::X,
            egui::Key::Y => KeyCode::Y,
            egui::Key::Z => KeyCode::Z,
            // Numbers
            egui::Key::Num0 => KeyCode::Num0,
            egui::Key::Num1 => KeyCode::Num1,
            egui::Key::Num2 => KeyCode::Num2,
            egui::Key::Num3 => KeyCode::Num3,
            egui::Key::Num4 => KeyCode::Num4,
            egui::Key::Num5 => KeyCode::Num5,
            egui::Key::Num6 => KeyCode::Num6,
            egui::Key::Num7 => KeyCode::Num7,
            egui::Key::Num8 => KeyCode::Num8,
            egui::Key::Num9 => KeyCode::Num9,
            // Function keys
            egui::Key::F1 => KeyCode::F1,
            egui::Key::F2 => KeyCode::F2,
            egui::Key::F3 => KeyCode::F3,
            egui::Key::F4 => KeyCode::F4,
            egui::Key::F5 => KeyCode::F5,
            egui::Key::F6 => KeyCode::F6,
            egui::Key::F7 => KeyCode::F7,
            egui::Key::F8 => KeyCode::F8,
            egui::Key::F9 => KeyCode::F9,
            egui::Key::F10 => KeyCode::F10,
            egui::Key::F11 => KeyCode::F11,
            egui::Key::F12 => KeyCode::F12,
            // Special keys
            egui::Key::Escape => KeyCode::Escape,
            egui::Key::Tab => KeyCode::Tab,
            egui::Key::Backspace => KeyCode::Backspace,
            egui::Key::Enter => KeyCode::Enter,
            egui::Key::Space => KeyCode::Space,
            egui::Key::ArrowLeft => KeyCode::ArrowLeft,
            egui::Key::ArrowRight => KeyCode::ArrowRight,
            egui::Key::ArrowUp => KeyCode::ArrowUp,
            egui::Key::ArrowDown => KeyCode::ArrowDown,
            egui::Key::Home => KeyCode::Home,
            egui::Key::End => KeyCode::End,
            egui::Key::PageUp => KeyCode::PageUp,
            egui::Key::PageDown => KeyCode::PageDown,
            egui::Key::Insert => KeyCode::Insert,
            egui::Key::Delete => KeyCode::Delete,
            // Punctuation
            egui::Key::Comma => KeyCode::Comma,
            egui::Key::Period => KeyCode::Period,
            egui::Key::Semicolon => KeyCode::Semicolon,
            egui::Key::Colon => KeyCode::Colon,
            egui::Key::OpenBracket => KeyCode::OpenBracket,
            egui::Key::CloseBracket => KeyCode::CloseBracket,
            egui::Key::Backslash => KeyCode::Backslash,
            egui::Key::Slash => KeyCode::Slash,
            egui::Key::Minus => KeyCode::Minus,
            egui::Key::Equals => KeyCode::Equals,
            egui::Key::Backtick => KeyCode::Backtick,
            _ => return None,
        })
    }

    /// Get display string for the key
    pub fn display_string(&self) -> &'static str {
        match self {
            // Letters
            KeyCode::A => "A", KeyCode::B => "B", KeyCode::C => "C", KeyCode::D => "D",
            KeyCode::E => "E", KeyCode::F => "F", KeyCode::G => "G", KeyCode::H => "H",
            KeyCode::I => "I", KeyCode::J => "J", KeyCode::K => "K", KeyCode::L => "L",
            KeyCode::M => "M", KeyCode::N => "N", KeyCode::O => "O", KeyCode::P => "P",
            KeyCode::Q => "Q", KeyCode::R => "R", KeyCode::S => "S", KeyCode::T => "T",
            KeyCode::U => "U", KeyCode::V => "V", KeyCode::W => "W", KeyCode::X => "X",
            KeyCode::Y => "Y", KeyCode::Z => "Z",
            // Numbers
            KeyCode::Num0 => "0", KeyCode::Num1 => "1", KeyCode::Num2 => "2",
            KeyCode::Num3 => "3", KeyCode::Num4 => "4", KeyCode::Num5 => "5",
            KeyCode::Num6 => "6", KeyCode::Num7 => "7", KeyCode::Num8 => "8",
            KeyCode::Num9 => "9",
            // Function keys
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", KeyCode::F3 => "F3",
            KeyCode::F4 => "F4", KeyCode::F5 => "F5", KeyCode::F6 => "F6",
            KeyCode::F7 => "F7", KeyCode::F8 => "F8", KeyCode::F9 => "F9",
            KeyCode::F10 => "F10", KeyCode::F11 => "F11", KeyCode::F12 => "F12",
            // Special keys
            KeyCode::Escape => "Esc", KeyCode::Tab => "Tab", KeyCode::Backspace => "Backspace",
            KeyCode::Enter => "Enter", KeyCode::Space => "Space",
            KeyCode::ArrowLeft => "←", KeyCode::ArrowRight => "→",
            KeyCode::ArrowUp => "↑", KeyCode::ArrowDown => "↓",
            KeyCode::Home => "Home", KeyCode::End => "End",
            KeyCode::PageUp => "PgUp", KeyCode::PageDown => "PgDn",
            KeyCode::Insert => "Ins", KeyCode::Delete => "Del",
            // Punctuation
            KeyCode::Comma => ",", KeyCode::Period => ".",
            KeyCode::Semicolon => ";", KeyCode::Colon => ":",
            KeyCode::OpenBracket => "[", KeyCode::CloseBracket => "]",
            KeyCode::Backslash => "\\", KeyCode::Slash => "/",
            KeyCode::Minus => "-", KeyCode::Equals => "=",
            KeyCode::Backtick => "`",
        }
    }
}

/// A keyboard shortcut binding with modifiers and key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBinding {
    /// Modifier keys (Ctrl, Shift, Alt)
    pub modifiers: KeyModifiers,
    /// The main key
    pub key: KeyCode,
}

impl KeyBinding {
    /// Create a new key binding
    pub const fn new(modifiers: KeyModifiers, key: KeyCode) -> Self {
        Self { modifiers, key }
    }

    /// Check if this binding matches the current input state
    pub fn matches(&self, input: &egui::InputState) -> bool {
        let key = self.key.to_egui();

        // Check modifiers match exactly
        // For raw_ctrl, we check the physical Ctrl key directly, not Command
        let mods_match = if self.modifiers.raw_ctrl {
            // Raw Ctrl mode: check physical Ctrl key and ensure Command/other modifiers are not pressed
            input.modifiers.ctrl
                && !input.modifiers.command
                && input.modifiers.shift == self.modifiers.shift
                && input.modifiers.alt == self.modifiers.alt
        } else {
            // Normal mode: check Command (Cmd on macOS, Ctrl on others)
            input.modifiers.command == self.modifiers.ctrl
                && input.modifiers.shift == self.modifiers.shift
                && input.modifiers.alt == self.modifiers.alt
        };

        mods_match && input.key_pressed(key)
    }

    /// Get the display string for this binding (e.g., "Ctrl+S")
    pub fn display_string(&self) -> String {
        let mods = self.modifiers.display_string();
        let key = self.key.display_string();
        if mods.is_empty() {
            key.to_string()
        } else {
            format!("{}+{}", mods, key)
        }
    }
}

/// Command identifier for keyboard shortcuts.
///
/// This enum identifies all commands that can be bound to keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutCommand {
    // File operations
    Save,
    SaveAs,
    Open,
    New,
    NewTab,
    CloseTab,
    // Navigation
    NextTab,
    PrevTab,
    GoToLine,
    QuickOpen,
    // View
    ToggleViewMode,
    CycleTheme,
    ToggleZenMode,
    ToggleFullscreen,
    ToggleOutline,
    ToggleFileTree,
    TogglePipeline,
    // Edit
    Undo,
    Redo,
    DeleteLine,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    SelectNextOccurrence,
    // Search
    Find,
    FindReplace,
    FindNext,
    FindPrev,
    SearchInFiles,
    // Formatting
    FormatBold,
    FormatItalic,
    FormatInlineCode,
    FormatCodeBlock,
    FormatLink,
    FormatImage,
    FormatBlockquote,
    FormatBulletList,
    FormatNumberedList,
    FormatHeading1,
    FormatHeading2,
    FormatHeading3,
    FormatHeading4,
    FormatHeading5,
    FormatHeading6,
    // Folding
    FoldAll,
    UnfoldAll,
    ToggleFoldAtCursor,
    // Other
    OpenSettings,
    OpenAbout,
    ExportHtml,
    InsertToc,
}

impl ShortcutCommand {
    /// Get all available commands
    pub fn all() -> &'static [ShortcutCommand] {
        use ShortcutCommand::*;
        &[
            // File operations
            Save, SaveAs, Open, New, NewTab, CloseTab,
            // Navigation
            NextTab, PrevTab, GoToLine, QuickOpen,
            // View
            ToggleViewMode, CycleTheme, ToggleZenMode, ToggleOutline, ToggleFileTree, TogglePipeline,
            // Edit
            Undo, Redo, DeleteLine, DuplicateLine, MoveLineUp, MoveLineDown, SelectNextOccurrence,
            // Search
            Find, FindReplace, FindNext, FindPrev, SearchInFiles,
            // Formatting
            FormatBold, FormatItalic, FormatInlineCode, FormatCodeBlock, FormatLink, FormatImage,
            FormatBlockquote, FormatBulletList, FormatNumberedList,
            FormatHeading1, FormatHeading2, FormatHeading3, FormatHeading4, FormatHeading5, FormatHeading6,
            // Folding
            FoldAll, UnfoldAll, ToggleFoldAtCursor,
            // Other
            OpenSettings, OpenAbout, ExportHtml, InsertToc,
        ]
    }

    /// Get display name for the command
    pub fn display_name(&self) -> &'static str {
        match self {
            // File operations
            ShortcutCommand::Save => "Save",
            ShortcutCommand::SaveAs => "Save As",
            ShortcutCommand::Open => "Open File",
            ShortcutCommand::New => "New File",
            ShortcutCommand::NewTab => "New Tab",
            ShortcutCommand::CloseTab => "Close Tab",
            // Navigation
            ShortcutCommand::NextTab => "Next Tab",
            ShortcutCommand::PrevTab => "Previous Tab",
            ShortcutCommand::GoToLine => "Go to Line",
            ShortcutCommand::QuickOpen => "Quick Open",
            // View
            ShortcutCommand::ToggleViewMode => "Toggle View Mode",
            ShortcutCommand::CycleTheme => "Cycle Theme",
            ShortcutCommand::ToggleZenMode => "Toggle Zen Mode",
            ShortcutCommand::ToggleFullscreen => "Toggle Fullscreen",
            ShortcutCommand::ToggleOutline => "Toggle Outline",
            ShortcutCommand::ToggleFileTree => "Toggle File Tree",
            ShortcutCommand::TogglePipeline => "Toggle Pipeline",
            // Edit
            ShortcutCommand::Undo => "Undo",
            ShortcutCommand::Redo => "Redo",
            ShortcutCommand::DeleteLine => "Delete Line",
            ShortcutCommand::DuplicateLine => "Duplicate Line",
            ShortcutCommand::MoveLineUp => "Move Line Up",
            ShortcutCommand::MoveLineDown => "Move Line Down",
            ShortcutCommand::SelectNextOccurrence => "Select Next Occurrence",
            // Search
            ShortcutCommand::Find => "Find",
            ShortcutCommand::FindReplace => "Find & Replace",
            ShortcutCommand::FindNext => "Find Next",
            ShortcutCommand::FindPrev => "Find Previous",
            ShortcutCommand::SearchInFiles => "Search in Files",
            // Formatting
            ShortcutCommand::FormatBold => "Bold",
            ShortcutCommand::FormatItalic => "Italic",
            ShortcutCommand::FormatInlineCode => "Inline Code",
            ShortcutCommand::FormatCodeBlock => "Code Block",
            ShortcutCommand::FormatLink => "Link",
            ShortcutCommand::FormatImage => "Image",
            ShortcutCommand::FormatBlockquote => "Blockquote",
            ShortcutCommand::FormatBulletList => "Bullet List",
            ShortcutCommand::FormatNumberedList => "Numbered List",
            ShortcutCommand::FormatHeading1 => "Heading 1",
            ShortcutCommand::FormatHeading2 => "Heading 2",
            ShortcutCommand::FormatHeading3 => "Heading 3",
            ShortcutCommand::FormatHeading4 => "Heading 4",
            ShortcutCommand::FormatHeading5 => "Heading 5",
            ShortcutCommand::FormatHeading6 => "Heading 6",
            // Folding
            ShortcutCommand::FoldAll => "Fold All",
            ShortcutCommand::UnfoldAll => "Unfold All",
            ShortcutCommand::ToggleFoldAtCursor => "Toggle Fold",
            // Other
            ShortcutCommand::OpenSettings => "Open Settings",
            ShortcutCommand::OpenAbout => "Open About",
            ShortcutCommand::ExportHtml => "Export HTML",
            ShortcutCommand::InsertToc => "Insert/Update TOC",
        }
    }

    /// Get the category for grouping in UI
    pub fn category(&self) -> &'static str {
        match self {
            ShortcutCommand::Save | ShortcutCommand::SaveAs | ShortcutCommand::Open
            | ShortcutCommand::New | ShortcutCommand::NewTab | ShortcutCommand::CloseTab => "File",

            ShortcutCommand::NextTab | ShortcutCommand::PrevTab | ShortcutCommand::GoToLine
            | ShortcutCommand::QuickOpen => "Navigation",

            ShortcutCommand::ToggleViewMode | ShortcutCommand::CycleTheme | ShortcutCommand::ToggleZenMode
            | ShortcutCommand::ToggleFullscreen | ShortcutCommand::ToggleOutline | ShortcutCommand::ToggleFileTree 
            | ShortcutCommand::TogglePipeline => "View",

            ShortcutCommand::Undo | ShortcutCommand::Redo | ShortcutCommand::DeleteLine
            | ShortcutCommand::DuplicateLine | ShortcutCommand::MoveLineUp | ShortcutCommand::MoveLineDown
            | ShortcutCommand::SelectNextOccurrence => "Edit",

            ShortcutCommand::Find | ShortcutCommand::FindReplace | ShortcutCommand::FindNext
            | ShortcutCommand::FindPrev | ShortcutCommand::SearchInFiles => "Search",

            ShortcutCommand::FormatBold | ShortcutCommand::FormatItalic | ShortcutCommand::FormatInlineCode
            | ShortcutCommand::FormatCodeBlock | ShortcutCommand::FormatLink | ShortcutCommand::FormatImage
            | ShortcutCommand::FormatBlockquote | ShortcutCommand::FormatBulletList | ShortcutCommand::FormatNumberedList
            | ShortcutCommand::FormatHeading1 | ShortcutCommand::FormatHeading2 | ShortcutCommand::FormatHeading3
            | ShortcutCommand::FormatHeading4 | ShortcutCommand::FormatHeading5 | ShortcutCommand::FormatHeading6 => "Format",

            ShortcutCommand::FoldAll | ShortcutCommand::UnfoldAll | ShortcutCommand::ToggleFoldAtCursor => "Folding",

            ShortcutCommand::OpenSettings | ShortcutCommand::OpenAbout | ShortcutCommand::ExportHtml
            | ShortcutCommand::InsertToc => "Other",
        }
    }

    /// Get the default key binding for this command
    pub fn default_binding(&self) -> KeyBinding {
        use KeyCode::*;
        use KeyModifiers as M;
        match self {
            // File operations
            ShortcutCommand::Save => KeyBinding::new(M::ctrl(), S),
            ShortcutCommand::SaveAs => KeyBinding::new(M::ctrl_shift(), S),
            ShortcutCommand::Open => KeyBinding::new(M::ctrl(), O),
            ShortcutCommand::New => KeyBinding::new(M::ctrl(), N),
            ShortcutCommand::NewTab => KeyBinding::new(M::ctrl(), T),
            ShortcutCommand::CloseTab => KeyBinding::new(M::ctrl(), W),
            // Navigation
            ShortcutCommand::NextTab => KeyBinding::new(M::ctrl(), Tab),
            ShortcutCommand::PrevTab => KeyBinding::new(M::ctrl_shift(), Tab),
            ShortcutCommand::GoToLine => KeyBinding::new(M::ctrl_shift(), G),
            ShortcutCommand::QuickOpen => KeyBinding::new(M::ctrl(), P),
            // View
            ShortcutCommand::ToggleViewMode => KeyBinding::new(M::ctrl(), E),
            ShortcutCommand::CycleTheme => KeyBinding::new(M::ctrl_shift(), T),
            ShortcutCommand::ToggleZenMode => KeyBinding::new(M::none(), F11),
            ShortcutCommand::ToggleFullscreen => KeyBinding::new(M::none(), F10),
            ShortcutCommand::ToggleOutline => KeyBinding::new(M::ctrl_shift(), O),
            ShortcutCommand::ToggleFileTree => KeyBinding::new(M::ctrl(), B),
            ShortcutCommand::TogglePipeline => KeyBinding::new(M::ctrl_shift(), L),
            // Edit
            ShortcutCommand::Undo => KeyBinding::new(M::ctrl(), Z),
            ShortcutCommand::Redo => KeyBinding::new(M::ctrl(), Y),
            ShortcutCommand::DeleteLine => KeyBinding::new(M::ctrl(), D),
            ShortcutCommand::DuplicateLine => KeyBinding::new(M::ctrl_shift(), D),
            ShortcutCommand::MoveLineUp => KeyBinding::new(M::alt(), ArrowUp),
            ShortcutCommand::MoveLineDown => KeyBinding::new(M::alt(), ArrowDown),
            ShortcutCommand::SelectNextOccurrence => KeyBinding::new(M::raw_ctrl(), G),
            // Search
            ShortcutCommand::Find => KeyBinding::new(M::ctrl(), F),
            ShortcutCommand::FindReplace => KeyBinding::new(M::ctrl(), H),
            ShortcutCommand::FindNext => KeyBinding::new(M::none(), F3),
            ShortcutCommand::FindPrev => KeyBinding::new(M::shift(), F3),
            ShortcutCommand::SearchInFiles => KeyBinding::new(M::ctrl_shift(), F),
            // Formatting
            ShortcutCommand::FormatBold => KeyBinding::new(M::ctrl(), B),
            ShortcutCommand::FormatItalic => KeyBinding::new(M::ctrl(), I),
            ShortcutCommand::FormatInlineCode => KeyBinding::new(M::ctrl(), Backtick),
            ShortcutCommand::FormatCodeBlock => KeyBinding::new(M::ctrl_shift(), C),
            ShortcutCommand::FormatLink => KeyBinding::new(M::ctrl(), K),
            ShortcutCommand::FormatImage => KeyBinding::new(M::ctrl_shift(), K),
            ShortcutCommand::FormatBlockquote => KeyBinding::new(M::ctrl(), Q),
            ShortcutCommand::FormatBulletList => KeyBinding::new(M::ctrl_shift(), B),
            ShortcutCommand::FormatNumberedList => KeyBinding::new(M::ctrl_shift(), N),
            ShortcutCommand::FormatHeading1 => KeyBinding::new(M::ctrl(), Num1),
            ShortcutCommand::FormatHeading2 => KeyBinding::new(M::ctrl(), Num2),
            ShortcutCommand::FormatHeading3 => KeyBinding::new(M::ctrl(), Num3),
            ShortcutCommand::FormatHeading4 => KeyBinding::new(M::ctrl(), Num4),
            ShortcutCommand::FormatHeading5 => KeyBinding::new(M::ctrl(), Num5),
            ShortcutCommand::FormatHeading6 => KeyBinding::new(M::ctrl(), Num6),
            // Folding
            ShortcutCommand::FoldAll => KeyBinding::new(M::ctrl_shift(), OpenBracket),
            ShortcutCommand::UnfoldAll => KeyBinding::new(M::ctrl_shift(), CloseBracket),
            ShortcutCommand::ToggleFoldAtCursor => KeyBinding::new(M::ctrl_shift(), Period),
            // Other
            ShortcutCommand::OpenSettings => KeyBinding::new(M::ctrl(), Comma),
            ShortcutCommand::OpenAbout => KeyBinding::new(M::none(), F1),
            ShortcutCommand::ExportHtml => KeyBinding::new(M::ctrl_shift(), E),
            ShortcutCommand::InsertToc => KeyBinding::new(M::ctrl_shift(), U),
        }
    }
}

/// Keyboard shortcuts configuration.
///
/// Maps commands to their key bindings. Uses a HashMap for flexible storage
/// while providing easy access methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardShortcuts {
    /// Custom bindings that override defaults (command -> binding)
    bindings: std::collections::HashMap<ShortcutCommand, KeyBinding>,
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
        }
    }
}

impl KeyboardShortcuts {
    /// Get the binding for a command (custom or default)
    pub fn get(&self, command: ShortcutCommand) -> KeyBinding {
        self.bindings
            .get(&command)
            .copied()
            .unwrap_or_else(|| command.default_binding())
    }

    /// Set a custom binding for a command
    pub fn set(&mut self, command: ShortcutCommand, binding: KeyBinding) {
        self.bindings.insert(command, binding);
    }

    /// Reset a command to its default binding
    pub fn reset(&mut self, command: ShortcutCommand) {
        self.bindings.remove(&command);
    }

    /// Reset all commands to default bindings
    pub fn reset_all(&mut self) {
        self.bindings.clear();
    }

    /// Check if a command has a custom binding
    pub fn is_custom(&self, command: ShortcutCommand) -> bool {
        self.bindings.contains_key(&command)
    }

    /// Find which command uses a given binding, if any
    pub fn find_conflict(&self, binding: &KeyBinding, exclude: Option<ShortcutCommand>) -> Option<ShortcutCommand> {
        for cmd in ShortcutCommand::all() {
            if exclude == Some(*cmd) {
                continue;
            }
            if self.get(*cmd) == *binding {
                return Some(*cmd);
            }
        }
        None
    }

    /// Get all commands grouped by category
    pub fn commands_by_category() -> Vec<(&'static str, Vec<ShortcutCommand>)> {
        let categories = ["File", "Navigation", "View", "Edit", "Search", "Format", "Folding", "Other"];
        categories
            .iter()
            .map(|&cat| {
                let cmds: Vec<_> = ShortcutCommand::all()
                    .iter()
                    .filter(|cmd| cmd.category() == cat)
                    .copied()
                    .collect();
                (cat, cmds)
            })
            .filter(|(_, cmds)| !cmds.is_empty())
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Language Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Available UI languages for the application.
///
/// Each variant corresponds to a locale file in the `locales/` directory.
/// The language code follows BCP 47 format (e.g., "en", "zh-CN").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English (default)
    #[default]
    #[serde(rename = "en")]
    English,
    /// Simplified Chinese (简体中文)
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
}

impl Language {
    /// Get the locale code for rust-i18n (e.g., "en", "zh_Hans").
    pub fn locale_code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::ChineseSimplified => "zh_Hans",
        }
    }

    /// Get the native display name (e.g., "English", "简体中文").
    pub fn native_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::ChineseSimplified => "简体中文",
        }
    }

    /// Get all available languages.
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::ChineseSimplified,
        ]
    }

    /// Try to match a system locale code to an available language.
    ///
    /// Accepts various locale formats:
    /// - Full locale: "en-US", "en_US", "zh-CN", "zh_CN"
    /// - Language only: "en", "zh"
    /// - Case-insensitive matching
    ///
    /// Returns `None` if no matching language is available.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(Language::from_locale_code("en-US"), Some(Language::English));
    /// assert_eq!(Language::from_locale_code("en"), Some(Language::English));
    /// assert_eq!(Language::from_locale_code("zh-CN"), Some(Language::ChineseSimplified));
    /// assert_eq!(Language::from_locale_code("unknown"), None);
    /// ```
    pub fn from_locale_code(locale: &str) -> Option<Language> {
        // Normalize: lowercase and replace underscore with hyphen
        let normalized = locale.to_lowercase().replace('_', "-");

        // Extract the primary language tag (before the first hyphen)
        let primary_lang = normalized.split('-').next().unwrap_or(&normalized);

        // Match against available languages
        match primary_lang {
            "en" => Some(Language::English),
            "zh" => Some(Language::ChineseSimplified),
            _ => None,
        }
    }

    /// Detect the best language based on system locale.
    ///
    /// Uses `sys_locale::get_locale()` to detect the system's preferred language,
    /// then maps it to an available `Language` variant. Falls back to English
    /// if the system locale is unavailable or not supported.
    ///
    /// This should only be called on first run (when no config exists) to avoid
    /// overriding user preferences.
    pub fn from_system_locale() -> Language {
        sys_locale::get_locale()
            .and_then(|locale| {
                log::debug!("Detected system locale: {}", locale);
                Self::from_locale_code(&locale)
            })
            .unwrap_or_else(|| {
                log::debug!("No matching language for system locale, defaulting to English");
                Language::English
            })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CJK Paragraph Indentation
// ─────────────────────────────────────────────────────────────────────────────

/// Paragraph indentation setting for CJK typography.
///
/// In Chinese/Japanese writing conventions, paragraphs traditionally begin with
/// indentation. Chinese uses 2 full-width spaces (2em), Japanese uses 1 full-width
/// space (1em). This setting applies text-indent styling to paragraphs in
/// Rendered/Preview mode and HTML export.
/// Reference: GitHub Issue #20
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParagraphIndent {
    /// No indentation (default)
    #[default]
    Off,
    /// Chinese convention: 2em (2 full-width characters)
    #[serde(rename = "chinese")]
    Chinese,
    /// Japanese convention: 1em (1 full-width character)
    #[serde(rename = "japanese")]
    Japanese,
    /// Custom em value (stored as tenths for precision, e.g., 15 = 1.5em)
    #[serde(rename = "custom")]
    Custom(u8),
}

impl ParagraphIndent {
    /// Get the display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            ParagraphIndent::Off => "Off",
            ParagraphIndent::Chinese => "Chinese (2em)",
            ParagraphIndent::Japanese => "Japanese (1em)",
            ParagraphIndent::Custom(_) => "Custom",
        }
    }

    /// Get a description of the setting.
    pub fn description(&self) -> &'static str {
        match self {
            ParagraphIndent::Off => "No paragraph indentation",
            ParagraphIndent::Chinese => "Two full-width characters indent",
            ParagraphIndent::Japanese => "One full-width character indent",
            ParagraphIndent::Custom(_) => "Custom em value",
        }
    }

    /// Get all preset options (excludes Custom).
    pub fn presets() -> &'static [ParagraphIndent] {
        &[
            ParagraphIndent::Off,
            ParagraphIndent::Chinese,
            ParagraphIndent::Japanese,
        ]
    }

    /// Get the indentation value in em units.
    ///
    /// Returns `None` for `Off` (no indentation).
    pub fn to_em(&self) -> Option<f32> {
        match self {
            ParagraphIndent::Off => None,
            ParagraphIndent::Chinese => Some(2.0),
            ParagraphIndent::Japanese => Some(1.0),
            ParagraphIndent::Custom(tenths) => Some(*tenths as f32 / 10.0),
        }
    }

    /// Get the indentation value in pixels given a font size.
    ///
    /// Returns `None` for `Off` (no indentation).
    pub fn to_pixels(&self, font_size: f32) -> Option<f32> {
        self.to_em().map(|em| em * font_size)
    }

    /// Check if this is a custom setting.
    pub fn is_custom(&self) -> bool {
        matches!(self, ParagraphIndent::Custom(_))
    }

    /// Get the custom value in tenths of em, if any.
    pub fn custom_value(&self) -> Option<u8> {
        if let ParagraphIndent::Custom(tenths) = self {
            Some(*tenths)
        } else {
            None
        }
    }

    /// Get CSS text-indent value string.
    ///
    /// Returns `None` for `Off`.
    pub fn to_css(&self) -> Option<String> {
        self.to_em().map(|em| format!("{}em", em))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Maximum Line Width Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum line width setting for the editor.
///
/// Controls the maximum width of text content in the editor. When enabled,
/// text is constrained to the specified width and centered in the viewport.
/// This applies to Raw, Rendered, Split, and Zen mode views.
/// Reference: GitHub Issue #15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MaxLineWidth {
    /// No width limit (current behavior)
    #[default]
    Off,
    /// 80 characters (traditional terminal width)
    #[serde(rename = "80")]
    Col80,
    /// 100 characters (comfortable reading width)
    #[serde(rename = "100")]
    Col100,
    /// 120 characters (wide monitors)
    #[serde(rename = "120")]
    Col120,
    /// Custom pixel width
    #[serde(rename = "custom")]
    Custom(u32),
}

impl MaxLineWidth {
    /// Get the display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            MaxLineWidth::Off => "Off",
            MaxLineWidth::Col80 => "80 characters",
            MaxLineWidth::Col100 => "100 characters",
            MaxLineWidth::Col120 => "120 characters",
            MaxLineWidth::Custom(_) => "Custom",
        }
    }

    /// Get a description of the setting.
    pub fn description(&self) -> &'static str {
        match self {
            MaxLineWidth::Off => "No width limit",
            MaxLineWidth::Col80 => "Traditional terminal width",
            MaxLineWidth::Col100 => "Comfortable reading width",
            MaxLineWidth::Col120 => "Wide monitor width",
            MaxLineWidth::Custom(_) => "Custom pixel width",
        }
    }

    /// Get all preset width options (excludes Custom).
    pub fn presets() -> &'static [MaxLineWidth] {
        &[
            MaxLineWidth::Off,
            MaxLineWidth::Col80,
            MaxLineWidth::Col100,
            MaxLineWidth::Col120,
        ]
    }

    /// Convert to pixel width given a font's approximate character width.
    ///
    /// Returns `None` for `Off` (no limit).
    pub fn to_pixels(&self, char_width: f32) -> Option<f32> {
        match self {
            MaxLineWidth::Off => None,
            MaxLineWidth::Col80 => Some(char_width * 80.0),
            MaxLineWidth::Col100 => Some(char_width * 100.0),
            MaxLineWidth::Col120 => Some(char_width * 120.0),
            MaxLineWidth::Custom(px) => Some(*px as f32),
        }
    }

    /// Check if this is a custom width setting.
    pub fn is_custom(&self) -> bool {
        matches!(self, MaxLineWidth::Custom(_))
    }

    /// Get the custom pixel value, if any.
    pub fn custom_value(&self) -> Option<u32> {
        if let MaxLineWidth::Custom(px) = self {
            Some(*px)
        } else {
            None
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Log Level Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Available log levels for controlling runtime log filtering.
///
/// Controls the verbosity of log output. Default is `Warn`.
/// Reference: GitHub Issue #11
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Most verbose - shows all debug messages
    Debug,
    /// Informational messages and above
    Info,
    /// Warnings and errors only (default)
    #[default]
    Warn,
    /// Errors only
    Error,
    /// Disable all logging
    Off,
}

impl LogLevel {
    /// Get the display name for the log level.
    pub fn display_name(&self) -> &'static str {
        match self {
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Warn => "Warn",
            LogLevel::Error => "Error",
            LogLevel::Off => "Off",
        }
    }

    /// Get a description of the log level.
    pub fn description(&self) -> &'static str {
        match self {
            LogLevel::Debug => "Most verbose, shows all debug messages",
            LogLevel::Info => "Informational messages and above",
            LogLevel::Warn => "Warnings and errors only (default)",
            LogLevel::Error => "Errors only",
            LogLevel::Off => "Disable all logging",
        }
    }

    /// Get all available log levels.
    pub fn all() -> &'static [LogLevel] {
        &[
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Off,
        ]
    }

    /// Convert to log::LevelFilter for env_logger initialization.
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Off => log::LevelFilter::Off,
        }
    }
}

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorFont {
    /// Inter - Modern, clean UI font (default)
    #[default]
    Inter,
    /// JetBrains Mono - Monospace font, good for code-heavy documents
    JetBrainsMono,
    /// Custom system font selected by user
    #[serde(rename = "custom")]
    Custom(String),
}

impl EditorFont {
    /// Get the display name for the font.
    pub fn display_name(&self) -> String {
        match self {
            EditorFont::Inter => "Inter".to_string(),
            EditorFont::JetBrainsMono => "JetBrains Mono".to_string(),
            EditorFont::Custom(name) => name.clone(),
        }
    }

    /// Get a description of the font.
    pub fn description(&self) -> &'static str {
        match self {
            EditorFont::Inter => "Modern, clean proportional font",
            EditorFont::JetBrainsMono => "Monospace font for code",
            EditorFont::Custom(_) => "Custom system font",
        }
    }

    /// Get the built-in font options (excluding Custom).
    pub fn builtin_fonts() -> &'static [EditorFont] {
        // Note: We use a static slice, so Custom can't be included here
        &[EditorFont::Inter, EditorFont::JetBrainsMono]
    }

    /// Check if this is a custom font.
    pub fn is_custom(&self) -> bool {
        matches!(self, EditorFont::Custom(_))
    }

    /// Get the custom font name if this is a custom font.
    pub fn custom_name(&self) -> Option<&str> {
        match self {
            EditorFont::Custom(name) => Some(name),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CJK Font Preference Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// CJK font preference for regional glyph variants.
///
/// Different CJK regions use different glyph variants for the same Unicode
/// code points. This setting controls which regional font takes priority
/// in the font fallback chain.
///
/// Reference: GitHub Issue #15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CjkFontPreference {
    /// Auto-detect based on system locale (default)
    #[default]
    Auto,
    /// Korean (Hangul) - Malgun Gothic, Apple SD Gothic Neo, NanumGothic
    Korean,
    /// Simplified Chinese - Microsoft YaHei, PingFang SC, Noto Sans CJK SC
    SimplifiedChinese,
    /// Traditional Chinese - Microsoft JhengHei, PingFang TC, Noto Sans CJK TC
    TraditionalChinese,
    /// Japanese - Yu Gothic, Hiragino Sans, Meiryo
    Japanese,
}

impl CjkFontPreference {
    /// Get the display name for the preference.
    pub fn display_name(&self) -> &'static str {
        match self {
            CjkFontPreference::Auto => "Auto (System Locale)",
            CjkFontPreference::Korean => "Korean (한국어)",
            CjkFontPreference::SimplifiedChinese => "Simplified Chinese (简体中文)",
            CjkFontPreference::TraditionalChinese => "Traditional Chinese (繁體中文)",
            CjkFontPreference::Japanese => "Japanese (日本語)",
        }
    }

    /// Get a description of the preference.
    pub fn description(&self) -> &'static str {
        match self {
            CjkFontPreference::Auto => "Use system locale to determine CJK font priority",
            CjkFontPreference::Korean => "Prioritize Korean glyph variants",
            CjkFontPreference::SimplifiedChinese => "Prioritize Simplified Chinese glyph variants",
            CjkFontPreference::TraditionalChinese => "Prioritize Traditional Chinese glyph variants",
            CjkFontPreference::Japanese => "Prioritize Japanese glyph variants",
        }
    }

    /// Get all available preferences.
    pub fn all() -> &'static [CjkFontPreference] {
        &[
            CjkFontPreference::Auto,
            CjkFontPreference::Korean,
            CjkFontPreference::SimplifiedChinese,
            CjkFontPreference::TraditionalChinese,
            CjkFontPreference::Japanese,
        ]
    }

    /// Get the font family order based on preference.
    ///
    /// Returns the CJK font keys in priority order for the font fallback chain.
    pub fn font_order(&self) -> &'static [&'static str] {
        match self {
            CjkFontPreference::Auto => &["CJK_KR", "CJK_SC", "CJK_TC", "CJK_JP"],
            CjkFontPreference::Korean => &["CJK_KR", "CJK_SC", "CJK_TC", "CJK_JP"],
            CjkFontPreference::SimplifiedChinese => &["CJK_SC", "CJK_TC", "CJK_KR", "CJK_JP"],
            CjkFontPreference::TraditionalChinese => &["CJK_TC", "CJK_SC", "CJK_KR", "CJK_JP"],
            CjkFontPreference::Japanese => &["CJK_JP", "CJK_KR", "CJK_SC", "CJK_TC"],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// View Mode Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Editor view modes for markdown editing.
///
/// Three modes are available:
/// - `Raw`: Plain markdown text editing using a standard text editor
/// - `Rendered`: WYSIWYG editing with rendered markdown elements
/// - `Split`: Side-by-side split view with raw editor on left and preview on right
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Raw markdown text editing (plain TextEdit)
    #[default]
    Raw,
    /// WYSIWYG rendered editing (MarkdownEditor)
    Rendered,
    /// Split view: raw editor (left) + rendered preview (right)
    Split,
}

impl ViewMode {
    /// Cycle through view modes: Raw → Split → Rendered → Raw
    pub fn toggle(&self) -> Self {
        match self {
            ViewMode::Raw => ViewMode::Split,
            ViewMode::Split => ViewMode::Rendered,
            ViewMode::Rendered => ViewMode::Raw,
        }
    }

    /// Get a display label for the mode.
    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::Raw => "Raw",
            ViewMode::Rendered => "Rendered",
            ViewMode::Split => "Split",
        }
    }

    /// Get an icon/symbol for the mode.
    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            ViewMode::Raw => "📝",
            ViewMode::Rendered => "👁",
            ViewMode::Split => "⫿",
        }
    }

    /// Check if this mode shows the raw editor.
    pub fn shows_raw(&self) -> bool {
        matches!(self, ViewMode::Raw | ViewMode::Split)
    }

    /// Check if this mode shows the rendered preview.
    pub fn shows_rendered(&self) -> bool {
        matches!(self, ViewMode::Rendered | ViewMode::Split)
    }

    /// Get all available view modes.
    pub fn all() -> &'static [ViewMode] {
        &[ViewMode::Raw, ViewMode::Rendered, ViewMode::Split]
    }

    /// Get a description of the view mode.
    pub fn description(&self) -> &'static str {
        match self {
            ViewMode::Raw => "Plain markdown text editing",
            ViewMode::Rendered => "WYSIWYG rendered editing",
            ViewMode::Split => "Raw editor + rendered preview side by side",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Mode Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Minimap display mode for the editor.
///
/// Controls which type of minimap is displayed:
/// - `Auto`: Semantic for markdown files, pixel for others (default)
/// - `Semantic`: Always show semantic minimap (heading/structure overview)
/// - `Pixel`: Always show pixel minimap (code overview)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MinimapMode {
    /// Automatic: semantic for markdown, pixel for code files (default)
    #[default]
    Auto,
    /// Always use semantic minimap (structure-based with headings)
    Semantic,
    /// Always use pixel minimap (code overview)
    Pixel,
}

impl MinimapMode {
    /// Get the display name for the minimap mode.
    pub fn display_name(&self) -> &'static str {
        match self {
            MinimapMode::Auto => "Auto",
            MinimapMode::Semantic => "Semantic",
            MinimapMode::Pixel => "Pixel",
        }
    }

    /// Get a description of the minimap mode.
    pub fn description(&self) -> &'static str {
        match self {
            MinimapMode::Auto => "Semantic for markdown, pixel for code",
            MinimapMode::Semantic => "Structure-based with headings and sections",
            MinimapMode::Pixel => "Code overview with character rendering",
        }
    }

    /// Get all available minimap modes.
    pub fn all() -> &'static [MinimapMode] {
        &[MinimapMode::Auto, MinimapMode::Semantic, MinimapMode::Pixel]
    }

    /// Determine if semantic minimap should be used based on mode and file type.
    ///
    /// # Arguments
    /// * `is_markdown` - Whether the current file is a markdown file
    ///
    /// # Returns
    /// `true` if semantic minimap should be used, `false` for pixel minimap
    pub fn use_semantic(&self, is_markdown: bool) -> bool {
        match self {
            MinimapMode::Auto => is_markdown,
            MinimapMode::Semantic => true,
            MinimapMode::Pixel => false,
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
    /// View mode for this tab (raw, rendered, or split)
    #[serde(default)]
    pub view_mode: ViewMode,
    /// Split view ratio (0.0 to 1.0, where ratio is the proportion for the left pane)
    /// Default is 0.5 (50/50 split). Only used when view_mode is Split.
    #[serde(default = "default_split_ratio")]
    pub split_ratio: f32,
}

/// Default split ratio for TabInfo (50/50 split)
fn default_split_ratio() -> f32 {
    0.5
}

impl Default for TabInfo {
    fn default() -> Self {
        Self {
            path: None,
            modified: false,
            cursor_position: (0, 0),
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw, // New documents default to raw mode
            split_ratio: 0.5,         // Default to 50/50 split
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

    /// CJK font preference for regional glyph variants.
    /// Controls which CJK font takes priority in the fallback chain.
    /// Important for users who need specific regional glyph variants.
    /// Reference: GitHub Issue #15
    pub cjk_font_preference: CjkFontPreference,

    // ─────────────────────────────────────────────────────────────────────────
    // Editor Behavior
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to enable word wrap
    pub word_wrap: bool,

    /// Tab size (number of spaces)
    pub tab_size: u8,

    /// Whether to use spaces instead of tabs
    pub use_spaces: bool,

    /// Default auto-save state for new tabs/documents
    /// When true, new documents will have auto-save enabled by default
    pub auto_save_enabled_default: bool,

    /// Auto-save delay in milliseconds after last edit before triggering save
    /// Uses temp-file based strategy to avoid overwriting main file prematurely
    /// Default is 15000ms (15 seconds)
    pub auto_save_delay_ms: u32,

    // ─────────────────────────────────────────────────────────────────────────
    // Session & History
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to restore the previous session on startup.
    /// When true, restores previously open tabs from the last session.
    /// When false, starts with a single empty tab.
    /// Session data is always saved regardless of this setting, so toggling
    /// it back on will restore the previous session.
    pub restore_session: bool,

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

    // ─────────────────────────────────────────────────────────────────────────
    // Zen Mode Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Maximum column width for Zen Mode (in characters, approx 70-90)
    pub zen_max_column_width: f32,

    /// Whether Zen Mode was enabled in the last session (for restore)
    pub zen_mode_enabled: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Code Folding Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether code folding is enabled globally
    pub folding_enabled: bool,

    /// Whether to show fold indicators in the gutter
    pub folding_show_indicators: bool,

    /// Whether to fold Markdown headings
    pub fold_headings: bool,

    /// Whether to fold fenced code blocks
    pub fold_code_blocks: bool,

    /// Whether to fold list hierarchies
    pub fold_lists: bool,

    /// Whether to use indentation-based folding for JSON/YAML
    pub fold_indentation: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Live Pipeline Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether the Live Pipeline feature is enabled (for JSON/YAML files)
    pub pipeline_enabled: bool,

    /// Debounce delay in milliseconds before auto-executing pipeline command
    pub pipeline_debounce_ms: u32,

    /// Maximum output size in bytes (to prevent memory issues)
    pub pipeline_max_output_bytes: u32,

    /// Maximum runtime in milliseconds before killing the process
    pub pipeline_max_runtime_ms: u32,

    /// Height of the pipeline panel in pixels
    pub pipeline_panel_height: f32,

    /// Recent pipeline commands (persisted across sessions)
    pub pipeline_recent_commands: Vec<String>,

    // ─────────────────────────────────────────────────────────────────────────
    // Minimap Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether the minimap is enabled
    pub minimap_enabled: bool,

    /// Width of the minimap in pixels
    pub minimap_width: f32,

    /// Minimap display mode (Auto, Semantic, or Pixel)
    /// - Auto: Semantic for markdown files, pixel for code files (default)
    /// - Semantic: Always show structure-based minimap with headings
    /// - Pixel: Always show code overview minimap
    pub minimap_mode: MinimapMode,

    // ─────────────────────────────────────────────────────────────────────────
    // Bracket Matching Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to highlight matching brackets and emphasis pairs when cursor is adjacent
    /// Supports (), [], {}, <>, and markdown emphasis ** and __
    pub highlight_matching_pairs: bool,

    /// Whether to automatically insert closing brackets and quotes when typing openers.
    /// When enabled:
    /// - Typing `(`, `[`, `{`, `"`, `'`, or `` ` `` inserts the closing pair
    /// - If text is selected, wraps the selection with the pair
    /// - Typing a closer when the next character is the same closer skips over it
    pub auto_close_brackets: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Syntax Highlighting Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to enable syntax highlighting for source code files in raw editor mode
    /// Supports Rust, Python, JavaScript, TypeScript, and many other languages
    pub syntax_highlighting_enabled: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Maximum Line Width Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Maximum line width for text content in the editor.
    /// When enabled, constrains text width and centers the text column.
    /// Applies to Raw, Rendered, Split, and Zen mode views.
    /// Reference: GitHub Issue #15
    pub max_line_width: MaxLineWidth,

    // ─────────────────────────────────────────────────────────────────────────
    // Logging Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Log level for controlling runtime log verbosity.
    /// Default is Warn. Can be overridden via --log-level CLI flag.
    /// Reference: GitHub Issue #11
    pub log_level: LogLevel,

    // ─────────────────────────────────────────────────────────────────────────
    // Default View Mode
    // ─────────────────────────────────────────────────────────────────────────
    /// Default view mode for new tabs.
    /// Controls whether new tabs open in Raw, Rendered, or Split view.
    /// Existing tabs retain their stored view mode (not overridden by this setting).
    /// Reference: GitHub Issue #3
    pub default_view_mode: ViewMode,

    // ─────────────────────────────────────────────────────────────────────────
    // Language Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// UI language for the application.
    /// Changes take effect immediately when selected in Settings.
    /// Persisted to config.json and loaded on startup.
    pub language: Language,

    // ─────────────────────────────────────────────────────────────────────────
    // CSV Viewer Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to apply subtle rainbow coloring to CSV columns.
    /// Uses perceptually uniform colors (Oklch) that work in both light and dark themes.
    /// Each column gets a slightly different background hue for easier visual tracking.
    pub csv_rainbow_columns: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // CJK Paragraph Indentation Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Paragraph first-line indentation for CJK typography conventions.
    /// Chinese convention uses 2em (two full-width characters).
    /// Japanese convention uses 1em (one full-width character).
    /// Applies to Rendered/Preview mode and HTML export.
    /// Reference: GitHub Issue #20
    pub paragraph_indent: ParagraphIndent,

    // ─────────────────────────────────────────────────────────────────────────
    // Snippets Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether snippet expansion is enabled.
    /// When enabled, typing a trigger word followed by space/tab expands it.
    /// Built-in snippets: ;date, ;time, ;datetime, ;now
    /// Custom snippets can be added via ~/.config/ferrite/snippets.json
    pub snippets_enabled: bool,

    // ─────────────────────────────────────────────────────────────────────────
    // Keyboard Shortcuts Settings
    // ─────────────────────────────────────────────────────────────────────────
    /// Custom keyboard shortcuts configuration.
    /// Only stores non-default bindings; defaults are used for unset commands.
    /// Reference: GitHub Issue #25
    pub keyboard_shortcuts: KeyboardShortcuts,
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
            cjk_font_preference: CjkFontPreference::default(),

            // Editor Behavior
            word_wrap: true,
            tab_size: 4,
            use_spaces: true,
            auto_save_enabled_default: false,
            auto_save_delay_ms: 15000, // 15 seconds default

            // Session & History
            restore_session: true, // Restore previous session by default
            recent_files: Vec::new(),
            max_recent_files: 20,
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

            // Zen Mode Settings
            zen_max_column_width: 80.0, // ~80 characters default
            zen_mode_enabled: false,

            // Code Folding Settings
            folding_enabled: true,           // Folding enabled by default
            folding_show_indicators: false,  // Hide fold indicators by default (they don't collapse yet)
            fold_headings: true,             // Fold headings by default
            fold_code_blocks: true,          // Fold code blocks by default
            fold_lists: true,                // Fold lists by default
            fold_indentation: true,          // Indentation folding for JSON/YAML

            // Live Pipeline Settings
            pipeline_enabled: true,          // Feature enabled by default
            pipeline_debounce_ms: 500,       // 500ms debounce
            pipeline_max_output_bytes: 1024 * 1024, // 1 MB max output
            pipeline_max_runtime_ms: 30000,  // 30 seconds max runtime
            pipeline_panel_height: 200.0,    // Default panel height
            pipeline_recent_commands: Vec::new(),

            // Minimap Settings
            minimap_enabled: true,           // Minimap enabled by default
            minimap_width: 120.0,            // Default semantic minimap width
            minimap_mode: MinimapMode::default(), // Auto mode by default

            // Bracket Matching Settings
            highlight_matching_pairs: true,  // Bracket matching enabled by default
            auto_close_brackets: true,       // Auto-close brackets enabled by default

            // Syntax Highlighting Settings
            syntax_highlighting_enabled: true, // Syntax highlighting enabled by default

            // Maximum Line Width Settings
            max_line_width: MaxLineWidth::default(), // Off by default (no limit)

            // Logging Settings
            log_level: LogLevel::default(), // Default to Warn level

            // Default View Mode
            default_view_mode: ViewMode::default(), // Default to Raw mode

            // Language Settings
            language: Language::default(), // Default to English

            // CSV Viewer Settings
            csv_rainbow_columns: false, // Disabled by default for clean look

            // CJK Paragraph Indentation Settings
            paragraph_indent: ParagraphIndent::default(), // Off by default

            // Snippets Settings
            snippets_enabled: true, // Snippet expansion enabled by default

            // Keyboard Shortcuts Settings
            keyboard_shortcuts: KeyboardShortcuts::default(),
        }
    }
}

impl Settings {
    /// Create default settings with system locale detection.
    ///
    /// This should only be called on first run (when no config file exists).
    /// It detects the system locale and sets the language accordingly,
    /// falling back to English if the locale is not supported.
    ///
    /// For subsequent runs, use `Settings::default()` or load from config
    /// to respect the user's saved preference.
    pub fn default_with_system_locale() -> Self {
        let detected_language = Language::from_system_locale();
        log::info!(
            "First run: detected system language as {} ({})",
            detected_language.native_name(),
            detected_language.locale_code()
        );
        Self {
            language: detected_language,
            ..Self::default()
        }
    }

    /// Add a file to the recent files list.
    ///
    /// If the file already exists in the list, it's moved to the front.
    /// The list is trimmed to `max_recent_files`.
    /// The path is normalized to remove Windows verbatim prefixes (\\?\).
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Normalize path to remove Windows \\?\ prefix
        let path = crate::path_utils::normalize_path(path);
        // Remove if already exists (check both normalized and original forms)
        self.recent_files
            .retain(|p| crate::path_utils::normalize_path(p.clone()) != path);
        // Add to front
        self.recent_files.insert(0, path);
        // Trim to max
        self.recent_files.truncate(self.max_recent_files);
    }

    /// Prune recent files that no longer exist on disk.
    ///
    /// Returns the number of files removed from the list.
    pub fn prune_stale_recent_files(&mut self) -> usize {
        let original_len = self.recent_files.len();
        self.recent_files.retain(|p| p.exists());
        let removed = original_len - self.recent_files.len();
        if removed > 0 {
            log::debug!("Pruned {} stale recent files", removed);
        }
        removed
    }

    /// Add a workspace (folder) to the recent workspaces list.
    ///
    /// If the workspace already exists in the list, it's moved to the front.
    /// The list is trimmed to `max_recent_workspaces`.
    /// The path is normalized to remove Windows verbatim prefixes (\\?\).
    pub fn add_recent_workspace(&mut self, path: PathBuf) {
        // Normalize path to remove Windows \\?\ prefix
        let path = crate::path_utils::normalize_path(path);
        // Remove if already exists (check both normalized and original forms)
        self.recent_workspaces
            .retain(|p| crate::path_utils::normalize_path(p.clone()) != path);
        // Add to front
        self.recent_workspaces.insert(0, path);
        // Trim to max
        self.recent_workspaces.truncate(self.max_recent_workspaces);
    }

    /// Prune recent workspaces that no longer exist on disk.
    ///
    /// Returns the number of workspaces removed from the list.
    pub fn prune_stale_recent_workspaces(&mut self) -> usize {
        let original_len = self.recent_workspaces.len();
        self.recent_workspaces.retain(|p| p.exists() && p.is_dir());
        let removed = original_len - self.recent_workspaces.len();
        if removed > 0 {
            log::debug!("Pruned {} stale recent workspaces", removed);
        }
        removed
    }

    /// Normalize all stored paths to remove Windows verbatim prefixes (\\?\).
    ///
    /// This fixes paths that were stored with the \\?\ prefix from previous
    /// versions that used canonicalize() without normalization. Also deduplicates
    /// paths that exist both with and without the prefix.
    ///
    /// Returns the number of paths that were normalized.
    pub fn normalize_stored_paths(&mut self) -> usize {
        let mut count = 0;

        // Normalize recent files and deduplicate
        let mut normalized_files: Vec<PathBuf> = Vec::new();
        for path in self.recent_files.drain(..) {
            let normalized = crate::path_utils::normalize_path(path.clone());
            if normalized != path {
                count += 1;
            }
            // Only add if not already in list (handles duplicates from mixed paths)
            if !normalized_files.contains(&normalized) {
                normalized_files.push(normalized);
            }
        }
        self.recent_files = normalized_files;

        // Normalize recent workspaces and deduplicate
        let mut normalized_workspaces: Vec<PathBuf> = Vec::new();
        for path in self.recent_workspaces.drain(..) {
            let normalized = crate::path_utils::normalize_path(path.clone());
            if normalized != path {
                count += 1;
            }
            // Only add if not already in list (handles duplicates from mixed paths)
            if !normalized_workspaces.contains(&normalized) {
                normalized_workspaces.push(normalized);
            }
        }
        self.recent_workspaces = normalized_workspaces;

        // Normalize tab paths
        for tab in &mut self.last_open_tabs {
            if let Some(path) = &tab.path {
                let normalized = crate::path_utils::normalize_path(path.clone());
                if &normalized != path {
                    tab.path = Some(normalized);
                    count += 1;
                }
            }
        }

        if count > 0 {
            log::debug!("Normalized {} paths (removed \\\\?\\ prefixes)", count);
        }

        count
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
    /// Minimum Zen Mode column width (characters).
    pub const MIN_ZEN_COLUMN_WIDTH: f32 = 50.0;
    /// Maximum Zen Mode column width (characters).
    pub const MAX_ZEN_COLUMN_WIDTH: f32 = 120.0;
    /// Minimum pipeline debounce in milliseconds.
    pub const MIN_PIPELINE_DEBOUNCE_MS: u32 = 100;
    /// Maximum pipeline debounce in milliseconds.
    pub const MAX_PIPELINE_DEBOUNCE_MS: u32 = 5000;
    /// Minimum pipeline output size in bytes (1 KB).
    pub const MIN_PIPELINE_OUTPUT_BYTES: u32 = 1024;
    /// Maximum pipeline output size in bytes (10 MB).
    pub const MAX_PIPELINE_OUTPUT_BYTES: u32 = 10 * 1024 * 1024;
    /// Minimum pipeline runtime in milliseconds (1 second).
    pub const MIN_PIPELINE_RUNTIME_MS: u32 = 1000;
    /// Maximum pipeline runtime in milliseconds (5 minutes).
    pub const MAX_PIPELINE_RUNTIME_MS: u32 = 300000;
    /// Minimum pipeline panel height.
    pub const MIN_PIPELINE_PANEL_HEIGHT: f32 = 100.0;
    /// Maximum pipeline panel height.
    pub const MAX_PIPELINE_PANEL_HEIGHT: f32 = 500.0;
    /// Maximum number of recent pipeline commands.
    pub const MAX_PIPELINE_RECENT_COMMANDS: usize = 20;
    /// Minimum minimap width.
    pub const MIN_MINIMAP_WIDTH: f32 = 80.0;
    /// Maximum minimap width.
    pub const MAX_MINIMAP_WIDTH: f32 = 200.0;
    /// Minimum custom line width in pixels.
    pub const MIN_CUSTOM_LINE_WIDTH: u32 = 400;
    /// Maximum custom line width in pixels.
    pub const MAX_CUSTOM_LINE_WIDTH: u32 = 2000;

    /// Sanitize settings by clamping values to valid ranges.
    ///
    /// This is useful after loading settings from a file that might have
    /// been manually edited with invalid values.
    pub fn sanitize(&mut self) {
        // Normalize stored paths (removes Windows \\?\ prefixes and deduplicates)
        self.normalize_stored_paths();

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

        // Ensure auto-save delay is reasonable (minimum 5 seconds, max 5 minutes)
        self.auto_save_delay_ms = self.auto_save_delay_ms.clamp(5000, 300000);

        // Ensure active_tab_index is valid
        if !self.last_open_tabs.is_empty() && self.active_tab_index >= self.last_open_tabs.len() {
            self.active_tab_index = self.last_open_tabs.len() - 1;
        }

        // Clamp outline width
        self.outline_width = self
            .outline_width
            .clamp(Self::MIN_OUTLINE_WIDTH, Self::MAX_OUTLINE_WIDTH);

        // Clamp Zen Mode column width
        self.zen_max_column_width = self
            .zen_max_column_width
            .clamp(Self::MIN_ZEN_COLUMN_WIDTH, Self::MAX_ZEN_COLUMN_WIDTH);

        // Clamp pipeline settings
        self.pipeline_debounce_ms = self
            .pipeline_debounce_ms
            .clamp(Self::MIN_PIPELINE_DEBOUNCE_MS, Self::MAX_PIPELINE_DEBOUNCE_MS);
        self.pipeline_max_output_bytes = self
            .pipeline_max_output_bytes
            .clamp(Self::MIN_PIPELINE_OUTPUT_BYTES, Self::MAX_PIPELINE_OUTPUT_BYTES);
        self.pipeline_max_runtime_ms = self
            .pipeline_max_runtime_ms
            .clamp(Self::MIN_PIPELINE_RUNTIME_MS, Self::MAX_PIPELINE_RUNTIME_MS);
        self.pipeline_panel_height = self
            .pipeline_panel_height
            .clamp(Self::MIN_PIPELINE_PANEL_HEIGHT, Self::MAX_PIPELINE_PANEL_HEIGHT);
        self.pipeline_recent_commands
            .truncate(Self::MAX_PIPELINE_RECENT_COMMANDS);

        // Clamp minimap width
        self.minimap_width = self
            .minimap_width
            .clamp(Self::MIN_MINIMAP_WIDTH, Self::MAX_MINIMAP_WIDTH);

        // Clamp custom line width if set
        if let MaxLineWidth::Custom(px) = self.max_line_width {
            let clamped = px.clamp(Self::MIN_CUSTOM_LINE_WIDTH, Self::MAX_CUSTOM_LINE_WIDTH);
            self.max_line_width = MaxLineWidth::Custom(clamped);
        }
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
        assert_eq!(settings.max_recent_files, 20);
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
    fn test_prune_stale_recent_files() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();

        // Create a real temp file that exists
        let existing_file = temp_dir.join("test_prune_existing.md");
        {
            let mut file = std::fs::File::create(&existing_file).unwrap();
            writeln!(file, "test content").unwrap();
        }

        // Create a path that doesn't exist
        let nonexistent_file = temp_dir.join("nonexistent_file_12345.md");

        let mut settings = Settings::default();
        settings.recent_files = vec![
            existing_file.clone(),
            nonexistent_file.clone(),
            PathBuf::from("/definitely/does/not/exist.md"),
        ];

        assert_eq!(settings.recent_files.len(), 3);

        // Prune stale files
        let removed = settings.prune_stale_recent_files();

        // Should have removed 2 non-existent files
        assert_eq!(removed, 2);
        assert_eq!(settings.recent_files.len(), 1);
        assert_eq!(settings.recent_files[0], existing_file);

        // Cleanup
        let _ = std::fs::remove_file(&existing_file);
    }

    #[test]
    fn test_prune_stale_recent_files_empty_list() {
        let mut settings = Settings::default();
        settings.recent_files = Vec::new();

        let removed = settings.prune_stale_recent_files();
        assert_eq!(removed, 0);
        assert!(settings.recent_files.is_empty());
    }

    #[test]
    fn test_prune_stale_recent_workspaces() {
        let temp_dir = std::env::temp_dir();

        // Create a real temp directory that exists
        let existing_dir = temp_dir.join("test_prune_workspace_existing");
        std::fs::create_dir_all(&existing_dir).unwrap();

        // Create a path that doesn't exist
        let nonexistent_dir = temp_dir.join("nonexistent_workspace_12345");

        let mut settings = Settings::default();
        settings.recent_workspaces = vec![
            existing_dir.clone(),
            nonexistent_dir.clone(),
            PathBuf::from("/definitely/does/not/exist/folder"),
        ];

        assert_eq!(settings.recent_workspaces.len(), 3);

        // Prune stale workspaces
        let removed = settings.prune_stale_recent_workspaces();

        // Should have removed 2 non-existent folders
        assert_eq!(removed, 2);
        assert_eq!(settings.recent_workspaces.len(), 1);
        assert_eq!(settings.recent_workspaces[0], existing_dir);

        // Cleanup
        let _ = std::fs::remove_dir(&existing_dir);
    }

    #[test]
    fn test_prune_stale_recent_workspaces_empty_list() {
        let mut settings = Settings::default();
        settings.recent_workspaces = Vec::new();

        let removed = settings.prune_stale_recent_workspaces();
        assert_eq!(removed, 0);
        assert!(settings.recent_workspaces.is_empty());
    }

    #[test]
    fn test_prune_stale_recent_workspaces_file_not_dir() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();

        // Create a file (not a directory) - should be pruned
        let file_path = temp_dir.join("test_prune_workspace_file.txt");
        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            writeln!(file, "test").unwrap();
        }

        let mut settings = Settings::default();
        settings.recent_workspaces = vec![file_path.clone()];

        // Should prune because it's a file, not a directory
        let removed = settings.prune_stale_recent_workspaces();
        assert_eq!(removed, 1);
        assert!(settings.recent_workspaces.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
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
        assert_eq!(
            serde_json::to_string(&ViewMode::Split).unwrap(),
            "\"split\""
        );
    }

    #[test]
    fn test_view_mode_toggle() {
        // Raw → Split → Rendered → Raw
        assert_eq!(ViewMode::Raw.toggle(), ViewMode::Split);
        assert_eq!(ViewMode::Split.toggle(), ViewMode::Rendered);
        assert_eq!(ViewMode::Rendered.toggle(), ViewMode::Raw);
    }

    #[test]
    fn test_view_mode_labels() {
        assert_eq!(ViewMode::Raw.label(), "Raw");
        assert_eq!(ViewMode::Rendered.label(), "Rendered");
        assert_eq!(ViewMode::Split.label(), "Split");
        assert_eq!(ViewMode::Raw.icon(), "📝");
        assert_eq!(ViewMode::Rendered.icon(), "👁");
        assert_eq!(ViewMode::Split.icon(), "⫿");
    }

    #[test]
    fn test_view_mode_shows_raw_rendered() {
        assert!(ViewMode::Raw.shows_raw());
        assert!(!ViewMode::Raw.shows_rendered());
        assert!(!ViewMode::Rendered.shows_raw());
        assert!(ViewMode::Rendered.shows_rendered());
        assert!(ViewMode::Split.shows_raw());
        assert!(ViewMode::Split.shows_rendered());
    }

    #[test]
    fn test_view_mode_all() {
        let all = ViewMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ViewMode::Raw));
        assert!(all.contains(&ViewMode::Rendered));
        assert!(all.contains(&ViewMode::Split));
    }

    #[test]
    fn test_view_mode_description() {
        assert!(!ViewMode::Raw.description().is_empty());
        assert!(!ViewMode::Rendered.description().is_empty());
        assert!(!ViewMode::Split.description().is_empty());
        // Ensure descriptions are different
        assert_ne!(ViewMode::Raw.description(), ViewMode::Rendered.description());
        assert_ne!(ViewMode::Raw.description(), ViewMode::Split.description());
    }

    #[test]
    fn test_settings_default_view_mode() {
        let settings = Settings::default();
        assert_eq!(settings.default_view_mode, ViewMode::Raw);
    }

    #[test]
    fn test_settings_backward_compatibility_default_view_mode() {
        // Old JSON without default_view_mode field should default to Raw
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.default_view_mode, ViewMode::Raw);
    }

    #[test]
    fn test_settings_serialize_default_view_mode() {
        let mut settings = Settings::default();
        settings.default_view_mode = ViewMode::Split;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"default_view_mode\":\"split\""));
    }

    #[test]
    fn test_settings_deserialize_default_view_mode() {
        let json = r#"{"default_view_mode": "rendered"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.default_view_mode, ViewMode::Rendered);
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
            split_ratio: 0.6,
        };

        let json = serde_json::to_string(&tab).unwrap();
        let deserialized: TabInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(tab, deserialized);
    }

    #[test]
    fn test_tab_info_default_split_ratio() {
        let tab = TabInfo::default();
        assert_eq!(tab.split_ratio, 0.5); // Default to 50/50 split
    }

    #[test]
    fn test_tab_info_backward_compatibility_split_ratio() {
        // Old JSON without split_ratio field should default to 0.5
        let json = r#"{"path": "/test.md", "modified": false, "cursor_position": [0, 0], "scroll_offset": 0.0, "view_mode": "raw"}"#;
        let tab: TabInfo = serde_json::from_str(json).unwrap();
        assert_eq!(tab.split_ratio, 0.5);
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
    // LogLevel tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_log_level_default() {
        assert_eq!(LogLevel::default(), LogLevel::Warn);
    }

    #[test]
    fn test_log_level_serialization() {
        assert_eq!(serde_json::to_string(&LogLevel::Debug).unwrap(), "\"debug\"");
        assert_eq!(serde_json::to_string(&LogLevel::Info).unwrap(), "\"info\"");
        assert_eq!(serde_json::to_string(&LogLevel::Warn).unwrap(), "\"warn\"");
        assert_eq!(serde_json::to_string(&LogLevel::Error).unwrap(), "\"error\"");
        assert_eq!(serde_json::to_string(&LogLevel::Off).unwrap(), "\"off\"");
    }

    #[test]
    fn test_log_level_deserialization() {
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"debug\"").unwrap(),
            LogLevel::Debug
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"info\"").unwrap(),
            LogLevel::Info
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"warn\"").unwrap(),
            LogLevel::Warn
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"error\"").unwrap(),
            LogLevel::Error
        );
        assert_eq!(
            serde_json::from_str::<LogLevel>("\"off\"").unwrap(),
            LogLevel::Off
        );
    }

    #[test]
    fn test_log_level_display_name() {
        assert_eq!(LogLevel::Debug.display_name(), "Debug");
        assert_eq!(LogLevel::Info.display_name(), "Info");
        assert_eq!(LogLevel::Warn.display_name(), "Warn");
        assert_eq!(LogLevel::Error.display_name(), "Error");
        assert_eq!(LogLevel::Off.display_name(), "Off");
    }

    #[test]
    fn test_log_level_all() {
        let all = LogLevel::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&LogLevel::Debug));
        assert!(all.contains(&LogLevel::Info));
        assert!(all.contains(&LogLevel::Warn));
        assert!(all.contains(&LogLevel::Error));
        assert!(all.contains(&LogLevel::Off));
    }

    #[test]
    fn test_log_level_to_level_filter() {
        assert_eq!(LogLevel::Debug.to_level_filter(), log::LevelFilter::Debug);
        assert_eq!(LogLevel::Info.to_level_filter(), log::LevelFilter::Info);
        assert_eq!(LogLevel::Warn.to_level_filter(), log::LevelFilter::Warn);
        assert_eq!(LogLevel::Error.to_level_filter(), log::LevelFilter::Error);
        assert_eq!(LogLevel::Off.to_level_filter(), log::LevelFilter::Off);
    }

    #[test]
    fn test_settings_log_level_default() {
        let settings = Settings::default();
        assert_eq!(settings.log_level, LogLevel::Warn);
    }

    #[test]
    fn test_settings_backward_compatibility_log_level() {
        // Old JSON without log_level field should default to Warn
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.log_level, LogLevel::Warn);
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

    // ─────────────────────────────────────────────────────────────────────────
    // Code Folding Settings tests (GitHub Issue #12)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_folding_show_indicators_default_false() {
        // Issue #12: Fold indicators are hidden by default because
        // they don't actually collapse yet (visual only)
        let settings = Settings::default();
        assert!(!settings.folding_show_indicators);
        // But folding detection is still enabled
        assert!(settings.folding_enabled);
    }

    #[test]
    fn test_folding_show_indicators_backward_compatibility() {
        // Old settings without folding_show_indicators should get the new default (false)
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(!settings.folding_show_indicators);
    }

    #[test]
    fn test_folding_show_indicators_explicit_true() {
        // Users can still enable it via settings
        let json = r#"{"folding_show_indicators": true}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(settings.folding_show_indicators);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Auto-close Brackets tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_auto_close_brackets_default_true() {
        let settings = Settings::default();
        assert!(settings.auto_close_brackets);
    }

    #[test]
    fn test_auto_close_brackets_backward_compatibility() {
        // Old settings without auto_close_brackets should get the new default (true)
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(settings.auto_close_brackets);
    }

    #[test]
    fn test_auto_close_brackets_explicit_false() {
        // Users can disable it via settings
        let json = r#"{"auto_close_brackets": false}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(!settings.auto_close_brackets);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MaxLineWidth tests (GitHub Issue #15)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_max_line_width_default() {
        assert_eq!(MaxLineWidth::default(), MaxLineWidth::Off);
    }

    #[test]
    fn test_max_line_width_serialization() {
        assert_eq!(serde_json::to_string(&MaxLineWidth::Off).unwrap(), "\"off\"");
        assert_eq!(serde_json::to_string(&MaxLineWidth::Col80).unwrap(), "\"80\"");
        assert_eq!(serde_json::to_string(&MaxLineWidth::Col100).unwrap(), "\"100\"");
        assert_eq!(serde_json::to_string(&MaxLineWidth::Col120).unwrap(), "\"120\"");
        assert_eq!(
            serde_json::to_string(&MaxLineWidth::Custom(600)).unwrap(),
            "{\"custom\":600}"
        );
    }

    #[test]
    fn test_max_line_width_deserialization() {
        assert_eq!(
            serde_json::from_str::<MaxLineWidth>("\"off\"").unwrap(),
            MaxLineWidth::Off
        );
        assert_eq!(
            serde_json::from_str::<MaxLineWidth>("\"80\"").unwrap(),
            MaxLineWidth::Col80
        );
        assert_eq!(
            serde_json::from_str::<MaxLineWidth>("\"100\"").unwrap(),
            MaxLineWidth::Col100
        );
        assert_eq!(
            serde_json::from_str::<MaxLineWidth>("\"120\"").unwrap(),
            MaxLineWidth::Col120
        );
        assert_eq!(
            serde_json::from_str::<MaxLineWidth>("{\"custom\":600}").unwrap(),
            MaxLineWidth::Custom(600)
        );
    }

    #[test]
    fn test_max_line_width_to_pixels() {
        let char_width = 10.0; // Example char width
        assert_eq!(MaxLineWidth::Off.to_pixels(char_width), None);
        assert_eq!(MaxLineWidth::Col80.to_pixels(char_width), Some(800.0));
        assert_eq!(MaxLineWidth::Col100.to_pixels(char_width), Some(1000.0));
        assert_eq!(MaxLineWidth::Col120.to_pixels(char_width), Some(1200.0));
        assert_eq!(MaxLineWidth::Custom(600).to_pixels(char_width), Some(600.0));
    }

    #[test]
    fn test_max_line_width_is_custom() {
        assert!(!MaxLineWidth::Off.is_custom());
        assert!(!MaxLineWidth::Col80.is_custom());
        assert!(!MaxLineWidth::Col100.is_custom());
        assert!(!MaxLineWidth::Col120.is_custom());
        assert!(MaxLineWidth::Custom(600).is_custom());
    }

    #[test]
    fn test_max_line_width_custom_value() {
        assert_eq!(MaxLineWidth::Off.custom_value(), None);
        assert_eq!(MaxLineWidth::Col80.custom_value(), None);
        assert_eq!(MaxLineWidth::Custom(600).custom_value(), Some(600));
    }

    #[test]
    fn test_settings_max_line_width_default() {
        let settings = Settings::default();
        assert_eq!(settings.max_line_width, MaxLineWidth::Off);
    }

    #[test]
    fn test_settings_backward_compatibility_max_line_width() {
        // Old JSON without max_line_width field should default to Off
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.max_line_width, MaxLineWidth::Off);
    }

    #[test]
    fn test_settings_serialize_max_line_width() {
        let mut settings = Settings::default();
        settings.max_line_width = MaxLineWidth::Col80;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"max_line_width\":\"80\""));
    }

    #[test]
    fn test_settings_deserialize_max_line_width() {
        let json = r#"{"max_line_width": "100"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.max_line_width, MaxLineWidth::Col100);
    }

    #[test]
    fn test_settings_sanitize_custom_line_width() {
        // Test clamping of custom line width
        let mut settings = Settings::default();
        
        // Below minimum
        settings.max_line_width = MaxLineWidth::Custom(100);
        settings.sanitize();
        assert_eq!(settings.max_line_width, MaxLineWidth::Custom(Settings::MIN_CUSTOM_LINE_WIDTH));
        
        // Above maximum
        settings.max_line_width = MaxLineWidth::Custom(5000);
        settings.sanitize();
        assert_eq!(settings.max_line_width, MaxLineWidth::Custom(Settings::MAX_CUSTOM_LINE_WIDTH));
        
        // Valid value unchanged
        settings.max_line_width = MaxLineWidth::Custom(800);
        settings.sanitize();
        assert_eq!(settings.max_line_width, MaxLineWidth::Custom(800));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Language tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_language_default() {
        assert_eq!(Language::default(), Language::English);
    }

    #[test]
    fn test_language_serialization() {
        assert_eq!(serde_json::to_string(&Language::English).unwrap(), "\"en\"");
    }

    #[test]
    fn test_language_deserialization() {
        assert_eq!(
            serde_json::from_str::<Language>("\"en\"").unwrap(),
            Language::English
        );
    }

    #[test]
    fn test_language_locale_code() {
        assert_eq!(Language::English.locale_code(), "en");
    }

    #[test]
    fn test_language_native_name() {
        assert_eq!(Language::English.native_name(), "English");
    }

    #[test]
    fn test_language_all() {
        let all = Language::all();
        assert!(!all.is_empty());
        assert!(all.contains(&Language::English));
    }

    #[test]
    fn test_settings_language_default() {
        let settings = Settings::default();
        assert_eq!(settings.language, Language::English);
    }

    #[test]
    fn test_settings_backward_compatibility_language() {
        // Old JSON without language field should default to English
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.language, Language::English);
    }

    #[test]
    fn test_settings_serialize_language() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"language\":\"en\""));
    }

    #[test]
    fn test_settings_deserialize_language() {
        let json = r#"{"language": "en"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.language, Language::English);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // System locale detection tests (Task 4)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_language_from_locale_code_english_variants() {
        // Full locale with hyphen
        assert_eq!(Language::from_locale_code("en-US"), Some(Language::English));
        assert_eq!(Language::from_locale_code("en-GB"), Some(Language::English));
        assert_eq!(Language::from_locale_code("en-AU"), Some(Language::English));

        // Full locale with underscore
        assert_eq!(Language::from_locale_code("en_US"), Some(Language::English));
        assert_eq!(Language::from_locale_code("en_GB"), Some(Language::English));

        // Language only
        assert_eq!(Language::from_locale_code("en"), Some(Language::English));

        // Case-insensitive
        assert_eq!(Language::from_locale_code("EN"), Some(Language::English));
        assert_eq!(Language::from_locale_code("En-Us"), Some(Language::English));
        assert_eq!(Language::from_locale_code("EN_us"), Some(Language::English));
    }

    #[test]
    fn test_language_from_locale_code_unknown() {
        // Unknown locales should return None
        assert_eq!(Language::from_locale_code("unknown"), None);
        assert_eq!(Language::from_locale_code("xx-YY"), None);
        assert_eq!(Language::from_locale_code(""), None);

        // Currently unsupported locales
        assert_eq!(Language::from_locale_code("ko"), None);
        assert_eq!(Language::from_locale_code("fr"), None);
        assert_eq!(Language::from_locale_code("ja"), None);
        assert_eq!(Language::from_locale_code("de"), None);
    }

    #[test]
    fn test_language_from_locale_code_supported_languages() {
        // Chinese Simplified
        assert_eq!(Language::from_locale_code("zh-CN"), Some(Language::ChineseSimplified));
        assert_eq!(Language::from_locale_code("zh_Hans"), Some(Language::ChineseSimplified));
        assert_eq!(Language::from_locale_code("zh"), Some(Language::ChineseSimplified));
    }

    #[test]
    fn test_language_from_system_locale_returns_valid_language() {
        // This test verifies that from_system_locale always returns a valid Language
        // (never panics, always falls back to English if needed)
        let detected = Language::from_system_locale();
        // Should be one of the available languages
        assert!(Language::all().contains(&detected));
    }

    #[test]
    fn test_settings_default_with_system_locale() {
        // Verify default_with_system_locale returns valid settings
        let settings = Settings::default_with_system_locale();

        // Language should be valid (will be English on most test systems)
        assert!(Language::all().contains(&settings.language));

        // All other fields should have their defaults
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.font_size, 14.0);
        assert_eq!(settings.view_mode, ViewMode::Raw);
        assert!(settings.show_line_numbers);
    }

    #[test]
    fn test_settings_default_vs_default_with_system_locale() {
        // Both should return valid settings
        let default_settings = Settings::default();
        let locale_settings = Settings::default_with_system_locale();

        // All fields except language should be identical
        // (language in default_with_system_locale depends on system locale)
        assert_eq!(default_settings.theme, locale_settings.theme);
        assert_eq!(default_settings.font_size, locale_settings.font_size);
        assert_eq!(default_settings.view_mode, locale_settings.view_mode);
        assert_eq!(
            default_settings.show_line_numbers,
            locale_settings.show_line_numbers
        );
        assert_eq!(
            default_settings.max_recent_files,
            locale_settings.max_recent_files
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // CSV Viewer Settings tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_csv_rainbow_columns_default() {
        let settings = Settings::default();
        assert!(!settings.csv_rainbow_columns, "Rainbow columns should be disabled by default");
    }

    #[test]
    fn test_csv_rainbow_columns_backward_compatibility() {
        // Old JSON without csv_rainbow_columns field should default to false
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(!settings.csv_rainbow_columns);
    }

    #[test]
    fn test_csv_rainbow_columns_serialization() {
        let mut settings = Settings::default();
        settings.csv_rainbow_columns = true;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"csv_rainbow_columns\":true"));
    }

    #[test]
    fn test_csv_rainbow_columns_deserialization() {
        let json = r#"{"csv_rainbow_columns": true}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(settings.csv_rainbow_columns);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // MinimapMode tests (Task 19)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_minimap_mode_default() {
        assert_eq!(MinimapMode::default(), MinimapMode::Auto);
    }

    #[test]
    fn test_minimap_mode_serialization() {
        assert_eq!(serde_json::to_string(&MinimapMode::Auto).unwrap(), "\"auto\"");
        assert_eq!(serde_json::to_string(&MinimapMode::Semantic).unwrap(), "\"semantic\"");
        assert_eq!(serde_json::to_string(&MinimapMode::Pixel).unwrap(), "\"pixel\"");
    }

    #[test]
    fn test_minimap_mode_deserialization() {
        assert_eq!(
            serde_json::from_str::<MinimapMode>("\"auto\"").unwrap(),
            MinimapMode::Auto
        );
        assert_eq!(
            serde_json::from_str::<MinimapMode>("\"semantic\"").unwrap(),
            MinimapMode::Semantic
        );
        assert_eq!(
            serde_json::from_str::<MinimapMode>("\"pixel\"").unwrap(),
            MinimapMode::Pixel
        );
    }

    #[test]
    fn test_minimap_mode_display_name() {
        assert_eq!(MinimapMode::Auto.display_name(), "Auto");
        assert_eq!(MinimapMode::Semantic.display_name(), "Semantic");
        assert_eq!(MinimapMode::Pixel.display_name(), "Pixel");
    }

    #[test]
    fn test_minimap_mode_description() {
        // Verify descriptions are non-empty and different
        assert!(!MinimapMode::Auto.description().is_empty());
        assert!(!MinimapMode::Semantic.description().is_empty());
        assert!(!MinimapMode::Pixel.description().is_empty());
        assert_ne!(MinimapMode::Auto.description(), MinimapMode::Semantic.description());
        assert_ne!(MinimapMode::Auto.description(), MinimapMode::Pixel.description());
    }

    #[test]
    fn test_minimap_mode_all() {
        let all = MinimapMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&MinimapMode::Auto));
        assert!(all.contains(&MinimapMode::Semantic));
        assert!(all.contains(&MinimapMode::Pixel));
    }

    #[test]
    fn test_minimap_mode_use_semantic() {
        // Auto mode: semantic for markdown, pixel for others
        assert!(MinimapMode::Auto.use_semantic(true));  // markdown -> semantic
        assert!(!MinimapMode::Auto.use_semantic(false)); // non-markdown -> pixel

        // Semantic mode: always semantic regardless of file type
        assert!(MinimapMode::Semantic.use_semantic(true));
        assert!(MinimapMode::Semantic.use_semantic(false));

        // Pixel mode: always pixel regardless of file type
        assert!(!MinimapMode::Pixel.use_semantic(true));
        assert!(!MinimapMode::Pixel.use_semantic(false));
    }

    #[test]
    fn test_settings_minimap_mode_default() {
        let settings = Settings::default();
        assert_eq!(settings.minimap_mode, MinimapMode::Auto);
    }

    #[test]
    fn test_settings_backward_compatibility_minimap_mode() {
        // Old JSON without minimap_mode field should default to Auto
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.minimap_mode, MinimapMode::Auto);
    }

    #[test]
    fn test_settings_serialize_minimap_mode() {
        let mut settings = Settings::default();
        settings.minimap_mode = MinimapMode::Semantic;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"minimap_mode\":\"semantic\""));
    }

    #[test]
    fn test_settings_deserialize_minimap_mode() {
        let json = r#"{"minimap_mode": "pixel"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.minimap_mode, MinimapMode::Pixel);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Keyboard Shortcuts tests (Task 25)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_key_modifiers_serialization() {
        let mods = KeyModifiers::ctrl_shift();
        let json = serde_json::to_string(&mods).unwrap();
        assert!(json.contains("\"ctrl\":true"));
        assert!(json.contains("\"shift\":true"));
        assert!(json.contains("\"alt\":false"));
    }

    #[test]
    fn test_key_modifiers_deserialization() {
        let json = r#"{"ctrl": true, "shift": false, "alt": true}"#;
        let mods: KeyModifiers = serde_json::from_str(json).unwrap();
        assert!(mods.ctrl);
        assert!(!mods.shift);
        assert!(mods.alt);
    }

    #[test]
    fn test_key_modifiers_display_string() {
        assert_eq!(KeyModifiers::none().display_string(), "");
        
        let ctrl_only = KeyModifiers::ctrl();
        let display = ctrl_only.display_string();
        // On macOS it's "Cmd", on others "Ctrl"
        assert!(display == "Ctrl" || display == "Cmd");
        
        let ctrl_shift = KeyModifiers::ctrl_shift();
        let display = ctrl_shift.display_string();
        assert!(display.contains("Shift"));
    }

    #[test]
    fn test_key_code_serialization() {
        assert_eq!(serde_json::to_string(&KeyCode::S).unwrap(), "\"s\"");
        assert_eq!(serde_json::to_string(&KeyCode::F1).unwrap(), "\"f1\"");
        assert_eq!(serde_json::to_string(&KeyCode::Tab).unwrap(), "\"tab\"");
        assert_eq!(serde_json::to_string(&KeyCode::Num1).unwrap(), "\"num1\"");
    }

    #[test]
    fn test_key_code_deserialization() {
        assert_eq!(serde_json::from_str::<KeyCode>("\"s\"").unwrap(), KeyCode::S);
        assert_eq!(serde_json::from_str::<KeyCode>("\"f11\"").unwrap(), KeyCode::F11);
        assert_eq!(serde_json::from_str::<KeyCode>("\"escape\"").unwrap(), KeyCode::Escape);
    }

    #[test]
    fn test_key_code_display_string() {
        assert_eq!(KeyCode::S.display_string(), "S");
        assert_eq!(KeyCode::F1.display_string(), "F1");
        assert_eq!(KeyCode::Tab.display_string(), "Tab");
        assert_eq!(KeyCode::ArrowUp.display_string(), "↑");
        assert_eq!(KeyCode::Backtick.display_string(), "`");
    }

    #[test]
    fn test_key_binding_display_string() {
        let binding = KeyBinding::new(KeyModifiers::ctrl(), KeyCode::S);
        let display = binding.display_string();
        assert!(display.contains("S"));
        
        let binding_no_mods = KeyBinding::new(KeyModifiers::none(), KeyCode::F11);
        assert_eq!(binding_no_mods.display_string(), "F11");
    }

    #[test]
    fn test_shortcut_command_default_binding() {
        let binding = ShortcutCommand::Save.default_binding();
        assert!(binding.modifiers.ctrl);
        assert!(!binding.modifiers.shift);
        assert_eq!(binding.key, KeyCode::S);

        let binding = ShortcutCommand::SaveAs.default_binding();
        assert!(binding.modifiers.ctrl);
        assert!(binding.modifiers.shift);
        assert_eq!(binding.key, KeyCode::S);

        let binding = ShortcutCommand::ToggleZenMode.default_binding();
        assert!(!binding.modifiers.ctrl);
        assert_eq!(binding.key, KeyCode::F11);
    }

    #[test]
    fn test_shortcut_command_all() {
        let all = ShortcutCommand::all();
        assert!(!all.is_empty());
        assert!(all.contains(&ShortcutCommand::Save));
        assert!(all.contains(&ShortcutCommand::Open));
        assert!(all.contains(&ShortcutCommand::FormatBold));
    }

    #[test]
    fn test_shortcut_command_category() {
        assert_eq!(ShortcutCommand::Save.category(), "File");
        assert_eq!(ShortcutCommand::Find.category(), "Search");
        assert_eq!(ShortcutCommand::FormatBold.category(), "Format");
        assert_eq!(ShortcutCommand::ToggleZenMode.category(), "View");
    }

    #[test]
    fn test_keyboard_shortcuts_default() {
        let shortcuts = KeyboardShortcuts::default();
        // Default shortcuts should return default bindings
        let save_binding = shortcuts.get(ShortcutCommand::Save);
        assert_eq!(save_binding.key, KeyCode::S);
        assert!(save_binding.modifiers.ctrl);
    }

    #[test]
    fn test_keyboard_shortcuts_custom_binding() {
        let mut shortcuts = KeyboardShortcuts::default();
        
        // Set a custom binding
        let custom = KeyBinding::new(KeyModifiers::alt(), KeyCode::S);
        shortcuts.set(ShortcutCommand::Save, custom);
        
        // Check it returns the custom binding
        let binding = shortcuts.get(ShortcutCommand::Save);
        assert!(binding.modifiers.alt);
        assert!(!binding.modifiers.ctrl);
        assert!(shortcuts.is_custom(ShortcutCommand::Save));
    }

    #[test]
    fn test_keyboard_shortcuts_reset() {
        let mut shortcuts = KeyboardShortcuts::default();
        
        // Set a custom binding
        shortcuts.set(ShortcutCommand::Save, KeyBinding::new(KeyModifiers::alt(), KeyCode::S));
        assert!(shortcuts.is_custom(ShortcutCommand::Save));
        
        // Reset to default
        shortcuts.reset(ShortcutCommand::Save);
        assert!(!shortcuts.is_custom(ShortcutCommand::Save));
        
        // Should return default binding again
        let binding = shortcuts.get(ShortcutCommand::Save);
        assert!(binding.modifiers.ctrl);
    }

    #[test]
    fn test_keyboard_shortcuts_reset_all() {
        let mut shortcuts = KeyboardShortcuts::default();
        
        // Set some custom bindings
        shortcuts.set(ShortcutCommand::Save, KeyBinding::new(KeyModifiers::alt(), KeyCode::S));
        shortcuts.set(ShortcutCommand::Open, KeyBinding::new(KeyModifiers::alt(), KeyCode::O));
        
        // Reset all
        shortcuts.reset_all();
        
        // All should be defaults now
        assert!(!shortcuts.is_custom(ShortcutCommand::Save));
        assert!(!shortcuts.is_custom(ShortcutCommand::Open));
    }

    #[test]
    fn test_keyboard_shortcuts_conflict_detection() {
        let mut shortcuts = KeyboardShortcuts::default();
        
        // Get Save binding
        let save_binding = shortcuts.get(ShortcutCommand::Save);
        
        // There should be no conflict when checking against Save itself
        assert!(shortcuts.find_conflict(&save_binding, Some(ShortcutCommand::Save)).is_none());
        
        // If we don't exclude, Save should be found as using this binding
        assert_eq!(
            shortcuts.find_conflict(&save_binding, None),
            Some(ShortcutCommand::Save)
        );
    }

    #[test]
    fn test_keyboard_shortcuts_serialization() {
        let mut shortcuts = KeyboardShortcuts::default();
        shortcuts.set(ShortcutCommand::Save, KeyBinding::new(KeyModifiers::alt(), KeyCode::S));
        
        let json = serde_json::to_string(&shortcuts).unwrap();
        assert!(json.contains("save"));
        assert!(json.contains("alt"));
    }

    #[test]
    fn test_keyboard_shortcuts_deserialization() {
        let json = r#"{"bindings": {"save": {"modifiers": {"ctrl": false, "shift": false, "alt": true}, "key": "s"}}}"#;
        let shortcuts: KeyboardShortcuts = serde_json::from_str(json).unwrap();
        
        let binding = shortcuts.get(ShortcutCommand::Save);
        assert!(binding.modifiers.alt);
        assert!(!binding.modifiers.ctrl);
    }

    #[test]
    fn test_settings_keyboard_shortcuts_default() {
        let settings = Settings::default();
        // Keyboard shortcuts should have default empty bindings map
        assert!(!settings.keyboard_shortcuts.is_custom(ShortcutCommand::Save));
    }

    #[test]
    fn test_settings_backward_compatibility_keyboard_shortcuts() {
        // Old JSON without keyboard_shortcuts field should default to empty bindings
        let json = r#"{"theme": "dark"}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert!(!settings.keyboard_shortcuts.is_custom(ShortcutCommand::Save));
    }

    #[test]
    fn test_commands_by_category() {
        let categories = KeyboardShortcuts::commands_by_category();
        
        // Should have multiple categories
        assert!(!categories.is_empty());
        
        // Each category should have at least one command
        for (name, commands) in &categories {
            assert!(!name.is_empty());
            assert!(!commands.is_empty());
        }
        
        // Find "File" category
        let file_cat = categories.iter().find(|(name, _)| *name == "File");
        assert!(file_cat.is_some());
        let (_, file_commands) = file_cat.unwrap();
        assert!(file_commands.contains(&ShortcutCommand::Save));
    }
}
