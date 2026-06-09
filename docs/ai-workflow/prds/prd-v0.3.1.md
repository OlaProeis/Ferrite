# Ferrite v0.3.1 — Mermaid Wave 2, Embeds, Multi-Window, Data UX & Polish

> **Status:** Formal PRD — source of truth for v0.3.1 scope.
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into the custom task orchestrator (Task Master is retired — see §9).
> **Supersedes:** The Task Master-era `prd-v0.3.1.md`. Merges `pre-prd-v0.3.1.md` (updated 2026-06-09) including the **LSP deferral pivot**, product decisions, and GitHub [#142](https://github.com/OlaProeis/Ferrite/issues/142), [#144](https://github.com/OlaProeis/Ferrite/issues/144), [#145](https://github.com/OlaProeis/Ferrite/issues/145).

## 1. Overview

Ferrite is a cross-platform Rust + egui markdown editor. **v0.3.0** (May 2026) shipped the platform refresh (egui **0.34.2**, Rust **1.92** MSRV), PDF/HTML export, executable code blocks, the rendered edit session, and Mermaid first wave. **v0.3.1** is the next feature release.

**Theme (revised):** **Mermaid wave 2**, rich embeds, multi-window, data/table UX, GitHub HTML subset, and polish — **not LSP**. The LSP epic is deferred in full (§1 deferral table); v0.3.1 engineering capacity goes to Mermaid instead.

### Release pillars

1. **Mermaid second wave (headline)** — Git graph rewrite ([#83](https://github.com/OlaProeis/Ferrite/issues/83) parity), mmdr evaluation, manual `%% @pos` layout, flowchart polish (FC-83b, `linkStyle`).
2. **Embedded video rendering** — `wry` WebView with mandatory thumbnail fallback ([#119](https://github.com/OlaProeis/Ferrite/issues/119)). Parsing already landed (§4).
3. **Multi-window** — OS-level second window for side-by-side document work ([#125](https://github.com/OlaProeis/Ferrite/issues/125)).
4. **GitHub HTML parity** — Phases 1–2 only (block + inline safe subset).
5. **Data & table UX** — GFM table column alignment ([#140](https://github.com/OlaProeis/Ferrite/issues/140)), raw-mode column guides.
6. **Preview & workflow QoL** — Preview lock mode ([#144](https://github.com/OlaProeis/Ferrite/issues/144)), word-wrap toggle shortcut ([#145](https://github.com/OlaProeis/Ferrite/issues/145)), external open fallback for unsupported files ([#142](https://github.com/OlaProeis/Ferrite/issues/142)).

### Product decisions (resolved 2026-06-09)

| Question | Decision |
|----------|----------|
| **LSP in v0.3.1** | **Deferred — all phases** (v0.3.2 or later). Stays behind the `lsp` Cargo feature flag; no problems panel, no flag removal this release. Capacity goes to Mermaid. |
| Preview lock default | **Unlocked** (current editable behaviour) |
| Preview lock persistence | **Stays locked/unlocked until the user toggles** — survives view-mode switches, tab switches, and **session restore** |
| Preview lock scope | **All preview/rendered panes** — markdown WYSIWYG, CSV/TSV table, Tree viewer, any split-right viewer |
| Word wrap toolbar icon | **No** — shortcut + command palette only; toolbar icon is **Tier C, explicitly deferred** |
| Completed work (video parse, CSV editing) | **Excluded from the task list entirely** — no implementation tasks, no verification tasks |

### Deferred to a future release (do not plan in v0.3.1)

| Feature | Target | Notes |
|---------|--------|-------|
| **LSP integration (all phases)** | **v0.3.2 or later** (confirm in ROADMAP sync) | Remains behind the `lsp` feature flag; deferral noted in CHANGELOG |
| macOS Developer ID signing / notarization | Not planned (cost) | [#130](https://github.com/OlaProeis/Ferrite/issues/130) stays docs-only: [`docs/install/macos.md`](../../install/macos.md) |
| FerriteEditor crate extraction | v0.3.2 | |
| Mermaid standalone crate extraction | v0.3.2 | |
| GitHub HTML Phase 3 (nested HTML, HTML tables) | v0.3.2 | |
| Additional file-format viewers (XML, INI, log) | v0.3.2 | |
| RTL/BiDi Phases 3–4 | v0.4.0 | |
| LaTeX math rendering | v0.4.0 | |
| Office documents (DOCX/XLSX/ODT) | v0.4.0 | Until then, #142 opens them externally |

### GitHub issues mapped to this PRD

| Issue | Scope | Tier |
|-------|-------|------|
| [#106](https://github.com/OlaProeis/Ferrite/issues/106) | Wayland keyboard verification (§5.1) | A |
| [#111](https://github.com/OlaProeis/Ferrite/issues/111) | Sonoma keyboard verification (§5.1) | A |
| [#112](https://github.com/OlaProeis/Ferrite/issues/112) | Windows borderless verification (§5.1) | A |
| [#119](https://github.com/OlaProeis/Ferrite/issues/119) | Video embed rendering (§5.2) | A |
| [#83](https://github.com/OlaProeis/Ferrite/issues/83) | Mermaid git graph parity (§5.3) | A |
| [#125](https://github.com/OlaProeis/Ferrite/issues/125) | Multi-window (§5.4) | A |
| [#140](https://github.com/OlaProeis/Ferrite/issues/140) | Table column alignment (§6.2) | B |
| [#135](https://github.com/OlaProeis/Ferrite/issues/135) | File tree hover/active emphasis (§6.3) | B |
| [#144](https://github.com/OlaProeis/Ferrite/issues/144) | **Preview lock mode (§6.8)** | **B** |
| [#145](https://github.com/OlaProeis/Ferrite/issues/145) | **Word wrap toggle shortcut (§6.9)** | **B** |
| [#142](https://github.com/OlaProeis/Ferrite/issues/142) | **External file open fallback (§6.10)** | **B** |
| [#115](https://github.com/OlaProeis/Ferrite/issues/115) | Optional native title bar (§7.1) | C |

---

## 2. Goals

- **Diagrams (headline):** Git graph is visually credible (horizontal lanes); the mmdr path is evaluated with a written decision; manual layout is authorable via `%% @pos`.
- **Rich preview:** Trusted video embeds render interactively, with a safe thumbnail fallback when the native WebView path fails.
- **Productivity:** Two documents visible in two OS windows; unsupported files open in the OS default app instead of erroring (#142).
- **Publishing fidelity:** Common GitHub HTML constructs render in the preview without scripts/iframes.
- **Safety & QoL:** Users can lock any preview pane so reading never mutates files (#144); word wrap toggles instantly from the keyboard (#145).
- **Polish:** Close v0.3.0 follow-ups (cursor precision, code-run edge cases, FC-83b, table alignment #140).

## 3. Non-goals

- **No LSP work in v0.3.1** — the entire epic (transport hardening, problems panel, hover, go-to-def, completion, flag removal) is deferred to v0.3.2 or later. The `lsp` feature flag and existing feature-gated code stay as-is. CHANGELOG must note the deferral.
- No Apple code signing or notarization for CI artifacts.
- No FerriteEditor or Mermaid **crate extraction** (v0.3.2).
- No new Mermaid diagram types beyond the git-graph rewrite + polish unless the mmdr spike proves trivial (types land in v0.3.2).
- No full iframe / arbitrary HTML / script execution in rendered view.
- No replacement of single-instance with many unrelated Ferrite processes unless required by the multi-window design (prefer one process, multiple viewports).
- No re-implementation (or "verification" tasks) for already-completed work: video embed **parsing** and CSV rendered cell editing are done (§4).
- Preview lock does **not** make the Raw editor read-only — Raw and the Split raw pane are always editable.
- No word wrap toolbar/ribbon icon in Tier B (Tier C, explicitly deferred).

---

## 4. Completed work — context only, generate NO tasks

> **Rule for the orchestrator:** the following are **done in the local codebase** (not yet pushed). Do **not** generate implementation tasks or verification tasks for them.

| Former task | Feature | Status | Evidence |
|-------------|---------|--------|----------|
| Task 7 | Video embed **parsing** | **Done locally** | `src/markdown/video_embed.rs` (allowlist, `extract_video_embeds()`, unit tests); `parser.rs` `VideoProvider` / `VideoEmbedInfo` / `MarkdownNodeType::VideoEmbed`; `widgets.rs` round-trips `source_text`; docs [`video-embed-parsing.md`](../../technical/markdown/video-embed-parsing.md) |
| Task 15 | CSV rendered cell editing | **Done locally** | `csv_viewer.rs` (`CsvCellEditParams`, double-click edit, Enter/Escape, arrow navigation, `serialize_csv_rows` RFC 4180 + tests, 1 MB gate + banner); undo wired in `central_panel.rs`; docs [`csv-viewer.md`](../../technical/viewers/csv-viewer.md) § Rendered Cell Editing |

Video **rendering** (wry + thumbnail fallback) is *not* done — `markdown/editor.rs` has no `VideoEmbed` render path yet. That is Tier A work (§5.2).

Also shipped in v0.3.0 (context, no tasks): CSV cell overflow fix, rendered edit session (tasks 94–105, archived in `docs/ai-workflow/tasks/tasks-v0.3.0-archive-2026-05-31.json`).

---

## 5. Functional requirements — Tier A (must ship, release blockers)

### 5.1 Platform verification (carry-over from v0.3.0)

- Execute [`v0.3.0-regression-matrix.md`](../../technical/platform/v0.3.0-regression-matrix.md) rows still blank on target OSes, focusing on **KBD-8** (Wayland, #106) and **KBD-9** (Sonoma, #111), plus Windows borderless (#112).
- Document outcomes in CHANGELOG; close issues when confirmed fixed, or file scoped follow-ups.
- **Key files:** none (manual QA + docs).

### 5.2 Embedded video — rendering ([#119](https://github.com/OlaProeis/Ferrite/issues/119))

Parsing is **done** (§4). This is rendering only:

- **Primary path:** [`wry`](https://lib.rs/crates/wry) child WebView positioned over the embed rect each frame; hide when off-screen or tab inactive; only for `VideoEmbedInfo { trusted: true }`.
- **Fallback (required, build first):** YouTube thumbnail (`img.youtube.com/vi/<id>/...`) + play affordance → open system browser. Must work even if wry fails or is unavailable on a platform. Untrusted embeds (`trusted: false`) get text/thumbnail fallback only — never a WebView.
- **Extensibility:** keep `VideoProvider` open for future providers (Vimeo etc., v0.3.2+).
- **Security:** no arbitrary URL iframes; the allowlist in `video_embed.rs` is the single gate for the WebView path.
- **Preview lock interaction:** playing a video is a read action — allowed while locked (§6.8).
- **Key files:** `src/markdown/editor.rs` (render path for `MarkdownNodeType::VideoEmbed`), new render module (e.g. `src/markdown/video_render.rs`), `Cargo.toml` (wry), `src/app/central_panel.rs` (overlay lifecycle).
- **Docs:** new `docs/technical/markdown/video-embeds.md` (rendering, wry, fallback, security).

### 5.3 Mermaid — second wave (headline epic)

This is the headline engineering investment of v0.3.1 (it absorbs the capacity freed by the LSP deferral). Four sub-features, independent of each other except where noted. All rendering is **native egui** — Ferrite has no JS/web runtime for diagrams.

**Current state (verified in code, 2026-06-09):**

- Mermaid lives in `src/markdown/mermaid/` — flowchart is modular (`flowchart/{types,parser,layout/,render/}.rs` with a Sugiyama layered layout), other diagram types are single files (`git_graph.rs`, `sequence.rs`, `state.rs`, …).
- Parse + layout results are cached per code block via blake3 content hashing ([`mermaid-caching.md`](../../technical/mermaid/mermaid-caching.md)) — any new syntax (e.g. `@pos` hints) participates automatically because the hash covers the full source, including comment lines.
- Parse-time validation shows a warning header (line + hint), falls back to last-good render, and squiggles the raw editor ([`mermaid-inline-validation.md`](../../technical/mermaid/mermaid-inline-validation.md)). New parse errors/warnings must integrate with this pipeline, not panic.
- Rendering is panic-guarded with `catch_unwind` ([`flowchart-crash-prevention.md`](../../technical/mermaid/flowchart-crash-prevention.md)).
- The feature/status map vs Mermaid.js is [`mermaid-parity-matrix.md`](../../technical/mermaid/mermaid-parity-matrix.md); repro fixtures live in `test_md/` (FC-83a/FC-83b in `test_md/test_mermaid_issue_83.md`).

#### 5.3.1 Git graph rewrite ([#83](https://github.com/OlaProeis/Ferrite/issues/83) parity)

**Problem.** `git_graph.rs` (~290 lines) renders a **vertical list stack**: each commit gets its own row (`y = commit_index × spacing`), branches are fixed x-columns, and commit labels sit in a left gutter. Mermaid.js renders the opposite orientation by default: **time flows left→right on the x-axis and each branch is a horizontal lane (row)**, with curved merge/branch connectors between lanes. The current output is not visually credible for real-world graphs (parity matrix: "Partial — vertical list; not horizontal lane layout").

**Parser — current coverage and required additions.** The existing parser handles `commit` (with `id:"…"` and `msg:"…"` options), `branch <name>`, `checkout <name>` (implicitly creating unknown branches), and `merge <name>` (with `id:`). The rewrite must extend it with the remaining core Mermaid gitGraph grammar:

| Syntax | Current | Required |
|--------|---------|----------|
| `commit id:"…"` / `msg:"…"` | Parsed | Keep |
| `commit tag:"v1.0"` | **Ignored** | Parse; render tag label near the commit dot |
| `commit type: NORMAL\|REVERSE\|HIGHLIGHT` | **Ignored** | Parse; distinct dot rendering (e.g. cross for REVERSE, filled ring for HIGHLIGHT) |
| `branch <name> order: <n>` | `order:` ignored | Parse; controls lane ordering |
| `checkout` / `switch` | `checkout` only | Accept `switch` as alias |
| `cherry-pick id:"…"` | **Ignored (silently dropped)** | Parse; render dot + dashed connector to source commit; unknown id → validation warning |
| `gitGraph LR:` / `gitGraph TB:` header | **Ignored** | Parse; LR (default) = horizontal lanes, TB = current vertical orientation retained as the TB mode |
| Quoted branch names (`branch "feat/x"`) | Unquoted only | Strip quotes |

Unknown statements must produce a **validation warning** (via the §5.3 validation pipeline), not silent omission and not a hard parse error.

**Layout model (the core of the rewrite).**

- Assign each branch a **lane index** (row): `main` (or the first branch seen) at lane 0, then declaration order, overridden by `order:` when present.
- Assign each commit a **sequence index** (column) in declaration order — Mermaid does the same; no topological re-sort is needed.
- Commit dot position = `(margin + seq × commit_spacing, margin + lane × lane_spacing)` for LR; transpose for TB (mirrors the axis-transform approach in [`flowchart-direction.md`](../../technical/mermaid/flowchart-direction.md)).
- **Branch line:** for each branch, a horizontal polyline in the branch color from its first to last commit; the line starts with a **branch-off curve** from the parent branch's commit where the branch was created.
- **Merge connector:** curved line from the last commit of the source lane into the merge commit's dot on the target lane (the existing 3-segment bezier approximation may be reused; quality bar is "smooth, no overlap with dots").
- Keep the existing per-branch color cycling and dark/light palettes; merge-commit dots keep the distinct outlined style.
- Labels: branch name labels at the lane's left edge (LR); `id`/`msg` under or above the dot, truncated with tooltip on hover if wide; `tag:` rendered as a small rounded label offset from the dot.
- Sizing: the painter must be allocated from real computed bounds (sum of lanes / max sequence), not hardcoded constants — same lesson as FC-83a (see [`flowchart-edge-obstacle-routing.md`](../../technical/mermaid/flowchart-edge-obstacle-routing.md), "painter sized from real node bounds").

**Module shape.** Split `git_graph.rs` into a submodule if it grows past ~500 lines (`git_graph/{types,parser,layout,render}.rs`), following the flowchart modular refactor pattern ([`flowchart-modular-refactor.md`](../../technical/mermaid/flowchart-modular-refactor.md)).

**Acceptance criteria:**

1. 2–3 real-world `gitGraph` fixtures (feature-branch + merge; multi-branch with `order:`; tags + cherry-pick) added under `test_md/` and compared manually against [Mermaid Live](https://mermaid.live) — lane structure, merge topology, and labels must match (pixel-exactness not required).
2. `LR` (default) and `TB` orientations both render.
3. `tag:`, `type:`, `cherry-pick` visible per the table above; unknown statements warn, never panic.
4. Parser unit tests for every new grammar row; layout unit tests for lane assignment and merge endpoints.
5. Existing blake3 cache and `catch_unwind` guard still wrap the new render path.

**Docs:** new `docs/technical/mermaid/git-graph-layout.md` (lane model, grammar table, fixtures).

#### 5.3.2 mmdr evaluation (deliverable = decision doc + spike, NOT integration)

**Question to answer:** should Ferrite adopt [mmdr](https://github.com/1jehuang/mermaid-rs-renderer) (a Rust Mermaid parser/renderer) as a **parser frontend** for diagram types Ferrite lacks, instead of hand-writing more parsers in v0.3.2?

- **Spike:** add mmdr with `default-features = false` in a throwaway branch/worktree; feed it 3–5 diagram sources Ferrite does not support today (candidates from the parity matrix gap list: quadrant, requirement, C4, sankey, xychart). Inspect the AST it produces.
- **Evaluate and write up:** API stability/versioning, dependency weight (transitive deps, compile time, binary size), license compatibility, AST→Ferrite-layout mapping effort per diagram type, maintenance risk (bus factor, release cadence).
- **Deliverable:** `docs/technical/mermaid/mmdr-evaluation.md` with a clear **adopt / partial-adopt / reject** recommendation and a proposed v0.3.2 rollout order if positive.
- **Hard constraint:** the native egui render pipeline is **not** replaced in v0.3.1; no mmdr code ships in the release binary.

#### 5.3.3 Manual layout — `%% @pos` hints

**Goal:** let authors pin node positions when the automatic Sugiyama layout fights them, without breaking Mermaid.js compatibility — `%%` lines are comments to Mermaid, so hinted diagrams still render (auto-laid-out) everywhere else.

- **Syntax:** `%% @pos <node_id> <x> <y>` on its own line anywhere in a `flowchart`/`graph` block. `x`/`y` are layout-space coordinates (document the unit and origin in the doc deliverable). **Scope: flowcharts only in v0.3.1**; state explicitly in docs that other types ignore hints.
- **Parse:** collect hints in `flowchart/parser.rs` (which currently skips `%%` comment lines — they already reach the parser, so no pipeline change). Store as `HashMap<NodeId, Pos2>` on the parsed graph.
- **Layout:** run the normal Sugiyama pass first, then **override** positions of hinted nodes. Edges connected to hinted nodes re-anchor to the overridden rects (the obstacle-routing pass from FC-83a must consume final positions, not pre-override ones). Hinted nodes are excluded from `resolve_layer_overlaps` adjustments.
- **Validation:** unknown node id, malformed coordinates, or duplicate hints for one node → warning via the inline-validation header (line number + hint); the hint is ignored and auto-layout used for that node. Never a hard error.
- **Caching:** no special handling needed — hints are part of the source text, so the blake3 key changes when hints change.
- **Out of scope (Tier C, §7.2):** drag-to-reposition in the rendered view with `@pos` write-back to source.

**Acceptance criteria:**

1. A fixture diagram with 2+ hinted nodes renders them at hinted positions while unhinted nodes auto-layout; edges stay attached.
2. Invalid hints produce the standard warning header and do not affect other nodes.
3. The same source renders identically in Mermaid Live (hints ignored as comments) — round-trip compatibility confirmed in the fixture file.

**Docs:** new `docs/technical/mermaid/manual-layout.md`.

#### 5.3.4 Flowchart polish (FC-83b + linkStyle)

Both gaps are documented in the parity matrix and reproduce in `test_md/test_mermaid_issue_83.md`:

- **FC-83b — `fa:fa-*` icon labels:** labels like `B["fa:fa-car Car"]` currently render the literal `fa:fa-car` text (no handling anywhere in `mermaid/`). Strip the `fa:fa-<name>` / `fab:fa-<name>` prefix from the displayed label (optionally substitute a generic Phosphor placeholder icon — author's choice, document it). Applies to node labels and edge labels.
- **`linkStyle … interpolate basis`:** the `interpolate` property is currently ignored (straight segments remain). Either (a) degrade gracefully: accept and discard the property **and** document it as unsupported in the Mermaid help (F1) + parity matrix, or (b) implement a curved (bezier/catmull-rom) edge path when `interpolate basis` is present. (a) is the minimum bar; (b) is preferred if edge-routing work makes it cheap.
- **Stretch (same code area, take if trivial):** `linkStyle … stroke-dasharray` (parity matrix P2).

**Acceptance criteria:** FC-83a/FC-83b repro files render without literal `fa:` text and without parse warnings for `interpolate`; parity matrix + F1 Mermaid help updated to reflect the final supported/unsupported status.

- **Key files (whole epic):** `src/markdown/mermaid/git_graph.rs` (→ possible submodule), `flowchart/{parser,types}.rs`, `flowchart/layout/` (sugiyama, config), `flowchart/render/edges.rs`, `validation.rs`, `mermaid-parity-matrix.md`, `ui/about.rs` (F1 Mermaid help).

### 5.4 Multi-window ([#125](https://github.com/OlaProeis/Ferrite/issues/125))

- **Design doc before implementation:** `docs/technical/platform/multi-window.md` — process model (prefer one process + multiple viewports), tab ownership, single-instance interaction, session persistence shape.
- **User-visible MVP:** menu/window action **New Window**; second OS window with its own tab strip; open file in the focused window.
- **Single-instance:** second OS launch either opens a new window in the existing process or focuses an existing window and opens tabs there — document the chosen behaviour ([`single-instance.md`](../../technical/platform/single-instance.md)).
- **Not required for MVP:** Productivity Hub pop-out on second monitor (follow-up uses the same viewport APIs).
- **QA:** Windows, macOS, Linux X11 + Wayland.
- **Key files:** `src/app/` (window lifecycle), `src/state.rs` (tab ownership), `src/platform/`, `src/config/session.rs`.

### 5.5 Release docs sweep

- New/updated docs per §10, `docs/index.md` links, `ROADMAP.md` + `CHANGELOG.md` updated, deferral notes (**LSP epic**, any cut Tier B) with issue links.

---

## 6. Functional requirements — Tier B (should ship; cut only with CHANGELOG justification)

### 6.1 GitHub HTML parity — Phases 1 & 2

**Phase 1 — Block**

- `<div align="left|center|right">` … `</div>`
- `<details>` / `<summary>` (collapsible; document default open state)
- `<br>` where comrak does not already emit breaks

**Phase 2 — Inline**

- `<kbd>`, `<sup>`, `<sub>`
- `<img width="…" height="…">` (respect dimensions in rendered view)

Safe subset only: **no** `<script>`, `<style>`, `<iframe>`, event handlers. Phase 3 (nested HTML, HTML tables) is v0.3.2.

- **Key files:** `src/markdown/parser.rs` (HtmlBlock handling), `src/markdown/editor.rs` / `widgets.rs` (render).
- **Docs:** new `docs/technical/markdown/github-html-subset.md`.

### 6.2 GFM table column alignment — rendered view ([#140](https://github.com/OlaProeis/Ferrite/issues/140))

- Paint cell text left/center/right per `TableAlignment` from the parser.
- Enable alignment controls in the `EditableTable` toolbar (wire the existing `cycle_column_alignment`).
- Preserve alignment through rendered edit session commits.
- **Key files:** `src/markdown/widgets.rs` (`EditableTable`), `src/markdown/parser.rs`.

### 6.3 Workspace file tree polish ([#135](https://github.com/OlaProeis/Ferrite/issues/135))

- Hover highlight on tree rows and file-type icons.
- Distinct style for the row matching the **active tab** path.
- **Key files:** `src/ui/` file tree module.

### 6.4 Rendered click-to-edit cursor precision

- Shared layout source for paint + hit-test on formatted blocks (AST-aligned).
- Per-block `layout_wrap_width` stored for mapping parity.
- Extend RS-2 tests in the regression matrix for wrapped lines and links.
- **Key files:** `src/markdown/editor.rs`, `src/markdown/rendered_session.rs`.
- **References:** [`rendered-edit-session.md`](../../technical/markdown/rendered-edit-session.md).

### 6.5 Executable code blocks — hardening

Per [`code-block-run.md`](../../technical/markdown/code-block-run.md) § Known limitations:

- Windows `bash`/`shell` without Git Bash: clear error or correct interpreter dispatch (no bash source in a `.ps1` temp).
- `sh` / `zsh` fallback chain documented and implemented.
- Run state keyed by **content hash** or stable block id, not `start_line` alone.
- "Waiting for output…" placeholder while running with empty streams.
- Copy / insert-output prefixes stderr consistently with the on-screen panel.
- **Key files:** `src/markdown/code_execution.rs`, `src/markdown/widgets.rs`.

### 6.6 Memory & runtime — Stats panel (Phase 1, read-only)

- Stats tab section: loaded CJK/complex fonts, Mermaid cache size, terminal session count.
- **LSP row:** drop it, or show "disabled" — LSP is deferred and must not imply active integration.
- Aggregate via a `RuntimeModulesInfo` struct (in `src/state.rs` or `src/ui/`).
- Phase 2 (manual unload/clear actions) is Tier C (§7.2).

### 6.7 Raw mode table column guides (display-only)

- Detect GFM table line ranges in the raw editor viewport.
- Faint vertical guides at computed column boundaries; **no** source mutation.
- Cache per `(start_line, content_hash)`; invalidate on edit.
- **Key files:** `src/editor/ferrite/` (render layer), `src/editor/widget.rs`.
- **Docs:** new `docs/technical/editor/raw-table-alignment.md`.

### 6.8 Preview lock mode ([#144](https://github.com/OlaProeis/Ferrite/issues/144)) — NEW

**Problem:** In Rendered or Split view, clicking tables and other WYSIWYG areas can mutate file content unintentionally. Users want a view-only mode for reading/previewing.

**UX (product owner approved):**

- Phosphor **padlock** icon (locked/unlocked states), **bottom-right of every preview pane**:
  - the **Rendered** full view, and
  - the **Split** view **preview side** — the raw editor pane stays editable.
- **Locked:** read-only preview, no rendered-side mutations. **Unlocked (default):** current editable behaviour.
- Tooltip + i18n via `t!(…)` and `locales/en.yaml`. Optional subtle "Preview" hint while locked.

**Scope — "all preview panes" includes:**

- Markdown Rendered + Split preview pane (WYSIWYG)
- CSV/TSV rendered table (and split preview)
- JSON/YAML/TOML tree viewer rendered mode (and split preview)
- Any other tab in `ViewMode::Rendered` or a split-right viewer

**Behaviour matrix:**

| Interaction | Locked | Unlocked |
|-------------|--------|----------|
| Markdown WYSIWYG (headings, paragraphs, lists, formatted, tables, checkboxes, code edit) | Disabled | Enabled |
| CSV/TSV cell editing in rendered view | Disabled | Enabled |
| Tree viewer inline edit in rendered view | Disabled | Enabled |
| Link / wikilink navigation | Enabled (read action) | Enabled |
| Video embed playback (§5.2) | Enabled (read action) | Enabled |
| Scroll, zoom, copy selection | Enabled | Enabled |
| Split **raw** pane | **Always editable** | Always editable |

**State model (decided):**

- Per-tab `preview_locked: bool` on `Tab` in `src/state.rs`, **default `false`** (unlocked).
- Stays locked/unlocked **until the user toggles** — survives view-mode switches, tab switches, and **session restore** (persist in session/tab JSON, `src/config/session.rs`). Older session files without the field default to unlocked (serde default).

**Technical touchpoints:**

- `src/state.rs` — `Tab::preview_locked` + session serde.
- `src/app/central_panel.rs` — padlock overlay on each preview pane.
- `src/markdown/editor.rs` / `rendered_session.rs` — gate session activation and commits when locked (no `switch_to_ui`, no buffer commits).
- `src/markdown/widgets.rs` — `EditableTable`, task checkboxes, code block edit affordances.
- `src/markdown/csv_viewer.rs` — suppress `begin_cell_edit` while locked.
- Tree viewer module — suppress inline edit while locked.

**Acceptance criteria:**

1. Default unlocked; padlock toggles lock on **all** preview pane types.
2. Lock persists across view modes, tab switches, and app restart, until the user unlocks.
3. No preview-pane interaction mutates `tab.content` while locked — including tables, checkboxes, formatted blur commits, CSV cells, and Tree edits.
4. The Split raw pane is unaffected by the lock.
5. Manual regression: RS-1…RS-7-style flows with lock on/off.

**Docs:** new `docs/technical/markdown/preview-lock-mode.md` (or a section in the WYSIWYG doc).

### 6.9 Word wrap toggle shortcut ([#145](https://github.com/OlaProeis/Ferrite/issues/145)) — NEW

**Problem:** Word wrap is settings-only today (`Settings.word_wrap`, default `true`, `src/config/settings.rs`). Users toggle it per file type / task frequently.

**Decisions:** default binding **Alt+Z** (Windows/Linux) / **Option+Z** (macOS) — matches [VS Code's `editor.action.toggleWordWrap`](https://code.visualstudio.com/docs/reference/default-keybindings). Verified free against the full `ShortcutCommand::default_binding()` map (`Z` is only bound as Ctrl+Z Undo; Alt is used only for Alt+Space palette and Alt+Up/Down move-line). **No toolbar/ribbon icon** — shortcut + command palette only; the icon is Tier C, explicitly deferred (§7.2).

**Scope:**

1. **Shortcut** — new `ShortcutCommand::ToggleWordWrap` with `Alt+Z` default; dispatch in `src/app/keyboard.rs` `handle_keyboard_shortcuts()`; participates in conflict detection and rebinding in Settings → Keyboard.
2. **Command palette** — "Toggle Word Wrap" entry (icon in `src/app/commands.rs`, dispatch in `src/app/central_panel.rs`), same handler.
3. **Behaviour** — flip `settings.word_wrap`, persist via the existing settings save path, immediate effect in FerriteEditor and rendered/split panes (`EditorWidget` / `central_panel.rs` already receive `word_wrap`).
4. **Large-file edge case** — uniform-height mode **force-disables** wrap on 100K+ line files ([`uniform-height-large-files.md`](../../technical/editor/uniform-height-large-files.md)). The toggle must **no-op with a toast** ("Word wrap is disabled for very large files") when the active tab is in uniform-height mode; define and document whether the underlying setting still flips for other tabs.
5. **i18n** — label/tooltip/toast via `t!(…)` + `locales/en.yaml`; shortcut listed in About/Help (F1) and the Settings keyboard list.

**Acceptance criteria:**

1. Alt+Z toggles wrap on/off; the change is visible in the editor immediately.
2. The setting persists across restart.
3. No conflict with the default shortcut map; rebindable like every other command.
4. Large-file uniform-height path handled gracefully (toast, no broken state).
5. Command palette entry works and stays in sync with the setting.

**Docs:** update [`keyboard-shortcuts.md`](../../technical/ui/keyboard-shortcuts.md) and [`word-wrap.md`](../../technical/editor/word-wrap.md).

### 6.10 Open unsupported files externally ([#142](https://github.com/OlaProeis/Ferrite/issues/142)) — NEW

**Problem:** Clicking files in the workspace tree that Ferrite cannot meaningfully open (e.g. `.docx`, `.xlsx`, executables, archives) shows a blocking error dialog. Users expect the OS **default application** to open instead.

**Current behaviour (verified in code):**

- File tree click → `file_clicked` → `app/mod.rs` (~lines 1926–1940) → `open_file_smart()`.
- `state.rs::open_file_with_focus()` rejects **binary** content via `is_binary_content()` (`state.rs:4124`) with an `InvalidData` error.
- `app/mod.rs` then shows `show_error("Failed to open file: …")`.
- The `open` crate (`open::that`) is already a dependency, used for links/releases (`ui/settings.rs`, export paths) — but **not** for tree open failures.

**Desired behaviour:**

- When Ferrite **cannot open a file in-app**, delegate to **`open::that(&path)`** (system default handler).
- Show a brief **toast** (e.g. "Opened in default application") — not a blocking error dialog.
- Apply to **file tree click** at minimum; apply the same fallback policy consistently where feasible: quick switcher, search results, wikilinks to non-markdown files, drag-drop.

**Open in-app (no external delegation):**

- Markdown, JSON/YAML/TOML, CSV/TSV
- Images (viewer tab), PDF (viewer tab)
- Plain text / code files: `FileType::Unknown` that passes `is_binary_content()` → FerriteEditor (`.rs`, `.py`, `.txt`, `.html`, etc.)

**Open externally (fallback):**

- Files failing binary detection (executables, archives, Office binaries, etc.)
- Optional: explicit extension denylist for formats planned as v0.4.0 viewers (docx/xlsx) even when the heuristic is ambiguous.

**Technical touchpoints:**

- `src/state.rs` — `open_file_with_focus` or a new `OpenResult` enum (`OpenedTab` | `OpenedExternal` | `Failed`).
- `src/app/file_ops.rs` — `open_file_smart` fallback.
- `src/app/mod.rs` — file tree handler (~lines 1926–1940).
- `locales/en.yaml` — toast strings.
- Tree context menu "Open with system default" is Tier C (§7.2).

**Acceptance criteria:**

1. Click `.docx` / `.xlsx` / `.exe` in the tree → opens in the OS default app; no error dialog.
2. Click `.md` / `.json` / `.png` → still opens in Ferrite as today.
3. If `open::that` fails, show an actionable error toast (not a silent fail).
4. Manual test on Windows + one Unix OS.

**Docs:** new `docs/technical/files/external-file-open-fallback.md` (or extend [`workspace-folder-support.md`](../../technical/files/workspace-folder-support.md)).

---

## 7. Functional requirements — Tier C (optional, ship if ahead of schedule)

### 7.1 Native window decorations ([#115](https://github.com/OlaProeis/Ferrite/issues/115))

- Settings → Appearance: **Use system title bar** (default off).
- Linux/macOS: optional `with_decorations(true)`; verify resize and maximize.
- Windows: document limitations with custom chrome; may remain unsupported initially.

### 7.2 Follow-ups

- CSV: Tab / Shift+Tab between cells.
- Mermaid manual layout: drag-to-reposition with `%% @pos` write-back to source.
- Stats panel Phase 2: manual font unload, clear Mermaid cache.
- **Word wrap toolbar/ribbon icon** (#145 — explicitly deferred from Tier B).
- File tree context menu: **"Open with system default"** (#142).

### 7.3 Windows Inno Setup installer

- Optional installer alongside MSI; document in [`docs/github-release-checklist.md`](../../github-release-checklist.md).

---

## 8. Non-functional requirements

- **Performance:** no regression on v0.3.0 rendered-view budgets (AST cache, viewport culling, block height cache). Preview lock checks must be O(1) per frame (a bool gate, not new per-frame scans).
- **Security:** video embed domain allowlist; HTML safe subset only; code execution unchanged (opt-in from v0.3.0).
- **Crash safety:** wry/WebView failures degrade to the thumbnail path without panic; `open::that` failures are surfaced to the user (toast), never silent.
- **i18n:** all new user-facing strings via `t!(…)` + `locales/en.yaml` (es/de/ja/zh where feasible).
- **Input:** new shortcuts respect customization + conflict detection; new panels keyboard-reachable.
- **Session compatibility:** new session JSON fields (`preview_locked`) must deserialize old sessions via serde defaults.

---

## 9. Orchestrator parsing notes (replaces Task Master workflow)

Task Master is **retired**. Parse this PRD into the custom task + orchestrator system. **The orchestrator decides task count and granularity** — this PRD defines scope, dependencies, and quality bars, not a task quota.

- Each generated task should carry: testable acceptance criteria, key files/modules (given per section above), a complexity hint (1–10), and links to the referenced technical docs.
- **Exclude entirely:** video embed parsing and CSV rendered editing — done locally (§4), no implementation or verification tasks; and **all LSP work** — deferred epic.
- Respect tiers: Tier A blocks release; Tier B cuts require a CHANGELOG justification; Tier C is opportunistic.
- Large sections (Mermaid §5.3, multi-window §5.4, preview lock §6.8) are natural epics — split them along the sub-feature boundaries already drawn in this document rather than inventing new seams.
- v0.3.0 history: tasks 57–106 archived in `docs/ai-workflow/tasks/tasks-v0.3.0-archive-2026-05-31.json`.

**Dependency structure (hard ordering constraints):**

- Multi-window **implementation** must not start before the multi-window **design doc** is merged (§5.4).
- Video rendering depends only on parsing, which is done — it can start immediately (§5.2).
- The Mermaid sub-features (§5.3.1–§5.3.4) are mutually independent and can run in parallel.
- The docs sweep (§5.5) depends on everything that ships.
- Everything else is independent unless a section says otherwise.

**Parallelization & priority (product owner):**

- The **Mermaid epic** (§5.3) is the headline — it absorbs the capacity freed by the LSP deferral.
- The QoL trio — preview lock (#144), word wrap (#145), external open (#142) — are independent quick wins; ship early if capacity allows.

---

## 10. Documentation deliverables

| Doc | Purpose | Status |
|-----|---------|--------|
| `docs/technical/markdown/video-embed-parsing.md` | Embed syntax + AST | **Exists** (parse) |
| `docs/technical/markdown/video-embeds.md` | Rendering: wry, fallback, security | Needed (§5.2) |
| `docs/technical/markdown/preview-lock-mode.md` | Lock UX, state model, gating points | Needed (§6.8, #144) |
| `docs/technical/files/external-file-open-fallback.md` | Open-in-app vs external policy | Needed (§6.10, #142) |
| `docs/technical/platform/multi-window.md` | Architecture + single-instance interaction | Needed (§5.4) |
| `docs/technical/markdown/github-html-subset.md` | Supported tags Phases 1–2 | Needed (§6.1) |
| `docs/technical/mermaid/git-graph-layout.md` | Lane layout algorithm | Needed (§5.3) |
| `docs/technical/mermaid/mmdr-evaluation.md` | Spike outcome + recommendation | Needed (§5.3) |
| `docs/technical/mermaid/manual-layout.md` | `%% @pos` hints | Needed (§5.3) |
| `docs/technical/editor/raw-table-alignment.md` | Raw column guides | Needed (§6.7) |
| `docs/technical/ui/keyboard-shortcuts.md` | Add Alt+Z | Update (§6.9, #145) |
| `docs/technical/editor/word-wrap.md` | Toggle behaviour + large-file note | Update (§6.9, #145) |
| ~~`docs/technical/lsp/lsp-problems-panel.md`~~ | — | **Deferred with the LSP epic** |
| `docs/index.md` | Link every new doc | On each new doc |
| `ROADMAP.md`, `CHANGELOG.md` | Release notes + deferrals (LSP epic) | On release |
| `docs/install/macos.md` | Ensure no "signing in v0.3.1" language | Verify |

---

## 11. Work inventory (complexity & dependency reference — NOT a task list)

> The orchestrator owns task count and granularity (§9). This table maps each scoped feature to its tier, hard dependencies, and an overall complexity hint so the orchestrator can size and split as it sees fit. **Excluded by design:** video parsing, CSV rendered editing (done — §4), and all LSP work (deferred).

| Feature | Tier | Hard deps | Cx | PRD § |
|---------|------|-----------|----|-------|
| Platform verification gates (#106, #111, #112) | A | — | 4 | 5.1 |
| Video embed rendering — thumbnail fallback + wry overlay (#119) | A | — | 8 | 5.2 |
| Mermaid git graph rewrite (lane layout, #83) | A | — | 8 | 5.3.1 |
| Mermaid mmdr evaluation spike + decision doc | A | — | 5 | 5.3.2 |
| Mermaid manual `%% @pos` layout hints | A | — | 6 | 5.3.3 |
| Mermaid flowchart polish (FC-83b, linkStyle interpolate) | A | — | 4 | 5.3.4 |
| Multi-window design doc | A | — | 4 | 5.4 |
| Multi-window MVP implementation (#125) | A | design doc | 9 | 5.4 |
| GitHub HTML Phases 1–2 | B | — | 6 | 6.1 |
| GFM table column alignment rendered (#140) | B | — | 5 | 6.2 |
| File tree hover + active emphasis (#135) | B | — | 3 | 6.3 |
| Rendered click-to-edit cursor precision | B | — | 7 | 6.4 |
| Code block run hardening | B | — | 5 | 6.5 |
| Stats panel runtime modules (Phase 1, no LSP row) | B | — | 4 | 6.6 |
| Raw mode table column guides | B | — | 6 | 6.7 |
| **Preview lock mode (#144)** — flag, all-pane gating, padlock UI, session persistence | B | — | 6 | 6.8 |
| **Word wrap toggle (#145)** — Alt+Z, palette, large-file toast | B | — | 2 | 6.9 |
| **External file open fallback (#142)** — `OpenResult`, `open::that`, toast | B | — | 3 | 6.10 |
| v0.3.1 docs sweep + index + CHANGELOG + ROADMAP | A | all shipped work | 3 | 5.5, 10 |

Tier C items (§7) are opportunistic — pick up only if ahead of schedule.

---

## 12. Acceptance criteria (release checklist)

v0.3.1 is ready when:

1. CI green: `cargo build --release`, `cargo clippy`, `cargo test` on Windows, macOS, Linux.
2. Platform gates #106 / #111 / #112 verified or explicitly re-scoped with user-visible notes.
3. Video: trusted embeds render interactive **or** thumbnail fallback on Windows + one Unix OS; untrusted embeds never get a WebView.
4. Mermaid: git graph matches Live Editor on the §5.3.1 acceptance fixtures (lanes, merges, tags); mmdr decision doc merged with a clear recommendation; manual `@pos` renders at least one diagram correctly with Mermaid Live round-trip compatibility; FC-83a/FC-83b repros render without literal `fa:` text or `interpolate` warnings.
5. Multi-window: two windows open; different files editable; OS open-with behaviour documented.
6. Table alignment #140: GitHub three-column alignment example renders correctly.
7. **Preview lock #144:** lock prevents all preview-pane edits (markdown, CSV, Tree); persists until the user unlocks (incl. restart); Split raw pane still edits.
8. **Word wrap #145:** Alt+Z toggles with persistence; command palette in sync; no toolbar icon required; large-file edge case handled with a toast.
9. **External open #142:** unsupported tree files open in the OS default app with a toast; supported files unchanged.
10. Tier B/C deferrals listed in CHANGELOG with issue links — **including the LSP epic deferral** (target v0.3.2 or later).
11. CHANGELOG.md and ROADMAP.md updated; no doc promises macOS signing for this version.

---

*Rewritten 2026-06-09 from `pre-prd-v0.3.1.md` (updated same day: LSP deferral pivot, #142 added, completed tasks 7 & 15 excluded, word-wrap toolbar dropped to Tier C).*
