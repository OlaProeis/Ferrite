# Ribbon UI System

## Overview

The ribbon UI replaces the traditional menu bar with a modern, icon-based interface that organizes controls into logical groups. The ribbon provides quick access to common actions while preserving all keyboard shortcuts.

## Architecture

### Module Structure

```
src/ui/
├── mod.rs        # UI module exports (Ribbon, RibbonAction)
└── ribbon.rs     # Ribbon implementation
```

### Key Types

#### `RibbonAction` Enum

Defines all actions that can be triggered from the ribbon:

```rust
pub enum RibbonAction {
    // File operations
    New,              // Create new file/tab (Ctrl+N)
    Open,             // Open file dialog (Ctrl+O)
    OpenWorkspace,    // Open folder/workspace dialog
    CloseWorkspace,   // Close current workspace
    Save,             // Save current file (Ctrl+S)
    SaveAs,           // Save As dialog (Ctrl+Shift+S)

    // Workspace operations (only visible in workspace mode)
    SearchInFiles,    // Search in files (Ctrl+Shift+F)
    QuickFileSwitcher, // Quick file switcher (Ctrl+P)

    // Edit operations
    Undo,             // Undo last change (Ctrl+Z)
    Redo,             // Redo last undone change (Ctrl+Y)

    // Formatting operations (Markdown)
    Format(MarkdownFormatCommand), // Apply markdown formatting

    // Structured data operations (JSON/YAML/TOML)
    FormatDocument,   // Format/pretty-print structured data
    ValidateSyntax,   // Validate syntax

    // View operations
    ToggleViewMode,   // Toggle Raw/Rendered (Ctrl+E)
    ToggleLineNumbers, // Toggle line numbers visibility
    ToggleSyncScroll, // Toggle sync scrolling

    // Tools
    FindReplace,      // Find/Replace dialog (Ctrl+F/Ctrl+H)
    ToggleOutline,    // Toggle outline panel

    // Export operations
    ExportHtml,       // Export as HTML file
    CopyAsHtml,       // Copy rendered HTML to clipboard

    // Settings
    CycleTheme,       // Cycle through themes (Ctrl+Shift+T)
    OpenSettings,     // Open settings panel (Ctrl+,)

    // Ribbon control
    ToggleCollapse,   // Collapse/expand ribbon
}
```

#### `Ribbon` Struct

Manages ribbon state and rendering:

```rust
pub struct Ribbon {
    collapsed: bool,  // Whether ribbon is in collapsed mode
}
```

## Layout

The ribbon appears below the title bar and above the tab bar:

```
┌────────────────────────────────────────────────────────────────┐
│ 📝 Document.md - Ferrite                       [_][□][×]      │  Title Bar
├────────────────────────────────────────────────────────────────┤
│ ◀ | File  📄 📂 💾 📥 | Edit  ↩ ↪ | View  👁 🔢 | Tools 🔍 | ⚙ │  Ribbon
├────────────────────────────────────────────────────────────────┤
│ [Tab 1] [Tab 2*] [+]                                          │  Tab Bar
├────────────────────────────────────────────────────────────────┤
│                                                                │
│                        Editor Content                          │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│ Untitled | Ln 1, Col 1 | UTF-8 | 0 words                      │  Status Bar
└────────────────────────────────────────────────────────────────┘
```

## Button Groups

### File Group
| Icon | Action | Shortcut | Description |
|------|--------|----------|-------------|
| 📄 | New | Ctrl+N | Create new file/tab |
| 📂 | Open | Ctrl+O | Open file dialog |
| 💾 | Save | Ctrl+S | Save current file |
| 📥 | Save As | Ctrl+Shift+S | Save with new name |

### Edit Group
| Icon | Action | Shortcut | Description |
|------|--------|----------|-------------|
| ↩ | Undo | Ctrl+Z | Undo last change |
| ↪ | Redo | Ctrl+Y | Redo undone change |

### View Group
| Icon | Action | Shortcut | Description |
|------|--------|----------|-------------|
| 📝/👁 | Toggle View | Ctrl+Shift+V | Switch Raw/Rendered |
| 🔢/# | Line Numbers | - | Toggle line numbers |

### Tools Group
| Icon | Action | Shortcut | Description |
|------|--------|----------|-------------|
| 🔍 | Find/Replace | - | Find/Replace (placeholder) |

### Settings Group (right-aligned)
| Icon | Action | Shortcut | Description |
|------|--------|----------|-------------|
| 🎨 | Theme | Ctrl+Shift+T | Cycle theme |
| ⚙ | Settings | - | Open settings (placeholder) |

## Collapsible Behavior

The ribbon supports two states:

1. **Expanded** (default): Shows group labels and full-height buttons (40px)
2. **Collapsed**: Icon-only mode with reduced height (28px)

Toggle with the ◀/▶ button on the left side of the ribbon.

## Theme Integration

The ribbon adapts to the current theme:

```rust
// Ribbon background colors
let ribbon_bg = if is_dark {
    Color32::from_rgb(40, 40, 40)   // Dark theme
} else {
    Color32::from_rgb(248, 248, 248) // Light theme
};

// Separator colors
let separator_color = if is_dark {
    Color32::from_rgb(70, 70, 70)
} else {
    Color32::from_rgb(210, 210, 210)
};
```

## Usage in App

```rust
// In FerriteApp struct
ribbon: Ribbon,

// In render_ui method
let ribbon_action = {
    let mut action = None;
    egui::TopBottomPanel::top("ribbon")
        .frame(/* frame config */)
        .show(ctx, |ui| {
            action = self.ribbon.show(
                ui,
                &theme_colors,
                view_mode,
                show_line_numbers,
                can_undo,
                can_redo,
                can_save,
            );
        });
    action
};

// Handle ribbon actions
if let Some(action) = ribbon_action {
    self.handle_ribbon_action(action, ctx);
}
```

## State Parameters

The `show()` method requires current application state:

| Parameter | Type | Purpose |
|-----------|------|---------|
| `theme_colors` | `&ThemeColors` | Theme-aware styling |
| `view_mode` | `ViewMode` | Current view mode (Raw/Rendered) |
| `show_line_numbers` | `bool` | Line numbers visibility |
| `can_undo` | `bool` | Enable/disable Undo button |
| `can_redo` | `bool` | Enable/disable Redo button |
| `can_save` | `bool` | Enable/disable Save button |

## Button Styling

Icon buttons use consistent styling:

```rust
const ICON_BUTTON_SIZE: Vec2 = Vec2::new(32.0, 28.0);

fn icon_button(ui, icon, tooltip, enabled, is_dark) -> Response {
    // Disabled state uses muted colors
    let text_color = if enabled {
        theme_text_color
    } else {
        muted_color
    };
    
    // Hover effect
    if btn.hovered() && enabled {
        ui.painter().rect_filled(btn.rect, 3.0, hover_bg);
    }
}
```

## Testing

```rust
#[test]
fn test_ribbon_new() {
    let ribbon = Ribbon::new();
    assert!(!ribbon.is_collapsed());
}

#[test]
fn test_ribbon_toggle_collapsed() {
    let mut ribbon = Ribbon::new();
    ribbon.toggle_collapsed();
    assert!(ribbon.is_collapsed());
}

#[test]
fn test_ribbon_height() {
    let mut ribbon = Ribbon::new();
    assert_eq!(ribbon.height(), 40.0); // expanded
    ribbon.toggle_collapsed();
    assert_eq!(ribbon.height(), 28.0); // collapsed
}
```

## Keyboard Shortcuts

All existing keyboard shortcuts continue to work independently of the ribbon:

| Shortcut | Action |
|----------|--------|
| Ctrl+N | New file |
| Ctrl+O | Open file |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save As |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+Shift+Z | Redo (alternative) |
| Ctrl+Shift+V | Toggle view mode |
| Ctrl+Shift+T | Cycle theme |
| Ctrl+T | New tab |
| Ctrl+W | Close tab |
| Ctrl+Tab | Next tab |
| Ctrl+Shift+Tab | Previous tab |

## Implemented Actions

All ribbon buttons are now fully functional:

- **Find/Replace** (🔍): Opens find/replace dialog (Ctrl+F / Ctrl+H)
- **Settings** (⚙): Opens settings panel modal (Ctrl+,)
- **Export HTML**: Exports document as styled HTML file
- **Copy as HTML**: Copies rendered HTML to clipboard
- **Outline** (📋): Toggles document outline panel
