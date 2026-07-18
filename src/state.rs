//! Application state management for Ferrite
//!
//! This module defines the central `AppState` struct that manages all
//! application data and UI state, including the current file, open tabs,
//! settings, and editor state.

// Allow dead code - this module has many state management methods for future use
// - redundant_closure: Sometimes closure is clearer for method reference
#![allow(dead_code)]
#![allow(clippy::redundant_closure)]

use crate::config::{load_config, save_config_silent, Settings, TabInfo, ViewMode};
use crate::editor::{compute_edit_ops, EditHistory, TextStats};
use crate::lsp::{DiagnosticMap, LspManager};
use crate::ui::TabPipelineState;
use crate::vcs::GitService;
use crate::workspaces::{filter_events, AppMode, Workspace, WorkspaceEvent, WorkspaceWatcher};
use egui;
use log::{debug, info, warn};
use rust_i18n::t;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// File size threshold (bytes) above which a performance warning toast is shown on open.
/// Kept as a constant for now; can be moved to settings later.
const LARGE_FILE_THRESHOLD_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

// ─────────────────────────────────────────────────────────────────────────────
// Content Hashing Helper
// ─────────────────────────────────────────────────────────────────────────────

/// Simple hash function for content (for auto-save change detection)
fn hash_content(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// File Type Detection
// ─────────────────────────────────────────────────────────────────────────────

/// File types supported by the editor for adaptive UI.
///
/// The editor uses this enum to determine which toolbar buttons and
/// menu items to display based on the active tab's file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    /// Markdown files (.md, .markdown)
    #[default]
    Markdown,
    /// JSON files (.json)
    Json,
    /// YAML files (.yaml, .yml)
    Yaml,
    /// TOML files (.toml)
    Toml,
    /// CSV files (.csv)
    Csv,
    /// TSV files (.tsv)
    Tsv,
    /// Image files (.png, .jpg, .jpeg, .gif, .webp, .bmp)
    Image,
    /// PDF files (.pdf)
    Pdf,
    /// Unknown or unsupported file type
    Unknown,
}

impl FileType {
    /// Detect file type from a file path based on extension.
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Unknown)
    }

    /// Detect file type from file extension string.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "md" | "markdown" => Self::Markdown,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "csv" => Self::Csv,
            "tsv" => Self::Tsv,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => Self::Image,
            "pdf" => Self::Pdf,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a markdown file type.
    pub fn is_markdown(&self) -> bool {
        matches!(self, Self::Markdown)
    }

    /// Check if this is a structured data file (JSON, YAML, or TOML).
    pub fn is_structured(&self) -> bool {
        matches!(self, Self::Json | Self::Yaml | Self::Toml)
    }

    /// Check if this is a tabular data file (CSV or TSV).
    pub fn is_tabular(&self) -> bool {
        matches!(self, Self::Csv | Self::Tsv)
    }

    /// Check if this is an image file type.
    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image)
    }

    /// Check if this is a PDF file type.
    pub fn is_pdf(&self) -> bool {
        matches!(self, Self::Pdf)
    }

    /// Check if this file type supports split view (raw + rendered side-by-side).
    pub fn supports_split(&self) -> bool {
        self.is_markdown() || self.is_tabular()
    }

    /// Get a display name for this file type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Image => "Image",
            Self::Pdf => "PDF",
            Self::Unknown => "Unknown",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Binary File Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Detect if file content appears to be binary (non-text) data.
///
/// This function uses heuristics to detect binary content:
/// 1. Presence of null bytes (strong indicator of binary data)
/// 2. High ratio of non-printable/control characters
///
/// Returns `true` if the content appears to be binary data.
pub fn is_binary_content(bytes: &[u8]) -> bool {
    // Empty files are not binary
    if bytes.is_empty() {
        return false;
    }

    // Check for null bytes - strong indicator of binary data
    if bytes.contains(&0) {
        return true;
    }

    // Sample at most 8KB for large files
    let sample_size = bytes.len().min(8192);
    let sample = &bytes[..sample_size];

    // Count non-printable characters (excluding common whitespace)
    let non_printable_count = sample
        .iter()
        .filter(|&&b| {
            // Control characters other than common whitespace
            b < 0x20 && b != 0x09 && b != 0x0A && b != 0x0D
        })
        .count();

    // If more than 10% of sampled bytes are non-printable control chars, treat as binary
    let threshold = sample_size / 10;
    non_printable_count > threshold
}

/// Get a human-readable description of why content was detected as binary.
fn binary_detection_reason(bytes: &[u8]) -> &'static str {
    if bytes.contains(&0) {
        "contains null bytes"
    } else {
        "has too many non-printable characters"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-Cursor Support
// ─────────────────────────────────────────────────────────────────────────────

/// A selection or cursor position in the editor.
///
/// A Selection represents either:
/// - A cursor with no selection (when `anchor == head`)
/// - A text selection (when `anchor != head`)
///
/// The anchor is the fixed end of the selection (where selection started),
/// and the head is the moving end (current cursor position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// The fixed end of the selection (where selection started), as a character index.
    pub anchor: usize,
    /// The moving end of the selection (current cursor position), as a character index.
    pub head: usize,
    /// Preferred visual column for vertical movement (preserved during up/down navigation).
    /// This is in visual columns, accounting for tabs and wide characters.
    pub preferred_column: Option<usize>,
}

impl Selection {
    /// Create a new cursor with no selection at the given character index.
    pub fn cursor(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
            preferred_column: None,
        }
    }

    /// Create a new selection from anchor to head.
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            preferred_column: None,
        }
    }

    /// Check if this is a cursor with no selection.
    pub fn is_cursor(&self) -> bool {
        self.anchor == self.head
    }

    /// Check if this is a selection (has a range).
    pub fn is_selection(&self) -> bool {
        self.anchor != self.head
    }

    /// Get the start of the selection (smaller index).
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// Get the end of the selection (larger index).
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Get the selection range as (start, end) tuple.
    pub fn range(&self) -> (usize, usize) {
        (self.start(), self.end())
    }

    /// Get the length of the selection.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Check if the selection is empty (cursor with no selection).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if this selection contains or overlaps with a position.
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start() && pos <= self.end()
    }

    /// Check if this selection overlaps with another selection.
    pub fn overlaps(&self, other: &Selection) -> bool {
        self.start() < other.end() && other.start() < self.end()
    }

    /// Merge this selection with another overlapping selection.
    pub fn merge(&self, other: &Selection) -> Selection {
        Selection {
            anchor: self.start().min(other.start()),
            head: self.end().max(other.end()),
            preferred_column: self.preferred_column.or(other.preferred_column),
        }
    }

    /// Move the cursor/selection by an offset.
    pub fn offset(self, delta: isize) -> Selection {
        let new_anchor = ((self.anchor as isize) + delta).max(0) as usize;
        let new_head = ((self.head as isize) + delta).max(0) as usize;
        Selection {
            anchor: new_anchor,
            head: new_head,
            preferred_column: self.preferred_column,
        }
    }

    /// Collapse the selection to a cursor at the head position.
    pub fn collapse_to_head(self) -> Selection {
        Selection::cursor(self.head)
    }

    /// Collapse the selection to a cursor at the start position.
    pub fn collapse_to_start(self) -> Selection {
        Selection::cursor(self.start())
    }

    /// Collapse the selection to a cursor at the end position.
    pub fn collapse_to_end(self) -> Selection {
        Selection::cursor(self.end())
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::cursor(0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Transient Highlight (Search Result Navigation)
// ─────────────────────────────────────────────────────────────────────────────

/// A temporary highlight for search result navigation.
///
/// This highlight is applied when the user clicks a search-in-files result,
/// and is automatically cleared on scroll, edit, or any mouse click.
/// It is independent of text selection and multi-cursor state.
#[derive(Debug, Clone, Default)]
pub struct TransientHighlight {
    /// The character range to highlight (start, end).
    /// None if no highlight is active.
    range: Option<(usize, usize)>,
    /// Guard flag to ignore the programmatic scroll that positions the match.
    /// Set to true when the highlight is first applied, cleared after one scroll event.
    ignore_next_scroll: bool,
}

impl TransientHighlight {
    /// Create a new transient highlight (initially inactive).
    pub fn new() -> Self {
        Self {
            range: None,
            ignore_next_scroll: false,
        }
    }

    /// Set the highlight range.
    ///
    /// This also sets the guard flag to ignore the programmatic scroll.
    pub fn set(&mut self, start: usize, end: usize) {
        self.range = Some((start, end));
        self.ignore_next_scroll = true;
    }

    /// Clear the highlight.
    pub fn clear(&mut self) {
        self.range = None;
        self.ignore_next_scroll = false;
    }

    /// Check if a highlight is active.
    pub fn is_active(&self) -> bool {
        self.range.is_some()
    }

    /// Get the highlight range if active.
    pub fn range(&self) -> Option<(usize, usize)> {
        self.range
    }

    /// Handle a scroll event.
    ///
    /// If this is the first scroll after applying the highlight (the programmatic
    /// scroll to position the match), ignore it. Otherwise, clear the highlight.
    ///
    /// Returns true if the highlight was cleared.
    pub fn on_scroll(&mut self) -> bool {
        if self.range.is_none() {
            return false;
        }

        if self.ignore_next_scroll {
            self.ignore_next_scroll = false;
            return false;
        }

        self.clear();
        true
    }

    /// Handle an edit event. Clears the highlight.
    ///
    /// Returns true if the highlight was cleared.
    pub fn on_edit(&mut self) -> bool {
        if self.range.is_some() {
            self.clear();
            true
        } else {
            false
        }
    }

    /// Handle a mouse click event. Clears the highlight.
    ///
    /// Returns true if the highlight was cleared.
    pub fn on_click(&mut self) -> bool {
        if self.range.is_some() {
            self.clear();
            true
        } else {
            false
        }
    }
}

/// Multi-cursor state - a collection of selections/cursors.
///
/// Invariants:
/// - Always contains at least one selection
/// - Selections are sorted by start position
/// - Selections do not overlap (merged if they would)
#[derive(Debug, Clone, Default)]
pub struct MultiCursor {
    /// All active selections/cursors (sorted, non-overlapping).
    selections: Vec<Selection>,
    /// Index of the primary selection (for status bar display, scroll anchoring).
    primary_index: usize,
}

impl MultiCursor {
    /// Create a new multi-cursor with a single cursor at position 0.
    pub fn new() -> Self {
        Self {
            selections: vec![Selection::cursor(0)],
            primary_index: 0,
        }
    }

    /// Create a multi-cursor with a single cursor at the given position.
    pub fn single(pos: usize) -> Self {
        Self {
            selections: vec![Selection::cursor(pos)],
            primary_index: 0,
        }
    }

    /// Create a multi-cursor with a single selection.
    pub fn from_selection(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary_index: 0,
        }
    }

    /// Get all selections.
    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    /// Get the number of cursors/selections.
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Check if there's only a single cursor/selection.
    pub fn is_single(&self) -> bool {
        self.selections.len() == 1
    }

    /// Check if this is empty (should never be true due to invariants).
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Get the primary selection (for status bar, scroll anchoring).
    pub fn primary(&self) -> &Selection {
        self.selections
            .get(self.primary_index)
            .unwrap_or(&self.selections[0])
    }

    /// Get a mutable reference to the primary selection.
    pub fn primary_mut(&mut self) -> &mut Selection {
        let idx = self
            .primary_index
            .min(self.selections.len().saturating_sub(1));
        &mut self.selections[idx]
    }

    /// Get the primary index.
    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    /// Set the primary selection by index.
    pub fn set_primary(&mut self, index: usize) {
        if index < self.selections.len() {
            self.primary_index = index;
        }
    }

    /// Add a new selection, maintaining invariants.
    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.normalize();
    }

    /// Replace all selections with a single one.
    pub fn set_single(&mut self, selection: Selection) {
        self.selections.clear();
        self.selections.push(selection);
        self.primary_index = 0;
    }

    /// Clear to a single cursor at position 0.
    pub fn clear(&mut self) {
        self.selections.clear();
        self.selections.push(Selection::cursor(0));
        self.primary_index = 0;
    }

    /// Normalize selections: sort and merge overlapping.
    fn normalize(&mut self) {
        if self.selections.is_empty() {
            self.selections.push(Selection::cursor(0));
            self.primary_index = 0;
            return;
        }

        // Sort by start position
        self.selections.sort_by_key(|s| s.start());

        // Merge overlapping selections
        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for sel in self.selections.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.overlaps(&sel) || last.end() == sel.start() {
                    *last = last.merge(&sel);
                    continue;
                }
            }
            merged.push(sel);
        }

        self.selections = merged;

        // Ensure primary_index is valid
        if self.primary_index >= self.selections.len() {
            self.primary_index = self.selections.len().saturating_sub(1);
        }
    }

    /// Apply an offset adjustment to all selections after a given position.
    /// Used after insertions/deletions to keep cursor positions valid.
    pub fn adjust_after(&mut self, pos: usize, delta: isize) {
        for sel in &mut self.selections {
            if sel.anchor >= pos {
                sel.anchor = ((sel.anchor as isize) + delta).max(0) as usize;
            }
            if sel.head >= pos {
                sel.head = ((sel.head as isize) + delta).max(0) as usize;
            }
        }
        self.normalize();
    }

    /// Get legacy cursor position (line, column) from primary selection.
    /// Used for backwards compatibility with status bar, etc.
    pub fn cursor_position(&self, text: &str) -> (usize, usize) {
        let pos = self.primary().head;
        char_index_to_line_col(text, pos)
    }

    /// Get legacy selection range from primary selection.
    /// Returns None if primary is a cursor with no selection.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let primary = self.primary();
        if primary.is_selection() {
            Some(primary.range())
        } else {
            None
        }
    }

    /// Iterate over all selections.
    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }

    /// Iterate mutably over all selections.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Selection> {
        self.selections.iter_mut()
    }
}

/// Convert character index to (line, column) position.
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

/// Convert (line, column) position to character index.
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
// Code Folding
// ─────────────────────────────────────────────────────────────────────────────

/// The kind/type of a foldable region.
///
/// Different fold kinds have different detection rules and may be
/// toggled on/off independently in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldKind {
    /// Markdown heading (## Section) - folds until next heading of same/higher level
    Heading(u8), // level 1-6
    /// Fenced code block (```...```)
    CodeBlock,
    /// List hierarchy (nested list items)
    List,
    /// Indentation-based region (for JSON/YAML/structured files)
    Indentation,
}

impl FoldKind {
    /// Get a display name for this fold kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            FoldKind::Heading(_) => "Heading",
            FoldKind::CodeBlock => "Code Block",
            FoldKind::List => "List",
            FoldKind::Indentation => "Indentation",
        }
    }

    /// Get an icon for this fold kind.
    pub fn icon(&self) -> &'static str {
        match self {
            FoldKind::Heading(_) => "§",
            FoldKind::CodeBlock => "{ }",
            FoldKind::List => "•",
            FoldKind::Indentation => "⤵",
        }
    }
}

/// A unique identifier for a fold region.
pub type FoldId = u32;

/// A foldable region in a document.
///
/// Represents a contiguous range of lines that can be collapsed/expanded.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldRegion {
    /// Unique identifier for this fold region
    pub id: FoldId,
    /// Starting line (0-indexed, inclusive)
    pub start_line: usize,
    /// Ending line (0-indexed, inclusive)
    pub end_line: usize,
    /// The kind of fold region
    pub kind: FoldKind,
    /// Whether this region is currently collapsed
    pub collapsed: bool,
    /// Preview text to show when collapsed (e.g., first line content)
    pub preview_text: String,
}

impl FoldRegion {
    /// Create a new fold region.
    pub fn new(id: FoldId, start_line: usize, end_line: usize, kind: FoldKind) -> Self {
        Self {
            id,
            start_line,
            end_line,
            kind,
            collapsed: false,
            preview_text: String::new(),
        }
    }

    /// Create a new fold region with preview text.
    pub fn with_preview(
        id: FoldId,
        start_line: usize,
        end_line: usize,
        kind: FoldKind,
        preview: String,
    ) -> Self {
        Self {
            id,
            start_line,
            end_line,
            kind,
            collapsed: false,
            preview_text: preview,
        }
    }

    /// Get the number of lines in this fold region.
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Get the number of hidden lines when collapsed.
    pub fn hidden_line_count(&self) -> usize {
        if self.collapsed {
            self.end_line.saturating_sub(self.start_line)
        } else {
            0
        }
    }

    /// Check if a line is within this fold region.
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    /// Check if a line is hidden by this fold (collapsed and not the start line).
    pub fn hides_line(&self, line: usize) -> bool {
        self.collapsed && line > self.start_line && line <= self.end_line
    }

    /// Toggle the collapsed state.
    pub fn toggle(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Adjust line numbers after an edit.
    ///
    /// `edit_line` is where the edit occurred, `delta` is the number of lines added (positive)
    /// or removed (negative).
    ///
    /// Returns `true` if the region is still valid, `false` if it should be removed.
    pub fn adjust_for_edit(&mut self, edit_line: usize, delta: isize) -> bool {
        // If edit is after this region, no change needed
        if edit_line > self.end_line {
            return true;
        }

        // If edit is within the region, adjust end line
        if edit_line >= self.start_line && edit_line <= self.end_line {
            let new_end = (self.end_line as isize) + delta;
            if new_end < (self.start_line as isize) {
                // Region collapsed to invalid state
                return false;
            }
            self.end_line = new_end as usize;
            return true;
        }

        // Edit is before this region, shift both lines
        let new_start = (self.start_line as isize) + delta;
        let new_end = (self.end_line as isize) + delta;

        if new_start < 0 || new_end < new_start {
            return false;
        }

        self.start_line = new_start as usize;
        self.end_line = new_end as usize;
        true
    }
}

/// State manager for all fold regions in a document.
///
/// Maintains an ordered list of fold regions and provides efficient
/// queries for rendering and interaction.
#[derive(Debug, Clone, Default)]
pub struct FoldState {
    /// All fold regions, sorted by start_line
    regions: Vec<FoldRegion>,
    /// Counter for generating unique fold IDs
    next_id: FoldId,
    /// Whether fold state needs recomputation
    dirty: bool,
}

impl FoldState {
    /// Create a new empty fold state.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            next_id: 1,
            dirty: true,
        }
    }

    /// Get all fold regions.
    pub fn regions(&self) -> &[FoldRegion] {
        &self.regions
    }

    /// Get mutable access to all fold regions.
    pub fn regions_mut(&mut self) -> &mut Vec<FoldRegion> {
        &mut self.regions
    }

    /// Check if there are any fold regions.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Get the number of fold regions.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Check if fold state needs recomputation.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark fold state as needing recomputation.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark fold state as clean (just recomputed).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Generate a new unique fold ID.
    pub fn next_id(&mut self) -> FoldId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Clear all fold regions.
    pub fn clear(&mut self) {
        self.regions.clear();
        self.dirty = true;
    }

    /// Replace all fold regions with new ones.
    pub fn set_regions(&mut self, regions: Vec<FoldRegion>) {
        self.regions = regions;
        self.sort_regions();
        self.dirty = false;
    }

    /// Add a fold region, maintaining sort order.
    pub fn add_region(&mut self, region: FoldRegion) {
        self.regions.push(region);
        self.sort_regions();
    }

    /// Sort regions by start line.
    fn sort_regions(&mut self) {
        self.regions.sort_by_key(|r| r.start_line);
    }

    /// Get the fold region that starts on a given line.
    pub fn region_at_line(&self, line: usize) -> Option<&FoldRegion> {
        self.regions.iter().find(|r| r.start_line == line)
    }

    /// Get mutable access to the fold region that starts on a given line.
    pub fn region_at_line_mut(&mut self, line: usize) -> Option<&mut FoldRegion> {
        self.regions.iter_mut().find(|r| r.start_line == line)
    }

    /// Get the fold region by ID.
    pub fn region_by_id(&self, id: FoldId) -> Option<&FoldRegion> {
        self.regions.iter().find(|r| r.id == id)
    }

    /// Get mutable access to a fold region by ID.
    pub fn region_by_id_mut(&mut self, id: FoldId) -> Option<&mut FoldRegion> {
        self.regions.iter_mut().find(|r| r.id == id)
    }

    /// Toggle the fold state at a given line.
    ///
    /// Returns `true` if a fold was toggled.
    pub fn toggle_at_line(&mut self, line: usize) -> bool {
        if let Some(region) = self.region_at_line_mut(line) {
            region.toggle();
            true
        } else {
            false
        }
    }

    /// Check if a line is hidden by any collapsed fold.
    pub fn is_line_hidden(&self, line: usize) -> bool {
        self.regions.iter().any(|r| r.hides_line(line))
    }

    /// Get the fold region that hides a given line.
    pub fn fold_hiding_line(&self, line: usize) -> Option<&FoldRegion> {
        self.regions.iter().find(|r| r.hides_line(line))
    }

    /// Expand any fold that contains the given line (to reveal it).
    ///
    /// Returns `true` if any fold was expanded.
    pub fn reveal_line(&mut self, line: usize) -> bool {
        let mut revealed = false;
        for region in &mut self.regions {
            if region.hides_line(line) {
                region.collapsed = false;
                revealed = true;
            }
        }
        revealed
    }

    /// Fold all regions of a specific kind.
    pub fn fold_all_of_kind(&mut self, kind_matches: impl Fn(&FoldKind) -> bool) {
        for region in &mut self.regions {
            if kind_matches(&region.kind) {
                region.collapsed = true;
            }
        }
    }

    /// Unfold all regions.
    pub fn unfold_all(&mut self) {
        for region in &mut self.regions {
            region.collapsed = false;
        }
    }

    /// Fold all regions.
    pub fn fold_all(&mut self) {
        for region in &mut self.regions {
            region.collapsed = true;
        }
    }

    /// Get the total number of hidden lines.
    pub fn hidden_line_count(&self) -> usize {
        self.regions.iter().map(|r| r.hidden_line_count()).sum()
    }

    /// Get all lines that have fold indicators (start lines of regions).
    pub fn fold_indicator_lines(&self) -> Vec<(usize, bool)> {
        self.regions
            .iter()
            .map(|r| (r.start_line, r.collapsed))
            .collect()
    }

    /// Map a visual line (accounting for folds) to the actual document line.
    ///
    /// Visual lines skip over hidden (folded) content.
    pub fn visual_to_document_line(&self, visual_line: usize) -> usize {
        let mut doc_line = 0;
        let mut vis_line = 0;

        while vis_line < visual_line {
            if !self.is_line_hidden(doc_line) {
                vis_line += 1;
            }
            doc_line += 1;
        }

        // Skip any hidden lines at the target position
        while self.is_line_hidden(doc_line) {
            doc_line += 1;
        }

        doc_line
    }

    /// Map a document line to the visual line (accounting for folds).
    pub fn document_to_visual_line(&self, doc_line: usize) -> usize {
        let mut vis_line = 0;
        for line in 0..doc_line {
            if !self.is_line_hidden(line) {
                vis_line += 1;
            }
        }
        vis_line
    }

    /// Adjust all fold regions after a document edit.
    ///
    /// `edit_line` is where the edit occurred, `delta` is the number of lines
    /// added (positive) or removed (negative).
    pub fn adjust_for_edit(&mut self, edit_line: usize, delta: isize) {
        self.regions
            .retain_mut(|r| r.adjust_for_edit(edit_line, delta));
        self.dirty = true;
    }

    /// Get the number of collapsed folds.
    pub fn collapsed_count(&self) -> usize {
        self.regions.iter().filter(|r| r.collapsed).count()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tab State (Runtime)
// ─────────────────────────────────────────────────────────────────────────────

/// Threshold in bytes above which a file is considered "large" and gets memory optimizations.
/// Large files:
/// - Use hash-based modification detection instead of storing full original_content
/// - Clear original_bytes after initial load to save memory
pub const LARGE_FILE_THRESHOLD: usize = 1_000_000; // 1MB

/// Reduced max undo groups for large files (operations are tiny, but cap defensively).
const LARGE_FILE_MAX_UNDO_GROUPS: usize = 200;

// ─────────────────────────────────────────────────────────────────────────────
// Tab Kind (Document vs Special)
// ─────────────────────────────────────────────────────────────────────────────

/// Types of special (non-editable) tabs that display application UI.
///
/// Special tabs render their own content (settings, help, etc.) instead of a
/// document editor. They cannot be edited, have no view mode, and never prompt
/// to save. This is designed to be extensible for future panel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialTabKind {
    /// Application settings panel
    Settings,
    /// About/Help information panel
    About,
    /// Application welcome panel
    Welcome,
}

impl SpecialTabKind {
    /// Get the display title for this special tab kind.
    pub fn title(&self) -> &'static str {
        match self {
            SpecialTabKind::Settings => "Settings",
            SpecialTabKind::About => "About / Help",
            SpecialTabKind::Welcome => "Welcome",
        }
    }

    /// Get the icon for this special tab kind.
    pub fn icon(&self) -> &'static str {
        use crate::ui::phosphor_icons::{GEAR, INFO, SPARKLE};
        match self {
            SpecialTabKind::Settings => GEAR,
            SpecialTabKind::About => INFO,
            SpecialTabKind::Welcome => SPARKLE,
        }
    }
}

/// State for an image viewer tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageViewerState {
    /// Current zoom level (1.0 = original size, fit-to-window by default)
    pub zoom: f32,
    /// Image dimensions (width, height) — populated after first load
    pub dimensions: Option<(u32, u32)>,
    /// Image file size in bytes
    pub file_size: u64,
    /// Image format string (e.g., "PNG", "JPEG")
    pub format_label: String,
    /// Whether initial fit-to-window has been applied
    pub fitted: bool,
    /// Markdown tab that opened this image, used by the viewer's Back button.
    pub parent_tab_id: Option<usize>,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            dimensions: None,
            file_size: 0,
            format_label: String::new(),
            fitted: false,
            parent_tab_id: None,
        }
    }
}

/// State for a PDF viewer tab.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfViewerState {
    /// Current page index (0-based)
    pub current_page: usize,
    /// Total number of pages
    pub page_count: usize,
    /// Current zoom level (1.0 = fit-to-window)
    pub zoom: f32,
    /// Whether initial fit-to-window has been applied
    pub fitted: bool,
    /// File size in bytes
    pub file_size: u64,
    /// Error message if PDF failed to load
    pub error: Option<String>,
    /// Overrides the tab label (used for print preview).
    pub display_title: Option<String>,
    /// When true, the file at [`Tab::path`] is deleted when this tab closes and the tab is not saved in session snapshots.
    pub ephemeral_temp_file: bool,
}

impl Default for PdfViewerState {
    fn default() -> Self {
        Self {
            current_page: 0,
            page_count: 0,
            zoom: 1.0,
            fitted: false,
            file_size: 0,
            error: None,
            display_title: None,
            ephemeral_temp_file: false,
        }
    }
}

/// Progress state for background file loading.
#[derive(Debug, Clone)]
pub struct LoadingProgress {
    /// File path being loaded
    pub path: PathBuf,
    /// Bytes loaded so far
    pub bytes_loaded: u64,
    /// Total file size in bytes
    pub total_size: u64,
}

impl LoadingProgress {
    /// Progress as a fraction (0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        if self.total_size == 0 {
            0.0
        } else {
            (self.bytes_loaded as f64 / self.total_size as f64) as f32
        }
    }

    /// Bytes loaded in megabytes.
    pub fn mb_loaded(&self) -> f64 {
        self.bytes_loaded as f64 / (1024.0 * 1024.0)
    }

    /// Total size in megabytes.
    pub fn mb_total(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }
}

/// Content state for a tab — either loading in background or fully loaded.
#[derive(Debug, Clone)]
pub enum TabContent {
    /// File is being loaded in background thread.
    Loading(LoadingProgress),
    /// Content is fully loaded and ready for editing.
    Ready,
    /// File loading failed with an error message.
    Error(String),
}

impl Default for TabContent {
    fn default() -> Self {
        TabContent::Ready
    }
}

/// The kind of content a tab holds.
#[derive(Debug, Clone, PartialEq)]
pub enum TabKind {
    /// Regular document tab (file editing)
    Document,
    /// Special non-editable tab (settings, about, etc.)
    Special(SpecialTabKind),
    /// Image viewer tab (read-only image display with zoom)
    ImageViewer(ImageViewerState),
    /// PDF viewer tab (read-only PDF display with page navigation)
    PdfViewer(PdfViewerState),
}

impl Default for TabKind {
    fn default() -> Self {
        TabKind::Document
    }
}

///
/// This struct holds the complete state of an open document tab,
/// including content and editing state. Different from `TabInfo` which
/// is used for persistence/session restoration.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique identifier for this tab
    pub id: usize,
    /// Kind of tab (document or special panel)
    pub kind: TabKind,
    /// Loading state: whether content is still being loaded from disk.
    pub tab_content: TabContent,
    /// File path (None for unsaved/new documents)
    pub path: Option<PathBuf>,
    /// Optional tab title for pathless documents (session-persisted quick notes).
    pub untitled_display_name: Option<String>,
    /// Document content
    pub content: String,
    /// Original content (for detecting modifications).
    /// For large files (>1MB), this is empty and `original_content_hash` is used instead.
    original_content: String,
    /// Hash of original content for large file modification detection.
    /// Only used when file size > LARGE_FILE_THRESHOLD.
    original_content_hash: Option<u64>,
    /// Whether this is a large file (> LARGE_FILE_THRESHOLD bytes).
    /// Used to enable memory optimizations.
    is_large_file: bool,
    /// Multi-cursor state (supports multiple selections/cursors)
    pub cursors: MultiCursor,
    /// Legacy: Cursor position (line, column) - 0-indexed.
    /// Computed from primary cursor, kept for backwards compatibility.
    pub cursor_position: (usize, usize),
    /// Legacy: Text selection range (start_char_index, end_char_index) - None if no selection.
    /// Computed from primary cursor, kept for backwards compatibility.
    pub selection: Option<(usize, usize)>,
    /// Scroll offset in the editor
    pub scroll_offset: f32,
    /// Total content height inside the scroll area (for sync scrolling)
    pub content_height: f32,
    /// Viewport height of the scroll area (for sync scrolling)
    pub viewport_height: f32,
    /// Pending scroll offset to apply on next render (for sync scrolling on mode switch)
    pub pending_scroll_offset: Option<f32>,
    /// Pending cursor position to restore on next render (for undo/redo)
    /// When Some, the editor widget will restore cursor to this char index
    pub pending_cursor_restore: Option<usize>,
    /// Pending scroll ratio to apply (0.0 to 1.0) - used when content_height is unknown
    pub pending_scroll_ratio: Option<f32>,
    /// Line-to-Y mappings from last rendered mode render (for scroll sync)
    /// Vec of (start_line, end_line, rendered_y)
    pub rendered_line_mappings: Vec<(usize, usize, f32)>,
    /// Actual line height in Raw mode (for accurate scroll sync)
    pub raw_line_height: f32,
    /// Pending target line to scroll to (for sync scrolling, used with line mappings)
    pub pending_scroll_to_line: Option<usize>,
    /// Pending scroll anchor for wrap-aware raw scroll (1-indexed line, fraction within line)
    pub pending_scroll_anchor: Option<(usize, f32)>,
    /// Skip cursor sync from editor on next frame (set when navigating from outline/minimap)
    pub skip_cursor_sync: bool,
    /// View mode for this tab (raw or rendered)
    pub view_mode: ViewMode,
    /// Unified operation-based undo/redo history (replaces snapshot stacks).
    edit_history: EditHistory,
    /// Content version counter - incremented on undo/redo to signal
    /// external content changes to the editor widget
    content_version: u64,
    /// Monotonic counter bumped only on **external** content invalidation (raw edits,
    /// file reload, undo/redo, split-pane raw sync). Rendered WYSIWYG block commits do
    /// not bump this — future stable egui widget id scope (`ui.push_id(source_epoch, …)`).
    /// Independent of per-frame `content_hash()` used for viewport culling / height caches.
    source_epoch: u64,
    /// Cached file type (computed from path, updated on path change)
    file_type: FileType,
    /// Whether the editor should request focus on next frame
    pub needs_focus: bool,
    /// Transient highlight for search result navigation (not persisted).
    pub transient_highlight: TransientHighlight,
    /// Whether auto-save is enabled for this tab (per-tab toggle)
    pub auto_save_enabled: bool,
    /// Time of last content edit (for idle-based auto-save scheduling)
    pub last_edit_time: Option<std::time::Instant>,
    /// Hash of content at last auto-save (to detect if content needs saving)
    last_auto_save_content_hash: Option<u64>,
    /// Fold state for code folding
    pub fold_state: FoldState,
    /// Split view ratio (0.0 to 1.0, proportion of width for left pane)
    /// Default is 0.5 (50/50 split). Only used when view_mode is Split.
    pub split_ratio: f32,
    /// Live Pipeline state for this tab (JSON/YAML command piping)
    pub pipeline_state: TabPipelineState,
    /// Detected encoding when the file was opened (e.g., "UTF-8", "WINDOWS-1252")
    /// None for new/unsaved documents that were created in-app
    pub detected_encoding: Option<&'static str>,
    /// Original file bytes for re-decoding when user changes encoding
    /// Empty for new documents created in-app
    pub original_bytes: Vec<u8>,
    /// Currently selected encoding label (used for save operations)
    /// Defaults to "utf-8" for new documents
    pub current_encoding: &'static str,
    /// Whether the original file had a BOM (Byte Order Mark).
    /// Used to preserve BOM when saving UTF-16 and UTF-8 with BOM files.
    pub had_bom: bool,
    /// Lazily-cloned content snapshot for diff-based undo recording.
    /// Populated by `prepare_undo_snapshot_hashed()` only when the blake3
    /// hash changes; `record_edit_from_snapshot()` updates it in-place.
    pending_undo_snapshot: Option<String>,
    /// Blake3 hash of content at the time of the last undo snapshot.
    /// Used to skip cloning when content hasn't changed between frames.
    undo_content_hash: [u8; 32],

    // ── Per-frame cache fields (invalidated via content_version) ─────────
    cached_text_stats: TextStats,
    cached_text_stats_version: u64,
    cached_is_modified: bool,
    cached_is_modified_version: u64,
    /// Tracks save events separately so is_modified cache invalidates on save.
    save_version: u64,
    cached_is_modified_save_version: u64,
    cached_needs_cjk: bool,
    cached_needs_cjk_version: u64,
    cached_needs_complex_script: bool,
    cached_needs_complex_script_version: u64,
    /// content_version at last auto-save (avoids O(N) hash_content per frame)
    last_auto_save_content_version: Option<u64>,
}

impl Tab {
    /// Clear programmatic scroll targets from split/mode sync (not app-level outline nav).
    pub fn clear_sync_pending_scroll(&mut self) {
        self.pending_scroll_offset = None;
        self.pending_scroll_anchor = None;
        self.pending_scroll_ratio = None;
        self.pending_scroll_to_line = None;
    }

    /// Compute a 64-bit hash of content for modification detection.
    fn compute_content_hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Create a new empty tab.
    ///
    /// New tabs default to Raw view mode and Markdown file type.
    /// The editor will automatically receive focus on the next frame.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            kind: TabKind::Document,
            tab_content: TabContent::Ready,
            path: None,
            untitled_display_name: None,
            content: String::new(),
            original_content: String::new(),
            original_content_hash: None,
            is_large_file: false,
            cursors: MultiCursor::new(),
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            content_height: 0.0,
            viewport_height: 0.0,
            pending_scroll_offset: None,
            pending_cursor_restore: None,
            pending_scroll_ratio: None,
            rendered_line_mappings: Vec::new(),
            raw_line_height: 20.0, // Default, updated on first render
            pending_scroll_to_line: None,
            pending_scroll_anchor: None,
            skip_cursor_sync: false,
            view_mode: ViewMode::Raw, // New documents default to raw mode
            edit_history: EditHistory::new(),
            content_version: 0,
            source_epoch: 0,
            file_type: FileType::Markdown, // New tabs default to markdown
            needs_focus: true,             // Auto-focus new tabs
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false, // Will be set from settings by caller
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: 0.5, // Default to 50/50 split
            pipeline_state: TabPipelineState::default(),
            detected_encoding: None, // New documents have no detected encoding
            original_bytes: Vec::new(), // No original bytes for new docs
            current_encoding: "utf-8", // Default to UTF-8 for new documents
            had_bom: false,          // New documents don't have a BOM
            pending_undo_snapshot: None,
            undo_content_hash: [0u8; 32],
            cached_text_stats: TextStats::default(),
            cached_text_stats_version: u64::MAX,
            cached_is_modified: false,
            cached_is_modified_version: u64::MAX,
            save_version: 0,
            cached_is_modified_save_version: u64::MAX,
            cached_needs_cjk: false,
            cached_needs_cjk_version: u64::MAX,
            cached_needs_complex_script: false,
            cached_needs_complex_script_version: u64::MAX,
            last_auto_save_content_version: None,
        }
    }

    /// Create a new empty tab with settings-based defaults.
    ///
    /// # Arguments
    /// * `id` - Unique tab identifier
    /// * `auto_save_default` - Whether auto-save is enabled by default
    /// * `default_view_mode` - Default view mode for new tabs (Raw, Rendered, or Split)
    pub fn new_with_settings(
        id: usize,
        auto_save_default: bool,
        default_view_mode: ViewMode,
    ) -> Self {
        let mut tab = Self::new(id);
        tab.auto_save_enabled = auto_save_default;
        tab.view_mode = default_view_mode;
        tab
    }

    /// Create a tab with content from a file.
    ///
    /// Newly opened files default to Raw view mode.
    /// File type is detected from the path extension.
    /// The editor will automatically receive focus on the next frame.
    pub fn with_file(id: usize, path: PathBuf, content: String) -> Self {
        let file_type = FileType::from_path(&path);
        let is_large_file = content.len() >= LARGE_FILE_THRESHOLD;

        // For large files, store hash instead of full content to save memory
        let (original_content, original_content_hash) = if is_large_file {
            log::info!(
                "Opening large file ({} bytes): using hash-based modification detection",
                content.len()
            );
            (String::new(), Some(Self::compute_content_hash(&content)))
        } else {
            (content.clone(), None)
        };

        let edit_history = if is_large_file {
            EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
        } else {
            EditHistory::new()
        };

        Self {
            id,
            kind: TabKind::Document,
            tab_content: TabContent::Ready,
            path: Some(path),
            untitled_display_name: None,
            content,
            original_content,
            original_content_hash,
            is_large_file,
            cursors: MultiCursor::new(),
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            content_height: 0.0,
            viewport_height: 0.0,
            pending_scroll_offset: None,
            pending_cursor_restore: None,
            pending_scroll_ratio: None,
            rendered_line_mappings: Vec::new(),
            raw_line_height: 20.0,
            pending_scroll_to_line: None,
            pending_scroll_anchor: None,
            skip_cursor_sync: false,
            view_mode: ViewMode::Raw,
            edit_history,
            content_version: 0,
            source_epoch: 0,
            file_type,
            needs_focus: true,
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false,
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: 0.5,
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some("utf-8"),
            original_bytes: Vec::new(),
            current_encoding: "utf-8",
            had_bom: false,
            pending_undo_snapshot: None,
            undo_content_hash: [0u8; 32],
            cached_text_stats: TextStats::default(),
            cached_text_stats_version: u64::MAX,
            cached_is_modified: false,
            cached_is_modified_version: u64::MAX,
            save_version: 0,
            cached_is_modified_save_version: u64::MAX,
            cached_needs_cjk: false,
            cached_needs_cjk_version: u64::MAX,
            cached_needs_complex_script: false,
            cached_needs_complex_script_version: u64::MAX,
            last_auto_save_content_version: None,
        }
    }

    /// Create a tab with content loaded from file bytes with automatic encoding detection.
    ///
    /// Uses chardetng for encoding detection and encoding_rs for decoding.
    /// For large files (>1MB), uses hash-based modification detection to save memory.
    pub fn with_file_bytes(id: usize, path: PathBuf, bytes: Vec<u8>) -> Self {
        use chardetng::EncodingDetector;

        let file_type = FileType::from_path(&path);
        let bytes_len = bytes.len();
        let is_large_file = bytes_len >= LARGE_FILE_THRESHOLD;

        // Detect encoding using chardetng
        let mut detector = EncodingDetector::new();
        detector.feed(&bytes, true);
        let detected = detector.guess(None, true);
        let encoding_label = detected.name();

        // Check for BOM first - encoding_rs handles this
        let (content, actual_encoding, _had_errors, had_bom) =
            if let Some((bom_encoding, bom_len)) = encoding_rs::Encoding::for_bom(&bytes) {
                // BOM detected, use that encoding and skip BOM bytes
                // Use decode_without_bom_handling since we already handled the BOM
                let (decoded, had_errors) =
                    bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
                (decoded.into_owned(), bom_encoding.name(), had_errors, true)
            } else {
                // No BOM, use detected encoding
                let (decoded, _, had_errors) = detected.decode(&bytes);
                (decoded.into_owned(), encoding_label, had_errors, false)
            };

        let (original_content, original_content_hash, original_bytes) = if is_large_file {
            log::info!(
                "Opening large file ({} bytes): using hash-based modification detection",
                bytes_len
            );
            (
                String::new(),
                Some(Self::compute_content_hash(&content)),
                Vec::new(),
            )
        } else {
            (content.clone(), None, bytes)
        };

        let edit_history = if is_large_file {
            EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
        } else {
            EditHistory::new()
        };

        Self {
            id,
            kind: TabKind::Document,
            tab_content: TabContent::Ready,
            path: Some(path),
            untitled_display_name: None,
            content,
            original_content,
            original_content_hash,
            is_large_file,
            cursors: MultiCursor::new(),
            cursor_position: (0, 0),
            selection: None,
            scroll_offset: 0.0,
            content_height: 0.0,
            viewport_height: 0.0,
            pending_scroll_offset: None,
            pending_cursor_restore: None,
            pending_scroll_ratio: None,
            rendered_line_mappings: Vec::new(),
            raw_line_height: 20.0,
            pending_scroll_to_line: None,
            pending_scroll_anchor: None,
            skip_cursor_sync: false,
            view_mode: ViewMode::Raw,
            edit_history,
            content_version: 0,
            source_epoch: 0,
            file_type,
            needs_focus: true,
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false,
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: 0.5,
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some(actual_encoding),
            original_bytes,
            current_encoding: actual_encoding,
            had_bom,
            pending_undo_snapshot: None,
            undo_content_hash: [0u8; 32],
            cached_text_stats: TextStats::default(),
            cached_text_stats_version: u64::MAX,
            cached_is_modified: false,
            cached_is_modified_version: u64::MAX,
            save_version: 0,
            cached_is_modified_save_version: u64::MAX,
            cached_needs_cjk: false,
            cached_needs_cjk_version: u64::MAX,
            cached_needs_complex_script: false,
            cached_needs_complex_script_version: u64::MAX,
            last_auto_save_content_version: None,
        }
    }

    /// Create a tab with content from a file, with settings-based defaults.
    ///
    /// # Arguments
    /// * `id` - Unique tab identifier
    /// * `path` - File path
    /// * `content` - File content
    /// * `auto_save_default` - Whether auto-save is enabled by default
    /// * `default_view_mode` - Default view mode for new tabs (Raw, Rendered, or Split)
    pub fn with_file_and_settings(
        id: usize,
        path: PathBuf,
        content: String,
        auto_save_default: bool,
        default_view_mode: ViewMode,
    ) -> Self {
        let mut tab = Self::with_file(id, path, content);
        tab.auto_save_enabled = auto_save_default;
        tab.view_mode = default_view_mode;
        tab
    }

    /// Create a tab from file bytes with encoding detection and settings.
    ///
    /// # Arguments
    /// * `id` - Unique tab identifier
    /// * `path` - File path
    /// * `bytes` - Raw file bytes for encoding detection
    /// * `auto_save_default` - Whether auto-save is enabled by default
    /// * `default_view_mode` - Default view mode for new tabs (Raw, Rendered, or Split)
    pub fn with_file_bytes_and_settings(
        id: usize,
        path: PathBuf,
        bytes: Vec<u8>,
        auto_save_default: bool,
        default_view_mode: ViewMode,
    ) -> Self {
        let mut tab = Self::with_file_bytes(id, path, bytes);
        tab.auto_save_enabled = auto_save_default;
        tab.view_mode = default_view_mode;
        tab
    }

    /// Create a tab from saved session info.
    ///
    /// Restores the view mode and split ratio from the saved session.
    /// File type is detected from the path extension.
    /// Restored tabs don't auto-focus since we're restoring previous state.
    pub fn from_tab_info(id: usize, info: &TabInfo, content: String) -> Self {
        let file_type = info
            .path
            .as_ref()
            .map(|p| FileType::from_path(p))
            .unwrap_or(FileType::Markdown);
        // Convert legacy cursor position to char index for MultiCursor
        let cursor_char_idx =
            line_col_to_char_index(&content, info.cursor_position.0, info.cursor_position.1);

        let is_large_file = content.len() >= LARGE_FILE_THRESHOLD;
        let (original_content, original_content_hash) = if is_large_file {
            (String::new(), Some(Self::compute_content_hash(&content)))
        } else {
            (content.clone(), None)
        };

        let edit_history = if is_large_file {
            EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
        } else {
            EditHistory::new()
        };

        Self {
            id,
            kind: TabKind::Document,
            tab_content: TabContent::Ready,
            path: info.path.clone(),
            untitled_display_name: None,
            content,
            original_content,
            original_content_hash,
            is_large_file,
            cursors: MultiCursor::single(cursor_char_idx),
            cursor_position: info.cursor_position,
            selection: None,
            scroll_offset: info.scroll_offset,
            content_height: 0.0,
            viewport_height: 0.0,
            pending_scroll_offset: None,
            pending_cursor_restore: None,
            pending_scroll_ratio: None,
            rendered_line_mappings: Vec::new(),
            raw_line_height: 20.0,
            pending_scroll_to_line: None,
            pending_scroll_anchor: None,
            skip_cursor_sync: false,
            view_mode: info.view_mode,
            edit_history,
            content_version: 0,
            source_epoch: 0,
            file_type,
            needs_focus: false,
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false,
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: info.split_ratio,
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some("utf-8"),
            original_bytes: Vec::new(),
            current_encoding: "utf-8",
            had_bom: false,
            pending_undo_snapshot: None,
            undo_content_hash: [0u8; 32],
            cached_text_stats: TextStats::default(),
            cached_text_stats_version: u64::MAX,
            cached_is_modified: false,
            cached_is_modified_version: u64::MAX,
            save_version: 0,
            cached_is_modified_save_version: u64::MAX,
            cached_needs_cjk: false,
            cached_needs_cjk_version: u64::MAX,
            cached_needs_complex_script: false,
            cached_needs_complex_script_version: u64::MAX,
            last_auto_save_content_version: None,
        }
    }

    /// Create a tab from session info with settings-based auto-save.
    pub fn from_tab_info_with_settings(
        id: usize,
        info: &TabInfo,
        content: String,
        auto_save_default: bool,
    ) -> Self {
        let mut tab = Self::from_tab_info(id, info, content);
        tab.auto_save_enabled = auto_save_default;
        tab
    }

    /// Create a tab from session info using raw file bytes with encoding detection.
    ///
    /// This combines tab info restoration (view mode, split ratio) with
    /// automatic encoding detection from the file bytes.
    /// For large files, uses hash-based modification detection.
    pub fn from_tab_info_with_bytes(
        id: usize,
        info: &TabInfo,
        bytes: Vec<u8>,
        auto_save_default: bool,
    ) -> Self {
        use chardetng::EncodingDetector;

        let file_type = info
            .path
            .as_ref()
            .map(|p| FileType::from_path(p))
            .unwrap_or(FileType::Markdown);

        let bytes_len = bytes.len();
        let is_large_file = bytes_len >= LARGE_FILE_THRESHOLD;

        // Detect encoding
        let mut detector = EncodingDetector::new();
        detector.feed(&bytes, true);
        let detected = detector.guess(None, true);

        // Check for BOM first
        let (content, actual_encoding, had_bom) =
            if let Some((bom_encoding, bom_len)) = encoding_rs::Encoding::for_bom(&bytes) {
                // Use decode_without_bom_handling since we already handled the BOM
                let (decoded, _had_errors) =
                    bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
                (decoded.into_owned(), bom_encoding.name(), true)
            } else {
                let (decoded, _, _) = detected.decode(&bytes);
                (decoded.into_owned(), detected.name(), false)
            };

        // Convert legacy cursor position to char index
        let cursor_char_idx =
            line_col_to_char_index(&content, info.cursor_position.0, info.cursor_position.1);

        let (original_content, original_content_hash, original_bytes) = if is_large_file {
            log::info!(
                "Restoring large file ({} bytes): using hash-based modification detection",
                bytes_len
            );
            (
                String::new(),
                Some(Self::compute_content_hash(&content)),
                Vec::new(),
            )
        } else {
            (content.clone(), None, bytes)
        };

        let edit_history = if is_large_file {
            EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
        } else {
            EditHistory::new()
        };

        Self {
            id,
            kind: TabKind::Document,
            tab_content: TabContent::Ready,
            path: info.path.clone(),
            untitled_display_name: None,
            content,
            original_content,
            original_content_hash,
            is_large_file,
            cursors: MultiCursor::single(cursor_char_idx),
            cursor_position: info.cursor_position,
            selection: None,
            scroll_offset: info.scroll_offset,
            content_height: 0.0,
            viewport_height: 0.0,
            pending_scroll_offset: None,
            pending_cursor_restore: None,
            pending_scroll_ratio: None,
            rendered_line_mappings: Vec::new(),
            raw_line_height: 20.0,
            pending_scroll_to_line: None,
            pending_scroll_anchor: None,
            skip_cursor_sync: false,
            view_mode: info.view_mode,
            edit_history,
            content_version: 0,
            source_epoch: 0,
            file_type,
            needs_focus: false,
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: auto_save_default,
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: info.split_ratio,
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some(actual_encoding),
            original_bytes,
            current_encoding: actual_encoding,
            had_bom,
            pending_undo_snapshot: None,
            undo_content_hash: [0u8; 32],
            cached_text_stats: TextStats::default(),
            cached_text_stats_version: u64::MAX,
            cached_is_modified: false,
            cached_is_modified_version: u64::MAX,
            save_version: 0,
            cached_is_modified_save_version: u64::MAX,
            cached_needs_cjk: false,
            cached_needs_cjk_version: u64::MAX,
            cached_needs_complex_script: false,
            cached_needs_complex_script_version: u64::MAX,
            last_auto_save_content_version: None,
        }
    }

    /// Create a placeholder tab for a file that is being loaded in the background.
    ///
    /// The tab is immediately added and visible but renders a progress indicator
    /// instead of editor content until loading completes.
    pub fn new_loading(id: usize, path: PathBuf, total_size: u64) -> Self {
        let file_type = FileType::from_path(&path);
        let mut tab = Self::new(id);
        tab.tab_content = TabContent::Loading(LoadingProgress {
            path: path.clone(),
            bytes_loaded: 0,
            total_size,
        });
        tab.path = Some(path);
        tab.file_type = file_type;
        tab.needs_focus = true;
        tab
    }

    /// Finalize background loading: populate tab with decoded content and transition to Ready.
    ///
    /// This is called from the main thread when the background reader sends the complete bytes.
    pub fn finish_loading(
        &mut self,
        bytes: Vec<u8>,
        auto_save_default: bool,
        default_view_mode: crate::config::ViewMode,
    ) {
        use chardetng::EncodingDetector;

        let bytes_len = bytes.len();
        let is_large_file = bytes_len >= LARGE_FILE_THRESHOLD;

        let mut detector = EncodingDetector::new();
        detector.feed(&bytes, true);
        let detected = detector.guess(None, true);

        let (content, actual_encoding, had_bom) =
            if let Some((bom_encoding, bom_len)) = encoding_rs::Encoding::for_bom(&bytes) {
                let (decoded, _had_errors) =
                    bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
                (decoded.into_owned(), bom_encoding.name(), true)
            } else {
                let (decoded, _, _) = detected.decode(&bytes);
                (decoded.into_owned(), detected.name(), false)
            };

        let (original_content, original_content_hash, original_bytes) = if is_large_file {
            log::info!(
                "Background load complete ({} bytes): using hash-based modification detection",
                bytes_len
            );
            (
                String::new(),
                Some(Self::compute_content_hash(&content)),
                Vec::new(),
            )
        } else {
            (content.clone(), None, bytes)
        };

        self.content = content;
        self.original_content = original_content;
        self.original_content_hash = original_content_hash;
        self.is_large_file = is_large_file;
        self.original_bytes = original_bytes;
        self.detected_encoding = Some(actual_encoding);
        self.current_encoding = actual_encoding;
        self.had_bom = had_bom;
        self.auto_save_enabled = auto_save_default;
        self.view_mode = default_view_mode;
        self.edit_history = if is_large_file {
            EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
        } else {
            EditHistory::new()
        };
        self.tab_content = TabContent::Ready;
        self.content_version = self.content_version.wrapping_add(1);
        self.bump_source_epoch();
    }

    /// Mark this tab's loading as failed with an error message.
    pub fn fail_loading(&mut self, error: String) {
        self.tab_content = TabContent::Error(error);
    }

    /// Check if the tab has unsaved changes (cached via content_version + save_version).
    ///
    /// For large files (>1MB), uses hash comparison instead of full string comparison
    /// to avoid storing a full copy of the original content.
    /// The result is cached and only recomputed when content or save state changes.
    pub fn is_modified(&self) -> bool {
        if self.cached_is_modified_version == self.content_version
            && self.cached_is_modified_save_version == self.save_version
        {
            return self.cached_is_modified;
        }
        self.is_modified_uncached()
    }

    /// Recompute is_modified and update cache (call from &mut self contexts).
    pub fn is_modified_cached(&mut self) -> bool {
        if self.cached_is_modified_version == self.content_version
            && self.cached_is_modified_save_version == self.save_version
        {
            return self.cached_is_modified;
        }
        let result = self.is_modified_uncached();
        self.cached_is_modified = result;
        self.cached_is_modified_version = self.content_version;
        self.cached_is_modified_save_version = self.save_version;
        result
    }

    fn is_modified_uncached(&self) -> bool {
        if self.is_special() {
            return false;
        }
        if let Some(hash) = self.original_content_hash {
            Self::compute_content_hash(&self.content) != hash
        } else {
            self.content != self.original_content
        }
    }

    /// Cached text statistics (word/char/line counts), recomputed only when content changes.
    pub fn text_stats(&mut self) -> TextStats {
        if self.cached_text_stats_version == self.content_version {
            return self.cached_text_stats;
        }
        self.cached_text_stats = TextStats::from_text(&self.content);
        self.cached_text_stats_version = self.content_version;
        self.cached_text_stats
    }

    /// Whether content contains CJK characters (cached via content_version).
    pub fn needs_cjk_cached(&mut self) -> bool {
        if self.cached_needs_cjk_version == self.content_version {
            return self.cached_needs_cjk;
        }
        self.cached_needs_cjk = crate::fonts::needs_cjk(&self.content);
        self.cached_needs_cjk_version = self.content_version;
        self.cached_needs_cjk
    }

    /// Whether content contains complex script characters (cached via content_version).
    pub fn needs_complex_script_cached(&mut self) -> bool {
        if self.cached_needs_complex_script_version == self.content_version {
            return self.cached_needs_complex_script;
        }
        self.cached_needs_complex_script = crate::fonts::needs_complex_script_fonts(&self.content);
        self.cached_needs_complex_script_version = self.content_version;
        self.cached_needs_complex_script
    }

    /// Check if this is a large file that uses memory-optimized storage.
    pub fn is_large_file(&self) -> bool {
        self.is_large_file
    }

    /// Hash of the disk content this tab was last loaded from (or last saved to).
    ///
    /// Used by session recovery (`RecoveryContent::original_content_hash`) and
    /// autosave identity checks to verify that a recovery file still belongs to
    /// the same on-disk file the tab was opened with — independent of whether
    /// the in-memory buffer has diverged.
    ///
    /// - Large files: returns the cached `original_content_hash` (computed at
    ///   load / `mark_saved`).
    /// - Small files (where `original_content` holds the disk text): hashes
    ///   `original_content` on the fly with the same `DefaultHasher` algorithm
    ///   used by `crate::config::session::hash_content`, so the value is
    ///   directly comparable to a hash of current disk content.
    /// - Untitled tabs that have never been written to disk: returns `None`.
    pub fn disk_content_hash(&self) -> Option<u64> {
        if let Some(hash) = self.original_content_hash {
            return Some(hash);
        }
        if self.path.is_none() && self.original_content.is_empty() {
            return None;
        }
        Some(Self::compute_content_hash(&self.original_content))
    }

    /// Check if this is a new/untitled file (not yet saved to disk).
    ///
    /// Returns `true` if the tab has no associated file path, meaning it was
    /// created in the app and has never been saved. This is distinct from
    /// files that were loaded from disk (even if they're empty).
    pub fn is_new_file(&self) -> bool {
        self.path.is_none()
    }

    /// Check if this is an unmodified empty untitled file.
    ///
    /// Returns `true` if:
    /// - The tab is a new file (no path, never saved)
    /// - The content matches the initial empty state
    ///
    /// These files can be closed without prompting to save since there's
    /// nothing meaningful to preserve.
    pub fn is_empty_untitled(&self) -> bool {
        self.is_new_file() && self.content.is_empty() && self.original_content.is_empty()
    }

    /// Determine if we should prompt to save before closing or exiting.
    ///
    /// The logic is:
    /// - If the file is modified (content differs from original), prompt to save
    /// - EXCEPTION: Skip prompt for empty untitled files (nothing to save)
    /// - EXCEPTION: When **Quick note workflow** is enabled, skip prompts for
    ///   pathless documents on **app exit** only; closing an individual untitled
    ///   tab with content still prompts. Session recovery preserves scratch buffers.
    pub fn should_prompt_to_save(&self, settings: &Settings, context: SavePromptContext) -> bool {
        // Special tabs, image viewer tabs, PDF viewer tabs, and loading tabs never need to save
        if self.is_special()
            || self.is_image_viewer()
            || self.is_pdf_viewer()
            || self.is_loading()
            || self.is_load_error()
        {
            return false;
        }

        if settings.quick_note_workflow
            && self.is_new_file()
            && context == SavePromptContext::AppExit
        {
            return false;
        }

        // Don't prompt for unmodified files
        if !self.is_modified() {
            return false;
        }

        // Don't prompt for empty untitled files (nothing meaningful to save)
        // This handles the case where content was typed and then deleted
        if self.is_new_file() && self.content.is_empty() {
            return false;
        }

        // All other modified files should prompt
        true
    }

    /// Label used for session persistence and crash recovery metadata (no `*` suffix).
    pub fn persisted_session_display_title(&self) -> String {
        if self.is_special() || self.is_image_viewer() || self.is_pdf_viewer() {
            return self.title();
        }
        if let Some(path) = &self.path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled")
                .to_string()
        } else {
            self.untitled_display_name
                .clone()
                .unwrap_or_else(|| "Untitled".to_string())
        }
    }

    /// Initial text for the rename-untitled dialog.
    pub fn untitled_rename_buffer_initial(&self) -> String {
        self.untitled_display_name
            .clone()
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Get the display title for this tab.
    pub fn title(&self) -> String {
        if let TabKind::Special(special) = &self.kind {
            return format!("{} {}", special.icon(), special.title());
        }

        if matches!(&self.kind, TabKind::ImageViewer(_)) {
            let name = self
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Image");
            return format!("\u{1F5BC} {}", name); // framed picture emoji
        }

        if let TabKind::PdfViewer(vs) = &self.kind {
            if let Some(title) = &vs.display_title {
                return format!("\u{1F4C4} {}", title);
            }
            let name = self
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("PDF");
            return format!("\u{1F4C4} {}", name); // page facing up emoji
        }

        let name: String = if let Some(path) = &self.path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled")
                .to_string()
        } else {
            self.untitled_display_name
                .clone()
                .unwrap_or_else(|| "Untitled".to_string())
        };

        if self.is_loading() {
            return format!("\u{23F3} {}", name); // hourglass
        }

        if self.is_load_error() {
            return format!("\u{26A0} {}", name); // warning sign
        }

        if self.is_modified() {
            format!("{}*", name)
        } else {
            name
        }
    }

    /// Check if this tab is a special (non-editable) tab.
    pub fn is_special(&self) -> bool {
        matches!(self.kind, TabKind::Special(_))
    }

    /// Check if this tab is an image viewer tab.
    pub fn is_image_viewer(&self) -> bool {
        matches!(self.kind, TabKind::ImageViewer(_))
    }

    /// Check if this tab is a PDF viewer tab.
    pub fn is_pdf_viewer(&self) -> bool {
        matches!(self.kind, TabKind::PdfViewer(_))
    }

    /// Check if this tab is currently loading file content from disk.
    pub fn is_loading(&self) -> bool {
        matches!(self.tab_content, TabContent::Loading(_))
    }

    /// Check if this tab had a loading error.
    pub fn is_load_error(&self) -> bool {
        matches!(self.tab_content, TabContent::Error(_))
    }

    /// Get loading progress if this tab is currently loading.
    pub fn loading_progress(&self) -> Option<&LoadingProgress> {
        if let TabContent::Loading(ref progress) = self.tab_content {
            Some(progress)
        } else {
            None
        }
    }

    /// Mark the current content as saved (updates original_content or hash).
    /// Also clears auto-save state since content is now persisted.
    pub fn mark_saved(&mut self) {
        if self.is_large_file {
            self.original_content_hash = Some(Self::compute_content_hash(&self.content));
        } else {
            self.original_content = self.content.clone();
        }
        self.last_auto_save_content_hash = None;
        self.last_edit_time = None;
        self.save_version = self.save_version.wrapping_add(1);
        self.cached_is_modified = false;
        self.cached_is_modified_version = self.content_version;
        self.cached_is_modified_save_version = self.save_version;
        if self.path.is_some() {
            self.untitled_display_name = None;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Encoding Methods
    // ─────────────────────────────────────────────────────────────────────────

    /// List of common encodings for the UI picker.
    pub const COMMON_ENCODINGS: &'static [&'static str] = &[
        "utf-8",
        "windows-1252",
        "iso-8859-1",
        "shift_jis",
        "euc-jp",
        "gbk",
        "euc-kr",
        "iso-8859-15",
        "utf-16le",
        "utf-16be",
    ];

    /// Get the display name for the current encoding (uppercase for UI).
    pub fn encoding_display_name(&self) -> String {
        self.current_encoding.to_uppercase()
    }

    /// Change the encoding and re-decode content from original bytes.
    ///
    /// Returns `Ok(())` if successful, or `Err` with a message if the encoding
    /// is invalid or the bytes cannot be decoded with the new encoding.
    ///
    /// Note: This only works if we have original_bytes stored (i.e., file was opened,
    /// not created new). For new documents, this just changes the save encoding.
    pub fn set_encoding(&mut self, new_encoding: &'static str) -> Result<(), String> {
        // Get the encoding from the label
        let encoding = encoding_rs::Encoding::for_label(new_encoding.as_bytes())
            .ok_or_else(|| format!("Unknown encoding: {}", new_encoding))?;

        // If we have original bytes, re-decode the content
        if !self.original_bytes.is_empty() {
            let (decoded, _actual_encoding, had_errors) = encoding.decode(&self.original_bytes);

            if had_errors {
                // Still update, but warn about errors
                log::warn!(
                    "Decoding with {} had errors - some characters may be replaced",
                    new_encoding
                );
            }

            // Update content (preserve cursor position as best we can)
            let old_len = self.content.len();
            self.content = decoded.into_owned();
            self.original_content = self.content.clone();

            // If content length changed significantly, reset cursor
            if ((self.content.len() as isize) - (old_len as isize)).abs() > 100 {
                self.cursors = MultiCursor::new();
                self.cursor_position = (0, 0);
            }
        }

        // Update the encoding label for future saves
        self.current_encoding = new_encoding;
        self.detected_encoding = Some(new_encoding);

        log::info!("Changed encoding to: {}", new_encoding);
        Ok(())
    }

    /// Encode the current content to bytes using the selected encoding.
    ///
    /// Returns the encoded bytes. If the encoding doesn't support certain characters,
    /// they may be replaced with fallback characters.
    ///
    /// If the original file had a BOM (Byte Order Mark), it will be prepended to the output.
    /// This is important for UTF-16 files which require a BOM for proper detection.
    ///
    /// Note: encoding_rs does NOT support encoding TO UTF-16, only decoding FROM it.
    /// We handle UTF-16 encoding manually using Rust's built-in encode_utf16().
    pub fn encode_content(&self) -> Vec<u8> {
        let encoding_lower = self.current_encoding.to_lowercase();

        // Handle UTF-16 specially - encoding_rs doesn't support encoding TO UTF-16
        if encoding_lower == "utf-16le" || encoding_lower == "utf-16-le" {
            let mut result = Vec::new();
            // Add BOM if original had one
            if self.had_bom {
                result.extend_from_slice(&[0xff, 0xfe]);
            }
            // Encode to UTF-16LE (little endian)
            for code_unit in self.content.encode_utf16() {
                result.extend_from_slice(&code_unit.to_le_bytes());
            }
            return result;
        }

        if encoding_lower == "utf-16be" || encoding_lower == "utf-16-be" {
            let mut result = Vec::new();
            // Add BOM if original had one
            if self.had_bom {
                result.extend_from_slice(&[0xfe, 0xff]);
            }
            // Encode to UTF-16BE (big endian)
            for code_unit in self.content.encode_utf16() {
                result.extend_from_slice(&code_unit.to_be_bytes());
            }
            return result;
        }

        // For all other encodings, use encoding_rs
        let encoding = encoding_rs::Encoding::for_label(self.current_encoding.as_bytes())
            .unwrap_or(encoding_rs::UTF_8);

        let (encoded, _actual_encoding, _had_errors) = encoding.encode(&self.content);
        let mut result = encoded.into_owned();

        // Prepend BOM if the original file had one (for UTF-8 with BOM)
        if self.had_bom {
            let bom = Self::get_bom_for_encoding(self.current_encoding);
            if !bom.is_empty() {
                let mut with_bom = bom.to_vec();
                with_bom.append(&mut result);
                return with_bom;
            }
        }

        result
    }

    /// Get the BOM bytes for a given encoding label.
    fn get_bom_for_encoding(encoding_label: &str) -> &'static [u8] {
        match encoding_label.to_lowercase().as_str() {
            "utf-8" => &[0xef, 0xbb, 0xbf],
            "utf-16le" | "utf-16-le" => &[0xff, 0xfe],
            "utf-16be" | "utf-16-be" => &[0xfe, 0xff],
            _ => &[], // Other encodings don't use BOM
        }
    }

    /// Check if the current encoding is UTF-8.
    pub fn is_utf8(&self) -> bool {
        self.current_encoding.eq_ignore_ascii_case("utf-8")
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Auto-Save Methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Toggle auto-save for this tab.
    pub fn toggle_auto_save(&mut self) {
        self.auto_save_enabled = !self.auto_save_enabled;
        if !self.auto_save_enabled {
            // Clear auto-save tracking when disabled
            self.last_edit_time = None;
        }
    }

    /// Mark that content was edited (updates last_edit_time for auto-save scheduling).
    pub fn mark_content_edited(&mut self) {
        if self.auto_save_enabled {
            self.last_edit_time = Some(std::time::Instant::now());
        }
    }

    /// Check if auto-save should trigger based on idle time.
    ///
    /// Uses cached is_modified() and content_version to avoid O(N) per-frame work.
    pub fn should_auto_save(&self, delay_ms: u32) -> bool {
        if !self.auto_save_enabled || !self.is_modified() {
            return false;
        }

        if let Some(last_ver) = self.last_auto_save_content_version {
            if self.content_version == last_ver {
                return false;
            }
        }

        if let Some(last_edit) = self.last_edit_time {
            last_edit.elapsed() >= std::time::Duration::from_millis(delay_ms as u64)
        } else {
            false
        }
    }

    /// Mark that auto-save was performed (stores content_version to avoid O(N) re-hash).
    pub fn mark_auto_saved(&mut self) {
        self.last_auto_save_content_hash = Some(hash_content(&self.content));
        self.last_auto_save_content_version = Some(self.content_version);
    }

    /// Get the content hash for change detection.
    pub fn content_hash(&self) -> u64 {
        hash_content(&self.content)
    }

    /// Set new content and record diff-based undo operation.
    pub fn set_content(&mut self, new_content: String) {
        if new_content != self.content {
            let ops = compute_edit_ops(&self.content, &new_content);
            self.edit_history.record_operations(ops);
            self.content = new_content;
            self.content_version = self.content_version.wrapping_add(1);
            self.bump_source_epoch();
            self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
            self.mark_content_edited();
        }
    }

    /// Undo the last edit group.
    ///
    /// Applies inverse operations to `self.content` and bumps `content_version`
    /// to signal UI widgets to re-sync. Updates the undo snapshot in-place
    /// (reusing buffer) and refreshes the blake3 hash to avoid a re-clone
    /// on the next frame. Returns cursor char position from the first
    /// operation in the group, or `None` if the undo stack was empty.
    pub fn undo(&mut self) -> Option<usize> {
        let cursor = self.edit_history.undo_string(&mut self.content);
        if cursor.is_some() {
            self.content_version = self.content_version.wrapping_add(1);
            self.bump_source_epoch();
            self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
            match self.pending_undo_snapshot.as_mut() {
                Some(snap) => snap.clone_from(&self.content),
                None => {
                    self.pending_undo_snapshot = Some(self.content.clone());
                }
            }
        }
        cursor
    }

    /// Redo the last undone edit group.
    ///
    /// Reapplies operations to `self.content` and bumps `content_version`.
    /// Updates the undo snapshot in-place and refreshes the blake3 hash.
    /// Returns cursor char position, or `None` if the redo stack was empty.
    pub fn redo(&mut self) -> Option<usize> {
        let cursor = self.edit_history.redo_string(&mut self.content);
        if cursor.is_some() {
            self.content_version = self.content_version.wrapping_add(1);
            self.bump_source_epoch();
            self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
            match self.pending_undo_snapshot.as_mut() {
                Some(snap) => snap.clone_from(&self.content),
                None => {
                    self.pending_undo_snapshot = Some(self.content.clone());
                }
            }
        }
        cursor
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.edit_history.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.edit_history.can_redo()
    }

    /// Get the number of undo groups.
    pub fn undo_count(&self) -> usize {
        self.edit_history.undo_count()
    }

    /// Get the number of redo groups.
    pub fn redo_count(&self) -> usize {
        self.edit_history.redo_count()
    }

    /// Break the current undo group so the next edit starts a new one.
    /// Used after formatting operations and other discrete actions.
    pub fn break_undo_group(&mut self) {
        self.edit_history.break_group();
    }

    /// Get the content version counter.
    pub fn content_version(&self) -> u64 {
        self.content_version
    }

    /// External-invalidation epoch for stable rendered-mode widget ids.
    ///
    /// Bumped on raw edits, file reload, undo/redo, and other changes that replace
    /// content outside the rendered edit session. Rendered WYSIWYG block commits do
    /// not bump this counter.
    pub fn source_epoch(&self) -> u64 {
        self.source_epoch
    }

    /// Bump [`source_epoch`] after external content invalidation.
    pub fn bump_source_epoch(&mut self) {
        self.source_epoch = self.source_epoch.saturating_add(1);
        log::trace!(
            "Tab {} source_epoch bumped to {}",
            self.id,
            self.source_epoch
        );
    }

    /// Signal external content change after assigning to [`Self::content`] directly.
    ///
    /// Bumps [`content_version`] and [`source_epoch`]. Does not record undo — pair with
    /// [`record_edit`](Self::record_edit) when undo support is needed.
    pub fn notify_external_content_change(&mut self) {
        self.content_version = self.content_version.wrapping_add(1);
        self.bump_source_epoch();
    }

    /// Increment the content version counter.
    ///
    /// Call this when content is modified programmatically (e.g., snippet expansion)
    /// to signal to UI widgets that they need to re-read content from the source.
    pub fn increment_content_version(&mut self) {
        self.content_version = self.content_version.wrapping_add(1);
    }

    /// Prepare an undo snapshot using blake3 hash-based change detection.
    ///
    /// Computes a blake3 hash of content and only clones when the hash
    /// differs from the stored hash (i.e., content actually changed).
    /// Uses `clone_from` to reuse existing buffer capacity when possible.
    /// Call before non-Raw view widgets that can modify `content`.
    pub fn prepare_undo_snapshot_hashed(&mut self) {
        let hash = *blake3::hash(self.content.as_bytes()).as_bytes();
        if hash != self.undo_content_hash {
            match self.pending_undo_snapshot.as_mut() {
                Some(snap) => snap.clone_from(&self.content),
                None => {
                    self.pending_undo_snapshot = Some(self.content.clone());
                }
            }
            self.undo_content_hash = hash;
        } else if self.pending_undo_snapshot.is_none() {
            self.pending_undo_snapshot = Some(self.content.clone());
            self.undo_content_hash = hash;
        }
    }

    /// Record an edit by diffing the pending snapshot against current content.
    ///
    /// Rendered-mode edits: use this method (does **not** bump [`source_epoch`]).
    /// Raw / external paths: use [`record_external_edit_from_snapshot`].
    pub fn record_edit_from_snapshot(&mut self) {
        self.record_edit_from_snapshot_inner(false);
    }

    /// Apply queued rendered block commits as logical undo steps (one per commit boundary).
    ///
    /// Called from `central_panel` after `MarkdownEditor::show` drains
    /// [`crate::markdown::rendered_commit_undo::take_pending_commits`].
    pub fn apply_rendered_commit_undo_entries(
        &mut self,
        entries: impl IntoIterator<
            Item = crate::markdown::rendered_commit_undo::PendingRenderedCommitUndo,
        >,
    ) {
        let final_content = self.content.clone();
        for entry in entries {
            if entry.break_group_before {
                self.break_undo_group();
            }
            self.content = entry.post_commit_snapshot;
            self.pending_undo_snapshot = Some(entry.pre_commit_snapshot);
            self.record_edit_from_snapshot();
        }
        self.content = final_content;
        self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
        if let Some(snap) = self.pending_undo_snapshot.as_mut() {
            snap.clone_from(&self.content);
        }
    }

    /// Like [`record_edit_from_snapshot`] but bumps [`source_epoch`] when content changes.
    pub fn record_external_edit_from_snapshot(&mut self) {
        self.record_edit_from_snapshot_inner(true);
    }

    fn record_edit_from_snapshot_inner(&mut self, bump_epoch: bool) {
        if let Some(mut old_content) = self.pending_undo_snapshot.take() {
            if old_content != self.content {
                let ops = compute_edit_ops(&old_content, &self.content);
                self.edit_history.record_operations(ops);
                old_content.clone_from(&self.content);
                self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
                self.content_version = self.content_version.wrapping_add(1);
                if bump_epoch {
                    self.bump_source_epoch();
                }
            }
            self.pending_undo_snapshot = Some(old_content);
        }
    }

    /// Legacy shim: record edit from explicit old content.
    /// Prefer `prepare_undo_snapshot_hashed` + `record_edit_from_snapshot` for new code.
    pub fn record_edit(&mut self, old_content: String, _old_cursor: usize) {
        if old_content != self.content {
            let ops = compute_edit_ops(&old_content, &self.content);
            self.edit_history.record_operations(ops);
            self.undo_content_hash = *blake3::hash(self.content.as_bytes()).as_bytes();
            self.pending_undo_snapshot = None;
            self.bump_source_epoch();
        }
    }

    /// Convert to TabInfo for session persistence.
    pub fn to_tab_info(&self) -> TabInfo {
        TabInfo {
            path: self.path.clone(),
            modified: self.is_modified(),
            cursor_position: self.cursor_position,
            scroll_offset: self.scroll_offset,
            view_mode: self.view_mode,
            split_ratio: self.split_ratio,
        }
    }

    /// Get the current view mode for this tab.
    pub fn get_view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Set the view mode for this tab.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    /// Toggle the view mode: Raw → Split → Rendered → Raw
    pub fn toggle_view_mode(&mut self) -> ViewMode {
        self.view_mode = self.view_mode.toggle();
        self.view_mode
    }

    /// Get the split view ratio for this tab.
    pub fn get_split_ratio(&self) -> f32 {
        self.split_ratio
    }

    /// Set the split view ratio for this tab.
    /// The ratio is clamped to a valid range (0.2 to 0.8).
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.2, 0.8);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Live Pipeline Support
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if pipeline panel is visible for this tab.
    pub fn pipeline_visible(&self) -> bool {
        self.pipeline_state.panel_visible
    }

    /// Toggle the pipeline panel visibility.
    pub fn toggle_pipeline_panel(&mut self) {
        self.pipeline_state.panel_visible = !self.pipeline_state.panel_visible;
    }

    /// Show the pipeline panel.
    pub fn show_pipeline_panel(&mut self) {
        self.pipeline_state.panel_visible = true;
    }

    /// Hide the pipeline panel.
    pub fn hide_pipeline_panel(&mut self) {
        self.pipeline_state.panel_visible = false;
    }

    /// Check if this tab's file type supports pipeline (JSON/YAML).
    pub fn supports_pipeline(&self) -> bool {
        matches!(self.file_type, FileType::Json | FileType::Yaml)
    }

    /// Get the file type for this tab.
    ///
    /// Returns the cached file type, which is determined from the
    /// file path extension. New/unsaved tabs default to Markdown.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Set the file path and update the cached file type.
    ///
    /// This should be called when saving a file with a new path
    /// (e.g., "Save As") to ensure the file type is updated.
    pub fn set_path(&mut self, path: PathBuf) {
        self.file_type = FileType::from_path(&path);
        self.path = Some(path);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Multi-Cursor Support
    // ─────────────────────────────────────────────────────────────────────────

    /// Sync legacy cursor_position and selection fields from the primary cursor.
    ///
    /// Call this after modifying the cursors to keep backwards compatibility
    /// with code that uses the legacy fields.
    pub fn sync_cursor_from_primary(&mut self) {
        self.cursor_position = self.cursors.cursor_position(&self.content);
        self.selection = self.cursors.selection_range();
    }

    /// Check if multi-cursor mode is active (more than one cursor).
    pub fn has_multiple_cursors(&self) -> bool {
        !self.cursors.is_single()
    }

    /// Get the number of active cursors.
    pub fn cursor_count(&self) -> usize {
        self.cursors.len()
    }

    /// Clear all cursors and reset to a single cursor at the given position.
    pub fn clear_to_single_cursor(&mut self, pos: usize) {
        self.cursors.set_single(Selection::cursor(pos));
        self.sync_cursor_from_primary();
    }

    /// Clear all cursors and reset to a single cursor at the primary position.
    pub fn exit_multi_cursor_mode(&mut self) {
        let primary_pos = self.cursors.primary().head;
        self.clear_to_single_cursor(primary_pos);
    }

    /// Add a new cursor at the given character position.
    pub fn add_cursor(&mut self, pos: usize) {
        self.cursors.add(Selection::cursor(pos));
        self.sync_cursor_from_primary();
    }

    /// Add a new selection (for Ctrl+D next occurrence).
    pub fn add_selection(&mut self, anchor: usize, head: usize) {
        self.cursors.add(Selection::new(anchor, head));
        self.sync_cursor_from_primary();
    }

    /// Set the primary cursor/selection (for single cursor operations).
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursors.set_single(Selection::cursor(pos));
        self.sync_cursor_from_primary();
    }

    /// Set the primary selection (for single selection operations).
    pub fn set_selection(&mut self, anchor: usize, head: usize) {
        self.cursors.set_single(Selection::new(anchor, head));
        self.sync_cursor_from_primary();
    }

    /// Update cursor state from egui's TextEdit cursor range.
    ///
    /// This syncs the multi-cursor state with egui's single-cursor model.
    /// When multi-cursor editing is active, this only updates the primary cursor.
    pub fn update_cursor_from_egui(&mut self, primary: usize, secondary: usize) {
        if self.cursors.is_single() {
            // Single cursor mode: sync from egui
            if primary == secondary {
                self.cursors.set_single(Selection::cursor(primary));
            } else {
                // egui uses primary as cursor position, secondary as anchor
                self.cursors.set_single(Selection::new(secondary, primary));
            }
        } else {
            // Multi-cursor mode: only update primary cursor, preserve others
            let primary_sel = self.cursors.primary_mut();
            if primary == secondary {
                primary_sel.anchor = primary;
                primary_sel.head = primary;
            } else {
                primary_sel.anchor = secondary;
                primary_sel.head = primary;
            }
        }
        self.sync_cursor_from_primary();
    }

    /// Find the next occurrence of the given text after the specified position.
    /// Returns (start, end) character indices if found.
    pub fn find_next_occurrence(
        &self,
        search_text: &str,
        after_pos: usize,
    ) -> Option<(usize, usize)> {
        if search_text.is_empty() {
            return None;
        }

        // Search from after_pos to end
        if let Some(rel_pos) = self.content[after_pos..].find(search_text) {
            let start = after_pos + rel_pos;
            let end = start + search_text.len();
            return Some((start, end));
        }

        // Wrap around: search from beginning to after_pos
        if let Some(rel_pos) = self.content[..after_pos].find(search_text) {
            let end = rel_pos + search_text.len();
            return Some((rel_pos, end));
        }

        None
    }

    /// Get the text under the primary cursor (word at cursor if no selection).
    pub fn get_primary_selection_text(&self) -> Option<String> {
        let primary = self.cursors.primary();

        if primary.is_selection() {
            // Return selected text
            let (start, end) = primary.range();
            if end <= self.content.len() {
                return Some(self.content[start..end].to_string());
            }
        } else {
            // No selection: find word at cursor
            return self.word_at_position(primary.head);
        }

        None
    }

    /// Get the word at the given character position.
    fn word_at_position(&self, pos: usize) -> Option<String> {
        if self.content.is_empty() || pos > self.content.len() {
            return None;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let char_pos = pos.min(chars.len().saturating_sub(1));

        // Find word boundaries
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        // Check if we're on a word character
        if char_pos < chars.len() && !is_word_char(chars[char_pos]) {
            return None;
        }

        // Find start of word
        let mut start = char_pos;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        // Find end of word
        let mut end = char_pos;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    /// Get the byte range of the word at the given character position.
    pub fn word_range_at_position(&self, pos: usize) -> Option<(usize, usize)> {
        if self.content.is_empty() || pos > self.content.len() {
            return None;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let char_pos = pos.min(chars.len().saturating_sub(1));

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        // Check if we're on a word character
        if char_pos < chars.len() && !is_word_char(chars[char_pos]) {
            return None;
        }

        // Find start of word
        let mut start = char_pos;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        // Find end of word
        let mut end = char_pos;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }

        if start < end {
            Some((start, end))
        } else {
            None
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Transient Highlight (Search Result Navigation)
    // ─────────────────────────────────────────────────────────────────────────

    /// Set a transient highlight for search result navigation.
    ///
    /// This highlight is temporary and will be cleared on scroll, edit, or click.
    /// The programmatic scroll that positions the match is ignored.
    pub fn set_transient_highlight(&mut self, start: usize, end: usize) {
        self.transient_highlight.set(start, end);
    }

    /// Clear the transient highlight.
    pub fn clear_transient_highlight(&mut self) {
        self.transient_highlight.clear();
    }

    /// Check if a transient highlight is active.
    pub fn has_transient_highlight(&self) -> bool {
        self.transient_highlight.is_active()
    }

    /// Get the transient highlight range if active.
    pub fn transient_highlight_range(&self) -> Option<(usize, usize)> {
        self.transient_highlight.range()
    }

    /// Notify that a scroll event occurred.
    ///
    /// This will clear the transient highlight unless it's the first scroll
    /// after the highlight was set (the programmatic scroll to position the match).
    pub fn on_scroll_event(&mut self) {
        self.transient_highlight.on_scroll();
    }

    /// Notify that an edit event occurred. Clears the transient highlight.
    pub fn on_edit_event(&mut self) {
        self.transient_highlight.on_edit();
    }

    /// Notify that a click event occurred. Clears the transient highlight.
    pub fn on_click_event(&mut self) {
        self.transient_highlight.on_click();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Code Folding
    // ─────────────────────────────────────────────────────────────────────────

    /// Update fold regions for this tab using the detection algorithm.
    ///
    /// This should be called when content changes significantly or when
    /// folding settings change. The current collapsed states are preserved
    /// where possible.
    pub fn update_folds(
        &mut self,
        fold_headings: bool,
        fold_code_blocks: bool,
        fold_lists: bool,
        fold_indentation: bool,
    ) {
        use crate::editor::folding::detect_fold_regions;

        // Remember currently collapsed fold positions
        let collapsed_lines: std::collections::HashSet<usize> = self
            .fold_state
            .regions()
            .iter()
            .filter(|r| r.collapsed)
            .map(|r| r.start_line)
            .collect();

        // Detect new fold regions
        let mut new_state = detect_fold_regions(
            &self.content,
            self.file_type,
            fold_headings,
            fold_code_blocks,
            fold_lists,
            fold_indentation,
        );

        // Restore collapsed state for matching start lines
        for region in new_state.regions_mut() {
            if collapsed_lines.contains(&region.start_line) {
                region.collapsed = true;
            }
        }

        self.fold_state = new_state;
    }

    /// Mark fold state as needing recomputation.
    pub fn mark_folds_dirty(&mut self) {
        self.fold_state.mark_dirty();
    }

    /// Check if fold state needs recomputation.
    pub fn folds_dirty(&self) -> bool {
        self.fold_state.is_dirty()
    }

    /// Toggle the fold at the given line.
    ///
    /// Returns true if a fold was toggled.
    pub fn toggle_fold_at_line(&mut self, line: usize) -> bool {
        self.fold_state.toggle_at_line(line)
    }

    /// Check if a line is hidden by a fold.
    pub fn is_line_folded(&self, line: usize) -> bool {
        self.fold_state.is_line_hidden(line)
    }

    /// Reveal a line by expanding any fold that hides it.
    pub fn reveal_line(&mut self, line: usize) -> bool {
        self.fold_state.reveal_line(line)
    }

    /// Get lines that should show fold indicators.
    ///
    /// Returns (line, is_collapsed) for each fold start line.
    pub fn fold_indicator_lines(&self) -> Vec<(usize, bool)> {
        self.fold_state.fold_indicator_lines()
    }

    /// Fold all regions.
    pub fn fold_all(&mut self) {
        self.fold_state.fold_all();
    }

    /// Unfold all regions.
    pub fn unfold_all(&mut self) {
        self.fold_state.unfold_all();
    }

    /// Fold all headings.
    pub fn fold_all_headings(&mut self) {
        self.fold_state
            .fold_all_of_kind(|k| matches!(k, FoldKind::Heading(_)));
    }

    /// Fold all code blocks.
    pub fn fold_all_code_blocks(&mut self) {
        self.fold_state
            .fold_all_of_kind(|k| matches!(k, FoldKind::CodeBlock));
    }

    /// Get the number of collapsed folds.
    pub fn collapsed_fold_count(&self) -> usize {
        self.fold_state.collapsed_count()
    }

    /// Get total hidden line count from folds.
    pub fn hidden_line_count(&self) -> usize {
        self.fold_state.hidden_line_count()
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self::new(0) // Defaults to Raw view mode and Markdown file type
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI State
// ─────────────────────────────────────────────────────────────────────────────

/// Payload stashed while the code-execution consent dialog is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingCodeRun {
    pub code: String,
    pub language: String,
    pub cwd: Option<PathBuf>,
    pub timeout_secs: u32,
    pub block_id: egui::Id,
}

/// UI-related state flags.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Runtime parent relationship for tabs opened from links in another Ferrite tab.
    pub navigation_parents: HashMap<usize, usize>,
    /// Whether the settings panel is open
    pub show_settings: bool,
    /// Whether the file browser/open dialog is active
    pub show_file_dialog: bool,
    /// Whether the "save as" dialog is active
    pub show_save_as_dialog: bool,
    /// Whether the about dialog is open
    pub show_about: bool,
    /// Whether a confirmation dialog is open (e.g., unsaved changes)
    pub show_confirm_dialog: bool,
    /// Message for the confirmation dialog
    pub confirm_dialog_message: String,
    /// Pending action after confirmation
    pub pending_action: Option<PendingAction>,
    /// Status bar message (deprecated, use toast_message instead)
    pub status_message: Option<String>,
    /// Whether the find/replace panel is open
    pub show_find_replace: bool,
    /// Find/replace state
    pub find_state: crate::editor::FindState,
    /// Whether to scroll to the current match (set when navigating)
    pub scroll_to_match: bool,
    /// Whether a find search is pending (debounced)
    pub find_search_pending: bool,
    /// When the find search was last requested (for debouncing)
    pub find_search_requested_at: Option<std::time::Instant>,
    /// Whether to show error modal
    pub show_error_modal: bool,
    /// Error message for modal
    pub error_message: String,
    /// Whether to show the portal error dialog (Linux xdg-desktop-portal missing)
    pub show_portal_error_dialog: bool,
    /// Portal error message with installation instructions
    pub portal_error_message: String,
    /// Portal error command to copy (install command)
    pub portal_error_command: String,
    /// Temporary toast message (shown in center of status bar)
    pub toast_message: Option<String>,
    /// When the toast message should expire (as seconds since app start)
    pub toast_expires_at: Option<f64>,
    /// Whether the recent files popup is open
    pub show_recent_files_popup: bool,
    /// Whether the recent folders popup is open
    pub show_recent_folders_popup: bool,
    /// Whether Zen Mode is enabled (distraction-free writing)
    pub zen_mode: bool,
    /// Go to Line dialog state (None = closed)
    pub go_to_line_dialog: Option<crate::ui::GoToLineDialog>,
    /// Current Vim mode label for status bar display (None = Vim disabled).
    pub vim_mode_indicator: Option<&'static str>,
    /// Whether the HTML export options dialog is open.
    pub show_html_export_dialog: bool,
    /// Whether the PDF export options dialog is open.
    pub show_pdf_export_dialog: bool,
    /// First-run / consent modal for running fenced code from markdown preview.
    pub show_code_execution_consent_dialog: bool,
    /// Queued run for **Enable & run** after consent.
    pub pending_code_run: Option<PendingCodeRun>,
    /// When true, focus the Cancel button once when the consent dialog opens.
    pub code_execution_consent_focus_cancel: bool,
    /// Rename a pathless document tab: `(tab_index, text buffer)`.
    pub rename_untitled_tab: Option<(usize, String)>,
}

/// True when a persisted session title matches a special tab (Settings, About, Welcome).
///
/// Legacy sessions incorrectly stored special tabs as pathless documents; skip those titles
/// when restoring custom untitled labels so they are not applied to document tabs.
fn is_reserved_special_tab_display_title(display_title: &str) -> bool {
    let s = display_title.trim().trim_end_matches('*').trim();
    [
        SpecialTabKind::Settings,
        SpecialTabKind::About,
        SpecialTabKind::Welcome,
    ]
    .into_iter()
    .any(|kind| {
        let with_icon = format!("{} {}", kind.icon(), kind.title());
        s == with_icon || s == kind.title()
    })
}

/// Parse persisted session title into an optional custom untitled tab label.
fn persisted_untitled_label_from_session(display_title: &str) -> Option<String> {
    let s = display_title.trim().trim_end_matches('*').trim();
    if s.is_empty()
        || s.eq_ignore_ascii_case("untitled")
        || is_reserved_special_tab_display_title(s)
    {
        None
    } else {
        Some(s.to_string())
    }
}

/// Whether a save confirmation applies to closing one tab or exiting the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePromptContext {
    /// Closing a single tab (tab strip, Ctrl+W, etc.)
    TabClose,
    /// App exit and other checks via [`AppState::has_unsaved_changes`]
    AppExit,
}

/// Actions that may need confirmation before execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    /// Close a specific tab
    CloseTab(usize),
    /// Close all tabs
    CloseAllTabs,
    /// Close every tab except the tab with this runtime ID.
    CloseOtherTabs(usize),
    /// Exit the application
    Exit,
    /// Open a new file (replacing current)
    OpenFile(PathBuf),
    /// Create a new document
    NewDocument,
}

// ─────────────────────────────────────────────────────────────────────────────
// Application State
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Session Content Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Snapshot of a recovered-vs-disk mismatch surfaced via the conflict banner.
///
/// Stored in [`AppState::recovery_conflicts`] keyed by `tab_id` while a tab
/// has both a recovered buffer (already applied) and a meaningfully different
/// on-disk version. The user picks one of two actions in the banner:
///
/// * **Keep Recovered** — drop the conflict entry; the buffer is unchanged
///   so the tab simply stays modified relative to disk and the user can
///   save manually.
/// * **Reload from Disk** — replace the tab's buffer with `on_disk_content`
///   and mark the tab saved (no longer modified).
///
/// See task 106.5 (hardened session recovery banner).
#[derive(Debug, Clone)]
pub struct RecoveryConflict {
    /// The buffer that was applied to the tab during session restore.
    pub recovered_content: String,
    /// The current on-disk content captured at restore time. Used to
    /// replace the buffer if the user picks `Reload from Disk`; also kept
    /// so a future enhancement can render a diff.
    pub on_disk_content: String,
}

/// Result of resolving tab content from various sources.
/// Contains the content and optional encoding information for files loaded from disk.
#[derive(Debug)]
enum ResolvedContent {
    /// Content recovered from crash recovery (already UTF-8).
    ///
    /// Either the recovery file matched the tab's identity exactly (path +
    /// hash), or it is a legacy pre-task-106 file with no identity to verify.
    Recovered(String),
    /// Content recovered from crash recovery whose buffer differs from the
    /// current on-disk content even though the recovery file's identity
    /// (path + `original_content_hash`) matched at restore time.
    ///
    /// The caller applies `content` to the tab and uses `on_disk_content` to
    /// surface a non-blocking conflict banner so the user can pick
    /// `Keep Recovered` (do nothing) or `Reload from Disk` (replace with the
    /// disk version) instead of silently keeping one side. See task 106.5.
    RecoveredWithDiskDivergence {
        content: String,
        on_disk_content: String,
    },
    /// Content loaded from disk with encoding detection
    FromDisk {
        content: String,
        original_bytes: Vec<u8>,
        encoding: &'static str,
        had_bom: bool,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Backlink Index
// ─────────────────────────────────────────────────────────────────────────────

/// A single backlink entry: a file that references the target file.
#[derive(Debug, Clone)]
pub struct BacklinkEntry {
    /// Absolute path to the file that contains the link
    pub source_path: PathBuf,
    /// Display name for the source file (filename without extension)
    pub display_name: String,
}

/// In-memory index mapping file names to the files that reference them.
///
/// For small workspaces (≤50 files), backlinks are scanned on demand when
/// the active tab changes. For larger workspaces, the index is built once
/// on workspace load and updated incrementally on file save events.
#[derive(Debug, Default)]
pub struct BacklinkIndex {
    /// Map from lowercase filename (without extension) → list of files that link to it.
    /// The key is normalized (lowercase, no extension) to match wikilink resolution.
    index: HashMap<String, Vec<BacklinkEntry>>,
    /// Number of files in the workspace when the index was last built.
    /// Used to decide between on-demand scanning vs cached index.
    pub file_count: usize,
    /// Whether the full index has been built (for large workspaces).
    pub is_built: bool,
}

impl BacklinkIndex {
    /// Create a new empty backlink index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the entire index.
    pub fn clear(&mut self) {
        self.index.clear();
        self.file_count = 0;
        self.is_built = false;
    }

    /// Get backlinks for a given filename.
    ///
    /// The filename is normalized to lowercase without extension for matching.
    pub fn get_backlinks(&self, filename: &str) -> Vec<BacklinkEntry> {
        let key = normalize_filename(filename);
        self.index.get(&key).cloned().unwrap_or_default()
    }

    /// Build the full index by scanning all workspace files.
    ///
    /// This reads every markdown file in the workspace and extracts wikilinks
    /// and standard markdown links, building a reverse mapping.
    pub fn build_from_files(&mut self, files: &[PathBuf]) {
        self.index.clear();
        self.file_count = files.len();

        let md_files: Vec<&PathBuf> = files
            .iter()
            .filter(|f| {
                f.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
                    .unwrap_or(false)
            })
            .collect();

        for file_path in &md_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                let links = extract_links_from_content(&content);
                let source_display = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                for link_target in links {
                    let key = normalize_filename(&link_target);
                    let entry = BacklinkEntry {
                        source_path: file_path.to_path_buf(),
                        display_name: source_display.clone(),
                    };
                    self.index.entry(key).or_default().push(entry);
                }
            }
        }

        self.is_built = true;
        log::debug!(
            "Backlink index built: {} files scanned, {} targets indexed",
            md_files.len(),
            self.index.len()
        );
    }

    /// Incrementally update the index for a single file that was saved.
    ///
    /// Removes all old entries from this source file, then re-scans it.
    pub fn update_file(&mut self, file_path: &Path) {
        // Remove old entries from this source
        for entries in self.index.values_mut() {
            entries.retain(|e| e.source_path != file_path);
        }
        // Remove empty keys
        self.index.retain(|_, v| !v.is_empty());

        // Re-scan the file
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let links = extract_links_from_content(&content);
            let source_display = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            for link_target in links {
                let key = normalize_filename(&link_target);
                let entry = BacklinkEntry {
                    source_path: file_path.to_path_buf(),
                    display_name: source_display.clone(),
                };
                self.index.entry(key).or_default().push(entry);
            }
        }
    }

    /// Scan a subset of files on demand (for small workspaces or single-file mode).
    ///
    /// Returns backlinks for the given target filename by scanning the provided files.
    pub fn scan_on_demand(
        target_filename: &str,
        files: &[PathBuf],
        target_path: Option<&Path>,
    ) -> Vec<BacklinkEntry> {
        let target_key = normalize_filename(target_filename);
        let mut results = Vec::new();

        for file_path in files {
            // Skip non-markdown files
            let is_md = file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
                .unwrap_or(false);
            if !is_md {
                continue;
            }

            // Skip the target file itself
            if let Some(tp) = target_path {
                if file_path == tp {
                    continue;
                }
            }

            if let Ok(content) = std::fs::read_to_string(file_path) {
                let links = extract_links_from_content(&content);
                for link in &links {
                    if normalize_filename(link) == target_key {
                        let display = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        results.push(BacklinkEntry {
                            source_path: file_path.clone(),
                            display_name: display,
                        });
                        break; // One match per file is enough
                    }
                }
            }
        }

        results
    }
}

/// Normalize a filename for backlink matching.
/// Removes `.md`/`.markdown` extension (case-insensitive) and converts to lowercase.
fn normalize_filename(name: &str) -> String {
    let name = name.trim();
    let lower = name.to_lowercase();
    let without_ext = lower
        .strip_suffix(".md")
        .or_else(|| lower.strip_suffix(".markdown"))
        .unwrap_or(&lower);
    without_ext.to_string()
}

/// Extract all link targets from markdown content.
///
/// Detects:
/// - `[[target]]` wikilinks
/// - `[[target|display]]` wikilinks with display text
/// - `[text](target.md)` standard markdown links to local .md files
fn extract_links_from_content(content: &str) -> Vec<String> {
    let mut targets = Vec::new();

    // Extract wikilinks: [[target]] and [[target|display]]
    let mut remaining = content;
    while let Some(open) = remaining.find("[[") {
        let after_open = &remaining[open + 2..];
        if let Some(close) = after_open.find("]]") {
            let inner = &after_open[..close];
            // Don't allow newlines inside wikilinks
            if !inner.contains('\n') && !inner.is_empty() {
                // Extract target (before | if present)
                let target = if let Some(pipe) = inner.find('|') {
                    inner[..pipe].trim()
                } else {
                    inner.trim()
                };
                if !target.is_empty() {
                    targets.push(target.to_string());
                }
            }
            remaining = &after_open[close + 2..];
        } else {
            remaining = after_open;
        }
    }

    // Extract standard markdown links: [text](target.md)
    // Only capture local .md file references (not http/https URLs)
    remaining = content;
    while let Some(open_paren) = remaining.find("](") {
        let after_paren = &remaining[open_paren + 2..];
        if let Some(close_paren) = after_paren.find(')') {
            let url = after_paren[..close_paren].trim();
            // Only match local .md links (not URLs)
            if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with('#')
                && (url.ends_with(".md") || url.ends_with(".markdown"))
            {
                // Extract just the filename from the path
                let filename = Path::new(url)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(url);
                targets.push(filename.to_string());
            }
            remaining = &after_paren[close_paren + 1..];
        } else {
            break;
        }
    }

    targets
}

#[derive(Debug)]
pub struct AppState {
    /// All open tabs
    tabs: Vec<Tab>,
    /// Index of the currently active tab
    active_tab_index: usize,
    /// Next tab ID (for unique identification)
    next_tab_id: usize,
    /// User settings (loaded from config)
    pub settings: Settings,
    /// UI-related state
    pub ui: UiState,
    /// Whether settings have been modified and need saving
    settings_dirty: bool,
    /// Current application mode (single file or workspace)
    pub app_mode: AppMode,
    /// Active workspace (only populated when app_mode is Workspace)
    pub workspace: Option<Workspace>,
    /// File system watcher for workspace mode
    workspace_watcher: Option<WorkspaceWatcher>,
    /// Pending file events from the watcher that need to be processed
    pub pending_file_events: Vec<WorkspaceEvent>,
    /// Git integration service
    pub git_service: GitService,
    /// Backlink index for tracking which files link to which
    pub backlink_index: BacklinkIndex,
    /// Language server processes and channels (background thread).
    pub lsp: LspManager,
    /// Aggregated LSP diagnostics keyed by file path.
    pub diagnostics: DiagnosticMap,
    /// Optional toast message to display on the first frame (for startup errors).
    pub pending_toast: Option<String>,
    /// Active recovery-vs-disk conflicts keyed by `tab_id`.
    ///
    /// Populated during [`Self::restore_from_session_result`] when an
    /// identity-validated recovery file is applied but its buffer differs
    /// from the current disk content. The active tab's banner reads from
    /// here and clears the entry when the user picks `Keep Recovered` or
    /// `Reload from Disk` (task 106.5).
    pub recovery_conflicts: HashMap<usize, RecoveryConflict>,
}

impl AppState {
    /// Create a new AppState with settings loaded from config.
    ///
    /// This initializes the application state by:
    /// 1. Loading settings from the config file (with graceful fallback to defaults)
    /// 2. Restoring previously open tabs from session data (if available)
    /// 3. Creating an initial empty tab if no tabs were restored
    /// 4. Setting up default UI state
    pub fn new() -> Self {
        let settings = load_config();
        info!("AppState initialized with settings");
        debug!(
            "Theme: {:?}, View mode: {:?}",
            settings.theme, settings.view_mode
        );

        let mut state = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            settings,
            ui: UiState::default(),
            settings_dirty: false,
            app_mode: AppMode::default(),
            workspace: None,
            workspace_watcher: None,
            pending_file_events: Vec::new(),
            git_service: GitService::new(),
            backlink_index: BacklinkIndex::new(),
            lsp: LspManager::new(),
            diagnostics: DiagnosticMap::new(),
            pending_toast: None,
            recovery_conflicts: HashMap::new(),
        };

        // Try to restore tabs from previous session
        state.restore_session_tabs();

        // If no tabs were restored, create an initial empty tab
        if state.tabs.is_empty() {
            state.new_tab();
        }

        state
    }

    /// Restore tabs from the previous session.
    ///
    /// This attempts to restore tabs from `settings.last_open_tabs`.
    /// Files that no longer exist are skipped with a warning.
    /// Unsaved tabs (no path) are not restored.
    ///
    /// If `settings.restore_session` is false, this method returns early
    /// without restoring any tabs (caller will create an empty tab).
    fn restore_session_tabs(&mut self) {
        // Check if session restore is enabled
        if !self.settings.restore_session {
            debug!("Session restore disabled in settings, skipping tab restoration");
            return;
        }

        let tab_infos: Vec<TabInfo> = self.settings.last_open_tabs.clone();
        let saved_active_index = self.settings.active_tab_index;

        if tab_infos.is_empty() {
            debug!("No tabs to restore from previous session");
            return;
        }

        info!("Restoring {} tab(s) from previous session", tab_infos.len());

        let auto_save_default = self.settings.auto_save_enabled_default;

        for tab_info in &tab_infos {
            if let Some(path) = &tab_info.path {
                let file_type = FileType::from_path(path);

                // Viewer tabs: restore as viewer instead of document
                if file_type.is_image() {
                    match self.open_image_tab(path.clone(), false) {
                        Ok(_) => debug!("Restored image viewer tab: {}", path.display()),
                        Err(e) => warn!("Could not restore image tab '{}': {}", path.display(), e),
                    }
                    continue;
                }
                if file_type.is_pdf() {
                    match self.open_pdf_tab(path.clone(), false) {
                        Ok(_) => debug!("Restored PDF viewer tab: {}", path.display()),
                        Err(e) => warn!("Could not restore PDF tab '{}': {}", path.display(), e),
                    }
                    continue;
                }

                // Regular document tabs: read bytes for encoding detection
                match std::fs::read(path) {
                    Ok(bytes) => {
                        let tab = Tab::from_tab_info_with_bytes(
                            self.next_tab_id,
                            tab_info,
                            bytes,
                            auto_save_default,
                        );
                        let encoding = tab.current_encoding;
                        self.next_tab_id += 1;
                        self.tabs.push(tab);
                        debug!("Restored tab: {} (encoding: {})", path.display(), encoding);
                    }
                    Err(e) => {
                        warn!(
                            "Could not restore tab for '{}': {}. File may have been moved or deleted.",
                            path.display(),
                            e
                        );
                    }
                }
            } else {
                // Skip tabs without a path (unsaved documents)
                debug!("Skipping unsaved tab from session restore");
            }
        }

        // Restore active tab index (clamped to valid range)
        if !self.tabs.is_empty() {
            self.active_tab_index = saved_active_index.min(self.tabs.len() - 1);
            info!(
                "Restored {} tab(s), active tab index: {}",
                self.tabs.len(),
                self.active_tab_index
            );
        }
    }

    /// Whether any non-special document tabs are open (saved files, scratch notes, viewers).
    pub fn has_open_documents(&self) -> bool {
        if self.is_workspace_mode() {
            return true;
        }
        self.tabs
            .iter()
            .any(|t| !t.is_special() && !t.is_empty_untitled())
    }

    /// True when the Welcome tab should appear on startup (no CLI paths).
    pub fn should_show_welcome_on_empty_launch(&self) -> bool {
        self.settings.show_welcome_on_empty_launch && !self.has_open_documents()
    }

    /// Remove default empty untitled placeholders before showing Welcome.
    fn remove_empty_untitled_tabs(&mut self) {
        let mut i = 0;
        while i < self.tabs.len() {
            if self.tabs[i].is_empty_untitled() {
                self.tabs.remove(i);
                if self.active_tab_index > i {
                    self.active_tab_index -= 1;
                } else if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
                    self.active_tab_index = self.tabs.len() - 1;
                }
            } else {
                i += 1;
            }
        }
    }

    /// Open the Welcome tab on empty launch, or activate it if it already exists.
    pub fn open_welcome_on_empty_launch(&mut self) {
        if !self.should_show_welcome_on_empty_launch() {
            return;
        }
        self.remove_empty_untitled_tabs();
        self.show_welcome_tab();
    }

    /// Open the Welcome tab, or activate it if it already exists.
    pub fn show_welcome_tab(&mut self) {
        // If Welcome tab already exists, just activate it.
        if let Some(i) = self
            .tabs
            .iter()
            .position(|t| matches!(&t.kind, TabKind::Special(SpecialTabKind::Welcome)))
        {
            self.active_tab_index = i;
            return;
        }

        // Otherwise create it (this should also set active, but we’ll be safe)
        self.open_special_tab(SpecialTabKind::Welcome);
    }

    /// Create AppState with custom settings (useful for testing).
    ///
    /// This also restores tabs from `settings.last_open_tabs` if available.
    pub fn with_settings(settings: Settings) -> Self {
        let mut state = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            settings,
            ui: UiState::default(),
            settings_dirty: false,
            app_mode: AppMode::default(),
            workspace: None,
            workspace_watcher: None,
            pending_file_events: Vec::new(),
            git_service: GitService::new(),
            backlink_index: BacklinkIndex::new(),
            lsp: LspManager::new(),
            diagnostics: DiagnosticMap::new(),
            pending_toast: None,
            recovery_conflicts: HashMap::new(),
        };

        // Try to restore tabs from session data
        state.restore_session_tabs();

        // If no tabs were restored, create an empty tab
        if state.tabs.is_empty() {
            state.new_tab();
        }

        state
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Warm per-frame caches (is_modified) for all tabs.
    /// Call once per frame before reading tab titles or is_modified via &self.
    pub fn warm_tab_caches(&mut self) {
        for tab in &mut self.tabs {
            tab.is_modified_cached();
        }
    }

    /// Get the number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get all tabs (read-only).
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get the active tab index.
    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }

    /// Get a reference to the active tab.
    ///
    /// Returns `None` if there are no tabs.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Get a mutable reference to the active tab.
    ///
    /// Returns `None` if there are no tabs.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Get a tab by index.
    pub fn tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Get a mutable tab by index.
    pub fn tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    /// Find a tab by its unique ID.
    pub fn tab_by_id(&self, tab_id: usize) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == tab_id)
    }

    /// Find a tab by its unique ID and return a mutable reference.
    pub fn tab_by_id_mut(&mut self, tab_id: usize) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == tab_id)
    }

    /// View mode and split ratio to use when opening a file (saved per path, else global default).
    pub fn opening_view_prefs_for_path(&self, path: &std::path::Path) -> (ViewMode, f32) {
        self.settings
            .tab_info_for_path(path)
            .map(|i| (i.view_mode, i.split_ratio))
            .unwrap_or((self.settings.default_view_mode, 0.5))
    }

    /// Create a new empty tab and make it active.
    ///
    /// Returns the index of the new tab.
    /// Applies auto_save_enabled_default and default_view_mode.
    pub fn new_tab(&mut self) -> usize {
        let auto_save_default = self.settings.auto_save_enabled_default;
        let default_view_mode = self.settings.default_view_mode;
        let tab = Tab::new_with_settings(self.next_tab_id, auto_save_default, default_view_mode);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        debug!(
            "Created new tab at index {} (auto-save: {}, view_mode: {:?})",
            self.active_tab_index, auto_save_default, default_view_mode
        );
        self.active_tab_index
    }

    /// Open or focus a special tab (settings, about, etc.).
    ///
    /// If a tab of this kind already exists, it will be focused instead of
    /// creating a duplicate. Returns the index of the (new or existing) tab.
    pub fn open_special_tab(&mut self, special_kind: SpecialTabKind) -> usize {
        // Check if a tab of this kind already exists
        if let Some(index) = self
            .tabs
            .iter()
            .position(|t| matches!(&t.kind, TabKind::Special(k) if *k == special_kind))
        {
            self.active_tab_index = index;
            debug!(
                "Focused existing special tab {:?} at index {}",
                special_kind, index
            );
            return index;
        }

        // Create a new special tab
        let mut tab = Tab::new(self.next_tab_id);
        tab.kind = TabKind::Special(special_kind);
        tab.needs_focus = false; // Special tabs don't need editor focus
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        debug!(
            "Created special tab {:?} at index {}",
            special_kind, self.active_tab_index
        );
        self.active_tab_index
    }

    /// Open an image file in an image viewer tab.
    ///
    /// If the same image is already open, focuses that tab instead.
    /// Returns the index of the (new or existing) tab.
    pub fn open_image_tab(&mut self, path: PathBuf, focus: bool) -> Result<usize, std::io::Error> {
        if let Some(index) = self.find_tab_by_path(&path) {
            if focus {
                self.active_tab_index = index;
            }
            return Ok(index);
        }

        let metadata = std::fs::metadata(&path)?;
        let file_size = metadata.len();
        let format_label = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "Unknown".to_string());

        let viewer_state = ImageViewerState {
            zoom: 1.0,
            dimensions: None,
            file_size,
            format_label,
            fitted: false,
            parent_tab_id: None,
        };

        let mut tab = Tab::new(self.next_tab_id);
        tab.kind = TabKind::ImageViewer(viewer_state);
        tab.path = Some(path.clone());
        tab.needs_focus = false;
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
            info!("Opened image viewer: {}", path.display());
        }
        Ok(new_index)
    }

    /// Open a PDF file in a PDF viewer tab.
    ///
    /// If the same PDF is already open, focuses that tab instead.
    /// Returns the index of the (new or existing) tab.
    pub fn open_pdf_tab(&mut self, path: PathBuf, focus: bool) -> Result<usize, std::io::Error> {
        if let Some(index) = self.find_tab_by_path(&path) {
            if focus {
                self.active_tab_index = index;
            }
            return Ok(index);
        }

        let metadata = std::fs::metadata(&path)?;
        let file_size = metadata.len();

        // Try to load PDF and get page count
        let bytes = std::fs::read(&path)?;
        let pdf_data = std::sync::Arc::new(bytes);
        let (page_count, error) = match hayro::hayro_interpret::hayro_syntax::Pdf::new(pdf_data) {
            Ok(pdf) => (pdf.pages().len(), None),
            Err(e) => (0, Some(format!("{:?}", e))),
        };

        let viewer_state = PdfViewerState {
            current_page: 0,
            page_count,
            zoom: 1.0,
            fitted: false,
            file_size,
            error,
            ..Default::default()
        };

        let mut tab = Tab::new(self.next_tab_id);
        tab.kind = TabKind::PdfViewer(viewer_state);
        tab.path = Some(path.clone());
        tab.needs_focus = false;
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
            info!("Opened PDF viewer: {}", path.display());
        }
        Ok(new_index)
    }

    /// Open a file in a new tab.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    /// Pass `app_time` when available so a non-blocking performance warning toast
    /// can be shown for files larger than 10 MB.
    pub fn open_file(
        &mut self,
        path: PathBuf,
        app_time: Option<f64>,
    ) -> Result<usize, std::io::Error> {
        self.open_file_with_focus(path, true, app_time)
    }

    /// Open a file in a new tab with optional focus control.
    ///
    /// If `focus` is true, the new tab becomes active. If false, the file opens
    /// in the background without switching tabs.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    /// Pass `app_time` when available so a non-blocking performance warning toast
    /// can be shown for files larger than 10 MB.
    pub fn open_file_with_focus(
        &mut self,
        path: PathBuf,
        focus: bool,
        app_time: Option<f64>,
    ) -> Result<usize, std::io::Error> {
        // Check if file is already open
        if let Some(index) = self.find_tab_by_path(&path) {
            if focus {
                self.active_tab_index = index;
                info!("File already open, switching to tab {}", index);
            } else {
                info!("File already open at tab {} (no focus change)", index);
            }
            return Ok(index);
        }

        // Intercept image files before binary detection — open as image viewer
        if FileType::from_path(&path).is_image() {
            return self.open_image_tab(path, focus);
        }

        // Intercept PDF files before binary detection — open as PDF viewer
        if FileType::from_path(&path).is_pdf() {
            return self.open_pdf_tab(path, focus);
        }

        // Show non-blocking performance warning for large files before loading
        if let (Some(time), Ok(meta)) = (app_time, std::fs::metadata(&path)) {
            let len = meta.len();
            if len > LARGE_FILE_THRESHOLD_BYTES {
                let size_mb = len / (1024 * 1024);
                self.show_toast(
                    t!(
                        "notification.large_file_performance",
                        size = size_mb.to_string()
                    )
                    .to_string(),
                    time,
                    3.0,
                );
            }
        }

        // Read file as bytes for encoding detection
        let bytes = std::fs::read(&path)?;

        // Check for binary files - we can't edit binary data as text
        if is_binary_content(&bytes) {
            let reason = binary_detection_reason(&bytes);
            log::warn!("Cannot open binary file: {} ({})", path.display(), reason);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Binary file detected ({}). Use a specialized tool to edit this file.",
                    reason
                ),
            ));
        }

        // Create new tab: restore per-file view mode when known, else use global default
        let auto_save_default = self.settings.auto_save_enabled_default;
        let (view_mode, split_ratio) = self.opening_view_prefs_for_path(&path);
        let mut tab = Tab::with_file_bytes_and_settings(
            self.next_tab_id,
            path.clone(),
            bytes,
            auto_save_default,
            view_mode,
        );
        tab.split_ratio = split_ratio;

        // If view mode is Split but file type doesn't support it, fall back to Raw
        if tab.view_mode == ViewMode::Split && !tab.file_type().supports_split() {
            tab.view_mode = ViewMode::Raw;
        }

        let detected_encoding = tab.current_encoding;
        let opened_view_mode = tab.view_mode;
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
            info!(
                "Opened file: {} (encoding: {}, auto-save: {}, view_mode: {:?})",
                path.display(),
                detected_encoding,
                auto_save_default,
                opened_view_mode
            );
        } else {
            info!(
                "Opened file: {} (encoding: {}, in background, auto-save: {}, view_mode: {:?})",
                path.display(),
                detected_encoding,
                auto_save_default,
                opened_view_mode
            );
        }

        // Update recent files and save immediately for persistence
        self.settings.add_recent_file(path.clone());
        self.settings_dirty = true;
        // Save immediately to survive app crashes/force-kills
        self.save_settings_if_dirty();

        Ok(new_index)
    }

    /// Create a loading-placeholder tab for a large file.
    ///
    /// The caller is responsible for spawning a background thread to read the
    /// file and calling `finish_loading()` on the tab when done.
    /// Returns `(tab_index, tab_id)` so the caller can track the loading task.
    pub fn open_file_loading(
        &mut self,
        path: PathBuf,
        file_size: u64,
        focus: bool,
    ) -> (usize, usize) {
        let tab_id = self.next_tab_id;
        let tab = Tab::new_loading(tab_id, path.clone(), file_size);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
        }

        info!(
            "Created loading tab for large file: {} ({:.1} MB)",
            path.display(),
            file_size as f64 / (1024.0 * 1024.0)
        );

        self.settings.add_recent_file(path);
        self.settings_dirty = true;
        self.save_settings_if_dirty();

        (new_index, tab_id)
    }

    /// Find a tab by file path.
    pub fn find_tab_by_path(&self, path: &PathBuf) -> Option<usize> {
        self.tabs.iter().position(|t| t.path.as_ref() == Some(path))
    }

    /// Swap two tabs by their indices, updating the active tab index if needed.
    ///
    /// Returns `true` if the swap was performed.
    pub fn swap_tabs(&mut self, a: usize, b: usize) -> bool {
        if a == b || a >= self.tabs.len() || b >= self.tabs.len() {
            return false;
        }
        self.tabs.swap(a, b);
        // Update active tab index to follow the moved tab
        if self.active_tab_index == a {
            self.active_tab_index = b;
        } else if self.active_tab_index == b {
            self.active_tab_index = a;
        }
        true
    }

    /// Set the active tab by index.
    ///
    /// Returns `true` if the index was valid and the tab was switched.
    pub fn set_active_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            // Request focus for the newly active tab so user can type immediately
            if let Some(tab) = self.tabs.get_mut(index) {
                tab.needs_focus = true;
            }
            debug!("Switched to tab {}", index);
            true
        } else {
            warn!("Invalid tab index: {}", index);
            false
        }
    }

    /// Close a tab by index.
    ///
    /// Returns `true` if the tab was closed, `false` if it has unsaved changes
    /// (use `force_close_tab` to close anyway).
    ///
    /// # Save Prompt Logic
    ///
    /// A save prompt is shown when the tab has modifications that should be saved.
    /// However, empty untitled files (new tabs with no content) are closed silently
    /// since there's nothing meaningful to preserve.
    pub fn close_tab(&mut self, index: usize) -> bool {
        if let Some(tab) = self.tabs.get(index) {
            if tab.should_prompt_to_save(&self.settings, SavePromptContext::TabClose) {
                // Set up confirmation dialog
                self.ui.show_confirm_dialog = true;
                self.ui.confirm_dialog_message =
                    format!("'{}' has unsaved changes. Close anyway?", tab.title());
                self.ui.pending_action = Some(PendingAction::CloseTab(index));
                return false;
            }
        }
        self.force_close_tab(index)
    }

    /// Force close a tab by index, ignoring unsaved changes.
    ///
    /// Returns `true` if the tab existed and was closed.
    pub fn force_close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }

        let ephemeral_pdf_path = self.tabs.get(index).and_then(|t| {
            if let TabKind::PdfViewer(vs) = &t.kind {
                if vs.ephemeral_temp_file {
                    return t.path.clone();
                }
            }
            None
        });

        // Persist view mode (and other tab state) before removing so reopen uses it
        if let Some(tab) = self.tabs.get(index) {
            if !tab.is_special() {
                self.settings.upsert_tab_info(tab.to_tab_info());
                self.settings_dirty = true;
            }
        }

        // Clear any pending recovery conflict for this tab so the banner does
        // not reappear if the runtime id is later reused (task 106.5).
        if let Some(tab) = self.tabs.get(index) {
            self.recovery_conflicts.remove(&tab.id);
        }

        self.tabs.remove(index);

        if let Some(path) = ephemeral_pdf_path {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!(
                    "Failed to delete ephemeral print-preview PDF {}: {}",
                    path.display(),
                    e
                );
            }
        }

        // Adjust active tab index
        if self.tabs.is_empty() {
            // Create a new empty tab if all tabs are closed
            self.new_tab();
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }

        debug!(
            "Closed tab {}, active is now {}",
            index, self.active_tab_index
        );
        true
    }

    /// Set the display-only title for a pathless document tab (persisted in session).
    pub fn apply_untitled_tab_rename(&mut self, index: usize, new_label: String) {
        let trimmed = new_label.trim().to_string();
        if let Some(tab) = self.tabs.get_mut(index) {
            if matches!(tab.kind, TabKind::Document) && tab.path.is_none() {
                tab.untitled_display_name =
                    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("untitled") {
                        None
                    } else {
                        Some(trimmed)
                    };
            }
        }
    }

    /// Close the active tab.
    pub fn close_active_tab(&mut self) -> bool {
        self.close_tab(self.active_tab_index)
    }

    /// Close every tab except the active one, prompting once for unsaved tabs.
    pub fn close_other_tabs(&mut self) -> bool {
        let Some(keep_tab_id) = self.active_tab().map(|tab| tab.id) else {
            return false;
        };
        let unsaved_count = self
            .tabs
            .iter()
            .filter(|tab| {
                tab.id != keep_tab_id
                    && tab.should_prompt_to_save(&self.settings, SavePromptContext::TabClose)
            })
            .count();

        if unsaved_count > 0 {
            self.ui.show_confirm_dialog = true;
            self.ui.confirm_dialog_message = format!(
                "{unsaved_count} other tab(s) have unsaved changes. Close other tabs anyway?"
            );
            self.ui.pending_action = Some(PendingAction::CloseOtherTabs(keep_tab_id));
            return false;
        }

        self.force_close_other_tabs(keep_tab_id)
    }

    /// Close every tab except `keep_tab_id` without prompting.
    pub fn force_close_other_tabs(&mut self, keep_tab_id: usize) -> bool {
        if !self.tabs.iter().any(|tab| tab.id == keep_tab_id) {
            return false;
        }

        let indices: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter_map(|(index, tab)| (tab.id != keep_tab_id).then_some(index))
            .collect();
        if indices.is_empty() {
            return false;
        }

        for index in indices.into_iter().rev() {
            self.force_close_tab(index);
        }
        true
    }

    /// Close every tab, prompting once if any tab has unsaved changes.
    pub fn close_all_tabs(&mut self) -> bool {
        let unsaved_count = self
            .tabs
            .iter()
            .filter(|tab| tab.should_prompt_to_save(&self.settings, SavePromptContext::TabClose))
            .count();

        if unsaved_count > 0 {
            self.ui.show_confirm_dialog = true;
            self.ui.confirm_dialog_message =
                format!("{unsaved_count} tab(s) have unsaved changes. Close all tabs anyway?");
            self.ui.pending_action = Some(PendingAction::CloseAllTabs);
            return false;
        }

        self.force_close_all_tabs()
    }

    /// Close every tab without prompting, preserving normal per-tab cleanup.
    pub fn force_close_all_tabs(&mut self) -> bool {
        let tab_count = self.tabs.len();
        if tab_count == 0 {
            self.new_tab();
            return false;
        }

        for index in (0..tab_count).rev() {
            self.force_close_tab(index);
        }
        true
    }

    /// Check if any tabs have unsaved changes that warrant a save prompt.
    ///
    /// Uses [`Tab::should_prompt_to_save`] with [`SavePromptContext::AppExit`] per tab.
    /// Empty untitled tabs never count. When **Quick note workflow** is enabled,
    /// modified pathless tabs are excluded so the app can exit without a dialog.
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| t.should_prompt_to_save(&self.settings, SavePromptContext::AppExit))
    }

    /// True if any editable document tab has unsaved content (for crash-recovery throttling).
    ///
    /// Wider than [`Self::has_unsaved_changes`]: includes pathless tabs when quick-note
    /// mode suppresses save prompts.
    pub fn any_modified_document_tab(&self) -> bool {
        self.tabs.iter().any(|t| {
            matches!(t.kind, TabKind::Document)
                && !t.is_loading()
                && !t.is_load_error()
                && t.is_modified()
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // File Operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Save the active tab to its file path.
    ///
    /// Returns an error if the tab has no path (use `save_as` instead).
    /// Uses the tab's current encoding for output.
    pub fn save_active_tab(&mut self) -> Result<(), crate::error::Error> {
        let tab = self
            .active_tab_mut()
            .ok_or_else(|| crate::error::Error::Application("No active tab".to_string()))?;

        let path = tab.path.clone().ok_or_else(|| {
            crate::error::Error::Application("No file path set. Use 'Save As' instead.".to_string())
        })?;

        // Encode content using the tab's current encoding
        let encoded_bytes = tab.encode_content();
        let encoding = tab.current_encoding;

        std::fs::write(&path, &encoded_bytes).map_err(|e| crate::error::Error::FileWrite {
            path: path.clone(),
            source: e,
        })?;

        // Update original_bytes to match what we saved
        tab.original_bytes = encoded_bytes;
        tab.mark_saved();
        let tab_id = tab.id;
        // Drop the stale recovery file now that the on-disk version is current
        // — prevents the file from hijacking another tab that may inherit this
        // id in a future session (see `resolve_tab_content`).
        crate::config::delete_recovery_content(tab_id);
        info!("Saved file: {} (encoding: {})", path.display(), encoding);
        Ok(())
    }

    /// Save the active tab to a new path.
    ///
    /// Uses the tab's current encoding for output. For "Save As" operations,
    /// the encoding is preserved from the original file or defaults to UTF-8.
    pub fn save_active_tab_as(&mut self, path: PathBuf) -> Result<(), crate::error::Error> {
        let tab = self
            .active_tab_mut()
            .ok_or_else(|| crate::error::Error::Application("No active tab".to_string()))?;

        // Encode content using the tab's current encoding
        let encoded_bytes = tab.encode_content();
        let encoding = tab.current_encoding;

        std::fs::write(&path, &encoded_bytes).map_err(|e| crate::error::Error::FileWrite {
            path: path.clone(),
            source: e,
        })?;

        tab.path = Some(path.clone());
        // Update original_bytes to match what we saved
        tab.original_bytes = encoded_bytes;
        tab.mark_saved();
        let tab_id = tab.id;
        // Drop the stale recovery file now that the on-disk version is current
        // — see note in `save_active_tab`.
        crate::config::delete_recovery_content(tab_id);

        // Update recent files and save immediately for persistence
        self.settings.add_recent_file(path.clone());
        self.settings_dirty = true;
        // Save immediately to survive app crashes/force-kills
        self.save_settings_if_dirty();

        info!("Saved file as: {} (encoding: {})", path.display(), encoding);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Workspace Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if the app is in workspace mode.
    pub fn is_workspace_mode(&self) -> bool {
        self.app_mode.is_workspace()
    }

    /// Get the workspace root path if in workspace mode.
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.app_mode.workspace_root()
    }

    /// Open a folder as a workspace.
    ///
    /// This switches the app to workspace mode and initializes the file tree.
    /// Returns `Ok(())` if successful, or an error if the folder can't be opened.
    pub fn open_workspace(&mut self, root: PathBuf) -> Result<(), crate::error::Error> {
        if !root.is_dir() {
            return Err(crate::error::Error::Application(format!(
                "Path is not a directory: {}",
                root.display()
            )));
        }

        info!("Opening workspace: {}", root.display());

        // Create the workspace
        let workspace = Workspace::new(root.clone());

        // Create the file watcher
        let watcher = match WorkspaceWatcher::new(root.clone()) {
            Ok(w) => {
                info!("File watcher started for workspace");
                Some(w)
            }
            Err(e) => {
                warn!("Failed to start file watcher: {}", e);
                None
            }
        };

        // Update app mode
        self.app_mode = AppMode::from_folder(root.clone());
        self.workspace = Some(workspace);
        self.workspace_watcher = watcher;
        self.pending_file_events.clear();

        // Initialize Git service for the workspace
        match self.git_service.open(&root) {
            Ok(true) => {
                if let Some(branch) = self.git_service.current_branch() {
                    info!("Git repository detected, branch: {}", branch);
                }
            }
            Ok(false) => {
                debug!("No Git repository in workspace");
            }
            Err(e) => {
                warn!("Error checking for Git repository: {}", e);
            }
        }

        // LSP servers are started on demand when a tab with a matching
        // file extension becomes active (see sync_active_doc_to_lsp).

        // Add to recent workspaces
        self.settings.add_recent_workspace(root);
        self.settings_dirty = true;

        info!("Workspace opened successfully");
        Ok(())
    }

    /// Close the current workspace and return to single-file mode.
    ///
    /// This saves the workspace state before closing.
    pub fn close_workspace(&mut self) {
        if let Some(workspace) = &self.workspace {
            // Save workspace state before closing
            if let Err(e) = workspace.save_state() {
                warn!("Failed to save workspace state: {}", e);
            }
        }

        // Stop all LSP servers before closing
        self.lsp.stop_all_servers();

        self.app_mode = AppMode::SingleFile;
        self.workspace = None;
        self.workspace_watcher = None;
        self.pending_file_events.clear();

        // Close Git service
        self.git_service.close();

        info!("Workspace closed, returned to single-file mode");
    }

    /// Previously started all detected LSP servers eagerly on workspace open.
    ///
    /// Now a no-op: servers are started on demand when a tab with a matching
    /// file extension becomes active. Kept for API compatibility with override
    /// restart logic in `handle_lsp_events`.
    #[allow(unused_variables)]
    pub fn start_lsp_for_workspace(&self, root: &Path) {
        // On-demand: servers are started lazily by sync_active_doc_to_lsp
    }

    /// Poll the file watcher for new events.
    ///
    /// This should be called periodically (e.g., in the update loop).
    /// Events are stored in pending_file_events for processing.
    pub fn poll_file_watcher(&mut self) {
        if let Some(watcher) = &self.workspace_watcher {
            if let Some(workspace) = &self.workspace {
                let raw_events = watcher.poll_events();
                if !raw_events.is_empty() {
                    // Filter out events for hidden paths
                    let filtered = filter_events(raw_events, &workspace.hidden_patterns);
                    self.pending_file_events.extend(filtered);
                }
            }
        }
    }

    /// Take pending file events (clears the list).
    pub fn take_file_events(&mut self) -> Vec<WorkspaceEvent> {
        std::mem::take(&mut self.pending_file_events)
    }

    /// Get a reference to the current workspace (if any).
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// Get a mutable reference to the current workspace (if any).
    pub fn workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspace.as_mut()
    }

    /// Refresh the workspace file tree.
    ///
    /// Call this after file operations that change the directory structure.
    pub fn refresh_workspace(&mut self) {
        if let Some(workspace) = &mut self.workspace {
            workspace.refresh_file_tree();
            debug!("Workspace file tree refreshed");
        }
    }

    /// Toggle the file tree panel visibility.
    pub fn toggle_file_tree(&mut self) {
        if let Some(workspace) = &mut self.workspace {
            workspace.show_file_tree = !workspace.show_file_tree;
            debug!("File tree visibility: {}", workspace.show_file_tree);
        }
    }

    /// Check if the file tree should be visible.
    pub fn should_show_file_tree(&self) -> bool {
        self.workspace
            .as_ref()
            .map(|w| w.show_file_tree)
            .unwrap_or(false)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Settings Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Update settings and mark as dirty.
    pub fn update_settings<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Settings),
    {
        f(&mut self.settings);
        self.settings_dirty = true;
    }

    /// Mark settings as dirty (needing to be saved).
    pub fn mark_settings_dirty(&mut self) {
        self.settings_dirty = true;
    }

    /// Save settings to config file if modified.
    ///
    /// Returns `true` if settings were saved.
    pub fn save_settings_if_dirty(&mut self) -> bool {
        if self.settings_dirty {
            // Merge open tabs into last_open_tabs (keep closed files' saved view modes)
            for info in self
                .tabs
                .iter()
                .filter(|t| !t.is_special())
                .map(|t| t.to_tab_info())
            {
                self.settings.upsert_tab_info(info);
            }
            self.settings.active_tab_index = self.active_tab_index;

            if save_config_silent(&self.settings) {
                self.settings_dirty = false;
                info!("Settings saved");
                return true;
            }
            warn!("Failed to save settings");
        }
        false
    }

    /// Force save settings to config file.
    pub fn save_settings(&mut self) -> bool {
        self.settings_dirty = true;
        self.save_settings_if_dirty()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Session State Persistence (Crash Recovery)
    // ─────────────────────────────────────────────────────────────────────────

    /// Capture the current session state for persistence.
    ///
    /// This creates a complete snapshot of the current editor session,
    /// including all open tabs, their content state, and editor positions.
    pub fn capture_session_state(&self) -> crate::config::SessionState {
        use crate::config::{hash_content, SessionAppMode, SessionState, SessionTabState};

        let tabs: Vec<SessionTabState> = self
            .tabs
            .iter()
            .filter(|tab| match &tab.kind {
                // Special tabs are UI panels — never persist (see docs/technical/ui/special-tabs.md).
                TabKind::Special(_) => false,
                TabKind::PdfViewer(vs) => !vs.ephemeral_temp_file,
                _ => true,
            })
            .map(|tab| {
                let file_mtime = tab.path.as_ref().and_then(|p| Self::get_file_mtime(p));

                let original_content_hash = if !tab.is_modified() {
                    Some(hash_content(&tab.content))
                } else {
                    None
                };

                SessionTabState {
                    tab_id: tab.id,
                    path: tab.path.clone(),
                    display_title: tab.persisted_session_display_title(),
                    view_mode: tab.view_mode,
                    cursor_char_index: tab.cursors.primary().head,
                    cursor_position: tab.cursor_position,
                    selection: tab.cursors.selection_range(),
                    scroll_offset: tab.scroll_offset,
                    rendered_scroll_offset: 0.0, // Will be captured if in rendered mode
                    has_unsaved_content: tab.is_modified(),
                    file_mtime,
                    original_content_hash,
                    csv_delimiter: None, // Will be populated by inject_csv_delimiters in app.rs
                }
            })
            .collect();

        let app_mode = if let Some(root) = self.app_mode.workspace_root() {
            // Canonicalize and normalize the path to ensure consistent storage across restarts
            // normalize_path removes Windows \\?\ prefix from canonicalized paths
            let canonical_root = root
                .canonicalize()
                .map(crate::path_utils::normalize_path)
                .unwrap_or_else(|_| root.clone());
            debug!(
                "Capturing session state with workspace: {} (canonical: {})",
                root.display(),
                canonical_root.display()
            );
            SessionAppMode::Workspace {
                root: Some(canonical_root),
            }
        } else {
            debug!("Capturing session state in single-file mode");
            SessionAppMode::SingleFile
        };

        SessionState {
            version: 1,
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            clean_shutdown: true,
            tabs,
            active_tab_index: self.active_tab_index,
            app_mode,
            zen_mode: self.ui.zen_mode,
        }
    }

    /// Save recovery content for tabs with unsaved changes.
    ///
    /// Captures each modified tab's current buffer along with its on-disk
    /// identity (`path` + hash of the content the tab was loaded from) so
    /// restore can reject recovery files whose tab id was reused for a
    /// different document or whose disk file was changed externally
    /// (task 106 — hardened session recovery).
    pub fn save_recovery_content(&self) {
        use crate::config::save_recovery_content;

        for tab in &self.tabs {
            if tab.is_special() {
                continue;
            }
            if tab.is_modified() {
                let ok = save_recovery_content(
                    tab.id,
                    &tab.content,
                    tab.path.as_deref(),
                    tab.disk_content_hash(),
                );
                if !ok {
                    warn!("Failed to save recovery content for tab {}", tab.id);
                }
            }
        }
    }

    /// Whether the given tab id currently has a recovery-vs-disk conflict.
    ///
    /// Used by the central panel to decide whether to render the conflict
    /// banner above the editor (task 106.5).
    pub fn has_recovery_conflict(&self, tab_id: usize) -> bool {
        self.recovery_conflicts.contains_key(&tab_id)
    }

    /// Read-only access to a recovery conflict (mostly for tests / UI).
    pub fn recovery_conflict(&self, tab_id: usize) -> Option<&RecoveryConflict> {
        self.recovery_conflicts.get(&tab_id)
    }

    /// Banner action: dismiss the recovery conflict, leaving the recovered
    /// buffer in place. The tab stays modified and the user can save manually
    /// (task 106.5). Returns `true` if a conflict was actually cleared.
    pub fn keep_recovered_buffer(&mut self, tab_id: usize) -> bool {
        let removed = self.recovery_conflicts.remove(&tab_id).is_some();
        if removed {
            log::info!(
                "Recovery conflict for tab {} resolved: kept recovered buffer",
                tab_id
            );
        }
        removed
    }

    /// Banner action: replace the tab's buffer with the on-disk content
    /// captured at restore time and mark the tab saved (no longer modified).
    /// Clears the conflict entry on success.
    ///
    /// Returns `true` if a conflict was found and the buffer was replaced.
    /// Untitled tabs and unknown tab ids return `false` without touching
    /// state. Used by the central panel banner (task 106.5).
    pub fn apply_reload_from_disk_for_conflict(&mut self, tab_id: usize) -> bool {
        let Some(conflict) = self.recovery_conflicts.remove(&tab_id) else {
            return false;
        };
        let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return false;
        };
        let Some(tab) = self.tabs.get_mut(idx) else {
            return false;
        };

        tab.content = conflict.on_disk_content;
        tab.notify_external_content_change();
        tab.mark_saved();

        // Clamp cursor to new content length so the user does not land past EOF.
        let max_chars = tab.content.chars().count();
        let cursor = tab.cursors.primary().head.min(max_chars);
        tab.pending_cursor_restore = Some(cursor);

        log::info!(
            "Recovery conflict for tab {} resolved: reloaded from disk",
            tab_id
        );
        true
    }

    /// Delete every `recovery/<tab_id>.json` whose id is NOT currently open
    /// AND every `untitled_<tab_id>.md.autosave` whose id is not in use
    /// (task 106.6 — autosave hardening).
    ///
    /// Call once after session restore is fully complete. Tab ids are
    /// reassigned every launch and both the recovery and autosave folders
    /// are append-only, so leftover files from previous sessions would
    /// otherwise sit there indefinitely and risk hijacking unrelated tabs
    /// in a future restore — see safety note on `resolve_tab_content`.
    pub fn prune_stale_recovery_files(&self) {
        use std::collections::HashSet;
        let valid_ids: HashSet<usize> = self
            .tabs
            .iter()
            .filter(|t| !t.is_special())
            .map(|t| t.id)
            .collect();
        crate::config::prune_recovery_dir(&valid_ids);
        crate::config::prune_auto_save_dir(&valid_ids);
    }

    /// Restore session from a SessionRestoreResult.
    ///
    /// This replaces the current tabs with those from the session state,
    /// optionally using recovered content for tabs with unsaved changes.
    ///
    /// Returns `true` if any tabs were restored.
    pub fn restore_from_session_result(
        &mut self,
        result: &crate::config::SessionRestoreResult,
    ) -> bool {
        let Some(session) = &result.session else {
            return false;
        };

        if session.tabs.is_empty() {
            return false;
        }

        // Clear existing tabs
        self.tabs.clear();

        let mut restored_count = 0;

        for session_tab in &session.tabs {
            // Viewer tabs: restore as viewer instead of document
            if let Some(path) = &session_tab.path {
                let file_type = FileType::from_path(path);
                if file_type.is_image() {
                    match self.open_image_tab(path.clone(), false) {
                        Ok(_) => {
                            restored_count += 1;
                        }
                        Err(e) => warn!("Could not restore image tab '{}': {}", path.display(), e),
                    }
                    continue;
                }
                if file_type.is_pdf() {
                    match self.open_pdf_tab(path.clone(), false) {
                        Ok(_) => {
                            restored_count += 1;
                        }
                        Err(e) => warn!("Could not restore PDF tab '{}': {}", path.display(), e),
                    }
                    continue;
                }
            }

            // Try to load content from various sources
            let resolved = self.resolve_tab_content(session_tab, result);

            // Extract the conflict (recovered + on-disk pair) before consuming
            // the resolved enum so the match arm below can apply the recovered
            // buffer the same way it does for plain `Recovered`. The conflict
            // is then stored in `self.recovery_conflicts` (task 106.5) keyed
            // by the tab id we are about to assign so the central panel banner
            // can render `Keep Recovered` / `Reload from Disk`.
            let pending_conflict: Option<(usize, RecoveryConflict)> = match &resolved {
                Some(ResolvedContent::RecoveredWithDiskDivergence {
                    content,
                    on_disk_content,
                }) => Some((
                    self.next_tab_id,
                    RecoveryConflict {
                        recovered_content: content.clone(),
                        on_disk_content: on_disk_content.clone(),
                    },
                )),
                _ => None,
            };

            if let Some(resolved) = resolved {
                let mut tab = match resolved {
                    ResolvedContent::Recovered(content) => {
                        // No divergence: try_apply_recovery already verified that
                        // either disk == content, or disk was unreadable. In both
                        // cases `original_content = content` is the only safe
                        // anchor we have, so `Tab::with_file` is correct here.
                        if let Some(path) = &session_tab.path {
                            let mut t =
                                Tab::with_file(self.next_tab_id, path.clone(), content.clone());
                            t.detected_encoding = Some("utf-8");
                            t.current_encoding = "utf-8";
                            t
                        } else {
                            let mut t = Tab::new(self.next_tab_id);
                            t.content = content.clone();
                            t
                        }
                    }
                    ResolvedContent::RecoveredWithDiskDivergence {
                        content,
                        on_disk_content,
                    } => {
                        // CRITICAL: original_content must be the on-disk text,
                        // NOT the recovered buffer. Otherwise the tab loses its
                        // identity link to the file on disk: `is_modified()`
                        // returns false and `disk_content_hash()` hashes the
                        // recovered buffer instead of disk, which poisons the
                        // next recovery snapshot's `original_content_hash`. The
                        // hash check in `try_apply_recovery` then rejects the
                        // recovery on the *following* launch and silently
                        // discards all edits made since the previous recovery
                        // (data-loss bug — see `docs/technical/files/
                        // session-persistence.md`, "Disk-hash anchoring across
                        // recovery cycles").
                        if let Some(path) = &session_tab.path {
                            let mut t = Tab::with_file(
                                self.next_tab_id,
                                path.clone(),
                                on_disk_content.clone(),
                            );
                            t.detected_encoding = Some("utf-8");
                            t.current_encoding = "utf-8";
                            // Swap in the recovered buffer without losing the
                            // disk anchor. `set_content` records one undo entry
                            // (so Ctrl+Z brings the user back to disk if they
                            // want), bumps `content_version`, and ensures all
                            // is_modified caches see content != original_content.
                            t.set_content(content.clone());
                            t
                        } else {
                            // Untitled tabs cannot have divergence (no disk),
                            // but handle for completeness.
                            let mut t = Tab::new(self.next_tab_id);
                            t.content = content.clone();
                            t
                        }
                    }
                    ResolvedContent::FromDisk {
                        content,
                        original_bytes,
                        encoding,
                        had_bom,
                    } => {
                        if let Some(path) = &session_tab.path {
                            let file_type = FileType::from_path(path);
                            let is_large_file = content.len() >= LARGE_FILE_THRESHOLD;

                            let (original_content_str, original_content_hash, final_original_bytes) =
                                if is_large_file {
                                    log::info!(
                                    "Restoring large file from disk ({} bytes): using hash-based modification detection",
                                    content.len()
                                );
                                    (
                                        String::new(),
                                        Some(Tab::compute_content_hash(&content)),
                                        Vec::new(),
                                    )
                                } else {
                                    (content.clone(), None, original_bytes)
                                };

                            let edit_history = if is_large_file {
                                EditHistory::with_max_groups(LARGE_FILE_MAX_UNDO_GROUPS)
                            } else {
                                EditHistory::new()
                            };

                            let t = Tab {
                                id: self.next_tab_id,
                                kind: TabKind::Document,
                                tab_content: TabContent::Ready,
                                path: Some(path.clone()),
                                untitled_display_name: None,
                                content,
                                original_content: original_content_str,
                                original_content_hash,
                                is_large_file,
                                cursors: MultiCursor::new(),
                                cursor_position: (0, 0),
                                selection: None,
                                scroll_offset: 0.0,
                                content_height: 0.0,
                                viewport_height: 0.0,
                                pending_scroll_offset: None,
                                pending_cursor_restore: None,
                                pending_scroll_ratio: None,
                                rendered_line_mappings: Vec::new(),
                                raw_line_height: 20.0,
                                pending_scroll_to_line: None,
                                pending_scroll_anchor: None,
                                skip_cursor_sync: false,
                                view_mode: ViewMode::Raw,
                                edit_history,
                                content_version: 0,
                                source_epoch: 0,
                                file_type,
                                needs_focus: false,
                                transient_highlight: TransientHighlight::new(),
                                auto_save_enabled: false,
                                last_edit_time: None,
                                last_auto_save_content_hash: None,
                                fold_state: FoldState::new(),
                                split_ratio: 0.5,
                                pipeline_state: TabPipelineState::default(),
                                detected_encoding: Some(encoding),
                                original_bytes: final_original_bytes,
                                current_encoding: encoding,
                                had_bom,
                                pending_undo_snapshot: None,
                                undo_content_hash: [0u8; 32],
                                cached_text_stats: TextStats::default(),
                                cached_text_stats_version: u64::MAX,
                                cached_is_modified: false,
                                cached_is_modified_version: u64::MAX,
                                save_version: 0,
                                cached_is_modified_save_version: u64::MAX,
                                cached_needs_cjk: false,
                                cached_needs_cjk_version: u64::MAX,
                                cached_needs_complex_script: false,
                                cached_needs_complex_script_version: u64::MAX,
                                last_auto_save_content_version: None,
                            };
                            t
                        } else {
                            let mut t = Tab::new(self.next_tab_id);
                            t.content = content.clone();
                            t
                        }
                    }
                };

                self.next_tab_id += 1;

                // Restore editor state
                tab.view_mode = session_tab.view_mode;
                tab.cursor_position = session_tab.cursor_position;
                tab.scroll_offset = session_tab.scroll_offset;

                // Restore cursor from char index
                tab.cursors.set_single(crate::state::Selection::cursor(
                    session_tab.cursor_char_index,
                ));
                if let Some((start, end)) = session_tab.selection {
                    tab.cursors
                        .set_single(crate::state::Selection::new(start, end));
                }
                tab.sync_cursor_from_primary();

                // If we loaded from recovery content, mark as modified
                if session_tab.has_unsaved_content
                    && result.recovered_content.contains_key(&session_tab.tab_id)
                {
                    // Content was recovered - it's modified relative to what's on disk
                    // The original_content field stays as the disk version
                }

                if session_tab.path.is_none() {
                    tab.untitled_display_name =
                        persisted_untitled_label_from_session(&session_tab.display_title);
                }

                self.tabs.push(tab);
                restored_count += 1;

                // If this tab was applied with a recovery-vs-disk divergence,
                // record the conflict so the central panel renders the
                // Keep Recovered / Reload from Disk banner above the editor
                // (task 106.5). Conflicts are keyed by the tab's runtime id.
                if let Some((tab_id, conflict)) = pending_conflict {
                    log::info!(
                        "Recovery conflict for tab {} ({}): recovered buffer differs \
                         from current disk content; banner will be shown.",
                        tab_id,
                        session_tab.display_title
                    );
                    self.recovery_conflicts.insert(tab_id, conflict);
                }

                debug!(
                    "Restored tab {} from session: {}",
                    session_tab.tab_id, session_tab.display_title
                );
            } else {
                warn!(
                    "Could not restore tab {}: {}",
                    session_tab.tab_id, session_tab.display_title
                );
            }
        }

        // Restore active tab index
        if !self.tabs.is_empty() {
            self.active_tab_index = session.active_tab_index.min(self.tabs.len() - 1);
        }

        // Restore Zen Mode state
        self.ui.zen_mode = session.zen_mode;

        // Restore workspace mode if it was active
        if let crate::config::SessionAppMode::Workspace { root: Some(root) } = &session.app_mode {
            // Validate the path is not empty
            if root.as_os_str().is_empty() {
                debug!(
                    "Session had workspace mode but path was empty, starting in single-file mode"
                );
            } else {
                debug!(
                    "Session had workspace mode active, attempting to restore: {}",
                    root.display()
                );

                // Try to canonicalize the path to resolve any relative paths or symlinks
                // normalize_path removes Windows \\?\ prefix from canonicalized paths
                let canonical_root = root
                    .canonicalize()
                    .map(crate::path_utils::normalize_path)
                    .unwrap_or_else(|e| {
                        debug!(
                            "Could not canonicalize workspace path {}: {}",
                            root.display(),
                            e
                        );
                        root.clone()
                    });

                if !canonical_root.exists() {
                    // Workspace path no longer exists - could be deleted or moved
                    warn!(
                        "Workspace folder no longer exists: {}. Starting in single-file mode.",
                        canonical_root.display()
                    );
                    debug!(
                        "The saved workspace path does not exist on disk. \
                         The folder may have been moved, renamed, or deleted. \
                         Original path: {}, canonical: {}",
                        root.display(),
                        canonical_root.display()
                    );
                } else if !canonical_root.is_dir() {
                    // Path exists but is not a directory - unlikely but handle it
                    warn!(
                        "Workspace path exists but is not a directory: {}. Starting in single-file mode.",
                        canonical_root.display()
                    );
                } else {
                    // Path exists and is a directory - try to open it
                    info!("Restoring workspace: {}", canonical_root.display());
                    match self.open_workspace(canonical_root.clone()) {
                        Ok(_) => {
                            debug!(
                                "Successfully restored workspace mode for: {}",
                                canonical_root.display()
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to restore workspace '{}': {}. Starting in single-file mode.",
                                canonical_root.display(),
                                e
                            );
                        }
                    }
                }
            }
        } else {
            debug!("Session was in single-file mode, no workspace to restore");
        }

        info!(
            "Restored {} of {} tabs from session{}{}",
            restored_count,
            session.tabs.len(),
            if session.zen_mode {
                " (Zen Mode enabled)"
            } else {
                ""
            },
            if self.app_mode.is_workspace() {
                " (Workspace mode)"
            } else {
                ""
            }
        );

        restored_count > 0
    }

    /// Identity-gated recovery application (task 106.4).
    ///
    /// Returns `Some(ResolvedContent)` only when the recovery file is safe to
    /// apply. Identity is verified in three layers:
    ///
    /// 1. **Legacy bypass** — pre-task-106 recovery files have neither `path`
    ///    nor `original_content_hash` set. Those fall back to the historical
    ///    "tab id only" matching (`has_unsaved_content` was already required
    ///    by the caller) so users upgrading from older Ferrite versions don't
    ///    silently lose recovered buffers. No conflict banner can be raised
    ///    in this branch because we have no identity to compare against.
    /// 2. **Path equality** — `recovered.path` must equal `session_tab.path`.
    ///    A mismatch indicates the `tab_id` was reused for an unrelated
    ///    document (the original "bleed" data-loss hazard). Untitled tabs are
    ///    covered by this check too: both sides must have `path == None`.
    /// 3. **Disk hash** — when the tab is path-backed, the file exists on
    ///    disk, and `recovered.original_content_hash` is `Some(want)`, the
    ///    current disk content is hashed (using the same algorithm as
    ///    [`crate::config::hash_content`]). A mismatch means the
    ///    file was edited externally between sessions; we reject the
    ///    recovery and fall through to a fresh disk load. If the file is
    ///    unreadable as UTF-8 we trust the path-only identity to avoid
    ///    losing the user's recovered edits to encoding edge cases.
    ///
    /// On a clean identity match the buffer is applied via either
    /// [`ResolvedContent::Recovered`] (no divergence — disk is byte-for-byte
    /// identical to the recovered buffer) or
    /// [`ResolvedContent::RecoveredWithDiskDivergence`] (disk has the same
    /// hash the recovery was anchored to but its current content differs
    /// from the recovered buffer — the user had unsaved edits we want to
    /// surface via the conflict banner from task 106.5).
    fn try_apply_recovery(
        session_tab: &crate::config::SessionTabState,
        recovered: &crate::config::RecoveryContent,
    ) -> Option<ResolvedContent> {
        let is_legacy = recovered.path.is_none() && recovered.original_content_hash.is_none();

        if is_legacy {
            // Pre-task-106 recovery file with no identity to verify.
            // Preserve historical behaviour so upgrading users keep their
            // recovered buffers; the caller already required has_unsaved_content
            // and `prune_stale_recovery_files` cleans up dangling tab ids.
            debug!(
                "Applying legacy recovery (no identity) for tab {} ({})",
                session_tab.tab_id, session_tab.display_title
            );
            return Some(ResolvedContent::Recovered(recovered.content.clone()));
        }

        // Layer 2: path equality (covers untitled-tab case via None == None).
        if recovered.path != session_tab.path {
            warn!(
                "Rejecting recovery for tab {} ({}): recovered path {:?} \
                 does not match session path {:?}; recovery file is from a \
                 reused tab id and will be pruned.",
                session_tab.tab_id, session_tab.display_title, recovered.path, session_tab.path
            );
            crate::diag::event(
                "session_recovery_identity_mismatch",
                format!(
                    "tab_id={} title={} session_path={:?} recovered_path={:?} \
                     reason=path_mismatch",
                    session_tab.tab_id, session_tab.display_title, session_tab.path, recovered.path,
                ),
            );
            return None;
        }

        // Layer 3: disk hash check (path-backed tabs whose file exists).
        // We read the disk text once and reuse it for the divergence check
        // below to avoid two reads.
        let disk_content: Option<String> = session_tab
            .path
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| std::fs::read_to_string(p).ok());

        if let (Some(want), Some(disk)) = (recovered.original_content_hash, disk_content.as_ref()) {
            let got = crate::config::hash_content(disk);
            if got != want {
                warn!(
                    "Rejecting recovery for tab {} ({}): disk content hash {:?} \
                     does not match recovered original_content_hash {:?}; the \
                     file changed externally between sessions.",
                    session_tab.tab_id, session_tab.display_title, got, want
                );
                crate::diag::event(
                    "session_recovery_identity_mismatch",
                    format!(
                        "tab_id={} title={} session_path={:?} recovered_path={:?} \
                         expected_hash={:?} disk_hash={:?} reason=hash_mismatch",
                        session_tab.tab_id,
                        session_tab.display_title,
                        session_tab.path,
                        recovered.path,
                        Some(want),
                        Some(got),
                    ),
                );
                return None;
            }
        }

        // Identity OK. If the path-backed tab has disk content that differs
        // from the recovered buffer, surface as RecoveredWithDiskDivergence so
        // the conflict banner can offer Keep Recovered / Reload from Disk.
        // If we couldn't read the disk (encoding failure, missing file), we
        // skip divergence detection — the banner is purely informational and
        // not safety-critical when identity already matched.
        let divergence = disk_content
            .as_deref()
            .filter(|disk| *disk != recovered.content.as_str())
            .map(|d| d.to_string());

        debug!(
            "Using recovered content for tab {} ({}) (divergent_disk={})",
            session_tab.tab_id,
            session_tab.display_title,
            divergence.is_some()
        );

        Some(match divergence {
            Some(on_disk_content) => ResolvedContent::RecoveredWithDiskDivergence {
                content: recovered.content.clone(),
                on_disk_content,
            },
            None => ResolvedContent::Recovered(recovered.content.clone()),
        })
    }

    /// Resolve content for a tab from various sources.
    ///
    /// Priority:
    /// 1. Recovery content (if the session itself flagged the tab as having
    ///    unsaved changes AND the recovery file's identity matches —
    ///    see [`Self::try_apply_recovery`])
    /// 2. File on disk (if path exists)
    /// 3. None (if file is missing and no recovery content)
    ///
    /// **Safety:** Recovery files live in `recovery/<tab_id>.json` and persist
    /// across launches, but the `tab_id` namespace is per-session and starts at
    /// 0 every launch. A leftover recovery file from a previous session can
    /// otherwise be applied to an unrelated tab in the current session that
    /// happens to be assigned the same id (silent data loss — the corrupted
    /// tab is presented as clean and a save would overwrite the real file).
    /// We therefore only consult recovery content when the session_tab itself
    /// declared `has_unsaved_content` AND the recovery file's `path` +
    /// `original_content_hash` line up with the tab's on-disk identity (task
    /// 106.4). Mismatched recovery is logged with a `session_recovery_*`
    /// diag event and pruned at startup by `prune_stale_recovery_files`.
    fn resolve_tab_content(
        &self,
        session_tab: &crate::config::SessionTabState,
        result: &crate::config::SessionRestoreResult,
    ) -> Option<ResolvedContent> {
        use chardetng::EncodingDetector;

        // First, check if we have recovery content — but only trust it if the
        // session itself flagged the tab as having unsaved changes AND the
        // recovery file's identity matches the tab's on-disk identity (task
        // 106.4). Identity here means the recovery file's `path` and
        // `original_content_hash` are consistent with the session tab's path
        // and the current disk content.
        if let Some(recovered) = result.recovered_content.get(&session_tab.tab_id) {
            if !session_tab.has_unsaved_content {
                warn!(
                    "Ignoring stale recovery file for tab {} ({}): session reports no unsaved \
                     content. The recovery file is likely from a previous session that reused \
                     the same tab id; it will be pruned.",
                    session_tab.tab_id, session_tab.display_title
                );
                crate::diag::event(
                    "session_recovery_stale_ignored",
                    format!(
                        "tab_id={} title={} path={:?}",
                        session_tab.tab_id, session_tab.display_title, session_tab.path
                    ),
                );
            } else if let Some(resolved) = Self::try_apply_recovery(session_tab, recovered) {
                return Some(resolved);
            }
            // Identity rejected (or hash mismatch) — fall through to disk load
            // below. The stale recovery file is pruned at startup by
            // `prune_stale_recovery_files`.
        }

        // Next, try to load from disk with encoding detection
        if let Some(path) = &session_tab.path {
            if path.exists() {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        // Detect encoding
                        let mut detector = EncodingDetector::new();
                        detector.feed(&bytes, true);
                        let detected = detector.guess(None, true);

                        // Check for BOM first
                        let (content, encoding, had_bom) = if let Some((bom_encoding, bom_len)) =
                            encoding_rs::Encoding::for_bom(&bytes)
                        {
                            // Use decode_without_bom_handling since we already handled the BOM
                            let (decoded, _had_errors) =
                                bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
                            (decoded.into_owned(), bom_encoding.name(), true)
                        } else {
                            let (decoded, _, _) = detected.decode(&bytes);
                            (decoded.into_owned(), detected.name(), false)
                        };

                        debug!(
                            "Loaded content from disk for tab {} (encoding: {}, had_bom: {})",
                            session_tab.tab_id, encoding, had_bom
                        );
                        return Some(ResolvedContent::FromDisk {
                            content,
                            original_bytes: bytes,
                            encoding,
                            had_bom,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "Failed to read file for tab {}: {}: {}",
                            session_tab.tab_id,
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        if session_tab.path.is_none() && !session_tab.has_unsaved_content {
            return Some(ResolvedContent::Recovered(String::new()));
        }

        // For tabs without a path (unsaved documents), we need recovery content
        if session_tab.path.is_none() && session_tab.has_unsaved_content {
            debug!(
                "Unsaved document {} has no recovery content",
                session_tab.tab_id
            );
            return None;
        }

        None
    }

    /// Get file modification time as Unix timestamp.
    fn get_file_mtime(path: &std::path::Path) -> Option<u64> {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Event Handling
    // ─────────────────────────────────────────────────────────────────────────

    /// Handle a confirmed pending action.
    pub fn handle_confirmed_action(&mut self) {
        if let Some(action) = self.ui.pending_action.take() {
            match action {
                PendingAction::CloseTab(index) => {
                    self.force_close_tab(index);
                }
                PendingAction::CloseAllTabs => {
                    self.force_close_all_tabs();
                }
                PendingAction::CloseOtherTabs(keep_tab_id) => {
                    self.force_close_other_tabs(keep_tab_id);
                }
                PendingAction::Exit => {
                    // Caller should handle exit
                    debug!("Exit confirmed");
                }
                PendingAction::OpenFile(path) => {
                    if let Err(e) = self.open_file(path, None) {
                        self.show_error(format!("Failed to open file:\n{}", e));
                    }
                }
                PendingAction::NewDocument => {
                    self.new_tab();
                }
            }
        }
        self.ui.show_confirm_dialog = false;
        self.ui.confirm_dialog_message.clear();
    }

    /// Cancel the pending action.
    pub fn cancel_pending_action(&mut self) {
        self.ui.pending_action = None;
        self.ui.show_confirm_dialog = false;
        self.ui.confirm_dialog_message.clear();
    }

    /// Request application exit.
    ///
    /// Returns `true` if exit can proceed immediately, `false` if confirmation is needed.
    pub fn request_exit(&mut self) -> bool {
        if self.has_unsaved_changes() {
            self.ui.show_confirm_dialog = true;
            self.ui.confirm_dialog_message = "You have unsaved changes. Exit anyway?".to_string();
            self.ui.pending_action = Some(PendingAction::Exit);
            false
        } else {
            true
        }
    }

    /// Prepare state for application shutdown.
    ///
    /// This saves settings, workspace state, and performs any necessary cleanup.
    pub fn shutdown(&mut self) {
        // Save workspace state if in workspace mode
        if let Some(workspace) = &self.workspace {
            if let Err(e) = workspace.save_state() {
                warn!("Failed to save workspace state during shutdown: {}", e);
            }
        }

        self.save_settings();
        info!("AppState shutdown complete");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UI State Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Set the status message.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.ui.status_message = Some(message.into());
    }

    /// Clear the status message.
    pub fn clear_status(&mut self) {
        self.ui.status_message = None;
    }

    /// Toggle the settings panel.
    pub fn toggle_settings(&mut self) {
        self.ui.show_settings = !self.ui.show_settings;
    }

    /// Toggle the find/replace panel.
    pub fn toggle_find_replace(&mut self) {
        self.ui.show_find_replace = !self.ui.show_find_replace;
    }

    /// Toggle the about/help panel.
    ///
    /// If already viewing the About tab, closes it and returns to the previous tab.
    /// Otherwise, opens the About tab.
    pub fn toggle_about(&mut self) {
        // Check if we're already viewing the About tab
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            if matches!(&tab.kind, TabKind::Special(SpecialTabKind::About)) {
                // Close it
                self.force_close_tab(self.active_tab_index);
                return;
            }
        }
        self.open_special_tab(SpecialTabKind::About);
    }

    /// Open the settings panel as a tab.
    ///
    /// If already viewing the Settings tab, closes it.
    /// Otherwise, opens the Settings tab.
    pub fn open_settings_tab(&mut self) {
        // Check if we're already viewing the Settings tab
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            if matches!(&tab.kind, TabKind::Special(SpecialTabKind::Settings)) {
                self.force_close_tab(self.active_tab_index);
                return;
            }
        }
        self.open_special_tab(SpecialTabKind::Settings);
    }

    /// Toggle Zen Mode (distraction-free writing).
    pub fn toggle_zen_mode(&mut self) {
        self.ui.zen_mode = !self.ui.zen_mode;
    }

    /// Check if Zen Mode is enabled.
    pub fn is_zen_mode(&self) -> bool {
        self.ui.zen_mode
    }

    /// Show an error in a modal dialog.
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.ui.error_message = message.into();
        self.ui.show_error_modal = true;
    }

    /// Dismiss the error modal.
    pub fn dismiss_error(&mut self) {
        self.ui.show_error_modal = false;
        self.ui.error_message.clear();
    }

    /// Show the portal error dialog with installation instructions.
    pub fn show_portal_error(&mut self, message: impl Into<String>, command: impl Into<String>) {
        self.ui.portal_error_message = message.into();
        self.ui.portal_error_command = command.into();
        self.ui.show_portal_error_dialog = true;
    }

    /// Dismiss the portal error dialog.
    pub fn dismiss_portal_error(&mut self) {
        self.ui.show_portal_error_dialog = false;
        self.ui.portal_error_message.clear();
        self.ui.portal_error_command.clear();
    }

    /// Show a temporary toast message (disappears after duration).
    ///
    /// `current_time` should be the current app time in seconds.
    /// `duration` is how long to show the message in seconds.
    pub fn show_toast(&mut self, message: impl Into<String>, current_time: f64, duration: f64) {
        self.ui.toast_message = Some(message.into());
        self.ui.toast_expires_at = Some(current_time + duration);
    }

    /// Update toast state - clears expired toasts.
    ///
    /// Call this each frame with the current time.
    pub fn update_toast(&mut self, current_time: f64) {
        if let Some(expires_at) = self.ui.toast_expires_at {
            if current_time >= expires_at {
                self.ui.toast_message = None;
                self.ui.toast_expires_at = None;
            }
        }
    }

    /// Clear any active toast message.
    pub fn clear_toast(&mut self) {
        self.ui.toast_message = None;
        self.ui.toast_expires_at = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Tests
    // ─────────────────────────────────────────────────────────────────────────

    // ─────────────────────────────────────────────────────────────────────────
    // FileType Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("md"), FileType::Markdown);
        assert_eq!(FileType::from_extension("markdown"), FileType::Markdown);
        assert_eq!(FileType::from_extension("MD"), FileType::Markdown);
        assert_eq!(FileType::from_extension("json"), FileType::Json);
        assert_eq!(FileType::from_extension("JSON"), FileType::Json);
        assert_eq!(FileType::from_extension("yaml"), FileType::Yaml);
        assert_eq!(FileType::from_extension("yml"), FileType::Yaml);
        assert_eq!(FileType::from_extension("toml"), FileType::Toml);
        assert_eq!(FileType::from_extension("csv"), FileType::Csv);
        assert_eq!(FileType::from_extension("CSV"), FileType::Csv);
        assert_eq!(FileType::from_extension("tsv"), FileType::Tsv);
        assert_eq!(FileType::from_extension("TSV"), FileType::Tsv);
        assert_eq!(FileType::from_extension("txt"), FileType::Unknown);
        assert_eq!(FileType::from_extension("rs"), FileType::Unknown);
    }

    #[test]
    fn test_file_type_from_path() {
        assert_eq!(
            FileType::from_path(Path::new("readme.md")),
            FileType::Markdown
        );
        assert_eq!(
            FileType::from_path(Path::new("config.json")),
            FileType::Json
        );
        assert_eq!(
            FileType::from_path(Path::new("docker-compose.yaml")),
            FileType::Yaml
        );
        assert_eq!(FileType::from_path(Path::new("Cargo.toml")), FileType::Toml);
        assert_eq!(FileType::from_path(Path::new("data.csv")), FileType::Csv);
        assert_eq!(FileType::from_path(Path::new("data.tsv")), FileType::Tsv);
        assert_eq!(FileType::from_path(Path::new("main.rs")), FileType::Unknown);
        assert_eq!(
            FileType::from_path(Path::new("no_extension")),
            FileType::Unknown
        );
    }

    #[test]
    fn test_file_type_helpers() {
        assert!(FileType::Markdown.is_markdown());
        assert!(!FileType::Json.is_markdown());

        assert!(FileType::Json.is_structured());
        assert!(FileType::Yaml.is_structured());
        assert!(FileType::Toml.is_structured());
        assert!(!FileType::Markdown.is_structured());
        assert!(!FileType::Csv.is_structured());
        assert!(!FileType::Tsv.is_structured());
        assert!(!FileType::Unknown.is_structured());

        assert!(FileType::Csv.is_tabular());
        assert!(FileType::Tsv.is_tabular());
        assert!(!FileType::Json.is_tabular());
        assert!(!FileType::Markdown.is_tabular());
        assert!(!FileType::Unknown.is_tabular());

        assert_eq!(FileType::Markdown.display_name(), "Markdown");
        assert_eq!(FileType::Json.display_name(), "JSON");
        assert_eq!(FileType::Yaml.display_name(), "YAML");
        assert_eq!(FileType::Toml.display_name(), "TOML");
        assert_eq!(FileType::Csv.display_name(), "CSV");
        assert_eq!(FileType::Tsv.display_name(), "TSV");
        assert_eq!(FileType::Unknown.display_name(), "Unknown");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Binary File Detection Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_binary_detection_empty() {
        // Empty files are not binary
        assert!(!is_binary_content(b""));
    }

    #[test]
    fn test_binary_detection_null_bytes() {
        // Files with null bytes are binary
        assert!(is_binary_content(b"Hello\x00World"));
        assert!(is_binary_content(b"\x00"));
        assert!(is_binary_content(b"\x00\x00\x00"));
    }

    #[test]
    fn test_binary_detection_text() {
        // Plain text is not binary
        assert!(!is_binary_content(b"Hello, World!"));
        assert!(!is_binary_content(b"Line 1\nLine 2\nLine 3"));
        // Unicode text in regular string (not byte string)
        assert!(!is_binary_content("Special chars: äöü émoji 🎉".as_bytes()));
    }

    #[test]
    fn test_binary_detection_markdown() {
        // Markdown content is not binary
        let md = b"# Heading\n\nThis is **bold** and _italic_.\n\n- Item 1\n- Item 2";
        assert!(!is_binary_content(md));
    }

    #[test]
    fn test_binary_detection_json() {
        // JSON content is not binary
        let json = b"{\"name\": \"test\", \"value\": 123, \"nested\": {\"key\": \"value\"}}";
        assert!(!is_binary_content(json));
    }

    #[test]
    fn test_binary_detection_control_chars() {
        // Some control characters are allowed (tab, newline, carriage return)
        assert!(!is_binary_content(b"Tab:\t Newline:\n CR:\r"));

        // But many control characters indicate binary
        let binary_with_control = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0B, 0x0C, 0x0E, 0x0F,
        ];
        assert!(is_binary_content(&binary_with_control));
    }

    #[test]
    fn test_binary_detection_simulated_image() {
        // Simulate PNG header - contains bytes that are control chars
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        // 0x89, 0x1A are considered control characters (non-printable)
        let png_like = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ];
        assert!(is_binary_content(&png_like));

        // Simulate a binary file with high non-printable ratio
        // JPEG-like with lots of 0xFF bytes and other control chars
        let binary_data: Vec<u8> = (0..100)
            .map(|i| if i % 3 == 0 { 0xFF } else { i as u8 })
            .collect();
        assert!(is_binary_content(&binary_data));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tab Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_new() {
        let tab = Tab::new(1);
        assert_eq!(tab.id, 1);
        assert!(tab.path.is_none());
        assert!(tab.content.is_empty());
        assert!(!tab.is_modified());
        assert_eq!(tab.view_mode, ViewMode::Raw); // New tabs default to raw mode
        assert_eq!(tab.file_type(), FileType::Markdown); // New tabs default to markdown
    }

    #[test]
    fn test_tab_with_file() {
        let path = PathBuf::from("/test/file.md");
        let content = "# Hello".to_string();
        let tab = Tab::with_file(1, path.clone(), content.clone());

        assert_eq!(tab.id, 1);
        assert_eq!(tab.path, Some(path));
        assert_eq!(tab.content, content);
        assert!(!tab.is_modified());
        assert_eq!(tab.file_type(), FileType::Markdown);
    }

    #[test]
    fn test_tab_file_type_detection() {
        // Markdown file
        let md_tab = Tab::with_file(1, PathBuf::from("readme.md"), String::new());
        assert_eq!(md_tab.file_type(), FileType::Markdown);

        // JSON file
        let json_tab = Tab::with_file(2, PathBuf::from("config.json"), String::new());
        assert_eq!(json_tab.file_type(), FileType::Json);

        // YAML file
        let yaml_tab = Tab::with_file(3, PathBuf::from("docker-compose.yml"), String::new());
        assert_eq!(yaml_tab.file_type(), FileType::Yaml);

        // TOML file
        let toml_tab = Tab::with_file(4, PathBuf::from("Cargo.toml"), String::new());
        assert_eq!(toml_tab.file_type(), FileType::Toml);

        // Unknown file
        let rs_tab = Tab::with_file(5, PathBuf::from("main.rs"), String::new());
        assert_eq!(rs_tab.file_type(), FileType::Unknown);
    }

    #[test]
    fn test_tab_set_path_updates_file_type() {
        let mut tab = Tab::new(1);
        assert_eq!(tab.file_type(), FileType::Markdown);

        tab.set_path(PathBuf::from("config.json"));
        assert_eq!(tab.file_type(), FileType::Json);
        assert_eq!(tab.path, Some(PathBuf::from("config.json")));

        tab.set_path(PathBuf::from("data.yaml"));
        assert_eq!(tab.file_type(), FileType::Yaml);
    }

    #[test]
    fn test_tab_modification_tracking() {
        let mut tab = Tab::new(0);
        assert!(!tab.is_modified());

        tab.set_content("new content".to_string());
        assert!(tab.is_modified());

        tab.mark_saved();
        assert!(!tab.is_modified());
    }

    #[test]
    fn test_tab_disk_content_hash_untitled_returns_none() {
        let tab = Tab::new(0);
        assert!(
            tab.disk_content_hash().is_none(),
            "untitled empty tab has no disk hash"
        );
    }

    #[test]
    fn test_tab_disk_content_hash_path_backed_small_file() {
        // Small files store the disk text in `original_content`, so the hash
        // is computed on demand. It must equal the hash of that content using
        // the same DefaultHasher algorithm as session::hash_content.
        let path = PathBuf::from("/tmp/example.md");
        let body = "# Hello\n\nbody".to_string();
        let tab = Tab::with_file(1, path, body.clone());

        let got = tab.disk_content_hash().expect("path-backed tab has hash");
        assert_eq!(got, crate::config::hash_content(&body));
    }

    #[test]
    fn test_tab_disk_content_hash_does_not_track_in_memory_edits() {
        // The hash must reflect *disk* content, not the in-memory buffer,
        // so a recovery file written from a modified tab still references the
        // unmodified disk identity used to detect external changes.
        let path = PathBuf::from("/tmp/example.md");
        let original = "v1".to_string();
        let mut tab = Tab::with_file(1, path, original.clone());
        let pristine_hash = tab.disk_content_hash().unwrap();

        tab.set_content("v2 unsaved".to_string());
        assert!(tab.is_modified());
        assert_eq!(
            tab.disk_content_hash(),
            Some(pristine_hash),
            "edits in the buffer must not change the disk hash"
        );
    }

    #[test]
    fn test_tab_disk_content_hash_reflects_save() {
        // After mark_saved, the disk hash should follow the new content
        // (the file on disk is now what's in the buffer).
        let path = PathBuf::from("/tmp/example.md");
        let mut tab = Tab::with_file(1, path, "v1".to_string());

        tab.set_content("v2".to_string());
        tab.mark_saved();

        let expected = crate::config::hash_content("v2");
        assert_eq!(tab.disk_content_hash(), Some(expected));
    }

    // ─────────────────────────────────────────────────────────────────────
    // 106.4 — resolve_tab_content / try_apply_recovery identity gating
    // ─────────────────────────────────────────────────────────────────────

    /// Build a `SessionTabState` that resembles a path-backed tab the user
    /// closed with unsaved edits. Used by the recovery-identity tests below.
    fn session_tab(
        tab_id: usize,
        path: Option<PathBuf>,
        has_unsaved: bool,
    ) -> crate::config::SessionTabState {
        crate::config::SessionTabState {
            tab_id,
            path,
            display_title: format!("test-tab-{tab_id}"),
            has_unsaved_content: has_unsaved,
            ..Default::default()
        }
    }

    #[test]
    fn test_recovery_identity_path_match_no_hash_returns_recovered() {
        use crate::config::RecoveryContent;
        // Path matches and there's no `original_content_hash` to verify
        // (e.g. tab whose disk file went away between sessions). Identity
        // is trusted on path equality alone — buffer is applied as Recovered
        // (no disk to diff against, no banner).
        let path = PathBuf::from("/non/existent/identity-1.md");
        let st = session_tab(7, Some(path.clone()), true);
        let rc = RecoveryContent::new_with_identity(7, "buf".into(), Some(path), None);

        let resolved = AppState::try_apply_recovery(&st, &rc).expect("identity ok");
        assert!(
            matches!(resolved, ResolvedContent::Recovered(ref c) if c == "buf"),
            "path-only match must apply as Recovered, got: {resolved:?}"
        );
    }

    #[test]
    fn test_recovery_identity_path_mismatch_rejected() {
        use crate::config::RecoveryContent;
        // Original "tab id bleed": tab 10 in the previous session was a
        // markdown file; in this session tab 10 is a different file. The
        // recovery file's path must not be silently applied to it.
        let st = session_tab(10, Some(PathBuf::from("/notes/task_50.md")), true);
        let rc = RecoveryContent::new_with_identity(
            10,
            "asdasd".into(),
            None, // recovery was for an untitled tab
            None,
        );

        assert!(
            AppState::try_apply_recovery(&st, &rc).is_none(),
            "untitled recovery must NOT apply to path-backed tab with same id"
        );
    }

    #[test]
    fn test_recovery_identity_path_mismatch_different_files_rejected() {
        use crate::config::RecoveryContent;
        // Path-backed in both sessions but for two different files.
        let st = session_tab(3, Some(PathBuf::from("/work/b.md")), true);
        let rc = RecoveryContent::new_with_identity(
            3,
            "x".into(),
            Some(PathBuf::from("/work/a.md")),
            Some(0xabc),
        );

        assert!(
            AppState::try_apply_recovery(&st, &rc).is_none(),
            "differing recovered.path must reject"
        );
    }

    #[test]
    fn test_recovery_identity_legacy_file_path_backed_session_applied() {
        use crate::config::RecoveryContent;
        // Pre-task-106 recovery file (no path, no hash) on a path-backed
        // session tab is applied via the back-compat fallback so users
        // upgrading from older Ferrite versions do not lose recovered text.
        let st = session_tab(4, Some(PathBuf::from("/legacy/notes.md")), true);
        let rc = RecoveryContent::new(4, "legacy buffer".into());
        assert!(rc.path.is_none() && rc.original_content_hash.is_none());

        let resolved = AppState::try_apply_recovery(&st, &rc).expect("legacy back-compat");
        match resolved {
            ResolvedContent::Recovered(c) => assert_eq!(c, "legacy buffer"),
            other => panic!("legacy file must apply as Recovered, got {other:?}"),
        }
    }

    #[test]
    fn test_recovery_identity_legacy_file_untitled_session_applied() {
        use crate::config::RecoveryContent;
        // Both sides have None paths. Same legacy fallback applies.
        let st = session_tab(8, None, true);
        let rc = RecoveryContent::new(8, "buf".into());
        let resolved = AppState::try_apply_recovery(&st, &rc).expect("legacy back-compat");
        assert!(matches!(resolved, ResolvedContent::Recovered(ref c) if c == "buf"));
    }

    #[test]
    fn test_recovery_identity_untitled_match_applied() {
        use crate::config::RecoveryContent;
        // Untitled tabs with explicit path=None on both sides match.
        let st = session_tab(11, None, true);
        let rc = RecoveryContent::new_with_identity(11, "buf".into(), None, None);
        let resolved = AppState::try_apply_recovery(&st, &rc).expect("untitled match");
        assert!(matches!(resolved, ResolvedContent::Recovered(ref c) if c == "buf"));
    }

    #[test]
    fn test_recovery_identity_hash_match_no_divergence_recovered() {
        use crate::config::RecoveryContent;
        // Path matches AND `original_content_hash` matches the disk content
        // AND the disk content is byte-for-byte identical to the recovery
        // buffer (i.e. the user had no unsaved edits when the recovery was
        // taken — unusual but possible). Apply as plain Recovered.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("identity-eq.md");
        let body = "shared body";
        std::fs::write(&path, body).expect("write");

        let st = session_tab(12, Some(path.clone()), true);
        let want = crate::config::hash_content(body);
        let rc = RecoveryContent::new_with_identity(12, body.to_string(), Some(path), Some(want));

        let resolved = AppState::try_apply_recovery(&st, &rc).expect("identity ok");
        assert!(
            matches!(resolved, ResolvedContent::Recovered(ref c) if c == body),
            "identical buffer + identical disk must apply as Recovered, got: {resolved:?}"
        );
    }

    #[test]
    fn test_recovery_identity_hash_match_with_divergence_returns_divergence() {
        use crate::config::RecoveryContent;
        // Path matches, disk-hash matches the recorded `original_content_hash`,
        // but the recovered buffer has unsaved edits on top of disk. Surface
        // RecoveredWithDiskDivergence so 106.5's banner can offer Reload from
        // Disk vs Keep Recovered.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("identity-divergent.md");
        let disk_body = "v1 disk";
        std::fs::write(&path, disk_body).expect("write");

        let st = session_tab(13, Some(path.clone()), true);
        let recovered_buffer = "v1 disk + unsaved edits";
        let rc = RecoveryContent::new_with_identity(
            13,
            recovered_buffer.to_string(),
            Some(path),
            Some(crate::config::hash_content(disk_body)),
        );

        let resolved = AppState::try_apply_recovery(&st, &rc).expect("identity ok");
        match resolved {
            ResolvedContent::RecoveredWithDiskDivergence {
                content,
                on_disk_content,
            } => {
                assert_eq!(content, recovered_buffer);
                assert_eq!(on_disk_content, disk_body);
            }
            other => panic!("expected divergence variant, got {other:?}"),
        }
    }

    #[test]
    fn test_recovery_identity_hash_mismatch_rejected() {
        use crate::config::RecoveryContent;
        // Path matches but disk has been edited externally between
        // sessions, so the disk hash no longer matches the recovery's
        // `original_content_hash`. Reject so the user does not silently
        // overwrite the new disk content with stale recovery.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("identity-hashmiss.md");
        std::fs::write(&path, "external edit").expect("write");

        let st = session_tab(14, Some(path.clone()), true);
        // Recovery was anchored to a different disk content hash.
        let rc = RecoveryContent::new_with_identity(
            14,
            "stale recovery buffer".into(),
            Some(path),
            Some(crate::config::hash_content("recovery-time disk content")),
        );

        assert!(
            AppState::try_apply_recovery(&st, &rc).is_none(),
            "hash mismatch must reject so caller falls through to disk load"
        );
    }

    #[test]
    fn test_recovery_identity_original_bleeding_repro_rejected() {
        use crate::config::RecoveryContent;
        // Acceptance regression for task 106: a leftover recovery file from
        // a previous session named `asdasd` (untitled, tab_id=10) must not
        // bleed into a path-backed tab that happens to be assigned the same
        // tab_id in the new session. Without the path check this exact
        // scenario silently overwrote `task_50_table_inline_formatting.md`
        // with the unrelated text on save.
        let st = session_tab(
            10,
            Some(PathBuf::from("/notes/task_50_table_inline_formatting.md")),
            true,
        );
        let rc = RecoveryContent::new_with_identity(10, "asdasd".into(), None, None);

        assert!(
            AppState::try_apply_recovery(&st, &rc).is_none(),
            "cross-tab bleed must be rejected even when prune_recovery_dir is bypassed"
        );
    }

    /// Regression: restoring a path-backed tab from a divergent recovery
    /// snapshot must anchor `original_content` (and therefore
    /// `disk_content_hash()`) to the actual on-disk text, NOT to the
    /// recovered buffer. Previously, `Tab::with_file(path, recovered)` made
    /// `original_content == content`, so:
    ///
    /// * `is_modified()` returned false right after Restore
    /// * `disk_content_hash()` hashed the recovered buffer
    /// * the next crash snapshot wrote that wrong hash into
    ///   `recovery/<id>.json::original_content_hash`
    /// * on the *following* launch, `try_apply_recovery`'s disk-hash check
    ///   rejected the recovery file and the tab silently fell back to disk
    ///   content — destroying every edit made since the previous recovery.
    ///
    /// This is the exact data-loss path reported as: edit-raw → kill →
    /// recover (ok) → edit-rendered → kill → recover (jumps back to
    /// pre-first-edit, losing both edits).
    #[test]
    fn test_restore_with_divergence_anchors_original_to_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("anchor-to-disk.md");
        let disk_body = "ORIG disk content\n";
        std::fs::write(&path, disk_body).expect("write disk");

        // Session captured one tab whose buffer had unsaved edits on top of disk.
        let recovered_buffer = "ORIG disk content\nedit1 from raw mode\n";

        let mut session_tab_state = session_tab(7, Some(path.clone()), true);
        session_tab_state.display_title = "anchor-to-disk.md".to_string();
        let session = crate::config::SessionState {
            version: 1,
            saved_at: 0,
            clean_shutdown: false,
            tabs: vec![session_tab_state],
            active_tab_index: 0,
            app_mode: crate::config::SessionAppMode::default(),
            zen_mode: false,
        };

        let mut recovered_content = std::collections::HashMap::new();
        recovered_content.insert(
            7,
            crate::config::RecoveryContent::new_with_identity(
                7,
                recovered_buffer.to_string(),
                Some(path.clone()),
                Some(crate::config::hash_content(disk_body)),
            ),
        );

        let result = crate::config::SessionRestoreResult {
            session: Some(session),
            is_crash_recovery: true,
            recovered_content,
            conflicted_tabs: Vec::new(),
            missing_file_tabs: Vec::new(),
        };

        let mut state = AppState::with_settings(Settings::default());
        assert!(state.restore_from_session_result(&result), "should restore");

        let tab = state
            .tabs
            .iter()
            .find(|t| t.path.as_deref() == Some(path.as_path()))
            .expect("restored tab present");

        // Buffer holds the recovered (divergent) text.
        assert_eq!(tab.content, recovered_buffer);

        // Disk anchor must be the actual disk content — NOT the recovered buffer.
        assert_eq!(
            tab.original_content, disk_body,
            "original_content must reflect disk text so disk_content_hash() returns hash(disk)"
        );

        // `is_modified()` must be true so subsequent recovery snapshots and
        // autosaves treat this tab as having unsaved changes.
        assert!(
            tab.is_modified(),
            "restored-with-divergence tab must report as modified"
        );

        // `disk_content_hash()` is the anchor written into future recovery
        // snapshots' `original_content_hash`; it must match the live disk.
        assert_eq!(
            tab.disk_content_hash(),
            Some(crate::config::hash_content(disk_body)),
            "disk_content_hash() must hash the on-disk text, not the recovered buffer"
        );
    }

    /// Regression cycle (the exact user repro): after a recovery cycle, a
    /// fresh edit + new recovery snapshot must still anchor to the live disk
    /// hash. Validates the failure mode by exercising the full
    /// `disk_content_hash()` API path that `save_recovery_content` reads.
    #[test]
    fn test_restore_then_edit_keeps_disk_hash_anchor() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("recover-cycle.md");
        let disk_body = "DISK only\n";
        std::fs::write(&path, disk_body).expect("write disk");

        let recovered_buffer = "DISK only\nfirst raw edit\n";

        let mut session_tab_state = session_tab(11, Some(path.clone()), true);
        session_tab_state.display_title = "recover-cycle.md".to_string();
        let session = crate::config::SessionState {
            version: 1,
            saved_at: 0,
            clean_shutdown: false,
            tabs: vec![session_tab_state],
            active_tab_index: 0,
            app_mode: crate::config::SessionAppMode::default(),
            zen_mode: false,
        };
        let mut recovered_content = std::collections::HashMap::new();
        recovered_content.insert(
            11,
            crate::config::RecoveryContent::new_with_identity(
                11,
                recovered_buffer.to_string(),
                Some(path.clone()),
                Some(crate::config::hash_content(disk_body)),
            ),
        );
        let result = crate::config::SessionRestoreResult {
            session: Some(session),
            is_crash_recovery: true,
            recovered_content,
            conflicted_tabs: Vec::new(),
            missing_file_tabs: Vec::new(),
        };

        let mut state = AppState::with_settings(Settings::default());
        state.restore_from_session_result(&result);

        // Simulate the rendered-mode commit path: tab.content gains another edit.
        let tab_idx = state
            .tabs
            .iter()
            .position(|t| t.path.as_deref() == Some(path.as_path()))
            .expect("restored tab present");
        let post_edit = "DISK only\nfirst raw edit\nsecond rendered edit\n";
        state.tabs[tab_idx].set_content(post_edit.to_string());

        let tab = &state.tabs[tab_idx];
        assert_eq!(tab.content, post_edit);
        // disk_content_hash() is what `save_recovery_content` writes into the
        // recovery file's `original_content_hash`. It MUST stay equal to the
        // live disk hash across the recovery cycle, otherwise the next launch
        // rejects the snapshot and falls back to disk (the data-loss bug).
        assert_eq!(
            tab.disk_content_hash(),
            Some(crate::config::hash_content(disk_body)),
            "disk hash anchor must survive a recover-then-edit cycle"
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // 106.5 — RecoveryConflict banner action handlers
    // ─────────────────────────────────────────────────────────────────────

    /// Build a fresh AppState with one path-backed document tab and a
    /// pre-populated recovery conflict for that tab. Returns (state, tab_id).
    fn state_with_conflict(recovered_buffer: &str, on_disk: &str) -> (AppState, usize) {
        let mut state = AppState::with_settings(Settings::default());
        // Reset to a known-clean tab list — `with_settings` always seeds an
        // empty untitled tab, but we want a single path-backed tab.
        state.tabs.clear();
        let tab_id = state.next_tab_id;
        let path = PathBuf::from("/tmp/conflict-test.md");
        let mut tab = Tab::with_file(tab_id, path, recovered_buffer.to_string());
        tab.detected_encoding = Some("utf-8");
        tab.current_encoding = "utf-8";
        state.tabs.push(tab);
        state.next_tab_id += 1;
        state.active_tab_index = 0;

        state.recovery_conflicts.insert(
            tab_id,
            RecoveryConflict {
                recovered_content: recovered_buffer.to_string(),
                on_disk_content: on_disk.to_string(),
            },
        );

        (state, tab_id)
    }

    #[test]
    fn test_recovery_conflict_keep_recovered_clears_entry_keeps_buffer() {
        let (mut state, tab_id) = state_with_conflict("recovered", "on disk");
        assert!(state.has_recovery_conflict(tab_id));

        let cleared = state.keep_recovered_buffer(tab_id);
        assert!(cleared, "Keep Recovered should report a cleared conflict");
        assert!(!state.has_recovery_conflict(tab_id));

        // Buffer untouched, tab still modified relative to disk so user can save.
        let tab = state.tabs.iter().find(|t| t.id == tab_id).unwrap();
        assert_eq!(tab.content, "recovered");
    }

    #[test]
    fn test_recovery_conflict_reload_from_disk_replaces_buffer_and_marks_saved() {
        let (mut state, tab_id) = state_with_conflict("recovered + edits", "fresh disk content");
        assert!(state.has_recovery_conflict(tab_id));

        let applied = state.apply_reload_from_disk_for_conflict(tab_id);
        assert!(applied, "Reload should report success");
        assert!(!state.has_recovery_conflict(tab_id));

        let tab = state.tabs.iter().find(|t| t.id == tab_id).unwrap();
        assert_eq!(tab.content, "fresh disk content");
        assert!(
            !tab.is_modified(),
            "after reload, tab must not be marked modified"
        );
        assert!(
            tab.pending_cursor_restore.is_some(),
            "cursor should be clamped to new content"
        );
    }

    #[test]
    fn test_recovery_conflict_reload_unknown_tab_id_is_no_op() {
        let mut state = AppState::with_settings(Settings::default());
        // No conflict registered for this id.
        assert!(!state.apply_reload_from_disk_for_conflict(99_999));
        assert!(!state.keep_recovered_buffer(99_999));
    }

    #[test]
    fn test_force_close_tab_clears_recovery_conflict() {
        let (mut state, tab_id) = state_with_conflict("buf", "disk");
        let idx = state.tabs.iter().position(|t| t.id == tab_id).unwrap();
        assert!(state.has_recovery_conflict(tab_id));

        // force_close_tab should drop the conflict so the banner does not
        // resurrect on a future tab whose runtime id collides with `tab_id`.
        let closed = state.force_close_tab(idx);
        assert!(closed);
        assert!(!state.has_recovery_conflict(tab_id));
    }

    #[test]
    fn test_resolve_tab_content_no_recovery_no_disk_returns_none() {
        // When there's no recovery file *and* no disk path, untitled tabs
        // without unsaved content should resolve to an empty Recovered, while
        // untitled tabs with unsaved content but no recovery return None.
        let state = AppState::with_settings(Settings::default());

        let st_clean = session_tab(99, None, false);
        let result = crate::config::SessionRestoreResult {
            session: None,
            is_crash_recovery: false,
            recovered_content: std::collections::HashMap::new(),
            conflicted_tabs: Vec::new(),
            missing_file_tabs: Vec::new(),
        };
        let resolved = state.resolve_tab_content(&st_clean, &result);
        assert!(
            matches!(resolved, Some(ResolvedContent::Recovered(ref s)) if s.is_empty()),
            "untitled clean tab returns empty Recovered"
        );

        let st_dirty = session_tab(100, None, true);
        assert!(
            state.resolve_tab_content(&st_dirty, &result).is_none(),
            "untitled dirty tab without recovery cannot be resolved"
        );
    }

    #[test]
    fn test_tab_is_new_file() {
        // New tab has no path - is a new file
        let tab = Tab::new(0);
        assert!(tab.is_new_file());

        // Tab with file is not new
        let tab_with_file = Tab::with_file(1, PathBuf::from("test.md"), "content".to_string());
        assert!(!tab_with_file.is_new_file());

        // Setting path changes new file status
        let mut tab2 = Tab::new(2);
        assert!(tab2.is_new_file());
        tab2.set_path(PathBuf::from("saved.md"));
        assert!(!tab2.is_new_file());
    }

    #[test]
    fn test_tab_is_empty_untitled() {
        // New empty tab is empty untitled
        let tab = Tab::new(0);
        assert!(tab.is_empty_untitled());

        // New tab with content is not empty untitled
        let mut tab_with_content = Tab::new(1);
        tab_with_content.set_content("hello".to_string());
        assert!(!tab_with_content.is_empty_untitled());

        // Existing empty file is not empty untitled (it has a path)
        let existing_empty = Tab::with_file(2, PathBuf::from("empty.md"), String::new());
        assert!(!existing_empty.is_empty_untitled());

        // Content typed then deleted returns to empty untitled
        let mut tab_typed_deleted = Tab::new(3);
        tab_typed_deleted.set_content("hello".to_string());
        assert!(!tab_typed_deleted.is_empty_untitled());
        tab_typed_deleted.set_content(String::new());
        assert!(tab_typed_deleted.is_empty_untitled());
    }

    #[test]
    fn test_should_show_welcome_on_empty_launch() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(state.should_show_welcome_on_empty_launch());

        state.settings.show_welcome_on_empty_launch = false;
        assert!(!state.should_show_welcome_on_empty_launch());

        state.settings.show_welcome_on_empty_launch = true;
        state.tabs[0].set_content("scratch note".to_string());
        assert!(!state.should_show_welcome_on_empty_launch());
    }

    #[test]
    fn test_open_welcome_on_empty_launch_replaces_placeholder_tab() {
        let mut state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().is_empty_untitled());

        state.open_welcome_on_empty_launch();

        assert_eq!(state.tab_count(), 1);
        assert!(matches!(
            state.active_tab().unwrap().kind,
            TabKind::Special(SpecialTabKind::Welcome)
        ));
    }

    #[test]
    fn test_persisted_untitled_label_rejects_special_tab_titles() {
        let settings_with_icon = format!(
            "{} {}",
            SpecialTabKind::Settings.icon(),
            SpecialTabKind::Settings.title()
        );
        assert!(persisted_untitled_label_from_session(&settings_with_icon).is_none());
        assert!(persisted_untitled_label_from_session("Settings*").is_none());

        let about_with_icon = format!(
            "{} {}",
            SpecialTabKind::About.icon(),
            SpecialTabKind::About.title()
        );
        assert!(persisted_untitled_label_from_session(&about_with_icon).is_none());

        assert_eq!(
            persisted_untitled_label_from_session("My quick note").as_deref(),
            Some("My quick note")
        );
    }

    #[test]
    fn test_capture_session_state_excludes_special_tabs() {
        let mut state = AppState::new();
        state.open_special_tab(SpecialTabKind::Settings);
        let session = state.capture_session_state();
        assert!(
            session
                .tabs
                .iter()
                .all(|t| !is_reserved_special_tab_display_title(&t.display_title)),
            "special tabs must not be written to session state"
        );
    }

    #[test]
    fn test_tab_should_prompt_to_save() {
        let settings = Settings::default();
        let mut settings_classic = Settings::default();
        settings_classic.quick_note_workflow = false;
        let tab_close = SavePromptContext::TabClose;
        let app_exit = SavePromptContext::AppExit;

        // Case 1: New file unmodified - NO prompt
        let new_unmodified = Tab::new(0);
        assert!(!new_unmodified.should_prompt_to_save(&settings, tab_close));

        // Case 2: New file with content - prompt on tab close; no prompt on app exit (quick note)
        let mut new_with_content = Tab::new(1);
        new_with_content.set_content("hello".to_string());
        assert!(!new_with_content.should_prompt_to_save(&settings, app_exit));
        assert!(new_with_content.should_prompt_to_save(&settings, tab_close));
        assert!(new_with_content.should_prompt_to_save(&settings_classic, tab_close));
        assert!(new_with_content.should_prompt_to_save(&settings_classic, app_exit));

        // Case 3: New file typed and deleted - NO prompt (back to empty)
        let mut new_typed_deleted = Tab::new(2);
        new_typed_deleted.set_content("hello".to_string());
        new_typed_deleted.set_content(String::new());
        assert!(!new_typed_deleted.should_prompt_to_save(&settings, tab_close));

        // Case 4: Saved file unmodified - NO prompt
        let saved_unmodified = Tab::with_file(3, PathBuf::from("test.md"), "content".to_string());
        assert!(!saved_unmodified.should_prompt_to_save(&settings, tab_close));

        // Case 5: Saved file modified - prompt
        let mut saved_modified = Tab::with_file(4, PathBuf::from("test.md"), "content".to_string());
        saved_modified.set_content("modified content".to_string());
        assert!(saved_modified.should_prompt_to_save(&settings, tab_close));

        // Case 6: Existing empty file (loaded from disk) unmodified - NO prompt
        let existing_empty = Tab::with_file(5, PathBuf::from("empty.md"), String::new());
        assert!(!existing_empty.should_prompt_to_save(&settings, tab_close));

        // Case 7: Existing empty file modified - prompt
        let mut existing_empty_modified =
            Tab::with_file(6, PathBuf::from("empty.md"), String::new());
        existing_empty_modified.set_content("now has content".to_string());
        assert!(existing_empty_modified.should_prompt_to_save(&settings, tab_close));

        // Case 8: Saved file, content deleted entirely - prompt (modified)
        let mut saved_then_cleared =
            Tab::with_file(7, PathBuf::from("content.md"), "original".to_string());
        saved_then_cleared.set_content(String::new());
        assert!(saved_then_cleared.should_prompt_to_save(&settings, tab_close));

        // Quick note (default): pathless modified — no prompt on exit, prompt on tab close
        let mut qn_tab = Tab::new(10);
        qn_tab.set_content("scratch".to_string());
        assert!(!qn_tab.should_prompt_to_save(&settings, app_exit));
        assert!(qn_tab.should_prompt_to_save(&settings, tab_close));
        // Saved files still prompt when modified
        let mut saved_qn = Tab::with_file(11, PathBuf::from("x.md"), "a".to_string());
        saved_qn.set_content("b".to_string());
        assert!(saved_qn.should_prompt_to_save(&settings, tab_close));
    }

    #[test]
    fn test_tab_title() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.title(), "Untitled");

        tab.set_content("modified".to_string());
        assert_eq!(tab.title(), "Untitled*");

        tab.path = Some(PathBuf::from("/test/document.md"));
        assert_eq!(tab.title(), "document.md*");

        tab.mark_saved();
        assert_eq!(tab.title(), "document.md");
    }

    #[test]
    fn test_tab_undo_redo() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.break_undo_group();
        tab.set_content("second".to_string());
        tab.break_undo_group();
        tab.set_content("third".to_string());

        assert!(tab.can_undo());
        assert!(!tab.can_redo());

        tab.undo();
        assert_eq!(tab.content, "second");
        assert!(tab.can_redo());

        tab.undo();
        assert_eq!(tab.content, "first");

        tab.redo();
        assert_eq!(tab.content, "second");
    }

    #[test]
    fn test_tab_source_epoch_starts_at_zero() {
        let tab = Tab::new(0);
        assert_eq!(tab.source_epoch(), 0);
    }

    #[test]
    fn test_tab_bump_source_epoch_saturates() {
        let mut tab = Tab::new(0);
        tab.bump_source_epoch();
        assert_eq!(tab.source_epoch(), 1);
        tab.source_epoch = u64::MAX;
        tab.bump_source_epoch();
        assert_eq!(tab.source_epoch(), u64::MAX);
    }

    #[test]
    fn test_tab_set_content_bumps_source_epoch() {
        let mut tab = Tab::new(0);
        tab.set_content("hello".to_string());
        assert_eq!(tab.source_epoch(), 1);
        tab.set_content("hello".to_string());
        assert_eq!(tab.source_epoch(), 1);
    }

    #[test]
    fn test_tab_undo_redo_bumps_source_epoch() {
        let mut tab = Tab::new(0);
        tab.set_content("a".to_string());
        let after_edit = tab.source_epoch();
        tab.undo();
        assert_eq!(tab.source_epoch(), after_edit + 1);
        tab.redo();
        assert_eq!(tab.source_epoch(), after_edit + 2);
    }

    #[test]
    fn test_tab_record_edit_from_snapshot_rendered_no_epoch_bump() {
        let mut tab = Tab::new(0);
        tab.prepare_undo_snapshot_hashed();
        tab.content = "rendered edit".to_string();
        tab.record_edit_from_snapshot();
        assert_eq!(tab.source_epoch(), 0);
    }

    #[test]
    fn test_tab_apply_rendered_commit_undo_entries_one_logical_step() {
        use crate::markdown::rendered_commit_undo::PendingRenderedCommitUndo;

        let mut tab = Tab::new(0);
        tab.content = "# Hello".to_string();
        tab.apply_rendered_commit_undo_entries([PendingRenderedCommitUndo {
            pre_commit_snapshot: "# Hi".to_string(),
            post_commit_snapshot: "# Hello".to_string(),
            break_group_before: false,
        }]);
        assert!(tab.can_undo());
        tab.undo();
        assert_eq!(tab.content, "# Hi");
    }

    #[test]
    fn test_tab_apply_rendered_commit_undo_entries_chained_blocks() {
        use crate::markdown::rendered_commit_undo::PendingRenderedCommitUndo;

        let mut tab = Tab::new(0);
        tab.content = "C".to_string();
        tab.apply_rendered_commit_undo_entries([
            PendingRenderedCommitUndo {
                pre_commit_snapshot: "A".to_string(),
                post_commit_snapshot: "B".to_string(),
                break_group_before: true,
            },
            PendingRenderedCommitUndo {
                pre_commit_snapshot: "B".to_string(),
                post_commit_snapshot: "C".to_string(),
                break_group_before: true,
            },
        ]);
        assert_eq!(tab.undo_count(), 2);
        tab.undo();
        assert_eq!(tab.content, "B");
        tab.undo();
        assert_eq!(tab.content, "A");
    }

    #[test]
    fn test_tab_record_external_edit_from_snapshot_bumps_epoch() {
        let mut tab = Tab::new(0);
        tab.prepare_undo_snapshot_hashed();
        tab.content = "raw edit".to_string();
        tab.record_external_edit_from_snapshot();
        assert_eq!(tab.source_epoch(), 1);
    }

    #[test]
    fn test_tab_undo_clears_redo_on_edit() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.break_undo_group();
        tab.set_content("second".to_string());

        tab.undo();
        assert!(tab.can_redo());

        tab.set_content("new edit".to_string());
        assert!(!tab.can_redo());
    }

    #[test]
    fn test_tab_record_edit() {
        let mut tab = Tab::new(0);

        let old_content = tab.content.clone();
        tab.content = "first edit".to_string();
        tab.record_edit(old_content, 0);

        assert!(tab.can_undo());
        assert_eq!(tab.undo_count(), 1);

        tab.break_undo_group();

        let old_content = tab.content.clone();
        tab.content = "second edit".to_string();
        tab.record_edit(old_content, 5);

        assert_eq!(tab.undo_count(), 2);
        assert!(!tab.can_redo());

        let cursor = tab.undo();
        assert_eq!(tab.content, "first edit");
        assert!(tab.can_redo());
        assert!(cursor.is_some());
    }

    #[test]
    fn test_tab_record_edit_no_change() {
        let mut tab = Tab::new(0);
        tab.content = "same content".to_string();

        let old_content = tab.content.clone();
        tab.record_edit(old_content, 0);

        assert!(!tab.can_undo());
        assert_eq!(tab.undo_count(), 0);
    }

    #[test]
    fn test_tab_record_edit_clears_redo() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.break_undo_group();
        tab.set_content("second".to_string());
        tab.undo();

        assert!(tab.can_redo());

        let old_content = tab.content.clone();
        tab.content = "new edit".to_string();
        tab.record_edit(old_content, 0);

        assert!(!tab.can_redo());
    }

    #[test]
    fn test_tab_undo_redo_counts() {
        let mut tab = Tab::new(0);

        assert_eq!(tab.undo_count(), 0);
        assert_eq!(tab.redo_count(), 0);

        tab.set_content("first".to_string());
        tab.break_undo_group();
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 0);

        tab.set_content("second".to_string());
        assert_eq!(tab.undo_count(), 2);

        tab.undo();
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 1);

        tab.undo();
        assert_eq!(tab.undo_count(), 0);
        assert_eq!(tab.redo_count(), 2);

        tab.redo();
        assert_eq!(tab.undo_count(), 1);
        assert_eq!(tab.redo_count(), 1);
    }

    #[test]
    fn test_tab_max_undo_groups() {
        let mut tab = Tab::new(0);
        // Default max groups is 500

        for i in 0..505 {
            tab.break_undo_group();
            tab.set_content(format!("edit {}", i));
        }

        assert_eq!(tab.undo_count(), 500);

        for _ in 0..500 {
            tab.undo();
        }

        assert_eq!(tab.content, "edit 4");
        assert!(!tab.can_undo());
    }

    #[test]
    fn test_tab_to_tab_info() {
        let mut tab = Tab::with_file(1, PathBuf::from("/test/file.md"), "content".to_string());
        tab.cursor_position = (10, 5);
        tab.scroll_offset = 100.0;
        tab.view_mode = ViewMode::Rendered;
        tab.split_ratio = 0.6;

        let info = tab.to_tab_info();
        assert_eq!(info.path, tab.path);
        assert!(!info.modified);
        assert_eq!(info.cursor_position, (10, 5));
        assert_eq!(info.scroll_offset, 100.0);
        assert_eq!(info.view_mode, ViewMode::Rendered);
        assert_eq!(info.split_ratio, 0.6);
    }

    #[test]
    fn test_tab_view_mode_toggle() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.view_mode, ViewMode::Raw);

        // Raw → Split
        let new_mode = tab.toggle_view_mode();
        assert_eq!(new_mode, ViewMode::Split);
        assert_eq!(tab.view_mode, ViewMode::Split);

        // Split → Rendered
        let new_mode = tab.toggle_view_mode();
        assert_eq!(new_mode, ViewMode::Rendered);
        assert_eq!(tab.view_mode, ViewMode::Rendered);

        // Rendered → Raw
        let new_mode = tab.toggle_view_mode();
        assert_eq!(new_mode, ViewMode::Raw);
        assert_eq!(tab.view_mode, ViewMode::Raw);
    }

    #[test]
    fn test_tab_split_ratio() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.get_split_ratio(), 0.5); // Default

        tab.set_split_ratio(0.7);
        assert_eq!(tab.get_split_ratio(), 0.7);

        // Test clamping
        tab.set_split_ratio(0.1);
        assert_eq!(tab.get_split_ratio(), 0.2); // Clamped to min

        tab.set_split_ratio(0.9);
        assert_eq!(tab.get_split_ratio(), 0.8); // Clamped to max
    }

    #[test]
    fn test_tab_view_mode_get_set() {
        let mut tab = Tab::new(0);
        assert_eq!(tab.get_view_mode(), ViewMode::Raw);

        tab.set_view_mode(ViewMode::Rendered);
        assert_eq!(tab.get_view_mode(), ViewMode::Rendered);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // AppState Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_appstate_new_has_one_tab() {
        let state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0);
    }

    #[test]
    fn test_appstate_with_custom_settings() {
        let mut settings = Settings::default();
        settings.theme = Theme::Dark;
        settings.font_size = 18.0;

        let state = AppState::with_settings(settings);
        assert_eq!(state.settings.theme, Theme::Dark);
        assert_eq!(state.settings.font_size, 18.0);
    }

    #[test]
    fn test_appstate_new_tab() {
        let mut state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);

        let index = state.new_tab();
        assert_eq!(state.tab_count(), 2);
        assert_eq!(state.active_tab_index(), index);
    }

    #[test]
    fn test_appstate_set_active_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        state.new_tab();

        assert!(state.set_active_tab(1));
        assert_eq!(state.active_tab_index(), 1);

        assert!(!state.set_active_tab(10)); // Invalid index
        assert_eq!(state.active_tab_index(), 1); // Unchanged
    }

    #[test]
    fn test_appstate_force_close_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        state.new_tab();
        assert_eq!(state.tab_count(), 3);

        state.force_close_tab(1);
        assert_eq!(state.tab_count(), 2);
    }

    #[test]
    fn test_appstate_close_last_tab_creates_new() {
        let mut state = AppState::with_settings(Settings::default());
        assert_eq!(state.tab_count(), 1);

        state.force_close_tab(0);
        // Should have created a new empty tab
        assert_eq!(state.tab_count(), 1);
    }

    #[test]
    fn test_appstate_active_tab_mut() {
        let mut state = AppState::with_settings(Settings::default());
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("Hello, World!".to_string());
        }

        assert_eq!(state.active_tab().unwrap().content, "Hello, World!");
    }

    #[test]
    fn test_appstate_has_unsaved_changes() {
        let mut settings = Settings::default();
        settings.quick_note_workflow = false;
        let mut state = AppState::with_settings(settings);
        assert!(!state.has_unsaved_changes());

        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("modified".to_string());
        }
        assert!(state.has_unsaved_changes());
    }

    #[test]
    fn test_appstate_update_settings() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(!state.settings_dirty);

        state.update_settings(|s| {
            s.theme = Theme::Dark;
        });

        assert_eq!(state.settings.theme, Theme::Dark);
        assert!(state.settings_dirty);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UI State Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ui_state_default() {
        let ui = UiState::default();
        assert!(!ui.show_settings);
        assert!(!ui.show_file_dialog);
        assert!(!ui.show_confirm_dialog);
        assert!(!ui.show_code_execution_consent_dialog);
        assert!(ui.pending_code_run.is_none());
        assert!(ui.status_message.is_none());
    }

    #[test]
    fn test_pending_code_run_slot_stashes_and_clears() {
        let mut ui = UiState::default();
        let pending = PendingCodeRun {
            code: "echo hi".to_string(),
            language: "bash".to_string(),
            cwd: None,
            timeout_secs: 30,
            block_id: egui::Id::new(42_u64),
        };
        ui.pending_code_run = Some(pending.clone());
        assert_eq!(ui.pending_code_run, Some(pending));
        ui.pending_code_run = None;
        assert!(ui.pending_code_run.is_none());
    }

    #[test]
    fn test_appstate_toggle_settings() {
        let mut state = AppState::with_settings(Settings::default());
        assert!(!state.ui.show_settings);

        state.toggle_settings();
        assert!(state.ui.show_settings);

        state.toggle_settings();
        assert!(!state.ui.show_settings);
    }

    #[test]
    fn test_appstate_set_status() {
        let mut state = AppState::with_settings(Settings::default());
        state.set_status("File saved");
        assert_eq!(state.ui.status_message, Some("File saved".to_string()));

        state.clear_status();
        assert!(state.ui.status_message.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Event Handling Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_appstate_request_exit_clean() {
        let mut state = AppState::with_settings(Settings::default());
        // No modifications, should exit immediately
        assert!(state.request_exit());
    }

    #[test]
    fn test_appstate_request_exit_with_changes() {
        let mut settings = Settings::default();
        settings.quick_note_workflow = false;
        let mut state = AppState::with_settings(settings);
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("modified".to_string());
        }

        // Has modifications, should show confirmation
        assert!(!state.request_exit());
        assert!(state.ui.show_confirm_dialog);
        assert_eq!(state.ui.pending_action, Some(PendingAction::Exit));
    }

    #[test]
    fn test_appstate_close_new_unmodified_tab_no_prompt() {
        let mut state = AppState::with_settings(Settings::default());
        // Initial tab is a new unmodified tab
        assert_eq!(state.tab_count(), 1);

        // Create another tab so we can test closing the first
        state.new_tab();
        assert_eq!(state.tab_count(), 2);

        // Close the first tab (new, unmodified) - should close without prompt
        let closed = state.close_tab(0);
        assert!(closed, "New unmodified tab should close without prompt");
        assert_eq!(state.tab_count(), 1);
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_appstate_quick_note_exit_without_prompt() {
        let mut state = AppState::with_settings(Settings::default());
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("scratch".to_string());
        }
        assert!(state.request_exit());
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_appstate_quick_note_close_modified_untitled_with_prompt() {
        let mut state = AppState::with_settings(Settings::default());
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("x".to_string());
        }
        state.new_tab();
        assert!(!state.close_tab(0));
        assert!(state.ui.show_confirm_dialog);
        assert_eq!(state.ui.pending_action, Some(PendingAction::CloseTab(0)));
    }

    #[test]
    fn test_appstate_close_new_modified_tab_prompts() {
        let mut settings = Settings::default();
        settings.quick_note_workflow = false;
        let mut state = AppState::with_settings(settings);

        // Modify the initial tab
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("user typed something".to_string());
        }

        // Create another tab so closing doesn't auto-create a new one
        state.new_tab();
        assert_eq!(state.tab_count(), 2);

        // Try to close the modified tab - should prompt
        let closed = state.close_tab(0);
        assert!(!closed, "Modified tab should show prompt, not close");
        assert!(state.ui.show_confirm_dialog);
        assert_eq!(state.ui.pending_action, Some(PendingAction::CloseTab(0)));
    }

    #[test]
    fn test_appstate_close_empty_typed_deleted_tab_no_prompt() {
        let mut state = AppState::with_settings(Settings::default());

        // Type something then delete it
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("temporary content".to_string());
            tab.set_content(String::new()); // Delete all content
        }

        // Create another tab
        state.new_tab();
        assert_eq!(state.tab_count(), 2);

        // Close the first tab - should close without prompt (back to empty untitled)
        let closed = state.close_tab(0);
        assert!(closed, "Empty untitled tab should close without prompt");
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_appstate_quit_with_mixed_tabs() {
        let mut settings = Settings::default();
        settings.quick_note_workflow = false;
        let mut state = AppState::with_settings(settings);

        // Tab 0: new unmodified (initial)
        // Tab 1: new with content (should trigger prompt)
        state.new_tab();
        if let Some(tab) = state.active_tab_mut() {
            tab.set_content("modified content".to_string());
        }

        // Tab 2: new unmodified
        state.new_tab();

        assert_eq!(state.tab_count(), 3);

        // has_unsaved_changes should be true because tab 1 has content
        assert!(state.has_unsaved_changes());

        // Quit should show prompt
        assert!(!state.request_exit());
        assert!(state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_appstate_quit_with_only_empty_untitled_tabs() {
        let mut state = AppState::with_settings(Settings::default());

        // Create multiple empty untitled tabs
        state.new_tab();
        state.new_tab();
        assert_eq!(state.tab_count(), 3);

        // None of them should be considered as having unsaved changes
        assert!(!state.has_unsaved_changes());

        // Quit should proceed without prompt
        assert!(state.request_exit());
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_appstate_handle_confirmed_close_tab() {
        let mut state = AppState::with_settings(Settings::default());
        state.new_tab();
        assert_eq!(state.tab_count(), 2);

        state.ui.pending_action = Some(PendingAction::CloseTab(0));
        state.handle_confirmed_action();

        assert_eq!(state.tab_count(), 1);
        assert!(state.ui.pending_action.is_none());
    }

    #[test]
    fn test_appstate_cancel_pending_action() {
        let mut state = AppState::with_settings(Settings::default());
        state.ui.pending_action = Some(PendingAction::Exit);
        state.ui.show_confirm_dialog = true;

        state.cancel_pending_action();

        assert!(state.ui.pending_action.is_none());
        assert!(!state.ui.show_confirm_dialog);
    }

    #[test]
    fn test_pending_action_equality() {
        assert_eq!(PendingAction::Exit, PendingAction::Exit);
        assert_eq!(PendingAction::CloseTab(1), PendingAction::CloseTab(1));
        assert_ne!(PendingAction::CloseTab(1), PendingAction::CloseTab(2));
        assert_ne!(PendingAction::Exit, PendingAction::NewDocument);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Session Restoration Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_from_tab_info() {
        let info = TabInfo {
            path: Some(PathBuf::from("/test/file.md")),
            modified: false,
            cursor_position: (10, 5),
            scroll_offset: 100.0,
            view_mode: ViewMode::Rendered, // Test restoring rendered mode
            split_ratio: 0.6,              // Test restoring split ratio
        };
        let content = "# Test Content".to_string();

        let tab = Tab::from_tab_info(42, &info, content.clone());

        assert_eq!(tab.id, 42);
        assert_eq!(tab.path, info.path);
        assert_eq!(tab.content, content);
        assert_eq!(tab.cursor_position, (10, 5));
        assert_eq!(tab.scroll_offset, 100.0);
        assert_eq!(tab.view_mode, ViewMode::Rendered); // View mode restored
        assert_eq!(tab.split_ratio, 0.6); // Split ratio restored
        assert!(!tab.is_modified()); // Content matches original
    }

    #[test]
    fn test_restore_session_tabs_empty_settings() {
        // When last_open_tabs is empty, should create one empty tab
        let settings = Settings::default();
        let state = AppState::with_settings(settings);

        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_with_missing_file() {
        // When a saved tab's file no longer exists, it should be skipped
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: Some(PathBuf::from("/nonexistent/file/that/does/not/exist.md")),
            modified: false,
            cursor_position: (0, 0),
            scroll_offset: 0.0,
            view_mode: ViewMode::Raw,
            split_ratio: 0.5,
        }];

        let state = AppState::with_settings(settings);

        // Should fall back to creating an empty tab since the file doesn't exist
        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_skips_unsaved() {
        // Tabs without a path (unsaved) should be skipped during restore
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: None, // Unsaved tab
            modified: true,
            cursor_position: (5, 10),
            scroll_offset: 50.0,
            view_mode: ViewMode::Raw,
            split_ratio: 0.5,
        }];

        let state = AppState::with_settings(settings);

        // Should fall back to creating an empty tab since unsaved tabs are skipped
        assert_eq!(state.tab_count(), 1);
        assert!(state.active_tab().unwrap().path.is_none());
    }

    #[test]
    fn test_restore_session_tabs_active_index_clamped() {
        // Active tab index should be clamped to valid range
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![]; // No tabs to restore
        settings.active_tab_index = 100; // Invalid index

        let state = AppState::with_settings(settings);

        // Should create one empty tab and active_tab_index should be 0
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0);
    }

    #[test]
    fn test_restore_session_tabs_with_temp_file() {
        use std::io::Write;

        // Create a temporary file to test actual restoration
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_restore.md");
        let test_content = "# Test Restored Content\n\nThis is a test.";

        // Write the test file
        let mut file = std::fs::File::create(&temp_file).expect("Failed to create temp file");
        file.write_all(test_content.as_bytes())
            .expect("Failed to write temp file");
        drop(file);

        // Set up settings with this file (with Rendered view mode)
        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: Some(temp_file.clone()),
            modified: false,
            cursor_position: (1, 5),
            scroll_offset: 25.0,
            view_mode: ViewMode::Rendered, // Test restoring view mode
            split_ratio: 0.5,
        }];
        settings.active_tab_index = 0;

        let state = AppState::with_settings(settings);

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        // Verify restoration
        assert_eq!(state.tab_count(), 1);
        let tab = state.active_tab().unwrap();
        assert_eq!(tab.path, Some(temp_file));
        assert_eq!(tab.content, test_content);
        assert_eq!(tab.cursor_position, (1, 5));
        assert_eq!(tab.scroll_offset, 25.0);
        assert_eq!(tab.view_mode, ViewMode::Rendered); // View mode restored
        assert!(!tab.is_modified());
    }

    #[test]
    fn test_restore_multiple_tabs_with_temp_files() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file1 = temp_dir.join("ferrite_test_restore1.md");
        let temp_file2 = temp_dir.join("ferrite_test_restore2.md");

        // Write test files
        std::fs::File::create(&temp_file1)
            .unwrap()
            .write_all(b"# File 1")
            .unwrap();
        std::fs::File::create(&temp_file2)
            .unwrap()
            .write_all(b"# File 2")
            .unwrap();

        let mut settings = Settings::default();
        settings.last_open_tabs = vec![
            TabInfo {
                path: Some(temp_file1.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Raw, // First tab in raw mode
                split_ratio: 0.5,
            },
            TabInfo {
                path: Some(temp_file2.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Rendered, // Second tab in rendered mode
                split_ratio: 0.5,
            },
        ];
        settings.active_tab_index = 1; // Second tab active

        let state = AppState::with_settings(settings);

        // Clean up
        let _ = std::fs::remove_file(&temp_file1);
        let _ = std::fs::remove_file(&temp_file2);

        // Verify
        assert_eq!(state.tab_count(), 2);
        assert_eq!(state.active_tab_index(), 1);
        assert_eq!(state.tab(0).unwrap().content, "# File 1");
        assert_eq!(state.tab(0).unwrap().view_mode, ViewMode::Raw);
        assert_eq!(state.tab(1).unwrap().content, "# File 2");
        assert_eq!(state.tab(1).unwrap().view_mode, ViewMode::Rendered);
    }

    #[test]
    fn test_restore_partial_tabs_missing_file() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_restore_partial.md");

        // Write only one test file
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Existing File")
            .unwrap();

        let mut settings = Settings::default();
        settings.last_open_tabs = vec![
            TabInfo {
                path: Some(PathBuf::from("/nonexistent/file.md")),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Raw,
                split_ratio: 0.5,
            },
            TabInfo {
                path: Some(temp_file.clone()),
                modified: false,
                cursor_position: (0, 0),
                scroll_offset: 0.0,
                view_mode: ViewMode::Rendered,
                split_ratio: 0.5,
            },
        ];
        settings.active_tab_index = 1;

        let state = AppState::with_settings(settings);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        // Only the existing file should be restored
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_index(), 0); // Clamped since only 1 tab
        assert_eq!(state.active_tab().unwrap().content, "# Existing File");
        assert_eq!(state.active_tab().unwrap().view_mode, ViewMode::Rendered);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Open File with Focus Control Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_open_file_restores_saved_view_mode() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_open_view_mode.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Test")
            .unwrap();

        let mut settings = Settings::default();
        settings.last_open_tabs = vec![TabInfo {
            path: Some(temp_file.clone()),
            view_mode: ViewMode::Split,
            split_ratio: 0.65,
            ..TabInfo::default()
        }];

        let mut state = AppState::with_settings(settings);
        state
            .open_file_with_focus(temp_file.clone(), true, None)
            .unwrap();
        let tab = state.active_tab().unwrap();
        assert_eq!(tab.view_mode, ViewMode::Split);
        assert_eq!(tab.split_ratio, 0.65);

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_close_tab_persists_view_mode_for_reopen() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_close_view_mode.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Test")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        state
            .open_file_with_focus(temp_file.clone(), true, None)
            .unwrap();
        state.active_tab_mut().unwrap().view_mode = ViewMode::Split;
        state.active_tab_mut().unwrap().split_ratio = 0.7;
        state.force_close_tab(0);

        state
            .open_file_with_focus(temp_file.clone(), true, None)
            .unwrap();
        let tab = state.active_tab().unwrap();
        assert_eq!(tab.view_mode, ViewMode::Split);
        assert_eq!(tab.split_ratio, 0.7);

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_open_file_with_focus_true() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_open_focus_true.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Test Content")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        let initial_tab_count = state.tab_count();

        // Open with focus=true
        let result = state.open_file_with_focus(temp_file.clone(), true, None);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        let new_index = result.unwrap();
        assert_eq!(state.tab_count(), initial_tab_count + 1);
        assert_eq!(state.active_tab_index(), new_index); // Should be focused
        assert_eq!(state.active_tab().unwrap().content, "# Test Content");
    }

    #[test]
    fn test_open_file_with_focus_false() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_open_focus_false.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Background File")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        let initial_active_index = state.active_tab_index();
        let initial_tab_count = state.tab_count();

        // Open with focus=false
        let result = state.open_file_with_focus(temp_file.clone(), false, None);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        let new_index = result.unwrap();
        assert_eq!(state.tab_count(), initial_tab_count + 1);
        // Active tab should NOT have changed
        assert_eq!(state.active_tab_index(), initial_active_index);
        // But the file should be in a new tab
        assert_eq!(state.tab(new_index).unwrap().content, "# Background File");
    }

    #[test]
    fn test_open_file_already_open_with_focus() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_already_open.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Already Open")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());

        // Open the file first
        let first_result = state.open_file_with_focus(temp_file.clone(), true, None);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        assert_ne!(state.active_tab_index(), first_index);

        // Open the same file again with focus=true
        let second_result = state.open_file_with_focus(temp_file.clone(), true, None);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(second_result.is_ok());
        let second_index = second_result.unwrap();
        // Should return the same index
        assert_eq!(first_index, second_index);
        // Should have switched focus to the existing tab
        assert_eq!(state.active_tab_index(), first_index);
    }

    #[test]
    fn test_open_file_already_open_without_focus() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_already_open_no_focus.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Already Open No Focus")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());

        // Open the file first
        let first_result = state.open_file_with_focus(temp_file.clone(), true, None);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        let new_tab_index = state.active_tab_index();
        assert_ne!(new_tab_index, first_index);

        // Open the same file again with focus=false
        let second_result = state.open_file_with_focus(temp_file.clone(), false, None);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(second_result.is_ok());
        let second_index = second_result.unwrap();
        // Should return the same index
        assert_eq!(first_index, second_index);
        // Should NOT have switched focus
        assert_eq!(state.active_tab_index(), new_tab_index);
    }

    #[test]
    fn test_open_file_updates_recent_files() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("ferrite_test_recent_update.md");
        std::fs::File::create(&temp_file)
            .unwrap()
            .write_all(b"# Recent Test")
            .unwrap();

        let mut state = AppState::with_settings(Settings::default());
        assert!(state.settings.recent_files.is_empty());

        // Open file (either focus mode should update recent files)
        let result = state.open_file_with_focus(temp_file.clone(), false, None);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        // Recent files should now contain the opened file
        assert!(!state.settings.recent_files.is_empty());
        assert_eq!(state.settings.recent_files[0], temp_file);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Backlink Index Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_normalize_filename() {
        assert_eq!(normalize_filename("MyNote"), "mynote");
        assert_eq!(normalize_filename("MyNote.md"), "mynote");
        assert_eq!(normalize_filename("MyNote.markdown"), "mynote");
        assert_eq!(normalize_filename("  spaces  "), "spaces");
        assert_eq!(normalize_filename("UPPER.MD"), "upper");
    }

    #[test]
    fn test_extract_wikilinks_from_content() {
        let content = "Check [[note-a]] and [[note-b|Display Text]] for details.";
        let links = extract_links_from_content(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "note-a");
        assert_eq!(links[1], "note-b");
    }

    #[test]
    fn test_extract_standard_links_from_content() {
        let content = "See [link](other.md) and [another](path/to/file.md) here.";
        let links = extract_links_from_content(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "other.md");
        assert_eq!(links[1], "file.md");
    }

    #[test]
    fn test_extract_links_ignores_urls() {
        let content = "Visit [link](https://example.com) and [local](note.md).";
        let links = extract_links_from_content(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "note.md");
    }

    #[test]
    fn test_extract_links_ignores_anchors() {
        let content = "Jump to [section](#heading) and [file](doc.md).";
        let links = extract_links_from_content(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "doc.md");
    }

    #[test]
    fn test_extract_mixed_links() {
        let content = "[[wiki-link]] and [text](local.md) and [[another|display]]";
        let links = extract_links_from_content(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"wiki-link".to_string()));
        assert!(links.contains(&"local.md".to_string()));
        assert!(links.contains(&"another".to_string()));
    }

    #[test]
    fn test_extract_unclosed_wikilink() {
        let content = "This [[unclosed stays as text";
        let links = extract_links_from_content(content);
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_empty_wikilink() {
        let content = "Empty [[]] wikilink";
        let links = extract_links_from_content(content);
        assert!(links.is_empty());
    }

    #[test]
    fn test_backlink_index_get_and_build() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join("ferrite_backlink_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Create test files
        let file_a = temp_dir.join("note-a.md");
        let file_b = temp_dir.join("note-b.md");
        let file_c = temp_dir.join("note-c.md");

        std::fs::File::create(&file_a)
            .unwrap()
            .write_all(b"# Note A\nLinks to [[note-b]] here.")
            .unwrap();
        std::fs::File::create(&file_b)
            .unwrap()
            .write_all(b"# Note B\nStandalone note.")
            .unwrap();
        std::fs::File::create(&file_c)
            .unwrap()
            .write_all(b"# Note C\nAlso links to [[note-b]] and [text](note-a.md).")
            .unwrap();

        let files = vec![file_a.clone(), file_b.clone(), file_c.clone()];

        let mut index = BacklinkIndex::new();
        index.build_from_files(&files);

        assert!(index.is_built);
        assert_eq!(index.file_count, 3);

        // note-b should have 2 backlinks (from note-a and note-c)
        let backlinks_b = index.get_backlinks("note-b");
        assert_eq!(backlinks_b.len(), 2);

        // note-a should have 1 backlink (from note-c via standard link)
        let backlinks_a = index.get_backlinks("note-a");
        assert_eq!(backlinks_a.len(), 1);
        assert_eq!(backlinks_a[0].source_path, file_c);

        // note-c should have 0 backlinks
        let backlinks_c = index.get_backlinks("note-c");
        assert!(backlinks_c.is_empty());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backlink_index_update_file() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join("ferrite_backlink_update_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let file_a = temp_dir.join("note-a.md");
        let file_b = temp_dir.join("note-b.md");

        std::fs::File::create(&file_a)
            .unwrap()
            .write_all(b"# Note A\nLinks to [[note-b]].")
            .unwrap();
        std::fs::File::create(&file_b)
            .unwrap()
            .write_all(b"# Note B")
            .unwrap();

        let files = vec![file_a.clone(), file_b.clone()];

        let mut index = BacklinkIndex::new();
        index.build_from_files(&files);

        assert_eq!(index.get_backlinks("note-b").len(), 1);

        // Update file_a to remove the link
        std::fs::File::create(&file_a)
            .unwrap()
            .write_all(b"# Note A\nNo more links.")
            .unwrap();

        index.update_file(&file_a);

        // note-b should now have 0 backlinks
        assert_eq!(index.get_backlinks("note-b").len(), 0);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backlink_scan_on_demand() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join("ferrite_backlink_ondemand_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let file_a = temp_dir.join("note-a.md");
        let file_b = temp_dir.join("note-b.md");
        let file_c = temp_dir.join("note-c.md");

        std::fs::File::create(&file_a)
            .unwrap()
            .write_all(b"# Note A\nLinks to [[note-c]].")
            .unwrap();
        std::fs::File::create(&file_b)
            .unwrap()
            .write_all(b"# Note B\nAlso links to [[note-c|See C]].")
            .unwrap();
        std::fs::File::create(&file_c)
            .unwrap()
            .write_all(b"# Note C\nTarget file.")
            .unwrap();

        let files = vec![file_a.clone(), file_b.clone(), file_c.clone()];

        let backlinks = BacklinkIndex::scan_on_demand("note-c", &files, Some(&file_c));
        assert_eq!(backlinks.len(), 2);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
