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
use crate::ui::TabPipelineState;
use crate::vcs::GitService;
use crate::workspaces::{filter_events, AppMode, Workspace, WorkspaceEvent, WorkspaceWatcher};
use log::{debug, info, warn};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

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

    /// Get a display name for this file type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Unknown => "Unknown",
        }
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
        let new_anchor = (self.anchor as isize + delta).max(0) as usize;
        let new_head = (self.head as isize + delta).max(0) as usize;
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
        let idx = self.primary_index.min(self.selections.len().saturating_sub(1));
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
                sel.anchor = (sel.anchor as isize + delta).max(0) as usize;
            }
            if sel.head >= pos {
                sel.head = (sel.head as isize + delta).max(0) as usize;
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
            let new_end = self.end_line as isize + delta;
            if new_end < self.start_line as isize {
                // Region collapsed to invalid state
                return false;
            }
            self.end_line = new_end as usize;
            return true;
        }

        // Edit is before this region, shift both lines
        let new_start = self.start_line as isize + delta;
        let new_end = self.end_line as isize + delta;

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
        self.regions.retain_mut(|r| r.adjust_for_edit(edit_line, delta));
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

/// An entry in the undo/redo stack.
///
/// Stores both the content state and cursor position so that undo/redo
/// can restore the cursor to the correct position.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// The content at this point in history
    pub content: String,
    /// The cursor position (character index) at this point
    pub cursor_position: usize,
}

impl UndoEntry {
    /// Create a new undo entry with the given content and cursor position.
    pub fn new(content: String, cursor_position: usize) -> Self {
        Self {
            content,
            cursor_position,
        }
    }
}

/// Runtime state for an open tab.
/// Threshold in bytes above which a file is considered "large" and gets memory optimizations.
/// Large files:
/// - Use hash-based modification detection instead of storing full original_content
/// - Have a smaller undo stack (10 entries instead of 100)
/// - Clear original_bytes after initial load to save memory
pub const LARGE_FILE_THRESHOLD: usize = 1_000_000; // 1MB

/// Maximum undo stack size for large files (reduced from 100 to save memory).
pub const LARGE_FILE_MAX_UNDO: usize = 10;

///
/// This struct holds the complete state of an open document tab,
/// including content and editing state. Different from `TabInfo` which
/// is used for persistence/session restoration.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique identifier for this tab
    pub id: usize,
    /// File path (None for unsaved/new documents)
    pub path: Option<PathBuf>,
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
    /// View mode for this tab (raw or rendered)
    pub view_mode: ViewMode,
    /// Undo history stack (stores content + cursor position)
    undo_stack: Vec<UndoEntry>,
    /// Redo history stack (stores content + cursor position)
    redo_stack: Vec<UndoEntry>,
    /// Maximum undo history size
    max_undo_size: usize,
    /// Content version counter - incremented on undo/redo to signal
    /// external content changes to the editor widget
    content_version: u64,
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
    /// Pending undo state - snapshot of content before the current edit session.
    /// Used to avoid cloning content every frame. Only clones when:
    /// 1. First frame after file open (to prepare for first edit)
    /// 2. After an edit is recorded (to prepare for next edit)
    /// This dramatically reduces memory allocation for large files.
    pending_undo_state: Option<UndoEntry>,
}

impl Tab {
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
            path: None,
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
            view_mode: ViewMode::Raw, // New documents default to raw mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size: 100,
            content_version: 0,
            file_type: FileType::Markdown, // New tabs default to markdown
            needs_focus: true, // Auto-focus new tabs
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
            pending_undo_state: None, // Will be populated on first frame
        }
    }

    /// Create a new empty tab with settings-based defaults.
    ///
    /// # Arguments
    /// * `id` - Unique tab identifier
    /// * `auto_save_default` - Whether auto-save is enabled by default
    /// * `default_view_mode` - Default view mode for new tabs (Raw, Rendered, or Split)
    pub fn new_with_settings(id: usize, auto_save_default: bool, default_view_mode: ViewMode) -> Self {
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
        let (original_content, original_content_hash, max_undo_size) = if is_large_file {
            log::info!(
                "Opening large file ({} bytes): using hash-based modification detection",
                content.len()
            );
            (String::new(), Some(Self::compute_content_hash(&content)), LARGE_FILE_MAX_UNDO)
        } else {
            (content.clone(), None, 100)
        };
        
        Self {
            id,
            path: Some(path),
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
            raw_line_height: 20.0, // Default, updated on first render
            pending_scroll_to_line: None,
            view_mode: ViewMode::Raw, // Newly opened files default to raw mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size,
            content_version: 0,
            file_type,
            needs_focus: true, // Auto-focus newly opened files
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false, // Will be set from settings by caller
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: 0.5, // Default to 50/50 split
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some("utf-8"), // Will be overridden by with_file_bytes
            original_bytes: Vec::new(), // Will be set by with_file_bytes
            current_encoding: "utf-8", // Will be overridden by with_file_bytes
            pending_undo_state: None, // Will be populated on first frame
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
        let (content, actual_encoding, _had_errors) = if let Some((bom_encoding, bom_len)) =
            encoding_rs::Encoding::for_bom(&bytes)
        {
            // BOM detected, use that encoding and skip BOM bytes
            // Use decode_without_bom_handling since we already handled the BOM
            let (decoded, had_errors) = bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
            (decoded.into_owned(), bom_encoding.name(), had_errors)
        } else {
            // No BOM, use detected encoding
            let (decoded, _, had_errors) = detected.decode(&bytes);
            (decoded.into_owned(), encoding_label, had_errors)
        };

        // For large files, store hash instead of full content to save memory
        // Also clear original_bytes for large files - can be reloaded from disk if needed
        let (original_content, original_content_hash, max_undo_size, original_bytes) = if is_large_file {
            log::info!(
                "Opening large file ({} bytes): using hash-based modification detection, reduced undo stack",
                bytes_len
            );
            // Don't store original_bytes for large files - saves significant memory
            // If user changes encoding, we can reload from disk
            (String::new(), Some(Self::compute_content_hash(&content)), LARGE_FILE_MAX_UNDO, Vec::new())
        } else {
            (content.clone(), None, 100, bytes)
        };

        Self {
            id,
            path: Some(path),
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
            view_mode: ViewMode::Raw,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size,
            content_version: 0,
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
            pending_undo_state: None, // Will be populated on first frame
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
        let cursor_char_idx = line_col_to_char_index(&content, info.cursor_position.0, info.cursor_position.1);
        
        let is_large_file = content.len() >= LARGE_FILE_THRESHOLD;
        let (original_content, original_content_hash, max_undo_size) = if is_large_file {
            (String::new(), Some(Self::compute_content_hash(&content)), LARGE_FILE_MAX_UNDO)
        } else {
            (content.clone(), None, 100)
        };
        
        Self {
            id,
            path: info.path.clone(),
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
            raw_line_height: 20.0, // Default, updated on first render
            pending_scroll_to_line: None,
            view_mode: info.view_mode, // Restore saved view mode
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size,
            content_version: 0,
            file_type,
            needs_focus: false, // Don't auto-focus restored tabs
            transient_highlight: TransientHighlight::new(),
            auto_save_enabled: false, // Will be set from settings by caller
            last_edit_time: None,
            last_auto_save_content_hash: None,
            fold_state: FoldState::new(),
            split_ratio: info.split_ratio, // Restore saved split ratio
            pipeline_state: TabPipelineState::default(),
            detected_encoding: Some("utf-8"), // Restored tabs default to UTF-8
            original_bytes: Vec::new(), // Bytes are reloaded if needed
            current_encoding: "utf-8", // Default encoding for restored tabs
            pending_undo_state: None, // Will be populated on first frame
        }
    }

    /// Create a tab from session info with settings-based auto-save.
    pub fn from_tab_info_with_settings(id: usize, info: &TabInfo, content: String, auto_save_default: bool) -> Self {
        let mut tab = Self::from_tab_info(id, info, content);
        tab.auto_save_enabled = auto_save_default;
        tab
    }

    /// Create a tab from session info using raw file bytes with encoding detection.
    ///
    /// This combines tab info restoration (view mode, split ratio) with
    /// automatic encoding detection from the file bytes.
    /// For large files, uses hash-based modification detection.
    pub fn from_tab_info_with_bytes(id: usize, info: &TabInfo, bytes: Vec<u8>, auto_save_default: bool) -> Self {
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
        let (content, actual_encoding) = if let Some((bom_encoding, bom_len)) =
            encoding_rs::Encoding::for_bom(&bytes)
        {
            // Use decode_without_bom_handling since we already handled the BOM
            let (decoded, _had_errors) = bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
            (decoded.into_owned(), bom_encoding.name())
        } else {
            let (decoded, _, _) = detected.decode(&bytes);
            (decoded.into_owned(), detected.name())
        };

        // Convert legacy cursor position to char index
        let cursor_char_idx = line_col_to_char_index(&content, info.cursor_position.0, info.cursor_position.1);

        // For large files, store hash instead of full content
        let (original_content, original_content_hash, max_undo_size, original_bytes) = if is_large_file {
            log::info!(
                "Restoring large file ({} bytes): using hash-based modification detection",
                bytes_len
            );
            (String::new(), Some(Self::compute_content_hash(&content)), LARGE_FILE_MAX_UNDO, Vec::new())
        } else {
            (content.clone(), None, 100, bytes)
        };

        Self {
            id,
            path: info.path.clone(),
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
            view_mode: info.view_mode,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size,
            content_version: 0,
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
            pending_undo_state: None, // Will be populated on first frame
        }
    }

    /// Check if the tab has unsaved changes.
    ///
    /// For large files (>1MB), uses hash comparison instead of full string comparison
    /// to avoid storing a full copy of the original content.
    pub fn is_modified(&self) -> bool {
        if let Some(hash) = self.original_content_hash {
            // Large file: use hash-based comparison
            Self::compute_content_hash(&self.content) != hash
        } else {
            // Normal file: use string comparison
            self.content != self.original_content
        }
    }
    
    /// Check if this is a large file that uses memory-optimized storage.
    pub fn is_large_file(&self) -> bool {
        self.is_large_file
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

    /// Determine if we should prompt to save before closing this tab.
    ///
    /// The logic is:
    /// - If the file is modified (content differs from original), prompt to save
    /// - EXCEPTION: Skip prompt for empty untitled files (nothing to save)
    ///
    /// This allows new tabs that haven't been touched to be closed silently,
    /// while still protecting any typed content from accidental loss.
    pub fn should_prompt_to_save(&self) -> bool {
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

    /// Get the display title for this tab.
    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");

        if self.is_modified() {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }

    /// Mark the current content as saved (updates original_content or hash).
    /// Also clears auto-save state since content is now persisted.
    pub fn mark_saved(&mut self) {
        if self.is_large_file {
            // Large file: update hash instead of storing full content
            self.original_content_hash = Some(Self::compute_content_hash(&self.content));
        } else {
            // Normal file: store full content for comparison
            self.original_content = self.content.clone();
        }
        // Clear auto-save tracking since content is now saved
        self.last_auto_save_content_hash = None;
        self.last_edit_time = None;
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
            if (self.content.len() as isize - old_len as isize).abs() > 100 {
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
    pub fn encode_content(&self) -> Vec<u8> {
        let encoding = encoding_rs::Encoding::for_label(self.current_encoding.as_bytes())
            .unwrap_or(encoding_rs::UTF_8);

        let (encoded, _actual_encoding, _had_errors) = encoding.encode(&self.content);
        encoded.into_owned()
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
    /// Returns true if:
    /// - Auto-save is enabled for this tab
    /// - Tab has unsaved changes (modified)
    /// - Content has changed since last auto-save
    /// - Idle delay has passed since last edit
    pub fn should_auto_save(&self, delay_ms: u32) -> bool {
        if !self.auto_save_enabled || !self.is_modified() {
            return false;
        }

        // Check if content changed since last auto-save
        let current_hash = hash_content(&self.content);
        if let Some(last_hash) = self.last_auto_save_content_hash {
            if current_hash == last_hash {
                return false; // No changes since last auto-save
            }
        }

        // Check if idle delay has passed
        if let Some(last_edit) = self.last_edit_time {
            let elapsed = last_edit.elapsed();
            elapsed >= std::time::Duration::from_millis(delay_ms as u64)
        } else {
            false
        }
    }

    /// Mark that auto-save was performed (updates content hash).
    pub fn mark_auto_saved(&mut self) {
        self.last_auto_save_content_hash = Some(hash_content(&self.content));
    }

    /// Get the content hash for change detection.
    pub fn content_hash(&self) -> u64 {
        hash_content(&self.content)
    }

    /// Set new content and push current to undo stack.
    ///
    /// The cursor position is captured from the current primary cursor.
    pub fn set_content(&mut self, new_content: String) {
        if new_content != self.content {
            // Push current state to undo stack (with cursor position)
            let cursor_pos = self.cursors.primary().head;
            self.undo_stack.push(UndoEntry::new(self.content.clone(), cursor_pos));
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            // Clear redo stack on new edit
            self.redo_stack.clear();
            self.content = new_content;
            // Update last edit time for auto-save
            self.mark_content_edited();
        }
    }

    /// Undo the last edit.
    ///
    /// Returns `Some(cursor_position)` if undo was performed, `None` otherwise.
    /// The cursor position is the position that was saved when the edit was made.
    /// Increments `content_version` to signal external content change to UI widgets.
    pub fn undo(&mut self) -> Option<usize> {
        if let Some(entry) = self.undo_stack.pop() {
            // Save current state to redo stack (with current cursor position)
            let current_cursor = self.cursors.primary().head;
            self.redo_stack.push(UndoEntry::new(self.content.clone(), current_cursor));
            // Restore previous state
            self.content = entry.content;
            self.content_version = self.content_version.wrapping_add(1);
            Some(entry.cursor_position)
        } else {
            None
        }
    }

    /// Redo the last undone edit.
    ///
    /// Returns `Some(cursor_position)` if redo was performed, `None` otherwise.
    /// The cursor position is the position that was saved when undo was performed.
    /// Increments `content_version` to signal external content change to UI widgets.
    pub fn redo(&mut self) -> Option<usize> {
        if let Some(entry) = self.redo_stack.pop() {
            // Save current state to undo stack (with current cursor position)
            let current_cursor = self.cursors.primary().head;
            self.undo_stack.push(UndoEntry::new(self.content.clone(), current_cursor));
            // Restore next state
            self.content = entry.content;
            self.content_version = self.content_version.wrapping_add(1);
            Some(entry.cursor_position)
        } else {
            None
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of items in the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of items in the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get the content version counter.
    ///
    /// This counter is incremented whenever content is modified externally
    /// (e.g., via undo/redo). UI widgets can use this to detect when they
    /// need to re-read content from the source.
    pub fn content_version(&self) -> u64 {
        self.content_version
    }

    /// Increment the content version counter.
    ///
    /// Call this when content is modified programmatically (e.g., snippet expansion)
    /// to signal to UI widgets that they need to re-read content from the source.
    pub fn increment_content_version(&mut self) {
        self.content_version = self.content_version.wrapping_add(1);
    }

    /// Record that an edit was made externally (e.g., by egui's TextEdit).
    ///
    /// Call this AFTER content has been modified, passing the OLD content
    /// and OLD cursor position before the modification. This is needed because
    /// TextEdit modifies the content string directly, bypassing `set_content()`.
    ///
    /// This method:
    /// - Pushes the old content and cursor to the undo stack
    /// - Clears the redo stack (new edits invalidate redo history)
    /// - Enforces the maximum undo history size
    pub fn record_edit(&mut self, old_content: String, old_cursor: usize) {
        // Only record if content actually changed
        if old_content != self.content {
            self.undo_stack.push(UndoEntry::new(old_content, old_cursor));
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
    }

    /// Prepare for potential edit by ensuring we have a pre-edit snapshot.
    ///
    /// This is an optimization to avoid cloning content every frame.
    /// Call this at the start of the editor render. It will only clone
    /// content if there's no pending snapshot (i.e., first frame after
    /// file open or after an edit was recorded).
    ///
    /// For large files (4MB+), this reduces memory allocation from
    /// 240MB/second (clone every frame at 60fps) to only cloning after edits.
    pub fn prepare_for_edit(&mut self) {
        if self.pending_undo_state.is_none() {
            self.pending_undo_state = Some(UndoEntry::new(
                self.content.clone(),
                self.cursors.primary().head,
            ));
        }
    }

    /// Record an edit that was detected via egui's response.changed().
    ///
    /// This uses the pending undo state (prepared by `prepare_for_edit`)
    /// to record the previous state for undo. After recording, the pending
    /// state is cleared and will be repopulated on the next frame.
    ///
    /// # Arguments
    /// * `old_cursor` - The cursor position before the edit (for undo positioning)
    pub fn record_edit_after_change(&mut self, old_cursor: usize) {
        if let Some(mut entry) = self.pending_undo_state.take() {
            // Use the provided cursor position (more accurate than stored one)
            entry.cursor_position = old_cursor;
            
            // Push to undo stack
            self.undo_stack.push(entry);
            if self.undo_stack.len() > self.max_undo_size {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
        // pending_undo_state is now None, will be repopulated by prepare_for_edit
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
    pub fn find_next_occurrence(&self, search_text: &str, after_pos: usize) -> Option<(usize, usize)> {
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
        self.fold_state.fold_all_of_kind(|k| matches!(k, FoldKind::Heading(_)));
    }

    /// Fold all code blocks.
    pub fn fold_all_code_blocks(&mut self) {
        self.fold_state.fold_all_of_kind(|k| matches!(k, FoldKind::CodeBlock));
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

/// UI-related state flags.
#[derive(Debug, Clone, Default)]
pub struct UiState {
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
}

/// Actions that may need confirmation before execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    /// Close a specific tab
    CloseTab(usize),
    /// Close all tabs
    CloseAllTabs,
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

/// Central application state struct.
///
/// This struct holds all runtime state for the application including:
/// - Open tabs and their content
/// - User settings (loaded from config)
/// - UI state (dialogs, panels, etc.)
/// - Application mode (single file or workspace)
///
/// # Example
///
/// ```ignore
/// let mut state = AppState::new();
/// state.new_tab();
/// state.active_tab_mut().set_content("# Hello".to_string());
/// ```

// ─────────────────────────────────────────────────────────────────────────────
// Session Content Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Result of resolving tab content from various sources.
/// Contains the content and optional encoding information for files loaded from disk.
enum ResolvedContent {
    /// Content recovered from crash recovery (already UTF-8)
    Recovered(String),
    /// Content loaded from disk with encoding detection
    FromDisk {
        content: String,
        original_bytes: Vec<u8>,
        encoding: &'static str,
    },
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
                // Try to read the file as bytes for encoding detection
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
                        // Skip this tab - file no longer exists
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

    /// Create a new empty tab and make it active.
    ///
    /// Returns the index of the new tab.
    /// Applies auto_save_enabled_default and default_view_mode from settings.
    pub fn new_tab(&mut self) -> usize {
        let auto_save_default = self.settings.auto_save_enabled_default;
        let default_view_mode = self.settings.default_view_mode;
        let tab = Tab::new_with_settings(self.next_tab_id, auto_save_default, default_view_mode);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        debug!("Created new tab at index {} (auto-save: {}, view_mode: {:?})", 
            self.active_tab_index, auto_save_default, default_view_mode);
        self.active_tab_index
    }

    /// Open a file in a new tab.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    pub fn open_file(&mut self, path: PathBuf) -> Result<usize, std::io::Error> {
        self.open_file_with_focus(path, true)
    }

    /// Open a file in a new tab with optional focus control.
    ///
    /// If `focus` is true, the new tab becomes active. If false, the file opens
    /// in the background without switching tabs.
    ///
    /// Returns the index of the new tab, or an error if the file couldn't be read.
    pub fn open_file_with_focus(
        &mut self,
        path: PathBuf,
        focus: bool,
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

        // Read file as bytes for encoding detection
        let bytes = std::fs::read(&path)?;

        // Create new tab with settings-based defaults and encoding detection
        let auto_save_default = self.settings.auto_save_enabled_default;
        let default_view_mode = self.settings.default_view_mode;
        let tab = Tab::with_file_bytes_and_settings(
            self.next_tab_id,
            path.clone(),
            bytes,
            auto_save_default,
            default_view_mode,
        );
        
        let detected_encoding = tab.current_encoding;
        self.next_tab_id += 1;
        self.tabs.push(tab);
        let new_index = self.tabs.len() - 1;

        if focus {
            self.active_tab_index = new_index;
            info!("Opened file: {} (encoding: {}, auto-save: {}, view_mode: {:?})", 
                path.display(), detected_encoding, auto_save_default, default_view_mode);
        } else {
            info!("Opened file: {} (encoding: {}, in background, auto-save: {}, view_mode: {:?})", 
                path.display(), detected_encoding, auto_save_default, default_view_mode);
        }

        // Update recent files and save immediately for persistence
        self.settings.add_recent_file(path.clone());
        self.settings_dirty = true;
        // Save immediately to survive app crashes/force-kills
        self.save_settings_if_dirty();

        Ok(new_index)
    }

    /// Find a tab by file path.
    pub fn find_tab_by_path(&self, path: &PathBuf) -> Option<usize> {
        self.tabs.iter().position(|t| t.path.as_ref() == Some(path))
    }

    /// Set the active tab by index.
    ///
    /// Returns `true` if the index was valid and the tab was switched.
    pub fn set_active_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
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
            if tab.should_prompt_to_save() {
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

        self.tabs.remove(index);

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

    /// Close the active tab.
    pub fn close_active_tab(&mut self) -> bool {
        self.close_tab(self.active_tab_index)
    }

    /// Check if any tabs have unsaved changes that warrant a save prompt.
    ///
    /// This uses `should_prompt_to_save()` for each tab, which means empty
    /// untitled files are not considered as having unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.iter().any(|t| t.should_prompt_to_save())
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

        self.app_mode = AppMode::SingleFile;
        self.workspace = None;
        self.workspace_watcher = None;
        self.pending_file_events.clear();

        // Close Git service
        self.git_service.close();

        info!("Workspace closed, returned to single-file mode");
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
            // Update session restoration data
            self.settings.last_open_tabs = self.tabs.iter().map(|t| t.to_tab_info()).collect();
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
            .map(|tab| {
                let file_mtime = tab
                    .path
                    .as_ref()
                    .and_then(|p| Self::get_file_mtime(p));

                let original_content_hash = if !tab.is_modified() {
                    Some(hash_content(&tab.content))
                } else {
                    None
                };

                SessionTabState {
                    tab_id: tab.id,
                    path: tab.path.clone(),
                    display_title: tab.title(),
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
    /// This saves the actual content of tabs that have unsaved changes,
    /// allowing crash recovery to restore the content even if the app crashes.
    pub fn save_recovery_content(&self) {
        use crate::config::save_recovery_content;

        for tab in &self.tabs {
            if tab.is_modified() {
                if !save_recovery_content(tab.id, &tab.content) {
                    warn!("Failed to save recovery content for tab {}", tab.id);
                }
            }
        }
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
            // Try to load content from various sources
            let resolved = self.resolve_tab_content(session_tab, result);

            if let Some(resolved) = resolved {
                let mut tab = match resolved {
                    ResolvedContent::Recovered(content) => {
                        // Recovery content is UTF-8
                        if let Some(path) = &session_tab.path {
                            let mut t = Tab::with_file(self.next_tab_id, path.clone(), content.clone());
                            // Set encoding to UTF-8 for recovered content
                            t.detected_encoding = Some("utf-8");
                            t.current_encoding = "utf-8";
                            t
                        } else {
                            let mut t = Tab::new(self.next_tab_id);
                            t.content = content.clone();
                            t
                        }
                    }
                    ResolvedContent::FromDisk { content, original_bytes, encoding } => {
                        if let Some(path) = &session_tab.path {
                            let file_type = FileType::from_path(path);
                            let is_large_file = content.len() >= LARGE_FILE_THRESHOLD;
                            
                            // For large files, use hash-based modification detection
                            let (original_content_str, original_content_hash, max_undo, final_original_bytes) = if is_large_file {
                                log::info!(
                                    "Restoring large file from disk ({} bytes): using hash-based modification detection",
                                    content.len()
                                );
                                (String::new(), Some(Tab::compute_content_hash(&content)), LARGE_FILE_MAX_UNDO, Vec::new())
                            } else {
                                (content.clone(), None, 100, original_bytes)
                            };
                            
                            let t = Tab {
                                id: self.next_tab_id,
                                path: Some(path.clone()),
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
                                view_mode: ViewMode::Raw,
                                undo_stack: Vec::new(),
                                redo_stack: Vec::new(),
                                max_undo_size: max_undo,
                                content_version: 0,
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
                                pending_undo_state: None,
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
                tab.cursors.set_single(crate::state::Selection::cursor(session_tab.cursor_char_index));
                if let Some((start, end)) = session_tab.selection {
                    tab.cursors.set_single(crate::state::Selection::new(start, end));
                }
                tab.sync_cursor_from_primary();

                // If we loaded from recovery content, mark as modified
                if session_tab.has_unsaved_content && result.recovered_content.contains_key(&session_tab.tab_id) {
                    // Content was recovered - it's modified relative to what's on disk
                    // The original_content field stays as the disk version
                }

                self.tabs.push(tab);
                restored_count += 1;

                debug!(
                    "Restored tab {} from session: {}",
                    session_tab.tab_id,
                    session_tab.display_title
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
                debug!("Session had workspace mode but path was empty, starting in single-file mode");
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
            if session.zen_mode { " (Zen Mode enabled)" } else { "" },
            if self.app_mode.is_workspace() { " (Workspace mode)" } else { "" }
        );

        restored_count > 0
    }

    /// Resolve content for a tab from various sources.
    ///
    /// Priority:
    /// 1. Recovery content (if tab had unsaved changes)
    /// 2. File on disk (if path exists)
    /// 3. None (if file is missing and no recovery content)
    fn resolve_tab_content(
        &self,
        session_tab: &crate::config::SessionTabState,
        result: &crate::config::SessionRestoreResult,
    ) -> Option<ResolvedContent> {
        use chardetng::EncodingDetector;

        // First, check if we have recovery content
        if let Some(recovered) = result.recovered_content.get(&session_tab.tab_id) {
            debug!(
                "Using recovered content for tab {} ({})",
                session_tab.tab_id, session_tab.display_title
            );
            return Some(ResolvedContent::Recovered(recovered.clone()));
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
                        let (content, encoding) = if let Some((bom_encoding, bom_len)) =
                            encoding_rs::Encoding::for_bom(&bytes)
                        {
                            // Use decode_without_bom_handling since we already handled the BOM
                            let (decoded, _had_errors) = bom_encoding.decode_without_bom_handling(&bytes[bom_len..]);
                            (decoded.into_owned(), bom_encoding.name())
                        } else {
                            let (decoded, _, _) = detected.decode(&bytes);
                            (decoded.into_owned(), detected.name())
                        };

                        debug!(
                            "Loaded content from disk for tab {} (encoding: {})",
                            session_tab.tab_id, encoding
                        );
                        return Some(ResolvedContent::FromDisk {
                            content,
                            original_bytes: bytes,
                            encoding,
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
                    self.tabs.clear();
                    self.new_tab();
                }
                PendingAction::Exit => {
                    // Caller should handle exit
                    debug!("Exit confirmed");
                }
                PendingAction::OpenFile(path) => {
                    if let Err(e) = self.open_file(path) {
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
    pub fn toggle_about(&mut self) {
        self.ui.show_about = !self.ui.show_about;
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
    fn test_tab_should_prompt_to_save() {
        // Case 1: New file unmodified - NO prompt
        let new_unmodified = Tab::new(0);
        assert!(!new_unmodified.should_prompt_to_save());

        // Case 2: New file with content - prompt
        let mut new_with_content = Tab::new(1);
        new_with_content.set_content("hello".to_string());
        assert!(new_with_content.should_prompt_to_save());

        // Case 3: New file typed and deleted - NO prompt (back to empty)
        let mut new_typed_deleted = Tab::new(2);
        new_typed_deleted.set_content("hello".to_string());
        new_typed_deleted.set_content(String::new());
        assert!(!new_typed_deleted.should_prompt_to_save());

        // Case 4: Saved file unmodified - NO prompt
        let saved_unmodified = Tab::with_file(3, PathBuf::from("test.md"), "content".to_string());
        assert!(!saved_unmodified.should_prompt_to_save());

        // Case 5: Saved file modified - prompt
        let mut saved_modified = Tab::with_file(4, PathBuf::from("test.md"), "content".to_string());
        saved_modified.set_content("modified content".to_string());
        assert!(saved_modified.should_prompt_to_save());

        // Case 6: Existing empty file (loaded from disk) unmodified - NO prompt
        let existing_empty = Tab::with_file(5, PathBuf::from("empty.md"), String::new());
        assert!(!existing_empty.should_prompt_to_save());

        // Case 7: Existing empty file modified - prompt
        let mut existing_empty_modified =
            Tab::with_file(6, PathBuf::from("empty.md"), String::new());
        existing_empty_modified.set_content("now has content".to_string());
        assert!(existing_empty_modified.should_prompt_to_save());

        // Case 8: Saved file, content deleted entirely - prompt (modified)
        let mut saved_then_cleared =
            Tab::with_file(7, PathBuf::from("content.md"), "original".to_string());
        saved_then_cleared.set_content(String::new());
        assert!(saved_then_cleared.should_prompt_to_save());
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
        tab.set_content("second".to_string());
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
    fn test_tab_undo_clears_redo_on_edit() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.set_content("second".to_string());

        tab.undo();
        assert!(tab.can_redo());

        tab.set_content("new edit".to_string());
        assert!(!tab.can_redo());
    }

    #[test]
    fn test_tab_record_edit() {
        let mut tab = Tab::new(0);

        // Simulate external edit (like TextEdit does)
        let old_content = tab.content.clone();
        tab.content = "first edit".to_string();
        tab.record_edit(old_content, 0);

        assert!(tab.can_undo());
        assert_eq!(tab.undo_count(), 1);

        // Simulate another edit
        let old_content = tab.content.clone();
        tab.content = "second edit".to_string();
        tab.record_edit(old_content, 5);

        assert_eq!(tab.undo_count(), 2);
        assert!(!tab.can_redo());

        // Undo should restore previous state and return cursor position
        let cursor = tab.undo();
        assert_eq!(tab.content, "first edit");
        assert!(tab.can_redo());
        assert_eq!(cursor, Some(5)); // Should restore cursor from undo entry
    }

    #[test]
    fn test_tab_record_edit_no_change() {
        let mut tab = Tab::new(0);
        tab.content = "same content".to_string();

        // Recording with same content should not add to undo stack
        let old_content = tab.content.clone();
        tab.record_edit(old_content, 0);

        assert!(!tab.can_undo());
        assert_eq!(tab.undo_count(), 0);
    }

    #[test]
    fn test_tab_record_edit_clears_redo() {
        let mut tab = Tab::new(0);
        tab.set_content("first".to_string());
        tab.set_content("second".to_string());
        tab.undo();

        assert!(tab.can_redo());

        // New edit via record_edit should clear redo
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
    fn test_tab_max_undo_size() {
        let mut tab = Tab::new(0);
        // Max undo size is 100 by default

        // Add 105 edits
        for i in 0..105 {
            tab.set_content(format!("edit {}", i));
        }

        // Should be capped at 100
        assert_eq!(tab.undo_count(), 100);

        // Oldest edits should be dropped, so undoing 100 times
        // should get us back to edit 4 (edits 0-4 were dropped)
        for _ in 0..100 {
            tab.undo();
        }

        // After 100 undos, we should be at the oldest kept state
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
        let mut state = AppState::with_settings(Settings::default());
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
        assert!(ui.status_message.is_none());
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
        let mut state = AppState::with_settings(Settings::default());
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
    fn test_appstate_close_new_modified_tab_prompts() {
        let mut state = AppState::with_settings(Settings::default());

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
        let mut state = AppState::with_settings(Settings::default());

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
        assert_eq!(tab.split_ratio, 0.6);              // Split ratio restored
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
        let result = state.open_file_with_focus(temp_file.clone(), true);

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
        let result = state.open_file_with_focus(temp_file.clone(), false);

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
        let first_result = state.open_file_with_focus(temp_file.clone(), true);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        assert_ne!(state.active_tab_index(), first_index);

        // Open the same file again with focus=true
        let second_result = state.open_file_with_focus(temp_file.clone(), true);

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
        let first_result = state.open_file_with_focus(temp_file.clone(), true);
        assert!(first_result.is_ok());
        let first_index = first_result.unwrap();

        // Create another tab to change active tab
        state.new_tab();
        let new_tab_index = state.active_tab_index();
        assert_ne!(new_tab_index, first_index);

        // Open the same file again with focus=false
        let second_result = state.open_file_with_focus(temp_file.clone(), false);

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
        let result = state.open_file_with_focus(temp_file.clone(), false);

        // Clean up
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_ok());
        // Recent files should now contain the opened file
        assert!(!state.settings.recent_files.is_empty());
        assert_eq!(state.settings.recent_files[0], temp_file);
    }
}
