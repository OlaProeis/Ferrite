# Settings & Configuration

## Overview

User preferences and application settings system with JSON serialization, validation, and sanitization for Ferrite.

## Key Files

- `src/config/mod.rs` - Config module declaration
- `src/config/settings.rs` - Settings struct, enums, validation

## Implementation Details

### Settings Struct

The `Settings` struct contains all user-configurable options:

```rust
pub struct Settings {
    // Appearance
    pub theme: Theme,              // Light, Dark, System
    pub view_mode: ViewMode,       // EditorOnly, PreviewOnly, SplitView
    pub show_line_numbers: bool,
    pub font_size: f32,
    pub font_family: EditorFont,   // Inter, JetBrainsMono (see font-system.md)
    
    // Editor Behavior
    pub word_wrap: bool,
    pub tab_size: u8,
    pub use_spaces: bool,
    pub auto_save: bool,
    pub auto_save_interval_secs: u32,
    
    // Session & History
    pub recent_files: Vec<PathBuf>,
    pub max_recent_files: usize,
    pub last_open_tabs: Vec<TabInfo>,
    pub active_tab_index: usize,
    
    // Window State
    pub window_size: WindowSize,
    pub split_ratio: f32,
    
    // Syntax Highlighting
    pub syntax_theme: String,
}
```

### Supporting Types

| Type | Purpose |
|------|---------|
| `Theme` | Color theme enum (Light, Dark, System) |
| `ViewMode` | Editor layout enum (EditorOnly, PreviewOnly, SplitView) |
| `EditorFont` | Font selection enum (Inter, JetBrainsMono) - see [Font System](./font-system.md) |
| `WindowSize` | Window dimensions and position |
| `TabInfo` | Open tab state for session restoration |

### Serialization

All types derive `Serialize` and `Deserialize` with serde attributes:

- `#[serde(default)]` - Use defaults for missing fields
- `#[serde(rename_all = "lowercase")]` - Consistent JSON keys
- `#[serde(skip_serializing_if = "Option::is_none")]` - Omit None values

### Validation

```rust
impl Settings {
    // Validation constraints
    pub const MIN_FONT_SIZE: f32 = 8.0;
    pub const MAX_FONT_SIZE: f32 = 72.0;
    pub const MIN_TAB_SIZE: u8 = 1;
    pub const MAX_TAB_SIZE: u8 = 8;
    
    /// Returns list of validation errors
    pub fn validate(&self) -> Vec<ValidationError> { ... }
    
    /// Check if settings are valid
    pub fn is_valid(&self) -> bool { ... }
    
    /// Clamp values to valid ranges
    pub fn sanitize(&mut self) { ... }
    
    /// Load and sanitize in one step
    pub fn from_json_sanitized(json: &str) -> Result<Self, serde_json::Error> { ... }
}
```

### Validated Fields

| Field | Constraint |
|-------|------------|
| `font_size` | 8.0 - 72.0 |
| `tab_size` | 1 - 8 |
| `window_size.width/height` | 200.0 - 10000.0 |
| `split_ratio` | 0.0 - 1.0 |
| `max_recent_files` | 1 - 100 |
| `auto_save_interval_secs` | ≥5 when auto_save enabled |

## Dependencies Used

- `serde` (1.x) - Serialization framework
- `serde_json` (1.x) - JSON support

## Usage

```rust
use crate::config::{Settings, Theme, ViewMode};

// Create with defaults
let settings = Settings::default();

// Load from JSON with sanitization
let settings = Settings::from_json_sanitized(json_str)?;

// Add recent file
settings.add_recent_file(PathBuf::from("/path/to/file.md"));

// Validate before saving
if !settings.is_valid() {
    settings.sanitize();
}
```

### JSON Example

```json
{
  "theme": "dark",
  "view_mode": "splitview",
  "show_line_numbers": true,
  "font_size": 14.0,
  "font_family": "inter",
  "word_wrap": true,
  "tab_size": 4,
  "recent_files": ["/path/to/file.md"],
  "window_size": {
    "width": 1200.0,
    "height": 800.0
  },
  "split_ratio": 0.5
}
```

Note: `font_family` accepts `"inter"` or `"jetbrainsmono"` (lowercase).

## Tests

Run settings tests:

```bash
cargo test config::settings::tests
```

**27 tests** covering:
- Default values
- Serialization roundtrip
- Missing field defaults
- Validation constraints
- Sanitization behavior
- Recent files management
