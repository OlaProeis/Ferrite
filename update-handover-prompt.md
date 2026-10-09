# Update Handover Instructions

The **implementation work for this cycle is finished**. Do not write more feature code. **Do not run any tests or builds in this phase** — the full `cargo test` is cyclopsctl's job (it already ran after implementation); the update phase is bookkeeping only.

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

Create feature-based documentation for what was implemented, then append durable facts. Facts live in `.cyclopsctl/memory/facts.jsonl` (via the CLI), not in `ai-context.md`.

1. Group by **feature**, not task number.
2. Add a doc under `docs/` or `docs/technical/`.
3. **Update `docs/index.md`** with the new entry and a one-line description.
4. **Append facts** with `cyclopsctl memory add` — whole-project memory for **all phases/tags**, not a changelog:
   - Add **at most a few** facts for durable details from this task (new modules, patterns, gotchas, how things connect).
   - Point to `docs/` for long detail — do not copy full doc text into a fact.
   - Use `--tag always` only for facts every later task needs.
   - If nothing new is worth remembering, add no fact.
   - **Never edit:** Rules (DO NOT UPDATE), Implementation Phase Rules, or Update Phase Rules.

```bash
cyclopsctl memory add --text "<durable fact>" --tag <area> --task-id <current-task-id> --project-root G:\DEV\markDownNotepad
```

**Naming:** Good: `feature-name.md`. Bad: `task-1.md`.

---

## 3. Leave the handover to cyclopsctl

Do not rewrite `current-handover-prompt.md` and do not pick the next task id. After this phase, cyclopsctl writes the next handover from `cyclopsctl tasks next` (the lowest pending task whose dependencies are done). An empty queue becomes `# Task ID: 0`.

---

## 4. Final checks

- [ ] `cyclopsctl tasks set-status` succeeded
- [ ] `docs/index.md` updated
- [ ] New facts, if any, were appended with `cyclopsctl memory add`
- [ ] `current-handover-prompt.md` was not rewritten
- [ ] `cargo test` (or any other test/build command) was **not** run in this phase
