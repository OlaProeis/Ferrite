# Git Graph Lane Layout

Layout stage for Mermaid `gitGraph` diagrams: branches as horizontal lanes, commits as sequence columns. Parsing lives in [git-graph-parser.md](./git-graph-parser.md); painting in [git-graph-render.md](./git-graph-render.md).

## Location

| File | Role |
|------|------|
| `src/markdown/mermaid/git_graph/layout.rs` | `layout_git_graph()`, lane assignment, connector endpoints, bounds |
| `src/markdown/mermaid/git_graph/render.rs` | `render_git_graph()` — consumes layout output |
| `src/markdown/mermaid/git_graph/types.rs` | AST types (`GitGraph`, `GitCommit`, `GitBranch`, …) |
| `src/markdown/mermaid/git_graph/parser.rs` | `parse_git_graph()` and grammar helpers |
| `src/markdown/mermaid/git_graph/mod.rs` | Module root, public re-exports |

## Public API

```rust
pub fn layout_git_graph(graph: &GitGraph, config: GitGraphLayoutConfig) -> GitGraphLayout;
pub fn assign_branch_lanes(branches: &[GitBranch]) -> HashMap<String, usize>;
```

Default spacing (`GitGraphLayoutConfig::default()`): margin 30, commit_spacing 50, lane_spacing 60, commit_radius 8.

## Lane model

| Rule | Behavior |
|------|----------|
| Default lanes | First branch (`main`) → lane 0; others follow **declaration order** in `GitGraph::branches` |
| `order:` override | `branch <name> order: N` assigns lane `N`; undeclared branches fill remaining slots in declaration order |
| Sequence | Commit `sequence` = declaration index (0-based); **no topological re-sort** |

## Coordinates

**LR (default):** time flows left → right.

```
x = margin + sequence × commit_spacing
y = margin + lane × lane_spacing
```

**TB:** axis transpose (same approach as [flowchart-direction.md](./flowchart-direction.md)) — swap x/y so time flows top → bottom.

## Layout output (`GitGraphLayout`)

| Field | Purpose |
|-------|---------|
| `commits` | Per-commit `sequence`, `lane`, dot `pos` |
| `branch_lines` | Polyline `start` → `end` per branch; `branch_off` on parent at commit before branch’s first commit |
| `merge_connectors` | Source branch tip (last commit before merge) → merge commit dot |
| `cherry_pick_connectors` | Source commit dot → cherry-pick commit dot |
| `branch_lanes` | Branch name → lane index |
| `bounds` | `Vec2` from max lane/sequence counts + margin/radius — **no hardcoded canvas size** |

## Branch-off inference

The parser does not store branch-creation events. Layout infers branch-off from declaration order: for a branch’s first commit at index `i`, `branch_off` is the dot at index `i - 1` (parent = that commit’s branch). Main has no branch-off.

## Grammar (layout-relevant statements)

| Statement | Layout effect |
|-----------|---------------|
| `commit` | New dot at current branch lane, next sequence column |
| `branch <name>` | Switches active branch; optional `order: N` overrides lane |
| `checkout` / `switch` | Switches active branch (must exist) |
| `merge <branch>` | Merge commit on current branch; connector from source branch tip |
| `cherry-pick id:"…"` | Cherry-pick commit; dashed connector to source commit dot |
| Header `gitGraph` / `LR:` / `TB:` | `Lr` (default) or `Tb` axis transpose |

Full parse grammar: [git-graph-parser.md](./git-graph-parser.md).

## Fixtures & Mermaid Live parity

Manual repros in `test_md/test_git_graphs.md`. Structural topology verified by `fixture_*_topology` tests in `git_graph/parser.rs`.

| ID | Scenario | Mermaid Live comparison |
|----|----------|-------------------------|
| GG-01 | Feature branch + merge | **Match** — main/develop lanes, branch-off at ROOT, merge connector DEV1 → MERGE1 |
| GG-02 | Multi-branch `order:` | **Match** — lanes main=0, feature=1, hotfix=2; branch-offs from BASE |
| GG-03 | Tags + cherry-pick | **Match** — tag badges, dashed cherry-pick arc abc → main, HIGHLIGHT dot |

Pixel-exact rendering is not required; lane structure, merge topology, and labels align with Mermaid Live for all three fixtures.

**Visual check:** Open `test_md/test_git_graphs.md` → Rendered or Split view → compare to [mermaid.live](https://mermaid.live).

## Tests

```bash
cargo test git_graph::layout
cargo test git_graph::parser::tests::fixture
```

Unit tests in `git_graph/layout.rs` cover lane assignment (with/without `order:`), sequence indices, LR vs TB transpose, merge/cherry-pick connectors, branch-off points, and bounds scaling. Fixture topology tests in `git_graph/parser.rs` lock GG-01–GG-03 against the table above.

## Related

- [Git graph parser](./git-graph-parser.md)
- [Git graph rendering](./git-graph-render.md)
- [Flowchart direction](./flowchart-direction.md) — axis transpose pattern
- [Mermaid parity matrix](./mermaid-parity-matrix.md)
