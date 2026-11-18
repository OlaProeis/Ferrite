# Math preview

Rendered and Split preview draw `$...$` inline and `$$...$$` as a block. The file stays LaTeX. Raw view and the Split raw pane do not paint formulas.

## Pipeline

`src/markdown/math.rs`:

1. Preview parse sets comrak `extension.math_dollars` (`MarkdownOptions::to_comrak_options`). `NodeMath` maps to `MarkdownNodeType::Math { literal, display }`. `math_code` and `math_latex` stay off. Display math is still one inline node with `display: true`.
2. `pulldown-latex` 0.8 `Parser` → `push_mathml` with `RenderConfig.display_mode` of `Inline` or `Block`. A parser `Err` is cached and never passed to `formulary::parse`. Before parse, Ferrite rewrites that MathML: a raw `<` inside an operator becomes `&lt;`, and `&nbsp;` / the literal text `&nbsp;` from `\` + space becomes `&#160;`.
3. `formulary` 0.3 `parse` + `layout` against one process-wide `MathFont` from `assets/fonts/STIXTwoMath-Regular.otf` (face 0, `OnceLock`). `LayoutOptions.font_size` is the body size in points. Display versus inline comes from the MathML `display` attribute, not a second font size.
4. Ferrite rasterizes the display list with `skrifa` 0.46 (unscaled outlines) and `tiny-skia`. `MathFont::outline` is private, so the crate draws outlines itself. MathML `mathcolor` is ignored; the pixmap uses the preview text color. `mathbackground` fills are skipped.

`formulary` is `default-features = false` so the build does not pull a second `harfrust` or the SVG helper. Do not enable `shaping` or `svg` here, and do not bump `rust-version`.

## Cache

Key: `(literal, display, font size in points, rgba, pixels_per_point quantized to 0.25)`.

- Prepare cache: MathML or the convert error, keyed by `(literal, display)`.
- Raster cache: pixmap, cap 256, LRU. A hit does not call `formulary::layout`.
- Texture cache: egui `TextureHandle`, same cap. A hit does not upload again.

Theme and zoom miss. A single raster over ~50 ms logs `warn!` once per process. A failing `(literal, display)` logs once, not once per frame.

## Placement

Inline math stays in the row. The row uses top alignment and pads text and the texture so baselines match the surrounding rendered font (`Glyph.pos.y` of `Mg`). The row grows for ascent and descent.

Display math ends the row, centers in the content width, and scales down uniformly when it is wider than the pane. The following inlines start on the next row. A paragraph that is only `$$...$$` and a `$$` in the middle of a sentence are both blocks. Table cells use the same rules and grow the row.

Clicking the paragraph, list item, heading, or cell edits the source, dollars included. Preview lock paints the texture and does not enter edit. `render_math: false` (Editor settings, default true, `#[serde(default = "default_true")]`) paints the delimited source in `colors.quote_text` and does not call the converter. A convert failure uses that same color plus tooltip `markdown.math.render_failed`.

## Export

HTML (`generate_html_fragment` / `generate_html_document_export`) and PDF (`Renderer::render`) build their own comrak `Options` and leave `math_dollars` off. `$E = mc^2$` stays visible text. Do not share the preview options with export; a Math leaf would otherwise be dropped.
