# Git Graph Parser

Parse-time support for Mermaid `gitGraph` diagrams in Ferrite. Implements the core Mermaid gitGraph grammar (#83 parity). Layout: [git-graph-layout.md](./git-graph-layout.md); rendering is a separate stage (task 7).

## Location

| File | Role |
|------|------|
| `src/markdown/mermaid/git_graph/types.rs` | AST types |
| `src/markdown/mermaid/git_graph/parser.rs` | `parse_git_graph()` |
| `src/markdown/mermaid/git_graph/layout.rs` | `layout_git_graph()`, lane assignment, connector endpoints |
| `src/markdown/mermaid/validation.rs` | Surfaces parser warnings via `validate_mermaid_source()` |

## Public API

```rust
pub fn parse_git_graph(source: &str) -> Result<GitGraph, String>;
```

Hard failure only when no commits are found. Non-fatal issues are collected in `GitGraph::warnings`.

## Parsed types

| Type | Purpose |
|------|---------|
| `GitGraphOrientation` | `Lr` (default) or `Tb` from header (`gitGraph`, `gitGraph LR:`, `gitGraph TB:`) |
| `GitCommitKind` | `Normal`, `Reverse`, `Highlight` from `commit type:` |
| `GitCommit` | `id`, `branch`, `message`, `tag`, `kind`, merge/cherry-pick flags |
| `GitBranch` | `name`, `color_idx`, optional `order` from `branch <name> order: <n>` |
| `GitGraphWarning` | `{ line, message }` — 1-indexed line within diagram body |

## Supported grammar

| Statement | Notes |
|-----------|-------|
| `commit` | Options: `id:`, `msg:`, `tag:`, `type: NORMAL\|REVERSE\|HIGHLIGHT` |
| `branch <name>` | Optional `order: <n>`; quoted names stripped |
| `checkout` / `switch` | `switch` is alias for `checkout` |
| `merge <branch>` | Optional `id:` |
| `cherry-pick id:"…"` | Creates cherry-pick commit; unknown id → warning |
| Header | `gitGraph` / `gitGraph LR:` → `Lr`; `gitGraph TB:` → `Tb` |

Unknown statements produce a warning and parsing continues — never silent drops.

## Validation integration

`validate_mermaid_source()` calls `parse_git_graph()` and, if `graph.warnings` is non-empty, returns `Err(MermaidError)` with the first warning formatted as `Line N: …`. The inline validation pipeline ([mermaid-inline-validation.md](./mermaid-inline-validation.md)) shows the warning banner and editor squiggles; the graph itself still parses successfully for rendering when invoked directly.

Cherry-pick with an unknown commit id emits: `cherry-pick references unknown commit id: …`.

## Tests

Parser unit tests live in `git_graph/parser.rs` (`#[cfg(test)] mod tests`). Validation tests in `validation.rs` cover warning surfacing.

```bash
cargo test git_graph
cargo test gitgraph
```

## Related

- [Mermaid diagrams overview](./mermaid-diagrams.md)
- [Mermaid inline validation](./mermaid-inline-validation.md)
- [Mermaid parity matrix](./mermaid-parity-matrix.md) — gitGraph layout/render status
