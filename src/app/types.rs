//! Type definitions for the Ferrite application module.
//!
//! This module contains enums and structs used across the app module
//! for keyboard actions, navigation requests, and deferred operations.

use crate::markdown::MarkdownFormatCommand;
use std::path::PathBuf;

/// Keyboard shortcut actions that need to be deferred.
///
/// These actions are detected in the input handling closure and executed
/// afterwards to avoid borrow conflicts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum KeyboardAction {
    /// Save current file (Ctrl+S)
    Save,
    /// Save As dialog (Ctrl+Shift+S)
    SaveAs,
    /// Open file dialog (Ctrl+O)
    Open,
    /// New file (Ctrl+N)
    New,
    /// New tab (Ctrl+T)
    NewTab,
    /// New document window (Ctrl+Shift+N)
    NewWindow,
    /// Close current tab (Ctrl+W)
    CloseTab,
    /// Next tab (Ctrl+Tab)
    NextTab,
    /// Previous tab (Ctrl+Shift+Tab)
    PrevTab,
    /// Toggle view mode (Ctrl+E)
    ToggleViewMode,
    /// Cycle theme (Ctrl+Shift+T)
    CycleTheme,
    /// Open settings panel (Ctrl+,)
    OpenSettings,
    /// Open find panel (Ctrl+F)
    OpenFind,
    /// Open find and replace panel (Ctrl+H)
    OpenFindReplace,
    /// Find next match (F3)
    FindNext,
    /// Find previous match (Shift+F3)
    FindPrev,
    /// Apply markdown formatting
    Format(MarkdownFormatCommand),
    /// Toggle outline panel (Ctrl+Shift+O)
    ToggleOutline,
    /// Toggle file tree panel (Ctrl+Shift+E)
    ToggleFileTree,
    /// Open quick file switcher (Ctrl+P)
    QuickOpen,
    /// Search in files (Ctrl+Shift+F)
    SearchInFiles,
    /// Export as HTML (Ctrl+Shift+X)
    ExportHtml,
    /// Export as PDF (Ctrl+Shift+P) — opens the export options dialog.
    ExportPdf,
    /// Print preview — same PDF pipeline, opens in-app viewer tab.
    PrintPreview,
    /// Open about/help panel (F1)
    OpenAbout,
    /// Select next occurrence of current word/selection (Ctrl+Shift+G)
    SelectNextOccurrence,
    /// Exit multi-cursor mode (Escape when multi-cursor active)
    ExitMultiCursor,
    /// Toggle Zen Mode (F11)
    ToggleZenMode,
    /// Toggle word wrap (Alt+Z / Option+Z)
    ToggleWordWrap,
    /// Toggle OS fullscreen (F10)
    ToggleFullscreen,
    /// Fold all regions (Ctrl+Shift+[)
    FoldAll,
    /// Unfold all regions (Ctrl+Shift+])
    UnfoldAll,
    /// Toggle fold at cursor (Ctrl+Shift+.)
    ToggleFoldAtCursor,
    /// Toggle Live Pipeline panel (Ctrl+Shift+L)
    TogglePipeline,
    /// Toggle Terminal panel (Ctrl+`)
    ToggleTerminal,
    /// Toggle Productivity Hub panel (Ctrl+Shift+H)
    ToggleProductivityHub,
    /// Open Go to Line dialog (Ctrl+G)
    GoToLine,
    /// Duplicate current line or selection (Ctrl+Shift+D)
    DuplicateLine,
    /// Delete current line (Ctrl+D)
    DeleteLine,
    /// Insert/Update Table of Contents (Ctrl+Shift+U)
    InsertToc,
    /// Toggle Frontmatter Panel (Ctrl+Shift+M)
    ToggleFrontmatter,
    /// Zoom in (Ctrl+=)
    ZoomIn,
    /// Zoom out (Ctrl+-)
    ZoomOut,
    /// Reset zoom (Ctrl+0)
    ResetZoom,
}

/// Request to navigate to a heading in the document.
/// Used for both outline panel and semantic minimap navigation.
#[derive(Debug, Clone)]
pub(crate) struct HeadingNavRequest {
    /// Target line number (1-indexed)
    pub line: usize,
    /// Heading title text (for text-based search and matching)
    pub title: Option<String>,
}

/// A deferred format action that captures the selection state at click time.
/// This ensures the formatting is applied to the correct selection even if
/// the editor loses focus between click and processing.
#[derive(Debug, Clone)]
pub(crate) struct DeferredFormatAction {
    /// The formatting command to apply
    pub cmd: MarkdownFormatCommand,
    /// The selection range in character positions (start, end) captured at click time
    /// If None, the cursor position will be used
    pub selection: Option<(usize, usize)>,
}

/// Messages sent from background file-loading threads to the UI thread.
pub(crate) enum FileLoadMsg {
    /// Progress update: tab_id, bytes_loaded so far.
    Progress { tab_id: usize, bytes_loaded: u64 },
    /// Loading complete: tab_id, full file bytes.
    Complete { tab_id: usize, bytes: Vec<u8> },
    /// Loading failed: tab_id, error message.
    Error { tab_id: usize, error: String },
    /// Binary/denylisted file detected during background load — delegate externally.
    OpenExternal { tab_id: usize, path: PathBuf },
}

/// Information about a pending auto-save recovery for user confirmation.
#[derive(Debug, Clone)]
pub(crate) struct AutoSaveRecoveryInfo {
    /// Tab ID that has recovery available
    pub tab_id: usize,
    /// Tab index in the tabs array
    pub tab_index: usize,
    /// File path (if any)
    pub path: Option<std::path::PathBuf>,
    /// Recovered content from auto-save
    pub recovered_content: String,
    /// Timestamp when auto-save was created
    pub saved_at: u64,
}
