# Ferrite v0.3.1 — Mermaid Wave 2, Embeds, Multi-Window, Data UX & Polish - AI Context

Ferrite: a Rust (edition 2021) + egui markdown editor. Immediate-mode GUI — no retained widget state, UI rebuilds every frame.

## Rules (DO NOT UPDATE)
- **Implementation sessions:** follow **Implementation Phase Rules** below only.
- **Update sessions:** follow **Update Phase Rules** below only when you receive the update handover prompt.
- Only do the task specified; do not start the next task or go over scope.
- Run `cargo test` after code changes to verify tests pass.
- Follow existing code patterns and conventions.
- Use Context7 MCP to fetch library documentation when needed (resolve library ID first, then fetch docs). Task operations use **`cyclopsctl tasks` CLI only**.

## Implementation Phase Rules
When working from **`current-handover-prompt.md`** (the normal case for every cyclopsctl task cycle):

- **DO:** Implement and test only the current parent task described in the handover.
- **DO:** Run `cargo test` before finishing; meet the task test strategy.
- **DO:** Use Context7 MCP for up-to-date library documentation when implementing unfamiliar APIs or frameworks.
- **DO NOT:** Read `prd.md` during cyclopsctl cycles — task scope, details, and test strategy are already in this handover.
- **DO NOT:** Mark tasks done or change task status.
- **DO NOT:** Run `cyclopsctl tasks next`, rewrite `current-handover-prompt.md`, or edit `ai-context.md`.
- **DO NOT:** Create or update docs in `docs/`, or edit `docs/index.md`.
- **DO NOT:** Edit `update-handover-prompt.md`.

Task completion and all documentation updates happen only in the **update phase** (`update-handover-prompt.md`).

## Update Phase Rules
When `update-handover-prompt.md` is provided (after implementation in the same agent session):

- **DO:** Follow every step in `update-handover-prompt.md`.
- **DO:** Use `cyclopsctl tasks list pending --project-root G:\DEV\markDownNotepad` and pick the **lowest numeric parent id** for the next handover — not `cyclopsctl tasks next` (priority can skip ahead).
- **DO:** Rewrite `current-handover-prompt.md` for the **next** task (this is the only time that file may change).
- **DO:** Update the **Project Memory** section below per update handover step 2 (key facts only, not a changelog).
- **DO:** Use `cyclopsctl tasks` with `--project-root G:\DEV\markDownNotepad` for all task commands (see Environment in the handover).
- **DO:** Document by feature (e.g., `auth-layer.md`), not by task number; update `docs/index.md` when adding documentation.
- **DO NOT:** Re-implement or extend the task you just finished unless tests are broken.

## Handover Files
| File | Who may edit | When |
|------|----------------|------|
| `current-handover-prompt.md` | Update-phase agent only | After implementation |
| `update-handover-prompt.md` | Human / template only | Never edited by agents |
| `ai-context.md` | Update-phase agent only | Every update phase — Project Memory bullets only |

## Tech Stack
- **Language:** Rust 2021 (MSRV **1.92**), egui **0.34.2** + eframe (glow on Windows)
- **Text:** ropey (rope buffer), comrak (Markdown AST), syntect (syntax highlighting), harfrust (OTL shaping)
- **Terminal:** portable-pty + vte | **VCS:** git2 | **Dialogs:** rfd | **i18n:** rust-i18n | **Hashing:** blake3 | **PDF read:** hayro | **PDF write:** krilla + krilla-svg
- **Memory:** mimalloc (Windows), jemalloc (Unix)

## Architecture
| Module | Purpose |
|--------|---------|
| `src/app/` | Main application (~15 modules: keyboard, file_ops, formatting, navigation, central_panel, …) |
| `src/state.rs` | All application state (`AppState`, `Tab`, `TabKind`, `SpecialTabKind`, `FileType`) |
| `src/editor/ferrite/` | Rope-based editor: buffer, cursor, history, view, rendering, line_cache |
| `src/editor/widget.rs` | EditorWidget wrapper, integrates FerriteEditor via egui memory |
| `src/markdown/` | Parsing (`parser.rs`), rendered view (`editor.rs`, `widgets.rs`), edit sessions, code execution, `mermaid/` |
| `src/terminal/` | Integrated terminal (PTY, VTE, screen, themes, split layouts) |
| `src/ui/` | Panels: ribbon, settings, file_tree, outline, search, terminal, productivity, frontmatter, welcome, command_palette; `action_registry.rs` (context-menu metadata) |
| `src/config/` | Settings persistence, session/crash recovery, snippets |
| `src/fonts.rs` | Font loading, lazy CJK, complex script lazy loading (11 families) |
| `src/theme/` | Light/dark themes, user accent color |
| `src/lsp/` | LSP integration (manager, transport, diagnostics) |
| `src/vcs/`, `src/workspaces/`, `src/export/`, `src/preview/`, `src/platform/` | Git, folder mode + file index, HTML/PDF export, sync scroll, platform-specific |

**FerriteEditor** (`src/editor/ferrite/`): rope-based, O(log n) ops, virtual scrolling, multi-cursor, code folding, IME/CJK. Docs: `docs/technical/editor/architecture.md`.

## Critical Patterns & Gotchas
```rust
// Line indices: always saturating math, be explicit about 0- vs 1-indexed
let idx = line_number.saturating_sub(1);
// Never unwrap in library code
if let Some(tab) = self.tabs.get_mut(self.active_tab) { ... }
```
- **Byte vs char index:** never slice `text[start..end]` with char positions — use byte offsets or `char_indices()`.
- **CPU spin:** use `request_repaint_after()` when idle, not unconditional `request_repaint()`.
- **Per-frame cost:** never call `buffer.to_string()` or scan full content per frame. `Tab.content_version` (u64) gates cached `is_modified()`, `text_stats()`, CJK/complex-script checks.
- **FerriteEditor perf tiers:** O(1)/O(log N) always allowed; O(visible) per-frame only; O(N) on user-initiated actions only (Find All, Save, Export).
- **Large files (>1MB):** hash-based `is_modified()`, reduced undo groups; **≥5MB** load on background thread via `open_file_smart()` (`TabContent::Loading/Ready/Error`).

## Conventions
- **Logging:** `log::info!` / `log::error!` (never `println!`); user-facing errors via `show_toast()`.
- **Errors:** `anyhow::Result` for propagation; `?` over `unwrap()`/`expect()`.
- **State:** `Tab` for per-tab state, `AppState` for global.
- **i18n:** `t!("key.path")`, keys in `locales/en.yaml` — every user-visible string.
- **Docs:** feature-based names in `docs/` (e.g., `auth-layer.md`); `docs/index.md` is the documentation map. Update in update phase only.

## Where Things Live
| Want to... | Look in... |
|------------|------------|
| Add a setting | `config/settings.rs` → `Settings` struct |
| Add keyboard shortcut | `app/keyboard.rs` → `handle_keyboard_shortcuts()` |
| Toggle word wrap | `ShortcutCommand::ToggleWordWrap` (Alt+Z); `navigation.rs` → `handle_toggle_word_wrap()`; `settings.word_wrap`. See `word-wrap.md` § User Toggle |
| Add command to palette | `config/settings.rs` → `ShortcutCommand`, `app/commands.rs` → icon, `app/central_panel.rs` → dispatch |
| Add/modify a UI panel | `ui/` → create or edit panel module |
| Modify editor core | `editor/ferrite/editor.rs` (behavior), `buffer.rs` (text), `view.rs` (viewport) |
| Modify markdown rendering / parsing | `markdown/editor.rs`, `markdown/widgets.rs` / `markdown/parser.rs` |
| GFM table column alignment | `markdown/widgets.rs` (`EditableTable`, `TableData::to_markdown`, `table_alignment_to_egui`); parse → `parser.rs` `TableAlignment`. See `gfm-table-column-alignment.md` |
| Raw-mode GFM table column guides | `editor/ferrite/table_guides.rs` (`detect_table_ranges`, `TableGuideCache`, `render_table_guides`); paint in `editor.rs` before text; markdown + no wrap only. See `raw-table-alignment.md` |
| GitHub HTML (Phases 1–2) | `markdown/parser.rs` (`process_github_html_blocks`, `process_github_html_inline`), render in `editor.rs`/`widgets.rs`. Full tag list → `github-html-subset.md`; Phase 1 blocks → `github-html-block-subset.md` |
| Video embeds | `markdown/video_embed.rs` (parse/allowlist), `markdown/video_render.rs` (WebView + thumbnail); manager on `FerriteApp`; lifecycle in `central_panel.rs` (`push`/`pop` + `clear_all`). See `docs/technical/markdown/video-embeds.md` |
| Mermaid diagrams | `markdown/mermaid/` (flowchart: `types`, `parser`, `layout/`, `render/`; FA label strip → `flowchart-fa-labels.md`; `@pos` hints → `manual-layout.md`; linkStyle interpolate → `flowchart-linkstyle-interpolate.md`); git graph: `git_graph/{types,parser,layout,render}.rs`; validation: `mermaid/validation.rs`; mmdr parser eval (not integrated) → `docs/technical/mermaid/mmdr-evaluation.md` |
| Add special/viewer tab | `state.rs` → `SpecialTabKind`/`TabKind`, `app/central_panel.rs` → render |
| Add global/per-tab state | `state.rs` → `AppState` / `Tab` struct |
| HTML / PDF export | `export/html.rs`, `export/pdf/` (krilla 2-pass) |
| Multi-window | Design → `multi-window.md`; MVP → `multi-window-implementation.md`; file routing → `multi-window-file-routing.md`; session v2 → `multi-window-session.md`; `src/app/windows.rs`, `src/config/session.rs`, `src/state.rs` (`capture_session_state`, `restore_from_session_result`, `focused_window_id`, `working_window_id`) |
| Preview lock (#144) | `Tab::preview_locked`; session via `SessionTabState.preview_locked`; padlock `render_preview_lock_overlay` in `central_panel.rs`; markdown gating via `MarkdownEditor::preview_locked` + `preview_locked_temp_id()` → `RenderedEditSession` / `widgets.rs`; CSV/Tree via `csv_viewer.rs`/`tree_viewer.rs` (`CsvCellEditParams` navigation vs edit). Split raw pane never gated. See `preview-lock.md` + `preview-lock-mode.md` |
| Workspace file tree UI | `ui/file_tree.rs` (`FileTreePanel::show`); active path from `active_tab().path`, accent via `ferrite_accent_rgb()`. See `file-tree-panel.md` |
| Stats runtime modules (Phase 1) | `ui/runtime_modules.rs` (`RuntimeModulesInfo::collect`); `fonts::get_loaded_runtime_font_names()`, `mermaid::get_cache_snapshot()`; Stats tab in `outline_panel.rs`; LSP row **Disabled**. See `stats-runtime-modules.md` |
| Formatted block click-to-edit layout | `rendered_session.rs` (`FormattedBlockLayout`, `paint_formatted_block_display`, `layout_for_formatted_click`, `BlockEditState::layout_wrap_width`); click → `enter_formatted_edit_on_display_click` in `editor.rs`. See `rendered-edit-session-formatted-layout.md` |
| Code block Run / interpreter dispatch | `markdown/code_execution.rs` — `code_run_state_key` (blake3 lang+code), `format_run_output_plain`, `ShellDispatch` chains; `PendingCodeRun.run_state_key` in consent path; POSIX fences never → PowerShell on Windows. See `code-block-run.md` |
| External file open fallback (#142) | `state.rs` → `OpenResult`, `should_open_externally`, `complete_external_file_open`; `file_ops.rs` → `open_file_smart_in_window`, `finalize_open_result`; background load → `FileLoadMsg::OpenExternal`. See `external-file-open-fallback.md` |
| System title bar (#115) | `settings.use_system_title_bar` + `native_window_decorations_enabled()`; `ui/window.rs` → `apply_window_chrome`; primary/secondary viewports in `main.rs`/`windows.rs`; skip custom title bar + borderless resize when native. Windows: setting disabled in UI. See `system-title-bar-setting.md` |
| Windows Inno Setup (optional) | `installer/ferrite.iss` + `installer/build.ps1`; output `ferrite-windows-x64-setup.exe`; unsigned in CI (separate artifact, not SignPath). MSI remains recommended. See `inno-setup-installer.md` + `github-release-checklist.md` |
| Release notes & deferrals | `CHANGELOG.md` (Unreleased = v0.3.1 on branch), `ROADMAP.md` (Recently Completed + v0.3.2 LSP target) |
| Feature deep-dives, docs map | `docs/index.md` |
| Tasks and complexity | `.cyclopsctl/tasks/tasks.json`, `.cyclopsctl/reports/complexity-report.json` |
| Cyclopsctl config | `cyclopsctl.toml` |
| Current implementation handover | `current-handover-prompt.md` |
| Post-task update rules | `update-handover-prompt.md` |

## Project Memory
*(Update-phase agents append/prune key facts here — newest first, max ~6 bullets, not a changelog.)*

- **2026-06:** Community PRs [#150](https://github.com/OlaProeis/Ferrite/pull/150), [#151](https://github.com/OlaProeis/Ferrite/pull/151), [#152](https://github.com/OlaProeis/Ferrite/pull/152) merged into `0.3.1-experimental` from [@Star-sumi](https://github.com/Star-sumi) (tab context menu, outline polish, test build fixes).
- **2026-06:** Exit / close discard & save-all fixes — `discard_unsaved_on_exit` + `save_recovery_content_excluding()` / `save_tab_by_id()` in `on_exit`; Don't Save deletes recovery + autosave for prompted tabs only (quick-note buffers preserved); Save-all via `handle_save_all_modified_tabs`; window-close Save branch; multi-window dialog cleanup. See `session-persistence.md`.
- **2026-06:** v0.3.1 epic complete on `0.3.1-experimental` (all cyclopsctl tasks done); `CHANGELOG.md` Unreleased + `ROADMAP.md` Recently Completed document shipped scope; **Deferred** lists LSP epic → v0.3.2+ (stays behind `lsp` flag) and Tier C cuts. Platform gates [#106](https://github.com/OlaProeis/Ferrite/issues/106)/[#111](https://github.com/OlaProeis/Ferrite/issues/111) still open (KBD-8/KBD-9 unverified); [#112](https://github.com/OlaProeis/Ferrite/issues/112) WIN-8 Intel iGPU retest pending.
- **2026-06:** Windows Inno Setup (optional) — `installer/ferrite.iss` builds `ferrite-windows-x64-setup.exe` from `target/release/ferrite.exe`; optional tasks mirror MSI (associations via OpenWithProgids, context menu, PATH); CI builds unsigned (not SignPath); MSI remains recommended signed install. See `inno-setup-installer.md`.
- **2026-06:** System title bar setting (#115) — `use_system_title_bar` (default off); `native_window_decorations_enabled()` on Linux/macOS only; `apply_window_chrome` in `ui/window.rs`; restart required; custom title bar + borderless resize skipped when native. Windows: disabled checkbox + tooltip. See `system-title-bar-setting.md`.
- **2026-06:** External file open fallback (#142) — `OpenResult` (`OpenedTab` | `OpenedExternal` | `Failed`); extension denylist + `is_binary_content` → `open::that` + toast via `complete_external_file_open`; `open_file_smart_in_window` + `finalize_open_result` in `file_ops.rs`; large-file loader sends `FileLoadMsg::OpenExternal`. See `external-file-open-fallback.md`.

## Build & Test
```bash
cargo check    # Quick compile check
cargo build    # Build debug
cargo clippy   # Lint
cargo test     # Run tests
cargo run      # Run app
```
