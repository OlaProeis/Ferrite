# Documentation Index

> **Index rules:** This file is a documentation map only. Do not add project history, task lists, architecture overviews, or session notes. When adding docs, append a single bullet with a one-line description under the appropriate section.

## Core Context
- `ai-context.md` - Ferrite — Editor UX Polish & Correctness Wave (PRD) agent rules, architecture, and where things live.

## Technical Docs
- `technical/files/session-persistence.md` - Session save/restore, crash recovery, identity-gated recovery, and workspace file-watcher external reload (`Tab::apply_external_disk_reload`).
- `technical/markdown/rendered-edit-source-range.md` - Span math for rendered paragraph/list commits (`block_replace_end_line`, `update_source_range`); fixes multi-line buffer duplication without bumping `source_epoch`.
- `technical/markdown/rendered-edit-flush.md` - `flush_rendered_edit_session` and app flush helpers; wires `commit_active` on view/tab/save/close/focus-loss so lone focused blocks reach `tab.content`.
- `technical/markdown/rendered-edit-session-paragraphs-lists.md` - Plain paragraph/list session model; Enter commit+exit, Shift+Enter soft break, buffer resync, and commit-on-switch behaviour.
- `technical/markdown/gfm-table-column-alignment.md` - GFM table per-column alignment in rendered view; `table_cell_galley_paint_pos` compensates galley offset and block-shifts short text within cell width.
- `technical/viewers/csv-viewer.md` - CSV/TSV rendered table viewer; inline cell editing, Tab/Shift+Tab + arrow keyboard navigation, and lazy parsing for large files.
- `technical/markdown/video-embed-parsing.md` - Video embed AST parsing; explicit `{{video URL}}` with optional `width`/`height` params, allowlist, and `source_text` round-trip.
- `technical/markdown/video-embed-rendering.md` - Video embed rendered view; WebView relay path, thumbnail fallback, `video_display_size()` sizing, and drag-resize handle with source write-back.
- `technical/ui/ribbon-window-control.md` - New Window icon in the ribbon right cluster (beside Export/Terminal); title-bar Window menu removed; `RibbonAction::NewWindow` wiring unchanged.
- `technical/ui/raw-editor-context-menu.md` - Raw FerriteEditor right-click menu (Copy/Cut/Paste/Select All/Undo); `EditorWidget` + app-level undo via `EditorOutput.request_undo`.
- `technical/ui/tab-context-menu.md` - Tab strip right-click menu; i18n labels (`tab.new_tab`/`tab.close`), hover rows via background-layer fill, content-sized popup in `action_registry.rs`.
- `technical/ui/command-palette.md` - Searchable command launcher (Alt+Space); deferred dispatch, fuzzy search, palette-only commands via unbound defaults.
- `technical/ui/preview-lock.md` - Per-tab preview read-only flag; padlock overlay, command-palette **Lock editing** toggle, session persistence, markdown/CSV/tree gating.
