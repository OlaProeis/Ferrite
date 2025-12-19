# Dead Code Cleanup

## Overview

Task 39 performed a comprehensive cleanup of unused code throughout the codebase, reducing cargo warnings from 153 to 0 while maintaining all existing functionality. The cleanup focused on removing genuinely unused code while preserving aspirational WYSIWYG features with `#![allow(dead_code)]` annotations.

## Summary of Changes

### Deleted Files
- `src/files/operations.rs` - File I/O utilities that were never used (app uses direct `std::fs` calls)
- `src/theme/colors.rs` - Color constants and utilities never imported outside the module

### Removed from `src/error.rs`
| Item | Type | Reason |
|------|------|--------|
| `FileNotFound` | Enum variant | Unused error state |
| `PermissionDenied` | Enum variant | Unused error state |
| `FileRead` | Enum variant | App uses `FileWrite` only |
| `ConfigInvalid` | Enum variant | Replaced by `ConfigParse` |
| `MarkdownParse` | Enum variant | Parser never fails |
| `MarkdownRender` | Enum variant | Render functions removed |
| `SyntaxLoad` | Enum variant | Syntax loading is infallible |
| `ThemeLoad` | Enum variant | Theme loading is infallible |
| `Initialization` | Enum variant | Not used in current flow |
| `log()`, `log_with_context()` | Methods | Unused error logging |
| `is_recoverable()`, `is_critical()` | Methods | Unused error categorization |
| `unwrap_or_log_default()`, `ok_or_log()` | Trait methods | Unused ResultExt methods |
| `init_logging()` | Function | Logging initialized in main.rs |

### Removed from `src/config/`
| Item | File | Reason |
|------|------|--------|
| `ValidationError` | settings.rs | Validation removed in favor of sanitization |
| `Settings::new()` | settings.rs | Use `Default::default()` |
| `Settings::validate()` | settings.rs | Use `sanitize()` instead |
| `Settings::is_valid()` | settings.rs | Validation removed |
| `Settings::clear_recent_files()` | settings.rs | Unused method |
| `load_config_strict()` | persistence.rs | Use `load_config()` |
| `load_or_create_config()` | persistence.rs | Use `load_config()` |
| `config_exists()` | persistence.rs | Unused check |
| `delete_config()` | persistence.rs | Unused operation |

### Removed from `src/editor/`
| Item | File | Reason |
|------|------|--------|
| `LineNumberGutter` | line_numbers.rs | Logic inlined into widget.rs |
| `LineNumberGutterOutput` | line_numbers.rs | Struct for removed widget |
| `calculate_gutter_width_for_lines()` | line_numbers.rs | Inlined into widget.rs |
| `TextStats::format_detailed()` | stats.rs | Only `format_compact()` used |
| Helper functions | stats.rs | All stats via `TextStats::from_text()` |
| `EditorWidget::frame()` | widget.rs | Unused builder method |
| `EditorWidget::with_settings()` | widget.rs | Settings passed differently |
| `EditorOutput::response` | widget.rs | Unused field |
| `EditorOutput::cursor_position` | widget.rs | Unused field |

### Removed from `src/theme/`
| Item | File | Reason |
|------|------|--------|
| `ThemeFonts` struct | mod.rs | Font scaling not implemented |
| Entire `colors.rs` | colors.rs | Module never imported |

### Removed from `src/markdown/parser.rs`
| Item | Reason |
|------|--------|
| `render_to_html()` | Rendering not used (AST parsing only) |
| `render_to_html_with_options()` | Rendering not used |
| `MarkdownOptions::gfm()` | Use `Default::default()` |
| `MarkdownOptions::minimal()` | Not used |
| `MarkdownDocument::headings()` | Document inspection not used |
| `MarkdownDocument::code_blocks()` | Document inspection not used |
| `MarkdownDocument::links()` | Document inspection not used |
| `MarkdownDocument::images()` | Document inspection not used |
| `MarkdownDocument::is_empty()` | Document inspection not used |
| `MarkdownNode::is_block()` | Node inspection not used |
| `MarkdownNode::is_inline()` | Node inspection not used |
| `MarkdownNode::start_column` | Field never read |
| `MarkdownNode::end_column` | Field never read |

### Allowed Dead Code (Aspirational Features)

The following modules have `#![allow(dead_code)]` because they contain designed-but-not-fully-integrated WYSIWYG editing features:

- `src/markdown/widgets.rs` - Editable heading/paragraph/list widgets
- `src/markdown/syntax.rs` - Extended syntax highlighting features  
- `src/markdown/editor.rs` - WYSIWYG editor internals
- `src/state.rs` - Comprehensive state management methods
- `src/theme/mod.rs`, `dark.rs`, `light.rs`, `manager.rs` - Extended theme utilities

## Module Export Cleanup

### `src/files/mod.rs`
- Removed `pub mod operations;` (file deleted)
- Removed re-exports (functions imported directly where needed)

### `src/editor/mod.rs`
- Removed `calculate_gutter_width_for_lines` export

### `src/markdown/mod.rs`
- Removed `HeadingLevel`, `get_highlighter`, `highlight_code` exports

### `src/theme/mod.rs`
- Removed `mod colors;` (file deleted)

## Test Updates

Tests were updated to:
1. Remove tests for deleted functionality
2. Use alternative assertions (e.g., `settings.font_size` instead of `settings.is_valid()`)
3. Remove references to deleted methods and types

Final test count: **262 tests pass**

## Verification

```bash
cargo check   # 0 warnings
cargo build   # Success
cargo test    # 262 passed
cargo fmt     # Formatted
cargo clippy  # Only style suggestions remain
```

## Impact on Existing Features

All existing functionality preserved:
- ✅ File operations (open, save, save as)
- ✅ Tab management
- ✅ Theme switching (light/dark/system)
- ✅ Markdown editing (raw and rendered modes)
- ✅ Line numbers and text statistics
- ✅ Recent files menu
- ✅ Custom title bar and status bar
- ✅ Keyboard shortcuts
