# Sequence Diagram Autonumber Rendering

When `SequenceDiagram.autonumber` is set, message indices are drawn as small rounded-rect badges at the **arrow start**, matching Mermaid.js (`from_x + dir * 12`, `y - 18`). Midpoint placement hid badges under labels.

## Behaviour

| Condition | Result |
|-----------|--------|
| `autonumber: None` | No badges; render unchanged from pre-autonumber behaviour |
| `autonumber: Some(an)` | Each `SeqStatement::Message` gets badge `msg_num`, then `msg_num = msg_num.saturating_add(an.step)` |

Numbering walks statements in source order, including messages inside `alt/else/opt/loop/par` blocks. Notes, `activate`/`deactivate`, and block header labels are **not** numbered. Use `saturating_add` — never `+=` (overflow panic in debug at `u64::MAX`).

## Implementation

In `src/markdown/mermaid/sequence.rs`:

- `render_sequence_diagram` — initializes `msg_num = an.start` and `msg_step = an.step` when autonumber is active.
- `draw_statements` / `draw_block` — thread `msg_num: &mut Option<u64>` and `msg_step` through recursive traversal.
- `draw_message` — accepts `msg_num: Option<u64>`; when present, `draw_autonumber_badge` paints a pill at `(from_x + dir * 12, y)` where `dir` is `±1` along the arrow.
- `collect_message_autonumbers` — test helper that mirrors the render walk without egui.

Sequence natural size and frontmatter title height use the `&dyn TextMeasurer` passed into `measure_mermaid_diagram` (`EguiTextMeasurer` from the widget) so CJK labels fill the pane.

## Out of scope

- `autonumber off` mid-diagram (Tier B stretch) — only the last top-level directive applies today.

## Tests

Unit tests in `sequence.rs` (`mod tests`):

- `autonumber_default_sequence_including_alt` — default `1, 2, 3, 4, 5` across five messages in an `alt/else` block
- `autonumber_custom_start_and_step` — `autonumber 10 5` → `10, 15, 20, 25, 30`
- `autonumber_skips_notes_and_activation_directives` — only messages numbered
- `autonumber_absent_produces_no_message_numbers` — regression when directive absent
- `autonumber_step_uses_saturating_add` — `u64::MAX` / `u32::MAX` step does not overflow
- `autonumbering_is_not_autonumber_directive` — first-token match only

## Related

- `docs/technical/mermaid/sequence-autonumber-parsing.md` — parsing `autonumber` into `AutoNumber`
- `docs/technical/mermaid/sequence-control-blocks.md` — broader sequence diagram semantics (`## autonumber` section)
