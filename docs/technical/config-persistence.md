# Configuration Persistence

## Overview

The configuration persistence module handles loading and saving application settings to platform-specific directories with robust error handling and graceful fallback to defaults.

## Key Files

- `src/config/persistence.rs` - Core persistence functions
- `src/config/mod.rs` - Module exports
- `src/config/settings.rs` - Settings struct being persisted

## Implementation Details

### Platform-Specific Directories

The module uses the `dirs` crate to determine the appropriate config directory for each platform:

| Platform | Config Directory |
|----------|------------------|
| Windows | `%APPDATA%\sleek-markdown-editor\` |
| macOS | `~/Library/Application Support/sleek-markdown-editor/` |
| Linux | `~/.config/sleek-markdown-editor/` |

Configuration is stored in `config.json` within this directory.

### Public API

```rust
// Load config (graceful fallback to defaults)
pub fn load_config() -> Settings

// Load config with full error information
pub fn load_config_strict() -> Result<Settings>

// Save config (atomic write)
pub fn save_config(settings: &Settings) -> Result<()>

// Save config, ignoring errors (returns bool)
pub fn save_config_silent(settings: &Settings) -> bool

// Load or create default config
pub fn load_or_create_config() -> Settings

// Utility functions
pub fn get_config_dir() -> Result<PathBuf>
pub fn get_config_file_path() -> Result<PathBuf>
pub fn config_exists() -> bool
pub fn delete_config() -> Result<()>
```

### Error Handling Strategy

The module uses a multi-level approach:

1. **`load_config()`** - Always returns valid `Settings`, falling back to defaults on any error
2. **`load_config_strict()`** - Returns `Result<Settings>` for explicit error handling
3. **`save_config_silent()`** - Best-effort save for non-critical scenarios (e.g., on exit)

### Graceful Fallback Scenarios

| Scenario | Behavior |
|----------|----------|
| Config file missing | Return defaults |
| Config file empty | Return defaults |
| Invalid JSON | Log warning, return defaults |
| Invalid values | Sanitize to valid ranges |
| Write permission denied | Log error, return false |

### Atomic Writes

The `save_config()` function uses an atomic write pattern:

1. Write settings to `config.json.bak`
2. Rename backup to `config.json`

This prevents data corruption if the app crashes during write.

## Session Restoration (Tab Persistence)

The configuration system includes support for restoring open tabs between sessions.

### Tab Info Structure

Each open tab's state is serialized to `TabInfo`:

```rust
pub struct TabInfo {
    pub path: Option<PathBuf>,       // File path (None for unsaved)
    pub modified: bool,              // Had unsaved changes
    pub cursor_position: (usize, usize), // (line, column)
    pub scroll_offset: f32,          // Scroll position
}
```

### Session Data in Settings

```rust
pub struct Settings {
    // ... other fields ...
    pub last_open_tabs: Vec<TabInfo>,  // Tabs from last session
    pub active_tab_index: usize,       // Which tab was active
}
```

### Persistence Flow

**On Application Exit:**
1. `AppState::shutdown()` is called
2. `save_settings()` serializes current tabs via `Tab::to_tab_info()`
3. `last_open_tabs` and `active_tab_index` are updated in settings
4. Config is written to disk

**On Application Startup:**
1. `AppState::new()` loads settings via `load_config()`
2. `restore_session_tabs()` is called
3. For each `TabInfo` with a valid path:
   - If file exists: read content and create `Tab::from_tab_info()`
   - If file missing: log warning and skip
   - If path is `None`: skip (unsaved tabs not restored)
4. `active_tab_index` is restored (clamped to valid range)
5. If no tabs restored, create an empty tab

### Error Handling

| Scenario | Behavior |
|----------|----------|
| File no longer exists | Skip tab, log warning |
| File read error | Skip tab, log warning |
| Unsaved tab (no path) | Skip tab |
| Invalid active_tab_index | Clamp to valid range |
| All tabs fail to restore | Create empty tab |

### Conversion Functions

```rust
// Tab to TabInfo (for persistence)
impl Tab {
    pub fn to_tab_info(&self) -> TabInfo { ... }
}

// TabInfo to Tab (for restoration)
impl Tab {
    pub fn from_tab_info(id: usize, info: &TabInfo, content: String) -> Self { ... }
}
```

## Dependencies Used

- `dirs` (5.x) - Platform-specific directory resolution
- `serde_json` (1.x) - JSON serialization/deserialization
- `log` (0.4) - Logging warnings and errors

## Usage

```rust
use crate::config::{load_config, save_config, Settings};

// On app startup
let settings = load_config(); // Always succeeds

// After user changes settings
settings.theme = Theme::Dark;
if let Err(e) = save_config(&settings) {
    eprintln!("Warning: Could not save settings: {}", e);
}

// On app exit (best effort)
save_config_silent(&settings);
```

## Tests

Run tests with:

```bash
cargo test config::persistence
```

Test coverage includes:
- Platform directory resolution
- Valid/invalid JSON loading
- Partial config (missing fields use defaults)
- Corrupted config fallback
- Save/load roundtrip
- Unknown fields ignored
- Value sanitization

**Session restoration tests** (in `src/state.rs`):
- `test_tab_from_tab_info` - TabInfo to Tab conversion
- `test_restore_session_tabs_empty_settings` - No tabs to restore
- `test_restore_session_tabs_with_missing_file` - Graceful handling
- `test_restore_session_tabs_skips_unsaved` - Unsaved tabs skipped
- `test_restore_session_tabs_active_index_clamped` - Index validation
- `test_restore_session_tabs_with_temp_file` - Full restoration
- `test_restore_multiple_tabs_with_temp_files` - Multiple tabs
- `test_restore_partial_tabs_missing_file` - Partial restoration

The module uses `tempfile` for isolated filesystem tests.
