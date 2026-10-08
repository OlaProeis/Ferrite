//! Spellcheck engine: Hunspell dictionary, Markdown tokenizer, background worker.

mod dictionary;
mod tokenize;
mod worker;

#[allow(unused_imports)]
pub use dictionary::{discover_dictionary_langs, load_user_words, user_words_path, Dictionary};
#[allow(unused_imports)]
pub use tokenize::{words_to_check, LineContext, TokenizeOptions};
#[allow(unused_imports)]
pub use worker::{Request, Response};

use crate::config::get_config_dir;
use crate::lsp::state::DiagnosticEntry;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Debounce after the last content edit before sending a new Check.
pub const CHECK_DEBOUNCE: Duration = Duration::from_millis(250);

/// Maximum spell diagnostics appended as squiggles.
pub const MAX_SQUIGGLES: usize = 300;

/// Extra lines around the raw viewport included in each Check.
pub const VIEWPORT_PAD_LINES: usize = 60;

/// Key that identifies a visible-window Check request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellcheckWindowKey {
    pub tab_id: usize,
    pub version: u64,
    pub first_line: usize,
    pub last_line: usize,
}

/// Whether a new Check should be sent for `new_key`.
///
/// A request is issued only when the key changed **and** at least `debounce`
/// has elapsed since `last_content_change` (`None` means no recent edit).
pub fn should_request_check(
    last_key: Option<&SpellcheckWindowKey>,
    new_key: &SpellcheckWindowKey,
    last_content_change: Option<Instant>,
    now: Instant,
    debounce: Duration,
) -> bool {
    if last_key == Some(new_key) {
        return false;
    }
    match last_content_change {
        None => true,
        Some(t) => now.saturating_duration_since(t) >= debounce,
    }
}

/// Keep up to `cap` diagnostics nearest the viewport midpoint.
pub fn cap_diagnostics_nearest_viewport(
    diags: &[DiagnosticEntry],
    viewport_first: usize,
    viewport_last: usize,
    cap: usize,
) -> Vec<DiagnosticEntry> {
    if diags.len() <= cap {
        return diags.to_vec();
    }
    let mid = viewport_first.saturating_add(viewport_last) / 2;
    let mut indexed: Vec<(usize, DiagnosticEntry)> = diags
        .iter()
        .cloned()
        .enumerate()
        .collect();
    indexed.sort_by_key(|(i, d)| (d.start_line.abs_diff(mid), *i));
    indexed.truncate(cap);
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, d)| d).collect()
}

/// Cached diagnostics for one tab from the last worker reply.
#[derive(Debug, Clone)]
pub struct SpellResult {
    pub version: u64,
    pub first_line: usize,
    pub diags: Vec<DiagnosticEntry>,
}

/// UI-thread facade over the spellcheck worker.
pub struct SpellcheckService {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    thread: Option<JoinHandle<()>>,
    results: HashMap<usize, SpellResult>,
    suggestions: HashMap<String, Vec<String>>,
    loaded: Option<(String, usize)>,
    load_error: Option<(String, String)>,
    requested_lang: String,
    requested_dir: Option<PathBuf>,
    last_check_key: Option<SpellcheckWindowKey>,
    suggest_requested: HashMap<String, ()>,
    ignore_all_caps: bool,
    ignore_words_with_digits: bool,
}

impl fmt::Debug for SpellcheckService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpellcheckService")
            .field("tabs", &self.results.len())
            .field("loaded", &self.loaded)
            .finish_non_exhaustive()
    }
}

impl SpellcheckService {
    /// Start a worker with bundled `en_US` and the app config directory.
    pub fn new() -> Self {
        let config_dir = get_config_dir().unwrap_or_else(|_| std::env::temp_dir());
        Self::spawn("en_US".into(), None, config_dir)
    }

    pub fn spawn(lang: String, dict_dir: Option<PathBuf>, config_dir: PathBuf) -> Self {
        let ch = worker::spawn(lang.clone(), dict_dir.clone(), config_dir);
        Self {
            tx: ch.tx,
            rx: ch.rx,
            thread: Some(ch.thread),
            results: HashMap::new(),
            suggestions: HashMap::new(),
            loaded: None,
            load_error: None,
            requested_lang: lang,
            requested_dir: dict_dir,
            last_check_key: None,
            suggest_requested: HashMap::new(),
            ignore_all_caps: true,
            ignore_words_with_digits: true,
        }
    }

    pub fn request_check(&self, tab_id: usize, version: u64, first_line: usize, lines: Vec<String>) {
        let _ = self.tx.send(Request::Check {
            tab_id,
            version,
            first_line,
            lines,
        });
    }

    pub fn request_suggest(&self, word: impl Into<String>) {
        let _ = self.tx.send(Request::Suggest { word: word.into() });
    }

    pub fn add_word(&mut self, word: impl Into<String>) {
        let word = word.into();
        self.drop_diagnostics_for_word(&word);
        let _ = self.tx.send(Request::AddWord(word));
    }

    pub fn ignore_word(&mut self, word: impl Into<String>) {
        let word = word.into();
        self.drop_diagnostics_for_word(&word);
        let _ = self.tx.send(Request::IgnoreWord(word));
    }

    pub fn reload(&mut self, lang: impl Into<String>, dir: Option<PathBuf>) {
        self.requested_lang = lang.into();
        self.requested_dir = dir.clone();
        self.loaded = None;
        self.load_error = None;
        self.last_check_key = None;
        self.results.clear();
        let _ = self.tx.send(Request::Reload {
            lang: self.requested_lang.clone(),
            dir,
        });
    }

    pub fn set_options(&mut self, ignore_all_caps: bool, ignore_words_with_digits: bool) {
        if self.ignore_all_caps == ignore_all_caps
            && self.ignore_words_with_digits == ignore_words_with_digits
        {
            return;
        }
        self.ignore_all_caps = ignore_all_caps;
        self.ignore_words_with_digits = ignore_words_with_digits;
        self.last_check_key = None;
        let _ = self.tx.send(Request::SetOptions {
            ignore_all_caps,
            ignore_words_with_digits,
        });
    }

    pub fn requested_lang(&self) -> &str {
        &self.requested_lang
    }

    pub fn requested_dir(&self) -> Option<&Path> {
        self.requested_dir.as_deref()
    }

    #[allow(dead_code)]
    pub fn clear_results(&mut self) {
        self.results.clear();
        self.last_check_key = None;
    }

    #[allow(dead_code)]
    pub fn last_check_key(&self) -> Option<&SpellcheckWindowKey> {
        self.last_check_key.as_ref()
    }

    /// Drop cached squiggles whose message refers to `word` (immediate UI update).
    pub fn drop_diagnostics_for_word(&mut self, word: &str) {
        let needle = format!("Unknown word: {word}");
        for result in self.results.values_mut() {
            result.diags.retain(|d| d.message != needle);
        }
    }

    pub fn request_suggest_once(&mut self, word: &str) {
        if self.suggestions.contains_key(word) || self.suggest_requested.contains_key(word) {
            return;
        }
        self.suggest_requested.insert(word.to_string(), ());
        self.request_suggest(word);
    }

    /// Send Check when the window key changed and the edit debounce has elapsed.
    pub fn maybe_request_check(
        &mut self,
        key: SpellcheckWindowKey,
        last_content_change: Option<Instant>,
        now: Instant,
        lines: Vec<String>,
    ) -> bool {
        if !should_request_check(
            self.last_check_key.as_ref(),
            &key,
            last_content_change,
            now,
            CHECK_DEBOUNCE,
        ) {
            return false;
        }
        self.request_check(key.tab_id, key.version, key.first_line, lines);
        self.last_check_key = Some(key);
        true
    }

    /// Drain worker responses. Returns `true` if any message was stored.
    pub fn poll(&mut self) -> bool {
        let mut any = false;
        while let Ok(resp) = self.rx.try_recv() {
            any = true;
            match resp {
                Response::Diagnostics {
                    tab_id,
                    version,
                    first_line,
                    diags,
                } => {
                    self.results.insert(
                        tab_id,
                        SpellResult {
                            version,
                            first_line,
                            diags,
                        },
                    );
                }
                Response::Suggestions { word, list } => {
                    self.suggest_requested.remove(&word);
                    self.suggestions.insert(word, list);
                }
                Response::Loaded { lang, word_count } => {
                    self.loaded = Some((lang, word_count));
                    self.load_error = None;
                }
                Response::LoadFailed { lang, error } => {
                    self.load_error = Some((lang, error));
                }
            }
        }
        any
    }

    /// `None` when there is no result or the stored version does not match.
    pub fn diagnostics_for(&self, tab_id: usize, version: u64) -> Option<&[DiagnosticEntry]> {
        let result = self.results.get(&tab_id)?;
        if result.version != version {
            return None;
        }
        Some(result.diags.as_slice())
    }

    pub fn suggestions_for(&self, word: &str) -> Option<&[String]> {
        self.suggestions.get(word).map(Vec::as_slice)
    }

    pub fn loaded(&self) -> Option<&(String, usize)> {
        self.loaded.as_ref()
    }

    pub fn load_error(&self) -> Option<&(String, String)> {
        self.load_error.as_ref()
    }

    pub fn shutdown(&mut self) {
        let _ = self.tx.send(Request::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Default for SpellcheckService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SpellcheckService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_until_loaded(svc: &mut SpellcheckService) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            svc.poll();
            if svc.loaded().is_some() {
                return;
            }
            if let Some((_, err)) = svc.load_error() {
                panic!("dictionary load failed: {err}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for dictionary load");
    }

    fn wait_for_version(svc: &mut SpellcheckService, tab_id: usize, version: u64) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            svc.poll();
            if svc.diagnostics_for(tab_id, version).is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        panic!("timed out waiting for diagnostics version {version}");
    }

    #[test]
    fn queued_checks_answer_newest_version_only() {
        let tmp = tempfile::tempdir().unwrap();
        // Queue both Checks while the worker is still loading so they sit
        // in the channel and are coalesced in one drain.
        let mut svc = SpellcheckService::spawn("en_US".into(), None, tmp.path().to_path_buf());
        svc.request_check(7, 1, 0, vec!["helo world".into()]);
        svc.request_check(7, 2, 0, vec!["helo world".into()]);
        wait_until_loaded(&mut svc);
        wait_for_version(&mut svc, 7, 2);

        assert!(
            svc.diagnostics_for(7, 1).is_none(),
            "stale version must not be served"
        );
        let diags = svc.diagnostics_for(7, 2).expect("newest version");
        assert!(
            diags.iter().any(|d| d.message.contains("helo")),
            "{diags:?}"
        );
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("spell")));
        assert!(diags
            .iter()
            .all(|d| d.severity == crate::lsp::state::DiagnosticSeverity::Hint));
    }

    #[test]
    fn diagnostics_for_stale_version_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let mut svc = SpellcheckService::spawn("en_US".into(), None, tmp.path().to_path_buf());
        wait_until_loaded(&mut svc);
        svc.request_check(3, 10, 0, vec!["helo".into()]);
        wait_for_version(&mut svc, 3, 10);
        assert!(svc.diagnostics_for(3, 9).is_none());
        assert!(svc.diagnostics_for(3, 10).is_some());
    }

    #[test]
    fn shutdown_joins_the_thread() {
        let tmp = tempfile::tempdir().unwrap();
        let mut svc = SpellcheckService::spawn("en_US".into(), None, tmp.path().to_path_buf());
        wait_until_loaded(&mut svc);
        let start = Instant::now();
        svc.shutdown();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "shutdown should join promptly"
        );
    }

    #[test]
    fn should_request_check_only_on_key_change_after_debounce() {
        let key_a = SpellcheckWindowKey {
            tab_id: 1,
            version: 1,
            first_line: 0,
            last_line: 10,
        };
        let key_b = SpellcheckWindowKey {
            tab_id: 1,
            version: 2,
            first_line: 0,
            last_line: 10,
        };
        let now = Instant::now();
        assert!(should_request_check(None, &key_a, None, now, CHECK_DEBOUNCE));
        assert!(!should_request_check(
            Some(&key_a),
            &key_a,
            None,
            now,
            CHECK_DEBOUNCE
        ));
        let just_edited = now - Duration::from_millis(50);
        assert!(!should_request_check(
            Some(&key_a),
            &key_b,
            Some(just_edited),
            now,
            CHECK_DEBOUNCE
        ));
        let settled = now - Duration::from_millis(300);
        assert!(should_request_check(
            Some(&key_a),
            &key_b,
            Some(settled),
            now,
            CHECK_DEBOUNCE
        ));
    }

    fn hint_at(line: usize) -> DiagnosticEntry {
        DiagnosticEntry {
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 4,
            severity: crate::lsp::state::DiagnosticSeverity::Hint,
            message: format!("Unknown word: w{line}"),
            source: Some("spell".into()),
        }
    }

    #[test]
    fn cap_keeps_300_nearest_viewport() {
        let diags: Vec<_> = (0..500).map(hint_at).collect();
        let capped = cap_diagnostics_nearest_viewport(&diags, 200, 220, MAX_SQUIGGLES);
        assert_eq!(capped.len(), 300);
        assert!(capped.iter().any(|d| d.start_line == 210));
        assert!(capped.iter().all(|d| d.start_line.abs_diff(210) <= 200));
        let under = cap_diagnostics_nearest_viewport(&diags[..10], 0, 20, MAX_SQUIGGLES);
        assert_eq!(under.len(), 10);
    }

    #[test]
    fn locale_keys_exist_in_all_yaml() {
        let keys = [
            "settings.editor.spellcheck.enabled",
            "settings.editor.spellcheck.enabled_tooltip",
            "settings.editor.spellcheck.language",
            "settings.editor.spellcheck.dictionary_dir",
            "settings.editor.spellcheck.browse",
            "settings.editor.spellcheck.clear_dir",
            "settings.editor.spellcheck.ignore_all_caps",
            "settings.editor.spellcheck.ignore_words_with_digits",
            "settings.editor.spellcheck.personal_dictionary",
            "settings.editor.spellcheck.open_file",
            "context_menu.spell.add_to_dictionary",
            "context_menu.spell.ignore_word",
            "context_menu.spell.loading_suggestions",
            "spellcheck.loading",
            "spellcheck.load_failed",
        ];
        let locales_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("locales");
        for entry in std::fs::read_dir(&locales_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut defined = std::collections::BTreeSet::new();
            flatten_yaml_keys(&yaml, "", &mut defined);
            let missing: Vec<_> = keys.iter().filter(|k| !defined.contains(**k)).collect();
            assert!(
                missing.is_empty(),
                "{} missing keys: {missing:?}",
                path.display()
            );
        }
    }

    fn flatten_yaml_keys(
        value: &serde_yaml::Value,
        prefix: &str,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        let serde_yaml::Value::Mapping(map) = value else {
            if !prefix.is_empty() {
                out.insert(prefix.to_string());
            }
            return;
        };
        for (k, v) in map {
            let Some(key) = k.as_str() else {
                continue;
            };
            let next = if prefix.is_empty() {
                key.to_string()
            } else {
                format!("{prefix}.{key}")
            };
            flatten_yaml_keys(v, &next, out);
        }
    }
}
