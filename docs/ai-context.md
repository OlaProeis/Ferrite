# Ferrite - AI Context

Rust (edition 2021) + egui 0.34.2 markdown editor. Immediate-mode GUI — no retained widget state, UI rebuilds each frame.

## Rules (DO NOT UPDATE)
- Never auto-update this file or `current-handover-prompt.md` — only update when explicitly requested.
- Only do the task specified; do not start the next task or go over scope.
- Run `cargo build` or `cargo check` after changes to verify code compiles.
- Follow existing code patterns and conventions.
- Document by feature (e.g., `lsp-inline-diagnostics.md`), not by task.
- Update `docs/index.md` when adding new documentation.
- Use Context7 MCP tool to fetch library documentation when needed (resolve library ID first, then fetch docs).

## Tech Stack
- **Language:** Rust 2021 (MSRV **1.92**), egui **0.34.2** + eframe (glow on Windows; bumped 0.28 → 0.31 → 0.34 — see `docs/technical/platform/eframe-egui-031-upgrade.md`, `eframe-egui-034-upgrade.md`)
- **Text:** ropey (rope buffer), comrak (Markdown AST), syntect (syntax highlighting), harfrust (OTL shaping)
- **Terminal:** portable-pty + vte | **VCS:** git2 | **Dialogs:** rfd | **i18n:** rust-i18n | **Hashing:** blake3 | **PDF read:** hayro | **PDF write:** krilla + krilla-svg
- **Memory:** mimalloc (Windows), jemalloc (Unix)

## Architecture

| Module | Purpose |
|--------|---------|
| `app/` | Main application (~15 modules: keyboard, file_ops, formatting, navigation, etc.) |
| `state.rs` | All application state (`AppState`, `Tab`, `TabKind`, `SpecialTabKind`, `FileType`) |
| `editor/ferrite/` | Rope-based editor (`ropey`): buffer, cursor, history, view, rendering, line_cache |
| `editor/widget.rs` | EditorWidget wrapper, integrates FerriteEditor via egui memory |
| `markdown/` | `editor.rs` (rendered view), `rendered_session.rs` (edit session coordinator), `rendered_commit_undo.rs` (commit-boundary undo queue), `code_execution.rs`, … |
| `terminal/` | Integrated terminal (PTY, VTE, screen, themes, split layouts) |
| `ui/` | Panels: ribbon, settings, file_tree, outline, search, terminal, productivity, frontmatter, welcome, command_palette |
| `config/` | Settings persistence, session/crash recovery, snippets |
| `fonts.rs` | Font loading, lazy CJK, complex script lazy loading (11 families) |
| `theme/` | Light/dark themes, **user accent** (`accent.rs`, ThemeManager sync) |
| `vcs/git.rs`, `workspaces/`, `export/`, `preview/`, `platform/` | Git, folder mode, HTML+PDF export (`export/pdf/` = krilla 2-pass renderer), sync scroll, platform-specific |

**FerriteEditor:** `src/editor/ferrite/` — rope-based, O(log n) ops, virtual scrolling, multi-cursor, code folding, IME/CJK. ~1x file size RAM. `EditorWidget` creates/retrieves from egui memory. Docs: `docs/technical/editor/architecture.md`

## Critical Patterns

```rust
// Always use saturating math for line indices
let idx = line_number.saturating_sub(1);

// Never unwrap in library code
if let Some(tab) = self.tabs.get_mut(self.active_tab) { ... }

// Prefer borrowing over clone
fn process(text: &str) -> Vec<&str> { text.lines().collect() }
```

## Common Gotchas

| Issue | Wrong | Right |
|-------|-------|-------|
| Byte vs char index | `text[start..end]` with char pos | Use `text.char_indices()` or byte offsets |
| Line indexing | Mixing 0/1-indexed | Explicit: `line.saturating_sub(1)` |
| CPU spin | Always `request_repaint()` | Use `request_repaint_after()` when idle |

## Conventions

- **Logging:** `log::info!`, `log::error!` (not println!)
- **i18n:** `t!("key.path")`, keys in `locales/en.yaml`
- **State:** `Tab` for per-tab, `AppState` for global
- **Errors:** User-facing via `show_toast()`, technical via `log::error!`
- **Large files (>1MB):** Hash-based `is_modified()`, reduced undo groups (200 vs 500), no `original_bytes`
- **Background file loading (≥5MB):** `open_file_smart()` on `FerriteApp` spawns background thread; `Tab.tab_content` (`TabContent::Loading`/`Ready`/`Error`) tracks state; `FileLoadMsg` channel polled in `update()`
- **Per-frame caching:** `Tab.content_version` (u64) gates cached `is_modified()`, `text_stats()`, `needs_cjk_cached()`, `needs_complex_script_cached()` — never scan full content per frame

## Where Things Live (common)

| Want to... | Look in... |
|------------|------------|
| Add a setting | `config/settings.rs` → `Settings` struct |
| Add keyboard shortcut | `app/keyboard.rs` → `handle_keyboard_shortcuts()` |
| Add command to palette | `config/settings.rs` → `ShortcutCommand`, `app/commands.rs` → icon, `app/central_panel.rs` → dispatch |
| Add/modify a UI panel | `ui/` → create or edit panel module |
| Modify editor core | `editor/ferrite/editor.rs` (behavior), `buffer.rs` (text), `view.rs` (viewport) |
| Modify markdown rendering | `markdown/editor.rs` or `markdown/widgets.rs` |
| Rendered edit session | `markdown/rendered_session.rs`; hub: [`rendered-edit-session.md`](./technical/markdown/rendered-edit-session.md); block wiring in `editor.rs` |
| Rendered commit undo | `markdown/rendered_commit_undo.rs` (pre/post snapshot queue); `Tab::apply_rendered_commit_undo_entries` in `state.rs`; drained from `central_panel.rs` after `MarkdownEditor::show` |
| Session / crash recovery | `config/session.rs` (`RecoveryContent`, autosave); `state.rs` `resolve_tab_content` — [`session-persistence.md`](./technical/files/session-persistence.md) |
| Rendered widget id scope | `markdown/editor.rs` `push_id(editor_id + source_epoch)` — [`rendered-widget-identity.md`](./technical/markdown/rendered-widget-identity.md) |
| External invalidation epoch (`source_epoch`) | `state.rs` → `Tab::source_epoch()`, `bump_source_epoch()`, `record_external_edit_from_snapshot()` |
| Rendered table cells (hits, Tab focus) | `markdown/widgets.rs` `EditableTable` — [`table-cell-focus-navigation.md`](./technical/markdown/table-cell-focus-navigation.md); session/force-commit in [`rendered-edit-session-tables.md`](./technical/markdown/rendered-edit-session-tables.md) |
| Modify markdown parsing | `markdown/parser.rs` |
| Video embed parsing | `markdown/video_embed.rs`, `parser.rs` (`VideoEmbed` AST) — [`video-embed-parsing.md`](./technical/markdown/video-embed-parsing.md) |
| Modify central panel | `app/central_panel.rs` |
| Add special tab | `state.rs` → `SpecialTabKind`, `app/central_panel.rs` |
| Add viewer tab | `state.rs` → `TabKind` variant + state struct, `app/central_panel.rs` → render method |
| Add global/per-tab state | `state.rs` → `AppState` / `Tab` struct |
| Add i18n string | `locales/en.yaml` + `t!("key")` |
| Mermaid diagrams | `markdown/mermaid/` (flowchart has `types`, `parser`, `layout/`, `render/`); insert snippets: `mermaid/templates.rs`; F1 **Mermaid** help: `ui/about.rs` + `docs/technical/mermaid/mermaid-syntax-help.md`; validation: `mermaid/validation.rs` |
| Themed HTML export | `export/html.rs`, `export/html_options.rs`, `export/flowchart_svg.rs`; `app/dialogs.rs` (HTML export dialog) |
| PDF print preview | `app/export.rs` (`handle_print_preview`), `state.rs` (`PdfViewerState` ephemeral temp + session skip); docs: `docs/technical/viewers/print-preview.md` |
| Terminal | `terminal/` (pty, screen, widget, layout) |
| Git/VCS | `vcs/git.rs` |
| Workspace file index | `workspaces/file_index.rs` — full-tree walk for Ctrl+P / search (not lazy tree) |

## Performance Rules (FerriteEditor)

| Tier | When Allowed | Examples |
|------|--------------|----------|
| O(1) | Always | `line_count()`, `is_dirty()` |
| O(log N) | Always | `get_line(idx)`, index conversions |
| O(visible) | Per-frame | Syntax highlighting visible lines |
| O(N) | User-initiated ONLY | Find All, Save, Export |

**Never** call `buffer.to_string()` in per-frame code.

## Recently Changed

- **2026-05:** **Workspace file index** — Background `walkdir` index for Ctrl+P and Ctrl+Shift+F (full tree, not lazy sidebar); progress bar on large folders. See `docs/technical/files/workspace-file-index.md`.
- **2026-05:** **v0.3.0** — egui/eframe **0.34.2**, Rust **1.92** MSRV, skrifa text backend, Popup/Tooltip API migrations, HarfRust validation, Phosphor **0.12**. See `docs/technical/platform/eframe-egui-034-upgrade.md`.
- **2026-05:** User-configurable **Ferrite accent** (`Settings.accent_color`): Settings/Welcome color picker; drives headings, selection tint, tabs, view R/S/V segment, productivity hub, status LSP/branch; markdown links unchanged. See `docs/technical/ui/theme-system.md`.

## Build & Test

```bash
cargo build          # Build debug
cargo run            # Run app
cargo clippy         # Lint
cargo test           # Run tests
```
