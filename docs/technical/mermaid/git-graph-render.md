# Git Graph Rendering

Paint stage for Mermaid `gitGraph` diagrams: consumes lane layout output and draws branch polylines, connectors, commit dots, and labels. Parsing and layout are documented in [git-graph-parser.md](./git-graph-parser.md) and [git-graph-layout.md](./git-graph-layout.md).

## Location

| File | Role |
|------|------|
| `src/markdown/mermaid/git_graph/render.rs` | `render_git_graph()` — painter allocation, paint order, label prep |
| `src/markdown/mermaid/git_graph/layout.rs` | `layout_git_graph()` input consumed each frame |
| `src/markdown/mermaid/git_graph.rs` | Parser AST; re-exports `render_git_graph` |
| `src/markdown/mermaid/mod.rs` | `render_mermaid_diagram()` dispatches `gitgraph` → parse + render |

## Public API

```rust
pub fn render_git_graph(ui: &mut Ui, graph: &GitGraph, dark_mode: bool, font_size: f32);
```

Each call runs `layout_git_graph()` with default `GitGraphLayoutConfig`, then allocates an egui painter sized from `GitGraphLayout::bounds` (minimum 300×100).

## Paint order

1. **Branch lane polylines** — horizontal segment per branch on its lane Y (LR) or X (TB); 3-segment connector from `branch_off` to first commit on branch when present
2. **Merge connectors** — 3-segment curve from source branch tip to merge commit; endpoints shortened by `commit_radius` to avoid dot overlap
3. **Cherry-pick connectors** — dashed line (`draw_dashed_line`) from source commit to cherry-pick commit
4. **Commit dots** — styled by commit kind (see below)
5. **Tag labels** — small rounded rect offset above-right of dot when `tag:` is set
6. **Commit id/msg labels** — message if present, else id; truncated with hover tooltip when wider than ~85% of `commit_spacing`
7. **Branch name labels** — left edge in LR (`x ≈ 4`), top edge in TB; truncated within margin width with hover tooltip

## Commit dot styles

| Kind | Visual |
|------|--------|
| Normal (default) | Filled circle in branch color |
| `type: REVERSE` | Filled circle + diagonal cross |
| `type: HIGHLIGHT` | Thick ring + small filled center |
| Merge commit | Filled circle + contrasting outline stroke |

## Connectors

Merge and branch-off curves reuse the legacy 3-segment approximation (horizontal bar at mid-lane):

```
source → (source.x, mid.y) → (target.x, mid.y) → target
```

Cherry-pick uses a straight dashed segment between shortened endpoints.

## Colors

Per-branch palette matches the previous vertical renderer (six colors, separate dark/light tables). Text, label background, tag, and dot-outline colors follow the same dark-mode split.

## Label measurement

Text is measured with `EguiTextMeasurer` **before** `allocate_painter` to avoid egui borrow conflicts (same pattern as flowchart render). Truncation uses `TextMeasurer::truncate_with_ellipsis`.

## Tests

Unit tests in `git_graph/render.rs`:

```bash
cargo test git_graph::render
```

Covers bounds-driven layout for a merge fixture, `shorten_toward` endpoint inset, and alternating commit label placement by lane parity.

## Related

- [Git graph lane layout](./git-graph-layout.md)
- [Git graph parser](./git-graph-parser.md)
- [Mermaid parity matrix](./mermaid-parity-matrix.md)
