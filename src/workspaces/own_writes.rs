//! Detect file-watcher events caused by this process's own saves (GIT-3).
//!
//! After a successful tab save we record `(path, mtime, len)`. The next watcher
//! event for that path with a matching stat is treated as our write and reload
//! is skipped. Entries expire after [`OWN_WRITE_TTL`] so the map cannot grow.

use crate::path_utils::normalize_path;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How long an own-write signature remains valid if no matching event arrives.
pub const OWN_WRITE_TTL: Duration = Duration::from_secs(5);

/// On-disk identity of a file immediately after we wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStatSignature {
    pub modified: SystemTime,
    pub len: u64,
}

#[derive(Debug, Clone)]
struct PendingOwnWrite {
    signature: FileStatSignature,
    recorded_at: Instant,
}

/// Pending own-write signatures keyed by normalized path.
#[derive(Debug, Default)]
pub struct OwnWriteRegistry {
    entries: HashMap<PathBuf, PendingOwnWrite>,
}

impl OwnWriteRegistry {
    pub fn record_path(&mut self, path: &Path) {
        let key = own_write_key(path);
        let Some(signature) = read_signature(path) else {
            return;
        };
        let now = Instant::now();
        self.prune_expired(now);
        self.entries.insert(
            key,
            PendingOwnWrite {
                signature,
                recorded_at: now,
            },
        );
    }

    /// Returns `true` when `path` matches a recorded own write (entry is removed).
    pub fn consume_if_own_write(&mut self, path: &Path) -> bool {
        self.consume_if_own_write_at(path, Instant::now())
    }

    fn consume_if_own_write_at(&mut self, path: &Path, now: Instant) -> bool {
        self.prune_expired(now);
        let key = own_write_key(path);
        let Some(pending) = self.entries.get(&key) else {
            return false;
        };
        let Some(current) = read_signature(path) else {
            return false;
        };
        if current == pending.signature {
            self.entries.remove(&key);
            true
        } else {
            false
        }
    }

    fn prune_expired(&mut self, now: Instant) {
        self.entries
            .retain(|_, entry| now.duration_since(entry.recorded_at) < OWN_WRITE_TTL);
    }
}

fn own_write_key(path: &Path) -> PathBuf {
    path.canonicalize()
        .map(normalize_path)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn read_signature(path: &Path) -> Option<FileStatSignature> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileStatSignature {
        modified: meta.modified().ok()?,
        len: meta.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn own_write_matching_stat_is_consumed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("own-write.md");
        std::fs::write(&path, b"hello").expect("write");

        let mut registry = OwnWriteRegistry::default();
        registry.record_path(&path);

        assert!(
            registry.consume_if_own_write(&path),
            "first event with matching stat must be own write"
        );
        assert!(
            !registry.consume_if_own_write(&path),
            "entry must be consumed"
        );
    }

    #[test]
    fn own_write_different_len_is_not_own_write() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("own-write-len.md");
        std::fs::write(&path, b"v1").expect("write");

        let mut registry = OwnWriteRegistry::default();
        registry.record_path(&path);

        std::fs::write(&path, b"v2-longer").expect("external write");

        assert!(
            !registry.consume_if_own_write(&path),
            "changed length must not match own-write signature"
        );
    }

    #[test]
    fn own_write_expires_after_ttl() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("own-write-expire.md");
        std::fs::write(&path, b"x").expect("write");

        let mut registry = OwnWriteRegistry::default();
        let recorded_at = Instant::now() - OWN_WRITE_TTL - StdDuration::from_millis(1);
        let key = own_write_key(&path);
        let signature = read_signature(&path).expect("stat");
        registry.entries.insert(
            key,
            PendingOwnWrite {
                signature,
                recorded_at,
            },
        );

        assert!(
            !registry.consume_if_own_write_at(&path, Instant::now()),
            "stale entry must not match"
        );
    }

    #[test]
    fn own_write_mtime_change_after_record_is_external() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("own-write-mtime.md");
        std::fs::write(&path, b"same-len").expect("write");

        let mut registry = OwnWriteRegistry::default();
        registry.record_path(&path);

        // Same length, different mtime (sleep is enough on common filesystems).
        thread::sleep(StdDuration::from_millis(1100));
        std::fs::write(&path, b"same-len").expect("rewrite");

        assert!(
            !registry.consume_if_own_write(&path),
            "mtime change must not match recorded signature"
        );
    }
}
