# Pre-PRD: Ferrite v0.3.1 (draft input for formal PRD)

> **Status:** Pre-PRD — planning input only. Do **not** treat this as the shippable PRD.  
> **Purpose:** Consolidate the v0.3.1 plan for the **new task orchestrator**, incorporate product-owner decisions (2026-06-09), and give the PRD author everything needed to write `prd-v0.3.1.md`.

---

## 1. Process & migration context

### What changed
- **Task Master is retired** for active development. Ferrite uses a **custom task + orchestrator system**.
- Source references: existing [`prd-v0.3.1.md`](./prd-v0.3.1.md), [`ROADMAP.md`](../../../ROADMAP.md), archived v0.3.0 tasks in [`tasks-v0.3.0-archive-2026-05-31.json`](../tasks/tasks-v0.3.0-archive-2026-05-31.json).

### What the formal PRD must do
1. Replace Task Master §9 with **orchestrator** task guidance (dependencies, complexity, ~18–22 tasks).
2. **Defer entire LSP epic** — not in v0.3.1 scope (see §2).
3. **Do not include already-completed work** as tasks (see §3).
4. Add GitHub issues **#144**, **#145**, **#142**.
5. Preserve tier structure: **Tier A**, **Tier B**, **Tier C**.

### Suggested orchestrator task shape
Each task: acceptance criteria, key files, complexity (1–10), doc links, dependencies.

---

## 2. Release identity & strategic pivot

| Field | Value |
|-------|-------|
| **Version** | v0.3.1 |
| **Theme (revised)** | **Mermaid wave 2**, rich embeds, multi-window, data/table UX, GitHub HTML subset, polish — **not LSP** |
| **Base** | v0.3.0 (May 2026) |
| **Platforms** | Windows 10/11, macOS 12+ (unsigned), Linux X11/Wayland |

### Deferred to a future release (explicit — do not put in v0.3.1 PRD)

| Feature | Target | Notes |
|---------|--------|-------|
| **LSP integration (all phases)** | **v0.3.2 or later** | Stays behind `lsp` feature flag; no problems panel, no flag removal in v0.3.1 |
| macOS signing | Never (cost) | #130 docs only |
| FerriteEditor crate | v0.3.2 | |
| Mermaid crate extraction | v0.3.2 | |
| GitHub HTML Phase 3 | v0.3.2 | |
| XML/INI/log viewers | v0.3.2 | |
| RTL/BiDi, LaTeX math | v0.4.0 | |
| Office documents | v0.4.0 | |

**PRD author:** Remove LSP from goals, Tier A, acceptance checklist, and task list. Mention deferral in Non-goals + CHANGELOG note.

---

## 3. Completed work — exclude from PRD task list

> **Rule:** These are done in the local codebase (not yet pushed). The formal PRD and orchestrator **must not generate implementation tasks** for them.

| Former task | Feature | Status |
|-------------|---------|--------|
| **Task 7** | Video embed **parsing** (`video_embed.rs`, AST, docs) | **Done locally** — rendering still a separate future task |
| **Task 15** | CSV rendered cell editing | **Done locally** |

**PRD author:** Omit tasks 7 and 15 entirely. Video **rendering** (wry + fallback) remains a valid new task if not already implemented. Do not add “verify CSV” tasks.

---

## 4. Product decisions (resolved 2026-06-09)

### Preview lock ([#144](https://github.com/OlaProeis/Ferrite/issues/144))

| Decision | Value |
|----------|-------|
| Default state | **Unlocked** (current editable behaviour) |
| Persistence | **Stays locked/unlocked until user toggles** — survives view-mode switches, tab switches, and **session restore** |
| Scope | **All preview / rendered panes** — not markdown-only |

**“All preview panes” includes:**
- Markdown Rendered + Split **preview** pane
- CSV/TSV rendered table (and split preview)
- JSON/YAML/TOML tree viewer rendered mode (and split preview)
- Any other tab `ViewMode::Rendered` or split-right viewer

**Split raw pane:** Always editable regardless of lock.

**State model:** Per-tab `preview_locked: bool` on `Tab`, persisted in session/tab restore JSON.

### Word wrap ([#145](https://github.com/OlaProeis/Ferrite/issues/145))

| Decision | Value |
|----------|-------|
| Shortcut | **Alt+Z** / **Option+Z** ([VS Code standard](https://code.visualstudio.com/docs/reference/default-keybindings)) |
| Toolbar icon | **No** — shortcut (+ command palette) only |
| Large files | No-op + toast when uniform-height mode force-disables wrap |

### LSP

**Deferred.** v0.3.1 capacity goes to **Mermaid** instead.

---

## 5. New & updated GitHub issues

### 5.1 [#144 — Preview lock mode](https://github.com/OlaProeis/Ferrite/issues/144)

**Problem:** Accidental edits in rendered/split preview (tables, WYSIWYG blocks).

**UX:**
- Phosphor **padlock** icon, **bottom-right** of every preview pane (Rendered full view + Split preview side)
- Locked = read-only preview; Unlocked = current behaviour

| Behaviour | Locked | Unlocked |
|-----------|--------|----------|
| Markdown WYSIWYG (headings, paragraphs, lists, formatted, tables, checkboxes, code edit) | Disabled | Enabled |
| CSV/TSV cell editing in rendered view | Disabled | Enabled |
| Tree viewer inline edit in rendered view | Disabled | Enabled |
| Link / wikilink navigation | Enabled | Enabled |
| Scroll, zoom, copy | Enabled | Enabled |
| Split **raw** pane | **Always editable** | Always editable |

**Technical touchpoints:** `state.rs` (`Tab::preview_locked` + session serde), `central_panel.rs` (overlay), `markdown/editor.rs`, `rendered_session.rs`, `widgets.rs`, `csv_viewer.rs`, `tree_viewer.rs`.

**Acceptance criteria:**
1. Default unlocked; padlock toggles lock on all preview pane types.
2. Lock persists across view modes, tabs, and app restart until user unlocks.
3. No preview-pane mutation of `tab.content` while locked.
4. Split raw pane unaffected.

**Tier:** **Tier B**

**Doc:** `docs/technical/markdown/preview-lock-mode.md` (or wysiwyg section)

---

### 5.2 [#145 — Word wrap toggle shortcut](https://github.com/OlaProeis/Ferrite/issues/145)

**Scope (shortcut only — no ribbon icon):**
1. `ShortcutCommand::ToggleWordWrap` → default **Alt+Z**
2. `app/keyboard.rs` handler toggles `Settings.word_wrap`, persists, refreshes editors
3. Command palette entry
4. Conflict-safe via existing keyboard customization
5. Large-file toast when wrap unavailable

**Conflict check:** Alt+Z is **free** in current defaults (Alt+Space, Alt+Arrows used elsewhere).

**Tier:** **Tier B** — good early win

**Docs:** `keyboard-shortcuts.md`, `word-wrap.md`

---

### 5.3 [#142 — Open unsupported files externally](https://github.com/OlaProeis/Ferrite/issues/142)

**Problem:** Clicking files in the workspace tree that Ferrite cannot meaningfully open shows an error dialog. Per issue discussion, users expect the OS **default application** to open instead (e.g. `.docx`, `.xlsx`, other non-editor formats).

**Current behaviour (code):**
- `file_tree.rs` → `file_clicked` → `app/mod.rs` → `open_file_smart()`
- `state.rs::open_file_with_focus()` rejects **binary** content with `InvalidData` error
- `app/mod.rs` shows `show_error("Failed to open file: …")` on failure
- `open::that` already used elsewhere (`export.rs`, links) — **not** used for tree open failures

**Desired behaviour:**
- When Ferrite **cannot open in-app**, delegate to **`open::that(&path)`** (system default handler)
- Show brief **toast** (e.g. “Opened in default application”) — not a blocking error
- Apply to **file tree click** at minimum; consider same fallback for: quick switcher, search results, wikilink to non-markdown file, drag-drop (consistent policy)

**In-app open (no external delegation):**
- Markdown, JSON/YAML/TOML, CSV/TSV
- Images (viewer tab), PDF (viewer tab)
- Plain text / code files: `FileType::Unknown` but passes `is_binary_content()` → FerriteEditor (`.rs`, `.py`, `.txt`, `.html`, etc.)

**External open (fallback):**
- Binary detection failure (executables, archives, Office binaries, etc.)
- Optional: explicit extension denylist for formats planned for v0.4.0 viewers (docx/xlsx) even if heuristic ambiguous

**Technical touchpoints:**
- `src/state.rs` — `open_file_with_focus` or new `OpenResult` enum (`OpenedTab` | `OpenedExternal` | `Failed`)
- `src/app/file_ops.rs` — `open_file_smart` fallback
- `src/app/mod.rs` — file tree handler (lines ~1926–1940)
- `locales/en.yaml` — toast strings
- Consider context menu: **“Open with system default”** on tree (Tier C)

**Acceptance criteria:**
1. Click `.docx` / `.xlsx` / `.exe` in tree → opens in OS default app, no error dialog.
2. Click `.md` / `.json` / `.png` → still opens in Ferrite as today.
3. If `open::that` fails, show actionable error toast (not silent fail).
4. Manual test: Windows + one Unix OS.

**Tier:** **Tier B**

**Doc:** `docs/technical/files/external-file-open-fallback.md` (or extend `workspace-folder-support.md`)

---

## 6. Full feature inventory (v0.3.1 scope)

### Tier A — Must ship

| ID | Feature | GitHub | Notes |
|----|---------|--------|-------|
| A1 | Platform verification gates | #106, #111, #112 | Regression matrix; CHANGELOG |
| A2 | Video embed **rendering** | #119 | wry + mandatory thumbnail fallback; parsing excluded (done) |
| A3 | Mermaid git graph rewrite | #83 | Horizontal lane layout |
| A4 | Mermaid mmdr evaluation | — | Decision doc + spike only |
| A5 | Mermaid manual layout (`%% @pos`) | — | Parse + render |
| A6 | Mermaid flowchart polish | FC-83b, linkStyle | `fa:` labels; linkStyle basis |
| A7 | Multi-window MVP | #125 | Design doc + second OS window |

### Tier B — Should ship

| ID | Feature | GitHub |
|----|---------|--------|
| B1 | GitHub HTML Phases 1–2 | — |
| B2 | GFM table column alignment | #140 |
| B3 | File tree hover + active file | #135 |
| B4 | Rendered cursor precision follow-up | — |
| B5 | Code block run hardening | — |
| B6 | Stats panel runtime (read-only) | — | Drop LSP row or show “disabled” |
| B7 | Raw table column guides | — |
| B8 | Preview lock mode | #144 |
| B9 | Word wrap shortcut | #145 |
| B10 | External file open fallback | #142 |

### Tier C — Optional

| ID | Feature | GitHub |
|----|---------|--------|
| C1 | Native window decorations | #115 |
| C2 | CSV Tab nav; Mermaid drag layout; Stats unload | — |
| C3 | Windows Inno Setup | — |
| C4 | Word wrap toolbar icon | #145 — **explicitly deferred** |
| C5 | Tree context “Open with default app” | #142 |

---

## 7. Suggested task breakdown for orchestrator

> **Exclude:** former tasks 7 (video parse) and 15 (CSV editing). **Exclude:** all LSP tasks.

| Task | Title | Tier | Deps | Complexity |
|------|-------|------|------|------------|
| 1 | Platform verification (#106, #111, #112) | A | — | 4 |
| 2 | Video embed rendering (wry + fallback) | A | parse done | 8 |
| 3 | Mermaid git graph rewrite | A | — | 8 |
| 4 | Mermaid mmdr evaluation doc + spike | A | — | 5 |
| 5 | Mermaid manual `@pos` layout | A | — | 6 |
| 6 | Mermaid FC-83b + linkStyle polish | A | — | 4 |
| 7 | Multi-window design doc | A | — | 4 |
| 8 | Multi-window MVP | A | 7 | 9 |
| 9 | GitHub HTML Phases 1–2 | B | — | 6 |
| 10 | GFM table column alignment #140 | B | — | 5 |
| 11 | File tree polish #135 | B | — | 3 |
| 12 | Rendered cursor precision | B | — | 7 |
| 13 | Code block run hardening | B | — | 5 |
| 14 | Stats panel runtime section | B | — | 4 |
| 15 | Raw table column guides | B | — | 6 |
| 16 | Preview lock mode #144 | B | — | 6 |
| 17 | Word wrap toggle #145 | B | — | 2 |
| 18 | External file open fallback #142 | B | — | 3 |
| 19 | v0.3.1 docs + index + CHANGELOG | A | most | 3 |

**Parallelization:** Tasks 16–18 are independent quick wins. Mermaid 3–6 can parallelize after git graph design. Multi-window 8 blocks on 7.

**Priority emphasis (product owner):** Mermaid epic over LSP; ship 16–18 early if capacity allows.

---

## 8. Non-functional requirements

- No v0.3.0 rendered-view performance regression
- Video/HTML security unchanged (allowlist / safe subset)
- wry failures → thumbnail path, no panic
- `open::that` failures surfaced to user
- i18n for new strings
- Shortcut customization + conflict detection

---

## 9. Documentation deliverables

| Doc | Status |
|-----|--------|
| `video-embed-parsing.md` | Exists (parse) |
| `video-embeds.md` (rendering) | Needed |
| `preview-lock-mode.md` | Needed (#144) |
| `external-file-open-fallback.md` | Needed (#142) |
| `multi-window.md` | Needed |
| `github-html-subset.md` | Needed |
| `git-graph-layout.md`, `mmdr-evaluation.md`, `manual-layout.md` | Needed |
| `raw-table-alignment.md` | Needed |
| Update `keyboard-shortcuts.md` | Needed (#145) |
| ~~`lsp-problems-panel.md`~~ | **Deferred with LSP** |
| Update `docs/index.md`, `ROADMAP.md`, `CHANGELOG.md` | On release |

---

## 10. Release acceptance checklist (skeleton)

1. CI green: build, clippy, test
2. Platform gates #106 / #111 / #112 verified or re-scoped
3. Video: interactive or thumbnail on Windows + one Unix OS
4. Mermaid: git graph fixtures; mmdr doc; `@pos` diagram
5. Multi-window: two windows, different files
6. Table alignment #140: GitHub three-column example
7. **#144:** lock prevents all preview-pane edits; persists until unlock; raw split editable
8. **#145:** Alt+Z toggles wrap; no toolbar required
9. **#142:** unsupported tree files open in OS default app
10. Tier B/C deferrals in CHANGELOG (including **LSP deferred**)

---

## 11. GitHub issue cross-reference

| Issue | Feature | Tier |
|-------|---------|------|
| [#106](https://github.com/OlaProeis/Ferrite/issues/106) | Wayland keyboard | A |
| [#111](https://github.com/OlaProeis/Ferrite/issues/111) | Sonoma keyboard | A |
| [#112](https://github.com/OlaProeis/Ferrite/issues/112) | Windows borderless | A |
| [#119](https://github.com/OlaProeis/Ferrite/issues/119) | Video embed rendering | A |
| [#125](https://github.com/OlaProeis/Ferrite/issues/125) | Multi-window | A |
| [#135](https://github.com/OlaProeis/Ferrite/issues/135) | File tree polish | B |
| [#140](https://github.com/OlaProeis/Ferrite/issues/140) | Table alignment | B |
| [#142](https://github.com/OlaProeis/Ferrite/issues/142) | External file open | B |
| [#144](https://github.com/OlaProeis/Ferrite/issues/144) | Preview lock | B |
| [#145](https://github.com/OlaProeis/Ferrite/issues/145) | Word wrap shortcut | B |
| [#115](https://github.com/OlaProeis/Ferrite/issues/115) | Native title bar | C |

---

## 12. Instructions for the PRD author

1. Merge with existing [`prd-v0.3.1.md`](./prd-v0.3.1.md) but **apply strategic pivot:** remove LSP; emphasize Mermaid.
2. **Do not create tasks** for completed work: **task 7** (video parse), **task 15** (CSV editing).
3. Apply all **§4 product decisions** verbatim.
4. Add full specs for **#142**, **#144**, **#145**.
5. Replace Task Master §9 with orchestrator notes (~19 tasks, dependency graph).
6. Update release **theme** line — no “LSP” in headline.
7. List LSP deferral in Non-goals with target version TBD (suggest v0.3.2 in ROADMAP sync).

---

*Updated: 2026-06-09 — product decisions, LSP deferral, #142, exclude tasks 7 & 15.*
