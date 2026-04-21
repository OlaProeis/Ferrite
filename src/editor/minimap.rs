//! Minimap Navigation Panel for Ferrite
//!
//! This module provides two minimap variants for the editor:
//!
//! 1. **Pixel Minimap** (`Minimap`) - VS Code-style zoomed-out document preview
//! 2. **Semantic Minimap** (`SemanticMinimap`) - Clickable header labels for navigation
//!
//! The semantic minimap shows document headings (H1-H6) in a scrollable list,
//! allowing quick navigation through the document structure.
//!
//! # Usage
//!
//! ```ignore
//! use crate::editor::{SemanticMinimap, extract_outline};
//!
//! let outline = extract_outline(content);
//! let minimap = SemanticMinimap::new(&outline.items)
//!     .width(120.0)
//!     .scroll_offset(current_scroll)
//!     .content_height(total_height)
//!     .line_height(line_height);
//!
//! let output = minimap.show(ui);
//! if let Some(char_offset) = output.scroll_to_char {
//!     // Navigate to the requested position
//! }
//! ```

// MinimapSettings is available for future settings UI integration
#![allow(dead_code)]

use crate::editor::{ContentType, OutlineItem};
use crate::theme::ThemeColors;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Default width of the minimap in pixels
const DEFAULT_MINIMAP_WIDTH: f32 = 80.0;

/// Minimum width for the minimap
const MIN_MINIMAP_WIDTH: f32 = 40.0;

/// Maximum width for the minimap
const MAX_MINIMAP_WIDTH: f32 = 150.0;

/// Scale factor for text in the minimap (pixels per character)
/// Higher value = wider lines filling more horizontal space
const MINIMAP_CHAR_SCALE: f32 = 0.8;

/// Line height in the minimap (pixels)
const MINIMAP_LINE_HEIGHT: f32 = 2.0;

/// Horizontal padding inside the minimap
const MINIMAP_PADDING: f32 = 4.0;

/// Maximum number of lines to render for performance
const MAX_LINES_TO_RENDER: usize = 10000;

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Output
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the minimap widget
#[derive(Debug, Clone, Default)]
pub struct MinimapOutput {
    /// If set, the editor should scroll to this offset
    pub scroll_to_offset: Option<f32>,
    /// Whether the minimap was clicked
    pub clicked: bool,
    /// Whether the minimap is being dragged
    pub dragging: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Widget
// ─────────────────────────────────────────────────────────────────────────────

/// A VS Code-style minimap widget showing a zoomed-out document preview.
///
/// The minimap displays the entire document in a compressed format, allowing
/// users to quickly navigate large documents by clicking or dragging on the
/// minimap surface.
pub struct Minimap<'a> {
    /// The document content to display
    content: &'a str,
    /// Width of the minimap in pixels
    width: f32,
    /// Current vertical scroll offset in the editor
    scroll_offset: f32,
    /// Height of the visible viewport in the editor
    viewport_height: f32,
    /// Total height of the document content in the editor
    content_height: f32,
    /// Line height in the editor (for scroll calculations)
    line_height: f32,
    /// Theme colors for styling
    theme_colors: Option<ThemeColors>,
    /// Search matches to highlight (start, end byte positions)
    search_highlights: Option<&'a [(usize, usize)]>,
    /// Current search match index (for distinct highlighting)
    current_match: usize,
}

impl<'a> Minimap<'a> {
    /// Create a new minimap widget for the given content.
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            width: DEFAULT_MINIMAP_WIDTH,
            scroll_offset: 0.0,
            viewport_height: 100.0,
            content_height: 100.0,
            line_height: 16.0,
            theme_colors: None,
            search_highlights: None,
            current_match: 0,
        }
    }

    /// Set the width of the minimap.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_MINIMAP_WIDTH, MAX_MINIMAP_WIDTH);
        self
    }

    /// Set the current scroll offset.
    #[must_use]
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the viewport height.
    #[must_use]
    pub fn viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Set the total content height.
    #[must_use]
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = height;
        self
    }

    /// Set the line height for scroll calculations.
    #[must_use]
    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = height;
        self
    }

    /// Set the theme colors for styling.
    #[must_use]
    pub fn theme_colors(mut self, colors: ThemeColors) -> Self {
        self.theme_colors = Some(colors);
        self
    }

    /// Set search highlights to display.
    #[must_use]
    pub fn search_highlights(mut self, matches: &'a [(usize, usize)]) -> Self {
        self.search_highlights = Some(matches);
        self
    }

    /// Set the current match index for distinct highlighting.
    #[must_use]
    pub fn current_match(mut self, index: usize) -> Self {
        self.current_match = index;
        self
    }

    /// Show the minimap widget and return the output.
    pub fn show(self, ui: &mut Ui) -> MinimapOutput {
        let mut output = MinimapOutput::default();

        // Determine colors based on theme
        let is_dark = self
            .theme_colors
            .as_ref()
            .map(|c| c.is_dark())
            .unwrap_or(false);
        let colors = MinimapColors::new(is_dark);

        // Calculate minimap dimensions
        let line_count = self.content.lines().count().max(1);
        let minimap_content_height = (line_count as f32 * MINIMAP_LINE_HEIGHT).max(1.0);
        let available_height = ui.available_height();

        // Calculate scale factor to fit content in available height
        let scale = if minimap_content_height > available_height {
            available_height / minimap_content_height
        } else {
            1.0
        };

        let minimap_height = (minimap_content_height * scale).min(available_height);

        // Allocate space for the minimap
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.width, available_height),
            Sense::click_and_drag(),
        );

        // Draw background
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, colors.background);

        // Draw left border
        painter.line_segment(
            [rect.left_top(), rect.left_bottom()],
            Stroke::new(1.0, colors.border),
        );

        // Build line information for rendering
        let lines: Vec<&str> = self.content.lines().take(MAX_LINES_TO_RENDER).collect();
        let content_rect = Rect::from_min_size(
            rect.min + Vec2::new(MINIMAP_PADDING, 0.0),
            Vec2::new(self.width - MINIMAP_PADDING * 2.0, minimap_height),
        );

        // Draw document lines
        self.render_lines(&painter, &lines, content_rect, scale, &colors);

        // Draw search highlights
        if let Some(highlights) = self.search_highlights {
            self.render_search_highlights(&painter, highlights, content_rect, scale, &colors);
        }

        // Draw viewport indicator
        let viewport_rect = self.calculate_viewport_rect(content_rect, scale, line_count);
        if let Some(vp_rect) = viewport_rect {
            painter.rect_filled(vp_rect, 2.0, colors.viewport_fill);
            painter.rect_stroke(vp_rect, 2.0, Stroke::new(1.0, colors.viewport_border));
        }

        // Handle interaction
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                output.scroll_to_offset =
                    Some(self.calculate_scroll_offset(pos, content_rect, scale, line_count));
                output.clicked = response.clicked();
                output.dragging = response.dragged();
            }
        }

        // Show hover cursor
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        output
    }

    /// Render document lines in the minimap.
    fn render_lines(
        &self,
        painter: &egui::Painter,
        lines: &[&str],
        content_rect: Rect,
        scale: f32,
        colors: &MinimapColors,
    ) {
        let usable_width = content_rect.width();
        let char_width = MINIMAP_CHAR_SCALE * scale;
        let line_height = MINIMAP_LINE_HEIGHT * scale;

        for (line_idx, line) in lines.iter().enumerate() {
            let y = content_rect.min.y + line_idx as f32 * line_height;

            if y > content_rect.max.y {
                break;
            }

            // Determine line color based on content
            let color = self.get_line_color(line, colors);

            // Calculate line width based on character count
            let char_count = line.chars().count();
            let line_width = (char_count as f32 * char_width).min(usable_width);

            if line_width > 0.5 {
                let line_rect = Rect::from_min_size(
                    Pos2::new(content_rect.min.x, y),
                    Vec2::new(line_width, line_height.max(1.0)),
                );
                painter.rect_filled(line_rect, 0.0, color);
            }
        }
    }

    /// Get the color for a line based on its content (simplified syntax highlighting).
    fn get_line_color(&self, line: &str, colors: &MinimapColors) -> Color32 {
        let trimmed = line.trim();

        // Heading detection
        if trimmed.starts_with('#') {
            return colors.heading;
        }

        // Code block markers
        if trimmed.starts_with("```") {
            return colors.code_marker;
        }

        // List items (check before comments since `* item` is a list, not a comment)
        if (trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+'))
            && trimmed
                .chars()
                .nth(1)
                .map(|c| c.is_whitespace())
                .unwrap_or(false)
        {
            return colors.list;
        }

        // Numbered list items
        if trimmed.len() > 1
            && trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            && trimmed.contains('.')
        {
            return colors.list;
        }

        // Comments (common patterns in code)
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with(" *") {
            return colors.comment;
        }

        // Blockquotes
        if trimmed.starts_with('>') {
            return colors.blockquote;
        }

        // Links or images
        if trimmed.contains("](") || trimmed.starts_with('[') {
            return colors.link;
        }

        // Empty lines get a lighter color
        if trimmed.is_empty() {
            return colors.empty_line;
        }

        // Default text color
        colors.text
    }

    /// Render search highlight indicators in the minimap.
    fn render_search_highlights(
        &self,
        painter: &egui::Painter,
        highlights: &[(usize, usize)],
        content_rect: Rect,
        scale: f32,
        colors: &MinimapColors,
    ) {
        let line_height = MINIMAP_LINE_HEIGHT * scale;

        // Build a map of byte offset to line number
        let mut line_offsets: Vec<usize> = vec![0];
        let mut current_offset = 0;
        for line in self.content.lines() {
            current_offset += line.len() + 1; // +1 for newline
            line_offsets.push(current_offset);
        }

        for (idx, &(start, _end)) in highlights.iter().enumerate() {
            // Find which line this match is on
            let line_idx = line_offsets
                .iter()
                .position(|&offset| offset > start)
                .map(|pos| pos.saturating_sub(1))
                .unwrap_or(0);

            let y = content_rect.min.y + line_idx as f32 * line_height;

            if y > content_rect.max.y {
                continue;
            }

            // Highlight indicator on the right edge
            let is_current = idx == self.current_match;
            let color = if is_current {
                colors.current_match
            } else {
                colors.other_match
            };

            let indicator_width = if is_current { 4.0 } else { 3.0 };
            let indicator_rect = Rect::from_min_size(
                Pos2::new(content_rect.max.x - indicator_width, y - line_height * 0.5),
                Vec2::new(indicator_width, line_height * 2.0),
            );
            painter.rect_filled(indicator_rect, 1.0, color);
        }
    }

    /// Calculate the viewport indicator rectangle.
    fn calculate_viewport_rect(
        &self,
        content_rect: Rect,
        scale: f32,
        line_count: usize,
    ) -> Option<Rect> {
        if self.content_height <= 0.0 {
            return None;
        }

        let minimap_content_height = line_count as f32 * MINIMAP_LINE_HEIGHT * scale;
        let scroll_ratio = self.scroll_offset / self.content_height.max(1.0);
        let viewport_ratio = self.viewport_height / self.content_height.max(1.0);

        let viewport_y = content_rect.min.y + scroll_ratio * minimap_content_height;
        let viewport_height = (viewport_ratio * minimap_content_height).max(10.0);

        Some(Rect::from_min_size(
            Pos2::new(content_rect.min.x - 2.0, viewport_y),
            Vec2::new(content_rect.width() + 4.0, viewport_height),
        ))
    }

    /// Calculate scroll offset from a click/drag position.
    fn calculate_scroll_offset(
        &self,
        pos: Pos2,
        content_rect: Rect,
        scale: f32,
        line_count: usize,
    ) -> f32 {
        let minimap_content_height = line_count as f32 * MINIMAP_LINE_HEIGHT * scale;
        let click_ratio = (pos.y - content_rect.min.y) / minimap_content_height.max(1.0);

        // Center the viewport on the clicked position
        let target_ratio = click_ratio - (self.viewport_height / self.content_height / 2.0);
        let target_offset = target_ratio * self.content_height;

        // Clamp to valid range
        target_offset.clamp(0.0, (self.content_height - self.viewport_height).max(0.0))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Semantic Minimap
// ─────────────────────────────────────────────────────────────────────────────

/// Default width of the semantic minimap in pixels
const DEFAULT_SEMANTIC_MINIMAP_WIDTH: f32 = 120.0;

/// Minimum width for the semantic minimap
const MIN_SEMANTIC_MINIMAP_WIDTH: f32 = 80.0;

/// Maximum width for the semantic minimap
const MAX_SEMANTIC_MINIMAP_WIDTH: f32 = 200.0;

/// Maximum characters to display before truncation
const MAX_HEADER_CHARS: usize = 20;

/// Base font size for H1 headings
const H1_FONT_SIZE: f32 = 8.0;

/// Font size decrement per heading level
const FONT_SIZE_DECREMENT: f32 = 0.3;

/// Minimum font size for deep headings
const MIN_FONT_SIZE: f32 = 6.5;

/// Indentation per heading level (pixels)
const INDENT_PER_LEVEL: f32 = 4.0;

/// Item height in the semantic minimap
const SEMANTIC_ITEM_HEIGHT: f32 = 11.0;

/// Padding at the top/bottom of the list
const SEMANTIC_PADDING: f32 = 4.0;

/// Height of density bars between items
const DENSITY_BAR_HEIGHT: f32 = 3.0;

/// Minimum density bar width (pixels)
const MIN_DENSITY_WIDTH: f32 = 2.0;

/// Maximum density bar width (as fraction of available width)
const MAX_DENSITY_WIDTH_FRACTION: f32 = 0.8;

/// Minimum density bar opacity
const MIN_DENSITY_OPACITY: f32 = 0.15;

/// Maximum density bar opacity
const MAX_DENSITY_OPACITY: f32 = 0.6;

/// Output from the semantic minimap widget
#[derive(Debug, Clone, Default)]
pub struct SemanticMinimapOutput {
    /// If set, the editor should scroll to this character offset
    pub scroll_to_char: Option<usize>,
    /// If set, the editor should scroll to this line number
    pub scroll_to_line: Option<usize>,
    /// Heading title text for the clicked item (for text-based navigation)
    pub scroll_to_title: Option<String>,
    /// Heading level (1-6) for the clicked item
    pub scroll_to_level: Option<u8>,
    /// Whether a header was clicked
    pub clicked: bool,
}

/// A semantic minimap widget showing document headings as clickable labels.
///
/// The semantic minimap provides a structural overview of the document,
/// displaying H1-H6 headings with visual hierarchy (font size and indentation).
/// Clicking a heading navigates to that position in the document.
///
/// Density bars between items show relative content density - darker/wider bars
/// indicate more content between sections.
pub struct SemanticMinimap<'a> {
    /// The heading items to display
    headers: &'a [OutlineItem],
    /// Width of the minimap in pixels
    width: f32,
    /// Current vertical scroll offset in the editor
    scroll_offset: f32,
    /// Total height of the document content in the editor
    content_height: f32,
    /// Line height in the editor (for position calculations)
    line_height: f32,
    /// Theme colors for styling
    theme_colors: Option<ThemeColors>,
    /// Current line number for highlighting current section
    current_line: Option<usize>,
    /// Total number of lines in the document (for density calculation)
    total_lines: usize,
    /// Whether to show density bars between items
    show_density: bool,
}

impl<'a> SemanticMinimap<'a> {
    /// Create a new semantic minimap widget for the given headers.
    pub fn new(headers: &'a [OutlineItem]) -> Self {
        Self {
            headers,
            width: DEFAULT_SEMANTIC_MINIMAP_WIDTH,
            scroll_offset: 0.0,
            content_height: 100.0,
            line_height: 16.0,
            theme_colors: None,
            current_line: None,
            total_lines: 0,
            show_density: true,
        }
    }

    /// Set the width of the minimap.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_SEMANTIC_MINIMAP_WIDTH, MAX_SEMANTIC_MINIMAP_WIDTH);
        self
    }

    /// Set the current scroll offset.
    #[must_use]
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the total content height.
    #[must_use]
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = height;
        self
    }

    /// Set the line height for scroll calculations.
    #[must_use]
    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = height;
        self
    }

    /// Set the theme colors for styling.
    #[must_use]
    pub fn theme_colors(mut self, colors: ThemeColors) -> Self {
        self.theme_colors = Some(colors);
        self
    }

    /// Set the current line number for section highlighting.
    #[must_use]
    pub fn current_line(mut self, line: Option<usize>) -> Self {
        self.current_line = line;
        self
    }

    /// Set the total number of lines in the document.
    ///
    /// Used for calculating the density of the last section.
    #[must_use]
    pub fn total_lines(mut self, lines: usize) -> Self {
        self.total_lines = lines;
        self
    }

    /// Set whether to show density bars between items.
    #[must_use]
    pub fn show_density(mut self, show: bool) -> Self {
        self.show_density = show;
        self
    }

    /// Show the semantic minimap widget and return the output.
    pub fn show(self, ui: &mut Ui) -> SemanticMinimapOutput {
        let mut output = SemanticMinimapOutput::default();

        // Determine colors based on theme
        let is_dark = self
            .theme_colors
            .as_ref()
            .map(|c| c.is_dark())
            .unwrap_or(false);
        let colors = SemanticMinimapColors::new(is_dark);

        // Get available space
        let available_height = ui.available_height();
        let rect = ui.available_rect_before_wrap();
        let minimap_rect = Rect::from_min_size(rect.min, Vec2::new(self.width, available_height));

        // Allocate the space (so parent knows we used it)
        ui.allocate_rect(minimap_rect, Sense::hover());

        // Draw background
        let painter = ui.painter_at(minimap_rect);
        painter.rect_filled(minimap_rect, 0.0, colors.background);

        // Draw left border
        painter.line_segment(
            [minimap_rect.left_top(), minimap_rect.left_bottom()],
            Stroke::new(1.0, colors.border),
        );

        // Find current section based on scroll position or cursor
        let current_section = self.find_current_section();

        // Calculate content area (inside padding)
        let content_rect = Rect::from_min_size(
            minimap_rect.min + Vec2::new(SEMANTIC_PADDING, SEMANTIC_PADDING),
            Vec2::new(
                self.width - SEMANTIC_PADDING * 2.0,
                available_height - SEMANTIC_PADDING * 2.0,
            ),
        );

        // Show empty state if no items
        if self.headers.is_empty() {
            let empty_text = "No content";
            painter.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                empty_text,
                FontId::proportional(10.0),
                colors.muted_text,
            );
            return output;
        }

        // Create a child UI for the content area so ScrollArea renders inside it
        let mut content_ui = ui.child_ui(
            content_rect,
            egui::Layout::top_down(egui::Align::LEFT),
            None,
        );

        // Calculate densities if enabled
        let densities = if self.show_density && !self.headers.is_empty() {
            self.calculate_densities()
        } else {
            Vec::new()
        };

        // Create a scrollable area for headers
        let scroll_id = content_ui.id().with("semantic_minimap_scroll");
        egui::ScrollArea::vertical()
            .id_source(scroll_id)
            .auto_shrink([false, false])
            .max_height(content_rect.height())
            .show(&mut content_ui, |ui| {
                ui.set_min_width(content_rect.width());

                for (index, item) in self.headers.iter().enumerate() {
                    let is_current = current_section == Some(index);

                    // Draw density bar BEFORE the item (shows density of previous section)
                    if self.show_density && index > 0 {
                        if let Some(&(line_count, normalized)) = densities.get(index - 1) {
                            self.render_density_bar(
                                ui,
                                content_rect.width(),
                                line_count,
                                normalized,
                                &colors,
                            );
                        }
                    }

                    // Determine rendering based on content type
                    let (font_size, indent, label_text, label_color) = match item.content_type {
                        ContentType::Heading(level) => {
                            let fs = (H1_FONT_SIZE
                                - (level.saturating_sub(1) as f32) * FONT_SIZE_DECREMENT)
                                .max(MIN_FONT_SIZE);
                            let ind = (level.saturating_sub(1) as f32) * INDENT_PER_LEVEL;
                            let lbl = format!("H{}", level);
                            let col = if is_current {
                                colors.current_level_indicator
                            } else {
                                heading_level_color_semantic(level, is_dark)
                            };
                            (fs, ind, lbl, col)
                        }
                        ContentType::CodeBlock => (
                            MIN_FONT_SIZE + 0.5,
                            0.0,
                            "</>".to_string(),
                            colors.code_block,
                        ),
                        ContentType::MermaidDiagram => (
                            MIN_FONT_SIZE + 0.5,
                            0.0,
                            "◇".to_string(),
                            colors.mermaid_diagram,
                        ),
                        ContentType::Table => {
                            (MIN_FONT_SIZE + 0.5, 0.0, "⊞".to_string(), colors.table)
                        }
                        ContentType::Image => {
                            (MIN_FONT_SIZE + 0.5, 0.0, "▣".to_string(), colors.image)
                        }
                        ContentType::Blockquote => {
                            (MIN_FONT_SIZE + 0.5, 0.0, "❝".to_string(), colors.blockquote)
                        }
                    };

                    // Truncate text if too long
                    let display_text = truncate_header(&item.title, MAX_HEADER_CHARS);

                    // Create the item - allocate first, then paint
                    let (item_rect, response) = ui.allocate_exact_size(
                        Vec2::new(content_rect.width(), SEMANTIC_ITEM_HEIGHT),
                        Sense::click(),
                    );

                    // Now get painter and draw (after mutable borrow of ui is done)
                    let item_painter = ui.painter_at(item_rect);

                    // Draw background for current/hovered item
                    if is_current {
                        item_painter.rect_filled(item_rect, 2.0, colors.current_section_bg);
                    } else if response.hovered() {
                        item_painter.rect_filled(item_rect, 2.0, colors.hover_bg);
                    }

                    // Get text color
                    let text_color = if is_current {
                        colors.current_text
                    } else {
                        colors.text
                    };

                    // Draw type indicator
                    let label_pos = Pos2::new(item_rect.min.x + indent + 2.0, item_rect.center().y);
                    item_painter.text(
                        label_pos,
                        egui::Align2::LEFT_CENTER,
                        &label_text,
                        FontId::proportional(6.0),
                        label_color,
                    );

                    // Calculate text position - content blocks use slightly less indent for their icons
                    let text_x_offset = if item.content_type.is_heading() {
                        22.0
                    } else {
                        16.0
                    };
                    let text_pos = Pos2::new(
                        item_rect.min.x + indent + text_x_offset,
                        item_rect.center().y,
                    );

                    // Use bold for H1 headings, regular for others
                    let font = if matches!(item.content_type, ContentType::Heading(1)) {
                        FontId::new(font_size, egui::FontFamily::Name("Inter-Bold".into()))
                    } else {
                        FontId::proportional(font_size)
                    };

                    item_painter.text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        &display_text,
                        font,
                        text_color,
                    );

                    // Handle click
                    if response.clicked() {
                        output.scroll_to_char = Some(item.char_offset);
                        output.scroll_to_line = Some(item.line);
                        output.scroll_to_title = Some(item.title.clone());
                        output.scroll_to_level = item.content_type.heading_level();
                        output.clicked = true;
                    }

                    // Show tooltip on hover
                    if response.hovered() {
                        let tooltip = match item.content_type {
                            ContentType::Heading(_) => {
                                format!("Line {}: {}", item.line, item.title)
                            }
                            ContentType::CodeBlock => format!("Code block at line {}", item.line),
                            ContentType::MermaidDiagram => {
                                format!("Mermaid diagram at line {}", item.line)
                            }
                            ContentType::Table => format!("Table at line {}", item.line),
                            ContentType::Image => {
                                format!("Image: {} (line {})", item.title, item.line)
                            }
                            ContentType::Blockquote => format!("Blockquote at line {}", item.line),
                        };
                        response.on_hover_text(tooltip);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }
            });

        output
    }

    /// Find the index of the current section based on scroll position.
    fn find_current_section(&self) -> Option<usize> {
        if self.headers.is_empty() {
            return None;
        }

        // If we have a current line, use that
        if let Some(current_line) = self.current_line {
            let mut result = None;
            for (i, header) in self.headers.iter().enumerate() {
                if header.line <= current_line {
                    result = Some(i);
                } else {
                    break;
                }
            }
            return result;
        }

        // Otherwise, calculate from scroll position
        let current_line = (self.scroll_offset / self.line_height.max(1.0)) as usize + 1;

        let mut result = None;
        for (i, header) in self.headers.iter().enumerate() {
            if header.line <= current_line {
                result = Some(i);
            } else {
                break;
            }
        }
        result
    }

    /// Calculate normalized density values for each section.
    ///
    /// Returns a vector of (line_count, normalized_value) tuples where:
    /// - line_count is the number of lines until the next section
    /// - normalized_value is in range [0.0, 1.0] based on min/max density
    fn calculate_densities(&self) -> Vec<(usize, f32)> {
        if self.headers.is_empty() {
            return Vec::new();
        }

        // Calculate line counts for each section
        let mut line_counts: Vec<usize> = Vec::with_capacity(self.headers.len());

        for i in 0..self.headers.len() {
            let current_line = self.headers[i].line;
            let next_line = if i + 1 < self.headers.len() {
                self.headers[i + 1].line
            } else {
                // Last item: use total_lines if available, otherwise estimate
                if self.total_lines > current_line {
                    self.total_lines
                } else {
                    current_line + 10 // Fallback: assume at least 10 lines after last heading
                }
            };

            let line_count = next_line.saturating_sub(current_line);
            line_counts.push(line_count);
        }

        // Find min and max for normalization
        let min_count = *line_counts.iter().min().unwrap_or(&1);
        let max_count = *line_counts.iter().max().unwrap_or(&1);
        let range = max_count - min_count;

        // Normalize to [0.0, 1.0]
        line_counts
            .into_iter()
            .map(|count| {
                let normalized = if range > 0 {
                    (count - min_count) as f32 / range as f32
                } else {
                    0.5 // All sections have same density
                };
                (count, normalized)
            })
            .collect()
    }

    /// Render a density bar indicating content volume between sections.
    ///
    /// The bar width and opacity scale with the normalized density value:
    /// - Higher density = wider bar, more opacity
    /// - Lower density = narrower bar, less opacity
    fn render_density_bar(
        &self,
        ui: &mut Ui,
        available_width: f32,
        _line_count: usize,
        normalized: f32,
        colors: &SemanticMinimapColors,
    ) {
        // Calculate bar width based on normalized density
        let max_width = available_width * MAX_DENSITY_WIDTH_FRACTION;
        let bar_width = MIN_DENSITY_WIDTH + (max_width - MIN_DENSITY_WIDTH) * normalized;

        // Calculate opacity based on normalized density
        let opacity =
            MIN_DENSITY_OPACITY + (MAX_DENSITY_OPACITY - MIN_DENSITY_OPACITY) * normalized;

        // Apply opacity to the density bar color
        let bar_color = Color32::from_rgba_unmultiplied(
            colors.density_bar.r(),
            colors.density_bar.g(),
            colors.density_bar.b(),
            (opacity * 255.0) as u8,
        );

        // Allocate space for the density bar
        let (rect, _response) = ui.allocate_exact_size(
            Vec2::new(available_width, DENSITY_BAR_HEIGHT),
            Sense::hover(),
        );

        // Draw the bar centered horizontally
        let bar_x = rect.min.x + (available_width - bar_width) / 2.0;
        let bar_rect = Rect::from_min_size(
            Pos2::new(bar_x, rect.min.y + 0.5),
            Vec2::new(bar_width, DENSITY_BAR_HEIGHT - 1.0),
        );

        // Draw with rounded corners for a softer look
        let painter = ui.painter_at(rect);
        painter.rect_filled(bar_rect, 1.5, bar_color);
    }
}

/// Truncate header text with ellipsis if too long.
fn truncate_header(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Get a color for the heading level indicator in semantic minimap.
fn heading_level_color_semantic(level: u8, is_dark: bool) -> Color32 {
    if is_dark {
        match level {
            1 => Color32::from_rgb(130, 180, 255), // Blue
            2 => Color32::from_rgb(150, 220, 150), // Green
            3 => Color32::from_rgb(220, 180, 120), // Orange
            4 => Color32::from_rgb(200, 150, 200), // Purple
            5 => Color32::from_rgb(180, 180, 180), // Gray
            _ => Color32::from_rgb(150, 150, 150), // Light gray
        }
    } else {
        match level {
            1 => Color32::from_rgb(40, 100, 180),  // Blue
            2 => Color32::from_rgb(50, 140, 50),   // Green
            3 => Color32::from_rgb(180, 120, 40),  // Orange
            4 => Color32::from_rgb(140, 80, 140),  // Purple
            5 => Color32::from_rgb(100, 100, 100), // Gray
            _ => Color32::from_rgb(120, 120, 120), // Dark gray
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Semantic Minimap Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors used for rendering the semantic minimap.
struct SemanticMinimapColors {
    background: Color32,
    border: Color32,
    text: Color32,
    muted_text: Color32,
    current_text: Color32,
    current_section_bg: Color32,
    current_level_indicator: Color32,
    hover_bg: Color32,
    // Content type colors
    code_block: Color32,
    mermaid_diagram: Color32,
    table: Color32,
    image: Color32,
    blockquote: Color32,
    // Density bar color (base color, opacity will be adjusted)
    density_bar: Color32,
}

impl SemanticMinimapColors {
    fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                background: Color32::from_rgb(25, 25, 25),
                border: Color32::from_rgb(50, 50, 50),
                text: Color32::from_rgb(180, 180, 180),
                muted_text: Color32::from_rgb(100, 100, 100),
                current_text: Color32::WHITE,
                current_section_bg: Color32::from_rgb(50, 70, 100),
                current_level_indicator: Color32::from_rgb(100, 180, 255),
                hover_bg: Color32::from_rgb(40, 40, 45),
                // Content type colors (dark theme)
                code_block: Color32::from_rgb(180, 140, 220), // Purple
                mermaid_diagram: Color32::from_rgb(100, 200, 180), // Teal
                table: Color32::from_rgb(220, 180, 100),      // Gold
                image: Color32::from_rgb(200, 130, 160),      // Pink
                blockquote: Color32::from_rgb(140, 160, 200), // Slate blue
                // Density bar (subtle blue-gray)
                density_bar: Color32::from_rgb(100, 140, 180),
            }
        } else {
            Self {
                background: Color32::from_rgb(248, 248, 248),
                border: Color32::from_rgb(210, 210, 210),
                text: Color32::from_rgb(60, 60, 60),
                muted_text: Color32::from_rgb(140, 140, 140),
                current_text: Color32::from_rgb(20, 20, 20),
                current_section_bg: Color32::from_rgb(220, 235, 250),
                current_level_indicator: Color32::from_rgb(0, 100, 200),
                hover_bg: Color32::from_rgb(235, 235, 240),
                // Content type colors (light theme)
                code_block: Color32::from_rgb(130, 80, 170), // Purple
                mermaid_diagram: Color32::from_rgb(50, 140, 120), // Teal
                table: Color32::from_rgb(180, 130, 40),      // Gold
                image: Color32::from_rgb(160, 80, 120),      // Pink
                blockquote: Color32::from_rgb(80, 100, 160), // Slate blue
                // Density bar (subtle blue-gray)
                density_bar: Color32::from_rgb(80, 120, 160),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors used for rendering the minimap.
struct MinimapColors {
    background: Color32,
    border: Color32,
    text: Color32,
    heading: Color32,
    code_marker: Color32,
    comment: Color32,
    list: Color32,
    blockquote: Color32,
    link: Color32,
    empty_line: Color32,
    viewport_fill: Color32,
    viewport_border: Color32,
    current_match: Color32,
    other_match: Color32,
}

impl MinimapColors {
    fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                background: Color32::from_rgb(25, 25, 25),
                border: Color32::from_rgb(50, 50, 50),
                text: Color32::from_rgba_unmultiplied(180, 180, 180, 200),
                heading: Color32::from_rgba_unmultiplied(100, 180, 255, 220),
                code_marker: Color32::from_rgba_unmultiplied(150, 120, 200, 200),
                comment: Color32::from_rgba_unmultiplied(100, 110, 120, 180),
                list: Color32::from_rgba_unmultiplied(120, 200, 120, 200),
                blockquote: Color32::from_rgba_unmultiplied(140, 140, 160, 180),
                link: Color32::from_rgba_unmultiplied(100, 180, 255, 180),
                empty_line: Color32::from_rgba_unmultiplied(60, 60, 60, 100),
                viewport_fill: Color32::from_rgba_unmultiplied(100, 100, 120, 40),
                viewport_border: Color32::from_rgba_unmultiplied(100, 180, 255, 150),
                current_match: Color32::from_rgba_unmultiplied(255, 200, 0, 255),
                other_match: Color32::from_rgba_unmultiplied(255, 200, 0, 120),
            }
        } else {
            Self {
                background: Color32::from_rgb(245, 245, 245),
                border: Color32::from_rgb(200, 200, 200),
                text: Color32::from_rgba_unmultiplied(80, 80, 80, 200),
                heading: Color32::from_rgba_unmultiplied(0, 90, 165, 220),
                code_marker: Color32::from_rgba_unmultiplied(120, 80, 160, 200),
                comment: Color32::from_rgba_unmultiplied(120, 120, 120, 180),
                list: Color32::from_rgba_unmultiplied(60, 140, 60, 200),
                blockquote: Color32::from_rgba_unmultiplied(100, 100, 120, 180),
                link: Color32::from_rgba_unmultiplied(0, 100, 200, 180),
                empty_line: Color32::from_rgba_unmultiplied(200, 200, 200, 100),
                viewport_fill: Color32::from_rgba_unmultiplied(100, 150, 200, 50),
                viewport_border: Color32::from_rgba_unmultiplied(0, 120, 212, 180),
                current_match: Color32::from_rgba_unmultiplied(255, 180, 0, 255),
                other_match: Color32::from_rgba_unmultiplied(255, 200, 100, 150),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Settings
// ─────────────────────────────────────────────────────────────────────────────

/// Settings for the minimap feature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapSettings {
    /// Whether the minimap is enabled
    pub enabled: bool,
    /// Width of the minimap in pixels
    pub width: f32,
}

impl Default for MinimapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            width: DEFAULT_MINIMAP_WIDTH,
        }
    }
}

impl MinimapSettings {
    /// Create new minimap settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create disabled minimap settings.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set whether the minimap is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the minimap width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_MINIMAP_WIDTH, MAX_MINIMAP_WIDTH);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimap_new() {
        let minimap = Minimap::new("Hello, World!");
        assert_eq!(minimap.content, "Hello, World!");
        assert_eq!(minimap.width, DEFAULT_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_width_clamping() {
        let minimap = Minimap::new("test").width(20.0);
        assert_eq!(minimap.width, MIN_MINIMAP_WIDTH);

        let minimap = Minimap::new("test").width(200.0);
        assert_eq!(minimap.width, MAX_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_settings_default() {
        let settings = MinimapSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.width, DEFAULT_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_settings_disabled() {
        let settings = MinimapSettings::disabled();
        assert!(!settings.enabled);
    }

    #[test]
    fn test_minimap_settings_width_clamping() {
        let settings = MinimapSettings::new().width(20.0);
        assert_eq!(settings.width, MIN_MINIMAP_WIDTH);

        let settings = MinimapSettings::new().width(200.0);
        assert_eq!(settings.width, MAX_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_colors_dark() {
        let colors = MinimapColors::new(true);
        // Dark theme should have dark background
        assert!(colors.background.r() < 50);
    }

    #[test]
    fn test_minimap_colors_light() {
        let colors = MinimapColors::new(false);
        // Light theme should have light background
        assert!(colors.background.r() > 200);
    }

    #[test]
    fn test_minimap_output_default() {
        let output = MinimapOutput::default();
        assert!(output.scroll_to_offset.is_none());
        assert!(!output.clicked);
        assert!(!output.dragging);
    }

    #[test]
    fn test_line_color_detection() {
        let minimap = Minimap::new("");
        let colors = MinimapColors::new(false);

        // Test heading detection
        assert_eq!(minimap.get_line_color("# Heading", &colors), colors.heading);
        assert_eq!(
            minimap.get_line_color("## Heading", &colors),
            colors.heading
        );

        // Test code marker detection
        assert_eq!(
            minimap.get_line_color("```rust", &colors),
            colors.code_marker
        );

        // Test comment detection
        assert_eq!(
            minimap.get_line_color("// comment", &colors),
            colors.comment
        );

        // Test list detection
        assert_eq!(minimap.get_line_color("- item", &colors), colors.list);
        assert_eq!(minimap.get_line_color("* item", &colors), colors.list);
        assert_eq!(minimap.get_line_color("1. item", &colors), colors.list);

        // Test blockquote detection
        assert_eq!(
            minimap.get_line_color("> quote", &colors),
            colors.blockquote
        );

        // Test link detection
        assert_eq!(minimap.get_line_color("[link](url)", &colors), colors.link);

        // Test empty line
        assert_eq!(minimap.get_line_color("", &colors), colors.empty_line);
        assert_eq!(minimap.get_line_color("   ", &colors), colors.empty_line);

        // Test regular text
        assert_eq!(minimap.get_line_color("Hello world", &colors), colors.text);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Semantic Minimap Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_semantic_minimap_output_default() {
        let output = SemanticMinimapOutput::default();
        assert!(output.scroll_to_char.is_none());
        assert!(output.scroll_to_line.is_none());
        assert!(!output.clicked);
    }

    #[test]
    fn test_truncate_header_short() {
        let text = "Short heading";
        assert_eq!(truncate_header(text, 30), "Short heading");
    }

    #[test]
    fn test_truncate_header_exact() {
        let text = "Exactly thirty character text!"; // 30 chars
        assert_eq!(truncate_header(text, 30), "Exactly thirty character text!");
    }

    #[test]
    fn test_truncate_header_long() {
        let text = "This is a very long heading that should be truncated";
        let result = truncate_header(text, 30);
        assert!(result.ends_with('…'));
        assert!(result.chars().count() <= 30);
    }

    #[test]
    fn test_heading_level_color_semantic_dark() {
        // Verify each level returns a different color
        let colors: Vec<_> = (1..=6)
            .map(|l| heading_level_color_semantic(l, true))
            .collect();

        // H1 should be blue-ish (high blue component)
        assert!(colors[0].b() > colors[0].r());

        // H2 should be green-ish (high green component)
        assert!(colors[1].g() > colors[1].r());
        assert!(colors[1].g() > colors[1].b());
    }

    #[test]
    fn test_heading_level_color_semantic_light() {
        // Verify each level returns a different color
        let colors: Vec<_> = (1..=6)
            .map(|l| heading_level_color_semantic(l, false))
            .collect();

        // H1 should be blue-ish
        assert!(colors[0].b() > colors[0].r());

        // H2 should be green-ish
        assert!(colors[1].g() > colors[1].r());
        assert!(colors[1].g() > colors[1].b());
    }

    #[test]
    fn test_semantic_minimap_colors_dark() {
        let colors = SemanticMinimapColors::new(true);
        // Dark theme should have dark background
        assert!(colors.background.r() < 50);
        // Current section background should be distinguishable
        assert!(colors.current_section_bg.r() > colors.background.r());
    }

    #[test]
    fn test_semantic_minimap_colors_light() {
        let colors = SemanticMinimapColors::new(false);
        // Light theme should have light background
        assert!(colors.background.r() > 200);
        // Text should be readable (darker than background)
        assert!(colors.text.r() < colors.background.r());
    }

    #[test]
    fn test_semantic_minimap_width_clamping() {
        let headers: Vec<OutlineItem> = vec![];

        let minimap = SemanticMinimap::new(&headers).width(50.0);
        assert_eq!(minimap.width, MIN_SEMANTIC_MINIMAP_WIDTH);

        let minimap = SemanticMinimap::new(&headers).width(300.0);
        assert_eq!(minimap.width, MAX_SEMANTIC_MINIMAP_WIDTH);

        let minimap = SemanticMinimap::new(&headers).width(150.0);
        assert_eq!(minimap.width, 150.0);
    }

    #[test]
    fn test_semantic_minimap_builder() {
        let headers: Vec<OutlineItem> = vec![];

        let minimap = SemanticMinimap::new(&headers)
            .width(120.0)
            .scroll_offset(100.0)
            .content_height(1000.0)
            .line_height(16.0)
            .current_line(Some(50))
            .total_lines(500)
            .show_density(true);

        assert_eq!(minimap.width, 120.0);
        assert_eq!(minimap.scroll_offset, 100.0);
        assert_eq!(minimap.content_height, 1000.0);
        assert_eq!(minimap.line_height, 16.0);
        assert_eq!(minimap.current_line, Some(50));
        assert_eq!(minimap.total_lines, 500);
        assert!(minimap.show_density);
    }

    #[test]
    fn test_semantic_minimap_find_current_section() {
        // Create mock headers
        let headers = vec![
            OutlineItem::new(1, "Title".to_string(), 1, 0, 0),
            OutlineItem::new(2, "Section 1".to_string(), 10, 50, 1),
            OutlineItem::new(2, "Section 2".to_string(), 30, 200, 2),
            OutlineItem::new(3, "Subsection".to_string(), 50, 400, 3),
        ];

        // Test with current_line
        let minimap = SemanticMinimap::new(&headers).current_line(Some(1));
        assert_eq!(minimap.find_current_section(), Some(0)); // At Title

        let minimap = SemanticMinimap::new(&headers).current_line(Some(15));
        assert_eq!(minimap.find_current_section(), Some(1)); // In Section 1

        let minimap = SemanticMinimap::new(&headers).current_line(Some(35));
        assert_eq!(minimap.find_current_section(), Some(2)); // In Section 2

        let minimap = SemanticMinimap::new(&headers).current_line(Some(100));
        assert_eq!(minimap.find_current_section(), Some(3)); // Past all headers, in Subsection
    }

    #[test]
    fn test_semantic_minimap_find_current_section_empty() {
        let headers: Vec<OutlineItem> = vec![];
        let minimap = SemanticMinimap::new(&headers).current_line(Some(10));
        assert_eq!(minimap.find_current_section(), None);
    }

    #[test]
    fn test_semantic_minimap_find_current_section_scroll() {
        // Create mock headers
        let headers = vec![
            OutlineItem::new(1, "Title".to_string(), 1, 0, 0),
            OutlineItem::new(2, "Section".to_string(), 20, 100, 1),
        ];

        // Test without current_line (uses scroll position)
        // scroll_offset = 0, line_height = 16 -> current_line = 1
        let minimap = SemanticMinimap::new(&headers)
            .scroll_offset(0.0)
            .line_height(16.0);
        assert_eq!(minimap.find_current_section(), Some(0));

        // scroll_offset = 320, line_height = 16 -> current_line = 21
        let minimap = SemanticMinimap::new(&headers)
            .scroll_offset(320.0)
            .line_height(16.0);
        assert_eq!(minimap.find_current_section(), Some(1));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Density Calculation Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_density_calculation_empty() {
        let headers: Vec<OutlineItem> = vec![];
        let minimap = SemanticMinimap::new(&headers);
        let densities = minimap.calculate_densities();
        assert!(densities.is_empty());
    }

    #[test]
    fn test_density_calculation_single_item() {
        let headers = vec![OutlineItem::new(1, "Title".to_string(), 1, 0, 0)];
        let minimap = SemanticMinimap::new(&headers).total_lines(100);
        let densities = minimap.calculate_densities();

        assert_eq!(densities.len(), 1);
        // Single item should have normalized value of 0.5 (since range is 0)
        assert_eq!(densities[0].0, 99); // 100 - 1 = 99 lines
        assert!((densities[0].1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_density_calculation_multiple_items() {
        let headers = vec![
            OutlineItem::new(1, "Title".to_string(), 1, 0, 0), // 9 lines to next
            OutlineItem::new(2, "Section 1".to_string(), 10, 50, 1), // 40 lines to next
            OutlineItem::new(2, "Section 2".to_string(), 50, 200, 2), // 50 lines to end
        ];
        let minimap = SemanticMinimap::new(&headers).total_lines(100);
        let densities = minimap.calculate_densities();

        assert_eq!(densities.len(), 3);

        // Line counts
        assert_eq!(densities[0].0, 9); // 10 - 1 = 9
        assert_eq!(densities[1].0, 40); // 50 - 10 = 40
        assert_eq!(densities[2].0, 50); // 100 - 50 = 50

        // Normalized values: min=9, max=50, range=41
        // densities[0] = (9-9)/41 = 0.0
        // densities[1] = (40-9)/41 ≈ 0.756
        // densities[2] = (50-9)/41 = 1.0
        assert!(
            densities[0].1 < 0.01,
            "First item should have lowest density"
        );
        assert!(
            densities[2].1 > 0.99,
            "Last item should have highest density"
        );
        assert!(densities[1].1 > densities[0].1, "Middle should be between");
        assert!(densities[1].1 < densities[2].1, "Middle should be between");
    }

    #[test]
    fn test_density_calculation_equal_sections() {
        // All sections have equal line counts
        let headers = vec![
            OutlineItem::new(1, "Title".to_string(), 1, 0, 0),
            OutlineItem::new(2, "Section 1".to_string(), 11, 50, 1),
            OutlineItem::new(2, "Section 2".to_string(), 21, 100, 2),
        ];
        let minimap = SemanticMinimap::new(&headers).total_lines(31);
        let densities = minimap.calculate_densities();

        // All should have 10 lines each
        assert_eq!(densities[0].0, 10);
        assert_eq!(densities[1].0, 10);
        assert_eq!(densities[2].0, 10);

        // All normalized values should be 0.5 (equal density)
        for (_, normalized) in &densities {
            assert!((normalized - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_density_disabled() {
        let headers = vec![
            OutlineItem::new(1, "Title".to_string(), 1, 0, 0),
            OutlineItem::new(2, "Section".to_string(), 10, 50, 1),
        ];

        // Density enabled by default
        let minimap_enabled = SemanticMinimap::new(&headers).show_density(true);
        assert!(minimap_enabled.show_density);

        // Density can be disabled
        let minimap_disabled = SemanticMinimap::new(&headers).show_density(false);
        assert!(!minimap_disabled.show_density);
    }
}
