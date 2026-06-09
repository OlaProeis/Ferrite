//! CSV/TSV Table Viewer for Ferrite
//!
//! This module provides a scrollable table view for CSV and TSV files,
//! with fixed-width columns, header highlighting, and cell tooltips.
//!
//! # Features
//! - Automatic delimiter detection (comma for CSV, tab for TSV)
//! - Fixed-width columns based on content
//! - Header row highlighting (first row)
//! - Horizontal and vertical scrolling
//! - Cell tooltips for truncated content
//! - Large file handling with row limiting

use crate::ui::phosphor_icons::{phosphor_rich_text, INFO};
use eframe::egui::{self, Color32, Key, RichText, ScrollArea, Sense, TextEdit, Ui, Vec2};
use log::warn;
use palette::{IntoColor, Oklch, Srgb};
use rust_i18n::t;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum column width in pixels
const MAX_COLUMN_WIDTH: f32 = 300.0;

/// Minimum column width in pixels
const MIN_COLUMN_WIDTH: f32 = 50.0;

/// Padding between columns
const COLUMN_PADDING: f32 = 16.0;

/// Maximum characters to display in a cell before truncating
const MAX_CELL_CHARS: usize = 50;

/// Large file threshold in bytes (1MB). Files at or above this size disable rendered cell editing.
pub const LARGE_FILE_THRESHOLD: usize = 1_000_000;

/// Number of lines to sample for delimiter detection
const DELIMITER_SAMPLE_LINES: usize = 10;

/// Candidate delimiters to test (comma, tab, semicolon, pipe)
pub const DELIMITERS: &[u8] = b",\t;|";

/// Minimum rows needed to attempt header detection
const MIN_ROWS_FOR_HEADER_DETECTION: usize = 2;

/// Number of extra rows to render above/below visible area for smooth scrolling
const VIRTUAL_SCROLL_BUFFER: usize = 5;

/// Number of extra rows to cache beyond the render buffer for lazy parsing.
/// Wider cache window = fewer re-parses during scrolling.
/// Total cached rows ≈ visible + 2*(VIRTUAL_SCROLL_BUFFER + LAZY_CACHE_BUFFER).
const LAZY_CACHE_BUFFER: usize = 50;

// ─────────────────────────────────────────────────────────────────────────────
// Tabular File Type
// ─────────────────────────────────────────────────────────────────────────────

/// Supported tabular file types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabularFileType {
    Csv,
    Tsv,
}

impl TabularFileType {
    /// Detect tabular file type from extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "tsv" => Some(Self::Tsv),
            _ => None,
        }
    }

    /// Get the delimiter character for this file type.
    pub fn delimiter(&self) -> u8 {
        match self {
            Self::Csv => b',',
            Self::Tsv => b'\t',
        }
    }

    /// Get display name for the file type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delimiter Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Information about a detected delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimiterInfo {
    /// The detected delimiter byte
    pub delimiter: u8,
    /// Confidence score (higher is better)
    pub score: usize,
}

impl DelimiterInfo {
    /// Get display name for the delimiter.
    #[allow(dead_code)] // Public API for delimiter info display
    pub fn display_name(&self) -> &'static str {
        delimiter_display_name(self.delimiter)
    }
}

/// Get display name for a delimiter byte.
pub fn delimiter_display_name(delimiter: u8) -> &'static str {
    match delimiter {
        b',' => "Comma",
        b'\t' => "Tab",
        b';' => "Semicolon",
        b'|' => "Pipe",
        _ => "Unknown",
    }
}

/// Get short display symbol for a delimiter byte (for status bar).
pub fn delimiter_symbol(delimiter: u8) -> &'static str {
    match delimiter {
        b',' => ",",
        b'\t' => "⇥",
        b';' => ";",
        b'|' => "|",
        _ => "?",
    }
}

/// Detect the most likely delimiter from file content.
///
/// Analyzes the first few lines and scores each candidate delimiter
/// based on parse success and column consistency.
pub fn detect_delimiter(content: &str) -> DelimiterInfo {
    let sample_lines: Vec<&str> = content.lines().take(DELIMITER_SAMPLE_LINES).collect();

    if sample_lines.is_empty() {
        // Default to comma for empty content
        return DelimiterInfo {
            delimiter: b',',
            score: 0,
        };
    }

    let mut best_delimiter = b',';
    let mut best_score = 0;

    for &delim in DELIMITERS {
        let score = score_delimiter(&sample_lines, delim);
        if score > best_score {
            best_score = score;
            best_delimiter = delim;
        }
    }

    DelimiterInfo {
        delimiter: best_delimiter,
        score: best_score,
    }
}

/// Score a delimiter based on how well it parses the sample lines.
///
/// Scoring criteria:
/// 1. Consistent column count across lines (major factor)
/// 2. Successful CSV parse without errors
/// 3. Reasonable number of columns (not too few, not too many)
/// 4. Non-empty columns preferred
fn score_delimiter(lines: &[&str], delimiter: u8) -> usize {
    if lines.is_empty() {
        return 0;
    }

    // Try to parse each line with this delimiter
    let mut column_counts: Vec<usize> = Vec::new();
    let mut total_non_empty_cells = 0;
    let mut parse_errors = 0;

    for line in lines {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Create a temporary reader for this single line
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(line.as_bytes());

        match reader.records().next() {
            Some(Ok(record)) => {
                let col_count = record.len();
                column_counts.push(col_count);

                // Count non-empty cells
                for field in record.iter() {
                    if !field.trim().is_empty() {
                        total_non_empty_cells += 1;
                    }
                }
            }
            Some(Err(_)) => {
                parse_errors += 1;
            }
            None => {
                // Empty record
                column_counts.push(0);
            }
        }
    }

    if column_counts.is_empty() {
        return 0;
    }

    // Calculate score based on:
    // 1. Column consistency (all lines should have similar column count)
    let first_count = column_counts[0];
    let consistent_count = column_counts.iter().filter(|&&c| c == first_count).count();
    let consistency_score = (consistent_count * 100) / column_counts.len();

    // 2. Penalize single-column results (probably wrong delimiter)
    let column_score = if first_count == 1 {
        10 // Very low score for single column
    } else if first_count >= 2 && first_count <= 50 {
        50 + first_count.min(20) // Good range
    } else {
        30 // Too many columns is suspicious
    };

    // 3. Non-empty cells bonus
    let non_empty_bonus = (total_non_empty_cells * 10) / (column_counts.len() * first_count.max(1));

    // 4. Parse error penalty
    let error_penalty = parse_errors * 20;

    // Calculate final score
    let raw_score = consistency_score + column_score + non_empty_bonus;
    raw_score.saturating_sub(error_penalty)
}

// ─────────────────────────────────────────────────────────────────────────────
// Header Row Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Determine if the first row looks like a header row.
///
/// Uses multiple heuristics:
/// 1. First row values are shorter on average than data rows
/// 2. First row has no numeric-only values (headers are usually text)
/// 3. First row values don't match patterns in data rows (e.g., dates, numbers)
/// 4. First row has more unique/distinct formatting than data rows
pub fn detect_header_row(rows: &[Vec<String>]) -> bool {
    if rows.len() < MIN_ROWS_FOR_HEADER_DETECTION {
        // Default to treating first row as header if we can't analyze
        return true;
    }

    let first_row = &rows[0];
    let data_rows = &rows[1..];

    // Skip if first row is empty
    if first_row.is_empty() || first_row.iter().all(|s| s.trim().is_empty()) {
        return false;
    }

    let mut header_score = 0i32;
    let max_score = 100;

    // Heuristic 1: First row should have no numeric-only values
    // Headers are typically text labels, not numbers
    let first_row_numeric_count = first_row
        .iter()
        .filter(|s| is_numeric_value(s.trim()))
        .count();
    let first_row_numeric_ratio = first_row_numeric_count as f32 / first_row.len().max(1) as f32;

    if first_row_numeric_ratio == 0.0 {
        header_score += 30; // Strong indicator: no numbers in first row
    } else if first_row_numeric_ratio < 0.3 {
        header_score += 15; // Some numbers, but mostly text
    } else {
        header_score -= 20; // Too many numbers for a header row
    }

    // Heuristic 2: Compare average length - headers tend to be shorter labels
    let first_row_avg_len: f32 = first_row.iter().map(|s| s.trim().len() as f32).sum::<f32>()
        / first_row.len().max(1) as f32;

    let data_avg_len: f32 = if !data_rows.is_empty() {
        data_rows
            .iter()
            .flat_map(|row| row.iter())
            .map(|s| s.trim().len() as f32)
            .sum::<f32>()
            / (data_rows.len() * first_row.len()).max(1) as f32
    } else {
        0.0
    };

    if first_row_avg_len > 0.0 && first_row_avg_len <= data_avg_len * 1.5 {
        header_score += 15;
    }

    // Heuristic 3: Data rows should have more numeric content than the header
    let data_numeric_ratio: f32 = if !data_rows.is_empty() {
        let total_cells: usize = data_rows.iter().map(|r| r.len()).sum();
        let numeric_cells: usize = data_rows
            .iter()
            .flat_map(|row| row.iter())
            .filter(|s| is_numeric_value(s.trim()))
            .count();
        numeric_cells as f32 / total_cells.max(1) as f32
    } else {
        0.0
    };

    // If data has more numbers than header, that's a good sign
    if data_numeric_ratio > first_row_numeric_ratio + 0.1 {
        header_score += 25;
    }

    // Heuristic 4: Check for common header patterns (case-insensitive)
    let header_keywords = [
        "id",
        "name",
        "date",
        "time",
        "value",
        "count",
        "total",
        "type",
        "status",
        "description",
        "title",
        "email",
        "phone",
        "address",
        "city",
        "state",
        "country",
        "zip",
        "code",
        "price",
        "amount",
        "quantity",
        "number",
        "created",
        "updated",
        "modified",
        "age",
        "year",
        "month",
        "day",
    ];

    let keyword_matches = first_row
        .iter()
        .filter(|s| {
            let lower = s.trim().to_lowercase();
            header_keywords.iter().any(|kw| lower.contains(kw))
        })
        .count();

    if keyword_matches > 0 {
        header_score += (keyword_matches as i32 * 10).min(30);
    }

    // Heuristic 5: First row values should be unique (no duplicates typical of data)
    let unique_first_row: std::collections::HashSet<_> =
        first_row.iter().map(|s| s.trim().to_lowercase()).collect();
    if unique_first_row.len() == first_row.len() {
        header_score += 10;
    }

    // Return true if score exceeds threshold
    header_score > max_score / 3
}

/// Check if a string looks like a numeric value (integer, float, currency, percentage).
fn is_numeric_value(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let s = s.trim();

    // Remove common prefixes/suffixes
    let cleaned: String = s
        .trim_start_matches(|c| c == '$' || c == '€' || c == '£' || c == '¥')
        .trim_end_matches('%')
        .trim()
        .replace(',', "")
        .replace(' ', "");

    // Check if it's a valid number
    cleaned.parse::<f64>().is_ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV Parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Parsed CSV data with metadata.
#[derive(Debug, Clone)]
pub struct CsvData {
    /// Parsed rows (each row is a vector of cell strings)
    pub rows: Vec<Vec<String>>,
    /// Calculated column widths (in characters)
    pub column_widths: Vec<usize>,
    /// Total number of columns
    pub num_columns: usize,
    /// Total row count
    pub row_count: usize,
}

impl CsvData {
    /// Update a cell value, growing the row vector if needed.
    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if row >= self.rows.len() {
            self.rows.resize(row + 1, Vec::new());
        }
        let row_ref = &mut self.rows[row];
        if col >= row_ref.len() {
            row_ref.resize(col + 1, String::new());
        }
        row_ref[col] = value;
        self.num_columns = self.num_columns.max(row_ref.len());
        for r in &mut self.rows {
            while r.len() < self.num_columns {
                r.push(String::new());
            }
        }
        self.row_count = self.rows.len();
    }
}

/// Serialize parsed rows back to CSV/TSV text using the `csv` crate writer.
pub fn serialize_csv_rows(rows: &[Vec<String>], delimiter: u8) -> Result<String, CsvParseError> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_writer(Vec::new());

    for row in rows {
        writer.write_record(row).map_err(|e| CsvParseError {
            message: e.to_string(),
            line: None,
        })?;
    }

    let bytes = writer.into_inner().map_err(|e| CsvParseError {
        message: e.to_string(),
        line: None,
    })?;

    String::from_utf8(bytes).map_err(|e| CsvParseError {
        message: e.to_string(),
        line: None,
    })
}

/// Whether rendered table cell editing is allowed for this content size.
pub fn csv_rendered_editing_enabled(content_len: usize) -> bool {
    content_len < LARGE_FILE_THRESHOLD
}

/// Parse error for CSV files.
#[derive(Debug, Clone)]
pub struct CsvParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl std::fmt::Display for CsvParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "Line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Parse CSV/TSV content into structured data using the file type's default delimiter.
#[allow(dead_code)] // Public API wrapper for parse_csv_with_delimiter
pub fn parse_csv(content: &str, file_type: TabularFileType) -> Result<CsvData, CsvParseError> {
    parse_csv_with_delimiter(content, file_type.delimiter())
}

/// Parse CSV/TSV content into structured data with an explicit delimiter.
///
/// Parses all rows from the content. Virtual scrolling is used during rendering
/// to handle large files efficiently.
pub fn parse_csv_with_delimiter(content: &str, delimiter: u8) -> Result<CsvData, CsvParseError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true) // Allow varying number of columns per row
        .from_reader(content.as_bytes());

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut max_columns = 0;

    // Sample first N rows for column width calculation (optimization for large files)
    const WIDTH_SAMPLE_ROWS: usize = 1000;

    for (line_num, result) in reader.records().enumerate() {
        match result {
            Ok(record) => {
                let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                max_columns = max_columns.max(row.len());
                rows.push(row);
            }
            Err(e) => {
                warn!("CSV parse error at line {}: {}", line_num + 1, e);
                return Err(CsvParseError {
                    message: e.to_string(),
                    line: Some(line_num + 1),
                });
            }
        }
    }

    let row_count = rows.len();

    // Normalize rows to have the same number of columns
    for row in &mut rows {
        while row.len() < max_columns {
            row.push(String::new());
        }
    }

    // Calculate column widths based on content (sample for large files)
    let sample_size = rows.len().min(WIDTH_SAMPLE_ROWS);
    let mut column_widths = vec![0usize; max_columns];
    for row in rows.iter().take(sample_size) {
        for (col_idx, cell) in row.iter().enumerate() {
            let char_count = cell.chars().count();
            column_widths[col_idx] = column_widths[col_idx].max(char_count);
        }
    }

    // Cap column widths
    for width in &mut column_widths {
        *width = (*width).clamp(3, MAX_CELL_CHARS);
    }

    Ok(CsvData {
        rows,
        column_widths,
        num_columns: max_columns,
        row_count,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Lazy CSV Parsing (byte-offset indexing)
// ─────────────────────────────────────────────────────────────────────────────

/// Lightweight row index for lazy CSV parsing.
///
/// Instead of parsing every row into `Vec<Vec<String>>`, this struct stores
/// only the byte offset where each row begins. Actual row data is parsed
/// on demand for visible rows only, keeping memory low for 100MB+ files.
///
/// Memory comparison for a 1M-row CSV:
/// - Full parse: ~100–200MB (String allocations per cell)
/// - Row index:  ~8MB (8 bytes per row offset)
#[derive(Debug, Clone)]
pub struct CsvRowIndex {
    /// Byte offset of each row start in the source content.
    /// `row_offsets[i]` is the byte position where row i begins.
    pub row_offsets: Vec<u64>,
    /// Total number of rows in the CSV.
    pub row_count: usize,
    /// Maximum number of columns detected across sampled rows.
    pub num_columns: usize,
    /// Calculated column widths (in characters), sampled from first N rows.
    pub column_widths: Vec<usize>,
    /// First row data (always kept for header detection/display).
    pub first_row: Vec<String>,
}

/// Cached window of parsed visible rows for lazy-rendered CSV data.
/// Stores a range of parsed rows around the current viewport to avoid
/// re-parsing on every frame during scrolling.
#[derive(Debug, Clone)]
struct CachedVisibleRows {
    /// Start row index in the full file (inclusive).
    start_row: usize,
    /// End row index in the full file (exclusive).
    end_row: usize,
    /// Parsed rows for this range.
    rows: Vec<Vec<String>>,
}

/// Fast content hash for cache invalidation.
///
/// For large content (>8KB), samples the beginning and end to avoid
/// hashing the entire 100MB+ buffer while still detecting changes.
fn hash_content_bytes(content: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.len().hash(&mut hasher);
    if content.len() > 8192 {
        content[..4096].hash(&mut hasher);
        content[content.len() - 4096..].hash(&mut hasher);
    } else {
        content.hash(&mut hasher);
    }
    hasher.finish()
}

/// Blake3 content hash truncated to u64.
///
/// Consistent with the Markdown AST cache pattern (Task 4).
/// Full-content hash — no sampling — so even mid-file edits are detected.
fn blake3_content_hash(content: &[u8]) -> u64 {
    let hash = blake3::hash(content);
    let bytes = hash.as_bytes();
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

/// Build a row offset index from CSV content.
///
/// Scans the entire content recording byte offsets per row, and samples
/// the first 1000 rows for column width calculation. This is O(N) in file
/// size but only allocates 8 bytes per row (vs full string parsing).
///
/// # Arguments
/// * `content` - Raw CSV file bytes
/// * `delimiter` - The delimiter byte (comma, tab, etc.)
pub fn build_csv_row_index(content: &[u8], delimiter: u8) -> Result<CsvRowIndex, CsvParseError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(content);

    let mut row_offsets: Vec<u64> = Vec::new();
    let mut max_columns: usize = 0;
    let mut column_widths: Vec<usize> = Vec::new();
    let mut first_row: Vec<String> = Vec::new();

    const WIDTH_SAMPLE_ROWS: usize = 1000;

    let mut record = csv::ByteRecord::new();
    let mut row_idx: usize = 0;

    loop {
        // Capture byte offset BEFORE reading (position of the next record)
        let byte_offset = reader.position().byte();

        match reader.read_byte_record(&mut record) {
            Ok(true) => {
                row_offsets.push(byte_offset);
                let col_count = record.len();
                max_columns = max_columns.max(col_count);

                // Always keep first row for header detection/display
                if row_idx == 0 {
                    first_row = record
                        .iter()
                        .map(|f| String::from_utf8_lossy(f).to_string())
                        .collect();
                }

                // Sample column widths from first N rows
                if row_idx < WIDTH_SAMPLE_ROWS {
                    while column_widths.len() < col_count {
                        column_widths.push(0);
                    }
                    for (ci, field) in record.iter().enumerate() {
                        let char_count = String::from_utf8_lossy(field).chars().count();
                        column_widths[ci] = column_widths[ci].max(char_count);
                    }
                }

                row_idx += 1;
            }
            Ok(false) => break,
            Err(e) => {
                warn!("CSV index build error at row {}: {}", row_idx, e);
                return Err(CsvParseError {
                    message: e.to_string(),
                    line: Some(row_idx + 1),
                });
            }
        }
    }

    // Cap column widths
    for width in &mut column_widths {
        *width = (*width).clamp(3, MAX_CELL_CHARS);
    }

    // Normalize first_row to have max_columns entries
    while first_row.len() < max_columns {
        first_row.push(String::new());
    }

    Ok(CsvRowIndex {
        row_offsets,
        row_count: row_idx,
        num_columns: max_columns,
        column_widths,
        first_row,
    })
}

/// Parse a specific range of rows from CSV content using the row index.
///
/// Only parses rows in `[start_row, end_row)` range by slicing the content
/// at the known byte offsets and running the CSV parser on just that slice.
///
/// # Arguments
/// * `content` - Raw CSV file bytes
/// * `delimiter` - The delimiter byte
/// * `index` - Row offset index built by `build_csv_row_index`
/// * `start_row` - First row to parse (inclusive, 0-based)
/// * `end_row` - Last row to parse (exclusive)
pub fn parse_csv_row_range(
    content: &[u8],
    delimiter: u8,
    index: &CsvRowIndex,
    start_row: usize,
    end_row: usize,
) -> Vec<Vec<String>> {
    if start_row >= index.row_count || start_row >= end_row {
        return Vec::new();
    }

    let end_row = end_row.min(index.row_count);
    let start_byte = index.row_offsets[start_row] as usize;
    let end_byte = if end_row < index.row_count {
        index.row_offsets[end_row] as usize
    } else {
        content.len()
    };

    // Bounds check
    if start_byte >= content.len() || start_byte >= end_byte {
        return Vec::new();
    }
    let end_byte = end_byte.min(content.len());

    let slice = &content[start_byte..end_byte];

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(slice);

    let expected_count = end_row - start_row;
    let mut rows = Vec::with_capacity(expected_count);

    for result in reader.records() {
        match result {
            Ok(record) => {
                let mut row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                // Normalize to num_columns
                while row.len() < index.num_columns {
                    row.push(String::new());
                }
                rows.push(row);
            }
            Err(e) => {
                warn!("CSV row range parse error: {}", e);
                break;
            }
        }
    }

    rows
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV Viewer State
// ─────────────────────────────────────────────────────────────────────────────

/// State for the CSV viewer widget.
#[derive(Debug, Clone, Default)]
pub struct CsvViewerState {
    /// Whether to show the raw view instead of table view
    pub show_raw: bool,
    /// Large file warning dismissed
    large_file_warning_dismissed: bool,
    /// Cached parsed data (used for small files < LARGE_FILE_THRESHOLD)
    cached_data: Option<CsvData>,
    /// Content hash for cache invalidation
    content_hash: u64,
    /// Detected or manually overridden delimiter
    /// None means auto-detect from content
    delimiter_override: Option<u8>,
    /// Last auto-detected delimiter (cached)
    detected_delimiter: Option<u8>,
    /// Auto-detected header status (None = not yet detected)
    detected_has_header: Option<bool>,
    /// Manual override for header row (None = use auto-detected)
    header_override: Option<bool>,
    /// Cached row index for lazy parsing (used for large files >= LARGE_FILE_THRESHOLD).
    /// Only stores byte offsets per row — actual rows are parsed on demand.
    cached_index: Option<CsvRowIndex>,
    /// Cached window of parsed visible rows for lazy rendering.
    /// Updated when the viewport scrolls beyond the cached range.
    cached_visible: Option<CachedVisibleRows>,
    /// Cached raw view text to avoid per-frame `.to_string()` allocation.
    /// Rebuilt only when content hash changes.
    raw_view_text: String,
    /// Blake3 hash (truncated to u64) of content when `raw_view_text` was last built.
    raw_view_hash: u64,
    /// Selected cell `(file_row_index, col_index)` for keyboard navigation.
    pub selected_cell: Option<(usize, usize)>,
    /// Cell currently in inline edit mode `(file_row_index, col_index)`.
    pub editing_cell: Option<(usize, usize)>,
    /// Draft text while a cell is being edited.
    pub edit_buffer: String,
    /// Cell edit committed during the last frame (applied after table render).
    pending_commit: Option<(usize, usize, String)>,
}

impl CsvViewerState {
    #[allow(dead_code)] // Public API for state initialization
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear cached data (call when content changes).
    #[allow(dead_code)] // Public API for cache management
    pub fn invalidate_cache(&mut self) {
        self.cached_data = None;
        self.cached_index = None;
        self.cached_visible = None;
        self.content_hash = 0;
        self.raw_view_text = String::new();
        self.raw_view_hash = 0;
        self.selected_cell = None;
        self.editing_cell = None;
        self.edit_buffer.clear();
        self.pending_commit = None;
        // Also clear detected delimiter since content changed
        self.detected_delimiter = None;
    }

    /// Get the effective delimiter (override or detected).
    pub fn effective_delimiter(&self) -> Option<u8> {
        self.delimiter_override.or(self.detected_delimiter)
    }

    /// Set a manual delimiter override.
    pub fn set_delimiter(&mut self, delimiter: u8) {
        self.delimiter_override = Some(delimiter);
        // Invalidate all caches when delimiter changes
        self.cached_data = None;
        self.cached_index = None;
        self.cached_visible = None;
        self.content_hash = 0;
        self.raw_view_text = String::new();
        self.raw_view_hash = 0;
    }

    /// Clear manual delimiter override (return to auto-detect).
    pub fn clear_delimiter_override(&mut self) {
        self.delimiter_override = None;
        self.cached_data = None;
        self.cached_index = None;
        self.cached_visible = None;
        self.content_hash = 0;
        self.raw_view_text = String::new();
        self.raw_view_hash = 0;
    }

    /// Check if delimiter is manually overridden.
    pub fn has_delimiter_override(&self) -> bool {
        self.delimiter_override.is_some()
    }

    /// Get the delimiter override if set.
    pub fn delimiter_override(&self) -> Option<u8> {
        self.delimiter_override
    }

    /// Set the detected delimiter (called during parsing).
    pub fn set_detected_delimiter(&mut self, delimiter: u8) {
        self.detected_delimiter = Some(delimiter);
    }

    /// Get the effective header status (override or detected, defaults to true).
    pub fn has_headers(&self) -> bool {
        self.header_override
            .unwrap_or_else(|| self.detected_has_header.unwrap_or(true))
    }

    /// Set the detected header status (called during parsing).
    pub fn set_detected_has_header(&mut self, has_header: bool) {
        self.detected_has_header = Some(has_header);
    }

    /// Toggle the header override.
    #[allow(dead_code)] // Public API for header management
    pub fn toggle_header(&mut self) {
        let current = self.has_headers();
        self.header_override = Some(!current);
    }

    /// Set a manual header override.
    pub fn set_header_override(&mut self, has_header: bool) {
        self.header_override = Some(has_header);
    }

    /// Clear the manual header override (return to auto-detect).
    pub fn clear_header_override(&mut self) {
        self.header_override = None;
    }

    /// Check if header is manually overridden.
    pub fn has_header_override(&self) -> bool {
        self.header_override.is_some()
    }

    /// Get the detected header status (before any override).
    #[allow(dead_code)] // Public API for header detection
    pub fn detected_has_header(&self) -> Option<bool> {
        self.detected_has_header
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV Viewer Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Number of colors in the rainbow column palette.
/// 12 colors gives 30° hue steps around the color wheel.
const RAINBOW_PALETTE_SIZE: usize = 12;

/// Chroma (saturation) for rainbow column colors.
/// Very subtle (0.03-0.05) to avoid distracting from content.
const RAINBOW_CHROMA: f32 = 0.04;

/// Generate theme-aware rainbow column colors using Oklch perceptual color space.
///
/// Colors are evenly distributed around the hue wheel with very subtle saturation
/// to provide visual differentiation without being distracting.
///
/// # Arguments
/// * `dark_mode` - Whether to generate colors for dark mode (lighter colors) or light mode (darker colors)
///
/// # Returns
/// A vector of 12 colors, each 30° apart on the hue wheel.
fn generate_rainbow_colors(dark_mode: bool) -> Vec<Color32> {
    // Lightness values chosen for good contrast with text:
    // - Dark mode: 0.30-0.35 (dark backgrounds need lighter tints)
    // - Light mode: 0.92-0.95 (light backgrounds need darker tints)
    let lightness = if dark_mode { 0.32 } else { 0.94 };

    (0..RAINBOW_PALETTE_SIZE)
        .map(|i| {
            let hue = (i as f32 * 30.0) % 360.0; // 30° steps = 12 colors
            let oklch = Oklch::new(lightness, RAINBOW_CHROMA, hue);
            let rgb: Srgb = oklch.into_color();

            // Clamp values to 0.0-1.0 range before converting to u8
            // (Oklch to sRGB conversion can produce out-of-gamut colors)
            let r = (rgb.red.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (rgb.green.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (rgb.blue.clamp(0.0, 1.0) * 255.0) as u8;

            Color32::from_rgb(r, g, b)
        })
        .collect()
}

/// Blend two colors together with a given factor.
///
/// # Arguments
/// * `base` - The base color
/// * `overlay` - The color to blend on top
/// * `factor` - Blend factor (0.0 = all base, 1.0 = all overlay)
///
/// # Returns
/// The blended color.
fn blend_colors(base: Color32, overlay: Color32, factor: f32) -> Color32 {
    let factor = factor.clamp(0.0, 1.0);
    let inv_factor = 1.0 - factor;

    let r = (base.r() as f32 * inv_factor + overlay.r() as f32 * factor) as u8;
    let g = (base.g() as f32 * inv_factor + overlay.g() as f32 * factor) as u8;
    let b = (base.b() as f32 * inv_factor + overlay.b() as f32 * factor) as u8;
    let a = (base.a() as f32 * inv_factor + overlay.a() as f32 * factor) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Colors for the CSV viewer.
#[derive(Debug, Clone)]
pub struct CsvViewerColors {
    pub header_bg: Color32,
    pub header_text: Color32,
    pub cell_text: Color32,
    pub row_even_bg: Color32,
    pub row_odd_bg: Color32,
    pub truncated_indicator: Color32,
    pub error: Color32,
    /// Rainbow column colors for visual column differentiation
    pub column_colors: Vec<Color32>,
    /// Whether rainbow column coloring is enabled
    pub rainbow_enabled: bool,
}

impl CsvViewerColors {
    pub fn dark() -> Self {
        Self {
            header_bg: Color32::from_rgb(50, 50, 60),
            header_text: Color32::from_rgb(200, 200, 220),
            cell_text: Color32::from_rgb(220, 220, 220),
            row_even_bg: Color32::from_rgb(30, 30, 35),
            row_odd_bg: Color32::from_rgb(40, 40, 45),
            truncated_indicator: Color32::from_rgb(150, 150, 150),
            error: Color32::from_rgb(255, 100, 100),
            column_colors: generate_rainbow_colors(true),
            rainbow_enabled: false,
        }
    }

    pub fn light() -> Self {
        Self {
            header_bg: Color32::from_rgb(230, 230, 240),
            header_text: Color32::from_rgb(30, 30, 40),
            cell_text: Color32::from_rgb(40, 40, 40),
            row_even_bg: Color32::from_rgb(255, 255, 255),
            row_odd_bg: Color32::from_rgb(245, 245, 250),
            truncated_indicator: Color32::from_rgb(120, 120, 120),
            error: Color32::from_rgb(200, 50, 50),
            column_colors: generate_rainbow_colors(false),
            rainbow_enabled: false,
        }
    }

    pub fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// Create colors with rainbow column coloring enabled/disabled.
    pub fn with_rainbow(mut self, enabled: bool) -> Self {
        self.rainbow_enabled = enabled;
        self
    }

    /// Get the background color for a cell, optionally blended with column color.
    ///
    /// # Arguments
    /// * `base_color` - The base row background color
    /// * `col_idx` - The column index for rainbow coloring
    /// * `blend_factor` - How much to blend the column color (0.0-1.0)
    pub fn cell_background(
        &self,
        base_color: Color32,
        col_idx: usize,
        blend_factor: f32,
    ) -> Color32 {
        if self.rainbow_enabled && !self.column_colors.is_empty() {
            let col_color = self.column_colors[col_idx % self.column_colors.len()];
            blend_colors(base_color, col_color, blend_factor)
        } else {
            base_color
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV Viewer Widget
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the CSV viewer widget.
#[derive(Debug, Clone)]
pub struct CsvViewerOutput {
    /// Whether the user requested to toggle raw view
    pub toggle_raw_requested: bool,
    /// Current scroll offset
    pub scroll_offset: f32,
    /// Whether headers are displayed (considers override)
    pub has_headers: bool,
    /// Whether headers were auto-detected (vs manually set)
    pub headers_auto_detected: bool,
    /// Tab content was modified (caller should record undo + mark dirty).
    pub content_changed: bool,
}

/// Context for inline cell editing in the rendered table (< 1 MB files).
struct CsvCellEditParams<'a> {
    state: &'a mut CsvViewerState,
    editing_enabled: bool,
}

fn apply_pending_cell_commit(
    state: &mut CsvViewerState,
    data: &mut CsvData,
    content: &mut String,
    delimiter: u8,
    content_changed: &mut bool,
) {
    let Some((row, col, new_value)) = state.pending_commit.take() else {
        return;
    };
    data.set_cell(row, col, new_value);
    match serialize_csv_rows(&data.rows, delimiter) {
        Ok(serialized) => {
            if *content != serialized {
                state.content_hash = hash_content_bytes(serialized.as_bytes());
                *content = serialized;
                *content_changed = true;
            }
        }
        Err(e) => {
            warn!("Failed to serialize CSV after cell edit: {}", e);
        }
    }
}

fn queue_cell_commit(state: &mut CsvViewerState, row: usize, col: usize, new_value: String) {
    state.pending_commit = Some((row, col, new_value));
    state.editing_cell = None;
    state.edit_buffer.clear();
}

fn begin_cell_edit(state: &mut CsvViewerState, row: usize, col: usize, value: &str) {
    state.selected_cell = Some((row, col));
    state.editing_cell = Some((row, col));
    state.edit_buffer = value.to_string();
}

fn cancel_cell_edit(state: &mut CsvViewerState) {
    state.editing_cell = None;
    state.edit_buffer.clear();
}

fn move_selected_cell(state: &mut CsvViewerState, row_count: usize, col_count: usize, dr: i32, dc: i32) {
    let col_count = col_count.max(1);
    let row_count = row_count.max(1);
    let (mut r, mut c) = state.selected_cell.unwrap_or((0, 0));
    if dr != 0 {
        r = (r as i32 + dr).clamp(0, row_count as i32 - 1) as usize;
    }
    if dc != 0 {
        c = (c as i32 + dc).clamp(0, col_count as i32 - 1) as usize;
    }
    state.selected_cell = Some((r, c));
}

/// Render a single row of CSV cells (standalone function for use by both full and lazy paths).
fn render_row_cells(
    ui: &mut Ui,
    row: &[String],
    is_header: bool,
    row_idx: usize,
    row_bg: Color32,
    colors: &CsvViewerColors,
    pixel_widths: &[f32],
    row_height: f32,
    font_size: f32,
    mut edit: Option<&mut CsvCellEditParams<'_>>,
) {
    const TABLE_LEFT_PADDING: f32 = 8.0;
    const TEXT_H_PADDING: f32 = 4.0;
    const COLUMN_COLOR_BLEND: f32 = 0.35;
    const SELECTION_OUTLINE: Color32 = Color32::from_rgb(100, 149, 237);

    let editing_enabled = edit.as_ref().map(|e| e.editing_enabled).unwrap_or(false);

    ui.add_space(TABLE_LEFT_PADDING);
    for (col_idx, cell) in row.iter().enumerate() {
        let col_width = pixel_widths
            .get(col_idx)
            .copied()
            .unwrap_or(MIN_COLUMN_WIDTH);
        let cell_width = col_width + COLUMN_PADDING;

        let font_id = egui::FontId::proportional(font_size);

        let text_color = if is_header {
            colors.header_text
        } else {
            colors.cell_text
        };

        let is_selected = editing_enabled
            && edit.as_ref().and_then(|e| e.state.selected_cell) == Some((row_idx, col_idx));
        let is_editing = editing_enabled
            && edit
                .as_ref()
                .and_then(|e| e.state.editing_cell)
                == Some((row_idx, col_idx));

        let mut cell_bg = colors.cell_background(row_bg, col_idx, COLUMN_COLOR_BLEND);
        if is_selected && !is_editing {
            cell_bg = blend_colors(cell_bg, SELECTION_OUTLINE, 0.25);
        }

        let sense = if editing_enabled {
            Sense::click()
        } else {
            Sense::hover()
        };

        let (rect, response) = ui.allocate_exact_size(Vec2::new(cell_width, row_height), sense);

        if editing_enabled {
            if let Some(params) = edit.as_mut() {
                if response.clicked() {
                    params.state.selected_cell = Some((row_idx, col_idx));
                }
                if response.double_clicked() {
                    begin_cell_edit(&mut params.state, row_idx, col_idx, cell);
                    response.request_focus();
                }
            }
        }

        if is_editing {
            if let Some(params) = edit.as_mut() {
                let edit_id = ui.id().with(("csv_cell_edit", row_idx, col_idx));
                ui.memory_mut(|mem| mem.request_focus(edit_id));

                let mut enter_pressed = false;
                let mut escape_pressed = false;
                let edit_response = ui.put(
                    rect,
                    TextEdit::singleline(&mut params.state.edit_buffer)
                        .id(edit_id)
                        .font(font_id.clone())
                        .desired_width(rect.width() - TEXT_H_PADDING * 2.0)
                        .margin(egui::vec2(TEXT_H_PADDING, 2.0)),
                );
                if edit_response.has_focus() {
                    ui.input_mut(|i| {
                        enter_pressed =
                            i.consume_key(egui::Modifiers::NONE, Key::Enter);
                        escape_pressed =
                            i.consume_key(egui::Modifiers::NONE, Key::Escape);
                    });
                }

                if enter_pressed {
                    let new_value = params.state.edit_buffer.clone();
                    queue_cell_commit(&mut params.state, row_idx, col_idx, new_value);
                } else if escape_pressed {
                    cancel_cell_edit(&mut params.state);
                } else if edit_response.lost_focus()
                    && !ui.input(|i| i.pointer.any_pressed())
                {
                    let new_value = params.state.edit_buffer.clone();
                    queue_cell_commit(&mut params.state, row_idx, col_idx, new_value);
                }
            }
        } else if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(rect, 0.0, cell_bg);

            let text_max_width = (cell_width - TEXT_H_PADDING * 2.0).max(0.0);
            let display_text =
                truncate_cell_to_pixel_width(ui, cell, &font_id, text_color, text_max_width);
            let is_truncated = display_text.as_str() != cell.as_str();

            let text_clip = egui::Rect::from_min_max(
                egui::pos2(rect.min.x + TEXT_H_PADDING, rect.min.y),
                egui::pos2(rect.max.x - TEXT_H_PADDING, rect.max.y),
            );
            let text_pos = egui::pos2(
                rect.min.x + TEXT_H_PADDING,
                rect.center().y - font_size / 2.0,
            );
            ui.painter()
                .with_clip_rect(text_clip)
                .text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &display_text,
                    font_id,
                    text_color,
                );

            if is_truncated && response.hovered() {
                let tooltip_id = if is_header {
                    egui::Id::new(("csv_header_tooltip", col_idx))
                } else {
                    egui::Id::new(("csv_tooltip", row_idx, col_idx))
                };
                egui::Tooltip::always_open(
                    ui.ctx().clone(),
                    ui.layer_id(),
                    tooltip_id,
                    egui::PopupAnchor::Pointer,
                )
                .show(|ui| {
                    ui.set_max_width(400.0);
                    ui.label(cell);
                });
            }
        }
    }
    ui.add_space(TABLE_LEFT_PADDING);
}

/// Render the table view for large files using lazy row parsing.
///
/// Instead of receiving pre-parsed `CsvData`, this function uses the row index
/// to parse only the visible rows on demand. Parsed rows are cached in
/// `state.cached_visible` and reused until the viewport scrolls beyond the
/// cached range.
fn show_table_view_lazy(
    content_bytes: &[u8],
    delimiter: u8,
    index: &CsvRowIndex,
    state: &mut CsvViewerState,
    ui: &mut Ui,
    colors: &CsvViewerColors,
    has_headers: bool,
    font_size: f32,
) -> f32 {
    const TABLE_LEFT_PADDING: f32 = 8.0;

    // Calculate pixel widths from character widths
    let char_width = font_size * 0.6;
    let pixel_widths: Vec<f32> = index
        .column_widths
        .iter()
        .map(|&w| (w as f32 * char_width).clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH))
        .collect();

    let total_width: f32 = pixel_widths.iter().sum::<f32>()
        + (index.num_columns as f32 * COLUMN_PADDING)
        + TABLE_LEFT_PADDING * 2.0;

    let row_height = font_size + 8.0;

    // Determine header row and data row count
    let (header_row, data_row_count) = if has_headers && index.row_count > 0 {
        (Some(&index.first_row), index.row_count - 1)
    } else {
        (None, index.row_count)
    };

    // Show row count info for large files
    if data_row_count > 1000 {
        ui.horizontal(|ui| {
            ui.add_space(TABLE_LEFT_PADDING);
            ui.label(phosphor_rich_text(INFO, 12.0).color(colors.truncated_indicator));
            ui.colored_label(
                colors.truncated_indicator,
                format!("{} rows total (lazy-parsed)", index.row_count),
            );
        });
        ui.add_space(4.0);
    }

    let header_height = if header_row.is_some() {
        row_height + 1.0
    } else {
        0.0
    };
    let total_content_height = header_height + (data_row_count as f32 * row_height);

    // Data rows start at this file-row offset (skip header if present)
    let data_offset = if has_headers { 1usize } else { 0usize };

    let scroll_output = ScrollArea::both()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_width(total_width);
            ui.set_min_height(total_content_height);
            ui.spacing_mut().item_spacing.y = 0.0;

            // Calculate visible data-row range from viewport
            let first_visible_row = if viewport.min.y <= header_height {
                0
            } else {
                ((viewport.min.y - header_height) / row_height).floor() as usize
            };
            let visible_row_count =
                (viewport.height() / row_height).ceil() as usize + VIRTUAL_SCROLL_BUFFER * 2;
            let last_visible_row = (first_visible_row + visible_row_count).min(data_row_count);

            let render_start = first_visible_row.saturating_sub(VIRTUAL_SCROLL_BUFFER);
            let render_end = (last_visible_row + VIRTUAL_SCROLL_BUFFER).min(data_row_count);

            // Map data-row indices to file-row indices
            let file_render_start = render_start + data_offset;
            let file_render_end = render_end + data_offset;

            // Check if cached visible rows cover the needed range
            let needs_reparse = match &state.cached_visible {
                Some(cache) => {
                    cache.start_row > file_render_start || cache.end_row < file_render_end
                }
                None => true,
            };

            if needs_reparse {
                // Parse a wider window for smooth scrolling (LAZY_CACHE_BUFFER extra on each side)
                let fetch_start = file_render_start.saturating_sub(LAZY_CACHE_BUFFER);
                let fetch_end = (file_render_end + LAZY_CACHE_BUFFER).min(index.row_count);

                let rows =
                    parse_csv_row_range(content_bytes, delimiter, index, fetch_start, fetch_end);
                state.cached_visible = Some(CachedVisibleRows {
                    start_row: fetch_start,
                    end_row: fetch_end,
                    rows,
                });
            }

            let cache = state.cached_visible.as_ref().unwrap();

            // Render header row (always at top of content)
            if let Some(header) = header_row {
                let header_bg = colors.header_bg;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    render_row_cells(
                        ui,
                        header,
                        true,
                        0,
                        header_bg,
                        colors,
                        &pixel_widths,
                        row_height,
                        font_size,
                        None,
                    );
                });
                ui.separator();
            }

            // Allocate space for rows before visible range
            if render_start > 0 {
                ui.allocate_space(Vec2::new(total_width, render_start as f32 * row_height));
            }

            // Render only visible rows from cache
            for data_row_idx in render_start..render_end {
                let file_row_idx = data_row_idx + data_offset;
                let cache_idx = file_row_idx.saturating_sub(cache.start_row);

                if let Some(row) = cache.rows.get(cache_idx) {
                    let bg_color = if data_row_idx % 2 == 0 {
                        colors.row_even_bg
                    } else {
                        colors.row_odd_bg
                    };
                    let original_row_idx = if has_headers {
                        data_row_idx + 1
                    } else {
                        data_row_idx
                    };

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        render_row_cells(
                            ui,
                            row,
                            false,
                            original_row_idx,
                            bg_color,
                            colors,
                            &pixel_widths,
                            row_height,
                            font_size,
                            None,
                        );
                    });
                }
            }

            // Allocate space for rows after visible range
            let remaining_rows = data_row_count.saturating_sub(render_end);
            if remaining_rows > 0 {
                ui.allocate_space(Vec2::new(total_width, remaining_rows as f32 * row_height));
            }
        });

    scroll_output.state.offset.y
}

/// CSV viewer widget.
pub struct CsvViewer<'a> {
    content: &'a mut String,
    file_type: TabularFileType,
    state: &'a mut CsvViewerState,
    font_size: f32,
    rainbow_columns: bool,
}

impl<'a> CsvViewer<'a> {
    pub fn new(
        content: &'a mut String,
        file_type: TabularFileType,
        state: &'a mut CsvViewerState,
    ) -> Self {
        Self {
            content,
            file_type,
            state,
            font_size: 14.0,
            rainbow_columns: false,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Enable or disable rainbow column coloring.
    pub fn rainbow_columns(mut self, enabled: bool) -> Self {
        self.rainbow_columns = enabled;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> CsvViewerOutput {
        let colors = CsvViewerColors::from_dark_mode(ui.visuals().dark_mode)
            .with_rainbow(self.rainbow_columns);

        // Determine the delimiter to use
        let (delimiter, _auto_detected) =
            if let Some(override_delim) = self.state.delimiter_override {
                // Use manual override
                (override_delim, false)
            } else if let Some(detected) = self.state.detected_delimiter {
                // Use cached detected delimiter
                (detected, true)
            } else {
                // Auto-detect from content
                let detected_info = detect_delimiter(self.content);
                self.state.set_detected_delimiter(detected_info.delimiter);
                (detected_info.delimiter, true)
            };

        // Determine header status
        let (has_headers, headers_auto_detected) = if self.state.header_override.is_some() {
            // Use manual override
            (self.state.has_headers(), false)
        } else if let Some(detected) = self.state.detected_has_header {
            // Use cached detected status
            (detected, true)
        } else {
            // Will be detected after parsing (deferred)
            (true, true) // Default to true until we parse
        };

        let mut output = CsvViewerOutput {
            toggle_raw_requested: false,
            scroll_offset: 0.0,
            has_headers,
            headers_auto_detected,
            content_changed: false,
        };

        // Large file warning
        let content_size = self.content.len();
        let is_large = content_size >= LARGE_FILE_THRESHOLD;
        let editing_enabled = csv_rendered_editing_enabled(content_size);

        if is_large && !self.state.show_raw {
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(INFO, 12.0).color(colors.truncated_indicator));
                ui.colored_label(
                    colors.truncated_indicator,
                    t!(
                        "csv.editing_disabled_large",
                        size = format!("{:.1}", content_size as f64 / 1_000_000.0)
                    )
                    .to_string(),
                );
            });
            ui.separator();
        }

        if is_large && !self.state.large_file_warning_dismissed && !self.state.show_raw {
            ui.horizontal(|ui| {
                ui.colored_label(
                    colors.error,
                    t!(
                        "csv.large_file_warning",
                        size = format!("{:.1}", content_size as f64 / 1_000_000.0)
                    )
                    .to_string(),
                );
                if ui.button(t!("common.dismiss").to_string()).clicked() {
                    self.state.large_file_warning_dismissed = true;
                }
                if ui.button(t!("csv.show_raw").to_string()).clicked() {
                    self.state.show_raw = true;
                    output.toggle_raw_requested = true;
                }
            });
            ui.separator();
        }

        // Toolbar
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.file_type.display_name()).strong());
            ui.separator();

            // Note: Raw View toggle removed - users should use view mode selector
        });
        ui.separator();

        // Content area
        if self.state.show_raw {
            output.scroll_offset = self.show_raw_view(ui);
        } else {
            let content_bytes = self.content.as_bytes();
            let content_hash = hash_content_bytes(content_bytes);
            let needs_rebuild = self.state.content_hash != content_hash;

            if is_large {
                if needs_rebuild {
                    self.state.selected_cell = None;
                    self.state.editing_cell = None;
                    self.state.edit_buffer.clear();
                    self.state.pending_commit = None;
                }
                // ━━━ LAZY PARSING PATH for large files (≥1MB) ━━━
                // Build row offset index instead of parsing all rows.
                // Only visible rows are parsed on demand during rendering.
                if needs_rebuild || self.state.cached_index.is_none() {
                    match build_csv_row_index(content_bytes, delimiter) {
                        Ok(index) => {
                            // Detect headers from first few rows
                            if self.state.detected_has_header.is_none() && index.row_count > 0 {
                                let sample_count = index.row_count.min(5);
                                let sample = parse_csv_row_range(
                                    content_bytes,
                                    delimiter,
                                    &index,
                                    0,
                                    sample_count,
                                );
                                let detected = detect_header_row(&sample);
                                self.state.set_detected_has_header(detected);
                            }
                            self.state.cached_index = Some(index);
                            self.state.cached_visible = None;
                            self.state.cached_data = None;
                            self.state.content_hash = content_hash;
                        }
                        Err(e) => {
                            self.show_parse_error(ui, &e, &colors);
                            output.scroll_offset = self.show_raw_view(ui);
                            return output;
                        }
                    }
                }

                output.has_headers = self.state.has_headers();
                output.headers_auto_detected = !self.state.has_header_override();
                let has_headers = self.state.has_headers();

                // Take index out temporarily to avoid borrow conflict with &mut state
                let index = self.state.cached_index.take().unwrap();

                output.scroll_offset = show_table_view_lazy(
                    content_bytes,
                    delimiter,
                    &index,
                    self.state,
                    ui,
                    &colors,
                    has_headers,
                    self.font_size,
                );

                // Put index back
                self.state.cached_index = Some(index);
            } else {
                if needs_rebuild {
                    self.state.selected_cell = None;
                    self.state.editing_cell = None;
                    self.state.edit_buffer.clear();
                    self.state.pending_commit = None;
                }
                // ━━━ FULL PARSE PATH for small files (<1MB) with caching ━━━
                // Parse all rows once and cache. Re-parse only when content changes.
                if needs_rebuild || self.state.cached_data.is_none() {
                    match parse_csv_with_delimiter(self.content, delimiter) {
                        Ok(data) => {
                            if self.state.detected_has_header.is_none() && !data.rows.is_empty() {
                                let detected = detect_header_row(&data.rows);
                                self.state.set_detected_has_header(detected);
                            }
                            self.state.cached_data = Some(data);
                            self.state.cached_index = None;
                            self.state.cached_visible = None;
                            self.state.content_hash = content_hash;
                        }
                        Err(e) => {
                            self.show_parse_error(ui, &e, &colors);
                            output.scroll_offset = self.show_raw_view(ui);
                            return output;
                        }
                    }
                }

                output.has_headers = self.state.has_headers();
                output.headers_auto_detected = !self.state.has_header_override();
                let has_headers = self.state.has_headers();

                // Take data out temporarily for borrow safety
                let mut data = self.state.cached_data.take().unwrap();
                output.scroll_offset = self.show_table_view(
                    ui,
                    &mut data,
                    &colors,
                    has_headers,
                    delimiter,
                    editing_enabled,
                    &mut output.content_changed,
                );
                self.state.cached_data = Some(data);
            }
        }

        output
    }

    fn show_parse_error(&self, ui: &mut Ui, error: &CsvParseError, colors: &CsvViewerColors) {
        ui.horizontal(|ui| {
            ui.colored_label(colors.error, t!("csv.error").to_string());
            ui.colored_label(colors.error, &error.message);
        });
        ui.separator();
    }

    fn show_raw_view(&mut self, ui: &mut Ui) -> f32 {
        // Hash-guarded rebuild: only allocate when content actually changes
        let current_hash = blake3_content_hash(self.content.as_bytes());
        if self.state.raw_view_hash != current_hash {
            self.state.raw_view_text = self.content.to_string();
            self.state.raw_view_hash = current_hash;
        }

        let scroll_output = ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.state.raw_view_text)
                        .code_editor()
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        scroll_output.state.offset.y
    }

    fn show_table_view(
        &mut self,
        ui: &mut Ui,
        data: &mut CsvData,
        colors: &CsvViewerColors,
        has_headers: bool,
        delimiter: u8,
        editing_enabled: bool,
        content_changed: &mut bool,
    ) -> f32 {
        // Left padding for the table
        const TABLE_LEFT_PADDING: f32 = 8.0;

        // Calculate pixel widths from character widths
        let char_width = self.font_size * 0.6; // Approximate monospace char width
        let pixel_widths: Vec<f32> = data
            .column_widths
            .iter()
            .map(|&w| (w as f32 * char_width).clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH))
            .collect();

        let total_width: f32 = pixel_widths.iter().sum::<f32>()
            + (data.num_columns as f32 * COLUMN_PADDING)
            + TABLE_LEFT_PADDING * 2.0; // Padding on both sides

        let row_height = self.font_size + 8.0;

        let has_header_row = has_headers && !data.rows.is_empty();
        let data_row_count = if has_header_row {
            data.rows.len().saturating_sub(1)
        } else {
            data.rows.len()
        };

        // Show row count info for large files
        if data_row_count > 1000 {
            ui.horizontal(|ui| {
                ui.add_space(TABLE_LEFT_PADDING);
                ui.label(phosphor_rich_text(INFO, 12.0).color(colors.truncated_indicator));
                ui.colored_label(
                    colors.truncated_indicator,
                    format!("{} rows total", data.row_count),
                );
            });
            ui.add_space(4.0);
        }

        // Blend factor for rainbow column colors (0.35 = 35% column color, 65% base)
        const _COLUMN_COLOR_BLEND: f32 = 0.35;

        // Calculate total content height for virtual scrolling
        let header_height = if has_header_row {
            row_height + 1.0
        } else {
            0.0
        }; // +1 for separator
        let total_content_height = header_height + (data_row_count as f32 * row_height);

        let table_focus_id = ui.id().with("csv_table_focus");

        // Use show_viewport for virtual scrolling
        let scroll_output = ScrollArea::both()
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                if editing_enabled && self.state.editing_cell.is_none() {
                    let table_rect = egui::Rect::from_min_size(
                        viewport.min,
                        egui::vec2(viewport.width(), viewport.height()),
                    );
                    let table_response =
                        ui.interact(table_rect, table_focus_id, Sense::click());
                    if table_response.clicked() {
                        ui.memory_mut(|mem| mem.request_focus(table_focus_id));
                    }

                    if ui.memory(|mem| mem.has_focus(table_focus_id)) {
                        if let Some((row, col)) = self.state.selected_cell {
                            if ui.input(|i| i.key_pressed(Key::Enter)) {
                                let value = data
                                    .rows
                                    .get(row)
                                    .and_then(|r| r.get(col))
                                    .map(|s| s.as_str())
                                    .unwrap_or("");
                                begin_cell_edit(&mut self.state, row, col, value);
                            }
                        }

                        let row_count = data.row_count;
                        let col_count = data.num_columns;
                        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            move_selected_cell(&mut self.state, row_count, col_count, -1, 0);
                        }
                        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            move_selected_cell(&mut self.state, row_count, col_count, 1, 0);
                        }
                        if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
                            move_selected_cell(&mut self.state, row_count, col_count, 0, -1);
                        }
                        if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
                            move_selected_cell(&mut self.state, row_count, col_count, 0, 1);
                        }
                    }
                }
                // Set minimum content dimensions for full scrolling range
                ui.set_min_width(total_width);
                ui.set_min_height(total_content_height);
                ui.spacing_mut().item_spacing.y = 0.0;

                // Calculate visible row range based on viewport
                // viewport.min.y is the scroll offset in content coordinates
                let first_visible_row = if viewport.min.y <= header_height {
                    0
                } else {
                    ((viewport.min.y - header_height) / row_height).floor() as usize
                };

                let visible_row_count =
                    (viewport.height() / row_height).ceil() as usize + VIRTUAL_SCROLL_BUFFER * 2;
                let last_visible_row = (first_visible_row + visible_row_count).min(data_row_count);

                // Apply buffer but clamp to valid range
                let render_start = first_visible_row.saturating_sub(VIRTUAL_SCROLL_BUFFER);
                let render_end = (last_visible_row + VIRTUAL_SCROLL_BUFFER).min(data_row_count);

                let mut edit_params = if editing_enabled {
                    Some(CsvCellEditParams {
                        state: &mut self.state,
                        editing_enabled: true,
                    })
                } else {
                    None
                };

                // Render header row (always at top of content)
                if has_header_row {
                    let header_bg = colors.header_bg;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        render_row_cells(
                            ui,
                            &data.rows[0],
                            true,
                            0,
                            header_bg,
                            colors,
                            &pixel_widths,
                            row_height,
                            self.font_size,
                            edit_params.as_mut(),
                        );
                    });
                    ui.separator();
                }

                // Add space for rows before visible range (virtual scrolling skip)
                if render_start > 0 {
                    ui.allocate_space(Vec2::new(total_width, render_start as f32 * row_height));
                }

                // Render only visible rows
                for row_idx in render_start..render_end {
                    let file_row_idx = if has_header_row {
                        row_idx + 1
                    } else {
                        row_idx
                    };

                    let bg_color = if row_idx % 2 == 0 {
                        colors.row_even_bg
                    } else {
                        colors.row_odd_bg
                    };

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        render_row_cells(
                            ui,
                            &data.rows[file_row_idx],
                            false,
                            file_row_idx,
                            bg_color,
                            colors,
                            &pixel_widths,
                            row_height,
                            self.font_size,
                            edit_params.as_mut(),
                        );
                    });
                }

                // Add space for rows after visible range
                let remaining_rows = data_row_count.saturating_sub(render_end);
                if remaining_rows > 0 {
                    ui.allocate_space(Vec2::new(total_width, remaining_rows as f32 * row_height));
                }
            });

        apply_pending_cell_commit(
            &mut self.state,
            data,
            self.content,
            delimiter,
            content_changed,
        );

        scroll_output.state.offset.y
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Truncate cell content for display, respecting UTF-8 boundaries.
fn truncate_cell(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Measure single-line text width in pixels using egui font metrics.
fn measure_text_width(ui: &Ui, text: &str, font_id: &egui::FontId, color: Color32) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font_id.clone(), color)
        .size()
        .x
}

/// Truncate cell text to fit a pixel width, with ellipsis when needed.
///
/// Applies [`MAX_CELL_CHARS`] first so pathological cells do not require measuring
/// every character.
fn truncate_cell_to_pixel_width(
    ui: &Ui,
    cell: &str,
    font_id: &egui::FontId,
    color: Color32,
    max_width: f32,
) -> String {
    if max_width <= 0.0 {
        return String::new();
    }

    let capped = truncate_cell(cell, MAX_CELL_CHARS);
    if measure_text_width(ui, &capped, font_id, color) <= max_width {
        return capped;
    }

    const ELLIPSIS: &str = "...";
    let ellipsis_width = measure_text_width(ui, ELLIPSIS, font_id, color);
    let available = max_width - ellipsis_width;
    if available <= 0.0 {
        return ELLIPSIS.to_string();
    }

    let chars: Vec<char> = capped.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        let candidate: String = chars.iter().take(mid).collect();
        if measure_text_width(ui, &candidate, font_id, color) <= available {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    format!("{}{}", chars.iter().take(lo).collect::<String>(), ELLIPSIS)
}

/// Check if a file path is a tabular file (CSV or TSV).
#[allow(dead_code)] // Public API for file type detection
pub fn is_tabular_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(TabularFileType::from_extension)
        .is_some()
}

/// Get the tabular file type from a path.
pub fn get_tabular_file_type(path: &Path) -> Option<TabularFileType> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(TabularFileType::from_extension)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            TabularFileType::from_extension("csv"),
            Some(TabularFileType::Csv)
        );
        assert_eq!(
            TabularFileType::from_extension("CSV"),
            Some(TabularFileType::Csv)
        );
        assert_eq!(
            TabularFileType::from_extension("tsv"),
            Some(TabularFileType::Tsv)
        );
        assert_eq!(
            TabularFileType::from_extension("TSV"),
            Some(TabularFileType::Tsv)
        );
        assert_eq!(TabularFileType::from_extension("json"), None);
    }

    #[test]
    fn test_delimiter() {
        assert_eq!(TabularFileType::Csv.delimiter(), b',');
        assert_eq!(TabularFileType::Tsv.delimiter(), b'\t');
    }

    #[test]
    fn test_parse_simple_csv() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let result = parse_csv(csv, TabularFileType::Csv);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.num_columns, 3);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0], vec!["name", "age", "city"]);
        assert_eq!(data.rows[1], vec!["Alice", "30", "NYC"]);
        assert_eq!(data.rows[2], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn test_parse_simple_tsv() {
        let tsv = "name\tage\tcity\nAlice\t30\tNYC\nBob\t25\tLA";
        let result = parse_csv(tsv, TabularFileType::Tsv);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.num_columns, 3);
        assert_eq!(data.rows.len(), 3);
    }

    #[test]
    fn test_parse_csv_with_quotes() {
        let csv = r#"name,description
"Alice","Has a comma, in description"
"Bob","Normal description""#;
        let result = parse_csv(csv, TabularFileType::Csv);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.rows[1][1], "Has a comma, in description");
    }

    #[test]
    fn test_parse_csv_flexible_columns() {
        let csv = "a,b,c\n1,2\n3,4,5,6";
        let result = parse_csv(csv, TabularFileType::Csv);
        assert!(result.is_ok());

        let data = result.unwrap();
        // Should normalize to max columns (4)
        assert_eq!(data.num_columns, 4);
        // Row with fewer columns should be padded
        assert_eq!(data.rows[1].len(), 4);
    }

    #[test]
    fn test_truncate_cell() {
        assert_eq!(truncate_cell("short", 10), "short");
        assert_eq!(
            truncate_cell("this is a very long string", 10),
            "this is..."
        );
    }

    #[test]
    fn test_is_tabular_file() {
        assert!(is_tabular_file(Path::new("data.csv")));
        assert!(is_tabular_file(Path::new("data.tsv")));
        assert!(!is_tabular_file(Path::new("data.json")));
        assert!(!is_tabular_file(Path::new("readme.md")));
    }

    #[test]
    fn test_empty_csv() {
        let csv = "";
        let result = parse_csv(csv, TabularFileType::Csv);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.rows.len(), 0);
        assert_eq!(data.num_columns, 0);
    }

    #[test]
    fn test_single_column_csv() {
        let csv = "name\nAlice\nBob";
        let result = parse_csv(csv, TabularFileType::Csv);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.num_columns, 1);
        assert_eq!(data.rows.len(), 3);
    }

    #[test]
    fn test_detect_delimiter_comma() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let info = detect_delimiter(csv);
        assert_eq!(info.delimiter, b',');
    }

    #[test]
    fn test_detect_delimiter_tab() {
        let tsv = "name\tage\tcity\nAlice\t30\tNYC\nBob\t25\tLA";
        let info = detect_delimiter(tsv);
        assert_eq!(info.delimiter, b'\t');
    }

    #[test]
    fn test_detect_delimiter_semicolon() {
        let csv = "name;age;city\nAlice;30;NYC\nBob;25;LA";
        let info = detect_delimiter(csv);
        assert_eq!(info.delimiter, b';');
    }

    #[test]
    fn test_detect_delimiter_pipe() {
        let csv = "name|age|city\nAlice|30|NYC\nBob|25|LA";
        let info = detect_delimiter(csv);
        assert_eq!(info.delimiter, b'|');
    }

    #[test]
    fn test_detect_delimiter_empty() {
        let csv = "";
        let info = detect_delimiter(csv);
        // Default to comma for empty content
        assert_eq!(info.delimiter, b',');
    }

    #[test]
    fn test_delimiter_display_names() {
        assert_eq!(delimiter_display_name(b','), "Comma");
        assert_eq!(delimiter_display_name(b'\t'), "Tab");
        assert_eq!(delimiter_display_name(b';'), "Semicolon");
        assert_eq!(delimiter_display_name(b'|'), "Pipe");
    }

    #[test]
    fn test_delimiter_symbols() {
        assert_eq!(delimiter_symbol(b','), ",");
        assert_eq!(delimiter_symbol(b'\t'), "⇥");
        assert_eq!(delimiter_symbol(b';'), ";");
        assert_eq!(delimiter_symbol(b'|'), "|");
    }

    #[test]
    fn test_csv_viewer_state_delimiter() {
        let mut state = CsvViewerState::new();

        // Initially no delimiter
        assert!(state.effective_delimiter().is_none());
        assert!(!state.has_delimiter_override());

        // Set detected delimiter
        state.set_detected_delimiter(b',');
        assert_eq!(state.effective_delimiter(), Some(b','));
        assert!(!state.has_delimiter_override());

        // Set override
        state.set_delimiter(b';');
        assert_eq!(state.effective_delimiter(), Some(b';'));
        assert!(state.has_delimiter_override());

        // Clear override
        state.clear_delimiter_override();
        assert_eq!(state.effective_delimiter(), Some(b','));
        assert!(!state.has_delimiter_override());
    }

    #[test]
    fn test_parse_csv_with_delimiter() {
        let content = "name;age;city\nAlice;30;NYC";

        // Parse with semicolon delimiter
        let result = parse_csv_with_delimiter(content, b';');
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.num_columns, 3);
        assert_eq!(data.rows[0], vec!["name", "age", "city"]);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Header Detection Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_detect_header_row_clear_headers() {
        // Clear text headers with numeric data
        let rows = vec![
            vec!["name".to_string(), "age".to_string(), "city".to_string()],
            vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
            vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
        ];
        assert!(detect_header_row(&rows));
    }

    #[test]
    fn test_detect_header_row_numeric_first_row() {
        // First row is all numbers - likely not a header
        let rows = vec![
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
            vec!["4".to_string(), "5".to_string(), "6".to_string()],
            vec!["7".to_string(), "8".to_string(), "9".to_string()],
        ];
        assert!(!detect_header_row(&rows));
    }

    #[test]
    fn test_detect_header_row_keyword_headers() {
        // Headers contain common keywords
        let rows = vec![
            vec![
                "id".to_string(),
                "email".to_string(),
                "created_date".to_string(),
            ],
            vec![
                "1".to_string(),
                "alice@test.com".to_string(),
                "2024-01-15".to_string(),
            ],
            vec![
                "2".to_string(),
                "bob@test.com".to_string(),
                "2024-01-16".to_string(),
            ],
        ];
        assert!(detect_header_row(&rows));
    }

    #[test]
    fn test_detect_header_row_single_row() {
        // Only one row - default to treating as header
        let rows = vec![vec![
            "name".to_string(),
            "age".to_string(),
            "city".to_string(),
        ]];
        assert!(detect_header_row(&rows));
    }

    #[test]
    fn test_detect_header_row_empty() {
        // Empty data - should return default (true)
        let rows: Vec<Vec<String>> = vec![];
        assert!(detect_header_row(&rows));
    }

    #[test]
    fn test_detect_header_row_mixed_content() {
        // Mixed first row with some numbers - might still be headers
        let rows = vec![
            vec![
                "Product".to_string(),
                "Price".to_string(),
                "Quantity".to_string(),
            ],
            vec!["Apple".to_string(), "1.50".to_string(), "100".to_string()],
            vec!["Banana".to_string(), "0.75".to_string(), "200".to_string()],
        ];
        assert!(detect_header_row(&rows));
    }

    #[test]
    fn test_is_numeric_value() {
        // Integer
        assert!(is_numeric_value("123"));
        assert!(is_numeric_value("-456"));

        // Float
        assert!(is_numeric_value("123.45"));
        assert!(is_numeric_value("-456.78"));

        // Currency
        assert!(is_numeric_value("$100"));
        assert!(is_numeric_value("€50.00"));

        // Percentage
        assert!(is_numeric_value("50%"));

        // With thousands separator
        assert!(is_numeric_value("1,234"));
        assert!(is_numeric_value("1,234.56"));

        // Non-numeric
        assert!(!is_numeric_value("hello"));
        assert!(!is_numeric_value("abc123"));
        assert!(!is_numeric_value(""));
    }

    #[test]
    fn test_csv_viewer_state_header() {
        let mut state = CsvViewerState::new();

        // Initially defaults to true (no detection yet)
        assert!(state.has_headers());
        assert!(!state.has_header_override());
        assert!(state.detected_has_header().is_none());

        // Set detected header status
        state.set_detected_has_header(true);
        assert!(state.has_headers());
        assert!(!state.has_header_override());

        // Toggle header
        state.toggle_header();
        assert!(!state.has_headers());
        assert!(state.has_header_override());

        // Set explicit override
        state.set_header_override(true);
        assert!(state.has_headers());
        assert!(state.has_header_override());

        // Clear override - returns to detected value
        state.clear_header_override();
        assert!(state.has_headers()); // detected was true
        assert!(!state.has_header_override());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Rainbow Column Color Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_generate_rainbow_colors_dark_mode() {
        let colors = generate_rainbow_colors(true);

        // Should generate 12 colors (RAINBOW_PALETTE_SIZE)
        assert_eq!(colors.len(), 12);

        // All colors should be valid (non-zero alpha)
        for color in &colors {
            assert_eq!(color.a(), 255, "All colors should have full alpha");
        }

        // Colors should be distinct (check hue variation)
        let first = colors[0];
        let sixth = colors[6]; // Opposite side of color wheel (180°)
        assert_ne!(first, sixth, "Colors at 0° and 180° should be different");
    }

    #[test]
    fn test_generate_rainbow_colors_light_mode() {
        let colors = generate_rainbow_colors(false);

        // Should generate 12 colors
        assert_eq!(colors.len(), 12);

        // Light mode colors should be lighter than dark mode
        let dark_colors = generate_rainbow_colors(true);

        // Compare average brightness (using simple RGB average)
        let light_avg: f32 = colors
            .iter()
            .map(|c| (c.r() as f32 + c.g() as f32 + c.b() as f32) / 3.0)
            .sum::<f32>()
            / colors.len() as f32;
        let dark_avg: f32 = dark_colors
            .iter()
            .map(|c| (c.r() as f32 + c.g() as f32 + c.b() as f32) / 3.0)
            .sum::<f32>()
            / dark_colors.len() as f32;

        assert!(
            light_avg > dark_avg,
            "Light mode colors should be brighter than dark mode"
        );
    }

    #[test]
    fn test_blend_colors() {
        let white = Color32::WHITE;
        let black = Color32::BLACK;

        // 0% blend = base color
        let result = blend_colors(white, black, 0.0);
        assert_eq!(result, white);

        // 100% blend = overlay color
        let result = blend_colors(white, black, 1.0);
        assert_eq!(result, black);

        // 50% blend = midpoint
        let result = blend_colors(white, black, 0.5);
        assert_eq!(result.r(), 127);
        assert_eq!(result.g(), 127);
        assert_eq!(result.b(), 127);
    }

    #[test]
    fn test_blend_colors_clamping() {
        let color1 = Color32::from_rgb(100, 100, 100);
        let color2 = Color32::from_rgb(200, 200, 200);

        // Factor > 1.0 should be clamped to 1.0
        let result = blend_colors(color1, color2, 2.0);
        assert_eq!(result, color2);

        // Factor < 0.0 should be clamped to 0.0
        let result = blend_colors(color1, color2, -1.0);
        assert_eq!(result, color1);
    }

    #[test]
    fn test_csv_viewer_colors_with_rainbow() {
        let colors = CsvViewerColors::dark().with_rainbow(true);
        assert!(colors.rainbow_enabled);
        assert!(!colors.column_colors.is_empty());

        let colors = CsvViewerColors::light().with_rainbow(false);
        assert!(!colors.rainbow_enabled);
    }

    #[test]
    fn test_cell_background_rainbow_disabled() {
        let colors = CsvViewerColors::dark().with_rainbow(false);
        let base = Color32::from_rgb(50, 50, 50);

        // With rainbow disabled, cell_background should return base color unchanged
        let result = colors.cell_background(base, 0, 0.5);
        assert_eq!(result, base);

        let result = colors.cell_background(base, 5, 0.5);
        assert_eq!(result, base);
    }

    #[test]
    fn test_cell_background_rainbow_enabled() {
        let colors = CsvViewerColors::dark().with_rainbow(true);
        let base = Color32::from_rgb(50, 50, 50);

        // With rainbow enabled, cell_background should blend with column color
        let result0 = colors.cell_background(base, 0, 0.5);
        let result6 = colors.cell_background(base, 6, 0.5);

        // Different columns should produce different results
        assert_ne!(
            result0, result6,
            "Different columns should have different colors"
        );

        // Result should be different from base
        assert_ne!(result0, base, "Blended color should differ from base");
    }

    #[test]
    fn test_cell_background_column_cycling() {
        let colors = CsvViewerColors::dark().with_rainbow(true);
        let base = Color32::from_rgb(50, 50, 50);

        // Column colors should cycle after 12 columns
        let result0 = colors.cell_background(base, 0, 0.5);
        let result12 = colors.cell_background(base, 12, 0.5);

        assert_eq!(
            result0, result12,
            "Column 0 and 12 should have same color (cycling)"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Virtual Scrolling / Large File Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_large_csv_parsing() {
        // Generate a 10,000 row CSV in memory
        let mut csv = String::with_capacity(500_000);
        csv.push_str("id,name,value\n");
        for i in 0..10_000 {
            csv.push_str(&format!("{},Name{},{:.2}\n", i, i, i as f64 * 1.5));
        }

        let result = parse_csv_with_delimiter(&csv, b',');
        assert!(result.is_ok());

        let data = result.unwrap();
        // Should parse ALL rows (no truncation with virtual scrolling)
        assert_eq!(data.row_count, 10_001); // header + 10,000 data rows
        assert_eq!(data.rows.len(), 10_001);
        assert_eq!(data.num_columns, 3);
    }

    #[test]
    fn test_large_csv_no_truncation() {
        // Verify that we parse more than the old MAX_DISPLAY_ROWS limit
        let mut csv = String::with_capacity(1_500_000);
        csv.push_str("a,b,c\n");
        for i in 0..15_000 {
            csv.push_str(&format!("{},{},{}\n", i, i * 2, i * 3));
        }

        let result = parse_csv_with_delimiter(&csv, b',');
        assert!(result.is_ok());

        let data = result.unwrap();
        // All 15,001 rows should be parsed (not truncated at 10,000)
        assert_eq!(data.row_count, 15_001);
        assert_eq!(data.rows.len(), 15_001);
    }

    #[test]
    fn test_column_width_sampling() {
        // Column widths should be calculated from sample (first 1000 rows)
        // but all rows should still be parsed
        let mut csv = String::with_capacity(200_000);
        csv.push_str("col1,col2\n");

        // First 500 rows with short values
        for i in 0..500 {
            csv.push_str(&format!("{},x\n", i));
        }
        // Next 500 rows with longer values (still in sample)
        for i in 500..1000 {
            csv.push_str(&format!("{},longer_value_here\n", i));
        }
        // Remaining rows with even longer values (outside sample)
        for i in 1000..2000 {
            csv.push_str(&format!(
                "{},this_is_a_very_long_value_that_exceeds_sample\n",
                i
            ));
        }

        let result = parse_csv_with_delimiter(&csv, b',');
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.row_count, 2001);
        assert_eq!(data.rows.len(), 2001);
        // Column widths should reflect the sample (first 1000 rows)
        assert!(data.column_widths[1] > 0);
        // The very long values outside sample shouldn't affect column width
        assert!(data.column_widths[1] <= MAX_CELL_CHARS);
    }

    #[test]
    fn test_virtual_scroll_buffer_constant() {
        // Verify the virtual scroll buffer is set appropriately
        assert_eq!(VIRTUAL_SCROLL_BUFFER, 5);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Lazy CSV Parsing (byte-offset indexing) Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_build_row_index_simple() {
        let csv = b"name,age,city\nAlice,30,NYC\nBob,25,LA";
        let index = build_csv_row_index(csv, b',').unwrap();

        assert_eq!(index.row_count, 3);
        assert_eq!(index.num_columns, 3);
        assert_eq!(index.row_offsets.len(), 3);
        assert_eq!(index.first_row, vec!["name", "age", "city"]);
        // First row starts at byte 0
        assert_eq!(index.row_offsets[0], 0);
        // Column widths should be reasonable
        assert!(index.column_widths.len() >= 3);
    }

    #[test]
    fn test_build_row_index_tsv() {
        let tsv = b"name\tage\tcity\nAlice\t30\tNYC\nBob\t25\tLA";
        let index = build_csv_row_index(tsv, b'\t').unwrap();

        assert_eq!(index.row_count, 3);
        assert_eq!(index.num_columns, 3);
        assert_eq!(index.first_row, vec!["name", "age", "city"]);
    }

    #[test]
    fn test_build_row_index_empty() {
        let csv = b"";
        let index = build_csv_row_index(csv, b',').unwrap();

        assert_eq!(index.row_count, 0);
        assert_eq!(index.num_columns, 0);
        assert!(index.row_offsets.is_empty());
    }

    #[test]
    fn test_build_row_index_large() {
        // Build index for 10,000 rows
        let mut csv = String::with_capacity(500_000);
        csv.push_str("id,name,value\n");
        for i in 0..10_000 {
            csv.push_str(&format!("{},Name{},{:.2}\n", i, i, i as f64 * 1.5));
        }

        let index = build_csv_row_index(csv.as_bytes(), b',').unwrap();

        assert_eq!(index.row_count, 10_001); // header + 10,000 data rows
        assert_eq!(index.num_columns, 3);
        assert_eq!(index.row_offsets.len(), 10_001);
        assert_eq!(index.first_row, vec!["id", "name", "value"]);
    }

    #[test]
    fn test_parse_row_range_full() {
        let csv = b"name,age,city\nAlice,30,NYC\nBob,25,LA";
        let index = build_csv_row_index(csv, b',').unwrap();

        // Parse all rows
        let rows = parse_csv_row_range(csv, b',', &index, 0, 3);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["name", "age", "city"]);
        assert_eq!(rows[1], vec!["Alice", "30", "NYC"]);
        assert_eq!(rows[2], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn test_parse_row_range_partial() {
        let csv = b"name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,SF";
        let index = build_csv_row_index(csv, b',').unwrap();

        // Parse only rows 1-2 (Alice and Bob, skipping header)
        let rows = parse_csv_row_range(csv, b',', &index, 1, 3);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Alice", "30", "NYC"]);
        assert_eq!(rows[1], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn test_parse_row_range_last_row() {
        let csv = b"a,b\n1,2\n3,4\n5,6";
        let index = build_csv_row_index(csv, b',').unwrap();

        // Parse only the last row
        let rows = parse_csv_row_range(csv, b',', &index, 3, 4);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], vec!["5", "6"]);
    }

    #[test]
    fn test_parse_row_range_empty() {
        let csv = b"a,b\n1,2\n3,4";
        let index = build_csv_row_index(csv, b',').unwrap();

        // Empty range
        let rows = parse_csv_row_range(csv, b',', &index, 2, 2);
        assert!(rows.is_empty());

        // Out of bounds
        let rows = parse_csv_row_range(csv, b',', &index, 10, 20);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_parse_row_range_with_quotes() {
        let csv = br#"name,desc
"Alice","Has a comma, here"
"Bob","Normal"
"Charlie","Quotes ""inside"""
"#;
        let index = build_csv_row_index(csv, b',').unwrap();
        assert_eq!(index.row_count, 4);

        // Parse row 1 (Alice)
        let rows = parse_csv_row_range(csv, b',', &index, 1, 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "Alice");
        assert_eq!(rows[0][1], "Has a comma, here");

        // Parse row 3 (Charlie with escaped quotes)
        let rows = parse_csv_row_range(csv, b',', &index, 3, 4);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "Charlie");
        assert_eq!(rows[0][1], "Quotes \"inside\"");
    }

    #[test]
    fn test_parse_row_range_large_dataset() {
        // Build a 5000-row CSV
        let mut csv = String::with_capacity(200_000);
        csv.push_str("id,value\n");
        for i in 0..5000 {
            csv.push_str(&format!("{},{}\n", i, i * 10));
        }

        let content = csv.as_bytes();
        let index = build_csv_row_index(content, b',').unwrap();
        assert_eq!(index.row_count, 5001);

        // Parse rows 2500-2510 (middle of file)
        let rows = parse_csv_row_range(content, b',', &index, 2500, 2510);
        assert_eq!(rows.len(), 10);
        // Row 2500 (0-indexed) = data row 2499 (since row 0 is header)
        assert_eq!(rows[0][0], "2499");
        assert_eq!(rows[0][1], "24990");
    }

    #[test]
    fn test_row_index_column_widths_sampled() {
        // Column widths should be sampled from first 1000 rows
        let mut csv = String::with_capacity(200_000);
        csv.push_str("col1,col2\n");
        for i in 0..500 {
            csv.push_str(&format!("{},x\n", i));
        }
        for i in 500..1000 {
            csv.push_str(&format!("{},longer_value_here\n", i));
        }
        for i in 1000..2000 {
            csv.push_str(&format!(
                "{},this_is_a_very_long_value_that_exceeds_sample\n",
                i
            ));
        }

        let index = build_csv_row_index(csv.as_bytes(), b',').unwrap();
        assert_eq!(index.row_count, 2001);
        // Column widths sampled from first 1000 rows, capped at MAX_CELL_CHARS
        assert!(index.column_widths[1] > 0);
        assert!(index.column_widths[1] <= MAX_CELL_CHARS);
    }

    #[test]
    fn test_row_index_flexible_columns() {
        let csv = b"a,b,c\n1,2\n3,4,5,6";
        let index = build_csv_row_index(csv, b',').unwrap();

        // Should detect max column count (4)
        assert_eq!(index.num_columns, 4);
        // First row should be normalized to 4 columns
        assert_eq!(index.first_row.len(), 4);
    }

    #[test]
    fn test_hash_content_bytes_consistency() {
        let content = b"some csv content";
        let hash1 = hash_content_bytes(content);
        let hash2 = hash_content_bytes(content);
        assert_eq!(hash1, hash2);

        let different = b"different content";
        let hash3 = hash_content_bytes(different);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_content_bytes_large() {
        // Large content uses sampling (beginning + end)
        let content = vec![b'x'; 20_000];
        let hash1 = hash_content_bytes(&content);

        let mut modified = content.clone();
        modified[10_000] = b'y'; // Modify the middle (outside sample)
        let hash2 = hash_content_bytes(&modified);
        // Middle change might not be detected (by design for speed)
        // This is acceptable — the hash is for fast invalidation, not cryptographic

        // But changing the beginning or end should produce different hash
        let mut modified_start = content.clone();
        modified_start[100] = b'z';
        let hash3 = hash_content_bytes(&modified_start);
        assert_ne!(hash1, hash3);

        let mut modified_end = content.clone();
        modified_end[19_900] = b'z';
        let hash4 = hash_content_bytes(&modified_end);
        assert_ne!(hash1, hash4);
    }

    #[test]
    fn test_lazy_cache_buffer_constant() {
        assert_eq!(LAZY_CACHE_BUFFER, 50);
    }

    #[test]
    fn test_csv_viewer_state_lazy_cache_invalidation() {
        let mut state = CsvViewerState::new();

        // Simulate building a cached index
        state.cached_index = Some(CsvRowIndex {
            row_offsets: vec![0, 10, 20],
            row_count: 3,
            num_columns: 2,
            column_widths: vec![5, 5],
            first_row: vec!["a".to_string(), "b".to_string()],
        });
        state.cached_visible = Some(CachedVisibleRows {
            start_row: 0,
            end_row: 3,
            rows: vec![vec!["1".to_string(), "2".to_string()]],
        });
        state.content_hash = 12345;

        // Invalidate cache should clear everything
        state.invalidate_cache();
        assert!(state.cached_index.is_none());
        assert!(state.cached_visible.is_none());
        assert_eq!(state.content_hash, 0);
    }

    #[test]
    fn test_csv_viewer_state_delimiter_clears_lazy_cache() {
        let mut state = CsvViewerState::new();
        state.cached_index = Some(CsvRowIndex {
            row_offsets: vec![0],
            row_count: 1,
            num_columns: 1,
            column_widths: vec![3],
            first_row: vec!["a".to_string()],
        });

        // Setting delimiter should clear lazy cache
        state.set_delimiter(b';');
        assert!(state.cached_index.is_none());
        assert!(state.cached_visible.is_none());
    }

    #[test]
    fn test_blake3_content_hash_consistency() {
        let content = b"some csv content";
        let hash1 = blake3_content_hash(content);
        let hash2 = blake3_content_hash(content);
        assert_eq!(hash1, hash2);

        let different = b"different content";
        let hash3 = blake3_content_hash(different);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_blake3_content_hash_detects_mid_file_edits() {
        let content = vec![b'x'; 20_000];
        let hash1 = blake3_content_hash(&content);

        let mut modified = content.clone();
        modified[10_000] = b'y';
        let hash2 = blake3_content_hash(&modified);
        // blake3 hashes the full content, so mid-file edits are always detected
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_invalidate_cache_clears_raw_view() {
        let mut state = CsvViewerState::new();
        state.raw_view_text = "cached text".to_string();
        state.raw_view_hash = 12345;

        state.invalidate_cache();
        assert!(state.raw_view_text.is_empty());
        assert_eq!(state.raw_view_hash, 0);
    }

    #[test]
    fn test_set_delimiter_clears_raw_view() {
        let mut state = CsvViewerState::new();
        state.raw_view_text = "cached text".to_string();
        state.raw_view_hash = 12345;

        state.set_delimiter(b';');
        assert!(state.raw_view_text.is_empty());
        assert_eq!(state.raw_view_hash, 0);
    }

    #[test]
    fn test_csv_rendered_editing_enabled_threshold() {
        assert!(csv_rendered_editing_enabled(LARGE_FILE_THRESHOLD - 1));
        assert!(!csv_rendered_editing_enabled(LARGE_FILE_THRESHOLD));
        assert!(!csv_rendered_editing_enabled(LARGE_FILE_THRESHOLD + 1));
    }

    #[test]
    fn test_set_cell_and_serialize_roundtrip() {
        let csv = "name,age\nAlice,30\nBob,25";
        let mut data = parse_csv_with_delimiter(csv, b',').unwrap();
        data.set_cell(1, 1, "31".to_string());
        let serialized = serialize_csv_rows(&data.rows, b',').unwrap();
        let reparsed = parse_csv_with_delimiter(&serialized, b',').unwrap();
        assert_eq!(reparsed.rows[1][1], "31");
    }

    #[test]
    fn test_serialize_preserves_quotes_and_commas() {
        let rows = vec![
            vec!["name".to_string(), "desc".to_string()],
            vec![
                "Alice".to_string(),
                "Has a comma, in field".to_string(),
            ],
        ];
        let serialized = serialize_csv_rows(&rows, b',').unwrap();
        let reparsed = parse_csv_with_delimiter(&serialized, b',').unwrap();
        assert_eq!(reparsed.rows[1][1], "Has a comma, in field");
    }
}
