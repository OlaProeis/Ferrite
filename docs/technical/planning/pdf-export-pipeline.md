# PDF Export Pipeline — Design Decision

> **Status:** Decision (research + spike comparison) — v1 has shipped, see [`pdf-export.md`](../viewers/pdf-export.md)
> **Target:** v0.3.x
> **Created:** 2026-05-09
> **Related:** [`pdf-export.md`](../viewers/pdf-export.md) (v1 implementation), [`document-export.md`](../viewers/document-export.md) (existing HTML export), [`pdf-viewer.md`](../viewers/pdf-viewer.md) (read path uses `hayro`)

## Executive Summary

Ferrite needs a **PDF export** path alongside the existing HTML export so that users can hand a single self‑contained file to colleagues, attach it to email, or archive a snapshot of a note with theme, fonts, code highlighting, and Mermaid diagrams preserved.

After comparing three families of approaches — **HTML intermediate**, **native Rust generation**, and **system print dialog** — the recommendation is:

> **Adopt the native‑Rust path built on [`krilla`](https://crates.io/crates/krilla) (+ `krilla-svg`), reusing Ferrite's existing comrak AST and font stack. Keep the existing HTML export as a parallel "open in browser → Print to PDF" escape hatch for users who want browser‑grade CSS fidelity without the in‑app constraints.**

Rationale in one paragraph: `krilla` is from the **same author as `hayro`** (already in the dependency tree for the PDF viewer), is pure Rust under MIT/Apache‑2.0, ships a modern font subsetting + shaping stack (`skrifa` + `subsetter` + `rustybuzz`), supports vector SVG embedding for Mermaid via `krilla-svg`, and adds roughly the same order of bundle weight as the existing `hayro` read path. Browser/Chromium options would force Ferrite to ship or fetch ~200 MB of Chromium per user; system print dialogs do not actually render HTML themselves; and the older pure‑Rust crates (`printpdf` HTML mode, `genpdf`) are either explicit stubs or effectively unmaintained.

---

## Goals & Non‑Goals

### Goals (v0.3.x)

- One‑click PDF export from a Markdown tab → single self‑contained `.pdf` on disk.
- **Self‑contained:** subsetted fonts and (where applicable) inlined raster images; no external dependencies at view time.
- **Theme parity:** light/dark + user accent color carried through (background, headings, code blocks, blockquotes, links).
- **Fidelity targets:** headings, body text, lists, blockquotes, tables (with inline formatting), fenced code blocks (syntect coloring), inline `code`, links, horizontal rules, raster images.
- **Mermaid:** all 11 diagram types render in the PDF as sharp vector graphics — no blurry bitmap.
- **Cross‑platform:** identical output on Windows / macOS / Linux from the same binary.
- **Bundle impact ceiling:** ≤ ~5 MB added to the release binary; no new C/C++ build dependency.

### Non‑Goals (this round)

- Full CSS print stylesheet support (page counters, custom margin boxes, `@page` rules at WeasyPrint level).
- LaTeX‑grade math typesetting (deferred to the [Math Support Plan](../../math-support-plan.md); pre‑math export is acceptable for v0.3.x).
- Editable / round‑trippable PDFs.
- Encrypted, signed, or PDF/A‑compliant output (could come later via the same `krilla` writer).
- Batch export, custom templates, or print preview UI (separate task).

---

## What Has to Survive the Pipeline

| Element | Source in Ferrite | Difficulty |
|---|---|---|
| Headings, paragraphs, lists, blockquotes, HR | comrak AST | Low — direct draw |
| Inline emphasis: bold / italic / strike / `code` | comrak AST | Low |
| Tables (incl. inline formatting in cells) | comrak AST + [`table-inline-formatting.md`](../markdown/table-inline-formatting.md) | Medium — column layout + inline shaping |
| Fenced code blocks | comrak + syntect | Medium — token color spans, monospace font |
| Links (inline + autolink) | comrak | Low — `LinkAnnotation` |
| Wikilinks `[[target]]` | Ferrite resolver | Low |
| Raster images (PNG/JPEG/GIF/WebP/BMP) | `image` crate | Low — PDF XObject |
| Local image path resolution | existing logic | Low |
| Mermaid diagrams (11 types) | `src/markdown/mermaid/*` (egui Painter) | **High** — needs dedicated emit path |
| Theme colors (light/dark + user accent) | `ThemeColors` | Low — passed into renderer |
| CJK + complex scripts (Arabic, Devanagari, Bengali, …) | `harfrust` shaping + lazy fonts | Medium — must subset chosen fonts |
| Color emoji | font fallback chain | Medium — depends on bundled font |
| GitHub‑style callouts | parsed in `markdown/widgets.rs` | Low — recolored block |
| Task‑list checkboxes | parsed | Low — small vector glyph |
| Frontmatter | hidden in render today | N/A — exclude by default |

The single hardest constraint is **Mermaid**: today every diagram is drawn straight onto an `egui::Painter` from CPU code. There is no SVG nor PNG cached anywhere. Whatever PDF strategy we pick must offer a way to reproduce those drawings without a GPU.

---

## Strategy Family A — HTML Intermediate

Reuse the existing `generate_html_document()` output and let *something* turn it into a PDF.

### A1 — Embed a headless Chromium (`chromiumoxide` / `headless_chrome`)

| | |
|---|---|
| **Fidelity** | Best in class. Real CSS, real web fonts, real SVG, JS could even re‑run Mermaid in a browser sense. |
| **Bundle / install cost** | **Devastating.** Neither crate bundles a browser; both speak CDP to an external Chrome. `chromiumoxide_fetcher` can auto‑download Chrome for Testing on first use, but the user still pays ~200 MB of disk and a multi‑hundred‑MB process at runtime. |
| **Cross‑platform reliability** | Good once installed; depends on user's Chrome version otherwise. Sandbox/Flatpak adds friction. |
| **Maintenance** | `chromiumoxide` 0.9.x is actively maintained (MIT/Apache); `headless_chrome` 1.0.x is alive but slower. CDP stability is good. |
| **Verdict** | **Reject for in‑app export.** Distribution cost is an order of magnitude over the rest of the binary. Could be a power‑user opt‑in later. |

### A2 — `wkhtmltopdf` / WeasyPrint / Prince

| | |
|---|---|
| **wkhtmltopdf** | Officially **archived January 2023** (Qt WebKit fork). Rust crate is similarly stalled. Treat as deprecated. |
| **WeasyPrint** | Solid paged‑media CSS, but Python — only realistic as a subprocess. No JavaScript, so any client‑side Mermaid would fail there too. |
| **Prince XML** | Best HTML/CSS fidelity in the industry, **commercial** (USD ~$495/desk, ~$3,800+/server). Wrong fit for an OSS editor. |
| **Verdict** | **Reject** — none of these can ship inside Ferrite as a Rust crate. |

### A3 — Pure‑Rust HTML/CSS rendering crates (`Fulgur`, `Ironpress`, `printpdf` `html` feature)

| | |
|---|---|
| **`Fulgur`** | Pure‑Rust paged‑media CLI, no browser; reasonable CSS subset, font subsetting, image embedding. The most credible Rust replacement for `wkhtmltopdf`. Bundling adds non‑trivial deps; we'd be adopting another full layout engine in parallel to egui's. |
| **`Ironpress`** | Pure‑Rust HTML/CSS/Markdown→PDF, advertises math + fast cold start, less battle‑tested. |
| **`printpdf`'s `html` feature** | The README is explicit: "**compiles, but won't produce usable output**." Stub. |
| **Verdict** | **Defer.** Promising but immature; would still need a separate Mermaid path because none of them know about our diagrams, and we'd be carrying *two* layout engines (theirs + egui) in the same binary. |

### A4 — Open the HTML in the system browser → user uses "Save as PDF"

| | |
|---|---|
| **Fidelity** | Excellent (Chromium, Firefox, Safari are all very capable on a static document). |
| **Bundle / install cost** | **Zero.** We already write HTML. We already use the `open` crate. Code change is one button. |
| **UX cost** | One extra modal — the browser print dialog — and the user picks a filename. |
| **Determinism** | Output depends on user's browser; may render emoji and Chinese fonts differently than the in‑app preview. |
| **Verdict** | **Keep as a parallel option.** It is essentially free, gives users a high‑fidelity escape hatch, and never blocks shipping the native path. |

---

## Strategy Family B — Native Rust Generation

We feed our own intermediate (the comrak AST already in hand) to a Rust PDF writer and lay it out ourselves.

| Crate | Latest | Status (May 2026) | API level | Self‑contained fonts | Vector / SVG | Bundle weight | Comments |
|---|---|---|---|---|---|---|---|
| **`krilla`** | 0.7.0 | **Active** (2026‑03‑31, same author as `hayro`) | Mid/high — `Document` → `Page` → `Surface` (text, paths, images, gradients, masks) | **Yes** — `skrifa` + `subsetter` (+ `rustybuzz` shaping under `simple-text`) | Paths + **`krilla-svg`** companion crate | Small (~174 KB crate; deps overlap with what `hayro` already pulls) | Backbone of `typst-pdf`; tagged‑PDF (a11y) ready; MIT/Apache‑2.0. |
| **`printpdf`** | 0.9.1 | Active, single maintainer | Mid (operation streams) + stub `html` | Yes (`allsorts`) | Paths + SVG via `svg2pdf` | **Heavy** (~8.1 MB crate; `azul-layout` + `kuchiki` deps) | Authoring more verbose than `krilla`. |
| **`genpdf`** | 0.2.0 | **Stalled** (last release 2021‑06) | High (paragraphs, tables) | `rusttype` (no shaping, no color emoji) | Rudimentary | Small | Pinned to a 5‑year‑old `printpdf 0.3` API. **Skip.** |
| **`typst-pdf`** | 0.14.2 | **Very active** (Typst Inc.) | Exporter for a Typst document | Yes (Typst engine) | Full (uses `krilla`) | **Multi‑MB** (entire Typst engine + assets) | Best CJK / math, but you must compile Typst markup, not Markdown. |
| **`pdf-writer`** | 0.14.0 | Active (Typst team) | **Low** — emit PDF objects | DIY | DIY | Lean | Building block; what `krilla` itself uses. |
| **`lopdf`** | 0.40.0 | Active | Low/mid object graph | DIY | DIY | ~7 MB crate (default features) | Best for read/merge/post‑process, not green‑field authoring. |

### B‑pick: `krilla` + `krilla-svg`

Why this and not `printpdf` or `typst-pdf`:

- **Affinity with the existing `hayro` dependency.** Same author (LaurenzV), same modern Rust PDF lineage, overlapping transitive deps (`pdf-writer`, `skrifa`, `subsetter`). Bundle delta should be the smallest of the bunch.
- **`krilla-svg`** is the documented path for embedding vector graphics. It is also what `typst-pdf` itself uses, so the integration is battle‑tested.
- **Tagged‑PDF / accessibility hooks** are a forward door (screen reader friendly exports) without changing the core API later.
- **Font subsetting + shaping** are first‑class — required for self‑contained CJK / Arabic / Bengali content that Ferrite already supports in‑editor via lazy font loading.
- **Pure Rust, no C build dependency.** Same posture as the rest of Ferrite.
- **License:** MIT / Apache‑2.0 — aligns with Ferrite's MIT.

What `krilla` does **not** do (and why that's OK):

- It is not an HTML/CSS engine. We already have the comrak AST and our own renderer; we own the layout and theming anyway, so feeding the AST straight to `krilla` is *less* glue than going through HTML.
- It is not a layout engine for paragraphs. We need a thin block layout pass on top: vertical stacking, page breaking, table column widths, list indentation, code‑block padding. This is ~hundreds of lines, not thousands, because the visual model already exists in `markdown/widgets.rs`.

---

## Strategy Family C — System Print Dialog

| | |
|---|---|
| **What it actually is** | Per‑OS APIs that take an *already‑rendered* document (PDF, image, raw print stream) and route it through the OS print pipeline, optionally selecting a virtual PDF printer (`Microsoft Print to PDF` on Windows, `Save as PDF` on macOS, `cups-pdf` on Linux). |
| **What it is not** | An HTML or Markdown renderer. The OS does not turn HTML into PDF for you. |
| **Rust crates** | `winprint` / `windows-rs` (Windows), `objc2-app-kit` (macOS `NSPrintOperation`), `cups_rs` (Linux). All low‑level. |
| **Friction** | Always opens a confirmation dialog (cannot silently print on macOS / Linux without explicit permission), platform‑specific code triplicated, sandboxing on Flatpak / macOS App Sandbox adds more friction. |
| **Verdict** | **Reject as the primary export path.** It only becomes interesting *after* we have a PDF in hand from Family B — at which point sending it to a physical printer is a separate (smaller) feature, not part of "PDF export." |

---

## Comparison Matrix

| Criterion | A1 Headless Chromium | A4 Browser fallback | B `krilla` (recommended) | B `printpdf` | B `typst-pdf` | C System print |
|---|---|---|---|---|---|---|
| Pure Rust, no C build dep | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Self‑contained PDF (no external assets) | ✅ | depends on browser | ✅ | ✅ | ✅ | n/a |
| CSS / theme fidelity | **Highest** | High | Driven by us (high — we own the styling) | Same as krilla | Highest of the natives | n/a |
| Mermaid diagrams (vector) | ✅ if rendered in browser | ✅ if pre‑rasterized in HTML | ✅ via `krilla-svg` (after adding SVG emit path) | ✅ via `svg2pdf` | ✅ via `krilla-svg` | n/a |
| Subsetted fonts | ✅ | n/a | ✅ | ✅ | ✅ | n/a |
| CJK / complex scripts | ✅ | ✅ | ✅ (rustybuzz) | ✅ (allsorts) | ✅ (best) | n/a |
| Bundle impact | **~200 MB Chromium** | **0 MB** | **small** (overlaps with `hayro`) | medium‑large | **multi‑MB** | small |
| New transitive C/C++ deps | none in‑binary, but ext. browser | none | none | none | none | platform FFI |
| Maintenance burden in our tree | high (CDP version drift) | trivial | low–medium (own AST → draw layer) | medium (verbose API) | high (Typst markup translator) | high (3× platforms) |
| Time to first credible spike | days (auto‑fetch path) | hours | days | days | weeks (Typst integration) | weeks (per‑platform) |
| License | MIT/Apache‑2.0 | n/a | MIT/Apache‑2.0 | MIT | Apache‑2.0 | n/a |

---

## Bundle‑Size Impact (rough, x86_64 release build)

These are **estimates** — exact deltas should be measured during the implementation spike with `cargo bloat --release --crates`.

| Option | Estimated added crate weight (compiled) | New transitive crates | Notes |
|---|---|---|---|
| A1 Chromium auto‑fetch | ~1 MB in binary, **+ ~200 MB on disk** at runtime | `chromiumoxide`, `chromiumoxide_fetcher`, `tokio` (already optional via `async-workers`) | Real cost is at the user's disk, not in the binary. |
| A4 Browser fallback | ~0 KB | none | Reuses existing `open` crate. |
| **B `krilla` + `krilla-svg`** | **~1.5 – 3 MB** | `krilla`, `krilla-svg`, `usvg`, `subsetter`, `skrifa` (overlap with `hayro`'s transitive set), `rustybuzz` (we also use `harfrust` — small duplication) | Most overlap already paid for by `hayro`. |
| B `printpdf` | ~3 – 6 MB | `azul-layout`, `kuchiki`, `allsorts`, `lopdf`, `svg2pdf` | Heavy parallel layout engine. |
| B `typst-pdf` | ~10+ MB | full Typst stack | Doubles or triples Ferrite's size. |
| C System print | ~100 – 300 KB | `windows`/`objc2-app-kit`/`cups_rs` | Platform code triplicated. |

We should keep the option to put `krilla` behind a Cargo feature flag (default‑on) so distributors who insist on the smallest possible build can opt out.

---

## Decision

**Go with Strategy B using `krilla` + `krilla-svg`, and keep Strategy A4 (browser fallback) as a always‑available secondary action in the Export menu.**

Concretely:

1. **Primary "Export → PDF":** comrak AST → Ferrite's own block renderer → `krilla` page surface. Self‑contained, themed, deterministic.
2. **Secondary "Export → HTML, then Print to PDF":** unchanged behavior — write the existing HTML, open it in the user's default browser, the user uses the browser's built‑in Print to PDF. Already free for us today.
3. **No Chromium, no Typst, no system‑print path** in v0.3.x. They remain on the table for a future version if user demand justifies the cost.

### Why this is the right call for Ferrite specifically

- **It matches the rest of the stack:** pure Rust, zero C deps, single binary, deterministic across platforms — the same ethos as `ferrite` editor, `harfrust` shaping, `hayro` viewer.
- **It piggybacks on a dependency we already pay for.** The transitive footprint of `krilla` overlaps heavily with `hayro`; we are not bringing in a brand‑new ecosystem.
- **It keeps Mermaid first‑class.** A custom SVG emit pass for the Mermaid module gives us crisp, infinitely‑zoomable diagrams in the exported PDF — the very property Ferrite users would expect from a vector‑native editor.
- **It preserves a no‑lock‑in escape hatch.** The browser fallback is one button, takes hours not days, and matches what most editors actually ship today.

---

## Implementation Roadmap (high level)

This is a sketch to validate that the chosen strategy is buildable in v0.3.x. **Implementation is a separate Task Master task** and is **not in scope for the current research task #60.**

### Phase 1 — Skeleton (≈1–2 days)

- Add `krilla` and `krilla-svg` to `Cargo.toml` behind a `pdf-export` Cargo feature (default‑on).
- New module `src/export/pdf/` (`mod.rs`, `document.rs`, `layout.rs`, `error.rs`).
- New `ExportFormat::PdfFile` variant in `src/export/options.rs`.
- New `RibbonAction::ExportPdf` and a stub handler that emits a one‑page hello‑world PDF.
- Wire `Ctrl+Shift+P` (or another free shortcut — confirm at impl time) and a ribbon button.

### Phase 2 — Block renderer (≈3–5 days)

- Walk the comrak AST and emit `krilla` draw calls for: headings, paragraphs (with inline shaping), lists (`ul`/`ol` + nesting), blockquotes, horizontal rules, links (with `LinkAnnotation`), tables (use existing column‑width logic from `markdown/widgets.rs`).
- Reuse `ThemeColors` exactly as the HTML exporter does — same source of truth, same accent.
- Page break logic: page size from `Settings` (default A4), top/bottom margins, simple "if next block doesn't fit, new page."

### Phase 3 — Code blocks + images (≈2 days)

- Fenced code blocks: re‑run syntect on the source (already cached), emit colored runs in the bundled monospace font.
- Images: load via the existing `image` crate path, embed as `krilla` image XObjects. Honor `ImageHandling::EmbedBase64` semantics (always embed for PDF — relative paths make no sense once the file leaves the workspace).

### Phase 4 — Mermaid via SVG (≈3–4 days)

- Add a `MermaidSvgEmitter` mirror of the existing egui `Painter` path. Each diagram type implements `to_svg(&self, theme: &ThemeColors) -> String`. Layout numbers are already produced by the existing `mermaid/flowchart/layout/` pipeline, so this is mostly serialization, not re‑geometry.
- Pipe SVG → `krilla-svg` → embedded vector graphics.
- Cache SVG strings keyed by `blake3(source + theme + version)` in the same store as the AST cache (`mermaid/cache.rs`).
- Fallback: if a diagram fails to emit SVG, rasterize off‑screen via `usvg`+`tiny-skia` to a PNG and embed as image. Never crash the export.

### Phase 5 — Fonts + scripts (≈2 days)

- Build the subset list from the rendered text (`HashSet<char>` walk).
- Reuse Ferrite's font selection logic to pick the right family per script (CJK lazy loading already maps script → font family).
- Subset via `krilla` / `subsetter` and embed.
- Color emoji: bundle Noto Color Emoji COLR/CPAL on first export (lazy), or fall back to the Twemoji svg path.

### Phase 6 — Polish + settings (≈1–2 days)

- "Page setup" sub‑section in the Export part of `Settings`: page size (A4 / Letter), margin presets, include‑header toggle, include‑page‑numbers toggle.
- Toast on success with "Reveal in folder" / "Open" actions, mirroring HTML export.
- Persist `last_pdf_export_directory`.

### Validation gates per phase

- `cargo build` and `cargo test` clean after each phase.
- Manual test against `docs/archive/release-qa/v0.2.6-manual-test-suite.md`‑style document (kitchen‑sink markdown).
- Manual test against `docs/technical/markdown/` examples covering tables, callouts, task lists, wikilinks.
- Manual test against every Mermaid type in `src/markdown/mermaid/*.rs`.
- Diff the PDF in a viewer (Adobe Reader, macOS Preview, Edge) to confirm fonts, links, and bookmarks survive.

---

## Risks & Open Questions

| # | Risk | Mitigation |
|---|---|---|
| R1 | `krilla` and `hayro` both depend on `pdf-writer`/`skrifa` but at different versions, causing duplicate compilation. | Pin compatible versions during the Phase 1 spike; if duplication is unavoidable, accept the small bundle hit (<500 KB). |
| R2 | Mermaid SVG emit doubles the maintenance surface for the diagram modules. | Implement the SVG emitter behind a single `MermaidExport` trait so each diagram module gets one `impl` block; reuse the geometry already produced by the layout pass — no new layout code. |
| R3 | Color emoji adds ~10 MB if we bundle Noto Color Emoji. | Make emoji font lazy: download or extract on first PDF export, store in `dirs::data_dir()`, mention in the privacy doc. Default to monochrome glyphs if not present. |
| R4 | Pagination of long code blocks is genuinely hard (no inner break points). | v0.3.x acceptance: break code blocks across pages line‑by‑line; revisit "syntax‑aware" breaking later. |
| R5 | Right‑to‑left text (Arabic, Hebrew) needs bidi reordering, which `krilla`'s `simple-text` feature does not do. | Use `unicode-bidi` (already in the dependency tree via `harfrust` siblings) to reorder runs before handing them to `krilla`. |
| R6 | RTL + tables together: column order. | Out of scope for v0.3.x; document the limitation in the user docs. |
| R7 | PDF/A or PDF/UA compliance for accessibility/archival. | `krilla`'s tagging module is already there; revisit when there is a concrete user request. |

### Open questions for the implementation task

- **Default page size:** A4 (Europe / rest of world) vs Letter (US). Suggestion: derive from system locale, override in Settings.
- **Print headers / footers:** include filename + page number by default? (Likely yes, off by default — opt‑in to keep export deterministic.)
- **Hyperlinks scope:** also emit a PDF outline from the document headings (TOC). Almost free with `krilla::outline` — recommend yes.
- **Wikilinks across files:** in a single‑file PDF, broken wikilinks should render as plain text (parity with HTML). Only emit a `LinkAnnotation` when the target is the same file's heading.

---

## Validation / Success Criteria (for this decision)

The chosen pipeline (krilla + krilla‑svg, with browser fallback as escape hatch) is "good enough" if a v0.3.x prototype can produce, **from a single button**, a `.pdf` that:

1. Opens in Adobe Reader, macOS Preview, Edge/Chromium PDF viewer, and Firefox PDF viewer with no warnings.
2. Has **no missing glyphs** for ASCII, accented Latin, Chinese, Japanese, Arabic, and Devanagari sample text — using the same fonts the in‑editor preview shows.
3. Renders one Mermaid flowchart, one sequence diagram, and one class diagram as **selectable‑text vector** content (zoom 8× with no pixelation).
4. Preserves theme background, heading color, link color, code‑block background, and the user's Ferrite accent color.
5. Includes a working clickable link (external URL) and a working internal link (TOC entry → heading).
6. Stays under **5 MB** for a 10‑page, image‑light document with subsetted CJK + Latin fonts.
7. Is **byte‑identical** when produced twice from the same source on the same OS (deterministic output).
8. Adds **no more than ~3 MB** to the Ferrite release binary on `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`.

If any of (1)–(5) fails after the Phase 1–4 spike, fall back to making the **browser path** the primary action and downgrade native PDF to "experimental, opt‑in via Cargo feature." This rollback is cheap because the HTML pipeline already exists.

---

## Cross‑references

- Existing HTML export: [`document-export.md`](../viewers/document-export.md)
- PDF read path (also `LaurenzV` / `hayro`): [`pdf-viewer.md`](../viewers/pdf-viewer.md)
- Theme colors used as the source of truth: [`theme-system.md`](../ui/theme-system.md)
- Mermaid module map (drives Phase 4): [`mermaid-modular-structure.md`](../mermaid/mermaid-modular-structure.md)
- Math export interplay (deferred): [`math-support-plan.md`](../../math-support-plan.md)
- Cargo features for distribution: see `[features]` in [`Cargo.toml`](../../../Cargo.toml)
