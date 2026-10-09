# Status Bar

## Overview

The status bar is a bottom panel displaying file information, text statistics, cursor position, and temporary toast notifications.

## Key Files

- `src/app/status_bar.rs` — `FerriteApp::render_status_bar` (path, toasts, cursor, stats, Vim label)
- `src/state.rs` — `UiState::vim_mode_indicator`, `Tab::text_stats` / `content_version`
- `src/editor/stats.rs` — `TextStats` (words, lines, characters; `format_compact()`)

## Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ G:\path\to\file.md     [Toast Message]     Words: 123 | Ln 1, Col 5│
│ ↑ Clickable           ↑ Centered           ↑ Right-aligned        │
└────────────────────────────────────────────────────────────────────┘
```

## Components

### File Path (Left)

Displays current file path or "Untitled" / "No file open":

```rust
let path_display = if let Some(tab) = self.state.active_tab() {
    tab.path.as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
} else {
    "No file open".to_string()
};
```

**Clickable**: Opens [Recent Files](./recent-files.md) popup when clicked.

### Toast Messages (Center)

Temporary notifications shown in italics:

```rust
if let Some(toast) = &self.state.ui.toast_message {
    ui.label(RichText::new(toast).italics());
}
```

Toast messages auto-expire after a configurable duration (typically 2-3 seconds).

### Right Side Information

| Field | Format | Description |
|-------|--------|-------------|
| Vim mode | `[NORMAL]` / `[INSERT]` / … | Only when **Settings → Vim** is on; label from raw/split editor each frame |
| Text stats | compact string | `Tab::text_stats()` → `TextStats::format_compact()` (includes line count) |
| Encoding | picker | Per-tab encoding for saved files |
| Cursor position | `Ln 1, Col 5` | 1-indexed from `tab.cursor_position` |

### Stats cache (`content_version`, ISS-25)

The status bar does **not** scan the buffer every frame. Before the right-aligned layout, it calls `active_tab_mut().map(|tab| tab.text_stats())`. `Tab::text_stats()` returns a cached `TextStats` while `cached_text_stats_version == content_version`, otherwise recomputes via `TextStats::from_text`.

Anything that mutates `tab.content` must bump `content_version` so line/word counts stay correct. Paths include:

- Raw editor sync: `record_edit_from_snapshot` / `record_external_edit_from_snapshot_coalesced` (increments on change)
- Programmatic edits: `Tab::record_edit` (duplicate/move/delete line, smart paste, image insert, etc.) — increments `content_version` and `source_epoch` when content differs
- Other helpers: `notify_external_content_change`, `increment_content_version` where used explicitly

Duplicate line (Ctrl+Shift+D) uses `handle_duplicate_line` in `src/app/line_ops.rs`: assign `tab.content`, then `record_edit` + `mark_content_edited`.

### Vim indicator (ISS-25)

`UiState.vim_mode_indicator` is set from `EditorWidget` output in raw/split central panel. Special or rendered-only tabs may not refresh it every frame. The status bar therefore shows the label only when **both** `settings.vim_mode` is true **and** `vim_mode_indicator` is `Some`, so disabling Vim clears `[NORMAL]` immediately on any tab type.

## Toast Message System

### Showing a Toast

```rust
// In AppState
pub fn show_toast(&mut self, message: impl Into<String>, current_time: f64, duration: f64) {
    self.ui.toast_message = Some(message.into());
    self.ui.toast_expires_at = Some(current_time + duration);
}
```

### Updating Toast State

Called each frame to clear expired toasts:

```rust
pub fn update_toast(&mut self, current_time: f64) {
    if let Some(expires_at) = self.ui.toast_expires_at {
        if current_time >= expires_at {
            self.ui.toast_message = None;
            self.ui.toast_expires_at = None;
        }
    }
}
```

### Common Toast Usage

```rust
// File saved notification
self.state.show_toast(format!("Saved: {}", path.display()), time, 3.0);

// Multiple files opened
self.state.show_toast(format!("Opened {} files", count), time, 2.0);

// Background file open
self.state.show_toast("Opened in background: file.md", time, 2.0);
```

## Text Statistics

The `TextStats` struct in `src/editor/stats.rs` provides:

```rust
pub struct TextStats {
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub characters_no_spaces: usize,
    pub paragraphs: usize,
}

impl TextStats {
    pub fn from_text(text: &str) -> Self { ... }
    pub fn format_compact(&self) -> String {
        format!("{} words", self.words)
    }
}
```

## Related Documentation

- [Recent Files](./recent-files.md) - File path click popup
- [Text Statistics](./text-statistics.md) - Detailed stats documentation
- [App State](./app-state.md) - UiState toast fields
