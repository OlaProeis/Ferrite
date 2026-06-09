# Ferrite Roadmap

## Next Up (Immediate Focus)

### v0.3.1 - LSP, Embeds, HTML, Mermaid (Heavy), Multi-Window & CSV

**Status:** **Next up** after **v0.3.0** (May 22, 2026). PRD: [`docs/ai-workflow/prds/prd-v0.3.1.md`](docs/ai-workflow/prds/prd-v0.3.1.md). See [v0.3.1 plan](#v031---lsp-embeds-html-mermaid-heavy-multi-window--csv) below.

> **v0.3.0 (May 22, 2026)** — shipped: egui **0.34.2**, **Rust 1.92** MSRV, PDF + themed HTML export + print preview, executable code blocks, Mermaid first wave + FC-83a, **rendered edit session** (Tasks 94–105), split-view scroll sync, Ferrite accent, **Phosphor 0.12**, workspace file index, session recovery (Tasks 106 / 106.6), quick note workflow, Spanish UI, zero build warnings (Tasks 90–93). Full list: [CHANGELOG.md](CHANGELOG.md) § 0.3.0 and [Recently Completed](#v030-may-22-2026--platform-export-run-diagrams). Post-tag checklist: [release checklist](docs/github-release-checklist.md) (GitHub Release CI, macOS Gatekeeper blurb, Flathub PR). Open release gates: Wayland ([#106](https://github.com/OlaProeis/Ferrite/issues/106)) and Sonoma ([#111](https://github.com/OlaProeis/Ferrite/issues/111)) on real hardware.

> **v0.2.9 (Apr 2026)** was a hotfix release for four critical v0.2.8 regressions — see [Recently Completed](#recently-completed-).

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
- [ ] **Wayland keyboard input on Ubuntu 24.04** ([#106](https://github.com/OlaProeis/Ferrite/issues/106)) - **v0.3.0** ships **egui 0.34 / winit 0.31+**. **Release gate:** confirm on real Ubuntu 24.04 Wayland before closing #106; until then the workaround remains `WAYLAND_DISPLAY= ferrite` for 0.2.x builds.
- [ ] **macOS Sonoma keyboard input** ([#111](https://github.com/OlaProeis/Ferrite/issues/111)) - **v0.3.0** ships the 0.34 stack; **release gate:** verify on Sonoma hardware before closing #111.
- [x] **Windows 11 borderless window offset** ([#112](https://github.com/OlaProeis/Ferrite/issues/112)) - Fixed in v0.2.8 with `.with_transparent(true)` DWM workaround. v0.3.0 ships **egui 0.34.2** / winit 0.31+ stack (Task 38); close #112 after verification on target hardware.

### Terminal
- [x] **CJK double-width character overlap in terminal** ([#110](https://github.com/OlaProeis/Ferrite/issues/110)) - Fixed in v0.2.8. Added `unicode-width` crate, 2-column cursor advancement, wide char rendering spanning 2 cells.

### Rendered View Limitations
- [x] **Slow rendering on large documents** ([#105](https://github.com/OlaProeis/Ferrite/issues/105)) - Fixed in v0.2.8. AST caching, viewport culling, block height cache, and lazy estimation bring large-file rendered view to usable performance.
- [x] **Mermaid flowchart edges cross node boxes** ([#83](https://github.com/OlaProeis/Ferrite/issues/83), FC-83a) — **Landed for v0.3.0.** Obstacle-aware forward routing, orthogonal back-edge side channels at `BACK_EDGE_LOOP_MARGIN = 24 px`, painter sizing from actual node/subgraph bounds (no clipped loops), asymmetric back-edge padding (loop clearance only on the side that needs it), TD/BT layer centering on `max_cross_size` (fixes large left gap / right-shifted diagrams in wide containers), parallel back-edge lanes (`E → B` and `F → B` no longer merge), inner `E → B` exits top-outer corner and rises vertically along the source edge before entering Preview at side-centre, and `{decide}` snaps under Preview via alone-on-layer barycenter shift. Same-layer sibling overlap (coffee-machine `C/H`, `D/G`) fixed via `resolve_layer_overlaps` safety net. Docs: [`flowchart-edge-obstacle-routing.md`](docs/technical/mermaid/flowchart-edge-obstacle-routing.md), [`flowchart-layout-algorithm.md`](docs/technical/mermaid/flowchart-layout-algorithm.md). **FC-83b** (`fa:…` Font Awesome labels) and `linkStyle interpolate basis` curves remain open — see parity matrix.
- [x] **Double-click / stuck edit in rendered WYSIWYG** — **Shipped in v0.3.0** (Tasks 94–105). Consolidated `RenderedEditSession` coordinator: one-click block switching, stable `source_epoch` widget ids, formatted click-to-edit lifecycle, tables on session model, split-view parity, block-commit undo. Manual acceptance: RS-1…RS-7 in [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12. Hub: [`rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md).
- [x] **Task list checkbox scroll jump** — Toggling `- [ ]` / `- [x]` in rendered/split view no longer shifts scroll position when the user has not scrolled. Viewport culling reuses layout when block line ranges are unchanged; checkbox clicks no longer trigger scroll cooldown. See [`task-list-checkbox.md`](docs/technical/markdown/task-list-checkbox.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [ ] **Click-to-edit cursor drift on mixed-format lines** — v0.3.0 session + galley mapping (Task 100 follow-up) improved most cases; **residual 1–2 character offset** may remain on wrapped lines, links, and heavy inline nesting. **Fix: v0.3.1** — see [Rendered click-to-edit cursor precision](#rendered-view--click-to-edit-cursor-precision).

### Executable Code Blocks (v0.3.0)
Core Run (shell + Python, inline output, timeout, **Stop**) works for typical use; manual checklist: [`test_md/test_code_execution.md`](test_md/test_code_execution.md). Remaining edge cases (Windows `bash` without Git Bash, `sh`/`zsh` fallback, run state keyed by line number, copy/insert stderr format) are documented in [`code-block-run.md`](docs/technical/markdown/code-block-run.md) § Known limitations. **Fixes: v0.3.1** — see [Planned Features → v0.3.1 → Executable code blocks — hardening](#executable-code-blocks--hardening--polish).

---

## Planned Features

### v0.3.0 - Platform Refresh, Publish, Run, and Better Diagrams *(shipped May 22, 2026)*

**Theme:** Modernize the platform stack, ship first-class export, give code blocks a Run button, and finally deliver the long-promised Mermaid improvements. Full user-facing notes: [CHANGELOG.md](CHANGELOG.md) § 0.3.0.

**Four legs:**
1. **eframe / egui 0.34.2 migration** (Tasks 38, 57, 89) — 0.28 → 0.31 → 0.34; **MSRV Rust 1.92**; intended to address [#106](https://github.com/OlaProeis/Ferrite/issues/106), [#111](https://github.com/OlaProeis/Ferrite/issues/111), [#112](https://github.com/OlaProeis/Ferrite/issues/112) (confirm on target OS before closing).
2. **PDF + HTML export** — markdown becomes shareable, complementing the v0.2.8 PDF *viewer*.
3. **Executable code blocks** — **Run** for shell and Python, opt-in with security dialog.
4. **Mermaid improvements (first wave)** — diagram insertion toolbar, syntax hints, authoring validation, flowchart shapes, state diagram fork/join + history states.

*Scope discipline:* **v0.3.1** — LSP, YouTube/video embeds, GitHub HTML parity (Phases 1–2), Mermaid second wave, multi-window ([#125](https://github.com/OlaProeis/Ferrite/issues/125)), CSV rendered editing, and polish (cursor precision, code-run hardening, table alignment [#140](https://github.com/OlaProeis/Ferrite/issues/140)). **v0.3.2** — FerriteEditor crate extraction, Mermaid crate extraction, HTML Phase 3, additional file-format viewers. RTL/BiDi and LaTeX math are **v0.4.0**. Workarounds (e.g. `WAYLAND_DISPLAY=` on Ubuntu Wayland) remain documented until #106 / #111 are verified on the 0.34 stack.

#### Platform & Dependency Upgrade (Tasks 38, 57, 89)
- [x] **Bump eframe / egui to 0.31.1** (Task 57) — First step of the 0.28 → 0.34 path; breaking API changes fixed across `main.rs`, editor input, themes, terminal, markdown UI, etc. See [`docs/technical/platform/eframe-egui-031-upgrade.md`](docs/technical/platform/eframe-egui-031-upgrade.md).
- [x] **Bump eframe / egui to 0.34.2** (Task 89) — skrifa + vello_cpu default text backend; **Phosphor 0.12**; viewport rects (`screen_rect` → `viewport_rect` / `content_rect`); **Popup** / **Tooltip** API migrations; ScrollArea edge-fade disabled globally; HarfRust validation under 0.34; mutex/deadlock audit on cross-thread paths. See [`docs/technical/platform/eframe-egui-034-upgrade.md`](docs/technical/platform/eframe-egui-034-upgrade.md).
- [x] **MSRV raised to Rust 1.92** — `rust-toolchain.toml`, `package.rust-version`, CI workflows read pinned toolchain (required by egui 0.34).
- [x] **Regression pass** (Task 58) — matrix in [`docs/technical/platform/v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md); Windows manual pass complete including 0.34 delta (Tasks 89.3–89.7; I-3 smart-paste crash fixed; I-1 WIN-5 and I-2 TRM-3 resolved). macOS-AS / macOS-Intel / Linux-X11 / Linux-Wayland rows **deferred to CI / community**; **KBD-8 (Wayland)** and **KBD-9 (macOS Sonoma)** remain open until verified on real hardware.
- [ ] **Close or update** GitHub issues #106, #111, #112 once verified on the 0.34 stack.

#### Build hygiene — rustc warning cleanup (Tasks 90–93)
- [x] **Phase 1 — unused imports (Task 90)** — `cargo fix` plus manual pass in `app/`, `editor/ferrite/mod.rs`, `ui/mod.rs`; ~55 warnings cleared.
- [x] **Phase 2 — unused variables & assignments (Task 91)** — Central panel scroll-sync locals, title bar, Vim view params, markdown widgets, terminal panel; ~9 warnings cleared.
- [x] **Phase 3 — egui 0.34 deprecated APIs (Task 92)** — Mechanical migration across terminal panel, central panel, settings, and app shell (`close_menu` → `close`, `child_ui` → `new_child`, `ComboBox::from_id_salt`, `Button::selectable`, panel `show_inside`, `Frame::corner_radius`, etc.); ~152 warnings cleared.
- [x] **Phase 4 — dead_code audit (Task 93)** — Surgical removals, wiring fixes, and documented `#[allow(dead_code)]` for intentional public API; policy in [`CONTRIBUTING.md`](CONTRIBUTING.md); ~49 warnings cleared.
- **Result:** ~268 `cargo build` warnings after the egui 0.34 stack landed → **0** (Tasks 90–93). See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Changed.

#### PDF & Print Export
- [x] **PDF export** — Native Rust pipeline via **krilla** + **krilla-svg** (2-pass: layout + PDF). File → Export → PDF… with page size, margins, optional page break before H1. See [`docs/technical/viewers/pdf-export.md`](docs/technical/viewers/pdf-export.md), decision doc [`docs/technical/planning/pdf-export-pipeline.md`](docs/technical/planning/pdf-export-pipeline.md).
- [x] **Print preview** — Reuses export PDF path; opens temp PDF in **PdfViewer** tab. See [`docs/technical/viewers/print-preview.md`](docs/technical/viewers/print-preview.md).

#### HTML Export
- [x] **Themed / self-contained HTML** — Theme-aware export, Mermaid → SVG, syntect-styled code; see [`docs/technical/viewers/themed-html-export.md`](docs/technical/viewers/themed-html-export.md).
- [x] **Export options** — Dialog toggles (outline, comments, base path, theme choice, etc.) as implemented in [`src/export/html_options.rs`](src/export/html_options.rs) / export flow.

#### Executable Code Blocks
- [x] **Run button on code blocks** — Rendered/split preview; supported fenced languages (shell, Python). See [`docs/technical/markdown/code-block-run.md`](docs/technical/markdown/code-block-run.md).
- [x] **Shell / Python execution** — Background worker, **ANSI** output inline, exit status, insert-output helpers.
- [x] **Timeout handling** — Configurable; hard kill; **Stop** control. See [`docs/technical/markdown/code-block-cancellation.md`](docs/technical/markdown/code-block-cancellation.md).
- [x] **Security** — Opt-in master toggle; first-run **consent** dialog; per-language gates + timeout in Settings. See [`docs/technical/markdown/code-execution-consent-dialog.md`](docs/technical/markdown/code-execution-consent-dialog.md), [`docs/technical/config/code-execution-settings.md`](docs/technical/config/code-execution-settings.md).

#### Mermaid Improvements — First Wave ([#4](https://github.com/OlaProeis/Ferrite/issues/4))
- [x] **Diagram insertion toolbar** — Insert → Mermaid… templates; see [`docs/technical/markdown/mermaid-insert-toolbar.md`](docs/technical/markdown/mermaid-insert-toolbar.md).
- [x] **Syntax hints in Help panel** — F1 / About; see [`docs/technical/mermaid/mermaid-syntax-help.md`](docs/technical/mermaid/mermaid-syntax-help.md).
- [x] **Inline validation** — Warning header + editor squiggles; see [`docs/technical/mermaid/mermaid-inline-validation.md`](docs/technical/mermaid/mermaid-inline-validation.md).
- [x] **Flowchart enhancements** — Shapes + `style` / classDef; see [`docs/technical/mermaid/flowchart-shapes-and-style.md`](docs/technical/mermaid/flowchart-shapes-and-style.md).
- [x] **State diagram enhancements** — Fork/join + history pseudostates; see [`docs/technical/mermaid/state-pseudostates-fork-join-history.md`](docs/technical/mermaid/state-pseudostates-fork-join-history.md).
- [x] **Flowchart edge routing parity** ([#83](https://github.com/OlaProeis/Ferrite/issues/83), FC-83a) — Pre-tag rendering polish on native egui flowcharts (not mmdr/SVG). Repro: [`test_md/test_mermaid_issue_83.md`](test_md/test_mermaid_issue_83.md).
  - [x] Forward edges detour around node obstacles via `route_forward_edge` → `try_orthogonal_route` → `route_via_side_corridor`; helpers in [`flowchart/utils.rs`](src/markdown/mermaid/flowchart/utils.rs). Back-edges use fixed-margin side channels (not ±40 px beziers).
  - [x] Painter allocation from actual node bounds (`layout_content_size`) + asymmetric horizontal padding (`back_edge_horizontal_padding`) so feedback loops are not clipped without a spurious left gutter.
  - [x] TD/BT horizontal alignment — layers centered on `max_cross_size` (not `available_width`); post-layout bounds normalized to margin (fixes FC-83a right-shift / left gap in diagram frame).
  - [x] Parallel back-edge lanes (`BACK_EDGE_LANE_SPACING = 36 px`) — separate loops for `E → B` vs `F → B` via `compute_back_edge_lanes`.
  - [x] Inner `E → B` routing: top-outer-corner exit + vertical-first along the source's outer edge into Preview side-centre (`try_inner_back_edge_direct_path`). Pinned by `fc_83a_inner_e_to_b_goes_up_first`.
  - [x] Layout: snap `{decide}` to the children's barycenter when it is alone on its layer (`sugiyama::align_branch_nodes_to_children`); same-layer sibling overlap (e.g. coffee-machine `C/H`, `D/G`) cleared by the `resolve_layer_overlaps` safety net. Pinned by `test_layout_coffee_machine_all_nodes`.
  - [x] Manual matrix: FC-83a vs Mermaid Live; regression spot-check on `test_md/test_flowcharts.md`.
  - *Out of scope here:* `linkStyle interpolate basis` parsing (P2), Font Awesome labels (FC-83b). See [`docs/technical/mermaid/mermaid-parity-matrix.md`](docs/technical/mermaid/mermaid-parity-matrix.md).

#### Files & session
- [x] **Quick note workflow** (on by default, **Settings → Files**) — Modified untitled tabs no longer block quit; closing an individual untitled tab with content still prompts (Save / Don't save / Cancel); empty untitled tabs close silently; double-click tab to rename; session recovery keeps scratch buffers (clean exit preserves `recovery/` content). Turn off in settings for classic save prompts. See [`docs/technical/config/quick-note-workflow.md`](docs/technical/config/quick-note-workflow.md).
- [x] **Crash recovery + cold-start file open** — Double-click / “Open with” paths after a crash no longer lost when choosing **Restore session**; startup paths defer until the recovery dialog is answered, then open (dedupe by path). See [`docs/technical/files/session-persistence.md`](docs/technical/files/session-persistence.md).
- [x] **Hardened session recovery — identity gating + conflict banner** (Task 106) — Recovery (`recovery/<tab_id>.json`) and untitled autosave (`autosave/untitled_<tab_id>.md.autosave`) now require **path-and-disk-hash identity** before they are applied, closing a cross-tab data-loss hazard where a leftover file from a previous session (e.g. untitled `asdasd` for `tab_id=10`) could silently overwrite an unrelated path-backed tab assigned the same id (e.g. `task_50_table_inline_formatting.md`). `RecoveryContent` gains `path` + `original_content_hash` + `schema_version`; `AutoSaveMetadata` gains `disk_content_hash`; legacy files without these fields keep historical "tab id only" matching for upgraders via serde defaults. Identity mismatches are rejected and logged as `session_recovery_identity_mismatch` diag events. When identity matches but the recovered buffer differs from current disk, the central panel renders a non-blocking **Recovered content differs from this file on disk** banner above the editor with **Keep Recovered** / **Reload from Disk** actions (editing stays unblocked; closing the tab clears the conflict). Pruning extended to untitled autosave files. See [`docs/technical/files/session-persistence.md`](docs/technical/files/session-persistence.md) § Identity-Gated Recovery.
- [x] **Disk-hash anchoring across recovery cycles** (Task 106.6) — Critical data-loss follow-up to Task 106. After clicking **Restore**, the recovered tab was built via `Tab::with_file(path, recovered_content)`, which set `original_content = recovered`. That broke the disk anchor: `disk_content_hash()` returned `hash(recovered)`, the next crash snapshot wrote that wrong hash into the recovery file, and the *next* launch's Layer-3 identity check rejected the snapshot — silently discarding every edit made since the previous Restore (repro: edit → kill → restore → edit → kill → restore = jumps back to pre-first-edit). `restore_from_session_result` now constructs the tab from `on_disk_content` when the resolver returned `RecoveredWithDiskDivergence`, then swaps in the recovered buffer via `set_content`, so `original_content` stays anchored to disk and Ctrl+Z walks back to disk content as one undo step. Pinned by `test_restore_with_divergence_anchors_original_to_disk` and `test_restore_then_edit_keeps_disk_hash_anchor` in `src/state.rs`. See [`docs/technical/files/session-persistence.md`](docs/technical/files/session-persistence.md) § Disk-hash anchoring across recovery cycles.
- [x] **Workspace file index** — Ctrl+P and Ctrl+Shift+F search the **full workspace** via background `walkdir` (not limited to expanded sidebar folders). Incremental batches + progress UI on large trees; rebuild on create/delete/rename and tree refresh. See [`docs/technical/files/workspace-file-index.md`](docs/technical/files/workspace-file-index.md).

#### Localization
- [x] **Spanish UI language** — **Español** in Settings / Welcome selector; `locales/es.yaml`; `Language::Spanish` + `es` / `es-*` system locale detection.

#### Editor — scroll sync (v0.3.0)
- [x] **Split-view live sync** — Raw + rendered preview stay aligned while scrolling (markdown split only). Content-based **line + fraction** anchors (not scroll %); idle snap after ~120ms; wheel, scrollbar drag, and keyboard; hybrid top/bottom (5px) vs middle mapping. Controls on semantic minimap footer: **Sync** (master) and **2-way** (preview → raw, default on). Settings: `sync_scroll_enabled` (default off), `sync_scroll_bidirectional`. See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md).
- [x] **Mode-toggle sync (Ctrl+E)** — Re-enabled with same hybrid boundaries; Raw↔Rendered uses interpolated line mappings; stale pending scroll cleared when sync is off.
- [x] **Rendered scroll stability** — Height fixup when viewport culling remeasures blocks; no ghost snaps from pending offsets when sync is disabled.
- [x] **Task list checkbox scroll stability** — Checkbox toggles no longer invalidate viewport culling or suppress height fixup via pointer-down scroll cooldown; block line-range matching preserves layout on inline-only edits. See [`task-list-checkbox.md`](docs/technical/markdown/task-list-checkbox.md).
- [x] **2-way split sync top/bottom (rendered → raw)** — Top/bottom idle snaps from the preview pane now target the raw editor via `set_raw_target` (not `tab.pending_scroll_offset`, which only the preview reads). Fixes preview jumping up when scrolling to the document bottom with **2-way** enabled.

#### Rendered edit session (Tasks 94–105)
Consolidated WYSIWYG editing: one active block at a time, explicit `switch_to_ui` at click boundaries, stable widget ids, unified buffer-then-commit policy. Replaces fragmented `rendered_focus`, `FormattedItemEditState`, and ad-hoc table defer paths. Hub: [`rendered-edit-session.md`](docs/technical/markdown/rendered-edit-session.md).
- [x] **Phase 0 formatted blur hotfix (Task 94)** — Immediate save/exit on blur for formatted items; stops stuck raw `**bold**` TextEdit while session architecture lands.
- [x] **`source_epoch` + widget identity (Tasks 95, 97)** — External invalidation bumps epoch; rendered commits do not. `ui.push_id(editor_id + source_epoch)` replaces `content_hash` for TextEdit ids. See [`rendered-widget-identity.md`](docs/technical/markdown/rendered-widget-identity.md).
- [x] **`RenderedEditSession` core (Task 96)** — `BlockRef`, `BlockEditState`, `PendingActivation`, tab-scoped egui storage in `src/markdown/rendered_session.rs`.
- [x] **Headings on session (Task 98)** — `switch_to_ui`, buffer commit, one-click cross-heading switch; heading-specific `rendered_focus` hacks removed. See [`rendered-edit-session-headings.md`](docs/technical/markdown/rendered-edit-session-headings.md).
- [x] **Paragraphs & list items (Task 99)** — Plain blocks on session; epoch invalidation clears buffers. See [`rendered-edit-session-paragraphs-lists.md`](docs/technical/markdown/rendered-edit-session-paragraphs-lists.md).
- [x] **Formatted blocks (Task 100)** — Click-to-edit with galley cursor mapping; `formatted_editing` flag; Escape discard; `FormattedItemEditState` removed. See [`rendered-edit-session-formatted.md`](docs/technical/markdown/rendered-edit-session-formatted.md).
- [x] **Tables on session (Task 101)** — `BlockRef::TableCell`; `signal_table_force_commit` on leave; Tab nav preserves deferred table commit. See [`rendered-edit-session-tables.md`](docs/technical/markdown/rendered-edit-session-tables.md).
- [x] **Split-view parity (Task 102)** — Shared `rendered_editor_id(tab.id)` and session; raw-pane epoch bumps invalidate rendered buffers (RS-6). See [`rendered-edit-session-split-view.md`](docs/technical/markdown/rendered-edit-session-split-view.md).
- [x] **Block-commit undo (Task 103)** — One logical undo step per block commit via `rendered_commit_undo.rs`. See [`rendered-edit-session-undo.md`](docs/technical/markdown/rendered-edit-session-undo.md).
- [x] **Legacy cleanup (Task 104)** — `rendered_focus.rs` and dead focus/commit paths removed.
- [x] **Docs & RS regression matrix (Task 105)** — RS-1…RS-7 + TBLE-1…TBLE-3 in [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12; related docs updated (`wysiwyg-editor.md`, `click-to-edit-formatting.md`, table focus docs).

#### Bugfixes & polish (v0.3.0 bucket)
- [x] **Smart-paste mixed-script crash** (I-3) — Pasting text whose first colon precedes a multi-byte UTF-8 codepoint no longer aborts via `is_url` non-char-boundary slice; `s.get(colon_pos..colon_pos + 3)` guard. See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Consecutive fenced code blocks** ([#129](https://github.com/OlaProeis/Ferrite/issues/129)) — Split/rendered view visibility; see [`docs/technical/markdown/consecutive-fenced-blocks-fix.md`](docs/technical/markdown/consecutive-fenced-blocks-fix.md).
- [x] **Empty table cells: click-to-edit & tab focus** ([#131](https://github.com/OlaProeis/Ferrite/issues/131)) — Hit targets and focus; see [`docs/technical/markdown/table-cell-focus-navigation.md`](docs/technical/markdown/table-cell-focus-navigation.md).
- [x] **Table cell focus after editing (same table & cross-table)** — One-click move to another cell in rendered/split view after typing; session-integrated `BlockRef::TableCell` + `signal_table_force_commit` on cross-block exit (supersedes earlier `TableGlobalFocus` defer). See [`table-cell-focus-navigation.md`](docs/technical/markdown/table-cell-focus-navigation.md), [`rendered-edit-session-tables.md`](docs/technical/markdown/rendered-edit-session-tables.md).
- [x] **macOS Gatekeeper / unsigned .app** ([#130](https://github.com/OlaProeis/Ferrite/issues/130)) — User-facing workaround docs: [`docs/install/macos.md`](docs/install/macos.md), release checklist notes.
- [x] **Custom font picker error on Intel macOS** ([#133](https://github.com/OlaProeis/Ferrite/issues/133)) — Deferred load / toast fix; see [`docs/technical/fonts/custom-font-picker-deferred-load.md`](docs/technical/fonts/custom-font-picker-deferred-load.md).
- [x] **Frontmatter panel stale on tab switch** — Per-frame cache regression introduced during v0.3.0 perf pass. `update_from_content_versioned` keyed on `content_version` alone, which collides across tabs (each tab's counter starts at `0`); the panel showed "No frontmatter detected" on files that had it, kept the previous tab's fields visible after switching, and could splice the previous tab's body into the active file when **Add frontmatter** was clicked. Fixed by keying the cache on `(tab_id, content_version)`; regression tests added. See [`docs/technical/ui/frontmatter-panel.md`](docs/technical/ui/frontmatter-panel.md) (*Caching*).
- [x] **Ribbon toolbar icon-only** — Removed collapse/expand toggle and section labels; fixed 28px icon bar. Save/Export menus unchanged. See [`src/ui/ribbon.rs`](src/ui/ribbon.rs).
- [x] **Export menu double icons** — Locale strings still contained emoji while the ribbon already prefixed Phosphor icons (PDF, Print preview); stripped duplicate glyphs from export menu labels in all locale files. See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Outline panel tab selector hit-testing** — Phosphor icons in tab labels broke click targets when allocated via nested `allocate_ui_at_rect`; fixed with painter-based dual-font tab labels (`paint_tab_label`). See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Undo granularity (raw typing)** — Removed 500 ms time-merge in `EditHistory`; each recorded diff is its own undo step so Ctrl+Z no longer reverts an entire fast-typing burst. Rendered mode: one undo step per block commit (Task 103). See [`docs/technical/editor/undo-redo.md`](docs/technical/editor/undo-redo.md), [`rendered-edit-session-undo.md`](docs/technical/markdown/rendered-edit-session-undo.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Changed.
- [x] **Quick file switcher (Ctrl+P) search** — Token-based matching on `-` / `_` / path separators; search pool = **full workspace index** + recent files; match quality no longer drowned by recent-file boost or full-path subsequence noise. See [`src/ui/quick_switcher.rs`](src/ui/quick_switcher.rs), [`docs/technical/files/workspace-file-index.md`](docs/technical/files/workspace-file-index.md).
- [x] **Quick note: save prompt on tab close** — Modified untitled tabs with content again show the unsaved-changes dialog when closed (× / Ctrl+W); app exit still skips the dialog when quick-note workflow is on. `SavePromptContext` splits tab-close vs quit logic in `Tab::should_prompt_to_save`. See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Per-document view mode persistence** — Raw / Split / Rendered (and split ratio) for a file are restored when reopening after tab close or across restarts; global **Default view mode** applies only to files with no saved entry. `last_open_tabs` upsert on close + merge on save; `open_file` / background load read saved prefs. See [`docs/technical/view-mode-persistence.md`](docs/technical/view-mode-persistence.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Document nav buttons above modal overlays** — Scroll-to top/middle/bottom controls no longer paint on top of Quick File Switcher, command palette, or Search in Files; nav buttons use `Order::Middle` and are hidden while those panels are open. See `src/ui/nav_buttons.rs`, [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Status-bar Help vs bottom-right resize** (I-1, WIN-5) — Resize click guards in `src/ui/window.rs`; see [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Terminal CJK paste / input local-echo** (I-2, TRM-3) — Prompt-line `????` on pasted/typed CJK fixed by UTF-8 shell init at spawn (`src/terminal/pty.rs`) plus lazy CJK font load on terminal input; new tabs only. See [`docs/technical/terminal/terminal-cjk-wide-chars.md`](docs/technical/terminal/terminal-cjk-wide-chars.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Mermaid flowchart horizontal alignment (FC-83a)** ([#83](https://github.com/OlaProeis/Ferrite/issues/83)) — TD/BT diagrams with back-edges no longer render with a large empty strip on the left; Sugiyama centers layers on `max_cross_size`, layout bounds normalize to margin, back-edge painter padding is side-specific. Repro: [`test_md/test_mermaid_issue_83.md`](test_md/test_mermaid_issue_83.md). See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Multi-cursor copy / cut** — Copy and cut with multiple selections now put every selected range on the clipboard (newline-separated), not only the primary cursor. See [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **CSV rendered view cell overflow** — Long values (e.g. wide numeric strings) no longer spill into adjacent columns or past the table edge; `truncate_cell_to_pixel_width()` + painter clip rect in `render_row_cells`, fixed-width column layout unchanged. Hover tooltips for truncated cells. See [`docs/technical/viewers/csv-viewer.md`](docs/technical/viewers/csv-viewer.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Split 2-way sync: preview jump at bottom** — Rendered→raw top/bottom idle snap used `tab.pending_scroll_offset` (preview channel); preview snapped upward when raw max scroll was applied. Fixed via `SyncScrollState::set_raw_target` for raw top/bottom. See [`docs/technical/sync-scrolling.md`](docs/technical/sync-scrolling.md) § Split-view scroll delivery, [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.
- [x] **Task list checkbox scroll jump** — Toggling checkboxes in rendered/split view no longer nudges scroll when the user has not scrolled. See [`docs/technical/markdown/task-list-checkbox.md`](docs/technical/markdown/task-list-checkbox.md), [CHANGELOG.md](CHANGELOG.md) § 0.3.0 Fixed.

#### UI iconography — Phosphor Icons
- [x] **Phosphor icon font integration** — Added `egui-phosphor = "0.12.0"` (egui 0.34); font registration in [`src/fonts.rs`](src/fonts.rs); helpers in [`src/ui/icons.rs`](src/ui/icons.rs) and re-exports in [`src/ui/phosphor_icons.rs`](src/ui/phosphor_icons.rs).
- [x] **App chrome migration** — Replaced emoji / ad-hoc Unicode glyphs with Phosphor across ribbon, format toolbar, outline & productivity panels, terminal, settings, about, command palette, quick switcher, file tree, status bar, dialogs, nav buttons, title bar, and tab close controls.
- [x] **Preview & data viewers** — Markdown widgets (tables, code **Run** / **Stop**, mermaid headers & warnings), tree viewer (expand/collapse, copy path), CSV row-count info, Gantt done markers, ER diagram PK/FK markers, editor gutter fold carets.
- [x] **Locale deduplication** — Removed emoji prefixes from strings where the UI now renders icons separately (`en`, `de`, `es`, `ja`, `zh_Hans` for tree viewer, outline stats, CSV headers).
- *Intentionally unchanged:* Git status badges in the file tree, markdown **callout** content emojis (document body, not chrome), view-mode **R/S/V** segment letters.

#### Appearance — Ferrite accent color
- [x] **User-configurable accent** — Color picker in **Settings → Appearance** and on the **Welcome** screen; persisted as `Settings.accent_color`. Replaces the default blue for rendered **H1–H6**, editor **selection** tint, **tabs/active controls**, **view mode segment** (R/S/V), **outline/productivity** highlights, **Productivity Hub** (Add / Start work / notes ➕ / dock-detach), and **status bar** (LSP line, git branch). **Hyperlinks** in rendered markdown stay classic blue (`theme::accent::standard_link_color`). Documented in [`docs/technical/ui/theme-system.md`](docs/technical/ui/theme-system.md) (section *Ferrite accent color*).

#### UX & Polish — Productivity Hub Refresh
- [x] **Hub visual redesign** - Card-based layout matching the rest of the app's design language; themed colors derived from `ui.visuals()`, bordered priority chips, prominent centered Pomodoro timer (34 px monospace), reorder/delete affordances that recede until hover.
- [x] **`×` re-docks instead of hiding** - Closing the floating Productivity Hub via the title-bar `×` now routes back to the docked sidebar tab (mirrors the explicit `⤵ Dock` button), so the panel can never become unreachable mid-session without a restart or hotkey.
- [x] **Stable docked panel resizing** - Productivity Hub no longer auto-expands the sidebar or "snaps back" when the user drags the resize handle. Root cause was egui's `SidePanel::PanelState` storing the content's `min_rect`, so any wide widget permanently grew the panel. Fix: lock the outer footprint via `allocate_exact_size` and render content inside a clipped `child_ui` whose allocations don't propagate to the parent.
- [x] **Detached window stops auto-growing** - Floating Productivity Hub opens at the current dock width (`default_size`); wide content is clipped inside a scrollable `child_ui` so `Resize` cannot grow the window each frame; user width/height limits are viewport-based only (removed the old 560px max-width floor that snapped the panel back when dragged narrow).
- [x] **Search in Files — snappy panel size** - Ctrl+Shift+F no longer animates vertically to near full-window height as matches load; results scroll inside a fixed region (max 480px, default 320px); window fade disabled. Same root cause as the hub: egui `desired_size = max(desired, last_content_size)`.
- [x] **Sidebar scrollbar/resize cursor flicker** - Increased `style.spacing.scroll.bar_outer_margin` to 6 px so vertical scrollbars no longer overlap the side panel resize hit zone (fixes the rapid cursor flicker between resize and normal pointer at the sidebar edge).
- [ ] **Productivity Hub — native OS pop-out window** - Detached hub remains an in-app `egui::Window` (cannot move onto a second monitor outside Ferrite). Follow-up: second viewport like integrated terminal pop-out (`show_viewport_immediate`); ships with [Multi-window & viewports](#multi-window--viewports-github-125) in v0.3.1 if capacity allows.

---

### v0.3.1 - LSP, Embeds, HTML, Mermaid (Heavy), Multi-Window & CSV

**Theme:** Ship LSP for real, land exploratory video embeds, reach GitHub-style HTML parity (early phases), tackle the Mermaid items that need real engineering effort, enable **multi-window** document comparison, and improve data-viewer / table UX (CSV editing, GFM column alignment).

**GitHub issues tagged `0.3.1`:** [#119](https://github.com/OlaProeis/Ferrite/issues/119) (embeds), [#125](https://github.com/OlaProeis/Ferrite/issues/125) (multi-window), [#140](https://github.com/OlaProeis/Ferrite/issues/140) (table alignment), [#135](https://github.com/OlaProeis/Ferrite/issues/135) (file tree polish), [#115](https://github.com/OlaProeis/Ferrite/issues/115) (native decorations — optional).

#### Platform verification (v0.3.0 release gates)
- [ ] **Close or update** [#106](https://github.com/OlaProeis/Ferrite/issues/106) (Ubuntu 24.04 Wayland keyboard) once verified on real hardware with egui 0.34.
- [ ] **Close or update** [#111](https://github.com/OlaProeis/Ferrite/issues/111) (macOS Sonoma keyboard) once verified on Sonoma hardware.
- [ ] **Close or update** [#112](https://github.com/OlaProeis/Ferrite/issues/112) (Windows borderless offset) once verified on target hardware.
- [ ] Re-run [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) on any platform where input/window behavior changed.

#### LSP Integration (All 4 Phases) — Drop the feature flag
*Deferred from v0.2.8: Phase 1–2 implementation had high memory usage (rust-analyzer ~3.8 GB) and no diagnostics panel to surface warnings. Code remains in-tree behind the `lsp` feature flag; this release fixes it and ships it.*

- [ ] **Phase 1 fixes: Infrastructure & lifecycle** - Fix unbounded channels (add backpressure), clear diagnostics on workspace switch, cap transport frame size, properly join reader threads on shutdown.
- [ ] **Phase 1 fix: Incremental document sync** - Switch from full-document `didChange` to `TextDocumentSyncKind::Incremental` to reduce memory churn.
- [ ] **Phase 2 fix: Diagnostics panel** - Dedicated problems panel with click-to-navigate (bare minimum for LSP to be useful). Fix UTF-16→char column conversion for squiggle accuracy.
- [ ] **Phase 2 fix: Memory** - Stop per-frame diagnostic cloning (`Arc<Vec<DiagnosticEntry>>`), bounded event channels, `DiagnosticMap` cleanup on workspace switch.
- [ ] **Phase 3: Hover & Go to Definition** - Hover documentation with configurable delay; Go to Definition (F12 or Ctrl+Click).
- [ ] **Phase 4: Autocomplete** - Completion popup on typing or Ctrl+Space, debounced (e.g. 150ms), navigable with arrow keys; request cancellation for stale completions.
- [ ] **Settings** - Per-language server path override; all processing local (no network calls).
- [ ] **Drop `lsp` Cargo feature flag** - LSP becomes a default feature once Phases 1–2 are field-tested.

#### Embedded Media — YouTube / Video Embeds ([#119](https://github.com/OlaProeis/Ferrite/issues/119))
*Depends on: stabilized eframe/egui 0.31+ (Task 38) for reliable `RawWindowHandle` access.*

- [ ] **Custom syntax detection** - Detect YouTube/video URLs in markdown (e.g. `{{video URL}}` or bare YouTube URLs in their own paragraph) in `markdown/parser.rs`.
- [ ] **Embedded web view via `wry`** - Use Tauri's [`wry`](https://lib.rs/crates/wry) crate to spawn a platform-native WebView (WebView2 on Windows, WebKitGTK on Linux, WebKit on macOS) as a child window positioned over the egui rendered view.
- [ ] **Viewport tracking** - Sync the child WebView position/size with the egui rect each frame; hide when scrolled off-screen or tab is inactive.
- [ ] **Fallback: thumbnail + open-in-browser** - For platforms where `wry` child windows aren't viable, fetch YouTube thumbnail (`img.youtube.com`) and render as clickable image with play overlay; click opens system browser.
- [ ] **Extensible embed system** - Design the embed trait/interface to support future providers (Vimeo, etc.).

*Note: This is an exploratory feature. The `wry` child-window-over-egui approach has known challenges (z-ordering, scroll sync, platform quirks). The thumbnail fallback ensures the feature ships something usable regardless.*

#### HTML Rendering — GitHub Parity (Phase 1 & 2)
**Phase 1 – Block Elements**
- [ ] `<div align="...">`, `<details><summary>`, `<br>`

**Phase 2 – Inline Elements**
- [ ] `<kbd>`, `<sup>`, `<sub>`, `<img width/height>`

*Note: Safe subset only (no scripts, styles, iframes). Phase 3 (nested HTML, HTML tables) is in v0.3.2.*

#### Mermaid Improvements — Second Wave (Heavy)
- [ ] **Git Graph rewrite** - Horizontal timeline, branch lanes, and merge visualization ([#83](https://github.com/OlaProeis/Ferrite/issues/83) parity; see [`mermaid-parity-matrix.md`](docs/technical/mermaid/mermaid-parity-matrix.md)).
- [ ] **Evaluate `mermaid-rs-renderer` (mmdr) parser integration** - The [mmdr crate](https://github.com/1jehuang/mermaid-rs-renderer) supports 23 diagram types in pure Rust. Evaluate parser reuse while keeping native egui rendering (mmdr outputs SVG). Deliverable: decision doc + spike; unlocks v0.3.2 diagram types if adopted.
- [ ] **Manual layout support**
  - Comment-based position hints: `%% @pos <node_id> <x> <y>`
  - Drag-to-reposition in rendered view with source auto-update
  - Export option to strip layout hints ("Export clean")

#### Mermaid Improvements — Flowchart polish (post–FC-83a)
*Parser already accepts these; rendering gaps remain on common repros.*
- [ ] **Font Awesome / `fa:` icon prefixes in node labels** (FC-83b) — Strip gracefully or render placeholder; repro: [`test_md/test_mermaid_issue_83.md`](test_md/test_mermaid_issue_83.md).
- [ ] **`linkStyle` `interpolate basis`** — Document as unsupported or map to existing curved routing default.

#### Memory & Runtime — Loaded Modules Panel
*Context: CJK and complex-script fonts load lazily at first use but stay session-pinned (no unload today — same one-way atomic flags as v0.2.6 CJK lazy loading; LSP idle shutdown is the only existing “unload” pattern). Opening multi-script test files can add ~80 MB that persists after tab close. This panel makes that visible and optionally reversible.*

**Phase 1 — Stats tab visibility (read-only)**
- [ ] **Runtime section in Stats panel** — New block at the bottom of the right-side **Stats** tab (app-global, not per-document): which CJK families (KR/JP/SC/TC) and complex-script families (Arabic, Bengali, Devanagari, Thai, Hebrew, Tamil, Georgian, Armenian, Ethiopic, Other Indic, Southeast Asian) are loaded; Mermaid diagram cache size; LSP server status; terminal panel visibility / session count.
- [ ] **`RuntimeModulesInfo` snapshot** — Aggregate from `fonts::get_loaded_cjk_fonts()`, new `get_loaded_complex_script_fonts()`, `mermaid::get_cache_stats()`, LSP status map, terminal manager.

**Phase 2 — Manual unload controls (opt-in)**
- [ ] **Per-family font unload** — `unload_cjk_script` / `unload_complex_script` in `fonts.rs`: clear atomic flag, rebuild `FontDefinitions`, `bump_font_generation()`, invalidate shaped/line caches. Disable button when an open tab or UI language still needs that script; confirm dialog before unload (tofu until reload).
- [ ] **Service actions** — Clear Mermaid cache (`clear_diagram_cache()`), stop LSP server, close terminal panel / kill PTY sessions.
- [ ] **Docs** — Note that OS working set may not drop immediately (mimalloc); unload is best-effort for session RAM hygiene.

#### Data Viewers — CSV Rendered Editing
*Context: CSV/TSV **Rendered** view is read-only today (painted cells + tooltips); **Raw** mode already supports full text editing via FerriteEditor. Tree viewer and markdown `EditableTable` provide proven edit→serialize→`tab.content` + undo patterns; `csv` crate parsing is in-tree — writing back is the main gap.*

**MVP (v0.3.1)**
- [ ] **Cell value editing in Rendered view** — Double-click (or click-to-focus) a cell → inline `TextEdit` overlay; Enter commits, Escape cancels. Mirror `TreeViewer` integration: `&mut tab.content`, `prepare_undo_snapshot_hashed`, `output.changed` → undo stack.
- [ ] **CSV serialization** — `serialize_csv` via `csv::Writer` (RFC 4180 quoting, respect delimiter + header settings).
- [ ] **Small files only (<1 MB full-parse path)** — Edit against cached `CsvData`; invalidate Blake3-guarded caches on commit. Large lazy-parsed files show “edit in Raw view” (same class of banner as tree viewer).

**Follow-ups (same release if time, else v0.3.2)**
- [ ] **Tab / Shift+Tab between cells** — Reuse deferred-commit + `lock_focus` patterns from [`EditableTable`](docs/technical/markdown/editable-tables.md) / [`table-cell-focus-navigation.md`](docs/technical/markdown/table-cell-focus-navigation.md).
- [ ] **Add/remove rows & columns** — Toolbar controls; structural changes commit immediately.
- [ ] **Large-file rendered editing** — Row-level patch or load-on-first-edit; architectural follow-up.

Docs: extend [`docs/technical/viewers/csv-viewer.md`](docs/technical/viewers/csv-viewer.md) when implemented.

#### Rendered View — Click-to-Edit Cursor Precision
*Context: v0.3.0 shipped `RenderedEditSession` + single-galley display for formatted blocks (Task 100). Follow-up work in-tree improved bold/code/link mapping and wrap-width parity; placement is **good enough** for release but not pixel-perfect — e.g. ~1–2 character drift on wrapped lines or markdown links on long list items. Hub: [`rendered-edit-session-formatted.md`](docs/technical/markdown/rendered-edit-session-formatted.md), [`galley-cursor-positioning.md`](docs/technical/editor/galley-cursor-positioning.md).*

- [ ] **Unified layout source of truth** — Build display + hit-test `LayoutJob` from the same AST walk as `render_inline_node` (or shared helper), not a parallel raw-string parser; eliminates drift when parser and comrak disagree (links, wikilinks, nested emphasis).
- [ ] **Wrap-width & multi-line parity** — Persist per-block `layout_wrap_width` (and line height) on the session or egui temp store so click mapping never re-layouts with a different width than paint; add regression tests for wrapped list items and long paragraphs (RS-2 extension).
- [ ] **Link & wikilink edge cases** — Reference-style links, autolinks, nested `()` in URLs, `[[target|display]]`; optional thin link hit-target overlays if galley-only display must stay (trade-off: click link vs enter edit).
- [ ] **Delimiter coverage** — `_italic_`, `__bold__`, strikethrough/ code combinations in `parse_inline_markdown` or superseded AST path; match `map_displayed_to_raw` exactly.
- [ ] **Manual acceptance matrix** — Extend [`v0.3.0-regression-matrix.md`](docs/technical/platform/v0.3.0-regression-matrix.md) §3.12 / RS-2 with link-heavy and wrapped-line click targets; document known limits if any remain.

#### Editor — Raw Mode Table Column Alignment (display-only)
*Context: GFM pipe tables in **Raw** view are hard to scan when `|`, `**`, `~~`, and links shift columns. **Rendered** mode already has `EditableTable` with galley-based column widths on **stripped** cell text (`TableData`, `layout_no_wrap` in `widgets.rs`). This feature improves Raw readability **without mutating the file** — padding is visual only (or optional column guides), same performance tier as FerriteEditor (cache per table block, recompute on edit — not per-frame full-doc parse).*

**MVP (v0.3.1)**
- [ ] **Table block detection** — Line-based GFM table regions (header + `|---|` separator); skip fenced code; reuse outline-style heuristics; optional comrak table node on cache miss for ambiguous blocks.
- [ ] **Shared column width cache** — Per-tab cache keyed by `(start_line, content hash)`; measure **visible** cell width (strip inline markdown like rendered `serialize_inline_content`); invalidate only when that block changes.
- [ ] **Column guide overlay (phase 0)** — Faint vertical guides at computed column boundaries in Raw; file and cursor unchanged; very low CPU cost.
- [ ] **Visual pipe alignment (phase 1)** — Draw table rows as positioned segments (`|` + padded cells) using cached widths; rope buffer stays byte-identical; cursor/selection mapping for table lines (display ↔ raw).

**Follow-ups (v0.3.1 if time, else v0.3.2)**
- [ ] **Galley-accurate widths** — Match rendered table `layout_no_wrap` for proportional fonts (shared helper with `EditableTable`).
- [ ] **Split-view cache warming** — Reuse measured widths when rendered table for the same `start_line` is already laid out.
- [ ] **Multi-line / continuation rows** — Stress-test tables where one logical row spans multiple source lines (non-standard GFM).

*Out of scope:* writing padded spaces into the source (`TableData::to_markdown()`-style formatting on save); full WYSIWYG table grid in Raw (rendered `EditableTable` remains the edit surface for rich cells).

Docs: add `docs/technical/editor/raw-table-alignment.md` when implemented; link from [`docs/technical/editor/architecture.md`](docs/technical/editor/architecture.md).

#### Rendered Markdown Tables — GFM Column Alignment ([#140](https://github.com/OlaProeis/Ferrite/issues/140))
*Context: `TableAlignment` is parsed (`:---`, `:---:`, `---:`) and preserved in `TableData` / markdown export, but `EditableTable` forces `show_alignment_controls = false` and cell text is painted left-aligned only.*

- [ ] **Render left / center / right per column** — Apply alignment in `EditableTable` cell layout / galley (match GitHub rendered tables).
- [ ] **Enable alignment toolbar** — Wire `with_alignment_controls(true)`; cycle alignment per column (data model already has `cycle_column_alignment`).
- [ ] **Regression** — GitHub docs example table; round-trip through rendered edit session without losing alignment markers.

#### Multi-Window & Viewports ([#125](https://github.com/OlaProeis/Ferrite/issues/125))
*Context: Ferrite is [single-instance](docs/technical/platform/single-instance.md) today — a second launch forwards file paths to the primary window. Users want two markdown files visible at once (e.g. compare LLM outputs). egui 0.34 `Viewport` / `show_viewport_deferred` APIs are the preferred path post–v0.3.0.*

- [ ] **Design doc** — Multi-window vs in-app dual-document split; impact on `AppState`, active tab, undo, LSP workspace, and single-instance protocol (allow second window vs secondary instance opens new window).
- [ ] **Second OS window with independent tab strip** — New viewport hosting a subset of tabs or a cloned workspace; file open routes to focused window.
- [ ] **Cross-window file open** — Extend single-instance or use per-window listeners so Explorer “Open with” targets the right window when multiple are open.
- [ ] **QA matrix** — Windows, macOS, Linux X11 + Wayland; z-order, focus, and close semantics.

*Follow-up (if capacity):* Productivity Hub detached on second monitor via same viewport infrastructure.

#### Workspace UI — File Tree Polish ([#135](https://github.com/OlaProeis/Ferrite/issues/135))
- [ ] **Hover states** — Visual feedback on file/folder rows and icons in the sidebar tree.
- [ ] **Active file emphasis** — Clear focus/highlight for the tree row matching the active editor tab.

#### Window Chrome — Optional Native Decorations ([#115](https://github.com/OlaProeis/Ferrite/issues/115)) *(optional / defer if risky)*
- [ ] **Settings toggle** — “Use system title bar” (default off; keep current borderless custom chrome).
- [ ] **Platform behavior** — `with_decorations(true)` on Linux/macOS where requested; document interaction with custom resize/title-bar code on Windows.
- [ ] **KDE / Hyprland use case** — Respect WM button placement when native decorations are enabled.

#### Executable Code Blocks — Hardening & Polish
*Context: v0.3.0 shipped Run for shell + Python (opt-in, consent, inline ANSI output, timeout, **Stop**). Manual regression passed on Windows ([`test_md/test_code_execution.md`](test_md/test_code_execution.md)). Items below are edge cases and polish — not v0.3.0 blockers. Documented limitations: [`docs/technical/markdown/code-block-run.md`](docs/technical/markdown/code-block-run.md) § Known limitations.*

- [ ] **Windows `bash` / `shell` fence fallback** — When `bash` is not in PATH, stop reusing bash source for `.ps1` / `.bat` temp files; either fail fast with a clear message (“install Git Bash or use a `powershell` fence”) or translate/re-dispatch per interpreter.
- [ ] **`sh` / `zsh` interpreter fallback** — Extend `shell_interpreters` with a sensible platform chain (e.g. `sh` → `bash` on Windows; document Unix expectations for `zsh`).
- [ ] **Stable run-state identity** — Key inline output / `RunHandle` by block content hash or AST node id, not `start_line` alone, so edits above the fence do not orphan output.
- [ ] **Running-with-no-output UX** — Show a “waiting for output…” placeholder in the inline panel while `RunStatus::Running` and both streams are empty.
- [ ] **Copy / Insert stderr labelling** — Prefix stderr in clipboard and ` ```output ` insertion to match the on-screen `stderr` section (or offer a toggle).

#### Platform & Distribution
**Windows**
- [ ] **Inno Setup installer** - Alternative to MSI for users who prefer it; smaller download.

---

### v0.3.2 - FerriteEditor Crate, Mermaid Crate, GitHub HTML Phase 3 & Format Coverage

**Theme:** Extract the text editor and Mermaid renderer as reusable crates, fill in the GitHub HTML rendering tail, and broaden the file-type viewer set.

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
