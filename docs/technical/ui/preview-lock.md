# Preview Lock (#144)

Per-tab flag that marks preview panes as read-only. Phases 1–4 (state, overlay, markdown WYSIWYG, CSV/Tree gating) are implemented. Full UX matrix and regression checklist: [Preview lock mode](../markdown/preview-lock-mode.md).

## State model

| Location | Field / API | Notes |
|----------|-------------|--------|
| `Tab` (`state.rs`) | `preview_locked: bool` | Default `false` on new tabs |
| `Tab` | `is_preview_locked()` | O(1) read for per-frame gating |
| `Tab` | `toggle_preview_locked()` | Flips flag; returns new state |

The flag is **per tab**, not global. It survives view-mode switches (`Raw` / `Split` / `Rendered`) and active-tab changes. Nothing resets it except an explicit toggle or session restore.

## Session persistence

| Location | Field | Notes |
|----------|-------|--------|
| `SessionTabState` (`config/session.rs`) | `preview_locked: bool` | `#[serde(default)]` — legacy session JSON without the field deserializes as unlocked |

**Capture:** `AppState::tab_to_session_tab_state` copies `tab.preview_locked` into each persisted tab.

**Restore:** `restore_from_session_result` assigns `tab.preview_locked = session_tab.preview_locked` after tab construction (alongside `view_mode`, cursor, scroll).

Works in both session schema v1 (flat tabs) and v2 (multi-window `windows[].tabs`).

## Padlock overlay (Phase 2)

Bottom-right Phosphor padlock on every **preview** pane in `src/app/central_panel.rs`.

| Helper | Role |
|--------|------|
| `render_preview_lock_overlay` | Paints `LOCK` / `LOCK_OPEN`, hover chrome, click hit-test; returns `true` when toggled |
| `preview_lock_button_colors` | Dark/light semi-transparent button background and icon tint |

### Pane coverage

| View mode | Pane | Overlay ID suffix |
|-----------|------|-------------------|
| `Rendered` | Markdown `MarkdownEditor` | `preview_lock_rendered` |
| `Rendered` | CSV/TSV `CsvViewer` | `preview_lock_csv` |
| `Rendered` | JSON/YAML/TOML `TreeViewer` | `preview_lock_tree` |
| `Split` | Right preview (markdown or CSV) | `preview_lock_split` |

**No padlock** on the raw editor pane (`ViewMode::Raw`, split left pane). Structured files (JSON/YAML/TOML) do not support split view — `central_panel` forces `ViewMode::Raw` when split is requested.

Each overlay ID is suffixed with `tab_id` for per-tab independence. Click calls `tab.toggle_preview_locked()` on the active tab.

### Visual behavior

- Fixed position: bottom-right of pane rect, `28×28` button, `10px` margin.
- Skips render when pane is too small to fit button + margins.
- When locked: subtle **"Preview"** hint (`settings.preview.locked_hint`) to the left of the icon.
- Tooltips: `settings.preview.lock_tooltip` / `unlock_tooltip` in `locales/en.yaml`.
- Painted on `Order::Middle` layer so it sits above scroll content.

## Markdown WYSIWYG gating (Phase 3)

When `tab.preview_locked`, rendered markdown mutations are blocked; read-only interactions remain enabled. The split **raw** pane is never affected.

### Flag plumbing

| Location | Role |
|----------|------|
| `MarkdownEditor::preview_locked(bool)` | Builder; set from `central_panel.rs` via `tab.is_preview_locked()` |
| `preview_locked_temp_id()` (`markdown/mod.rs`) | Per-frame egui temp bool; O(1) read via `preview_locked_from_ui` / `preview_locked_from_ctx` |
| `RenderedEditSession` | `switch_to_ui` no-ops when locked; `close_active_ui` maps `SaveIfDirty` → `Discard` |

Active edit sessions are discarded (no commit) at the start of each locked rendered frame.

### Gating points

| Area | Module | Locked behaviour |
|------|--------|------------------|
| Session activation / blur commit | `rendered_session.rs`, `editor.rs` | No `switch_to_ui`; no source commits |
| Headings, paragraphs, formatted blocks | `editor.rs` | `TextEdit::interactive(false)`; display click does not enter edit |
| Task checkboxes | `editor.rs` | Disabled checkbox widget; no source toggle |
| Tables | `widgets.rs` (`EditableTable`) | Display-only cells; no toolbar, resize, or cell focus |
| Code blocks | `widgets.rs` (`EditableCodeBlock`) | Edit button hidden; edit mode forced off; Run/Copy still work |
| Structural edits | `editor.rs` | Pending structural edits not applied |
| Links, wikilink nav, video, scroll, zoom, copy | unchanged | Read actions stay enabled |

Unit test: `rendered_session::tests::preview_locked_blocks_switch_to_ui_and_discards_on_close`.

## CSV / Tree gating (Phase 4)

When `tab.preview_locked`, rendered CSV and tree mutations are blocked; navigation and read-only actions remain. See [Preview lock mode](../markdown/preview-lock-mode.md) for the full behaviour matrix.

| Viewer | Builder | Locked behaviour |
|--------|---------|------------------|
| `CsvViewer` | `preview_locked(bool)` | No cell inline edit (double-click / Enter); selection + arrow keys OK |
| `TreeViewer` | `preview_locked(bool)` | No leaf inline edit (double-click); expand/collapse + copy path OK |

Active CSV cell edits and tree inline edits are discarded at the start of each locked frame (no commit).

## Tests

- `config/session.rs`: legacy tab JSON defaults unlocked; round-trip with `preview_locked: true`
- `state.rs`: `test_tab_toggle_preview_locked`, view-mode persistence, full capture/restore roundtrip

## Roadmap (same epic)

| Phase | Task | Scope |
|-------|------|--------|
| 1 | 28 | State + session — **done** |
| 2 | 29 | Padlock overlay on preview panes (`central_panel.rs`) — **done** |
| 3 | 30 | Gate markdown WYSIWYG mutations — **done** |
| 4 | 31 | Gate CSV / Tree viewer edits + regression doc — **done** |

Related: [Session Persistence](../files/session-persistence.md), [Split View](./split-view.md), [View Mode Persistence](../view-mode-persistence.md).
