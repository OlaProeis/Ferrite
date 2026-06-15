# mmdr (`mermaid-rs-renderer`) evaluation

**Date:** 2026-06-09  
**Task:** v0.3.1 spike — parser frontend evaluation for diagram types Ferrite lacks  
**Crate:** [mermaid-rs-renderer](https://crates.io/crates/mermaid-rs-renderer) v0.2.2 (CLI binary: `mmdr`)  
**Repository:** [1jehuang/mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer)

## Executive summary

**Recommendation: partial-adopt**

Adopt **mmdr as a parser frontend only** for diagram types Ferrite does not implement natively today. Keep Ferrite's existing native parsers and egui render pipeline for the 11 shipped types. Do **not** ship mmdr's SVG renderer or layout in the editor UI.

Rationale: `parse_mermaid()` produces a usable intermediate representation (IR) for the five gap types tested here, with rich dedicated structs for quadrant, XY chart, and C4. Requirement and Sankey parse into generic `nodes`/`edges` (structured attrs flattened into labels or edge weights). Dependency weight is modest when `default-features = false` (~12 new transitive crates on Ferrite's graph, ~380 KB release binary delta on Windows). The project is young (0.2.x, first release Jan 2026) with a single primary maintainer — acceptable for a parser-only integration behind a thin adapter, but not as a full renderer replacement.

**Hard constraint met:** spike ran on a throwaway worktree; `master` / `0.3.1-experimental` `Cargo.toml` and `Cargo.lock` contain **no** `mermaid-rs-renderer` dependency.

---

## Evaluation scope

| Gap type (parity matrix) | Mermaid header | Ferrite today |
|--------------------------|----------------|---------------|
| Quadrant | `quadrantChart` | Missing |
| Requirement | `requirementDiagram` | Missing |
| C4 | `C4Context` (and siblings) | Missing |
| Sankey | `sankey-beta` | Missing |
| XY chart | `xychart-beta` | Missing |

Spike method: standalone crate `tools/mmdr-spike` with `mermaid-rs-renderer = { version = "0.2.2", default-features = false }`, plus a separate Ferrite link-size measurement with `parse_mermaid` force-linked in `main.rs` (not merged).

---

## API surface and stability

### Pipeline stages (exposed at crate root)

```rust
use mermaid_rs_renderer::{parse_mermaid, compute_layout, render_svg, Theme, LayoutConfig};

let parsed = parse_mermaid(source)?;
let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());
```

Ferrite would use **stage 1 only** (`parse_mermaid` → `Graph`), then map `Graph` into Ferrite layout structs and paint with egui.

### Core types

| Type | Role |
|------|------|
| `ParseOutput` | `{ graph: Graph, init_config: … }` |
| `Graph` | Unified IR — wide struct with `kind: DiagramKind` plus type-specific fields |
| `DiagramKind` | 23 variants (`Flowchart`, `Quadrant`, `Sankey`, `C4`, `Requirement`, `XYChart`, …) |
| `Node`, `Edge`, `Subgraph` | Shared flowchart-style graph (also used by Requirement, Sankey) |
| `QuadrantData`, `XYChartData`, `C4Data`, … | Dedicated payloads on `Graph` |

### Versioning and churn risk

| Signal | Finding |
|--------|---------|
| Current version | 0.2.2 (2026-04-24) |
| Release cadence | 6 releases since 2026-01-23 (~one per month) |
| API stability | Pre-1.0; `Graph` field set expanded between 0.1 → 0.2 |
| MSRV / edition | Rust edition **2024** in crate metadata; builds under Ferrite's 1.92 toolchain |
| Maintainers | ~3 contributors; primary author drives most commits |
| Community | ~1.1k GitHub stars, ~40 open issues — active but bus-factor concern |
| Reverse deps | ~19 crates.io dependents (early adoption) |

**Risk:** parser IR fields may change on minor releases. Mitigation: pin exact version, thin `mmdr → Ferrite` adapter module, integration tests on fixture diagrams.

### License

MIT — compatible with Ferrite (MIT).

---

## Dependency weight

Measured on Windows, `cargo build --release --no-default-features --features bundle-icon`, cold build at spike commit `3ba085c`.

### Standalone spike crate (`mmdr-spike` only)

| Metric | Value |
|--------|-------|
| Transitive deps (`default-features = false`) | **35** direct+transitive packages |
| Release binary | **5.05 MB** |
| First compile (cold) | ~56 s |

mmdr README claims ~80 crates with `default-features = false`; our spike resolved **35** (overlap with Ferrite's existing `regex`, `serde`, `anyhow`, etc. reduces the delta when embedded).

### Ferrite + mmdr (force-linked `parse_mermaid`)

| Metric | Without mmdr | With mmdr 0.2.2 | Delta |
|--------|--------------|-----------------|-------|
| Transitive deps | 875 | 887 | **+12** |
| Release binary | 30.26 MB | 30.65 MB | **+382 KB (+1.3%)** |
| Cold compile | ~197 s | ~200 s | **+3 s** |

Current `0.3.1-experimental` branch (additional deps e.g. `wry`) reports ~946 transitive packages and ~30.0 MB release binary without mmdr; expect a similar **~1–2%** binary delta when mmdr is added there.

**Features:** `default-features = false` disables `cli` (clap) and `png` (resvg/usvg). Ferrite does not need either for parse-only use.

---

## Per-diagram AST findings

All five gap types **parsed successfully** and produced layout + SVG in the spike.

### `quadrantChart`

| IR path | Content |
|---------|---------|
| `graph.kind` | `DiagramKind::Quadrant` |
| `graph.quadrant` | `QuadrantData`: `title`, `x_axis_left/right`, `y_axis_bottom/top`, `quadrant_labels[4]`, `points: Vec<QuadrantPoint { label, x, y }>` |
| `graph.nodes` | Also contains synthetic nodes (`quadrant_0`, …) mirroring point labels |

**Ferrite mapping:** Medium. Map `QuadrantData` → axis labels, quadrant text, scatter points (normalized 0–1 coords). Ignore or dedupe synthetic `nodes`. egui: `Rect` grid + `circle`/`text` for points.

### `requirementDiagram`

| IR path | Content |
|---------|---------|
| `graph.kind` | `DiagramKind::Requirement` |
| `graph.nodes` | Requirements/elements as rectangles; `label` embeds kind + attrs (`<<Requirement>>`, ID, Text, Risk, Verification) |
| `graph.edges` | Relations with `label` = relation type (`satisfies`, `traces`, …) |

No dedicated `RequirementData` struct — attrs from `{ … }` blocks are normalized into node label strings at parse time.

**Ferrite mapping:** Medium–high. Box layout similar to flowchart; relation styling per edge label. Loses structured attr access unless Ferrite re-parses labels or extends adapter to read mmdr internals.

### `C4Context`

| IR path | Content |
|---------|---------|
| `graph.kind` | `DiagramKind::C4` |
| `graph.c4` | `C4Data`: `shapes[]` (Person/System/…), `boundaries[]`, `rels[]`, `c4_type` |
| `graph.nodes` | **Empty** — C4 does not populate generic node map |

`C4Shape`: `id`, `label`, `type_label`, `kind` (Person/System/…), `parent_boundary`, optional colors. `C4Rel`: `from`, `to`, `label`, offsets, colors.

**Ferrite mapping:** High. Custom person/system/container glyphs, nested boundaries, relationship routing. Rich IR helps, but egui paint is essentially a new diagram renderer.

### `sankey-beta`

| IR path | Content |
|---------|---------|
| `graph.kind` | `DiagramKind::Sankey` |
| `graph.nodes` | Node per source/target name |
| `graph.edges` | Links; `label` holds numeric **value** |

CSV header row (`Source,Target,Value`) is incorrectly parsed as an extra link in v0.2.2 spike — filter header lines in adapter.

No dedicated `SankeyData` on `Graph`; layout lives in mmdr's `layout/sankey.rs` only.

**Ferrite mapping:** High. Need Sankey-specific layout (node columns, link ribbons). IR gives graph topology + weights only.

### `xychart-beta`

| IR path | Content |
|---------|---------|
| `graph.kind` | `DiagramKind::XYChart` |
| `graph.xychart` | `XYChartData`: `title`, `x_axis_categories`, `y_axis_label`, `y_axis_min/max`, `series: Vec<XYSeries { kind: Bar|Line, values }>` |
| `graph.nodes` | Empty |

**Ferrite mapping:** Medium. Map to axis + bar/line series; standard chart paint in egui (`Rect` bars, line strips).

---

## IR design assessment (parser frontend fit)

**Strengths**

- Single entry point (`parse_mermaid`) for 23 diagram kinds.
- Dedicated structs for several “new” types (quadrant, xychart, c4, gantt, git graph, mindmap, …).
- `DiagramKind` discriminant aligns with Ferrite's `mermaid/mod.rs` routing.

**Weaknesses**

- `Graph` is a kitchen-sink struct (~30+ fields); easy to misuse wrong field for a kind.
- Requirement, Sankey (and partially Quadrant) duplicate or flatten into `nodes`/`edges`.
- No stable per-type AST outside `Graph`; no serde on IR types in public API.
- Tight coupling to mmdr's layout assumptions — Ferrite cannot reuse mmdr layout without adopting their SVG path.

**Conclusion for Ferrite:** suitable as **external parser** behind `fn parse_via_mmdr(kind, src) -> FerriteDiagram` with explicit per-kind mappers; not a replacement for Ferrite's flowchart/sequence parsers without regression risk.

---

## Proposed v0.3.2 rollout (if partial-adopt approved)

Order by **IR richness + render effort + user demand**:

| Phase | Types | Effort | Notes |
|-------|-------|--------|-------|
| **1** | `xychart-beta`, `quadrantChart` | M | Structured `XYChartData` / `QuadrantData`; no boundary nesting |
| **2** | `sankey-beta` | L | Topology from nodes/edges; custom Sankey layout |
| **3** | `requirementDiagram` | M–L | Flowchart-like; attrs in labels today |
| **4** | `C4Context` / `C4Container` | L | `C4Data` rich but heavy egui shapes |
| **5** | Kanban, Block, Architecture, Radar, Treemap, … | L | Parser available; render from scratch |

**Integration sketch (no code in v0.3.1)**

1. Add `mermaid-rs-renderer` with `default-features = false`, pinned version.
2. `src/markdown/mermaid/mmdr_adapter.rs` — `parse_mmdr(source) -> Result<MmdrGraphView, MermaidError>`.
3. In `mermaid/mod.rs`, route unknown / gap kinds through adapter; existing 11 kinds stay on native parsers.
4. Fixture tests: copy minimal diagrams from this doc + `test_md/` per type.
5. Optional: HTML export could call mmdr `render_svg` for gap types only (export path separate from egui).

---

## Alternatives considered

| Option | Verdict |
|--------|---------|
| **Full adopt** (mmdr parse + layout + SVG in UI) | Reject — bypasses egui pipeline, inconsistent with native theme/zoom/CJK, doubles maintenance for 11 existing types |
| **Reject** (hand-write all parsers) | Reject — 12+ diagram types at high cost; mmdr parser saves months on syntax edge cases |
| **Partial adopt** (parser only) | **Accept** — best match for Ferrite architecture |

---

## Spike artifacts

| Artifact | Location | Merged? |
|----------|----------|---------|
| Spike crate | `tools/mmdr-spike/` on throwaway branch | No |
| Worktree | `markDownNotepad-mmdr-spike` | Removed / discard manually if folder remains |
| Ferrite source changes | Force-link in `main.rs` | No |

---

## References

- [Mermaid parity matrix](./mermaid-parity-matrix.md) — gap list and priorities
- [mmdr README](https://github.com/1jehuang/mermaid-rs-renderer/blob/master/README.md)
- [docs.rs mermaid_rs_renderer](https://docs.rs/mermaid-rs-renderer/latest/mermaid_rs_renderer/)
