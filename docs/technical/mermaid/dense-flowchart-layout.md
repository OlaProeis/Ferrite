# Dense Flowchart Layout

Targeted Sugiyama tuning for heavy `&` fan-out, cross-subgraph edges, and bidirectional links (GitHub #165). Best-effort within v0.3.1 — not a new layout algorithm.

## Problem

Dense layered-architecture flowcharts (many nodes per layer from `&` expansion, ~40 edges, five subgraphs) crowd siblings, increase crossings, and stack parallel edges. Residual visual gaps vs mermaid.live are tracked in the parity matrix (task 14).

## Configuration

`FlowLayoutConfig` in `src/markdown/mermaid/flowchart/layout/config.rs`:

| Helper | Behaviour |
|--------|-----------|
| `adaptive_cross_spacing(base, layer_len)` | When `layer_len > 4`, scale cross-axis spacing by `(1 + 0.1 × (N − 4)).min(2.0)`. |
| `crossing_iterations_for(edge_count)` | 18 passes when `edge_count > 30`, 12 when `> 15`, else base (4). |

`layout_flowchart()` in `layout/mod.rs` applies a **dense profile** when `edge_count > 30`:

- `node_spacing`: `(65, 120)` instead of `(50, 60)`
- `crossing_reduction_iterations`: from `crossing_iterations_for()`

Adaptive cross spacing is used in Sugiyama coordinate assignment (`sugiyama.rs`) and subgraph internal layout (`subgraph.rs`).

## Overlap resolution (same layer)

After branch alignment in `assign_coordinates_with_subgraphs()`:

1. **`resolve_layer_overlaps`** — per layer, iterative split of cross-axis violations down to `min_spacing`.
2. **`shift_overlapping_siblings_right`** — one-sided push right when pairs still overlap.

Nodes with `%% @pos` hints are excluded. Overlap checks in tests must use `Rect::intersects()` — `intersect().width()` alone is positive for column-aligned nodes on different layers (false positive).

Sibling positions after relaxation/shift are clamped `.max(margin)`; one-sided shifts use `.max(0.0)` so coordinates stay non-negative (`sugiyama.rs`).

## Edge rendering

**Parallel forward lanes** (`render/edges.rs`):

- `compute_forward_edge_lanes()` groups non-back edges by **`(from, to)` node pair** — not by layer pair. Layer-pair grouping on `A & B & C & D & E --> X & Y` stacked ten offsets and detached arrows from node boxes.
- `forward_lane_pixel_offset` clamps to `±(min(from_span, to_span) / 2 − 6)` (width for TD/BU, height for LR/RL).
- `FORWARD_EDGE_LANE_SPACING` is 10px in `utils.rs`.

**Bidirectional dedup** (`render/edges.rs`, `render/mod.rs`):

- Parser emits one `FlowEdge` for `A <--> B` with both arrowheads.
- `should_skip_bidirectional_duplicate()` / `mark_bidirectional_drawn()` skip the reverse canonical pair at draw time so only one path is rendered.

## Tests

In `src/markdown/mermaid/mod.rs`:

- `ISSUE_165_FLOWCHART` — #165 layered-architecture fixture (20 nodes, 5 subgraphs).
- `test_layout_issue_165_dense_flowchart_no_overlap` — all node pairs `!intersects()`; forward edge paths avoid unrelated node bodies.
- `test_bidirectional_edge_single_draw` — one parsed edge, reverse skipped at render.

Also: `forward_lanes_grouped_per_node_pair_and_clamped` in `edges.rs`; `sugiyama_overlap_resolution_keeps_non_negative_coords` in `layout/mod.rs`.

Use a wide layout width (e.g. 2400px) in tests to limit label wrapping.

## Related

- [Flowchart Subgraphs](./flowchart-subgraphs.md) — subgraph layout and bounding boxes
- [Flowchart Subgraph Header Parsing](./flowchart-subgraph-header-parsing.md) — CJK/multi-word titles in #165 fixture
