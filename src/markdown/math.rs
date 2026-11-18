//! Preview math: `$...$` / `$$...$$` → MathML → display list → cached texture.
//!
//! The file on disk stays LaTeX. Export does not use this module.
//! `formulary` is built with `default-features = false` (no harfrust, no SVG).
//! Glyph outlines come from `skrifa` because `MathFont::outline` is crate-private.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};
use eframe::egui::{
    self, Color32, ColorImage, Context, FontId, Rect, TextureHandle, TextureOptions, Vec2,
};
use formulary::{layout, LayoutOptions, MathFont};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{
    instance::LocationRef, instance::Size, FontRef, GlyphId as SkrifaGlyphId, MetadataProvider,
};

const STIX_TWO_MATH: &[u8] = include_bytes!("../../assets/fonts/STIXTwoMath-Regular.otf");

/// Drop a pixmap instead of allocating a bitmap this large on a side.
const MAX_PIXMAP_PX: u32 = 8192;
const CACHE_CAP: usize = 256;
const SLOW_RASTER: std::time::Duration = std::time::Duration::from_millis(50);
/// One device pixel of padding so antialiasing is not clipped.
const PAD_PX: f32 = 1.0;

static LAYOUT_CALLS: AtomicU64 = AtomicU64::new(0);
static SLOW_WARNED: AtomicBool = AtomicBool::new(false);

/// How many times [`formulary::layout`] has run in this process.
/// Cache hits do not increment it.
pub fn layout_call_count() -> u64 {
    LAYOUT_CALLS.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathError {
    Convert(String),
    Font(String),
    TooLarge,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MathError::Convert(msg) | MathError::Font(msg) => f.write_str(msg),
            MathError::TooLarge => f.write_str("formula is too large to rasterize"),
        }
    }
}

impl std::error::Error for MathError {}

/// Straight-alpha RGBA bitmap of one formula.
pub struct MathRaster {
    key: RasterKey,
    pub width: u32,
    pub height: u32,
    /// Pixels from the top of the bitmap to the math baseline.
    pub baseline_from_top: f32,
    /// Quantized `pixels_per_point` the bitmap was rasterized at.
    pub pixels_per_point: f32,
    pub rgba: Vec<u8>,
}

/// A cached egui texture plus sizes in logical points.
pub struct MathImage {
    pub texture: TextureHandle,
    pub logical_width: f32,
    pub logical_height: f32,
    /// Logical points from the top of the texture to the baseline.
    pub baseline_from_top: f32,
}

/// One dollar-math span in a source line, byte offsets into that line.
#[derive(Debug, Clone)]
pub struct DollarSpan {
    pub start: usize,
    pub end: usize,
    pub literal: String,
    pub display: bool,
    /// `false` when LaTeX or MathML parsing fails. The span is still the
    /// delimited source; the preview paints that source instead of a texture.
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RasterKey {
    literal: String,
    display: bool,
    font_size_bits: u32,
    rgba: u32,
    ppp_bits: u32,
}

#[derive(Clone)]
enum Prepared {
    Mathml(String),
    Failed(String),
}

struct Lru<T> {
    map: HashMap<RasterKey, T>,
    order: VecDeque<RasterKey>,
}

impl<T> Lru<T> {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &RasterKey) -> Option<&T> {
        if self.map.contains_key(key) {
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    fn insert(&mut self, key: RasterKey, value: T) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
        while self.order.len() > CACHE_CAP {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
    }
}

fn prep_cache() -> &'static Mutex<HashMap<(String, bool), Prepared>> {
    static CACHE: OnceLock<Mutex<HashMap<(String, bool), Prepared>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn raster_cache() -> &'static Mutex<Lru<std::sync::Arc<MathRaster>>> {
    static CACHE: OnceLock<Mutex<Lru<std::sync::Arc<MathRaster>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Lru::new()))
}

fn texture_cache() -> &'static Mutex<Lru<TextureHandle>> {
    static CACHE: OnceLock<Mutex<Lru<TextureHandle>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Lru::new()))
}

fn logged_failures() -> &'static Mutex<HashSet<(String, bool)>> {
    static SET: OnceLock<Mutex<HashSet<(String, bool)>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

pub fn delimited_source(literal: &str, display: bool) -> String {
    if display {
        format!("$${literal}$$")
    } else {
        format!("${literal}$")
    }
}

fn quantize_ppp(pixels_per_point: f32) -> f32 {
    let ppp = if pixels_per_point.is_finite() {
        pixels_per_point
    } else {
        1.0
    };
    let q = (ppp.max(0.25) * 4.0).round() / 4.0;
    if q.is_finite() && q > 0.0 {
        q
    } else {
        1.0
    }
}

fn math_font() -> Result<&'static MathFont<'static>, MathError> {
    static FONT: OnceLock<Result<MathFont<'static>, String>> = OnceLock::new();
    match FONT.get_or_init(|| MathFont::new(STIX_TWO_MATH, 0).map_err(|err| err.to_string())) {
        Ok(font) => Ok(font),
        Err(err) => Err(MathError::Font(err.clone())),
    }
}

fn log_fail_once(literal: &str, display: bool, message: &str) {
    let mut seen = lock(logged_failures());
    if seen.insert((literal.to_string(), display)) {
        let snippet: String = literal.chars().take(80).collect();
        log::warn!("math preview failed (display={display}): {message} [{snippet}]");
    }
}

/// LaTeX → MathML, cached by `(literal, display)`. A hard parse error is cached
/// too, so a failing formula is not converted again every frame.
fn prepare(literal: &str, display: bool) -> Result<String, MathError> {
    let key = (literal.to_string(), display);
    if let Some(hit) = lock(prep_cache()).get(&key).cloned() {
        return match hit {
            Prepared::Mathml(mathml) => Ok(mathml),
            Prepared::Failed(message) => Err(MathError::Convert(message)),
        };
    }
    let prepared = match latex_to_mathml(literal, display) {
        Ok(mathml) => match formulary::parse(&mathml) {
            Ok(_) => Prepared::Mathml(mathml),
            Err(err) => Prepared::Failed(err.to_string()),
        },
        Err(message) => Prepared::Failed(message),
    };
    if let Prepared::Failed(message) = &prepared {
        log_fail_once(literal, display, message);
    }
    lock(prep_cache()).insert(key, prepared.clone());
    match prepared {
        Prepared::Mathml(mathml) => Ok(mathml),
        Prepared::Failed(message) => Err(MathError::Convert(message)),
    }
}

fn latex_to_mathml(literal: &str, display: bool) -> Result<String, String> {
    let storage = pulldown_latex::Storage::new();
    let parser = pulldown_latex::Parser::new(literal, &storage);
    let events: Vec<_> = parser.collect();
    if let Some(err) = events.iter().find_map(|event| event.as_ref().err()) {
        return Err(err.to_string());
    }
    let mut mathml = String::new();
    let config = pulldown_latex::RenderConfig {
        display_mode: if display {
            pulldown_latex::config::DisplayMode::Block
        } else {
            pulldown_latex::config::DisplayMode::Inline
        },
        ..pulldown_latex::RenderConfig::default()
    };
    pulldown_latex::push_mathml(&mut mathml, events.into_iter(), config)
        .map_err(|err| err.to_string())?;
    let mathml = repair_pulldown_mathml(&mathml);
    if mathml.contains("<merror") {
        return Err("invalid math".to_string());
    }
    Ok(mathml)
}

/// pulldown-latex writes operator `<` as a raw character (`<mo><</mo>`) and
/// uses the HTML entity `&nbsp;` for `\` + space and `~`. Neither is well-formed
/// XML, and the escaped form of that entity paints the letters `&nbsp;`.
fn repair_pulldown_mathml(input: &str) -> String {
    let input = input
        .replace("&amp;nbsp;", "&#160;")
        .replace("&nbsp;", "&#160;");
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => {
                let next = bytes.get(i + 1).copied();
                let is_tag = matches!(next, Some(b'/' | b'!' | b'?' | b'a'..=b'z' | b'A'..=b'Z'));
                if is_tag {
                    if let Some(end) = bytes[i..].iter().position(|&b| b == b'>') {
                        out.push_str(&input[i..i + end + 1]);
                        i += end + 1;
                        continue;
                    }
                }
                out.push_str("&lt;");
                i += 1;
            }
            b'&' => {
                if let Some(end) = bytes[i..].iter().position(|&b| b == b';') {
                    let entity = &input[i..=i + end];
                    if is_xml_entity(entity) {
                        out.push_str(entity);
                        i += end + 1;
                        continue;
                    }
                }
                out.push_str("&amp;");
                i += 1;
            }
            _ => {
                let ch = input[i..].chars().next().unwrap_or('\u{FFFD}');
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    out
}

fn is_xml_entity(entity: &str) -> bool {
    if matches!(entity, "&amp;" | "&lt;" | "&gt;" | "&quot;" | "&apos;") {
        return true;
    }
    let Some(body) = entity.strip_prefix("&#").and_then(|s| s.strip_suffix(';')) else {
        return false;
    };
    let (hex, digits) = if let Some(rest) = body.strip_prefix('x') {
        (true, rest)
    } else if let Some(rest) = body.strip_prefix('X') {
        (true, rest)
    } else {
        (false, body)
    };
    !digits.is_empty()
        && digits.chars().all(|c| {
            if hex {
                c.is_ascii_hexdigit()
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Rasterize one formula. Same key does not lay out or rebuild the pixmap.
pub fn render_math(
    literal: &str,
    display: bool,
    font_size_pt: f32,
    rgba: [u8; 4],
    pixels_per_point: f32,
) -> Result<std::sync::Arc<MathRaster>, MathError> {
    let font_size = if font_size_pt.is_finite() && font_size_pt > 0.0 {
        font_size_pt
    } else {
        16.0
    };
    let ppp = quantize_ppp(pixels_per_point);
    let key = RasterKey {
        literal: literal.to_string(),
        display,
        font_size_bits: font_size.to_bits(),
        rgba: u32::from_be_bytes(rgba),
        ppp_bits: ppp.to_bits(),
    };
    if let Some(hit) = lock(raster_cache()).get(&key).cloned() {
        return Ok(hit);
    }

    let mathml = prepare(literal, display)?;
    let font = math_font()?;
    let started = Instant::now();
    LAYOUT_CALLS.fetch_add(1, Ordering::Relaxed);
    let root = formulary::parse(&mathml).map_err(|err| MathError::Convert(err.to_string()))?;
    let laid = layout(&root, font, &LayoutOptions { font_size });
    let raster = rasterize(&laid, font, rgba, ppp, key)?;
    if started.elapsed() > SLOW_RASTER && !SLOW_WARNED.swap(true, Ordering::Relaxed) {
        let snippet: String = literal.chars().take(80).collect();
        log::warn!(
            "math raster took {} ms [{snippet}]",
            started.elapsed().as_millis()
        );
    }
    let arc = std::sync::Arc::new(raster);
    lock(raster_cache()).insert(arc.key.clone(), std::sync::Arc::clone(&arc));
    Ok(arc)
}

/// Raster plus an egui texture. A hit does not upload again.
pub fn render_math_image(
    ctx: &Context,
    literal: &str,
    display: bool,
    font_size_pt: f32,
    color: Color32,
) -> Result<MathImage, MathError> {
    let rgba = [color.r(), color.g(), color.b(), color.a()];
    let raster = render_math(literal, display, font_size_pt, rgba, ctx.pixels_per_point())?;
    let texture = texture_for(ctx, &raster);
    let ppp = raster.pixels_per_point.max(0.25);
    Ok(MathImage {
        texture,
        logical_width: raster.width as f32 / ppp,
        logical_height: raster.height as f32 / ppp,
        baseline_from_top: raster.baseline_from_top / ppp,
    })
}

fn texture_for(ctx: &Context, raster: &MathRaster) -> TextureHandle {
    if let Some(hit) = lock(texture_cache()).get(&raster.key).cloned() {
        return hit;
    }
    let pixels: Vec<Color32> = raster
        .rgba
        .chunks_exact(4)
        .map(|px| Color32::from_rgba_unmultiplied(px[0], px[1], px[2], px[3]))
        .collect();
    let color_image = ColorImage {
        size: [raster.width as usize, raster.height as usize],
        source_size: Vec2::new(
            raster.width as f32 / raster.pixels_per_point.max(0.25),
            raster.height as f32 / raster.pixels_per_point.max(0.25),
        ),
        pixels,
    };
    let mut hasher = DefaultHasher::new();
    raster.key.hash(&mut hasher);
    let handle = ctx.load_texture(
        format!("ferrite-math-{:x}", hasher.finish()),
        color_image,
        TextureOptions::LINEAR,
    );
    lock(texture_cache()).insert(raster.key.clone(), handle.clone());
    handle
}

/// Tallest successful formula in `text`, in logical points. `0.0` when there
/// is no dollar math. Used to grow a table row before paint.
pub fn source_formula_height(
    text: &str,
    font_size_pt: f32,
    color: Color32,
    pixels_per_point: f32,
) -> f32 {
    if !text.contains('$') {
        return 0.0;
    }
    let rgba = [color.r(), color.g(), color.b(), color.a()];
    let mut height = 0.0_f32;
    for span in dollar_spans(text) {
        if !span.ok {
            continue;
        }
        if let Ok(raster) = render_math(
            &span.literal,
            span.display,
            font_size_pt,
            rgba,
            pixels_per_point,
        ) {
            let logical = raster.height as f32 / raster.pixels_per_point.max(0.25);
            height = height.max(logical);
        }
    }
    height
}

fn rasterize(
    laid: &formulary::Layout,
    font: &MathFont<'_>,
    rgba: [u8; 4],
    ppp: f32,
    key: RasterKey,
) -> Result<MathRaster, MathError> {
    let width_px = (PAD_PX * 2.0 + laid.width * ppp).ceil().max(1.0) as u32;
    let height_px = (PAD_PX * 2.0 + (laid.ascent + laid.descent) * ppp)
        .ceil()
        .max(1.0) as u32;
    if width_px > MAX_PIXMAP_PX || height_px > MAX_PIXMAP_PX {
        return Err(MathError::TooLarge);
    }
    let mut pixmap = tiny_skia::Pixmap::new(width_px, height_px).ok_or(MathError::TooLarge)?;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
    paint.anti_alias = true;

    let face =
        FontRef::from_index(STIX_TWO_MATH, 0).map_err(|err| MathError::Font(err.to_string()))?;
    let outlines = face.outline_glyphs();
    let em = font.units_per_em().max(1.0);

    for item in &laid.items {
        match *item {
            formulary::Item::Glyph {
                id,
                x,
                y,
                size,
                advance,
                mirrored,
                color: _,
            } => {
                let Some(glyph) = outlines.get(SkrifaGlyphId::new(u32::from(id.0))) else {
                    continue;
                };
                let s = size / em;
                let (tx, sx) = if mirrored { (x + advance, -s) } else { (x, s) };
                let mut pen = GlyphPen {
                    path: tiny_skia::PathBuilder::new(),
                    tx,
                    sx,
                    s,
                    glyph_y: y,
                    ascent: laid.ascent,
                    ppp,
                };
                if glyph
                    .draw(
                        DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                        &mut pen,
                    )
                    .is_err()
                {
                    continue;
                }
                if let Some(path) = pen.path.finish() {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
            formulary::Item::Rule {
                x,
                y,
                w,
                h,
                color: _,
            } => {
                let rx = PAD_PX + x * ppp;
                let ry = PAD_PX + (laid.ascent + y) * ppp;
                if let Some(rect) =
                    tiny_skia::Rect::from_xywh(rx, ry, (w * ppp).max(0.0), (h * ppp).max(0.0))
                {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
            }
            formulary::Item::Background { .. } => {}
            _ => {}
        }
    }

    let mut rgba_bytes = Vec::with_capacity(pixmap.pixels().len() * 4);
    for pixel in pixmap.pixels() {
        let straight = pixel.demultiply();
        rgba_bytes.extend_from_slice(&[
            straight.red(),
            straight.green(),
            straight.blue(),
            straight.alpha(),
        ]);
    }

    Ok(MathRaster {
        key,
        width: width_px,
        height: height_px,
        baseline_from_top: PAD_PX + laid.ascent * ppp,
        pixels_per_point: ppp,
        rgba: rgba_bytes,
    })
}

/// Font-unit outline → pixmap pixels.
///
/// Layout `y` grows downward from the baseline. The outline is y-up.
/// `sx` is `± size/em` so a mirrored glyph flips about its advance box
/// (`tx = x + advance`).
struct GlyphPen {
    path: tiny_skia::PathBuilder,
    tx: f32,
    sx: f32,
    s: f32,
    glyph_y: f32,
    ascent: f32,
    ppp: f32,
}

impl GlyphPen {
    fn map(&self, fx: f32, fy: f32) -> (f32, f32) {
        let px = PAD_PX + (self.tx + fx * self.sx) * self.ppp;
        let py = PAD_PX + (self.ascent + self.glyph_y) * self.ppp - fy * self.s * self.ppp;
        (px, py)
    }
}

impl OutlinePen for GlyphPen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.map(x, y);
        self.path.move_to(px, py);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.map(x, y);
        self.path.line_to(px, py);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx, cy) = self.map(cx0, cy0);
        let (px, py) = self.map(x, y);
        self.path.quad_to(cx, cy, px, py);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (c0x, c0y) = self.map(cx0, cy0);
        let (c1x, c1y) = self.map(cx1, cy1);
        let (px, py) = self.map(x, y);
        self.path.cubic_to(c0x, c0y, c1x, c1y, px, py);
    }

    fn close(&mut self) {
        self.path.close();
    }
}

/// Dollar spans comrak accepts in `text`. Empty when the line has no `$`.
/// Code spans are not math. `$20 and $30` yields nothing.
pub fn dollar_spans(text: &str) -> Vec<DollarSpan> {
    if !text.contains('$') {
        return Vec::new();
    }
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.math_dollars = true;
    let root = parse_document(&arena, text, &options);
    let mut out = Vec::new();
    collect_dollar_spans(root, text, &mut out);
    out
}

fn collect_dollar_spans<'a>(node: &'a AstNode<'a>, text: &str, out: &mut Vec<DollarSpan>) {
    for child in node.children() {
        enum Step {
            Math {
                start: usize,
                end: usize,
                literal: String,
                display: bool,
            },
            Skip,
            Walk,
        }
        let step = {
            let data = child.data.borrow();
            match &data.value {
                NodeValue::Math(math) => Step::Math {
                    start: data.sourcepos.start.column.saturating_sub(1),
                    end: data.sourcepos.end.column,
                    literal: math.literal.clone(),
                    display: math.display_math,
                },
                NodeValue::Code(_) | NodeValue::CodeBlock(_) => Step::Skip,
                _ => Step::Walk,
            }
        };
        match step {
            Step::Math {
                start,
                end,
                literal,
                display,
            } => {
                // Comrak's sourcepos covers the literal, not the `$` / `$$` delimiters.
                let delim = if display { 2 } else { 1 };
                let raw_start = start.saturating_sub(delim);
                let raw_end = end.saturating_add(delim);
                let fences = if display { "$$" } else { "$" };
                if raw_start < start
                    && end <= raw_end
                    && raw_end <= text.len()
                    && text.is_char_boundary(raw_start)
                    && text.is_char_boundary(raw_end)
                    && text.get(raw_start..start) == Some(fences)
                    && text.get(end..raw_end) == Some(fences)
                {
                    let ok = prepare(&literal, display).is_ok();
                    out.push(DollarSpan {
                        start: raw_start,
                        end: raw_end,
                        literal,
                        display,
                        ok,
                    });
                }
            }
            Step::Skip => {}
            Step::Walk => collect_dollar_spans(child, text, out),
        }
    }
}

/// Paint a source line that may contain dollar math into `rect` (table cells).
/// Returns false when there is nothing to draw as a formula.
pub fn paint_dollar_source(
    ui: &mut egui::Ui,
    rect: Rect,
    text: &str,
    font: FontId,
    text_color: Color32,
    quote_color: Color32,
) -> bool {
    let spans = dollar_spans(text);
    if spans.is_empty() {
        return false;
    }
    let enabled = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(render_math_temp_id()).unwrap_or(true));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true)),
    );
    child.spacing_mut().item_spacing = Vec2::ZERO;
    let mut cursor = 0usize;
    for span in &spans {
        if span.start > cursor && text.is_char_boundary(cursor) && text.is_char_boundary(span.start)
        {
            paint_plain(
                &mut child,
                &text[cursor..span.start],
                font.clone(),
                text_color,
            );
        }
        if !enabled || !span.ok {
            let source = if span.end <= text.len() && text.is_char_boundary(span.end) {
                text[span.start..span.end].to_string()
            } else {
                delimited_source(&span.literal, span.display)
            };
            let label = child.label(
                egui::RichText::new(source)
                    .color(quote_color)
                    .font(font.clone()),
            );
            if enabled && !span.ok {
                if let Err(err) = prepare(&span.literal, span.display) {
                    label.on_hover_text(failure_tooltip(&err));
                }
            }
        } else {
            place_formula(
                &mut child,
                &span.literal,
                span.display,
                font.size,
                text_color,
            );
        }
        cursor = span.end;
    }
    if cursor < text.len() && text.is_char_boundary(cursor) {
        paint_plain(&mut child, &text[cursor..], font, text_color);
    }
    true
}

fn paint_plain(ui: &mut egui::Ui, text: &str, font: FontId, color: Color32) {
    if text.is_empty() {
        return;
    }
    ui.label(egui::RichText::new(text).color(color).font(font));
}

fn place_formula(ui: &mut egui::Ui, literal: &str, display: bool, font_size: f32, color: Color32) {
    match render_math_image(ui.ctx(), literal, display, font_size, color) {
        Ok(image) => {
            if display {
                place_display(ui, &image);
            } else {
                place_inline(ui, &image);
            }
        }
        Err(err) => {
            ui.label(egui::RichText::new(delimited_source(literal, display)).color(color))
                .on_hover_text(failure_tooltip(&err));
        }
    }
}

fn place_inline(ui: &mut egui::Ui, image: &MathImage) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(image.logical_width.max(1.0), image.logical_height.max(1.0)),
        egui::Sense::hover(),
    );
    paint_image(ui, rect, image);
}

fn place_display(ui: &mut egui::Ui, image: &MathImage) {
    ui.end_row();
    let avail = ui.available_width().max(1.0);
    let mut width = image.logical_width.max(1.0);
    let mut height = image.logical_height.max(1.0);
    if width > avail {
        let scale = avail / width;
        width = avail;
        height *= scale;
    }
    let left = ((avail - width) * 0.5).max(0.0);
    if left > 0.0 {
        ui.add_space(left);
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    paint_image(ui, rect, image);
    ui.end_row();
}

fn paint_image(ui: &egui::Ui, rect: Rect, image: &MathImage) {
    ui.painter().image(
        image.texture.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub(crate) fn render_math_temp_id() -> egui::Id {
    egui::Id::new("render_math")
}

pub(crate) fn math_row_ascent_id() -> egui::Id {
    egui::Id::new("ferrite.math_row_ascent")
}

fn failure_tooltip(err: &MathError) -> String {
    rust_i18n::t!("markdown.math.render_failed", error = err.to_string()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|err| err.into_inner())
    }

    #[test]
    fn summation_is_taller_than_inline_and_baseline_is_inside() {
        let _guard = test_lock();
        let inline = render_math("E = mc^2", false, 16.0, [0, 0, 0, 255], 1.0).unwrap();
        let display = render_math(
            r"\sum_{i=0}^{n} i = \frac{n(n+1)}{2}",
            true,
            16.0,
            [0, 0, 0, 255],
            1.0,
        )
        .unwrap();
        assert!(display.height > inline.height);
        assert!(display.baseline_from_top > 0.0);
        assert!(display.baseline_from_top < display.height as f32);
        assert!(inline.baseline_from_top > 0.0);
        assert!(inline.baseline_from_top < inline.height as f32);
    }

    #[test]
    fn less_than_in_cases_renders_and_control_space_is_a_space() {
        let _guard = test_lock();
        let literal = r"|x| = \begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases} \qquad \begin{pmatrix} a & b \\ c & d \end{pmatrix}";
        let mathml = latex_to_mathml(literal, true).unwrap();
        assert!(mathml.contains("&lt;"), "{mathml}");
        assert!(!mathml.contains("<mo><"), "{mathml}");
        formulary::parse(&mathml).unwrap();
        let raster = render_math(literal, true, 16.0, [0, 0, 0, 255], 1.0).unwrap();
        assert!(raster.height > 8);

        let spaced = latex_to_mathml(r"\forall x \in \mathbb{R},\ \hat{x}+\bar{y}+\vec{z}", false)
            .unwrap();
        assert!(spaced.contains("&#160;"), "{spaced}");
        assert!(!spaced.contains("nbsp"), "{spaced}");
        formulary::parse(&spaced).unwrap();
    }

    #[test]
    fn broken_formula_is_an_error() {
        let _guard = test_lock();
        let err = render_math(r"\frac{", false, 16.0, [0, 0, 0, 255], 1.0);
        assert!(err.is_err());
    }

    #[test]
    fn same_key_does_not_lay_out_twice_and_color_misses() {
        let _guard = test_lock();
        let before = layout_call_count();
        let color = [10, 20, 30, 255];
        render_math("x^2", false, 18.0, color, 1.0).unwrap();
        render_math("x^2", false, 18.0, color, 1.0).unwrap();
        assert_eq!(layout_call_count(), before + 1);
        render_math("x^2", false, 18.0, [11, 20, 30, 255], 1.0).unwrap();
        assert_eq!(layout_call_count(), before + 2);
    }

    #[test]
    fn dollar_span_covers_delimiters_and_skips_currency() {
        let _guard = test_lock();
        let spans = dollar_spans("$x$");
        assert_eq!(spans.len(), 1);
        assert_eq!(&spans[0].literal, "x");
        assert!(!spans[0].display);
        assert!(spans[0].ok);
        assert_eq!(&"$x$"[spans[0].start..spans[0].end], "$x$");

        let display = dollar_spans("$$x^2$$");
        assert_eq!(display.len(), 1);
        assert!(display[0].display);
        assert_eq!(&"$$x^2$$"[display[0].start..display[0].end], "$$x^2$$");

        assert!(dollar_spans("$20 and $30").is_empty());
        let broken = dollar_spans(r"$\frac{$");
        assert_eq!(broken.len(), 1);
        assert!(!broken[0].ok);
    }
}
