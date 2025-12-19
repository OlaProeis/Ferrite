# Ferrite - Documentation

A fast, lightweight text editor for Markdown, JSON, and more. Built with Rust and egui.

## Quick Links

- [README](../README.md) - Project overview and installation
- [Building Guide](./building.md) - Build from source instructions
- [Contributing](../CONTRIBUTING.md) - Contribution guidelines

---

## Technical Documentation

| Document | Description |
|----------|-------------|
| [Project Setup](./technical/project-setup.md) | Initial project configuration, dependencies, and build setup |
| [Error Handling](./technical/error-handling.md) | Centralized error system, Result type, logging, graceful degradation |
| [Settings & Config](./technical/settings-config.md) | Settings struct, serialization, validation, sanitization |
| [Config Persistence](./technical/config-persistence.md) | Platform-specific config storage, load/save functions, fallback handling |
| [App State](./technical/app-state.md) | AppState, Tab, UiState structs, undo/redo, event handling |
| [eframe Window](./technical/eframe-window.md) | Window lifecycle, dynamic titles, responsive layout, state persistence |
| [Editor Widget](./technical/editor-widget.md) | Text editor widget, cursor tracking, scroll persistence, egui TextEdit integration |
| [File Dialogs](./technical/file-dialogs.md) | Native file dialogs with rfd, open/save operations |
| [Tab System](./technical/tab-system.md) | Tab data structure, tab bar UI, close buttons, unsaved changes dialog |
| [Keyboard Shortcuts](./technical/keyboard-shortcuts.md) | Global shortcuts for file ops, tab navigation, deferred action pattern |
| [Markdown Parser](./technical/markdown-parser.md) | Comrak integration, AST parsing, GFM support |
| [WYSIWYG Editor](./technical/wysiwyg-editor.md) | WYSIWYG markdown editing widget, source synchronization, theming |
| [Editable Widgets](./technical/editable-widgets.md) | Standalone editable widgets for headings, paragraphs, lists |
| [View Mode Persistence](./technical/view-mode-persistence.md) | Per-tab view mode storage, session restoration, backward compatibility |
| [Theme System](./technical/theme-system.md) | Unified theming with ThemeColors, ThemeManager, light/dark themes, runtime switching |
| [Line Numbers](./technical/line-numbers.md) | Editor line number display with scroll sync, dynamic width, theme integration |
| [Line Number Alignment](./technical/line-number-alignment.md) | Technical fix for line number drift, galley-based positioning |
| [Syntax Highlighting](./technical/syntax-highlighting.md) | Syntect integration for code block highlighting |
| [Text Statistics](./technical/text-statistics.md) | Word, character, line counting for status bar |
| [Recent Files](./technical/recent-files.md) | Recent files menu in status bar |
| [Custom Title Bar](./technical/custom-title-bar.md) | Windows-style custom title bar implementation |
| [Status Bar](./technical/status-bar.md) | Bottom status bar with file path, stats, toast messages |
| [Undo/Redo System](./technical/undo-redo.md) | Per-tab undo/redo with keyboard shortcuts (Ctrl+Z, Ctrl+Y) |
| [Ribbon UI](./technical/ribbon-ui.md) | Modern ribbon interface replacing menu bar, icon-based controls |
| [Settings Panel](./technical/settings-panel.md) | Modal settings UI with live preview, appearance/editor/files sections |
| [Find and Replace](./technical/find-replace.md) | Search functionality with regex, match highlighting, replace operations |
| [Dead Code Cleanup](./technical/dead-code-cleanup.md) | Task 39 cleanup summary, removed code, module changes |
| [Editable Code Blocks](./technical/editable-code-blocks.md) | Syntax-highlighted code blocks with edit mode, language selection |
| [Editable Links](./technical/editable-links.md) | Hover-based link editing with popup menu, autolink support |
| [Font System](./technical/font-system.md) | Custom font loading, EditorFont enum, bold/italic variants |
| [Click-to-Edit Formatting](./technical/click-to-edit-formatting.md) | Hybrid editing for formatted list items and paragraphs |
| [Formatting Toolbar](./technical/formatting-toolbar.md) | Markdown formatting toolbar, keyboard shortcuts, selection handling |
| [Outline Panel](./technical/outline-panel.md) | Document outline side panel, heading extraction, statistics for structured files |
| [Tree Viewer](./technical/tree-viewer.md) | JSON/YAML/TOML tree viewer with inline editing, expand/collapse, path copying |
| [Sync Scrolling](./technical/sync-scrolling.md) | Bidirectional scroll sync between Raw and Rendered views |
| [Document Export](./technical/document-export.md) | HTML export with themed CSS, Copy-as-HTML clipboard functionality |
| [Workspace Folder Support](./technical/workspace-folder-support.md) | Folder workspace mode, file tree, quick switcher, search in files, file watching |
| [Window Resize](./technical/window-resize.md) | Custom resize handles for borderless windows, edge detection, cursor icons |
| [Adaptive Toolbar](./technical/adaptive-toolbar.md) | File-type aware toolbar, conditional buttons for Markdown vs JSON/YAML/TOML |
| [About/Help Panel](./technical/about-help.md) | About dialog with version info, Help panel with keyboard shortcuts reference |
| [Branding](./branding.md) | Icon design, asset generation, platform integration guidelines |

---

## Guides

| Guide | Description |
|-------|-------------|
| *Coming soon* | Usage guides will be added as the app develops |

---

## Architecture Overview

```
ferrite/
├── src/
│   ├── main.rs           # Entry point, eframe setup
│   ├── app.rs            # Main App struct, update loop, custom title bar
│   ├── state.rs          # AppState, Tab, UiState, event handling
│   ├── error.rs          # Error types and handling
│   ├── fonts.rs          # Custom font loading and family selection
│   ├── config/           # Settings and persistence
│   │   ├── mod.rs        # Module exports
│   │   ├── settings.rs   # Settings struct, TabInfo, validation
│   │   └── persistence.rs # Config file load/save
│   ├── editor/           # Text editor widget
│   │   ├── mod.rs        # Module exports
│   │   ├── widget.rs     # EditorWidget with line numbers, search highlights
│   │   ├── line_numbers.rs # Line counting utilities
│   │   ├── stats.rs      # Text statistics (words, chars, lines)
│   │   ├── find_replace.rs # Find/replace panel and search logic
│   │   └── outline.rs    # Document outline extraction
│   ├── files/            # File operations
│   │   ├── mod.rs        # Module exports
│   │   └── dialogs.rs    # Native file dialogs (rfd)
│   ├── markdown/         # Parser and WYSIWYG editor
│   │   ├── mod.rs        # Module exports
│   │   ├── parser.rs     # Comrak integration, AST parsing
│   │   ├── editor.rs     # WYSIWYG markdown editor
│   │   ├── widgets.rs    # Editable heading/list/table widgets
│   │   ├── syntax.rs     # Syntax highlighting (syntect)
│   │   ├── ast_ops.rs    # AST operations and manipulation
│   │   ├── formatting.rs # Markdown formatting commands
│   │   └── tree_viewer.rs # JSON/YAML/TOML tree viewer widget
│   ├── preview/          # Preview and sync scrolling
│   │   ├── mod.rs        # Module exports
│   │   └── sync_scroll.rs # Bidirectional scroll synchronization
│   ├── export/           # Document export
│   │   ├── mod.rs        # Module exports
│   │   ├── html.rs       # HTML generation with theme CSS
│   │   ├── clipboard.rs  # Clipboard operations (arboard)
│   │   └── options.rs    # Export options and settings
│   ├── theme/            # Theming system
│   │   ├── mod.rs        # ThemeColors struct
│   │   ├── light.rs      # Light theme egui::Visuals
│   │   ├── dark.rs       # Dark theme egui::Visuals
│   │   └── manager.rs    # ThemeManager for runtime switching
│   ├── ui/               # UI components
│   │   ├── mod.rs        # Module exports
│   │   ├── about.rs      # About/Help panel with shortcuts reference
│   │   ├── icons.rs      # Icon loading for window/taskbar icons
│   │   ├── ribbon.rs     # Ribbon interface (replaces menu bar)
│   │   ├── settings.rs   # Settings panel modal
│   │   ├── outline_panel.rs # Document outline side panel
│   │   ├── file_tree.rs  # File tree sidebar panel
│   │   ├── quick_switcher.rs # Quick file switcher (Ctrl+P)
│   │   ├── search.rs     # Search in files (Ctrl+Shift+F)
│   │   ├── dialogs.rs    # File operation dialogs
│   │   └── window.rs     # Custom window resize for borderless windows
│   └── workspaces/       # Workspace/folder management
│       ├── mod.rs        # AppMode, Workspace, module exports
│       ├── file_tree.rs  # FileTreeNode, directory scanning
│       ├── settings.rs   # WorkspaceSettings persistence
│       ├── persistence.rs # WorkspaceState persistence
│       └── watcher.rs    # File system watcher (notify)
├── assets/               # Static assets
│   ├── fonts/            # TTF fonts (Inter, JetBrains Mono)
│   ├── icons/            # Application icons
│   │   ├── icon_*.png    # PNG icons (16-512px)
│   │   ├── windows/      # Windows .ico and .rc files
│   │   └── linux/        # Linux icons and .desktop file
│   └── web/              # Web favicon assets
├── build.rs              # Build script for Windows icon embedding
├── docs/                 # Documentation
└── .taskmaster/          # Task management
```

---

## Tech Stack

| Component | Technology | Version |
|-----------|------------|---------|
| Language | Rust | 1.70+ |
| GUI | egui + eframe | 0.28 |
| Markdown | comrak | 0.22 |
| Syntax Highlighting | syntect | 5.1 |
| Serialization | serde + serde_json | 1.x |
| YAML Parsing | serde_yaml | 0.9 |
| TOML Parsing | toml | 0.8 |
| File Dialogs | rfd | 0.14 |
| Platform Paths | dirs | 5 |
| URL Opening | open | 5 |
| Logging | log + env_logger | 0.4, 0.11 |
| Regex | regex | 1.x |
| Clipboard | arboard | 3 |
| File Watcher | notify | 6 |
| Fuzzy Matching | fuzzy-matcher | 0.3 |
| Icon Loading | image | 0.25 |
| Windows Icon | embed-resource | 2.4 |

---

## Development Notes

- Use `cargo build` to compile
- Use `cargo run` to run the application
- Use `cargo test` to run tests
- Use `cargo clippy` for linting

