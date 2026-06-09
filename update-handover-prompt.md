# Update Handover Instructions

The **implementation work for this cycle is finished**. Do not write more feature code unless fixing a failing test you just ran.

This prompt runs only in the **update phase** (same agent session after implementation). Follow these steps in order.

**Project root for all `cyclopsctl tasks` commands:** `G:\DEV\markDownNotepad` (`--project-root G:\DEV\markDownNotepad`).

---

## 1. Mark current task done

Read `# Task ID:` from `current-handover-prompt.md` (parent task id only).

```bash
cyclopsctl tasks set-status --id=<current-task-id> --status=done --project-root G:\DEV\markDownNotepad
```

Confirm success before continuing.

---

## 2. Documentation and project memory

Create feature-based documentation for what was implemented, then update project memory in `ai-context.md`.

1. Group by **feature**, not task number.
2. Add a doc under `docs/` or `docs/technical/`.
3. **Update `docs/index.md`** with the new entry and a one-line description.
4. **Update `ai-context.md`** — key facts the next implementation agent must know (not a changelog):
   - **Editable sections:** Architecture & Data Model, Conventions, Where Things Live; add `## Project Memory` if missing.
   - **Never edit:** Rules (DO NOT UPDATE), Implementation Phase Rules, Update Phase Rules, or Handover Files rules.
   - Add **1–3 bullets max** for durable facts from this task (new modules, patterns, gotchas, how things connect).
   - Merge or prune duplicates; point to `docs/` for long detail — do not copy full doc text here.
   - If nothing new worth remembering: add one line under Project Memory, e.g. `Task <id>: no new project memory — <brief reason>`.

**Naming:** Good: `feature-name.md`. Bad: `task-1.md`.

---

## 3. Get next task

Use the **lowest numeric pending parent task id** — not `cyclopsctl tasks next` priority ordering.

```bash
cyclopsctl tasks list pending --format json --project-root G:\DEV\markDownNotepad
```

From the pending list, pick the task with the **smallest numeric id**. Load full details:

```bash
cyclopsctl tasks show <next-task-id> --format json --project-root G:\DEV\markDownNotepad
```

If there is no next task, set `# Task ID: 0` in `current-handover-prompt.md` and stop.

---

## 4. Rewrite `current-handover-prompt.md`

This is the **only** step that may edit `current-handover-prompt.md`. Preserve the standard section order from `prompts/sync-current-handover.md`.

Include:
- `# Task ID: <next-id>`
- `## Environment` with project name, root, tech stack, branch
- `## Core Handover Rules`, `## Implementation Phase — Do Only This`
- `## Current Task: <id> — <title>` with Description, Implementation Details, Test Strategy
- `## Verification` with `cargo test`
- `## Model Selection` from complexity report when available

---

## 5. Final checks

- [ ] `cyclopsctl tasks set-status` succeeded
- [ ] `docs/index.md` updated
- [ ] `ai-context.md` updated with project memory (or explicit “no new memory” line with reason)
- [ ] `current-handover-prompt.md` rewritten with a new `# Task ID:`
- [ ] `cargo test` passes
