//! Language Server Protocol client infrastructure (stdio JSON-RPC, process management).
//!
//! Editor integration (diagnostics, completions) is built on top in later tasks.

#![allow(dead_code)] // API surface used by upcoming LSP tasks; UI not wired yet

pub mod detection;
pub mod manager;
pub mod state;
pub mod transport;

pub use detection::{
    detect_lsp_server_for_path, detect_servers_for_workspace, install_hint, LspServerSpec,
};
pub use manager::{LspCommand, LspManager, LspManagerEvent};
pub use state::{DiagnosticEntry, DiagnosticMap, DiagnosticSeverity, ServerStatus};

pub(crate) fn overrides_fingerprint(map: &std::collections::HashMap<String, String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|(k, _)| *k);
    let mut h = DefaultHasher::new();
    for (k, v) in entries {
        k.hash(&mut h);
        v.hash(&mut h);
    }
    h.finish()
}
pub use transport::{
    encode_lsp_message, read_lsp_message, write_lsp_message, MessageReader, TransportError,
};

/// Normalize a path for use as a diagnostic map key.
///
/// Strips Windows `\\?\` prefix, uppercases the drive letter, and uses
/// backslash separators on Windows so that paths from `uri_to_path()` and
/// `Tab::path` always match.
pub fn normalize_lsp_path(path: &std::path::Path) -> std::path::PathBuf {
    let p = crate::path_utils::normalize_path(path.to_path_buf());
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        // Uppercase drive letter: "g:\..." → "G:\..."
        if s.len() >= 2 && s.as_bytes()[1] == b':' {
            let mut chars: Vec<u8> = s.as_bytes().to_vec();
            chars[0] = chars[0].to_ascii_uppercase();
            return std::path::PathBuf::from(String::from_utf8_lossy(&chars).into_owned());
        }
        p
    }
    #[cfg(not(windows))]
    {
        p
    }
}

/// Convert a file path to a `file://` URI.
pub fn path_to_uri(path: &std::path::Path) -> String {
    let normalized = normalize_lsp_path(path);
    let s = normalized.display().to_string().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{}", s)
    } else {
        format!("file:///{}", s)
    }
}

/// Map a file extension to an LSP `languageId`.
pub fn language_id_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "go" => "go",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "json" => "json",
            "css" | "scss" | "less" => "css",
            "html" | "htm" => "html",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" | "cxx" => "cpp",
            "md" | "markdown" => "markdown",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            _ => "plaintext",
        },
        None => "plaintext",
    }
}
