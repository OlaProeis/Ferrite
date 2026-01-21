//! Text editor widget for Ferrite
//!
//! This module implements the main text editor widget using egui's TextEdit,
//! with support for text input, cursor movement, selection, clipboard operations,
//! scrolling, and optional line numbers.

use crate::config::{EditorFont, MaxLineWidth};
use crate::editor::matching::DelimiterMatcher;
use crate::fonts;
use crate::markdown::syntax::{highlight_code, highlight_code_with_theme, language_from_path, HighlightedLine};
use crate::state::Tab;
use crate::theme::ThemeColors;
use eframe::egui::{self, FontId, ScrollArea, TextEdit, Ui};
use log::debug;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Syntax Highlighting Cache
// ─────────────────────────────────────────────────────────────────────────────

/// Cached syntax highlighting entry.
#[derive(Clone)]
struct SyntaxCacheEntry {
    /// Hash of the content that was highlighted
    content_hash: u64,
    /// Language used for highlighting
    language: String,
    /// Whether dark mode was active
    is_dark: bool,
    /// Syntax theme name (None = auto based on dark mode)
    syntax_theme: Option<String>,
    /// The highlighted lines
    lines: Vec<HighlightedLine>,
}

/// Cached Galley for large files - avoids rebuilding LayoutJob every frame
#[derive(Clone)]
struct GalleyCacheEntry {
    /// Hash of content + settings that affect the galley
    cache_key: u64,
    /// The cached galley
    galley: Arc<egui::Galley>,
}

/// State for deferred syntax highlighting
#[derive(Clone, Default)]
struct DeferredHighlightState {
    /// Content hash when highlighting was last deferred
    deferred_hash: u64,
    /// Frame count when content last changed
    last_change_frame: u64,
}

/// Cached syntax highlighting data stored in egui's memory.
#[derive(Clone, Default)]
struct SyntaxHighlightCache {
    /// Map from editor ID to cached highlight entry
    cache: HashMap<egui::Id, SyntaxCacheEntry>,
    /// Map from editor ID to cached galley (for large files)
    galley_cache: HashMap<egui::Id, GalleyCacheEntry>,
    /// Map from editor ID to deferred highlight state
    deferred_state: HashMap<egui::Id, DeferredHighlightState>,
}

/// Compute a fast hash of a string for cache invalidation.
fn compute_content_hash(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Compute a hash for galley cache key (content + layout settings).
fn compute_galley_cache_key(content_hash: u64, wrap_width: f32, font_size: f32, is_dark: bool) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content_hash.hash(&mut hasher);
    wrap_width.to_bits().hash(&mut hasher);
    font_size.to_bits().hash(&mut hasher);
    is_dark.hash(&mut hasher);
    hasher.finish()
}

/// Result of showing the editor widget.
pub struct EditorOutput {
    /// Whether the content was modified.
    pub changed: bool,
    /// Whether Ctrl+Click was detected (for adding cursors).
    pub ctrl_click_pos: Option<usize>,
    /// Line where fold toggle was clicked (if any).
    pub fold_toggle_line: Option<usize>,
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
/// This widget wraps egui's TextEdit with additional functionality:
/// - Integration with Tab's undo/redo stack
/// - Cursor position tracking (line, column)
/// - Scroll offset persistence
/// - Font size and styling from Settings
/// - Optional line number gutter
/// - Search match highlighting
/// - Scroll-to-line navigation (for outline panel)
/// - Zen Mode centered text column
/// - Bracket and emphasis matching highlights
/// - Syntax highlighting for source code files
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
    /// Whether to show a frame around the editor.
    frame: bool,
    /// Whether word wrap is enabled.
    word_wrap: bool,
    /// ID for the editor (for state persistence).
    id: Option<egui::Id>,
    /// Whether to show line numbers.
    show_line_numbers: bool,
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
    /// Whether we're in dark mode (for syntax theme selection).
    is_dark_mode: bool,
    /// Maximum line width setting (applies when not in Zen Mode).
    max_line_width: MaxLineWidth,
    /// Syntax highlighting theme name (overrides dark/light mode auto-selection).
    syntax_theme: Option<String>,
}

impl<'a> EditorWidget<'a> {
    /// Create a new editor widget for the given tab.
    pub fn new(tab: &'a mut Tab) -> Self {
        Self {
            tab,
            font_size: 14.0,
            frame: false,
            word_wrap: true,
            id: None,
            show_line_numbers: true,
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
            is_dark_mode: true,
            max_line_width: MaxLineWidth::Off,
            syntax_theme: None,
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

    /// Show the editor widget and return the output.
    pub fn show(self, ui: &mut Ui) -> EditorOutput {
        // Include content_version in the ID so that egui treats the TextEdit as
        // a new widget when content changes externally (e.g., via undo/redo).
        // This forces the TextEdit to re-read from the source string.
        let base_id = self.id.unwrap_or_else(|| ui.id().with("editor"));
        let id = base_id.with(self.tab.content_version());

        // Check if we need to request focus (new tab) and clear the flag
        let needs_focus = self.tab.needs_focus;
        if needs_focus {
            self.tab.needs_focus = false;
        }

        // Check if we need to restore cursor position (after undo/redo)
        let pending_cursor = self.tab.pending_cursor_restore.take();

        // Store original content and cursor for change detection and undo
        let original_content = self.tab.content.clone();
        let original_cursor = self.tab.cursors.primary().head;

        // Capture values for closures
        let font_size = self.font_size;
        let word_wrap = self.word_wrap;
        let show_line_numbers = self.show_line_numbers;
        let show_fold_indicators = self.show_fold_indicators;
        let theme_colors = self.theme_colors.clone();
        let search_highlights = self.search_highlights.clone();
        let transient_highlight = self.transient_highlight;
        let highlight_matching_pairs = self.highlight_matching_pairs;

        // Get fold indicator lines if folding is enabled
        let fold_indicators: Vec<(usize, bool)> = if show_fold_indicators {
            self.tab.fold_indicator_lines()
        } else {
            Vec::new()
        };
        let has_folds = !fold_indicators.is_empty();
        

        // Calculate gutter width if line numbers are enabled
        // Add extra space for fold indicators if there are folds
        let fold_indicator_width = if show_fold_indicators && has_folds { 16.0 } else { 0.0 };
        let gutter_width = if show_line_numbers {
            let line_count = super::line_numbers::count_lines(&self.tab.content);
            let digit_count = if line_count == 0 {
                1
            } else {
                (line_count as f32).log10().floor() as usize + 1
            };
            let char_width = font_size * 0.6;
            let content_width = char_width * digit_count as f32;
            (content_width + 20.0 + fold_indicator_width).max(30.0 + fold_indicator_width)
        } else if show_fold_indicators && has_folds {
            fold_indicator_width + 4.0 // Just fold indicators, small padding
        } else {
            0.0
        };

        // Create a mutable reference to the content
        let content = &mut self.tab.content;

        // Get font family for the editor
        let font_family = fonts::get_styled_font_family(false, false, &self.font_family);

        // Determine syntax language from file path (if syntax highlighting is enabled)
        let syntax_language = if self.syntax_highlighting {
            self.file_path
                .as_ref()
                .and_then(|p| language_from_path(p))
        } else {
            None
        };
        let is_dark_mode = self.is_dark_mode;

        // Get current frame count for deferred highlighting timing
        let current_frame = ui.ctx().frame_nr();
        
        // Get cached data (highlights, galley, deferred state)
        let (cached_entry, cached_galley, deferred_state): (Option<SyntaxCacheEntry>, Option<GalleyCacheEntry>, Option<DeferredHighlightState>) = 
            if syntax_language.is_some() {
                ui.ctx().data_mut(|data| {
                    let cache = data.get_temp_mut_or_default::<SyntaxHighlightCache>(egui::Id::NULL);
                    (
                        cache.cache.get(&id).cloned(),
                        cache.galley_cache.get(&id).cloned(),
                        cache.deferred_state.get(&id).cloned(),
                    )
                })
            } else {
                (None, None, None)
            };

        // Clone what we need for the layouter closure
        let syntax_lang_for_layouter = syntax_language.clone();
        let syntax_theme_for_layouter = self.syntax_theme.clone();
        let ctx_clone = ui.ctx().clone();

        // Configure the text layout based on word wrap
        let font_family_clone = font_family.clone();
        let mut layouter = move |ui: &Ui, text: &str, wrap_width: f32| -> Arc<egui::Galley> {
            let font_id = FontId::new(font_size, font_family_clone.clone());
            let default_text_color = ui.visuals().text_color();

            // Use syntax highlighting if we have a recognized language
            if let Some(ref lang) = syntax_lang_for_layouter {
                let text_hash = compute_content_hash(text);
                let galley_key = compute_galley_cache_key(text_hash, wrap_width, font_size, is_dark_mode);
                let line_count = text.lines().count();
                
                // PERFORMANCE THRESHOLDS:
                // - Small files (< 500 lines): Always full syntax highlighting
                // - Medium files (500-1500 lines): Deferred highlighting while typing
                // - Large files (1500+ lines): Deferred + galley caching
                const SMALL_THRESHOLD: usize = 500;
                const LARGE_THRESHOLD: usize = 1500;
                
                // Check if we can use cached galley (only for large files)
                if line_count >= LARGE_THRESHOLD {
                    if let Some(ref cached) = cached_galley {
                        if cached.cache_key == galley_key {
                            // Galley cache hit - return immediately, no work needed!
                            return cached.galley.clone();
                        }
                    }
                }
                
                // For medium and large files, use deferred highlighting
                // This keeps typing responsive - colors appear after ~0.5s pause
                // v0.3.0 will have a rebuilt text editor with proper virtual scrolling
                let use_deferred = line_count >= SMALL_THRESHOLD;
                
                let should_use_fast_mode = if use_deferred {
                    let state = deferred_state.clone().unwrap_or_default();
                    let content_changed = state.deferred_hash != text_hash;
                    let frames_since = current_frame.saturating_sub(state.last_change_frame);
                    
                    // Update deferred state
                    ctx_clone.data_mut(|data| {
                        let cache = data.get_temp_mut_or_default::<SyntaxHighlightCache>(egui::Id::NULL);
                        let entry = cache.deferred_state.entry(id).or_default();
                        
                        if content_changed {
                            entry.deferred_hash = text_hash;
                            entry.last_change_frame = current_frame;
                        }
                    });
                    
                    // Use fast mode while typing - exit after ~0.5s of no changes
                    // (~30 frames at 60fps = 500ms)
                    content_changed || frames_since < 30
                } else {
                    // Small files always get immediate highlighting
                    false
                };
                
                if should_use_fast_mode {
                    // FAST PATH: Simple layout while typing
                    // Colors will appear ~0.5s after typing stops
                    let job = if word_wrap {
                        egui::text::LayoutJob::simple(
                            text.to_owned(),
                            font_id,
                            default_text_color,
                            wrap_width,
                        )
                    } else {
                        egui::text::LayoutJob::simple_singleline(
                            text.to_owned(),
                            font_id,
                            default_text_color,
                        )
                    };
                    
                    // Schedule repaint to restore colors after pause
                    ctx_clone.request_repaint_after(std::time::Duration::from_millis(550));
                    
                    return ui.fonts(|f| f.layout_job(job));
                }
                
                // HIGHLIGHTING PATH: Build full syntax-highlighted layout
                
                // Check if we have valid cached highlights
                let use_cached_highlights = cached_entry.as_ref().map_or(false, |entry| {
                    entry.content_hash == text_hash
                        && entry.language == *lang
                        && entry.is_dark == is_dark_mode
                        && entry.syntax_theme == syntax_theme_for_layouter
                });

                let highlighted_lines = if use_cached_highlights {
                    cached_entry.as_ref().unwrap().lines.clone()
                } else {
                    debug!("Syntax highlighting: cache miss, regenerating for {} lines", line_count);
                    // Use theme-specific highlighting if a theme is set, otherwise use dark/light mode
                    let lines = if let Some(ref theme_name) = syntax_theme_for_layouter {
                        highlight_code_with_theme(text, lang, theme_name, is_dark_mode)
                    } else {
                        highlight_code(text, lang, is_dark_mode)
                    };

                    ctx_clone.data_mut(|data| {
                        let cache = data.get_temp_mut_or_default::<SyntaxHighlightCache>(egui::Id::NULL);
                        cache.cache.insert(
                            id,
                            SyntaxCacheEntry {
                                content_hash: text_hash,
                                language: lang.clone(),
                                is_dark: is_dark_mode,
                                syntax_theme: syntax_theme_for_layouter.clone(),
                                lines: lines.clone(),
                            },
                        );
                    });

                    lines
                };

                // Build the LayoutJob
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = if word_wrap { wrap_width } else { f32::INFINITY };

                for line in &highlighted_lines {
                    for segment in &line.segments {
                        let mut format = egui::text::TextFormat::default();
                        format.font_id = font_id.clone();
                        format.color = segment.foreground;
                        job.append(&segment.text, 0.0, format);
                    }
                }

                if text.is_empty() {
                    let mut format = egui::text::TextFormat::default();
                    format.font_id = font_id.clone();
                    format.color = default_text_color;
                    job.append("", 0.0, format);
                }

                let galley = ui.fonts(|f| f.layout_job(job));
                
                // Cache the galley for large files
                if line_count >= LARGE_THRESHOLD {
                    ctx_clone.data_mut(|data| {
                        let cache = data.get_temp_mut_or_default::<SyntaxHighlightCache>(egui::Id::NULL);
                        cache.galley_cache.insert(
                            id,
                            GalleyCacheEntry {
                                cache_key: galley_key,
                                galley: galley.clone(),
                            },
                        );
                    });
                }
                
                galley
            } else {
                // No syntax highlighting - use simple layout
                let job = if word_wrap {
                    egui::text::LayoutJob::simple(
                        text.to_owned(),
                        font_id,
                        default_text_color,
                        wrap_width,
                    )
                } else {
                    egui::text::LayoutJob::simple_singleline(
                        text.to_owned(),
                        font_id,
                        default_text_color,
                    )
                };
                ui.fonts(|f| f.layout_job(job))
            }
        };

        // Calculate scroll offset for current match if needed
        let mut target_scroll_offset: Option<f32> = None;
        // Track target line for post-render verification (1-indexed)
        let mut scroll_target_line: Option<usize> = None;

        // Get fresh line height from UI fonts (not stale raw_line_height)
        let line_height = ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family.clone())));
        let viewport_height = ui.available_height();

        // Priority 1: Scroll to specific line (from outline navigation)
        if let Some(target_line) = self.scroll_to_line {
            // Use unified scroll calculation with tolerance
            target_scroll_offset = Some(calculate_scroll_for_line(
                target_line,
                line_height,
                viewport_height,
            ));
            scroll_target_line = Some(target_line);
            debug!(
                "Scrolling to line {} (line_height={:.1}, viewport={:.1}, offset={:.1})",
                target_line,
                line_height,
                viewport_height,
                target_scroll_offset.unwrap_or(0.0)
            );
        }
        // Priority 2: Scroll to search match
        else if let Some(ref highlights) = search_highlights {
            if highlights.scroll_to_match && !highlights.matches.is_empty() {
                if let Some(&(match_start, _)) = highlights.matches.get(highlights.current_match) {
                    // Calculate line number of the match (0-indexed from char_index_to_line_col)
                    let (match_line_0indexed, _) = char_index_to_line_col(content, match_start);
                    // Convert to 1-indexed for unified calculation
                    let match_line_1indexed = match_line_0indexed + 1;
                    target_scroll_offset = Some(calculate_scroll_for_line(
                        match_line_1indexed,
                        line_height,
                        viewport_height,
                    ));
                    scroll_target_line = Some(match_line_1indexed);
                }
            }
        }

        // Use ScrollArea for viewport management - line numbers scroll with content
        // IMPORTANT: Use base_id (not id with content_version) for ScrollArea to preserve
        // scroll position across undo/redo operations. Only TextEdit needs the content_version
        // in its ID to force re-reading content after external changes.
        let mut scroll_area = ScrollArea::vertical()
            .id_source(base_id.with("scroll"))
            .auto_shrink([false, false]);

        // Priority: Apply pending scroll offset from mode switch first
        if let Some(offset) = self.tab.pending_scroll_offset.take() {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
            debug!("Applied pending scroll offset: {}", offset);
        }
        // Otherwise, apply scroll offset if we need to jump to a match or line
        else if let Some(offset) = target_scroll_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        // Calculate content width and centering margin
        // Both Zen mode and non-zen mode use max_line_width setting
        // Zen mode: centers content; Non-zen mode: left-aligned
        let char_width = font_size * 0.6; // Approximate average character width
        let outer_available_width = ui.available_width();
        
        let (content_margin, effective_content_width) = if let Some(max_width_px) = self.max_line_width.to_pixels(char_width) {
            // max_line_width is set - constrain width
            // Cap to available width to prevent overflow
            let effective_width = max_width_px.min(outer_available_width);
            
            if self.zen_mode {
                // Zen mode: center the content
                let margin = if outer_available_width > effective_width {
                    (outer_available_width - effective_width) / 2.0
                } else {
                    0.0
                };
                (margin, Some(effective_width))
            } else {
                // Non-zen mode: left-aligned (no margin)
                (0.0, Some(effective_width))
            }
        } else {
            // No max_line_width set - use full available width, no centering
            (0.0, None)
        };
        
        // For backward compatibility, keep zen_margin variable name
        let zen_margin = content_margin;
        
        let scroll_output = scroll_area.show(ui, |ui| {
            // Use horizontal layout inside ScrollArea so gutter and editor scroll together
            ui.horizontal_top(|ui| {
                // Add left margin for Zen Mode centering
                if zen_margin > 0.0 {
                    ui.add_space(zen_margin);
                }
                
                // Reserve space for the gutter (will be drawn after we know text positions)
                // Create gutter if we have line numbers OR fold indicators
                let gutter_rect = if show_line_numbers || (show_fold_indicators && has_folds) {
                    let line_count = super::line_numbers::count_lines(content);
                    let line_height =
                        ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family.clone())));
                    let total_height = line_count as f32 * line_height;

                    // Use Sense::click() so we can detect fold indicator clicks
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(gutter_width, total_height.max(ui.available_height())),
                        egui::Sense::click(),
                    );
                    Some((rect, response))
                } else {
                    None
                };

                // Create the multiline text editor
                // Constrain width when Zen Mode is enabled or max_line_width is set
                // Use pre-calculated effective width (already capped to available space)
                let desired_width = effective_content_width.unwrap_or(f32::INFINITY);
                
                let text_edit = TextEdit::multiline(content)
                    .id(id)
                    .frame(self.frame)
                    .font(FontId::new(font_size, font_family.clone()))
                    .desired_width(desired_width)
                    .lock_focus(true) // Prevent Tab from losing focus; Tab inserts indent instead
                    .layouter(&mut layouter);

                // Show the editor and get the output
                let text_output = text_edit.show(ui);

                // Request focus if this is a new tab that needs it
                if needs_focus {
                    text_output.response.request_focus();
                }

                // Draw search match highlights
                if let Some(ref highlights) = search_highlights {
                    if !highlights.matches.is_empty() {
                        let galley = &text_output.galley;
                        let galley_pos = text_output.galley_pos;
                        let painter = ui.painter();
                        let is_dark = theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);

                        // Highlight colors
                        let current_match_color = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 200, 0, 150)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 220, 0, 180)
                        };
                        let other_match_color = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(180, 150, 50, 80)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 100, 120)
                        };

                        for (idx, &(match_start, match_end)) in
                            highlights.matches.iter().enumerate()
                        {
                            let is_current = idx == highlights.current_match;
                            let color = if is_current {
                                current_match_color
                            } else {
                                other_match_color
                            };

                            // Get rectangles for this text range from the galley
                            // Convert byte positions to character positions for galley
                            let char_start = byte_to_char_pos(content, match_start);
                            let char_end = byte_to_char_pos(content, match_end);
                            let cursor_start = egui::text::CCursor::new(char_start);
                            let cursor_end = egui::text::CCursor::new(char_end);

                            // Get the row and position for start and end
                            let start_cursor = galley.from_ccursor(cursor_start);
                            let end_cursor = galley.from_ccursor(cursor_end);
                            let start_rcursor = start_cursor.rcursor;
                            let end_rcursor = end_cursor.rcursor;

                            // Handle single-row or multi-row highlights
                            if start_rcursor.row == end_rcursor.row {
                                // Single row - draw one rectangle
                                if let Some(row) = galley.rows.get(start_rcursor.row) {
                                    let row_rect = row.rect;
                                    let x_start = row.x_offset(start_rcursor.column);
                                    let x_end = row.x_offset(end_rcursor.column);

                                    let highlight_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            galley_pos.x + x_start,
                                            galley_pos.y + row_rect.min.y,
                                        ),
                                        egui::pos2(
                                            galley_pos.x + x_end,
                                            galley_pos.y + row_rect.max.y,
                                        ),
                                    );
                                    painter.rect_filled(highlight_rect, 2.0, color);
                                }
                            } else {
                                // Multi-row highlight
                                for row_idx in start_rcursor.row..=end_rcursor.row {
                                    if let Some(row) = galley.rows.get(row_idx) {
                                        let row_rect = row.rect;

                                        let x_start = if row_idx == start_rcursor.row {
                                            row.x_offset(start_rcursor.column)
                                        } else {
                                            0.0
                                        };

                                        let x_end = if row_idx == end_rcursor.row {
                                            row.x_offset(end_rcursor.column)
                                        } else {
                                            row_rect.width()
                                        };

                                        let highlight_rect = egui::Rect::from_min_max(
                                            egui::pos2(
                                                galley_pos.x + x_start,
                                                galley_pos.y + row_rect.min.y,
                                            ),
                                            egui::pos2(
                                                galley_pos.x + x_end,
                                                galley_pos.y + row_rect.max.y,
                                            ),
                                        );
                                        painter.rect_filled(highlight_rect, 2.0, color);
                                    }
                                }
                            }
                        }
                    }
                }

                // Draw transient highlight for search result navigation
                if let Some((hl_start, hl_end)) = transient_highlight {
                    let galley = &text_output.galley;
                    let galley_pos = text_output.galley_pos;
                    let painter = ui.painter();
                    let is_dark = theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);

                    // Distinct color for transient highlight - soft orange/amber
                    let highlight_color = if is_dark {
                        egui::Color32::from_rgba_unmultiplied(255, 165, 50, 120) // Amber
                    } else {
                        egui::Color32::from_rgba_unmultiplied(255, 180, 80, 150) // Light orange
                    };

                    // Get rectangles for the highlight range from the galley
                    // Convert byte positions to character positions for galley
                    let char_start = byte_to_char_pos(content, hl_start);
                    let char_end = byte_to_char_pos(content, hl_end);
                    let cursor_start = egui::text::CCursor::new(char_start);
                    let cursor_end = egui::text::CCursor::new(char_end);

                    let start_cursor = galley.from_ccursor(cursor_start);
                    let end_cursor = galley.from_ccursor(cursor_end);
                    let start_rcursor = start_cursor.rcursor;
                    let end_rcursor = end_cursor.rcursor;

                    // Handle single-row or multi-row highlights
                    if start_rcursor.row == end_rcursor.row {
                        // Single row - draw one rectangle
                        if let Some(row) = galley.rows.get(start_rcursor.row) {
                            let row_rect = row.rect;
                            let x_start = row.x_offset(start_rcursor.column);
                            let x_end = row.x_offset(end_rcursor.column);

                            let highlight_rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    galley_pos.x + x_start,
                                    galley_pos.y + row_rect.min.y,
                                ),
                                egui::pos2(
                                    galley_pos.x + x_end,
                                    galley_pos.y + row_rect.max.y,
                                ),
                            );
                            painter.rect_filled(highlight_rect, 2.0, highlight_color);
                        }
                    } else {
                        // Multi-row highlight
                        for row_idx in start_rcursor.row..=end_rcursor.row {
                            if let Some(row) = galley.rows.get(row_idx) {
                                let row_rect = row.rect;

                                let x_start = if row_idx == start_rcursor.row {
                                    row.x_offset(start_rcursor.column)
                                } else {
                                    0.0
                                };

                                let x_end = if row_idx == end_rcursor.row {
                                    row.x_offset(end_rcursor.column)
                                } else {
                                    row_rect.width()
                                };

                                let highlight_rect = egui::Rect::from_min_max(
                                    egui::pos2(
                                        galley_pos.x + x_start,
                                        galley_pos.y + row_rect.min.y,
                                    ),
                                    egui::pos2(
                                        galley_pos.x + x_end,
                                        galley_pos.y + row_rect.max.y,
                                    ),
                                );
                                painter.rect_filled(highlight_rect, 2.0, highlight_color);
                            }
                        }
                    }
                }

                // Draw bracket/emphasis matching highlights
                if highlight_matching_pairs {
                    // Get the primary cursor position (in character units)
                    let primary_cursor_pos = self.tab.cursors.primary().head;
                    // Convert to byte position for the delimiter matcher
                    let cursor_byte_pos = char_to_byte_pos(content, primary_cursor_pos);
                    
                    // Find matching delimiter pair
                    let matcher = DelimiterMatcher::new(content);
                    if let Some(matching_pair) = matcher.find_match(cursor_byte_pos) {
                        let galley = &text_output.galley;
                        let galley_pos = text_output.galley_pos;
                        let painter = ui.painter();
                        
                        // Get theme-aware colors for bracket matching
                        let (bg_color, border_color) = theme_colors
                            .as_ref()
                            .map(|c| (c.ui.matching_bracket_bg, c.ui.matching_bracket_border))
                            .unwrap_or_else(|| {
                                // Fallback colors if no theme is set
                                let is_dark = ui.visuals().dark_mode;
                                if is_dark {
                                    (
                                        egui::Color32::from_rgba_unmultiplied(80, 180, 220, 60),
                                        egui::Color32::from_rgb(100, 180, 220),
                                    )
                                } else {
                                    (
                                        egui::Color32::from_rgba_unmultiplied(255, 220, 100, 80),
                                        egui::Color32::from_rgb(200, 170, 50),
                                    )
                                }
                            });
                        
                        // Draw highlights for both source and target delimiters
                        for token in [&matching_pair.source, &matching_pair.target] {
                            // Convert byte positions to character positions for galley
                            let char_start = byte_to_char_pos(content, token.start);
                            let char_end = byte_to_char_pos(content, token.end);
                            
                            let cursor_start = egui::text::CCursor::new(char_start);
                            let cursor_end = egui::text::CCursor::new(char_end);
                            
                            let start_cursor = galley.from_ccursor(cursor_start);
                            let end_cursor = galley.from_ccursor(cursor_end);
                            let start_rcursor = start_cursor.rcursor;
                            let end_rcursor = end_cursor.rcursor;
                            
                            // Single row highlight (brackets are typically on one row)
                            if start_rcursor.row == end_rcursor.row {
                                if let Some(row) = galley.rows.get(start_rcursor.row) {
                                    let row_rect = row.rect;
                                    let x_start = row.x_offset(start_rcursor.column);
                                    let x_end = row.x_offset(end_rcursor.column);
                                    
                                    let highlight_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            galley_pos.x + x_start,
                                            galley_pos.y + row_rect.min.y,
                                        ),
                                        egui::pos2(
                                            galley_pos.x + x_end,
                                            galley_pos.y + row_rect.max.y,
                                        ),
                                    );
                                    
                                    // Draw background fill
                                    painter.rect_filled(highlight_rect, 2.0, bg_color);
                                    // Draw border for better visibility
                                    painter.rect_stroke(
                                        highlight_rect,
                                        2.0,
                                        egui::Stroke::new(1.0, border_color),
                                    );
                                }
                            }
                        }
                    }
                }

                // Draw multi-cursor highlights and carets (for additional cursors beyond primary)
                if !self.tab.cursors.is_single() {
                    let galley = &text_output.galley;
                    let galley_pos = text_output.galley_pos;
                    let painter = ui.painter();
                    let is_dark = theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);
                    let primary_idx = self.tab.cursors.primary_index();

                    // Multi-cursor colors
                    let cursor_color = if is_dark {
                        egui::Color32::from_rgba_unmultiplied(100, 180, 255, 255) // Light blue cursor
                    } else {
                        egui::Color32::from_rgba_unmultiplied(0, 100, 200, 255) // Dark blue cursor
                    };
                    let selection_color = if is_dark {
                        egui::Color32::from_rgba_unmultiplied(100, 180, 255, 60) // Light blue selection
                    } else {
                        egui::Color32::from_rgba_unmultiplied(0, 100, 200, 50) // Dark blue selection
                    };

                    for (idx, sel) in self.tab.cursors.selections().iter().enumerate() {
                        // Skip primary cursor (it's handled by egui's TextEdit)
                        if idx == primary_idx {
                            continue;
                        }

                        // Draw selection highlight if there's a selection
                        if sel.is_selection() {
                            let (start, end) = sel.range();
                            draw_selection_highlight(
                                painter,
                                galley,
                                galley_pos,
                                start,
                                end,
                                selection_color,
                            );
                        }

                        // Draw cursor caret
                        draw_cursor_caret(painter, galley, galley_pos, sel.head, cursor_color);
                    }
                }

                // Track fold toggle clicks
                let mut fold_toggle_line: Option<usize> = None;

                // Now draw line numbers and fold indicators using the actual galley positions
                if let Some((gutter_rect, gutter_response)) = gutter_rect {
                    let galley = &text_output.galley;
                    let galley_pos = text_output.galley_pos;

                    // Get colors for styling
                    let is_dark = theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);
                    let line_color = theme_colors
                        .as_ref()
                        .map(|c| c.text.muted)
                        .unwrap_or(egui::Color32::from_rgb(120, 120, 120));
                    let bg_color = theme_colors
                        .as_ref()
                        .map(|c| c.base.background_secondary)
                        .unwrap_or(egui::Color32::from_rgb(245, 245, 245));
                    let border_color = theme_colors
                        .as_ref()
                        .map(|c| c.base.border_subtle)
                        .unwrap_or(egui::Color32::from_rgb(200, 200, 200));
                    
                    // Fold indicator colors
                    let fold_color = if is_dark {
                        egui::Color32::from_rgb(140, 140, 140)
                    } else {
                        egui::Color32::from_rgb(100, 100, 100)
                    };
                    let fold_hover_color = if is_dark {
                        egui::Color32::from_rgb(180, 180, 180)
                    } else {
                        egui::Color32::from_rgb(60, 60, 60)
                    };

                    let painter = ui.painter();

                    // Draw gutter background
                    painter.rect_filled(gutter_rect, 0.0, bg_color);

                    // Draw separator line
                    painter.line_segment(
                        [
                            gutter_rect.right_top() + egui::vec2(-1.0, 0.0),
                            gutter_rect.right_bottom() + egui::vec2(-1.0, 0.0),
                        ],
                        egui::Stroke::new(1.0, border_color),
                    );

                    // Draw line numbers aligned with actual galley rows
                    // Always use monospace font for line numbers for proper alignment
                    let line_number_font_id = FontId::monospace(font_size);
                    let line_height = ui.fonts(|f| f.row_height(&line_number_font_id));

                    // Build a map of logical_line -> (row_y, row_height) for fold indicators
                    let mut line_y_map: std::collections::HashMap<usize, (f32, f32)> = std::collections::HashMap::new();

                    // Track logical line number
                    // With word wrap, multiple rows can belong to the same logical line
                    // A row ends a logical line when ends_with_newline is true
                    let mut logical_line = 0usize;
                    let mut line_number_drawn_for_line = false;

                    for row in galley.rows.iter() {
                        // Get the absolute Y position of this row (screen coordinates)
                        let row_y = galley_pos.y + row.min_y();
                        let row_height = row.rect.height();

                        // Draw line number only once per logical line (at the first row of a wrapped line)
                        if !line_number_drawn_for_line {
                            let display_num = logical_line + 1; // 1-indexed

                            // Record Y position for fold indicators
                            line_y_map.insert(logical_line, (row_y, row_height));

                            // Position line number at EXACT same Y as the text row
                            // Use absolute row_y to ensure perfect alignment regardless of
                            // any offset between gutter_rect and galley_pos
                            if show_line_numbers {
                                // Offset line numbers to make room for fold indicators
                                let line_num_x = if show_fold_indicators && has_folds {
                                    gutter_rect.right() - 12.0 // Right padding after fold indicator space
                                } else {
                                    gutter_rect.right() - 12.0 // Right padding
                                };
                                
                                let text_pos = egui::pos2(line_num_x, row_y);

                                painter.text(
                                    text_pos,
                                    egui::Align2::RIGHT_TOP,
                                    format!("{}", display_num),
                                    line_number_font_id.clone(),
                                    line_color,
                                );
                            }

                            line_number_drawn_for_line = true;
                        }

                        // Check if this row ends a logical line (has newline at the end)
                        if row.ends_with_newline {
                            logical_line += 1;
                            line_number_drawn_for_line = false;
                        }
                    }

                    // Handle empty content (no rows in galley)
                    if galley.rows.is_empty() && show_line_numbers {
                        let text_pos = egui::pos2(
                            gutter_rect.right() - 12.0,
                            galley_pos.y, // Use galley position for empty content
                        );
                        painter.text(
                            text_pos,
                            egui::Align2::RIGHT_TOP,
                            "1",
                            line_number_font_id.clone(),
                            line_color,
                        );
                        line_y_map.insert(0, (galley_pos.y, line_height));
                    }

                    // Draw fold indicators
                    if show_fold_indicators && has_folds {
                        let fold_x = gutter_rect.left() + 4.0; // Left padding for fold indicators
                        let indicator_size = (font_size * 0.6).min(12.0);
                        
                        // Check for hover position
                        let hover_pos = ui.input(|i| i.pointer.hover_pos());
                        let click_pos = if gutter_response.clicked() {
                            ui.input(|i| i.pointer.interact_pos())
                        } else {
                            None
                        };

                        for (line, is_collapsed) in &fold_indicators {
                            if let Some(&(row_y, row_height)) = line_y_map.get(line) {
                                // Calculate indicator position centered vertically in the row
                                let indicator_y = row_y + (row_height - indicator_size) / 2.0;
                                let indicator_rect = egui::Rect::from_min_size(
                                    egui::pos2(fold_x, indicator_y),
                                    egui::vec2(indicator_size, indicator_size),
                                );

                                // Check if mouse is hovering over this indicator
                                let is_hovered = hover_pos
                                    .map(|pos| indicator_rect.expand(2.0).contains(pos))
                                    .unwrap_or(false);

                                // Check if this indicator was clicked
                                if let Some(click) = click_pos {
                                    if indicator_rect.expand(4.0).contains(click) {
                                        fold_toggle_line = Some(*line);
                                    }
                                }

                                let color = if is_hovered { fold_hover_color } else { fold_color };

                                // Draw the fold indicator (triangle)
                                if *is_collapsed {
                                    // Collapsed: right-pointing triangle ▶ with highlight color
                                    let collapsed_color = egui::Color32::from_rgb(255, 165, 0); // Orange for collapsed
                                    let center = indicator_rect.center();
                                    let half = indicator_size / 2.0 * 0.8;
                                    let points = vec![
                                        egui::pos2(center.x - half * 0.4, center.y - half),
                                        egui::pos2(center.x + half * 0.8, center.y),
                                        egui::pos2(center.x - half * 0.4, center.y + half),
                                    ];
                                    painter.add(egui::Shape::convex_polygon(
                                        points,
                                        if is_hovered { fold_hover_color } else { collapsed_color },
                                        egui::Stroke::NONE,
                                    ));
                                } else {
                                    // Expanded: down-pointing triangle ▼
                                    let center = indicator_rect.center();
                                    let half = indicator_size / 2.0 * 0.8;
                                    let points = vec![
                                        egui::pos2(center.x - half, center.y - half * 0.4),
                                        egui::pos2(center.x + half, center.y - half * 0.4),
                                        egui::pos2(center.x, center.y + half * 0.8),
                                    ];
                                    painter.add(egui::Shape::convex_polygon(
                                        points,
                                        color,
                                        egui::Stroke::NONE,
                                    ));
                                }

                                // Show cursor hint on hover
                                if is_hovered {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            }
                        }
                    }
                }

                (text_output, fold_toggle_line)
            })
            .inner
        });

        let (mut text_output, fold_toggle_line) = scroll_output.inner;
        
        // Restore cursor position after undo/redo if pending
        // This must happen after TextEdit is shown but before we use cursor_range
        if let Some(cursor_pos) = pending_cursor {
            // Create a cursor at the specified position
            let ccursor = egui::text::CCursor::new(cursor_pos);
            let cursor_range = egui::text::CCursorRange::one(ccursor);
            
            // Set the cursor in the TextEditState and persist it
            text_output.state.cursor.set_char_range(Some(cursor_range));
            text_output.state.store(ui.ctx(), id);
            
            // Also update our internal cursor tracking
            self.tab.cursors.set_single(crate::state::Selection::cursor(cursor_pos));
            self.tab.sync_cursor_from_primary();
        }
        
        let cursor_range_opt = text_output.cursor_range;

        // Determine if content changed
        let changed = self.tab.content != original_content;

        // If content changed, record for undo tracking and auto-save
        if changed {
            // TextEdit modifies content directly, so we need to manually
            // record the edit for undo/redo functionality (with old cursor position)
            self.tab.record_edit(original_content, original_cursor);
            // Mark content as edited for auto-save scheduling
            self.tab.mark_content_edited();
            debug!("Editor content changed, recorded for undo");
        }

        // Detect Ctrl+Click for multi-cursor support
        let ctrl_click_pos = ui.input(|i| {
            // Check if Ctrl is held and there was a primary click
            if i.modifiers.ctrl && i.pointer.primary_clicked() {
                // Get the new cursor position from the cursor range
                cursor_range_opt.map(|cr| cr.primary.ccursor.index)
            } else {
                None
            }
        });

        // Update cursor state from egui's TextEdit cursor range
        if let Some(cursor_range) = cursor_range_opt {
            let primary = cursor_range.primary.ccursor.index;
            let secondary = cursor_range.secondary.ccursor.index;

            // For Ctrl+Click, don't update via egui - we'll handle it separately
            if ctrl_click_pos.is_none() {
                // Update multi-cursor state (also syncs legacy cursor_position and selection)
                self.tab.update_cursor_from_egui(primary, secondary);
            }
        }

        // Update scroll metrics from ScrollArea state
        self.tab.scroll_offset = scroll_output.state.offset.y;
        self.tab.content_height = scroll_output.content_size.y;
        self.tab.viewport_height = scroll_output.inner_rect.height();

        // Update line height for accurate scroll sync
        self.tab.raw_line_height = ui.fonts(|f| f.row_height(&FontId::new(font_size, font_family)));

        // POST-RENDER SCROLL VERIFICATION:
        // If we just scrolled to a line (from outline, search, or find navigation),
        // verify the target is actually visible using the galley's actual row positions.
        // This accounts for word wrap where a single logical line spans multiple visual rows.
        // This is a second-pass correction for the initial estimate.
        if let Some(target_line) = scroll_target_line {
            let galley = &text_output.galley;
            let viewport_height = scroll_output.inner_rect.height();
            let current_scroll = scroll_output.state.offset.y;
            
            // Find the actual Y position of the target line in the galley
            if let Some(actual_y) = find_line_y_in_galley(galley, target_line) {
                // Calculate where the line should be (1/4 from top of viewport)
                let desired_top_margin = viewport_height * 0.25;
                let ideal_scroll = (actual_y - desired_top_margin).max(0.0);
                
                // Clamp to valid scroll range
                let max_scroll = (scroll_output.content_size.y - viewport_height).max(0.0);
                let ideal_scroll = ideal_scroll.min(max_scroll);
                
                // Check if the line is visible and reasonably positioned
                let line_top = actual_y - current_scroll;
                let visible_top = 0.0;
                let visible_bottom = viewport_height;
                
                // If line is outside viewport OR significantly off from ideal position
                let is_out_of_view = line_top < visible_top || line_top > visible_bottom - 20.0;
                let is_significantly_off = (current_scroll - ideal_scroll).abs() > 30.0;
                
                if is_out_of_view || is_significantly_off {
                    debug!(
                        "Scroll correction: line={}, actual_y={:.1}, current={:.1}, ideal={:.1}, diff={:.1}",
                        target_line, actual_y, current_scroll, ideal_scroll, (current_scroll - ideal_scroll).abs()
                    );
                    self.tab.pending_scroll_offset = Some(ideal_scroll);
                    ui.ctx().request_repaint();
                }
            }
        }

        // Handle pending scroll ratio: convert to offset now that we have content_height
        if let Some(ratio) = self.tab.pending_scroll_ratio.take() {
            let max_scroll = (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);
            if max_scroll > 0.0 {
                let target_offset = ratio * max_scroll;
                self.tab.pending_scroll_offset = Some(target_offset);
                debug!(
                    "Converted scroll ratio {:.3} to offset {:.1} in raw editor",
                    ratio, target_offset
                );
                // Request repaint to apply the offset on next frame
                ui.ctx().request_repaint();
            }
        }

        EditorOutput {
            changed,
            ctrl_click_pos,
            fold_toggle_line,
        }
    }
}

/// Convert a byte position to a character position.
///
/// This is needed because the galley uses character indices while
/// the delimiter matcher uses byte indices.
fn byte_to_char_pos(text: &str, byte_pos: usize) -> usize {
    text[..byte_pos.min(text.len())].chars().count()
}

/// Convert a character position to a byte position.
///
/// This is needed because cursor positions are in character units while
/// the delimiter matcher uses byte indices.
fn char_to_byte_pos(text: &str, char_pos: usize) -> usize {
    text.char_indices()
        .nth(char_pos)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

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

/// Calculate scroll offset to position a target line in the viewport.
///
/// This is the unified scroll calculation function used by all navigation features
/// (outline, search, find, etc.) to ensure consistent positioning.
///
/// # Arguments
/// * `target_line` - 1-indexed line number to scroll to
/// * `line_height` - Height of a single line in pixels (from ui.fonts())
/// * `viewport_height` - Height of the visible viewport in pixels
///
/// # Returns
/// Scroll offset in pixels that positions the target line at approximately
/// 1/4 from the top of the viewport (with tolerance for visibility).
///
/// # Note
/// Uses 1/4 from top instead of 1/3 to provide better visibility buffer,
/// especially when word wrap causes lines to take multiple visual rows.
/// This is an initial estimate; post-render verification using actual galley
/// positions provides pixel-perfect accuracy.
fn calculate_scroll_for_line(target_line: usize, line_height: f32, viewport_height: f32) -> f32 {
    // Convert 1-indexed to 0-indexed for calculation
    let line_index = target_line.saturating_sub(1);
    
    // Calculate target Y position
    let target_y = line_index as f32 * line_height;
    
    // Position target at 1/4 from top of viewport (better visibility than 1/3)
    // This provides more tolerance for word-wrapped lines and ensures
    // the target is clearly visible even if line height estimation is slightly off
    let offset = target_y - (viewport_height * 0.25);
    
    // Ensure we don't scroll past the start
    offset.max(0.0)
}

/// Find the actual Y position of a logical line in the galley.
///
/// This function iterates through the galley's visual rows to find the first
/// row belonging to the target logical line. This accounts for word wrap
/// where a single logical line can span multiple visual rows.
///
/// # Arguments
/// * `galley` - The text galley containing visual row information
/// * `target_line` - 1-indexed line number to find
///
/// # Returns
/// The Y position (in galley coordinates) of the first row of the target line,
/// or None if the line doesn't exist.
fn find_line_y_in_galley(galley: &egui::Galley, target_line: usize) -> Option<f32> {
    if target_line == 0 {
        return None;
    }
    
    // Target line in 0-indexed form
    let target_line_0indexed = target_line - 1;
    
    // Track logical line number as we iterate through visual rows
    let mut current_logical_line = 0usize;
    
    for row in galley.rows.iter() {
        // Check if we've reached our target line
        if current_logical_line == target_line_0indexed {
            return Some(row.min_y());
        }
        
        // A row ends a logical line when it has a newline at the end
        // (word-wrapped continuations don't have ends_with_newline)
        if row.ends_with_newline {
            current_logical_line += 1;
        }
    }
    
    // If target is beyond the last line, return the position of the last row
    // This handles edge cases where we're scrolling to a line near the end
    if target_line_0indexed >= current_logical_line {
        galley.rows.last().map(|row| row.min_y())
    } else {
        None
    }
}

/// Convert (line, column) position to a character index.
///
/// Both line and column are 0-indexed.
/// Returns the closest valid index if position is out of bounds.
#[allow(dead_code)]
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
// Multi-Cursor Rendering Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Draw a selection highlight for a multi-cursor selection.
fn draw_selection_highlight(
    painter: &egui::Painter,
    galley: &Arc<egui::Galley>,
    galley_pos: egui::Pos2,
    start: usize,
    end: usize,
    color: egui::Color32,
) {
    let cursor_start = egui::text::CCursor::new(start);
    let cursor_end = egui::text::CCursor::new(end);

    let start_cursor = galley.from_ccursor(cursor_start);
    let end_cursor = galley.from_ccursor(cursor_end);
    let start_rcursor = start_cursor.rcursor;
    let end_rcursor = end_cursor.rcursor;

    if start_rcursor.row == end_rcursor.row {
        // Single row selection
        if let Some(row) = galley.rows.get(start_rcursor.row) {
            let row_rect = row.rect;
            let x_start = row.x_offset(start_rcursor.column);
            let x_end = row.x_offset(end_rcursor.column);

            let highlight_rect = egui::Rect::from_min_max(
                egui::pos2(galley_pos.x + x_start, galley_pos.y + row_rect.min.y),
                egui::pos2(galley_pos.x + x_end, galley_pos.y + row_rect.max.y),
            );
            painter.rect_filled(highlight_rect, 2.0, color);
        }
    } else {
        // Multi-row selection
        for row_idx in start_rcursor.row..=end_rcursor.row {
            if let Some(row) = galley.rows.get(row_idx) {
                let row_rect = row.rect;

                let x_start = if row_idx == start_rcursor.row {
                    row.x_offset(start_rcursor.column)
                } else {
                    0.0
                };

                let x_end = if row_idx == end_rcursor.row {
                    row.x_offset(end_rcursor.column)
                } else {
                    row_rect.width()
                };

                let highlight_rect = egui::Rect::from_min_max(
                    egui::pos2(galley_pos.x + x_start, galley_pos.y + row_rect.min.y),
                    egui::pos2(galley_pos.x + x_end, galley_pos.y + row_rect.max.y),
                );
                painter.rect_filled(highlight_rect, 2.0, color);
            }
        }
    }
}

/// Draw a cursor caret at the given position.
fn draw_cursor_caret(
    painter: &egui::Painter,
    galley: &Arc<egui::Galley>,
    galley_pos: egui::Pos2,
    pos: usize,
    color: egui::Color32,
) {
    let cursor = galley.from_ccursor(egui::text::CCursor::new(pos));
    let rcursor = cursor.rcursor;

    if let Some(row) = galley.rows.get(rcursor.row) {
        let row_rect = row.rect;
        let x = row.x_offset(rcursor.column);

        // Draw a thin vertical line for the cursor
        let caret_rect = egui::Rect::from_min_max(
            egui::pos2(galley_pos.x + x - 1.0, galley_pos.y + row_rect.min.y),
            egui::pos2(galley_pos.x + x + 1.0, galley_pos.y + row_rect.max.y),
        );
        painter.rect_filled(caret_rect, 0.5, color);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
