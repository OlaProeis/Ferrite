//! File system watcher for workspace mode.
//!
//! Watches the workspace root for file system changes and notifies
//! the application when files are created, modified, or deleted.

// Allow dead code - includes FileRenamed event and root_path accessor for future
// file rename detection and watcher management
#![allow(dead_code)]

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// File system events that the workspace cares about.
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// A file was modified externally
    FileModified(PathBuf),
    /// A file was created
    FileCreated(PathBuf),
    /// A file was deleted
    FileDeleted(PathBuf),
    /// A file was renamed (from, to)
    FileRenamed(PathBuf, PathBuf),
    /// The watcher encountered an error
    Error(String),
}

/// Manages file system watching for a workspace.
#[derive(Debug)]
pub struct WorkspaceWatcher {
    /// The internal notify watcher
    _watcher: RecommendedWatcher,
    /// Receiver for file system events
    receiver: Receiver<WorkspaceEvent>,
    /// Root path being watched
    root_path: PathBuf,
}

impl WorkspaceWatcher {
    /// Create a new workspace watcher for the given root path.
    ///
    /// Returns an error if the watcher cannot be created.
    pub fn new(root_path: PathBuf) -> Result<Self, String> {
        let (tx, rx) = channel();

        // Create the watcher with a debounce of 500ms
        let tx_clone = tx.clone();
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                Self::handle_event(result, &tx_clone);
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        let mut instance = Self {
            _watcher: watcher,
            receiver: rx,
            root_path: root_path.clone(),
        };

        // Start watching the root path
        instance.watch_path(&root_path)?;

        Ok(instance)
    }

    /// Start watching a specific path.
    fn watch_path(&mut self, path: &Path) -> Result<(), String> {
        self._watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path {}: {}", path.display(), e))
    }

    /// Handle a raw notify event and convert to WorkspaceEvent.
    fn handle_event(result: Result<Event, notify::Error>, tx: &Sender<WorkspaceEvent>) {
        match result {
            Ok(event) => {
                for path in event.paths {
                    let workspace_event = match event.kind {
                        EventKind::Create(_) => Some(WorkspaceEvent::FileCreated(path)),
                        EventKind::Modify(_) => Some(WorkspaceEvent::FileModified(path)),
                        EventKind::Remove(_) => Some(WorkspaceEvent::FileDeleted(path)),
                        // Other events (Access, Other) - ignore for now
                        _ => None,
                    };

                    if let Some(evt) = workspace_event {
                        let _ = tx.send(evt);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(WorkspaceEvent::Error(e.to_string()));
            }
        }
    }

    /// Poll for pending workspace events.
    ///
    /// Returns all events that have occurred since the last poll.
    /// This is non-blocking.
    pub fn poll_events(&self) -> Vec<WorkspaceEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// Get the root path being watched.
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

/// Filter events to exclude hidden/ignored paths.
pub fn filter_events(
    events: Vec<WorkspaceEvent>,
    hidden_patterns: &[String],
) -> Vec<WorkspaceEvent> {
    events
        .into_iter()
        .filter(|event| {
            let path = match event {
                WorkspaceEvent::FileModified(p)
                | WorkspaceEvent::FileCreated(p)
                | WorkspaceEvent::FileDeleted(p) => p,
                WorkspaceEvent::FileRenamed(_, to) => to,
                WorkspaceEvent::Error(_) => return true, // Always pass through errors
            };

            // Check if any component of the path matches a hidden pattern
            for component in path.components() {
                if let std::path::Component::Normal(name) = component {
                    let name_str = name.to_string_lossy();
                    for pattern in hidden_patterns {
                        if name_str.contains(pattern) || name_str == *pattern {
                            return false;
                        }
                    }
                }
            }

            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_events_passes_non_hidden() {
        let events = vec![
            WorkspaceEvent::FileModified(PathBuf::from("/workspace/src/main.rs")),
            WorkspaceEvent::FileCreated(PathBuf::from("/workspace/docs/readme.md")),
        ];
        let hidden = vec![".git".to_string(), "node_modules".to_string()];

        let filtered = filter_events(events, &hidden);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_events_removes_hidden() {
        let events = vec![
            WorkspaceEvent::FileModified(PathBuf::from("/workspace/.git/config")),
            WorkspaceEvent::FileCreated(PathBuf::from("/workspace/node_modules/foo/index.js")),
            WorkspaceEvent::FileDeleted(PathBuf::from("/workspace/src/main.rs")),
        ];
        let hidden = vec![".git".to_string(), "node_modules".to_string()];

        let filtered = filter_events(events, &hidden);
        assert_eq!(filtered.len(), 1);
        match &filtered[0] {
            WorkspaceEvent::FileDeleted(p) => {
                assert!(p.to_string_lossy().contains("main.rs"));
            }
            _ => panic!("Expected FileDeleted event"),
        }
    }

    #[test]
    fn test_filter_events_passes_errors() {
        let events = vec![WorkspaceEvent::Error("Test error".to_string())];
        let hidden = vec![".git".to_string()];

        let filtered = filter_events(events, &hidden);
        assert_eq!(filtered.len(), 1);
    }
}
