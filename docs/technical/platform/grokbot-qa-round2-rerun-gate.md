# GrokBot Linux QA — Round 2 re-run gate

After round 2 close-out (cyclopsctl tasks 1–33 on `ferrite-v0-3-1-grokbot-qa-round-2-close-out-prd`), Linux GUI verification uses a single handoff list instead of editing the PRD when rows still fail.

## Artifacts

| Artifact | Role |
|----------|------|
| [`grokbot/New/rerun-v0.3.1-round2.md`](../../../grokbot/New/rerun-v0.3.1-round2.md) | **Hand to GrokBot** — rows, fix commits, expected results, run order |
| [`grokbot/New/qa-report-v0.3.1-linux.md`](../../../grokbot/New/qa-report-v0.3.1-linux.md) | Original full QA at `8e6ef50`; **Issues** table **Status** column updated for tasks 1–33 + prior fixes (GrokBot re-run pending until green) |
| [`docs/ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md) §9 | Ship-gate acceptance; GrokBot re-run tick stays open until the handoff passes |

## Pass criteria (summary)

1. Every row in `rerun-v0.3.1-round2.md` is **PASS** or **BLOCKED-by-input/environment** (document why).
2. **Tier 1** of [`v0.3.1-minimal-release-test.md`](v0.3.1-minimal-release-test.md): all **PASS** or **BLOCKED-by-input/environment** only.
3. Any **FAIL** is filed as a **new issue** — not a round-2 PRD edit.

## What the handoff covers

- **Round 1 blockers:** ISS-1, 2, 5, 7, 9 (commits `5325e9b`, `97347d3`, `51f5a3a`) — see also [`grokbot-qa-tier-a-rerun.md`](grokbot-qa-tier-a-rerun.md) for Tier A detail.
- **Direct fixes:** `5eec844` (settings gear/Ctrl+, with system title bar, SES dialog wording, RUN-1 stdout, PRI-3, IMG-X toasts, MMD-3/6, UX-11, terminal auto-load, Alt+Z, etc.).
- **Round 2 PRD:** rendered-edit cluster, links/HTML, search/spell/indent/locales, ISS-36 hardening, mermaid ISS-59/60 (16/17), observation fixes (SI-3/4, GIT-3, CSV-4, SPL-3, HTML-4).

## Status column semantics (New QA report)

- **`fixed` / `hardened` / `by design` / `documented limitation`** — code or docs on HEAD; **Actual** text still describes the `8e6ef50` run until GrokBot re-run updates it.
- **`triaged — defer 0.3.2`** — ISS-32 (mermaid layout); not a gate FAIL unless regression.
- **Tier B (round 1):** ISS-12/13/14/18/24/29 — see [`grokbot-qa-tier-b-status.md`](grokbot-qa-tier-b-status.md).

## Environment

Same as the original GrokBot run: Debian, X11 + Xvfb, xfwm4, Mesa llvmpipe, `run-ferrite.sh`, debug logs under `grokbot/New/logs/`.

## Related

- Round 2 PRD: [`prd-v0.3.1-qa-round2-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md)
- Mermaid decisions: [`../mermaid/grokbot-round2-fidelity-decisions.md`](../mermaid/grokbot-round2-fidelity-decisions.md)
- Agent brief: [`v0.3.1-grok-bot-qa-brief.md`](v0.3.1-grok-bot-qa-brief.md)
