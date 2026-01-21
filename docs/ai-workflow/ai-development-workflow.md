# AI-Assisted Development Workflow

This document describes how Ferrite is built using AI-assisted development. The goal is transparency and to help others learn from (and improve upon) this approach.

---

## Overview

Ferrite is 100% AI-generated code. All Rust code, documentation, and configuration was written by Claude (Anthropic) via [Cursor](https://cursor.com) with MCP tools. My role as the human developer is:

- **Product direction** — Deciding what to build and why
- **Testing** — Running the app, finding bugs, verifying features work
- **Feedback** — Describing problems when things don't work
- **Orchestration** — Managing the AI workflow effectively

I'm not a programmer — I don't read code line by line. Instead, I focus on **what** should be built and **whether it works**. The AI handles the **how**.

---

## The Workflow

### Phase 1: Ideation

Before writing any code, I discuss ideas with multiple AI assistants:

| AI | Role |
|----|------|
| **Claude** | Architecture decisions, Rust-specific guidance |
| **Perplexity** | Research on libraries, best practices |
| **Gemini Pro** | Alternative perspectives, code review |

This multi-AI approach provides diverse viewpoints and catches blind spots.

### Phase 2: PRD Creation

Requirements are captured in **Product Requirements Documents (PRDs)**. These are structured documents that define:

- Problem statement
- Target users
- Feature specifications with acceptance criteria
- Technical considerations
- Implementation priority

Example PRD structure:
```markdown
# PRD: Feature Name

## Problem Statement
What problem are we solving?

## Features
### Feature 1.1 Name
- **Description:** What it does
- **Acceptance Criteria:**
  - Specific, testable requirements
  - User can do X
  - System handles Y

## Technical Requirements
- Dependencies
- Performance targets
- Backward compatibility
```

PRDs are stored in [`docs/ai-workflow/prds/`](ai-workflow/prds/).

### Phase 3: Task Generation

PRDs are parsed by [Task Master](https://github.com/task-master-ai/task-master), an AI-powered task management tool. It:

1. Reads the PRD
2. Generates structured tasks with dependencies
3. Provides complexity analysis
4. Breaks down complex tasks into subtasks

The result is a `tasks.json` file with actionable implementation steps.

Historical task files are in [`docs/ai-workflow/tasks/`](ai-workflow/tasks/).

### Phase 4: Implementation

With tasks defined, Claude (via Cursor) handles implementation:

1. **Read task** — Understand requirements from task + PRD context
2. **Explore codebase** — Find relevant files, understand patterns
3. **Implement** — Write code following project conventions
4. **Test** — Verify the implementation works
5. **Document** — Update docs if needed

**Task sizing depends on model capability:**
- **Claude Opus 4.5 (max mode)** — Can handle full tasks in one session
- **Earlier/smaller models** — Break into subtasks with handovers between each

Both approaches work. Subtasks with handovers is the proven, reliable method. Full tasks in one session is faster but requires a capable model.

### Phase 5: Session Handover

AI assistants don't have persistent memory between sessions. Each task gets a **fresh chat** with the handover pasted in at the start.

**The cycle:**
1. Start new chat → paste `current-handover-prompt.md` content
2. Work on task until done
3. Paste `update-handover-prompt.md` → AI updates handover for next task
4. Review updated handover, close chat
5. Repeat with fresh chat

**Key principle:** Handovers should be **task-focused, not project-focused**. Include only what's needed for the immediate task.

See the [Handover System](#handover-system) section below for templates and detailed workflow.

### Phase 6: Human Review

Every session ends with human review:

1. **Test the changes** — Run the app, verify the feature works as expected
2. **Check for regressions** — Make sure existing features still work
3. **Evaluate the result** — Does it match what was requested?
4. **Commit or iterate** — Accept changes or request fixes

I don't read every line of code — I'm not a programmer. Instead, I focus on **functional testing**: does it work? Does it break anything? Is this what I wanted?

The AI handles code quality (patterns, style, edge cases). I handle product quality (does this feature make sense?).

---

## Tools Used

| Tool | Purpose |
|------|---------|
| [Cursor](https://cursor.com) | AI-powered IDE with Claude integration |
| [Task Master](https://github.com/task-master-ai/task-master) | AI task management, PRD parsing |
| [Context7](https://context7.com) | MCP tool for fetching up-to-date library documentation |
| Claude (Anthropic) | Primary coding assistant |
| Perplexity | Research, documentation lookup |
| Git | Version control, history |
| AI Context File | Architectural context for writing code that fits the project |

---

## AI Context File

A lean, architectural reference file (`docs/ai-context.md`) that helps AI assistants write code that **fits the codebase**.

### The Problem

When starting a new AI session, common approaches fall short:
- **README.md** — Bloated with installation, marketing, feature lists (user-facing, not AI-facing)
- **docs/index.md** — Good for finding docs, but doesn't explain patterns
- **Full handover prompts** — Task-specific, not general project knowledge

### The Solution: Architectural Context

The AI context file is **not** about what you're working on — that comes from handovers. It's about **how to write code that fits this project**:

- Architecture (modules, key types)
- Language patterns (Rust idioms used here)
- Framework patterns (egui-specific approaches)
- Critical gotchas (things that cause bugs)
- Code conventions (naming, organization, logging)
- Navigation (where to look for common tasks)

**Key insight:** Task-specific context (current focus, recently changed) belongs in handovers, not here. This file should be useful for ANY task.

### Example: Ferrite's AI Context File

```markdown
# Ferrite - AI Context

Rust (edition 2021) + egui 0.28 markdown editor. Immediate-mode GUI, no retained widget state.

## Architecture

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `app.rs` | Update loop, event dispatch, title bar | `FerriteApp` |
| `state.rs` | All application state | `AppState`, `Tab`, `TabState` |
| `editor/widget.rs` | Text editing, line numbers | `EditorWidget` |
| `markdown/editor.rs` | WYSIWYG rendered editing | `MarkdownEditor` |
| `config/settings.rs` | Persistent settings | `Settings`, `Language` |

## Rust Patterns Used

- Error propagation: `anyhow::Result`, use `?` operator
- Option handling: `if let Some(x)`, never `.unwrap()` in library code
- Borrowing: prefer `&str` over `String`, avoid unnecessary `.clone()`
- Saturating math: `idx.saturating_sub(1)` prevents underflow

## egui Patterns

- Immediate mode: UI rebuilds every frame, state lives in AppState
- Response handling: `if ui.button("Save").clicked() { ... }`
- Repaint scheduling: `ctx.request_repaint_after(Duration::from_millis(100))`

## Critical Gotchas

| Issue | Wrong | Right |
|-------|-------|-------|
| **Byte vs char index** | `text[start..end]` with char positions | `text.char_indices()` or byte offsets |
| **Line indexing** | Mixing 0-indexed and 1-indexed | Explicit: `line.saturating_sub(1)` |
| **CPU spin** | Always `request_repaint()` | Use `request_repaint_after()` when idle |

## Code Conventions

- **Logging**: `log::info!`, `log::warn!`, `log::error!` (not println!)
- **i18n**: `t!("key.path")` for UI strings, keys in `locales/en.yaml`
- **State mutation**: Modify `TabState` for per-tab, `AppState` for global

## Where Things Live

| Want to... | Look in... |
|------------|------------|
| Add keyboard shortcut | `app.rs` → `handle_keyboard_shortcuts()` |
| Add setting | `config/settings.rs` → `Settings` struct |
| Add translation | `locales/en.yaml` + use `t!("key")` |
```

### Constraints

- **Maximum 80-100 lines** — Forces focus on essentials
- **No current focus/roadmap** — That's task-specific (handover territory)
- **No recently changed** — That's task-specific (handover territory)
- **No installation instructions** — User-facing, not AI-facing
- **No feature lists** — User-facing, not AI-facing

### Usage

**Starting a quick session:**
```
@docs/ai-context.md

[your task here]
```

**For complex tasks:** Use full handover prompt instead (or both).

### Cursor Rule Integration

A cursor rule (`.cursor/rules/ai-context.mdc`) tells the AI:
- Use this file as primary project reference when attached
- Prefer this over README.md (README is user-facing, this is AI-facing)
- Keep it under 100 lines
- Never add task-specific content

### When to Use What

**Always attach `@docs/ai-context.md`** — it provides architectural context for any task.

| Situation | Attach |
|-----------|--------|
| Quick task | `@docs/ai-context.md` |
| Standard task | `@docs/ai-context.md` + handover prompt |
| Task in unfamiliar module | `@docs/ai-context.md` + `@docs/index.md` + handover prompt |
| Bug fix with history | `@docs/ai-context.md` + bug fix handover template |

### Comparison to Handovers

| Aspect | AI Context File | Handover Prompts |
|--------|-----------------|------------------|
| **Purpose** | How to write code that fits | What to work on now |
| **Content** | Architecture, patterns, conventions | Task details, rules, key files |
| **Maintenance** | Rarely (when architecture changes) | After each task |
| **Scope** | Any task | Specific task |

**They complement each other.** AI context file = project knowledge. Handovers = task knowledge.

---

## Handover System

Handovers are the mechanism for maintaining context between AI sessions. The goal is to give the AI exactly what it needs to complete the current task — nothing more.

### Core Principle: Task-Focused, Not Project-Focused

A common mistake is including too much context:
- ❌ Full task lists and project overviews
- ❌ Previous task details (unless directly relevant)
- ❌ Next task previews (AI can fetch this when needed)
- ❌ Unrelated bugs or features

Instead, keep handovers **minimal and focused**:
- ✅ Rules for AI behavior
- ✅ Environment info (project, path, tech stack)
- ✅ Current task details
- ✅ Key files for THIS task only
- ✅ Context only if needed for this specific task

### Templates

Different situations need different handover structures:

| Template | Use When |
|----------|----------|
| [`template-handover-minimal.md`](template-handover-minimal.md) | Independent tasks (most common) |
| [`template-handover-subtask.md`](template-handover-subtask.md) | Working through subtask chains |
| [`template-handover-bugfix.md`](template-handover-bugfix.md) | Bug investigation and fixes |
| [`template-update-handover.md`](template-update-handover.md) | Instructions to update handover after task |

### The Handover Workflow

Each task gets a **fresh chat session**. The handover is pasted in at the start.

```
┌─────────────────────────────────────────────────────────────────┐
│  1. START NEW CHAT                                              │
│     - Paste current-handover-prompt.md content into chat        │
│     - AI reads task and begins work                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. WORK ON TASK                                                │
│     - AI implements the task                                    │
│     - Human reviews, tests, provides feedback                   │
│     - Iterate until task is complete                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. COMPLETE TASK                                               │
│     - Paste update-handover-prompt.md into same chat            │
│     - AI marks task done in Task Master                         │
│     - AI creates feature documentation (not task-1.md!)         │
│     - AI updates current-handover-prompt for next task          │
│     - Human reviews handover, adjusts rules if needed           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. CLOSE CHAT, START FRESH                                     │
│     - Close current chat (context is now stale)                 │
│     - Go to step 1 with new chat                                │
└─────────────────────────────────────────────────────────────────┘
```

**Why fresh chats?** AI context accumulates noise over a session. Starting fresh ensures the AI only has the clean, relevant context from the handover.

### Template Details

#### Minimal Template (Independent Tasks)

Use for most tasks. Contains only:

```markdown
# Session Handover - [Project] [Version]

## Rules
(Behavioral instructions for AI - customize per project)

## Environment
(Project path, tech stack - rarely changes)

## Current Task
(Task details from Task Master)

## Key Files
(Only files relevant to THIS task)
```

#### Subtask Template (Continuation)

Use when working through a chain of subtasks where context carries forward:

```markdown
## Last Session
- What was completed
- Important context to carry forward

## Current Task
(Current subtask details)

## Next Task Preview
(Brief look at what's coming)
```

#### Bug Fix Template

Use for bug investigation with structured tracking:

```markdown
## Bug Details
- Issue number, severity, reproducibility

## Reproduction Steps
1. Step by step...

## Previous Attempts
- What was tried, why it didn't work
```

### Customizing Rules

The "Rules" section should be customized per project. Common rules to consider:

```markdown
## Rules

- Never auto-update this file - only update when explicitly requested
- Complete entire task before requesting next instruction
- Run build/check after changes to verify code compiles
- Follow existing code patterns and conventions
- Update task status via Task Master (in-progress → done)
- Use Context7 MCP tool to fetch library documentation when needed
- Document by feature (e.g., `editor-widget.md`), not by task
- Update `docs/index.md` when adding new documentation
```

**Context7** is particularly useful when working with libraries the AI may not have current documentation for. It fetches up-to-date docs via MCP.

### When to Include "Last Session" and "Next Task"

| Workflow | Last Session | Next Task |
|----------|--------------|-----------|
| Independent tasks (no subtasks) | ❌ Remove | ❌ Remove |
| Subtask chains | ✅ Include | ✅ Include |
| Bug fixes (multiple attempts) | ✅ Include attempts | ❌ Remove |

**Why?** Independent tasks don't benefit from previous task context. The AI can always fetch the next task from Task Master when needed. Including this information adds noise without value.

### Anti-Patterns to Avoid

1. **Project overview in every handover** — The AI doesn't need to know the full v0.2.5 roadmap to implement Task 7

2. **Previous task details** — Unless the current task directly continues from it, remove it

3. **Full task lists** — Use Task Master to fetch tasks, don't embed lists in handovers

4. **Archived work references** — Irrelevant to current task

5. **GitHub issues lists** — Reference specific issues only if the current task addresses them

### Documentation Requirements

Every completed task should produce feature-based documentation:

**Naming convention:**
- ✅ Good: `editor-widget.md`, `markdown-parser.md`, `theme-system.md`
- ❌ Bad: `task-1.md`, `subtask-2-3.md`

**Where to put docs:**
- `docs/technical/` — Implementation details
- `docs/guides/` — User-facing guides (if applicable)

**Always update `docs/index.md`** — This is how the AI finds relevant documentation in future sessions. A well-maintained index saves context and improves AI accuracy.

**Template:**
```markdown
# [Feature Name]

## Overview
Brief description of what was implemented.

## Key Files
- `src/path/to/file.rs` - Description

## Implementation Details
Key technical decisions and approach.

## Usage
How to use or test this feature.
```

---

## Why This Approach?

### Pros

- **Speed** — Features that would take days can be done in hours
- **Consistency** — AI follows patterns well once established
- **Documentation** — AI writes good docs when asked
- **Learning** — I learn Rust by reviewing AI-generated code

### Cons

- **Context limits** — AI can't hold entire codebase in context
- **Subtle bugs** — AI can miss edge cases humans would catch
- **Handover friction** — Maintaining context between sessions takes effort
- **Over-engineering** — AI sometimes adds unnecessary complexity

### What I've Learned

1. **PRDs matter** — Clear requirements produce better code
2. **Match task size to model capability** — With Claude Opus 4.5 (max mode), I can do full tasks in one session. With earlier/smaller models, subtasks with handovers between them work better. The subtask approach is proven and works well — it's just slower.
3. **Test everything** — Functional testing catches what I can't see in code
4. **Multiple AIs help** — Different perspectives catch different issues
5. **Handovers are essential** — Good context documents save hours

---

## Historical Archive

All the actual documents used to build Ferrite are public:

| Folder | Contents |
|--------|----------|
| [`prds/`](prds/) | Product Requirements Documents |
| [`tasks/`](tasks/) | Task Master JSON files |
| [`handovers/`](handovers/) | Historical handover prompts |
| [`notes/`](notes/) | Feedback and review notes |

### Templates

Reusable templates for your own projects (in [`docs/handover/`](../handover/)):

| Template | Purpose |
|----------|---------|
| [`template-handover-minimal.md`](../handover/template-handover-minimal.md) | Handover for independent tasks |
| [`template-handover-subtask.md`](../handover/template-handover-subtask.md) | Handover for subtask chains |
| [`template-handover-bugfix.md`](../handover/template-handover-bugfix.md) | Handover for bug fixes |
| [`template-update-handover.md`](../handover/template-update-handover.md) | Instructions to update handover |

Current session files are in [`docs/`](../):
- [`current-handover-prompt.md`](../current-handover-prompt.md) — Active session
- [`update-handover-prompt.md`](../update-handover-prompt.md) — Update instructions

---

## FAQ

### Is this "real" programming?

That depends on your definition. I don't write code — the AI does. But I do:
- Define what needs to be built (product design)
- Test whether it works (QA)
- Describe problems for the AI to fix (debugging via feedback)
- Manage the development process (orchestration)

Is a product manager who can ship working software a "programmer"? I'll leave that to others to decide.

### Do you understand the code?

At a high level, yes. I understand the architecture, how features connect, and can follow the logic when I need to. But I don't read every line — I'm not a programmer. I rely on functional testing (does it work?) and trust the AI for code quality. When something breaks, I describe the problem and the AI debugs it.

### Could someone replicate this?

Absolutely — that's why I'm sharing these documents. The workflow is:
1. Write clear PRDs
2. Use Task Master to generate tasks
3. Use Cursor + Claude for implementation
4. Maintain handover documents
5. Review and test everything

### What about code quality?

Code quality has improved dramatically with newer models. With Claude Opus 4.5 (and likely Gemini Pro 2.5), I've had significantly fewer things break compared to earlier models.

**Current experience (Opus 4.5):**
- Follows established patterns well
- Handles edge cases better than before
- Makes reasonable architecture decisions
- Fewer bugs, less back-and-forth fixing

**Earlier models required more caution with:**
- Novel architecture decisions
- Performance-critical code
- Edge cases and error handling

The gap is closing. I trust the AI more now than I did a year ago.

### Is this sustainable long-term?

The experiment continues. So far (v0.1.0 through v0.2.5), it's working well. The codebase is maintainable, documented, and extensible.

---

## Contributing

Contributions are welcome! If you're curious about this workflow or want to suggest improvements:

- Read the [CONTRIBUTING.md](../../CONTRIBUTING.md) guide
- Open an issue to discuss ideas
- PRs are reviewed like any other code

Human-written code is just as welcome as AI-assisted code — what matters is quality and consistency.
