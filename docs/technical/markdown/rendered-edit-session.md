# Rendered Edit Session — Architecture Overview

Status: **implemented** (Phases 0–5; legacy `rendered_focus` removed in task 104)

Rendered (WYSIWYG) mode editing is coordinated by a **single per-tab session** instead of per-widget focus/defer hacks. One block (heading, paragraph, list item, formatted block, or table cell) is active at a time; switching blocks commits the previous buffer and opens the new target in **one click**.

**PRD:** [`prd-rendered-edit-session.md`](../../ai-workflow/prds/prd-rendered-edit-session.md)

---

## Why the prior architecture failed

| Symptom | Root cause |
|---------|------------|
| Double-click to switch blocks | egui consumed the first click to defocus; commit → re-parse → id remap raced with new focus |
| Cursor flash / disappear | `ui.push_id(content_hash, …)` remapped all TextEdit ids on every keystroke commit |
| Stuck in raw formatted edit | `formatted_exit_should_save` deferred on blur but was never re-checked on later frames |
| Tables vs headings behaved differently | Separate defer paths (`TableGlobalFocus`, `rendered_focus`, `FormattedItemEditState`) with no single owner |

**Fix:** Stable widget ids (`source_epoch` scope) + explicit `switch_to_ui` at click boundaries + session-owned buffers.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Tab / MarkdownEditor                                        │
│  ┌─────────────────────┐    ┌──────────────────────────────┐ │
│  │ Tab::source_epoch   │    │ RenderedEditSession          │ │
│  │ (external invalid.) │    │  active: Option<BlockRef>    │ │
│  └──────────┬──────────┘    │  blocks: HashMap<BlockRef>   │ │
│             │               │  switch_to / close / commit  │ │
│             v               └──────────────┬───────────────┘ │
│  ui.push_id(editor_id)                     │                 │
│    ui.push_id(source_epoch)  ◄─────────────┘                 │
│      render blocks → session at click/focus boundaries       │
└─────────────────────────────────────────────────────────────┘
```

| Module | Role |
|--------|------|
| `src/markdown/rendered_session.rs` | Types, state machine, tab-scoped egui storage |
| `src/markdown/editor.rs` | Render paths, `switch_to_ui`, dismiss, table sync |
| `src/markdown/widgets.rs` | `EditableTable`, force-commit signal, cell focus |
| `src/markdown/rendered_commit_undo.rs` | One undo step per commit boundary |
| `src/state.rs` | `Tab::source_epoch()`, bump helpers |

---

## `source_epoch` — widget identity scope

Rendered TextEdit ids live under `ui.push_id(editor_id)` + `ui.push_id(source_epoch)`, **not** `content_hash`. Viewport culling still uses content hash independently.

See [`rendered-widget-identity.md`](./rendered-widget-identity.md) and [`rendered-edit-session-phase0.md`](./rendered-edit-session-phase0.md).

| Event | Bumps `source_epoch`? |
|-------|------------------------|
| Rendered block commit (session close/switch) | **No** |
| Raw FerriteEditor edit (same tab) | **Yes** |
| Undo / redo / file reload / find-replace | **Yes** |
| Split raw pane edit | **Yes** (rendered pane reloads buffers on next frame) |
| Rendered keystrokes while block stays active | **No** |

On epoch mismatch, `load_for_epoch` calls `invalidate_buffers()` — session buffers reload from source on next interaction (RS-6).

---

## `BlockRef` — stable block identity

**Module:** `rendered_session::BlockRef`

| Variant | Identity fields | Widget id suffix (under scope) |
|---------|-----------------|--------------------------------|
| `Heading { line, structural }` | 1-indexed source line; `structural` → `*_sk` keys | `heading_text[_sk]` + line |
| `Paragraph { line }` | 1-indexed start line | `para_text` + line |
| `ListItem { line, item }` | Start line + enumerate index within list node | `list_item_text` + line (`item` is map key only) |
| `FormattedParagraph { line, structural }` | Same as paragraph | `formatted_paragraph[_sk]` + line + `text_edit` |
| `FormattedListItem { line, item, structural }` | Line + item index | `formatted_list_item[_sk]` + line + item + `text_edit` |
| `TableCell { table_line, row, col }` | Table start line (1-indexed) + cell coords | `table` + table_line + `cell` + row + col |

**Rules:**

- All `line` values are **1-indexed** (consistent with `EditState` / AST).
- List `item` matches the enumerate counter used when rendering (unique per sibling item).
- `structural: true` selects structural-key code paths (`render_*_with_structural_keys`).

---

## `RenderedEditSession` — data model & API

```rust
pub struct RenderedEditSession {
    pub active: Option<BlockRef>,
    pub blocks: HashMap<BlockRef, BlockEditState>,
}

pub struct BlockEditState {
    pub text: String,              // TextEdit buffer (raw markdown for formatted blocks)
    pub formatted_editing: bool,     // false = styled display, true = raw TextEdit
    pub dirty: bool,
    pub pending_activation: Option<PendingActivation>,
}
```

| Method | Behavior |
|--------|----------|
| `switch_to` / `switch_to_ui` | Close previous (`SaveIfDirty`), set active, queue `PendingActivation`; `_ui` variant surrenders previous egui focus |
| `close_active` / `close_active_ui` | Clear active; commit or discard per `CommitPolicy` |
| `on_text_changed` | Update buffer, mark dirty — **no source write** |
| `commit_active` | Force-commit active buffer via callback |
| `discard_active` | Reload buffer from source (Escape on formatted) |
| `invalidate_buffers` | Clear all state after epoch bump |
| `load_for_epoch` / `save_for_epoch` | Tab-scoped temp memory + epoch tracking |

Storage id: `editor_id.with("rendered_edit_session")`. Editor id: `rendered_editor_id(tab.id)` — shared by rendered-only and split preview ([split view doc](./rendered-edit-session-split-view.md)).

---

## Commit policy (by block type)

| Block type | While editing | On close / switch |
|------------|---------------|-------------------|
| Heading | Buffer in session | `update_source_line` via `commit_session_block` |
| Plain paragraph / list item | Buffer in session | `update_source_range` |
| Formatted paragraph / list item | Raw in buffer; display when `formatted_editing == false` | Save raw → source; set `formatted_editing = false` |
| Table cell | Text in `TableData` (widget), not session buffer | `signal_table_force_commit` when session leaves table; widget flushes on next frame |
| Code blocks / mermaid | **Not session-backed** | Existing widget-local commit paths unchanged |

**Close triggers:** `switch_to` another block; click-outside dismiss (`session_dismiss_if_clicked_outside`); Enter/Escape on formatted and plain blocks (see [paragraphs/lists](./rendered-edit-session-paragraphs-lists.md#enter--newline-semantics-plain-paragraphs--list-items)); TextEdit `lost_focus` on any session-backed block.

**Flush triggers (no block switch required):** view-mode toggle, tab switch, save, close/exit, primary viewport focus loss — see [`rendered-edit-flush.md`](./rendered-edit-flush.md).

**Undo:** One logical undo step per commit boundary — see [`rendered-edit-session-undo.md`](./rendered-edit-session-undo.md).

---

## Focus switch flow (old vs new)

### Prior architecture (removed)

```
User clicks B while A focused
  → egui defocuses A (first click)
  → A: lost_focus → defer commit → content_hash changes
  → all widget ids remap
  → B: request_focus / restore_switch_focus fights egui
  → cursor lost; formatted: editing flag stuck
```

### Current (session)

```
User clicks B while A active
  → session.switch_to_ui(B, PendingActivation { cursor, request_focus })
      → close A: save buffer if dirty, surrender A's egui focus (no epoch bump)
      → open B: set pending_activation
  → next frame: B TextEdit applies focus + cursor
  → stable ids throughout (source_epoch unchanged)
```

Tables: cross-block exit runs `commit_fn` → `signal_table_force_commit(table_line)`; intra-table Tab uses direct `session.active` assign (no commit). See [`rendered-edit-session-tables.md`](./rendered-edit-session-tables.md).

Formatted blocks: display click → galley cursor → `PendingActivation`. See [`click-to-edit-formatting.md`](./click-to-edit-formatting.md) and [`galley-cursor-positioning.md`](../editor/galley-cursor-positioning.md).

---

## Block-type documentation

| Block type | Detail doc |
|------------|------------|
| Core types & API | [`rendered-edit-session-core.md`](./rendered-edit-session-core.md) |
| Headings | [`rendered-edit-session-headings.md`](./rendered-edit-session-headings.md) |
| Paragraphs & lists | [`rendered-edit-session-paragraphs-lists.md`](./rendered-edit-session-paragraphs-lists.md) |
| Formatted blocks | [`rendered-edit-session-formatted.md`](./rendered-edit-session-formatted.md) |
| Tables | [`rendered-edit-session-tables.md`](./rendered-edit-session-tables.md) |
| Session flush (view/tab/save/close) | [`rendered-edit-flush.md`](./rendered-edit-flush.md) |
| Split view / RS-6 | [`rendered-edit-session-split-view.md`](./rendered-edit-session-split-view.md) |
| Undo | [`rendered-edit-session-undo.md`](./rendered-edit-session-undo.md) |
| Widget ids | [`rendered-widget-identity.md`](./rendered-widget-identity.md) |
| Phase 0 history | [`rendered-edit-session-phase0.md`](./rendered-edit-session-phase0.md) |

---

## Resolved design decisions (PRD open questions)

| Question | Decision |
|----------|----------|
| **Click outside document** | Save and close active block (`session_dismiss_if_clicked_outside` at end of rendered frame). Uses `session_active_clicked` flag — active block must receive `response.clicked()` or dismiss fires with `SaveIfDirty`. |
| **Buffer eviction** | Full `invalidate_buffers()` on `source_epoch` mismatch; no LRU cache of closed blocks. Re-open cold-loads from source via `ensure_formatted_block_initialized` / render paths. |
| **Code blocks / mermaid** | **Out of scope** for session in v0.3.x — separate widget-local edit state. |
| **Split conflict** | Raw pane wins on content: raw edit bumps epoch → rendered session invalidated. No simultaneous edit merge; rendered buffers reload from updated source. User should not expect live dual-pane co-editing of the same block without epoch refresh. |

---

## Regression matrix — rendered editing (RS-1…RS-7)

Manual acceptance tests for the session model. Full execution log: [`v0.3.0-regression-matrix.md`](../platform/v0.3.0-regression-matrix.md) §3.12.

| ID | Steps | Expected result |
|----|-------|-----------------|
| **RS-1** | Doc with `# Alpha` and `# Beta`. Edit Alpha text. Single-click Beta. | Beta focused at click; Alpha text persisted in source; **no second click** required. |
| **RS-2** | Click between characters in a heading. Type continuously for 3+ seconds. | Caret stays visible and stable; no flash/disappear each keystroke. |
| **RS-3** | Bullet item with `**bold**`. Click → edit raw → click empty space outside any block. | Returns to styled display; not stuck showing `**bold**` TextEdit. |
| **RS-4** | Two formatted list items. Edit first; single-click second. | Second enters edit; first saved; one click after typing. |
| **RS-5** | Doc with table + heading. Edit cell; single-click heading. | Table committed to source; heading focused; one click. |
| **RS-6** | Split view: edit raw left pane; observe rendered right. | Rendered reflects change after epoch bump (may need one frame / re-focus). Session buffers cleared — re-click block loads new source. |
| **RS-7** | Enable trace logging. Edit heading, switch away (commit). | `source_epoch` **unchanged** across rendered edit + commit; only external/raw edits bump. |

### Table scenarios (TBLE-1…TBLE-3)

Under session model, table **intra-grid** behaviour is unchanged; **cross-block** exit goes through session + force-commit signal. See §3.11 in the regression matrix.

| ID | Steps | Expected result |
|----|-------|-----------------|
| **TBLE-1** | Rendered/split: **Add column**; click each new empty cell; type; click outside table. | Each empty cell accepts click; edits commit when leaving table; source shows new `\|` columns. |
| **TBLE-2** | **Add row**; click empties; type; blur table. | New `\|` lines in source. |
| **TBLE-3** | From populated cell, Tab across several empty cells. | Each cell opens edit with empty buffer; Tab does **not** commit table mid-navigation; text only in active cell. |

---

## Automated tests

```bash
cargo test rendered_session::
```

Covers switch/commit/discard, widget id suffixes, table cross-block/intra-table, epoch invalidation, force-commit signal (see `widgets.rs` tests).

---

## Related user-facing docs

| Doc | Update |
|-----|--------|
| [`wysiwyg-editor.md`](./wysiwyg-editor.md) | Rendered mode overview + session pointer |
| [`click-to-edit-formatting.md`](./click-to-edit-formatting.md) | Session owns formatted buffers |
| [`table-editing-focus.md`](./table-editing-focus.md) | Session triggers commit on leave |
| [`table-cell-focus-navigation.md`](./table-cell-focus-navigation.md) | Tab/empty cells + session sync |
