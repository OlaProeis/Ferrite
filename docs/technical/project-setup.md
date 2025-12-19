# Project Setup

## Overview

This document describes the initial project setup and dependency configuration for Ferrite. The project was initialized as a Rust binary application using Cargo, with all required dependencies specified and verified.

## Key Files

- `Cargo.toml` - Project configuration and dependencies
- `src/main.rs` - Application entry point
- `Cargo.lock` - Locked dependency versions (auto-generated)

## Implementation Details

### Project Initialization

The project was created using:
```bash
cargo init --name sleek-markdown-editor --bin
```

This created:
- Binary application structure (not a library)
- Default `Cargo.toml` with package metadata
- Basic `src/main.rs` with "Hello, world!" template
- Rust edition set to 2021

### Dependencies Added

All dependencies were added to `Cargo.toml` according to the PRD specifications:

| Dependency | Version | Purpose |
|------------|---------|---------|
| `egui` | 0.28 | Immediate mode GUI framework |
| `eframe` | 0.28 | egui application framework |
| `comrak` | 0.22 | CommonMark-compliant markdown parser |
| `syntect` | 5.1 | Syntax highlighting for code blocks |
| `serde` | 1.x | Serialization framework (with derive feature) |
| `serde_json` | 1.x | JSON serialization support |
| `dirs` | 5 | Platform-specific directory paths |
| `open` | 5 | Open URLs in default browser |
| `rfd` | 0.14 | Native file dialogs |
| `log` | 0.4 | Logging facade |
| `env_logger` | 0.11 | Environment-based log configuration |
| `regex` | 1.x | Regular expressions for text processing |

### Build Verification

The project was verified to build successfully:
- All 516 dependency packages resolved and locked
- Compilation completed without errors
- `Cargo.lock` generated with exact dependency versions

## Dependencies Used

### GUI Framework
- **egui 0.28**: Immediate mode GUI that provides fast, cross-platform rendering
- **eframe 0.28**: Application framework that wraps egui with window management

### Markdown Processing
- **comrak 0.22**: Fast, CommonMark-compliant markdown parser
- **syntect 5.1**: Syntax highlighting engine (Sublime Text compatible)

### Serialization
- **serde 1.x**: Rust's standard serialization framework
- **serde_json 1.x**: JSON format support for serde

### Platform Integration
- **dirs 5**: Provides platform-specific config directories
- **rfd 0.14**: Native file dialogs (Windows, macOS, Linux)
- **open 5**: Opens URLs/links in default browser

### Utilities
- **log 0.4**: Rust logging facade
- **env_logger 0.11**: Environment-based log configuration (use `RUST_LOG=debug`)
- **regex 1.x**: Regular expressions for list/checkbox parsing

## Usage

### Building the Project

```bash
cargo build
```

### Running the Application

```bash
cargo run
```

### Checking for Compilation Errors

```bash
cargo check
```

## Project Structure

```
markDownNotepad/
├── Cargo.toml          # Project configuration
├── Cargo.lock          # Locked dependencies (auto-generated)
├── docs/               # Documentation
│   ├── index.md        # Documentation index
│   └── technical/      # Technical documentation
└── src/
    ├── main.rs         # Entry point, eframe setup
    ├── app.rs          # Main App struct, UI rendering
    ├── state.rs        # AppState, Tab, UiState
    ├── error.rs        # Error types
    ├── config/         # Settings and persistence
    ├── editor/         # Text editor widget with line numbers
    ├── files/          # File dialogs and operations
    ├── markdown/       # Parser, WYSIWYG editor, syntax highlighting
    └── theme/          # Theme colors and manager
```

## Next Steps

With the project initialized and dependencies configured, the next phase involves:
1. Creating the error handling module (`src/error.rs`)
2. Implementing configuration system
3. Building the application state management

## Related Documentation

- [PRD](../.taskmaster/docs/prd.txt) - Full project requirements
- [Architecture Overview](./index.md#architecture-overview) - Project structure

