# Workspace File Tree Panel

## Overview

Left sidebar in workspace mode showing the lazy-loaded project file tree. Supports expand/collapse, click-to-open, Git status badges, and context-menu file operations. Row styling highlights hover and the file matching the active document tab ([#135](https://github.com/OlaProeis/Ferrite/issues/135)).

## Key Files

- `src/ui/file_tree.rs` — Panel UI (`FileTreePanel`, row rendering, context menu)
- `src/workspaces/file_tree.rs` — Tree data (`FileTreeNode`, lazy directory scan)
- `src/app/mod.rs` — Wires panel into layout; passes active tab path and accent each frame

## Row Styling

Each tree row is a single `allocate_exact_size` hit target. Background is painted before icon and label text.

| State | Background | Icon | Label |
|-------|------------|------|-------|
| Default | none | theme text color | Git-status tint or default text |
| Hover | neutral tint (`hover_bg`) | slightly brighter text | unchanged |
| Active tab | `accent::panel_highlight_fill` | user accent | `Inter-Bold`, high-contrast text |

Active styling applies to **files only** (`is_active_file_row`). Directories never receive active emphasis even if paths coincidentally match.

Active takes precedence over hover when both apply.

## Active Tab Path

`FileTreePanel::show()` accepts `active_tab_path: Option<&Path>` and `ui_accent: Color32`.

The app resolves these each frame from `AppState::active_tab()`:

```rust
let active_tab_path = self.state.active_tab().and_then(|tab| tab.path.as_deref());
let output = self.file_tree_panel.show(
    ui,
    &workspace.file_tree,
    workspace_name,
    is_dark,
    git_statuses.as_ref(),
    active_tab_path,
    self.state.settings.ferrite_accent_rgb(),
);
```

Matching is a direct `Path` equality check against `FileTreeNode::path`. Per-frame cost is O(visible rows) with trivial comparisons only — no canonicalization or tree search.

Tabs whose path is outside the workspace (or untitled tabs with `path: None`) simply match no row; no panic.

## Testing

Unit test `is_active_file_row` in `src/ui/file_tree.rs` covers file vs directory, `None` path, and mismatched paths.

Manual: verify hover in light and dark themes; switch tabs and confirm emphasis follows the open file.
