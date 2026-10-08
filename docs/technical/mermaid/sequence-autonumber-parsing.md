# Sequence Diagram Autonumber Parsing

The `autonumber` directive is parsed from sequence diagram source and stored on `SequenceDiagram` for rendering.

## Mermaid syntax

| Line | Parsed result |
|------|---------------|
| *(absent)* | `autonumber: None` |
| `autonumber` | `Some(AutoNumber { start: 1, step: 1 })` |
| `autonumber 10` | `Some(AutoNumber { start: 10, step: 1 })` |
| `autonumber 10 5` | `Some(AutoNumber { start: 10, step: 5 })` |
| `autonumber off` | `None` |

Keyword matching is case-insensitive on the **first whitespace token** (`is_autonumber_directive`). `autonumber 10` matches; `autonumbering` does **not** (`starts_with("autonumber")` is wrong). Invalid integer arguments fall back to defaults (`start: 1`, `step: 1`). Multiple `autonumber` lines: **last wins**.

## Types

In `src/markdown/mermaid/sequence.rs`:

- `AutoNumber { start: u64, step: u64 }` — numbering configuration.
- `SequenceDiagram.autonumber: Option<AutoNumber>` — `None` when directive absent or disabled.

## Implementation

`parse_sequence_diagram()` handles `autonumber` early in the line loop (after blank/comment skip, before participants). `parse_autonumber_args()` splits optional start/step integers with resilient parsing.

Rendering: see `docs/technical/mermaid/sequence-autonumber-rendering.md`.

## Tests

Unit tests in `sequence.rs` (`mod tests`):

- `autonumber_without_args`
- `autonumber_with_start_and_step`
- `autonumber_with_start_only`
- `autonumber_absent`
- `autonumber_off`
- `autonumber_last_wins`
- `autonumber_invalid_args_use_defaults`
- `autonumbering_is_not_autonumber_directive`

## Related

- `docs/technical/mermaid/sequence-control-blocks.md` — broader sequence diagram semantics (`## autonumber` section).
