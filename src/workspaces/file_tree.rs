//! File tree data structures and directory scanning.

// Allow dead code - includes tree traversal methods and statistics for future
// file tree features like search, bulk operations, and state restoration
// - manual_strip: Pattern matching for glob patterns is clearer with explicit slicing
#![allow(dead_code)]
#![allow(clippy::manual_strip)]

use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// File Tree Node
// ─────────────────────────────────────────────────────────────────────────────

/// A node in the file tree representing a file or directory.
#[derive(Debug, Clone)]
pub struct FileTreeNode {
    /// Display name of the file or folder
    pub name: String,

    /// Full path to this item
    pub path: PathBuf,

    /// Type of node (file or directory with children)
    pub kind: FileTreeNodeKind,

    /// Whether this node is expanded in the UI (for directories)
    pub is_expanded: bool,
}

/// The kind of file tree node.
#[derive(Debug, Clone)]
pub enum FileTreeNodeKind {
    /// A regular file
    File,

    /// A directory with children
    Directory {
        /// Child nodes (files and subdirectories)
        children: Vec<FileTreeNode>,
    },
}

impl FileTreeNode {
    /// Create a new file node.
    pub fn file(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            kind: FileTreeNodeKind::File,
            is_expanded: false,
        }
    }

    /// Create a new directory node.
    pub fn directory(name: String, path: PathBuf, children: Vec<FileTreeNode>) -> Self {
        Self {
            name,
            path,
            kind: FileTreeNodeKind::Directory { children },
            is_expanded: false,
        }
    }

    /// Check if this node is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self.kind, FileTreeNodeKind::Directory { .. })
    }

    /// Check if this node is a file.
    pub fn is_file(&self) -> bool {
        matches!(self.kind, FileTreeNodeKind::File)
    }

    /// Get children if this is a directory.
    pub fn children(&self) -> Option<&[FileTreeNode]> {
        match &self.kind {
            FileTreeNodeKind::Directory { children } => Some(children),
            FileTreeNodeKind::File => None,
        }
    }

    /// Get mutable children if this is a directory.
    pub fn children_mut(&mut self) -> Option<&mut Vec<FileTreeNode>> {
        match &mut self.kind {
            FileTreeNodeKind::Directory { children } => Some(children),
            FileTreeNodeKind::File => None,
        }
    }

    /// Toggle the expanded state of this node.
    pub fn toggle_expanded(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    /// Set expanded state for a node at the given path.
    pub fn set_expanded(&mut self, target_path: &Path, expanded: bool) -> bool {
        if self.path == target_path {
            self.is_expanded = expanded;
            return true;
        }

        if let FileTreeNodeKind::Directory { children } = &mut self.kind {
            for child in children {
                if child.set_expanded(target_path, expanded) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all expanded paths in this tree (for persistence).
    pub fn get_expanded_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        self.collect_expanded_paths(&mut paths);
        paths
    }

    fn collect_expanded_paths(&self, paths: &mut Vec<PathBuf>) {
        if self.is_expanded {
            paths.push(self.path.clone());
        }
        if let FileTreeNodeKind::Directory { children } = &self.kind {
            for child in children {
                child.collect_expanded_paths(paths);
            }
        }
    }

    /// Restore expanded state from a list of paths.
    pub fn restore_expanded_paths(&mut self, expanded_paths: &[PathBuf]) {
        self.is_expanded = expanded_paths.contains(&self.path);
        if let FileTreeNodeKind::Directory { children } = &mut self.kind {
            for child in children {
                child.restore_expanded_paths(expanded_paths);
            }
        }
    }

    /// Find a node by path.
    pub fn find(&self, target_path: &Path) -> Option<&FileTreeNode> {
        if self.path == target_path {
            return Some(self);
        }

        if let FileTreeNodeKind::Directory { children } = &self.kind {
            for child in children {
                if let Some(found) = child.find(target_path) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Find a mutable node by path.
    pub fn find_mut(&mut self, target_path: &Path) -> Option<&mut FileTreeNode> {
        if self.path == target_path {
            return Some(self);
        }

        if let FileTreeNodeKind::Directory { children } = &mut self.kind {
            for child in children {
                if let Some(found) = child.find_mut(target_path) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Get the file extension (lowercase) for this node.
    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Get an icon character based on the file type.
    pub fn icon(&self) -> &'static str {
        match &self.kind {
            FileTreeNodeKind::Directory { .. } => {
                if self.is_expanded {
                    "📂" // Open folder
                } else {
                    "📁" // Closed folder
                }
            }
            FileTreeNodeKind::File => {
                match self.extension().as_deref() {
                    // Markdown/Text
                    Some("md" | "markdown" | "txt" | "text") => "📄",
                    // Code files
                    Some("rs") => "🦀",
                    Some("js" | "jsx" | "ts" | "tsx") => "📜",
                    Some("py") => "🐍",
                    Some("html" | "htm") => "🌐",
                    Some("css" | "scss" | "sass") => "🎨",
                    Some("json") => "📋",
                    Some("yaml" | "yml") => "📋",
                    Some("toml") => "⚙️",
                    Some("xml") => "📰",
                    // Config files
                    Some("gitignore" | "env") => "⚙️",
                    // Images
                    Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico") => "🖼️",
                    // Documents
                    Some("pdf") => "📕",
                    Some("doc" | "docx") => "📘",
                    // Archives
                    Some("zip" | "tar" | "gz" | "rar" | "7z") => "📦",
                    // Default
                    _ => "📄",
                }
            }
        }
    }

    /// Count all files in this tree (recursive).
    pub fn file_count(&self) -> usize {
        match &self.kind {
            FileTreeNodeKind::File => 1,
            FileTreeNodeKind::Directory { children } => {
                children.iter().map(|c| c.file_count()).sum()
            }
        }
    }

    /// Count all directories in this tree (recursive).
    pub fn directory_count(&self) -> usize {
        match &self.kind {
            FileTreeNodeKind::File => 0,
            FileTreeNodeKind::Directory { children } => {
                1 + children.iter().map(|c| c.directory_count()).sum::<usize>()
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Directory Scanning
// ─────────────────────────────────────────────────────────────────────────────

/// Scan a directory and build a file tree.
///
/// Hidden patterns are glob-like patterns for files/folders to ignore.
/// Default patterns like .git, node_modules, target are always excluded.
pub fn scan_directory(root: &Path, hidden_patterns: &[String]) -> FileTreeNode {
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string();

    let children = scan_children(root, hidden_patterns);

    let mut node = FileTreeNode::directory(name, root.to_path_buf(), children);
    node.is_expanded = true; // Root is always expanded
    node
}

/// Scan children of a directory.
fn scan_children(dir: &Path, hidden_patterns: &[String]) -> Vec<FileTreeNode> {
    let mut entries: Vec<FileTreeNode> = Vec::new();

    // Read directory entries
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue, // Skip entries with invalid UTF-8 names
        };

        // Skip hidden files (starting with .)
        if name.starts_with('.') && !is_allowed_dot_file(&name) {
            continue;
        }

        // Skip files matching hidden patterns
        if should_hide(&name, hidden_patterns) {
            continue;
        }

        let node = if path.is_dir() {
            let children = scan_children(&path, hidden_patterns);
            FileTreeNode::directory(name, path, children)
        } else {
            FileTreeNode::file(name, path)
        };

        entries.push(node);
    }

    // Sort: directories first, then alphabetically (case-insensitive)
    entries.sort_by(|a, b| match (a.is_directory(), b.is_directory()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

/// Check if a dot file should be shown (some are important).
fn is_allowed_dot_file(name: &str) -> bool {
    matches!(
        name,
        ".gitignore" | ".env" | ".env.example" | ".editorconfig" | ".prettierrc" | ".eslintrc"
    )
}

/// Check if a file/folder should be hidden based on patterns.
fn should_hide(name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Simple pattern matching: exact match or glob-like
        if pattern == name {
            return true;
        }
        // Simple wildcard: *.ext
        if let Some(suffix) = pattern.strip_prefix('*') {
            if name.ends_with(suffix) {
                return true;
            }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_node_file() {
        let node = FileTreeNode::file("test.md".to_string(), PathBuf::from("/test.md"));
        assert!(node.is_file());
        assert!(!node.is_directory());
        assert!(node.children().is_none());
        assert_eq!(node.extension(), Some("md".to_string()));
    }

    #[test]
    fn test_file_tree_node_directory() {
        let children = vec![
            FileTreeNode::file("a.txt".to_string(), PathBuf::from("/dir/a.txt")),
            FileTreeNode::file("b.txt".to_string(), PathBuf::from("/dir/b.txt")),
        ];
        let node = FileTreeNode::directory("dir".to_string(), PathBuf::from("/dir"), children);

        assert!(!node.is_file());
        assert!(node.is_directory());
        assert_eq!(node.children().unwrap().len(), 2);
    }

    #[test]
    fn test_file_tree_node_icons() {
        let md = FileTreeNode::file("test.md".to_string(), PathBuf::from("/test.md"));
        assert_eq!(md.icon(), "📄");

        let rs = FileTreeNode::file("main.rs".to_string(), PathBuf::from("/main.rs"));
        assert_eq!(rs.icon(), "🦀");

        let dir = FileTreeNode::directory("src".to_string(), PathBuf::from("/src"), vec![]);
        assert_eq!(dir.icon(), "📁");
    }

    #[test]
    fn test_expanded_paths() {
        let mut child =
            FileTreeNode::directory("child".to_string(), PathBuf::from("/root/child"), vec![]);
        child.is_expanded = true;

        let mut root =
            FileTreeNode::directory("root".to_string(), PathBuf::from("/root"), vec![child]);
        root.is_expanded = true;

        let paths = root.get_expanded_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&PathBuf::from("/root")));
        assert!(paths.contains(&PathBuf::from("/root/child")));
    }

    #[test]
    fn test_should_hide() {
        let patterns = vec!["node_modules".to_string(), "*.log".to_string()];

        assert!(should_hide("node_modules", &patterns));
        assert!(should_hide("debug.log", &patterns));
        assert!(!should_hide("src", &patterns));
        assert!(!should_hide("main.rs", &patterns));
    }

    #[test]
    fn test_file_count() {
        let tree = FileTreeNode::directory(
            "root".to_string(),
            PathBuf::from("/root"),
            vec![
                FileTreeNode::file("a.txt".to_string(), PathBuf::from("/root/a.txt")),
                FileTreeNode::directory(
                    "sub".to_string(),
                    PathBuf::from("/root/sub"),
                    vec![
                        FileTreeNode::file("b.txt".to_string(), PathBuf::from("/root/sub/b.txt")),
                        FileTreeNode::file("c.txt".to_string(), PathBuf::from("/root/sub/c.txt")),
                    ],
                ),
            ],
        );

        assert_eq!(tree.file_count(), 3);
        assert_eq!(tree.directory_count(), 2); // root + sub
    }
}
