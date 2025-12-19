# Keyboard Shortcuts

## Overview

Global keyboard shortcuts for file operations, tab management, and navigation. Implemented in `src/app.rs` using egui's input handling with deferred action execution to avoid borrow conflicts.

## Key Files

| File | Purpose |
|------|---------|
| `src/app.rs` | `KeyboardAction` enum, `handle_keyboard_shortcuts()`, action handlers |

## Shortcut Reference

### File Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+N** | New file | Creates new empty tab |
| **Ctrl+O** | Open file | Opens native file dialog |
| **Ctrl+S** | Save | Saves current file (or triggers Save As if no path) |
| **Ctrl+Shift+S** | Save As | Opens native save dialog |

### Tab Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+T** | New tab | Creates new empty tab |
| **Ctrl+W** | Close tab | Closes current tab (prompts if unsaved) |
| **Ctrl+Tab** | Next tab | Switches to next tab (wraps to first) |
| **Ctrl+Shift+Tab** | Previous tab | Switches to previous tab (wraps to last) |

### Edit Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+Z** | Undo | Undo last change |
| **Ctrl+Y** | Redo | Redo last undone change |
| **Ctrl+F** | Find | Open find panel |
| **Ctrl+H** | Find & Replace | Open find/replace panel |
| **Ctrl+A** | Select All | Select all text |

### View Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+E** | Toggle View | Switch between Raw and Rendered modes |
| **Ctrl+Shift+O** | Toggle Outline | Show/hide document outline panel |
| **Ctrl++** | Zoom In | Increase font size |
| **Ctrl+-** | Zoom Out | Decrease font size |
| **Ctrl+0** | Reset Zoom | Reset font size to default |
| **Ctrl+,** | Settings | Open settings panel |
| **F1** | About/Help | Open about and shortcuts reference |

### Formatting (Markdown)

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+B** | Bold | Toggle bold formatting |
| **Ctrl+I** | Italic | Toggle italic formatting |
| **Ctrl+K** | Link | Insert link |
| **Ctrl+`** | Inline Code | Toggle inline code |

### Workspace Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+P** | Quick File Switcher | Open file palette (workspace mode) |
| **Ctrl+Shift+F** | Search in Files | Search across workspace (workspace mode) |
| **Ctrl+Shift+E** | Toggle File Tree | Show/hide file tree panel |

### Navigation

| Shortcut | Action | Description |
|----------|--------|-------------|
| **Ctrl+G** | Go to Line | Jump to specific line number |
| **F3** | Find Next | Jump to next search match |
| **Shift+F3** | Find Previous | Jump to previous search match |

## Implementation

### KeyboardAction Enum

Actions are detected in an input closure and deferred for execution to avoid borrow conflicts:

```rust
#[derive(Debug, Clone, Copy)]
enum KeyboardAction {
    // File operations
    Save,           // Ctrl+S
    SaveAs,         // Ctrl+Shift+S
    Open,           // Ctrl+O
    New,            // Ctrl+N
    NewTab,         // Ctrl+T
    CloseTab,       // Ctrl+W
    NextTab,        // Ctrl+Tab
    PrevTab,        // Ctrl+Shift+Tab
    // Edit operations
    Undo,           // Ctrl+Z
    Redo,           // Ctrl+Y
    Find,           // Ctrl+F
    FindReplace,    // Ctrl+H
    // View operations
    ToggleView,     // Ctrl+E
    ToggleOutline,  // Ctrl+Shift+O
    OpenSettings,   // Ctrl+,
    OpenAbout,      // F1
    // Workspace operations
    QuickSwitcher,  // Ctrl+P
    SearchInFiles,  // Ctrl+Shift+F
    ToggleFileTree, // Ctrl+Shift+E
    // Formatting
    FormatBold,     // Ctrl+B
    FormatItalic,   // Ctrl+I
    FormatLink,     // Ctrl+K
    FormatCode,     // Ctrl+`
}
```

### Detection Pattern

```rust
fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
    ctx.input(|i| {
        // Check more specific shortcuts first (Ctrl+Shift+X before Ctrl+X)
        
        // Ctrl+Shift+S: Save As
        if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
            return Some(KeyboardAction::SaveAs);
        }
        
        // Ctrl+S: Save (must check !shift to avoid conflict)
        if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
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
// ✅ Correct order
if ctrl && shift && key == S { SaveAs }
if ctrl && !shift && key == S { Save }

// ❌ Wrong order - SaveAs would never trigger
if ctrl && key == S { Save }
if ctrl && shift && key == S { SaveAs }
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
i.modifiers.ctrl   // Ctrl key held
i.modifiers.shift  // Shift key held
i.modifiers.alt    // Alt key held
i.modifiers.command // Cmd (Mac) / Win key
```

## Testing

Keyboard shortcuts are tested through integration testing by running the application:

```bash
cargo run
```

Manual test checklist:
- [ ] Ctrl+N creates new tab
- [ ] Ctrl+T creates new tab
- [ ] Ctrl+O opens file dialog
- [ ] Ctrl+S saves (or Save As if no path)
- [ ] Ctrl+Shift+S opens Save As dialog
- [ ] Ctrl+W closes current tab (with prompt if unsaved)
- [ ] Ctrl+Tab cycles to next tab
- [ ] Ctrl+Shift+Tab cycles to previous tab

## Related Documentation

- [Tab System](./tab-system.md) - Tab management details
- [File Dialogs](./file-dialogs.md) - Save/Open operations
- [eframe Window](./eframe-window.md) - App lifecycle
