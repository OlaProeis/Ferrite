# Keyboard Shortcuts

## Overview

Global keyboard shortcuts for file operations, tab management, and navigation. Implemented in `src/app.rs` using egui's input handling with deferred action execution to avoid borrow conflicts.

**Platform Note:** Ferrite uses standard platform modifiers:
- **macOS:** Command (Cmd) key
- **Windows/Linux:** Control (Ctrl) key

The shortcuts below show `Cmd/Ctrl` to indicate this cross-platform behavior.

## Key Files

| File | Purpose |
|------|---------|
| `src/app.rs` | `KeyboardAction` enum, `handle_keyboard_shortcuts()`, action handlers, `modifier_symbol()` |
| `src/ui/ribbon.rs` | Tooltip display with platform-aware modifier names |

## Shortcut Reference

### File Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+N** | New file | Creates new empty tab |
| **Cmd/Ctrl+O** | Open file | Opens native file dialog |
| **Cmd/Ctrl+S** | Save | Saves current file (or triggers Save As if no path) |
| **Cmd/Ctrl+Shift+S** | Save As | Opens native save dialog |

### Tab Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+T** | New tab | Creates new empty tab |
| **Cmd/Ctrl+W** | Close tab | Closes current tab (prompts if unsaved) |
| **Cmd/Ctrl+Tab** | Next tab | Switches to next tab (wraps to first) |
| **Cmd/Ctrl+Shift+Tab** | Previous tab | Switches to previous tab (wraps to last) |

### Edit Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+Z** | Undo | Undo last change |
| **Cmd/Ctrl+Y** | Redo | Redo last undone change |
| **Cmd/Ctrl+Shift+Z** | Redo | Redo (alternative) |
| **Cmd/Ctrl+F** | Find | Open find panel |
| **Cmd/Ctrl+H** | Find & Replace | Open find/replace panel |
| **Cmd/Ctrl+A** | Select All | Select all text |

### View Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+E** | Toggle View | Switch between Raw and Rendered modes |
| **Cmd/Ctrl+Shift+O** | Toggle Outline | Show/hide document outline panel |
| **Cmd/Ctrl++** | Zoom In | Increase font size |
| **Cmd/Ctrl+-** | Zoom Out | Decrease font size |
| **Cmd/Ctrl+0** | Reset Zoom | Reset font size to default |
| **Cmd/Ctrl+,** | Settings | Open settings panel |
| **F1** | About/Help | Open about and shortcuts reference |

### Formatting (Markdown)

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+B** | Bold | Toggle bold formatting |
| **Cmd/Ctrl+I** | Italic | Toggle italic formatting |
| **Cmd/Ctrl+K** | Link | Insert link |
| **Cmd/Ctrl+`** | Inline Code | Toggle inline code |
| **Cmd/Ctrl+1-6** | Headings | Apply heading level 1-6 |

### Workspace Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+P** | Quick File Switcher | Open file palette (workspace mode) |
| **Cmd/Ctrl+Shift+F** | Search in Files | Search across workspace (workspace mode) |
| **Cmd/Ctrl+Shift+E** | Export HTML | Export current document as HTML |

### Navigation

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Cmd/Ctrl+G** | Go to Line | Jump to specific line number |
| **F3** | Find Next | Jump to next search match |
| **Shift+F3** | Find Previous | Jump to previous search match |

## Implementation

### Cross-Platform Modifier Handling

egui provides built-in cross-platform support through `modifiers.command`:
- On macOS: Maps to Command key
- On Windows/Linux: Maps to Control key

```rust
/// Get the display name for the primary modifier key.
/// Returns "Cmd" on macOS, "Ctrl" on Windows/Linux.
pub fn modifier_symbol() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}
```

### KeyboardAction Enum

Actions are detected in an input closure and deferred for execution to avoid borrow conflicts:

```rust
#[derive(Debug, Clone, Copy)]
enum KeyboardAction {
    // File operations
    Save,           // Cmd/Ctrl+S
    SaveAs,         // Cmd/Ctrl+Shift+S
    Open,           // Cmd/Ctrl+O
    New,            // Cmd/Ctrl+N
    NewTab,         // Cmd/Ctrl+T
    CloseTab,       // Cmd/Ctrl+W
    NextTab,        // Cmd/Ctrl+Tab
    PrevTab,        // Cmd/Ctrl+Shift+Tab
    // Edit operations
    Undo,           // Cmd/Ctrl+Z
    Redo,           // Cmd/Ctrl+Y
    Find,           // Cmd/Ctrl+F
    FindReplace,    // Cmd/Ctrl+H
    // View operations
    ToggleView,     // Cmd/Ctrl+E
    ToggleOutline,  // Cmd/Ctrl+Shift+O
    OpenSettings,   // Cmd/Ctrl+,
    OpenAbout,      // F1
    // Workspace operations
    QuickSwitcher,  // Cmd/Ctrl+P
    SearchInFiles,  // Cmd/Ctrl+Shift+F
    ToggleFileTree, // Cmd/Ctrl+Shift+E
    // Formatting
    FormatBold,     // Cmd/Ctrl+B
    FormatItalic,   // Cmd/Ctrl+I
    FormatLink,     // Cmd/Ctrl+K
    FormatCode,     // Cmd/Ctrl+`
}
```

### Detection Pattern

```rust
fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
    ctx.input(|i| {
        // Check more specific shortcuts first (Cmd/Ctrl+Shift+X before Cmd/Ctrl+X)

        // Cmd/Ctrl+Shift+S: Save As
        if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::S) {
            return Some(KeyboardAction::SaveAs);
        }

        // Cmd/Ctrl+S: Save (must check !shift to avoid conflict)
        if i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
            return Some(KeyboardAction::Save);
        }

        // ... more shortcuts

        None
    }).map(|action| {
        // Execute action after input closure
        match action {
            KeyboardAction::Save => self.handle_save_file(),
            KeyboardAction::SaveAs => self.handle_save_as_file(),
            KeyboardAction::Open => self.handle_open_file(),
            KeyboardAction::New => self.state.new_tab(),
            KeyboardAction::NewTab => self.state.new_tab(),
            KeyboardAction::CloseTab => self.handle_close_current_tab(),
            KeyboardAction::NextTab => self.handle_next_tab(),
            KeyboardAction::PrevTab => self.handle_prev_tab(),
        }
    });
}
```

### Action Handlers

#### Tab Navigation

```rust
/// Switch to the next tab (cycles to first if at end)
fn handle_next_tab(&mut self) {
    let count = self.state.tab_count();
    if count > 1 {
        let current = self.state.active_tab_index();
        let next = (current + 1) % count;
        self.state.set_active_tab(next);
    }
}

/// Switch to the previous tab (cycles to last if at beginning)
fn handle_prev_tab(&mut self) {
    let count = self.state.tab_count();
    if count > 1 {
        let current = self.state.active_tab_index();
        let prev = if current == 0 { count - 1 } else { current - 1 };
        self.state.set_active_tab(prev);
    }
}

/// Close current tab (triggers unsaved prompt if needed)
fn handle_close_current_tab(&mut self) {
    let index = self.state.active_tab_index();
    self.state.close_tab(index);
}
```

## Key Detection Notes

### Modifier Order

Always check more specific shortcuts first:

```rust
// Correct order
if command && shift && key == S { SaveAs }
if command && !shift && key == S { Save }

// Wrong order - SaveAs would never trigger
if command && key == S { Save }
if command && shift && key == S { SaveAs }
```

### egui Key Constants

Common keys used:

```rust
egui::Key::S      // S key
egui::Key::O      // O key
egui::Key::N      // N key
egui::Key::T      // T key
egui::Key::W      // W key
egui::Key::Tab    // Tab key
```

### Modifier Flags

```rust
i.modifiers.command // Cmd (Mac) / Ctrl (Win/Linux) - USE THIS for cross-platform
i.modifiers.ctrl    // Raw Ctrl key (avoid for shortcuts)
i.modifiers.shift   // Shift key held
i.modifiers.alt     // Alt key held
i.modifiers.mac_cmd // Mac Command key only
```

## Testing

Keyboard shortcuts are tested through integration testing by running the application:

```bash
cargo run
```

Manual test checklist (use Cmd on macOS, Ctrl on Windows/Linux):
- [ ] Cmd/Ctrl+N creates new tab
- [ ] Cmd/Ctrl+T creates new tab
- [ ] Cmd/Ctrl+O opens file dialog
- [ ] Cmd/Ctrl+S saves (or Save As if no path)
- [ ] Cmd/Ctrl+Shift+S opens Save As dialog
- [ ] Cmd/Ctrl+W closes current tab (with prompt if unsaved)
- [ ] Cmd/Ctrl+Tab cycles to next tab
- [ ] Cmd/Ctrl+Shift+Tab cycles to previous tab

## Related Documentation

- [Tab System](./tab-system.md) - Tab management details
- [File Dialogs](./file-dialogs.md) - Save/Open operations
- [eframe Window](./eframe-window.md) - App lifecycle
