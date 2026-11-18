# Frontmatter Panel

## Overview

Visual editor for YAML frontmatter in markdown files. Displayed as the **"FM" tab** in the right-side outline panel, alongside Outline, Statistics, Links, and Hub tabs. Renders frontmatter key-value pairs as a form with type-aware widgets, supporting bidirectional sync with the raw editor.

## Key Files

- `src/ui/frontmatter_panel.rs` - Content widget, YAML parsing/serialization, form rendering
- `src/ui/outline_panel.rs` - Hosts the FM tab via `OutlinePanelTab::Frontmatter`
- `src/app/mod.rs` - `FrontmatterPanel` field on `FerriteApp`, content update + output handling
- `src/app/types.rs` - `KeyboardAction::ToggleFrontmatter`
- `src/app/keyboard.rs` - Shortcut wiring (`Ctrl+Shift+M` opens outline panel and switches to FM tab)
- `src/ui/ribbon.rs` - `RibbonAction::ToggleFrontmatter`

## Implementation Details

### Architecture

The frontmatter panel follows the same pattern as `BacklinksPanel` — a content widget rendered inside the outline panel's tab system:

- `FrontmatterPanel` struct holds cached state (parsed fields, `(tab_id, content_version)` cache key, input buffers)
- `show_content(ui, is_dark)` renders inside a parent `Ui` (the outline panel tab area)
- `FrontmatterPanelOutput` carries edit results (`new_content`) back to the caller
- `OutlinePanelOutput.frontmatter_new_content` propagates edits to `app/mod.rs`, which applies them to the active tab's content
- `update_from_content_versioned(content, tab_id, content_version)` is called before rendering to re-parse if either the active tab or its content changed (see **Caching** below)

The FM tab is always visible in the outline panel tab bar. When the active file is not markdown, the tab shows a "not available" message. The `Ctrl+Shift+M` shortcut opens the outline panel (if closed) and switches to the FM tab.

### Frontmatter Extraction

Uses a custom `extract_frontmatter()` function (not the full markdown parser) for efficiency:
- Finds `---\n` at document start
- Scans for closing `\n---\n`
- Returns YAML body and byte offset of the closing delimiter
- Handles BOM, CRLF, and edge cases

### Value Types

`FrontmatterValue` enum maps YAML types to appropriate egui widgets:

| YAML Type | Enum Variant | Widget |
|-----------|-------------|--------|
| String | `String(String)` | `TextEdit::singleline` |
| Boolean | `Bool(bool)` | `egui::Checkbox` |
| Number | `Number(String)` | `TextEdit::singleline` |
| Sequence | `List(Vec<String>)` | Pill/chip tags with add/remove |
| Mapping | `Mapping(String)` | `TextEdit::multiline` (code editor) |
| Null | `Null` | Italic "null" label |

### Bidirectional Sync

- **Raw → Panel**: `(tab_id, content_version)` comparison each frame; re-parses only when the active tab or its content changes (see **Caching** below)
- **Panel → Raw**: On any field edit, serializes all fields back to YAML and replaces the frontmatter block in the document content via `replace_frontmatter_in_content()`

### Caching

`update_from_content_versioned(content, tab_id, content_version)` gates re-parse on a `(tab_id, content_version)` cache key (`cached_key: Option<(usize, u64)>`).

`content_version` alone is **not** safe — it's a per-tab counter that starts at `0` for every new tab, so two different tabs frequently collide on the same version (e.g. both unedited). Pairing it with the stable `Tab.id` ensures cache invalidation on tab switch while keeping per-frame cost O(1). Regression covered by `cache_invalidates_on_tab_switch_with_matching_version` and `cache_skips_reparse_when_key_unchanged`.

`cached_key` is reset to `None` after panel-driven edits (Add frontmatter, field edit, tag add/remove, field delete) so the next frame re-parses from the new content.

> **History:** v0.2.8 used a `DefaultHasher` over the full content (O(N) per frame). v0.3.0's [per-frame cache elimination](../performance/per-frame-cache-elimination.md) replaced that with the version counter — but the initial version dropped the tab-id component, which caused stale-content bugs on tab switch (panel stuck on previous tab, "No frontmatter detected" on files with frontmatter, and "Add frontmatter" splicing the previous tab's body into the active tab). Fixed by re-introducing tab id into the cache key.

## Dependencies Used

- `serde_norway` 0.9 - YAML parsing and serialization (already in Cargo.toml). `serde_yaml` was abandoned; mappings still walk in file order.
- `chrono` 0.4 - Date formatting for "Add frontmatter" template

## Usage

- **Access**: Click the "FM" tab in the right-side outline/document panel
- **Shortcut**: `Ctrl+Shift+M` opens the outline panel and switches to the FM tab
- **Visibility**: FM tab always visible in tab bar; content only renders for markdown files
- **Empty state**: Shows "Add frontmatter" button to insert a default template (title, date, tags)
- **Test file**: `test_md/test_frontmatter.md`
