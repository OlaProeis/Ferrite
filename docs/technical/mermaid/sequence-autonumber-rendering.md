# Sequence Diagram Autonumber Rendering

When autonumber is active at parse time, each message may carry a stamped `sequence_number`; the renderer draws a small rounded-rect badge at the **arrow start** for those messages only, matching Mermaid.js (`from_x + dir * 12`, `y - 18`). Midpoint placement hid badges under labels; numbered messages also shift the label center right so text clears the badge.

## Behaviour

| Condition | Result |
|-----------|--------|
| No `autonumber` directive (or only `autonumber off` before any on) | `Message.sequence_number` stays `None`; no badges |
| `autonumber` / `autonumber start step` | Subsequent messages get `Some(n)` in source order until `autonumber off` |
| `autonumber off` mid-diagram | Stops stamping; later messages unnumbered until another `autonumber` line |
| `autonumber` again after `off` | Resumes from the new start (default 1, step 1) |

`SequenceDiagram.autonumber` still reflects the **last** top-level directive (for settings display and tests). Per-message badges use **`Message.sequence_number`**, not a render-time counter from that field.

Numbering walks statements in source order, including messages inside `alt/else/opt/loop/par` blocks. Notes, `activate`/`deactivate`, and block header labels are **not** numbered. Step uses `saturating_add` — never `+=` (overflow panic in debug at `u64::MAX`).

## Implementation

In `src/markdown/mermaid/sequence.rs`:

- **`AutonumberParseState`** — while parsing, applies each `autonumber` / `autonumber off` line and stamps `sequence_number` on each `Message` when active.
- **`draw_message`** — draws a badge only when `message.sequence_number` is `Some`; shifts label center x by `autonumber_badge_width` + padding when a badge is present.
- **`draw_autonumber_badge`** / **`autonumber_badge_width`** — pill layout at `(from_x + dir * 12, y)` where `dir` is `±1` along the arrow.
- **`collect_message_autonumbers`** — test helper that collects stamped `sequence_number` values in statement order.

Sequence natural size and frontmatter title height use the `&dyn TextMeasurer` passed into `measure_mermaid_diagram` (`EguiTextMeasurer` from the widget) so CJK labels fill the pane.

## Tests

Unit tests in `sequence.rs` (`mod tests`):

- `autonumber_default_sequence_including_alt` — default `1, 2, 3, 4, 5` across five messages in an `alt/else` block
- `autonumber_custom_start_and_step` — `autonumber 10 5` → `10, 15, 20, 25, 30`
- `autonumber_off_mid_diagram_stops_numbering` — two numbered messages, then `autonumber off`, third unnumbered
- `autonumber_off_then_on_resumes_numbering` — off → unnumbered → on → `1, 2`
- `autonumber_skips_notes_and_activation_directives` — only messages numbered
- `autonumber_absent_produces_no_message_numbers` — regression when directive absent
- `autonumber_step_uses_saturating_add` — `u64::MAX` / `u32::MAX` step does not overflow
- `autonumber_label_center_clears_badge` — label layout clears badge right edge
- `autonumbering_is_not_autonumber_directive` — first-token match only

## Related

- `docs/technical/mermaid/sequence-autonumber-parsing.md` — parsing `autonumber` into `AutoNumber` and final `SequenceDiagram.autonumber`
- `docs/technical/mermaid/sequence-control-blocks.md` — broader sequence diagram semantics (`## autonumber` section)
