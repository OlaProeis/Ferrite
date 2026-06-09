# Ferrite v0.3.0 — Platform Refresh, Publish, Run, and Better Diagrams

## 1. Overview

Ferrite is a cross-platform Rust + egui markdown editor. Version 0.2.9 (Apr 2026) shipped as a hotfix release; v0.3.0 is the next feature release.

The theme of v0.3.0 is **modernizing the platform stack and turning Ferrite from a *viewer/editor* into a *publisher*** — markdown becomes shareable as PDF and themed HTML, code blocks become executable, and the long-promised first wave of Mermaid improvements lands.

The release rests on **four legs**:

1. **eframe / egui 0.31+ migration** (the dependency upgrade Ferrite has been deferring since v0.2.8).
2. **PDF + themed HTML export** (markdown → publishable artifact).
3. **Executable code blocks** (a `▶ Run` button for shell and Python, opt-in with a security dialog).
4. **Mermaid improvements — first wave** (insertion toolbar, syntax hints, inline validation, flowchart shapes/styles, state-diagram fork/join + history states).

### Out of scope for v0.3.0 (explicitly deferred)

| Feature                                       | Target  |
|-----------------------------------------------|---------|
| LSP shipped without the feature flag          | v0.3.1  |
| YouTube / video embeds via `wry`              | v0.3.1  |
| GitHub HTML rendering parity (Phases 1–3)     | v0.3.1 / v0.3.2 |
| Mermaid Git Graph rewrite, mmdr integration, manual layout | v0.3.1 |
| Mermaid crate extraction                      | v0.3.2  |
| Additional file-format viewers (XML, INI, log) | v0.3.2 |
| RTL/BiDi (Phase 3 & 4)                        | v0.4.0  |
| LaTeX math rendering                          | v0.4.0  |
| Office documents (DOCX/XLSX/ODT)              | v0.4.0  |

The `WAYLAND_DISPLAY=` workaround for Ubuntu Wayland and other documented workarounds remain in place until v0.3.0 ships and is verified.

---

## 2. Goals

- **Stability:** Get off eframe/egui 0.28 (which is a year+ behind upstream and the root cause of three open input/window bugs) and onto 0.31+ with a clean cross-platform regression pass.
- **Publishing:** Make Ferrite produce shareable PDF and themed HTML artifacts that look like the in-app rendering.
- **Interactivity:** Add an opt-in "execute this code block" button with sane safety defaults.
- **Diagram authoring:** Reduce friction when creating Mermaid diagrams (toolbar, syntax help, inline validation) and broaden Flowchart / State diagram coverage.

## 3. Non-goals

- No new LSP features, no autocomplete, no Go-to-Definition (those are v0.3.1).
- No webview / iframe / video embed (v0.3.1).
- No HTML tag rendering beyond what comrak already does in the rendered view (v0.3.1+).
- No Mermaid Git Graph rewrite, no manual layout, no `mmdr` parser integration (v0.3.1).
- No new file-format viewers (v0.3.2).
- No LaTeX math, no RTL/BiDi (v0.4.0).
- No Mermaid crate extraction (v0.3.2).

## 4. Target platforms

Ferrite must build and run cleanly on:
- Windows 10 / 11 (MSI, portable, signed)
- macOS 12+ (Intel + Apple Silicon, .app bundle)
- Linux X11 (most distros)
- **Ubuntu 24.04 native Wayland** (no `WAYLAND_DISPLAY=` workaround required)
- Linux Wayland on GNOME / KDE / Hyprland / Sway (best effort, document portal requirements)

---

## 5. Functional requirements

### Leg 1 — Platform & dependency upgrade (closes #106, #111, #112)

#### 5.1 eframe / egui upgrade
- Bump `eframe` and `egui` (and any transitive crates we control: `egui_extras`, optional `egui-wgpu`) to the latest compatible 0.31+ release.
- Run `cargo update`; resolve any dependency conflicts (notably with `image`, `wgpu`, `winit`, `arboard`, `serde`).
- Fix all breaking API changes across the codebase. Expect changes in:
  - `src/main.rs` and `src/app.rs` — `App::update()` signature, `eframe::NativeOptions` / `ViewportBuilder` shape, `Frame` API.
  - `src/editor/widget.rs` and `src/editor/ferrite/*` — input event matching (`Key::*`, `Modifiers`, `PointerButton`), galley layout, `LayerId` / `Order`, IME output.
  - `src/markdown/*` (rendered view, mermaid renderers) — paint primitives (`Shape`, `Stroke`, `Color32`), text layout via `Galley`.
  - `src/terminal/*` — input handling (raw key events for terminal passthrough).
  - `src/theme/*`, `src/ui/*` — `Style`, `Visuals`, widget gallery API.
  - `src/platform/*` — title bar, window resize, custom decoration code (Windows borderless transparent fix may need to be re-applied).
- Re-validate any feature-flagged code paths (LSP behind `lsp` feature, async workers behind feature flags) still compile and run.
- Re-validate HarfRust shaped-text integration with the new galley/text layout.

#### 5.2 Cross-platform regression pass
A manual / scripted regression matrix must pass before release:

| Surface                              | Win 11 | macOS 14 | Linux X11 | Ubuntu 24.04 Wayland |
|--------------------------------------|--------|----------|-----------|----------------------|
| Launch + load file                   | ✓      | ✓        | ✓         | ✓                    |
| Keyboard input (English)             | ✓      | ✓        | ✓         | ✓ (no `WAYLAND_DISPLAY=`) |
| Modifier keys (Ctrl/Cmd, Shift, Alt) | ✓      | ✓        | ✓         | ✓                    |
| IME input (Chinese / Japanese / Korean) | ✓   | ✓        | ✓         | ✓                    |
| HarfRust shaped scripts (Arabic, Bengali, Devanagari) | ✓ | ✓ | ✓ | ✓ |
| Terminal input + ANSI rendering      | ✓      | ✓        | ✓         | ✓                    |
| File dialogs (rfd / portal)          | ✓      | ✓        | ✓         | ✓                    |
| Borderless title bar / window resize | ✓      | ✓        | ✓         | ✓                    |
| Custom font loading                  | ✓      | ✓        | ✓         | ✓                    |

#### 5.3 GitHub issue housekeeping
- Verify and close (or re-scope) issues #106, #111, #112.
- Update README / docs to remove the `WAYLAND_DISPLAY=` workaround.
- Update `docs/ai-context.md` "Tech Stack" line for the new egui version.

---

### Leg 2 — PDF & HTML export

#### 5.4 PDF export
- New menu/ribbon entry: **File → Export → PDF…**
- Output is the rendered view of the active markdown tab as a paginated PDF with sensible defaults:
  - Page size: A4 (default) or US Letter (auto-detect by locale).
  - Margins: 20mm top/bottom, 15mm left/right (configurable later).
  - Fonts embedded so PDF is self-contained.
  - Code blocks: monospace font, line wrapping, syntax highlighting preserved.
  - Tables: full borders, alternating row shading, page breaks at row boundaries when possible.
  - Images: scaled to fit page width.
  - Mermaid diagrams: rendered as vector (preferred) or high-DPI raster.
  - Page breaks before each `<h1>` (configurable).
- Implementation strategy (decision in design phase):
  - **Option A — HTML intermediate + Chromium / wkhtmltopdf headless print pipeline.** Highest fidelity, biggest dependency.
  - **Option B — Native Rust pipeline** (e.g. `printpdf`, `pdf-writer`, or composing via `hayro-write`). Smaller binary, more layout work for us.
  - **Option C — System print dialog on each OS.** Lowest effort but least consistent.
  - The PRD does not pre-commit to one; the implementation tasks must include a "spike + decision" task with explicit pros/cons, bundle-size impact, and a write-up.
- Errors must surface via the toast system; partial-output files must be cleaned up on failure.

#### 5.5 Print preview *(optional, ship if pipeline supports it)*
- New view mode or modal: **File → Print Preview**.
- Shows paginated output before export so users can spot bad page breaks.
- May simply be the PDF export pipeline rendered to an in-app PDF viewer tab (the v0.2.8 PDF viewer can be reused).

#### 5.6 Themed / self-contained HTML export
- Replace / improve the existing HTML export (`src/export/`) so the output is much closer to the in-app rendered view.
- Embedded CSS reflecting the user's current theme (light or dark). User can pick: **Auto (current theme)** / **Light** / **Dark**.
- Mermaid diagrams export as inline SVG (preferred) or rasterized PNG fallback when SVG isn't yet available for a diagram type.
- Code blocks: same syntax-highlighting palette as the in-app view.
- Tables: GFM styling, alternating rows, border consistent with rendered view.
- Self-contained option (default ON): embed images and stylesheets so the file works offline. Optional toggle to keep external asset references.
- Optional toggles in an export dialog:
  - Include outline / table of contents
  - Strip HTML comments
  - Set `<base>` path for relative asset references
  - Choose dark / light / current theme
- Output destination chosen via the existing rfd file dialog.

---

### Leg 3 — Executable code blocks

> Security note: code execution is opt-in. Default = **disabled**. First-run dialog explains the risk and lets the user enable it globally or per-tab.

#### 5.7 Settings & gating
- Add `enable_code_execution: bool` (default `false`) to `Settings`.
- Add to Settings UI under a new **Editor → Code Execution** section with a clear warning paragraph.
- Add a per-language allowlist (`allow_shell: bool`, `allow_python: bool`, default both `true` once master toggle is on).
- Configurable timeout in seconds (default `30`, max `300`).

#### 5.8 Run button on code blocks
- In rendered / split view, fenced code blocks of supported languages get a `▶ Run` button in their toolbar (same area as the existing "copy" button).
- Supported languages for v0.3.0: `bash`, `sh`, `shell`, `zsh`, `python`, `python3`, `py`.
- When code execution is disabled in settings, the button is hidden (not greyed out — stay clean).

#### 5.9 Execution backend
- Run via `std::process::Command` on a background thread (we already have a worker pattern in `src/workers/`).
- Shell: spawn the platform default (`bash` on Linux/macOS, `pwsh` falling back to `cmd` on Windows).
- Python: detect `python3` first, fall back to `python`. If neither is on `PATH`, surface a toast and disable the Python Run button for the rest of the session.
- Capture `stdout` and `stderr` separately. Combine into a single output panel below the code block.
- Exit code shown next to the panel (✓ for 0, ✗ otherwise).
- ANSI escape sequences in output should be rendered with color (reuse terminal renderer).
- Output is **transient by default** (not written into the markdown source). Optional "insert as fenced output block" action.

#### 5.10 Timeout & cancellation
- Hard timeout configured in 5.7 — the spawned process is killed when exceeded.
- Live "running" indicator with a Stop button while the process is alive.
- Output panel shows "Timed out after Ns" or "Stopped by user" when applicable.

#### 5.11 First-run security dialog
- The first time the user clicks `▶ Run` (or toggles the master switch in settings), show a modal dialog:
  - "Code execution lets Ferrite run code from your documents on your computer. This can damage your system or leak data if the code is malicious. Only enable this for documents you trust."
  - Buttons: **Enable & run**, **Just enable (don't run yet)**, **Cancel**.
- Dialog content must be i18n-friendly (`locales/en.yaml` keys).

---

### Leg 4 — Mermaid improvements (first wave) — issue #4

#### 5.12 Diagram insertion toolbar
- New ribbon / toolbar dropdown: **Insert → Mermaid…**
- Options for each currently-supported diagram type (flowchart, sequence, state, class, ER, pie, gantt, journey, mindmap, timeline, gitGraph if present).
- Each option inserts a fenced ```mermaid block with a minimal working template at the cursor.
- Templates live in `src/markdown/mermaid/templates.rs` (or similar) so they're easy to update.
- Available in raw, rendered, and split views.

#### 5.13 Syntax hints in Help panel
- New section in the About/Help special tab: **Mermaid syntax**.
- One subsection per diagram type with:
  - 1–2 line description
  - A working snippet (the same minimal template used by the insertion toolbar)
  - A link to the relevant page on `mermaid.js.org` for full syntax (opens in system browser)
- All strings i18n-keyed in `locales/en.yaml`.

#### 5.14 Mermaid authoring hints / inline validation
- Parse-time validation hooks into the existing Mermaid parser (`src/markdown/mermaid/`).
- When a Mermaid block has parse errors, the rendered view shows:
  - A yellow/red warning header inside the diagram area with the line number and error message.
  - The previously-successfully-rendered diagram is preserved if available (don't blank out on every keystroke).
- Inline validation hints in raw view: simple squiggle-style underline on the offending line in the editor (reuse LSP inline diagnostic plumbing if cheap; otherwise a lightweight Mermaid-only path is fine).
- Common-mistake hints (best effort): missing diagram-type header, unmatched brackets, unknown direction keyword, unknown shape suffix.

#### 5.15 Flowchart enhancements
- More node shapes — at minimum the following Mermaid-standard shapes that we don't yet render:
  - `[/Trapezoid/]`, `[\Trapezoid\]`
  - `(((Double-circle)))`
  - `[(Cylinder)]`
  - `((Circle))` (verify)
  - Hexagon / parallelogram completeness check
- `style` directive support: `style nodeId fill:#f9f,stroke:#333,stroke-width:2px,color:#000`. Per-node fill, stroke colour, stroke width, text colour.
- The existing `classDef` / `class` plumbing is the natural place to extend.
- Update `docs/technical/mermaid/` with the supported subset.

#### 5.16 State diagram enhancements
- Fork / join pseudostates: `state fork_state <<fork>>` and `state join_state <<join>>`. Render as a thick horizontal/vertical bar; multiple incoming/outgoing transitions group cleanly.
- Shallow history `[H]` and deep history `[H*]` pseudostates inside composite states. Render as a small circle with `H` or `H*` glyph.
- Update parser, layout, and renderer in the state-diagram modules.

---

## 6. Non-functional requirements

- **Performance:** No regression on the rendered-view performance budgets that were tightened in v0.2.8 (AST cache, viewport culling, block height cache, lazy estimation must continue to hold).
- **Memory:** No regression on the v0.2.6 baseline of ~100–150 MB idle.
- **Bundle size:** Total release binary should not grow by more than ~15 MB after eframe/egui upgrade and PDF export landing. If the chosen PDF backend is heavier, that's a release-blocker the team accepts knowingly.
- **Crash safety:** No new panics on the cross-platform regression matrix. PDF export and code execution must never panic the UI thread; failures are reported via toast.
- **Security:** Code execution is off by default; the first-run dialog is unskippable; processes always have a hard timeout.
- **Accessibility:** New UI surfaces (Run button, Insert Mermaid menu, Export dialog) are keyboard-reachable and respect the existing keyboard-shortcut customization system.
- **i18n:** All new user-facing strings go through `t!("...")` and have entries in `locales/en.yaml`.

## 7. Documentation deliverables

- `docs/technical/platform/eframe-egui-031-upgrade.md` — what changed, gotchas, the regression matrix table.
- `docs/technical/export/pdf-export.md` — chosen backend, options, known limitations.
- `docs/technical/export/html-export-themed.md` — what changed vs. the v0.2.x exporter.
- `docs/technical/markdown/code-execution.md` — security model, settings, supported languages.
- `docs/technical/mermaid/mermaid-insertion-toolbar.md` — toolbar UX and template list.
- `docs/technical/mermaid/mermaid-validation.md` — inline validation pipeline.
- `docs/technical/mermaid/flowchart-shapes-and-style.md` — new shapes, the `style` directive.
- `docs/technical/mermaid/state-diagram-fork-join-history.md` — new state-diagram features.
- Update `docs/index.md` to link all new docs.
- Update `docs/ai-context.md` only when the new egui version actually ships (per existing rule).
- Update `ROADMAP.md` "Recently Completed" section once v0.3.0 ships.

## 8. Acceptance criteria (release checklist)

The v0.3.0 release is ready when:

1. `cargo build --release` succeeds on Windows, macOS, Linux X11, and Ubuntu Wayland CI runners.
2. `cargo clippy -- -D warnings` is clean (or any new warnings are explicitly justified).
3. `cargo test` passes.
4. The cross-platform regression matrix in §5.2 is fully ✓.
5. GitHub issues #106, #111, #112 are verified fixed and either closed or commented with the fixing PR.
6. PDF export of a representative document (mixed text, tables, code, images, mermaid) produces a self-contained, openable PDF with no missing fonts or broken layout on all three OSes.
7. HTML export of the same document opens in Chrome, Firefox, and Safari, looking visually consistent with the in-app rendered view in the chosen theme.
8. Code execution: the security dialog appears on first run; toggling settings off hides the Run button; timeouts kill the child process; output panel renders ANSI colour.
9. All 5 Mermaid items (toolbar, help, validation, flowchart shapes/style, state fork/join + history) are reachable, documented, and produce visually correct output on the existing test corpus.
10. CHANGELOG.md and ROADMAP.md updated; Recently Completed entry written; release build artifacts produced by CI.

---

## 9. Migration / archive note for Task Master

The 56 tasks of v0.2.x development (44 done + 12 deferred) are preserved in the `v0-2-x-archive` tag. The deferred Code Execution group (#11–17) and the eframe upgrade (#38) from that archive are intentionally re-scoped and re-derived from this PRD instead of carried over by ID — the new tasks supersede them.
