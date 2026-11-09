# GrokBot Linux QA — Tier B issue status (ISS-12/13/14/18/24/29/54)

The autonomous Linux QA run is recorded in [`grokbot/New/qa-report-v0.3.1-linux.md`](../../../grokbot/New/qa-report-v0.3.1-linux.md) (commit `8e6ef50` on `0.3.1-experimental`). The **Issues found** table includes a **Status** column for v0.3.1 closeout: `implemented`, `does not reproduce`, `mitigated`, or `triaged`; `—` means still open on the QA commit.

## Tier B mapping (cyclopsctl tasks 22–27)

| ISS | Status (closeout) | Task | Technical detail |
|-----|-------------------|------|------------------|
| ISS-12 | closed — implemented | 22 | [`settings-esc-open-shortcut.md`](../ui/settings-esc-open-shortcut.md) |
| ISS-18 | closed — implemented | 23 | [`settings-search-initial-focus.md`](../ui/settings-search-initial-focus.md) |
| ISS-14 | closed — does not reproduce | 24 | [`settings-all-view-scroll.md`](../ui/settings-all-view-scroll.md) |
| ISS-24 | closed — mitigated | 25 | [`repaint-after-deferred-ui-updates.md`](../ui/repaint-after-deferred-ui-updates.md) |
| ISS-29 | closed — implemented | 26 | [`settings-search-emoji-font.md`](../ui/settings-search-emoji-font.md) |
| ISS-13 | triaged — not a defect | — | [`iss-13-terminal-shell-settings-triage.md`](iss-13-terminal-shell-settings-triage.md) |
| ISS-54 | closed — implemented | 27 | [`linux-custom-title-bar-iss-54.md`](linux-custom-title-bar-iss-54.md) |

## How to read the table

- **Severity** — intrinsic impact on the QA commit (blocker / major / minor / polish).
- **Status** — outcome after Tier B verification and v0.3.1 fixes; does not re-run GrokBot.
- **Actual** — preserves what GrokBot saw on `8e6ef50`, then appends closeout notes and doc links.

When adding new Tier B closes, update the same six rows (or extend Status for new ISS ids) in the QA report only; keep feature behavior documented under `docs/technical/` by area, not by task number.

## Related

- Agent brief: [`v0.3.1-grok-bot-qa-brief.md`](v0.3.1-grok-bot-qa-brief.md)
- PRD context: [`ai-workflow/prds/prd-v0.3.1-qa-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-closeout.md)
