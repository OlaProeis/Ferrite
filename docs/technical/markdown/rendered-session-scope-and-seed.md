# Rendered Session — Scope-Stable Widget Ids, Cold Seeds, Active-Only Live Writes

Round-3 QA closeout for v0.3.1 (GrokBot ISS-64, ISS-65, ISS-68; test rows T1-14-adjacent T1-17/18/19, ARR-2/4, UND-1/3, HTML-6). Three defects in the rendered edit session shared one shape: state that *looked* correct in unit tests but was keyed or flagged in a way the live frame loop misread.

**Related:** [Per-frame write-through](./rendered-session-write-through.md), [Widget identity / `source_epoch`](./rendered-widget-identity.md), [Arrow navigation](./rendered-arrow-navigation.md), [Headings](./rendered-edit-session-headings.md), [Undo](./rendered-edit-session-undo.md)

## 1. Scope-stable widget ids (ISS-64 — arrow navigation re-grabs previous block)

### Symptom

Arrow Down/Up out of a rendered block moved the caret visually for one frame, then the previous block re-took focus. Worse inside blockquotes (T1-18), where a block's TextEdit is nested two `Ui`s deeper than its siblings.

### Root cause

`BlockRef::widget_id(ui)` was `self.widget_id_in_scope(ui.id())`. egui child `Ui` ids are `parent.id.with(...)`, so the id a block computed **inside** `ui.horizontal(...)` / blockquote indent differed from the id the session computed at scope level in `apply_pending_arrow_navigation` and `surrender_focus`. The focus comparison never matched, `surrender_focus` released a non-existent id, the old TextEdit kept egui focus, and `load_for_epoch_ctx` re-activated the old block on the next frame. The round-2 fix for ARR-2 was correct at the session layer and never worked end-to-end.

### Fix (`src/markdown/rendered_session.rs`)

```rust
fn rendered_widget_scope_key() -> Id { Id::new("ferrite.rendered_widget_scope") }
pub fn set_rendered_widget_scope(ui: &Ui)   // stores ui.id() in temp memory
pub fn rendered_widget_scope(ui: &Ui) -> Id // reads it; falls back to ui.id()

impl BlockRef {
    pub fn widget_id(self, ui: &Ui) -> Id {
        self.widget_id_in_scope(rendered_widget_scope(ui))
    }
}
```

`show_rendered_editor` calls `set_rendered_widget_scope(ui)` immediately inside `push_id(editor_id) → push_id(source_epoch)`, so every block — regardless of nesting depth — resolves the same scope id. `widget_id_in_scope` suffixes are unchanged.

`surrender_focus` now computes the id **before** taking `memory_mut`; the scope lookup reads memory, and egui's context lock is not re-entrant (the first attempt deadlocked nine session tests with "Failed to acquire RwLock read after 10s").

Test: `widget_id_is_scope_stable_across_nested_uis` — ids computed in nested `horizontal`/`vertical` uis equal the scope-level id, and `surrender_focus` from a nested ui clears focus set at scope level.

## 2. Non-dirty cold seeds (ISS-68 / ISS-65 — headings rewritten, document shrinks)

### Symptom

Opening a document in rendered view marked the tab unsaved without any edit (HTML-6, MMD dirty-on-view). After editing a code block, headings containing inline code or HTML lost characters (6376 → 6374 chars on `test_github_html.md`), and undo split into two steps that left a heading written into a blank line (T1-19, UND-1/3).

### Root cause

`render_heading` seeded a not-yet-edited heading into the session via `on_text_changed`, which sets `dirty = true`. That had been harmless since `041d8ec` because nothing consumed cold dirty blocks — until the round-2 write-through (`apply_rendered_session_live_frame`) started committing **every** dirty block each frame. Two consequences:

1. `heading_plain_text` strips backticks / inline HTML, so every frame rewrote headings with the lossy plain text.
2. After a code-block commit shifted source lines, a stale `Heading { line: 49 }` still sat dirty in the session and was written into whatever now lived on line 49 — a blank line.

### Fix

`RenderedEditSession::seed_block_text(block, text) -> bool` inserts a `BlockEditState` with `dirty = false` and returns `false` if the block already exists. `render_heading`, `ensure_formatted_block_initialized`, `render_session_plain_text_block`, and `seed_session_block_text` all seed through it; only real user input goes through `on_text_changed`.

Test: `test_live_frame_does_not_write_cold_seeded_headings` (`markdown::editor::tests`) — a seeded heading plus a live frame leaves `tab.content` byte-identical and the tab unmodified.

## 3. Active-only live write-through

Defence in depth for §2. `apply_rendered_session_live_frame` now writes **only** the active block, and only when dirty and not a `TableCell` (table cells persist via `TableEditState`):

```rust
let live_block = session.active
    .filter(|b| !matches!(b, BlockRef::TableCell { .. }))
    .and_then(|b| session.blocks.get(&b).map(|s| (b, s)))
    .filter(|(_, s)| s.dirty);
```

Inactive dirty blocks are still committed by `close_active(SaveIfDirty)` / the flush path on focus change, exactly as before round 2. A stale block can no longer be written while another block is being typed into.

## 4. Heading commit guard

`heading_level_from_source(source, line) -> Option<HeadingLevel>` returns `None` when the line no longer starts with `#`. `write_session_block_to_source` skips the Heading branch with

```
log::warn!("Skipping heading commit: source line {line} is not a heading any more (stale session block {block:?})")
```

instead of overwriting a non-heading line. Line indices are 0-based throughout these helpers.

Test: `test_stale_heading_commit_into_blank_line_is_skipped`.

## Files

| File | Change |
|------|--------|
| `src/markdown/rendered_session.rs` | scope key helpers, `BlockRef::widget_id` via scope, `seed_block_text`, `surrender_focus` lock order |
| `src/markdown/editor.rs` | `set_rendered_widget_scope` inside push_id chain; cold seeds via `seed_block_text`; active-only live write; `heading_level_from_source` → `Option` + warn skip |

## Verification

```
cargo test --bin ferrite -- markdown::rendered_session markdown::editor::tests
```

Linux GrokBot re-run rows: T1-17, T1-18, T1-19, ARR-2, ARR-4, UND-1, UND-3, HTML-6 (see `grokbot/november/rerun-v0.3.1-round3.md`).
