# Ferrite — Math preview (PRD)

> **Status:** Implemented for v0.3.2. Reviewed 2026-11-18.
> **Version target:** v0.3.2. Preview rendering of `$...$` and `$$...$$` only.
> **Issue:** [Ferrite#191](https://github.com/OlaProeis/Ferrite/issues/191).
> **Consumers:** Human review, then a cyclopsctl task queue.
> **Later work:** [`docs/math-support-plan.md`](../../math-support-plan.md) and the v0.4.0 math section of `ROADMAP.md` (symbol palette, formula editing, PDF/HTML math typesetting). This PRD does not do that. It also does not build the from-scratch TeX box model that plan describes — `pulldown-latex` and `formulary` replace that path for the preview slice.

## 1. Overview

Rendered and Split preview should draw math the way Typora does for the common case: `$E = mc^2$` sits in the sentence, and `$$...$$` is a display formula. The file on disk stays LaTeX. Raw view stays the source text. Clicking a paragraph edits that source, dollars included.

Ferrite draws the preview itself. egui will not lay out MathML, and a `Label` cannot draw the glyph ids a math font uses (stretchy parentheses and large operators are not Unicode characters). The crates stop at a display list. Ferrite rasterizes that list into a cached texture and places it in the preview.

### Why this shape

| Choice | Decision |
|--------|----------|
| Input | Dollar math only, the syntax issue 191 asks for. Comrak 0.22 already parses it. |
| LaTeX → MathML | `pulldown-latex` 0.8 (MIT, MSRV 1.74). Fits `rust-version = "1.92"`. API: `Parser` → `push_mathml` with `RenderConfig { display_mode: Inline \| Block, .. }`. |
| MathML → positions | `formulary` 0.3 (MIT OR Apache-2.0, MSRV 1.87). Display list of glyph ids and rules. |
| Paint | Rasterize with `skrifa` (via formulary) and **new** dep `tiny-skia`. Upload one egui texture per cache key. |
| Font | Bundle STIX Two Math Regular (SIL OFL 1.1), ~820 KB–1.5 MB depending on the release artifact. One face. No font picker, no subsetter in this release. Accepted binary-size cost. |
| Not used | `math-core` 0.8+ needs Rust 1.96. `formel` 0.1.0 is too new and targets an HTML/CSS host. Typst is a different language and a whole compiler. No JS engine. |

**`formulary` features (important):** defaults are `["svg", "shaping"]`. `shaping` pulls `harfrust`; Ferrite already pins `harfrust = "0.5.2"` for editor text. Do **not** take the defaults. Depend on:

```toml
formulary = { version = "0.3", default-features = false }
```

That keeps deps to `read-fonts` / `skrifa` / `roxmltree` only — no second `harfrust`, no unused SVG helper. Stretchy operators and large ops still come from the OpenType MATH table (variants + assembly). Kerning / `ssty` shaping via harfrust is a later opt-in if quality needs it; do not unify harfrust versions in this release either way. Do not bump `rust-version`.

## 2. Goals

- With math dollars enabled, `$E = mc^2$` in a sentence renders as a formula in Rendered and in the Split preview pane.
- `$$...$$` renders as display math: larger operators, limits above and below where the font does that, on its own row.
- A formula that the converter or layout rejects shows the original delimited source and does not panic.
- `$20 and $30` stays text. Comrak's dollar rules stay in charge of what is math.
- Clicking the paragraph edits the markdown source, including the `$` delimiters. Leaving the edit paints the formula again.
- The same formula at the same size and color is rasterized once and reused across frames.
- Light and dark text colors both apply to the formula (theme text color wins over any MathML `mathcolor`).

## 3. Non-goals

- **Full LaTeX.** No preamble, no packages, no `\documentclass`, no prose typesetting. Only the math commands `pulldown-latex` accepts inside one formula.
- **A Ferrite-written layout engine.** Do not consume `pulldown-latex`'s event stream for layout. Do not port TeX's atom spacing. `formulary` is the layout.
- **`math-core`, `formel`, Typst, KaTeX, MathJax, or a JS runtime.**
- **Code math.** Do not enable comrak `math_code`. `` $`x`$ `` and ` ```math ` stay code.
- **`\(` `\)` and `\[` `\]`.** Do not enable comrak `math_latex`. Do not bump comrak to get `math_dollars_block`. On 0.22, display math is still a `NodeMath` span with `display_math: true`, including a `$$` fence that sits on its own lines inside a paragraph.
- **Raw rendering, syntax coloring of math, a symbol palette, WYSIWYG math editing.**
- **PDF or HTML math typesetting.** Export keeps the literal dollars as text (see 5.6). It does not draw the formula.
- **A math-aware spellchecker.** Raw squiggles inside a formula are acceptable.
- **A second math font, a font subsetter, or a background raster thread.** Rasterize on the UI thread. `pulldown-latex` 0.8 already caps macro expansion (depth 64, 1 MiB of expansion text). Rely on that.
- **Enabling formulary `shaping` / `svg`, or unifying `harfrust` copies.**

## 4. Current state (verified in code, 2026-11-18)

### 4.1 Parser

`comrak = "0.22"` (`Cargo.toml`). `MarkdownOptions::to_comrak_options` (`src/markdown/parser.rs`) does not set `extension.math_dollars`. Comrak 0.22's `NodeValue::Math(NodeMath)` is an inline node with `dollar_math`, `display_math`, and `literal` (the inside of the delimiters, not parsed as Markdown).

`MarkdownNodeType` has no math variant. `convert_node_value`'s `_` arm maps unknown nodes to `Text("")`. **Enabling `math_dollars` without a `Math` arm would silently delete formulas from the AST.** Mapping must land in the same change that flips the flag.

Comrak 0.22 already refuses currency-like dollars (`$20,000 and $30,000` stays text; `$ 4 $` is not math). Keep those rules. Do not reimplement them.

### 4.2 Preview

`render_inline_node` (`src/markdown/editor.rs`) walks inline nodes inside paragraphs, lists, headings, and table cells. `MarkdownNodeType::Image` ends the row and paints a cached texture (`render_image`). There is no inline widget that shares the text baseline yet — math must allocate a rect and paint with a baseline offset (see 5.4).

Paragraph click-to-edit seeds the paragraph source and writes it back on commit. A math texture has to live in that formatted view. The edit session keeps showing source.

### 4.3 AST / export surfaces that must learn `Math`

When `Math` is added, update every match that today falls through or would drop content:

| Surface | File / symbol | Required behavior |
|---------|---------------|-------------------|
| Comrak → AST | `convert_node_value` | `NodeValue::Math` → `MarkdownNodeType::Math { literal, display }` |
| Source round-trip | `serialize_inline_node` / paragraph commit | Emit `$literal$` or `$$literal$$` |
| Outline / heading plain text | `heading_plain_text` (and similar collectors) | Include the literal (without needing the texture); dollars optional but consistent |
| Display↔raw caret map | `parse_inline_display_spans` / `map_displayed_to_raw` | Treat a successful formula like one atomic span whose raw width is the delimited source; failure path is the delimited source text itself |
| HTML export | `src/export/html.rs` comrak options | **Leave `math_dollars` off** so `$...$` stays ordinary text in the markdown string comrak HTML-escapes |
| PDF export | `src/export/pdf/render.rs` `walk_inline` | **Leave `math_dollars` off**. If it is ever enabled, Math is a leaf — the current `_ => walk children` arm would emit nothing; must push `$literal$` / `$$literal$$` as text |

### 4.4 What must keep working

- Existing inline nodes, image textures, and paragraph/list/heading sessions.
- Outline headings and find-in-page still see the source bytes. A formula is not a new arrow-nav block.
- Files with no dollars parse and paint as they do today.

## 5. Functional requirements

### 5.1 Parse dollar math into our AST

**Behavior.**

- Set `options.extension.math_dollars = true` on the comrak options used for the **preview** parse only. Leave `math_code` off.
- Map `NodeValue::Math` to a new `MarkdownNodeType::Math { literal, display }` (`display` from `display_math`). `literal` is the formula text without the surrounding `$` / `$$`.
- Round-trip the source delimiters when a session commits. The buffer keeps `$literal$` or `$$literal$$`. Do not write MathML back into the file.
- A paragraph, list item, heading, blockquote, or table cell can contain math. Display math is still that inline AST node. Section 5.4 is what makes it a block on screen.

### 5.2 Convert and lay out

**Behavior.**

- New module **`src/markdown/math.rs`** (not a top-level `src/math/`). Public surface used by the preview: given `(literal, display, font_size_pt, color, pixels_per_point)`, return either a raster (`width`, `height`, `baseline_from_top`, RGBA) or a typed failure.
- Pipeline:
  1. `pulldown_latex::Parser` over the literal → `push_mathml` with `RenderConfig { display_mode: if display { Block } else { Inline }, ..Default::default() }`.
  2. On `Err`, return the failure. Do not pass user LaTeX to `formulary::parse`.
  3. `formulary::parse` then `formulary::layout` against one process-wide `MathFont` built from the bundled STIX Two Math bytes (`include_bytes!`, face index 0), held in a `OnceLock` (or equivalent). Rebuild layout on size change; do not re-parse the font bytes every formula.
- `LayoutOptions.font_size` is the rendered body size in points. Display vs inline is selected by the MathML `display` attribute from step 1, not by a second font size in this release.
- Unknown MathML that `formulary` recovers from (its warnings) still produces a raster. A `formulary` hard error (malformed XML / non-`<math>` root) is a failure, same as a LaTeX error.
- No network, no process spawn, no read of the user's font directory.
- Ignore formulary item colors / MathML `mathcolor`; always paint with the Ferrite preview text color so light/dark themes stay consistent.

### 5.3 Rasterize and cache

**Behavior.**

- Draw each layout item: glyph outlines from `skrifa` at the item's position and size, and rules (fraction bars, radical bars) as filled rects, in the preview text color, into a `tiny-skia` pixmap.
- Scale the pixmap by `pixels_per_point` so it is sharp on that monitor. Record `baseline_from_top` in pixels from `Layout.ascent` (formulary's baseline contract: place so ascent sits above the text baseline).
- Cache by `(literal, display, font size in points, rgba color, pixels_per_point quantized to 0.25)`. Cache the egui `TextureId` (or `TextureHandle`), not only the pixmap. Cap the cache (256 entries, drop oldest / LRU). A theme or zoom change misses the cache and rebuilds.
- Do not rebuild a cache hit on later frames.
- Ship `STIXTwoMath-Regular.otf` plus its `OFL.txt` under `assets/fonts/`. The font is data, not a UI font family. Do not register it on Proportional or Monospace.
- Soft budget: prefer cache hits after first paint. A cold miss that takes tens of ms on a complex formula is acceptable for v0.3.2; do not add a worker thread. Log `warn!` once if a single raster exceeds ~50 ms so QA can spot pathological inputs.

### 5.4 Place it in the preview

**Behavior.**

- Inline (`display == false`): stay in the current inline row. Allocate a rect of the texture size; paint so the formula baseline matches the surrounding rendered font baseline (measure body text the same way other preview text is measured; do **not** vertically center the texture on the row). Grow the row when the formula is taller than the text (ascent and descent both matter).
- Display (`display == true`): end the current row, paint the texture as a block in the content width — centered when narrower than the pane; **scale down uniformly to fit** when wider (do not clip through glyphs). Then continue following inlines on a new row. This covers a paragraph that is only

```text
$$
E = mc^2
$$
```

  and also a `$$...$$` in the middle of a sentence. Both are blocks on screen. That is the v0.3.2 rule.
- Table cells use the same paths. A display formula in a cell is still a block inside the cell (cell grows).
- Preview lock paints the texture and does not enter an edit.
- Raw view and the Split raw pane do not call the painter.
- Click target for edit remains the parent paragraph / list item / heading / cell, not a separate math widget.

### 5.5 Failure and editing

**Behavior.**

- On failure (or when `render_math` is off), paint the delimited source (`$...$` or `$$...$$`) in the editor / rendered font, in the quote/muted color already used for unrendered HTML (`«HTML»` / `colors.quote_text`). Tooltip is a fixed locale string plus the converter's message. Do not show a blank gap. Do not log once per frame; log once per `(literal, display)` while it stays failing.
- Clicking a formatted paragraph, list item, or heading that contains math enters the existing session. The seeded text includes the dollar delimiters and the literal. While the session is open, the user sees source, not the texture. Commit paints again.
- Do not add a math-only editor, a caret inside the texture, or a new arrow-nav stop for the formula.
- Locale: one new key for the failure tooltip, in the English catalog and the other shipped catalogs (empty-or-English fallback is not enough; follow `locales/*.yaml` the way other preview strings do).

### 5.6 Export and search

**Behavior.**

- **Do not enable `math_dollars` on HTML or PDF export comrak options.** Dollars remain ordinary characters in the source string and survive as visible text. That meets "must not skip the node / must not emit MathML" without teaching export about `NodeValue::Math`.
- If a future change shares preview options with export, PDF `walk_inline` and HTML rendering must emit the delimited source for Math leaves explicitly (see 4.3).
- Find uses the buffer, so the literal remains searchable. No preview-only search index for formulas.

### 5.7 Escape hatch

**Behavior.**

- One setting, `render_math: bool`, default `true`, `#[serde(default = "default_true")]` (or `#[serde(default)]` with `Default` impl `true`) so old `config.json` files load. When false, the preview paints the delimited source and does not call the converter. Put it on the editor settings page (same area as other preview-facing toggles such as strict line breaks). One registry entry, one locale label.

## 6. Acceptance

Fixtures, Rendered view, default settings, STIX Two Math:

| Input | Pass |
|-------|------|
| `Energy $E = mc^2$ is rest energy.` | Sentence stays one paragraph. `E = mc^2` is a formula on the text baseline, not the characters `E = mc^2` in the body font. |
| A paragraph that is only `$$E = mc^2$$` | One centered display formula. |
| `$$\sum_{i=0}^{n} i = \frac{n(n+1)}{2}$$` | Summation and fraction are visible. The fraction bar is a rule, not the character `/`. |
| `$\sqrt{x^2 + y^2}$` | Radical drawn around the radicand. |
| `$\alpha + \beta = \gamma$` | Greek letters from the math font. |
| `$20 and $30` | Both amounts stay body text. |
| `$\frac{$` | Delimited source is visible, no panic, tooltip explains the failure. |
| Click the first fixture, type a character, click away | Source edit commits. The formula paints again. Undo restores the paragraph. |
| `render_math` off | The first fixture shows `$E = mc^2$` as text. |
| Split view | Preview pane matches Rendered. Raw pane shows the dollars. |
| HTML / PDF export of the first fixture | Output still contains the characters `$E = mc^2$` (not blank, not MathML). |
| Theme switch light ↔ dark | Formula recolors; no stale wrong-color texture. |

Out of scope for the pass bar: matrices, `\mathbb`, custom macros, and matching KaTeX pixel for pixel. If those happen to work, keep them. If `formulary` drops a construct, the failure path in 5.5 is the correct result.

## 7. Tests

Targeted:

- Parser: `$x^2$` becomes `Math { display: false, literal: "x^2" }`. `$$x^2$$` is `display: true`. `$20 and $30` does not produce a math node. Enabling the flag without the Math arm is covered by the positive mapping test.
- Round-trip: `serialize_inline_node` (or the paragraph serializer) of a node containing `$x$` yields those three characters in the committed source.
- Layout: the summation fixture returns a raster whose height is greater than the inline `E = mc^2` fixture at the same font size, and `baseline_from_top` is inside the bitmap. Needs the bundled font bytes, not the network.
- Failure: `$\frac{$` returns the failure type and does not panic.
- Cache: two calls with the same key do not lay out twice (a counter on the layout function is enough). A different color misses.
- Export: a markdown string with one inline formula still contains `$E = mc^2$` in the HTML output (and PDF text path if cheap to assert).
- Existing paragraph-session tests still pass. Add one only if a current harness can show that seeding a paragraph which contains `$x$` keeps those three characters. Do not stand up a new egui harness for the texture.

No pixel golden files. No full `cargo test` as the deliverable of each edit. No RSS check.

## 8. Suggested task split (for cyclopsctl)

Keep parent tasks small enough for one implementation session each:

1. **Deps + font asset** — add `pulldown-latex`, `formulary` (`default-features = false`), `tiny-skia`; vendor STIX + OFL; smoke `MathFont::new`.
2. **AST + parse flag** — `MarkdownNodeType::Math`, converter arm, preview `math_dollars`, serializer / outline / display-span hooks, parser tests.
3. **Convert → layout → raster + cache** — `src/markdown/math.rs` public API, OnceLock font, cache, unit tests (no egui).
4. **Preview placement** — `render_inline_node` inline + display paths, baseline alignment, failure paint, preview lock.
5. **Setting + i18n + export check** — `render_math`, registry, locales, confirm HTML/PDF still show dollars; docs bullet.

## 9. Docs

When the code lands, add `docs/technical/markdown/math-preview.md` (pipeline, cache key, display-math-is-a-block rule, formulary `default-features = false`, export leaves `math_dollars` off) and a single bullet in `docs/index.md`. Point `docs/math-support-plan.md` at this PRD for the preview slice so the from-scratch layout section is not the next implementation step. Leave the v0.4.0 roadmap items in place.

## 10. Risks (accepted or mitigated)

| Risk | Mitigation |
|------|------------|
| Enabling `math_dollars` before `Math` AST arm | Same PR/task; converter currently maps unknown → empty `Text` |
| Dual `harfrust` / lock churn | `formulary` with `default-features = false` |
| UI hitch on first paint of many unique formulas | Texture cache; soft 50 ms warn; escape hatch `render_math` |
| ~1 MB binary growth | Accepted; no subsetter in v0.3.2 |
| egui baseline alignment fiddly | Explicit `baseline_from_top`; acceptance row for inline fixture |
| Export dropping math if options are shared later | Documented; keep export flag off |
