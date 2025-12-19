# Settings Panel

The Settings Panel provides a modal interface for configuring application preferences with live preview. All changes are applied immediately and saved automatically.

## Features

- **Modal overlay** - Semi-transparent backdrop prevents interaction with main window
- **Section navigation** - Tabbed interface with Appearance, Editor, and Files sections
- **Live preview** - Changes apply immediately without requiring a save action
- **Auto-save** - Settings are persisted automatically when modified
- **Reset to defaults** - One-click option to restore all settings to defaults

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+,` | Open/close settings panel |
| `Escape` | Close settings panel |

## Access Methods

1. **Keyboard**: Press `Ctrl+,` from anywhere in the application
2. **Ribbon**: Click the gear icon (⚙) in the Settings group

## Sections

### Appearance

Configure visual preferences:

| Setting | Description | Range/Options |
|---------|-------------|---------------|
| Theme | Color scheme | Light, Dark, System |
| Font Size | Editor text size | 8-72px (Small/Medium/Large presets) |

### Editor

Configure editing behavior:

| Setting | Description | Default |
|---------|-------------|---------|
| Word Wrap | Wrap long lines | Enabled |
| Show Line Numbers | Display line numbers | Enabled |
| Use Spaces | Spaces instead of tabs | Enabled |
| Tab Size | Indentation width | 4 spaces (2-8 range) |

### Files

Configure file handling:

| Setting | Description | Default |
|---------|-------------|---------|
| Auto-Save | Save files automatically | Disabled |
| Auto-Save Interval | Seconds between saves | 60 (5-300 range) |
| Recent Files | Number to remember | 10 (0-20 range) |
| Clear Recent Files | Remove all recent entries | Button |

## Architecture

### Components

```
src/ui/settings.rs
├── SettingsSection    - Enum for navigation tabs
├── SettingsPanelOutput - Result of showing the panel
└── SettingsPanel      - Main panel component
```

### State Flow

```
User Action
    ↓
SettingsPanel::show()
    ↓
Modifies &mut Settings directly
    ↓
Returns SettingsPanelOutput { changed, close_requested, reset_requested }
    ↓
App handles:
  - changed → Apply theme, mark dirty
  - reset_requested → Restore defaults, apply theme
  - close_requested → Hide panel
```

### Integration Points

1. **AppState.ui.show_settings** - Boolean flag to show/hide panel
2. **AppState.settings** - Direct mutation for live preview
3. **ThemeManager** - Theme changes applied immediately via `set_theme()` and `apply()`
4. **mark_settings_dirty()** - Triggers persistence on next save interval

## Implementation Details

### Modal Behavior

The panel uses a layered approach:
1. **Overlay Area** (Order::Middle) - Captures clicks outside the window
2. **Settings Window** (Order::Foreground) - The actual panel

```rust
// Overlay captures outside clicks
egui::Area::new("settings_overlay")
    .order(egui::Order::Middle)
    .show(ctx, |ui| {
        if response.clicked() {
            output.close_requested = true;
        }
    });

// Window is on top
egui::Window::new("⚙ Settings")
    .order(egui::Order::Foreground)
    .show(ctx, |ui| { ... });
```

### Live Preview

Changes modify settings directly, enabling immediate visual feedback:

```rust
if ui.selectable_value(&mut settings.theme, Theme::Dark, "Dark").changed() {
    changed = true;  // Signal for theme application
}
```

The app then applies theme changes:

```rust
if output.changed {
    self.theme_manager.set_theme(self.state.settings.theme);
    self.theme_manager.apply(ctx);
    self.state.mark_settings_dirty();
}
```

### Persistence

Settings are marked dirty on change and saved automatically via the existing config persistence system:

1. `mark_settings_dirty()` sets the dirty flag
2. `eframe::App::save()` calls `save_settings_if_dirty()`
3. Settings serialize to `~/.config/sleek-markdown-editor/config.json`

## Testing

Unit tests cover:
- Panel initialization (`test_settings_panel_new`, `test_settings_panel_default`)
- Section enumerations (`test_settings_section_label`, `test_settings_section_icon`)
- Output struct defaults (`test_settings_panel_output_default`)

The panel integrates with existing settings tests for validation and serialization.

## Future Enhancements

Potential additions for future iterations:
- Accent color picker for UI customization
- Default save location preference
- Font family selection
- Keyboard shortcut customization
- Import/export settings
