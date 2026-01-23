//! Font management for Ferrite
//!
//! This module handles loading custom fonts with proper bold/italic variants.
//! Fonts are embedded at compile time using `include_bytes!`.
//!
//! ## Font Selection Features
//!
//! - Built-in fonts: Inter (proportional) and JetBrains Mono (monospace)
//! - Custom system font selection via font-kit
//! - CJK regional font preferences for correct glyph variants
//! - Runtime font reloading without restart

// Allow dead code - includes utility functions for font styling that may be
// used for advanced text rendering features
#![allow(dead_code)]

use egui::{FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use log::{info, warn};
use std::collections::BTreeMap;
use std::sync::OnceLock;

// ─────────────────────────────────────────────────────────────────────────────
// Font Data - Embedded at compile time
// ─────────────────────────────────────────────────────────────────────────────

// Inter font family (UI/proportional)
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
const INTER_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-Italic.ttf");
const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/Inter-BoldItalic.ttf");

// JetBrains Mono font family (code/monospace)
const JETBRAINS_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const JETBRAINS_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");
const JETBRAINS_ITALIC: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf");
const JETBRAINS_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf");

/// Cache for system font list (expensive to compute, do once)
static SYSTEM_FONTS_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// ─────────────────────────────────────────────────────────────────────────────
// Per-Language CJK Font Loading State
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Track which CJK font sets have been lazily loaded.
/// Each language can be loaded independently for memory efficiency.
static KOREAN_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static JAPANESE_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static CHINESE_SC_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static CHINESE_TC_FONTS_LOADED: AtomicBool = AtomicBool::new(false);

/// Font generation counter - increments whenever fonts are set up or changed.
/// Used to invalidate galley caches that may have been built with missing glyphs
/// before the font atlas was fully populated.
static FONT_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Flag indicating that font atlas pre-warming is needed on the next frame.
/// This is set during font setup and cleared after pre-warming is complete.
static NEEDS_PREWARM: AtomicBool = AtomicBool::new(false);

/// Get the current font generation counter.
///
/// This value changes whenever fonts are set up or reloaded. Use this in
/// galley cache keys to ensure cached galleys are invalidated when fonts change.
/// This is especially important for characters that may not be in the initial
/// font atlas (like box-drawing characters) which would render as squares
/// until the atlas is populated.
pub fn font_generation() -> u64 {
    FONT_GENERATION.load(Ordering::Relaxed)
}

/// Increment the font generation counter.
///
/// Called internally whenever ctx.set_fonts() is called.
fn bump_font_generation() {
    let gen = FONT_GENERATION.fetch_add(1, Ordering::Relaxed);
    info!("Font generation bumped to {}", gen + 1);
}

/// Schedule font atlas pre-warming for the next frame.
///
/// Pre-warming cannot happen during font setup because ctx.fonts() is not
/// available until after the first Context::run() call.
fn schedule_prewarm() {
    NEEDS_PREWARM.store(true, Ordering::Relaxed);
}

/// Check if pre-warming is needed and perform it if so.
///
/// This should be called during update() after the context is fully initialized.
/// It pre-warms the font atlas with box-drawing and common symbol characters,
/// then bumps the font generation to invalidate any galleys created before
/// the atlas was fully populated.
pub fn check_and_prewarm_if_needed(ctx: &egui::Context) {
    if NEEDS_PREWARM.swap(false, Ordering::Relaxed) {
        prewarm_font_atlas(ctx);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CJK Script Detection
// ─────────────────────────────────────────────────────────────────────────────

/// CJK scripts that can be detected in text.
/// Used for granular font loading - only load fonts for detected scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CjkScript {
    /// Korean (Hangul)
    Korean,
    /// Japanese (Hiragana, Katakana, or mixed with Kanji)
    Japanese,
    /// Chinese (Simplified or Traditional - uses Han characters)
    Chinese,
}

/// Result of scanning text for CJK scripts.
#[derive(Debug, Clone, Default)]
pub struct CjkScriptDetection {
    /// Korean script detected (Hangul characters)
    pub has_korean: bool,
    /// Japanese script detected (Hiragana or Katakana)
    pub has_japanese: bool,
    /// Han characters detected (shared by Chinese, Japanese Kanji, Korean Hanja)
    pub has_han: bool,
    /// Any CJK detected at all
    pub has_any_cjk: bool,
}

// Unicode ranges for script-specific detection
const HANGUL_SYLLABLES: (u32, u32) = (0xAC00, 0xD7AF);
const HANGUL_JAMO: (u32, u32) = (0x1100, 0x11FF);
const HANGUL_COMPAT_JAMO: (u32, u32) = (0x3130, 0x318F);

const HIRAGANA: (u32, u32) = (0x3040, 0x309F);
const KATAKANA: (u32, u32) = (0x30A0, 0x30FF);
const KATAKANA_EXT: (u32, u32) = (0x31F0, 0x31FF);

const CJK_UNIFIED: (u32, u32) = (0x4E00, 0x9FFF);
const CJK_EXT_A: (u32, u32) = (0x3400, 0x4DBF);
const CJK_COMPAT: (u32, u32) = (0xF900, 0xFAFF);
const CJK_RADICALS: (u32, u32) = (0x2E80, 0x2EFF);
const KANGXI_RADICALS: (u32, u32) = (0x2F00, 0x2FDF);
const CJK_SYMBOLS: (u32, u32) = (0x3000, 0x303F);
const BOPOMOFO: (u32, u32) = (0x3100, 0x312F);

#[inline]
fn in_range(cp: u32, range: (u32, u32)) -> bool {
    cp >= range.0 && cp <= range.1
}

/// Check if a character is Korean (Hangul).
#[inline]
fn is_korean_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, HANGUL_SYLLABLES) || in_range(cp, HANGUL_JAMO) || in_range(cp, HANGUL_COMPAT_JAMO)
}

/// Check if a character is uniquely Japanese (Hiragana or Katakana).
#[inline]
fn is_japanese_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, HIRAGANA) || in_range(cp, KATAKANA) || in_range(cp, KATAKANA_EXT)
}

/// Check if a character is Han (shared by Chinese, Japanese Kanji, Korean Hanja).
#[inline]
fn is_han_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, CJK_UNIFIED)
        || in_range(cp, CJK_EXT_A)
        || in_range(cp, CJK_COMPAT)
        || in_range(cp, CJK_RADICALS)
        || in_range(cp, KANGXI_RADICALS)
        || in_range(cp, BOPOMOFO)
}

/// Check if a character is any CJK character.
#[inline]
fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, CJK_UNIFIED)
        || in_range(cp, HIRAGANA)
        || in_range(cp, KATAKANA)
        || in_range(cp, HANGUL_SYLLABLES)
        || in_range(cp, CJK_EXT_A)
        || in_range(cp, KATAKANA_EXT)
        || in_range(cp, BOPOMOFO)
        || in_range(cp, HANGUL_COMPAT_JAMO)
        || in_range(cp, HANGUL_JAMO)
        || in_range(cp, CJK_COMPAT)
        || in_range(cp, CJK_RADICALS)
        || in_range(cp, KANGXI_RADICALS)
        || in_range(cp, CJK_SYMBOLS)
}

/// Detect which CJK scripts are present in text.
///
/// This function scans text and identifies which specific CJK writing systems are used.
/// This enables loading only the necessary fonts instead of all CJK fonts at once.
///
/// # Script Detection Logic
///
/// - **Korean**: Hangul syllables or Jamo characters
/// - **Japanese**: Hiragana or Katakana characters
/// - **Han/Chinese**: CJK Unified Ideographs (shared by Chinese, Japanese Kanji, Korean Hanja)
///
/// Note: Han characters alone could be any of the three languages. The user's CJK
/// preference setting determines which regional font to use for Han-only text.
pub fn detect_cjk_scripts(text: &str) -> CjkScriptDetection {
    let mut result = CjkScriptDetection::default();

    for c in text.chars() {
        if is_korean_char(c) {
            result.has_korean = true;
            result.has_any_cjk = true;
        } else if is_japanese_char(c) {
            result.has_japanese = true;
            result.has_any_cjk = true;
        } else if is_han_char(c) {
            result.has_han = true;
            result.has_any_cjk = true;
        }

        // Early exit if we've found all script types
        if result.has_korean && result.has_japanese && result.has_han {
            break;
        }
    }

    result
}

/// Check if text contains any CJK characters requiring specialized font support.
///
/// This function efficiently scans text to detect CJK characters (Chinese, Japanese, Korean).
/// Used for lazy loading of CJK fonts - we only load system CJK fonts when needed.
///
/// # Examples
///
/// ```
/// assert!(needs_cjk("你好世界")); // Chinese
/// assert!(needs_cjk("こんにちは")); // Japanese
/// assert!(needs_cjk("안녕하세요")); // Korean
/// assert!(!needs_cjk("Hello World")); // ASCII only
/// assert!(needs_cjk("Hello 世界")); // Mixed text
/// ```
pub fn needs_cjk(text: &str) -> bool {
    text.chars().any(is_cjk_char)
}

/// Check if any CJK fonts have been loaded.
pub fn are_cjk_fonts_loaded() -> bool {
    KOREAN_FONTS_LOADED.load(Ordering::Relaxed)
        || JAPANESE_FONTS_LOADED.load(Ordering::Relaxed)
        || CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed)
        || CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed)
}

/// Check which specific CJK font sets are loaded.
pub fn get_loaded_cjk_fonts() -> (bool, bool, bool, bool) {
    (
        KOREAN_FONTS_LOADED.load(Ordering::Relaxed),
        JAPANESE_FONTS_LOADED.load(Ordering::Relaxed),
        CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed),
        CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// System Font Detection
// ─────────────────────────────────────────────────────────────────────────────

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

// NanumGothic bundled fallback removed per user request.
// We strictly rely on system fonts now.

/// Attempt to load a specific system font from a list of candidates.
///
/// Returns `Some(FontData)` for the first candidate found on the system.
fn load_system_font(families: &[&str]) -> Option<FontData> {
    let source = SystemSource::new();

    for family in families {
        info!("Attempting to load system font: {}", family);
        if let Ok(handle) =
            source.select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new())
        {
            match handle {
                Handle::Path { path, .. } => {
                    info!("Found system font at: {:?}", path);
                    // Read file content
                    if let Ok(bytes) = std::fs::read(&path) {
                        return Some(FontData::from_owned(bytes));
                    }
                }
                Handle::Memory { bytes, .. } => {
                    info!("Found system font in memory ({} bytes)", bytes.len());
                    return Some(FontData::from_owned(bytes.to_vec()));
                }
            }
        }
    }
    None
}

/// Load a specific system font by exact family name.
///
/// Returns `Some(FontData)` if the font is found on the system.
fn load_system_font_by_name(family_name: &str) -> Option<FontData> {
    let source = SystemSource::new();

    info!("Attempting to load custom font: {}", family_name);
    if let Ok(handle) = source.select_best_match(
        &[FamilyName::Title(family_name.to_string())],
        &Properties::new(),
    ) {
        match handle {
            Handle::Path { path, .. } => {
                info!("Found custom font at: {:?}", path);
                if let Ok(bytes) = std::fs::read(&path) {
                    return Some(FontData::from_owned(bytes));
                }
            }
            Handle::Memory { bytes, .. } => {
                info!("Found custom font in memory ({} bytes)", bytes.len());
                return Some(FontData::from_owned(bytes.to_vec()));
            }
        }
    }
    warn!("Custom font '{}' not found on system", family_name);
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// System Font Enumeration
// ─────────────────────────────────────────────────────────────────────────────

/// Get a list of all available system font family names.
///
/// This function caches the result since font enumeration is expensive.
/// The list is sorted alphabetically and deduplicated.
pub fn list_system_fonts() -> &'static [String] {
    SYSTEM_FONTS_CACHE.get_or_init(|| {
        let mut families = std::collections::HashSet::new();
        let source = SystemSource::new();

        info!("Enumerating system fonts...");

        match source.all_families() {
            Ok(family_names) => {
                for name in family_names {
                    // Filter out internal/system fonts that users typically don't want
                    if !name.starts_with('.')
                        && !name.starts_with("System")
                        && !name.contains("LastResort")
                    {
                        families.insert(name);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to enumerate system fonts: {}", e);
            }
        }

        let mut sorted: Vec<String> = families.into_iter().collect();
        sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

        info!("Found {} system font families", sorted.len());
        sorted
    })
}

/// Check if a font family name is available on the system.
pub fn is_font_available(family_name: &str) -> bool {
    list_system_fonts()
        .iter()
        .any(|f| f.eq_ignore_ascii_case(family_name))
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Family Names
// ─────────────────────────────────────────────────────────────────────────────

/// Custom font family for Inter (proportional UI font)
pub const FONT_INTER: &str = "Inter";
/// Custom font family for Inter Bold
pub const FONT_INTER_BOLD: &str = "Inter-Bold";
/// Custom font family for Inter Italic
pub const FONT_INTER_ITALIC: &str = "Inter-Italic";
/// Custom font family for Inter Bold Italic
pub const FONT_INTER_BOLD_ITALIC: &str = "Inter-BoldItalic";

/// Custom font family for JetBrains Mono (monospace/code font)
pub const FONT_JETBRAINS: &str = "JetBrainsMono";
/// Custom font family for JetBrains Mono Bold
pub const FONT_JETBRAINS_BOLD: &str = "JetBrainsMono-Bold";
/// Custom font family for JetBrains Mono Italic
pub const FONT_JETBRAINS_ITALIC: &str = "JetBrainsMono-Italic";
/// Custom font family for JetBrains Mono Bold Italic
pub const FONT_JETBRAINS_BOLD_ITALIC: &str = "JetBrainsMono-BoldItalic";

/// Keys for dynamically loaded CJK system fonts
const FONT_CJK_KR: &str = "CJK_KR";
const FONT_CJK_SC: &str = "CJK_SC";
const FONT_CJK_TC: &str = "CJK_TC";
const FONT_CJK_JP: &str = "CJK_JP";

/// Key for custom user-selected font
const FONT_CUSTOM: &str = "Custom";

// ─────────────────────────────────────────────────────────────────────────────
// Font Loading
// ─────────────────────────────────────────────────────────────────────────────

use crate::config::CjkFontPreference;

/// Track which CJK fonts were successfully loaded.
#[derive(Default, Clone)]
pub struct CjkFontState {
    pub kr_loaded: bool,
    pub sc_loaded: bool,
    pub tc_loaded: bool,
    pub jp_loaded: bool,
}

impl CjkFontState {
    /// Check if a font key was loaded.
    fn is_loaded(&self, key: &str) -> bool {
        match key {
            FONT_CJK_KR => self.kr_loaded,
            FONT_CJK_SC => self.sc_loaded,
            FONT_CJK_TC => self.tc_loaded,
            FONT_CJK_JP => self.jp_loaded,
            _ => false,
        }
    }

    /// Check if any CJK font is loaded.
    pub fn any_loaded(&self) -> bool {
        self.kr_loaded || self.sc_loaded || self.tc_loaded || self.jp_loaded
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-Language Font Loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load Korean system font.
fn load_korean_font() -> Option<FontData> {
    // MacOS: Apple SD Gothic Neo
    // Windows: Malgun Gothic
    // Linux: Noto Sans CJK KR, NanumGothic
    let candidates = [
        "Apple SD Gothic Neo",
        "Malgun Gothic",
        "Noto Sans CJK KR",
        "NanumGothic",
    ];
    load_system_font(&candidates)
}

/// Load Japanese system font.
fn load_japanese_font() -> Option<FontData> {
    // MacOS: Hiragino Sans, Hiragino Kaku Gothic ProN
    // Windows: Yu Gothic, Meiryo
    // Linux: Noto Sans CJK JP
    let candidates = [
        "Hiragino Sans",
        "Hiragino Kaku Gothic ProN",
        "Yu Gothic",
        "Meiryo",
        "Noto Sans CJK JP",
    ];
    load_system_font(&candidates)
}

/// Load Simplified Chinese system font.
fn load_chinese_sc_font() -> Option<FontData> {
    // MacOS: PingFang SC
    // Windows: Microsoft YaHei
    // Linux: Noto Sans CJK SC
    let candidates = ["PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC"];
    load_system_font(&candidates)
}

/// Load Traditional Chinese system font.
fn load_chinese_tc_font() -> Option<FontData> {
    // MacOS: PingFang TC
    // Windows: Microsoft JhengHei
    // Linux: Noto Sans CJK TC
    let candidates = ["PingFang TC", "Microsoft JhengHei", "Noto Sans CJK TC"];
    load_system_font(&candidates)
}

/// Specification of which CJK fonts to load.
#[derive(Debug, Clone, Default)]
pub struct CjkLoadSpec {
    pub load_korean: bool,
    pub load_japanese: bool,
    pub load_chinese_sc: bool,
    pub load_chinese_tc: bool,
}

impl CjkLoadSpec {
    /// Load all CJK fonts.
    pub fn all() -> Self {
        Self {
            load_korean: true,
            load_japanese: true,
            load_chinese_sc: true,
            load_chinese_tc: true,
        }
    }

    /// Create spec from script detection result and user preference.
    ///
    /// This determines which fonts to load based on what scripts were detected:
    /// - Korean script → load Korean font
    /// - Japanese script (Hiragana/Katakana) → load Japanese font
    /// - Han characters only → load based on user's CJK preference
    ///
    /// IMPORTANT: This also includes any fonts that were previously loaded,
    /// to ensure font family references remain valid when rebuilding.
    pub fn from_detection(detection: &CjkScriptDetection, preference: CjkFontPreference) -> Self {
        let mut spec = Self::default();

        // CRITICAL: Include any fonts that were already loaded
        // This prevents crashes when rebuilding fonts after detecting new scripts
        if KOREAN_FONTS_LOADED.load(Ordering::Relaxed) {
            spec.load_korean = true;
        }
        if JAPANESE_FONTS_LOADED.load(Ordering::Relaxed) {
            spec.load_japanese = true;
        }
        if CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed) {
            spec.load_chinese_sc = true;
        }
        if CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed) {
            spec.load_chinese_tc = true;
        }

        // Load Korean if Hangul detected
        if detection.has_korean {
            spec.load_korean = true;
        }

        // Load Japanese if Hiragana/Katakana detected
        if detection.has_japanese {
            spec.load_japanese = true;
        }

        // If Han characters detected without script-specific markers,
        // load based on user preference
        if detection.has_han && !detection.has_korean && !detection.has_japanese {
            match preference {
                CjkFontPreference::Korean => spec.load_korean = true,
                CjkFontPreference::Japanese => spec.load_japanese = true,
                CjkFontPreference::SimplifiedChinese => spec.load_chinese_sc = true,
                CjkFontPreference::TraditionalChinese => spec.load_chinese_tc = true,
                CjkFontPreference::Auto => {
                    // Default to Simplified Chinese for Han-only text
                    spec.load_chinese_sc = true;
                }
            }
        }

        spec
    }

    /// Check if any fonts should be loaded.
    pub fn any(&self) -> bool {
        self.load_korean || self.load_japanese || self.load_chinese_sc || self.load_chinese_tc
    }
}

/// Load CJK system fonts based on specification.
///
/// IMPORTANT: This always loads font data for fonts in the spec, because
/// ctx.set_fonts() completely replaces all fonts. The atomic bools track
/// what has been loaded historically for `from_detection` to include
/// previously loaded fonts in new specs.
fn load_cjk_fonts_selective(fonts: &mut FontDefinitions, spec: &CjkLoadSpec) -> CjkFontState {
    let mut state = CjkFontState::default();

    // Always load font data if spec requires it - set_fonts() replaces everything
    if spec.load_korean {
        if let Some(data) = load_korean_font() {
            fonts.font_data.insert(FONT_CJK_KR.to_owned(), data);
            state.kr_loaded = true;
            if !KOREAN_FONTS_LOADED.load(Ordering::Relaxed) {
                KOREAN_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Korean font (first time)");
            }
        }
    }

    if spec.load_japanese {
        if let Some(data) = load_japanese_font() {
            fonts.font_data.insert(FONT_CJK_JP.to_owned(), data);
            state.jp_loaded = true;
            if !JAPANESE_FONTS_LOADED.load(Ordering::Relaxed) {
                JAPANESE_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Japanese font (first time)");
            }
        }
    }

    if spec.load_chinese_sc {
        if let Some(data) = load_chinese_sc_font() {
            fonts.font_data.insert(FONT_CJK_SC.to_owned(), data);
            state.sc_loaded = true;
            if !CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed) {
                CHINESE_SC_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Simplified Chinese font (first time)");
            }
        }
    }

    if spec.load_chinese_tc {
        if let Some(data) = load_chinese_tc_font() {
            fonts.font_data.insert(FONT_CJK_TC.to_owned(), data);
            state.tc_loaded = true;
            if !CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed) {
                CHINESE_TC_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Traditional Chinese font (first time)");
            }
        }
    }

    if spec.any() {
        info!(
            "CJK fonts state: KR={}, JP={}, SC={}, TC={}",
            state.kr_loaded, state.jp_loaded, state.sc_loaded, state.tc_loaded
        );
    }

    state
}

/// Load all CJK system fonts (legacy function for full loading).
fn load_cjk_fonts(fonts: &mut FontDefinitions) -> CjkFontState {
    load_cjk_fonts_selective(fonts, &CjkLoadSpec::all())
}

/// Add CJK fonts to a font family in the specified order.
fn add_cjk_fallbacks(
    fonts: &mut FontDefinitions,
    family: FontFamily,
    cjk_state: &CjkFontState,
    preference: CjkFontPreference,
) {
    let order = preference.font_order();
    for key in order {
        if cjk_state.is_loaded(key) {
            fonts
                .families
                .entry(family.clone())
                .or_default()
                .push((*key).to_owned());
        }
    }
}

/// Create font definitions with custom fonts loaded.
///
/// This sets up:
/// - Inter as the proportional (UI) font with bold/italic variants
/// - JetBrains Mono as the monospace (code) font with bold/italic variants
/// - Custom named font families for explicit bold/italic access
/// - Optional custom system font
/// - CJK fonts in order based on user preference
pub fn create_font_definitions() -> FontDefinitions {
    create_font_definitions_with_settings(None, CjkFontPreference::Auto, true)
}

/// Create font definitions without loading CJK fonts.
///
/// Use this for faster startup when CJK support is not immediately needed.
/// Call `load_cjk_for_text()` later when CJK text is detected.
pub fn create_font_definitions_lazy() -> FontDefinitions {
    create_font_definitions_with_settings(None, CjkFontPreference::Auto, false)
}

/// Create font definitions with selective CJK font loading.
///
/// This function loads only the specific CJK fonts specified in the `CjkLoadSpec`,
/// enabling memory-efficient font loading based on detected scripts.
pub fn create_font_definitions_with_cjk_spec(
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    spec: &CjkLoadSpec,
) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    // Insert Inter font variants (always available as UI fallback)
    fonts
        .font_data
        .insert(FONT_INTER.to_owned(), FontData::from_static(INTER_REGULAR));
    fonts.font_data.insert(
        FONT_INTER_BOLD.to_owned(),
        FontData::from_static(INTER_BOLD),
    );
    fonts.font_data.insert(
        FONT_INTER_ITALIC.to_owned(),
        FontData::from_static(INTER_ITALIC),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD_ITALIC.to_owned(),
        FontData::from_static(INTER_BOLD_ITALIC),
    );

    // Insert JetBrains Mono font variants
    fonts.font_data.insert(
        FONT_JETBRAINS.to_owned(),
        FontData::from_static(JETBRAINS_REGULAR),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD.to_owned(),
        FontData::from_static(JETBRAINS_BOLD),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_ITALIC),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_BOLD_ITALIC),
    );

    // Load custom font if specified
    let custom_loaded = if let Some(font_name) = custom_font {
        if let Some(data) = load_system_font_by_name(font_name) {
            fonts.font_data.insert(FONT_CUSTOM.to_owned(), data);
            info!("Loaded custom font: {}", font_name);
            true
        } else {
            warn!("Custom font '{}' not found, falling back to Inter", font_name);
            false
        }
    } else {
        false
    };

    // Load only the specified CJK fonts
    let cjk_state = load_cjk_fonts_selective(&mut fonts, spec);

    // Set up Proportional font family
    if custom_loaded {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push(FONT_CUSTOM.to_owned());
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(FONT_INTER.to_owned());

    // Add CJK fallbacks for loaded fonts
    if cjk_state.any_loaded() {
        add_cjk_fallbacks(&mut fonts, FontFamily::Proportional, &cjk_state, cjk_preference);
    }

    // Set up Monospace font family
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if cjk_state.any_loaded() {
        add_cjk_fallbacks(&mut fonts, FontFamily::Monospace, &cjk_state, cjk_preference);
    }

    // Get fallback fonts from default families
    let proportional_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Create custom named font families for explicit style access
    if custom_loaded {
        let mut custom_family = vec![FONT_CUSTOM.to_owned()];
        custom_family.extend(proportional_fallbacks.clone());
        fonts
            .families
            .insert(FontFamily::Name(FONT_CUSTOM.into()), custom_family);
    }

    // Inter variants with JetBrains Mono as fallback for missing glyphs (box-drawing, etc.)
    // Inter doesn't include box-drawing characters (U+2500-U+257F), but JetBrains Mono does.
    // This ensures code comments with decorative lines render correctly.
    let mut inter_family = vec![FONT_INTER.to_owned(), FONT_JETBRAINS.to_owned()];
    inter_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER.into()), inter_family);

    let mut inter_bold_family = vec![FONT_INTER_BOLD.to_owned(), FONT_JETBRAINS_BOLD.to_owned()];
    inter_bold_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER_BOLD.into()), inter_bold_family);

    let mut inter_italic_family = vec![FONT_INTER_ITALIC.to_owned(), FONT_JETBRAINS_ITALIC.to_owned()];
    inter_italic_family.extend(proportional_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_ITALIC.into()),
        inter_italic_family,
    );

    let mut inter_bold_italic_family = vec![FONT_INTER_BOLD_ITALIC.to_owned(), FONT_JETBRAINS_BOLD_ITALIC.to_owned()];
    inter_bold_italic_family.extend(proportional_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_BOLD_ITALIC.into()),
        inter_bold_italic_family,
    );

    // JetBrains Mono variants with monospace fallbacks
    let mut jetbrains_family = vec![FONT_JETBRAINS.to_owned()];
    jetbrains_family.extend(monospace_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_JETBRAINS.into()), jetbrains_family);

    let mut jetbrains_bold_family = vec![FONT_JETBRAINS_BOLD.to_owned()];
    jetbrains_bold_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD.into()),
        jetbrains_bold_family,
    );

    let mut jetbrains_italic_family = vec![FONT_JETBRAINS_ITALIC.to_owned()];
    jetbrains_italic_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_ITALIC.into()),
        jetbrains_italic_family,
    );

    let mut jetbrains_bold_italic_family = vec![FONT_JETBRAINS_BOLD_ITALIC.to_owned()];
    jetbrains_bold_italic_family.extend(monospace_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into()),
        jetbrains_bold_italic_family,
    );

    info!(
        "Loaded fonts with selective CJK: KR={}, JP={}, SC={}, TC={}",
        cjk_state.kr_loaded, cjk_state.jp_loaded, cjk_state.sc_loaded, cjk_state.tc_loaded
    );

    fonts
}

/// Create font definitions with custom settings.
///
/// # Arguments
///
/// * `custom_font` - Optional custom system font name to use as primary editor font
/// * `cjk_preference` - CJK font preference for regional glyph variants
/// * `load_cjk` - Whether to load CJK fonts immediately (false for lazy loading)
pub fn create_font_definitions_with_settings(
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    load_cjk: bool,
) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    // Insert Inter font variants (always available as UI fallback)
    fonts
        .font_data
        .insert(FONT_INTER.to_owned(), FontData::from_static(INTER_REGULAR));
    fonts.font_data.insert(
        FONT_INTER_BOLD.to_owned(),
        FontData::from_static(INTER_BOLD),
    );
    fonts.font_data.insert(
        FONT_INTER_ITALIC.to_owned(),
        FontData::from_static(INTER_ITALIC),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD_ITALIC.to_owned(),
        FontData::from_static(INTER_BOLD_ITALIC),
    );

    // Insert JetBrains Mono font variants
    fonts.font_data.insert(
        FONT_JETBRAINS.to_owned(),
        FontData::from_static(JETBRAINS_REGULAR),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD.to_owned(),
        FontData::from_static(JETBRAINS_BOLD),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_ITALIC),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
        FontData::from_static(JETBRAINS_BOLD_ITALIC),
    );

    // Load custom font if specified
    let custom_loaded = if let Some(font_name) = custom_font {
        if let Some(data) = load_system_font_by_name(font_name) {
            fonts.font_data.insert(FONT_CUSTOM.to_owned(), data);
            info!("Loaded custom font: {}", font_name);
            true
        } else {
            warn!("Custom font '{}' not found, falling back to Inter", font_name);
            false
        }
    } else {
        false
    };

    // Load CJK fonts only if requested (supports lazy loading)
    let cjk_state = if load_cjk {
        load_cjk_fonts(&mut fonts)
    } else {
        info!("Skipping CJK font loading (lazy mode)");
        CjkFontState::default()
    };

    // Set up Proportional font family
    // Order: Custom (if set) -> Inter -> CJK fonts (in preference order)
    if custom_loaded {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push(FONT_CUSTOM.to_owned());
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(FONT_INTER.to_owned());

    // Only add CJK fallbacks if fonts were loaded
    if load_cjk {
        add_cjk_fallbacks(&mut fonts, FontFamily::Proportional, &cjk_state, cjk_preference);
    }

    // Set up Monospace font family
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if load_cjk {
        add_cjk_fallbacks(&mut fonts, FontFamily::Monospace, &cjk_state, cjk_preference);
    }

    // Get fallback fonts from default families for CJK/Korean support
    let proportional_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let monospace_fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Create custom named font families for explicit style access
    // These allow us to directly select bold/italic fonts
    // Each family includes fallbacks for CJK character support

    // Custom font family (if loaded)
    if custom_loaded {
        let mut custom_family = vec![FONT_CUSTOM.to_owned()];
        custom_family.extend(proportional_fallbacks.clone());
        fonts
            .families
            .insert(FontFamily::Name(FONT_CUSTOM.into()), custom_family);
    }

    // Inter variants with JetBrains Mono as fallback for missing glyphs (box-drawing, etc.)
    // Inter doesn't include box-drawing characters (U+2500-U+257F), but JetBrains Mono does.
    // This ensures code comments with decorative lines render correctly.
    let mut inter_family = vec![FONT_INTER.to_owned(), FONT_JETBRAINS.to_owned()];
    inter_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER.into()), inter_family);

    let mut inter_bold_family = vec![FONT_INTER_BOLD.to_owned(), FONT_JETBRAINS_BOLD.to_owned()];
    inter_bold_family.extend(proportional_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_INTER_BOLD.into()), inter_bold_family);

    let mut inter_italic_family = vec![FONT_INTER_ITALIC.to_owned(), FONT_JETBRAINS_ITALIC.to_owned()];
    inter_italic_family.extend(proportional_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_ITALIC.into()),
        inter_italic_family,
    );

    let mut inter_bold_italic_family = vec![FONT_INTER_BOLD_ITALIC.to_owned(), FONT_JETBRAINS_BOLD_ITALIC.to_owned()];
    inter_bold_italic_family.extend(proportional_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_BOLD_ITALIC.into()),
        inter_bold_italic_family,
    );

    // JetBrains Mono variants with monospace fallbacks
    let mut jetbrains_family = vec![FONT_JETBRAINS.to_owned()];
    jetbrains_family.extend(monospace_fallbacks.clone());
    fonts
        .families
        .insert(FontFamily::Name(FONT_JETBRAINS.into()), jetbrains_family);

    let mut jetbrains_bold_family = vec![FONT_JETBRAINS_BOLD.to_owned()];
    jetbrains_bold_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD.into()),
        jetbrains_bold_family,
    );

    let mut jetbrains_italic_family = vec![FONT_JETBRAINS_ITALIC.to_owned()];
    jetbrains_italic_family.extend(monospace_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_ITALIC.into()),
        jetbrains_italic_family,
    );

    let mut jetbrains_bold_italic_family = vec![FONT_JETBRAINS_BOLD_ITALIC.to_owned()];
    jetbrains_bold_italic_family.extend(monospace_fallbacks);
    fonts.families.insert(
        FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into()),
        jetbrains_bold_italic_family,
    );

    info!(
        "Loaded fonts: Inter, JetBrains Mono, CJK={} (preference: {:?}), custom: {}",
        if load_cjk { "loaded" } else { "deferred" },
        cjk_preference,
        custom_font.unwrap_or("none")
    );

    fonts
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Atlas Pre-warming
// ─────────────────────────────────────────────────────────────────────────────

/// Common box-drawing characters used in ASCII diagrams.
/// These are in the Unicode Box Drawing block (U+2500–U+257F).
const BOX_DRAWING_CHARS: &str = "─│┌┐└┘├┤┬┴┼━┃┏┓┗┛┣┫┳┻╋╔╗╚╝╠╣╦╩╬═║▀▄█▌▐░▒▓";

/// Common symbols that might not be in the initial font atlas.
/// Includes arrows, bullets, checkmarks, and common UI symbols.
const COMMON_SYMBOLS: &str = "←→↑↓↔↕⇐⇒⇑⇓⇄⇅↳↵•◦●○■□▪▫◆◇★☆✓✗✘✔✕✖…⋯";

/// Pre-warm the font atlas with commonly used special characters.
///
/// egui's font atlas is built lazily, only rasterizing glyphs when first needed.
/// This can cause box-drawing characters (used in ASCII diagrams) to appear as
/// squares on the first render. By pre-warming the atlas with these characters,
/// we ensure they're available from the start.
///
/// This function queries glyph widths for the characters, which forces egui to
/// rasterize them into the font texture atlas.
fn prewarm_font_atlas(ctx: &egui::Context) {
    // Use a reasonable font size that matches typical editor usage
    let font_id = FontId::new(14.0, FontFamily::Proportional);
    
    // Pre-warm by querying glyph widths - this forces rasterization
    ctx.fonts(|fonts| {
        for c in BOX_DRAWING_CHARS.chars() {
            let _ = fonts.glyph_width(&font_id, c);
        }
        for c in COMMON_SYMBOLS.chars() {
            let _ = fonts.glyph_width(&font_id, c);
        }
    });
    
    // Also pre-warm monospace font for code blocks
    let mono_font_id = FontId::new(14.0, FontFamily::Monospace);
    ctx.fonts(|fonts| {
        for c in BOX_DRAWING_CHARS.chars() {
            let _ = fonts.glyph_width(&mono_font_id, c);
        }
    });
    
    // Bump font generation again after pre-warming to invalidate any galleys
    // that might have been created with incomplete atlas during the first frame
    bump_font_generation();
    
    info!("Pre-warmed font atlas with {} box-drawing and {} symbol characters",
          BOX_DRAWING_CHARS.chars().count(),
          COMMON_SYMBOLS.chars().count());
}

/// Apply custom fonts to an egui context.
///
/// This should be called once during application initialization.
/// Loads all fonts including CJK immediately.
pub fn setup_fonts(ctx: &egui::Context) {
    setup_fonts_with_settings(ctx, None, CjkFontPreference::Auto);
}

/// Apply custom fonts to an egui context with lazy CJK loading.
///
/// This version skips CJK font loading at startup for faster initialization.
/// Call `ensure_cjk_fonts_loaded()` when CJK text is detected.
pub fn setup_fonts_lazy(ctx: &egui::Context) {
    let fonts = create_font_definitions_lazy();
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    // Schedule font atlas pre-warming for the first frame
    // (can't call ctx.fonts() until after Context::run())
    schedule_prewarm();
    info!("Configured fonts in lazy mode (CJK deferred)");
}

/// Apply custom fonts to an egui context with settings.
///
/// # Arguments
///
/// * `ctx` - The egui context
/// * `custom_font` - Optional custom system font name
/// * `cjk_preference` - CJK font preference for regional glyph variants
pub fn setup_fonts_with_settings(
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
) {
    let fonts = create_font_definitions_with_settings(custom_font, cjk_preference, true);
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    // Schedule font atlas pre-warming for the first frame
    schedule_prewarm();
    info!("Configured egui text styles with custom_font={:?}, cjk_preference={:?}", 
          custom_font, cjk_preference);
}

/// Configure text styles for the egui context.
fn configure_text_styles(ctx: &egui::Context) {
    let text_styles: BTreeMap<TextStyle, FontId> = [
        (
            TextStyle::Heading,
            FontId::new(24.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
    ]
    .into();

    ctx.style_mut(|style| {
        style.text_styles = text_styles.clone();
    });
}

/// Reload fonts at runtime with new settings.
///
/// This can be called when font settings change in the UI.
pub fn reload_fonts(
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
) {
    info!("Reloading fonts with custom_font={:?}, cjk_preference={:?}", 
          custom_font, cjk_preference);
    let fonts = create_font_definitions_with_settings(custom_font, cjk_preference, true);
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    // Pre-warm immediately since reload_fonts is called after context is running
    prewarm_font_atlas(ctx);
}

/// Ensure CJK fonts are loaded on-demand (loads ALL CJK fonts).
///
/// This function loads all CJK fonts regardless of what scripts are detected.
/// For more memory-efficient loading, use `load_cjk_for_text()` instead.
///
/// # Arguments
///
/// * `ctx` - The egui context
/// * `custom_font` - Optional custom system font name
/// * `cjk_preference` - CJK font preference for regional glyph variants
///
/// # Returns
///
/// `true` if any new CJK fonts were loaded, `false` if all were already loaded.
pub fn ensure_cjk_fonts_loaded(
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
) -> bool {
    // Load all CJK fonts
    info!("Loading all CJK fonts");
    let fonts = create_font_definitions_with_settings(custom_font, cjk_preference, true);
    ctx.set_fonts(fonts);
    bump_font_generation();
    true
}

/// Load only the CJK fonts needed for specific text content.
///
/// This function detects which CJK scripts are present in the text and loads
/// only the necessary fonts, saving significant memory:
/// - Korean text → loads only Korean font (~15-20MB)
/// - Japanese text → loads only Japanese font (~15-20MB)
/// - Chinese text → loads only Chinese font (~15-20MB based on preference)
///
/// # Arguments
///
/// * `text` - The text to analyze for CJK scripts
/// * `ctx` - The egui context
/// * `custom_font` - Optional custom system font name
/// * `cjk_preference` - CJK font preference (used for Han-only text)
///
/// # Returns
///
/// `true` if any new CJK fonts were loaded, `false` otherwise.
pub fn load_cjk_for_text(
    text: &str,
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
) -> bool {
    // Detect which scripts are in the text
    let detection = detect_cjk_scripts(text);
    
    if !detection.has_any_cjk {
        return false;
    }

    // Determine which fonts we need to load
    let spec = CjkLoadSpec::from_detection(&detection, cjk_preference);

    // Check if we actually need to load anything new
    let needs_korean = spec.load_korean && !KOREAN_FONTS_LOADED.load(Ordering::Relaxed);
    let needs_japanese = spec.load_japanese && !JAPANESE_FONTS_LOADED.load(Ordering::Relaxed);
    let needs_chinese_sc = spec.load_chinese_sc && !CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed);
    let needs_chinese_tc = spec.load_chinese_tc && !CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed);

    if !needs_korean && !needs_japanese && !needs_chinese_sc && !needs_chinese_tc {
        return false; // All needed fonts are already loaded
    }

    info!(
        "Lazily loading CJK fonts for detected scripts: Korean={}, Japanese={}, Han={}",
        detection.has_korean, detection.has_japanese, detection.has_han
    );

    // Rebuild fonts with the new CJK fonts
    let fonts = create_font_definitions_with_cjk_spec(custom_font, cjk_preference, &spec);
    ctx.set_fonts(fonts);
    bump_font_generation();

    true
}

/// Check if text needs CJK fonts and load only the necessary ones.
///
/// This is a convenience function that combines script detection with
/// selective font loading for memory-efficient CJK support.
///
/// # Arguments
///
/// * `text` - The text to check for CJK characters
/// * `ctx` - The egui context
/// * `custom_font` - Optional custom system font name
/// * `cjk_preference` - CJK font preference for regional glyph variants
///
/// # Returns
///
/// `true` if CJK fonts were newly loaded, `false` otherwise.
pub fn check_and_load_cjk_if_needed(
    text: &str,
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
) -> bool {
    load_cjk_for_text(text, ctx, custom_font, cjk_preference)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions for Getting Font Families
// ─────────────────────────────────────────────────────────────────────────────

use crate::config::EditorFont;

/// Get the appropriate font family for styled text based on editor font setting.
///
/// This returns the correct font variant based on bold/italic flags and the
/// user's selected editor font.
///
/// Note: Custom system fonts don't have separate bold/italic variants loaded,
/// so they use the base custom font for all styles. The OS may synthesize
/// bold/italic styles, but this depends on the specific font and platform.
pub fn get_styled_font_family(bold: bool, italic: bool, editor_font: &EditorFont) -> FontFamily {
    match editor_font {
        EditorFont::JetBrainsMono => match (bold, italic) {
            (true, true) => FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into()),
            (true, false) => FontFamily::Name(FONT_JETBRAINS_BOLD.into()),
            (false, true) => FontFamily::Name(FONT_JETBRAINS_ITALIC.into()),
            (false, false) => FontFamily::Name(FONT_JETBRAINS.into()),
        },
        EditorFont::Inter => match (bold, italic) {
            (true, true) => FontFamily::Name(FONT_INTER_BOLD_ITALIC.into()),
            (true, false) => FontFamily::Name(FONT_INTER_BOLD.into()),
            (false, true) => FontFamily::Name(FONT_INTER_ITALIC.into()),
            (false, false) => FontFamily::Name(FONT_INTER.into()),
        },
        // Custom fonts don't have separate bold/italic variants
        // Use the custom font family which has CJK fallbacks
        EditorFont::Custom(_) => FontFamily::Name(FONT_CUSTOM.into()),
    }
}

/// Get the base font family for an editor font (regular weight, no style).
pub fn get_base_font_family(editor_font: &EditorFont) -> FontFamily {
    match editor_font {
        EditorFont::Inter => FontFamily::Name(FONT_INTER.into()),
        EditorFont::JetBrainsMono => FontFamily::Name(FONT_JETBRAINS.into()),
        EditorFont::Custom(_) => FontFamily::Name(FONT_CUSTOM.into()),
    }
}

/// Create a FontId for styled text.
///
/// Convenience function that combines size with the appropriate styled font family.
pub fn styled_font_id(size: f32, bold: bool, italic: bool, editor_font: &EditorFont) -> FontId {
    FontId::new(size, get_styled_font_family(bold, italic, editor_font))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_font_definitions() {
        let fonts = create_font_definitions();

        // Check that all font data is loaded
        assert!(fonts.font_data.contains_key(FONT_INTER));
        assert!(fonts.font_data.contains_key(FONT_INTER_BOLD));
        assert!(fonts.font_data.contains_key(FONT_INTER_ITALIC));
        assert!(fonts.font_data.contains_key(FONT_INTER_BOLD_ITALIC));

        assert!(fonts.font_data.contains_key(FONT_JETBRAINS));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_BOLD));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_ITALIC));
        assert!(fonts.font_data.contains_key(FONT_JETBRAINS_BOLD_ITALIC));

        // Check that font families are set up
        assert!(fonts.families.contains_key(&FontFamily::Proportional));
        assert!(fonts.families.contains_key(&FontFamily::Monospace));
    }

    #[test]
    fn test_get_styled_font_family_inter() {
        // Inter variants
        assert_eq!(
            get_styled_font_family(false, false, &EditorFont::Inter),
            FontFamily::Name(FONT_INTER.into())
        );
        assert_eq!(
            get_styled_font_family(true, false, &EditorFont::Inter),
            FontFamily::Name(FONT_INTER_BOLD.into())
        );
        assert_eq!(
            get_styled_font_family(false, true, &EditorFont::Inter),
            FontFamily::Name(FONT_INTER_ITALIC.into())
        );
        assert_eq!(
            get_styled_font_family(true, true, &EditorFont::Inter),
            FontFamily::Name(FONT_INTER_BOLD_ITALIC.into())
        );
    }

    #[test]
    fn test_get_styled_font_family_jetbrains() {
        // JetBrains Mono variants
        assert_eq!(
            get_styled_font_family(false, false, &EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS.into())
        );
        assert_eq!(
            get_styled_font_family(true, false, &EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_BOLD.into())
        );
        assert_eq!(
            get_styled_font_family(false, true, &EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_ITALIC.into())
        );
        assert_eq!(
            get_styled_font_family(true, true, &EditorFont::JetBrainsMono),
            FontFamily::Name(FONT_JETBRAINS_BOLD_ITALIC.into())
        );
    }

    #[test]
    fn test_get_styled_font_family_custom() {
        // Custom font always returns FONT_CUSTOM
        let custom = EditorFont::Custom("Test Font".to_string());
        assert_eq!(
            get_styled_font_family(false, false, &custom),
            FontFamily::Name(FONT_CUSTOM.into())
        );
        assert_eq!(
            get_styled_font_family(true, true, &custom),
            FontFamily::Name(FONT_CUSTOM.into())
        );
    }

    #[test]
    fn test_styled_font_id() {
        let font_id = styled_font_id(16.0, true, false, &EditorFont::Inter);
        assert_eq!(font_id.size, 16.0);
        assert_eq!(font_id.family, FontFamily::Name(FONT_INTER_BOLD.into()));
    }

    #[test]
    fn test_cjk_font_preference_order() {
        // Test that preference returns correct font order
        assert_eq!(
            CjkFontPreference::Korean.font_order(),
            &["CJK_KR", "CJK_SC", "CJK_TC", "CJK_JP"]
        );
        assert_eq!(
            CjkFontPreference::Japanese.font_order(),
            &["CJK_JP", "CJK_KR", "CJK_SC", "CJK_TC"]
        );
        assert_eq!(
            CjkFontPreference::SimplifiedChinese.font_order(),
            &["CJK_SC", "CJK_TC", "CJK_KR", "CJK_JP"]
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // CJK Detection Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_needs_cjk_chinese() {
        // CJK Unified Ideographs (Chinese characters)
        assert!(needs_cjk("你好世界"));           // Chinese: Hello World
        assert!(needs_cjk("中文测试"));           // Chinese: Chinese test
        assert!(needs_cjk("一"));                 // U+4E00 - start of CJK Unified Ideographs
        assert!(needs_cjk("龿"));                 // U+9FFF - near end of CJK Unified Ideographs
    }

    #[test]
    fn test_needs_cjk_japanese() {
        // Hiragana
        assert!(needs_cjk("こんにちは"));         // Japanese: Hello
        assert!(needs_cjk("ぁ"));                 // U+3041 - start of Hiragana
        assert!(needs_cjk("ゟ"));                 // U+309F - end of Hiragana

        // Katakana
        assert!(needs_cjk("カタカナ"));           // Japanese: Katakana
        assert!(needs_cjk("ァ"));                 // U+30A1 - start of Katakana
        assert!(needs_cjk("ヿ"));                 // U+30FF - end of Katakana

        // Mixed Japanese
        assert!(needs_cjk("日本語"));             // Japanese: Japanese language (uses Kanji)
    }

    #[test]
    fn test_needs_cjk_korean() {
        // Hangul Syllables
        assert!(needs_cjk("안녕하세요"));         // Korean: Hello
        assert!(needs_cjk("가"));                 // U+AC00 - start of Hangul Syllables
        assert!(needs_cjk("힣"));                 // U+D7A3 - near end of Hangul Syllables
        assert!(needs_cjk("한국어"));             // Korean: Korean language
    }

    #[test]
    fn test_needs_cjk_ascii_only() {
        // ASCII/Latin text should NOT need CJK fonts
        assert!(!needs_cjk("Hello World"));
        assert!(!needs_cjk("The quick brown fox"));
        assert!(!needs_cjk(""));
        assert!(!needs_cjk("   "));
        assert!(!needs_cjk("12345"));
        assert!(!needs_cjk("!@#$%^&*()"));
        assert!(!needs_cjk("café résumé naïve"));  // Latin with diacritics
    }

    #[test]
    fn test_needs_cjk_mixed_text() {
        // Mixed CJK and ASCII
        assert!(needs_cjk("Hello 世界"));          // English + Chinese
        assert!(needs_cjk("Test 테스트"));         // English + Korean
        assert!(needs_cjk("Hello こんにちは"));    // English + Japanese
        assert!(needs_cjk("- 你好世界"));          // Markdown list with Chinese
        assert!(needs_cjk("# Header 标题"));       // Markdown header with Chinese
    }

    #[test]
    fn test_needs_cjk_edge_cases() {
        // CJK punctuation and symbols (U+3000-303F)
        assert!(needs_cjk("。"));                  // CJK full stop
        assert!(needs_cjk("、"));                  // CJK comma
        assert!(needs_cjk("「」"));               // CJK brackets

        // CJK Radicals Supplement (U+2E80-2EFF)
        assert!(needs_cjk("⺀"));                  // CJK radical

        // Single CJK character in long ASCII text
        assert!(needs_cjk("This is a very long sentence with one Chinese character: 中"));
    }

    #[test]
    fn test_is_cjk_char_boundaries() {
        // Test exact range boundaries
        assert!(is_cjk_char('\u{4E00}'));   // CJK Unified Ideographs start
        assert!(is_cjk_char('\u{9FFF}'));   // CJK Unified Ideographs end
        assert!(is_cjk_char('\u{3040}'));   // Hiragana start
        assert!(is_cjk_char('\u{309F}'));   // Hiragana end
        assert!(is_cjk_char('\u{30A0}'));   // Katakana start
        assert!(is_cjk_char('\u{30FF}'));   // Katakana end
        assert!(is_cjk_char('\u{AC00}'));   // Hangul Syllables start
        assert!(is_cjk_char('\u{D7AF}'));   // Hangul Syllables end

        // Just outside ranges
        assert!(!is_cjk_char('\u{4DFF}'));  // Just before CJK Unified Ideographs
        assert!(!is_cjk_char('\u{A000}'));  // Just after CJK Unified Ideographs
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Script Detection Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_detect_korean_script() {
        // Pure Korean text should detect Korean only
        let result = detect_cjk_scripts("안녕하세요");
        assert!(result.has_korean);
        assert!(!result.has_japanese);
        assert!(!result.has_han);
        assert!(result.has_any_cjk);

        // Single Hangul character
        let result = detect_cjk_scripts("가");
        assert!(result.has_korean);
        assert!(!result.has_japanese);
    }

    #[test]
    fn test_detect_japanese_script() {
        // Hiragana only
        let result = detect_cjk_scripts("こんにちは");
        assert!(!result.has_korean);
        assert!(result.has_japanese);
        assert!(!result.has_han);
        assert!(result.has_any_cjk);

        // Katakana only
        let result = detect_cjk_scripts("カタカナ");
        assert!(!result.has_korean);
        assert!(result.has_japanese);
        assert!(!result.has_han);

        // Japanese with Kanji
        let result = detect_cjk_scripts("日本語");
        assert!(!result.has_korean);
        assert!(!result.has_japanese); // No Hiragana/Katakana
        assert!(result.has_han);       // Kanji counts as Han
    }

    #[test]
    fn test_detect_chinese_script() {
        // Pure Chinese (Han characters only)
        let result = detect_cjk_scripts("你好世界");
        assert!(!result.has_korean);
        assert!(!result.has_japanese);
        assert!(result.has_han);
        assert!(result.has_any_cjk);
    }

    #[test]
    fn test_detect_mixed_scripts() {
        // Korean + Chinese
        let result = detect_cjk_scripts("한국어 中文");
        assert!(result.has_korean);
        assert!(!result.has_japanese);
        assert!(result.has_han);

        // Japanese + Chinese
        let result = detect_cjk_scripts("こんにちは 你好");
        assert!(!result.has_korean);
        assert!(result.has_japanese);
        assert!(result.has_han);

        // All three scripts
        let result = detect_cjk_scripts("한글 ひらがな 中文");
        assert!(result.has_korean);
        assert!(result.has_japanese);
        assert!(result.has_han);
    }

    #[test]
    fn test_detect_no_cjk() {
        let result = detect_cjk_scripts("Hello World");
        assert!(!result.has_korean);
        assert!(!result.has_japanese);
        assert!(!result.has_han);
        assert!(!result.has_any_cjk);

        let result = detect_cjk_scripts("");
        assert!(!result.has_any_cjk);
    }

    #[test]
    fn test_cjk_load_spec_korean() {
        let detection = CjkScriptDetection {
            has_korean: true,
            has_japanese: false,
            has_han: false,
            has_any_cjk: true,
        };
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Auto);
        assert!(spec.load_korean);
        assert!(!spec.load_japanese);
        assert!(!spec.load_chinese_sc);
        assert!(!spec.load_chinese_tc);
    }

    #[test]
    fn test_cjk_load_spec_japanese() {
        let detection = CjkScriptDetection {
            has_korean: false,
            has_japanese: true,
            has_han: false,
            has_any_cjk: true,
        };
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Auto);
        assert!(!spec.load_korean);
        assert!(spec.load_japanese);
        assert!(!spec.load_chinese_sc);
        assert!(!spec.load_chinese_tc);
    }

    #[test]
    fn test_cjk_load_spec_han_only_uses_preference() {
        // Han only with Korean preference
        let detection = CjkScriptDetection {
            has_korean: false,
            has_japanese: false,
            has_han: true,
            has_any_cjk: true,
        };
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Korean);
        assert!(spec.load_korean);
        assert!(!spec.load_japanese);

        // Han only with Japanese preference
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Japanese);
        assert!(spec.load_japanese);
        assert!(!spec.load_korean);

        // Han only with Simplified Chinese preference
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::SimplifiedChinese);
        assert!(spec.load_chinese_sc);
        assert!(!spec.load_chinese_tc);

        // Han only with Traditional Chinese preference
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::TraditionalChinese);
        assert!(spec.load_chinese_tc);
        assert!(!spec.load_chinese_sc);
    }
}
