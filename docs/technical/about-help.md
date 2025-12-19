# About/Help Panel

## Overview

The About/Help panel provides application information and a comprehensive keyboard shortcuts reference. It's accessible via F1 or the `?` button in the status bar.

## Key Files

- `src/ui/about.rs` - About panel implementation with sections and shortcuts
- `src/app.rs` - F1 shortcut handling and panel toggle

## Features

### Two-Section Layout

The panel has a sidebar with two sections:

1. **About** (○): Application info, version, links
2. **Shortcuts** (⌘): Complete keyboard shortcuts reference

### About Section

Displays:
- Application name and version
- GitHub repository link (clickable)
- Documentation link (clickable)
- Credits and technology stack

### Shortcuts Section

Organized by category with collapsible sections:

| Category | Icon | Shortcuts |
|----------|------|-----------|
| File | 📄 | New, Open, Save, Save As, Close Tab |
| Edit | / | Undo, Redo, Find, Replace, Select All, Copy, Cut, Paste |
| View | 👁 | Toggle Raw/Rendered, Toggle Outline, Zoom, Settings, About |
| Formatting | Aa | Bold, Italic, Underline, Link, Inline Code |
| Workspace | 📁 | Quick File Switcher, Search in Files, Toggle File Tree |
| Navigation | ↔ | Next/Previous Tab, Go to Line, Find Next/Previous |

## Implementation Details

### Types

```rust
/// Shortcut category for organized display.
pub enum ShortcutCategory {
    File,
    Edit,
    View,
    Formatting,
    Workspace,
    Navigation,
}

/// About panel sections.
pub enum AboutSection {
    About,
    Shortcuts,
}

/// About panel state and rendering.
pub struct AboutPanel {
    active_section: AboutSection,
    collapsed_categories: Vec<ShortcutCategory>,
}

/// Result of showing the about panel.
pub struct AboutPanelOutput {
    pub close_requested: bool,
}
```

### Panel Display

The panel is shown as a modal window with:
- Semi-transparent overlay (closes on click outside)
- Escape key to close
- Fixed 550-650px width
- Centered on screen
- Sidebar navigation with section icons

### Theme Integration

Colors adapt to light/dark mode:
- Overlay: Darker in dark mode (180 alpha vs 120 alpha)
- Text and background colors from theme

## Keyboard Shortcuts

### Opening the Panel

| Shortcut | Action |
|----------|--------|
| F1 | Open About/Help panel |
| ? button | Click in status bar |

### Within the Panel

| Shortcut | Action |
|----------|--------|
| Escape | Close panel |
| Click outside | Close panel |

## Usage

### Show Panel

```rust
// In AppState or wherever ui state is managed
state.ui.show_about = true;

// In app.rs render loop
if state.ui.show_about {
    let output = about_panel.show(ctx, is_dark);
    if output.close_requested {
        state.ui.show_about = false;
    }
}
```

### F1 Shortcut Handling

```rust
// In keyboard shortcut handler
if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
    state.ui.show_about = !state.ui.show_about;
}
```

## Status Bar Integration

The status bar includes a `?` button that toggles the About/Help panel:
- Positioned at the right end of the status bar
- Same functionality as F1 key
- Visual indicator when panel is open

## Related Documentation

- [Keyboard Shortcuts](./keyboard-shortcuts.md) - Full shortcut reference
- [Settings Panel](./settings-panel.md) - Similar modal panel pattern
- [Status Bar](./status-bar.md) - Status bar implementation
