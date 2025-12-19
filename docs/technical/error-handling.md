# Error Handling System

## Overview

Centralized error handling module providing a unified error type, custom Result alias, logging integration, and graceful degradation utilities for Ferrite.

## Key Files

- `src/error.rs` - Error enum, Display/Error traits, Result alias, logging helpers

## Implementation Details

### Error Enum Variants

The `Error` enum covers all application error scenarios:

| Category | Variants |
|----------|----------|
| **File I/O** | `Io`, `FileNotFound`, `PermissionDenied`, `FileRead`, `FileWrite` |
| **Configuration** | `ConfigLoad`, `ConfigSave`, `ConfigParse`, `ConfigInvalid`, `ConfigDirNotFound` |
| **Markdown** | `MarkdownParse`, `MarkdownRender` |
| **Syntax Highlighting** | `SyntaxLoad`, `ThemeLoad` |
| **Application** | `Application`, `Initialization` |

### Trait Implementations

- **`std::fmt::Display`** - User-friendly error messages
- **`std::error::Error`** - Error chaining via `source()` method
- **`From<io::Error>`** - Automatic I/O error conversion
- **`From<serde_json::Error>`** - JSON parsing error conversion

### Custom Result Type

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

### Error Classification

```rust
impl Error {
    /// Returns true if app can continue with defaults
    pub fn is_recoverable(&self) -> bool { ... }
    
    /// Returns true if app may need to exit
    pub fn is_critical(&self) -> bool { ... }
}
```

### Graceful Degradation

The `ResultExt` trait provides utilities for handling errors gracefully:

```rust
// Use default on error, log the error
let settings = load_settings()
    .unwrap_or_log_default(Settings::default(), "Failed to load settings");

// Convert recoverable errors to None
let optional = risky_operation().ok_or_log()?;
```

### Logging Integration

```rust
// Log error and return self (for chaining)
some_operation().map_err(|e| e.log())?;

// Log with additional context
some_operation().map_err(|e| e.log_with_context("During startup"))?;

// Initialize logging at app startup
error::init_logging();
```

## Dependencies Used

- `log` (0.4) - Logging facade
- `env_logger` (0.11) - Logger implementation
- `serde_json` - For JSON error conversion

## Usage

```rust
use crate::error::{Error, Result, ResultExt};

fn load_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::FileRead {
            path: path.to_path_buf(),
            source: e,
        })
}

// With graceful degradation
let content = load_file(path)
    .unwrap_or_log_default(String::new(), "File load failed");
```

## Tests

Run error handling tests:

```bash
cargo test error::tests
```

**27 tests** covering:
- Error variant creation
- Display trait formatting
- Error trait source chaining
- Result type alias
- Error classification (recoverable/critical)
- Graceful degradation utilities
