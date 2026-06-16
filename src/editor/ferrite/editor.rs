//! FerriteEditor - Custom text editor widget for Ferrite.
//!
//! This module provides `FerriteEditor`, a custom egui widget that integrates:
//! - `TextBuffer` for rope-based text storage
//! - `ViewState` for viewport tracking and virtual scrolling
//! - `LineCache` for efficient galley caching
//!
//! # Phase 1 Scope
//! - Basic text rendering with virtual scrolling
//! - Line number gutter
//! - Simple cursor (no selection yet)
//! - Keyboard input for text editing
//!
//! # Example
//! ```rust,ignore
//! use crate::editor::FerriteEditor;
//!
//! let mut editor = FerriteEditor::from_string("Hello, World!\nLine 2");
//!
//! // In your egui update loop:
//! egui::CentralPanel::default().show(ctx, |ui| {
//!     editor.ui(ctx, ui);
//! });
//! ```

use std::sync::Arc;

use egui::{Color32, Context, EventFilter, FontId, ImeEvent, Response, Sense, Stroke, Ui, Vec2};

use super::buffer::TextBuffer;
use super::cursor::{Cursor, Selection};
use super::grapheme;
use super::input::{InputHandler, InputResult};
use super::line_cache::{HighlightedSegment, LineCache};
use super::rendering::{cursor as cursor_render, gutter, text as text_render};
use super::table_guides::{self, TableGuideCache};
use super::view::ViewState;

// Import syntax highlighting, font utilities, fold state, and nav buttons
use crate::config::EditorFont;
use crate::fonts;
use crate::markdown::syntax::{highlight_code, highlight_code_with_theme};
use crate::state::FoldState;
use crate::ui::{render_nav_buttons, NavAction};

/// Default font size for the editor.
const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Fixed line height for Phase 1 rendering.
/// This provides consistent vertical spacing regardless of font metrics.
const FIXED_LINE_HEIGHT: f32 = 20.0;

/// A search match with pre-computed metadata for efficient rendering.
///
/// Pre-computing the line number avoids O(n) string iteration on every frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchMatch {
    /// Start byte position in the buffer.
    pub start_byte: usize,
    /// End byte position in the buffer.
    pub end_byte: usize,
    /// Pre-computed line number (0-indexed) for efficient scroll-to-match.
    pub line: usize,
}

/// Custom text editor widget integrating Phase 1 modules.
///
/// `FerriteEditor` provides a high-performance text editing widget with:
/// - Rope-based text storage for O(log n) operations
/// - Virtual scrolling (only renders visible lines)
/// - Galley caching to avoid expensive text layout every frame
/// - Operation-based undo/redo history
/// - Text selection with shift+arrow, click-drag, double/triple click
/// - Clipboard operations (Ctrl+A/C/X/V)
/// - Optional word wrap support
/// - Syntax highlighting for source code files
///
/// # Thread Safety
/// This struct is not thread-safe and should only be used from the UI thread.
#[derive(Debug, Clone)]
pub struct FerriteEditor {
    /// The text content.
    pub(crate) buffer: TextBuffer,
    /// Viewport state for virtual scrolling.
    pub(crate) view: ViewState,
    /// Galley cache for efficient rendering.
    pub(crate) line_cache: LineCache,
    // ─────────────────────────────────────────────────────────────────────────
    // Multi-Cursor Support (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────
    /// All active selections (sorted by anchor position, non-overlapping).
    /// Each selection includes cursor position as `selection.head`.
    /// A single cursor is represented as a Vec with one collapsed Selection.
    pub(crate) selections: Vec<Selection>,
    /// Index of the primary selection (for status bar display, scroll anchoring).
    pub(crate) primary_selection_index: usize,
    /// Font size for rendering.
    pub(crate) font_size: f32,
    /// Font family for rendering (from Settings).
    pub(crate) font_family: EditorFont,
    /// Whether content has changed since last render.
    pub(crate) content_dirty: bool,
    /// Accumulated dirty line range since last render.
    /// `Some((start, end))` where both are 0-indexed line numbers (inclusive).
    /// Grows via [`extend_dirty_range`] and is consumed on the next frame.
    dirty_range: Option<(usize, usize)>,
    /// Whether word wrap is enabled.
    pub(crate) wrap_enabled: bool,
    /// Maximum wrap width in pixels (when set, text wraps at this width or available width, whichever is smaller).
    pub(crate) max_wrap_width: Option<f32>,
    /// Preferred column for vertical cursor movement (preserved when moving up/down).
    pub(crate) preferred_column: Option<usize>,
    /// Last click time for double/triple click detection.
    pub(crate) last_click_time: Option<std::time::Instant>,
    /// Click count for double/triple click (1, 2, or 3).
    pub(crate) click_count: u8,
    /// Position of last click for multi-click detection.
    pub(crate) last_click_pos: Option<Cursor>,
    /// Position where mouse button was pressed (for accurate drag anchor).
    pub(crate) drag_start_cursor: Option<Cursor>,
    // ─────────────────────────────────────────────────────────────────────────
    // Syntax Highlighting (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether syntax highlighting is enabled.
    pub(crate) syntax_enabled: bool,
    /// Language identifier for syntax highlighting (e.g., "rust", "python").
    pub(crate) syntax_language: Option<String>,
    /// Whether to use dark mode syntax theme.
    pub(crate) syntax_dark_mode: bool,
    /// Syntax theme name (e.g., "Dracula", "Nord"). None = use dark/light mode default.
    pub(crate) syntax_theme_name: Option<String>,
    /// Cached hash of the current syntax theme (for cache invalidation).
    pub(crate) syntax_theme_hash: u64,
    // ─────────────────────────────────────────────────────────────────────────
    // Search Highlights (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────
    /// Search match positions with pre-computed line numbers.
    /// All matches are stored, but only up to MAX_DISPLAYED_MATCHES are rendered.
    pub(crate) search_matches: Vec<SearchMatch>,
    /// Index of the current/focused search match (for distinct highlighting).
    pub(crate) current_search_match: usize,
    /// Whether to scroll to the current search match on next render.
    pub(crate) scroll_to_search_match: bool,
    // ─────────────────────────────────────────────────────────────────────────
    // Bracket Matching (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether bracket matching is enabled.
    pub(crate) bracket_matching_enabled: bool,
    /// Colors for bracket matching (bg_color, border_color). None = use theme defaults.
    pub(crate) bracket_colors: Option<(Color32, Color32)>,
    // ─────────────────────────────────────────────────────────────────────────
    // LSP Diagnostics
    // ─────────────────────────────────────────────────────────────────────────
    /// Diagnostics for the current file (set by the app layer each frame).
    pub(crate) diagnostics: Vec<crate::lsp::state::DiagnosticEntry>,
    // ─────────────────────────────────────────────────────────────────────────
    // IME/CJK Support (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether IME is currently enabled/active.
    pub(crate) ime_enabled: bool,
    /// Current IME preedit (composition) text. None when no composition is active.
    pub(crate) ime_preedit: Option<String>,
    /// Cursor range saved when IME was enabled, used to restore state on commit.
    pub(crate) ime_cursor_range: Option<Selection>,
    /// Text committed via IME that may need CJK font loading.
    /// Cleared after being checked by the caller.
    pub(crate) ime_committed_text: Option<String>,
    /// Last font generation seen - used to invalidate cache when fonts change.
    pub(crate) last_font_generation: u64,
    /// Last zoom factor seen - used to invalidate cache when Ctrl+scroll zoom changes.
    /// Cached galleys are tied to pixels_per_point; stale galleys render as garbled text.
    pub(crate) last_zoom_factor: f32,
    // ─────────────────────────────────────────────────────────────────────────
    // Cursor Blink
    // ─────────────────────────────────────────────────────────────────────────
    /// Instant when cursor visibility was last toggled.
    pub(crate) cursor_blink_instant: std::time::Instant,
    /// Whether the cursor is currently visible (toggles for blink effect).
    pub(crate) cursor_visible: bool,
    // ─────────────────────────────────────────────────────────────────────────
    // Code Folding (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────
    /// Fold state for collapsing/expanding regions of code.
    pub(crate) fold_state: FoldState,
    /// Whether to show fold indicators in the gutter.
    pub(crate) show_fold_indicators: bool,
    /// Line where fold was toggled this frame (for returning to caller).
    pub(crate) fold_toggle_line: Option<usize>,
    // ─────────────────────────────────────────────────────────────────────────
    // Gutter Display (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether to show line numbers in the gutter.
    pub(crate) show_line_numbers: bool,
    // ─────────────────────────────────────────────────────────────────────────
    // Content Centering (Zen Mode)
    // ─────────────────────────────────────────────────────────────────────────
    /// Horizontal offset for centering content (used in Zen Mode).
    /// When non-zero, text is rendered starting at text_start_x + content_offset_x.
    pub(crate) content_offset_x: f32,
    // ─────────────────────────────────────────────────────────────────────────
    // Auto-Close Brackets
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether auto-close brackets is enabled (e.g., typing '(' inserts '()').
    pub(crate) auto_close_brackets: bool,
    // ─────────────────────────────────────────────────────────────────────────
    // Vim Mode
    // ─────────────────────────────────────────────────────────────────────────
    /// Whether Vim modal editing is enabled.
    pub(crate) vim_mode_enabled: bool,
    /// Persistent Vim editing state (mode, yank register, pending operator).
    pub(crate) vim_state: super::vim::VimState,
    /// Cached GFM table column guides for the raw editor viewport.
    pub(crate) table_guide_cache: TableGuideCache,
}

impl Default for FerriteEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // FerriteEditor public API surface (widgets, tests, future crate extraction)
impl FerriteEditor {
    /// Creates a new empty editor.
    ///
    /// # Example
    /// ```rust,ignore
    /// let editor = FerriteEditor::new();
    /// assert!(editor.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            view: ViewState::new(),
            line_cache: LineCache::new(),
            // Multi-cursor: start with single cursor at position 0
            selections: vec![Selection::start()],
            primary_selection_index: 0,
            font_size: DEFAULT_FONT_SIZE,
            font_family: EditorFont::default(),
            content_dirty: false,
            dirty_range: None,
            wrap_enabled: false,
            max_wrap_width: None,
            preferred_column: None,
            last_click_time: None,
            click_count: 0,
            last_click_pos: None,
            drag_start_cursor: None,
            // Syntax highlighting defaults
            syntax_enabled: false,
            syntax_language: None,
            syntax_dark_mode: true,
            syntax_theme_name: None,
            syntax_theme_hash: 0,
            // Search highlight defaults
            search_matches: Vec::new(),
            current_search_match: 0,
            scroll_to_search_match: false,
            // Bracket matching defaults
            bracket_matching_enabled: false,
            bracket_colors: None,
            diagnostics: Vec::new(),
            // IME defaults
            ime_enabled: false,
            ime_preedit: None,
            ime_cursor_range: None,
            ime_committed_text: None,
            // Font tracking
            last_font_generation: fonts::font_generation(),
            last_zoom_factor: 1.0,
            // Cursor blink defaults
            cursor_blink_instant: std::time::Instant::now(),
            cursor_visible: true,
            // Code folding defaults
            fold_state: FoldState::new(),
            show_fold_indicators: true,
            fold_toggle_line: None,
            // Gutter display defaults
            show_line_numbers: true,
            // Content centering (Zen Mode)
            content_offset_x: 0.0,
            // Auto-close brackets (default off, configured via EditorWidget)
            auto_close_brackets: false,
            // Vim mode (default off, configured via EditorWidget)
            vim_mode_enabled: false,
            vim_state: super::vim::VimState::new(),
            table_guide_cache: TableGuideCache::default(),
        }
    }

    /// Creates an editor with initial content.
    ///
    /// # Arguments
    /// * `content` - The initial text content
    ///
    /// # Example
    /// ```rust,ignore
    /// let editor = FerriteEditor::from_string("Hello\nWorld");
    /// assert_eq!(editor.line_count(), 2);
    /// ```
    #[must_use]
    pub fn from_string(content: &str) -> Self {
        Self {
            buffer: TextBuffer::from_string(content),
            view: ViewState::new(),
            line_cache: LineCache::new(),
            // Multi-cursor: start with single cursor at position 0
            selections: vec![Selection::start()],
            primary_selection_index: 0,
            font_size: DEFAULT_FONT_SIZE,
            font_family: EditorFont::default(),
            content_dirty: true, // Mark dirty to ensure initial cache population
            dirty_range: None,
            wrap_enabled: false,
            max_wrap_width: None,
            preferred_column: None,
            last_click_time: None,
            click_count: 0,
            last_click_pos: None,
            drag_start_cursor: None,
            // Syntax highlighting defaults
            syntax_enabled: false,
            syntax_language: None,
            syntax_dark_mode: true,
            syntax_theme_name: None,
            syntax_theme_hash: 0,
            // Search highlight defaults
            search_matches: Vec::new(),
            current_search_match: 0,
            scroll_to_search_match: false,
            // Bracket matching defaults
            bracket_matching_enabled: false,
            bracket_colors: None,
            diagnostics: Vec::new(),
            // IME defaults
            ime_enabled: false,
            ime_preedit: None,
            ime_cursor_range: None,
            ime_committed_text: None,
            // Font tracking
            last_font_generation: fonts::font_generation(),
            last_zoom_factor: 1.0,
            // Cursor blink defaults
            cursor_blink_instant: std::time::Instant::now(),
            cursor_visible: true,
            // Code folding defaults
            fold_state: FoldState::new(),
            show_fold_indicators: true,
            fold_toggle_line: None,
            // Gutter display defaults
            show_line_numbers: true,
            // Content centering (Zen Mode)
            content_offset_x: 0.0,
            // Auto-close brackets (default off, configured via EditorWidget)
            auto_close_brackets: false,
            // Vim mode (default off, configured via EditorWidget)
            vim_mode_enabled: false,
            vim_state: super::vim::VimState::new(),
            table_guide_cache: TableGuideCache::default(),
        }
    }

    /// Replace the buffer content while preserving editor state (view, syntax, etc.).
    ///
    /// This is used when external changes (e.g., WYSIWYG editing, file reload) modify
    /// tab.content and the FerriteEditor needs to be updated without full recreation.
    /// Unlike `from_string()`, this preserves:
    /// - ViewState (scroll position, viewport)
    /// - Syntax highlighting configuration
    /// - Font settings
    /// - Fold state configuration (folds are cleared since content changed)
    /// - Search state
    ///
    /// The cursor is clamped to valid bounds after content replacement.
    pub fn set_content(&mut self, content: &str) {
        self.buffer = TextBuffer::from_string(content);
        self.line_cache.invalidate();
        self.table_guide_cache.clear();
        self.content_dirty = true;
        self.fold_state = FoldState::new();
        // Clamp all selections to valid bounds
        let max_line = self.buffer.line_count().saturating_sub(1);
        for sel in &mut self.selections {
            sel.anchor.line = sel.anchor.line.min(max_line);
            sel.head.line = sel.head.line.min(max_line);
            let anchor_line_len = self
                .buffer
                .get_line(sel.anchor.line)
                .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                .unwrap_or(0);
            sel.anchor.column = sel.anchor.column.min(anchor_line_len);
            let head_line_len = self
                .buffer
                .get_line(sel.head.line)
                .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                .unwrap_or(0);
            sel.head.column = sel.head.column.min(head_line_len);
        }
    }

    /// Sets the font size for rendering.
    ///
    /// # Arguments
    /// * `size` - Font size in points (clamped to 8.0..=72.0)
    pub fn set_font_size(&mut self, size: f32) {
        let new_size = size.clamp(8.0, 72.0);
        if (self.font_size - new_size).abs() > 0.01 {
            self.font_size = new_size;
            // Font size change invalidates all cached galleys
            self.line_cache.invalidate();
        }
    }

    /// Returns the current font size.
    #[must_use]
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Sets the font family for rendering.
    ///
    /// # Arguments
    /// * `font` - The font family to use (Inter, JetBrainsMono, or Custom)
    pub fn set_font_family(&mut self, font: EditorFont) {
        if self.font_family != font {
            self.font_family = font;
            // Font family change invalidates all cached galleys
            self.line_cache.invalidate();
        }
    }

    /// Returns the current font family.
    #[must_use]
    pub fn font_family(&self) -> &EditorFont {
        &self.font_family
    }

    /// Returns the number of lines in the buffer.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    /// Returns `true` if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns a reference to the text buffer.
    #[must_use]
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// Returns a mutable reference to the text buffer.
    ///
    /// Note: Modifications through this reference bypass undo/redo tracking.
    /// Use the editor's edit methods for tracked changes.
    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        self.content_dirty = true;
        &mut self.buffer
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Selection & Cursor Access (Multi-cursor aware)
    // ─────────────────────────────────────────────────────────────────────────

    /// Returns the current cursor position (head of primary selection).
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.primary_selection().head
    }

    /// Sets the cursor position, collapsing any selection and clearing extra cursors.
    ///
    /// # Arguments
    /// * `cursor` - The new cursor position (will be clamped to valid range)
    pub fn set_cursor(&mut self, cursor: Cursor) {
        let clamped = self.clamp_cursor(cursor);
        self.selections = vec![Selection::collapsed(clamped)];
        self.primary_selection_index = 0;
    }

    /// Returns the primary selection.
    #[must_use]
    pub fn selection(&self) -> Selection {
        self.primary_selection()
    }

    /// Returns all selections (for multi-cursor support).
    #[must_use]
    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    /// Returns the number of active cursors/selections.
    #[must_use]
    pub fn cursor_count(&self) -> usize {
        self.selections.len()
    }

    /// Resets the cursor blink state, making the cursor visible immediately.
    ///
    /// This should be called when the user performs any input action (typing, clicking)
    /// to provide immediate visual feedback. The cursor will then resume blinking
    /// after the blink interval has elapsed.
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.cursor_blink_instant = std::time::Instant::now();
    }

    /// Returns whether multiple cursors are active.
    #[must_use]
    pub fn has_multiple_cursors(&self) -> bool {
        self.selections.len() > 1
    }

    /// Returns the primary selection (the one used for scroll anchoring and status display).
    #[must_use]
    pub fn primary_selection(&self) -> Selection {
        self.selections
            .get(self.primary_selection_index)
            .copied()
            .unwrap_or_else(Selection::start)
    }

    /// Returns a mutable reference to the primary selection.
    fn primary_selection_mut(&mut self) -> &mut Selection {
        // Ensure primary_selection_index is valid
        if self.primary_selection_index >= self.selections.len() {
            self.primary_selection_index = self.selections.len().saturating_sub(1);
        }
        if self.selections.is_empty() {
            self.selections.push(Selection::start());
        }
        &mut self.selections[self.primary_selection_index]
    }

    /// Sets the primary selection, clearing all other cursors.
    ///
    /// # Arguments
    /// * `selection` - The new selection (will be clamped to valid range)
    pub fn set_selection(&mut self, selection: Selection) {
        let anchor = self.clamp_cursor(selection.anchor);
        let head = self.clamp_cursor(selection.head);
        self.selections = vec![Selection::new(anchor, head)];
        self.primary_selection_index = 0;
    }

    /// Returns whether the primary selection has a non-empty range.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.primary_selection().is_range()
    }

    /// Returns whether any selection has a non-empty range.
    #[must_use]
    pub fn has_any_selection(&self) -> bool {
        self.selections.iter().any(|s| s.is_range())
    }

    /// Clears all extra cursors, keeping only the primary.
    pub fn clear_extra_cursors(&mut self) {
        if self.selections.len() > 1 {
            let primary = self.primary_selection();
            self.selections = vec![primary];
            self.primary_selection_index = 0;
        }
    }

    /// Adds a new cursor at the given position.
    /// Returns the index of the new cursor.
    pub fn add_cursor(&mut self, cursor: Cursor) -> usize {
        let clamped = self.clamp_cursor(cursor);
        let new_selection = Selection::collapsed(clamped);
        self.selections.push(new_selection);
        self.merge_overlapping_selections();
        self.selections.len() - 1
    }

    /// Adds a new selection.
    /// Returns the index of the new selection.
    pub fn add_selection(&mut self, selection: Selection) -> usize {
        let anchor = self.clamp_cursor(selection.anchor);
        let head = self.clamp_cursor(selection.head);
        self.selections.push(Selection::new(anchor, head));
        self.merge_overlapping_selections();
        self.selections.len() - 1
    }

    /// Sorts selections by position and merges overlapping ones.
    fn merge_overlapping_selections(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        // Sort by start position
        self.selections.sort_by(|a, b| {
            let (a_start, _) = a.ordered();
            let (b_start, _) = b.ordered();
            a_start.cmp(&b_start)
        });

        // Merge overlapping selections
        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        let mut current = self.selections[0];

        for sel in self.selections.iter().skip(1) {
            let (_, current_end) = current.ordered();
            let (sel_start, sel_end) = sel.ordered();

            if sel_start <= current_end {
                // Overlap - merge by extending current to cover both
                let (current_start, _) = current.ordered();
                let new_end = current_end.max(sel_end);
                // Preserve anchor/head direction of the current selection
                if current.anchor <= current.head {
                    current = Selection::new(current_start, new_end);
                } else {
                    current = Selection::new(new_end, current_start);
                }
            } else {
                // No overlap - push current and start new
                merged.push(current);
                current = *sel;
            }
        }
        merged.push(current);

        // Update primary index if needed
        if self.primary_selection_index >= merged.len() {
            self.primary_selection_index = merged.len().saturating_sub(1);
        }

        self.selections = merged;
    }

    /// Clamps a cursor position to valid bounds.
    fn clamp_cursor(&self, cursor: Cursor) -> Cursor {
        let line = cursor.line.min(self.buffer.line_count().saturating_sub(1));
        let line_len = self
            .buffer
            .get_line(line)
            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
            .unwrap_or(0);
        let column = cursor.column.min(line_len);
        Cursor::new(line, column)
    }

    /// Returns selected text. With a single range, returns that range's text.
    /// With multiple non-empty selections (multi-cursor), returns each range joined by `\n`
    /// (VS Code-style clipboard for column / multi-line selections).
    #[must_use]
    pub fn selected_text(&self) -> String {
        let mut ranges: Vec<Selection> = self
            .selections
            .iter()
            .filter(|s| s.is_range())
            .copied()
            .collect();

        if ranges.is_empty() {
            return String::new();
        }

        if ranges.len() == 1 {
            let (start, end) = ranges[0].ordered();
            return self.get_text_range(start, end);
        }

        ranges.sort_by(|a, b| {
            let (a_start, _) = a.ordered();
            let (b_start, _) = b.ordered();
            a_start.cmp(&b_start)
        });

        ranges
            .iter()
            .map(|s| {
                let (start, end) = s.ordered();
                self.get_text_range(start, end)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Line span covering all selections (for syntax invalidation after cut/delete).
    fn selection_dirty_line_span(&self) -> Option<(usize, usize)> {
        let mut start_line = usize::MAX;
        let mut end_line = 0usize;
        let mut any = false;
        for sel in &self.selections {
            if sel.is_range() {
                any = true;
                start_line = start_line.min(sel.anchor.line.min(sel.head.line));
                end_line = end_line.max(sel.anchor.line.max(sel.head.line));
            }
        }
        any.then_some((start_line, end_line))
    }

    /// Gets text between two cursor positions.
    fn get_text_range(&self, start: Cursor, end: Cursor) -> String {
        if start == end {
            return String::new();
        }

        let mut result = String::new();

        for line_idx in start.line..=end.line {
            if let Some(line_content) = self.buffer.get_line(line_idx) {
                let line_chars: Vec<char> = line_content.chars().collect();

                let start_col = if line_idx == start.line {
                    start.column
                } else {
                    0
                };
                let end_col = if line_idx == end.line {
                    end.column
                } else {
                    line_chars.len()
                };

                let start_col = start_col.min(line_chars.len());
                let end_col = end_col.min(line_chars.len());

                for &ch in &line_chars[start_col..end_col] {
                    result.push(ch);
                }
            }
        }

        result
    }

    /// Deletes text from all selections that have a range.
    /// Returns true if any text was deleted.
    ///
    /// For multi-cursor: processes selections in reverse order (highest position first)
    /// to avoid invalidating earlier positions when deleting.
    pub fn delete_selection(&mut self) -> bool {
        // Check if any selection has a range
        if !self.has_any_selection() {
            return false;
        }

        // Collect all selections with ranges, sorted by start position (descending)
        let mut selections_to_delete: Vec<(usize, Selection)> = self
            .selections
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_range())
            .map(|(i, s)| (i, *s))
            .collect();

        if selections_to_delete.is_empty() {
            return false;
        }

        // Sort by start position descending (delete from end first to preserve positions)
        selections_to_delete.sort_by(|a, b| {
            let (a_start, _) = a.1.ordered();
            let (b_start, _) = b.1.ordered();
            b_start.cmp(&a_start)
        });

        // Delete each selection (from end to beginning)
        for (_, selection) in &selections_to_delete {
            let (start, end) = selection.ordered();
            let start_pos = InputHandler::cursor_to_char_pos(&self.buffer, &start);
            let end_pos = InputHandler::cursor_to_char_pos(&self.buffer, &end);
            let len = end_pos.saturating_sub(start_pos);

            if len > 0 {
                self.buffer.remove(start_pos, len);
            }
        }

        // Update all selections to collapsed at their start positions
        // Need to recalculate positions after deletions
        let mut new_selections = Vec::with_capacity(self.selections.len());
        let mut offset_adjustment: isize = 0;

        // Sort original selections by start position (ascending) for offset calculation
        let mut indexed_selections: Vec<(usize, Selection)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(i, s)| (i, *s))
            .collect();
        indexed_selections.sort_by(|a, b| {
            let (a_start, _) = a.1.ordered();
            let (b_start, _) = b.1.ordered();
            a_start.cmp(&b_start)
        });

        for (_, selection) in indexed_selections {
            let (start, end) = selection.ordered();
            if selection.is_range() {
                // This selection was deleted - collapse to start with offset adjustment
                let adjusted_line = start.line;
                let adjusted_col = if offset_adjustment >= 0 {
                    start.column
                } else {
                    start.column.saturating_sub((-offset_adjustment) as usize)
                };
                new_selections.push(Selection::collapsed(Cursor::new(
                    adjusted_line,
                    adjusted_col,
                )));

                // Calculate how many characters were deleted
                let start_pos = InputHandler::cursor_to_char_pos(&self.buffer, &start);
                let end_pos_original = InputHandler::cursor_to_char_pos(&self.buffer, &end);
                offset_adjustment -= end_pos_original as isize - start_pos as isize;
            } else {
                // No range - just adjust position based on prior deletions
                new_selections.push(selection);
            }
        }

        self.selections = if new_selections.is_empty() {
            vec![Selection::start()]
        } else {
            new_selections
        };

        // Clamp primary index
        if self.primary_selection_index >= self.selections.len() {
            self.primary_selection_index = self.selections.len().saturating_sub(1);
        }

        self.merge_overlapping_selections();
        self.content_dirty = true;
        true
    }

    /// Copies the current selection to the clipboard via egui.
    pub fn copy_selection_to_clipboard(&self, ui: &Ui) {
        if self.has_any_selection() {
            ui.copy_text(self.selected_text());
        }
    }

    /// Cuts the current selection to the clipboard via egui.
    pub fn cut_selection_to_clipboard(&mut self, ui: &Ui) {
        if self.has_any_selection() {
            let dirty_span = self.selection_dirty_line_span();
            let text = self.selected_text();
            ui.copy_text(text);
            self.delete_selection();
            if let Some((start_line, end_line)) = dirty_span {
                self.mark_lines_dirty(start_line, end_line);
            }
            self.reset_cursor_blink();
        }
    }

    /// Inserts text at all cursor positions (paste / multi-cursor).
    pub fn paste_text(&mut self, text: &str) {
        self.insert_text_at_all_cursors(text);
    }

    /// Applies a markdown formatting command to the current selection or cursor position.
    ///
    /// This method integrates with the markdown formatting module to apply formatting
    /// like bold, italic, headings, lists, etc. It handles:
    /// - Selection wrapping (e.g., selecting "text" and pressing Bold → "**text**")
    /// - Cursor insertion (e.g., no selection + heading → adds "# " at line start)
    /// - Toggle behavior (e.g., selecting "**text**" and pressing Bold → "text")
    ///
    /// # Arguments
    /// * `cmd` - The markdown formatting command to apply
    ///
    /// # Returns
    /// `true` if the formatting was applied, `false` if not (e.g., no selection for inline formats)
    ///
    /// # Example
    /// ```rust,ignore
    /// use crate::markdown::formatting::MarkdownFormatCommand;
    ///
    /// // Apply bold formatting to selected text
    /// editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(0, 5)));
    /// editor.apply_markdown_format(MarkdownFormatCommand::Bold);
    /// ```
    pub fn apply_markdown_format(
        &mut self,
        cmd: crate::markdown::formatting::MarkdownFormatCommand,
    ) -> bool {
        use crate::markdown::formatting::apply_raw_format;

        // Get the full text content
        let content = self.buffer.to_string();

        // Get selection as character indices, then convert to byte indices
        // (apply_raw_format works with byte indices for string slicing)
        let selection = self.primary_selection();
        let (start_cursor, end_cursor) = selection.ordered();
        let start_char = InputHandler::cursor_to_char_pos(&self.buffer, &start_cursor);
        let end_char = InputHandler::cursor_to_char_pos(&self.buffer, &end_cursor);

        // Convert character indices to byte indices for the formatting function
        let start_byte = crate::string_utils::char_index_to_byte_index(&content, start_char);
        let end_byte = crate::string_utils::char_index_to_byte_index(&content, end_char);

        // Apply the formatting (uses byte indices)
        let result = apply_raw_format(&content, Some((start_byte, end_byte)), cmd);

        // If formatting was not applied (e.g., no selection for inline format), return early
        if !result.applied && result.text == content {
            return false;
        }

        // Replace buffer content
        let old_len = self.buffer.len();
        if old_len > 0 {
            self.buffer.remove(0, old_len);
        }
        self.buffer.insert(0, &result.text);

        // Update cursor/selection based on result
        // IMPORTANT: result.cursor and result.selection are in BYTE positions,
        // but char_pos_to_cursor expects CHARACTER positions. Convert first!
        if let Some((sel_start_byte, sel_end_byte)) = result.selection {
            // Convert byte positions to character positions
            let sel_start_char =
                crate::string_utils::byte_index_to_char_index(&result.text, sel_start_byte);
            let sel_end_char =
                crate::string_utils::byte_index_to_char_index(&result.text, sel_end_byte);
            let start_cursor = self.char_pos_to_cursor(sel_start_char);
            let end_cursor = self.char_pos_to_cursor(sel_end_char);
            self.set_selection(Selection::new(start_cursor, end_cursor));
        } else {
            // Convert byte position to character position
            let cursor_char =
                crate::string_utils::byte_index_to_char_index(&result.text, result.cursor);
            let new_cursor = self.char_pos_to_cursor(cursor_char);
            self.set_cursor(new_cursor);
        }

        // Ensure the cursor is visible
        self.view
            .ensure_line_visible(self.primary_selection().head.line, self.buffer.line_count());

        self.content_dirty = true;
        true
    }

    /// Applies a markdown formatting command using a pre-captured selection.
    ///
    /// This variant is used when the selection was captured earlier (e.g., at button click time)
    /// to avoid issues where the selection might have changed by the time formatting is applied.
    ///
    /// # Arguments
    /// * `cmd` - The markdown formatting command to apply
    /// * `captured_selection` - Pre-captured selection as (start_char, end_char)
    ///
    /// # Returns
    /// `true` if the formatting was applied, `false` if not
    pub fn apply_markdown_format_with_selection(
        &mut self,
        cmd: crate::markdown::formatting::MarkdownFormatCommand,
        captured_selection: (usize, usize),
    ) -> bool {
        use crate::markdown::formatting::apply_raw_format;

        // Get the full text content
        let content = self.buffer.to_string();

        // Use the pre-captured selection (in BYTE indices)
        let (start_byte, end_byte) = captured_selection;

        // Debug: show what we're formatting (using safe slicing)
        let selected_preview =
            if start_byte < content.len() && end_byte <= content.len() && start_byte <= end_byte {
                crate::string_utils::safe_slice(&content, start_byte, end_byte.min(start_byte + 30))
            } else {
                "<invalid range>"
            };
        log::debug!(
            "apply_markdown_format_with_selection: cmd={:?}, bytes={}..{}, preview='{}', content_len={}",
            cmd, start_byte, end_byte, selected_preview, content.len()
        );

        // Apply the formatting (uses byte indices)
        let result = apply_raw_format(&content, Some((start_byte, end_byte)), cmd);

        log::debug!(
            "Format result: applied={}, result_len={}, cursor={}, result_preview='{}'",
            result.applied,
            result.text.len(),
            result.cursor,
            &result.text[..result.text.len().min(50)]
        );

        // If formatting was not applied (e.g., no selection for inline format), return early
        if !result.applied && result.text == content {
            return false;
        }

        // Replace buffer content
        let old_len = self.buffer.len();
        if old_len > 0 {
            self.buffer.remove(0, old_len);
        }
        self.buffer.insert(0, &result.text);

        // Update cursor/selection based on result
        // IMPORTANT: result.cursor and result.selection are in BYTE positions,
        // but char_pos_to_cursor expects CHARACTER positions. Convert first!
        if let Some((sel_start_byte, sel_end_byte)) = result.selection {
            // Convert byte positions to character positions
            let sel_start_char =
                crate::string_utils::byte_index_to_char_index(&result.text, sel_start_byte);
            let sel_end_char =
                crate::string_utils::byte_index_to_char_index(&result.text, sel_end_byte);
            let start_cursor = self.char_pos_to_cursor(sel_start_char);
            let end_cursor = self.char_pos_to_cursor(sel_end_char);
            self.set_selection(Selection::new(start_cursor, end_cursor));
        } else {
            // Convert byte position to character position
            let cursor_char =
                crate::string_utils::byte_index_to_char_index(&result.text, result.cursor);
            let new_cursor = self.char_pos_to_cursor(cursor_char);
            self.set_cursor(new_cursor);
        }

        self.view
            .ensure_line_visible(self.primary_selection().head.line, self.buffer.line_count());

        self.content_dirty = true;
        true
    }

    /// Inserts text at all cursor positions with proper offset adjustment.
    ///
    /// Handles multi-cursor editing by processing cursors from end to start,
    /// ensuring that earlier cursor positions remain valid after each insertion.
    fn insert_text_at_all_cursors(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        // First delete any selections
        self.delete_selection();

        // Get all cursor positions and sort by char position (descending)
        let mut cursor_positions: Vec<(usize, usize)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let char_pos = InputHandler::cursor_to_char_pos(&self.buffer, &s.head);
                (i, char_pos)
            })
            .collect();

        // Sort by position descending (insert from end first)
        cursor_positions.sort_by(|a, b| b.1.cmp(&a.1));

        let text_chars = text.chars().count();
        let mut new_cursors: Vec<(usize, Cursor)> = Vec::with_capacity(self.selections.len());

        for (sel_idx, char_pos) in cursor_positions {
            // Insert text at this position
            self.buffer.insert(char_pos, text);

            // Calculate new cursor position after insertion
            let new_char_pos = char_pos + text_chars;
            let new_cursor = self.char_pos_to_cursor(new_char_pos);
            new_cursors.push((sel_idx, new_cursor));
        }

        // Update selections with new cursor positions
        for (sel_idx, new_cursor) in new_cursors {
            if let Some(sel) = self.selections.get_mut(sel_idx) {
                *sel = Selection::collapsed(new_cursor);
            }
        }

        self.merge_overlapping_selections();
        self.content_dirty = true;
        self.view
            .ensure_line_visible(self.primary_selection().head.line, self.buffer.line_count());
    }

    /// Performs backspace at all cursor positions with proper offset adjustment.
    ///
    /// Deletes the full grapheme cluster before each cursor, correctly handling
    /// multi-char clusters (emoji ZWJ, Bengali conjuncts, combining marks, etc.).
    fn backspace_at_all_cursors(&mut self) {
        if self.selections.iter().any(|s| s.is_range()) {
            self.delete_selection();
            return;
        }

        // Capture original char positions BEFORE any modifications.
        let original_positions: Vec<(usize, usize)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(idx, s)| (idx, InputHandler::cursor_to_char_pos(&self.buffer, &s.head)))
            .collect();

        // For each cursor compute the grapheme-aware deletion range:
        //   (delete_start_char_pos, delete_length)
        let mut delete_ranges: Vec<(usize, usize)> = Vec::new();
        for (_, char_pos) in &original_positions {
            if *char_pos == 0 {
                continue;
            }
            let cursor = self.char_pos_to_cursor(*char_pos);
            let grapheme_chars = if cursor.column > 0 {
                if let Some(line) = self.buffer.get_line(cursor.line) {
                    let stripped = grapheme::line_text_stripped(&line);
                    let prev = grapheme::prev_grapheme_boundary(stripped, cursor.column);
                    cursor.column - prev
                } else {
                    1
                }
            } else {
                1 // newline join — single char
            };
            let start = char_pos.saturating_sub(grapheme_chars);
            delete_ranges.push((start, grapheme_chars));
        }

        // Deduplicate overlapping ranges, sort descending to delete end-first.
        delete_ranges.sort_by(|a, b| b.0.cmp(&a.0));
        delete_ranges.dedup();

        for &(start, len) in &delete_ranges {
            let end = (start + len).min(self.buffer.len());
            if start < end {
                self.buffer.remove(start, end - start);
            }
        }

        // Sort ascending for offset calculation.
        delete_ranges.sort_by_key(|&(start, _)| start);

        let new_selections: Vec<Selection> = original_positions
            .iter()
            .map(|(_idx, original_pos)| {
                let total_deleted_before: usize = delete_ranges
                    .iter()
                    .filter(|(del_start, _)| *del_start < *original_pos)
                    .map(|(_, del_len)| del_len)
                    .sum();
                let new_char_pos = original_pos
                    .saturating_sub(total_deleted_before)
                    .min(self.buffer.len());
                Selection::collapsed(self.char_pos_to_cursor(new_char_pos))
            })
            .collect();

        self.selections = new_selections;
        self.merge_overlapping_selections();
        self.content_dirty = true;

        if !self.selections.is_empty() {
            let line = self
                .primary_selection()
                .head
                .line
                .min(self.buffer.line_count().saturating_sub(1));
            self.view
                .ensure_line_visible(line, self.buffer.line_count());
        }
    }

    /// Performs forward-delete at all cursor positions with proper offset adjustment.
    ///
    /// Deletes the full grapheme cluster at each cursor position.
    fn delete_at_all_cursors(&mut self) {
        if self.selections.iter().any(|s| s.is_range()) {
            self.delete_selection();
            return;
        }

        let original_positions: Vec<(usize, usize)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(idx, s)| (idx, InputHandler::cursor_to_char_pos(&self.buffer, &s.head)))
            .collect();

        // For each cursor compute the grapheme-aware deletion range at the cursor.
        let mut delete_ranges: Vec<(usize, usize)> = Vec::new();
        for (_, char_pos) in &original_positions {
            if *char_pos >= self.buffer.len() {
                continue;
            }
            let cursor = self.char_pos_to_cursor(*char_pos);
            let line_len = InputHandler::line_length(&self.buffer, cursor.line);
            let del_len = if cursor.column < line_len {
                if let Some(line) = self.buffer.get_line(cursor.line) {
                    let stripped = grapheme::line_text_stripped(&line);
                    grapheme::next_grapheme_boundary(stripped, cursor.column) - cursor.column
                } else {
                    1
                }
            } else {
                1 // newline join
            };
            delete_ranges.push((*char_pos, del_len));
        }

        delete_ranges.sort_by(|a, b| b.0.cmp(&a.0));
        delete_ranges.dedup();

        for &(start, len) in &delete_ranges {
            let end = (start + len).min(self.buffer.len());
            if start < end {
                self.buffer.remove(start, end - start);
            }
        }

        delete_ranges.sort_by_key(|&(start, _)| start);

        // Forward delete: cursor stays at same logical position, adjusted only
        // by deletions that occurred before it in the buffer.
        let new_selections: Vec<Selection> = original_positions
            .iter()
            .map(|(_idx, original_pos)| {
                let total_deleted_before: usize = delete_ranges
                    .iter()
                    .filter(|(del_start, _)| *del_start < *original_pos)
                    .map(|(_, del_len)| del_len)
                    .sum();
                let new_char_pos = original_pos
                    .saturating_sub(total_deleted_before)
                    .min(self.buffer.len());
                Selection::collapsed(self.char_pos_to_cursor(new_char_pos))
            })
            .collect();

        self.selections = new_selections;
        self.merge_overlapping_selections();
        self.content_dirty = true;

        if !self.selections.is_empty() {
            let line = self
                .primary_selection()
                .head
                .line
                .min(self.buffer.line_count().saturating_sub(1));
            self.view
                .ensure_line_visible(line, self.buffer.line_count());
        }
    }

    /// Moves all cursors in the specified direction.
    fn move_all_cursors(&mut self, key: egui::Key, modifiers: &egui::Modifiers) {
        let total_lines = self.buffer.line_count();
        let shift = modifiers.shift;

        for sel in &mut self.selections {
            let cursor = sel.head;
            let new_cursor = match key {
                egui::Key::ArrowLeft => {
                    if cursor.column > 0 {
                        let new_col = self
                            .buffer
                            .get_line(cursor.line)
                            .map(|l| {
                                let stripped = grapheme::line_text_stripped(&l);
                                grapheme::prev_grapheme_boundary(stripped, cursor.column)
                            })
                            .unwrap_or(cursor.column.saturating_sub(1));
                        Cursor::new(cursor.line, new_col)
                    } else if cursor.line > 0 {
                        let prev_line_len = self
                            .buffer
                            .get_line(cursor.line - 1)
                            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                            .unwrap_or(0);
                        Cursor::new(cursor.line - 1, prev_line_len)
                    } else {
                        cursor
                    }
                }
                egui::Key::ArrowRight => {
                    let line_len = self
                        .buffer
                        .get_line(cursor.line)
                        .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                        .unwrap_or(0);
                    if cursor.column < line_len {
                        let new_col = self
                            .buffer
                            .get_line(cursor.line)
                            .map(|l| {
                                let stripped = grapheme::line_text_stripped(&l);
                                grapheme::next_grapheme_boundary(stripped, cursor.column)
                            })
                            .unwrap_or(cursor.column + 1);
                        Cursor::new(cursor.line, new_col)
                    } else if cursor.line + 1 < total_lines {
                        Cursor::new(cursor.line + 1, 0)
                    } else {
                        cursor
                    }
                }
                egui::Key::ArrowUp => {
                    if cursor.line > 0 {
                        let prev_line_len = self
                            .buffer
                            .get_line(cursor.line - 1)
                            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                            .unwrap_or(0);
                        Cursor::new(cursor.line - 1, cursor.column.min(prev_line_len))
                    } else {
                        Cursor::new(0, 0)
                    }
                }
                egui::Key::ArrowDown => {
                    if cursor.line + 1 < total_lines {
                        let next_line_len = self
                            .buffer
                            .get_line(cursor.line + 1)
                            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                            .unwrap_or(0);
                        Cursor::new(cursor.line + 1, cursor.column.min(next_line_len))
                    } else {
                        let line_len = self
                            .buffer
                            .get_line(cursor.line)
                            .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                            .unwrap_or(0);
                        Cursor::new(cursor.line, line_len)
                    }
                }
                egui::Key::Home => Cursor::new(cursor.line, 0),
                egui::Key::End => {
                    let line_len = self
                        .buffer
                        .get_line(cursor.line)
                        .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                        .unwrap_or(0);
                    Cursor::new(cursor.line, line_len)
                }
                _ => cursor,
            };

            if shift {
                // Extend selection
                *sel = sel.with_head(new_cursor);
            } else {
                // Move cursor (collapse selection)
                *sel = Selection::collapsed(new_cursor);
            }
        }

        self.merge_overlapping_selections();
    }

    /// Converts a character position to a cursor (line, column).
    ///
    /// Handles out-of-bounds positions gracefully by clamping to valid ranges.
    fn char_pos_to_cursor(&self, char_pos: usize) -> Cursor {
        // Clamp char_pos to valid range to prevent panics
        let clamped_pos = char_pos.min(self.buffer.len());

        // Use try methods for safety
        let line = self
            .buffer
            .try_char_to_line(clamped_pos)
            .unwrap_or_else(|| self.buffer.line_count().saturating_sub(1));
        let line_start = self.buffer.try_line_to_char(line).unwrap_or(0);
        let column = clamped_pos.saturating_sub(line_start);
        Cursor::new(line, column)
    }

    /// Returns a reference to the view state.
    #[must_use]
    pub fn view(&self) -> &ViewState {
        &self.view
    }

    /// Returns a mutable reference to the view state.
    pub fn view_mut(&mut self) -> &mut ViewState {
        &mut self.view
    }

    /// Main egui widget method - renders the editor.
    ///
    /// This method:
    /// 1. Clears line cache if content changed
    /// 2. Calculates visible line range
    /// 3. Renders line number gutter
    /// 4. Renders text content using cached galleys (wrapped or unwrapped)
    ///
    /// # Arguments
    /// * `_ctx` - The egui context (for future use)
    /// * `ui` - The UI to render into
    ///
    /// # Returns
    /// An egui `Response` for interaction handling.
    pub fn ui(&mut self, _ctx: &Context, ui: &mut Ui) -> Response {
        // Check if fonts have changed (e.g., CJK fonts were lazily loaded)
        // If so, invalidate the cache to ensure text is re-rendered with new fonts
        let current_font_gen = fonts::font_generation();
        if current_font_gen != self.last_font_generation {
            log::debug!(
                "Font generation changed ({} -> {}), invalidating line cache",
                self.last_font_generation,
                current_font_gen
            );
            self.line_cache.invalidate();
            self.table_guide_cache.clear();
            self.last_font_generation = current_font_gen;
        }

        let current_zoom = ui.ctx().zoom_factor();
        if (current_zoom - self.last_zoom_factor).abs() > f32::EPSILON {
            self.line_cache.invalidate();
            self.table_guide_cache.clear();
            self.last_zoom_factor = current_zoom;
        }

        // Handle content changes with targeted cache invalidation.
        // We use invalidate_range (not full invalidate) to preserve cached
        // galleys for unchanged lines — only the edited line range is evicted.
        // CacheKey is content-hash-based, so unchanged text at any position
        // will still produce cache hits even after line-index shifts.
        if self.content_dirty {
            if let Some((dirty_start, dirty_end)) = self.dirty_range.take() {
                self.line_cache.invalidate_range(dirty_start, dirty_end);
            } else {
                // No specific range known (e.g. mark_dirty() was called) — full clear
                self.line_cache.invalidate();
                self.table_guide_cache.clear();
            }
            let current_lines = self.buffer.line_count();
            self.view.truncate_wrap_info(current_lines);
            self.content_dirty = false;
        }

        // Get available space
        let available_size = ui.available_size();

        // Calculate font based on font_family setting and use fixed line height for base calculations
        let font_family = fonts::get_base_font_family(&self.font_family);
        let font_id = FontId::new(self.font_size, font_family);
        let base_line_height = FIXED_LINE_HEIGHT;
        self.view.set_line_height(base_line_height);
        self.view.update_viewport(available_size.y);

        // Ensure scroll position is valid after viewport update (prevents edge cases)
        let total_lines = self.buffer.line_count();
        self.view.clamp_scroll_position(total_lines);

        // Configure for large file optimization - this sets uniform height mode
        // for files > 100k lines to avoid O(N) performance overhead
        self.view.configure_for_file_size(total_lines);

        // When uniform heights are active (very large files), word wrap is
        // force-disabled for this frame. Wrapped galleys can be much taller
        // than the uniform line_height, which causes the line overlap
        // regression that previously forced this feature to be disabled.
        let effective_wrap_enabled = self.wrap_enabled && !self.view.uses_uniform_heights();

        // Calculate gutter width based on what's enabled (line numbers and/or fold indicators)
        let gutter_width = gutter::calculate_gutter_width(
            ui,
            &font_id,
            self.buffer.line_count(),
            self.show_line_numbers,
            self.show_fold_indicators,
        );
        // Only add gutter padding if gutter has content
        let gutter_padding = if gutter_width > 0.0 {
            gutter::GUTTER_PADDING
        } else {
            0.0
        };
        let text_area_width = (available_size.x - gutter_width - gutter_padding).max(100.0);

        // Calculate effective wrap width (considering max_wrap_width setting)
        // Use the smaller of text_area_width and max_wrap_width (if set)
        let effective_wrap_width = if let Some(max_width) = self.max_wrap_width {
            log::debug!(
                "Word wrap: text_area_width={:.1}, max_wrap_width={:.1}, effective={:.1}",
                text_area_width,
                max_width,
                text_area_width.min(max_width)
            );
            text_area_width.min(max_width)
        } else {
            log::debug!(
                "Word wrap: text_area_width={:.1}, no max_wrap_width set",
                text_area_width
            );
            text_area_width
        };

        // Configure word wrap if enabled, otherwise ensure wrap state is cleared.
        // This prevents stale wrap_info from causing y-position mismatches between
        // rendering (which uses get_line_height()) and click detection (which uses pixel_to_line()).
        // Note: effective_wrap_enabled is false for large files in uniform-height mode.
        if effective_wrap_enabled {
            self.view.enable_wrap(effective_wrap_width);
        } else if self.view.is_wrap_enabled() {
            self.view.disable_wrap();
        }

        // Get visible line range (total_lines already calculated above)
        let (start_line, end_line) = self.view.get_visible_line_range(total_lines);

        // Scale cache capacity to the visible window so large viewports or
        // files keep a healthy hit rate without unbounded growth.
        let visible_lines = end_line.saturating_sub(start_line);
        self.line_cache.update_capacity(visible_lines);

        // Allocate painter for the entire editor area
        let desired_size = Vec2::new(available_size.x, available_size.y);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());
        let rect = response.rect;

        // Handle cursor blink timing - only blink when editor has focus
        // When not focused, cursor is hidden (cursor_visible = false when unfocused)
        if response.has_focus() {
            // Reset blink when focus is gained (cursor appears immediately)
            if response.gained_focus() {
                self.reset_cursor_blink();
            }

            let blink_interval =
                std::time::Duration::from_millis(cursor_render::CURSOR_BLINK_INTERVAL_MS);
            let elapsed = self.cursor_blink_instant.elapsed();
            if elapsed >= blink_interval {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_instant = std::time::Instant::now();
            }
            // Schedule repaint for next blink toggle
            let time_until_toggle = blink_interval.saturating_sub(elapsed);
            _ctx.request_repaint_after(time_until_toggle);
        } else {
            // Not focused - hide cursor (don't show blinking cursor in unfocused editor)
            self.cursor_visible = false;
        }

        // Get colors from the UI style
        let text_color = ui.visuals().text_color();
        let gutter_text_color = ui.visuals().weak_text_color();
        let gutter_bg_color = ui.visuals().extreme_bg_color;
        let separator_color = ui.visuals().widgets.noninteractive.bg_stroke.color;

        // Draw gutter background and separator (only if gutter has content)
        if gutter_width > 0.0 {
            gutter::render_gutter_background(
                &painter,
                rect,
                gutter_width,
                gutter_bg_color,
                separator_color,
            );
        }

        // Render visible lines
        // Add content_offset_x for centering (Zen Mode)
        let text_start_x = rect.min.x + gutter_width + gutter_padding + self.content_offset_x;

        // Track max line width for horizontal scrollbar (only when word wrap is off)
        let mut max_line_width: f32 = 0.0;

        // Calculate y-positions incrementally for accurate placement
        // Start from first_visible_line and work backwards/forwards
        // IMPORTANT: Hidden lines (inside collapsed folds) should NOT add to the y position
        let first_visible = self.view.first_visible_line();

        // Calculate the y position where first_visible_line starts
        let first_visible_y = rect.min.y - self.view.scroll_offset_y();

        // Build a map of y-positions for all lines we need to render
        // This ensures consistent positioning regardless of cached state
        let mut line_y_positions: Vec<f32> =
            Vec::with_capacity(end_line.saturating_sub(start_line));

        // Calculate y for lines from start_line to end_line
        // First, calculate cumulative heights from first_visible backwards to start_line
        // Skip hidden lines - they don't contribute to layout height
        let mut y = first_visible_y;
        for line_idx in (start_line..first_visible).rev() {
            // Only subtract height for visible lines
            if !self.fold_state.is_line_hidden(line_idx) {
                let height = self.view.get_line_height(line_idx);
                y -= height;
            }
        }
        // Now y is the position for start_line (accounting for hidden lines)

        // Calculate positions for all lines
        // Hidden lines get the same y-position as the next visible line (they won't be rendered anyway)
        for line_idx in start_line..end_line {
            if line_idx < first_visible {
                // We already calculated this position above
                line_y_positions.push(y);
                // Only add height for visible lines
                if !self.fold_state.is_line_hidden(line_idx) {
                    y += self.view.get_line_height(line_idx);
                }
            } else if line_idx == first_visible {
                line_y_positions.push(first_visible_y);
                // Only add height if this line is visible
                if !self.fold_state.is_line_hidden(line_idx) {
                    y = first_visible_y + self.view.get_line_height(line_idx);
                } else {
                    y = first_visible_y;
                }
            } else {
                line_y_positions.push(y);
                // Only add height for visible lines
                if !self.fold_state.is_line_hidden(line_idx) {
                    y += self.view.get_line_height(line_idx);
                }
            }
        }

        let shows_table_guides = self
            .syntax_language
            .as_deref()
            .is_none_or(|lang| lang == "md" || lang == "markdown");

        if shows_table_guides {
            let guide_base = ui.visuals().weak_text_color();
            let guide_color = Color32::from_rgba_unmultiplied(
                guide_base.r(),
                guide_base.g(),
                guide_base.b(),
                80,
            );
            let line_y_map: Vec<(usize, f32)> = (start_line..end_line)
                .enumerate()
                .map(|(i, line_idx)| (line_idx, line_y_positions[i]))
                .collect();
            let total_lines = self.buffer.line_count();
            table_guides::render_table_guides(
                &mut self.table_guide_cache,
                &painter,
                &|idx| {
                    self.buffer
                        .get_line(idx)
                        .map(|s| s.trim_end_matches(['\r', '\n']).to_string())
                },
                total_lines,
                start_line,
                end_line,
                &line_y_map,
                text_start_x,
                self.view.horizontal_scroll(),
                &font_id,
                &self.line_cache,
                guide_color,
                effective_wrap_enabled,
                &|line_idx| self.view.get_line_height(line_idx),
            );
        }

        for (i, line_idx) in (start_line..end_line).enumerate() {
            let y = line_y_positions[i];
            let line_height = self.view.get_line_height(line_idx);

            // Skip if line is above visible area
            if y + line_height < rect.min.y {
                continue;
            }
            // Stop if line is below visible area
            if y > rect.max.y {
                break;
            }

            // Skip hidden lines (inside collapsed folds)
            if self.fold_state.is_line_hidden(line_idx) {
                continue;
            }

            // Render fold indicator if this line starts a fold region and setting is enabled
            // Fold indicators are rendered at the left edge of the gutter
            if self.show_fold_indicators {
                if let Some(region) = self.fold_state.region_at_line(line_idx) {
                    gutter::render_fold_indicator(
                        &painter,
                        rect.min.x + 2.0, // Left edge of gutter
                        y,
                        line_height,
                        region.collapsed,
                        gutter_text_color,
                    );
                }
            }

            // Render line number in gutter (only once per logical line)
            // Line numbers are right-aligned, offset by fold indicator width if fold indicators are shown
            if self.show_line_numbers {
                let line_num_x = if self.show_fold_indicators {
                    rect.min.x + gutter::FOLD_INDICATOR_WIDTH
                } else {
                    rect.min.x
                };
                let line_num_width = if self.show_fold_indicators {
                    gutter_width - gutter::FOLD_INDICATOR_WIDTH - 4.0
                } else {
                    gutter_width - 4.0
                };
                gutter::render_line_number(
                    &painter,
                    line_idx,
                    line_num_x,
                    y,
                    line_num_width,
                    &font_id,
                    gutter_text_color,
                );
            }

            // Render line content
            if let Some(line_content) = self.buffer.get_line(line_idx) {
                let display_content = line_content.trim_end_matches(['\r', '\n']);

                // Track this line's key so invalidate_range can evict it later.
                self.line_cache
                    .register_line(line_idx, display_content, &font_id, text_color);

                // Check if syntax highlighting is enabled for this line
                let use_syntax = self.syntax_enabled && self.syntax_language.is_some();

                if effective_wrap_enabled {
                    let galley = if use_syntax {
                        // Syntax-highlighted wrapped galley
                        // Check cache first to avoid expensive highlighting on every frame
                        let wrap_width_opt = Some(effective_wrap_width);
                        if let Some(cached) = self.line_cache.get_cached_highlighted_galley(
                            display_content,
                            &font_id,
                            text_color,
                            self.syntax_theme_hash,
                            wrap_width_opt,
                        ) {
                            cached
                        } else {
                            let lang = self.syntax_language.as_deref().unwrap();
                            let segments = self.highlight_line(display_content, lang);
                            self.line_cache.get_galley_highlighted(
                                display_content,
                                &segments,
                                &painter,
                                font_id.clone(),
                                text_color,
                                self.syntax_theme_hash,
                                wrap_width_opt,
                            )
                        }
                    } else {
                        // Plain wrapped galley
                        self.line_cache.get_galley_wrapped(
                            display_content,
                            &painter,
                            font_id.clone(),
                            text_color,
                            effective_wrap_width,
                        )
                    };

                    // Update wrap info for this line
                    let visual_rows = galley.rows.len();
                    let height = galley.size().y;
                    self.view.set_line_wrap_info(line_idx, visual_rows, height);

                    // Draw the wrapped galley
                    painter.galley(egui::Pos2::new(text_start_x, y), galley, text_color);
                } else {
                    // Apply horizontal scroll offset for non-wrapped mode
                    let x = text_start_x - self.view.horizontal_scroll();

                    if use_syntax {
                        // Syntax-highlighted non-wrapped galley
                        // Check cache first to avoid expensive highlighting on every frame
                        let galley = if let Some(cached) =
                            self.line_cache.get_cached_highlighted_galley(
                                display_content,
                                &font_id,
                                text_color,
                                self.syntax_theme_hash,
                                None,
                            ) {
                            cached
                        } else {
                            let lang = self.syntax_language.as_deref().unwrap();
                            let segments = self.highlight_line(display_content, lang);
                            self.line_cache.get_galley_highlighted(
                                display_content,
                                &segments,
                                &painter,
                                font_id.clone(),
                                text_color,
                                self.syntax_theme_hash,
                                None, // No wrap
                            )
                        };
                        // Track max line width for horizontal scrollbar
                        max_line_width = max_line_width.max(galley.size().x);
                        painter.galley(egui::Pos2::new(x, y), galley, text_color);
                    } else if let Some(shaped) = self.line_cache.get_shaped_line(
                        display_content,
                        &painter,
                        font_id.clone(),
                        text_color,
                    ) {
                        for cg in &shaped.clusters {
                            painter.galley(
                                egui::Pos2::new(x + cg.x_offset, y),
                                Arc::clone(&cg.galley),
                                text_color,
                            );
                        }
                        max_line_width = max_line_width.max(shaped.total_width);
                    } else {
                        // Plain non-wrapped galley (Latin / fallback)
                        let galley = text_render::render_line(
                            &painter,
                            &mut self.line_cache,
                            &line_content,
                            x,
                            y,
                            font_id.clone(),
                            text_color,
                        );
                        max_line_width = max_line_width.max(galley.size().x);
                    }
                }
            }
        }

        // Rebuild height cache after updating wrap info (only when wrap_info changed)
        // and advance scrollbar smoothing every frame for smooth transitions
        if effective_wrap_enabled {
            self.view.rebuild_height_cache(total_lines);
            self.view.advance_scrollbar_smoothing(total_lines);
        }

        // Render selection backgrounds (after text, before cursors).
        //
        // The overlay is drawn *on top* of the text galley (moving the draw
        // order is risky because per-line wrap heights are computed inside
        // the text loop), so we keep the fill partly transparent so syntax-
        // coloured glyphs stay legible through the highlight. This means the
        // theme's `selection.bg_fill` must be saturated enough to remain
        // visible at ~40% alpha on both light and dark backgrounds — see
        // `BaseColors::light()/dark()` for the values chosen to satisfy this
        // (issue #121).
        if self.has_any_selection() {
            let selection_base = ui.visuals().selection.bg_fill;
            let selection_color = Color32::from_rgba_unmultiplied(
                selection_base.r(),
                selection_base.g(),
                selection_base.b(),
                100, // ~40% alpha — lets text shine through in both themes
            );
            self.render_all_selections(
                &painter,
                rect,
                text_start_x,
                &font_id,
                effective_wrap_width,
                start_line,
                end_line,
                selection_color,
            );
        }

        // Render search match highlights
        if !self.search_matches.is_empty() {
            self.render_search_highlights(
                &painter,
                rect,
                text_start_x,
                &font_id,
                effective_wrap_width,
                start_line,
                end_line,
                ui.visuals().dark_mode,
            );
        }

        // Render bracket matching highlights
        if self.bracket_matching_enabled {
            self.render_bracket_matching(
                &painter,
                rect,
                text_start_x,
                &font_id,
                effective_wrap_width,
                start_line,
                end_line,
                ui.visuals().dark_mode,
            );
        }

        // Render diagnostic squiggles (errors/warnings from LSP)
        if !self.diagnostics.is_empty() {
            self.render_diagnostic_squiggles(
                &painter,
                rect,
                text_start_x,
                &font_id,
                effective_wrap_width,
                start_line,
                end_line,
                &line_y_positions,
                ui.visuals().dark_mode,
            );
        }

        // Handle scroll-to-match for search highlights
        if self.scroll_to_search_match && !self.search_matches.is_empty() {
            if let Some(search_match) = self.search_matches.get(self.current_search_match) {
                // Use pre-computed line number and center in viewport for better visibility
                self.view
                    .scroll_to_center_line(search_match.line, total_lines);
            }
            self.scroll_to_search_match = false;
        }

        // Render all cursors (multi-cursor support)
        // Use text color for cursor to match theme (blends well in both light/dark modes)
        let cursor_color = text_color;
        let primary_cursor = self.primary_selection().head;
        let mut cursor_rect_for_ime: Option<egui::Rect> = None;

        for (idx, sel) in self.selections.iter().enumerate() {
            let cursor = sel.head;
            if cursor.line >= start_line && cursor.line < end_line {
                // Use the pre-calculated y position from our line_y_positions array
                let cursor_y = line_y_positions[cursor.line - start_line];

                cursor_render::render_cursor(
                    &painter,
                    &self.buffer,
                    &cursor,
                    &self.view,
                    &font_id,
                    text_start_x,
                    cursor_y,
                    effective_wrap_width,
                    cursor_color,
                    self.cursor_visible,
                );

                // Calculate cursor position for IME (use primary cursor only)
                if idx == self.primary_selection_index {
                    let cursor_x =
                        self.calculate_cursor_x(&cursor, &font_id, text_start_x, &painter);
                    let line_height = self.view.get_line_height(cursor.line);
                    cursor_rect_for_ime = Some(egui::Rect::from_min_size(
                        egui::Pos2::new(cursor_x, cursor_y),
                        egui::Vec2::new(2.0, line_height),
                    ));
                }
            }
        }

        // Primary cursor reference for IME preedit
        let cursor = primary_cursor;

        // Render IME preedit (composition) text
        if let Some(ref preedit_text) = self.ime_preedit {
            if cursor.line >= start_line && cursor.line < end_line && !preedit_text.is_empty() {
                let cursor_y = line_y_positions[cursor.line - start_line];
                let cursor_x = self.calculate_cursor_x(&cursor, &font_id, text_start_x, &painter);

                // Create a galley for the preedit text
                let preedit_galley =
                    painter.layout_no_wrap(preedit_text.clone(), font_id.clone(), text_color);

                let preedit_width = preedit_galley.size().x;
                let preedit_height = preedit_galley.size().y;

                // Draw preedit background (subtle highlight)
                let preedit_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(cursor_x, cursor_y),
                    egui::Vec2::new(preedit_width, preedit_height),
                );
                let ime_bg_color = if ui.visuals().dark_mode {
                    Color32::from_rgba_unmultiplied(100, 100, 150, 40)
                } else {
                    Color32::from_rgba_unmultiplied(100, 100, 200, 30)
                };
                painter.rect_filled(preedit_rect, 0.0, ime_bg_color);

                // Draw preedit text
                painter.galley(
                    egui::Pos2::new(cursor_x, cursor_y),
                    preedit_galley,
                    text_color,
                );

                // Draw underline to indicate composition in progress
                let underline_y = cursor_y + preedit_height - 2.0;
                let underline_color = if ui.visuals().dark_mode {
                    Color32::from_rgb(150, 150, 255)
                } else {
                    Color32::from_rgb(80, 80, 200)
                };
                painter.line_segment(
                    [
                        egui::Pos2::new(cursor_x, underline_y),
                        egui::Pos2::new(cursor_x + preedit_width, underline_y),
                    ],
                    Stroke::new(1.5, underline_color),
                );

                // Update cursor rect to be at end of preedit for IME candidate window positioning
                cursor_rect_for_ime = Some(egui::Rect::from_min_size(
                    egui::Pos2::new(cursor_x + preedit_width, cursor_y),
                    egui::Vec2::new(2.0, preedit_height),
                ));
            }
        }

        // Set IME cursor area for candidate window positioning (screen space for the OS).
        // Same as egui TextEdit: apply the layer's TSTransform so IME works with scaled layers
        // and custom chrome (local widget coords are not sufficient on Windows).
        if let Some(cursor_rect) = cursor_rect_for_ime {
            let layer_transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            ui.ctx().output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: layer_transform * rect,
                    cursor_rect: layer_transform * cursor_rect,
                });
            });
        }

        // Handle mouse interactions for cursor and selection
        //
        // Key insight: egui's drag_started() fires AFTER the mouse has moved (to distinguish
        // from clicks), so interact_pointer_pos() at that moment is NOT the original click
        // position. We need to capture the position when the button first goes down.

        // Pre-calculate fold indicator area boundary for click detection
        let fold_indicator_area_end = rect.min.x + gutter::FOLD_INDICATOR_WIDTH;

        // Helper to check if a position is in the fold indicator area
        let is_in_fold_indicator_area = |pos: egui::Pos2| -> bool {
            self.show_fold_indicators && pos.x < fold_indicator_area_end
        };

        // Step 1: Capture the initial press position (before egui decides if it's a drag)
        // Skip capture if clicking in fold indicator area (fold clicks shouldn't move cursor)
        if response.is_pointer_button_down_on() && self.drag_start_cursor.is_none() {
            if let Some(pos) = response.interact_pointer_pos() {
                if !is_in_fold_indicator_area(pos) {
                    let press_cursor = self.pos_to_cursor(
                        pos,
                        rect,
                        text_start_x,
                        &font_id,
                        effective_wrap_width,
                        total_lines,
                        ui,
                    );
                    self.drag_start_cursor = Some(press_cursor);
                }
            }
        }

        // Step 2: When drag actually starts, use our stored position as the anchor
        // Only if we have a drag_start_cursor (i.e., not a fold indicator click)
        if response.drag_started() {
            if let Some(anchor_cursor) = self.drag_start_cursor {
                response.request_focus();
                self.reset_cursor_blink(); // Make cursor visible on click/drag

                // Check if shift is held (extend selection from existing anchor)
                let shift_held = ui.input(|i| i.modifiers.shift);

                if shift_held {
                    // Extend from existing anchor
                    *self.primary_selection_mut() =
                        self.primary_selection().with_head(anchor_cursor);
                } else {
                    // New selection starting point - use our captured press position
                    // Also clear extra cursors when starting a new drag
                    self.set_cursor(anchor_cursor);
                }

                // Reset click count for drag
                self.click_count = 1;
            }
        }

        // Step 3: Handle ongoing drag - update head while preserving anchor
        // Only if we have a drag_start_cursor (i.e., not a fold indicator click)
        if response.dragged() && self.drag_start_cursor.is_some() {
            if let Some(pos) = response.interact_pointer_pos() {
                let drag_cursor = self.pos_to_cursor(
                    pos,
                    rect,
                    text_start_x,
                    &font_id,
                    effective_wrap_width,
                    total_lines,
                    ui,
                );
                // Update only the head, anchor stays where we initially pressed
                *self.primary_selection_mut() = self.primary_selection().with_head(drag_cursor);
            }
        }

        // Step 4: Clear the drag start cursor when button is released
        if !response.is_pointer_button_down_on() {
            self.drag_start_cursor = None;
        }

        // Handle click (only fires when drag did NOT happen)
        if response.clicked() {
            response.request_focus();
            self.reset_cursor_blink(); // Make cursor visible on click

            if let Some(pos) = response.interact_pointer_pos() {
                // Check if click is in the fold indicator area (left side of gutter)
                // Note: fold_indicator_area_end is already defined above
                let mut handled_fold_click = false;

                if self.show_fold_indicators && pos.x < fold_indicator_area_end {
                    // Click is in the fold indicator area - check if there's a fold on this line
                    let clicked_line = self.y_to_line(pos.y, rect.min.y, total_lines);
                    log::debug!(
                        "Fold indicator click: pos.x={:.1}, area_end={:.1}, line={}, has_region={}",
                        pos.x,
                        fold_indicator_area_end,
                        clicked_line,
                        self.fold_state.region_at_line(clicked_line).is_some()
                    );
                    if self.fold_state.region_at_line(clicked_line).is_some() {
                        // Toggle the fold
                        let toggled = self.fold_state.toggle_at_line(clicked_line);
                        log::debug!("Fold toggled: line={}, result={}", clicked_line, toggled);
                        self.fold_toggle_line = Some(clicked_line);
                        handled_fold_click = true;
                    }
                }

                // Only handle cursor positioning if we didn't handle a fold toggle
                if !handled_fold_click {
                    let clicked_cursor = self.pos_to_cursor(
                        pos,
                        rect,
                        text_start_x,
                        &font_id,
                        effective_wrap_width,
                        total_lines,
                        ui,
                    );

                    // Detect double/triple click
                    let now = std::time::Instant::now();
                    let is_same_pos = self.last_click_pos.map_or(false, |last| {
                        last.line == clicked_cursor.line
                            && (last.column as i32 - clicked_cursor.column as i32).abs() <= 2
                    });

                    let time_since_last = self
                        .last_click_time
                        .map_or(std::time::Duration::from_secs(10), |t| {
                            now.duration_since(t)
                        });

                    if is_same_pos && time_since_last < std::time::Duration::from_millis(400) {
                        self.click_count = (self.click_count % 3) + 1;
                    } else {
                        self.click_count = 1;
                    }

                    self.last_click_time = Some(now);
                    self.last_click_pos = Some(clicked_cursor);

                    // Check modifier keys
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

                    match self.click_count {
                        1 => {
                            // Single click
                            if ctrl_held && !shift_held {
                                // Ctrl+Click: add a new cursor (multi-cursor)
                                self.add_cursor(clicked_cursor);
                            } else if shift_held {
                                // Shift+Click: extend selection
                                *self.primary_selection_mut() =
                                    self.primary_selection().with_head(clicked_cursor);
                            } else {
                                // Regular click: new cursor position, clear extra cursors
                                self.set_cursor(clicked_cursor);
                            }
                        }
                        2 => {
                            // Double click: select word (clears extra cursors)
                            let (word_start, word_end) = self.find_word_boundaries(clicked_cursor);
                            self.set_selection(Selection::new(word_start, word_end));
                        }
                        3 => {
                            // Triple click: select line (clears extra cursors)
                            let line_start = Cursor::new(clicked_cursor.line, 0);
                            let line_end = if clicked_cursor.line + 1 < self.buffer.line_count() {
                                Cursor::new(clicked_cursor.line + 1, 0)
                            } else {
                                let line_len = self
                                    .buffer
                                    .get_line(clicked_cursor.line)
                                    .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                                    .unwrap_or(0);
                                Cursor::new(clicked_cursor.line, line_len)
                            };
                            self.set_selection(Selection::new(line_start, line_end));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Diagnostic hover tooltips
        if !self.diagnostics.is_empty() {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(hover_pos) {
                    let hover_cursor = self.pos_to_cursor(
                        hover_pos,
                        rect,
                        text_start_x,
                        &font_id,
                        effective_wrap_width,
                        total_lines,
                        ui,
                    );
                    let mut tooltip_parts: Vec<String> = Vec::new();
                    for diag in &self.diagnostics {
                        if self.cursor_in_diagnostic_range(&hover_cursor, diag) {
                            let sev = match diag.severity {
                                crate::lsp::state::DiagnosticSeverity::Error => "error",
                                crate::lsp::state::DiagnosticSeverity::Warning => "warning",
                                crate::lsp::state::DiagnosticSeverity::Information => "info",
                                crate::lsp::state::DiagnosticSeverity::Hint => "hint",
                            };
                            let src = diag.source.as_deref().unwrap_or("");
                            if src.is_empty() {
                                tooltip_parts.push(format!("[{}] {}", sev, diag.message));
                            } else {
                                tooltip_parts.push(format!("[{}/{}] {}", sev, src, diag.message));
                            }
                        }
                    }
                    if !tooltip_parts.is_empty() {
                        egui::Tooltip::always_open(
                            ui.ctx().clone(),
                            ui.layer_id(),
                            egui::Id::new("diag_tooltip"),
                            egui::PopupAnchor::Pointer,
                        )
                        .show(|ui| {
                            for part in &tooltip_parts {
                                ui.label(part);
                            }
                        });
                    }
                }
            }
        }

        // Handle mouse wheel scrolling when hovering (doesn't require focus)
        // Note: We use explicit pointer position check instead of response.hovered() because
        // smooth_scroll_delta is a global value that persists when scrolling in other panels
        // (like file tree or preview). Checking rect.contains(pointer_pos) ensures we only
        // process scroll events when the mouse is actually over this editor's area.
        let pointer_over_editor = ui
            .input(|i| i.pointer.hover_pos())
            .map(|pos| rect.contains(pos))
            .unwrap_or(false);

        if pointer_over_editor {
            let ctrl_scroll_zoom: Option<bool> = ui.input(|i| {
                if !i.modifiers.command {
                    return None;
                }
                for event in &i.events {
                    if let egui::Event::MouseWheel { delta, .. } = event {
                        if delta.y.abs() > 0.01 {
                            return Some(delta.y > 0.0);
                        }
                    }
                }
                None
            });

            if let Some(zoom_in) = ctrl_scroll_zoom {
                if zoom_in {
                    egui::gui_zoom::zoom_in(ui.ctx());
                } else {
                    egui::gui_zoom::zoom_out(ui.ctx());
                }
            } else {
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                if scroll_delta.y.abs() > 0.01 {
                    // smooth_scroll_delta is already in points (pixel-equivalent units)
                    // Just apply sensitivity multiplier (scroll_lines = 3.0 is typical)
                    let scroll_lines = 3.0;
                    let scroll_amount = -scroll_delta.y * scroll_lines;
                    self.view.scroll_by(scroll_amount, total_lines);
                }
            }
        }

        // Handle keyboard input when focused
        if response.has_focus() {
            // Lock focus to prevent arrow keys from navigating to other widgets
            // This is essential for text editors - we want arrow keys to move the cursor,
            // not move focus to buttons/panels
            let event_filter = EventFilter {
                horizontal_arrows: true, // Capture left/right arrow keys
                vertical_arrows: true,   // Capture up/down arrow keys
                tab: true, // Capture tab to insert tab character instead of focus cycling
                escape: true, // Capture escape (we might use it later)
            };
            ui.memory_mut(|m| m.set_focus_lock_filter(response.id, event_filter));

            let events: Vec<egui::Event> = ui.input(|i| i.events.clone());

            for event in &events {
                // Skip MouseWheel events when pointer is not over the editor
                // or when Ctrl is held (Ctrl+scroll = zoom, handled above).
                if let egui::Event::MouseWheel { modifiers, .. } = event {
                    if !pointer_over_editor || modifiers.command {
                        continue;
                    }
                }

                // Handle IME events first (for CJK input)
                if let egui::Event::Ime(ime_event) = event {
                    match ime_event {
                        ImeEvent::Enabled => {
                            self.ime_enabled = true;
                            self.ime_cursor_range = Some(self.primary_selection());
                            continue;
                        }
                        ImeEvent::Preedit(text_mark) => {
                            if text_mark == "\n" || text_mark == "\r" {
                                continue;
                            }

                            if text_mark.is_empty() {
                                // Empty preedit = composition cancelled (backspace/escape during IME)
                                self.ime_preedit = None;
                            } else {
                                // Store preedit text for rendering
                                self.ime_preedit = Some(text_mark.clone());
                            }
                            continue;
                        }
                        ImeEvent::Commit(prediction) => {
                            if prediction == "\n" || prediction == "\r" {
                                continue;
                            }

                            self.ime_enabled = false;
                            self.ime_preedit = None;

                            if !prediction.is_empty() {
                                // Store committed text for CJK font loading check
                                self.ime_committed_text = Some(prediction.clone());

                                // Delete selection first if any
                                self.delete_selection();

                                // Insert committed text at primary cursor
                                let mut cursor = self.primary_selection().head;
                                let start_line = cursor.line;
                                super::input::keyboard::insert_text(
                                    &mut self.buffer,
                                    &mut cursor,
                                    prediction,
                                );
                                *self.primary_selection_mut() = Selection::collapsed(cursor);
                                self.mark_lines_dirty(start_line, cursor.line);
                                self.reset_cursor_blink();
                                self.view
                                    .ensure_line_visible(cursor.line, self.buffer.line_count());
                            }
                            continue;
                        }
                        ImeEvent::Disabled => {
                            self.ime_enabled = false;
                            self.ime_preedit = None;
                            continue;
                        }
                    }
                }

                // Skip keyboard and text events during active IME composition.
                // When composing (e.g., Chinese pinyin), keys like Backspace modify the
                // composition buffer inside the IME — they must not also modify editor text.
                if self.ime_enabled {
                    if matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)) {
                        continue;
                    }
                }

                // Handle egui's Copy/Cut events (generated by OS or egui itself)
                match event {
                    egui::Event::Copy => {
                        if self.has_any_selection() {
                            let text = self.selected_text();
                            ui.copy_text(text);
                        }
                        continue;
                    }
                    egui::Event::Cut => {
                        if self.has_any_selection() {
                            let dirty_span = self.selection_dirty_line_span();
                            let text = self.selected_text();
                            ui.copy_text(text);
                            self.delete_selection();
                            if let Some((start_line, end_line)) = dirty_span {
                                self.mark_lines_dirty(start_line, end_line);
                            }
                            self.reset_cursor_blink();
                        }
                        continue;
                    }
                    _ => {}
                }

                // Handle clipboard operations via Key events (fallback)
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    let ctrl_or_cmd = modifiers.ctrl || modifiers.command;

                    // Handle Escape (without modifiers) - clear extra cursors
                    if *key == egui::Key::Escape
                        && !ctrl_or_cmd
                        && !modifiers.shift
                        && !modifiers.alt
                    {
                        if self.has_multiple_cursors() {
                            self.clear_extra_cursors();
                            continue;
                        }
                    }

                    if ctrl_or_cmd {
                        match key {
                            egui::Key::A => {
                                // Select all
                                self.select_all();
                                continue;
                            }
                            egui::Key::C => {
                                // Copy - fallback if Event::Copy wasn't received
                                if self.has_any_selection() {
                                    let text = self.selected_text();
                                    ui.copy_text(text);
                                }
                                continue;
                            }
                            egui::Key::X => {
                                // Cut - fallback if Event::Cut wasn't received
                                if self.has_any_selection() {
                                    let dirty_span = self.selection_dirty_line_span();
                                    let text = self.selected_text();
                                    ui.copy_text(text);
                                    self.delete_selection();
                                    if let Some((start_line, end_line)) = dirty_span {
                                        self.mark_lines_dirty(start_line, end_line);
                                    }
                                    self.reset_cursor_blink();
                                    self.view.ensure_line_visible(
                                        self.primary_selection().head.line,
                                        self.buffer.line_count(),
                                    );
                                }
                                continue;
                            }
                            egui::Key::V => {
                                // Paste - handled via egui::Event::Paste below
                                // But we can also check clipboard directly
                                continue;
                            }
                            _ => {}
                        }
                    }
                }

                // ── Vim mode interception ─────────────────────────────────────
                // When Vim mode is active, route key events through VimState first.
                // In Normal/Visual mode most keys are consumed by Vim; in Insert
                // mode, only Escape is consumed (everything else passes through).
                if self.vim_mode_enabled {
                    // Vim intercepts Key events
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    {
                        let idx = self
                            .primary_selection_index
                            .min(self.selections.len().saturating_sub(1));
                        let mut sel = self
                            .selections
                            .get(idx)
                            .copied()
                            .unwrap_or_else(Selection::start);

                        let vim_result = self.vim_state.handle_key(
                            *key,
                            modifiers,
                            &mut self.buffer,
                            &mut sel,
                            &mut self.view,
                        );

                        if let Some(s) = self.selections.get_mut(idx) {
                            *s = sel;
                        }

                        match vim_result {
                            super::vim::VimKeyResult::Handled(result) => {
                                match result {
                                    InputResult::TextChanged => {
                                        let line = self.primary_selection().head.line;
                                        self.mark_lines_dirty(line, line);
                                        self.reset_cursor_blink();
                                        self.view
                                            .ensure_line_visible(line, self.buffer.line_count());
                                    }
                                    InputResult::CursorMoved => {
                                        self.reset_cursor_blink();
                                        self.view.ensure_line_visible(
                                            self.primary_selection().head.line,
                                            self.buffer.line_count(),
                                        );
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                            super::vim::VimKeyResult::Consumed => {
                                continue;
                            }
                            super::vim::VimKeyResult::Passthrough => {
                                // Fall through to normal handling below
                            }
                        }
                    }

                    // In Normal/Visual mode, suppress text insertion events
                    if let egui::Event::Text(_) = event {
                        if !self.vim_state.should_insert_text() {
                            continue;
                        }
                    }
                }

                // Handle paste event - multi-cursor paste
                if let egui::Event::Paste(text) = event {
                    self.insert_text_at_all_cursors(text);
                    continue;
                }

                // Handle text input event - multi-cursor text insertion
                if let egui::Event::Text(text) = event {
                    if !text.is_empty() && text != "\n" && text != "\r" {
                        // Check for auto-close brackets (single character only)
                        if text.chars().count() == 1 {
                            let ch = text.chars().next().unwrap();
                            if self.handle_auto_close(ch) {
                                continue;
                            }
                        }
                        self.insert_text_at_all_cursors(text);
                        continue;
                    }
                }

                // Handle keyboard input for all cursors
                // Text-modifying keys (Backspace, Delete, Enter, Tab) need special handling
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    // Check for text-modifying keys that need multi-cursor handling
                    let is_text_modifying = matches!(
                        key,
                        egui::Key::Backspace
                            | egui::Key::Delete
                            | egui::Key::Enter
                            | egui::Key::Tab
                    );

                    if is_text_modifying && !modifiers.ctrl && !modifiers.command {
                        match key {
                            egui::Key::Tab => {
                                // Insert tab character (TODO: respect use_spaces and tab_size settings)
                                // This also prevents Tab from cycling focus to other UI elements
                                self.insert_text_at_all_cursors("\t");
                                continue;
                            }
                            egui::Key::Enter => {
                                self.insert_text_at_all_cursors("\n");
                                continue;
                            }
                            egui::Key::Backspace => {
                                self.backspace_at_all_cursors();
                                continue;
                            }
                            egui::Key::Delete => {
                                self.delete_at_all_cursors();
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle navigation keys for all cursors
                    let is_navigation = matches!(
                        key,
                        egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::Home
                            | egui::Key::End
                    );

                    if is_navigation && self.has_multiple_cursors() {
                        self.move_all_cursors(*key, modifiers);
                        self.view.ensure_line_visible(
                            self.primary_selection().head.line,
                            self.buffer.line_count(),
                        );
                        continue;
                    }
                }

                // Handle remaining keyboard input (navigation, etc.) for primary selection only
                // Note: We take the selection out temporarily to satisfy the borrow checker
                let idx = self
                    .primary_selection_index
                    .min(self.selections.len().saturating_sub(1));
                let mut primary_sel = self
                    .selections
                    .get(idx)
                    .copied()
                    .unwrap_or_else(Selection::start);

                let result = InputHandler::handle_event_with_selection(
                    event,
                    &mut self.buffer,
                    &mut primary_sel,
                    &mut self.view,
                );

                // Put the modified selection back
                if let Some(sel) = self.selections.get_mut(idx) {
                    *sel = primary_sel;
                }

                match result {
                    InputResult::TextChanged => {
                        let line = self.primary_selection().head.line;
                        self.mark_lines_dirty(line, line);
                        self.reset_cursor_blink();
                        self.view
                            .ensure_line_visible(line, self.buffer.line_count());
                    }
                    InputResult::CursorMoved => {
                        // Reset cursor blink so cursor is visible immediately after movement
                        self.reset_cursor_blink();
                        // Ensure cursor is visible after movement
                        self.view.ensure_line_visible(
                            self.primary_selection().head.line,
                            self.buffer.line_count(),
                        );
                    }
                    InputResult::ViewScrolled => {
                        // View scrolled (mouse wheel) - no cursor adjustment needed
                        // The view has already been updated by the input handler
                    }
                    InputResult::NoChange => {}
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // Render scrollbars (using egui's animation system for consistency)
        // Scrollbars fade out completely when mouse leaves the editor area
        // ═══════════════════════════════════════════════════════════════════════
        let viewport_height = self.view.viewport_height();
        // Use smoothed content height for scrollbar to prevent jumping
        let scrollbar_height = self.view.scrollbar_content_height(total_lines);

        // Request repaint while scrollbar height is still smoothing toward target
        let actual_height = self.view.total_content_height(total_lines);
        if (scrollbar_height - actual_height).abs() > 1.0 {
            ui.ctx().request_repaint();
        }

        // Check if mouse is over the editor area (for scrollbar visibility)
        let mouse_over_editor = ui.rect_contains_pointer(rect);

        // Only show vertical scrollbar if content exceeds viewport
        if scrollbar_height > viewport_height && viewport_height > 0.0 {
            Self::render_scrollbar(
                ui,
                &painter,
                response.id.with("v_scrollbar"),
                rect,
                true, // vertical
                gutter_width + gutter_padding,
                viewport_height,
                scrollbar_height,
                self.view.current_scroll_y(),
                mouse_over_editor,
                |target_scroll| {
                    self.view.scroll_to_absolute(target_scroll, total_lines);
                },
            );
        }

        // Horizontal scrollbar (only when word wrap is off and content is wider)
        let text_viewport_width = text_area_width;
        if !effective_wrap_enabled
            && max_line_width > text_viewport_width
            && text_viewport_width > 0.0
        {
            Self::render_scrollbar(
                ui,
                &painter,
                response.id.with("h_scrollbar"),
                rect,
                false, // horizontal
                gutter_width + gutter_padding,
                text_viewport_width,
                max_line_width,
                self.view.horizontal_scroll(),
                mouse_over_editor,
                |target_scroll| {
                    self.view.set_horizontal_scroll(target_scroll);
                },
            );
        }

        // Render navigation buttons overlay (top-left corner)
        // These buttons allow quick jumping to top, middle, or bottom of the document
        let is_dark_mode = ui.visuals().dark_mode;
        let nav_action = render_nav_buttons(ui, rect, is_dark_mode);

        // Handle navigation button actions
        match nav_action {
            NavAction::Top => {
                // Jump to top: scroll to line 0, cursor at start
                self.view.scroll_to_line(0);
                self.set_cursor(Cursor::new(0, 0));
            }
            NavAction::Middle => {
                // Jump to middle: scroll to middle of document, cursor at that line
                let middle_line = total_lines / 2;
                self.view.scroll_to_center_line(middle_line, total_lines);
                self.set_cursor(Cursor::new(middle_line, 0));
            }
            NavAction::Bottom => {
                // Jump to bottom: scroll to last line, cursor at end
                let last_line = total_lines.saturating_sub(1);
                let last_col = self
                    .buffer
                    .get_line(last_line)
                    .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                    .unwrap_or(0);
                self.view.scroll_to_line(
                    last_line
                        .saturating_sub(
                            (self.view.viewport_height() / self.view.line_height()) as usize,
                        )
                        .min(last_line),
                );
                self.set_cursor(Cursor::new(last_line, last_col));
            }
            NavAction::None => {}
        }

        response
    }

    /// Marks the content as dirty, causing cache invalidation on next render.
    ///
    /// This performs a **full** invalidation (no line range).
    /// Prefer [`mark_lines_dirty`] when the affected line range is known.
    pub fn mark_dirty(&mut self) {
        self.content_dirty = true;
        // Full invalidation — clear line_keys so next frame rebuilds them
        self.line_cache.clear_line_keys();
        self.dirty_range = None;
    }

    /// Marks specific lines as dirty for targeted cache invalidation.
    ///
    /// Accumulates into a single `(start, end)` range across multiple
    /// edits within the same frame; consumed by the next `ui()` call.
    fn mark_lines_dirty(&mut self, start_line: usize, end_line: usize) {
        self.content_dirty = true;
        if let Some((ref mut s, ref mut e)) = self.dirty_range {
            *s = (*s).min(start_line);
            *e = (*e).max(end_line);
        } else {
            self.dirty_range = Some((start_line, end_line));
        }
    }

    /// Returns whether the content has been modified since the last frame.
    ///
    /// This flag is reset at the start of each `ui()` call and set to `true`
    /// if any edits occur during the frame. Use this after `ui()` returns to
    /// check if content changed and needs to be synced.
    ///
    /// # Performance
    /// This avoids expensive string comparison/conversion for large files.
    /// Only sync content when this returns `true`.
    #[must_use]
    pub fn is_content_dirty(&self) -> bool {
        self.content_dirty
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Word Wrap Support (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns whether word wrap is enabled.
    #[must_use]
    pub fn is_wrap_enabled(&self) -> bool {
        self.wrap_enabled
    }

    /// Enables word wrap.
    ///
    /// When enabled, long lines will wrap to fit within the available width.
    /// Horizontal scrolling is disabled when word wrap is on.
    ///
    /// # Note
    /// This invalidates the line cache and recalculates wrap info on next render.
    pub fn enable_wrap(&mut self) {
        if !self.wrap_enabled {
            self.wrap_enabled = true;
            self.line_cache.invalidate();
            self.view.clear_wrap_info();
            self.content_dirty = true;
        }
    }

    /// Disables word wrap.
    ///
    /// When disabled, long lines extend horizontally and can be scrolled.
    ///
    /// # Note
    /// This invalidates the line cache.
    pub fn disable_wrap(&mut self) {
        if self.wrap_enabled {
            self.wrap_enabled = false;
            self.view.disable_wrap();
            self.line_cache.invalidate();
            self.content_dirty = true;
        }
    }

    /// Sets word wrap enabled/disabled.
    pub fn set_wrap_enabled(&mut self, enabled: bool) {
        if enabled {
            self.enable_wrap();
        } else {
            self.disable_wrap();
        }
    }

    /// Sets the maximum wrap width in pixels.
    ///
    /// When set, text will wrap at this width or the available width,
    /// whichever is smaller. This is used to implement the "max line width"
    /// setting (e.g., 80 characters, 100 characters).
    ///
    /// # Arguments
    /// * `width` - Maximum width in pixels, or None for no limit
    pub fn set_max_wrap_width(&mut self, width: Option<f32>) {
        self.max_wrap_width = width;
    }

    /// Returns the maximum wrap width in pixels.
    #[must_use]
    pub fn max_wrap_width(&self) -> Option<f32> {
        self.max_wrap_width
    }

    /// Sets the horizontal offset for centering content (used in Zen Mode).
    ///
    /// When set to a positive value, text is rendered starting at
    /// text_start_x + content_offset_x, effectively centering the content.
    ///
    /// # Arguments
    /// * `offset` - Horizontal offset in pixels (0.0 for no centering)
    pub fn set_content_offset_x(&mut self, offset: f32) {
        self.content_offset_x = offset;
    }

    /// Returns the preferred column for vertical movement.
    #[must_use]
    pub fn preferred_column(&self) -> Option<usize> {
        self.preferred_column
    }

    /// Sets the preferred column for vertical movement.
    pub fn set_preferred_column(&mut self, column: Option<usize>) {
        self.preferred_column = column;
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Syntax Highlighting (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns whether syntax highlighting is enabled.
    #[must_use]
    pub fn is_syntax_enabled(&self) -> bool {
        self.syntax_enabled
    }

    /// Sets whether syntax highlighting is enabled.
    ///
    /// When enabled and a language is set, the editor will apply syntax-aware
    /// coloring to the text. This only affects visible lines for performance.
    ///
    /// # Note
    /// Changing this invalidates the line cache.
    pub fn set_syntax_enabled(&mut self, enabled: bool) {
        if self.syntax_enabled != enabled {
            self.syntax_enabled = enabled;
            self.line_cache.invalidate();
        }
    }

    /// Returns the current syntax language.
    #[must_use]
    pub fn syntax_language(&self) -> Option<&str> {
        self.syntax_language.as_deref()
    }

    /// Sets the syntax highlighting language.
    ///
    /// # Arguments
    /// * `language` - Language identifier (e.g., "rust", "python", "js")
    ///
    /// # Note
    /// Changing the language invalidates the line cache.
    pub fn set_syntax_language(&mut self, language: Option<String>) {
        if self.syntax_language != language {
            self.syntax_language = language;
            if self.syntax_enabled {
                self.line_cache.invalidate();
            }
        }
    }

    /// Returns whether dark mode syntax theme is active.
    #[must_use]
    pub fn is_syntax_dark_mode(&self) -> bool {
        self.syntax_dark_mode
    }

    /// Sets whether to use dark mode syntax theme.
    ///
    /// # Note
    /// Changing the theme invalidates the line cache.
    /// Only affects theme selection when no specific theme name is set.
    pub fn set_syntax_dark_mode(&mut self, dark_mode: bool) {
        if self.syntax_dark_mode != dark_mode {
            self.syntax_dark_mode = dark_mode;
            // Update theme hash when dark mode changes
            // (only affects theme selection when no specific theme name is set)
            self.syntax_theme_hash = Self::compute_theme_hash(&self.syntax_theme_name, dark_mode);
            if self.syntax_enabled {
                self.line_cache.invalidate();
            }
        }
    }

    /// Returns the current syntax theme hash (for cache key).
    #[must_use]
    pub fn syntax_theme_hash(&self) -> u64 {
        self.syntax_theme_hash
    }

    /// Sets the syntax theme hash directly.
    ///
    /// This is used when the syntax theme is specified by name
    /// (e.g., "Dracula", "Nord") rather than just dark/light mode.
    ///
    /// # Note
    /// Changing the theme hash invalidates the line cache.
    pub fn set_syntax_theme_hash(&mut self, hash: u64) {
        if self.syntax_theme_hash != hash {
            self.syntax_theme_hash = hash;
            if self.syntax_enabled {
                self.line_cache.invalidate();
            }
        }
    }

    /// Configures syntax highlighting settings at once.
    ///
    /// This is a convenience method for setting all syntax options together.
    ///
    /// # Arguments
    /// * `enabled` - Whether syntax highlighting is enabled
    /// * `language` - Language identifier (e.g., "rust", "python")
    /// * `dark_mode` - Whether to use dark mode syntax theme
    /// * `theme_name` - Optional theme name (e.g., "Dracula", "Nord"). If None, uses dark/light default.
    pub fn configure_syntax(
        &mut self,
        enabled: bool,
        language: Option<String>,
        dark_mode: bool,
        theme_name: Option<String>,
    ) {
        // Compute the new theme hash based on theme name or dark/light mode
        let new_theme_hash = Self::compute_theme_hash(&theme_name, dark_mode);

        let needs_invalidation = enabled != self.syntax_enabled
            || language != self.syntax_language
            || dark_mode != self.syntax_dark_mode
            || theme_name != self.syntax_theme_name
            || new_theme_hash != self.syntax_theme_hash;

        self.syntax_enabled = enabled;
        self.syntax_language = language;
        self.syntax_dark_mode = dark_mode;
        self.syntax_theme_name = theme_name;
        self.syntax_theme_hash = new_theme_hash;

        if needs_invalidation {
            self.line_cache.invalidate();
        }
    }

    /// Computes a hash for the syntax theme based on theme name or dark/light mode.
    fn compute_theme_hash(theme_name: &Option<String>, dark_mode: bool) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        match theme_name {
            Some(name) if !name.is_empty() => {
                // Hash the theme name for specific themes
                name.hash(&mut hasher);
            }
            _ => {
                // Use dark/light mode marker for auto-selection
                if dark_mode { 1u8 } else { 2u8 }.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Highlights a single line of text for rendering.
    ///
    /// This method uses the syntax highlighter to colorize a single line,
    /// converting the output to the `HighlightedSegment` format used by the cache.
    ///
    /// # Arguments
    /// * `line_content` - The text content to highlight (without trailing newline)
    /// * `language` - Language identifier for syntax highlighting
    ///
    /// # Returns
    /// A vector of `HighlightedSegment` for the line.
    fn highlight_line(&self, line_content: &str, language: &str) -> Vec<HighlightedSegment> {
        // Add newline for proper syntax parsing (syntect expects it)
        let content_with_newline = format!("{}\n", line_content);

        // Use the syntax highlighter with the configured theme
        let highlighted_lines = match &self.syntax_theme_name {
            Some(theme_name) if !theme_name.is_empty() => {
                // Use the specific theme name
                highlight_code_with_theme(
                    &content_with_newline,
                    language,
                    theme_name,
                    self.syntax_dark_mode,
                )
            }
            _ => {
                // Fall back to dark/light mode auto-selection
                highlight_code(&content_with_newline, language, self.syntax_dark_mode)
            }
        };

        // Convert to HighlightedSegment format
        // The highlighter returns one HighlightedLine per input line
        if let Some(first_line) = highlighted_lines.first() {
            first_line
                .segments
                .iter()
                .map(|seg| {
                    // Strip trailing newline from the last segment if present
                    let text = seg.text.trim_end_matches(['\r', '\n']).to_string();
                    HighlightedSegment {
                        text,
                        color: seg.foreground,
                    }
                })
                .filter(|seg| !seg.text.is_empty() || highlighted_lines.len() == 1)
                .collect()
        } else {
            // Empty line - return empty segments
            vec![]
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // IME/CJK Support Helpers (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Calculates the X coordinate of the cursor position.
    ///
    /// This is used for IME preedit rendering and candidate window positioning.
    /// Handles both wrapped and non-wrapped text modes.
    ///
    /// # Arguments
    /// * `cursor` - The cursor position to calculate X for
    /// * `font_id` - Font used for text measurement
    /// * `text_start_x` - X coordinate where text area begins (after gutter)
    /// * `painter` - Painter for layout calculations
    fn calculate_cursor_x(
        &self,
        cursor: &Cursor,
        font_id: &FontId,
        text_start_x: f32,
        painter: &egui::Painter,
    ) -> f32 {
        // Use the view's wrap state which accounts for uniform-height override
        let wrap_active = self.view.is_wrap_enabled();

        if cursor.column == 0 {
            if wrap_active {
                text_start_x
            } else {
                text_start_x - self.view.horizontal_scroll()
            }
        } else if let Some(line_content) = self.buffer.get_line(cursor.line) {
            let display_content = line_content.trim_end_matches(['\r', '\n']);

            if wrap_active {
                let effective_wrap_width = self.max_wrap_width.unwrap_or(f32::INFINITY);
                let galley = painter.layout(
                    display_content.to_string(),
                    font_id.clone(),
                    Color32::WHITE,
                    effective_wrap_width,
                );
                let char_count = display_content.chars().count();
                let cursor_col = cursor.column.min(char_count);
                let ccursor = egui::text::CCursor::new(cursor_col);
                let cursor_rect = galley.pos_from_cursor(ccursor);
                text_start_x + cursor_rect.min.x
            } else if crate::fonts::needs_complex_script_fonts(display_content) {
                let font_bytes = crate::fonts::ttf_bytes_for_font_id_shaping(font_id);
                if let Some(x) = super::shaping::shaped_column_to_x(
                    display_content,
                    font_bytes,
                    font_id.size,
                    cursor.column,
                ) {
                    text_start_x + x - self.view.horizontal_scroll()
                } else {
                    let chars_before: String =
                        display_content.chars().take(cursor.column).collect();
                    let galley =
                        painter.layout_no_wrap(chars_before, font_id.clone(), Color32::WHITE);
                    text_start_x + galley.size().x - self.view.horizontal_scroll()
                }
            } else {
                let chars_before: String = display_content.chars().take(cursor.column).collect();
                let galley = painter.layout_no_wrap(chars_before, font_id.clone(), Color32::WHITE);
                text_start_x + galley.size().x - self.view.horizontal_scroll()
            }
        } else if wrap_active {
            text_start_x
        } else {
            text_start_x - self.view.horizontal_scroll()
        }
    }

    /// Returns whether IME composition is currently active.
    #[must_use]
    pub fn is_ime_active(&self) -> bool {
        self.ime_enabled && self.ime_preedit.is_some()
    }

    /// Returns the current IME preedit text, if any.
    #[must_use]
    pub fn ime_preedit_text(&self) -> Option<&str> {
        self.ime_preedit.as_deref()
    }

    /// Takes the last IME committed text, clearing it after retrieval.
    ///
    /// This is used by the caller to check if CJK fonts need to be loaded.
    /// Returns `None` if no text was recently committed via IME.
    pub fn take_ime_committed_text(&mut self) -> Option<String> {
        self.ime_committed_text.take()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Code Folding API (Phase 3)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns a reference to the fold state.
    #[must_use]
    pub fn fold_state(&self) -> &FoldState {
        &self.fold_state
    }

    /// Returns a mutable reference to the fold state.
    pub fn fold_state_mut(&mut self) -> &mut FoldState {
        &mut self.fold_state
    }

    /// Sets the fold state (typically from external fold detection).
    pub fn set_fold_state(&mut self, fold_state: FoldState) {
        self.fold_state = fold_state;
    }

    /// Sets whether to show fold indicators in the gutter.
    pub fn set_show_fold_indicators(&mut self, show: bool) {
        self.show_fold_indicators = show;
    }

    /// Sets whether to show line numbers in the gutter.
    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show;
    }

    /// Sets whether auto-close brackets is enabled.
    ///
    /// When enabled, typing an opening bracket/quote automatically inserts
    /// the matching closing character and positions the cursor between them.
    ///
    /// Supported pairs: `()`, `[]`, `{}`, `''`, `""`, ``` `` ```
    ///
    /// Smart handling for quotes: Won't auto-close after alphanumeric characters
    /// (to avoid interfering with contractions like "don't").
    pub fn set_auto_close_brackets(&mut self, enabled: bool) {
        self.auto_close_brackets = enabled;
    }

    /// Enables or disables Vim modal editing mode.
    /// When enabled, the editor uses Normal/Insert/Visual modes with Vim keybindings.
    /// When transitioning from enabled to disabled, resets Vim state to Normal mode.
    pub fn set_vim_mode(&mut self, enabled: bool) {
        if self.vim_mode_enabled && !enabled {
            self.vim_state = super::vim::VimState::new();
        }
        self.vim_mode_enabled = enabled;
    }

    /// Returns the current Vim mode if Vim editing is enabled.
    pub fn vim_mode(&self) -> Option<super::vim::VimMode> {
        if self.vim_mode_enabled {
            Some(self.vim_state.mode)
        } else {
            None
        }
    }

    /// Takes the fold toggle line if a fold was toggled this frame.
    /// Returns the line number where a fold was toggled, or None.
    /// This clears the value after reading.
    pub fn take_fold_toggle_line(&mut self) -> Option<usize> {
        self.fold_toggle_line.take()
    }

    /// Checks if a line is hidden by any collapsed fold.
    #[must_use]
    pub fn is_line_folded(&self, line: usize) -> bool {
        self.fold_state.is_line_hidden(line)
    }

    /// Toggles fold state at the given line (if it's a fold start line).
    /// Returns `true` if a fold was toggled.
    pub fn toggle_fold(&mut self, line: usize) -> bool {
        self.fold_state.toggle_at_line(line)
    }

    /// Reveals a line by expanding any fold that hides it.
    /// Returns `true` if any fold was expanded.
    pub fn reveal_line(&mut self, line: usize) -> bool {
        self.fold_state.reveal_line(line)
    }

    /// Folds all fold regions.
    pub fn fold_all(&mut self) {
        self.fold_state.fold_all();
    }

    /// Unfolds all fold regions.
    pub fn unfold_all(&mut self) {
        self.fold_state.unfold_all();
    }

    /// Returns lines that should show fold indicators in the gutter.
    /// Each tuple is (line_number, is_collapsed).
    #[must_use]
    pub fn fold_indicator_lines(&self) -> Vec<(usize, bool)> {
        self.fold_state.fold_indicator_lines()
    }

    /// Returns the number of visible lines (accounting for collapsed folds).
    #[must_use]
    pub fn visible_line_count(&self) -> usize {
        let total = self.buffer.line_count();
        let mut visible = 0;
        for line in 0..total {
            if !self.fold_state.is_line_hidden(line) {
                visible += 1;
            }
        }
        visible
    }

    /// Converts a visual line (accounting for folds) to a document line.
    #[must_use]
    pub fn visual_to_document_line(&self, visual_line: usize) -> usize {
        self.fold_state.visual_to_document_line(visual_line)
    }

    /// Converts a document line to a visual line (accounting for folds).
    #[must_use]
    pub fn document_to_visual_line(&self, doc_line: usize) -> usize {
        self.fold_state.document_to_visual_line(doc_line)
    }

    /// Adjusts cursor position to skip folded regions.
    ///
    /// If the cursor is on a hidden line (inside a collapsed fold),
    /// moves it to the next visible line.
    pub fn adjust_cursor_for_folds(&mut self) {
        let cursor = self.primary_selection().head;
        if self.fold_state.is_line_hidden(cursor.line) {
            // Find the next visible line
            let total_lines = self.buffer.line_count();
            let mut new_line = cursor.line;

            // Try moving down first
            while new_line < total_lines && self.fold_state.is_line_hidden(new_line) {
                new_line += 1;
            }

            // If we hit the end, try moving up
            if new_line >= total_lines {
                new_line = cursor.line;
                while new_line > 0 && self.fold_state.is_line_hidden(new_line) {
                    new_line -= 1;
                }
            }

            // Clamp column to new line length
            let new_col = cursor.column.min(
                self.buffer
                    .get_line(new_line)
                    .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                    .unwrap_or(0),
            );

            *self.primary_selection_mut() = Selection::collapsed(Cursor::new(new_line, new_col));
        }
    }

    /// Gets the next visible line after the given line (accounting for folds).
    /// Returns None if there is no visible line after.
    #[must_use]
    pub fn next_visible_line(&self, line: usize) -> Option<usize> {
        let total_lines = self.buffer.line_count();
        let mut next = line + 1;
        while next < total_lines {
            if !self.fold_state.is_line_hidden(next) {
                return Some(next);
            }
            next += 1;
        }
        None
    }

    /// Gets the previous visible line before the given line (accounting for folds).
    /// Returns None if there is no visible line before.
    #[must_use]
    pub fn prev_visible_line(&self, line: usize) -> Option<usize> {
        if line == 0 {
            return None;
        }
        let mut prev = line - 1;
        loop {
            if !self.fold_state.is_line_hidden(prev) {
                return Some(prev);
            }
            if prev == 0 {
                return None;
            }
            prev -= 1;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Auto-Close Brackets
    // ─────────────────────────────────────────────────────────────────────────

    /// Gets the closing bracket/quote for an opening character.
    /// Returns None if the character is not an opener.
    #[must_use]
    fn get_closing_bracket(ch: char) -> Option<char> {
        match ch {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            '`' => Some('`'),
            _ => None,
        }
    }

    /// Checks if a character is a closing bracket/quote.
    #[must_use]
    fn is_closing_bracket(ch: char) -> bool {
        matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
    }

    /// Gets the character at the cursor position (if any).
    #[must_use]
    fn char_at_cursor(&self) -> Option<char> {
        let cursor = self.primary_selection().head;
        let line_content = self.buffer.get_line(cursor.line)?;
        line_content.chars().nth(cursor.column)
    }

    /// Gets the character before the cursor position (if any).
    #[must_use]
    fn char_before_cursor(&self) -> Option<char> {
        let cursor = self.primary_selection().head;
        if cursor.column == 0 {
            return None;
        }
        let line_content = self.buffer.get_line(cursor.line)?;
        line_content.chars().nth(cursor.column - 1)
    }

    /// Handles auto-close bracket insertion for a single character.
    ///
    /// Returns true if the character was handled (auto-close or skip-over),
    /// false if normal insertion should proceed.
    fn handle_auto_close(&mut self, ch: char) -> bool {
        if !self.auto_close_brackets {
            return false;
        }

        // Case 1: Skip-over - typing a closer when next char is the same closer
        if Self::is_closing_bracket(ch) {
            if let Some(next_char) = self.char_at_cursor() {
                if next_char == ch {
                    // Just move cursor forward, don't insert
                    self.move_all_cursors_right();
                    return true;
                }
            }
        }

        // Case 2: Auto-pair insertion - typing an opener
        if let Some(closer) = Self::get_closing_bracket(ch) {
            // Smart handling for quotes: don't auto-close after alphanumeric
            // (to avoid interfering with contractions like "don't")
            if matches!(ch, '"' | '\'' | '`') {
                if let Some(prev_char) = self.char_before_cursor() {
                    if prev_char.is_alphanumeric() {
                        return false; // Let normal insertion handle it
                    }
                }
            }

            // Insert opener + closer, position cursor between them
            self.insert_text_with_cursor_position(&format!("{}{}", ch, closer), 1);
            return true;
        }

        false
    }

    /// Inserts text and positions cursor at an offset from the start.
    ///
    /// Used for auto-close to insert "()" with cursor between the brackets.
    fn insert_text_with_cursor_position(&mut self, text: &str, cursor_offset: usize) {
        if text.is_empty() {
            return;
        }

        // First delete any selections
        self.delete_selection();

        // Get all cursor positions and sort by char position (descending)
        let mut cursor_positions: Vec<(usize, usize)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let char_pos = InputHandler::cursor_to_char_pos(&self.buffer, &s.head);
                (i, char_pos)
            })
            .collect();

        // Sort by position descending (insert from end first)
        cursor_positions.sort_by(|a, b| b.1.cmp(&a.1));

        let mut new_cursors: Vec<(usize, Cursor)> = Vec::with_capacity(self.selections.len());

        for (sel_idx, char_pos) in cursor_positions {
            // Insert text at this position
            self.buffer.insert(char_pos, text);

            // Calculate new cursor position (at offset from start of insertion)
            let new_char_pos = char_pos + cursor_offset;
            let new_cursor = self.char_pos_to_cursor(new_char_pos);
            new_cursors.push((sel_idx, new_cursor));
        }

        // Update selections with new cursor positions
        for (sel_idx, new_cursor) in new_cursors {
            if let Some(sel) = self.selections.get_mut(sel_idx) {
                *sel = Selection::collapsed(new_cursor);
            }
        }

        self.merge_overlapping_selections();
        self.content_dirty = true;
        self.view
            .ensure_line_visible(self.primary_selection().head.line, self.buffer.line_count());
    }

    /// Moves all cursors one grapheme cluster to the right (for skip-over).
    fn move_all_cursors_right(&mut self) {
        for sel in &mut self.selections {
            let line_len = self
                .buffer
                .get_line(sel.head.line)
                .map(|l| l.trim_end_matches(['\r', '\n']).chars().count())
                .unwrap_or(0);

            if sel.head.column < line_len {
                let new_col = self
                    .buffer
                    .get_line(sel.head.line)
                    .map(|l| {
                        let stripped = grapheme::line_text_stripped(&l);
                        grapheme::next_grapheme_boundary(stripped, sel.head.column)
                    })
                    .unwrap_or(sel.head.column + 1);
                sel.head.column = new_col;
                sel.anchor = sel.head;
            }
        }
        self.view
            .ensure_line_visible(self.primary_selection().head.line, self.buffer.line_count());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scrollbar Rendering (shared between vertical and horizontal)
    // ─────────────────────────────────────────────────────────────────────────

    /// Renders a scrollbar using egui's animation system for consistent behavior.
    ///
    /// This matches the style and behavior of egui's built-in ScrollArea scrollbars:
    /// - Thin when idle (4px), expands on hover (8px)
    /// - Smooth fade in/out animations
    /// - Fades out completely when mouse leaves the editor area
    /// - Proper colors from theme
    fn render_scrollbar<F>(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        id: egui::Id,
        rect: egui::Rect,
        vertical: bool,
        content_offset: f32, // gutter width for horizontal scrollbar
        viewport_size: f32,
        content_size: f32,
        current_scroll: f32,
        mouse_over_editor: bool,
        mut on_scroll: F,
    ) where
        F: FnMut(f32),
    {
        // Scrollbar dimensions - match egui defaults
        const WIDTH_NARROW: f32 = 4.0;
        const WIDTH_WIDE: f32 = 8.0;
        const INTERACT_WIDTH: f32 = 12.0;
        const MIN_THUMB_SIZE: f32 = 20.0;
        const MARGIN: f32 = 2.0;

        // Calculate interaction rectangle (larger area for easier grabbing)
        let interact_rect = if vertical {
            egui::Rect::from_min_size(
                egui::pos2(rect.max.x - INTERACT_WIDTH, rect.min.y + MARGIN),
                egui::vec2(INTERACT_WIDTH, rect.height() - MARGIN * 2.0),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(
                    rect.min.x + content_offset + MARGIN,
                    rect.max.y - INTERACT_WIDTH,
                ),
                egui::vec2(viewport_size - MARGIN * 2.0, INTERACT_WIDTH),
            )
        };

        // Check direct scrollbar interaction
        let response = ui.interact(interact_rect, id, Sense::click_and_drag());
        let scrollbar_hovered = response.hovered();
        let scrollbar_dragged = response.dragged();

        // Scrollbar visibility: show when mouse is over editor area OR being dragged
        let should_show = mouse_over_editor || scrollbar_dragged;

        // Use egui's animation for smooth fade in/out (like ScrollArea does)
        // Fade in fast (0.1s), fade out slower (0.2s)
        let fade_time = if should_show { 0.1 } else { 0.2 };
        let visibility_anim =
            ui.ctx()
                .animate_bool_with_time(id.with("visible"), should_show, fade_time);

        // Don't render at all if completely faded out
        if visibility_anim <= 0.001 {
            return;
        }

        // Width animation: expand when directly hovering the scrollbar
        let width_anim = ui.ctx().animate_bool_with_time(
            id.with("hover"),
            scrollbar_hovered || scrollbar_dragged,
            0.1,
        );
        let visual_width = egui::lerp(WIDTH_NARROW..=WIDTH_WIDE, width_anim);

        // Calculate visual rectangle based on animated width
        let visual_rect = if vertical {
            egui::Rect::from_min_size(
                egui::pos2(rect.max.x - visual_width - MARGIN, rect.min.y + MARGIN),
                egui::vec2(visual_width, rect.height() - MARGIN * 2.0),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(
                    rect.min.x + content_offset + MARGIN,
                    rect.max.y - visual_width - MARGIN,
                ),
                egui::vec2(viewport_size - MARGIN * 2.0, visual_width),
            )
        };

        // Calculate thumb position and size
        let visible_ratio = (viewport_size / content_size).min(1.0);
        let track_length = if vertical {
            visual_rect.height()
        } else {
            visual_rect.width()
        };
        let thumb_size = (track_length * visible_ratio).max(MIN_THUMB_SIZE);

        let max_scroll = (content_size - viewport_size).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
            (current_scroll / max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let thumb_travel = track_length - thumb_size;
        let thumb_offset = thumb_travel * scroll_ratio;

        let thumb_rect = if vertical {
            egui::Rect::from_min_size(
                egui::pos2(visual_rect.min.x, visual_rect.min.y + thumb_offset),
                egui::vec2(visual_width, thumb_size),
            )
        } else {
            egui::Rect::from_min_size(
                egui::pos2(visual_rect.min.x + thumb_offset, visual_rect.min.y),
                egui::vec2(thumb_size, visual_width),
            )
        };

        // Handle scrollbar interaction
        if scrollbar_dragged {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let relative = if vertical {
                    pointer_pos.y - interact_rect.min.y - thumb_size / 2.0
                } else {
                    pointer_pos.x - interact_rect.min.x - thumb_size / 2.0
                };
                let drag_ratio = (relative / thumb_travel).clamp(0.0, 1.0);
                on_scroll(drag_ratio * max_scroll);
            }
        }

        // Handle click on track (jump to position)
        if response.clicked()
            && !thumb_rect.contains(response.interact_pointer_pos().unwrap_or_default())
        {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let relative = if vertical {
                    pointer_pos.y - interact_rect.min.y - thumb_size / 2.0
                } else {
                    pointer_pos.x - interact_rect.min.x - thumb_size / 2.0
                };
                let click_ratio = (relative / thumb_travel).clamp(0.0, 1.0);
                on_scroll(click_ratio * max_scroll);
            }
        }

        // Determine thumb color based on interaction state
        // Base alpha: higher when dragged/hovered, lower when just visible
        let base_alpha = if scrollbar_dragged {
            200.0
        } else if scrollbar_hovered {
            160.0
        } else {
            100.0
        };

        // Apply visibility animation for fade in/out
        let alpha = (base_alpha * visibility_anim) as u8;

        let thumb_color = if ui.visuals().dark_mode {
            egui::Color32::from_white_alpha(alpha)
        } else {
            egui::Color32::from_black_alpha(alpha)
        };

        // Draw the scrollbar thumb with rounded corners
        painter.rect_filled(thumb_rect, visual_width / 2.0, thumb_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor() {
        let editor = FerriteEditor::new();
        assert!(editor.is_empty());
        assert_eq!(editor.line_count(), 1); // Empty buffer has 1 line
        assert_eq!(editor.cursor(), Cursor::start());
        assert!(!editor.has_selection());
    }

    #[test]
    fn test_from_string() {
        let editor = FerriteEditor::from_string("Hello\nWorld\nTest");
        assert!(!editor.is_empty());
        assert_eq!(editor.line_count(), 3);
    }

    #[test]
    fn test_from_string_empty() {
        let editor = FerriteEditor::from_string("");
        assert!(editor.is_empty());
        assert_eq!(editor.line_count(), 1);
    }

    #[test]
    fn test_default() {
        let editor = FerriteEditor::default();
        assert!(editor.is_empty());
    }

    #[test]
    fn test_set_cursor() {
        let mut editor = FerriteEditor::from_string("Hello\nWorld");

        // Normal set
        editor.set_cursor(Cursor::new(1, 3));
        assert_eq!(editor.cursor(), Cursor::new(1, 3));
        assert!(!editor.has_selection()); // set_cursor collapses selection

        // Clamp to valid line
        editor.set_cursor(Cursor::new(100, 0));
        assert_eq!(editor.cursor().line, 1); // Clamped to last line

        // Clamp to valid column
        editor.set_cursor(Cursor::new(0, 100));
        assert_eq!(editor.cursor().column, 5); // "Hello" has 5 chars
    }

    #[test]
    fn test_selection() {
        let mut editor = FerriteEditor::from_string("Hello\nWorld");

        // Set a selection
        let sel = Selection::new(Cursor::new(0, 0), Cursor::new(0, 5));
        editor.set_selection(sel);

        assert!(editor.has_selection());
        assert_eq!(editor.selection().anchor, Cursor::new(0, 0));
        assert_eq!(editor.selection().head, Cursor::new(0, 5));
        assert_eq!(editor.cursor(), Cursor::new(0, 5)); // cursor is head
    }

    #[test]
    fn test_selected_text() {
        let mut editor = FerriteEditor::from_string("Hello World");

        // No selection
        assert_eq!(editor.selected_text(), "");

        // Select "Hello"
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(0, 5)));
        assert_eq!(editor.selected_text(), "Hello");

        // Select "World"
        editor.set_selection(Selection::new(Cursor::new(0, 6), Cursor::new(0, 11)));
        assert_eq!(editor.selected_text(), "World");
    }

    #[test]
    fn test_selected_text_multiline() {
        let mut editor = FerriteEditor::from_string("Hello\nWorld\nTest");

        // Select from "ello" to "Wor"
        editor.set_selection(Selection::new(Cursor::new(0, 1), Cursor::new(1, 3)));
        assert_eq!(editor.selected_text(), "ello\nWor");
    }

    #[test]
    fn test_selected_text_multi_cursor() {
        let mut editor = FerriteEditor::from_string("alpha\nbeta\ngamma");

        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(0, 5)));
        editor.add_selection(Selection::new(Cursor::new(1, 0), Cursor::new(1, 4)));
        editor.add_selection(Selection::new(Cursor::new(2, 0), Cursor::new(2, 5)));

        assert!(editor.has_multiple_cursors());
        assert!(editor.has_any_selection());
        assert_eq!(editor.selected_text(), "alpha\nbeta\ngamma");
    }

    #[test]
    fn test_delete_selection() {
        let mut editor = FerriteEditor::from_string("Hello World");

        // No selection - should return false
        assert!(!editor.delete_selection());
        assert_eq!(editor.buffer().to_string(), "Hello World");

        // Select and delete "Hello "
        editor.set_selection(Selection::new(Cursor::new(0, 0), Cursor::new(0, 6)));
        assert!(editor.delete_selection());
        assert_eq!(editor.buffer().to_string(), "World");
        assert!(!editor.has_selection());
        assert_eq!(editor.cursor(), Cursor::new(0, 0));
    }

    #[test]
    fn test_set_font_size() {
        let mut editor = FerriteEditor::new();

        editor.set_font_size(16.0);
        assert!((editor.font_size() - 16.0).abs() < 0.01);

        // Clamp to minimum
        editor.set_font_size(4.0);
        assert!((editor.font_size() - 8.0).abs() < 0.01);

        // Clamp to maximum
        editor.set_font_size(100.0);
        assert!((editor.font_size() - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_buffer_access() {
        let mut editor = FerriteEditor::from_string("Test");

        // Read access
        assert_eq!(editor.buffer().line_count(), 1);

        // Mutable access marks dirty
        {
            let _buffer = editor.buffer_mut();
        }
        // content_dirty is internal, but we can verify by checking the effect
    }

    #[test]
    fn test_view_access() {
        let mut editor = FerriteEditor::new();

        // Default view
        assert_eq!(editor.view().first_visible_line(), 0);

        // Modify view
        editor.view_mut().scroll_to_line(5);
        assert_eq!(editor.view().first_visible_line(), 5);
    }

    #[test]
    fn test_mark_dirty() {
        let mut editor = FerriteEditor::new();
        editor.mark_dirty();
        // Internal state change, verified by cache invalidation on next ui() call
    }

    #[test]
    fn test_large_buffer() {
        // Create a buffer with 1000 lines
        let content: String = (0..1000).map(|i| format!("Line {i}\n")).collect();
        let editor = FerriteEditor::from_string(&content);

        assert_eq!(editor.line_count(), 1001); // 1000 lines + 1 from trailing newline
    }

    #[test]
    fn test_unicode_content() {
        let editor = FerriteEditor::from_string("こんにちは\n世界\n🌍🌎🌏");
        assert_eq!(editor.line_count(), 3);
    }

    #[test]
    fn test_cursor_with_unicode() {
        let mut editor = FerriteEditor::from_string("こんにちは");

        // Set cursor to middle of Japanese text
        editor.set_cursor(Cursor::new(0, 2));
        assert_eq!(editor.cursor().column, 2);

        // Set cursor beyond line length (should clamp)
        editor.set_cursor(Cursor::new(0, 100));
        assert_eq!(editor.cursor().column, 5); // 5 Japanese characters
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Phase 1: Viewport Rendering Tests (Task 7)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_fixed_line_height() {
        // Verify the fixed line height constant is 20.0
        assert!((FIXED_LINE_HEIGHT - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_view_visible_line_range_large_file() {
        // Create a buffer with 100,000 lines (simulating large file)
        let content: String = (0..100_000).map(|i| format!("Line {i}\n")).collect();
        let mut editor = FerriteEditor::from_string(&content);

        // Set up viewport: 400px height with 20px line height = ~20 visible lines
        editor.view.update_viewport(400.0);
        editor.view.set_line_height(FIXED_LINE_HEIGHT);

        let total_lines = editor.line_count();
        let (start, end) = editor.view.get_visible_line_range(total_lines);

        // With overscan of 5, we should have:
        // 20 visible lines + 5 overscan above + 5 overscan below = 25-30 lines max
        let rendered_lines = end - start;
        assert!(
            rendered_lines <= 30,
            "Should render ~20-30 lines, not {rendered_lines}"
        );
        assert!(
            rendered_lines >= 20,
            "Should render at least 20 lines for 400px viewport"
        );

        // Verify we're NOT rendering all 100k lines
        assert!(
            rendered_lines < 100,
            "Should only render visible lines, not all {total_lines}"
        );
    }

    #[test]
    fn test_view_visible_line_range_scrolled() {
        let content: String = (0..1000).map(|i| format!("Line {i}\n")).collect();
        let mut editor = FerriteEditor::from_string(&content);

        // Set up viewport and scroll to middle
        editor.view.update_viewport(400.0);
        editor.view.set_line_height(FIXED_LINE_HEIGHT);
        editor.view.scroll_to_line(500);

        let total_lines = editor.line_count();
        let (start, end) = editor.view.get_visible_line_range(total_lines);

        // Verify we're rendering lines around position 500
        assert!(
            start >= 495 - 5,
            "Start should be near 495 (500 - overscan)"
        );
        assert!(
            end <= 520 + 5,
            "End should be near 520 (500 + visible + overscan)"
        );

        // Verify range makes sense
        assert!(start < end, "Start should be less than end");
        let rendered = end - start;
        assert!(rendered <= 35, "Should render ~30 lines with overscan");
    }

    #[test]
    fn test_horizontal_scroll_offset() {
        let mut editor = FerriteEditor::from_string("Short line\nThis is a very long line that extends far to the right and would require horizontal scrolling to see fully");

        // Default horizontal scroll should be 0
        assert!((editor.view.horizontal_scroll() - 0.0).abs() < 0.01);

        // Set horizontal scroll
        editor.view.set_horizontal_scroll(150.0);
        assert!((editor.view.horizontal_scroll() - 150.0).abs() < 0.01);

        // Negative values should be clamped to 0
        editor.view.set_horizontal_scroll(-50.0);
        assert!((editor.view.horizontal_scroll() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_view_line_to_pixel_alignment() {
        let mut editor = FerriteEditor::from_string("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");

        editor.view.set_line_height(FIXED_LINE_HEIGHT);
        editor.view.scroll_to_line(0);

        // Line 0 should be at y=0
        assert!((editor.view.line_to_pixel(0) - 0.0).abs() < 0.01);

        // Line 1 should be at y=20 (1 * FIXED_LINE_HEIGHT)
        assert!((editor.view.line_to_pixel(1) - 20.0).abs() < 0.01);

        // Line 2 should be at y=40 (2 * FIXED_LINE_HEIGHT)
        assert!((editor.view.line_to_pixel(2) - 40.0).abs() < 0.01);

        // Line 4 should be at y=80 (4 * FIXED_LINE_HEIGHT)
        assert!((editor.view.line_to_pixel(4) - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_view_pixel_to_line_conversion() {
        let mut editor = FerriteEditor::from_string("Line 1\nLine 2\nLine 3");

        editor.view.set_line_height(FIXED_LINE_HEIGHT);
        editor.view.scroll_to_line(0);

        // Pixel 0 should be line 0
        assert_eq!(editor.view.pixel_to_line(0.0), 0);

        // Pixel 19 should still be line 0 (within first line)
        assert_eq!(editor.view.pixel_to_line(19.0), 0);

        // Pixel 20 should be line 1
        assert_eq!(editor.view.pixel_to_line(20.0), 1);

        // Pixel 40 should be line 2
        assert_eq!(editor.view.pixel_to_line(40.0), 2);
    }

    #[test]
    fn test_long_lines_content() {
        // Create content with very long lines (simulating code with no wrapping)
        let long_line = "x".repeat(500); // 500 character line
        let content = format!("{long_line}\nShort\n{long_line}");
        let editor = FerriteEditor::from_string(&content);

        assert_eq!(editor.line_count(), 3);

        // Verify the buffer stores long lines correctly
        if let Some(line) = editor.buffer.get_line(0) {
            let line_len = line.trim_end_matches(['\r', '\n']).chars().count();
            assert_eq!(line_len, 500, "First line should have 500 characters");
        }
    }

    #[test]
    fn test_ensure_line_visible() {
        let content: String = (0..100).map(|i| format!("Line {i}\n")).collect();
        let mut editor = FerriteEditor::from_string(&content);

        // Set up a small viewport
        editor.view.update_viewport(200.0);
        editor.view.set_line_height(FIXED_LINE_HEIGHT); // 10 visible lines

        // Start at top
        editor.view.scroll_to_line(0);

        // Line 5 is visible, should return false (no scroll needed)
        let scrolled = editor.view.ensure_line_visible(5, 101);
        assert!(!scrolled, "Line 5 should be visible from line 0");

        // Line 50 is not visible, should scroll
        let scrolled = editor.view.ensure_line_visible(50, 101);
        assert!(scrolled, "Line 50 should require scrolling");
        assert!(
            editor.view.is_line_visible(50, 101),
            "Line 50 should now be visible"
        );
    }

    #[test]
    fn test_scroll_by_incremental() {
        let content: String = (0..100).map(|i| format!("Line {i}\n")).collect();
        let mut editor = FerriteEditor::from_string(&content);

        editor.view.update_viewport(200.0);
        editor.view.set_line_height(FIXED_LINE_HEIGHT);
        editor.view.scroll_to_line(0);

        // Scroll down by one line (20 pixels)
        editor.view.scroll_by(20.0, 101);
        assert_eq!(editor.view.first_visible_line(), 1);

        // Scroll down by 3 more lines (60 pixels)
        editor.view.scroll_by(60.0, 101);
        assert_eq!(editor.view.first_visible_line(), 4);

        // Scroll up by 2 lines (-40 pixels)
        editor.view.scroll_by(-40.0, 101);
        assert_eq!(editor.view.first_visible_line(), 2);
    }
}
