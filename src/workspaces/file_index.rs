//! Background workspace file index for quick switcher and search-in-files.
//!
//! The sidebar file tree uses lazy loading, so `Workspace::all_files()` only sees
//! expanded folders. This module walks the full tree on a background thread and
//! exposes incremental progress for large workspaces.

use super::file_tree;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use walkdir::WalkDir;

/// How often the background thread reports progress (entries scanned).
const PROGRESS_INTERVAL: usize = 250;
/// Batch size for incremental file list updates sent to the UI thread.
const FILE_BATCH_SIZE: usize = 128;
/// Maximum index messages to apply on one UI tick.
const MAX_MESSAGES_PER_POLL: usize = 64;

/// Progress snapshot for UI (quick switcher / search panel).
#[derive(Debug, Clone, Copy)]
pub struct FileIndexProgress {
    /// Directory entries visited so far (files + folders).
    pub entries_scanned: usize,
    /// Files discovered so far.
    pub files_found: usize,
}

/// Messages from the background indexing thread.
enum FileIndexMsg {
    Batch {
        generation: u64,
        entries_scanned: usize,
        files: Vec<PathBuf>,
    },
    Complete {
        generation: u64,
        entries_scanned: usize,
        total_files: usize,
    },
}

/// Background workspace file index.
pub struct WorkspaceFileIndex {
    root: Option<PathBuf>,
    generation: u64,
    needs_rebuild: bool,
    entries_scanned: usize,
    files: Vec<PathBuf>,
    is_complete: bool,
    rx: Receiver<FileIndexMsg>,
}

impl Default for WorkspaceFileIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceFileIndex {
    pub fn new() -> Self {
        let (_tx, rx) = mpsc::channel();
        Self {
            root: None,
            generation: 0,
            needs_rebuild: false,
            entries_scanned: 0,
            files: Vec::new(),
            is_complete: false,
            rx,
        }
    }

    /// Mark the index stale (e.g. after files are created or deleted).
    pub fn invalidate(&mut self) {
        self.needs_rebuild = true;
    }

    /// Reset when leaving workspace mode.
    pub fn reset(&mut self) {
        self.root = None;
        self.generation = self.generation.wrapping_add(1);
        self.needs_rebuild = false;
        self.entries_scanned = 0;
        self.files.clear();
        self.is_complete = false;
        let (_tx, rx) = mpsc::channel();
        self.rx = rx;
    }

    /// Start or restart indexing when the workspace changes or was invalidated.
    pub fn sync(&mut self, root: &Path, hidden_patterns: &[String]) {
        let same_root = self.root.as_deref() == Some(root);
        if same_root && !self.needs_rebuild && (self.is_indexing() || self.is_complete) {
            return;
        }

        self.root = Some(root.to_path_buf());
        self.needs_rebuild = false;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.entries_scanned = 0;
        self.files.clear();
        self.is_complete = false;

        let (tx, rx) = mpsc::channel();
        self.rx = rx;

        let root_owned = root.to_path_buf();
        let patterns = hidden_patterns.to_vec();
        std::thread::spawn(move || index_workspace_thread(root_owned, patterns, generation, tx));
    }

    /// Drain background messages. Returns true if state changed (repaint needed).
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for _ in 0..MAX_MESSAGES_PER_POLL {
            let Ok(msg) = self.rx.try_recv() else {
                break;
            };
            match msg {
                FileIndexMsg::Batch {
                    generation,
                    entries_scanned,
                    files,
                } if generation == self.generation => {
                    self.entries_scanned = entries_scanned;
                    self.files.extend(files);
                    changed = true;
                }
                FileIndexMsg::Complete {
                    generation,
                    entries_scanned,
                    total_files,
                } if generation == self.generation => {
                    self.entries_scanned = entries_scanned;
                    debug_assert_eq!(self.files.len(), total_files);
                    self.is_complete = true;
                    changed = true;
                }
                _ => {}
            }
        }
        changed
    }

    /// Indexed files (partial while scanning, complete when done).
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn progress(&self) -> Option<FileIndexProgress> {
        if self.root.is_none() || self.is_complete {
            return None;
        }
        Some(FileIndexProgress {
            entries_scanned: self.entries_scanned,
            files_found: self.files.len(),
        })
    }

    pub fn is_indexing(&self) -> bool {
        self.root.is_some() && !self.is_complete
    }
}

/// Walk the workspace and collect all regular files (sync, for tests and backlinks).
pub fn collect_all_files(root: &Path, hidden_patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(root, hidden_patterns, |path| {
        files.push(path.to_path_buf());
    });
    files
}

/// Collect markdown files under `root` (sync).
pub fn collect_markdown_files(root: &Path, hidden_patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(root, hidden_patterns, |path| {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") => {
                files.push(path.to_path_buf());
            }
            _ => {}
        }
    });
    files
}

fn walk_files(root: &Path, hidden_patterns: &[String], mut on_file: impl FnMut(&Path)) {
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !should_skip_walk_entry(entry, root, hidden_patterns))
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            on_file(entry.path());
        }
    }
}

/// Whether a walkdir entry should be excluded (never skips the workspace root itself).
fn should_skip_walk_entry(
    entry: &walkdir::DirEntry,
    root: &Path,
    hidden_patterns: &[String],
) -> bool {
    if entry.path() == root {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    file_tree::entry_hidden(&name, hidden_patterns)
}

fn index_workspace_thread(
    root: PathBuf,
    hidden_patterns: Vec<String>,
    generation: u64,
    tx: Sender<FileIndexMsg>,
) {
    let mut entries_scanned = 0usize;
    let mut batch: Vec<PathBuf> = Vec::with_capacity(FILE_BATCH_SIZE);
    let mut total_files = 0usize;

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|entry| !should_skip_walk_entry(entry, &root, &hidden_patterns))
    {
        entries_scanned += 1;

        let Ok(entry) = entry else {
            if entries_scanned % PROGRESS_INTERVAL == 0 {
                let _ = tx.send(FileIndexMsg::Batch {
                    generation,
                    entries_scanned,
                    files: std::mem::take(&mut batch),
                });
            }
            continue;
        };

        if entry.file_type().is_file() {
            batch.push(entry.into_path());
            total_files += 1;
            if batch.len() >= FILE_BATCH_SIZE {
                let _ = tx.send(FileIndexMsg::Batch {
                    generation,
                    entries_scanned,
                    files: std::mem::take(&mut batch),
                });
            }
        }

        if entries_scanned % PROGRESS_INTERVAL == 0 {
            let _ = tx.send(FileIndexMsg::Batch {
                generation,
                entries_scanned,
                files: std::mem::take(&mut batch),
            });
        }
    }

    if !batch.is_empty() {
        let _ = tx.send(FileIndexMsg::Batch {
            generation,
            entries_scanned,
            files: batch,
        });
    }

    let _ = tx.send(FileIndexMsg::Complete {
        generation,
        entries_scanned,
        total_files,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn collect_all_files_includes_nested_unexpanded_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("nested/deep")).unwrap();
        fs::write(root.join("root.md"), "# hi").unwrap();
        fs::write(root.join("nested/deep/hidden.md"), "# nested").unwrap();

        let files = collect_all_files(root, &["node_modules".to_string()]);
        assert!(files.iter().any(|p| p.ends_with("root.md")));
        assert!(files.iter().any(|p| p.ends_with("hidden.md")));
    }

    #[test]
    fn collect_all_files_skips_hidden_folders() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "x").unwrap();
        fs::write(root.join("visible.txt"), "y").unwrap();

        let files = collect_all_files(root, &["node_modules".to_string()]);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("visible.txt"));
    }
}
