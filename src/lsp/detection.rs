//! Map file extensions to default language server commands.
//!
//! Paths are hints only; users will override via settings in later tasks.

use std::path::Path;

/// How to launch a language server (stdio JSON-RPC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspServerSpec {
    /// Executable name or path (`PATH` resolved by the OS).
    pub program: String,
    pub args: Vec<String>,
}

impl LspServerSpec {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

/// Returns a built-in server mapping for well-known extensions, if any.
pub fn detect_lsp_server_for_path(path: &Path) -> Option<LspServerSpec> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "rs" => Some(LspServerSpec::new("rust-analyzer", Vec::<String>::new())),
        "py" => Some(LspServerSpec::new("pylsp", Vec::<String>::new())),
        "go" => Some(LspServerSpec::new("gopls", Vec::<String>::new())),
        "ts" | "tsx" => Some(LspServerSpec::new(
            "typescript-language-server",
            ["--stdio"],
        )),
        "js" | "jsx" | "mjs" | "cjs" => Some(LspServerSpec::new(
            "typescript-language-server",
            ["--stdio"],
        )),
        "json" => Some(LspServerSpec::new(
            "vscode-json-language-server",
            ["--stdio"],
        )),
        "css" | "scss" | "less" => Some(LspServerSpec::new(
            "vscode-css-language-server",
            ["--stdio"],
        )),
        "html" | "htm" => Some(LspServerSpec::new(
            "vscode-html-language-server",
            ["--stdio"],
        )),
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => {
            Some(LspServerSpec::new("clangd", Vec::<String>::new()))
        }
        _ => None,
    }
}

/// User-facing install hint for a server binary that was not found on PATH.
pub fn install_hint(program: &str) -> &'static str {
    match program {
        "rust-analyzer" => "Install via: rustup component add rust-analyzer",
        "pylsp" => "Install via: pip install python-lsp-server",
        "gopls" => "Install via: go install golang.org/x/tools/gopls@latest",
        "typescript-language-server" => {
            "Install via: npm i -g typescript-language-server typescript"
        }
        "vscode-json-language-server" => "Install via: npm i -g vscode-langservers-extracted",
        "vscode-css-language-server" => "Install via: npm i -g vscode-langservers-extracted",
        "vscode-html-language-server" => "Install via: npm i -g vscode-langservers-extracted",
        "clangd" => "Install clangd from your system package manager or LLVM releases",
        _ => "Ensure the language server binary is on your PATH",
    }
}

/// Scan a workspace root for file extensions and return unique server specs.
/// Walks the first two directory levels to avoid deep traversals.
pub fn detect_servers_for_workspace(root: &std::path::Path) -> Vec<(String, LspServerSpec)> {
    use std::collections::HashMap;
    let mut found: HashMap<String, LspServerSpec> = HashMap::new();

    let walker = walkdir::WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        if let Some(spec) = detect_lsp_server_for_path(entry.path()) {
            found.entry(spec.program.clone()).or_insert(spec);
        }
    }

    found
        .into_iter()
        .map(|(program, spec)| (program, spec))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_maps_to_rust_analyzer() {
        let s = detect_lsp_server_for_path(Path::new("src/main.rs")).expect("rs");
        assert_eq!(s.program, "rust-analyzer");
        assert!(s.args.is_empty());
    }

    #[test]
    fn unknown_ext_returns_none() {
        assert!(detect_lsp_server_for_path(Path::new("readme.md")).is_none());
    }
}
