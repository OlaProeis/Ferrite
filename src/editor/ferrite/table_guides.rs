//! GFM table column guides for the raw editor (display-only).
//!
//! Detects markdown table blocks in the visible viewport, computes vertical
//! guide positions at pipe (`|`) column boundaries, and caches results per
//! `(start_line, content_hash)` so edits invalidate only affected tables.

use std::collections::HashMap;

use egui::{Color32, FontId, Painter, Stroke};

use super::line_cache::LineCache;

/// A contiguous GFM table block in the buffer (0-indexed line numbers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRange {
    pub start_line: usize,
    /// Exclusive end line.
    pub end_line: usize,
}

/// Cached pipe column indices for a table block.
#[derive(Debug, Clone, PartialEq)]
struct CachedTableGuides {
    end_line: usize,
    pipe_columns: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TableGuideKey {
    start_line: usize,
    content_hash: [u8; 32],
}

/// Per-table guide cache keyed by `(start_line, blake3 content hash)`.
#[derive(Debug, Default, Clone)]
pub struct TableGuideCache {
    entries: HashMap<TableGuideKey, CachedTableGuides>,
}

impl TableGuideCache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn get_or_compute(
        &mut self,
        range: &TableRange,
        get_line: &dyn Fn(usize) -> Option<String>,
    ) -> Option<&CachedTableGuides> {
        let hash = hash_table_lines(range, get_line)?;
        let key = TableGuideKey {
            start_line: range.start_line,
            content_hash: hash,
        };

        if !self.entries.contains_key(&key) {
            let reference = get_line(range.start_line)?;
            let pipe_columns = pipe_char_columns(&reference);
            if pipe_columns.len() < 2 {
                return None;
            }
            self.entries.insert(
                key.clone(),
                CachedTableGuides {
                    end_line: range.end_line,
                    pipe_columns,
                },
            );
        }

        self.entries.get(&key)
    }
}

/// Returns true when `line` looks like a GFM/markdown table row.
#[must_use]
pub fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('|') {
        return trimmed.matches('|').count() >= 2;
    }
    if trimmed.contains('|') && !trimmed.starts_with('>') {
        let parts: Vec<&str> = trimmed.split('|').collect();
        return parts.len() >= 2;
    }
    false
}

/// Returns true when `line` is a GFM delimiter/separator row (`|---|---|`).
#[must_use]
pub fn is_delimiter_row(line: &str) -> bool {
    if !is_table_line(line) {
        return false;
    }
    line.split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .all(|cell| {
            !cell.is_empty()
                && cell
                    .chars()
                    .all(|c| c == '-' || c == ':' || c == ' ')
        })
}

/// Character indices of every pipe in `line`.
#[must_use]
pub fn pipe_char_columns(line: &str) -> Vec<usize> {
    line.chars()
        .enumerate()
        .filter(|(_, c)| *c == '|')
        .map(|(col, _)| col)
        .collect()
}

/// Detect table blocks overlapping `[scan_start, scan_end)` (0-indexed, end exclusive).
///
/// Skips lines inside fenced code blocks. A block qualifies when it has at least
/// two consecutive table lines and the second line is a delimiter row, or when
/// three or more consecutive table lines appear (ragged tables without a strict
/// delimiter still get guides).
pub fn detect_table_ranges(
    get_line: &dyn Fn(usize) -> Option<String>,
    total_lines: usize,
    scan_start: usize,
    scan_end: usize,
) -> Vec<TableRange> {
    let scan_start = scan_start.min(total_lines);
    let scan_end = scan_end.min(total_lines);
    if scan_start >= scan_end {
        return Vec::new();
    }

    // Extend backward so a table that starts above the viewport is still captured.
    let mut extend_start = scan_start;
    while extend_start > 0 {
        let prev = extend_start - 1;
        if in_code_block_at_line(get_line, total_lines, prev) {
            break;
        }
        if let Some(line) = get_line(prev) {
            if is_table_line(&line) {
                extend_start = prev;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let mut ranges = Vec::new();
    let mut line = extend_start;
    let mut in_code_block = in_code_block_at_line(get_line, total_lines, line);

    while line < scan_end {
        if in_code_block {
            if let Some(content) = get_line(line) {
                if content.trim().starts_with("```") {
                    in_code_block = false;
                }
            }
            line += 1;
            continue;
        }

        if let Some(content) = get_line(line) {
            let trimmed = content.trim();
            if trimmed.starts_with("```") {
                in_code_block = true;
                line += 1;
                continue;
            }
        }

        if let Some(content) = get_line(line) {
            if is_table_line(&content) {
                let start = line;
                line += 1;
                let mut table_lines = 1usize;

                while line < total_lines {
                    if in_code_block_at_line(get_line, total_lines, line) {
                        break;
                    }
                    match get_line(line) {
                        Some(next) if is_table_line(&next) => {
                            table_lines += 1;
                            line += 1;
                        }
                        _ => break,
                    }
                }

                let end = line;
                let qualifies = if table_lines >= 2 {
                    if let Some(second) = get_line(start + 1) {
                        is_delimiter_row(&second) || table_lines >= 3
                    } else {
                        false
                    }
                } else {
                    false
                };

                if qualifies {
                    ranges.push(TableRange {
                        start_line: start,
                        end_line: end,
                    });
                }
                continue;
            }
        }

        line += 1;
    }

    ranges
}

/// Returns true when `line_idx` lies inside an opening fenced code block.
fn in_code_block_at_line(
    get_line: &dyn Fn(usize) -> Option<String>,
    total_lines: usize,
    line_idx: usize,
) -> bool {
    if line_idx >= total_lines {
        return false;
    }
    let mut in_block = false;
    for i in 0..line_idx {
        if let Some(content) = get_line(i) {
            if content.trim().starts_with("```") {
                in_block = !in_block;
            }
        }
    }
    in_block
}

fn hash_table_lines(range: &TableRange, get_line: &dyn Fn(usize) -> Option<String>) -> Option<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    for line_idx in range.start_line..range.end_line {
        let line = get_line(line_idx)?;
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    Some(*hasher.finalize().as_bytes())
}

/// Maps pipe character columns to x-offsets (relative to line start).
#[must_use]
pub fn pipe_columns_to_x_offsets(
    line: &str,
    pipe_columns: &[usize],
    painter: &Painter,
    font_id: &FontId,
    line_cache: &LineCache,
) -> Vec<f32> {
    pipe_columns
        .iter()
        .map(|&col| measure_char_x(line, col, painter, font_id, line_cache))
        .collect()
}

fn measure_char_x(
    line: &str,
    col: usize,
    painter: &Painter,
    font_id: &FontId,
    _line_cache: &LineCache,
) -> f32 {
    if col == 0 {
        return 0.0;
    }

    let prefix: String = line.chars().take(col).collect();
    painter
        .layout_no_wrap(prefix, font_id.clone(), Color32::WHITE)
        .size()
        .x
}

/// Paint faint vertical column guides for visible GFM tables.
#[allow(clippy::too_many_arguments)]
pub fn render_table_guides(
    cache: &mut TableGuideCache,
    painter: &Painter,
    get_line: &dyn Fn(usize) -> Option<String>,
    total_lines: usize,
    start_line: usize,
    end_line: usize,
    line_y_positions: &[(usize, f32)],
    text_start_x: f32,
    horizontal_scroll: f32,
    font_id: &FontId,
    line_cache: &LineCache,
    guide_color: Color32,
    wrap_enabled: bool,
    get_line_height: &dyn Fn(usize) -> f32,
) {
    if wrap_enabled {
        return;
    }

    let ranges = detect_table_ranges(get_line, total_lines, start_line, end_line);
    if ranges.is_empty() {
        return;
    }

    let y_for_line = |line_idx: usize| -> Option<f32> {
        line_y_positions
            .iter()
            .find(|(idx, _)| *idx == line_idx)
            .map(|(_, y)| *y)
    };

    let stroke = Stroke::new(1.0, guide_color);

    for range in &ranges {
        let guides = match cache.get_or_compute(range, get_line) {
            Some(g) => g,
            None => continue,
        };

        let reference = match get_line(range.start_line) {
            Some(r) => r,
            None => continue,
        };
        let display_reference = reference.trim_end_matches(['\r', '\n']);

        let x_offsets = pipe_columns_to_x_offsets(
            display_reference,
            &guides.pipe_columns,
            painter,
            font_id,
            line_cache,
        );

        let top_y = match y_for_line(range.start_line) {
            Some(y) => y,
            None => continue,
        };

        let last_line = range.end_line.saturating_sub(1);
        let bottom_y = match y_for_line(last_line) {
            Some(y) => y + get_line_height(last_line),
            None => continue,
        };

        for x in x_offsets {
            let x_screen = text_start_x + x - horizontal_scroll;
            painter.line_segment(
                [egui::pos2(x_screen, top_y), egui::pos2(x_screen, bottom_y)],
                stroke,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_from(source: &str) -> impl Fn(usize) -> Option<String> + '_ {
        let lines: Vec<String> = source.lines().map(str::to_string).collect();
        move |idx| lines.get(idx).cloned()
    }

    #[test]
    fn is_table_line_recognizes_gfm_rows() {
        assert!(is_table_line("| A | B |"));
        assert!(is_table_line("| --- | --- |"));
        assert!(is_table_line("col1 | col2 | col3"));
        assert!(!is_table_line("regular text"));
        assert!(!is_table_line("> | not a table"));
        assert!(!is_table_line(""));
    }

    #[test]
    fn is_delimiter_row_detects_alignment_markers() {
        assert!(is_delimiter_row("| :--- | :----: | ---: |"));
        assert!(is_delimiter_row("|---|---|"));
        assert!(!is_delimiter_row("| data | row |"));
    }

    #[test]
    fn pipe_char_columns_finds_all_pipes() {
        assert_eq!(pipe_char_columns("| A | B |"), vec![0, 4, 8]);
        assert_eq!(pipe_char_columns("|ragged| row|"), vec![0, 7, 12]);
    }

    #[test]
    fn detect_table_range_aligned_table() {
        let source = "intro\n| H1 | H2 |\n| --- | --- |\n| a | b |\n\nafter";
        let get_line = lines_from(source);
        let total = source.lines().count();
        let ranges = detect_table_ranges(&get_line, total, 0, total);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 1);
        assert_eq!(ranges[0].end_line, 4);
    }

    #[test]
    fn detect_table_range_ragged_table() {
        let source = "| long header | x |\n| --- | --- |\n| y | wide cell here |";
        let get_line = lines_from(source);
        let total = source.lines().count();
        let ranges = detect_table_ranges(&get_line, total, 0, total);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 3);

        let header_cols = pipe_char_columns(&get_line(0).unwrap());
        let body_cols = pipe_char_columns(&get_line(2).unwrap());
        assert_ne!(header_cols, body_cols);
    }

    #[test]
    fn detect_table_skips_fenced_code_block() {
        let source = "```\n| fake | table |\n| --- | --- |\n```\n| real | table |\n| --- | --- |";
        let get_line = lines_from(source);
        let total = source.lines().count();
        let ranges = detect_table_ranges(&get_line, total, 0, total);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 4);
    }

    #[test]
    fn detect_table_extends_backward_into_viewport() {
        let source = "| H | V |\n| - | - |\n| 1 | 2 |\n| 3 | 4 |";
        let get_line = lines_from(source);
        let total = source.lines().count();
        // Viewport starts at line 2 but table begins at line 0.
        let ranges = detect_table_ranges(&get_line, total, 2, total);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0);
    }

    #[test]
    fn cache_invalidates_on_content_change() {
        let mut cache = TableGuideCache::default();
        let range = TableRange {
            start_line: 0,
            end_line: 2,
        };

        let v1 = "| A | B |\n| --- | --- |";
        let get_v1 = lines_from(v1);
        let first = cache.get_or_compute(&range, &get_v1);
        assert!(first.is_some());
        assert_eq!(first.unwrap().pipe_columns, vec![0, 4, 8]);

        let v2 = "| AA | BB |\n| --- | --- |";
        let get_v2 = lines_from(v2);
        let second = cache.get_or_compute(&range, &get_v2);
        assert!(second.is_some());
        assert_eq!(second.unwrap().pipe_columns, vec![0, 5, 10]);
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn hash_table_lines_changes_when_edited() {
        let range = TableRange {
            start_line: 0,
            end_line: 2,
        };
        let before = lines_from("| a | b |\n| - | - |");
        let after = lines_from("| a | b |\n| - | c |");
        assert_ne!(
            hash_table_lines(&range, &before),
            hash_table_lines(&range, &after)
        );
    }
}
