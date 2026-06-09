# AI Development Workflow

This folder contains documentation about Ferrite's AI-assisted development process, plus historical archives of PRDs, tasks, and handovers.

## Main Documentation

- [**ai-development-workflow.md**](ai-development-workflow.md) — Full workflow explanation, handover system, templates
- [**ai-workflow-disclosure-plan.md**](ai-workflow-disclosure-plan.md) — Planning doc for this disclosure

## Folder Structure

```
docs/ai-workflow/
├── README.md                        ← You are here
├── ai-development-workflow.md       ← Main workflow documentation
├── ai-workflow-disclosure-plan.md   ← Planning document
├── handovers/                       ← Historical session handovers
├── prds/                            ← Product Requirements Documents
├── tasks/                           ← Task Master JSON files
└── notes/                           ← Feedback and review notes
```

---

## `/handovers/` - Historical Handover Prompts

Past handover prompts from earlier development phases.

| Document | Version | Description |
|----------|---------|-------------|
| `handover-about-help.md` | v0.1.x | About/Help dialog feature |
| `handover-list-editing-bug.md` | v0.2.0 | List item editing bug fix |
| `handover-minor-bugs.md` | v0.2.x | Minor UI bug fixes |
| `code-review-handover.md` | v0.2.0 | Code review instructions |
| `code-review-v0.2.0-findings.md` | v0.2.0 | Code review findings |

---

## `/prds/` - Product Requirements Documents

PRDs define features and acceptance criteria for each version.

| Document | Version | Description |
|----------|---------|-------------|
| `prd-v0.1.x.md` | v0.1.x | Initial MVP |
| `prd-v0.2.0-list-editing-bug.md` | v0.2.0 | List editing bug fix |
| `prd-v0.2.2.md` | v0.2.2 | Stability, CLI, QoL |
| `prd-v0.2.3.md` | v0.2.3 | Editor productivity |
| `prd-v0.2.5.md` | v0.2.5 | Mermaid, CSV, i18n |
| `prd-v0.2.6.1.md` | v0.2.6.1 | Patch & stability, PR #74, code signing |
| `prd-v0.2.7.md` | v0.2.7 | Pre–v0.2.8 cycle (tasks 11–45); archived from `.taskmaster` Mar 2026 |
| `prd-v0.3.0.md` | v0.3.0 | Platform refresh, PDF/HTML export, code run, Mermaid wave 1 (egui 0.34); archived from `.taskmaster` May 2026 |
| `prd-v0.3.1.md` | v0.3.1 | LSP, video embeds, HTML Ph 1–2, Mermaid wave 2, multi-window, CSV/table UX (active PRD) |
| `prd-v0.3.0-mermaid-crate.md` | v0.3.0 | Mermaid crate extraction (deferred scope doc) |

---

## `/tasks/` - Task Master JSON Files

Task files generated from PRDs showing how requirements were broken down.

| File | Version | Description |
|------|---------|-------------|
| `tasks-v0.1.x-completed.json` | v0.1.x | MVP tasks |
| `tasks-v0.2.0-completed.json` | v0.2.0 | v0.2.0 tasks |
| `tasks-v0.2.0.json` | v0.2.0 | v0.2.0 structure |
| `tasks-v0.2.2.json` | v0.2.2 | v0.2.2 structure |
| `tasks-v0.2.3.json` | v0.2.3 | v0.2.3 structure |
| `tasks-v0.2.6.1.json` | v0.2.6.1 | v0.2.6.1 patch release tasks |
| `tasks-v0.2.7-archive-2026-03-25.json` | v0.2.7 | Full `tasks.json` snapshot (tasks 11–45) before v0.2.8 reset |
| `task-complexity-report-v0.2.7-archive-2026-03-25.json` | v0.2.7 | Complexity report snapshot for that cycle |
| `tasks-v0.3.0-archive-2026-05-31.json` | v0.3.0 | Full `tasks.json` snapshot (tasks 57–106, 50 tasks) before v0.3.1 reset |
| `task-complexity-report-v0.3.0-archive-2026-05-31.json` | v0.3.0 | Complexity report snapshot for v0.3.0 cycle |
| `tasks-v0.3.0-mermaid-crate.json` | v0.3.0 | Mermaid crate tasks |

---

## `/notes/` - Feedback and Review Notes

| Document | Description |
|----------|-------------|
| `gemini-code-review-v0.1.0.md` | Gemini code review (Dec 2025) |

---

## Current Session Files

Active handover files are in the parent `docs/` folder for easy access:
- [`current-handover-prompt.md`](../current-handover-prompt.md) — Active session context
- [`update-handover-prompt.md`](../update-handover-prompt.md) — Update instructions

## Handover Templates

Reusable templates are in [`docs/handover/`](../handover/):
- `template-handover-minimal.md` — Independent tasks
- `template-handover-subtask.md` — Subtask chains  
- `template-handover-bugfix.md` — Bug fixes
- `template-update-handover.md` — Update instructions template

---

## Learn More

- [README - AI Section](../../README.md#-ai-assisted-development) — Project AI disclosure
