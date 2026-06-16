# Ferrite — Editor UX Polish & Correctness Wave (PRD)

> **Status:** Formal PRD — source of truth for this build phase.
> **Consumers:** Human orchestrator + AI implementation sessions (composer 2.5) driven by **cyclopsctl**. Parse this file into the native task queue.
> **Scope philosophy:** This is a *correctness + polish* wave, not a feature epic. Each requirement below is a discrete, mostly-independent fix. **Do not force a fixed number of tasks** — the parser should size tasks by complexity. A couple of items (rendered-view edit sync, video embeds) are large enough to warrant subtasks; several (i18n keys, button relocation, context menus) are small.

## 1. Overview

Ferrite is a cross-platform Rust (edition 2021) + egui markdown editor. **v0.3.1** shipped Mermaid wave 2, video embeds, multi-window, GFM table alignment, CSV rendered editing, preview lock, and the rendered edit session. This wave addresses a set of **bugs and UX rough edges** discovered during dogfooding of those features.

The work clusters into three themes:

1. **Rendered/WYSIWYG editing correctness** — edits in rendered view don't reliably sync back to the raw buffer, and newlines can duplicate content. This is the highest-impact correctness fix in this PRD.
2. **Data & table UX** — CSV cells can be edited but not keyboard-navigated; GFM table column alignment is parsed and serialized but does not visibly affect rendering.
3. **Editor & chrome polish** — external-change reload falsely marks files modified; video embeds over-trigger on bare links and can't be resized; window controls are duplicated; the raw editor lacks a context menu; the tab context menu is unpolished and shows raw i18n keys; preview lock isn't in the command palette.

All rendering is **native egui** (immediate mode — UI rebuilds each frame, no retained widget state). Follow existing patterns and the conventions in `ai-context.md`. Use Context7 MCP for egui/library API details when implementing.

## 2. Goals

- Rendered-view text edits are reliably reflected in the raw buffer **without** requiring focus to move to another block, and **never** duplicate content when the user presses Enter.
- CSV cells in rendered mode are fully keyboard-navigable (Tab/Shift+Tab + arrow keys), mirroring the GFM `EditableTable` pattern.
- GFM table per-column alignment (left/center/right) is **visible** in the rendered view, matching the parsed delimiter row.
- A file changed by another application reloads cleanly with **no** false "modified" state and **no** save/discard prompt on close (standard editor behavior for clean buffers).
- Video embeds render **only** from explicit `{{video …}}` syntax (bare YouTube links stay normal clickable links), and the rendered player is **resizable**.
- Window controls appear in **one** place — the icon bar — not duplicated in the title bar.
- The raw editor has a right-click context menu (Copy/Cut/Paste/Select All).
- The tab right-click menu has proper hover feedback, sensible width, and correct English labels.
- "Lock editing" (preview lock) is toggleable from the command palette.

## 3. Non-goals

- No change to the *architecture* of the rendered edit session beyond what's needed to fix the sync/duplication bugs (keep `RenderedEditSession`, `BlockRef`, `source_epoch`).
- No CSV add/remove rows/columns toolbar in this wave (separate ROADMAP item); navigation only.
- No new video providers (Vimeo etc.) — YouTube allowlist unchanged; only the *trigger syntax* and *sizing* change.
- No file watching in single-file (non-workspace) mode — external-change handling remains workspace-scoped (document the limitation; do not add a new watcher).
- No redesign of the ribbon/icon bar layout beyond relocating the window control.
- No new keyboard shortcut is *required* for preview lock (palette entry is the deliverable; a default binding is optional).

## 4. Conventions (apply to every task)

- **i18n:** every user-visible string uses `t!("key.path")` with the key defined in `locales/en.yaml`. Never display a raw key.
- **Logging:** `log::info!` / `log::error!`; user-facing errors/notices via `show_toast()`.
- **Errors:** `anyhow::Result` + `?`; no `unwrap()`/`expect()` in non-test code.
- **State:** per-tab state on `Tab`, global on `AppState`. Be explicit about 0- vs 1-indexed line math; use saturating arithmetic.
- **Performance:** no full-buffer scans per frame; respect the FerriteEditor complexity tiers.
- **Tests:** add/extend unit tests where logic is testable (parsing, range math, modified-state). Run `cargo test` and `cargo clippy` before finishing. UI-only changes get a manual test note in the relevant doc.
- **Docs:** update or add a feature doc under `docs/technical/…` and register it in `docs/index.md` (update phase only, per `ai-context.md`).

---

## 5. Functional requirements

Each subsection is a self-contained problem statement with: current behavior, root cause (from code investigation), desired behavior, recommended approach, key files, and acceptance criteria. Tasks may be split further by the parser where noted.

---

### 5.1 Rendered-view edit sync correctness (highest priority)

**Problem.** When editing normal text (a paragraph) in rendered/WYSIWYG view:

1. **Single-paragraph never commits.** The change does not propagate to the raw buffer until focus moves to *another* block. If the document has only one paragraph (no other block to switch to), the edit is never written back to `tab.content`.
2. **Newline duplication.** Start with one line `test`, edit it in rendered view, press Enter, type `test2`. When the cursor is placed back in raw mode, raw updates but the `test2` line appears **doubled** in the rendered view. We must guarantee raw and rendered always show the same content with **no doubling**.

**Root cause (verified in code).** Rendered editing is owned by `RenderedEditSession` (`src/markdown/rendered_session.rs`). Each block has a `BlockEditState` buffer. Keystrokes only update the session buffer (`on_text_changed`, ~line 187–197); the source (`tab.content`) is written **only** on a commit trigger:

- block switch (`switch_to_ui` → `close_active(SaveIfDirty)`),
- focus loss / `response.lost_focus()` (`render_session_plain_text_block`, `src/markdown/editor.rs` ~2967),
- click-outside dismiss (`session_dismiss_if_clicked_outside`, `src/markdown/editor.rs` ~2515).

Commit triggers are **edge-triggered only**. There is **no flush** on view-mode toggle (`handle_toggle_view_mode` in `src/app/navigation.rs` ~61–98 changes `tab.view_mode` without flushing the session) and no commit for a lone, continuously-focused block. `commit_active()` exists (`rendered_session.rs` ~200–214) but is **never called** from production code.

For duplication: commit for paragraphs calls `update_source_range(source, start_line, end_line, &state.text)` (`src/markdown/editor.rs` ~6705–6751). `end_line` is read from the AST parsed at **frame start** (stale relative to a buffer that has grown via Enter), and `update_source_range` replaces `start_line..=end_line` then **keeps all trailing source lines**. If a multi-line buffer is committed against a stale single-line `end_line`, the new lines are written **and** the old trailing line is preserved → duplication. Additionally `BlockRef::Paragraph` is keyed only by `start_line`, so a session buffer can survive and re-render stale multi-line text because rendered commits do **not** bump `source_epoch` (so `load_for_epoch`/`invalidate_buffers` never fires for a rendered-only edit).

**Desired behavior.**

- Any rendered-view edit is reflected in `tab.content` (and therefore raw view / split raw pane) **without** requiring a move to another block. At minimum, flush the active session buffer when: switching view mode (Rendered→Raw / Rendered→Split / via Ctrl+E), switching tabs, losing window focus, saving, and before close/exit checks.
- Enter behavior in plain paragraphs is predictable and never duplicates lines. Choose and document one model and apply it consistently with the formatted-block path (`render_session_formatted_edit_text`, where plain Enter commits+exits and Shift+Enter inserts a soft newline):
  - **Recommended:** plain Enter splits the paragraph (structural) OR commits+exits; Shift+Enter inserts a soft line break. Whatever is chosen, the committed source must round-trip back to an identical rendered view.
- After a commit that changes the block's line span, the source range used for the *next* commit must reflect the new span (re-derive `end_line` from the committed text length, not the stale AST), and/or invalidate the session buffer so it cold-loads fresh from source. No path may both keep the old trailing lines and re-insert the new ones.

**Recommended approach.**

1. Add an explicit **flush** entry point: call `RenderedEditSession::commit_active` (the existing unused API) from `handle_toggle_view_mode` (`src/app/navigation.rs`), from tab-switch, and from the save/close/exit paths. Wire it so the active dirty block writes to source before the view changes.
2. Fix `update_source_range` / the paragraph commit so the replaced range is computed from the **current** buffer span, not a stale frame-start `end_line`. Either (a) recompute `end_line` as `start_line + committed_line_count - 1` and replace exactly that many old lines, or (b) after commit, bump `source_epoch` (or call `invalidate_buffers`) so the session reloads from the now-authoritative source on the next frame, preventing stale multi-line buffers from re-rendering.
3. Ensure that after a rendered commit the rendered view re-parses from source (single source of truth) so the same content can't exist both in a session buffer and as a freshly-parsed block.
4. Add a regression test for the range math: given source `"test"` and a committed buffer `"test\ntest2"`, the result must be exactly `"test\ntest2"` (not `"test\ntest2\ntest2"`).

**Key files.** `src/markdown/rendered_session.rs` (`commit_active`, `close_active`, `switch_to_ui`, `invalidate_buffers`, `BlockRef`), `src/markdown/editor.rs` (`render_session_plain_text_block`, `write_session_block_to_source`, `commit_session_block`, `update_source_range`, `show_rendered_editor`), `src/app/navigation.rs` (`handle_toggle_view_mode`), `src/state.rs` (`source_epoch`, `notify_external_content_change`).

**Acceptance criteria.**

- Editing the only paragraph in a one-line document and switching to Raw shows the edit in Raw — no second block needed.
- The Enter→`test2` scenario yields identical content in Raw and Rendered with no duplicated line, in both rendered-only and split views.
- Toggling view mode (Ctrl+E) flushes pending rendered edits.
- Unit test covers the `update_source_range` span/duplication case.
- `RS-1…RS-7` rendered-session regression checks (see `docs/technical/markdown/rendered-edit-session.md`) still pass; no new stuck-edit or focus regressions.
- Docs: update `rendered-edit-session.md` (and the paragraphs/lists sub-doc) with the new flush points and Enter semantics.

---

### 5.2 CSV rendered-mode cell keyboard navigation

**Problem.** CSV/TSV cells are editable in rendered mode (double-click → edit, Enter/Escape commit/cancel), but there is no way to **move between cells** with the keyboard. We want:

- **Tab** → next cell, **Shift+Tab** → previous cell (wrapping across row ends like a spreadsheet).
- **Arrow keys** (Up/Down/Left/Right) → move the selected/active cell.

**Current behavior (verified in code).** CSV lives in `src/markdown/csv_viewer.rs` (not `src/ui/`). `CsvViewerState` (~828–835) holds `selected_cell`, `editing_cell`, `edit_buffer`, `pending_commit`. Arrow-key navigation exists in `show_table_view` (~1901–1914) via `move_selected_cell` (~1160–1171) but only fires when the table focus id has focus AND no cell is editing — and clicking a cell sets `selected_cell` without requesting that focus, so arrows often don't work after a click. **Tab/Shift+Tab is entirely absent.** The inline edit `TextEdit::singleline` (~1249–1262) does not call `lock_focus(true)`, so Tab escapes to egui's global tab order. The large-file lazy path (`show_table_view_lazy`) passes no edit/navigation params (out of scope here — keep ≥1 MB files non-navigable, but the gate must not break).

**Reference pattern.** The GFM `EditableTable` already implements Tab/Shift+Tab/Enter navigation in `src/markdown/widgets.rs` (~2590–2688): it uses `ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab))` (checked **before** plain Tab), `TextEdit::lock_focus(true)`, and `TableEditState::move_next/move_prev/move_up/move_down` (~1842–1870) plus `request_table_cell_focus` (~1767–1796). See `docs/technical/markdown/table-cell-focus-navigation.md` and `editable-tables.md`.

**Desired behavior.** Mirror the `EditableTable` keyboard model for CSV:

- While a cell is focused/editing: `Shift+Tab` (consume first) → commit current + move to previous cell and focus it; `Tab` → commit + next cell; arrow keys move the active cell (commit current first when leaving). At a row boundary, Tab/Shift+Tab wrap to the next/previous row; clamp at table start/end.
- Clicking a cell must also establish keyboard focus so arrows work immediately.
- Use `lock_focus(true)` on the edit widget and `consume_key` so Tab never escapes the table.
- Commit-before-navigate must use the existing `queue_cell_commit`/`serialize_csv_rows` path so undo and content-change signaling keep working (`src/app/central_panel.rs` undo hook on `output.content_changed`).
- Respect preview lock: when `CsvCellEditParams.cell_edit_enabled` is false, navigation (selection move) may still work but editing must not.

**Key files.** `src/markdown/csv_viewer.rs` (`CsvViewerState`, `render_row_cells`, `show_table_view`, `begin_cell_edit`, `queue_cell_commit`, `move_selected_cell`, `CsvCellEditParams`), reference `src/markdown/widgets.rs` (`TableEditState`). Consider extracting a shared `move_next/move_prev` helper if clean.

**Acceptance criteria.**

- In a CSV file in rendered mode: Tab/Shift+Tab move and wrap across cells; arrows move the active cell; edits commit before the move.
- Clicking a cell then pressing an arrow moves selection (no extra click on empty chrome needed).
- Tab does not move focus out of the table.
- Preview-locked CSV: no edit, navigation behavior consistent with the lock matrix.
- `cargo test` passes; add a unit test for the cell-index navigation/wrap math if it's factored into a pure helper.
- Docs: update `docs/technical/viewers/csv-viewer.md` (Rendered Cell Editing section) with the navigation keys; note the ROADMAP "CSV Tab/Shift+Tab" item is now done.

---

### 5.3 GFM table column alignment renders correctly

**Problem.** Markdown GFM tables support per-column text alignment via the delimiter row (`:---`, `:---:`, `---:`). Parsing and serialization work, but the alignment **does nothing visible** in the rendered view — center/right-aligned columns still appear left-aligned.

**Root cause (verified in code).** Alignment is parsed (`TableAlignment` in `src/markdown/parser.rs` ~122–138), stored on `TableData.alignments` (`src/markdown/widgets.rs` ~1915), and serialized correctly (`to_markdown` ~2105–2124). At render time, `table_alignment_to_egui` maps to an egui align and the display branch sets `job.halign` on the `LayoutJob`, then paints with `ui.painter().galley(response.rect.min, galley, …)` (`src/markdown/widgets.rs` ~2708–2748). Two problems:

1. **Galley paint offset ignored.** For `LayoutJob.halign` Center/Right, epaint anchors the galley with a non-zero/negative `rect.min.x`. Standard egui widgets compensate (e.g. `text_pos - galley.rect.min.to_vec2()`). The CSV-style direct paint at `response.rect.min` skips this, so left works but center/right look left-aligned or clipped.
2. **halign uses content width, not cell width.** With `justify == false`, epaint repositions glyphs only within the text's natural width, not the full cell width, so center/right have no visible effect on typical short cells.

Note the codebase already has a *working* alignment pattern for HTML `<div align>` that avoids `LayoutJob.halign` and uses a layout/justify approach instead (`with_block_align_widget`, `src/markdown/editor.rs` ~4890–4910; see `docs/technical/markdown/github-html-block-subset.md`).

**Desired behavior.** Center- and right-aligned table columns visibly align their cell content (display mode at minimum; verify edit mode too). Left stays the default.

**Recommended approach.** In the `EditableTable` display branch, either:

- paint the galley at `response.rect.min - galley.rect.min.to_vec2()` AND lay out against the full cell inner width so center/right have room (e.g. set the layout job to justify within `inner_w`, or position the galley horizontally within `inner_w` based on alignment), or
- adopt the proven `with_block_align_widget`-style layout used for `<div align>` instead of `LayoutJob.halign`.

Keep `table_cell_raw_cursor_at_click` (`widgets.rs` ~1133–1181) in sync with whatever paint-position math is chosen, so click-to-edit still lands the cursor correctly.

**Key files.** `src/markdown/widgets.rs` (`EditableTable::show` display branch ~2700–2748, `table_alignment_to_egui`, `table_cell_raw_cursor_at_click`, cell layout-job builders), reference `src/markdown/editor.rs` (`with_block_align_widget`, `render_table`).

**Acceptance criteria.**

- A table with `:---:` / `---:` columns shows centered / right-aligned cell text in rendered and split-preview view.
- Toolbar alignment cycling (`TableAction::CycleAlignment`) produces an immediately visible change.
- Click-to-edit still places the cursor at the correct character for all three alignments.
- Round-trip (parse → render → serialize) of the delimiter row is unchanged; existing alignment tests still pass; add a layout/positioning assertion if feasible.
- Docs: update `docs/technical/markdown/gfm-table-column-alignment.md` to describe the actual rendering mechanism (correct any overstatement about visual support).

---

### 5.4 External file change reloads without false "modified" state or save prompt

**Problem.** When a file open in Ferrite is modified by another application, Ferrite reloads the new content (good) — but then marks the tab as **modified** (`*` indicator) and prompts **Save or Discard** when the tab/app closes. This is wrong: after reloading from disk, the in-memory content matches disk, so the tab should be clean.

**Common practice (for reference).** Editors (VS Code, Sublime, etc.) treat a clean buffer + external change as: silently reload (optionally with a subtle notice), buffer stays **clean**, no prompt. Only when the buffer has **unsaved** local edits AND disk changes do they show a conflict prompt. Ferrite already only auto-reloads when the tab is unmodified and only shows a toast when it has unsaved edits — so the policy is correct; only the post-reload bookkeeping is buggy.

**Root cause (verified in code).** External change detection runs in **workspace mode** via the `notify` crate (`src/workspaces/watcher.rs`, polled by `AppState::poll_file_watcher` in `src/state.rs` ~5147, handled in `FerriteApp::handle_file_watcher_events` in `src/app/file_ops.rs` ~1315–1468). The reload body (~1412–1421) does:

```
tab.content = new_content.clone();
tab.notify_external_content_change();
// ... cursor clamp ...
```

It **does not call `tab.mark_saved()`**, so the saved baseline (`original_content` / `original_content_hash`) still reflects the pre-reload content. `is_modified()` (`src/state.rs` ~2094–2103) then compares new content to the stale baseline → returns `true` → `*` + close/exit prompt.

The **correct pattern already exists** in the recovery-conflict "Reload from Disk" path (`src/state.rs` ~5454–5456):

```
tab.content = conflict.on_disk_content;
tab.notify_external_content_change();
tab.mark_saved();   // <-- updates baseline so is_modified() == false
```

with a test asserting the reloaded tab is not modified (`src/state.rs` ~7334–7348).

**Desired behavior.** After an external auto-reload of an unmodified tab: `tab.is_modified()` is `false`, no `*`, no save/discard prompt on close. Behavior for tabs with unsaved local edits is unchanged (skip reload + existing toast).

**Recommended approach.** In `handle_file_watcher_events`, after assigning `tab.content` on reload, call `tab.mark_saved()` (mirroring the recovery path). Also refresh encoding metadata so a subsequent save doesn't re-encode stale bytes: update `original_bytes`/`detected_encoding`/`had_bom` if reasonably available (at minimum keep them consistent with the new content; the simple read path uses UTF-8/lossy — document this). Consider clearing/avoiding spurious undo entries from the reload.

**Key files.** `src/app/file_ops.rs` (`handle_file_watcher_events` reload body), `src/state.rs` (`mark_saved`, `is_modified`, reference `apply_reload_from_disk_for_conflict`).

**Acceptance criteria.**

- Open a file in a workspace, modify it in another app: Ferrite reloads, tab shows **no** `*`, and closing it does **not** prompt to save.
- A tab with unsaved local edits is **not** silently overwritten (existing skip + toast behavior intact).
- Add a unit/integration test: simulate external reload of an unmodified tab → assert `!tab.is_modified()` (mirror the existing recovery test).
- Docs: note the fix and the workspace-only scope in `docs/technical/files/session-persistence.md` (or the file-watching doc).

---

### 5.5 Video embeds: require explicit syntax + resizable player

**Two related changes to the video embed feature** (`src/markdown/video_embed.rs` parse, `src/markdown/video_render.rs` render). May be split into two tasks.

#### 5.5a Require explicit `{{video …}}` syntax (no bare-link auto-embed)

**Problem.** A bare YouTube link in a paragraph currently auto-renders as a playable embed. We want embeds to require explicit syntax; a plain link should stay a normal clickable link.

**Root cause (verified in code).** `try_parse_video_paragraph` (`src/markdown/video_embed.rs` ~126–144) first tries braced `{{video …}}`, then falls through to `extract_bare_youtube_url` (~146–187) which converts a paragraph that is a single bare YouTube URL (text or autolink) into a `VideoEmbed`. Removing/gating the bare-link branch (~137–143) makes bare links remain normal paragraphs/links (comrak autolink already renders them as clickable `Link` nodes).

**Desired behavior.** Only `{{video URL}}` (and any extended syntax from 5.5b) produces a `VideoEmbed`. A bare `https://youtu.be/…` paragraph renders as a normal clickable link. Existing documents that relied on bare-link embeds will show links instead (acceptable; note in CHANGELOG).

**Recommended approach.** Remove the bare-link fallback call in `try_parse_video_paragraph`; delete/retire `extract_bare_youtube_url` and its test (`document_parses_bare_youtube_url_paragraph` ~266–276); keep the braced parser and allowlist. Update README line ~135 and docs.

#### 5.5b Resizable rendered video player

**Problem.** The rendered player is a fixed size: full pane width × 16:9 (`video_display_size` in `src/markdown/video_render.rs` ~695–698; `EMBED_ASPECT_RATIO = 9/16`). It is not resizable by mouse or syntax.

**Desired behavior.** The user can resize the embedded player. Decide and implement a sensible model (research norms; egui has no built-in video sizing convention):

- **Syntax (recommended primary):** extend the braced grammar to accept optional dimensions, e.g. `{{video URL width=640}}` or `{{video URL width=640 height=360}}`. Width-only keeps the 16:9 aspect; both sets explicit size. Persist via `source_text` round-trip so edits survive.
- **Mouse drag (optional/secondary):** a drag handle (e.g. bottom-right corner) that resizes the embed; on release, write the new dimensions back into the `{{video …}}` source so it persists. If mouse-resize is implemented, it must round-trip to syntax (no hidden state).

**Root cause / current state.** No width/height fields on `VideoEmbedInfo` (`src/markdown/parser.rs` ~300–310); `parse_braced_video_syntax` (~94–108) passes the whole inner string to URL parsing, so it can't currently carry params. The WebView already tracks the egui layout rect each frame via `set_bounds`/`sync_trusted_embed`, so once the egui rect is resizable the WebView follows automatically.

**Recommended approach.**

1. Add optional `width`/`height` (e.g. `Option<f32>` or `Option<u32>`) to `VideoEmbedInfo`.
2. Extend `parse_braced_video_syntax` to split the URL token from optional `key=value` params (`width`, `height`); validate and clamp to sane bounds; ignore unknown keys; keep `source_text` exact for round-trip.
3. In `video_render.rs`, compute the display size from explicit dimensions when present (else current full-width 16:9). Clamp to available width.
4. If implementing mouse drag: add a resize affordance with `Sense::drag()`, update a per-embed size, and on release rewrite the `{{video …}}` source line with the new `width`/`height` so it persists (use the existing source-edit machinery). Keep the image-embed sizing conventions (`MarkdownNodeType::Image` width/height, `<img width= height=>`) as a style reference.

**Key files.** `src/markdown/video_embed.rs` (`try_parse_video_paragraph`, `parse_braced_video_syntax`, `extract_bare_youtube_url`), `src/markdown/parser.rs` (`VideoEmbedInfo`), `src/markdown/video_render.rs` (`video_display_size`, `render_video_embed`), `src/markdown/widgets.rs` (`serialize_node` round-trip), README + `docs/technical/markdown/video-embed-parsing.md` / `video-embed-rendering.md`.

**Acceptance criteria.**

- A bare YouTube URL paragraph renders as a clickable link, **not** a player.
- `{{video URL}}` still renders the player; `{{video URL width=…}}` / `… height=…` set the player size; unknown/invalid params are ignored gracefully.
- Dimensions round-trip through save/reload via `source_text` (and via mouse-drag write-back if implemented).
- Allowlist/security gates unchanged; untrusted hosts never get a WebView.
- Update tests (retire bare-link test, add param-parsing tests), README, and the two video-embed docs.

---

### 5.6 Consolidate window controls into the icon bar

**Problem.** The "New Window" control is duplicated: a **Window** dropdown in the top title bar (`src/app/title_bar.rs`) **and** a **Window** ComboBox in the ribbon/icon bar (`src/ui/ribbon.rs`). Having it in the title bar is redundant.

**Desired behavior.** Remove the Window control from the **title bar**. Keep it in the **icon bar**, placed on the **right side** next to the **Export** and **Terminal** buttons (which already live in the ribbon's right-aligned, right-to-left block).

**Current state (verified in code).**

- Title bar Window menu: `src/app/title_bar.rs` ~102–107 (`ui.menu_button(t!("menu.window.label"), …)` → `t!("menu.window.new_window")`), with the deferred handler at ~599–601 (`title_bar_new_window` flag → `handle_new_window`).
- Ribbon Window control (currently on the **left** group): `src/ui/ribbon.rs` ~285–301 (`ComboBox` "window_dropdown" → `RibbonAction::NewWindow`).
- Ribbon right block (RTL): Terminal `icon_button` (~391–401, `RibbonAction::ToggleTerminal`) and Export ComboBox (~403–462). Layout wrapper at ~389 `ui.with_layout(right_to_left(Align::Center), …)`.
- Handler `handle_new_window` (`src/app/windows.rs` ~220–225); shortcut already wired (`ShortcutCommand::NewWindow`, Ctrl+Shift+N).

**Recommended approach.**

1. Delete the title-bar Window menu (`title_bar.rs` ~102–107) and its now-unused `title_bar_new_window` flag/handler (~78, ~599–601) if nothing else uses them.
2. Move the window control out of the ribbon's left group (~285–301) and into the right-to-left block (~389–464), adjacent to Export/Terminal. Match the surrounding style — prefer the compact `icon_button` style (consistent with Terminal) or a small ComboBox like Export; reuse `RibbonAction::NewWindow` and `handle_new_window`. Choose an appropriate phosphor icon for a window/new-window action.
3. Keep the `menu.window.*` i18n keys (or repoint to a tooltip key) so no raw keys show.

**Key files.** `src/app/title_bar.rs`, `src/ui/ribbon.rs`, `src/app/mod.rs` (ribbon action wiring ~2306–2308), `src/app/windows.rs`, `locales/en.yaml`.

**Acceptance criteria.**

- No Window control in the title bar.
- A window/new-window control sits in the icon bar's right cluster beside Export and Terminal, with a tooltip (incl. the Ctrl+Shift+N hint) and proper English label.
- New Window still works from the icon bar and the keyboard shortcut.
- Zen mode (which hides both bars) is unaffected.

---

### 5.7 Right-click context menu in the raw editor

**Problem.** The raw text editor (FerriteEditor / EditorWidget) has no right-click context menu. Add a simple one: **Copy, Cut, Paste, Select All** (and optionally **Undo**).

**Current state (verified in code).** No `context_menu` anywhere under `src/editor/`. Clipboard logic lives inside `FerriteEditor::ui()`'s event loop (only when focused): Copy (`src/editor/ferrite/editor.rs` ~2391–2396 / Ctrl+C ~2443–2449), Cut (~2398–2409 / Ctrl+X ~2451–2467), Paste (`Event::Paste` ~2551–2554), Select All (Ctrl+A ~2438–2441). Public helpers: `selected_text()` (~657–688), `delete_selection()` (~745+), `select_all()` (`selection.rs` ~88–98); `insert_text_at_all_cursors()` is private. The editor `Response` is returned at `editor.rs` ~2781 and consumed in `EditorWidget::show()` (`src/editor/widget.rs` ~783).

**Reference pattern.** Use egui's `response.context_menu(|ui| { … })` like the file tree (`src/ui/file_tree.rs` ~417–419, items ~635–698) and terminal (`src/ui/terminal_panel.rs` ~1466+): menu items are `ui.button(...).clicked()` followed by `ui.close()`.

**Recommended approach.**

1. Attach `response.context_menu(…)` to the editor `Response` (in `widget.rs` after ~783, or just before `return response` in `editor.rs` ~2781).
2. Menu items: Copy, Cut, Paste, Select All (optionally Undo). Disable Copy/Cut when there is no selection (`has_any_selection`). 
3. Implement actions through existing mechanisms: Copy/Cut via `selected_text()` + `ui.copy_text()` + `delete_selection()`; Select All via `select_all()`; Paste by reading the clipboard (mirror the terminal's `arboard::Clipboard` usage) or by injecting an `Event::Paste`, then `insert_text_at_all_cursors` (add a small public wrapper if needed). Undo should call the app-level `handle_undo()` (`src/app/navigation.rs` ~300–331), not the editor's internal history.
4. Show shortcut hints on items where convenient (optional). All labels via `t!()` with keys in `en.yaml` (reuse existing copy/cut/paste/select-all keys if present, else add them).

**Key files.** `src/editor/widget.rs`, `src/editor/ferrite/editor.rs`, `src/editor/ferrite/selection.rs`, reference `src/ui/file_tree.rs` / `src/ui/terminal_panel.rs`, `locales/en.yaml`.

**Acceptance criteria.**

- Right-clicking in the raw editor opens a menu with Copy/Cut/Paste/Select All (and Undo if included).
- Each action behaves identically to its keyboard shortcut; Copy/Cut disabled with no selection.
- Menu uses standard egui hover/click styling and localized labels.
- Works in both the main raw editor and the split-view raw pane.

---

### 5.8 Tab context-menu polish

**Problem.** The tab right-click menu (added in v0.3.1) has three issues:
(a) no hover highlight on menu items; (b) it's too wide/big; (c) it shows raw keys **`tab.new_tab`** and **`tab.close`** instead of English labels.

**Root cause (verified in code).** The menu is rendered in `src/ui/action_registry.rs` `render_action_menu_with_shortcuts` (~99–149) using **custom-drawn rows**: a `ui.horizontal` with `ui.label(...)` plus a separate `ui.interact(row.rect, …, Sense::click())` (~118–145). Because items are plain labels (not `ui.button`/`selectable_label`), egui applies no hover styling. Fixed widths force the size: popup frame `ui.set_min_width(230.0)` (`src/app/central_panel.rs` ~724) and per-row `ui.set_min_width(210.0)` (`action_registry.rs` ~120). The labels use `t!("tab.new_tab")` and `t!("tab.close")` (~44–51), but **`tab.new_tab` and `tab.close` are not defined** in `locales/en.yaml` (the `tab:` section ~898–899 only has `reveal_in_explorer`). rust_i18n returns the key string for missing keys, and `localized_label` only substitutes its fallback when the label is **empty**, so the raw key is displayed.

**Desired behavior.** Menu items have hover feedback consistent with the rest of the app (compare the file-tree context menu), the menu is appropriately sized (not overly wide), and labels read "New Tab" / "Close Tab" in English.

**Recommended approach.**

1. **i18n:** add `tab.new_tab: "New Tab"` and `tab.close: "Close Tab"` under the top-level `tab:` section in `en.yaml` (and other locale files as the repo convention dictates). Alternatively repoint to existing keys (`shortcuts.commands.new_tab` / `…close_tab`, `tooltip.new_tab`). Prefer adding the `tab.*` keys for clarity.
2. **Hover:** render rows as interactive widgets (`ui.button(...)` / `selectable_label`) like `file_tree.rs` (~643–648), or paint a hover background when `response.hovered()`. Keep the optional right-aligned shortcut hint.
3. **Width:** reduce/remove the fixed `set_min_width(230.0)` and `set_min_width(210.0)`; let the menu size to content (mirror the file-tree menu).

**Key files.** `src/ui/action_registry.rs` (`render_action_menu_with_shortcuts`, `actions_for`, `localized_label`), `src/app/central_panel.rs` (popup frame ~710–761), `locales/en.yaml`.

**Acceptance criteria.**

- Tab right-click menu shows "New Tab", "Close Tab" (and Copy Path / Reveal in Explorer when applicable) — no raw keys.
- Items highlight on hover like other menus.
- Menu width is reasonable for its content.
- Existing actions (new tab, close, copy path, reveal) still work.

---

### 5.9 "Lock editing" in the command palette

**Problem.** Preview lock (`Tab::preview_locked`, padlock overlay) can only be toggled via the on-pane padlock overlay. Add a **"Lock editing"** toggle to the command palette.

**Current state (verified in code).** Per-tab flag with `Tab::is_preview_locked()` / `toggle_preview_locked()` (`src/state.rs` ~2791–2799), persisted in session. The only toggle UI is `render_preview_lock_overlay` (`src/app/central_panel.rs` ~83–173, callback `tab.toggle_preview_locked()` at ~2271+ and other panes). There is **no** `ShortcutCommand` variant and **no** `handle_toggle_preview_lock()`.

**Palette registration pattern (reference: `ToggleWordWrap`, end-to-end).**

- `src/config/settings.rs`: add the variant to `ShortcutCommand` (~583), to `all()` (~663), `display_name()` (~740), `category()` → "View" (~817), optional default `KeyBinding` (~897).
- `src/app/commands.rs`: add icon in `icon_for_command()` (~39–121) — reuse `LOCK`/`LOCK_OPEN` from `phosphor_icons`.
- `src/ui/settings.rs`: add the localized name arm in `shortcut_command_name()` (~47+).
- `locales/en.yaml`: add `shortcuts.commands.toggle_preview_lock` (and the display string).
- `src/app/navigation.rs`: add `handle_toggle_preview_lock()` (toggle active tab's `preview_locked`, optional toast).
- `src/app/central_panel.rs`: add the arm in `dispatch_palette_command()` (~3751–3905) → `self.handle_toggle_preview_lock()`.
- The palette auto-includes any command in `ShortcutCommand::all()` — no change needed in `command_palette.rs`.
- Optional keyboard shortcut requires a `KeyboardAction` variant + `check_shortcut!` in `app/types.rs` / `app/keyboard.rs` (mirror word-wrap steps); not required for this PRD.

**Desired behavior.** A "Lock editing" command appears in the command palette; invoking it toggles the active tab's preview lock (same effect as the padlock overlay), with feedback (toast and/or the overlay icon state). Since the flag is per-tab and flows into preview widgets next frame, it works regardless of current view mode; consider a toast when the active tab is raw-only (overlay hidden) so the action is discoverable.

**Key files.** `src/config/settings.rs`, `src/app/commands.rs`, `src/ui/settings.rs`, `src/app/navigation.rs`, `src/app/central_panel.rs`, `locales/en.yaml`. Docs: `docs/technical/ui/preview-lock.md` / `command-palette.md`.

**Acceptance criteria.**

- "Lock editing" (or similar) appears in the command palette and toggles the active tab's preview lock.
- The padlock overlay state reflects the toggle; a toast confirms the change.
- Localized label (no raw key); existing overlay toggle still works.
- `cargo test` / `cargo clippy` clean.

---

## 6. Suggested priority / sizing (guidance for the parser, not prescriptive)

| Requirement | Impact | Complexity | Notes |
|-------------|--------|------------|-------|
| 5.1 Rendered-view edit sync | High (correctness) | High | Likely 2–3 subtasks: flush points; range/duplication fix; Enter semantics + tests |
| 5.4 External reload modified-state | High (correctness) | Low | One-line fix + test, mirrors existing recovery path |
| 5.3 Table alignment rendering | Medium | Medium | egui galley offset / full-width layout |
| 5.2 CSV cell navigation | Medium | Medium | Mirror `EditableTable` Tab/Shift+Tab + arrows |
| 5.5 Video embeds (syntax + resize) | Medium | Medium | Split: 5.5a require syntax (small), 5.5b resizable (medium) |
| 5.6 Window control consolidation | Low | Low | UI move + delete |
| 5.8 Tab menu polish | Low | Low | i18n keys + hover + width |
| 5.7 Raw editor context menu | Low | Low–Medium | New `context_menu`, reuse clipboard ops |
| 5.9 Lock editing in palette | Low | Low | Follow `ToggleWordWrap` pattern |

Requirements are largely independent and can be parsed into separate parent tasks. 5.1 is the most valuable and should not be bundled with unrelated items.

## 7. Testing & verification

- **Automated (`cargo test`):** range/duplication math for 5.1; modified-state after external reload for 5.4; video param parsing + bare-link removal for 5.5; CSV navigation/wrap helper for 5.2 if factored out; table alignment positioning assertion for 5.3 if feasible.
- **Lint:** `cargo clippy` clean for all changed crates.
- **Manual smoke tests** (record in the relevant feature doc):
  - 5.1: single-paragraph edit → Raw reflects it; Enter→`test2` no duplication (rendered-only and split); Ctrl+E flushes.
  - 5.2: Tab/Shift+Tab/arrows across CSV cells; click-then-arrow; preview-locked CSV.
  - 5.3: centered/right columns visibly aligned; click-to-edit cursor accuracy.
  - 5.4: external edit in another app → no `*`, no close prompt.
  - 5.5: bare link = link; `{{video …}}` = player; width/height applied + round-trip.
  - 5.6: window control only in icon bar (right cluster); New Window works.
  - 5.7: raw editor right-click Copy/Cut/Paste/Select All.
  - 5.8: tab menu labels English, hover works, width reasonable.
  - 5.9: palette "Lock editing" toggles lock + overlay reflects it.
- **No regressions:** rendered-session RS-1…RS-7, preview-lock matrix, multi-window, and split-view sync must still pass.

## 8. Key-files reference (consolidated)

> Line numbers are from investigation snapshots and may drift — locate by symbol name.

| Area | Files / symbols |
|------|-----------------|
| Rendered edit session (5.1) | `src/markdown/rendered_session.rs` (`RenderedEditSession`, `commit_active`, `close_active`, `switch_to_ui`, `invalidate_buffers`, `BlockRef`); `src/markdown/editor.rs` (`render_session_plain_text_block`, `write_session_block_to_source`, `commit_session_block`, `update_source_range`, `show_rendered_editor`); `src/app/navigation.rs` (`handle_toggle_view_mode`); `src/state.rs` (`source_epoch`, `notify_external_content_change`) |
| CSV navigation (5.2) | `src/markdown/csv_viewer.rs` (`CsvViewerState`, `render_row_cells`, `show_table_view`, `begin_cell_edit`, `queue_cell_commit`, `move_selected_cell`, `CsvCellEditParams`); ref `src/markdown/widgets.rs` (`TableEditState`, `request_table_cell_focus`) |
| Table alignment (5.3) | `src/markdown/widgets.rs` (`EditableTable::show` display branch, `table_alignment_to_egui`, `table_cell_raw_cursor_at_click`); `src/markdown/parser.rs` (`TableAlignment`); ref `src/markdown/editor.rs` (`with_block_align_widget`, `render_table`) |
| External reload (5.4) | `src/app/file_ops.rs` (`handle_file_watcher_events`); `src/state.rs` (`mark_saved`, `is_modified`, `apply_reload_from_disk_for_conflict`); `src/workspaces/watcher.rs` |
| Video embeds (5.5) | `src/markdown/video_embed.rs` (`try_parse_video_paragraph`, `parse_braced_video_syntax`, `extract_bare_youtube_url`); `src/markdown/parser.rs` (`VideoEmbedInfo`); `src/markdown/video_render.rs` (`video_display_size`, `render_video_embed`); `src/markdown/widgets.rs` (`serialize_node`); `README.md` |
| Window controls (5.6) | `src/app/title_bar.rs`; `src/ui/ribbon.rs` (`Ribbon::show`, `RibbonAction::NewWindow`, `icon_button`); `src/app/mod.rs`; `src/app/windows.rs` (`handle_new_window`) |
| Raw editor menu (5.7) | `src/editor/widget.rs`; `src/editor/ferrite/editor.rs` (`ui`, `selected_text`, `delete_selection`); `src/editor/ferrite/selection.rs` (`select_all`); ref `src/ui/file_tree.rs`, `src/ui/terminal_panel.rs` |
| Tab menu (5.8) | `src/ui/action_registry.rs` (`render_action_menu_with_shortcuts`, `actions_for`, `localized_label`); `src/app/central_panel.rs` (popup); `locales/en.yaml` |
| Lock in palette (5.9) | `src/config/settings.rs` (`ShortcutCommand`); `src/app/commands.rs` (`icon_for_command`); `src/ui/settings.rs` (`shortcut_command_name`); `src/app/navigation.rs`; `src/app/central_panel.rs` (`dispatch_palette_command`); `src/state.rs` (`toggle_preview_locked`); `locales/en.yaml` |

## 9. Documentation deliverables (update phase)

Per `ai-context.md`, docs are written/updated in the **update phase**, by feature, and registered in `docs/index.md`:

- Update `rendered-edit-session.md` (+ paragraphs/lists sub-doc) — flush points, Enter semantics, range fix.
- Update `csv-viewer.md` — cell navigation keys.
- Update `gfm-table-column-alignment.md` — actual render mechanism.
- Update `session-persistence.md` (or file-watching doc) — external-reload clean-state fix + workspace-only scope.
- Update `video-embed-parsing.md` + `video-embed-rendering.md` + `README.md` — explicit-syntax requirement, sizing params.
- Update `preview-lock.md` / `command-palette.md` — new palette command.
- Brief notes for the raw-editor context menu, tab-menu polish, and window-control relocation in the relevant UI docs.
- Reflect shipped items in `CHANGELOG.md` (Unreleased) and tick the matching `ROADMAP.md` follow-ups (e.g. CSV Tab/Shift+Tab).
