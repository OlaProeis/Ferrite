//! Bundled Hunspell dictionary load + personal-word persistence.
//!
//! `load` must run on the worker thread only — `spellbook::Dictionary::new`
//! can take tens to hundreds of milliseconds.

use encoding_rs::WINDOWS_1252;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLED_AFF: &[u8] = include_bytes!("../../assets/dictionaries/en_US.aff");
const BUNDLED_DIC: &[u8] = include_bytes!("../../assets/dictionaries/en_US.dic");

/// Languages served from the binary (SCOWL-derived American English).
const BUNDLED_LANGS: &[&str] = &["en_US", "en-US", "en"];

/// Bundled language id shown in the settings ComboBox.
pub const BUNDLED_LANG_ID: &str = "en_US";

/// Language stems for the settings picker: bundled `en_US` plus every `.dic` in `dir`.
pub fn discover_dictionary_langs(dir: Option<&Path>) -> Vec<String> {
    let mut langs = vec![BUNDLED_LANG_ID.to_string()];
    let Some(dir) = dir else {
        return langs;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return langs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("dic") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem.is_empty() {
            continue;
        }
        if !langs.iter().any(|l| l == stem) {
            langs.push(stem.to_string());
        }
    }
    langs.sort();
    if let Some(idx) = langs.iter().position(|l| l == BUNDLED_LANG_ID) {
        langs.remove(idx);
        langs.insert(0, BUNDLED_LANG_ID.to_string());
    }
    langs
}

/// Hunspell dictionary plus the stem count reported at load time.
pub struct Dictionary {
    inner: spellbook::Dictionary,
    word_count: usize,
}

impl Dictionary {
    /// Load `lang`. Bundled `en_US` is tried first; otherwise `dir/<lang>.aff|.dic`.
    ///
    /// Files are decoded as UTF-8, then WINDOWS_1252 when the `.aff` `SET` line
    /// declares ISO-8859-1 (or UTF-8 decode fails and that SET is present).
    pub fn load(lang: &str, dir: Option<&Path>) -> Result<Self, String> {
        if is_bundled_lang(lang) {
            return Self::from_bytes(BUNDLED_AFF, BUNDLED_DIC);
        }
        let dir = dir.ok_or_else(|| {
            format!("no bundled dictionary for '{lang}' and no dictionary directory set")
        })?;
        Self::load_from_dir(lang, dir)
    }

    fn load_from_dir(lang: &str, dir: &Path) -> Result<Self, String> {
        let aff_path = dir.join(format!("{lang}.aff"));
        let dic_path = dir.join(format!("{lang}.dic"));
        let aff_bytes = fs::read(&aff_path).map_err(|e| {
            format!("failed to read {}: {e}", aff_path.display())
        })?;
        let dic_bytes = fs::read(&dic_path).map_err(|e| {
            format!("failed to read {}: {e}", dic_path.display())
        })?;
        Self::from_bytes(&aff_bytes, &dic_bytes)
    }

    fn from_bytes(aff_bytes: &[u8], dic_bytes: &[u8]) -> Result<Self, String> {
        let aff = decode_hunspell_file(aff_bytes, true, None)?;
        let dic = decode_hunspell_file(dic_bytes, false, Some(&aff))?;
        let inner = spellbook::Dictionary::new(&aff, &dic)
            .map_err(|e| format!("failed to parse Hunspell dictionary: {e}"))?;
        let word_count = dic_stem_count(&dic);
        Ok(Self { inner, word_count })
    }

    pub fn check(&self, word: &str) -> bool {
        self.inner.check(word)
    }

    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.inner.suggest(word, &mut out);
        out
    }

    pub fn add(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }
        if self.inner.add(word).is_ok() {
            self.word_count = self.word_count.saturating_add(1);
        }
    }

    pub fn word_count(&self) -> usize {
        self.word_count
    }
}

fn is_bundled_lang(lang: &str) -> bool {
    BUNDLED_LANGS.iter().any(|l| l.eq_ignore_ascii_case(lang))
}

/// Persist personal words under `<config_dir>/spellcheck/user-words.txt`.
pub fn user_words_path(config_dir: &Path) -> PathBuf {
    config_dir.join("spellcheck").join("user-words.txt")
}

pub fn load_user_words(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

/// Atomic write: temp file in the same directory, then rename.
pub fn save_user_words(path: &Path, words: &HashSet<String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let mut sorted: Vec<&str> = words.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    let mut body = sorted.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    let tmp = path.with_extension("txt.tmp");
    fs::write(&tmp, body.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("failed to replace {}: {e}", path.display())
    })?;
    Ok(())
}

fn decode_hunspell_file(
    bytes: &[u8],
    is_aff: bool,
    aff_text: Option<&str>,
) -> Result<String, String> {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok(s.to_owned());
    }
    let use_1252 = if is_aff {
        aff_declares_iso8859_1_bytes(bytes)
    } else {
        aff_text
            .map(aff_declares_iso8859_1)
            .unwrap_or_else(|| aff_declares_iso8859_1_bytes(bytes))
    };
    if use_1252 {
        let (cow, _, _) = WINDOWS_1252.decode(bytes);
        return Ok(cow.into_owned());
    }
    Err("dictionary file is not valid UTF-8 and SET is not ISO8859-1".into())
}

fn aff_declares_iso8859_1(aff: &str) -> bool {
    aff.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .take(32)
        .any(|l| {
            let Some(rest) = l.strip_prefix("SET") else {
                return false;
            };
            let enc = rest.trim().replace(['-', '_'], "");
            enc.eq_ignore_ascii_case("ISO88591") || enc.eq_ignore_ascii_case("ISO8859")
        })
}

fn aff_declares_iso8859_1_bytes(bytes: &[u8]) -> bool {
    let (cow, _, _) = WINDOWS_1252.decode(bytes);
    aff_declares_iso8859_1(&cow)
}

fn dic_stem_count(dic: &str) -> usize {
    dic.lines()
        .next()
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| dic.lines().count().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bundled_en_us_loads() {
        let dict = Dictionary::load("en_US", None).expect("bundled en_US");
        assert!(dict.word_count() > 1_000);
    }

    #[test]
    fn check_hello_and_helo() {
        let dict = Dictionary::load("en_US", None).unwrap();
        assert!(dict.check("hello"));
        assert!(!dict.check("helo"));
    }

    #[test]
    fn suggest_helo_contains_hello() {
        let dict = Dictionary::load("en_US", None).unwrap();
        let suggestions = dict.suggest("helo");
        assert!(
            suggestions.iter().any(|s| s == "hello"),
            "expected hello in {suggestions:?}"
        );
    }

    #[test]
    fn user_words_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = user_words_path(tmp.path());
        let mut dict = Dictionary::load("en_US", None).unwrap();
        assert!(!dict.check("foobarbazxyz"));

        dict.add("foobarbazxyz");
        assert!(dict.check("foobarbazxyz"));

        let mut words = HashSet::new();
        words.insert("foobarbazxyz".into());
        save_user_words(&path, &words).unwrap();
        assert!(path.exists());

        let loaded = load_user_words(&path);
        assert_eq!(loaded, vec!["foobarbazxyz".to_string()]);

        let mut reloaded = Dictionary::load("en_US", None).unwrap();
        assert!(!reloaded.check("foobarbazxyz"));
        for w in loaded {
            reloaded.add(&w);
        }
        assert!(reloaded.check("foobarbazxyz"));
    }

    #[test]
    fn load_from_dir_custom_lang() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("xx_XX.aff"), BUNDLED_AFF).unwrap();
        fs::write(tmp.path().join("xx_XX.dic"), BUNDLED_DIC).unwrap();
        let dict = Dictionary::load("xx_XX", Some(tmp.path())).unwrap();
        assert!(dict.check("hello"));
    }

    #[test]
    fn iso8859_1_fallback_when_set_declares_it() {
        let tmp = tempfile::tempdir().unwrap();
        // Minimal aff declaring Latin-1; bytes are not valid UTF-8 (0xE9 = é).
        let aff = b"SET ISO8859-1\nTRY esianrtolcdugmphbyfvkwz\n";
        let mut dic = Vec::from(b"1\n");
        dic.extend_from_slice(b"caf\xe9\n");
        fs::write(tmp.path().join("la_XX.aff"), aff).unwrap();
        fs::write(tmp.path().join("la_XX.dic"), &dic).unwrap();
        let result = Dictionary::load("la_XX", Some(tmp.path()));
        // spellbook may or may not accept this tiny aff; decoding must not fail
        // with a UTF-8 error.
        if let Err(e) = result {
            assert!(
                !e.contains("not valid UTF-8"),
                "expected 1252 fallback, got {e}"
            );
        }
    }
}
