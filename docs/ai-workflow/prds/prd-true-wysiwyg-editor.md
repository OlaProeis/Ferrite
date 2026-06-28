# PRD: True WYSIWYG Markdown Editor (Source-Backed Document Model)

## Status

**Future / long-term — not scheduled for v0.3.x.** This is a forward-looking architecture
PRD that captures the target design and the reasoning behind it. It is a **complete rewrite
of rendered-mode editing**, not an incremental change to the current `RenderedEditSession`.
Scope, phasing, and a de-risking spike are defined below; actual scheduling is deferred.

## Version Target

**v0.4.0+ (post-FerriteEditor-crate)** — likely spread across multiple minor releases. Depends
on, and composes with, the v0.4.0 Unicode/RTL/BiDi shaping work and the FerriteEditor crate
extraction (v0.3.2).

## Priority

**HIGH value, HIGH effort.** Rendered-mode editing is a core product differentiator. The
current per-block-widget model has hit a structural ceiling (see Problem Statement). This is
the "do it properly" path to editor-grade WYSIWYG.

---

## Overview

Replace the current per-block-widget rendered editor with a **single source-backed document
model** that renders markdown as true WYSIWYG — formatting markers hidden, block elements
edited inline — while keeping the markdown source as the canonical, exactly round-tripped
storage format. This is the **Typora / MarkText model** adapted to Rust + egui, built on the
existing `FerriteEditor` rope/selection/undo infrastructure.

The goal is **editor-grade selection and editing in rendered mode**: drag-select across
paragraphs, lists, and headings; select-all; copy; and edit content without ever seeing raw
`**`/`` ` ``/link syntax — all while never corrupting the user's original markdown.

---

## Problem Statement

### User-visible symptoms (current rendered mode)

| Symptom | Cause |
|---------|-------|
| Cannot select across two paragraphs / a list / multiple blocks | Each block is a separate egui widget; egui selection cannot span widgets |
| Clicking a formatted paragraph "expands" it (text gets longer) | Formatted blocks toggle from styled display to **raw `**bold**` `TextEdit`** on click |
| No document-level Select All / copy of a styled range | Selection is tracked only inside the single focused block (`EditState.focused_selection`) |
| Two inconsistent editing paradigms | Plain blocks = always-on `TextEdit`; formatted blocks = display→raw toggle; tables/code = widget-local state |

### Root cause (architectural)

The rendered view is a **block compiler**: parse AST → render each top-level node as its own
egui widget, with at most **one active block** (`RenderedEditSession.active: Option<BlockRef>`).
This was the right fix for the v0.3.0 focus/commit bugs, but it makes cross-block selection and
true WYSIWYG **structurally impossible** — not a bug that can be patched. egui's immediate-mode
`TextEdit` cannot select across separate widgets, and cannot hide characters (which is why the
current code reveals raw markdown on click instead of editing in place).

### Why not just refactor the current model

Refactoring `editor.rs` (≈7.5k lines) into submodules improves maintainability but does **not**
unlock cross-block selection or marker-hidden editing. Those require a different editing surface.

---

## Product Decisions (locked)

These three decisions were made explicitly and drive the entire architecture:

| Decision | Choice | Consequence |
|----------|--------|-------------|
| **Markdown markers while editing** | **Always hidden** (Typora style) — never show `**`, `` ` ``, link syntax | Cursor/selection live in a *visible projection* space; every edit maps back to source with no visible markers to anchor on |
| **Block elements (tables, code, mermaid, images, video)** | **Fully inline-editable** in the document flow | Blocks are model nodes embedded in the text flow; selection enters/leaves them; inner edit mode on entry |
| **Markdown round-trip** | **Exact** — preserve the user's original source style/whitespace; never rewrite untouched source | Cannot reserialize a whole tree on save; must patch only changed byte ranges |

### Why these choices rule out the "obvious" architectures

- **Pure decoration-over-source ("Live Preview"/CodeMirror, reveal-on-edit):** assumes source is
  *sometimes shown* while editing. "Always hidden" removes that escape hatch — so a real
  visible↔source mapping is required for every edit, which is most of a document model.
- **Pure document tree that reserializes to markdown (ProseMirror/Notion):** breaks "exact
  round-trip" — a serializer rewrites untouched source (whitespace, link styles, list markers).

The correct architecture is therefore a **hybrid** that neither extreme names cleanly.

---

## Proposed Architecture

### High-level

```
Canonical markdown source (rope — FerriteEditor TextBuffer)
        │  parse (cached AST — existing markdown/parser.rs + cache.rs)
        ▼
Document model  ◄─── the actual edit + render surface
        │              • markers hidden, styled inline rendering
        │              • selection/cursor in a "visible position" space
        │              • position ↔ source-offset mapping (projection)
        │              • block nodes embedded in the flow (atomic for doc selection,
        │                inner edit on entry)
        ▼
Edits mutate the MODEL
        ▼
Incremental source patcher  ──► writes ONLY changed byte ranges back to the rope
                                 (untouched source keeps exact original style)
        ▼
Save = current rope contents (no whole-document reserialization)
```

### Core components

1. **Source store (rope)** — Reuse `FerriteEditor`'s `TextBuffer` (`ropey::Rope`) as canonical
   storage. O(log n) range patches; the rope content *is* what gets saved. Markdown stays
   authoritative; no lossy intermediate.

2. **Document model + projection** — A render/edit model derived from the cached AST, plus a
   **bidirectional position map** between:
   - **visible position space** (what the caret/selection move through — styled text with
     markers concealed, block embeds as atomic stops), and
   - **source offsets** (byte/char ranges in the rope).

   This is the heart of the rewrite. Cursor motion, click hit-testing, and selection rectangles
   all operate in visible space and resolve to source ranges via the map.

3. **Incremental source patcher** — Translates a model edit (insert text, toggle bold, split
   paragraph, merge list item, edit a table cell) into the **minimal source byte-range edit**.
   Generalizes the existing `update_source_range` / `block_replace_end_line` logic from
   `markdown/editor.rs`. Untouched source is never rewritten → exact round-trip.

4. **Hidden-marker inline renderer** — Per-line/per-span styled layout (`LayoutJob`) with marker
   characters concealed. Requires `FerriteEditor`'s renderer to support:
   - per-range inline styling (bold/italic/code/link colors + fonts within a line), and
   - **concealment**: source characters that produce **no glyph**, so the offset↔glyph mapping
     is no longer 1:1.

5. **Inline block embeds** — Tables, code fences (syntax + Run), mermaid, images, video as
   **variable-height widget rows** occupying a source line-range. Atomic for document-level
   selection; clicking enters an inner edit surface (reuse `EditableTable`, code editor, etc.).

6. **Reused infrastructure (FerriteEditor)** — rope, undo/history, HarfRust shaping, line cache,
   mouse drag / double / triple-click selection primitives, search. The genuinely **new** layers
   are the **projection/position map**, **concealment rendering**, and the **block-embed protocol**.

---

## Goals

### Must have

1. **Cross-block selection** — Drag-select across paragraphs, list items, and headings as one
   continuous selection; render the highlight across block boundaries.
2. **Select All + copy** — Document-level Ctrl+A; copy yields correct markdown source for the
   selected range.
3. **Markers always hidden** — `**`, `*`, `` ` ``, link/heading syntax never shown; formatting
   applied via toolbar/shortcuts.
4. **Exact round-trip** — Saving a file edited in WYSIWYG never alters untouched source bytes.
5. **Inline block editing** — Tables/code/mermaid/images render in-flow; click to edit in place.
6. **Caret affinity model** — Deterministic "inside vs outside a span" behavior at hidden-marker
   boundaries.
7. **Undo integration** — Composes with existing tab undo; one logical step per user action.

### Should have

8. **Literal-marker input** — A way to type a literal `*`/`` ` ``; transient marker reveal in
   genuinely ambiguous edit positions (pragmatic fallback even within an always-hidden design).
9. **Split-view parity** — Same model drives the split rendered pane.
10. **Large-file viability** — Viewport culling over the model (current rendered view already
    culls; preserve it).
11. **Complex-script / RTL composition** — Concealment math composes with shaped advances and the
    v0.4.0 BiDi work.

### Non-goals (this PRD)

- Replacing the **raw** editor (FerriteEditor raw mode stays as-is).
- A non-markdown canonical format / database-backed document.
- Fully seamless selection *flowing through the interior* of block embeds (tables/code) in v1 —
  blocks are atomic for document selection; inner edit on entry. Seamless flow is a stretch goal.
- Collaborative / multi-user editing.

---

## The Hard Problems (eyes open)

These are inherent to the locked product decisions and must be designed before implementation:

1. **Caret affinity at hidden-marker boundaries** — With no visible `**`, "is the caret inside or
   outside the bold?" needs an explicit affinity model (the classic WYSIWYG problem).
2. **Typing literal markdown / creating formatting** — With markers hidden, formatting is
   toolbar/shortcut-driven; literal `*` needs escape handling; ambiguous spots may transiently
   reveal source.
3. **Selection across block embeds** — Treat tables/code as semi-atomic with inner edit mode
   first; truly seamless interior flow is the hardest sub-problem (even Typora limits this).
4. **Structural keyboard edits** — Enter splits, Backspace-at-list-start merges, Tab indents —
   operate in model space; reuse `markdown/ast_ops.rs` where possible.
5. **Concealment ↔ shaping ↔ IME ↔ RTL** — All compose; the offset↔glyph map must remain correct
   under each.
6. **Performance** — Projection + concealment must stay viewport-culled and not regress large-file
   rendering gains from v0.2.8.

---

## Prior Art

- **MarkText** (open source) — custom document model with markdown as the source format; proves
  the *markdown-as-canonical + model-as-surface* shape is viable.
- **Typora** (closed) — the reference UX for always-hidden markers + inline block editing.

These validate the architecture's shape and confirm it is a large, multi-phase undertaking.

---

## De-Risking Spike (required before committing)

Build a throwaway branch proving the three things that decide the whole approach. If these feel
right, the remainder is "a large amount of well-understood work" rather than "unknown if possible."

1. **Hidden-marker inline edit** — A paragraph with a bold span: type into it, backspace across the
   boundary, and verify the source patches correctly with markers **never shown**.
2. **Cross-paragraph selection + copy** — Drag a selection across two paragraphs; copy returns the
   correct markdown source.
3. **Atomic block embed** — A code block embedded as a model node that selection can step over and
   click-to-enter.

---

## Phased Plan (indicative)

Each phase is independently demoable; ordering favors de-risking the projection/concealment core.

| Phase | Deliverable |
|-------|-------------|
| **0 — Spike** | The three spike proofs above; go/no-go on architecture |
| **1 — Projection core** | Source-backed model + bidirectional position map; read-only styled render with markers hidden; cross-block **selection + copy** (no editing yet) |
| **2 — Inline text editing** | Type/delete in projection space → incremental source patcher; caret affinity; undo integration |
| **3 — Formatting commands** | Toolbar/shortcut bold/italic/code/link with hidden markers; literal-marker input + transient reveal |
| **4 — Structural edits** | Enter/Backspace/Tab semantics via `ast_ops`; headings, lists, blockquotes |
| **5 — Block embeds** | Tables, code (syntax + Run), mermaid, images, video as inline atomic embeds with inner edit |
| **6 — Parity & polish** | Split-view parity, large-file culling, complex-script/RTL composition, full regression matrix; retire the old `RenderedEditSession` block path |

---

## Migration & Compatibility

- The new surface **replaces** the rendered/split-rendered path; raw mode is untouched.
- `RenderedEditSession` and the per-block `render_*` functions are removed only at Phase 6, after
  parity is proven. Until then, the rewrite lives behind a feature flag / experimental toggle.
- No file-format change: markdown remains canonical; existing documents open unchanged.

---

## Risks

| Risk | Mitigation |
|------|------------|
| egui is a constraint, not an enabler, for marker-hidden WYSIWYG | Paint + handle input directly in FerriteEditor (already proven in raw mode); spike validates feasibility |
| Projection/position-map complexity | Phase 1 ships it read-only first (selection/copy) before any editing |
| Round-trip regressions | Incremental patcher only; golden-file round-trip tests on every change |
| Scope creep into a multi-year rewrite | Strict phase gates; each phase demoable; feature-flagged until Phase 6 |
| Composition with RTL/BiDi/shaping (v0.4.0) | Sequence after FerriteEditor crate + shaping foundations; design map to be shaping-aware from Phase 1 |

---

## Open Questions

- Affinity UX details: how is "inside vs outside a span" surfaced to the user (caret shape,
  transient marker hint)?
- Literal-marker entry: dedicated escape, transient reveal, or both?
- Exact set of block types inline-editable in v1 vs deferred (e.g., video embeds).
- Interaction with the FerriteEditor **crate** boundary — does the WYSIWYG surface live in the
  crate or in the app integration layer?

---

## References

- Current architecture being replaced: [`rendered-edit-session.md`](../../technical/markdown/rendered-edit-session.md),
  [`prd-rendered-edit-session.md`](./prd-rendered-edit-session.md)
- Editor foundation: `src/editor/ferrite/` (rope, selection, undo, shaping, line cache)
- Source-range patch precedent: `markdown/editor.rs` (`update_source_range`, `block_replace_end_line`)
- Roadmap entry: [`ROADMAP.md`](../../../ROADMAP.md) → Future & Long-Term Vision
