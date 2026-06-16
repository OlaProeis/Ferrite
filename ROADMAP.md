# Ferrite Roadmap

Forward-looking plan for Ferrite — what we're building next. **Shipped releases:** [CHANGELOG.md](CHANGELOG.md) (**latest stable: [v0.3.0](CHANGELOG.md#030---2026-05-22)**, May 22, 2026). **v0.3.1** is complete on `0.3.1-experimental` — see [CHANGELOG § Unreleased](CHANGELOG.md#unreleased--v031) (awaiting tag).

---

## Recently Completed — v0.3.1

**Theme:** Mermaid wave 2, rich embeds, multi-window, data/table UX, GitHub HTML subset, and polish — **not LSP** (deferred to v0.3.2+).

**PRD:** [`docs/ai-workflow/prds/prd-v0.3.1.md`](docs/ai-workflow/prds/prd-v0.3.1.md) · **Changelog:** [CHANGELOG.md](CHANGELOG.md) § Unreleased

### Shipped highlights

| Area | What landed |
|------|-------------|
| **Mermaid** | Git graph rewrite, mmdr evaluation (partial-adopt), `@pos` manual layout, FC-83b / `linkStyle interpolate` |
| **Embeds** | YouTube wry WebView (custom-protocol relay page + iframe; Error 153 fix) + thumbnail fallback ([#119](https://github.com/OlaProeis/Ferrite/issues/119)); modal/overlay z-order occlusion; navigation allowlist + popup→browser hardening; primary-window-only playback (thumbnail fallback in secondary windows) |
| **Multi-window** | New Window, per-window tabs, session v2, focused-window routing ([#125](https://github.com/OlaProeis/Ferrite/issues/125)) |
| **Tables & CSV** | GFM column alignment ([#140](https://github.com/OlaProeis/Ferrite/issues/140)); CSV rendered cell editing MVP; raw-mode column guides |
| **GitHub HTML** | Phases 1–2 (`<div align>`, `<details>`, `<kbd>`, `<sup>`/`<sub>`, sized images); post-ship parser/render fixes (inline coalesce panic, single-line divs, center alignment, AST inline display). Fixture: [`test_md/test_github_html.md`](test_md/test_github_html.md) |
| **Editor UX** | Preview lock ([#144](https://github.com/OlaProeis/Ferrite/issues/144)); **Alt+Z** word wrap ([#145](https://github.com/OlaProeis/Ferrite/issues/145)); click-to-edit layout parity |
| **Community UI** | Tab strip right-click menu + path tooltip ([PR #150](https://github.com/OlaProeis/Ferrite/pull/150)); outline tree visual polish ([PR #151](https://github.com/OlaProeis/Ferrite/pull/151)); test build fixes ([PR #152](https://github.com/OlaProeis/Ferrite/pull/152)) — [@Star-sumi](https://github.com/Star-sumi) |
| **Files** | External open fallback ([#142](https://github.com/OlaProeis/Ferrite/issues/142)); file tree polish ([#135](https://github.com/OlaProeis/Ferrite/issues/135)) |
| **Code Run** | Shell dispatch hardening, blake3 run-state keying, waiting placeholder, stderr copy/insert parity |
| **Runtime visibility** | Stats tab runtime modules (Phase 1, read-only) |
| **Platform (Tier C)** | Optional native title bar ([#115](https://github.com/OlaProeis/Ferrite/issues/115)); Inno Setup installer (unsigned, MSI recommended) |
| **Fix** | Windows single-instance foreground ([#147](https://github.com/OlaProeis/Ferrite/issues/147), [PR #148](https://github.com/OlaProeis/Ferrite/pull/148)) |
| **Session / exit** | Don't Save on exit no longer resurrects discarded buffers on restart; Save-all on exit/window-close; autosave cleanup on discard; multi-window dialog cleanup fixes. See [`session-persistence.md`](docs/technical/files/session-persistence.md) |

**Explicitly deferred:** LSP integration (all phases) → **v0.3.2+** (remains behind the `lsp` Cargo feature flag). Tier C follow-ups (CSV Tab nav, Stats Phase 2, word-wrap toolbar icon, Mermaid drag-reposition, file-tree "Open with system default") → v0.3.2.

Full user-facing list: [CHANGELOG.md](CHANGELOG.md) § Unreleased.

---

## Known Issues

### FerriteEditor Limitations
With the v0.2.6 custom editor, most previous egui TextEdit limitations are resolved. Remaining issues:

- [x] **IME candidate box positioning** ([#15](https://github.com/OlaProeis/Ferrite/issues/15), [#103](https://github.com/OlaProeis/Ferrite/issues/103)) - Fixed in v0.2.8. Applied `layer_transform_to_global()` to IME coordinates.
- [x] **IME backspace deleting text** ([#91](https://github.com/OlaProeis/Ferrite/issues/91)) - Fixed in v0.2.7. Backspace during IME composition no longer deletes editor text.
- [ ] **Wrapped line scroll stuttering** - Scrolling through documents with many word-wrapped lines still shows micro-stuttering. Likely related to per-line galley layout cost or height cache granularity. Needs further investigation.

### Deferred
- [x] **Bidirectional scroll sync** — **Shipped in v0.3.0.** Split-view live sync with line+fraction anchors, idle snap (~120ms), top/bottom boundaries, minimap footer **Sync** / **2-way**, and mode-toggle (Ctrl+E) hybrid sync. See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md).
- [ ] **New file templates** - Optional frontmatter templates when creating new markdown files. Deferred from v0.2.7.

### Platform & Distribution
- [x] **macOS Gatekeeper blocking** ([#93](https://github.com/OlaProeis/Ferrite/issues/93)) - Fixed: CI now packages proper `.app` bundle via `cargo-bundle`.
- [ ] **macOS 15.x Gatekeeper on unsigned GitHub releases** ([#130](https://github.com/OlaProeis/Ferrite/issues/130)) - GitHub CI `.app` artifacts are **unsigned** (Apple Developer Program not planned). Users may need quarantine removal or **Open Anyway**. Documented: [`docs/install/macos.md`](docs/install/macos.md). Workaround docs remain the long-term approach.
- [ ] **Wayland keyboard input on Ubuntu 24.04** ([#106](https://github.com/OlaProeis/Ferrite/issues/106)) - **KBD-8** not yet verified on Ubuntu 24.04 Wayland (v0.3.1 matrix re-run); issue stays open. Workaround for affected builds: `WAYLAND_DISPLAY= ferrite`.
- [ ] **macOS Sonoma keyboard input** ([#111](https://github.com/OlaProeis/Ferrite/issues/111)) - **KBD-9** not yet verified on Sonoma hardware (v0.3.1 matrix re-run); issue stays open.
- [ ] **Windows 11 borderless window offset** ([#112](https://github.com/OlaProeis/Ferrite/issues/112)) - Fixed in v0.2.8 with `.with_transparent(true)` DWM workaround. **WIN-1…WIN-7** pass on Win11 with egui 0.34.2; **WIN-8** (Intel iGPU) row still open — close #112 after dedicated hardware retest.

### Terminal
- [x] **CJK double-width character overlap in terminal** ([#110](https://github.com/OlaProeis/Ferrite/issues/110)) - Fixed in v0.2.8. Added `unicode-width` crate, 2-column cursor advancement, wide char rendering spanning 2 cells.

### Rendered View Limitations
- [x] **Slow rendering on large documents** ([#105](https://github.com/OlaProeis/Ferrite/issues/105)) - Fixed in v0.2.8. AST caching, viewport culling, block height cache, and lazy estimation bring large-file rendered view to usable performance.
- [x] **Mermaid flowchart edges cross node boxes** ([#83](https://github.com/OlaProeis/Ferrite/issues/83), FC-83a) — **Landed for v0.3.0.** Obstacle-aware forward routing, orthogonal back-edge side channels at `BACK_EDGE_LOOP_MARGIN = 24 px`, painter sizing from actual node/subgraph bounds (no clipped loops), asymmetric back-edge padding (loop clearance only on the side that needs it), TD/BT layer centering on `max_cross_size` (fixes large left gap / right-shifted diagrams in wide containers), parallel back-edge lanes (`E → B` and `F → B` no longer merge), inner `E → B` exits top-outer corner and rises vertically along the source edge before entering Preview at side-centre, and `{decide}` snaps under Preview via alone-on-layer barycenter shift. Same-layer sibling overlap (coffee-machine `C/H`, `D/G`) fixed via `resolve_layer_overlaps` safety net. Docs: [`flowchart-edge-obstacle-routing.md`](docs/technical/mermaid/flowchart-edge-obstacle-routing.md), [`flowchart-layout-algorithm.md`](docs/technical/mermaid/flowchart-layout-algorithm.md). **FC-83b** and `linkStyle interpolate basis` shipped in v0.3.1 — see [`flowchart-fa-labels.md`](docs/technical/mermaid/flowchart-fa-labels.md), [`flowchart-linkstyle-interpolate.md`](docs/technical/mermaid/flowchart-linkstyle-interpolate.md).
- [x] **Double-click / stuck edit in rendered WYSIWYG** — **Shipped in v0.3.0** (Tasks 94–105). Consolidated `RenderedEditSession` coordinator: one-click block switching, stable `source_epoch` widget ids, formatted click-to-edit lifecycle, tables on session model, split-view parity, block-commit undo. Manual acceptance: RS-1…RS-7 in [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12. Hub: [`rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md).
- [x] **Task list checkbox scroll jump** — Toggling `- [ ]` / `- [x]` in rendered/split view no longer shifts scroll position when the user has not scrolled. Viewport culling reuses layout when block line ranges are unchanged; checkbox clicks no longer trigger scroll cooldown. See [`task-list-checkbox.md`](docs/technical/markdown/task-list-checkbox.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Click-to-edit cursor drift on mixed-format lines** — v0.3.1 shipped shared `FormattedBlockLayout` + per-block `layout_wrap_width` (Tasks 22–23). Residual edge cases on heavy inline nesting may remain; see [`rendered-edit-session-formatted-layout.md`](docs/technical/markdown/rendered-edit-session-formatted-layout.md).

### Executable Code Blocks (v0.3.0+)
Core Run (shell + Python, inline output, timeout, **Stop**) works for typical use; manual checklist: [`test_md/test_code_execution.md`](test_md/test_code_execution.md). v0.3.1 hardening (interpreter dispatch, blake3 run-state keying, waiting placeholder, stderr copy/insert parity) landed; see [`code-block-run.md`](docs/technical/markdown/code-block-run.md).

---

## Planned Features

### v0.3.1 follow-ups → v0.3.2

v0.3.1 scope is **complete** on `0.3.1-experimental` — see [Recently Completed — v0.3.1](#recently-completed--v031) and [CHANGELOG § Unreleased](CHANGELOG.md#unreleased--v031). Remaining items from the v0.3.1 PRD that did not ship:

#### Platform verification (carry-over)
- [ ] **Close** [#106](https://github.com/OlaProeis/Ferrite/issues/106) (Ubuntu 24.04 Wayland, KBD-8) once verified on real hardware.
- [ ] **Close** [#111](https://github.com/OlaProeis/Ferrite/issues/111) (macOS Sonoma, KBD-9) once verified on Sonoma hardware.
- [ ] **Close** [#112](https://github.com/OlaProeis/Ferrite/issues/112) (Windows borderless Intel iGPU, WIN-8) once verified on dedicated hardware.

#### CSV rendered editing follow-ups
- [ ] **Tab / Shift+Tab between cells** — Reuse deferred-commit + `lock_focus` patterns from [`EditableTable`](docs/technical/markdown/editable-tables.md).
- [ ] **Add/remove rows & columns** — Toolbar controls.
- [ ] **Large-file rendered editing** — Row-level patch or load-on-first-edit.

#### Mermaid follow-ups
- [ ] **Drag-to-reposition** in rendered view with `%% @pos` write-back to source.
- [ ] **Extensible embed system** — Future providers (Vimeo, etc.) beyond YouTube.

#### Memory & runtime — Stats panel Phase 2
- [ ] **Per-family font unload** and **Clear Mermaid cache** actions in the Stats tab. See [`stats-runtime-modules.md`](docs/technical/ui/stats-runtime-modules.md).

#### Editor UX (Tier C)
- [ ] **Word wrap toolbar/ribbon icon** ([#145](https://github.com/OlaProeis/Ferrite/issues/145)) — Alt+Z + command palette ship; icon deferred.
- [ ] **File tree context menu: Open with system default** ([#142](https://github.com/OlaProeis/Ferrite/issues/142)) — automatic fallback ships; explicit menu item deferred.

#### Raw table guides follow-ups
- [ ] **Galley-accurate widths** — Match rendered table `layout_no_wrap` for proportional fonts.
- [ ] **Split-view cache warming** — Reuse measured widths from rendered table layout.

---

### v0.3.2 - LSP, FerriteEditor Crate, Mermaid Crate, GitHub HTML Phase 3 & Format Coverage

**Theme:** Ship LSP for real (deferred from v0.3.1), extract the text editor and Mermaid renderer as reusable crates, fill in the GitHub HTML rendering tail, and broaden the file-type viewer set.

#### LSP Integration (All 4 Phases) — Drop the feature flag
*Deferred from v0.3.1: capacity reserved for Mermaid wave 2 and multi-window. Code remains in-tree behind the `lsp` feature flag. Target: **v0.3.2+**.*

- [ ] **Phase 1 fixes: Infrastructure & lifecycle** — Backpressure on channels, clear diagnostics on workspace switch, cap transport frame size, join reader threads on shutdown.
- [ ] **Phase 1 fix: Incremental document sync** — `TextDocumentSyncKind::Incremental` instead of full-document `didChange`.
- [ ] **Phase 2 fix: Diagnostics panel** — Problems panel with click-to-navigate; UTF-16→char column conversion for squiggles.
- [ ] **Phase 2 fix: Memory** — Stop per-frame diagnostic cloning; bounded event channels; `DiagnosticMap` cleanup on workspace switch.
- [ ] **Phase 3: Hover & Go to Definition** — Hover with configurable delay; F12 / Ctrl+Click go-to-def.
- [ ] **Phase 4: Autocomplete** — Completion popup (Ctrl+Space), debounced, cancellable requests.
- [ ] **Settings** — Per-language server path override; all processing local.
- [ ] **Drop `lsp` Cargo feature flag** — Default feature once Phases 1–2 are field-tested.

#### FerriteEditor Crate Extraction
*Deferred from v0.3.1 to avoid colliding with LSP integration and multi-window refactors. Raw / split-left editing uses `src/editor/ferrite/` (~14k lines); undo stays on app `Tab`.*

- [ ] **Cargo workspace** — New `ferrite-editor/` crate (path dep in Ferrite); move `src/editor/ferrite/` + minimal `EditorWidget` glue; keep Ferrite app as integration layer.
- [ ] **Decouple app types** — Trait or builder hooks for: font family / shaping bytes, syntax highlighting (syntect), fold state, theme colors; optional `lsp` feature for diagnostic squiggles.
- [ ] **Public API** — `FerriteEditor`, `TextBuffer`, `ViewState`, `LineCache`, `EditHistory` types; `ui()` entry point; feature flags (`vim`, `lsp`, `syntax`).
- [ ] **Examples & docs** — `examples/minimal.rs` (basic egui app); crate README + link from [`docs/technical/editor/architecture.md`](docs/technical/editor/architecture.md).
- [ ] **Regression pass** — Large files, word wrap, multi-cursor, IME/CJK/complex script, find/replace, code folding, bracket matching.

*Out of scope:* rendered WYSIWYG crate, headless/non-egui backends (see v0.4.0 long-term).

#### Mermaid Crate Extraction
- [ ] **Standalone crate** - Backend-agnostic architecture with SVG, PNG, and egui outputs.
- [ ] **Public API** - `parse()`, `layout()`, `render()` pipeline.
- [ ] **SVG export** - Generate valid SVG files from diagrams.
- [ ] **PNG export** - Rasterize via `resvg`.
- [ ] **WASM compatibility** - SVG backend usable in browsers.

#### Mermaid Improvements — Tail (mmdr-unlocked diagram types)
*Conditional on the v0.3.1 mmdr evaluation succeeding.*
- [ ] **New diagram types** (subset of: Sankey, Kanban, Quadrant, XY Chart, C4, Block, Architecture, Requirement, ZenUML, Packet, Radar, Treemap) — pick the most user-requested.

#### HTML Rendering — GitHub Parity (Phase 3)
- [ ] **Phase 3 – Advanced** - Nested HTML, HTML tables.

#### Additional Format Support

##### XML Tree Viewer
- [ ] **XML file support** - Open `.xml` files with syntax highlighting.
- [ ] **Tree view** - Reuse JSON/YAML tree viewer for hierarchical XML display.
- [ ] **Attribute display** - Show element attributes in tree nodes.

##### Configuration Files
- [ ] **INI / CONF / CFG support** - Parse and display `.ini`, `.conf`, `.cfg` files.
- [ ] **Java properties files** - Support for `.properties` files.
- [ ] **ENV files** - `.env` file support with optional secret masking.

##### Log File Viewing
- [ ] **Log file detection** - Recognize `.log` files and common log formats.
- [ ] **Level highlighting** - Color-code `ERROR`, `WARN`, `INFO`, `DEBUG`.
- [ ] **Timestamp recognition** - Highlight ISO timestamps and common date formats.

---

### v0.4.0 - Math, Complex Scripts, Office Documents

**Theme:** Three of the hardest text-rendering problems, taken seriously: native LaTeX math, full RTL/BiDi support, and "page-less" Office document viewing.

#### Math Rendering Engine
*Plan: parse via [`pulldown-latex`](https://crates.io/crates/pulldown-latex) (LaTeX → MathML, ~95% KaTeX-compatible, actively maintained); build the MathML→egui layout/render layer ourselves. Avoids reinventing the parser — lets us focus on TeX-style box layout and glyph metrics. See `docs/math-support-plan.md` for details.*

- [ ] **LaTeX parser integration** - Adopt `pulldown-latex` (or evaluate [`math-core`](https://github.com/tmke8/math-core)) for `$...$` inline and `$$...$$` display math.
- [ ] **MathML → egui layout engine** - TeX-style box model (fractions, radicals, scripts, large operators).
- [ ] **Math fonts** - Embedded glyph subset (Latin Modern Math or STIX) for consistent rendering.
- [ ] **egui integration** - Render in preview and split views; pick up math automatically in PDF/HTML export.

**Supported LaTeX (Target)**
- [ ] Fractions, subscripts/superscripts, Greek letters
- [ ] Operators (`\sum`, `\int`, `\prod`, `\lim`)
- [ ] Roots, delimiters, matrices
- [ ] Font styles (`\mathbf`, `\mathit`, `\mathrm`)

**WYSIWYG Features**
- [ ] Inline math preview while typing
- [ ] Click-to-edit rendered math
- [ ] Symbol palette

#### Unicode & Complex Script Support — Phase 3 & 4: RTL, BiDi, WYSIWYG
*Depends on: Phase 2 text shaping from v0.2.8. Full RTL+BiDi is one of the hardest problems in text editing; pairing it with the v0.4.0 "complex documents done right" theme rather than rushing it into v0.3.x.*

**Phase 3: Right-to-Left Layout & Bidirectional Text**
- [ ] **RTL text layout in FerriteEditor** - Render Arabic, Hebrew, and other RTL scripts right-to-left within lines. Shaped glyph runs are placed from the right edge; line alignment respects detected paragraph direction.
- [ ] **Unicode BiDi algorithm** - Implement the Unicode Bidirectional Algorithm (UAX #9) via the `unicode-bidi` crate for mixed-direction text (e.g., English embedded in Arabic). Resolves embedding levels, reorders glyph runs per line, and handles directional isolates/overrides.
- [ ] **RTL cursor navigation** - Arrow keys move in visual order (left arrow moves left visually, regardless of text direction). Home/End respect paragraph direction. Selection handles disjoint byte ranges in BiDi text.
- [ ] **RTL selection rendering** - Selection highlighting for BiDi text may produce multiple visual rectangles per logical selection range. Click-to-position respects visual glyph boundaries.
- [ ] **RTL line wrapping** - Word wrap respects script direction. Break opportunities follow UAX #14 (Unicode Line Breaking Algorithm) for correct behavior with Arabic, Hebrew, Thai, and other scripts.

**Phase 4: WYSIWYG & UI Chrome**
- [ ] **Shaped text in WYSIWYG editor** - Integrate text shaping into the rendered markdown view (`markdown/editor.rs`). RichText labels use shaped runs for correct Arabic/Bengali rendering in headings, paragraphs, lists, and tables.
- [ ] **Shaped text in Mermaid diagrams** - Update `TextMeasurer` to use shaped advance widths so diagram node labels render complex scripts correctly.
- [ ] **UI label shaping** - If egui has native shaping by this point (via Parley or direct HarfRust integration), adopt it. Otherwise, provide a shaping wrapper for critical UI surfaces (file tree, outline panel, status bar) where non-Latin file/heading names appear.

#### Office Document Support (Read‑Only)
**DOCX**
- [ ] Page-less rendering, text & tables, images
- [ ] Export DOCX → Markdown (lossy, with warnings)

**XLSX**
- [ ] Sheet selector, table rendering
- [ ] Basic number/date formatting
- [ ] Lazy loading for large sheets

**OpenDocument**
- [ ] ODT / ODS viewing with shared renderers

*FerriteEditor crate extraction is **v0.3.2** — see [FerriteEditor Crate Extraction](#ferriteeditor-crate-extraction).*

---

## Future & Long-Term Vision

### Linux Portable Packaging ([#146](https://github.com/OlaProeis/Ferrite/issues/146))
*Low priority.* Current GitHub releases (`.tar.gz`, `.deb`, `.rpm`) are built on Ubuntu 22.04 and require a matching host **glibc** — they fail on older distros (e.g. Debian 10), **musl** systems (Void), and some non-FHS setups. A fully **static** binary is not realistic for a GUI app (font rendering, OpenGL/glow, GTK dialogs, wry WebView).

- [ ] **Flathub (Flatpak)** — Primary long-term answer for “runs on most Linux setups”; see [`docs/linux-package-distribution-plan.md`](docs/linux-package-distribution-plan.md).
- [ ] **AnyLinux AppImage** — Evaluate [Anylinux-AppImages](https://github.com/pkgforge-dev/Anylinux-AppImages) / [sharun](https://github.com/VHSgunzo/sharun) (`quick-sharun.sh`): bundle glibc, dynamic linker, and all runtime libs (including `dlopen` deps) into a truly portable AppImage. Suggested by [#146](https://github.com/OlaProeis/Ferrite/issues/146) reporter; spike on Arch + test on Debian 10 / Void / NixOS. **Risk:** wry/webkit2gtk for YouTube embeds; expect larger artifact (~40–100+ MB). Not shipped today — `docs/building.md` AppImage recipe uses the older partial-bundle approach and CI does not build AppImage yet.
- [ ] **Older glibc rebuild** — Optional fallback (e.g. build on Ubuntu 18.04 for libc 2.27); partial fix only — does not cover musl/NixOS.

Until then: build from source ([`docs/building.md`](docs/building.md)) or Nix flake ([`flake.nix`](flake.nix)).

### Core Improvements
- [ ] **Persistent undo history** - Disk-backed, diff-based history.
- [ ] **Memory-mapped I/O** ([#19](https://github.com/OlaProeis/Ferrite/issues/19)) - GB-scale files.
- [ ] **TODO list UX** - Smarter cursor behavior in task lists.
- [ ] **Spell checking** - Custom dictionaries.
- [ ] **Custom themes** - Import/export.
- [ ] **Virtual/ghost text** - AI suggestions.
- [ ] **Column/box selection** - Rectangular selection.
- [ ] **Accessibility** - Full keyboard navigation for all menu items, screen reader support.

### Additional Document Formats (Candidates)
- [ ] **PDF viewing (read-only)** - Page-by-page PDF rendering via native library bindings (PDFium or MuPDF). Requires shipping platform-specific native libraries (~20MB per platform). Complex cross-compilation. Low priority — OS viewers handle this well.
- [ ] **Jupyter Notebooks (.ipynb)** - Read-only viewing of cells and outputs.
- [ ] **EPUB** - Page-less e-book reading with TOC and position memory.
- [ ] **LaTeX source (.tex)** - Syntax highlighting, math preview, outline.
- [ ] **Alternative Markup Languages** ([#21](https://github.com/OlaProeis/Ferrite/issues/21))
  - reStructuredText, Org-mode, AsciiDoc, Zim-Wiki
  - Auto-detection by extension/content

### Plugin System
- [ ] Plugin API & extension points
- [ ] Scripting (Lua / WASM / Rhai)
- [ ] Community plugin distribution

### Headless Editor Library
- [ ] Framework-agnostic core extraction
- [ ] Abstract rendering backends (egui, wgpu, SVG)
- [ ] Advanced text layout integration (HarfRust/skrifa, with Parley as future option)

**Note:** These are ideas under consideration.

---

## Recently Completed ✅

### v0.3.0 (May 22, 2026) — platform, export, run, diagrams
See **[0.3.0]** in [CHANGELOG.md](CHANGELOG.md) for the full user-facing list. Highlights:
- **eframe / egui 0.34.2** platform bump (Tasks 57–58, **89**; 0.31 → 0.34; **MSRV Rust 1.92**; skrifa text backend, Popup/Tooltip APIs, HarfRust validation; Windows 0.34 delta regression complete). See [`eframe-egui-034-upgrade.md`](docs/technical/platform/eframe-egui-034-upgrade.md).
- **Zero `cargo build` warnings** (Tasks 90–93: ~268 → 0).
- **PDF export** (krilla + krilla-svg) and **print preview** (temp PDF → viewer tab).
- **Themed HTML export** with options dialog and Mermaid as SVG.
- **Executable fenced code blocks** — Run, shell/Python, ANSI output, timeout + Stop, first-run consent, Settings (opt-in).
- **Quick note workflow** (on by default; quit without save dialog, tab close still prompts when modified) and **Spanish** UI language.
- **Mermaid first wave** — insert templates, F1 syntax help, inline validation, flowchart shapes/style, state fork/join + history.
- **Mermaid FC-83a ([#83](https://github.com/OlaProeis/Ferrite/issues/83))** — flowchart obstacle routing, back-edge side channels, parallel lanes, inner `E → B` path, branch-parent snap, TD/BT horizontal alignment fix (no left-gap / right-shift in wide containers); docs [`flowchart-edge-obstacle-routing.md`](docs/technical/mermaid/flowchart-edge-obstacle-routing.md), [`flowchart-layout-algorithm.md`](docs/technical/mermaid/flowchart-layout-algorithm.md). **Still open:** FC-83b Font Awesome labels, `linkStyle interpolate basis` curves (parity matrix).
- **Rendered edit session (Tasks 94–105)** — `RenderedEditSession` coordinator, `source_epoch` stable widget ids, one-click block switching (headings / paragraphs / lists / formatted / tables), split-view parity, block-commit undo; legacy `rendered_focus` removed. Docs: [`rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md); QA: RS-1…RS-7 in [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12.
- **Split-view scroll sync** — minimap footer **Sync** / **2-way**, content anchors, mode-toggle (Ctrl+E) preservation; docs [`sync-scrolling.md`](docs/technical/sync-scrolling.md).
- **Ferrite accent color** (Settings + Welcome) and **Productivity Hub** UI polish (dock/resize/scrollbar, snappy detached window).
- **Search in Files** — fixed-height panel; no content-driven vertical growth.
- **Workspace file index** — Ctrl+P and Ctrl+Shift+F search all files under the open folder (background walk + progress on large trees); see [`workspace-file-index.md`](docs/technical/files/workspace-file-index.md).
- **Phosphor Icons** (`egui-phosphor` **0.12.0**) — unified icon font across app chrome, preview widgets, and data viewers; locale strings deduplicated where icons are rendered in code.
- **Ribbon toolbar** — always icon-only (collapse toggle and section labels removed).
- **Undo granularity (raw mode)** — per-keystroke Ctrl+Z steps (500 ms merge removed); rendered mode one undo step per block commit (Task 103).
- **CSV rendered view** — pixel-width cell truncation so long values stay inside fixed columns (v0.3.0 fix).
- **Notable fixes:** smart-paste UTF-8 `is_url` panic (I-3), consecutive fenced blocks ([#129](https://github.com/OlaProeis/Ferrite/issues/129)), empty table cell hit-testing ([#131](https://github.com/OlaProeis/Ferrite/issues/131)), rendered WYSIWYG double-click / stuck edit (Tasks 94–105), table cell focus after typing (session model), **split 2-way sync bottom jump** (rendered→raw top/bottom delivery path), **task list checkbox scroll jump** (structure-preserving viewport culling), frontmatter panel stale on tab switch, export menu double icons, outline panel tab hit-testing, crash recovery + cold-start file open, **hardened session recovery** (Task 106 — identity gating + non-blocking conflict banner closes cross-tab data-loss hazard), **disk-hash anchoring across recovery cycles** (Task 106.6 — restored tabs anchor `original_content` to disk so the second restore no longer reverts to pre-first-edit), **workspace file index** (Ctrl+P / search in collapsed folders), quick file switcher (Ctrl+P) token/recent-file search, quick note save prompt on untitled tab close, per-document view mode restore on reopen, document nav buttons above modal overlays, status-bar Help vs resize corner (I-1), terminal CJK paste/input local-echo with spawn-time UTF-8 init (I-2), Search in Files / detached Productivity Hub panel growth & resize snap, multi-cursor copy/cut, CSV rendered view cell overflow, Mermaid flowchart horizontal alignment (FC-83a), Intel macOS font picker ([#133](https://github.com/OlaProeis/Ferrite/issues/133)), macOS Gatekeeper doc path ([#130](https://github.com/OlaProeis/Ferrite/issues/130)). Full list: [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.

### v0.2.9 (Apr 2026) - Hotfix Release
Hotfix for four critical v0.2.8 regressions. No new features.
- **Crash in Split / Rendered view on empty documents** ([#127](https://github.com/OlaProeis/Ferrite/issues/127)) — viewport-culling bootstrap indexed `doc.root.children[0]` when `block_count == 0`. Fixed with a half-open render range.
- **No unsaved-changes indicator (`*`) and no save prompt on close, causing silent data loss** — raw-mode edits bypassed `content_version`, so `is_modified()` stayed cached at `false`. `content_version` bumps centralized in `record_edit_from_snapshot()` / `set_content()`.
- **Undo / redo reporting "Nothing to undo" after typing** — FerriteEditor's internal edits were never diffed into `tab.edit_history`, which is the stack Ctrl+Z / Ctrl+Y read. Fixed by snapshotting pre-edit content and recording ops per dirty frame.
- **Selection invisible in Light mode** ([#121](https://github.com/OlaProeis/Ferrite/issues/121)) — 40% alpha made the pale light-theme selection blend into the panel. Alpha reduction is now dark-mode-only.
- **Document side panel tab labels overlapping at default width** — raised default outline panel width from 200 → 300 px, minimum from 120 → 260 px; existing users auto-migrated by settings validator.

### v0.2.8 (Apr 2026) - Performance, Text Shaping, LSP Integration & Viewers
Command Palette (Alt+Space) with fuzzy search across all actions. LSP integration (Phases 1-2): inline diagnostics, server lifecycle, status bar, on-demand startup. HarfRust text shaping for Arabic, Bengali, Devanagari, and other complex scripts. Image viewer tabs (PNG/JPEG/GIF/WebP/BMP) and PDF viewer tabs (hayro, pure Rust). Major rendered view performance overhaul: AST caching, viewport culling, block height cache, lazy estimation. Per-frame O(N) elimination for large files. Background file loading for 5MB+ files. Strict line breaks (Obsidian model). Middle-click to close tabs. CSV/TreeViewer/central panel per-frame allocation fixes. Table cell rich text rendering with click-to-edit (bold, italic, strikethrough, code, nesting). 13 bug fixes including macOS .md file association (#102), Windows IME positioning (#103), custom font crash on Linux (#114), Linux Cinnamon dialog detection (#116), table inline formatting preservation and rendering (#117), terminal CJK rendering (#110), Windows 11 borderless offset (#112), and more.

### v0.2.7 (Mar 2026) - Performance, Features & Polish
Wikilinks & backlinks, Vim mode, welcome view, GitHub-style callouts, check for updates, Ctrl+Scroll Wheel zoom, keep text selected after formatting, lazy CSV parsing, large file detection, single-instance protocol, MSI installer overhaul with optional file associations, PortableApps.com Format packaging with automated CI build, Nix/NixOS flake support, German and Japanese localization, Unicode complex script font loading (Phase 1: 11 script families, 22 Unicode ranges), complex script font preferences UI (Settings → Additional Scripts), visual frontmatter editor, format toolbar moved to editor bottom, side panel toggle strip, Linux file dialog error handling with portal failure detection, flowchart modular refactoring, window control redesign, macOS .app bundle CI, task list checkbox rendering, word-wrap scroll correctness & performance fixes, preview list item wrapping fix, false setext heading fix, IME backspace fix (#91), binary file crash fix, rendered mode copy spacing fix, 20+ bug fixes including light mode visibility, scrollbar accuracy, and crash on large selection delete.

### v0.2.6.1 (Released Feb 2026) - Terminal, Productivity Hub & Refactoring
**First code-signed release.** Integrated Terminal Workspace and Productivity Hub contributed by [@wolverin0](https://github.com/wolverin0) ([PR #74](https://github.com/OlaProeis/Ferrite/pull/74)) — the first major community contribution. Major app.rs refactoring into ~15 modules. 8+ bug fixes.

### v0.2.6 (Released Jan 2026) - Custom Text Editor
**The critical rewrite.** Replaced the default egui editor with a custom-built virtual scrolling editor engine.

* **Memory Fixed:**
* **Virtual Scrolling:** Only renders visible lines; massive performance boost.
* **Code Folding:** Visual collapse for code regions.
* **Editor Polish:** Word wrap, bracket matching, undo/redo, search highlights.

### Prior Releases
* **v0.2.5.x:** Syntax themes, Code signing prep, Multi-encoding support, Memory optimizations.
* **v0.2.5:** Mermaid modular refactor, CSV viewer, Semantic minimap.
* **v0.2.0:** Split view, Native Mermaid rendering.

> For detailed logs of all previous versions, see [CHANGELOG.md](CHANGELOG.md).
