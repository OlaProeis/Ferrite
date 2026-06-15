# Preview Lock Mode

Per-tab read-only mode for **preview panes** (rendered markdown, CSV/TSV table, JSON/YAML/TOML tree). Users toggle it via the bottom-right padlock overlay. Implementation overview: [Preview lock](../ui/preview-lock.md).

## UX

| Action | Locked | Unlocked |
|--------|--------|----------|
| Toggle padlock on preview pane | Unlocks tab | Locks tab |
| Switch view mode (Raw / Split / Rendered) | Lock state persists | Lock state persists |
| Switch active tab | Each tab keeps its own lock | Each tab keeps its own lock |
| Restart app | Lock restored from session | Lock restored from session |
| Edit in **raw** pane | Always allowed (split left, Raw mode) | Always allowed |

When locked, a subtle **"Preview"** hint appears beside the padlock (`settings.preview.locked_hint`). Tooltips: `settings.preview.lock_tooltip` / `unlock_tooltip`.

## State model

| Location | Field / API | Notes |
|----------|-------------|--------|
| `Tab` (`state.rs`) | `preview_locked: bool` | Default `false`; per-tab, not global |
| `Tab` | `is_preview_locked()` | O(1) read each frame |
| `Tab` | `toggle_preview_locked()` | Flips flag; returns new state |
| `SessionTabState` (`config/session.rs`) | `preview_locked: bool` | `#[serde(default)]` for legacy sessions |

**Capture / restore:** `tab_to_session_tab_state` and `restore_from_session_result` round-trip the flag with view mode, cursor, and scroll. Works in session v1 (flat tabs) and v2 (multi-window).

**Per-frame plumbing:** `preview_locked_temp_id()` in `markdown/mod.rs` — stamped by `MarkdownEditor`, `CsvViewer`, and `TreeViewer` at the start of each `show()` call. Child widgets read via `preview_locked_from_ui(ui)` (O(1)).

## Gating points

### Padlock overlay (all preview panes)

`render_preview_lock_overlay` in `central_panel.rs` — painted on rendered markdown, CSV, tree, and split preview panes. **Not** on raw editor panes.

### Markdown WYSIWYG

See [Preview lock § Markdown WYSIWYG gating](../ui/preview-lock.md#markdown-wysiwyg-gating-phase-3). Split **raw** pane is never gated.

### CSV / TSV (`csv_viewer.rs`)

`CsvViewer::preview_locked(bool)` from `central_panel.rs`. Files ≥ 1 MB already disable rendered editing (lazy read-only table).

When locked on editable files (< 1 MB):

| Interaction | Behaviour |
|-------------|-----------|
| Click cell | Selection highlight — **enabled** |
| Arrow keys | Move selection — **enabled** |
| Double-click cell | No inline editor |
| Enter on selected cell | No inline editor |
| Active inline edit | Discarded at frame start; `pending_commit` cleared |
| Copy (system / selection) | **Enabled** (native text selection where applicable) |
| Scroll, tooltips | **Enabled** |
| Raw view toggle / delimiter toolbar | **Enabled** (non-mutating UI) |

Implementation: `CsvCellEditParams` splits `navigation_enabled` (selection + arrows) from `cell_edit_enabled` (`!preview_locked`).

### JSON / YAML / TOML tree (`tree_viewer.rs`)

`TreeViewer::preview_locked(bool)` from `central_panel.rs`.

| Interaction | Behaviour |
|-------------|-----------|
| Expand / collapse | **Enabled** |
| Copy path (context menu) | **Enabled** |
| Double-click leaf value | No inline editor |
| Active inline edit | Cancelled at frame start |
| Scroll | **Enabled** |

Structured files do not support split view; use **Raw** mode for full text editing.

## Behaviour matrix

| Pane | View mode | Lock overlay | Mutations when locked |
|------|-----------|--------------|------------------------|
| Markdown WYSIWYG | Rendered | Yes | Blocked (headings, paragraphs, tables, code edit, tasks) |
| Markdown WYSIWYG | Split right | Yes | Blocked |
| Markdown raw editor | Raw | No | **Allowed** |
| Markdown raw editor | Split left | No | **Allowed** |
| CSV / TSV table | Rendered | Yes | Cell inline edit blocked; navigation OK |
| CSV / TSV table | Split right | Yes | Same as rendered |
| CSV / TSV | Raw (if used) | No | **Allowed** |
| JSON / YAML / TOML tree | Rendered | Yes | Inline value edit blocked |
| JSON / YAML / TOML | Raw | No | **Allowed** |

Read-only actions that stay enabled everywhere on preview panes: link navigation, wikilinks, video embed view, scroll sync, zoom, code block Run/Copy (markdown), Mermaid pan/zoom.

## Regression checklist (RS-1…RS-7 + CSV/Tree)

Manual flows to verify after changes:

| ID | Flow |
|----|------|
| RS-1 | New tab defaults unlocked; padlock shows open icon |
| RS-2 | Lock markdown rendered → no WYSIWYG edit; raw/split-left still editable |
| RS-3 | Lock CSV tab → double-click/Enter no cell editor; arrows navigate |
| RS-4 | Lock tree tab → double-click leaf no inline edit; expand/copy still work |
| RS-5 | Toggle lock off → editing restored on markdown, CSV, tree |
| RS-6 | Locked + unlocked tabs → switch tabs; each retains its lock state |
| RS-7 | Lock → change view mode → lock persists; restart → lock persists per tab |

## Tests

| Area | Location |
|------|----------|
| Session round-trip | `config/session.rs` |
| Tab toggle / capture | `state.rs` |
| Markdown session gating | `rendered_session.rs` (`preview_locked_blocks_switch_to_ui_and_discards_on_close`) |
| CSV edit threshold | `csv_viewer.rs` (`test_csv_rendered_editing_enabled_threshold`) |

## Related

- [Preview lock](../ui/preview-lock.md) — padlock UI and markdown gating detail
- [Session persistence](../files/session-persistence.md)
- [Split view](../ui/split-view.md)
