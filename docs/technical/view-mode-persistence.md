# View Mode Persistence

## Overview

Per-tab view mode persistence allows each tab to maintain its own view mode (Raw or Rendered) independently, with modes persisting across application restarts.

## Key Files

- `src/config/settings.rs` - `TabInfo` struct with `view_mode` field, `ViewMode` enum
- `src/state.rs` - `Tab` struct with view mode methods, session restoration
- `src/app.rs` - Per-tab view mode usage in UI

## Implementation Details

### ViewMode Enum

Defined in `settings.rs`:

```rust
pub enum ViewMode {
    Raw,       // Plain markdown text editing
    Rendered,  // WYSIWYG rendered editing
}
```

### Per-Tab Storage

View mode is stored at two levels:

1. **Runtime (`Tab` struct)**: `view_mode: ViewMode` field on each tab
2. **Persistence (`TabInfo` struct)**: Serialized to config JSON via `last_open_tabs`

### Default Behavior

| Scenario | Default Mode |
|----------|--------------|
| New tab created (`Tab::new()`) | Raw |
| File opened (`Tab::with_file()`) | Raw |
| Session restored (`Tab::from_tab_info()`) | Saved mode |
| Old config (no `view_mode` field) | Raw (backward compatible) |

### Tab Methods

```rust
impl Tab {
    pub fn get_view_mode(&self) -> ViewMode;
    pub fn set_view_mode(&mut self, mode: ViewMode);
    pub fn toggle_view_mode(&mut self) -> ViewMode;
}
```

### Session Persistence

On shutdown:
1. `Tab::to_tab_info()` includes `view_mode` 
2. `AppState::save_settings_if_dirty()` serializes to config

On startup:
1. `Tab::from_tab_info()` restores saved view mode
2. Missing `view_mode` field defaults to `Raw` (serde default)

## Config Format

```json
{
  "last_open_tabs": [
    {
      "path": "/path/to/file.md",
      "modified": false,
      "cursor_position": [0, 0],
      "scroll_offset": 0.0,
      "view_mode": "rendered"
    }
  ]
}
```

## Usage

### Toggle View Mode (Active Tab)

```rust
// In app.rs
if let Some(tab) = self.state.active_tab_mut() {
    let new_mode = tab.toggle_view_mode();
}
```

### Get Current Tab's Mode

```rust
let view_mode = self.state.active_tab()
    .map(|t| t.view_mode)
    .unwrap_or(ViewMode::Raw);
```

### Keyboard Shortcut

`Ctrl+Shift+V` toggles view mode for the active tab.

## Tests

Run tests with:
```bash
cargo test view_mode
cargo test tab_from_tab_info
cargo test restore_session
```

Key tests:
- `test_tab_view_mode_toggle` - Toggle functionality
- `test_tab_view_mode_get_set` - Getter/setter
- `test_tab_info_backward_compatibility` - Old config support
- `test_restore_session_tabs_with_temp_file` - Session restoration
- `test_restore_multiple_tabs_with_temp_files` - Multi-tab persistence

## Backward Compatibility

Old config files without `view_mode` in `TabInfo` will deserialize correctly due to `#[serde(default)]` attribute, defaulting to `ViewMode::Raw`.
