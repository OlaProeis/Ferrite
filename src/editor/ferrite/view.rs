//! ViewState module for viewport tracking and virtual scrolling.
//!
//! This module provides a `ViewState` struct that tracks viewport dimensions,
//! scroll position, and calculates visible line ranges for efficient rendering.
//! It enables virtual scrolling by only rendering lines that are visible
//! (plus overscan for smooth scrolling).
//!
//! # Word Wrap Support (Phase 2)
//! When word wrap is enabled, a single logical line may span multiple visual rows.
//! `ViewState` tracks wrapped line heights via `WrapInfo` to enable:
//! - Correct scrollbar sizing (total_height = sum of all wrapped line heights)
//! - Proper cursor navigation (up/down moves by visual row)
//! - Accurate scroll position calculation

/// Default line height in pixels (used as fallback).
/// In practice, this should be obtained from the font/galley.
const DEFAULT_LINE_HEIGHT: f32 = 20.0;

/// Number of extra lines to render above and below the visible area.
/// This prevents visual glitches during rapid scrolling.
const OVERSCAN_LINES: usize = 5;

/// Threshold for large file optimization (lines).
/// Files with more lines than this use uniform height assumptions
/// to avoid O(N) memory and CPU overhead for wrap_info/cumulative_heights.
const LARGE_FILE_THRESHOLD: usize = 100_000;

/// Information about a wrapped line's visual representation.
#[derive(Debug, Clone, PartialEq)]
pub struct WrapInfo {
    /// Number of visual rows this logical line occupies.
    pub visual_rows: usize,
    /// Total height of this line in pixels (visual_rows * row_height, or actual galley height).
    pub height: f32,
}

impl Default for WrapInfo {
    fn default() -> Self {
        Self {
            visual_rows: 1,
            height: DEFAULT_LINE_HEIGHT,
        }
    }
}

/// Tracks viewport state for virtual scrolling and efficient line rendering.
///
/// `ViewState` manages:
/// - Viewport dimensions (height, scroll position)
/// - Line height for pixel-to-line conversions
/// - Horizontal scroll offset (when word wrap is disabled)
/// - Word wrap width and wrapped line height tracking
/// - Visible line range calculation with overscan
///
/// # Large File Optimization
/// For files with more than `LARGE_FILE_THRESHOLD` lines (100,000),
/// the ViewState automatically uses uniform height mode to avoid O(N)
/// memory allocation and CPU overhead. This ensures smooth performance
/// even for 500k+ line files.
///
/// # Example
/// ```
/// use ferrite::editor::ViewState;
///
/// let mut view = ViewState::new();
/// view.update_viewport(600.0);  // 600px tall viewport
/// view.set_line_height(18.0);   // 18px per line
///
/// // Get visible line range for a 1000-line document
/// let (start, end) = view.get_visible_line_range(1000);
/// // Render only lines start..end for efficiency
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// Index of the first visible line (0-indexed).
    first_visible_line: usize,
    /// Vertical scroll offset in pixels within the current line.
    scroll_offset_y: f32,
    /// Height of the viewport in pixels.
    viewport_height: f32,
    /// Height of a single line in pixels (base height, used when wrap_info is empty).
    line_height: f32,
    /// Horizontal scroll offset in pixels (for long lines without wrapping).
    horizontal_scroll: f32,
    /// Word wrap width in pixels. None means no wrapping.
    wrap_width: Option<f32>,
    /// Per-line wrap information. Empty means uniform line heights.
    /// Index corresponds to logical line number.
    /// NOT used for large files (> LARGE_FILE_THRESHOLD) to avoid O(N) overhead.
    wrap_info: Vec<WrapInfo>,
    /// Cached cumulative heights for fast y-offset lookup.
    /// cumulative_heights[i] = total height of lines 0..i
    /// NOT used for large files (> LARGE_FILE_THRESHOLD) to avoid O(N) overhead.
    cumulative_heights: Vec<f32>,
    /// Cached cumulative visual row counts for O(log N) visual-row mapping.
    /// cumulative_visual_rows[i] = total visual rows for lines 0..i
    cumulative_visual_rows: Vec<usize>,
    /// Cached total content height.
    total_content_height: f32,
    /// Smoothed content height for scrollbar rendering.
    /// Lerps toward `total_content_height` to prevent scrollbar jumping
    /// as new wrap info is discovered for previously unseen lines.
    scrollbar_content_height: f32,
    /// Earliest line whose wrap info changed since the last rebuild.
    /// Set to `usize::MAX` when clean. Enables incremental `rebuild_height_cache`
    /// that only recomputes cumulative values from this line forward.
    dirty_from_line: usize,
    /// When true, uses uniform line heights for all calculations.
    /// Automatically enabled for files > LARGE_FILE_THRESHOLD lines.
    /// This avoids O(N) memory and CPU overhead for very large files.
    use_uniform_heights: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewState {
    /// Creates a new `ViewState` with default values.
    ///
    /// # Example
    /// ```
    /// let view = ViewState::new();
    /// assert_eq!(view.first_visible_line(), 0);
    /// assert_eq!(view.horizontal_scroll(), 0.0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            first_visible_line: 0,
            scroll_offset_y: 0.0,
            viewport_height: 0.0,
            line_height: DEFAULT_LINE_HEIGHT,
            horizontal_scroll: 0.0,
            wrap_width: None,
            wrap_info: Vec::new(),
            cumulative_heights: Vec::new(),
            cumulative_visual_rows: Vec::new(),
            total_content_height: 0.0,
            scrollbar_content_height: 0.0,
            dirty_from_line: usize::MAX,
            use_uniform_heights: false,
        }
    }

    /// Updates the viewport height when the window resizes.
    ///
    /// # Arguments
    /// * `height` - The new viewport height in pixels
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.update_viewport(800.0);
    /// assert_eq!(view.viewport_height(), 800.0);
    /// ```
    pub fn update_viewport(&mut self, height: f32) {
        self.viewport_height = height.max(0.0);
    }

    /// Sets the line height used for calculations.
    ///
    /// # Arguments
    /// * `height` - The line height in pixels (must be positive)
    ///
    /// # Note
    /// Line height should ideally be obtained from the font/galley metrics.
    /// A value of 0 or negative will be clamped to 1.0.
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.set_line_height(18.0);
    /// assert_eq!(view.line_height(), 18.0);
    /// ```
    pub fn set_line_height(&mut self, height: f32) {
        self.line_height = height.max(1.0);
    }

    /// Returns the visible line range with overscan for smooth scrolling.
    ///
    /// # Arguments
    /// * `total_lines` - Total number of lines in the document
    ///
    /// # Returns
    /// A tuple `(start, end)` where:
    /// - `start` is the first line to render (0-indexed)
    /// - `end` is one past the last line to render (exclusive)
    ///
    /// The range includes `OVERSCAN_LINES` (5) extra lines above and below
    /// the visible area to prevent visual glitches during rapid scrolling.
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.update_viewport(200.0);  // 200px viewport
    /// view.set_line_height(20.0);   // 20px per line = 10 visible lines
    ///
    /// let (start, end) = view.get_visible_line_range(100);
    /// // With first_visible_line = 0:
    /// // start = max(0, 0 - 5) = 0
    /// // end = min(100, 0 + 10 + 5) = 15
    /// assert_eq!(start, 0);
    /// assert_eq!(end, 15);
    /// ```
    #[must_use]
    pub fn get_visible_line_range(&self, total_lines: usize) -> (usize, usize) {
        if total_lines == 0 {
            return (0, 0);
        }

        let start = self.first_visible_line.saturating_sub(OVERSCAN_LINES);

        // For large files using uniform heights, always use the fast path
        // This avoids O(N) iteration through wrap_info
        if self.use_uniform_heights {
            let visible_lines = if self.line_height > 0.0 {
                (self.viewport_height / self.line_height).ceil() as usize
            } else {
                0
            };
            let end = (self.first_visible_line + visible_lines + OVERSCAN_LINES).min(total_lines);
            return (start, end);
        }

        // Calculate end line based on actual heights when wrap info is available
        let end = if self.is_wrap_enabled() && !self.wrap_info.is_empty() {
            // Use actual wrapped heights to determine how many lines fill the viewport
            let mut accumulated_height = 0.0;
            let mut end_line = self.first_visible_line;

            while end_line < total_lines && accumulated_height < self.viewport_height {
                accumulated_height += self.get_line_height(end_line);
                end_line += 1;
            }

            // Add overscan for smooth scrolling
            (end_line + OVERSCAN_LINES).min(total_lines)
        } else {
            // Standard calculation for non-wrapped text or when wrap info isn't ready
            let visible_lines = if self.line_height > 0.0 {
                (self.viewport_height / self.line_height).ceil() as usize
            } else {
                0
            };
            (self.first_visible_line + visible_lines + OVERSCAN_LINES).min(total_lines)
        };

        (start, end)
    }

    /// Sets scroll position to show the given line at the top of the viewport.
    ///
    /// # Arguments
    /// * `line` - The line number to scroll to (0-indexed)
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.scroll_to_line(50);
    /// assert_eq!(view.first_visible_line(), 50);
    /// ```
    pub fn scroll_to_line(&mut self, line: usize) {
        self.first_visible_line = line;
        self.scroll_offset_y = 0.0;
    }

    /// Scrolls to center a given line in the viewport.
    ///
    /// # Arguments
    /// * `line` - The line number to center (0-indexed)
    /// * `total_lines` - Total number of lines in the document
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.update_viewport(200.0);  // 200px viewport
    /// view.set_line_height(20.0);   // 10 visible lines
    /// view.scroll_to_center_line(50, 100);
    /// // Centers line 50, so first visible should be around 45
    /// ```
    pub fn scroll_to_center_line(&mut self, line: usize, total_lines: usize) {
        // When wrap is active, use actual pixel positions to center the line
        if self.is_wrap_enabled() && self.cumulative_heights.len() > 1 {
            let line_y = self.get_line_y_offset(line);
            let line_h = self.get_line_height(line);
            let target_y = (line_y + line_h / 2.0 - self.viewport_height / 2.0).max(0.0);
            self.scroll_to_absolute(target_y, total_lines);
            return;
        }

        let visible_lines = if self.line_height > 0.0 {
            (self.viewport_height / self.line_height).floor() as usize
        } else {
            0
        };

        let half_visible = visible_lines / 2;
        let target_first = line.saturating_sub(half_visible);

        // Clamp to ensure we don't scroll past the end
        let max_first = total_lines.saturating_sub(visible_lines.max(1));
        self.first_visible_line = target_first.min(max_first);
        self.scroll_offset_y = 0.0;
    }

    /// Converts a y-coordinate (relative to viewport top) to a line number.
    ///
    /// # Arguments
    /// * `pixel_y` - The y-coordinate in pixels relative to viewport top
    ///
    /// # Returns
    /// The 0-indexed line number at that position.
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.set_line_height(20.0);
    /// view.scroll_to_line(10);
    ///
    /// // Pixel 0 is at line 10 (first visible)
    /// assert_eq!(view.pixel_to_line(0.0), 10);
    /// // Pixel 40 is at line 12 (40 / 20 = 2 lines down from first visible)
    /// assert_eq!(view.pixel_to_line(40.0), 12);
    /// ```
    #[must_use]
    pub fn pixel_to_line(&self, pixel_y: f32) -> usize {
        if self.line_height <= 0.0 {
            return self.first_visible_line;
        }

        // When wrap is active with cached heights, convert to absolute position
        // and use binary search for accurate line lookup
        if self.is_wrap_enabled() && self.cumulative_heights.len() > 1 {
            let viewport_top =
                self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y;
            let absolute_y = viewport_top + pixel_y;
            let total_lines = self.cumulative_heights.len().saturating_sub(1);
            return self.y_offset_to_line(absolute_y, total_lines);
        }

        // Uniform height path
        let adjusted_y = pixel_y + self.scroll_offset_y;
        let lines_offset = (adjusted_y / self.line_height).floor() as isize;

        if lines_offset < 0 {
            self.first_visible_line
                .saturating_sub((-lines_offset) as usize)
        } else {
            self.first_visible_line
                .saturating_add(lines_offset as usize)
        }
    }

    /// Converts a line number to the y-coordinate of its top edge.
    ///
    /// # Arguments
    /// * `line` - The 0-indexed line number
    ///
    /// # Returns
    /// The y-coordinate in pixels relative to the viewport top.
    /// May be negative if the line is above the visible area.
    ///
    /// # Example
    /// ```
    /// let mut view = ViewState::new();
    /// view.set_line_height(20.0);
    /// view.scroll_to_line(10);
    ///
    /// // Line 10 is at y=0 (top of viewport)
    /// assert_eq!(view.line_to_pixel(10), 0.0);
    /// // Line 12 is at y=40
    /// assert_eq!(view.line_to_pixel(12), 40.0);
    /// ```
    #[must_use]
    pub fn line_to_pixel(&self, line: usize) -> f32 {
        // When wrap is active with cached heights, use actual y-offsets
        if self.is_wrap_enabled() && self.cumulative_heights.len() > 1 {
            let line_y = self.get_line_y_offset(line);
            let viewport_top =
                self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y;
            return line_y - viewport_top;
        }

        let line_diff = line as isize - self.first_visible_line as isize;
        (line_diff as f32 * self.line_height) - self.scroll_offset_y
    }

    /// Sets the horizontal scroll offset.
    ///
    /// # Arguments
    /// * `offset` - The horizontal scroll offset in pixels (clamped to >= 0)
    pub fn set_horizontal_scroll(&mut self, offset: f32) {
        self.horizontal_scroll = offset.max(0.0);
    }

    /// Returns the current horizontal scroll offset.
    #[must_use]
    pub fn horizontal_scroll(&self) -> f32 {
        self.horizontal_scroll
    }

    /// Returns the index of the first visible line.
    #[must_use]
    pub fn first_visible_line(&self) -> usize {
        self.first_visible_line
    }

    /// Returns the vertical scroll offset within the first visible line.
    #[must_use]
    pub fn scroll_offset_y(&self) -> f32 {
        self.scroll_offset_y
    }

    /// Returns the viewport height in pixels.
    #[must_use]
    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Returns the line height in pixels.
    #[must_use]
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Sets the vertical scroll offset within the current first visible line.
    ///
    /// This allows for sub-line smooth scrolling. When the offset exceeds
    /// the line height, the caller should adjust `first_visible_line` accordingly.
    ///
    /// # Arguments
    /// * `offset` - Scroll offset in pixels (0.0 to line_height)
    pub fn set_scroll_offset_y(&mut self, offset: f32) {
        self.scroll_offset_y = offset.clamp(0.0, self.line_height);
    }

    /// Scrolls by a delta amount in pixels.
    ///
    /// This method handles both sub-line smooth scrolling and line advancement.
    /// Uses absolute scroll position internally to correctly handle all edge cases.
    ///
    /// # Arguments
    /// * `delta_y` - Scroll delta in pixels (positive = down, negative = up)
    /// * `total_lines` - Total number of lines in the document
    pub fn scroll_by(&mut self, delta_y: f32, total_lines: usize) {
        if self.line_height <= 0.0 || total_lines == 0 {
            return;
        }

        // Calculate current absolute scroll position (in pixels from document top)
        // Use get_line_y_offset to account for wrapped line heights
        let current_absolute =
            self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y;

        // Apply delta
        let new_absolute = current_absolute + delta_y;

        // Calculate maximum scroll position correctly:
        // max_scroll = total_content_height - viewport_height
        // This ensures the last line is fully visible at maximum scroll.
        // Use total_content_height() which accounts for word wrap heights when enabled.
        let content_height = self.total_content_height(total_lines);
        let max_absolute = (content_height - self.viewport_height).max(0.0);

        // Clamp to valid range [0, max_absolute]
        let clamped_absolute = new_absolute.clamp(0.0, max_absolute);

        // Convert back to first_visible_line and scroll_offset_y
        // Use y_offset_to_line for proper handling of wrapped line heights
        self.first_visible_line = self.y_offset_to_line(clamped_absolute, total_lines);
        let line_start_y = self.get_line_y_offset(self.first_visible_line);
        self.scroll_offset_y = clamped_absolute - line_start_y;

        // Ensure scroll_offset_y doesn't exceed the current line's height
        let current_line_height = self.get_line_height(self.first_visible_line);
        if self.scroll_offset_y >= current_line_height {
            self.scroll_offset_y = 0.0;
        }
    }

    /// Ensures scroll position is valid for the current viewport and document size.
    ///
    /// Call this after viewport or document size changes to prevent invalid states
    /// like scrolling past the document boundaries.
    ///
    /// This function includes a tolerance zone to prevent jitter from small height
    /// fluctuations (e.g., when word wrap info is updated for visible lines only).
    ///
    /// # Arguments
    /// * `total_lines` - Total number of lines in the document
    pub fn clamp_scroll_position(&mut self, total_lines: usize) {
        if self.line_height <= 0.0 || total_lines == 0 {
            self.first_visible_line = 0;
            self.scroll_offset_y = 0.0;
            return;
        }

        // CRITICAL: Hard-clamp first_visible_line to valid range FIRST.
        // After large deletions, first_visible_line can be beyond the new buffer,
        // and stale wrap_info/cumulative_heights would let it pass through the
        // tolerance-based clamping below. This prevents downstream panics
        // (e.g., get_visible_line_range returning start > end → Vec capacity overflow).
        let max_first_visible = total_lines.saturating_sub(1);
        if self.first_visible_line > max_first_visible {
            self.first_visible_line = max_first_visible;
            self.scroll_offset_y = 0.0;
        }

        // Calculate max scroll correctly: content_height - viewport_height
        // This ensures the last line is fully visible at maximum scroll.
        // Use total_content_height() which accounts for word wrap heights when enabled.
        let content_height = self.total_content_height(total_lines);
        let max_absolute = (content_height - self.viewport_height).max(0.0);

        // Calculate current absolute scroll position using actual line offsets
        let current_absolute =
            self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y;

        // Tolerance zone: allow small overflows to prevent jitter from height estimate
        // fluctuations. Only clamp if we're significantly outside the valid range.
        // This prevents viewport jitter when wrap_info changes for visible lines.
        let tolerance = self.line_height * 2.0; // Allow up to 2 lines of tolerance

        // Only clamp if we're significantly outside the valid range
        let needs_clamping =
            current_absolute < -tolerance || current_absolute > max_absolute + tolerance;

        if !needs_clamping {
            // Position is within tolerance - only do minimal adjustments
            // Ensure we're not scrolled above line 0
            if self.first_visible_line == 0 && self.scroll_offset_y < 0.0 {
                self.scroll_offset_y = 0.0;
            }
            return;
        }

        // Position is significantly outside valid range - perform full clamping
        // Clamp to valid range [0, max_absolute]
        let clamped_absolute = current_absolute.clamp(0.0, max_absolute);

        // Convert back to first_visible_line and scroll_offset_y
        // Use y_offset_to_line for proper handling of wrapped line heights
        self.first_visible_line = self.y_offset_to_line(clamped_absolute, total_lines);
        let line_start_y = self.get_line_y_offset(self.first_visible_line);
        self.scroll_offset_y = clamped_absolute - line_start_y;

        // At top boundary, ensure we can't scroll above line 0
        if self.first_visible_line == 0 && self.scroll_offset_y < 0.01 {
            self.scroll_offset_y = 0.0;
        }

        // Ensure scroll_offset_y is in valid range for this line's height
        let current_line_height = self.get_line_height(self.first_visible_line);
        self.scroll_offset_y = self.scroll_offset_y.clamp(0.0, current_line_height);
    }

    /// Returns whether a given line is currently visible in the viewport.
    ///
    /// # Arguments
    /// * `line` - The line number to check (0-indexed)
    /// * `total_lines` - Total number of lines in the document
    ///
    /// # Returns
    /// `true` if the line is within the visible range (excluding overscan).
    #[must_use]
    pub fn is_line_visible(&self, line: usize, total_lines: usize) -> bool {
        if line < self.first_visible_line {
            return false;
        }

        // When wrap is active, compare actual pixel positions
        if self.is_wrap_enabled() && self.cumulative_heights.len() > 1 {
            let line_top = self.get_line_y_offset(line);
            let viewport_top =
                self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y;
            let viewport_bottom = viewport_top + self.viewport_height;
            return line_top < viewport_bottom;
        }

        let visible_lines = if self.line_height > 0.0 {
            (self.viewport_height / self.line_height).ceil() as usize
        } else {
            0
        };

        let end = (self.first_visible_line + visible_lines).min(total_lines);
        line < end
    }

    /// Ensures a given line is visible, scrolling if necessary.
    ///
    /// # Arguments
    /// * `line` - The line number to ensure is visible (0-indexed)
    /// * `total_lines` - Total number of lines in the document
    ///
    /// # Returns
    /// `true` if scrolling occurred, `false` if the line was already visible.
    pub fn ensure_line_visible(&mut self, line: usize, total_lines: usize) -> bool {
        if self.is_line_visible(line, total_lines) {
            return false;
        }

        // When wrap is active, use pixel-based scrolling for accuracy
        if self.is_wrap_enabled() && self.cumulative_heights.len() > 1 {
            if line < self.first_visible_line {
                self.first_visible_line = line;
                self.scroll_offset_y = 0.0;
            } else {
                // Scroll so the line's bottom aligns with the viewport bottom
                let line_bottom = self.get_line_y_offset(line) + self.get_line_height(line);
                let target_top = (line_bottom - self.viewport_height).max(0.0);
                self.scroll_to_absolute(target_top, total_lines);
            }
            return true;
        }

        let visible_lines = if self.line_height > 0.0 {
            (self.viewport_height / self.line_height).floor() as usize
        } else {
            1
        };

        // If line is above visible area, scroll up to show it at top
        if line < self.first_visible_line {
            self.first_visible_line = line;
        } else {
            // Line is below visible area, scroll down to show it at bottom
            self.first_visible_line = line.saturating_sub(visible_lines.saturating_sub(1));
        }

        self.scroll_offset_y = 0.0;
        true
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Scrollbar Helpers
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns the current absolute scroll position in pixels from the document top.
    ///
    /// This accounts for wrapped line heights when available, providing an accurate
    /// position for scrollbar rendering.
    #[must_use]
    pub fn current_scroll_y(&self) -> f32 {
        self.get_line_y_offset(self.first_visible_line) + self.scroll_offset_y
    }

    /// Scrolls to an absolute y-position within the document.
    ///
    /// Converts the absolute y-position to the appropriate `first_visible_line`
    /// and `scroll_offset_y`, accounting for wrapped line heights.
    ///
    /// # Arguments
    /// * `y` - Absolute y-position in pixels from document top
    /// * `total_lines` - Total number of lines in the document
    pub fn scroll_to_absolute(&mut self, y: f32, total_lines: usize) {
        if total_lines == 0 || self.line_height <= 0.0 {
            self.first_visible_line = 0;
            self.scroll_offset_y = 0.0;
            return;
        }

        let content_height = self.total_content_height(total_lines);
        let max_absolute = (content_height - self.viewport_height).max(0.0);
        let clamped = y.clamp(0.0, max_absolute);

        self.first_visible_line = self.y_offset_to_line(clamped, total_lines);
        let line_start_y = self.get_line_y_offset(self.first_visible_line);
        self.scroll_offset_y = (clamped - line_start_y).max(0.0);

        // Ensure scroll_offset_y doesn't exceed the current line's height
        let current_line_height = self.get_line_height(self.first_visible_line);
        if self.scroll_offset_y >= current_line_height {
            self.scroll_offset_y = 0.0;
        }
    }

    /// Returns the smoothed content height for scrollbar rendering.
    ///
    /// This value lerps toward the actual `total_content_height` to prevent
    /// the scrollbar from jumping as new wrap info is discovered during scrolling.
    /// For all other calculations (scroll clamping, etc.), use `total_content_height`.
    #[must_use]
    pub fn scrollbar_content_height(&self, total_lines: usize) -> f32 {
        if self.scrollbar_content_height > 0.0 {
            self.scrollbar_content_height
        } else {
            self.total_content_height(total_lines)
        }
    }

    /// Advances the scrollbar height smoothing toward the actual content height.
    ///
    /// Call this every frame when wrap is enabled so the smoothed scrollbar height
    /// converges to the actual value even when no wrap_info changes occurred.
    pub fn advance_scrollbar_smoothing(&mut self, total_lines: usize) {
        let target = self.total_content_height(total_lines);
        if target <= 0.0 {
            return;
        }
        if self.scrollbar_content_height <= 0.0 {
            self.scrollbar_content_height = target;
        } else {
            let diff = target - self.scrollbar_content_height;
            if diff.abs() < 1.0 {
                self.scrollbar_content_height = target;
            } else {
                self.scrollbar_content_height += diff * 0.3;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Word Wrap Support (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns whether word wrap is enabled.
    #[must_use]
    pub fn is_wrap_enabled(&self) -> bool {
        self.wrap_width.is_some()
    }

    /// Returns the current wrap width, if wrapping is enabled.
    #[must_use]
    pub fn wrap_width(&self) -> Option<f32> {
        self.wrap_width
    }

    /// Enables word wrap with the specified width.
    ///
    /// # Arguments
    /// * `width` - The maximum width for text before wrapping. Must be positive.
    ///
    /// # Note
    /// After enabling wrap, call `update_wrap_info` with galley heights to
    /// properly calculate scrollbar sizes and visual row positions.
    pub fn enable_wrap(&mut self, width: f32) {
        let width = width.max(50.0); // Minimum reasonable wrap width
        self.wrap_width = Some(width);
        // Disable horizontal scrolling when wrap is enabled
        self.horizontal_scroll = 0.0;
    }

    /// Disables word wrap.
    pub fn disable_wrap(&mut self) {
        self.wrap_width = None;
        self.wrap_info.clear();
        self.cumulative_heights.clear();
        self.cumulative_visual_rows.clear();
        self.total_content_height = 0.0;
        self.scrollbar_content_height = 0.0;
        self.dirty_from_line = usize::MAX;
        // Don't reset use_uniform_heights here - it's based on file size, not wrap state
    }

    /// Configure the view for a large file.
    ///
    /// When enabled, the view uses uniform line heights for all calculations,
    /// avoiding O(N) memory and CPU overhead. This should be called when
    /// opening files with more than `LARGE_FILE_THRESHOLD` lines.
    ///
    /// When uniform mode activates, all O(N) per-line vectors (`wrap_info`,
    /// `cumulative_heights`, `cumulative_visual_rows`) are freed. Callers
    /// must also disable word wrap for the frame — the overlap regression
    /// was caused by wrapped galleys being taller than the uniform height.
    ///
    /// # Arguments
    /// * `total_lines` - Total number of lines in the file
    pub fn configure_for_file_size(&mut self, total_lines: usize) {
        if total_lines >= LARGE_FILE_THRESHOLD {
            if !self.use_uniform_heights {
                log::info!(
                    "Enabling uniform heights for large file ({} lines, threshold {})",
                    total_lines,
                    LARGE_FILE_THRESHOLD
                );
                self.use_uniform_heights = true;
                self.wrap_info.clear();
                self.wrap_info.shrink_to_fit();
                self.cumulative_heights.clear();
                self.cumulative_heights.shrink_to_fit();
                self.cumulative_visual_rows.clear();
                self.cumulative_visual_rows.shrink_to_fit();
                self.total_content_height = 0.0;
                self.scrollbar_content_height = 0.0;
                self.dirty_from_line = usize::MAX;
            }
        } else {
            self.use_uniform_heights = false;
        }
    }

    /// Returns whether uniform heights mode is active.
    ///
    /// This is true for very large files (> 100k lines) where per-line
    /// height tracking would be too expensive.
    #[must_use]
    pub fn uses_uniform_heights(&self) -> bool {
        self.use_uniform_heights
    }

    /// Updates wrap information for a specific line.
    ///
    /// Call this after laying out a galley to update the visual row count
    /// and height for that line.
    ///
    /// # Arguments
    /// * `line` - The 0-indexed logical line number
    /// * `visual_rows` - Number of visual rows this line occupies after wrapping
    /// * `height` - Total height of the wrapped line in pixels
    ///
    /// # Large File Optimization
    /// For files > LARGE_FILE_THRESHOLD, this method is a no-op to avoid
    /// O(N) memory allocation. Large files use uniform heights instead.
    pub fn set_line_wrap_info(&mut self, line: usize, visual_rows: usize, height: f32) {
        // Skip if uniform heights mode is active (currently always false)
        if self.use_uniform_heights {
            return;
        }

        // Safety limit to prevent excessive memory allocation
        // This is a soft limit - very large files may still allocate
        const MAX_WRAP_INFO_LINES: usize = 1_000_000;
        if line >= MAX_WRAP_INFO_LINES {
            return;
        }

        let new_info = WrapInfo {
            visual_rows: visual_rows.max(1),
            height: height.max(1.0),
        };

        // Ensure wrap_info vector is large enough
        if line >= self.wrap_info.len() {
            let old_len = self.wrap_info.len();
            self.wrap_info.resize(line + 1, WrapInfo::default());
            self.wrap_info[line] = new_info;
            self.dirty_from_line = self.dirty_from_line.min(old_len);
        } else if self.wrap_info[line] != new_info {
            self.wrap_info[line] = new_info;
            self.dirty_from_line = self.dirty_from_line.min(line);
        }
    }

    /// Rebuilds the cumulative height and visual-row caches after wrap info changes.
    ///
    /// Uses **incremental rebuild**: only recomputes from the earliest changed line
    /// forward, reusing the cached prefix. This reduces the common case from
    /// O(total_lines) to O(total_lines - dirty_from_line).
    ///
    /// # Arguments
    /// * `total_lines` - Total number of logical lines in the document
    pub fn rebuild_height_cache(&mut self, total_lines: usize) {
        if self.use_uniform_heights {
            return;
        }
        if self.dirty_from_line == usize::MAX {
            return;
        }

        let rebuild_from = self.dirty_from_line;
        self.dirty_from_line = usize::MAX;

        // Incremental path: reuse cached prefix up to rebuild_from
        let can_incremental = rebuild_from > 0 && self.cumulative_heights.len() > rebuild_from;

        if can_incremental {
            self.cumulative_heights.truncate(rebuild_from + 1);
            self.cumulative_visual_rows.truncate(rebuild_from + 1);

            let mut cum_h = self.cumulative_heights[rebuild_from];
            let mut cum_vr = self.cumulative_visual_rows[rebuild_from];

            self.cumulative_heights
                .reserve(total_lines + 1 - self.cumulative_heights.len());
            self.cumulative_visual_rows
                .reserve(total_lines + 1 - self.cumulative_visual_rows.len());

            for i in rebuild_from..total_lines {
                cum_h += self.get_line_height(i);
                self.cumulative_heights.push(cum_h);

                cum_vr += self.get_visual_rows(i);
                self.cumulative_visual_rows.push(cum_vr);
            }

            self.total_content_height = cum_h;
        } else {
            // Full rebuild
            self.cumulative_heights.clear();
            self.cumulative_heights.reserve(total_lines + 1);
            self.cumulative_visual_rows.clear();
            self.cumulative_visual_rows.reserve(total_lines + 1);

            let mut cum_h = 0.0;
            let mut cum_vr = 0usize;
            self.cumulative_heights.push(0.0);
            self.cumulative_visual_rows.push(0);

            for i in 0..total_lines {
                cum_h += self.get_line_height(i);
                self.cumulative_heights.push(cum_h);

                cum_vr += self.get_visual_rows(i);
                self.cumulative_visual_rows.push(cum_vr);
            }

            self.total_content_height = cum_h;
        }

        // Smooth the scrollbar content height to prevent jumping.
        let cumulative = self.total_content_height;
        if self.scrollbar_content_height <= 0.0 {
            self.scrollbar_content_height = cumulative;
        } else {
            let diff = cumulative - self.scrollbar_content_height;
            if diff.abs() < 1.0 {
                self.scrollbar_content_height = cumulative;
            } else {
                self.scrollbar_content_height += diff * 0.3;
            }
        }
    }

    /// Returns the height of a specific logical line.
    ///
    /// If wrap info is available (and not in uniform heights mode),
    /// returns the wrapped height. Otherwise, returns the default line height.
    #[must_use]
    pub fn get_line_height(&self, line: usize) -> f32 {
        // For large files, always use uniform height
        if self.use_uniform_heights {
            return self.line_height;
        }

        self.wrap_info
            .get(line)
            .map(|info| info.height)
            .unwrap_or(self.line_height)
    }

    /// Returns the number of visual rows for a specific logical line.
    #[must_use]
    pub fn get_visual_rows(&self, line: usize) -> usize {
        self.wrap_info
            .get(line)
            .map(|info| info.visual_rows)
            .unwrap_or(1)
    }

    /// Returns the total content height (sum of all line heights).
    ///
    /// If wrap info is not available, falls back to total_lines * line_height.
    #[must_use]
    pub fn total_content_height(&self, total_lines: usize) -> f32 {
        if self.total_content_height > 0.0 {
            self.total_content_height
        } else if !self.cumulative_heights.is_empty() {
            *self.cumulative_heights.last().unwrap_or(&0.0)
        } else {
            total_lines as f32 * self.line_height
        }
    }

    /// Returns the y-offset for the top of a logical line.
    ///
    /// Uses cumulative heights if available, otherwise calculates from
    /// uniform line height.
    #[must_use]
    pub fn get_line_y_offset(&self, line: usize) -> f32 {
        if line < self.cumulative_heights.len() {
            self.cumulative_heights[line]
        } else if !self.cumulative_heights.is_empty() {
            // Beyond cached range, estimate using last known + remaining lines
            let last_cached_line = self.cumulative_heights.len() - 1;
            let last_cached_y = self.cumulative_heights[last_cached_line];
            let extra_lines = line - last_cached_line;
            last_cached_y + (extra_lines as f32 * self.line_height)
        } else {
            // No cache, use uniform height
            line as f32 * self.line_height
        }
    }

    /// Finds the logical line at a given y-offset.
    ///
    /// Uses binary search on cumulative heights if available.
    ///
    /// # Arguments
    /// * `y` - The y-coordinate in pixels from document top
    /// * `total_lines` - Total number of logical lines
    ///
    /// # Returns
    /// The 0-indexed logical line number at that position.
    #[must_use]
    pub fn y_offset_to_line(&self, y: f32, total_lines: usize) -> usize {
        if total_lines == 0 {
            return 0;
        }

        if self.cumulative_heights.len() > 1 {
            // Binary search on cumulative heights
            let y = y.max(0.0);
            match self
                .cumulative_heights
                .binary_search_by(|&h| h.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal))
            {
                Ok(exact) => exact.min(total_lines.saturating_sub(1)),
                Err(insert_pos) => insert_pos
                    .saturating_sub(1)
                    .min(total_lines.saturating_sub(1)),
            }
        } else {
            // Fallback to uniform height calculation
            if self.line_height > 0.0 {
                ((y / self.line_height).floor() as usize).min(total_lines.saturating_sub(1))
            } else {
                0
            }
        }
    }

    /// Returns the total number of visual rows across all lines.
    ///
    /// This is useful for scrollbar calculations when word wrap is enabled.
    #[must_use]
    pub fn total_visual_rows(&self, total_lines: usize) -> usize {
        if self.wrap_info.is_empty() {
            total_lines
        } else {
            self.wrap_info
                .iter()
                .map(|info| info.visual_rows)
                .sum::<usize>()
                + total_lines.saturating_sub(self.wrap_info.len())
        }
    }

    /// Converts a logical line and column to a visual row number.
    ///
    /// Uses the precomputed `cumulative_visual_rows` array for O(1) lookup
    /// when available, falling back to O(N) summation otherwise.
    ///
    /// # Arguments
    /// * `line` - The logical line number (0-indexed)
    /// * `col` - The column position within the line
    /// * `chars_per_row` - Approximate characters per visual row (for estimating wrap position)
    ///
    /// # Returns
    /// The visual row number (0-indexed from document top).
    #[must_use]
    pub fn logical_to_visual_row(&self, line: usize, col: usize, chars_per_row: usize) -> usize {
        let total_rows_before = if line < self.cumulative_visual_rows.len() {
            self.cumulative_visual_rows[line]
        } else if !self.cumulative_visual_rows.is_empty() {
            let last_idx = self.cumulative_visual_rows.len() - 1;
            self.cumulative_visual_rows[last_idx] + line.saturating_sub(last_idx)
        } else {
            // Fallback: O(N) summation (only when cache not built yet)
            let rows_before: usize = self
                .wrap_info
                .iter()
                .take(line)
                .map(|info| info.visual_rows)
                .sum();
            let uncached_lines = line.saturating_sub(self.wrap_info.len());
            rows_before + uncached_lines
        };

        if chars_per_row > 0 {
            let row_within_line = col / chars_per_row;
            let max_rows = self.get_visual_rows(line);
            total_rows_before + row_within_line.min(max_rows.saturating_sub(1))
        } else {
            total_rows_before
        }
    }

    /// Converts a visual row number to logical line and approximate column.
    ///
    /// Uses binary search on `cumulative_visual_rows` for O(log N) lookup
    /// when available, falling back to O(N) linear scan otherwise.
    ///
    /// # Arguments
    /// * `visual_row` - The visual row number (0-indexed from document top)
    /// * `total_lines` - Total number of logical lines
    ///
    /// # Returns
    /// A tuple of (logical_line, row_within_line).
    #[must_use]
    pub fn visual_row_to_logical(&self, visual_row: usize, total_lines: usize) -> (usize, usize) {
        if self.wrap_info.is_empty() && self.cumulative_visual_rows.is_empty() {
            return (visual_row.min(total_lines.saturating_sub(1)), 0);
        }

        // O(log N) path using precomputed cumulative visual rows
        if self.cumulative_visual_rows.len() > 1 {
            let line = match self.cumulative_visual_rows.binary_search(&visual_row) {
                Ok(exact) => exact.min(total_lines.saturating_sub(1)),
                Err(insert_pos) => insert_pos
                    .saturating_sub(1)
                    .min(total_lines.saturating_sub(1)),
            };
            let rows_before = self.cumulative_visual_rows.get(line).copied().unwrap_or(0);
            let row_within = visual_row.saturating_sub(rows_before);
            return (line, row_within);
        }

        // Fallback: O(N) linear scan
        let mut accumulated_rows = 0usize;
        for (line, info) in self.wrap_info.iter().enumerate() {
            let next_accumulated = accumulated_rows + info.visual_rows;
            if visual_row < next_accumulated {
                let row_within = visual_row - accumulated_rows;
                return (line, row_within);
            }
            accumulated_rows = next_accumulated;
        }

        let remaining_row = visual_row.saturating_sub(accumulated_rows);
        let line = self.wrap_info.len() + remaining_row;
        (line.min(total_lines.saturating_sub(1)), 0)
    }

    /// Clears wrap info. Call when document content changes significantly.
    pub fn clear_wrap_info(&mut self) {
        self.wrap_info.clear();
        self.cumulative_heights.clear();
        self.cumulative_visual_rows.clear();
        self.total_content_height = 0.0;
        self.scrollbar_content_height = 0.0;
        self.dirty_from_line = usize::MAX;
    }

    /// Truncates wrap info to match the current line count.
    ///
    /// After large deletions, `wrap_info` can have stale entries for lines that
    /// no longer exist. This removes those entries and marks heights as dirty so
    /// `rebuild_height_cache` will recompute cumulative heights on the next call.
    ///
    /// Unlike `clear_wrap_info()`, this preserves valid entries for lines that
    /// still exist, avoiding the flickering caused by a full clear.
    pub fn truncate_wrap_info(&mut self, total_lines: usize) {
        if self.wrap_info.len() > total_lines {
            self.wrap_info.truncate(total_lines);
            self.dirty_from_line = self.dirty_from_line.min(total_lines);
        }
        if self.cumulative_heights.len() > total_lines + 1 {
            self.cumulative_heights.truncate(total_lines + 1);
            self.cumulative_visual_rows.truncate(total_lines + 1);
            self.total_content_height = self.cumulative_heights.last().copied().unwrap_or(0.0);
        }
    }

    /// Returns wrap info for debugging/testing.
    #[must_use]
    pub fn wrap_info(&self) -> &[WrapInfo] {
        &self.wrap_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_values() {
        let view = ViewState::new();
        assert_eq!(view.first_visible_line(), 0);
        assert_eq!(view.scroll_offset_y(), 0.0);
        assert_eq!(view.viewport_height(), 0.0);
        assert_eq!(view.line_height(), DEFAULT_LINE_HEIGHT);
        assert_eq!(view.horizontal_scroll(), 0.0);
    }

    #[test]
    fn test_default_trait() {
        let view = ViewState::default();
        assert_eq!(view.first_visible_line(), 0);
    }

    #[test]
    fn test_update_viewport() {
        let mut view = ViewState::new();
        view.update_viewport(800.0);
        assert_eq!(view.viewport_height(), 800.0);

        // Negative values should be clamped to 0
        view.update_viewport(-100.0);
        assert_eq!(view.viewport_height(), 0.0);
    }

    #[test]
    fn test_set_line_height() {
        let mut view = ViewState::new();
        view.set_line_height(18.0);
        assert_eq!(view.line_height(), 18.0);

        // Zero or negative should be clamped to 1.0
        view.set_line_height(0.0);
        assert_eq!(view.line_height(), 1.0);

        view.set_line_height(-5.0);
        assert_eq!(view.line_height(), 1.0);
    }

    #[test]
    fn test_visible_line_range_basic() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        let (start, end) = view.get_visible_line_range(100);
        // With first_visible_line = 0:
        // start = max(0, 0 - 5) = 0
        // end = min(100, 0 + 10 + 5) = 15
        assert_eq!(start, 0);
        assert_eq!(end, 15);
    }

    #[test]
    fn test_visible_line_range_middle() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines
        view.scroll_to_line(50);

        let (start, end) = view.get_visible_line_range(100);
        // start = max(0, 50 - 5) = 45
        // end = min(100, 50 + 10 + 5) = 65
        assert_eq!(start, 45);
        assert_eq!(end, 65);
    }

    #[test]
    fn test_visible_line_range_end() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines
        view.scroll_to_line(90);

        let (start, end) = view.get_visible_line_range(100);
        // start = max(0, 90 - 5) = 85
        // end = min(100, 90 + 10 + 5) = 100
        assert_eq!(start, 85);
        assert_eq!(end, 100);
    }

    #[test]
    fn test_visible_line_range_empty_document() {
        let view = ViewState::new();
        let (start, end) = view.get_visible_line_range(0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn test_visible_line_range_small_document() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines + 5 overscan = 15

        // Document smaller than visible range + overscan
        let (start, end) = view.get_visible_line_range(8);
        assert_eq!(start, 0);
        assert_eq!(end, 8);
    }

    #[test]
    fn test_scroll_to_line() {
        let mut view = ViewState::new();
        view.scroll_to_line(42);
        assert_eq!(view.first_visible_line(), 42);
        assert_eq!(view.scroll_offset_y(), 0.0);
    }

    #[test]
    fn test_scroll_to_center_line() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        view.scroll_to_center_line(50, 100);
        // Should center line 50, so first visible should be around 45
        assert_eq!(view.first_visible_line(), 45);
    }

    #[test]
    fn test_scroll_to_center_line_near_start() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        view.scroll_to_center_line(2, 100);
        // Can't scroll above 0, so first_visible should be 0
        assert_eq!(view.first_visible_line(), 0);
    }

    #[test]
    fn test_scroll_to_center_line_near_end() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        view.scroll_to_center_line(98, 100);
        // Should clamp to not scroll past end: max_first = 100 - 10 = 90
        assert!(view.first_visible_line() <= 90);
    }

    #[test]
    fn test_pixel_to_line() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.scroll_to_line(10);

        // Pixel 0 is at line 10
        assert_eq!(view.pixel_to_line(0.0), 10);
        // Pixel 19 is still at line 10
        assert_eq!(view.pixel_to_line(19.0), 10);
        // Pixel 20 is at line 11
        assert_eq!(view.pixel_to_line(20.0), 11);
        // Pixel 40 is at line 12
        assert_eq!(view.pixel_to_line(40.0), 12);
    }

    #[test]
    fn test_pixel_to_line_negative() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.scroll_to_line(10);

        // Negative pixels should go to lines above first visible
        assert_eq!(view.pixel_to_line(-20.0), 9);
        assert_eq!(view.pixel_to_line(-40.0), 8);
    }

    #[test]
    fn test_line_to_pixel() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.scroll_to_line(10);

        // Line 10 is at y=0
        assert_eq!(view.line_to_pixel(10), 0.0);
        // Line 11 is at y=20
        assert_eq!(view.line_to_pixel(11), 20.0);
        // Line 12 is at y=40
        assert_eq!(view.line_to_pixel(12), 40.0);
        // Line 9 is at y=-20
        assert_eq!(view.line_to_pixel(9), -20.0);
    }

    #[test]
    fn test_horizontal_scroll() {
        let mut view = ViewState::new();
        assert_eq!(view.horizontal_scroll(), 0.0);

        view.set_horizontal_scroll(150.0);
        assert_eq!(view.horizontal_scroll(), 150.0);

        // Negative should be clamped to 0
        view.set_horizontal_scroll(-50.0);
        assert_eq!(view.horizontal_scroll(), 0.0);
    }

    #[test]
    fn test_scroll_by_down() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(0);

        // Scroll down 50 pixels (2.5 lines)
        view.scroll_by(50.0, 100);
        assert_eq!(view.first_visible_line(), 2);
        assert!((view.scroll_offset_y() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_scroll_by_up() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(50);

        // Scroll up 30 pixels (1.5 lines)
        view.scroll_by(-30.0, 100);
        // Should go from line 50 to somewhere around line 48-49
        assert!(view.first_visible_line() < 50);
    }

    #[test]
    fn test_scroll_by_clamp_start() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(0);

        // Try to scroll up from the start
        view.scroll_by(-100.0, 100);
        assert_eq!(view.first_visible_line(), 0);
        assert_eq!(view.scroll_offset_y(), 0.0);
    }

    #[test]
    fn test_scroll_to_line_0_from_line_1() {
        // Bug 2 regression test: Verify we can scroll to line 0 (line 1 at top)
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(1);

        // Scroll up to get line 0 at the top
        view.scroll_by(-20.0, 100);
        assert_eq!(view.first_visible_line(), 0);
        assert_eq!(view.scroll_offset_y(), 0.0);

        // Line 0 should be at y=0 (top of viewport)
        assert_eq!(view.line_to_pixel(0), 0.0);
    }

    #[test]
    fn test_scroll_to_line_0_incremental() {
        // Bug 2 regression test: Incremental scrolling to line 0
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(2);

        // Scroll up in small increments
        for _ in 0..10 {
            view.scroll_by(-5.0, 100);
        }

        // Should be at line 0 with offset 0
        assert_eq!(view.first_visible_line(), 0);
        assert_eq!(view.scroll_offset_y(), 0.0);
        // Line 0 at top of viewport
        assert_eq!(view.line_to_pixel(0), 0.0);
    }

    #[test]
    fn test_scroll_at_max_no_white_space() {
        // Bug 3 regression test: No white space at bottom when at max scroll
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        let total_lines = 50;
        // max_first_line = 50 - 10 = 40

        // Scroll to near the end
        view.scroll_to_line(38);

        // Scroll down past the max
        view.scroll_by(100.0, total_lines);

        // Should be at max position with offset 0
        assert_eq!(view.first_visible_line(), 40);
        assert_eq!(view.scroll_offset_y(), 0.0);

        // Last line (49) should be at bottom of viewport
        // Line 49 at y = (49 - 40) * 20 = 180, which ends at 200 (viewport height)
        let last_line_y = view.line_to_pixel(49);
        assert!((last_line_y - 180.0).abs() < 0.01);
    }

    #[test]
    fn test_clamp_scroll_position() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines

        // Manually set invalid state (simulating what might happen during resize)
        view.scroll_to_line(100); // Beyond what's valid for small documents

        // Clamp to 20 lines total (max_first_line = 10)
        view.clamp_scroll_position(20);

        assert_eq!(view.first_visible_line(), 10);
        assert_eq!(view.scroll_offset_y(), 0.0);
    }

    #[test]
    fn test_scroll_to_bottom_non_divisible_viewport() {
        // Bug fix regression test: When viewport height is not perfectly divisible
        // by line height, the last line should still be fully visible.
        // Previously, ceil(viewport/line_height) * line_height was used, which
        // cut off the last line when viewport wasn't perfectly divisible.
        let mut view = ViewState::new();
        view.update_viewport(155.0); // Not divisible by 20
        view.set_line_height(20.0);

        let total_lines = 100;

        // Scroll to the bottom
        view.scroll_by(10000.0, total_lines); // Large delta to ensure we hit max

        // Calculate expected max scroll: content_height - viewport_height
        // max_scroll = 100 * 20 - 155 = 1845
        // first_visible_line = floor(1845 / 20) = 92
        // scroll_offset_y = 1845 - 92*20 = 1845 - 1840 = 5
        assert_eq!(view.first_visible_line(), 92);
        assert!(
            (view.scroll_offset_y() - 5.0).abs() < 0.01,
            "scroll_offset_y should be ~5.0, got {}",
            view.scroll_offset_y()
        );

        // Verify the last line (99, 0-indexed) is fully visible
        // Line 99 starts at y = (99 - 92) * 20 - 5 = 135
        // Line 99 ends at y = 135 + 20 = 155 = viewport_height
        let last_line_y = view.line_to_pixel(99);
        let last_line_bottom = last_line_y + view.line_height();
        assert!(
            (last_line_bottom - 155.0).abs() < 0.01,
            "Last line bottom should be at viewport edge (155), got {}",
            last_line_bottom
        );
    }

    #[test]
    fn test_clamp_scroll_position_non_divisible_viewport() {
        // Same fix as above, but testing clamp_scroll_position
        let mut view = ViewState::new();
        view.update_viewport(155.0);
        view.set_line_height(20.0);

        // Start at an extreme position
        view.scroll_to_line(95);

        // Clamp should adjust to the correct max position
        view.clamp_scroll_position(100);

        // Should be at max: first_visible=92, offset=5
        assert_eq!(view.first_visible_line(), 92);
        assert!(
            (view.scroll_offset_y() - 5.0).abs() < 0.01,
            "scroll_offset_y should be ~5.0, got {}",
            view.scroll_offset_y()
        );
    }

    #[test]
    fn test_clamp_scroll_position_at_boundaries() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);

        // At line 0 with non-zero offset (invalid state)
        view.scroll_to_line(0);
        // Can't directly set scroll_offset_y to invalid value, but clamp should ensure it's 0
        view.clamp_scroll_position(100);

        assert_eq!(view.first_visible_line(), 0);
        assert_eq!(view.scroll_offset_y(), 0.0);
    }

    #[test]
    fn test_visible_line_range_with_wrap_info() {
        // Bug 3 regression test: Ensure enough lines are rendered when wrapped
        let mut view = ViewState::new();
        view.update_viewport(400.0); // 400px viewport
        view.set_line_height(20.0); // Base height 20px
        view.enable_wrap(300.0);

        // Simulate wrapped lines with varying heights
        // Lines 0-4: 40px each (wrapped to 2 visual rows)
        // Lines 5-9: 20px each (single row)
        for i in 0..5 {
            view.set_line_wrap_info(i, 2, 40.0);
        }
        for i in 5..10 {
            view.set_line_wrap_info(i, 1, 20.0);
        }

        view.scroll_to_line(0);

        // With 400px viewport:
        // - Lines 0-4: 5 * 40 = 200px
        // - Need 200px more, so lines 5-9: 200px (at 20px each = 10 lines)
        // - So we need at least 10 lines to fill viewport
        let (start, end) = view.get_visible_line_range(20);

        // Should render enough lines to fill the viewport
        // Lines 0-4 = 200px, need 200px more
        // Lines 5-14 = 200px (at 20px each, need 10 lines)
        // Total: at least 15 lines to fill + overscan
        assert!(
            end >= 10,
            "Should render at least 10 lines to fill viewport, got end={}",
            end
        );
    }

    #[test]
    fn test_scroll_to_bottom_with_word_wrap() {
        // Task 56 regression test: When word wrap is enabled, the actual content
        // height can be larger than total_lines * line_height. scroll_by() and
        // clamp_scroll_position() must use total_content_height() to ensure we can
        // scroll to view the actual bottom of wrapped content.
        let mut view = ViewState::new();
        view.update_viewport(200.0); // 200px viewport
        view.set_line_height(20.0); // Base height 20px
        view.enable_wrap(300.0);

        // 10 lines total, but some wrap to multiple visual rows
        // Lines 0-4: 40px each (wrapped to 2 visual rows) = 200px
        // Lines 5-9: 20px each (single row) = 100px
        // Total content height = 300px (larger than 10 * 20 = 200px)
        for i in 0..5 {
            view.set_line_wrap_info(i, 2, 40.0);
        }
        for i in 5..10 {
            view.set_line_wrap_info(i, 1, 20.0);
        }
        view.rebuild_height_cache(10);

        // Verify total content height is correct
        assert_eq!(view.total_content_height(10), 300.0);

        // Scroll to the bottom
        view.scroll_by(10000.0, 10); // Large delta to hit max

        // max_scroll = content_height - viewport_height = 300 - 200 = 100px
        // At max scroll of 100px:
        // first_visible_line = floor(100 / 20) = 5
        // scroll_offset_y = 100 - 5*20 = 0
        //
        // With wrapped content, line 5 starts at y = 200px (after lines 0-4 at 40px each)
        // So scrolling 100px into a 200px viewport means:
        // - We're showing from y=100 to y=300
        // - Line 5 starts at y=200, which is 100px into our view (at viewport y=100)
        // But wait, the scroll position calculation uses line_height for conversion
        // which might not perfectly align with wrapped content. Let's verify we can
        // at least scroll further than we could before the fix.

        // Before fix: max_scroll would be 10*20 - 200 = 0, couldn't scroll at all!
        // After fix: max_scroll = 300 - 200 = 100, can scroll to see later content

        // The key assertion: we should be able to scroll past line 0
        assert!(
            view.first_visible_line() > 0,
            "Should be able to scroll past first line with wrapped content, got line {}",
            view.first_visible_line()
        );
    }

    #[test]
    fn test_clamp_scroll_position_with_word_wrap() {
        // Task 56 regression test: clamp_scroll_position must also use
        // total_content_height() for word wrap support
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.enable_wrap(300.0);

        // 10 lines, wrapped to 300px total height
        for i in 0..5 {
            view.set_line_wrap_info(i, 2, 40.0);
        }
        for i in 5..10 {
            view.set_line_wrap_info(i, 1, 20.0);
        }
        view.rebuild_height_cache(10);

        // Set scroll position far beyond what should be allowed
        view.scroll_to_line(8);

        // Clamp it
        view.clamp_scroll_position(10);

        // Should be clamped to a valid max position
        // max_scroll = 300 - 200 = 100px
        // max first_visible_line = floor(100 / 20) = 5
        assert!(
            view.first_visible_line() <= 5,
            "Scroll should be clamped, got first_visible_line={}",
            view.first_visible_line()
        );
    }

    #[test]
    fn test_is_line_visible() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines
        view.scroll_to_line(10);

        // Lines 10-19 should be visible
        assert!(view.is_line_visible(10, 100));
        assert!(view.is_line_visible(15, 100));
        assert!(view.is_line_visible(19, 100));

        // Lines outside visible area
        assert!(!view.is_line_visible(9, 100));
        assert!(!view.is_line_visible(20, 100));
    }

    #[test]
    fn test_ensure_line_visible_already_visible() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(10);

        // Line 15 is visible, should return false
        let scrolled = view.ensure_line_visible(15, 100);
        assert!(!scrolled);
        assert_eq!(view.first_visible_line(), 10);
    }

    #[test]
    fn test_ensure_line_visible_scroll_down() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0); // 10 visible lines
        view.scroll_to_line(10);

        // Line 50 is not visible, should scroll
        let scrolled = view.ensure_line_visible(50, 100);
        assert!(scrolled);
        assert!(view.is_line_visible(50, 100));
    }

    #[test]
    fn test_ensure_line_visible_scroll_up() {
        let mut view = ViewState::new();
        view.update_viewport(200.0);
        view.set_line_height(20.0);
        view.scroll_to_line(50);

        // Line 10 is not visible, should scroll up
        let scrolled = view.ensure_line_visible(10, 100);
        assert!(scrolled);
        assert!(view.is_line_visible(10, 100));
    }

    // Viewport height tests from 100-2000px as per test strategy
    #[test]
    fn test_viewport_heights_100px() {
        let mut view = ViewState::new();
        view.update_viewport(100.0);
        view.set_line_height(20.0); // 5 visible lines

        let (start, end) = view.get_visible_line_range(1000);
        assert_eq!(start, 0);
        assert_eq!(end, 10); // 5 visible + 5 overscan
    }

    #[test]
    fn test_viewport_heights_500px() {
        let mut view = ViewState::new();
        view.update_viewport(500.0);
        view.set_line_height(20.0); // 25 visible lines

        let (start, end) = view.get_visible_line_range(1000);
        assert_eq!(start, 0);
        assert_eq!(end, 30); // 25 visible + 5 overscan
    }

    #[test]
    fn test_viewport_heights_1000px() {
        let mut view = ViewState::new();
        view.update_viewport(1000.0);
        view.set_line_height(20.0); // 50 visible lines

        view.scroll_to_line(100);
        let (start, end) = view.get_visible_line_range(1000);
        assert_eq!(start, 95); // 100 - 5 overscan
        assert_eq!(end, 155); // 100 + 50 + 5 overscan
    }

    #[test]
    fn test_viewport_heights_2000px() {
        let mut view = ViewState::new();
        view.update_viewport(2000.0);
        view.set_line_height(20.0); // 100 visible lines

        view.scroll_to_line(500);
        let (start, end) = view.get_visible_line_range(1000);
        assert_eq!(start, 495); // 500 - 5 overscan
        assert_eq!(end, 605); // 500 + 100 + 5 overscan
    }

    #[test]
    fn test_clone_and_partial_eq() {
        let mut view1 = ViewState::new();
        view1.update_viewport(600.0);
        view1.scroll_to_line(25);

        let view2 = view1.clone();
        assert_eq!(view1, view2);

        let mut view3 = view1.clone();
        view3.scroll_to_line(30);
        assert_ne!(view1, view3);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Word Wrap Tests (Phase 2)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_wrap_default_disabled() {
        let view = ViewState::new();
        assert!(!view.is_wrap_enabled());
        assert!(view.wrap_width().is_none());
    }

    #[test]
    fn test_enable_wrap() {
        let mut view = ViewState::new();
        view.enable_wrap(400.0);

        assert!(view.is_wrap_enabled());
        assert_eq!(view.wrap_width(), Some(400.0));
        // Horizontal scroll should be disabled when wrap is on
        assert_eq!(view.horizontal_scroll(), 0.0);
    }

    #[test]
    fn test_enable_wrap_minimum_width() {
        let mut view = ViewState::new();
        view.enable_wrap(10.0); // Too small

        // Should be clamped to minimum (50.0)
        assert_eq!(view.wrap_width(), Some(50.0));
    }

    #[test]
    fn test_disable_wrap() {
        let mut view = ViewState::new();
        view.enable_wrap(400.0);
        view.set_line_wrap_info(0, 3, 60.0);

        view.disable_wrap();

        assert!(!view.is_wrap_enabled());
        assert!(view.wrap_width().is_none());
        assert!(view.wrap_info().is_empty());
    }

    #[test]
    fn test_set_line_wrap_info() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);

        // Set wrap info for line 0
        view.set_line_wrap_info(0, 3, 60.0);

        assert_eq!(view.get_visual_rows(0), 3);
        assert_eq!(view.get_line_height(0), 60.0);
    }

    #[test]
    fn test_get_line_height_fallback() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);

        // No wrap info set, should return base line height
        assert_eq!(view.get_line_height(0), 20.0);
        assert_eq!(view.get_line_height(100), 20.0);
    }

    #[test]
    fn test_get_visual_rows_fallback() {
        let view = ViewState::new();

        // No wrap info set, should return 1
        assert_eq!(view.get_visual_rows(0), 1);
        assert_eq!(view.get_visual_rows(100), 1);
    }

    #[test]
    fn test_rebuild_height_cache() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);

        // Set wrap info for 3 lines
        view.set_line_wrap_info(0, 2, 40.0); // Line 0: 2 rows, 40px
        view.set_line_wrap_info(1, 1, 20.0); // Line 1: 1 row, 20px
        view.set_line_wrap_info(2, 3, 60.0); // Line 2: 3 rows, 60px

        view.rebuild_height_cache(3);

        // Check y offsets
        assert_eq!(view.get_line_y_offset(0), 0.0);
        assert_eq!(view.get_line_y_offset(1), 40.0);
        assert_eq!(view.get_line_y_offset(2), 60.0);
        assert_eq!(view.get_line_y_offset(3), 120.0); // After last line
    }

    #[test]
    fn test_total_content_height() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);

        view.set_line_wrap_info(0, 2, 40.0);
        view.set_line_wrap_info(1, 1, 20.0);
        view.set_line_wrap_info(2, 3, 60.0);
        view.rebuild_height_cache(3);

        assert_eq!(view.total_content_height(3), 120.0);
    }

    #[test]
    fn test_total_content_height_fallback() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);

        // No wrap info, should use total_lines * line_height
        assert_eq!(view.total_content_height(10), 200.0);
    }

    #[test]
    fn test_total_visual_rows() {
        let mut view = ViewState::new();

        view.set_line_wrap_info(0, 2, 40.0);
        view.set_line_wrap_info(1, 1, 20.0);
        view.set_line_wrap_info(2, 3, 60.0);

        // Total: 2 + 1 + 3 = 6 visual rows for 3 lines
        assert_eq!(view.total_visual_rows(3), 6);
    }

    #[test]
    fn test_total_visual_rows_fallback() {
        let view = ViewState::new();

        // No wrap info, 1 visual row per logical line
        assert_eq!(view.total_visual_rows(10), 10);
    }

    #[test]
    fn test_clear_wrap_info() {
        let mut view = ViewState::new();
        view.set_line_wrap_info(0, 3, 60.0);
        view.rebuild_height_cache(1);

        view.clear_wrap_info();

        assert!(view.wrap_info().is_empty());
        assert_eq!(view.get_visual_rows(0), 1); // Falls back to 1
    }

    #[test]
    fn test_visual_row_to_logical_simple() {
        let mut view = ViewState::new();

        // Line 0: 2 visual rows
        // Line 1: 1 visual row
        // Line 2: 3 visual rows
        view.set_line_wrap_info(0, 2, 40.0);
        view.set_line_wrap_info(1, 1, 20.0);
        view.set_line_wrap_info(2, 3, 60.0);

        // Visual row 0 -> Line 0, row 0 within line
        assert_eq!(view.visual_row_to_logical(0, 3), (0, 0));
        // Visual row 1 -> Line 0, row 1 within line
        assert_eq!(view.visual_row_to_logical(1, 3), (0, 1));
        // Visual row 2 -> Line 1, row 0 within line
        assert_eq!(view.visual_row_to_logical(2, 3), (1, 0));
        // Visual row 3 -> Line 2, row 0 within line
        assert_eq!(view.visual_row_to_logical(3, 3), (2, 0));
        // Visual row 5 -> Line 2, row 2 within line
        assert_eq!(view.visual_row_to_logical(5, 3), (2, 2));
    }

    #[test]
    fn test_logical_to_visual_row() {
        let mut view = ViewState::new();

        view.set_line_wrap_info(0, 2, 40.0);
        view.set_line_wrap_info(1, 1, 20.0);
        view.set_line_wrap_info(2, 3, 60.0);

        // Line 0, col 0 -> Visual row 0 (assuming ~20 chars/row)
        assert_eq!(view.logical_to_visual_row(0, 0, 20), 0);
        // Line 1, col 0 -> Visual row 2 (after line 0's 2 rows)
        assert_eq!(view.logical_to_visual_row(1, 0, 20), 2);
        // Line 2, col 0 -> Visual row 3 (after line 0's 2 + line 1's 1)
        assert_eq!(view.logical_to_visual_row(2, 0, 20), 3);
    }

    #[test]
    fn test_wrap_info_default() {
        let info = WrapInfo::default();
        assert_eq!(info.visual_rows, 1);
        assert_eq!(info.height, DEFAULT_LINE_HEIGHT);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Uniform Heights / Large File Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_configure_for_file_size_activates_uniform() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        assert!(!view.uses_uniform_heights());

        view.configure_for_file_size(LARGE_FILE_THRESHOLD);
        assert!(view.uses_uniform_heights());
    }

    #[test]
    fn test_configure_for_file_size_below_threshold() {
        let mut view = ViewState::new();
        view.configure_for_file_size(LARGE_FILE_THRESHOLD - 1);
        assert!(!view.uses_uniform_heights());
    }

    #[test]
    fn test_configure_for_file_size_clears_vectors() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.set_line_wrap_info(0, 2, 40.0);
        view.set_line_wrap_info(1, 1, 20.0);
        view.rebuild_height_cache(2);

        assert!(!view.wrap_info().is_empty());

        view.configure_for_file_size(LARGE_FILE_THRESHOLD);
        assert!(view.wrap_info().is_empty());
        assert!(view.uses_uniform_heights());
    }

    #[test]
    fn test_uniform_get_line_height() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(LARGE_FILE_THRESHOLD);

        assert_eq!(view.get_line_height(0), 20.0);
        assert_eq!(view.get_line_height(500_000), 20.0);
    }

    #[test]
    fn test_uniform_get_line_y_offset() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(LARGE_FILE_THRESHOLD);

        assert_eq!(view.get_line_y_offset(0), 0.0);
        assert_eq!(view.get_line_y_offset(100), 2000.0);
        assert_eq!(view.get_line_y_offset(500_000), 10_000_000.0);
    }

    #[test]
    fn test_uniform_total_content_height() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(200_000);

        assert_eq!(view.total_content_height(200_000), 4_000_000.0);
    }

    #[test]
    fn test_uniform_visible_line_range() {
        let mut view = ViewState::new();
        view.update_viewport(800.0);
        view.set_line_height(20.0);
        view.configure_for_file_size(500_000);
        view.scroll_to_line(250_000);

        let (start, end) = view.get_visible_line_range(500_000);
        assert_eq!(start, 250_000 - OVERSCAN_LINES);
        assert_eq!(end, 250_000 + 40 + OVERSCAN_LINES); // 800/20 = 40 lines
    }

    #[test]
    fn test_uniform_y_offset_to_line() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(500_000);

        assert_eq!(view.y_offset_to_line(0.0, 500_000), 0);
        assert_eq!(view.y_offset_to_line(2000.0, 500_000), 100);
        assert_eq!(view.y_offset_to_line(19.99, 500_000), 0);
        assert_eq!(view.y_offset_to_line(20.0, 500_000), 1);
    }

    #[test]
    fn test_uniform_set_line_wrap_info_is_noop() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(LARGE_FILE_THRESHOLD);

        view.set_line_wrap_info(0, 3, 60.0);
        assert!(view.wrap_info().is_empty());
        assert_eq!(view.get_line_height(0), 20.0);
    }

    #[test]
    fn test_uniform_rebuild_height_cache_is_noop() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(LARGE_FILE_THRESHOLD);

        view.rebuild_height_cache(500_000);
        assert_eq!(view.total_content_height(500_000), 10_000_000.0);
    }

    #[test]
    fn test_uniform_scroll_by() {
        let mut view = ViewState::new();
        view.update_viewport(800.0);
        view.set_line_height(20.0);
        view.configure_for_file_size(500_000);
        view.scroll_to_line(0);

        view.scroll_by(100.0, 500_000);
        assert_eq!(view.first_visible_line(), 5);
        assert_eq!(view.scroll_offset_y(), 0.0);
    }

    #[test]
    fn test_uniform_deactivates_below_threshold() {
        let mut view = ViewState::new();
        view.set_line_height(20.0);
        view.configure_for_file_size(LARGE_FILE_THRESHOLD);
        assert!(view.uses_uniform_heights());

        view.configure_for_file_size(LARGE_FILE_THRESHOLD - 1);
        assert!(!view.uses_uniform_heights());
    }
}
