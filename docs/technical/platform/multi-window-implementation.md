# Multi-Window Implementation (MVP — Task 15)

**Status:** Implemented (v0.3.1-experimental).  
**Design reference:** [multi-window.md](./multi-window.md)  
**Issue:** [#125](https://github.com/OlaProeis/Ferrite/issues/125)

## What shipped

| Feature | Behaviour |
|---------|-----------|
| **New Window** | Ribbon right cluster (New Window icon), command palette, **Ctrl+Shift+N** |
| **Independent tab strips** | Each OS window shows only its own tabs; global `Tab` store keyed by `Tab::id` |
| **Close secondary window** | Removes viewport and that window's tabs; primary unaffected |
| **Close last window** | Existing app-exit flow (unsaved prompts across all windows) |

**Session v2 restore:** see [multi-window-session.md](./multi-window-session.md). **Not in MVP:** Productivity Hub OS pop-out. **File routing + single-instance:** see [multi-window-file-routing.md](./multi-window-file-routing.md).

## State model (`src/state.rs`)

```text
AppState
├── windows: Vec<DocumentWindowState>   // per-window strip + geometry
├── focused_window_id: WindowId         // last OS-focused window
├── working_window_id: WindowId         // viewport currently rendering (set per frame)
├── tabs: Vec<Tab>                      // global document store
└── next_window_id / next_tab_id
```

`DocumentWindowState` fields:

- `id` — stable `WindowId` (`PRIMARY_WINDOW_ID = 0` for ROOT)
- `viewport_id` — `ViewportId::ROOT` or `from_hash_of(("document_window", id))`
- `tab_ids` — ordered strip (`Tab::id` values)
- `active_tab_index` — strip index (not global vec index)
- `geometry` — per-window size/position (captured each frame; persisted in session v2 — see [multi-window-session.md](./multi-window-session.md))

Tab accessors (`tab()`, `active_tab()`, `tab_count()`, `new_tab()`, `close_tab()`, …) resolve against **`working_window_id`** during render and **`focused_window_id`** for file operations initiated from menus/shortcuts.

`new_document_window()` allocates a window id, creates one empty tab, and focuses the new window.

## Viewport lifecycle (`src/app/windows.rs`)

| Window | Viewport | Render path |
|--------|----------|-------------|
| Primary (`id == 0`) | `ViewportId::ROOT` | `eframe::App::ui()` — sets `working_window_id = 0` |
| Secondary | child via `show_viewport_immediate` | `render_secondary_document_windows()` in `update()` |

Secondary windows reuse the full `render_ui()` path (title bar, ribbon, panels, `central_panel`). Borderless resize uses per-window `secondary_window_resize_states`.

**Primary close with secondaries open:** ROOT cannot be destroyed; closing primary clears its tab strip and focuses the surviving window with the smallest `window_id`.

## User actions

| Entry point | Handler |
|-------------|---------|
| Ribbon → New Window icon (right cluster) | `RibbonAction::NewWindow` → `handle_new_window()` |
| Command palette / `ShortcutCommand::NewWindow` | `dispatch_palette_command` / `keyboard.rs` |
| i18n | `menu.window.label`, `menu.window.new_window` in `locales/en.yaml` |

Default shortcut: **Ctrl+Shift+N** (`ShortcutCommand::NewWindow` in `config/settings.rs`).

## Tests

- `state::tests::test_new_document_window_independent_tab_strips` — two windows, independent strips
- Existing tab/session tests adapted for per-window `active_tab_index`

## Follow-up tasks

| Task | Scope |
|------|-------|
| **17** | Session schema v2: restore all windows, tab lists, geometries |
