//! HarfRust (`harfrust`) OpenType shaping for FerriteEditor.
//!
//! Converts `&str` into a glyph run with per-glyph advances and cluster mapping
//! (UTF-8 byte indices), using the same font bytes egui loads for the editor.
//!
//! **Rendering:** egui 0.28 still builds [`egui::Galley`] via `ab_glyph` without
//! full OTL shaping. This module is the foundation for a follow-up task that will
//! cache shaped runs and align cursor/hit-testing with harfrust output.
//! [`crate::editor::ferrite::line_cache::LineCache`] calls [`shape_text`] on
//! complex-script lines before galley creation to exercise the pipeline and surface
//! parse/shaping failures early (see `RUST_LOG=ferrite::shaping`).

use harfrust::{BufferFlags, FontRef, ShaperData, UnicodeBuffer};
use std::fmt;
use unicode_script::UnicodeScript;

/// One glyph after shaping, with advances in **points** (same unit as [`egui::FontId::size`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// OpenType glyph ID in the shaped font.
    pub glyph_id: u32,
    /// Start of the source cluster in the original string, as a UTF-8 **byte** index.
    pub cluster: u32,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

/// Failure to parse the font or read metrics required for scaling.
#[derive(Debug, Clone)]
pub enum ShapeError {
    Font(String),
    InvalidUnitsPerEm,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShapeError::Font(s) => write!(f, "font: {s}"),
            ShapeError::InvalidUnitsPerEm => write!(f, "invalid units-per-EM"),
        }
    }
}

impl std::error::Error for ShapeError {}

/// Shape `text` with HarfRust using the given TrueType/OpenType bytes.
///
/// Positions are scaled from font units using `font_size_pt / units_per_em`.
/// On success, each output glyph includes HarfBuzz-style `cluster` byte offsets.
///
/// Returns an empty vector for empty `text`. For `font_size_pt <= 0.0`, returns `Ok(vec![])`.
pub fn shape_text(
    text: &str,
    font_bytes: &[u8],
    font_size_pt: f32,
) -> Result<Vec<ShapedGlyph>, ShapeError> {
    if text.is_empty() || font_size_pt <= 0.0 {
        return Ok(Vec::new());
    }

    let font = FontRef::new(font_bytes).map_err(|e| ShapeError::Font(e.to_string()))?;

    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let upe = shaper.units_per_em();
    if upe <= 0 {
        return Err(ShapeError::InvalidUnitsPerEm);
    }
    let scale = font_size_pt / upe as f32;

    let mut buffer = UnicodeBuffer::new();
    buffer.set_flags(BufferFlags::BEGINNING_OF_TEXT | BufferFlags::END_OF_TEXT);
    buffer.push_str(text);
    buffer.guess_segment_properties();
    if buffer.script() == harfrust::script::UNKNOWN {
        hint_script_from_first_strong_char(text, &mut buffer);
    }

    let output = shaper.shape(buffer, &[]);
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();

    let mut glyphs = Vec::with_capacity(infos.len());
    for (info, pos) in infos.iter().zip(positions.iter()) {
        glyphs.push(ShapedGlyph {
            glyph_id: info.glyph_id,
            cluster: info.cluster,
            x_advance: pos.x_advance as f32 * scale,
            x_offset: pos.x_offset as f32 * scale,
            y_offset: pos.y_offset as f32 * scale,
        });
    }
    Ok(glyphs)
}

/// Sum of horizontal advances (points), useful for width estimates.
#[must_use]
pub fn shaped_width(glyphs: &[ShapedGlyph]) -> f32 {
    glyphs.iter().map(|g| g.x_advance).sum()
}

/// A visual cluster: one or more shaped glyphs sharing a source-text byte range.
///
/// Clusters are in **visual order** (left-to-right on screen regardless of text direction).
#[derive(Debug, Clone)]
pub struct ShapedCluster {
    pub byte_start: usize,
    pub byte_end: usize,
    /// Combined horizontal advance for all glyphs in this cluster.
    pub advance: f32,
}

/// Group shaped glyphs by cluster ID into visual-order clusters with byte ranges.
///
/// Adjacent glyphs sharing the same `cluster` value are merged.  Byte-range boundaries
/// are derived from the sorted set of unique cluster offsets plus `text_byte_len`.
pub fn group_clusters(glyphs: &[ShapedGlyph], text_byte_len: usize) -> Vec<ShapedCluster> {
    if glyphs.is_empty() {
        return Vec::new();
    }

    // Merge consecutive glyphs with the same cluster value (visual order).
    let mut groups: Vec<(u32, f32)> = Vec::new();
    for g in glyphs {
        if let Some(last) = groups.last_mut() {
            if last.0 == g.cluster {
                last.1 += g.x_advance;
                continue;
            }
        }
        groups.push((g.cluster, g.x_advance));
    }

    // Sorted unique byte offsets → byte-range boundaries.
    let mut offsets: Vec<usize> = groups.iter().map(|(c, _)| *c as usize).collect();
    offsets.sort_unstable();
    offsets.dedup();

    let byte_end_for = |start: usize| -> usize {
        match offsets.binary_search(&start) {
            Ok(i) if i + 1 < offsets.len() => offsets[i + 1],
            _ => text_byte_len,
        }
    };

    groups
        .iter()
        .map(|(cluster_byte, advance)| {
            let byte_start = *cluster_byte as usize;
            ShapedCluster {
                byte_start,
                byte_end: byte_end_for(byte_start),
                advance: *advance,
            }
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Position-mapping helpers (Task 22 — shaped measurements)
// ─────────────────────────────────────────────────────────────────────────────

/// Maps a character column to an x-pixel offset using pre-computed shaped clusters.
///
/// Walks the cluster list, accumulating advances until the byte offset
/// corresponding to `char_column` is reached. For positions that fall
/// *within* a multi-character cluster (e.g. a ligature), the advance is
/// linearly interpolated.
#[must_use]
pub fn column_to_x_offset(text: &str, clusters: &[ShapedCluster], char_column: usize) -> f32 {
    if clusters.is_empty() || char_column == 0 {
        return 0.0;
    }

    let byte_offset: usize = text
        .char_indices()
        .nth(char_column)
        .map(|(i, _)| i)
        .unwrap_or(text.len());

    let mut x: f32 = 0.0;
    for cluster in clusters {
        if byte_offset <= cluster.byte_start {
            return x;
        }
        if byte_offset >= cluster.byte_end {
            x += cluster.advance;
        } else {
            let cluster_chars = text[cluster.byte_start..cluster.byte_end].chars().count();
            let chars_into = text[cluster.byte_start..byte_offset].chars().count();
            if cluster_chars > 0 {
                x += cluster.advance * (chars_into as f32 / cluster_chars as f32);
            }
            return x;
        }
    }
    x
}

/// Maps an x-pixel offset to the nearest character column using shaped clusters.
///
/// Walks clusters left-to-right; for a position *within* a cluster the
/// column is rounded to the nearest character boundary (snap-to-midpoint
/// for single-glyph clusters, linear interpolation for multi-char).
#[must_use]
pub fn x_to_column(text: &str, clusters: &[ShapedCluster], x: f32) -> usize {
    if x <= 0.0 || clusters.is_empty() {
        return 0;
    }

    let mut cumulative: f32 = 0.0;
    let mut char_col: usize = 0;

    for cluster in clusters {
        let next_x = cumulative + cluster.advance;
        let end = cluster.byte_end.min(text.len());
        let start = cluster.byte_start.min(end);
        let cluster_chars = text[start..end].chars().count();

        if x < next_x {
            if cluster_chars <= 1 {
                let mid = cumulative + cluster.advance / 2.0;
                return if x < mid {
                    char_col
                } else {
                    char_col + cluster_chars
                };
            }
            let frac = (x - cumulative) / cluster.advance;
            let chars_in = (frac * cluster_chars as f32).round() as usize;
            return char_col + chars_in.min(cluster_chars);
        }

        cumulative = next_x;
        char_col += cluster_chars;
    }

    char_col
}

/// Convenience: shape the line with HarfRust and return the x-offset at `char_column`.
///
/// Returns `None` when the text is empty, shaping fails, or font_size is <= 0.
pub fn shaped_column_to_x(
    text: &str,
    font_bytes: &[u8],
    font_size: f32,
    char_column: usize,
) -> Option<f32> {
    let glyphs = shape_text(text, font_bytes, font_size).ok()?;
    if glyphs.is_empty() {
        return None;
    }
    let clusters = group_clusters(&glyphs, text.len());
    Some(column_to_x_offset(text, &clusters, char_column))
}

/// Convenience: shape the line with HarfRust and return the column nearest to `x`.
///
/// Returns `None` when the text is empty, shaping fails, or font_size is <= 0.
pub fn shaped_x_to_column(text: &str, font_bytes: &[u8], font_size: f32, x: f32) -> Option<usize> {
    let glyphs = shape_text(text, font_bytes, font_size).ok()?;
    if glyphs.is_empty() {
        return None;
    }
    let clusters = group_clusters(&glyphs, text.len());
    Some(x_to_column(text, &clusters, x))
}

fn hint_script_from_first_strong_char(text: &str, buf: &mut UnicodeBuffer) {
    use std::str::FromStr;

    for ch in text.chars() {
        let us = ch.script();
        if matches!(
            us,
            unicode_script::Script::Common | unicode_script::Script::Inherited
        ) {
            continue;
        }
        if let Ok(hs) = harfrust::Script::from_str(us.short_name()) {
            buf.set_script(hs);
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::ttf_bytes_for_font_id_shaping;
    use egui::FontId;

    fn inter_bytes() -> &'static [u8] {
        ttf_bytes_for_font_id_shaping(&FontId::proportional(14.0))
    }

    #[test]
    fn empty_and_zero_size() {
        assert!(shape_text("", inter_bytes(), 14.0).unwrap().is_empty());
        assert!(shape_text("a", inter_bytes(), 0.0).unwrap().is_empty());
    }

    #[test]
    fn latin_no_panic_and_non_empty() {
        let g = shape_text("Hello", inter_bytes(), 14.0).expect("latin shape");
        assert!(!g.is_empty());
        assert!(shaped_width(&g) > 0.0);
    }

    #[test]
    fn arabic_contextual_shaping() {
        // Lam + Alef often ligates to one glyph in fonts with Arabic GSUB.
        let s = "\u{0644}\u{0627}";
        let g = shape_text(s, inter_bytes(), 14.0).expect("arabic shape");
        assert!(!g.is_empty());
        assert!(g.len() <= s.chars().count());
    }

    #[test]
    fn arabic_word_multiple_glyphs() {
        let s = "\u{0633}\u{0644}\u{0627}\u{0645}"; // salam
        let g = shape_text(s, inter_bytes(), 16.0).expect("arabic word");
        assert!(!g.is_empty());
    }

    #[test]
    fn bengali_conjunct() {
        let s = "\u{0995}\u{09CD}\u{09B7}"; // kssa
        let g = shape_text(s, inter_bytes(), 14.0).expect("bengali shape");
        assert!(!g.is_empty());
    }

    #[test]
    fn mixed_latin_and_arabic() {
        let s = "Hi \u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        let g = shape_text(s, inter_bytes(), 14.0).expect("mixed shape");
        assert!(!g.is_empty());
    }

    #[test]
    fn jetbrains_bytes_shape() {
        let bytes = ttf_bytes_for_font_id_shaping(&FontId::monospace(13.0));
        let g = shape_text("fn main() {}", bytes, 13.0).expect("mono shape");
        assert!(!g.is_empty());
    }

    #[test]
    fn invalid_font_bytes_error() {
        let err = shape_text("x", b"not a font", 12.0).unwrap_err();
        assert!(matches!(err, ShapeError::Font(_)));
    }

    // ── group_clusters ──────────────────────────────────────────────────

    #[test]
    fn group_clusters_empty() {
        assert!(group_clusters(&[], 0).is_empty());
    }

    #[test]
    fn group_clusters_single_glyph() {
        let glyphs = vec![ShapedGlyph {
            glyph_id: 1,
            cluster: 0,
            x_advance: 8.0,
            x_offset: 0.0,
            y_offset: 0.0,
        }];
        let cs = group_clusters(&glyphs, 1);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].byte_start, 0);
        assert_eq!(cs[0].byte_end, 1);
        assert!((cs[0].advance - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn group_clusters_merges_same_cluster() {
        let glyphs = vec![
            ShapedGlyph {
                glyph_id: 1,
                cluster: 0,
                x_advance: 4.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
            ShapedGlyph {
                glyph_id: 2,
                cluster: 0,
                x_advance: 3.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        ];
        let cs = group_clusters(&glyphs, 2);
        assert_eq!(cs.len(), 1);
        assert!((cs[0].advance - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn group_clusters_latin_per_char() {
        let text = "Hello";
        let g = shape_text(text, inter_bytes(), 14.0).expect("shape");
        let cs = group_clusters(&g, text.len());
        assert_eq!(cs.len(), text.len());

        let total: f32 = cs.iter().map(|c| c.advance).sum();
        assert!((total - shaped_width(&g)).abs() < 0.01);
    }

    #[test]
    fn group_clusters_arabic_word() {
        let text = "\u{0633}\u{0644}\u{0627}\u{0645}"; // salam
        let g = shape_text(text, inter_bytes(), 14.0).expect("shape");
        let cs = group_clusters(&g, text.len());
        assert!(!cs.is_empty());

        let total: f32 = cs.iter().map(|c| c.advance).sum();
        assert!((total - shaped_width(&g)).abs() < 0.01);
    }

    #[test]
    fn group_clusters_byte_ranges_cover_text() {
        let text = "ABC";
        let g = shape_text(text, inter_bytes(), 14.0).expect("shape");
        let cs = group_clusters(&g, text.len());

        let mut covered: Vec<bool> = vec![false; text.len()];
        for c in &cs {
            for i in c.byte_start..c.byte_end {
                covered[i] = true;
            }
        }
        assert!(
            covered.iter().all(|&b| b),
            "byte ranges must cover entire text"
        );
    }

    // ── Position-mapping helpers ────────────────────────────────────────

    #[test]
    fn column_to_x_zero_column() {
        let text = "Hello";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        assert_eq!(column_to_x_offset(text, &cs, 0), 0.0);
    }

    #[test]
    fn column_to_x_full_line() {
        let text = "Hello";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        let total = shaped_width(&g);
        let x = column_to_x_offset(text, &cs, text.chars().count());
        assert!((x - total).abs() < 0.01, "full-line x={x}, total={total}");
    }

    #[test]
    fn column_to_x_monotonic() {
        let text = "ABCDE";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        let mut prev = 0.0_f32;
        for col in 0..=text.len() {
            let x = column_to_x_offset(text, &cs, col);
            assert!(x >= prev, "col {col}: {x} < prev {prev}");
            prev = x;
        }
    }

    #[test]
    fn x_to_column_zero() {
        let text = "Hello";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        assert_eq!(x_to_column(text, &cs, 0.0), 0);
        assert_eq!(x_to_column(text, &cs, -5.0), 0);
    }

    #[test]
    fn x_to_column_past_end() {
        let text = "Hi";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        let col = x_to_column(text, &cs, 9999.0);
        assert_eq!(col, text.chars().count());
    }

    #[test]
    fn roundtrip_column_x_latin() {
        let text = "Hello";
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        for col in 0..=text.chars().count() {
            let x = column_to_x_offset(text, &cs, col);
            let back = x_to_column(text, &cs, x);
            assert_eq!(back, col, "roundtrip failed at col {col}");
        }
    }

    #[test]
    fn shaped_column_to_x_convenience() {
        let text = "Hello";
        let x = shaped_column_to_x(text, inter_bytes(), 14.0, 3);
        assert!(x.is_some());
        assert!(x.unwrap() > 0.0);
    }

    #[test]
    fn shaped_x_to_column_convenience() {
        let text = "Hello";
        let col = shaped_x_to_column(text, inter_bytes(), 14.0, 10.0);
        assert!(col.is_some());
    }

    #[test]
    fn shaped_convenience_empty_text() {
        assert!(shaped_column_to_x("", inter_bytes(), 14.0, 0).is_none());
        assert!(shaped_x_to_column("", inter_bytes(), 14.0, 5.0).is_none());
    }

    #[test]
    fn arabic_column_to_x_roundtrip() {
        let text = "\u{0633}\u{0644}\u{0627}\u{0645}"; // salam
        let g = shape_text(text, inter_bytes(), 14.0).unwrap();
        let cs = group_clusters(&g, text.len());
        let total = shaped_width(&g);
        let x_end = column_to_x_offset(text, &cs, text.chars().count());
        assert!(
            (x_end - total).abs() < 0.5,
            "Arabic full width: x_end={x_end}, total={total}"
        );
    }
}
