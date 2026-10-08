# Issue #165 Mermaid Fidelity Fixtures

Manual QA and regression repros for [GitHub #165](https://github.com/OlaProeis/Ferrite/issues/165) — inline fit-to-pane, dense flowchart layout, CJK subgraph titles, and sequence `autonumber`.

## Fixture file

**Path:** [`test_md/test_mermaid_issue_165.md`](../../../test_md/test_mermaid_issue_165.md)

Open in **Rendered** or **Split** view. Each section includes expected behaviour, a QA checklist, and a pre-loaded [Mermaid Live Editor](https://mermaid.live) edit link (`#pako:` URL) for side-by-side comparison.

## Sections

| ID | Diagram type | What it exercises |
|----|--------------|-------------------|
| **FC-165a** | `graph TD` flowchart | Five subgraphs with CJK/multi-word titles; `&` fan-out (5×2 and cross-layer); `<-->` bidirectional edges across subgraphs; inline fit-to-pane at default preview width |
| **FC-165b** | `sequenceDiagram` | `autonumber`; `participant … as …` aliases; `alt`/`else` with ten numbered messages in source order |

## Expected outcomes (v0.3.1 target)

**FC-165a**

- All five subgraph titles render in full (e.g. `业务客户端 PEP`, not truncated to `PEP` alone).
- No node–node overlap; edges do not pass through unrelated node bodies.
- `<-->` edges show arrowheads on both ends (single path, not double-drawn).
- Wide diagram fits preview pane in **Fit** mode; **Native** toggle restores horizontal scroll.

**FC-165b**

- Messages numbered **1–10** in diagram order (including inside `alt`/`else`).
- Participant columns show alias labels, not ids only.
- `alt 认证失败` / `else 认证成功` frames render with branch labels.

## Code cross-reference

The FC-165a flowchart source matches `ISSUE_165_FLOWCHART` in `src/markdown/mermaid/mod.rs` (layout overlap unit test `test_layout_issue_165_dense_flowchart_no_overlap`). The markdown fixture adds `%%` layer comments from the original issue reporter and Mermaid Live links for manual QA.

## Related docs

- [Dense Flowchart Layout](./dense-flowchart-layout.md) — Sugiyama tuning for this graph
- [Flowchart Subgraph Header Parsing](./flowchart-subgraph-header-parsing.md) — multi-word/CJK titles
- [Sequence Autonumber Rendering](./sequence-autonumber-rendering.md) — message number badges
- [Inline Fit to Pane](./inline-fit-to-pane.md) — preview scaling
- [v0.3.1 Test Checklist §3 Mermaid](../platform/v0.3.1-test-checklist.md) — `MMD-8`–`MMD-11` rows reference this fixture
- [v0.3.1 Mermaid Manual QA Rows](../platform/v0.3.1-mermaid-manual-qa.md) — row-to-feature mapping for pre-tag QA
