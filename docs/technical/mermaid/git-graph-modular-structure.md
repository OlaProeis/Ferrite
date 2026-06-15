# Git Graph Modular Structure

The monolithic `git_graph.rs` (~500 lines) was split into focused modules under `src/markdown/mermaid/git_graph/`, mirroring the flowchart modular pattern.

**Date:** Jun 2026

## Module structure

```
src/markdown/mermaid/git_graph/
├── mod.rs       # Public API re-exports
├── types.rs     # AST types (GitGraph, GitCommit, GitBranch, …)
├── parser.rs    # parse_git_graph() + unit tests (incl. fixture topology)
├── layout.rs    # layout_git_graph(), lane assignment, connectors, bounds
└── render.rs    # render_git_graph() — paints layout output
```

## Public API

Unchanged for external consumers (`mermaid/mod.rs`):

```rust
use git_graph::{parse_git_graph, render_git_graph};
```

Internal submodules use `super::` for types and `super::layout::*` for layout types.

## Fixtures

Manual repros and Mermaid Live parity notes: `test_md/test_git_graphs.md` (GG-01–GG-03). Structural topology locked by `fixture_*_topology` tests in `parser.rs`.

## Related

- [Git graph parser](./git-graph-parser.md)
- [Git graph lane layout](./git-graph-layout.md)
- [Git graph rendering](./git-graph-render.md)
- [Flowchart modular refactor](./flowchart-modular-refactor.md) — pattern reference
- [Mermaid parity matrix](./mermaid-parity-matrix.md)
