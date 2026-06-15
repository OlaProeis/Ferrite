# Mermaid Parity Matrix

Living map of Ferrite's **native** Mermaid renderer vs Mermaid.js / common real-world usage.
Use this to pick pre-0.3.0 rendering work, track GitHub issues, and avoid re-discovering gaps.

**Last updated:** 2026-06-09  
**Renderer:** `src/markdown/mermaid/` (11 diagram types, egui primitives, no Mermaid.js)  
**Manual repros:** `test_md/test_flowcharts.md`, `test_md/test_mermaid_issue_83.md`, `test_md/test_git_graphs.md`

---

## How to read status

| Status | Meaning |
|--------|---------|
| **OK** | Matches Mermaid.js for typical docs/examples |
| **Partial** | Parses/renders but visibly or structurally wrong |
| **Missing** | Not implemented; may parse-ignore or fail validation |
| **N/A** | Out of scope for native renderer (by design) |

**Priority** (for pre-0.3.0 rendering push):

| P | Meaning |
|---|---------|
| **P0** | User-visible bug / top GitHub report — fix before ship if doing Mermaid work |
| **P1** | Common syntax in READMEs & tutorials |
| **P2** | Nice parity; defer to v0.3.1+ |
| **P3** | Rare / needs mmdr or new diagram types |

---

## GitHub issues cross-reference

| Issue | Summary | Matrix area | Status in tree | Target |
|-------|---------|-------------|----------------|--------|
| [#4](https://github.com/OlaProeis/Ferrite/issues/4) | Insert toolbar + Help syntax | Authoring UX | **Shipped** (0.3.0 first wave) | Close on release |
| [#83](https://github.com/OlaProeis/Ferrite/issues/83) | Edges draw through node boxes | Flowchart → edge routing | **Open** — see P0 below | Pre-0.3.0 candidate |
| [#129](https://github.com/OlaProeis/Ferrite/issues/129) | Consecutive fenced blocks hidden | Widget layout (not routing) | **Fixed** | Close on release |

---

## Diagram type coverage

Ferrite implements **11** native types. Mermaid.js / [mmdr](https://github.com/1jehuang/mermaid-rs-renderer) cover **~23**.

| Diagram | Header keyword(s) | Ferrite module | Overall | Notes |
|---------|-------------------|----------------|---------|-------|
| Flowchart | `flowchart`, `graph` | `flowchart/` | **Partial** | Strong layout; weak edge routing (#83) |
| Sequence | `sequenceDiagram` | `sequence.rs` | **Partial** | Control blocks, activations, notes — see docs |
| State | `stateDiagram`, `stateDiagram-v2` | `state.rs` | **Partial** | Composite, fork/join, history (0.3.0) |
| Class | `classDiagram` | `class_diagram.rs` | **Partial** | Basic classes/relations; limited UML |
| ER | `erDiagram` | `er_diagram.rs` | **Partial** | Basic entities/relationships |
| Pie | `pie` | `pie.rs` | **OK** | Simple charts |
| Gantt | `gantt` | `gantt.rs` | **Partial** | Sections/tasks; no real dates/critical path |
| Git graph | `gitGraph` | `git_graph/` | **Partial** | Lane layout + render shipped (v0.3.1); see [git-graph-layout.md](./git-graph-layout.md), fixtures GG-01–GG-03 |
| Mindmap | `mindmap` | `mindmap.rs` | **Partial** | Basic tree |
| Timeline | `timeline` | `timeline.rs` | **Partial** | Basic events |
| User journey | `journey` | `journey.rs` | **Partial** | Basic sections/scores |

**Not implemented (Mermaid.js / mmdr — v0.3.1+ / mmdr eval):**  
Sankey, Kanban, Quadrant, XY chart, C4, Block, Architecture, Requirement, ZenUML, Packet, Radar, Treemap, etc.

---

## Flowchart — feature matrix

Primary focus for rendering improvements. Parser: `flowchart/parser.rs`. Layout: `flowchart/layout/`. Render: `flowchart/render/`.

### Layout & direction

| Feature | Status | Priority | Notes / repro |
|---------|--------|----------|---------------|
| Directions TD/TB/LR/RL/BT | OK | — | `test_flowcharts.md` |
| Sugiyama layered layout | OK | — | `flowchart-layout-algorithm.md` |
| Branch order (later edge → left) | OK | — | `flowchart-branch-ordering.md` |
| Same-layer sibling spacing (no overlap) | OK | — | 2026-05 — branch-snap gated to alone-on-layer + `resolve_layer_overlaps` safety net |
| Back-edge curved routing | Partial | P1 | Curves exist; still crosses nodes (#83 loops) |
| Subgraphs (flat + nested) | OK | — | `test_flowcharts.md`, `flowchart-subgraphs.md` |
| Subgraph `direction` override | OK | — | `nested-subgraph-layout.md` |
| Subgraph boundary edge routing | OK | — | `subgraph-edge-routing.md` — borders only |
| **Edge–node obstacle avoidance** | **Missing** | **P0** | **#83** — straight segments through intermediate nodes |
| Viewport clipping / negative coords | OK | — | `flowchart-viewport-clipping.md` |

### Nodes & shapes

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| Rectangle, round, stadium, diamond, hexagon | OK | — | `test_flowcharts.md` "All Node Shapes" |
| Circle, cylinder, subroutine | OK | — | |
| Asymmetric `>text]` | OK | — | `flowchart-asymmetric-shape.md` |
| Trapezoid, inv trapezoid, double circle | OK | — | 0.3.0 — `flowchart-shapes-and-style.md` |
| Parallelogram `[/text/]` | OK | — | 0.3.0 |
| HTML in labels `<br/>` | Partial | P1 | Some paths handle `<br>`; verify per shape |
| **`fa:fa-*` Font Awesome icons** | **Partial** | P1 | #83 ex. 2 — leading `fa:fa-*` / `fab:fa-*` stripped in `clean_label()`; icon glyph not rendered — `flowchart-fa-labels.md` |
| Markdown in labels | Missing | P2 | |
| `click` callbacks | N/A | P3 | No interactivity in static renderer |

### Edges & labels

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| `-->`, `---`, `-.->`, thick `==>`, bidirectional | OK | — | `ARROW_PATTERNS` in parser |
| Edge labels `\|text\|` | OK | — | |
| Chained edges `A --> B --> C` | OK | — | |
| `linkStyle N stroke, stroke-width` | OK | — | `flowchart-linkstyle.md` |
| `linkStyle default …` | OK | — | |
| **`linkStyle … interpolate basis`** | **OK** | — | #83 ex. 1 — Catmull-Rom curves on routed paths |
| `linkStyle … stroke-dasharray` | Missing | P2 | |
| Orthogonal / spline edge paths | Missing | P0–P1 | Mermaid.js default look |

### Styling

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| `classDef` / `class` | OK | — | `mermaid-classdef-styling.md` |
| `style nodeId …` | OK | — | 0.3.0 — overrides classDef |
| `color:` in classDef/style | OK | — | Label text color |
| Theme from YAML `config:` | Missing | P2 | Title works; `config.theme` parsed not applied — `test_flowcharts.md` |
| YAML frontmatter `title:` | OK | — | `mermaid-frontmatter.md` |

### Infrastructure

| Feature | Status | Notes |
|---------|--------|-------|
| Parse + layout cache (blake3) | OK | `mermaid-caching.md` |
| Inline validation + squiggles | OK | 0.3.0 — `mermaid-inline-validation.md` |
| Panic-safe render (`catch_unwind`) | OK | `flowchart-crash-prevention.md` |
| HTML export → SVG (flowchart) | OK | `export/flowchart_svg.rs` |
| Shaped text / CJK in labels | Partial | P2 — ROADMAP v0.4.0 item |

---

## Other diagram types — summary

| Type | OK / strong | Partial / gaps | Priority if touching |
|------|-------------|----------------|----------------------|
| **Sequence** | Participants, messages, loop/alt/opt/par, activate/deactivate, notes | `create`/`destroy`, fragments styling, autonumber | P2 |
| **State** | Simple + composite, fork/join, `[H]`/`[H*]` | Concurrent regions, choice pseudostate | P2 |
| **Class** | Classes, members, basic relations | Namespaces, generics, notes, packages | P2 |
| **ER** | Entities, cardinality labels | Attribute types, keys | P2 |
| **Gantt** | Sections, `after` deps, duration | Real calendar dates, milestones, excludes | P2 |
| **Git graph** | Lane layout, merge/cherry-pick connectors, tags, dot styles | TB orientation parity, reset/amend grammar | P2 |
| **Mindmap** | Basic hierarchy | Radial layout parity, icons | P3 |
| **Timeline** | Events on axis | Sections, granular formatting | P3 |
| **Journey** | Sections + scores | Task grouping | P3 |
| **Pie** | Slices + labels | `showData`, config | P3 |

---

## Repro catalog

| ID | File | What it tests |
|----|------|---------------|
| FC-01 | `test_md/test_flowcharts.md` | Layout, subgraphs, shapes, linkStyle, frontmatter |
| FC-83a | `test_md/test_mermaid_issue_83.md` | Feedback loop + `linkStyle default interpolate basis` |
| FC-83b | `test_md/test_mermaid_issue_83.md` | Shopping flowchart + `fa:fa-car` label |
| MMD-129 | `test_md/test_consecutive_code_blocks.md` | Multiple fenced blocks (incl. mermaid) visible |
| GG-01 | `test_md/test_git_graphs.md` | Feature branch + merge lane topology |
| GG-02 | `test_md/test_git_graphs.md` | Multi-branch `order:` lane assignment |
| GG-03 | `test_md/test_git_graphs.md` | Tags, cherry-pick connector, HIGHLIGHT dot |

**Visual check:** Open repro file → Rendered or Split view → compare to [Mermaid Live Editor](https://mermaid.live).

---

## Recommended pre-0.3.0 backlog (rendering only)

Ordered by impact vs effort for a focused sprint:

| # | Task | Priority | Effort | Issue / doc |
|---|------|----------|--------|-------------|
| 1 | **Edge–node collision avoidance** (flowchart forward edges) | P0 | High | #83 |
| 2 | Improve back-edge / loop routing (same repros) | P0 | Medium | #83 |
| 3 | Strip or gracefully ignore `fa:…` icon prefixes in labels | P1 | Low | #83 ex. 2 — **Done (v0.3.1)** — `flowchart-fa-labels.md` |
| 4 | `linkStyle interpolate basis` → smooth edge curves | P2 | Low | #83 ex. 1 — **Done (v0.3.1)** |
| 5 | Add `test_md/test_mermaid_issue_83.md` to manual release matrix | — | Done | This doc |

**Defer (v0.3.1 per ROADMAP):** Git Graph lane rewrite, mmdr parser eval, manual `%% @pos` layout, new diagram types, crate extraction.

---

## Authoring UX (0.3.0 first wave — complete)

Not rendering, but closes #4:

| Feature | Status | Doc |
|---------|--------|-----|
| Insert → Mermaid… templates (11 types) | OK | `mermaid-insert-toolbar.md` |
| F1 / About syntax help (same snippets) | OK | `mermaid-syntax-help.md` |
| Inline validation + last-good fallback | OK | `mermaid-inline-validation.md` |

---

## Next steps after this map

1. Visually run **FC-83a/b** on current `main` and note pass/fail per row above.
2. Implement **backlog #1** in `flowchart/render/edges.rs` (+ geometry helpers in `flowchart/utils.rs`).
3. Add unit tests for line–rect intersection against node obstacles (no egui needed).
4. Close **#4** and update **#83** when P0 routing lands.
