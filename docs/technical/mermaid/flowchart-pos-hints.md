# Flowchart `%% @pos` Layout Hints (Parsing)

Parse-time collection of manual node position hints in Mermaid flowcharts.
Hints are stored on the `Flowchart` AST for the layout stage (task #11) to
apply; this document covers **syntax, parsing, and validation only**.

## Syntax

```
%% @pos <node_id> <x> <y>
```

- One hint per line, anywhere in a `flowchart` / `graph` block.
- `node_id` must match an existing node in the diagram.
- `x` and `y` are layout-space floats (top-left anchor, pixels — see
  [manual-layout.md](./manual-layout.md)).
- Regular `%%` comments without `@pos` are ignored.

**Scope:** flowcharts only. Other diagram types ignore `@pos` lines.

## Data model

`Flowchart` (`flowchart/types.rs`) carries:

| Field | Type | Purpose |
|-------|------|---------|
| `position_hints` | `HashMap<String, Pos2>` | Valid hints keyed by node id |
| `warnings` | `Vec<FlowchartWarning>` | Non-fatal parse warnings |

`FlowchartWarning` has a 1-indexed `line` and `message` (same pattern as
`GitGraphWarning`).

## Parsing

`collect_position_hints()` in `flowchart/parser.rs` runs a post-pass over the
full source after nodes and edges are parsed. This allows hints to appear
before or after node definitions.

Valid hints are inserted into `position_hints`. Invalid hints produce a
warning and are **not** stored.

## Validation rules

All invalid cases are **warnings**, never hard parse errors. The diagram
still parses and can render (layout ignores bad hints).

| Condition | Behaviour |
|-----------|-----------|
| Unknown `node_id` | Warning; hint ignored |
| Malformed syntax (wrong token count) | Warning; hint ignored |
| Non-numeric `x` or `y` | Warning; hint ignored |
| Duplicate hint for same node | Warning on second line; **first** hint kept |

Warnings surface through the existing inline-validation pipeline:

1. `validate_mermaid_source()` checks `flowchart.warnings.first()` and
   returns `MermaidError` with `Line N:` (like git graph).
2. Rendered widget shows the amber warning header + optional `@pos` hint text.
3. `compute_mermaid_diagnostics()` draws squiggles in the raw editor.

Parse still returns `Ok(Flowchart)` — rendering uses the AST; validation
failure only affects the warning banner / diagnostics.

## Caching

No cache changes required. Blake3 content hashing already covers comment
lines, so hint edits invalidate the flowchart cache automatically.

## Out of scope (v0.3.1)

- Drag-to-reposition with `@pos` write-back — Tier C

Layout application is documented in [manual-layout.md](./manual-layout.md).

## Tests

Unit tests in `flowchart/parser.rs` (`pos_hint_*`):

- Valid hints stored in `position_hints`
- Unknown node id → warning
- Malformed coordinates → warning
- Duplicate hint → warning, first position retained
- Regular `%%` comments unchanged
