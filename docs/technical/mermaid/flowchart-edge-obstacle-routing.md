# Flowchart Edge Obstacle Routing

Native flowchart edges avoid passing through node boxes that are not the edge's source or target. This complements [subgraph edge routing](./subgraph-edge-routing.md), which handles cluster borders only.

## Problem

Sugiyama layout assigns layers correctly and marks back-edges, but the renderer previously drew:

- **Forward edges** as a single straight segment between anchor points, which could cut through intermediate nodes (e.g. `D → F` diagonals crossing `E`).
- **Back-edges** with a fixed ±40 px cubic-bezier offset, which was too tight for feedback loops like `E → B` and `F → B` in [FC-83a](../../../test_md/test_mermaid_issue_83.md).

## Approach

### Obstacle set

For each edge, build padded rects for every node **except** the source and target (`collect_node_obstacles` in `utils.rs`, padding `NODE_OBSTACLE_PADDING = 8`).

### Rendering allocation

`render_flowchart` sizes the painter from **actual node/subgraph bounds** (not only cached `total_size`) and adds horizontal padding when back-edges exist so side-channel loops are not clipped by the egui painter rect.

1. Compute anchor points (`compute_edge_endpoints`).
2. Apply existing subgraph boundary waypoints when needed.
3. If any segment hits an obstacle, try orthogonal detours:
   - Midpoint horizontal bus (`start → (start.x, mid) → (end.x, mid) → end`) for TD/BT.
   - L-shaped shortcuts and left/right side corridors outside the obstacle union.
4. Fall back to a wide side corridor (`BACK_EDGE_LOOP_MARGIN × 2`) if no candidate clears all obstacles.

### Back-edges

1. Pick loop side from source position (right-half sources loop on the right, left-half on the left).
2. Fixed offset from `graph_bounds` (`BACK_EDGE_LOOP_MARGIN = 24 px`) — no iterative expansion.
3. Parallel back-edges to the same target/side get distinct lanes (`BACK_EDGE_LANE_SPACING = 36 px`).
4. **Inner lane** (lane 0, source below target in TD): exit **top-outer corner** → **vertical segment along source east/west edge** (not the centre column) → horizontal into target **side at centre height**. Stepped-outward variants clear branch siblings (FC-83a: `decide`); tight loop fallback stays just outside the source before the graph margin.
5. **Outer lanes** (lane 1+): exit source side centre → horizontal to margin at source row → vertical to target entry (bottom corner for separate arrowheads).
6. If the horizontal exit leg would cross same-row nodes, detour below/above the source before reaching the margin.

### Inner back-edge direct path

Lane-0 back-edges (the loop nearest the graph) use `try_inner_back_edge_direct_path`:

1. Exit the source at the **top-outer corner** (right corner if loop side is right, left corner if left).
2. Rise vertically along the source's outer edge until just above the target row.
3. Turn 90° and enter the target at its **side centre** (Preview right-edge mid-height in FC-83a).

The router tries several stepped-outward variants (`inner_back_edge_path_candidates`) so siblings at the same branch level (e.g. FC-83a `decide` between Preview and Edit Definition) are cleared. If every candidate hits an obstacle, `try_inner_back_edge_tight_loop` keeps the loop just outside the source rect rather than expanding to the graph margin.

### Layer-spacing safety net (related)

The sister layout pass (`sugiyama::resolve_layer_overlaps`, documented in [`flowchart-layout-algorithm.md`](./flowchart-layout-algorithm.md)) enforces `node_spacing.x` between every pair of siblings on a layer. Without it the branch-parent barycenter snap can pull a node onto a sibling (originally seen on coffee-machine `C` colliding with `H` by ~68 px); with it, every same-layer pair is guaranteed a clean gap before the renderer ever runs. This is why the obstacle router can assume the layout it sees is non-overlapping.

## Key files

| File | Role |
|------|------|
| `src/markdown/mermaid/flowchart/utils.rs` | `segment_intersects_rect`, `path_intersects_any`, `bezier_intersects_any`, `collect_node_obstacles`, `union_rect_bounds`, `layout_content_size`; constants `NODE_OBSTACLE_PADDING`, `BACK_EDGE_LOOP_MARGIN`, `BACK_EDGE_LANE_SPACING` |
| `src/markdown/mermaid/flowchart/render/edges.rs` | `route_forward_edge`, `try_orthogonal_route`, `route_via_side_corridor`, `compute_back_edge_lanes`, `compute_back_edge_path`, `try_inner_back_edge_direct_path`, `build_back_edge_side_path`, `draw_edge` integration |
| `src/markdown/mermaid/flowchart/render/mod.rs` | Painter sized from `layout_content_size` + side padding; passes `BackEdgeLane` + full `layout.nodes` into `draw_edge` |
| `src/markdown/mermaid/flowchart/layout/sugiyama.rs` | `align_branch_nodes_to_children` (alone-on-layer barycenter snap) + `resolve_layer_overlaps` (sibling-spacing safety net) |

## Verification

```bash
cargo build
cargo test obstacle_tests
cargo test fc_83a
cargo test test_layout_coffee_machine_all_nodes
```

Manual: open `test_md/test_mermaid_issue_83.md` (FC-83a) and the coffee-machine chart at the top of `test_md/test_flowcharts.md` in Rendered/Split view and compare to [Mermaid Live Editor](https://mermaid.live). Spot-check the rest of `test_md/test_flowcharts.md` for subgraph and linkStyle regressions.

## Out of scope (FC-83a)

- `linkStyle interpolate basis` renders smooth Catmull-Rom curves on routed edge paths (v0.3.1).
- Font Awesome icon **glyph** rendering (FC-83b prefix stripping done — `flowchart-fa-labels.md`)
- Per-subtree column packing (Reingold–Tilford-style tree layout) — would tighten branches further but requires a deeper layout rewrite; tracked in the parity matrix.
