# Ferrite v0.3.1 — Mermaid Fidelity & Auto-Fit Addendum (PRD)

> **Status:** Formal PRD addendum — folds into the **0.3.1 branch** alongside `prd-v0.3.1.md`.
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into the custom task orchestrator (cyclopsctl).
> **Relationship:** Extends `prd-v0.3.1.md` §5.3 (Mermaid second wave). This addendum is **Mermaid Wave 2.5** — fidelity, fit-to-pane, and parser robustness — driven by post-PRD field reports [#165](https://github.com/OlaProeis/Ferrite/issues/165) and [#166](https://github.com/OlaProeis/Ferrite/issues/166).
> **Hard constraint (unchanged):** All diagram rendering stays **native egui** — no mermaid.js, no headless browser, no JS runtime in the binary.

## 1. Overview

Two issues filed 2026-06-29 against the released **0.3.0** flatpak expose a gap between Ferrite's native Mermaid engine and user expectation set by [mermaid.live](https://mermaid.live):

- **[#165](https://github.com/OlaProeis/Ferrite/issues/165)** — Mermaid diagrams (a) do **not auto-fit/scale** to the preview pane (they overflow / are clipped behind a horizontal scrollbar), and (b) **render incorrectly** for complex inputs (dense `&` fan-out, cross-subgraph `<-->` edges, CJK subgraph titles, `sequenceDiagram` with `autonumber`).
- **[#166](https://github.com/OlaProeis/Ferrite/issues/166)** — The **Outline panel does not unescape** Markdown backslash escapes, so `## 1\.3\.1 Title` shows `1\.3\.1` in the outline while the preview correctly shows `1.3.1`.

0.3.1 is **not yet tagged**, so these land on the same branch. The 0.3.1 Mermaid Wave 2 work (popup zoom/pan/fit viewer, subgraph-title width stability #159, FC-83b, `linkStyle`, `%% @pos`, git-graph rewrite) is the foundation; this addendum closes the **fidelity + ergonomics** gaps on top of it.

**Theme:** "A diagram that parses should also *fit* and *look right*." Make the inline preview usable for real-world diagrams without forcing the popup, harden the parser against valid-but-untested syntax, and stand up a repeatable fidelity baseline against mermaid.live.

### Pillars

1. **Fit-to-pane (headline, #165a)** — inline diagrams scale down to the preview width by default, with an opt-out for native-size + scroll.
2. **Layout & parser fidelity (#165b)** — dense fan-out spacing, cross-subgraph edge routing, `subgraph` headers with spaces/CJK, `sequenceDiagram` `autonumber`.
3. **Outline correctness (#166)** — backslash unescape in the outline extractor to match the rendered document.
4. **Fidelity harness & parity refresh** — a fixture suite + checklist comparing Ferrite vs mermaid.live for the reported diagrams, feeding `mermaid-parity-matrix.md`.

### Issues mapped

| Issue | Scope | Tier |
|-------|-------|------|
| [#165](https://github.com/OlaProeis/Ferrite/issues/165) (a) auto-fit | §5.1 | A |
| [#165](https://github.com/OlaProeis/Ferrite/issues/165) (b) layout/parser fidelity | §5.2, §5.3, §5.4 | A/B |
| [#166](https://github.com/OlaProeis/Ferrite/issues/166) outline unescape | §5.5 | A |
| Fidelity harness + parity refresh | §6.1 | B |

---

## 2. Goals

- A diagram wider than the preview pane **shrinks to fit by default** instead of overflowing; users can still get native size + scroll when they want it.
- Dense flowcharts (`A & B & … --> X & Y`, bidirectional cross-subgraph edges) lay out **legibly** — edges spaced, no node overlap — even if not pixel-identical to mermaid.js.
- `subgraph` headers that contain spaces or CJK keep their **full title**.
- `sequenceDiagram` renders **`autonumber`** message indices; `alt/else` and `participant … as …` already work.
- The Outline shows heading text that **matches the rendered preview** (escapes resolved).
- A repeatable **fidelity checklist + fixtures** captures the #165 diagrams so regressions are caught and the parity matrix stays honest.

## 3. Non-goals

- **No mermaid.js / no browser / no WASM diagram renderer** — the native egui pipeline stays. Pixel-exact parity with mermaid.live is explicitly *not* a goal; "same logical structure, legible, fits" is the bar.
- No new diagram **types** (quadrant, C4, sankey, etc.) — those remain the mmdr-evaluation track (`prd-v0.3.1.md` §5.3.2, target v0.3.2).
- No drag-to-reposition / `@pos` write-back (Tier C in `prd-v0.3.1.md` §7.2).
- No change to the popup viewer's interaction model (it already does fit/zoom/pan) beyond what fit-to-pane sharing requires.
- No full CommonMark inline parser rewrite for the outline — a scoped backslash-unescape only.

---

## 4. Current state (verified in code, 2026-06-30)

### 4.1 Inline rendering & sizing
- `MermaidBlock::show` (`src/markdown/widgets.rs`) renders the diagram inside `egui::ScrollArea::horizontal()` with `auto_shrink([false, true])`. There is **no scale transform** — wide diagrams produce a horizontal scrollbar (the #165a "overflow/clipped" report). Fit-to-pane exists **only** in the popup (`src/ui/mermaid_popup.rs`, `scene_fit_scale()` + `Scene`).
- `render_mermaid_diagram` (`src/markdown/mermaid/mod.rs`) dispatches by first keyword; flowcharts cache parse+layout by `CacheKey::new(source, font_size, available_width)` (blake3, width rounded to 10px). The layout is computed to `available_width` but **not** scaled to it afterward.
- `render_flowchart` (`flowchart/render/mod.rs`) allocates `alloc_size` from real content bounds (`layout_content_size` + back-edge padding) via `ui.allocate_exact_size`, then paints with `ui.painter_at(rect)`. Painting is in layout space at 1:1.

### 4.2 Flowchart parser (`flowchart/parser.rs`)
- `split_by_ampersand` correctly expands `A & B & … --> X & Y` into a full bipartite edge set (e.g. 5×2 = 10 edges from one statement). `<-->` is in `ARROW_PATTERNS` (bidirectional). So the #165 flowchart **parses**; the problem is layout density, not parsing.
- `parse_subgraph_header` **bug:** for a bare header with a space (`subgraph 业务客户端 PEP`) it splits on whitespace and takes the *first token as id, the remainder as title* → title becomes `PEP`, dropping `业务客户端`. Single-token CJK headers (`统一接入层`) are fine. Real mermaid treats the whole trailing string as the title when there is no `id [title]` / quoted form.

### 4.3 Layout engine (`flowchart/layout/`)
- Sugiyama layered layout (`sugiyama.rs`, `graph.rs`, `subgraph.rs`, `config.rs`). `FlowLayoutConfig` holds `node_spacing`, `crossing_reduction_iterations`, `margin`, subgraph paddings. Edge obstacle routing exists (see `flowchart-edge-obstacle-routing.md`). Dense fan-out and cross-subgraph edges stress crossing-reduction + spacing; current output overlaps/crosses on the #165 graph.

### 4.4 Sequence diagram (`sequence.rs`)
- Supports `participant`/`actor` (+ ` as `), `alt`/`else`/`opt`/`loop`/`par`, `activate`/`deactivate`, `Note`. **`autonumber` is not handled** (grep: zero matches) — the line is silently ignored; message numbers never render.
- Width: sequence diagrams render at native participant spacing; like flowcharts they are subject to the same inline no-scale behaviour (§4.1).

### 4.5 Text measurement (`text.rs`)
- Real rendering uses `EguiTextMeasurer` (egui galley) → CJK measures correctly when CJK fonts are loaded (Stats panel WS-2 confirms they are). `EstimatedTextMeasurer` (tests/headless) uses `text.len()` **bytes** × factor — over-measures CJK ~3× — but is **not** on the render path. CJK is a layout-fidelity question, not a glyph one.

### 4.6 Outline extractor (`src/editor/outline.rs`)
- `extract_outline` parses raw text itself (independent of the CommonMark preview parser). `parse_atx_heading` → `strip_inline_formatting` strips `**`, `*`, `` ` ``, `~~`, links, images — but has **no backslash-unescape**. The preview uses pulldown-cmark (`src/markdown/parser.rs`) which resolves `\.` → `.`, hence the mismatch (#166). Root cause is exactly as the reporter guessed.

---

## 5. Functional requirements — Tier A (must ship)

### 5.1 Inline fit-to-pane scaling ([#165](https://github.com/OlaProeis/Ferrite/issues/165) a) — headline

**Problem.** A diagram laid out wider than the preview pane overflows behind a horizontal scrollbar; users expect it to shrink to fit (mermaid.live behaviour).

**Behaviour.**
- Default **Fit width**: when the diagram's natural layout width exceeds the available pane width, render it scaled down by `scale = available_width / natural_width` (clamped to a sane floor, e.g. `MIN_FIT_SCALE = 0.25`, below which fall back to scroll so text never becomes unreadable). Never scale **up** past 1.0 inline (avoid blurry/oversized small diagrams).
- Height follows the same scale (uniform aspect); the block height shrinks accordingly so following content flows naturally (preserve the issue #129 `auto_shrink([false, true])` intent).
- **Opt-out:** a per-diagram toggle (hover affordance in the block header, next to the existing badge / popup button) to switch between **Fit width** and **Native size (scroll)**. Persisted heuristically is *not* required for v0.3.1; a per-frame default of Fit with a session-free toggle is acceptable. Document the chosen persistence (likely egui temp memory keyed by block id).
- The **popup viewer is unchanged** — it already fits/zooms; this is about the inline pane only.

**Implementation approach (author's choice, two viable paths).**
- **(Preferred) Scaled paint:** wrap the diagram paint in a transform so the whole egui sub-scene is scaled. Cleanest is to reuse `egui::Scene` (as the popup does) in a fixed, non-interactive "fit" configuration sized to the pane — gives correct hit-rects and crisp vector scaling for free. Validate that nested interactive bits (none inline today) and the click-to-open-popup gesture still work.
- **(Alternative) Layout-space downscale:** multiply node/edge geometry by `scale` at paint time in `render_flowchart` and the other renderers. More invasive (every renderer must honour a scale arg) and risks per-renderer drift; only take this if `Scene` proves unsuitable inline.

**Key files:** `src/markdown/widgets.rs` (`MermaidBlock::show` — the `ScrollArea::horizontal` block, toggle UI), `src/markdown/mermaid/mod.rs` (`render_mermaid_diagram` may need to return natural size, or expose a `measure`/`natural_size` so the caller can compute scale before painting), possibly a thin `render_mermaid_diagram_fit(ui, source, …, max_width)` wrapper. `src/ui/mermaid_popup.rs` (reuse `scene_fit_scale` helper if shared).

**Acceptance criteria.**
1. The wide #165 flowchart fixture, opened in a normal-width preview, **fits the pane** with no horizontal scrollbar at default; toggling to Native restores scroll.
2. A small diagram is **not** scaled up (renders at ≤1.0).
3. Extremely wide diagrams hit the `MIN_FIT_SCALE` floor and fall back to scroll rather than rendering illegibly.
4. Clicking the (still-fitted) diagram opens the popup as before; the badge/popup button still work.
5. Block height tracks the scaled height; following blocks are not pushed off-screen (issue #129 regression guard).

**Docs:** new `docs/technical/mermaid/inline-fit-to-pane.md`.

### 5.2 `subgraph` header with spaces / CJK ([#165](https://github.com/OlaProeis/Ferrite/issues/165) b)

**Problem.** `parse_subgraph_header` mis-splits a bare multi-word/CJK title, keeping only the trailing token.

**Required behaviour (match mermaid.js).**
- `subgraph id [Title text]` → id + title (existing, keep).
- `subgraph "Quoted title"` / `'…'` → generated id + full quoted title (existing, keep).
- `subgraph BareTitle With Spaces` (no brackets, no quotes) → the **entire trailing string is the title**; the id is generated (`subgraph_N`) **unless** the trailing string is a single token (then id = title = token, as today). This fixes `subgraph 业务客户端 PEP` → title `业务客户端 PEP`.
- Edge case: keep behaviour where a single bare token doubles as both id and title (so `A1 --> subgraphTokenId` style references still resolve).

**Key files:** `src/markdown/mermaid/flowchart/parser.rs` (`parse_subgraph_header`).

**Acceptance criteria.**
1. `subgraph 业务客户端 PEP` renders the title `业务客户端 PEP` (verified via parser unit test on `FlowSubgraph.title`).
2. `subgraph id [Title]`, quoted, and single-token forms are unchanged (existing tests still pass; add CJK + multi-word cases).
3. The full #165 flowchart's five subgraph titles all render in full.

### 5.3 `sequenceDiagram` `autonumber` ([#165](https://github.com/OlaProeis/Ferrite/issues/165) b)

**Problem.** `autonumber` is silently ignored; the reporter's diagram expects numbered messages.

**Required behaviour.**
- Parse a standalone `autonumber` line (optionally `autonumber <start>` / `autonumber <start> <step>` — mermaid supports both; minimum bar is bare `autonumber`). Set a flag on `SequenceDiagram`.
- When set, prefix each **message** (not notes/control-block headers) with its sequence number, starting at `start` (default 1), incrementing by `step` (default 1), drawn as a small badge/prefix on the message line per mermaid's style.
- Numbers reset per diagram; counting follows message order including inside `alt/else/opt/loop` segments (match mermaid: every message gets a number).

**Key files:** `src/markdown/mermaid/sequence.rs` (parser: new branch for `autonumber`; types: `autonumber: Option<AutoNumber>`; render: number prefix on messages).

**Acceptance criteria.**
1. The #165 sequence fixture renders ascending numbers on each message; `alt/else` messages are numbered in order.
2. `autonumber 10 5` (if implemented) starts at 10, steps by 5; bare `autonumber` starts at 1 step 1.
3. Diagrams without `autonumber` are visually unchanged (no numbers).
4. Parser unit tests for bare/with-args forms; a render-smoke test that the flag propagates.

### 5.4 Dense flowchart layout legibility ([#165](https://github.com/OlaProeis/Ferrite/issues/165) b)

**Problem.** Heavy `&` fan-out + bidirectional cross-subgraph edges (~36 edges in the #165 graph) overlap/cross in the current Sugiyama output.

**Scope (pragmatic, not a layout rewrite).** Target *legibility*, measured against the fixture, via tuning + targeted fixes — **not** a new layout algorithm:
- **Sibling spacing under fan-out:** ensure many same-layer targets/sources get adequate horizontal spacing (revisit `node_spacing`/overlap resolution under high fan degree; the existing layer-overlap safety net from FC-83a must scale to N-wide rows).
- **Cross-subgraph edge routing:** confirm the obstacle-routing pass (`flowchart-edge-obstacle-routing.md`, `subgraph-edge-routing.md`) handles edges that span subgraph boundaries without crossing node/subgraph rects; add bundling or lane offset for parallel `&`-expanded edges if cheap.
- **Bidirectional `<-->`:** ensure both arrowheads render and the edge isn't double-drawn when the same pair also appears reversed.

This is explicitly **best-effort within v0.3.1**; if the fixture cannot reach "legible" via tuning, the remaining delta is documented in the parity matrix as a known limitation with a v0.3.2 follow-up, rather than blocking the release.

**Key files:** `flowchart/layout/sugiyama.rs`, `graph.rs`, `subgraph.rs`, `config.rs`; `flowchart/render/edges.rs`.

**Acceptance criteria.**
1. The #165 flowchart fixture renders with **no node–node overlap** and no edge passing through a node body (overlap assertions in a layout unit test, mirroring `test_layout_coffee_machine_all_nodes`).
2. `<-->` edges show arrowheads on both ends and are not duplicated.
3. Any residual fidelity gap vs mermaid.live is recorded in `mermaid-parity-matrix.md` with a screenshot reference, not left silent.

### 5.5 Outline backslash unescape ([#166](https://github.com/OlaProeis/Ferrite/issues/166))

**Problem.** Outline shows literal `\` where the preview shows the unescaped character.

**Required behaviour.** Add a CommonMark-compatible backslash-unescape step to the outline heading-text cleanup so a backslash **immediately before ASCII punctuation** is dropped (`\.` → `.`, `\*` → `*`, `\[` → `[`, `\\` → `\`), while a backslash before a non-punctuation char is left intact (CommonMark rule). Apply it in `strip_inline_formatting` (or just before it) so it composes with the existing formatting strip.

**Key files:** `src/editor/outline.rs` (`strip_inline_formatting` / `parse_atx_heading`).

**Acceptance criteria.**
1. `## 1\.3\.1 Title` → outline item title `1.3.1 Title`.
2. `\*literal asterisks\*` in a heading → `*literal asterisks*` in the outline (not stripped as emphasis, because the escape is resolved to literal punctuation — define and test interaction order with the emphasis strip).
3. A backslash before a non-punctuation char (e.g. `C:\temp`) is preserved.
4. Existing outline tests still pass; new tests cover the escape cases above.

---

## 6. Functional requirements — Tier B (should ship)

### 6.1 Mermaid fidelity harness + parity refresh

**Goal.** Make "are we aligned with mermaid.live?" a repeatable check rather than an ad-hoc eyeball.

- **Fixtures:** add the two #165 diagrams (the layered-architecture flowchart and the auth `sequenceDiagram`) to `test_md/` — e.g. `test_md/test_mermaid_issue_165.md` — annotated with the expected structure and a link to their mermaid.live render for manual comparison.
- **Checklist:** extend the v0.3.1 manual QA (`docs/technical/platform/v0.3.1-test-checklist.md` §3 Mermaid `MMD-*`) with rows for: inline fit-to-pane (MMD-fit), subgraph-title-with-spaces, sequence `autonumber`, dense fan-out legibility, and the #165 fixtures vs Live.
- **Parity matrix:** update `docs/technical/mermaid/mermaid-parity-matrix.md` with the new supported items (`autonumber`, multi-word subgraph titles, inline fit) and any documented residual gaps from §5.4.
- **Unit tests:** parser/layout tests as specified per §5.2–§5.5 acceptance criteria.

**Key files:** `test_md/test_mermaid_issue_165.md` (new), `docs/technical/platform/v0.3.1-test-checklist.md`, `docs/technical/mermaid/mermaid-parity-matrix.md`.

### 6.2 Stretch (take only if §5 lands early)

- `autonumber off` mid-diagram (mermaid toggles numbering off) — accept and stop numbering from that point.
- Sequence diagram inline fit shares the §5.1 fit path (verify it benefits automatically once §5.1 is renderer-agnostic).
- `subgraph` direction override interplay with multi-word titles (regression check only).

---

## 7. Non-functional requirements

- **Performance:** fit-scaling must not break the blake3 parse/layout cache. The scale is a paint-time transform; layout still keys on `available_width` as today. Fit must be O(1) per frame (one division + a clamp), no extra layout pass. Avoid re-layout on every pane resize beyond the existing 10px-rounded width key.
- **Crash safety:** keep the `catch_unwind` guard around layout/paint; the fit transform must not introduce a panic path (guard against `scale == 0` / NaN when `available_width` is tiny).
- **Compatibility:** `autonumber` and multi-word subgraph titles are pure additions; diagrams that don't use them are byte-for-byte unchanged. Outline unescape changes only displayed text, not navigation offsets (verify `char_offset`/`line` are untouched).
- **i18n:** any new inline toggle label/tooltip via `t!(…)` + `locales/en.yaml`.
- **No JS/browser dependency** introduced (pillar constraint).

---

## 8. Orchestrator parsing notes

Parse this addendum into the cyclopsctl queue **alongside** `prd-v0.3.1.md`. The orchestrator owns task count/granularity.

- Each task carries: testable acceptance criteria, key files (given per §5–§6), a complexity hint (1–10), and links to the referenced `docs/technical/mermaid/*` docs.
- **Independence:** §5.1 (fit), §5.2 (subgraph header), §5.3 (autonumber), §5.5 (outline unescape) are mutually independent and can run in parallel. §5.4 (dense layout) is independent but riskier — schedule it with slack and allow a documented-gap exit. §6.1 (harness) depends on §5.x landing to fill in the parity/checklist rows.
- **Headline:** §5.1 inline fit-to-pane is the highest-leverage user-visible fix; prioritize it.
- **Quick wins:** §5.2, §5.3, §5.5 are small, contained, high-value — good early tasks.

**Dependency structure:**
- §6.1 parity/checklist updates depend on the corresponding §5.x feature being implemented.
- §5.4 layout tuning should reuse, not duplicate, the FC-83a overlap test pattern.
- Everything else is independent.

---

## 9. Documentation deliverables

| Doc | Purpose | Status |
|-----|---------|--------|
| `docs/technical/mermaid/inline-fit-to-pane.md` | Fit-vs-native model, scale math, toggle, `Scene` reuse | Needed (§5.1) |
| `docs/technical/mermaid/flowchart-subgraphs.md` | Note multi-word/CJK bare-title parsing rule | Update (§5.2) |
| `docs/technical/mermaid/sequence-control-blocks.md` | Add `autonumber` semantics | Update (§5.3) |
| `docs/technical/mermaid/mermaid-parity-matrix.md` | Add autonumber, multi-word titles, inline fit; record §5.4 residuals | Update (§5.4, §6.1) |
| `docs/technical/platform/v0.3.1-test-checklist.md` | New `MMD-*` rows + #165 fixtures vs Live | Update (§6.1) |
| `docs/technical/editor/document-outline.md` (or outline doc) | Note backslash-unescape behaviour | Update/Needed (§5.5) |
| `test_md/test_mermaid_issue_165.md` | Repro fixtures for #165 | Needed (§6.1) |
| `docs/index.md` | Link every new doc | On each new doc |
| `CHANGELOG.md` | #165 / #166 fixes + any documented residual gap | On release |

---

## 10. Work inventory (complexity & dependency reference — NOT a task list)

| Feature | Tier | Hard deps | Cx | § |
|---------|------|-----------|----|---|
| Inline fit-to-pane scaling (#165a) | A | — | 6 | 5.1 |
| `subgraph` multi-word/CJK title parse fix (#165b) | A | — | 2 | 5.2 |
| `sequenceDiagram` `autonumber` (#165b) | A | — | 4 | 5.3 |
| Dense flowchart layout legibility (#165b) | A | — | 7 | 5.4 |
| Outline backslash unescape (#166) | A | — | 2 | 5.5 |
| Fidelity harness + parity/checklist refresh | B | §5.x | 3 | 6.1 |

---

## 11. Acceptance criteria (release checklist for this addendum)

This addendum is done when:

1. CI green: `cargo build --release`, `cargo clippy`, `cargo test`.
2. **#165a:** the wide flowchart fixture fits the preview pane by default (no horizontal scrollbar); Native toggle restores scroll; small diagrams not upscaled; popup still opens.
3. **#165b parsing:** `subgraph 业务客户端 PEP` keeps its full title; `sequenceDiagram autonumber` renders message numbers.
4. **#165b layout:** the #165 flowchart fixture renders with no node overlap and no edges through node bodies; `<-->` shows both arrowheads; any residual mermaid.live delta is documented in the parity matrix.
5. **#166:** `## 1\.3\.1 Title` shows `1.3.1 Title` in the Outline, matching the preview; non-punctuation backslashes preserved.
6. Fixtures (`test_md/test_mermaid_issue_165.md`) and the `MMD-*` checklist rows are added; parity matrix updated.
7. CHANGELOG notes the #165 / #166 fixes and any documented limitation.

---

*Authored 2026-06-30 as a fidelity-focused addendum to `prd-v0.3.1.md` (Mermaid Wave 2), seeded by GitHub issues [#165](https://github.com/OlaProeis/Ferrite/issues/165) and [#166](https://github.com/OlaProeis/Ferrite/issues/166). No code changed in authoring this PRD.*
