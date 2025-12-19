//! Bidirectional Sync Scrolling for Raw and Rendered Views
//!
//! This module implements synchronized scrolling between the Raw markdown editor
//! and the Rendered WYSIWYG view. It provides:

// Allow dead code for infrastructure that will be used in future split-view implementation
// - double_ended_iterator_last: filter().last() is clearer for finding last matching element
#![allow(dead_code)]
#![allow(clippy::double_ended_iterator_last)]
//!
//! - Source line to rendered block mapping
//! - Debounced scroll event handling
//! - Bidirectional scroll synchronization
//! - Visual indicators for scroll position
//! - Feedback loop prevention
//!
//! # Architecture
//!
//! The sync scrolling system uses a "scroll origin" token to prevent feedback loops.
//! When one view initiates a scroll, it sets itself as the origin, and the other
//! view will sync to it. The debounce timer prevents rapid back-and-forth syncing.
//!
//! # Usage
//!
//! ```ignore
//! let mut sync_state = SyncScrollState::new();
//!
//! // When Raw editor scrolls
//! if sync_state.should_sync_from(ScrollOrigin::Raw) {
//!     let target_line = sync_state.get_raw_topmost_line(scroll_offset, line_height);
//!     let rendered_offset = sync_state.line_to_rendered_offset(target_line);
//!     // Apply rendered_offset to rendered view
//! }
//! ```

use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for sync scrolling behavior.
#[derive(Debug, Clone)]
pub struct SyncScrollConfig {
    /// Debounce duration for scroll events (default: 16ms for ~60fps)
    pub debounce_duration: Duration,
    /// Whether to use smooth animated scrolling
    pub smooth_scrolling: bool,
    /// Animation duration for smooth scrolling (in seconds)
    pub animation_duration: f32,
    /// Minimum scroll delta to trigger sync (pixels)
    pub min_scroll_delta: f32,
}

impl Default for SyncScrollConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(16),
            smooth_scrolling: true,
            animation_duration: 0.15,
            min_scroll_delta: 5.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scroll Origin
// ─────────────────────────────────────────────────────────────────────────────

/// Origin of a scroll event, used to prevent feedback loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOrigin {
    /// Scroll originated from the Raw markdown editor
    Raw,
    /// Scroll originated from the Rendered WYSIWYG view
    Rendered,
    /// Scroll originated from external navigation (e.g., outline panel)
    External,
    /// No active scroll origin (idle state)
    None,
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a mapping between source lines and rendered block positions.
#[derive(Debug, Clone)]
pub struct BlockMapping {
    /// Source line range (start_line, end_line) - 1-indexed
    pub source_lines: (usize, usize),
    /// Rendered Y offset range (start_y, end_y) in pixels
    pub rendered_range: (f32, f32),
    /// Block type for debugging/visualization
    pub block_type: BlockType,
}

/// Types of markdown blocks for mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// Heading (H1-H6)
    Heading,
    /// Regular paragraph
    Paragraph,
    /// Code block
    CodeBlock,
    /// List (ordered or unordered)
    List,
    /// Block quote
    BlockQuote,
    /// Table
    Table,
    /// Horizontal rule
    HorizontalRule,
    /// Other block type
    Other,
}

impl BlockMapping {
    /// Create a new block mapping.
    pub fn new(
        start_line: usize,
        end_line: usize,
        rendered_start: f32,
        rendered_end: f32,
        block_type: BlockType,
    ) -> Self {
        Self {
            source_lines: (start_line, end_line),
            rendered_range: (rendered_start, rendered_end),
            block_type,
        }
    }

    /// Check if a source line falls within this block.
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.source_lines.0 && line <= self.source_lines.1
    }

    /// Check if a rendered Y offset falls within this block.
    pub fn contains_rendered_y(&self, y: f32) -> bool {
        y >= self.rendered_range.0 && y < self.rendered_range.1
    }

    /// Get the midpoint Y position of this rendered block.
    pub fn rendered_midpoint(&self) -> f32 {
        (self.rendered_range.0 + self.rendered_range.1) / 2.0
    }

    /// Get the source line midpoint.
    pub fn source_midpoint(&self) -> usize {
        (self.source_lines.0 + self.source_lines.1) / 2
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sync Scroll State
// ─────────────────────────────────────────────────────────────────────────────

/// State machine for managing synchronized scrolling between views.
#[derive(Debug)]
pub struct SyncScrollState {
    /// Whether sync scrolling is enabled
    pub enabled: bool,
    /// Block mappings from source to rendered
    mappings: Vec<BlockMapping>,
    /// Current scroll origin (who initiated the last scroll)
    scroll_origin: ScrollOrigin,
    /// Last scroll event time for debouncing
    last_scroll_time: Option<Instant>,
    /// Configuration settings
    config: SyncScrollConfig,
    /// Last known Raw scroll offset
    last_raw_offset: f32,
    /// Last known Rendered scroll offset
    last_rendered_offset: f32,
    /// Target scroll offset for animation (Raw)
    target_raw_offset: Option<f32>,
    /// Target scroll offset for animation (Rendered)
    target_rendered_offset: Option<f32>,
    /// Animation start time
    animation_start: Option<Instant>,
    /// Animation start offset (Raw)
    animation_start_raw: f32,
    /// Animation start offset (Rendered)
    animation_start_rendered: f32,
    /// Total source line count (for proportional fallback)
    source_line_count: usize,
    /// Total rendered height (for proportional fallback)
    rendered_total_height: f32,
}

impl Default for SyncScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncScrollState {
    /// Create a new sync scroll state.
    pub fn new() -> Self {
        Self {
            enabled: true,
            mappings: Vec::new(),
            scroll_origin: ScrollOrigin::None,
            last_scroll_time: None,
            config: SyncScrollConfig::default(),
            last_raw_offset: 0.0,
            last_rendered_offset: 0.0,
            target_raw_offset: None,
            target_rendered_offset: None,
            animation_start: None,
            animation_start_raw: 0.0,
            animation_start_rendered: 0.0,
            source_line_count: 0,
            rendered_total_height: 0.0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SyncScrollConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Set whether sync scrolling is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.clear_animation();
        }
    }

    /// Toggle sync scrolling on/off.
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if !self.enabled {
            self.clear_animation();
        }
        self.enabled
    }

    /// Clear all mappings (call when content changes).
    pub fn clear_mappings(&mut self) {
        self.mappings.clear();
        self.source_line_count = 0;
        self.rendered_total_height = 0.0;
    }

    /// Add a block mapping.
    pub fn add_mapping(&mut self, mapping: BlockMapping) {
        self.mappings.push(mapping);
    }

    /// Set source metadata for proportional fallback.
    pub fn set_source_metadata(&mut self, line_count: usize, rendered_height: f32) {
        self.source_line_count = line_count;
        self.rendered_total_height = rendered_height;
    }

    /// Build mappings from parsed markdown document.
    ///
    /// This creates block-level mappings between source line ranges and
    /// rendered Y positions. Call this after rendering the WYSIWYG view.
    pub fn build_mappings_from_blocks(&mut self, blocks: Vec<BlockMapping>) {
        self.mappings = blocks;
        // Sort by source line for efficient lookup
        self.mappings.sort_by_key(|m| m.source_lines.0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scroll Origin Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if we should sync from the given origin.
    ///
    /// Returns true if:
    /// - Sync is enabled
    /// - The current origin is the same (no feedback loop)
    /// - OR the debounce period has passed for cross-origin syncs
    pub fn should_sync_from(&self, origin: ScrollOrigin) -> bool {
        if !self.enabled {
            return false;
        }

        // Allow if no recent scroll
        if self.scroll_origin == ScrollOrigin::None {
            return true;
        }

        // Same origin is always allowed (continuing the current sync operation)
        if self.scroll_origin == origin {
            return true;
        }

        // Cross-origin: check debounce to prevent feedback loops
        if let Some(last_time) = self.last_scroll_time {
            // Allow if enough time has passed (3x debounce to be safe)
            return last_time.elapsed() >= self.config.debounce_duration * 3;
        }

        true
    }

    /// Mark a scroll event from the given origin.
    pub fn mark_scroll(&mut self, origin: ScrollOrigin) {
        self.scroll_origin = origin;
        self.last_scroll_time = Some(Instant::now());
    }

    /// Clear the scroll origin (call after sync is complete).
    pub fn clear_origin(&mut self) {
        if let Some(last_time) = self.last_scroll_time {
            // Only clear if debounce period has passed
            if last_time.elapsed() >= self.config.debounce_duration * 2 {
                self.scroll_origin = ScrollOrigin::None;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line/Offset Conversion
    // ─────────────────────────────────────────────────────────────────────────

    /// Get the topmost visible source line from a Raw scroll offset.
    pub fn raw_offset_to_line(&self, scroll_offset: f32, line_height: f32) -> usize {
        if line_height <= 0.0 {
            return 1;
        }
        ((scroll_offset / line_height) as usize)
            .saturating_add(1)
            .max(1)
    }

    /// Get the Raw scroll offset for a given source line.
    pub fn line_to_raw_offset(&self, line: usize, line_height: f32) -> f32 {
        (line.saturating_sub(1) as f32) * line_height
    }

    /// Convert a source line to the corresponding rendered Y offset.
    ///
    /// Uses block mappings if available, falls back to proportional calculation.
    pub fn line_to_rendered_offset(&self, line: usize) -> f32 {
        // Try to find a mapping containing this line
        if let Some(mapping) = self.mappings.iter().find(|m| m.contains_line(line)) {
            // Interpolate within the block
            let line_range = mapping.source_lines.1 - mapping.source_lines.0;
            if line_range > 0 {
                let line_progress = (line - mapping.source_lines.0) as f32 / line_range as f32;
                let rendered_range = mapping.rendered_range.1 - mapping.rendered_range.0;
                return mapping.rendered_range.0 + (line_progress * rendered_range);
            }
            return mapping.rendered_range.0;
        }

        // Find the closest mapping before this line
        let before = self
            .mappings
            .iter()
            .filter(|m| m.source_lines.1 < line)
            .next_back();

        // Find the closest mapping after this line
        let after = self.mappings.iter().find(|m| m.source_lines.0 > line);

        // Interpolate between the two
        match (before, after) {
            (Some(b), Some(a)) => {
                let line_progress =
                    (line - b.source_lines.1) as f32 / (a.source_lines.0 - b.source_lines.1) as f32;
                b.rendered_range.1 + line_progress * (a.rendered_range.0 - b.rendered_range.1)
            }
            (Some(b), None) => {
                // After the last mapping - extrapolate
                b.rendered_range.1
            }
            (None, Some(a)) => {
                // Before the first mapping
                let line_progress = line as f32 / a.source_lines.0 as f32;
                line_progress * a.rendered_range.0
            }
            (None, None) => {
                // No mappings - use proportional fallback
                self.proportional_line_to_rendered(line)
            }
        }
    }

    /// Convert a rendered Y offset to the corresponding source line.
    pub fn rendered_offset_to_line(&self, rendered_y: f32) -> usize {
        // Try to find a mapping containing this Y position
        if let Some(mapping) = self
            .mappings
            .iter()
            .find(|m| m.contains_rendered_y(rendered_y))
        {
            // Interpolate within the block
            let rendered_range = mapping.rendered_range.1 - mapping.rendered_range.0;
            if rendered_range > 0.0 {
                let y_progress = (rendered_y - mapping.rendered_range.0) / rendered_range;
                let line_range = mapping.source_lines.1 - mapping.source_lines.0;
                return mapping.source_lines.0 + (y_progress * line_range as f32) as usize;
            }
            return mapping.source_lines.0;
        }

        // Find closest mappings and interpolate
        let before = self
            .mappings
            .iter()
            .filter(|m| m.rendered_range.1 < rendered_y)
            .next_back();

        let after = self
            .mappings
            .iter()
            .find(|m| m.rendered_range.0 > rendered_y);

        match (before, after) {
            (Some(b), Some(a)) => {
                let y_progress =
                    (rendered_y - b.rendered_range.1) / (a.rendered_range.0 - b.rendered_range.1);
                let line_range = a.source_lines.0 - b.source_lines.1;
                b.source_lines.1 + (y_progress * line_range as f32) as usize
            }
            (Some(b), None) => b.source_lines.1,
            (None, Some(a)) => {
                let y_progress = rendered_y / a.rendered_range.0;
                (y_progress * a.source_lines.0 as f32) as usize
            }
            (None, None) => self.proportional_rendered_to_line(rendered_y),
        }
    }

    /// Proportional fallback: line to rendered offset.
    fn proportional_line_to_rendered(&self, line: usize) -> f32 {
        if self.source_line_count == 0 || self.rendered_total_height <= 0.0 {
            return 0.0;
        }
        (line as f32 / self.source_line_count as f32) * self.rendered_total_height
    }

    /// Proportional fallback: rendered offset to line.
    fn proportional_rendered_to_line(&self, rendered_y: f32) -> usize {
        if self.rendered_total_height <= 0.0 || self.source_line_count == 0 {
            return 1;
        }
        let line = (rendered_y / self.rendered_total_height) * self.source_line_count as f32;
        (line as usize).max(1)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Animation Support
    // ─────────────────────────────────────────────────────────────────────────

    /// Start an animated scroll to a target offset (Raw view).
    pub fn animate_raw_to(&mut self, target: f32) {
        if !self.config.smooth_scrolling {
            self.target_raw_offset = Some(target);
            return;
        }
        self.target_raw_offset = Some(target);
        self.animation_start = Some(Instant::now());
        self.animation_start_raw = self.last_raw_offset;
    }

    /// Start an animated scroll to a target offset (Rendered view).
    pub fn animate_rendered_to(&mut self, target: f32) {
        if !self.config.smooth_scrolling {
            self.target_rendered_offset = Some(target);
            return;
        }
        self.target_rendered_offset = Some(target);
        self.animation_start = Some(Instant::now());
        self.animation_start_rendered = self.last_rendered_offset;
    }

    /// Get the current animated Raw offset (or None if no animation).
    pub fn get_animated_raw_offset(&mut self) -> Option<f32> {
        let target = self.target_raw_offset?;

        if !self.config.smooth_scrolling {
            let result = target;
            self.target_raw_offset = None;
            return Some(result);
        }

        let start_time = self.animation_start?;
        let elapsed = start_time.elapsed().as_secs_f32();
        let progress = (elapsed / self.config.animation_duration).min(1.0);

        // Use ease-out quad for smooth deceleration
        let eased = 1.0 - (1.0 - progress).powi(2);

        let current = self.animation_start_raw + (target - self.animation_start_raw) * eased;

        if progress >= 1.0 {
            self.target_raw_offset = None;
            self.animation_start = None;
            return Some(target);
        }

        Some(current)
    }

    /// Get the current animated Rendered offset (or None if no animation).
    pub fn get_animated_rendered_offset(&mut self) -> Option<f32> {
        let target = self.target_rendered_offset?;

        if !self.config.smooth_scrolling {
            let result = target;
            self.target_rendered_offset = None;
            return Some(result);
        }

        let start_time = self.animation_start?;
        let elapsed = start_time.elapsed().as_secs_f32();
        let progress = (elapsed / self.config.animation_duration).min(1.0);

        // Use ease-out quad for smooth deceleration
        let eased = 1.0 - (1.0 - progress).powi(2);

        let current =
            self.animation_start_rendered + (target - self.animation_start_rendered) * eased;

        if progress >= 1.0 {
            self.target_rendered_offset = None;
            self.animation_start = None;
            return Some(target);
        }

        Some(current)
    }

    /// Check if an animation is currently running.
    pub fn is_animating(&self) -> bool {
        self.target_raw_offset.is_some() || self.target_rendered_offset.is_some()
    }

    /// Clear any pending animations.
    pub fn clear_animation(&mut self) {
        self.target_raw_offset = None;
        self.target_rendered_offset = None;
        self.animation_start = None;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scroll Tracking
    // ─────────────────────────────────────────────────────────────────────────

    /// Update the last known Raw scroll offset.
    pub fn update_raw_offset(&mut self, offset: f32) {
        self.last_raw_offset = offset;
    }

    /// Update the last known Rendered scroll offset.
    pub fn update_rendered_offset(&mut self, offset: f32) {
        self.last_rendered_offset = offset;
    }

    /// Get the last known Raw scroll offset.
    pub fn last_raw_offset(&self) -> f32 {
        self.last_raw_offset
    }

    /// Get the last known Rendered scroll offset.
    pub fn last_rendered_offset(&self) -> f32 {
        self.last_rendered_offset
    }

    /// Check if the scroll offset has changed significantly.
    pub fn has_significant_delta(&self, new_offset: f32, old_offset: f32) -> bool {
        (new_offset - old_offset).abs() >= self.config.min_scroll_delta
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Visual Indicators
    // ─────────────────────────────────────────────────────────────────────────

    /// Get the visible line range in the Raw view.
    pub fn get_visible_raw_lines(
        &self,
        scroll_offset: f32,
        viewport_height: f32,
        line_height: f32,
    ) -> (usize, usize) {
        if line_height <= 0.0 {
            return (1, 1);
        }
        let first_line = (scroll_offset / line_height) as usize + 1;
        let visible_lines = (viewport_height / line_height).ceil() as usize;
        let last_line = first_line + visible_lines;
        (first_line, last_line)
    }

    /// Get the Y range corresponding to visible Raw lines in the Rendered view.
    pub fn get_rendered_indicator_range(
        &self,
        raw_scroll_offset: f32,
        raw_viewport_height: f32,
        line_height: f32,
    ) -> (f32, f32) {
        let (first_line, last_line) =
            self.get_visible_raw_lines(raw_scroll_offset, raw_viewport_height, line_height);
        let start_y = self.line_to_rendered_offset(first_line);
        let end_y = self.line_to_rendered_offset(last_line);
        (start_y, end_y)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_scroll_state_new() {
        let state = SyncScrollState::new();
        assert!(state.enabled);
        assert_eq!(state.scroll_origin, ScrollOrigin::None);
        assert!(state.mappings.is_empty());
    }

    #[test]
    fn test_toggle_sync() {
        let mut state = SyncScrollState::new();
        assert!(state.enabled);

        let result = state.toggle();
        assert!(!result);
        assert!(!state.enabled);

        let result = state.toggle();
        assert!(result);
        assert!(state.enabled);
    }

    #[test]
    fn test_block_mapping_contains() {
        let mapping = BlockMapping::new(5, 10, 100.0, 200.0, BlockType::Paragraph);

        assert!(mapping.contains_line(5));
        assert!(mapping.contains_line(7));
        assert!(mapping.contains_line(10));
        assert!(!mapping.contains_line(4));
        assert!(!mapping.contains_line(11));

        assert!(mapping.contains_rendered_y(100.0));
        assert!(mapping.contains_rendered_y(150.0));
        assert!(!mapping.contains_rendered_y(200.0));
        assert!(!mapping.contains_rendered_y(99.0));
    }

    #[test]
    fn test_raw_offset_to_line() {
        let state = SyncScrollState::new();
        let line_height = 20.0;

        assert_eq!(state.raw_offset_to_line(0.0, line_height), 1);
        assert_eq!(state.raw_offset_to_line(20.0, line_height), 2);
        assert_eq!(state.raw_offset_to_line(45.0, line_height), 3);
        assert_eq!(state.raw_offset_to_line(100.0, line_height), 6);
    }

    #[test]
    fn test_line_to_raw_offset() {
        let state = SyncScrollState::new();
        let line_height = 20.0;

        assert_eq!(state.line_to_raw_offset(1, line_height), 0.0);
        assert_eq!(state.line_to_raw_offset(2, line_height), 20.0);
        assert_eq!(state.line_to_raw_offset(5, line_height), 80.0);
    }

    #[test]
    fn test_line_to_rendered_with_mappings() {
        let mut state = SyncScrollState::new();

        state.add_mapping(BlockMapping::new(1, 3, 0.0, 60.0, BlockType::Heading));
        state.add_mapping(BlockMapping::new(4, 8, 60.0, 160.0, BlockType::Paragraph));
        state.add_mapping(BlockMapping::new(9, 12, 160.0, 240.0, BlockType::CodeBlock));

        // Within first block
        assert!((state.line_to_rendered_offset(1) - 0.0).abs() < 0.01);
        assert!((state.line_to_rendered_offset(2) - 30.0).abs() < 0.01);

        // Within second block
        assert!((state.line_to_rendered_offset(4) - 60.0).abs() < 0.01);
        assert!((state.line_to_rendered_offset(6) - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_rendered_offset_to_line() {
        let mut state = SyncScrollState::new();

        state.add_mapping(BlockMapping::new(1, 5, 0.0, 100.0, BlockType::Paragraph));
        state.add_mapping(BlockMapping::new(6, 10, 100.0, 200.0, BlockType::Paragraph));

        assert_eq!(state.rendered_offset_to_line(0.0), 1);
        assert_eq!(state.rendered_offset_to_line(50.0), 3);
        assert_eq!(state.rendered_offset_to_line(100.0), 6);
        assert_eq!(state.rendered_offset_to_line(150.0), 8);
    }

    #[test]
    fn test_proportional_fallback() {
        let mut state = SyncScrollState::new();
        state.set_source_metadata(100, 2000.0);

        // No mappings - should use proportional
        assert!((state.line_to_rendered_offset(50) - 1000.0).abs() < 0.01);
        assert_eq!(state.rendered_offset_to_line(1000.0), 50);
    }

    #[test]
    fn test_should_sync_from() {
        let mut state = SyncScrollState::new();

        // Should sync when enabled and no recent scroll
        assert!(state.should_sync_from(ScrollOrigin::Raw));

        // Mark a scroll from Raw
        state.mark_scroll(ScrollOrigin::Raw);

        // Same origin should work immediately
        assert!(state.should_sync_from(ScrollOrigin::Raw));

        // Disable and check
        state.set_enabled(false);
        assert!(!state.should_sync_from(ScrollOrigin::Raw));
    }

    #[test]
    fn test_visible_line_range() {
        let state = SyncScrollState::new();

        let (first, last) = state.get_visible_raw_lines(0.0, 200.0, 20.0);
        assert_eq!(first, 1);
        assert_eq!(last, 11);

        let (first, last) = state.get_visible_raw_lines(100.0, 200.0, 20.0);
        assert_eq!(first, 6);
        assert_eq!(last, 16);
    }

    #[test]
    fn test_significant_delta() {
        let state = SyncScrollState::new();

        assert!(!state.has_significant_delta(5.0, 3.0));
        assert!(state.has_significant_delta(10.0, 3.0));
        assert!(state.has_significant_delta(0.0, 10.0));
    }
}
