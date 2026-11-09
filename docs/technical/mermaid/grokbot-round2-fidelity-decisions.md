# GrokBot round 2 — Mermaid fidelity decisions (ISS-32 / ISS-59 / ISS-60)

Round 2 Linux QA (`grokbot/New/qa-report-v0.3.1-linux.md`, commit `8e6ef50`) filed three mermaid rows that sit outside the #165/#166 mermaid-fidelity addendum’s shipped Tier A work. Task 15 recorded go/no-go without changing layout code.

## Authoritative decision tables

| Artifact | Location |
|----------|----------|
| Per sub-finding decisions + reasoning | [`prd-v0.3.1-mermaid-fidelity.md`](../../ai-workflow/prds/prd-v0.3.1-mermaid-fidelity.md) **§12 Round 2 QA hand-off** |
| Close-out §8 decision log (five rows) | [`prd-v0.3.1-qa-round2-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md) |
| QA report **Status** cells | [`grokbot/New/qa-report-v0.3.1-linux.md`](../../../grokbot/New/qa-report-v0.3.1-linux.md) — ISS-32, ISS-59, ISS-60 |

## Summary

| Issue | QA rows | Decision | Follow-up |
|-------|---------|----------|-----------|
| **ISS-32** | T3-43, MMD-7; MMD-11 frames; MMD-4 side note | **defer 0.3.2** | Subgraph hierarchy: title-band edge routing, true nesting, `direction LR`, inter-subgraph links, asymmetric dash-label drops, nested frame overlap on FC-165a. §5.4 #165 node-overlap tuning does not cover these. |
| **ISS-59** | MMD-13 | **done — task 17** | Measurer/paint galley ≤1.1×; residual box slack is layout padding + 1.15× factor — `cjk-label-measure-paint-parity.md`. |
| **ISS-60** | MMD-10 | **in-mermaid-PRD — cyclopsctl task 16** | Mid-diagram `autonumber off`, align MMD-10 / `sequence-autonumber-rendering.md`, badge–label overlap offset if cheap. |

## Decision labels

- **in-mermaid-PRD** — Scheduled on the 0.3.1 branch via the mermaid fidelity PRD or round 2 orchestrator tasks.
- **defer 0.3.2** — Valid mermaid.live gap; needs layout-engine or parser work beyond §5.4 best-effort; track in parity matrix / next mermaid wave.
- **won't fix** — Out of bar (not used for these three issues).

## Related

- Dense fan-out / FC-165a node overlap (shipped): [`dense-flowchart-layout.md`](dense-flowchart-layout.md)
- #165 fixtures: [`issue-165-fixtures.md`](issue-165-fixtures.md)
- Round 2 PRD overview: [`prd-v0.3.1-qa-round2-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md) §5.4
