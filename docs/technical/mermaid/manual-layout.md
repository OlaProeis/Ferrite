# Flowchart Manual Layout (`%% @pos`)

Ferrite-only manual node positioning for Mermaid flowcharts. Hints are stored
at parse time and applied during layout; other renderers treat `%%` lines as
comments and ignore them.

## Syntax

```
%% @pos <node_id> <x> <y>
```

- One hint per line, anywhere in a `flowchart` / `graph` block (before or after
  node definitions).
- `node_id` must match an existing node id.
- `x` and `y` are floating-point layout coordinates (see below).
- Invalid hints produce a non-fatal warning; the diagram still renders. See
  [Flowchart @pos hints](./flowchart-pos-hints.md) for validation rules.

**Scope:** flowcharts only. Git graph, sequence, state, and other diagram types
do not read `@pos` lines.

## Coordinate system

| Property | Value |
|----------|--------|
| Unit | Layout pixels (same space as rendered node rects) |
| Origin | Top-left of the diagram content area |
| Anchor | **Top-left corner** of the node bounding box |

Hints are applied **after** the normal Sugiyama layout and margin
normalization (`compute_subgraph_layouts` shift). Values are absolute — not
relative to auto-computed positions.

Example: `%% @pos A 120 80` places node `A` so `NodeLayout.pos` is
`(120.0, 80.0)`.

## Layout pipeline

`layout_flowchart()` in `flowchart/layout/mod.rs`:

1. **Sugiyama pass** — layered layout, branch-parent alignment (FC-83a),
   `resolve_layer_overlaps` sibling spacing.
2. **Subgraph bounds + margin shift** — content normalized so minimum extent
   starts at `config.margin` (20 px).
3. **`apply_position_hints()`** — overwrite `NodeLayout.pos` for each entry in
   `Flowchart.position_hints`.
4. **`recompute_layout_bounds()`** — expand `total_size` if hints extend past
   auto bounds.

### Hinted nodes and overlap resolution

Nodes with a valid `@pos` hint are **excluded** from
`resolve_layer_overlaps` in `sugiyama.rs`. Their Sugiyama position may still
be computed (for unhinted siblings), but overlap resolution will not move
hinted ids before the override step.

Unhinted nodes keep full automatic layout; only hinted ids are replaced.

### Edges and obstacle routing

Edge anchoring and FC-83a obstacle routing read `NodeLayout` rects from the
final layout each frame. No separate re-anchoring pass is required — overrides
run before render, so connectors attach to post-hint rectangles automatically.

## Mermaid Live round-trip

`%%` lines are standard Mermaid comments. The same source file:

- **Ferrite:** applies hints; positions match `@pos` values.
- **Mermaid Live / mermaid.js:** ignores hints; uses automatic dagre layout.

This makes `@pos` a safe, portable annotation for Ferrite-specific tuning
without breaking other tools.

## Fixture and tests

Manual repro: `test_md/test_pos_hints.md` (two valid hints + invalid-id
warning case).

Unit tests in `flowchart/layout/mod.rs` (`pos_hint_layout_tests`):

- Hinted nodes land at exact coordinates
- Unhinted nodes match auto-layout baseline
- Edge endpoints lie on overridden node rects
- Unknown hint id → warning, layout unchanged

Parse tests remain in `flowchart/parser.rs` (`pos_hint_*`).

## Out of scope (v0.3.1)

- Drag-to-reposition with `@pos` write-back (Tier C)
- `@pos` on non-flowchart diagram types
- Subgraph bounds recomputation after hint moves (hinted nodes inside subgraphs
  may extend past the pre-hint subgraph box)

## Related docs

- [Flowchart @pos hints (parsing)](./flowchart-pos-hints.md)
- [Flowchart layout algorithm](./flowchart-layout-algorithm.md)
- [Mermaid parity matrix](./mermaid-parity-matrix.md)
