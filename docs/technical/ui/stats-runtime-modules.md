# Stats Panel — Runtime Modules (Phase 1)

Read-only runtime diagnostics in the outline side panel **Stats** tab: loaded lazy fonts, Mermaid diagram cache occupancy, terminal session count, and a disabled LSP row (no active integration implied).

## Overview

Phase 1 is display-only. Manual unload/clear actions (Phase 2) are out of scope.

The section appears on the **Stats** tab for markdown files and on the Stats tab for structured (JSON/YAML/TOML) files. It is not shown on the Outline tab.

## Data model

`RuntimeModulesInfo` (`src/ui/runtime_modules.rs`) aggregates:

| Field | Source |
|-------|--------|
| `loaded_font_names` | `fonts::get_loaded_runtime_font_names()` — CJK/complex-script family keys (`CJK_JP`, `Arabic`, …) |
| `mermaid_cache_entries` / `mermaid_cache_max_entries` | `markdown::mermaid::get_cache_snapshot()` |
| `terminal_session_count` | `TerminalManager::terminal_count()` passed in from the app |

Collected each frame when the outline panel renders:

```rust
RuntimeModulesInfo::collect(self.terminal_panel_state.manager.terminal_count())
```

## UI

`OutlinePanel::show` takes `runtime_modules: Option<&RuntimeModulesInfo>`. `render_runtime_modules` in `outline_panel.rs` draws:

- **Loaded fonts** — comma-separated localized labels (CJK names reuse `settings.editor.cjk_*` keys)
- **Mermaid cache** — `entries / max` (e.g. `2 / 50`)
- **Terminal sessions** — integer count
- **Language Server** — muted **Disabled** text

Labels: `stats.runtime_modules`, `stats.runtime.*` in `locales/en.yaml`.

## Font and cache APIs

- `fonts::get_loaded_runtime_font_names()` — reads per-script `AtomicBool` load flags; no buffer scan
- `mermaid::get_cache_snapshot()` — returns `MermaidCacheSnapshot` without initializing the global cache if empty
- `MermaidCacheManager::max_entries()` — LRU capacity (default 50)

## LSP row

The LSP line is intentionally non-actionable and labeled **Disabled** so the Stats panel does not suggest an active language-server connection. Full LSP work remains deferred.

## Tests

Unit tests in `src/ui/runtime_modules.rs`: terminal count propagation, cache size formatting, font list aggregation, empty cache snapshot defaults.
