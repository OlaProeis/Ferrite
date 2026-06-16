//! Command registry for the command palette.
//!
//! Provides a unified list of all executable commands with metadata
//! (display name, category, shortcut hint, icon) for the command palette UI.

use crate::config::ShortcutCommand;

/// A command entry for the palette, combining a shortcut command with its
/// display metadata and an optional icon.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    pub command: ShortcutCommand,
    pub icon: &'static str,
}

impl PaletteCommand {
    pub fn label(&self) -> &'static str {
        self.command.display_name()
    }

    pub fn category(&self) -> &'static str {
        self.command.category()
    }
}

/// Build the full list of commands available in the palette.
/// Excludes CommandPalette itself (no recursion).
pub fn all_palette_commands() -> Vec<PaletteCommand> {
    ShortcutCommand::all()
        .iter()
        .filter(|cmd| !matches!(cmd, ShortcutCommand::CommandPalette))
        .map(|&command| PaletteCommand {
            command,
            icon: icon_for_command(command),
        })
        .collect()
}

fn icon_for_command(cmd: ShortcutCommand) -> &'static str {
    use crate::ui::phosphor_icons::{
        ARROWS_IN, ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_LEFT, ARROW_RIGHT,
        ARROW_UP, BRACKETS_CURLY, CARET_DOWN, CARET_RIGHT, CARET_UP, CLIPBOARD, CODE, CORNERS_OUT,
        EYE, FILE_MAGNIFYING_GLASS, FILE_PDF, FILE_PLUS, FILE_TEXT, FLOPPY_DISK, FOLDERS, GEAR,
        GLOBE, IMAGE, INFO, LIGHTNING, LINK, LIST, LIST_BULLETS, LIST_CHECKS, LIST_NUMBERS, LOCK,
        MAGNIFYING_GLASS, MAGNIFYING_GLASS_MINUS, MAGNIFYING_GLASS_PLUS, MINUS, NOTE_PENCIL,
        PALETTE, PENCIL, PIPE, PLUS, PRINTER, SCROLL, TERMINAL_WINDOW, TEXT_ALIGN_LEFT, TEXT_B,
        TEXT_H_ONE, TEXT_ITALIC, TEXT_T, TRASH, TREE, X,
    };

    match cmd {
        // File
        ShortcutCommand::Save | ShortcutCommand::SaveAs => FLOPPY_DISK,
        ShortcutCommand::Open => FILE_TEXT,
        ShortcutCommand::New => FILE_PLUS,
        ShortcutCommand::NewTab => PLUS,
        ShortcutCommand::NewWindow => NOTE_PENCIL,
        ShortcutCommand::CloseTab => X,
        ShortcutCommand::OpenWorkspace | ShortcutCommand::CloseWorkspace => FOLDERS,
        // Navigation
        ShortcutCommand::NextTab => ARROW_RIGHT,
        ShortcutCommand::PrevTab => ARROW_LEFT,
        ShortcutCommand::GoToLine => LIST_NUMBERS,
        ShortcutCommand::QuickOpen => LIGHTNING,
        // View
        ShortcutCommand::ToggleViewMode => EYE,
        ShortcutCommand::CycleTheme => PALETTE,
        ShortcutCommand::ToggleZenMode => ARROWS_IN,
        ShortcutCommand::ToggleWordWrap => TEXT_ALIGN_LEFT,
        ShortcutCommand::TogglePreviewLock => LOCK,
        ShortcutCommand::ToggleFullscreen => CORNERS_OUT,
        ShortcutCommand::ToggleOutline => LIST,
        ShortcutCommand::ToggleFileTree => TREE,
        ShortcutCommand::TogglePipeline => PIPE,
        ShortcutCommand::ToggleTerminal => TERMINAL_WINDOW,
        ShortcutCommand::ToggleProductivityHub => LIST_CHECKS,
        ShortcutCommand::ZoomIn => MAGNIFYING_GLASS_PLUS,
        ShortcutCommand::ZoomOut => MAGNIFYING_GLASS_MINUS,
        ShortcutCommand::ResetZoom => MAGNIFYING_GLASS,
        // Edit
        ShortcutCommand::Undo => ARROW_COUNTER_CLOCKWISE,
        ShortcutCommand::Redo => ARROW_CLOCKWISE,
        ShortcutCommand::DeleteLine => TRASH,
        ShortcutCommand::DuplicateLine => CLIPBOARD,
        ShortcutCommand::MoveLineUp => ARROW_UP,
        ShortcutCommand::MoveLineDown => ARROW_DOWN,
        ShortcutCommand::SelectNextOccurrence => TEXT_T,
        // Search
        ShortcutCommand::Find => MAGNIFYING_GLASS,
        ShortcutCommand::FindReplace => MAGNIFYING_GLASS,
        ShortcutCommand::FindNext => CARET_DOWN,
        ShortcutCommand::FindPrev => CARET_UP,
        ShortcutCommand::SearchInFiles => FILE_MAGNIFYING_GLASS,
        // Format
        ShortcutCommand::FormatBold => TEXT_B,
        ShortcutCommand::FormatItalic => TEXT_ITALIC,
        ShortcutCommand::FormatInlineCode => CODE,
        ShortcutCommand::FormatCodeBlock => BRACKETS_CURLY,
        ShortcutCommand::FormatLink => LINK,
        ShortcutCommand::FormatImage => IMAGE,
        ShortcutCommand::FormatBlockquote => NOTE_PENCIL,
        ShortcutCommand::FormatBulletList => LIST_BULLETS,
        ShortcutCommand::FormatNumberedList => LIST_NUMBERS,
        ShortcutCommand::FormatHeading1 => TEXT_H_ONE,
        ShortcutCommand::FormatHeading2
        | ShortcutCommand::FormatHeading3
        | ShortcutCommand::FormatHeading4
        | ShortcutCommand::FormatHeading5
        | ShortcutCommand::FormatHeading6 => TEXT_H_ONE,
        // Folding
        ShortcutCommand::FoldAll => CARET_RIGHT,
        ShortcutCommand::UnfoldAll => CARET_DOWN,
        ShortcutCommand::ToggleFoldAtCursor => MINUS,
        // Other
        ShortcutCommand::CommandPalette => MAGNIFYING_GLASS,
        ShortcutCommand::OpenSettings => GEAR,
        ShortcutCommand::OpenAbout => INFO,
        ShortcutCommand::ExportHtml => GLOBE,
        ShortcutCommand::ExportPdf => FILE_PDF,
        ShortcutCommand::PrintPreview => PRINTER,
        ShortcutCommand::InsertToc => SCROLL,
        ShortcutCommand::ToggleFrontmatter => PENCIL,
    }
}
