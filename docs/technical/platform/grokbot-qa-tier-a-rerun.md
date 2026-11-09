# GrokBot Linux QA — Tier A re-run (ISS-2, ISS-5, ISS-6, ISS-7, ISS-9)

Ship-gate task: re-execute the Linux GUI paths for Tier A fixes after implementation tasks 1–21. The canonical prior run is [`grokbot/New/qa-report-v0.3.1-linux.md`](../../../grokbot/New/qa-report-v0.3.1-linux.md) (commit `8e6ef50`). Bucket A/B code for ISS-2 and ISS-5 landed in `5325e9b` / `97347d3`; ISS-6, ISS-7, and ISS-9 have dedicated feature docs below.

## Windows dev host (cyclopsctl agent)

Not runnable on Windows per PRD. The agent **preflight only**:

- `cargo test --bin ferrite`
- `cargo clippy --all-targets`

Full `cargo test` remains cyclopsctl verification after implementation.

## Linux host (manual / GrokBot)

Environment: Debian + **X11 + Xvfb** as documented in the QA report (Mesa llvmpipe, `run-ferrite.sh`, debug logs under `logs/`). Re-run the rows below and update the QA report **Issues** table **Status** column and the linked Tier 1 / EXTRA rows when results change.

| ISS | QA rows | Re-run focus | Feature / fix reference |
|-----|---------|--------------|-------------------------|
| ISS-2 | T2-36, VID-1 | Trusted YouTube embed in Rendered/Split — no GDK panic | Paths from `97347d3`; video embed stack |
| ISS-5 | T1-14 | Ctrl+V PNG → `![](assets/…)` + file write (not only context-menu Paste) | [`clipboard-image-paste.md`](../markdown/clipboard-image-paste.md) |
| ISS-6 | T1-19 / UND-1 | One Ctrl+Z after Split preview code-block edit | [`preview-code-block-undo.md`](../markdown/preview-code-block-undo.md), [`undo-coalescing.md`](../editor/undo-coalescing.md) |
| ISS-7 | T1-21 / MMD-12 | Char-by-char `` ```mermaid `` — no backtick auto-close corruption | [`auto-close-brackets.md`](../editor/auto-close-brackets.md) |
| ISS-9 | EXTRA-SET-9a/b | Invalid JSON / bad field: tolerant load, toasts, corrupt rename — no silent full reset on exit | [`tolerant-settings-load.md`](../config/tolerant-settings-load.md), [`config-load-toasts.md`](../config/config-load-toasts.md), [`corrupt-config-rename.md`](../config/corrupt-config-rename.md) |

Attach new `logs/` and screenshots to the grokbot bundle on failure.

## Related

- Round 2 full re-run gate (tasks 1–33 + `5eec844`): [`grokbot-qa-round2-rerun-gate.md`](grokbot-qa-round2-rerun-gate.md)
- Tier B closeout (no full GrokBot re-run): [`grokbot-qa-tier-b-status.md`](grokbot-qa-tier-b-status.md)
- Agent brief: [`v0.3.1-grok-bot-qa-brief.md`](v0.3.1-grok-bot-qa-brief.md)
- PRD: [`ai-workflow/prds/prd-v0.3.1-qa-closeout.md`](../../ai-workflow/prds/prd-v0.3.1-qa-closeout.md)
