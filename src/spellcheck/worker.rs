//! Background spellcheck thread: owns the Hunspell dictionary and personal set.

#![allow(dead_code)] // AddWord / IgnoreWord / Reload are sent from the facade (task 16)

use super::dictionary::{self, Dictionary};
use crate::lsp::state::{DiagnosticEntry, DiagnosticSeverity};
use crate::spellcheck::tokenize::{words_to_check_opts, LineContext, TokenizeOptions};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

const SUGGEST_CACHE_CAP: usize = 2_000;

#[derive(Debug, Clone)]
pub enum Request {
    Check {
        tab_id: usize,
        version: u64,
        first_line: usize,
        lines: Vec<String>,
    },
    Suggest {
        word: String,
    },
    AddWord(String),
    IgnoreWord(String),
    Reload {
        lang: String,
        dir: Option<PathBuf>,
    },
    SetOptions {
        ignore_all_caps: bool,
        ignore_words_with_digits: bool,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum Response {
    Diagnostics {
        tab_id: usize,
        version: u64,
        first_line: usize,
        diags: Vec<DiagnosticEntry>,
    },
    Suggestions {
        word: String,
        list: Vec<String>,
    },
    Loaded {
        lang: String,
        word_count: usize,
    },
    LoadFailed {
        lang: String,
        error: String,
    },
}

pub(crate) struct WorkerChannels {
    pub tx: Sender<Request>,
    pub rx: Receiver<Response>,
    pub thread: JoinHandle<()>,
}

pub(crate) fn spawn(
    lang: String,
    dict_dir: Option<PathBuf>,
    config_dir: PathBuf,
) -> WorkerChannels {
    let (req_tx, req_rx) = mpsc::channel();
    let (resp_tx, resp_rx) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("ferrite-spellcheck".into())
        .spawn(move || run(lang, dict_dir, config_dir, req_rx, resp_tx))
        .expect("failed to spawn spellcheck worker");
    WorkerChannels {
        tx: req_tx,
        rx: resp_rx,
        thread,
    }
}

struct WorkerState {
    lang: String,
    dict_dir: Option<PathBuf>,
    dict: Option<Dictionary>,
    /// Personal dictionary words (persisted) plus session-ignored words.
    extra_words: HashSet<String>,
    personal: HashSet<String>,
    user_words_path: PathBuf,
    suggest_cache: HashMap<String, Vec<String>>,
    tokenize: TokenizeOptions,
}

fn run(
    lang: String,
    dict_dir: Option<PathBuf>,
    config_dir: PathBuf,
    req_rx: Receiver<Request>,
    resp_tx: Sender<Response>,
) {
    let user_words_path = dictionary::user_words_path(&config_dir);
    let personal: HashSet<String> = dictionary::load_user_words(&user_words_path)
        .into_iter()
        .collect();
    let extra_words = personal.clone();

    let mut state = WorkerState {
        lang: lang.clone(),
        dict_dir: dict_dir.clone(),
        dict: None,
        extra_words,
        personal,
        user_words_path,
        suggest_cache: HashMap::new(),
        tokenize: TokenizeOptions::default(),
    };

    match Dictionary::load(&lang, dict_dir.as_deref()) {
        Ok(mut dict) => {
            for w in &state.personal {
                dict.add(w);
            }
            let word_count = dict.word_count();
            state.dict = Some(dict);
            let _ = resp_tx.send(Response::Loaded { lang, word_count });
        }
        Err(error) => {
            let _ = resp_tx.send(Response::LoadFailed { lang, error });
        }
    }

    loop {
        let first = match req_rx.recv() {
            Ok(r) => r,
            Err(_) => break,
        };
        let mut batch = vec![first];
        while let Ok(more) = req_rx.try_recv() {
            batch.push(more);
        }
        let (to_run, shutdown) = coalesce_requests(batch);
        if shutdown {
            break;
        }
        for req in to_run {
            if !handle_request(req, &mut state, &resp_tx) {
                return;
            }
        }
    }
}

/// Keep the newest `Check` per tab; preserve other requests in arrival order.
/// `Shutdown` anywhere in the batch ends the worker.
pub(crate) fn coalesce_requests(batch: Vec<Request>) -> (Vec<Request>, bool) {
    if batch.iter().any(|r| matches!(r, Request::Shutdown)) {
        return (Vec::new(), true);
    }
    let mut last_check: HashMap<usize, Request> = HashMap::new();
    let mut others = Vec::new();
    for req in batch {
        if let Request::Check { tab_id, .. } = &req {
            last_check.insert(*tab_id, req);
        } else {
            others.push(req);
        }
    }
    others.extend(last_check.into_values());
    (others, false)
}

fn handle_request(req: Request, state: &mut WorkerState, resp_tx: &Sender<Response>) -> bool {
    match req {
        Request::Shutdown => false,
        Request::AddWord(word) => {
            if word.is_empty() {
                return true;
            }
            state.personal.insert(word.clone());
            state.extra_words.insert(word.clone());
            if let Some(dict) = state.dict.as_mut() {
                dict.add(&word);
            }
            state.suggest_cache.remove(&word);
            if let Err(e) = dictionary::save_user_words(&state.user_words_path, &state.personal) {
                log::warn!("spellcheck: failed to persist user word: {e}");
            }
            true
        }
        Request::IgnoreWord(word) => {
            if !word.is_empty() {
                state.extra_words.insert(word);
            }
            true
        }
        Request::SetOptions {
            ignore_all_caps,
            ignore_words_with_digits,
        } => {
            state.tokenize = TokenizeOptions {
                ignore_all_caps,
                ignore_words_with_digits,
            };
            true
        }
        Request::Reload { lang, dir } => {
            state.lang = lang.clone();
            state.dict_dir = dir.clone();
            state.suggest_cache.clear();
            match Dictionary::load(&lang, dir.as_deref()) {
                Ok(mut dict) => {
                    for w in &state.personal {
                        dict.add(w);
                    }
                    let word_count = dict.word_count();
                    state.dict = Some(dict);
                    let _ = resp_tx.send(Response::Loaded { lang, word_count });
                }
                Err(error) => {
                    state.dict = None;
                    let _ = resp_tx.send(Response::LoadFailed { lang, error });
                }
            }
            true
        }
        Request::Suggest { word } => {
            let list = if let Some(cached) = state.suggest_cache.get(&word) {
                cached.clone()
            } else if let Some(dict) = state.dict.as_ref() {
                let list = dict.suggest(&word);
                if state.suggest_cache.len() >= SUGGEST_CACHE_CAP {
                    state.suggest_cache.clear();
                }
                state.suggest_cache.insert(word.clone(), list.clone());
                list
            } else {
                Vec::new()
            };
            let _ = resp_tx.send(Response::Suggestions { word, list });
            true
        }
        Request::Check {
            tab_id,
            version,
            first_line,
            lines,
        } => {
            let diags = check_lines(state, first_line, &lines);
            let _ = resp_tx.send(Response::Diagnostics {
                tab_id,
                version,
                first_line,
                diags,
            });
            true
        }
    }
}

fn check_lines(state: &WorkerState, first_line: usize, lines: &[String]) -> Vec<DiagnosticEntry> {
    let Some(dict) = state.dict.as_ref() else {
        return Vec::new();
    };
    let mut ctx = LineContext::starting_at(first_line);
    let mut diags = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for (start, end, word) in words_to_check_opts(line, &mut ctx, state.tokenize) {
            if state.extra_words.contains(word) {
                continue;
            }
            if dict.check(word) {
                continue;
            }
            let line_no = first_line + i;
            diags.push(DiagnosticEntry {
                start_line: line_no,
                start_col: start,
                end_line: line_no,
                end_col: end,
                severity: DiagnosticSeverity::Hint,
                message: format!("Unknown word: {word}"),
                source: Some("spell".into()),
            });
        }
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_keeps_newest_check_per_tab() {
        let batch = vec![
            Request::Check {
                tab_id: 1,
                version: 1,
                first_line: 0,
                lines: vec!["a".into()],
            },
            Request::Suggest {
                word: "helo".into(),
            },
            Request::Check {
                tab_id: 1,
                version: 2,
                first_line: 0,
                lines: vec!["b".into()],
            },
            Request::Check {
                tab_id: 2,
                version: 1,
                first_line: 0,
                lines: vec!["c".into()],
            },
        ];
        let (out, shutdown) = coalesce_requests(batch);
        assert!(!shutdown);
        let checks: Vec<(usize, u64)> = out
            .iter()
            .filter_map(|r| match r {
                Request::Check {
                    tab_id, version, ..
                } => Some((*tab_id, *version)),
                _ => None,
            })
            .collect();
        assert!(checks.contains(&(1, 2)));
        assert!(!checks.contains(&(1, 1)));
        assert!(checks.contains(&(2, 1)));
        assert!(out.iter().any(|r| matches!(r, Request::Suggest { .. })));
    }

    #[test]
    fn coalesce_shutdown_drops_batch() {
        let batch = vec![
            Request::Check {
                tab_id: 1,
                version: 1,
                first_line: 0,
                lines: vec![],
            },
            Request::Shutdown,
        ];
        let (out, shutdown) = coalesce_requests(batch);
        assert!(shutdown);
        assert!(out.is_empty());
    }
}
