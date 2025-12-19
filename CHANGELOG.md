# Changelog

All notable changes to Ferrite will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-XX

### Added

#### Core Editor
- Multi-tab file editing with unsaved changes tracking
- Three view modes: Raw, Rendered, and Split (Both)
- Full undo/redo support per tab (Ctrl+Z, Ctrl+Y)
- Line numbers with scroll synchronization
- Text statistics (words, characters, lines) in status bar

#### Markdown Support
- WYSIWYG markdown editing with live preview
- Click-to-edit formatting for lists, headings, and paragraphs
- Formatting toolbar (bold, italic, headings, lists, links, code)
- Sync scrolling between raw and rendered views
- Syntax highlighting for code blocks (syntect)
- GFM (GitHub Flavored Markdown) support via comrak

#### Multi-Format Support
- JSON file editing with tree viewer
- YAML file editing with tree viewer
- TOML file editing with tree viewer
- Tree viewer features: expand/collapse, inline editing, path copying
- File-type aware adaptive toolbar

#### Workspace Features
- Open folders as workspaces
- File tree sidebar with expand/collapse
- Quick file switcher (Ctrl+P) with fuzzy matching
- Search in files (Ctrl+Shift+F) with results panel
- File system watching for external changes
- Workspace settings persistence (.ferrite/ folder)

#### User Interface
- Modern ribbon-style toolbar
- Custom borderless window with title bar
- Custom resize handles for all edges and corners
- Light and dark themes with runtime switching
- Document outline panel for navigation
- Settings panel with appearance, editor, and file options
- About dialog with version info
- Help panel with keyboard shortcuts reference
- Native file dialogs (open, save, save as)
- Recent files menu in status bar
- Toast notifications for user feedback

#### Export Features
- Export document to HTML file with themed CSS
- Copy as HTML to clipboard

#### Platform Support
- Windows executable with embedded icon
- Linux .desktop file for application integration
- macOS support (untested)

#### Developer Experience
- Comprehensive technical documentation
- Optimized release profile (LTO, symbol stripping)
- Makefile for common build tasks
- Clean codebase with zero clippy warnings

### Technical Details
- Built with Rust 1.70+ and egui 0.28
- Immediate mode GUI architecture
- Per-tab state management
- Platform-specific configuration storage
- Graceful error handling with fallbacks

---

## Version History

- **0.1.0** - Initial public release

[Unreleased]: https://github.com/OlaProeis/Ferrite/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/OlaProeis/Ferrite/releases/tag/v0.1.0
