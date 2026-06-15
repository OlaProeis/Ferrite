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
use std::sync::{Arc, OnceLock};

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

/// Cached raw bytes of the currently loaded custom font, for HarfRust shaping.
/// Stored as a leaked `&'static [u8]` so `ttf_bytes_for_font_id_shaping` can return `&'static [u8]`.
static CUSTOM_FONT_BYTES: std::sync::Mutex<Option<&'static [u8]>> = std::sync::Mutex::new(None);

/// Last error from custom font loading, used to propagate errors from
/// `create_font_definitions_*` back to `reload_fonts`.
static LAST_CUSTOM_FONT_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

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

// ─────────────────────────────────────────────────────────────────────────────
// Per-Script Complex Script Font Loading State
// ─────────────────────────────────────────────────────────────────────────────

static ARABIC_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static BENGALI_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static DEVANAGARI_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static THAI_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static HEBREW_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static TAMIL_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static GEORGIAN_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static ARMENIAN_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static ETHIOPIC_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static OTHER_INDIC_FONTS_LOADED: AtomicBool = AtomicBool::new(false);
static SOUTHEAST_ASIAN_FONTS_LOADED: AtomicBool = AtomicBool::new(false);

// ─────────────────────────────────────────────────────────────────────────────
// System Locale Detection for CJK Font Preloading
// ─────────────────────────────────────────────────────────────────────────────

use crate::config::CjkFontPreference;

/// Detect the system locale and return the appropriate CJK font to preload.
///
/// This checks the Windows system locale and returns the CJK preference that
/// matches the user's system language. This enables preloading only the ONE
/// CJK font the user likely needs (~20MB) instead of all four (~80MB).
///
/// Returns `None` for non-CJK locales (font loading remains fully lazy).
#[cfg(target_os = "windows")]
pub fn detect_system_cjk_locale() -> Option<CjkFontPreference> {
    // Try Windows API first via GetUserDefaultLocaleName
    // Locale names follow BCP-47 format: "ja-JP", "ko-KR", "zh-CN", "zh-TW", etc.

    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_len: i32) -> i32;
    }

    let mut buffer = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
    let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };

    if len > 0 {
        let locale = String::from_utf16_lossy(&buffer[..(len as usize - 1)]);
        let locale_lower = locale.to_lowercase();

        info!("Detected system locale: {}", locale);

        // Check for CJK locales
        if locale_lower.starts_with("ja") {
            info!("System locale is Japanese - will preload Japanese font");
            return Some(CjkFontPreference::Japanese);
        } else if locale_lower.starts_with("ko") {
            info!("System locale is Korean - will preload Korean font");
            return Some(CjkFontPreference::Korean);
        } else if locale_lower.starts_with("zh-cn")
            || locale_lower.starts_with("zh-hans")
            || locale_lower.starts_with("zh-sg")
        {
            info!("System locale is Simplified Chinese - will preload SC font");
            return Some(CjkFontPreference::SimplifiedChinese);
        } else if locale_lower.starts_with("zh-tw")
            || locale_lower.starts_with("zh-hant")
            || locale_lower.starts_with("zh-hk")
            || locale_lower.starts_with("zh-mo")
        {
            info!("System locale is Traditional Chinese - will preload TC font");
            return Some(CjkFontPreference::TraditionalChinese);
        }
    }

    info!("System locale is not CJK - fonts will load on-demand");
    None
}

#[cfg(target_os = "macos")]
pub fn detect_system_cjk_locale() -> Option<CjkFontPreference> {
    // On macOS, check LANG environment variable or use defaults read
    if let Ok(lang) = std::env::var("LANG") {
        let lang_lower = lang.to_lowercase();
        if lang_lower.starts_with("ja") {
            return Some(CjkFontPreference::Japanese);
        } else if lang_lower.starts_with("ko") {
            return Some(CjkFontPreference::Korean);
        } else if lang_lower.contains("zh_cn") || lang_lower.contains("zh-hans") {
            return Some(CjkFontPreference::SimplifiedChinese);
        } else if lang_lower.contains("zh_tw") || lang_lower.contains("zh-hant") {
            return Some(CjkFontPreference::TraditionalChinese);
        }
    }
    None
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn detect_system_cjk_locale() -> Option<CjkFontPreference> {
    // On Linux, check LANG or LC_ALL environment variables
    let lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();

    if lang.starts_with("ja") {
        Some(CjkFontPreference::Japanese)
    } else if lang.starts_with("ko") {
        Some(CjkFontPreference::Korean)
    } else if lang.contains("zh_cn") || lang.contains("zh.") {
        Some(CjkFontPreference::SimplifiedChinese)
    } else if lang.contains("zh_tw") || lang.contains("zh_hk") {
        Some(CjkFontPreference::TraditionalChinese)
    } else {
        None
    }
}

/// Preload the CJK font for the system locale if detected.
///
/// This should be called early in app initialization to preload the user's
/// likely-needed CJK font based on their system language setting.
/// Only preloads if `cjk_preference` is Auto (user hasn't explicitly chosen).
///
/// Returns `true` if a font was preloaded, `false` otherwise.
pub fn preload_system_locale_cjk_font(
    ctx: &egui::Context,
    cjk_preference: CjkFontPreference,
) -> bool {
    // Only preload based on system locale if user preference is Auto
    if cjk_preference != CjkFontPreference::Auto {
        info!(
            "User has explicit CJK preference {:?} - skipping system locale preload",
            cjk_preference
        );
        return false;
    }

    if let Some(detected) = detect_system_cjk_locale() {
        // Build a spec that loads only the detected locale's font
        let spec = match detected {
            CjkFontPreference::Japanese => CjkLoadSpec {
                load_japanese: true,
                ..Default::default()
            },
            CjkFontPreference::Korean => CjkLoadSpec {
                load_korean: true,
                ..Default::default()
            },
            CjkFontPreference::SimplifiedChinese => CjkLoadSpec {
                load_chinese_sc: true,
                ..Default::default()
            },
            CjkFontPreference::TraditionalChinese => CjkLoadSpec {
                load_chinese_tc: true,
                ..Default::default()
            },
            CjkFontPreference::Auto => return false,
        };

        info!("Preloading CJK font for system locale: {:?}", detected);
        let fonts = create_font_definitions_with_cjk_spec(None, detected, &spec, None);
        ctx.set_fonts(fonts);
        bump_font_generation();
        configure_text_styles(ctx);
        schedule_prewarm();

        return true;
    }

    false
}

/// Preload the CJK font for an explicit user preference.
///
/// When the user has explicitly chosen a CJK font preference (non-Auto),
/// preload that single font at startup so restored tabs render correctly
/// without waiting for lazy detection.
///
/// Returns `true` if a font was preloaded, `false` otherwise.
pub fn preload_explicit_cjk_font(ctx: &egui::Context, cjk_preference: CjkFontPreference) -> bool {
    preload_explicit_cjk_font_with_custom(ctx, cjk_preference, None)
}

/// Preload the CJK font for an explicit preference, preserving custom font.
///
/// Same as `preload_explicit_cjk_font` but also accepts a custom font name
/// so that an existing custom font selection is not lost during font rebuild.
pub fn preload_explicit_cjk_font_with_custom(
    ctx: &egui::Context,
    cjk_preference: CjkFontPreference,
    custom_font: Option<&str>,
) -> bool {
    // Only preload for explicit preferences (not Auto)
    if cjk_preference == CjkFontPreference::Auto {
        return false;
    }

    let spec = match cjk_preference {
        CjkFontPreference::Japanese => CjkLoadSpec {
            load_japanese: true,
            ..Default::default()
        },
        CjkFontPreference::Korean => CjkLoadSpec {
            load_korean: true,
            ..Default::default()
        },
        CjkFontPreference::SimplifiedChinese => CjkLoadSpec {
            load_chinese_sc: true,
            ..Default::default()
        },
        CjkFontPreference::TraditionalChinese => CjkLoadSpec {
            load_chinese_tc: true,
            ..Default::default()
        },
        CjkFontPreference::Auto => return false,
    };

    info!(
        "Preloading CJK font for explicit preference: {:?}",
        cjk_preference
    );
    let fonts = create_font_definitions_with_cjk_spec(custom_font, cjk_preference, &spec, None);
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    schedule_prewarm();

    true
}

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

/// Font family keys for lazily loaded CJK and complex-script system fonts.
pub fn get_loaded_runtime_font_names() -> Vec<String> {
    let mut names = Vec::new();
    let (kr, jp, sc, tc) = get_loaded_cjk_fonts();
    if kr {
        names.push(FONT_CJK_KR.to_string());
    }
    if jp {
        names.push(FONT_CJK_JP.to_string());
    }
    if sc {
        names.push(FONT_CJK_SC.to_string());
    }
    if tc {
        names.push(FONT_CJK_TC.to_string());
    }
    if ARABIC_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_ARABIC.to_string());
    }
    if BENGALI_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_BENGALI.to_string());
    }
    if DEVANAGARI_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_DEVANAGARI.to_string());
    }
    if THAI_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_THAI.to_string());
    }
    if HEBREW_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_HEBREW.to_string());
    }
    if TAMIL_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_TAMIL.to_string());
    }
    if GEORGIAN_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_GEORGIAN.to_string());
    }
    if ARMENIAN_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_ARMENIAN.to_string());
    }
    if ETHIOPIC_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_ETHIOPIC.to_string());
    }
    if OTHER_INDIC_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_OTHER_INDIC.to_string());
    }
    if SOUTHEAST_ASIAN_FONTS_LOADED.load(Ordering::Relaxed) {
        names.push(FONT_SOUTHEAST_ASIAN.to_string());
    }
    names
}

// ─────────────────────────────────────────────────────────────────────────────
// Complex Script Detection
// ─────────────────────────────────────────────────────────────────────────────

// Unicode ranges for complex script detection
const ARABIC: (u32, u32) = (0x0600, 0x06FF);
const ARABIC_SUPPLEMENT: (u32, u32) = (0x0750, 0x077F);
const ARABIC_EXTENDED_A: (u32, u32) = (0x08A0, 0x08FF);
const ARABIC_PRESENTATION_A: (u32, u32) = (0xFB50, 0xFDFF);
const ARABIC_PRESENTATION_B: (u32, u32) = (0xFE70, 0xFEFF);

const BENGALI: (u32, u32) = (0x0980, 0x09FF);
const DEVANAGARI: (u32, u32) = (0x0900, 0x097F);
const DEVANAGARI_EXTENDED: (u32, u32) = (0xA8E0, 0xA8FF);
const THAI: (u32, u32) = (0x0E00, 0x0E7F);
const HEBREW: (u32, u32) = (0x0590, 0x05FF);
const TAMIL: (u32, u32) = (0x0B80, 0x0BFF);
const GEORGIAN: (u32, u32) = (0x10A0, 0x10FF);
const ARMENIAN: (u32, u32) = (0x0530, 0x058F);
const ETHIOPIC: (u32, u32) = (0x1200, 0x137F);

const GUJARATI: (u32, u32) = (0x0A80, 0x0AFF);
const GURMUKHI: (u32, u32) = (0x0A00, 0x0A7F);
const KANNADA: (u32, u32) = (0x0C80, 0x0CFF);
const MALAYALAM: (u32, u32) = (0x0D00, 0x0D7F);
const TELUGU: (u32, u32) = (0x0C00, 0x0C7F);

const MYANMAR: (u32, u32) = (0x1000, 0x109F);
const KHMER: (u32, u32) = (0x1780, 0x17FF);
const SINHALA: (u32, u32) = (0x0D80, 0x0DFF);

/// Result of scanning text for complex (non-Latin, non-CJK) scripts.
#[derive(Debug, Clone, Default)]
pub struct ComplexScriptDetection {
    pub has_arabic: bool,
    pub has_bengali: bool,
    pub has_devanagari: bool,
    pub has_thai: bool,
    pub has_hebrew: bool,
    pub has_tamil: bool,
    pub has_georgian: bool,
    pub has_armenian: bool,
    pub has_ethiopic: bool,
    pub has_other_indic: bool,
    pub has_southeast_asian: bool,
    pub has_any: bool,
}

#[inline]
fn is_arabic_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, ARABIC)
        || in_range(cp, ARABIC_SUPPLEMENT)
        || in_range(cp, ARABIC_EXTENDED_A)
        || in_range(cp, ARABIC_PRESENTATION_A)
        || in_range(cp, ARABIC_PRESENTATION_B)
}

#[inline]
fn is_bengali_char(c: char) -> bool {
    in_range(c as u32, BENGALI)
}

#[inline]
fn is_devanagari_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, DEVANAGARI) || in_range(cp, DEVANAGARI_EXTENDED)
}

#[inline]
fn is_thai_char(c: char) -> bool {
    in_range(c as u32, THAI)
}

#[inline]
fn is_hebrew_char(c: char) -> bool {
    in_range(c as u32, HEBREW)
}

#[inline]
fn is_tamil_char(c: char) -> bool {
    in_range(c as u32, TAMIL)
}

#[inline]
fn is_georgian_char(c: char) -> bool {
    in_range(c as u32, GEORGIAN)
}

#[inline]
fn is_armenian_char(c: char) -> bool {
    in_range(c as u32, ARMENIAN)
}

#[inline]
fn is_ethiopic_char(c: char) -> bool {
    in_range(c as u32, ETHIOPIC)
}

#[inline]
fn is_other_indic_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, GUJARATI)
        || in_range(cp, GURMUKHI)
        || in_range(cp, KANNADA)
        || in_range(cp, MALAYALAM)
        || in_range(cp, TELUGU)
}

#[inline]
fn is_southeast_asian_char(c: char) -> bool {
    let cp = c as u32;
    in_range(cp, MYANMAR) || in_range(cp, KHMER) || in_range(cp, SINHALA)
}

#[inline]
fn is_complex_script_char(c: char) -> bool {
    is_arabic_char(c)
        || is_bengali_char(c)
        || is_devanagari_char(c)
        || is_thai_char(c)
        || is_hebrew_char(c)
        || is_tamil_char(c)
        || is_georgian_char(c)
        || is_armenian_char(c)
        || is_ethiopic_char(c)
        || is_other_indic_char(c)
        || is_southeast_asian_char(c)
}

/// Detect which complex scripts are present in text.
pub fn detect_complex_scripts(text: &str) -> ComplexScriptDetection {
    let mut result = ComplexScriptDetection::default();

    for c in text.chars() {
        if is_arabic_char(c) {
            result.has_arabic = true;
            result.has_any = true;
        } else if is_bengali_char(c) {
            result.has_bengali = true;
            result.has_any = true;
        } else if is_devanagari_char(c) {
            result.has_devanagari = true;
            result.has_any = true;
        } else if is_thai_char(c) {
            result.has_thai = true;
            result.has_any = true;
        } else if is_hebrew_char(c) {
            result.has_hebrew = true;
            result.has_any = true;
        } else if is_tamil_char(c) {
            result.has_tamil = true;
            result.has_any = true;
        } else if is_georgian_char(c) {
            result.has_georgian = true;
            result.has_any = true;
        } else if is_armenian_char(c) {
            result.has_armenian = true;
            result.has_any = true;
        } else if is_ethiopic_char(c) {
            result.has_ethiopic = true;
            result.has_any = true;
        } else if is_other_indic_char(c) {
            result.has_other_indic = true;
            result.has_any = true;
        } else if is_southeast_asian_char(c) {
            result.has_southeast_asian = true;
            result.has_any = true;
        }

        if result.has_arabic
            && result.has_bengali
            && result.has_devanagari
            && result.has_thai
            && result.has_hebrew
            && result.has_tamil
            && result.has_georgian
            && result.has_armenian
            && result.has_ethiopic
            && result.has_other_indic
            && result.has_southeast_asian
        {
            break;
        }
    }

    result
}

/// Check if text contains any complex script characters requiring specialized font support.
pub fn needs_complex_script_fonts(text: &str) -> bool {
    text.chars().any(is_complex_script_char)
}

/// Check if any complex script fonts have been loaded.
pub fn are_complex_script_fonts_loaded() -> bool {
    ARABIC_FONTS_LOADED.load(Ordering::Relaxed)
        || BENGALI_FONTS_LOADED.load(Ordering::Relaxed)
        || DEVANAGARI_FONTS_LOADED.load(Ordering::Relaxed)
        || THAI_FONTS_LOADED.load(Ordering::Relaxed)
        || HEBREW_FONTS_LOADED.load(Ordering::Relaxed)
        || TAMIL_FONTS_LOADED.load(Ordering::Relaxed)
        || GEORGIAN_FONTS_LOADED.load(Ordering::Relaxed)
        || ARMENIAN_FONTS_LOADED.load(Ordering::Relaxed)
        || ETHIOPIC_FONTS_LOADED.load(Ordering::Relaxed)
        || OTHER_INDIC_FONTS_LOADED.load(Ordering::Relaxed)
        || SOUTHEAST_ASIAN_FONTS_LOADED.load(Ordering::Relaxed)
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
    load_system_font_with_preference(None, families)
}

/// Load a system font, trying user preference first (if set), then falling back to candidates.
fn load_system_font_with_preference(
    preference: Option<&str>,
    candidates: &[&str],
) -> Option<FontData> {
    if let Some(pref) = preference {
        if !pref.is_empty() {
            match load_system_font_by_name(pref) {
                Ok(data) => return Some(data),
                Err(reason) => warn!("Preferred font '{}' not available: {}", pref, reason),
            }
        }
    }
    let source = SystemSource::new();
    for family in candidates {
        info!("Attempting to load system font: {}", family);
        if let Ok(handle) =
            source.select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new())
        {
            match handle {
                Handle::Path { path, .. } => {
                    info!("Found system font at: {:?}", path);
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

/// Validate that raw font bytes are a supported single-font format (TTF or OTF).
///
/// Rejects font collections (.ttc/.otc), Type 1, WOFF/WOFF2, and corrupt data.
fn validate_font_bytes(bytes: &[u8], family_name: &str) -> Result<(), String> {
    if bytes.len() < 4 {
        return Err(format!(
            "Font '{family_name}' file is too small ({} bytes)",
            bytes.len()
        ));
    }

    let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    match &magic {
        // TrueType single font
        [0x00, 0x01, 0x00, 0x00] => Ok(()),
        // OpenType (CFF) single font
        b"OTTO" => Ok(()),
        // TrueType/OpenType collection — epaint cannot handle font indices
        b"ttcf" => Err(format!(
            "Font '{family_name}' is a .ttc/.otc collection, which is not supported"
        )),
        // WOFF
        b"wOFF" => Err(format!(
            "Font '{family_name}' is WOFF format, which is not supported"
        )),
        // WOFF2
        b"wOF2" => Err(format!(
            "Font '{family_name}' is WOFF2 format, which is not supported"
        )),
        // Type 1 (starts with '%!')
        [0x25, 0x21, ..] => Err(format!(
            "Font '{family_name}' is Type 1 format, which is not supported"
        )),
        _ => Err(format!(
            "Font '{family_name}' has unrecognized format (magic: {:02x} {:02x} {:02x} {:02x})",
            magic[0], magic[1], magic[2], magic[3]
        )),
    }
}

/// Load a specific system font by exact family name.
///
/// Returns `Ok(FontData)` on success, or `Err(message)` describing why it failed.
/// Validates font bytes and wraps font-kit calls in `catch_unwind` for safety.
fn load_system_font_by_name(family_name: &str) -> Result<FontData, String> {
    let source = SystemSource::new();

    info!("Attempting to load custom font: {}", family_name);
    let handle = source
        .select_best_match(
            &[FamilyName::Title(family_name.to_string())],
            &Properties::new(),
        )
        .map_err(|_| format!("Font '{family_name}' not found on system"))?;

    let raw_bytes = match handle {
        Handle::Path { ref path, .. } => {
            info!("Found custom font at: {:?}", path);
            std::fs::read(path).map_err(|e| format!("Failed to read font file {:?}: {e}", path))?
        }
        Handle::Memory { ref bytes, .. } => {
            info!("Found custom font in memory ({} bytes)", bytes.len());
            bytes.to_vec()
        }
    };

    if raw_bytes.is_empty() {
        return Err(format!("Font '{family_name}' file is empty"));
    }

    validate_font_bytes(&raw_bytes, family_name)?;

    // Wrap FontData construction in catch_unwind to protect against epaint panics
    let family_owned = family_name.to_string();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        FontData::from_owned(raw_bytes)
    })) {
        Ok(data) => Ok(data),
        Err(_) => Err(format!(
            "Font '{family_owned}' caused a panic during loading — file may be corrupt"
        )),
    }
}

/// Ignore whitespace-only custom names so we never attempt to load an empty family (GitHub #133).
#[inline]
fn non_empty_custom_font_name(custom_font: Option<&str>) -> Option<&str> {
    custom_font.and_then(|s| {
        let t = s.trim();
        (!t.is_empty()).then_some(t)
    })
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

/// Phosphor icon font (MIT) for toolbar glyphs — see `egui-phosphor`.
const FONT_PHOSPHOR: &str = "phosphor";

/// Keys for dynamically loaded CJK system fonts
const FONT_CJK_KR: &str = "CJK_KR";
const FONT_CJK_SC: &str = "CJK_SC";
const FONT_CJK_TC: &str = "CJK_TC";
const FONT_CJK_JP: &str = "CJK_JP";

/// Keys for dynamically loaded complex script system fonts
const FONT_ARABIC: &str = "Arabic";
const FONT_BENGALI: &str = "Bengali";
const FONT_DEVANAGARI: &str = "Devanagari";
const FONT_THAI: &str = "Thai";
const FONT_HEBREW: &str = "Hebrew";
const FONT_TAMIL: &str = "Tamil";
const FONT_GEORGIAN: &str = "Georgian";
const FONT_ARMENIAN: &str = "Armenian";
const FONT_ETHIOPIC: &str = "Ethiopic";
const FONT_OTHER_INDIC: &str = "OtherIndic";
const FONT_SOUTHEAST_ASIAN: &str = "SoutheastAsian";

/// Key for custom user-selected font
const FONT_CUSTOM: &str = "Custom";

// ─────────────────────────────────────────────────────────────────────────────
// HarfRust shaping: TTF bytes aligned with egui font families
// ─────────────────────────────────────────────────────────────────────────────

/// Embedded TTF bytes for the primary face used with [`FontFamily::Proportional`] (Inter Regular).
#[must_use]
pub fn ttf_bytes_proportional_regular() -> &'static [u8] {
    INTER_REGULAR
}

/// Embedded TTF bytes for the primary face used with [`FontFamily::Monospace`] (JetBrains Mono Regular).
#[must_use]
pub fn ttf_bytes_monospace_regular() -> &'static [u8] {
    JETBRAINS_REGULAR
}

/// Vertical line height in **points** for `font_id`, from egui's loaded font metrics.
///
/// Prefer this over laying out an empty string: with egui 0.34's skrifa backend an
/// empty galley can report zero height while [`Fonts::row_height`] stays correct.
#[must_use]
pub fn row_height_for_font(ctx: &egui::Context, font_id: &FontId) -> f32 {
    ctx.fonts_mut(|fonts| fonts.row_height(font_id))
}

/// Map an egui [`FontId`] to embedded font bytes for [`harfrust`](crate::editor::ferrite::shaping).
///
/// Named Inter/JetBrains families resolve to the matching weight/style TTF.
/// `FONT_CUSTOM` resolves to the cached custom font bytes when available.
/// Unknown names fall back to Inter Regular (closest default for multilingual text).
#[must_use]
pub fn ttf_bytes_for_font_id_shaping(font_id: &FontId) -> &'static [u8] {
    match &font_id.family {
        FontFamily::Proportional => INTER_REGULAR,
        FontFamily::Monospace => JETBRAINS_REGULAR,
        FontFamily::Name(name) => match name.as_ref() {
            FONT_INTER => INTER_REGULAR,
            FONT_INTER_BOLD => INTER_BOLD,
            FONT_INTER_ITALIC => INTER_ITALIC,
            FONT_INTER_BOLD_ITALIC => INTER_BOLD_ITALIC,
            FONT_JETBRAINS => JETBRAINS_REGULAR,
            FONT_JETBRAINS_BOLD => JETBRAINS_BOLD,
            FONT_JETBRAINS_ITALIC => JETBRAINS_ITALIC,
            FONT_JETBRAINS_BOLD_ITALIC => JETBRAINS_BOLD_ITALIC,
            FONT_CUSTOM => CUSTOM_FONT_BYTES
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .unwrap_or(INTER_REGULAR),
            _ => INTER_REGULAR,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Loading
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Per-Script Complex Script Font Loading
// ─────────────────────────────────────────────────────────────────────────────

fn load_arabic_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Geeza Pro",
        "Al Nile",
        "Arabic Typesetting",
        "Segoe UI",
        "Noto Sans Arabic",
        "Noto Naskh Arabic",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_bengali_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Bangla MN",
        "Bangla Sangam MN",
        "Nirmala UI",
        "Vrinda",
        "Noto Sans Bengali",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_devanagari_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Devanagari MT",
        "Kohinoor Devanagari",
        "Nirmala UI",
        "Mangal",
        "Noto Sans Devanagari",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_thai_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Thonburi",
        "Sathu",
        "Leelawadee UI",
        "Tahoma",
        "Noto Sans Thai",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_hebrew_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Arial Hebrew",
        "Lucida Grande",
        "David",
        "Segoe UI",
        "Noto Sans Hebrew",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_tamil_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Tamil MN",
        "Tamil Sangam MN",
        "Nirmala UI",
        "Latha",
        "Noto Sans Tamil",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_georgian_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = ["Segoe UI", "Noto Sans Georgian"];
    load_system_font_with_preference(preference, &candidates)
}

fn load_armenian_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = ["Segoe UI", "Noto Sans Armenian"];
    load_system_font_with_preference(preference, &candidates)
}

fn load_ethiopic_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = ["Kefa", "Nyala", "Noto Sans Ethiopic"];
    load_system_font_with_preference(preference, &candidates)
}

fn load_other_indic_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Nirmala UI",
        "Noto Sans Gujarati",
        "Noto Sans Gurmukhi",
        "Noto Sans Kannada",
        "Noto Sans Malayalam",
        "Noto Sans Telugu",
    ];
    load_system_font_with_preference(preference, &candidates)
}

fn load_southeast_asian_font(preference: Option<&str>) -> Option<FontData> {
    let candidates = [
        "Myanmar MN",
        "Myanmar Text",
        "Noto Sans Myanmar",
        "Noto Sans Khmer",
        "Noto Sans Sinhala",
    ];
    load_system_font_with_preference(preference, &candidates)
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

        // If Han characters detected, ALWAYS load a Chinese font as fallback.
        // Korean and Japanese fonts don't contain all Han characters, so we need
        // a Chinese font to ensure complete coverage of Han characters.
        // The user's preference determines which Chinese variant to load.
        if detection.has_han {
            match preference {
                CjkFontPreference::Korean => {
                    // User prefers Korean, but still need Chinese for Han coverage
                    spec.load_chinese_sc = true;
                }
                CjkFontPreference::Japanese => {
                    // Japanese fonts have good Han coverage, but add Chinese as backup
                    spec.load_chinese_sc = true;
                }
                CjkFontPreference::SimplifiedChinese | CjkFontPreference::Auto => {
                    spec.load_chinese_sc = true;
                }
                CjkFontPreference::TraditionalChinese => {
                    spec.load_chinese_tc = true;
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
            fonts
                .font_data
                .insert(FONT_CJK_KR.to_owned(), Arc::new(data));
            state.kr_loaded = true;
            if !KOREAN_FONTS_LOADED.load(Ordering::Relaxed) {
                KOREAN_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Korean font (first time)");
            }
        }
    }

    if spec.load_japanese {
        if let Some(data) = load_japanese_font() {
            fonts
                .font_data
                .insert(FONT_CJK_JP.to_owned(), Arc::new(data));
            state.jp_loaded = true;
            if !JAPANESE_FONTS_LOADED.load(Ordering::Relaxed) {
                JAPANESE_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Japanese font (first time)");
            }
        }
    }

    if spec.load_chinese_sc {
        if let Some(data) = load_chinese_sc_font() {
            fonts
                .font_data
                .insert(FONT_CJK_SC.to_owned(), Arc::new(data));
            state.sc_loaded = true;
            if !CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed) {
                CHINESE_SC_FONTS_LOADED.store(true, Ordering::Relaxed);
                info!("Loaded Simplified Chinese font (first time)");
            }
        }
    }

    if spec.load_chinese_tc {
        if let Some(data) = load_chinese_tc_font() {
            fonts
                .font_data
                .insert(FONT_CJK_TC.to_owned(), Arc::new(data));
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

// ─────────────────────────────────────────────────────────────────────────────
// Complex Script Font Loading Infrastructure
// ─────────────────────────────────────────────────────────────────────────────

/// Specification of which complex script fonts to load.
#[derive(Debug, Clone, Default)]
pub struct ComplexScriptLoadSpec {
    pub load_arabic: bool,
    pub load_bengali: bool,
    pub load_devanagari: bool,
    pub load_thai: bool,
    pub load_hebrew: bool,
    pub load_tamil: bool,
    pub load_georgian: bool,
    pub load_armenian: bool,
    pub load_ethiopic: bool,
    pub load_other_indic: bool,
    pub load_southeast_asian: bool,
}

impl ComplexScriptLoadSpec {
    /// Create spec from script detection, including already-loaded fonts.
    pub fn from_detection(detection: &ComplexScriptDetection) -> Self {
        let mut spec = Self::from_loaded_flags();

        if detection.has_arabic {
            spec.load_arabic = true;
        }
        if detection.has_bengali {
            spec.load_bengali = true;
        }
        if detection.has_devanagari {
            spec.load_devanagari = true;
        }
        if detection.has_thai {
            spec.load_thai = true;
        }
        if detection.has_hebrew {
            spec.load_hebrew = true;
        }
        if detection.has_tamil {
            spec.load_tamil = true;
        }
        if detection.has_georgian {
            spec.load_georgian = true;
        }
        if detection.has_armenian {
            spec.load_armenian = true;
        }
        if detection.has_ethiopic {
            spec.load_ethiopic = true;
        }
        if detection.has_other_indic {
            spec.load_other_indic = true;
        }
        if detection.has_southeast_asian {
            spec.load_southeast_asian = true;
        }

        spec
    }

    /// Build spec from the atomic bools (already-loaded fonts only).
    pub fn from_loaded_flags() -> Self {
        Self {
            load_arabic: ARABIC_FONTS_LOADED.load(Ordering::Relaxed),
            load_bengali: BENGALI_FONTS_LOADED.load(Ordering::Relaxed),
            load_devanagari: DEVANAGARI_FONTS_LOADED.load(Ordering::Relaxed),
            load_thai: THAI_FONTS_LOADED.load(Ordering::Relaxed),
            load_hebrew: HEBREW_FONTS_LOADED.load(Ordering::Relaxed),
            load_tamil: TAMIL_FONTS_LOADED.load(Ordering::Relaxed),
            load_georgian: GEORGIAN_FONTS_LOADED.load(Ordering::Relaxed),
            load_armenian: ARMENIAN_FONTS_LOADED.load(Ordering::Relaxed),
            load_ethiopic: ETHIOPIC_FONTS_LOADED.load(Ordering::Relaxed),
            load_other_indic: OTHER_INDIC_FONTS_LOADED.load(Ordering::Relaxed),
            load_southeast_asian: SOUTHEAST_ASIAN_FONTS_LOADED.load(Ordering::Relaxed),
        }
    }

    pub fn any(&self) -> bool {
        self.load_arabic
            || self.load_bengali
            || self.load_devanagari
            || self.load_thai
            || self.load_hebrew
            || self.load_tamil
            || self.load_georgian
            || self.load_armenian
            || self.load_ethiopic
            || self.load_other_indic
            || self.load_southeast_asian
    }
}

/// Track which complex script fonts were successfully loaded in a single build.
#[derive(Default, Clone)]
struct ComplexScriptFontState {
    arabic: bool,
    bengali: bool,
    devanagari: bool,
    thai: bool,
    hebrew: bool,
    tamil: bool,
    georgian: bool,
    armenian: bool,
    ethiopic: bool,
    other_indic: bool,
    southeast_asian: bool,
}

impl ComplexScriptFontState {
    fn any_loaded(&self) -> bool {
        self.arabic
            || self.bengali
            || self.devanagari
            || self.thai
            || self.hebrew
            || self.tamil
            || self.georgian
            || self.armenian
            || self.ethiopic
            || self.other_indic
            || self.southeast_asian
    }
}

/// All complex script font keys in a fixed order for the fallback chain.
const COMPLEX_SCRIPT_FONT_KEYS: &[&str] = &[
    FONT_ARABIC,
    FONT_HEBREW,
    FONT_DEVANAGARI,
    FONT_BENGALI,
    FONT_TAMIL,
    FONT_OTHER_INDIC,
    FONT_THAI,
    FONT_SOUTHEAST_ASIAN,
    FONT_GEORGIAN,
    FONT_ARMENIAN,
    FONT_ETHIOPIC,
];

/// Type for per-script font preferences (script_id -> font family name).
pub type ComplexScriptFontPreferences = std::collections::BTreeMap<String, String>;

/// Load complex script system fonts based on specification.
fn load_complex_script_fonts_selective(
    fonts: &mut FontDefinitions,
    spec: &ComplexScriptLoadSpec,
    preferences: Option<&ComplexScriptFontPreferences>,
) -> ComplexScriptFontState {
    let mut state = ComplexScriptFontState::default();

    let pref = |key: &str| -> Option<&str> {
        preferences
            .and_then(|p| p.get(key))
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
    };

    macro_rules! load_script {
        ($spec_field:expr, $loader:ident, $font_key:expr, $state_field:ident, $flag:ident, $name:expr, $pref_key:expr) => {
            if $spec_field {
                let p = pref($pref_key);
                if let Some(data) = $loader(p) {
                    fonts.font_data.insert($font_key.to_owned(), Arc::new(data));
                    state.$state_field = true;
                    if !$flag.load(Ordering::Relaxed) {
                        $flag.store(true, Ordering::Relaxed);
                        info!("Loaded {} font (first time)", $name);
                    }
                }
            }
        };
    }

    load_script!(
        spec.load_arabic,
        load_arabic_font,
        FONT_ARABIC,
        arabic,
        ARABIC_FONTS_LOADED,
        "Arabic",
        "arabic"
    );
    load_script!(
        spec.load_bengali,
        load_bengali_font,
        FONT_BENGALI,
        bengali,
        BENGALI_FONTS_LOADED,
        "Bengali",
        "bengali"
    );
    load_script!(
        spec.load_devanagari,
        load_devanagari_font,
        FONT_DEVANAGARI,
        devanagari,
        DEVANAGARI_FONTS_LOADED,
        "Devanagari",
        "devanagari"
    );
    load_script!(
        spec.load_thai,
        load_thai_font,
        FONT_THAI,
        thai,
        THAI_FONTS_LOADED,
        "Thai",
        "thai"
    );
    load_script!(
        spec.load_hebrew,
        load_hebrew_font,
        FONT_HEBREW,
        hebrew,
        HEBREW_FONTS_LOADED,
        "Hebrew",
        "hebrew"
    );
    load_script!(
        spec.load_tamil,
        load_tamil_font,
        FONT_TAMIL,
        tamil,
        TAMIL_FONTS_LOADED,
        "Tamil",
        "tamil"
    );
    load_script!(
        spec.load_georgian,
        load_georgian_font,
        FONT_GEORGIAN,
        georgian,
        GEORGIAN_FONTS_LOADED,
        "Georgian",
        "georgian"
    );
    load_script!(
        spec.load_armenian,
        load_armenian_font,
        FONT_ARMENIAN,
        armenian,
        ARMENIAN_FONTS_LOADED,
        "Armenian",
        "armenian"
    );
    load_script!(
        spec.load_ethiopic,
        load_ethiopic_font,
        FONT_ETHIOPIC,
        ethiopic,
        ETHIOPIC_FONTS_LOADED,
        "Ethiopic",
        "ethiopic"
    );
    load_script!(
        spec.load_other_indic,
        load_other_indic_font,
        FONT_OTHER_INDIC,
        other_indic,
        OTHER_INDIC_FONTS_LOADED,
        "Other Indic",
        "other_indic"
    );
    load_script!(
        spec.load_southeast_asian,
        load_southeast_asian_font,
        FONT_SOUTHEAST_ASIAN,
        southeast_asian,
        SOUTHEAST_ASIAN_FONTS_LOADED,
        "Southeast Asian",
        "southeast_asian"
    );

    if spec.any() {
        info!("Complex script fonts loaded: {:?}", spec);
    }

    state
}

/// Add loaded complex script fonts to a font family's fallback chain.
fn add_complex_script_fallbacks(
    fonts: &mut FontDefinitions,
    family: FontFamily,
    cs_state: &ComplexScriptFontState,
) {
    let loaded_flags = [
        (FONT_ARABIC, cs_state.arabic),
        (FONT_HEBREW, cs_state.hebrew),
        (FONT_DEVANAGARI, cs_state.devanagari),
        (FONT_BENGALI, cs_state.bengali),
        (FONT_TAMIL, cs_state.tamil),
        (FONT_OTHER_INDIC, cs_state.other_indic),
        (FONT_THAI, cs_state.thai),
        (FONT_SOUTHEAST_ASIAN, cs_state.southeast_asian),
        (FONT_GEORGIAN, cs_state.georgian),
        (FONT_ARMENIAN, cs_state.armenian),
        (FONT_ETHIOPIC, cs_state.ethiopic),
    ];

    for (key, loaded) in &loaded_flags {
        if *loaded {
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
    create_font_definitions_with_settings(None, CjkFontPreference::Auto, true, None)
}

/// Create font definitions without loading CJK fonts.
///
/// Use this for faster startup when CJK support is not immediately needed.
/// Call `load_cjk_for_text()` later when CJK text is detected.
pub fn create_font_definitions_lazy() -> FontDefinitions {
    create_font_definitions_with_settings(None, CjkFontPreference::Auto, false, None)
}

/// Create font definitions with selective CJK font loading.
///
/// This function loads only the specific CJK fonts specified in the `CjkLoadSpec`,
/// enabling memory-efficient font loading based on detected scripts.
pub fn create_font_definitions_with_cjk_spec(
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    spec: &CjkLoadSpec,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> FontDefinitions {
    let custom_font = non_empty_custom_font_name(custom_font);
    let mut fonts = FontDefinitions::default();

    // Insert Inter font variants (always available as UI fallback)
    fonts.font_data.insert(
        FONT_INTER.to_owned(),
        Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD.to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD)),
    );
    fonts.font_data.insert(
        FONT_INTER_ITALIC.to_owned(),
        Arc::new(FontData::from_static(INTER_ITALIC)),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD_ITALIC.to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD_ITALIC)),
    );

    // Insert JetBrains Mono font variants
    fonts.font_data.insert(
        FONT_JETBRAINS.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_REGULAR)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_BOLD)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_ITALIC.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_ITALIC)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_BOLD_ITALIC)),
    );

    // Load custom font if specified
    let custom_loaded = if let Some(font_name) = custom_font {
        match load_system_font_by_name(font_name) {
            Ok(data) => {
                // Cache raw bytes for HarfRust shaping
                let raw: &'static [u8] = Box::leak(data.font.to_vec().into_boxed_slice());
                *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = Some(raw);
                *LAST_CUSTOM_FONT_ERROR
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                fonts
                    .font_data
                    .insert(FONT_CUSTOM.to_owned(), Arc::new(data));
                info!("Loaded custom font: {}", font_name);
                true
            }
            Err(reason) => {
                warn!("Custom font failed: {}", reason);
                *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = None;
                *LAST_CUSTOM_FONT_ERROR
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(reason);
                false
            }
        }
    } else {
        *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *LAST_CUSTOM_FONT_ERROR
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        false
    };

    // Load only the specified CJK fonts
    let cjk_state = load_cjk_fonts_selective(&mut fonts, spec);

    // Load complex script fonts from atomic flags (preserves already-loaded fonts across rebuilds)
    let cs_spec = ComplexScriptLoadSpec::from_loaded_flags();
    let cs_state =
        load_complex_script_fonts_selective(&mut fonts, &cs_spec, complex_script_preferences);

    // Set up Proportional font family
    // Order: Custom (if set) -> Inter -> JetBrains Mono (for box-drawing/symbols) -> CJK -> complex scripts
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
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if cjk_state.any_loaded() {
        add_cjk_fallbacks(
            &mut fonts,
            FontFamily::Proportional,
            &cjk_state,
            cjk_preference,
        );
    }
    if cs_state.any_loaded() {
        add_complex_script_fallbacks(&mut fonts, FontFamily::Proportional, &cs_state);
    }

    // Set up Monospace font family
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if cjk_state.any_loaded() {
        add_cjk_fallbacks(
            &mut fonts,
            FontFamily::Monospace,
            &cjk_state,
            cjk_preference,
        );
    }
    if cs_state.any_loaded() {
        add_complex_script_fallbacks(&mut fonts, FontFamily::Monospace, &cs_state);
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

    let mut inter_italic_family = vec![
        FONT_INTER_ITALIC.to_owned(),
        FONT_JETBRAINS_ITALIC.to_owned(),
    ];
    inter_italic_family.extend(proportional_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_ITALIC.into()),
        inter_italic_family,
    );

    let mut inter_bold_italic_family = vec![
        FONT_INTER_BOLD_ITALIC.to_owned(),
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
    ];
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
        "Loaded fonts: CJK(KR={}, JP={}, SC={}, TC={}), ComplexScript={}",
        cjk_state.kr_loaded,
        cjk_state.jp_loaded,
        cjk_state.sc_loaded,
        cjk_state.tc_loaded,
        cs_state.any_loaded()
    );

    register_phosphor_icon_font(&mut fonts);
    fonts
}

/// Create font definitions with custom settings.
///
/// # Arguments
///
/// * `custom_font` - Optional custom system font name to use as primary editor font
/// * `cjk_preference` - CJK font preference for regional glyph variants
/// * `load_cjk` - Whether to load CJK fonts immediately (false for lazy loading)
/// * `complex_script_preferences` - Optional per-script font preferences

pub fn create_font_definitions_with_settings(
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    load_cjk: bool,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> FontDefinitions {
    let custom_font = non_empty_custom_font_name(custom_font);
    let mut fonts = FontDefinitions::default();

    // Insert Inter font variants (always available as UI fallback)
    fonts.font_data.insert(
        FONT_INTER.to_owned(),
        Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD.to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD)),
    );
    fonts.font_data.insert(
        FONT_INTER_ITALIC.to_owned(),
        Arc::new(FontData::from_static(INTER_ITALIC)),
    );
    fonts.font_data.insert(
        FONT_INTER_BOLD_ITALIC.to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD_ITALIC)),
    );

    // Insert JetBrains Mono font variants
    fonts.font_data.insert(
        FONT_JETBRAINS.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_REGULAR)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_BOLD)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_ITALIC.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_ITALIC)),
    );
    fonts.font_data.insert(
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_BOLD_ITALIC)),
    );

    // Load custom font if specified
    let custom_loaded = if let Some(font_name) = custom_font {
        match load_system_font_by_name(font_name) {
            Ok(data) => {
                let raw: &'static [u8] = Box::leak(data.font.to_vec().into_boxed_slice());
                *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = Some(raw);
                *LAST_CUSTOM_FONT_ERROR
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                fonts
                    .font_data
                    .insert(FONT_CUSTOM.to_owned(), Arc::new(data));
                info!("Loaded custom font: {}", font_name);
                true
            }
            Err(reason) => {
                warn!("Custom font failed: {}", reason);
                *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = None;
                *LAST_CUSTOM_FONT_ERROR
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(reason);
                false
            }
        }
    } else {
        *CUSTOM_FONT_BYTES.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *LAST_CUSTOM_FONT_ERROR
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        false
    };

    // Load CJK fonts only if requested (supports lazy loading)
    let cjk_state = if load_cjk {
        load_cjk_fonts(&mut fonts)
    } else {
        info!("Skipping CJK font loading (lazy mode)");
        CjkFontState::default()
    };

    // Load complex script fonts from atomic flags (preserves already-loaded fonts across rebuilds)
    let cs_spec = ComplexScriptLoadSpec::from_loaded_flags();
    let cs_state =
        load_complex_script_fonts_selective(&mut fonts, &cs_spec, complex_script_preferences);

    // Set up Proportional font family
    // Order: Custom (if set) -> Inter -> JetBrains Mono (box-drawing) -> CJK -> complex scripts
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
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if load_cjk {
        add_cjk_fallbacks(
            &mut fonts,
            FontFamily::Proportional,
            &cjk_state,
            cjk_preference,
        );
    }
    if cs_state.any_loaded() {
        add_complex_script_fallbacks(&mut fonts, FontFamily::Proportional, &cs_state);
    }

    // Set up Monospace font family
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_JETBRAINS.to_owned());

    if load_cjk {
        add_cjk_fallbacks(
            &mut fonts,
            FontFamily::Monospace,
            &cjk_state,
            cjk_preference,
        );
    }
    if cs_state.any_loaded() {
        add_complex_script_fallbacks(&mut fonts, FontFamily::Monospace, &cs_state);
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

    let mut inter_italic_family = vec![
        FONT_INTER_ITALIC.to_owned(),
        FONT_JETBRAINS_ITALIC.to_owned(),
    ];
    inter_italic_family.extend(proportional_fallbacks.clone());
    fonts.families.insert(
        FontFamily::Name(FONT_INTER_ITALIC.into()),
        inter_italic_family,
    );

    let mut inter_bold_italic_family = vec![
        FONT_INTER_BOLD_ITALIC.to_owned(),
        FONT_JETBRAINS_BOLD_ITALIC.to_owned(),
    ];
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

    register_phosphor_icon_font(&mut fonts);
    fonts
}

/// Register the Phosphor icon font for ribbon/toolbar glyphs.
fn register_phosphor_icon_font(fonts: &mut FontDefinitions) {
    egui_phosphor::add_to_fonts(fonts, egui_phosphor::Variant::Regular);
    fonts
        .families
        .entry(FontFamily::Name(FONT_PHOSPHOR.into()))
        .or_default()
        .push(FONT_PHOSPHOR.to_owned());
}

// ─────────────────────────────────────────────────────────────────────────────
// Font Atlas Pre-warming
// ─────────────────────────────────────────────────────────────────────────────

/// Common box-drawing characters used in ASCII diagrams.
/// These are in the Unicode Box Drawing block (U+2500–U+257F).
const BOX_DRAWING_CHARS: &str = "─│┌┐└┘├┤┬┴┼━┃┏┓┗┛┣┫┳┻╋╔╗╚╝╠╣╦╩╬═║▀▄█▌▐░▒▓";

/// Common symbols that might not be in the initial font atlas.
/// Includes arrows, bullets, checkmarks, mathematical brackets, and common UI symbols.
/// Note: ⟨⟩ (U+27E8/U+27E9) are mathematical angle brackets used for HTML indicators in preview.
/// Note: ↻↺ (U+21BB/U+21BA) are clockwise/counter-clockwise arrows for refresh actions.
const COMMON_SYMBOLS: &str = "←→↑↓↔↕⇐⇒⇑⇓⇄⇅↳↵⤵•◦●○■□▪▫◆◇★☆✓✗✘✔✕✖…⋯⟨⟩«»⚠◐↻↺";

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
    ctx.fonts_mut(|fonts| {
        for c in BOX_DRAWING_CHARS.chars() {
            let _ = fonts.glyph_width(&font_id, c);
        }
        for c in COMMON_SYMBOLS.chars() {
            let _ = fonts.glyph_width(&font_id, c);
        }
    });

    // Also pre-warm monospace font for code blocks
    let mono_font_id = FontId::new(14.0, FontFamily::Monospace);
    ctx.fonts_mut(|fonts| {
        for c in BOX_DRAWING_CHARS.chars() {
            let _ = fonts.glyph_width(&mono_font_id, c);
        }
    });

    // Bump font generation again after pre-warming to invalidate any galleys
    // that might have been created with incomplete atlas during the first frame
    bump_font_generation();

    info!(
        "Pre-warmed font atlas with {} box-drawing and {} symbol characters",
        BOX_DRAWING_CHARS.chars().count(),
        COMMON_SYMBOLS.chars().count()
    );
}

/// Apply custom fonts to an egui context.
///
/// This should be called once during application initialization.
/// Loads all fonts including CJK immediately.
pub fn setup_fonts(ctx: &egui::Context) {
    setup_fonts_with_settings(ctx, None, CjkFontPreference::Auto, None);
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
/// * `complex_script_preferences` - Optional per-script font preferences
pub fn setup_fonts_with_settings(
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) {
    let fonts = create_font_definitions_with_settings(
        custom_font,
        cjk_preference,
        true,
        complex_script_preferences,
    );

    // Defensive fallback: if a non-Auto CJK preference resulted in *no CJK fonts
    // being added to this FontDefinitions*, retry with Auto.
    let has_any_cjk_font = fonts.font_data.contains_key("CJK_KR")
        || fonts.font_data.contains_key("CJK_JP")
        || fonts.font_data.contains_key("CJK_SC")
        || fonts.font_data.contains_key("CJK_TC");

    let final_fonts = if cjk_preference != CjkFontPreference::Auto && !has_any_cjk_font {
        warn!(
            "CJK preference {:?} produced no CJK fonts; falling back to Auto",
            cjk_preference
        );
        create_font_definitions_with_settings(
            custom_font,
            CjkFontPreference::Auto,
            true,
            complex_script_preferences,
        )
    } else {
        fonts
    };

    ctx.set_fonts(final_fonts);

    bump_font_generation();
    configure_text_styles(ctx);
    schedule_prewarm();

    info!(
        "Configured egui text styles with custom_font={:?}, cjk_preference={:?}",
        custom_font, cjk_preference
    );
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

    ctx.global_style_mut(|style| {
        style.text_styles = text_styles.clone();
    });
}

/// Reload fonts at runtime with new settings.
///
/// This can be called when font settings change in the UI.
/// IMPORTANT: This only reloads CJK fonts that are ALREADY loaded to avoid
/// loading all 4 CJK fonts (~80MB) just because the preference changed.
/// New CJK fonts are loaded lazily when text containing those scripts is detected.
///
/// Returns `Some(error_message)` if the custom font failed to load (the font
/// system still falls back to Inter so the app keeps working).
pub fn reload_fonts(
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> Option<String> {
    info!(
        "Reloading fonts with custom_font={:?}, cjk_preference={:?}",
        custom_font, cjk_preference
    );

    // Build a CjkLoadSpec from what's ALREADY loaded - don't load new ones
    // This preserves memory by not eagerly loading all CJK fonts
    let spec = CjkLoadSpec {
        load_korean: KOREAN_FONTS_LOADED.load(Ordering::Relaxed),
        load_japanese: JAPANESE_FONTS_LOADED.load(Ordering::Relaxed),
        load_chinese_sc: CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed),
        load_chinese_tc: CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed),
    };

    info!(
        "Reloading with already-loaded CJK fonts: KR={}, JP={}, SC={}, TC={}",
        spec.load_korean, spec.load_japanese, spec.load_chinese_sc, spec.load_chinese_tc
    );

    let fonts = create_font_definitions_with_cjk_spec(
        custom_font,
        cjk_preference,
        &spec,
        complex_script_preferences,
    );

    ctx.set_fonts(fonts);

    bump_font_generation();
    configure_text_styles(ctx);
    // Font atlas cannot be accessed until after the first Context::run()
    schedule_prewarm();

    // Retrieve any error that occurred during custom font loading
    LAST_CUSTOM_FONT_ERROR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
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
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> bool {
    // Load all CJK fonts
    info!("Loading all CJK fonts");
    let fonts = create_font_definitions_with_settings(
        custom_font,
        cjk_preference,
        true,
        complex_script_preferences,
    );
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    schedule_prewarm();
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
/// * `complex_script_preferences` - Optional per-script font preferences (for font rebuild)
///
/// # Returns
///
/// `true` if any new CJK fonts were loaded, `false` otherwise.
pub fn load_cjk_for_text(
    text: &str,
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
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
    let fonts = create_font_definitions_with_cjk_spec(
        custom_font,
        cjk_preference,
        &spec,
        complex_script_preferences,
    );
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);

    // Schedule prewarm for the NEXT frame. This is critical because
    // ctx.set_fonts() only takes effect on the next egui frame. Without this,
    // galley caches get invalidated and rebuilt with the OLD fonts (squares),
    // then on the next frame the generation matches so the cache isn't
    // re-invalidated — leaving stale square glyphs in the raw editor forever.
    // The prewarm bumps font_generation a second time when fonts are active.
    schedule_prewarm();
    ctx.request_repaint();

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
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> bool {
    load_cjk_for_text(
        text,
        ctx,
        custom_font,
        cjk_preference,
        complex_script_preferences,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Lazy Complex Script Font Loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load only the complex script fonts needed for specific text content.
///
/// Detects which scripts are present and loads only the necessary system fonts.
/// Each script font is typically 1-5MB (much lighter than CJK fonts).
///
/// Returns `true` if any new fonts were loaded.
pub fn load_complex_script_fonts_for_text(
    text: &str,
    ctx: &egui::Context,
    custom_font: Option<&str>,
    cjk_preference: CjkFontPreference,
    complex_script_preferences: Option<&ComplexScriptFontPreferences>,
) -> bool {
    let detection = detect_complex_scripts(text);

    if !detection.has_any {
        return false;
    }

    let spec = ComplexScriptLoadSpec::from_detection(&detection);

    // Check if we need to load anything new
    let needs_any_new = (spec.load_arabic && !ARABIC_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_bengali && !BENGALI_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_devanagari && !DEVANAGARI_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_thai && !THAI_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_hebrew && !HEBREW_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_tamil && !TAMIL_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_georgian && !GEORGIAN_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_armenian && !ARMENIAN_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_ethiopic && !ETHIOPIC_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_other_indic && !OTHER_INDIC_FONTS_LOADED.load(Ordering::Relaxed))
        || (spec.load_southeast_asian && !SOUTHEAST_ASIAN_FONTS_LOADED.load(Ordering::Relaxed));

    if !needs_any_new {
        return false;
    }

    info!("Lazily loading complex script fonts for detected scripts");

    // Set atomic flags for newly detected scripts BEFORE rebuild so
    // create_font_definitions_with_cjk_spec picks them up via from_loaded_flags()
    if spec.load_arabic {
        ARABIC_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_bengali {
        BENGALI_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_devanagari {
        DEVANAGARI_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_thai {
        THAI_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_hebrew {
        HEBREW_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_tamil {
        TAMIL_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_georgian {
        GEORGIAN_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_armenian {
        ARMENIAN_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_ethiopic {
        ETHIOPIC_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_other_indic {
        OTHER_INDIC_FONTS_LOADED.store(true, Ordering::Relaxed);
    }
    if spec.load_southeast_asian {
        SOUTHEAST_ASIAN_FONTS_LOADED.store(true, Ordering::Relaxed);
    }

    // Rebuild all font definitions — includes CJK from their atomic flags too
    let cjk_spec = CjkLoadSpec {
        load_korean: KOREAN_FONTS_LOADED.load(Ordering::Relaxed),
        load_japanese: JAPANESE_FONTS_LOADED.load(Ordering::Relaxed),
        load_chinese_sc: CHINESE_SC_FONTS_LOADED.load(Ordering::Relaxed),
        load_chinese_tc: CHINESE_TC_FONTS_LOADED.load(Ordering::Relaxed),
    };

    let fonts = create_font_definitions_with_cjk_spec(
        custom_font,
        cjk_preference,
        &cjk_spec,
        complex_script_preferences,
    );
    ctx.set_fonts(fonts);
    bump_font_generation();
    configure_text_styles(ctx);
    schedule_prewarm();
    ctx.request_repaint();

    true
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
    if let EditorFont::Custom(name) = editor_font {
        if name.trim().is_empty() {
            return get_styled_font_family(bold, italic, &EditorFont::Inter);
        }
    }
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
    if let EditorFont::Custom(name) = editor_font {
        if name.trim().is_empty() {
            return get_base_font_family(&EditorFont::Inter);
        }
    }
    match editor_font {
        // Use Proportional instead of Named family because Named families
        // don't properly inherit CJK fallbacks when fonts are lazily loaded.
        // FontFamily::Proportional has CJK fonts added via add_cjk_fallbacks.
        EditorFont::Inter => FontFamily::Proportional,
        EditorFont::JetBrainsMono => FontFamily::Monospace,
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
    fn test_get_styled_font_family_custom_pending_uses_inter() {
        let pending = EditorFont::Custom(String::new());
        assert_eq!(
            get_styled_font_family(false, false, &pending),
            get_styled_font_family(false, false, &EditorFont::Inter)
        );
        assert_eq!(
            get_base_font_family(&pending),
            get_base_font_family(&EditorFont::Inter)
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
        assert!(needs_cjk("你好世界")); // Chinese: Hello World
        assert!(needs_cjk("中文测试")); // Chinese: Chinese test
        assert!(needs_cjk("一")); // U+4E00 - start of CJK Unified Ideographs
        assert!(needs_cjk("龿")); // U+9FFF - near end of CJK Unified Ideographs
    }

    #[test]
    fn test_needs_cjk_japanese() {
        // Hiragana
        assert!(needs_cjk("こんにちは")); // Japanese: Hello
        assert!(needs_cjk("ぁ")); // U+3041 - start of Hiragana
        assert!(needs_cjk("ゟ")); // U+309F - end of Hiragana

        // Katakana
        assert!(needs_cjk("カタカナ")); // Japanese: Katakana
        assert!(needs_cjk("ァ")); // U+30A1 - start of Katakana
        assert!(needs_cjk("ヿ")); // U+30FF - end of Katakana

        // Mixed Japanese
        assert!(needs_cjk("日本語")); // Japanese: Japanese language (uses Kanji)
    }

    #[test]
    fn test_needs_cjk_korean() {
        // Hangul Syllables
        assert!(needs_cjk("안녕하세요")); // Korean: Hello
        assert!(needs_cjk("가")); // U+AC00 - start of Hangul Syllables
        assert!(needs_cjk("힣")); // U+D7A3 - near end of Hangul Syllables
        assert!(needs_cjk("한국어")); // Korean: Korean language
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
        assert!(!needs_cjk("café résumé naïve")); // Latin with diacritics
    }

    #[test]
    fn test_needs_cjk_mixed_text() {
        // Mixed CJK and ASCII
        assert!(needs_cjk("Hello 世界")); // English + Chinese
        assert!(needs_cjk("Test 테스트")); // English + Korean
        assert!(needs_cjk("Hello こんにちは")); // English + Japanese
        assert!(needs_cjk("- 你好世界")); // Markdown list with Chinese
        assert!(needs_cjk("# Header 标题")); // Markdown header with Chinese
    }

    #[test]
    fn test_needs_cjk_edge_cases() {
        // CJK punctuation and symbols (U+3000-303F)
        assert!(needs_cjk("。")); // CJK full stop
        assert!(needs_cjk("、")); // CJK comma
        assert!(needs_cjk("「」")); // CJK brackets

        // CJK Radicals Supplement (U+2E80-2EFF)
        assert!(needs_cjk("⺀")); // CJK radical

        // Single CJK character in long ASCII text
        assert!(needs_cjk(
            "This is a very long sentence with one Chinese character: 中"
        ));
    }

    #[test]
    fn test_is_cjk_char_boundaries() {
        // Test exact range boundaries
        assert!(is_cjk_char('\u{4E00}')); // CJK Unified Ideographs start
        assert!(is_cjk_char('\u{9FFF}')); // CJK Unified Ideographs end
        assert!(is_cjk_char('\u{3040}')); // Hiragana start
        assert!(is_cjk_char('\u{309F}')); // Hiragana end
        assert!(is_cjk_char('\u{30A0}')); // Katakana start
        assert!(is_cjk_char('\u{30FF}')); // Katakana end
        assert!(is_cjk_char('\u{AC00}')); // Hangul Syllables start
        assert!(is_cjk_char('\u{D7AF}')); // Hangul Syllables end

        // Just outside ranges
        assert!(!is_cjk_char('\u{4DFF}')); // Just before CJK Unified Ideographs
        assert!(!is_cjk_char('\u{A000}')); // Just after CJK Unified Ideographs
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
        assert!(result.has_han); // Kanji counts as Han
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
        // Han-only always loads a Chinese font for Han character coverage,
        // since Korean/Japanese fonts don't contain all Han characters.
        // The preference determines WHICH Chinese variant to load.
        let detection = CjkScriptDetection {
            has_korean: false,
            has_japanese: false,
            has_han: true,
            has_any_cjk: true,
        };

        // Han only with Korean preference → loads Chinese SC for Han coverage
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Korean);
        assert!(
            spec.load_chinese_sc,
            "Korean pref + Han should load Chinese SC for Han coverage"
        );

        // Han only with Japanese preference → loads Chinese SC for Han coverage
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::Japanese);
        assert!(
            spec.load_chinese_sc,
            "Japanese pref + Han should load Chinese SC for Han coverage"
        );

        // Han only with Simplified Chinese preference
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::SimplifiedChinese);
        assert!(spec.load_chinese_sc);
        assert!(!spec.load_chinese_tc);

        // Han only with Traditional Chinese preference
        let spec = CjkLoadSpec::from_detection(&detection, CjkFontPreference::TraditionalChinese);
        assert!(spec.load_chinese_tc);
        assert!(!spec.load_chinese_sc);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Complex Script Detection Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_needs_complex_scripts_arabic() {
        assert!(needs_complex_script_fonts("مرحبا"));
        assert!(needs_complex_script_fonts("Hello مرحبا"));
    }

    #[test]
    fn test_needs_complex_scripts_bengali() {
        assert!(needs_complex_script_fonts("বাংলা"));
    }

    #[test]
    fn test_needs_complex_scripts_devanagari() {
        assert!(needs_complex_script_fonts("हिन्दी"));
    }

    #[test]
    fn test_needs_complex_scripts_thai() {
        assert!(needs_complex_script_fonts("สวัสดี"));
    }

    #[test]
    fn test_needs_complex_scripts_hebrew() {
        assert!(needs_complex_script_fonts("שלום"));
    }

    #[test]
    fn test_needs_complex_scripts_tamil() {
        assert!(needs_complex_script_fonts("தமிழ்"));
    }

    #[test]
    fn test_needs_complex_scripts_georgian() {
        assert!(needs_complex_script_fonts("ქართული"));
    }

    #[test]
    fn test_needs_complex_scripts_armenian() {
        assert!(needs_complex_script_fonts("Հայերեն"));
    }

    #[test]
    fn test_needs_complex_scripts_ethiopic() {
        assert!(needs_complex_script_fonts("ኢትዮጵያ"));
    }

    #[test]
    fn test_needs_complex_scripts_other_indic() {
        assert!(needs_complex_script_fonts("ગુજરાતી")); // Gujarati
        assert!(needs_complex_script_fonts("ਪੰਜਾਬੀ")); // Gurmukhi
        assert!(needs_complex_script_fonts("ಕನ್ನಡ")); // Kannada
        assert!(needs_complex_script_fonts("മലയാളം")); // Malayalam
        assert!(needs_complex_script_fonts("తెలుగు")); // Telugu
    }

    #[test]
    fn test_needs_complex_scripts_southeast_asian() {
        assert!(needs_complex_script_fonts("မြန်မာ")); // Myanmar
        assert!(needs_complex_script_fonts("ខ្មែរ")); // Khmer
        assert!(needs_complex_script_fonts("සිංහල")); // Sinhala
    }

    #[test]
    fn test_no_complex_scripts_ascii() {
        assert!(!needs_complex_script_fonts("Hello World"));
        assert!(!needs_complex_script_fonts("café résumé"));
    }

    #[test]
    fn test_no_complex_scripts_cjk() {
        assert!(!needs_complex_script_fonts("你好世界"));
        assert!(!needs_complex_script_fonts("こんにちは"));
        assert!(!needs_complex_script_fonts("안녕하세요"));
    }

    #[test]
    fn test_detect_complex_scripts_multiple() {
        let detection = detect_complex_scripts("Hello مرحبا বাংলা");
        assert!(detection.has_arabic);
        assert!(detection.has_bengali);
        assert!(!detection.has_thai);
        assert!(detection.has_any);
    }

    #[test]
    fn test_detect_complex_scripts_arabic_ranges() {
        // Basic Arabic
        assert!(is_arabic_char('\u{0600}'));
        assert!(is_arabic_char('\u{06FF}'));
        // Arabic Supplement
        assert!(is_arabic_char('\u{0750}'));
        assert!(is_arabic_char('\u{077F}'));
        // Arabic Presentation Forms-A
        assert!(is_arabic_char('\u{FB50}'));
        assert!(is_arabic_char('\u{FDFF}'));
        // Arabic Presentation Forms-B
        assert!(is_arabic_char('\u{FE70}'));
        assert!(is_arabic_char('\u{FEFF}'));
        // Not Arabic
        assert!(!is_arabic_char('A'));
        assert!(!is_arabic_char('\u{0500}')); // Cyrillic Supplement
    }

    #[test]
    fn test_detect_complex_scripts_devanagari_ranges() {
        assert!(is_devanagari_char('\u{0900}'));
        assert!(is_devanagari_char('\u{097F}'));
        // Devanagari Extended
        assert!(is_devanagari_char('\u{A8E0}'));
        assert!(is_devanagari_char('\u{A8FF}'));
        assert!(!is_devanagari_char('A'));
    }

    #[test]
    fn test_detect_complex_scripts_none() {
        let detection = detect_complex_scripts("Hello World 123");
        assert!(!detection.has_any);
        assert!(!detection.has_arabic);
        assert!(!detection.has_bengali);
        assert!(!detection.has_devanagari);
        assert!(!detection.has_thai);
        assert!(!detection.has_hebrew);
    }

    #[test]
    fn test_detect_complex_scripts_all_families() {
        let text = "مرحبا বাংলা हिन्दी สวัสดี שלום தமிழ் ქართული Հայերեն ኢትዮጵያ ગુજરાતી မြန်မာ";
        let detection = detect_complex_scripts(text);
        assert!(detection.has_arabic);
        assert!(detection.has_bengali);
        assert!(detection.has_devanagari);
        assert!(detection.has_thai);
        assert!(detection.has_hebrew);
        assert!(detection.has_tamil);
        assert!(detection.has_georgian);
        assert!(detection.has_armenian);
        assert!(detection.has_ethiopic);
        assert!(detection.has_other_indic);
        assert!(detection.has_southeast_asian);
        assert!(detection.has_any);
    }

    #[test]
    fn test_complex_script_does_not_detect_cjk() {
        let detection = detect_complex_scripts("你好世界 こんにちは 안녕하세요");
        assert!(!detection.has_any);
    }
}
