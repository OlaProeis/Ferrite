//! Text editor widget for Ferrite
//!
//! This module implements the main text editor widget using the custom
//! FerriteEditor, which provides high performance for large files through
//! virtual scrolling and rope-based text storage.

use crate::config::{EditorFont, MaxLineWidth};
use crate::state::Tab;
use crate::theme::ThemeColors;
use arboard::Clipboard;
use eframe::egui::{self, Ui};
use log::debug;
use rust_i18n::t;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[cfg(test)]
use super::ferrite::Cursor;
use super::ferrite::FerriteEditor;

// ─────────────────────────────────────────────────────────────────────────────
// FerriteEditor Storage
// ─────────────────────────────────────────────────────────────────────────────

/// Storage for FerriteEditor instances, keyed by tab ID.
///
/// This is stored in egui's memory to persist across frames.
/// Each tab gets its own FerriteEditor instance.
#[derive(Clone, Default)]
struct FerriteEditorStorage {
    /// Map from tab ID to FerriteEditor instance (`None` = checked out for `EditorWidget::show`)
    editors: HashMap<usize, Option<FerriteEditor>>,
    /// Hash of content for each editor (to detect external changes)
    content_hashes: HashMap<usize, u64>,
    /// Length of content for each editor (quick change detection without hashing)
    content_lengths: HashMap<usize, usize>,
    /// Last synced [`Tab::content_version`] per editor
    content_versions: HashMap<usize, u64>,
    /// Frame number for coalesced checked-out access warnings
    checked_out_warn_frame: u64,
    /// Tab IDs already warned this frame (see `checked_out_warn_frame`)
    checked_out_warned_tabs: HashSet<usize>,
}

/// Whether a large one-frame buffer shrink should trigger undo protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShrinkVerdict {
    /// Likely accidental whole-buffer replacement with a single keystroke.
    Anomalous,
    /// Expected shrink (partial delete, select-all replace, etc.).
    Normal,
}

/// Classifies a one-frame buffer shrink for ISS-36 protection (see `EditorWidget::show_inner`).
#[must_use]
pub(crate) fn classify_buffer_shrink(
    pre_len: usize,
    post_len: usize,
    pre_selection_empty: bool,
    events: &[egui::Event],
) -> ShrinkVerdict {
    if pre_len <= 64 || post_len * 2 >= pre_len {
        return ShrinkVerdict::Normal;
    }
    if !pre_selection_empty {
        return ShrinkVerdict::Normal;
    }
    if !frame_input_is_single_text_or_key(events) {
        return ShrinkVerdict::Normal;
    }
    if frame_contains_select_all(events) {
        return ShrinkVerdict::Normal;
    }
    ShrinkVerdict::Anomalous
}

fn frame_input_is_single_text_or_key(events: &[egui::Event]) -> bool {
    events
        .iter()
        .filter(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { .. }))
        .count()
        == 1
}

fn frame_contains_select_all(events: &[egui::Event]) -> bool {
    events.iter().any(|e| {
        let egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = e
        else {
            return false;
        };
        *key == egui::Key::A && (modifiers.command || modifiers.ctrl)
    })
}

fn warn_editor_checked_out(storage: &mut FerriteEditorStorage, frame: u64, tab_id: usize) {
    if storage.checked_out_warn_frame != frame {
        storage.checked_out_warn_frame = frame;
        storage.checked_out_warned_tabs.clear();
    }
    if storage.checked_out_warned_tabs.insert(tab_id) {
        log::warn!(
            "FerriteEditor for tab {} is checked out (EditorWidget::show); access denied",
            tab_id
        );
    }
}

/// Take the FerriteEditor for a tab out of egui storage for the duration of a show frame.
fn take_ferrite_editor_from_storage(
    storage: &mut FerriteEditorStorage,
    tab_id: usize,
    create: impl FnOnce() -> FerriteEditor,
) -> FerriteEditor {
    let slot = storage.editors.entry(tab_id).or_insert_with(|| {
        debug!("Creating new FerriteEditor for tab {}", tab_id);
        Some(create())
    });
    debug_assert!(
        slot.is_some(),
        "FerriteEditor for tab {} must not be double-checked-out",
        tab_id
    );
    slot.take().expect("editor slot Some before take")
}

/// Return a FerriteEditor to egui storage after `EditorWidget::show`.
fn put_ferrite_editor_in_storage(
    storage: &mut FerriteEditorStorage,
    tab_id: usize,
    editor: FerriteEditor,
) {
    let slot = storage.editors.entry(tab_id).or_insert(None);
    debug_assert!(
        slot.is_none(),
        "FerriteEditor for tab {} slot must be empty before put-back",
        tab_id
    );
    *slot = Some(editor);
    debug_assert!(slot.is_some());
}

/// Gets mutable access to a FerriteEditor by tab ID.
///
/// This function provides a way to access and mutate a FerriteEditor instance
/// from outside the widget rendering code (e.g., from app.rs handlers).
///
/// # Arguments
/// * `ctx` - The egui context (needed to access egui's data storage)
/// * `tab_id` - The ID of the tab whose editor to access
/// * `f` - A closure that receives mutable access to the editor
///
/// # Returns
/// `Some(R)` if the editor exists and the closure completed, `None` if no editor
/// exists for the given tab ID.
///
/// # Example
/// ```rust,ignore
/// let replaced = get_ferrite_editor_mut(ctx, tab_id, |editor| {
///     editor.replace_current_match("replacement")
/// });
/// ```
pub fn get_ferrite_editor_mut<R, F>(ctx: &egui::Context, tab_id: usize, f: F) -> Option<R>
where
    F: FnOnce(&mut FerriteEditor) -> R,
{
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| {
        let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);
        match storage.editors.get_mut(&tab_id) {
            None => None,
            Some(slot) => match slot.as_mut() {
                Some(editor) => Some(f(editor)),
                None => {
                    warn_editor_checked_out(storage, frame, tab_id);
                    None
                }
            },
        }
    })
}

/// Removes the FerriteEditor instance for a closed tab, freeing its memory.
///
/// This should be called when a tab is closed to prevent memory retention.
/// Each FerriteEditor contains:
/// - `TextBuffer` (rope-based storage, can be 100MB+ for large files)
/// - `LineCache` (up to 200 cached galleys)
/// - `EditHistory` (undo/redo operation stacks)
///
/// Without cleanup, these remain in egui's memory indefinitely.
///
/// # Arguments
/// * `ctx` - The egui context
/// * `tab_id` - The ID of the tab being closed
///
/// # Example
/// ```rust,ignore
/// // In tab close handler:
/// state.close_tab(index);
/// cleanup_ferrite_editor(ctx, tab_id);
/// ```
pub fn cleanup_ferrite_editor(ctx: &egui::Context, tab_id: usize) {
    ctx.data_mut(|data| {
        let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);
        // Drop the sync caches with the editor so a later tab with the same id
        // can never be judged "already in sync" against a stale entry.
        storage.content_hashes.remove(&tab_id);
        storage.content_lengths.remove(&tab_id);
        storage.content_versions.remove(&tab_id);
        match storage.editors.remove(&tab_id) {
            Some(Some(mut editor)) => {
                let buffer_chars = editor.buffer.len();
                let cache_entries = editor.line_cache.len();

                // Explicitly clear large data structures before drop
                // This helps the allocator reclaim memory more efficiently
                editor.line_cache.invalidate();
                editor.search_matches.clear();
                editor.search_matches.shrink_to_fit();

                log::info!(
                    "Cleaned up FerriteEditor for tab {}: buffer={} chars, cache={} entries",
                    tab_id,
                    buffer_chars,
                    cache_entries
                );
                // editor is dropped here, freeing TextBuffer (Rope) and remaining fields
            }
            Some(None) => {
                log::debug!(
                    "Cleaned up checked-out FerriteEditor slot for tab {} (editor owned by show frame)",
                    tab_id
                );
            }
            None => {}
        }
        storage.content_hashes.remove(&tab_id);
        storage.content_lengths.remove(&tab_id);
        storage.content_versions.remove(&tab_id);
    });
}

/// File size threshold above which some features are disabled (5MB)
const LARGE_FILE_THRESHOLD: usize = 5 * 1024 * 1024;

/// Compute a fast hash of a string for cache invalidation.
fn compute_content_hash(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Decide whether Tab content must be pushed into the FerriteEditor rope.
fn editor_needs_content_sync(
    has_editor: bool,
    existing_len: Option<usize>,
    content_len: usize,
    existing_hash: Option<u64>,
    existing_version: Option<u64>,
    tab_content_version: u64,
    is_large_file: bool,
    content: &str,
) -> (bool, u64, &'static str) {
    if !has_editor {
        let hash = compute_content_hash(content);
        return (true, hash, "no_editor");
    }
    if existing_version != Some(tab_content_version) {
        let hash = compute_content_hash(content);
        return (true, hash, "version_changed");
    }
    if existing_len != Some(content_len) {
        let hash = compute_content_hash(content);
        return (true, hash, "length_changed");
    }
    if let Some(existing) = existing_hash {
        if is_large_file {
            (false, existing, "large_file_skip")
        } else {
            let hash = compute_content_hash(content);
            if hash != existing {
                (true, hash, "hash_mismatch")
            } else {
                (false, hash, "hash_match")
            }
        }
    } else {
        let hash = compute_content_hash(content);
        (true, hash, "no_hash")
    }
}

/// Result of showing the editor widget.
pub struct EditorOutput {
    /// Whether the content was modified.
    pub changed: bool,
    /// Whether Ctrl+Click was detected (for adding cursors).
    pub ctrl_click_pos: Option<usize>,
    /// Line where fold toggle was clicked (if any).
    pub fold_toggle_line: Option<usize>,
    /// Text committed via IME that may need CJK font loading.
    /// Caller should check this and load fonts if CJK characters are detected.
    pub ime_committed_text: Option<String>,
    // ─────────────────────────────────────────────────────────────────────────
    // Scroll Metrics (for sync scrolling)
    // ─────────────────────────────────────────────────────────────────────────
    /// Total scroll offset in pixels (wrap-aware absolute Y from ViewState).
    pub scroll_offset: f32,
    /// Top-of-viewport source line (1-indexed) for scroll sync anchors.
    pub scroll_anchor_line: usize,
    /// Fraction within the anchor line (0..1) for sub-line alignment.
    pub scroll_anchor_fraction: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
    /// Total content height in pixels.
    pub content_height: f32,
    /// Current Vim mode label (None when Vim mode is disabled).
    pub vim_mode_label: Option<&'static str>,
    /// App-level undo was requested from the raw editor context menu.
    pub request_undo: bool,
    /// Context-menu Paste found a clipboard image but the tab has no `assets/`
    /// directory yet (untitled, no workspace). The app offers Save As and then
    /// pastes it, exactly like Ctrl/Cmd+V.
    pub request_image_paste: bool,
}

/// App-level actions requested from the raw editor context menu.
#[derive(Debug, Clone, Copy, Default)]
struct ContextMenuRequests {
    undo: bool,
    image_paste: bool,
}

/// Search match highlight information.
#[derive(Debug, Clone, Default)]
pub struct SearchHighlights {
    /// All matches as (start, end) byte positions
    pub matches: Vec<(usize, usize)>,
    /// Index of the current match (for distinct highlighting)
    pub current_match: usize,
    /// Whether to scroll to the current match
    pub scroll_to_match: bool,
}

/// A text editor widget that integrates with the Tab state.
///
/// This widget uses the custom FerriteEditor for high-performance editing:
/// - Virtual scrolling (only renders visible lines)
/// - Rope-based text storage (O(log n) operations)
/// - Galley caching for efficient re-rendering
///
/// # Example
///
/// ```ignore
/// EditorWidget::new(&mut tab)
///     .font_size(settings.font_size)
///     .show_line_numbers(true)
///     .search_highlights(highlights)
///     .scroll_to_line(Some(42))
///     .zen_mode(true, 80.0)
///     .highlight_matching_pairs(true)
///     .syntax_highlighting(true, Some(path), is_dark)
///     .show(ui);
/// ```
pub struct EditorWidget<'a> {
    /// The tab being edited.
    tab: &'a mut Tab,
    /// Font size for the editor.
    font_size: f32,
    /// Whether word wrap is enabled.
    word_wrap: bool,
    /// ID for the editor (for state persistence).
    id: Option<egui::Id>,
    /// Whether to show line numbers.
    show_line_numbers: bool,
    /// Whether the vertical scrollbar stays visible when the pointer leaves.
    scrollbar_always_visible: bool,
    /// Theme colors for styling line numbers.
    theme_colors: Option<ThemeColors>,
    /// Search match highlights to render.
    search_highlights: Option<SearchHighlights>,
    /// Font family for the editor.
    font_family: EditorFont,
    /// Line number to scroll to (1-indexed, from outline navigation).
    scroll_to_line: Option<usize>,
    /// Whether Zen Mode is enabled (centered text column).
    zen_mode: bool,
    /// Maximum column width in characters for Zen Mode centering.
    zen_max_column_width: f32,
    /// Transient highlight for search result navigation (char range).
    transient_highlight: Option<(usize, usize)>,
    /// Whether to show fold indicators in the gutter.
    show_fold_indicators: bool,
    /// Whether to highlight matching bracket/emphasis pairs.
    highlight_matching_pairs: bool,
    /// Whether syntax highlighting is enabled.
    syntax_highlighting: bool,
    /// File path for syntax detection (needed for determining language).
    file_path: Option<PathBuf>,
    /// Default language for syntax highlighting when no language is detected from file path.
    /// This is used for unsaved/new documents. Empty string means no default.
    default_syntax_language: String,
    /// Whether we're in dark mode (for syntax theme selection).
    is_dark_mode: bool,
    /// Maximum line width setting (applies when not in Zen Mode).
    max_line_width: MaxLineWidth,
    /// Syntax highlighting theme name (overrides dark/light mode auto-selection).
    syntax_theme: Option<String>,
    /// Pending scroll offset for sync scrolling (pixels from document top).
    /// When set, the editor will scroll to this absolute position.
    pending_sync_scroll_offset: Option<f32>,
    /// Whether auto-close brackets is enabled.
    auto_close_brackets: bool,
    /// Whether Vim modal editing is enabled.
    vim_mode: bool,
    /// Insert spaces vs tab character on Tab.
    use_spaces: bool,
    /// Tab width when `use_spaces` is true.
    tab_size: u8,
    /// When true, Enter inserts `\r\n` (tab's file uses CRLF line endings).
    newline_crlf: bool,
    /// When true, middle-click pastes the Linux primary selection.
    middle_click_paste: bool,
    /// Workspace root for resolving the `assets/` directory of unsaved docs
    /// (context-menu image paste).
    workspace_root: Option<PathBuf>,
    /// LSP diagnostics for this file (set each frame from AppState).
    diagnostics: Vec<crate::lsp::state::DiagnosticEntry>,
}

impl<'a> EditorWidget<'a> {
    /// Create a new editor widget for the given tab.
    pub fn new(tab: &'a mut Tab) -> Self {
        Self {
            tab,
            font_size: 14.0,
            word_wrap: true,
            id: None,
            show_line_numbers: true,
            scrollbar_always_visible: true,
            theme_colors: None,
            search_highlights: None,
            font_family: EditorFont::default(),
            scroll_to_line: None,
            zen_mode: false,
            zen_max_column_width: 80.0,
            transient_highlight: None,
            show_fold_indicators: true,
            highlight_matching_pairs: true,
            syntax_highlighting: false,
            file_path: None,
            default_syntax_language: String::new(),
            is_dark_mode: true,
            max_line_width: MaxLineWidth::Off,
            syntax_theme: None,
            pending_sync_scroll_offset: None,
            auto_close_brackets: false,
            vim_mode: false,
            use_spaces: true,
            tab_size: 4,
            newline_crlf: false,
            middle_click_paste: true,
            workspace_root: None,
            diagnostics: Vec::new(),
        }
    }

    /// Set the font size for the editor.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set whether word wrap is enabled.
    #[must_use]
    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    /// Set a custom ID for the editor.
    #[must_use]
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set whether to show line numbers.
    #[must_use]
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set whether the vertical scrollbar stays visible when the pointer leaves.
    #[must_use]
    pub fn scrollbar_always_visible(mut self, always: bool) -> Self {
        self.scrollbar_always_visible = always;
        self
    }

    /// Set theme colors for styling (used for line numbers).
    #[must_use]
    pub fn theme_colors(mut self, colors: ThemeColors) -> Self {
        self.theme_colors = Some(colors);
        self
    }

    /// Set search highlights to render.
    #[must_use]
    pub fn search_highlights(mut self, highlights: SearchHighlights) -> Self {
        self.search_highlights = Some(highlights);
        self
    }

    /// Set the font family for the editor.
    #[must_use]
    pub fn font_family(mut self, font_family: EditorFont) -> Self {
        self.font_family = font_family;
        self
    }

    /// Set a line to scroll to (1-indexed, for outline navigation).
    #[must_use]
    pub fn scroll_to_line(mut self, line: Option<usize>) -> Self {
        self.scroll_to_line = line;
        self
    }

    /// Enable Zen Mode with centered text column.
    ///
    /// When enabled, the text content is centered horizontally with a maximum
    /// column width, while the editor background fills the available space.
    #[must_use]
    pub fn zen_mode(mut self, enabled: bool, max_column_width: f32) -> Self {
        self.zen_mode = enabled;
        self.zen_max_column_width = max_column_width;
        self
    }

    /// Set a transient highlight for search result navigation.
    ///
    /// This is rendered as a distinct background highlight that is independent
    /// of text selection. Used when navigating to search-in-files results.
    #[must_use]
    pub fn transient_highlight(mut self, range: Option<(usize, usize)>) -> Self {
        self.transient_highlight = range;
        self
    }

    /// Set whether to show fold indicators in the gutter.
    #[must_use]
    pub fn show_fold_indicators(mut self, show: bool) -> Self {
        self.show_fold_indicators = show;
        self
    }

    /// Set whether to highlight matching bracket/emphasis pairs.
    ///
    /// When enabled, positions the cursor adjacent to a bracket like `(`, `[`, `{`, `<`
    /// or their closing counterparts, or markdown emphasis markers `**` and `__`,
    /// will highlight both the delimiter and its matching counterpart.
    #[must_use]
    pub fn highlight_matching_pairs(mut self, enabled: bool) -> Self {
        self.highlight_matching_pairs = enabled;
        self
    }

    /// Enable syntax highlighting for source code files.
    ///
    /// When enabled and the file has a recognized extension (Rust, Python, JS, etc.),
    /// the editor will apply syntax-aware coloring to the text.
    ///
    /// # Arguments
    /// * `enabled` - Whether syntax highlighting is enabled
    /// * `file_path` - Optional file path for language detection
    /// * `is_dark` - Whether to use dark mode syntax colors
    #[must_use]
    pub fn syntax_highlighting(
        mut self,
        enabled: bool,
        file_path: Option<PathBuf>,
        is_dark: bool,
    ) -> Self {
        self.syntax_highlighting = enabled;
        self.file_path = file_path;
        self.is_dark_mode = is_dark;
        self
    }

    /// Set the default syntax language for unsaved/new documents.
    ///
    /// This language is used when no language can be detected from the file path,
    /// such as for unsaved documents. Set to empty string to disable default language.
    ///
    /// # Arguments
    /// * `language` - Language identifier (e.g., "markdown", "rust", "python")
    #[must_use]
    pub fn default_syntax_language(mut self, language: String) -> Self {
        self.default_syntax_language = language;
        self
    }

    /// Set the syntax highlighting theme.
    ///
    /// When set, this theme will be used instead of the automatic dark/light mode selection.
    /// If the theme is not found, falls back to the dark/light mode default.
    ///
    /// # Arguments
    /// * `theme` - Theme name (e.g., "Dracula", "Nord", "Solarized (dark)")
    #[must_use]
    pub fn syntax_theme(mut self, theme: Option<String>) -> Self {
        self.syntax_theme = theme;
        self
    }

    /// Set the maximum line width for text centering.
    ///
    /// When enabled and the viewport is wider than the specified width,
    /// text is constrained to that width and centered horizontally.
    /// This setting is used when NOT in Zen Mode (Zen Mode has its own width setting).
    #[must_use]
    pub fn max_line_width(mut self, width: MaxLineWidth) -> Self {
        self.max_line_width = width;
        self
    }

    /// Set a pending scroll offset for sync scrolling.
    ///
    /// When set, the editor will scroll to this absolute position (pixels from document top)
    /// on the next render. Used for bidirectional scroll synchronization in split view.
    #[must_use]
    pub fn pending_sync_scroll_offset(mut self, offset: Option<f32>) -> Self {
        self.pending_sync_scroll_offset = offset;
        self
    }

    /// Set whether auto-close brackets is enabled.
    ///
    /// When enabled, typing an opening bracket/quote automatically inserts
    /// the matching closing character and positions the cursor between them.
    ///
    /// Supported pairs: `()`, `[]`, `{}`, `''`, `""`, ``` `` ```
    #[must_use]
    pub fn auto_close_brackets(mut self, enabled: bool) -> Self {
        self.auto_close_brackets = enabled;
        self
    }

    /// Set whether Vim modal editing is enabled.
    #[must_use]
    pub fn vim_mode(mut self, enabled: bool) -> Self {
        self.vim_mode = enabled;
        self
    }

    /// Configure Tab key indent style (spaces vs tab character).
    #[must_use]
    pub fn tab_settings(mut self, use_spaces: bool, tab_size: u8) -> Self {
        self.use_spaces = use_spaces;
        self.tab_size = tab_size;
        self
    }

    /// Configure whether Enter inserts `\r\n` (CRLF files) or `\n`.
    #[must_use]
    pub fn newline_crlf(mut self, crlf: bool) -> Self {
        self.newline_crlf = crlf;
        self
    }

    /// Configure Linux middle-click primary-selection paste.
    #[must_use]
    pub fn middle_click_paste(mut self, enabled: bool) -> Self {
        self.middle_click_paste = enabled;
        self
    }

    /// Workspace root used to resolve `assets/` for context-menu image paste.
    #[must_use]
    pub fn workspace_root(mut self, root: Option<PathBuf>) -> Self {
        self.workspace_root = root;
        self
    }

    /// Set LSP diagnostics for this file.
    #[must_use]
    pub fn diagnostics(mut self, diags: Vec<crate::lsp::state::DiagnosticEntry>) -> Self {
        self.diagnostics = diags;
        self
    }

    /// Show the editor widget and return the output.
    ///
    /// With the `spellcheck` feature the app calls [`Self::show_with_spellcheck`] instead.
    #[cfg_attr(feature = "spellcheck", allow(dead_code))]
    ///
    /// This uses the custom FerriteEditor which provides:
    /// - Virtual scrolling (only renders visible lines)
    /// - Rope-based text storage (O(log n) operations)
    /// - Galley caching for efficient re-rendering
    pub fn show(self, ui: &mut Ui) -> EditorOutput {
        #[cfg(feature = "spellcheck")]
        {
            self.show_inner(ui, None)
        }
        #[cfg(not(feature = "spellcheck"))]
        {
            self.show_inner(ui)
        }
    }

    /// Same as [`show`] with a live spellcheck service for the context menu.
    #[cfg(feature = "spellcheck")]
    pub fn show_with_spellcheck(
        self,
        ui: &mut Ui,
        spellcheck: Option<&mut crate::spellcheck::SpellcheckService>,
    ) -> EditorOutput {
        self.show_inner(ui, spellcheck)
    }

    fn show_inner(
        mut self,
        ui: &mut Ui,
        #[cfg(feature = "spellcheck")] spellcheck: Option<
            &mut crate::spellcheck::SpellcheckService,
        >,
    ) -> EditorOutput {
        let tab_id = self.tab.id;
        let _base_id = self.id.unwrap_or_else(|| ui.id().with("ferrite_editor"));

        // Capture an undo snapshot of the pre-edit content so edits made
        // through FerriteEditor are recorded in `tab.edit_history` (the stack
        // that Ctrl+Z / Ctrl+Y consult via `handle_undo` / `handle_redo`).
        // Without this the tab's edit history stays empty and users see
        // "Nothing to undo" even after typing — a v0.2.8 regression.
        self.tab.prepare_undo_snapshot_hashed();

        // Check if this is a large file (disable some features for performance)
        let content_len = self.tab.content.len();
        let is_large_file = content_len > LARGE_FILE_THRESHOLD;
        if is_large_file {
            debug!(
                "Large file detected ({}MB), some features may be disabled",
                content_len / (1024 * 1024)
            );
        }

        // Get or create FerriteEditor from storage
        // PERFORMANCE: Avoid hashing large content every frame!
        // Use content length as a quick check, only hash if length matches
        let (mut editor, needs_content_sync, content_hash, _sync_reason) =
            ui.ctx().data_mut(|data| {
                let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);

                // Check if we have an existing editor for this tab (not checked out)
                let has_editor = matches!(storage.editors.get(&tab_id), Some(Some(_)));
                let existing_hash = storage.content_hashes.get(&tab_id).copied();
                let existing_len = storage.content_lengths.get(&tab_id).copied();
                let existing_version = storage.content_versions.get(&tab_id).copied();
                let tab_content_version = self.tab.content_version();

                let (needs_sync, hash, sync_reason) = editor_needs_content_sync(
                    has_editor,
                    existing_len,
                    content_len,
                    existing_hash,
                    existing_version,
                    tab_content_version,
                    is_large_file,
                    &self.tab.content,
                );
                if sync_reason == "length_changed" {
                    debug!(
                        "EditorWidget sync: length changed ({:?} -> {})",
                        existing_len, content_len
                    );
                } else if sync_reason == "hash_mismatch" {
                    debug!(
                        "EditorWidget sync: hash mismatch (existing={:?}, computed={}, len={})",
                        existing_hash, hash, content_len
                    );
                }

                // Log sync decision (only when sync is needed, to avoid spam)
                if needs_sync && has_editor {
                    debug!(
                        "EditorWidget: needs_content_sync=true for tab {} (reason: {})",
                        tab_id, sync_reason
                    );
                }

                let editor = take_ferrite_editor_from_storage(storage, tab_id, || {
                    FerriteEditor::from_string(&self.tab.content)
                });

                (editor, needs_sync, hash, sync_reason)
            });

        // Sync content from Tab to FerriteEditor if external changes detected
        if needs_content_sync {
            // Hash can desync from the stored cache while the rope still matches `tab.content`
            // (e.g. after session restore). Avoid `buffer().to_string()` on large files every frame.
            let editor_hash = compute_content_hash(&editor.buffer().to_string());
            let content_actually_differs = editor_hash != content_hash;

            if !content_actually_differs {
                crate::diag::event(
                    "editor_spurious_sync",
                    format!(
                        "tab {} hash cache stale but buffer matches tab (len={})",
                        tab_id,
                        self.tab.content.len()
                    ),
                );
            } else if editor.buffer().to_string() == self.tab.content {
                // Hash collision (extremely rare) — strings match, skip sync.
                crate::diag::event(
                    "editor_hash_collision",
                    format!(
                        "tab {} DefaultHasher collision at len {}",
                        tab_id,
                        self.tab.content.len()
                    ),
                );
            } else {
                // Content actually differs - perform sync
                // Use set_content() to update buffer in-place, preserving view state,
                // syntax highlighting, and other editor configuration. This avoids
                // visual glitching that occurs when fully recreating the editor.
                debug!(
                    "Syncing Tab content to FerriteEditor for tab {} (content_len={}, scroll_offset={:.1}, cursor=({},{}))",
                    tab_id, self.tab.content.len(), self.tab.scroll_offset,
                    self.tab.cursor_position.0, self.tab.cursor_position.1
                );
                editor.set_content(&self.tab.content);

                // Restore cursor position from Tab
                let cursor_line = self.tab.cursor_position.0;
                let cursor_col = self.tab.cursor_position.1;
                editor.set_cursor(super::ferrite::Cursor::new(cursor_line, cursor_col));

                // Restore viewport/scroll position from Tab if the view was reset
                // The set_content() method preserves ViewState, so only restore
                // if the current viewport seems wrong (e.g., editor was just created)
                let line_height = editor.view().line_height();
                if line_height > 0.0 && self.tab.scroll_offset > 0.0 {
                    let current_first_line = editor.view().first_visible_line();
                    let expected_first_line = (self.tab.scroll_offset / line_height) as usize;
                    // Only restore if significantly off (more than 5 lines)
                    if current_first_line.abs_diff(expected_first_line) > 5 {
                        let total_lines = editor.buffer().line_count();
                        editor
                            .view_mut()
                            .scroll_to_absolute(self.tab.scroll_offset, total_lines);
                        debug!(
                            "Restored viewport to absolute offset {:.1}px (line_height={:.1})",
                            self.tab.scroll_offset, line_height
                        );
                    }
                }
            }
        }

        if self.tab.pending_restore_editor_reset {
            self.tab.pending_restore_editor_reset = false;
            use super::ferrite::Cursor;
            let (line, col) = self.tab.cursor_position;
            editor.set_cursor(Cursor::new(line, col));
            editor.reset_transient_input_state();
            debug!(
                "EditorWidget: reset transient input state after session restore for tab {}",
                tab_id
            );
        }

        // Apply settings from EditorWidget configuration
        editor.set_font_size(self.font_size);
        editor.set_font_family(self.font_family.clone());
        editor.set_wrap_enabled(self.word_wrap);
        editor.set_auto_close_brackets(self.auto_close_brackets);
        editor.set_vim_mode(self.vim_mode);
        editor.set_tab_settings(self.use_spaces, self.tab_size);
        editor.set_newline_crlf(self.newline_crlf);
        editor.set_middle_click_paste(self.middle_click_paste);
        editor.set_scrollbar_always_visible(self.scrollbar_always_visible);

        // Apply max line width setting (convert character count to pixels)
        // Use approximate character width based on font size
        let char_width = self.font_size * 0.6; // Approximate average character width
        let max_wrap_width_px = self.max_line_width.to_pixels(char_width);
        debug!(
            "FerriteEditor max_line_width setting: {:?} -> {:?}px (char_width={:.1})",
            self.max_line_width, max_wrap_width_px, char_width
        );
        editor.set_max_wrap_width(max_wrap_width_px);

        // Calculate centering offset for Zen Mode
        // When zen_mode is enabled and we have a max width, center the content
        let available_width = ui.available_width();
        let content_offset_x = if self.zen_mode {
            if let Some(max_width) = max_wrap_width_px {
                // Center the content: (available - content_width) / 2
                let margin = (available_width - max_width).max(0.0) / 2.0;
                debug!(
                    "Zen mode centering: available={:.1}, max_width={:.1}, margin={:.1}",
                    available_width, max_width, margin
                );
                margin
            } else {
                0.0
            }
        } else {
            0.0
        };
        editor.set_content_offset_x(content_offset_x);

        // Configure syntax highlighting
        // PERFORMANCE: Disable syntax highlighting for very large files (>10MB)
        // as even per-line highlighting has overhead from syntect parsing
        const SYNTAX_SIZE_LIMIT: usize = 10 * 1024 * 1024; // 10MB
        let syntax_enabled = self.syntax_highlighting && content_len < SYNTAX_SIZE_LIMIT;

        // Determine language from file path if syntax highlighting is enabled
        // Fall back to default language for unsaved/new documents
        let syntax_language = if syntax_enabled {
            self.file_path
                .as_ref()
                .and_then(|p| crate::markdown::syntax::language_from_path(p))
                .or_else(|| {
                    // Use default language if set and not empty
                    if self.default_syntax_language.is_empty() {
                        None
                    } else {
                        Some(self.default_syntax_language.clone())
                    }
                })
        } else {
            None
        };
        editor.configure_syntax(
            syntax_enabled && syntax_language.is_some(),
            syntax_language,
            self.is_dark_mode,
            self.syntax_theme.clone(),
        );

        // Configure search highlights
        if let Some(ref highlights) = self.search_highlights {
            editor.set_search_matches(
                highlights.matches.clone(),
                highlights.current_match,
                highlights.scroll_to_match,
            );
        } else {
            editor.clear_search_matches();
        }

        // Configure bracket matching
        // Now uses windowed search (cursor ±100 lines), safe for any file size
        editor.set_bracket_matching_enabled(self.highlight_matching_pairs);

        // Pass LSP diagnostics for this file
        editor.diagnostics = std::mem::take(&mut self.diagnostics);

        // Set bracket colors from theme if available
        if let Some(ref colors) = self.theme_colors {
            editor.set_bracket_colors(Some((
                colors.ui.matching_bracket_bg,
                colors.ui.matching_bracket_border,
            )));
        }

        // Sync fold state from Tab to FerriteEditor
        // This allows the editor to skip rendering folded lines and show fold indicators
        // Always sync fold state (not just when indicators are shown) because it affects
        // which lines are hidden from rendering
        let fold_region_count = self.tab.fold_state.regions().len();
        if fold_region_count > 0 {
            debug!(
                "Syncing {} fold regions to FerriteEditor, show_indicators={}",
                fold_region_count, self.show_fold_indicators
            );
        }
        editor.set_fold_state(self.tab.fold_state.clone());
        editor.set_show_fold_indicators(self.show_fold_indicators);
        editor.set_show_line_numbers(self.show_line_numbers);

        // Handle pending scroll offset from Tab (absolute pixel position)
        if let Some(offset) = self.tab.pending_scroll_offset.take() {
            let total_lines = editor.buffer().line_count();
            editor.view_mut().scroll_to_absolute(offset, total_lines);
        }

        // Handle pending sync scroll offset (for bidirectional scroll sync in split view)
        // This sets an absolute scroll position without moving the cursor
        if let Some(offset) = self.pending_sync_scroll_offset {
            let total_lines = editor.buffer().line_count();
            editor.view_mut().scroll_to_absolute(offset, total_lines);
            debug!(
                "EditorWidget: sync scroll to absolute offset {:.1}px",
                offset
            );
        }

        // Wrap-aware anchor scroll from rendered preview (split sync)
        if let Some((line, fraction)) = self.tab.pending_scroll_anchor.take() {
            let total_lines = editor.buffer().line_count();
            let line_0 = line.saturating_sub(1).min(total_lines.saturating_sub(1));
            let line_start = editor.view().get_line_y_offset(line_0);
            let line_h = editor.view().get_line_height(line_0);
            let target_y = line_start + fraction.clamp(0.0, 1.0) * line_h;
            editor.view_mut().scroll_to_absolute(target_y, total_lines);
            debug!(
                "EditorWidget: sync anchor scroll line {} frac {:.2} → {:.1}px",
                line, fraction, target_y
            );
        }

        // Handle scroll_to_line from outline panel / minimap navigation
        // scroll_to_line is 1-indexed, ViewState expects 0-indexed
        if let Some(line_1indexed) = self.scroll_to_line {
            let line_0indexed = line_1indexed.saturating_sub(1);
            let total_lines = editor.buffer().line_count();
            debug!(
                "EditorWidget: scroll_to_line {} (0-indexed: {}), total_lines={}, cursor_before={:?}",
                line_1indexed, line_0indexed, total_lines, editor.cursor()
            );
            // Always put the target line at the TOP of the viewport.
            // Using scroll_to_line instead of ensure_line_visible because:
            // - ensure_line_visible puts line at bottom if below viewport
            // - For outline/minimap navigation, users expect the clicked item at the top
            editor.view_mut().scroll_to_line(line_0indexed);
            // Also move the cursor to that line so outline/minimap sync works.
            // When the caller already placed the Tab cursor on this line (e.g.
            // Search-in-Files → `tab.set_cursor(match_char_offset)`), keep its
            // column instead of snapping to 0 — otherwise the caret lands at the
            // start of the line and the match highlight is left behind (QA ISS-4).
            use super::ferrite::Cursor;
            let (tab_line, tab_col) = self.tab.cursor_position;
            let column = if tab_line == line_0indexed {
                tab_col
            } else {
                0
            };
            editor.set_cursor(Cursor::new(line_0indexed, column));
            debug!(
                "EditorWidget: after set_cursor, cursor_after={:?}",
                editor.cursor()
            );
        }

        // Auto-close (and undo/redo, line ops) stash a char index here after
        // the editor has already rendered. Apply it before input so the caret
        // sits on that index — between an opener and its closer — instead of
        // remaining after the closer (QA ISS-7).
        if let Some(char_pos) = self.tab.pending_cursor_restore.take() {
            let max_chars = self.tab.content.chars().count();
            let char_pos = char_pos.min(max_chars);
            let (line, col) = char_index_to_line_col(&self.tab.content, char_pos);
            editor.set_cursor(super::ferrite::Cursor::new(line, col));
            self.tab.set_cursor(char_pos);
            debug!(
                "EditorWidget: restored pending cursor to char {} → ({}, {})",
                char_pos, line, col
            );
        }

        // Render the FerriteEditor
        // Clone the context to avoid borrow conflict
        let ctx = ui.ctx().clone();
        let cursor_before_frame = editor.cursor();
        let selection_before_frame = editor.selection();
        let buffer_len_before_frame = editor.buffer().len();
        let response = editor.ui(&ctx, ui);

        let frame_events: Vec<egui::Event> = ctx.input(|input| input.events.clone());
        let mut anomalous_buffer_shrink = false;

        // ISS-36 diagnostic net: a single frame that discards most of a
        // non-trivial buffer is almost always a whole-document selection being
        // replaced by typed text. Record enough context (pre-frame selection,
        // click count, the frame's input events) to explain the next
        // occurrence from a plain debug log.
        if editor.is_content_dirty() {
            let len_after = editor.buffer().len();
            if buffer_len_before_frame > 64 && len_after * 2 < buffer_len_before_frame {
                let events: Vec<String> = frame_events
                    .iter()
                    .map(|e| match e {
                        egui::Event::Text(t) => format!("Text({} chars)", t.chars().count()),
                        egui::Event::Key {
                            key,
                            pressed,
                            modifiers,
                            ..
                        } => format!("Key({key:?}, pressed={pressed}, mods={modifiers:?})"),
                        egui::Event::PointerButton {
                            button, pressed, ..
                        } => format!("PointerButton({button:?}, pressed={pressed})"),
                        egui::Event::Paste(_) => "Paste".to_string(),
                        egui::Event::Cut => "Cut".to_string(),
                        other => format!("{other:?}")
                            .split('(')
                            .next()
                            .unwrap_or("Event")
                            .to_string(),
                    })
                    .collect();
                anomalous_buffer_shrink = classify_buffer_shrink(
                    buffer_len_before_frame,
                    len_after,
                    selection_before_frame.is_collapsed(),
                    &frame_events,
                ) == ShrinkVerdict::Anomalous;
                if anomalous_buffer_shrink {
                    log::error!(
                        "EditorWidget: buffer for tab {} shrank {} -> {} chars in one frame \
                         (selection before frame: anchor=({},{}) head=({},{}), click_count={}, \
                         events={:?})",
                        tab_id,
                        buffer_len_before_frame,
                        len_after,
                        selection_before_frame.anchor.line,
                        selection_before_frame.anchor.column,
                        selection_before_frame.head.line,
                        selection_before_frame.head.column,
                        editor.click_count,
                        events
                    );
                } else {
                    log::warn!(
                        "EditorWidget: buffer for tab {} shrank {} -> {} chars in one frame \
                         (selection before frame: anchor=({},{}) head=({},{}), click_count={}, \
                         events={:?})",
                        tab_id,
                        buffer_len_before_frame,
                        len_after,
                        selection_before_frame.anchor.line,
                        selection_before_frame.anchor.column,
                        selection_before_frame.head.line,
                        selection_before_frame.head.column,
                        editor.click_count,
                        events
                    );
                }
            }
        }

        if response.lost_focus() {
            self.tab.break_undo_group();
        }

        let context_menu_requests = show_raw_editor_context_menu(
            &response,
            &mut editor,
            self.tab,
            self.workspace_root.as_deref(),
            #[cfg(feature = "spellcheck")]
            spellcheck,
        );

        // Handle auto-focus for new tabs
        // When needs_focus is set (new tab, newly opened file), request keyboard focus
        // so the user can start typing immediately without clicking
        if self.tab.needs_focus {
            response.request_focus();
            self.tab.needs_focus = false;
            debug!("EditorWidget: auto-focused tab {}", self.tab.id);
        }

        // Update Tab's scroll metrics from FerriteEditor
        // These are used by the minimap and outline panel for position sync
        // Capture all scroll metrics at once to avoid borrow conflicts later
        let (line_height, viewport_height_val, content_height_val, absolute_scroll_y) = {
            let view = editor.view();
            let total_lines = editor.buffer().line_count();
            (
                view.line_height(),
                view.viewport_height(),
                view.total_content_height(total_lines),
                view.current_scroll_y(),
            )
        };
        self.tab.scroll_offset = absolute_scroll_y;
        self.tab.raw_line_height = line_height;
        self.tab.content_height = content_height_val;

        // Sync content from FerriteEditor back to Tab
        // This is critical for find/replace, preview, and other features that read tab.content
        //
        // PERFORMANCE: Only convert rope to string when content actually changed!
        // Previously this was doing 80MB string allocation + comparison EVERY FRAME,
        // causing massive lag and memory pressure for large files.
        // Now we use FerriteEditor's dirty flag (set during input processing) to
        // detect changes without any string operations.
        let changed = editor.is_content_dirty();
        if changed {
            // Only allocate string when we know content changed
            self.tab.content = editor.buffer().to_string();
            // Diff against the pre-edit snapshot captured at the top of
            // show(). This:
            //   1. Pushes ops onto `tab.edit_history` so Ctrl+Z / Ctrl+Y
            //      (which read this stack via handle_undo/handle_redo)
            //      work for raw-mode edits.
            //   2. Bumps `content_version`, invalidating the cached
            //      `is_modified()` so the title bar dirty-mark, the
            //      on-close save prompt and auto-save all notice the
            //      edit.
            // Hotfix for v0.2.8 undo-broken / data-loss regression.
            if anomalous_buffer_shrink {
                self.tab.record_anomalous_buffer_shrink_from_snapshot();
            } else {
                let cursor = editor.cursor();
                let (editor_kind, timestamp_ms, break_before) =
                    editor.take_external_undo_recording();
                self.tab.record_external_edit_from_snapshot_coalesced(
                    break_before,
                    cursor.line,
                    cursor.column,
                    timestamp_ms,
                    editor_kind,
                );
            }
            self.tab.mark_content_edited();
        } else if !self.tab.skip_cursor_sync {
            let cursor = editor.cursor();
            if (cursor.line, cursor.column)
                != (cursor_before_frame.line, cursor_before_frame.column)
            {
                self.tab.break_undo_group();
            }
        }

        // Sync cursor position for outline panel / minimap bidirectional sync
        // Skip if navigation just happened (to preserve the position set by navigate_to_heading)
        if self.tab.skip_cursor_sync {
            self.tab.skip_cursor_sync = false; // Clear flag for next frame
            debug!(
                "EditorWidget: skipping cursor sync (navigation in progress), keeping cursor at {:?}",
                self.tab.cursor_position
            );
        } else {
            // This is cheap (no string allocation) and needed for proper UI coordination.
            // Also publish the char index on `tab.cursors` so post-render auto-close
            // reads the caret the editor just painted (it inserts the closer at
            // `cursors.primary().head`). A stale head inserts at the previous
            // index and the next keystroke lands behind the closer.
            let cursor = editor.cursor();
            let old_pos = self.tab.cursor_position;
            if self.tab.cursors.is_single() {
                let sel = editor.selection();
                let head =
                    line_col_to_char_index(&self.tab.content, sel.head.line, sel.head.column);
                let anchor =
                    line_col_to_char_index(&self.tab.content, sel.anchor.line, sel.anchor.column);
                if head == anchor {
                    self.tab.set_cursor(head);
                } else {
                    self.tab.set_selection(anchor, head);
                }
            } else {
                self.tab.cursor_position = (cursor.line, cursor.column);
            }
            if old_pos != self.tab.cursor_position {
                debug!(
                    "EditorWidget: cursor sync changed: {:?} -> {:?}",
                    old_pos, self.tab.cursor_position
                );
            }
        }

        // Check for IME committed text (for CJK font loading)
        let ime_committed_text = editor.take_ime_committed_text();

        // Check for fold toggle events
        let fold_toggle_line = editor.take_fold_toggle_line();

        // Sync fold state back to Tab if a fold was toggled
        if fold_toggle_line.is_some() {
            self.tab.fold_state = editor.fold_state().clone();
        }

        // Update content hash to reflect the new content (always persist `content_hash`
        // after a mismatch check — fixes spurious per-frame re-sync).
        let new_content_hash = if changed {
            compute_content_hash(&self.tab.content)
        } else if needs_content_sync {
            content_hash
        } else {
            content_hash
        };
        let new_content_len = self.tab.content.len();

        // Use absolute scroll position for output (accounts for wrapped line heights)
        let scroll_total_offset = absolute_scroll_y;

        let (scroll_anchor_line, scroll_anchor_fraction) = {
            let view = editor.view();
            let line_0 = view.first_visible_line();
            let line_start = view.get_line_y_offset(line_0);
            let line_h = view.get_line_height(line_0);
            let fraction = if line_h > 0.0 {
                ((absolute_scroll_y - line_start) / line_h).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (line_0.saturating_add(1), fraction)
        };

        // Capture Vim mode label before storing editor back
        let vim_mode_label = editor.vim_mode().map(|m| m.label());

        // Store the editor back
        ui.ctx().data_mut(|data| {
            let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);
            put_ferrite_editor_in_storage(storage, tab_id, editor);
            storage.content_hashes.insert(tab_id, new_content_hash);
            storage.content_lengths.insert(tab_id, new_content_len);
            storage
                .content_versions
                .insert(tab_id, self.tab.content_version());
        });

        EditorOutput {
            changed,
            ctrl_click_pos: None, // Multi-cursor handled internally by FerriteEditor
            fold_toggle_line,
            ime_committed_text,
            scroll_offset: scroll_total_offset,
            scroll_anchor_line,
            scroll_anchor_fraction,
            viewport_height: viewport_height_val,
            content_height: content_height_val,
            vim_mode_label,
            request_undo: context_menu_requests.undo,
            request_image_paste: context_menu_requests.image_paste,
        }
    }
}

#[cfg(feature = "spellcheck")]
fn spell_word_from_diag(
    editor: &FerriteEditor,
    diag: &crate::lsp::state::DiagnosticEntry,
) -> String {
    let Some(line) = editor.buffer().get_line(diag.start_line) else {
        return String::new();
    };
    let line = line.trim_end_matches(['\r', '\n']);
    let take = diag.end_col.saturating_sub(diag.start_col);
    line.chars().skip(diag.start_col).take(take).collect()
}

#[cfg(feature = "spellcheck")]
fn prepend_spellcheck_menu_items(
    ui: &mut Ui,
    editor: &mut FerriteEditor,
    spellcheck: Option<&mut crate::spellcheck::SpellcheckService>,
) {
    let Some(svc) = spellcheck else {
        return;
    };
    let cursor = editor
        .last_pointer_cursor()
        .unwrap_or_else(|| editor.primary_selection().head);
    let Some(diag) = editor
        .diagnostics_at(&cursor)
        .find(|d| d.source.as_deref() == Some("spell"))
        .cloned()
    else {
        return;
    };
    let word = spell_word_from_diag(editor, &diag);
    if word.is_empty() {
        return;
    }

    svc.request_suggest_once(&word);
    if let Some(list) = svc.suggestions_for(&word) {
        for suggestion in list.iter().take(5) {
            if ui.button(suggestion).clicked() {
                let start = super::ferrite::Cursor::new(diag.start_line, diag.start_col);
                let end = super::ferrite::Cursor::new(diag.end_line, diag.end_col);
                editor.replace_word_range(start, end, suggestion);
                ui.close();
            }
        }
    } else {
        ui.label(t!("context_menu.spell.loading_suggestions").to_string());
    }
    ui.separator();
    if ui
        .button(t!("context_menu.spell.add_to_dictionary").to_string())
        .clicked()
    {
        svc.add_word(&word);
        ui.close();
    }
    if ui
        .button(t!("context_menu.spell.ignore_word").to_string())
        .clicked()
    {
        svc.ignore_word(&word);
        ui.close();
    }
    ui.separator();
}

/// Right-click context menu for the raw FerriteEditor (Copy/Cut/Paste/Select All/Undo).
fn show_raw_editor_context_menu(
    response: &egui::Response,
    editor: &mut FerriteEditor,
    tab: &Tab,
    workspace_root: Option<&std::path::Path>,
    #[cfg(feature = "spellcheck")] mut spellcheck: Option<
        &mut crate::spellcheck::SpellcheckService,
    >,
) -> ContextMenuRequests {
    let mut requests = ContextMenuRequests::default();
    response.context_menu(|ui| {
        #[cfg(feature = "spellcheck")]
        {
            prepend_spellcheck_menu_items(ui, editor, spellcheck.as_deref_mut());
        }
        if ui
            .add_enabled(
                editor.has_any_selection(),
                egui::Button::new(t!("shortcuts.edit.copy").to_string()),
            )
            .clicked()
        {
            editor.copy_selection_to_clipboard(ui);
            ui.close();
        }
        if ui
            .add_enabled(
                editor.has_any_selection(),
                egui::Button::new(t!("shortcuts.edit.cut").to_string()),
            )
            .clicked()
        {
            editor.cut_selection_to_clipboard(ui);
            ui.close();
        }
        if ui.button(t!("shortcuts.edit.paste").to_string()).clicked() {
            if let Ok(mut clipboard) = Clipboard::new() {
                // Prefer prose text; URL text yields to image bytes (browser Copy image).
                let pasted_text = clipboard.get_text().ok().filter(|t| !t.trim().is_empty());
                let prefer_image = pasted_text
                    .as_ref()
                    .map(|t| crate::app::FerriteApp::is_url(t.trim()))
                    .unwrap_or(true);
                let mut pasted_image = false;
                if prefer_image {
                    if let Ok(image) = clipboard.get_image() {
                        // Workspace root keeps unsaved-doc pastes out of the
                        // process CWD (same resolution as keyboard paste).
                        if let Some(assets_dir) = crate::path_utils::assets_dir_for_write(
                            tab.path.as_deref(),
                            workspace_root,
                        ) {
                            let filename = format!(
                                "{}-clipboard.png",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0)
                            );
                            match crate::app::save_clipboard_rgba_to_assets(
                                &assets_dir,
                                image.width,
                                image.height,
                                image.bytes.as_ref(),
                                &filename,
                            ) {
                                Ok((_, markdown)) => {
                                    // Same path as text paste: editor dirty → tab sync → undo snapshot.
                                    editor.paste_text(&markdown);
                                    pasted_image = true;
                                }
                                Err(e) => {
                                    debug!("Context menu clipboard image paste failed: {}", e);
                                }
                            }
                        } else {
                            // Untitled tab, no workspace: nowhere to write the PNG
                            // from here. Hand off to the app, which offers Save As
                            // and then pastes (or toasts if declined).
                            requests.image_paste = true;
                        }
                    }
                }
                // A deferred image paste owns this clipboard, as with Ctrl/Cmd+V:
                // do not also paste its URL text.
                if !pasted_image && !requests.image_paste {
                    if let Some(text) = pasted_text {
                        editor.paste_text(&text);
                    }
                }
            }
            ui.close();
        }
        if ui
            .button(t!("shortcuts.edit.select_all").to_string())
            .clicked()
        {
            editor.select_all();
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                tab.can_undo(),
                egui::Button::new(t!("shortcuts.edit.undo").to_string()),
            )
            .clicked()
        {
            requests.undo = true;
            ui.close();
        }
    });
    requests
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a character index to (line, column) position.
///
/// Both line and column are 0-indexed.
fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in text.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Convert (line, column) position to a character index.
///
/// Both line and column are 0-indexed.
/// Returns the closest valid index if position is out of bounds.
fn line_col_to_char_index(text: &str, line: usize, col: usize) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;

    for (i, ch) in text.chars().enumerate() {
        if current_line == line && current_col == col {
            return i;
        }
        if ch == '\n' {
            if current_line == line {
                // Reached end of target line before reaching column
                return i;
            }
            current_line += 1;
            current_col = 0;
        } else if current_line == line {
            current_col += 1;
        }
    }

    // Return end of text if position is beyond
    text.chars().count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Editor-side half of the ISS-36 restore-then-type script.
///
/// Applies the first-frame caret + transient reset (`pending_restore_editor_reset`),
/// moves to the end of the current line, and inserts three characters.
/// `prior_triple_click` plants `click_count = 3` before that reset.
#[cfg(test)]
pub(crate) fn type_three_chars_after_line_end(
    content: &str,
    cursor_line: usize,
    cursor_col: usize,
    vim_mode: bool,
    prior_triple_click: bool,
    word_wrap: bool,
) -> String {
    let mut editor = FerriteEditor::from_string(content);
    editor.set_cursor(Cursor::new(cursor_line, cursor_col));
    if prior_triple_click {
        editor.click_count = 3;
    }
    editor.reset_transient_input_state();
    editor.set_vim_mode(vim_mode);
    if vim_mode {
        editor.vim_state.mode = super::ferrite::vim::VimMode::Insert;
    }
    editor.set_wrap_enabled(word_wrap);
    assert!(
        !editor.selection().is_range(),
        "first frame after restore must not keep a selection (click_count={})",
        editor.click_count
    );
    assert_eq!(editor.click_count, 0);

    let head = editor.selection().head;
    let line_text = editor.buffer().line(head.line);
    let end_col = line_text.trim_end_matches(['\n', '\r']).chars().count();
    editor.set_cursor(Cursor::new(head.line, end_col));
    editor.insert_text_at_all_cursors("abc");
    editor.buffer().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_cursor_restore_sits_between_auto_close_pair() {
        // Closer was inserted at the caret; the stored index is between them.
        let text = "()`";
        assert_eq!(char_index_to_line_col(text, 1), (0, 1));
        let fence = "``";
        assert_eq!(char_index_to_line_col(fence, 1), (0, 1));
        // Round-trip: that column is the same char index post-render will read.
        assert_eq!(line_col_to_char_index(fence, 0, 1), 1);
    }

    #[test]
    fn test_char_index_to_line_col_empty() {
        assert_eq!(char_index_to_line_col("", 0), (0, 0));
    }

    #[test]
    fn test_char_index_to_line_col_single_line() {
        let text = "Hello, World!";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0));
        assert_eq!(char_index_to_line_col(text, 5), (0, 5));
        assert_eq!(char_index_to_line_col(text, 13), (0, 13));
    }

    #[test]
    fn test_char_index_to_line_col_multiline() {
        let text = "Hello\nWorld\n!";
        assert_eq!(char_index_to_line_col(text, 0), (0, 0)); // 'H'
        assert_eq!(char_index_to_line_col(text, 5), (0, 5)); // '\n'
        assert_eq!(char_index_to_line_col(text, 6), (1, 0)); // 'W'
        assert_eq!(char_index_to_line_col(text, 11), (1, 5)); // '\n'
        assert_eq!(char_index_to_line_col(text, 12), (2, 0)); // '!'
    }

    #[test]
    fn test_line_col_to_char_index_empty() {
        assert_eq!(line_col_to_char_index("", 0, 0), 0);
    }

    #[test]
    fn test_line_col_to_char_index_single_line() {
        let text = "Hello, World!";
        assert_eq!(line_col_to_char_index(text, 0, 0), 0);
        assert_eq!(line_col_to_char_index(text, 0, 5), 5);
        assert_eq!(line_col_to_char_index(text, 0, 13), 13);
    }

    #[test]
    fn test_line_col_to_char_index_multiline() {
        let text = "Hello\nWorld\n!";
        assert_eq!(line_col_to_char_index(text, 0, 0), 0); // 'H'
        assert_eq!(line_col_to_char_index(text, 1, 0), 6); // 'W'
        assert_eq!(line_col_to_char_index(text, 2, 0), 12); // '!'
    }

    #[test]
    fn test_line_col_to_char_index_out_of_bounds() {
        let text = "Hi\nBye";
        // Column beyond line length
        assert_eq!(line_col_to_char_index(text, 0, 10), 2); // end of first line
                                                            // Line beyond text
        assert_eq!(line_col_to_char_index(text, 5, 0), 6); // end of text
    }

    #[test]
    fn test_roundtrip_conversion() {
        let text = "Line 1\nLine 2\nLine 3";

        // Test various positions
        for char_idx in [0, 3, 6, 7, 10, 13, 14, 17, 20] {
            if char_idx <= text.chars().count() {
                let (line, col) = char_index_to_line_col(text, char_idx);
                let back = line_col_to_char_index(text, line, col);
                assert_eq!(back, char_idx, "Roundtrip failed for index {}", char_idx);
            }
        }
    }

    #[test]
    fn test_editor_needs_content_sync_large_file_version_bump() {
        let len = 6 * 1024 * 1024;
        let content_a = "a".repeat(len);
        let content_b = "b".repeat(len);

        let (needs, _, reason) = editor_needs_content_sync(
            true,
            Some(len),
            len,
            Some(compute_content_hash(&content_a)),
            Some(1),
            1,
            true,
            &content_b,
        );
        assert!(!needs, "same version must skip hash on large files");
        assert_eq!(reason, "large_file_skip");

        let (needs, _, reason) = editor_needs_content_sync(
            true,
            Some(len),
            len,
            Some(compute_content_hash(&content_a)),
            Some(1),
            2,
            true,
            &content_b,
        );
        assert!(needs, "bumped content_version must force large-file resync");
        assert_eq!(reason, "version_changed");
    }

    #[test]
    fn editor_slot_checkout_blocks_get_mut_until_put_back() {
        let ctx = egui::Context::default();
        const TAB: usize = 4242;
        const CONTENT: &str = "slot-checkout-markers";

        let checked_out = ctx.data_mut(|data| {
            let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);
            take_ferrite_editor_from_storage(storage, TAB, || FerriteEditor::from_string(CONTENT))
        });
        assert_eq!(checked_out.buffer().to_string(), CONTENT);

        assert!(
            get_ferrite_editor_mut(&ctx, TAB, |e| e.buffer().to_string()).is_none(),
            "get_ferrite_editor_mut must not observe a placeholder while checked out"
        );

        let marker = "after-checkout-mut";
        let mut checked_out = checked_out;
        checked_out.set_content(marker);

        ctx.data_mut(|data| {
            let storage = data.get_temp_mut_or_default::<FerriteEditorStorage>(egui::Id::NULL);
            put_ferrite_editor_in_storage(storage, TAB, checked_out);
        });

        let after = get_ferrite_editor_mut(&ctx, TAB, |e| e.buffer().to_string());
        assert_eq!(after.as_deref(), Some(marker));
    }

    #[test]
    fn classify_buffer_shrink_anomalous_single_text_event() {
        let events = vec![egui::Event::Text("abc".into())];
        assert_eq!(
            classify_buffer_shrink(200, 3, true, &events),
            ShrinkVerdict::Anomalous
        );
    }

    #[test]
    fn classify_buffer_shrink_normal_with_select_all_shortcut() {
        let events = vec![
            egui::Event::Key {
                key: egui::Key::A,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
                physical_key: None,
            },
            egui::Event::Text("abc".into()),
        ];
        assert_eq!(
            classify_buffer_shrink(200, 3, true, &events),
            ShrinkVerdict::Normal
        );
    }

    #[test]
    fn classify_buffer_shrink_normal_partial_shrink() {
        let events = vec![egui::Event::Text("x".into())];
        assert_eq!(
            classify_buffer_shrink(200, 150, true, &events),
            ShrinkVerdict::Normal
        );
    }

    #[test]
    fn classify_buffer_shrink_normal_with_prior_selection() {
        let events = vec![egui::Event::Text("abc".into())];
        assert_eq!(
            classify_buffer_shrink(200, 3, false, &events),
            ShrinkVerdict::Normal
        );
    }

    #[test]
    fn spell_replacement_is_one_undo_group() {
        let mut tab = crate::state::Tab::new(1);
        tab.content = "helo world".into();
        tab.prepare_undo_snapshot_hashed();
        let mut editor = FerriteEditor::from_string(&tab.content);
        editor.replace_word_range(
            crate::editor::ferrite::Cursor::new(0, 0),
            crate::editor::ferrite::Cursor::new(0, 4),
            "hello",
        );
        tab.content = editor.buffer().to_string();
        tab.record_external_edit_from_snapshot();
        assert_eq!(tab.undo_count(), 1, "replacement must be one undo step");
        assert_eq!(tab.content, "hello world");
    }
}
