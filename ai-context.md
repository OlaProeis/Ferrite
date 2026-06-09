# Ferrite v0.3.1 — Mermaid Wave 2, Embeds, Multi-Window, Data UX & Polish - AI Context

## Rules (DO NOT UPDATE)
- **Implementation sessions:** follow **Implementation Phase Rules** below only.
- **Update sessions:** follow **Update Phase Rules** below only when you receive the update handover prompt.
- Only do the task specified; do not start the next task or go over scope.
- Run `cargo test` after code changes to verify tests pass.
- Follow existing code patterns and conventions.
- Use Context7 MCP to fetch library documentation when needed (resolve library ID first, then fetch docs). Task operations use **`cyclopsctl tasks` CLI only**.

## Implementation Phase Rules
When working from **`current-handover-prompt.md`** (the normal case for every cyclopsctl task cycle):

- **DO:** Implement and test only the current parent task described in the handover.
- **DO:** Run `cargo test` before finishing; meet the task test strategy.
- **DO:** Use Context7 MCP for up-to-date library documentation when implementing unfamiliar APIs or frameworks.
- **DO NOT:** Read `prd.md` during cyclopsctl cycles — task scope, details, and test strategy are already in this handover.
- **DO NOT:** Mark tasks done or change task status.
- **DO NOT:** Run `cyclopsctl tasks next`, rewrite `current-handover-prompt.md`, or edit `ai-context.md`.
- **DO NOT:** Create or update docs in `docs/`, or edit `docs/index.md`.
- **DO NOT:** Edit `update-handover-prompt.md`.

Task completion and all documentation updates happen only in the **update phase** (`update-handover-prompt.md`).

## Update Phase Rules
When `update-handover-prompt.md` is provided (after implementation in the same agent session):

- **DO:** Follow every step in `update-handover-prompt.md`.
- **DO:** Use `cyclopsctl tasks list pending --project-root G:\DEV\markDownNotepad` and pick the **lowest numeric parent id** for the next handover — not `cyclopsctl tasks next` (priority can skip ahead).
- **DO:** Rewrite `current-handover-prompt.md` for the **next** task (this is the only time that file may change).
- **DO:** Update `ai-context.md` project memory per update handover step 2 (key facts only, not a changelog).
- **DO:** Use `cyclopsctl tasks` with `--project-root G:\DEV\markDownNotepad` for all task commands (see Environment in the handover).
- **DO:** Document by feature (e.g., `auth-layer.md`), not by task number; update `docs/index.md` when adding documentation.
- **DO NOT:** Re-implement or extend the task you just finished unless tests are broken.

## Conventions
- **Documentation:** Feature-based names in `docs/` (e.g., `auth-layer.md`), not `task-1.md`. Update `docs/index.md` in the update phase only.
- **Tasks:** `cyclopsctl tasks` CLI only from agents.

## Handover Files
| File | Who may edit | When |
|------|----------------|------|
| `current-handover-prompt.md` | Update-phase agent only | After implementation |
| `update-handover-prompt.md` | Human / template only | Never edited by agents |
| `ai-context.md` | Update-phase agent only | Every update phase — project memory bullets (see update handover step 2) |

## Tech Stack
Rust

## Architecture & Data Model
See `prd.md` for product architecture. This file captures agent workflow rules and where project artifacts live.

## Where Things Live
| Want to... | Look in... |
|------------|------------|
| Product requirements | `prd.md` |
| Current implementation handover | `current-handover-prompt.md` |
| Post-task update rules | `update-handover-prompt.md` |
| Documentation map | `docs/index.md` |
| Tasks and complexity | `.cyclopsctl/tasks/tasks.json`, `.cyclopsctl/reports/complexity-report.json` |
| Cyclopsctl config | `cyclopsctl.toml` |
