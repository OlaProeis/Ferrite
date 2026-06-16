//! Path Utilities
//!
//! This module provides utilities for normalizing file paths, particularly
//! on Windows where `canonicalize()` returns verbatim paths with `\\?\` prefix.
//!
//! # Problem
//! On Windows, `std::fs::canonicalize()` returns paths with the extended-length
//! path prefix `\\?\` (e.g., `\\?\G:\DEV\project` instead of `G:\DEV\project`).
//!
//! This causes several issues:
//! - Paths appear confusing to users with the `\\?\` prefix
//! - Same path can appear as duplicates (with and without prefix)
//! - Some libraries (like git2) may not handle verbatim paths properly
//! - Path comparisons fail because `\\?\G:\path` != `G:\path`
//!
//! # Solution
//! Use `normalize_path()` after `canonicalize()` to strip the verbatim prefix
//! and ensure consistent path representation throughout the application.
//!
//! # Example
//! ```ignore
//! use crate::path_utils::normalize_path;
//!
//! let canonical = path.canonicalize()?;
//! let normalized = normalize_path(canonical);
//! // normalized is now "G:\DEV\project" instead of "\\?\G:\DEV\project"
//! ```

use std::path::{Path, PathBuf};

/// A link target that Ferrite can open directly from rendered markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenableLinkTarget {
    /// Open in the default browser.
    WebUrl(String),
    /// Reveal the location in the system file manager.
    LocalPath(PathBuf),
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Normalization
// ─────────────────────────────────────────────────────────────────────────────

/// Normalize a path by stripping Windows extended-length path prefixes.
///
/// On Windows, this removes the `\\?\` prefix that `canonicalize()` adds.
/// On other platforms, this is a no-op and returns the path unchanged.
///
/// # Windows Extended-Length Paths
/// Windows uses these prefixes for extended-length paths:
/// - `\\?\` - Verbatim disk path (e.g., `\\?\C:\path`)
/// - `\\.\` - Verbatim device path (e.g., `\\.\COM1`)
/// - `\??\` - NT namespace path
///
/// This function handles the common `\\?\` prefix that `canonicalize()` adds.
///
/// # Example
/// ```ignore
/// use std::path::PathBuf;
/// use crate::path_utils::normalize_path;
///
/// let path = PathBuf::from(r"\\?\G:\DEV\project");
/// let normalized = normalize_path(path);
/// assert_eq!(normalized, PathBuf::from(r"G:\DEV\project"));
/// ```
pub fn normalize_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        normalize_windows_path(path)
    }

    #[cfg(not(windows))]
    {
        path
    }
}

/// Normalize a path reference, returning an owned PathBuf.
///
/// This is useful when you have a path reference and need a normalized owned path.
#[allow(dead_code)] // Public API for path normalization
pub fn normalize_path_ref(path: &Path) -> PathBuf {
    normalize_path(path.to_path_buf())
}

#[cfg(windows)]
fn normalize_windows_path(path: PathBuf) -> PathBuf {
    use std::path::Prefix;

    // Check if the path has a verbatim prefix
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            // \\?\C:\... -> C:\...
            Prefix::VerbatimDisk(disk) => {
                let drive = (disk as char).to_ascii_uppercase();
                let rest: PathBuf = path
                    .components()
                    .skip(1) // Skip the prefix
                    .collect();

                let mut normalized = PathBuf::from(format!("{}:", drive));
                if !rest.as_os_str().is_empty() {
                    normalized.push(rest);
                } else {
                    // Ensure we have a root (C: -> C:\)
                    normalized.push(std::path::MAIN_SEPARATOR.to_string());
                }
                return normalized;
            }
            // \\?\UNC\server\share -> \\server\share
            Prefix::VerbatimUNC(server, share) => {
                let rest: PathBuf = path.components().skip(1).collect();
                let mut normalized = PathBuf::from(format!(
                    r"\\{}\{}",
                    server.to_string_lossy(),
                    share.to_string_lossy()
                ));
                if !rest.as_os_str().is_empty() {
                    normalized.push(rest);
                }
                return normalized;
            }
            // Other prefixes (Disk, UNC, etc.) are already normalized
            _ => {}
        }
    }

    // Path doesn't have a verbatim prefix, return as-is
    path
}

/// Canonicalize a path and normalize it to remove Windows verbatim prefixes.
///
/// This is a convenience function that combines `canonicalize()` and `normalize_path()`.
/// Returns `None` if canonicalization fails (e.g., path doesn't exist).
///
/// # Example
/// ```ignore
/// use crate::path_utils::canonicalize_and_normalize;
///
/// if let Some(path) = canonicalize_and_normalize(&some_path) {
///     // path is fully resolved and normalized (no \\?\ prefix)
/// }
/// ```
#[allow(dead_code)] // Public API for path canonicalization
pub fn canonicalize_and_normalize(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok().map(normalize_path)
}

/// Canonicalize a path with fallback, and normalize the result.
///
/// If canonicalization fails, returns the original path (potentially cleaned).
/// Always normalizes the result to remove Windows verbatim prefixes.
///
/// # Example
/// ```ignore
/// use crate::path_utils::canonicalize_or_normalize;
///
/// // Even if path doesn't exist, returns a usable path
/// let path = canonicalize_or_normalize(&some_path);
/// ```
#[allow(dead_code)] // Public API for path canonicalization
pub fn canonicalize_or_normalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => normalize_path(canonical),
        Err(_) => {
            // If canonicalization fails, try to at least normalize what we have
            // This handles the case where the file doesn't exist yet
            normalize_path(path.to_path_buf())
        }
    }
}

/// True when the target is an HTTP(S) URL.
pub fn is_http_url(target: &str) -> bool {
    let target = target.trim();
    target.starts_with("http://") || target.starts_with("https://")
}

/// Return the path that should be opened in the system file manager.
///
/// Directories open directly. Files reveal their parent directory.
pub fn explorer_target(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    }
}

/// Open a path in the system file manager.
pub fn open_in_file_manager(path: &Path) -> std::io::Result<()> {
    open::that(explorer_target(path))
}

/// Resolve an openable markdown link target.
pub fn resolve_openable_link_target(
    target: &str,
    current_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> Option<OpenableLinkTarget> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    if is_http_url(target) {
        return Some(OpenableLinkTarget::WebUrl(target.to_string()));
    }

    resolve_local_link_path(target, current_dir, workspace_root).map(OpenableLinkTarget::LocalPath)
}

/// Resolve a local markdown link target to an existing file or directory.
pub fn resolve_local_link_path(
    target: &str,
    current_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> Option<PathBuf> {
    let target = target.trim();
    if target.is_empty() || has_non_local_scheme(target) {
        return None;
    }

    if let Some(path) = parse_absolute_or_file_url_path(target) {
        if path.exists() {
            return Some(canonicalize_or_normalize(&path));
        }
    }

    let relative = Path::new(target);
    if let Some(dir) = current_dir {
        let candidate = dir.join(relative);
        if candidate.exists() {
            return Some(canonicalize_or_normalize(&candidate));
        }
    }

    if let Some(root) = workspace_root {
        let candidate = root.join(relative);
        if candidate.exists() {
            return Some(canonicalize_or_normalize(&candidate));
        }
    }

    None
}

fn has_non_local_scheme(target: &str) -> bool {
    let target = target.trim();
    target.starts_with("data:")
        || target.starts_with("mailto:")
        || target.starts_with("tel:")
        || target.starts_with("ftp://")
        || target.starts_with("ws://")
        || target.starts_with("wss://")
}

fn parse_absolute_or_file_url_path(target: &str) -> Option<PathBuf> {
    if let Some(path) = parse_file_url_path(target) {
        return Some(normalize_local_link_candidate(path));
    }

    #[cfg(windows)]
    if let Some(path) = wsl_mount_to_windows_path(target) {
        return Some(path);
    }

    if looks_like_windows_absolute_path(target) || target.starts_with(r"\\") {
        return Some(normalize_path(PathBuf::from(target)));
    }

    let path = Path::new(target);
    if path.is_absolute() {
        return Some(normalize_local_link_candidate(path.to_path_buf()));
    }

    None
}

fn parse_file_url_path(target: &str) -> Option<PathBuf> {
    let parsed = url::Url::parse(target).ok()?;
    if parsed.scheme() != "file" {
        return None;
    }
    parsed.to_file_path().ok()
}

fn looks_like_windows_absolute_path(target: &str) -> bool {
    let bytes = target.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn normalize_local_link_candidate(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let path_str = path.to_string_lossy();
        if let Some(converted) = wsl_mount_to_windows_path(&path_str) {
            return converted;
        }
    }

    normalize_path(path)
}

#[cfg(windows)]
fn wsl_mount_to_windows_path(path: &str) -> Option<PathBuf> {
    let normalized = path.replace('\\', "/");
    let rest = normalized.strip_prefix("/mnt/")?;
    let (drive, suffix) = rest
        .split_once('/')
        .map(|(drive, suffix)| (drive, Some(suffix)))
        .unwrap_or((rest, None));
    let drive_char = drive.chars().next()?;
    if drive.len() != 1 || !drive_char.is_ascii_alphabetic() {
        return None;
    }

    let mut converted = PathBuf::from(format!("{}:\\", drive_char.to_ascii_uppercase()));
    if let Some(suffix) = suffix {
        if !suffix.is_empty() {
            converted.push(suffix.replace('/', "\\"));
        }
    }
    Some(normalize_path(converted))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_regular_path() {
        // Regular paths should be unchanged
        let path = PathBuf::from("/home/user/project");
        assert_eq!(normalize_path(path.clone()), path);
    }

    #[cfg(windows)]
    mod windows_tests {
        use super::*;

        #[test]
        fn test_normalize_verbatim_disk_path() {
            let path = PathBuf::from(r"\\?\G:\DEV\markDownNotepad");
            let normalized = normalize_path(path);
            assert_eq!(normalized, PathBuf::from(r"G:\DEV\markDownNotepad"));
        }

        #[test]
        fn test_normalize_verbatim_disk_root() {
            let path = PathBuf::from(r"\\?\C:");
            let normalized = normalize_path(path);
            // Should have trailing separator for root
            assert!(normalized.to_string_lossy().starts_with("C:"));
        }

        #[test]
        fn test_normalize_regular_windows_path() {
            // Regular Windows paths should be unchanged
            let path = PathBuf::from(r"G:\DEV\project");
            assert_eq!(normalize_path(path.clone()), path);
        }

        #[test]
        fn test_normalize_unc_path() {
            // Regular UNC paths should be unchanged
            let path = PathBuf::from(r"\\server\share\folder");
            assert_eq!(normalize_path(path.clone()), path);
        }

        #[test]
        fn test_normalize_lowercase_drive() {
            // Should normalize drive letter to uppercase
            let path = PathBuf::from(r"\\?\g:\dev\project");
            let normalized = normalize_path(path);
            assert!(normalized.to_string_lossy().starts_with("G:"));
        }

        #[test]
        fn test_canonicalize_and_normalize() {
            // Test with current directory (which should exist)
            if let Some(normalized) = canonicalize_and_normalize(Path::new(".")) {
                // Should not contain \\?\
                assert!(
                    !normalized.to_string_lossy().contains(r"\\?\"),
                    "Path should not contain verbatim prefix: {:?}",
                    normalized
                );
            }
        }

        #[test]
        fn test_canonicalize_or_normalize_nonexistent() {
            // Even for non-existent paths, should return something usable
            let path = Path::new(r"\\?\C:\nonexistent\path");
            let result = canonicalize_or_normalize(path);
            // Should strip the prefix even if canonicalization fails
            assert!(
                !result.to_string_lossy().starts_with(r"\\?\"),
                "Path should not start with verbatim prefix: {:?}",
                result
            );
        }
    }

    #[test]
    fn test_normalize_path_ref() {
        let path = PathBuf::from("/some/path");
        let normalized = normalize_path_ref(&path);
        assert_eq!(normalized, path);
    }

    #[test]
    fn test_http_url_detection() {
        assert!(is_http_url("https://example.com"));
        assert!(is_http_url("http://example.com"));
        assert!(!is_http_url("file:///tmp/test.md"));
    }

    #[test]
    fn test_openable_link_target_web_url() {
        let resolved = resolve_openable_link_target("https://example.com", None, None);
        assert_eq!(
            resolved,
            Some(OpenableLinkTarget::WebUrl(
                "https://example.com".to_string()
            ))
        );
    }

    #[test]
    fn test_explorer_target_reveals_parent_for_files() {
        let temp_dir = std::env::temp_dir().join("ferrite_explorer_target_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let file = temp_dir.join("note.md");
        std::fs::write(&file, "# test").unwrap();

        assert_eq!(explorer_target(&file), temp_dir);
        assert_eq!(explorer_target(&temp_dir), temp_dir);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_local_link_path_relative_file() {
        let temp_dir = std::env::temp_dir().join("ferrite_local_link_relative_file");
        let _ = std::fs::create_dir_all(&temp_dir);
        let file = temp_dir.join("target.md");
        std::fs::write(&file, "# target").unwrap();

        let resolved = resolve_local_link_path("target.md", Some(&temp_dir), None);
        assert_eq!(resolved, Some(canonicalize_or_normalize(&file)));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_local_link_path_relative_directory() {
        let temp_dir = std::env::temp_dir().join("ferrite_local_link_relative_dir");
        let nested = temp_dir.join("assets");
        let _ = std::fs::create_dir_all(&nested);

        let resolved = resolve_local_link_path("assets", Some(&temp_dir), None);
        assert_eq!(resolved, Some(canonicalize_or_normalize(&nested)));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[cfg(windows)]
    #[test]
    fn test_parse_windows_absolute_path() {
        let resolved = parse_absolute_or_file_url_path(r"E:\Zero_Base\sumi_os\WSL");
        assert_eq!(resolved, Some(PathBuf::from(r"E:\Zero_Base\sumi_os\WSL")));
    }

    #[cfg(windows)]
    #[test]
    fn test_convert_wsl_mount_path_to_windows_path() {
        let resolved = parse_absolute_or_file_url_path("/mnt/e/Zero_Base/sumi_os/WSL");
        assert_eq!(resolved, Some(PathBuf::from(r"E:\Zero_Base\sumi_os\WSL")));
    }
}
