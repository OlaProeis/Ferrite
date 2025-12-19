# Status Bar

## Overview

The status bar is a bottom panel displaying file information, text statistics, cursor position, and temporary toast notifications.

## Key Files

- `src/app.rs` - Status bar rendering in `render_ui()`
- `src/state.rs` - Toast message handling in `UiState` and `AppState`
- `src/editor/stats.rs` - `TextStats` struct for word/character counting

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
| Text Stats | "123 words" | Word count from `TextStats::format_compact()` |
| Encoding | "UTF-8" | Always UTF-8 (Rust strings) |
| Cursor Position | "Ln 1, Col 5" | 1-indexed line and column |

```rust
let stats = TextStats::from_text(&tab.content);
ui.label(stats.format_compact());

ui.separator();
ui.label("UTF-8");

ui.separator();
let (line, col) = tab.cursor_position;
ui.label(format!("Ln {}, Col {}", line + 1, col + 1));
```

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
