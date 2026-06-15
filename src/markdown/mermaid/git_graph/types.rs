//! AST types for Mermaid `gitGraph` diagrams.

/// Layout orientation parsed from the `gitGraph` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitGraphOrientation {
    /// Horizontal lanes, time flows left → right (Mermaid default).
    #[default]
    Lr,
    /// Vertical stack, time flows top → bottom (legacy Ferrite layout).
    Tb,
}

/// Visual kind for a commit dot (`commit type:` in Mermaid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitCommitKind {
    #[default]
    Normal,
    Reverse,
    Highlight,
}

impl GitCommitKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "NORMAL" => Some(Self::Normal),
            "REVERSE" => Some(Self::Reverse),
            "HIGHLIGHT" => Some(Self::Highlight),
            _ => None,
        }
    }
}

/// A parse-time warning (non-fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitGraphWarning {
    /// 1-indexed line within the diagram source (header is line 1).
    pub line: usize,
    pub message: String,
}

/// A commit in a git graph.
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub id: String,
    pub branch: String,
    pub message: Option<String>,
    pub tag: Option<String>,
    pub kind: GitCommitKind,
    pub is_merge: bool,
    pub merge_from: Option<String>,
    pub is_cherry_pick: bool,
    pub cherry_pick_from_id: Option<String>,
}

/// A branch in a git graph.
#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub color_idx: usize,
    /// Optional lane order from `branch <name> order: <n>`.
    pub order: Option<u32>,
}

/// A git graph.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub orientation: GitGraphOrientation,
    pub commits: Vec<GitCommit>,
    pub branches: Vec<GitBranch>,
    pub warnings: Vec<GitGraphWarning>,
}
