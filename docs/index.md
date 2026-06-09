# Documentation Index

<!-- RULES: This file is a pure documentation map. NO project history, NO task lists,
     NO architecture overviews or directory trees, NO tech stack tables.
     Only curated links with one-line descriptions. Update when adding new docs. -->

## Core Context
- [AI Context](./ai-context.md) - Core project architecture, rules, and conventions (attach to every AI chat)
- [README](../README.md) - Project overview and installation
- [macOS install & Gatekeeper](./install/macos.md) - Unsigned CI bundles, Gatekeeper / quarantine workarounds (#130)
- [Building Guide](./building.md) - Build from source instructions
- [CLI Reference](./cli.md) - Command-line interface documentation
- [Contributing](../CONTRIBUTING.md) - Contribution guidelines

---

## Technical Documentation

### Configuration & Setup

| Document | Description |
|----------|-------------|
| [Project Setup](./technical/config/project-setup.md) | Initial project configuration, dependencies, and build setup |
| [Error Handling](./technical/config/error-handling.md) | Centralized error system, Result type, logging, graceful degradation |
| [Settings & Config](./technical/config/settings-config.md) | Settings struct, serialization, validation, sanitization |
| [Config Persistence](./technical/config/config-persistence.md) | Platform-specific config storage, load/save functions, fallback handling |
| [Log Level Config](./technical/config/log-level-config.md) | Configurable log verbosity via config.json and --log-level CLI flag |
| [Internationalization](./technical/config/i18n.md) | rust-i18n integration, Language enum, translation keys, adding languages |
| [Multi-Encoding Support](./technical/config/multi-encoding.md) | Character encoding detection (chardetng), manual selection, save in original encoding |
| [Snippets System](./technical/config/snippets-system.md) | Text expansion system with built-in date/time snippets and custom user snippets |
| [New File Save Prompt](./technical/config/new-file-save-prompt.md) | Skip save prompt for unmodified untitled files, `should_prompt_to_save(&Settings)` logic |
| [Quick note workflow](./technical/config/quick-note-workflow.md) | Ephemeral untitled tabs: on by default (no-prompt quit; tab close still prompts); turn off in Settings; session recovery; rename |
| [Default View Mode](./technical/config/default-view-mode.md) | Per-file-type default view mode configuration |
| [Code execution settings](./technical/config/code-execution-settings.md) | Opt-in prefs for markdown code-block runners (timeout, shell/Python gates) |

### Editor Core

| Document | Description |
|----------|-------------|
| **[Architecture](./technical/editor/architecture.md)** | **REQUIRED READING: Core principles, complexity tiers, memory budget, anti-patterns** |
| **[FerriteEditor](./technical/editor/ferrite-editor.md)** | **Custom editor widget: TextBuffer, ViewState, LineCache; undo via Tab::edit_history** |
| **[TextBuffer](./technical/editor/text-buffer.md)** | **Rope-based text buffer for O(log n) editing operations on large files** |
| **[EditHistory](./technical/editor/edit-history.md)** | **Operation-based undo/redo for memory-efficient large file editing** |
| **[ViewState](./technical/editor/view-state.md)** | **Viewport tracking and visible line range calculation for virtual scrolling** |
| **[LineCache](./technical/editor/line-cache.md)** | **LRU-cached galley storage for efficient text rendering without recreation each frame** |
| **[LineCache Smart Invalidation](./technical/editor/line-cache-smart-invalidation.md)** | **Targeted range invalidation and dynamic cache sizing for large-file editing performance** |
| **[Large File Performance](./technical/editor/large-file-performance.md)** | **Per-frame optimizations for 5MB+ files; open-time warning toast for 10MB+** |
| **[Memory Optimization](./technical/editor/memory-optimization.md)** | **Tab closure cleanup, FerriteEditorStorage management, debug vs release performance** |
| **[Word Wrap](./technical/editor/word-wrap.md)** | **Phase 2 word wrap support: visual row tracking, wrapped galley caching, cursor navigation** |
| [Editor Widget](./technical/editor/editor-widget.md) | Text editor widget, cursor tracking, scroll persistence, egui TextEdit integration |
| [Line Numbers & Gutter](./technical/editor/line-numbers.md) | Gutter system with toggleable line numbers and fold indicators, dynamic width calculation |
| [Line Number Alignment](./technical/editor/line-number-alignment.md) | Technical fix for line number drift, galley-based positioning |
| [Cursor Position Mapping](./technical/editor/cursor-position-mapping.md) | Raw-to-displayed text position mapping for formatted content editing |
| [Galley Cursor Positioning](./technical/editor/galley-cursor-positioning.md) | Pixel-accurate cursor placement using egui Galley text layout |
| [Undo/Redo System](./technical/editor/undo-redo.md) | Per-tab undo/redo with keyboard shortcuts (Ctrl+Z, Ctrl+Y) |
| [Undo Hash Change Detection](./technical/editor/undo-hash-change-detection.md) | Blake3 hash-based undo snapshot to eliminate per-frame content clones |
| [Find and Replace](./technical/editor/find-replace.md) | Search functionality with regex, match highlighting, replace operations |
| [Go to Line](./technical/editor/go-to-line.md) | Ctrl+G modal dialog for line navigation, viewport centering |
| [Duplicate Line](./technical/editor/duplicate-line.md) | Ctrl+Shift+D line/selection duplication, char-to-byte index handling |
| [Move Line](./technical/editor/move-line.md) | Alt+Up/Down line reordering, pre-render key consumption, cursor following |
| [Code Folding](./technical/editor/code-folding.md) | Fold region detection, gutter indicators, content hiding |
| [Code Folding UI](./technical/editor/code-folding-ui.md) | Code folding user interface and interactions |
| [Multi-Cursor Editing](./technical/editor/multi-cursor.md) | Multiple cursor support with Ctrl+Click, simultaneous editing, selection merging |
| [Semantic Minimap](./technical/editor/semantic-minimap.md) | Semantic minimap with clickable heading labels, content type indicators, density bars |
| [Editor Minimap (Legacy)](./technical/editor/minimap.md) | VS Code-style pixel minimap (replaced by semantic minimap) |
| [Search Highlight](./technical/editor/search-highlight.md) | Search-in-files result navigation with transient highlight, auto Raw mode switch |
| [Search Highlight Rendered View](./technical/editor/search-highlight-rendered-view.md) | Rendered view search highlights (incl. tables) and floating panel z-order stacking |
| [Search Highlight Edit Recompute](./technical/editor/search-highlight-edit-recompute.md) | Fix stale search highlights after document edits by recomputing match positions |
| [Syntax Highlighting](./technical/editor/syntax-highlighting.md) | Syntect integration for code block highlighting |
| [Auto-close Brackets](./technical/editor/auto-close-brackets.md) | Auto-pair insertion, selection wrapping, skip-over behavior for brackets/quotes |
| [Bracket Matching](./technical/editor/bracket-matching.md) | Highlight matching brackets and parentheses |
| [Vim Mode](./technical/editor/vim-mode.md) | Optional modal editing with Normal/Insert/Visual modes, Vim keybindings |
| [Windows IME layer transform](./technical/editor/windows-ime-layer-transform.md) | IMEOutput in screen space via layer TSTransform (candidate box alignment) |
| [Word Wrap Scroll Fixes](./technical/editor/word-wrap-scroll-fixes.md) | Correctness fixes for pixel_to_line, line_to_pixel, scroll sync when word wrap active |
| [Word Wrap Performance](./technical/editor/word-wrap-performance.md) | Incremental height cache, O(1) LRU, O(log N) visual row mapping |
| [Ctrl+Scroll Zoom](./technical/editor/ctrl-scroll-zoom.md) | Ctrl+Mouse Wheel zoom mapped to egui::gui_zoom, ZoomIn/ZoomOut/ResetZoom shortcuts |
| [Font System](./technical/editor/font-system.md) | Custom font loading, EditorFont enum, bold/italic variants, CJK/complex script lazy loading |
| [HarfRust text shaping](./technical/editor/harfrust-text-shaping.md) | harfrust 0.5.2 OTL shaping: cluster grouping, shaped-line cache, per-cluster rendering |
| [Grapheme-Cluster Cursor](./technical/editor/grapheme-cluster-cursor.md) | Grapheme-cluster-aware arrow keys, backspace, delete for emoji ZWJ, Bengali, Korean |
| [Uniform Height Large Files](./technical/editor/uniform-height-large-files.md) | Uniform line heights for 100K+ line files: O(1) memory, force-disabled word wrap |
| [Custom Font Selection](./technical/editor/custom-font-selection.md) | System font enumeration, custom font picker, CJK regional preferences |
| [Complex Script Font Preferences](./technical/config/complex-script-font-preferences.md) | Per-script font preferences for Arabic, Bengali, Devanagari, Thai, Hebrew, Tamil, etc. |
| [CJK Font Preloading Verification](./technical/fonts/cjk-font-preloading-verification.md) | Verification that explicit CJK preferences preload correctly at startup |
| [Custom Font Crash Prevention](./technical/fonts/custom-font-crash-prevention.md) | Magic-byte validation, catch_unwind, graceful fallback for invalid custom fonts |
| [Custom font picker deferred load](./technical/fonts/custom-font-picker-deferred-load.md) | Custom mode waits for an explicit combo pick before loading; fixes macOS #133 toast |

### UI Components

| Document | Description |
|----------|-------------|
| [Ribbon UI](./technical/ui/ribbon-ui.md) | Modern ribbon interface replacing menu bar, icon-based controls |
| [Ribbon Redesign](./technical/ui/ribbon-redesign.md) | Design C streamlined ribbon, title bar integration, dropdown menus |
| **[Special Tabs](./technical/ui/special-tabs.md)** | **Tab-based UI panels (Settings, About/Help) replacing modal windows** |
| [Settings Panel](./technical/ui/settings-panel.md) | Settings UI in a special tab, live preview, appearance/editor/files/keyboard/terminal sections |
| [Outline Panel](./technical/ui/outline-panel.md) | Document outline side panel, heading extraction, statistics for structured files |
| [Backlinks Panel](./technical/ui/backlinks-panel.md) | Backlinks panel showing files linking to current file, adaptive indexing, click-to-navigate |
| [Status Bar](./technical/ui/status-bar.md) | Bottom status bar with file path, stats, toast messages |
| [About/Help Panel](./technical/ui/about-help.md) | About/Help in a special tab, version info, keyboard shortcuts reference |
| [Zen Mode](./technical/ui/zen-mode.md) | Distraction-free writing mode, centered text column, chrome hiding, F11 toggle |
| [Split View](./technical/ui/split-view.md) | Side-by-side raw editor + rendered preview, draggable splitter, independent scrolling |
| [Search Panel Viewport](./technical/ui/search-panel-viewport.md) | Viewport constraints for Search panel, DPI handling, resize behavior |
| [Quick Switcher Mouse Support](./technical/ui/quick-switcher-mouse-support.md) | Mouse hover/click fix with layer-based background, interaction overlay |
| [Command Palette](./technical/ui/command-palette.md) | Alt+Space searchable command launcher with fuzzy search, recent commands, deferred dispatch |
| [Keyboard Shortcuts](./technical/ui/keyboard-shortcuts.md) | Global shortcuts for file ops, tab navigation, deferred action pattern |
| [Keyboard Shortcut Customization](./technical/ui/keyboard-shortcut-customization.md) | Settings panel for rebinding shortcuts with conflict detection, persistence |
| [Light Mode Contrast](./technical/ui/light-mode-contrast.md) | WCAG AA color tokens, contrast ratios, border/text improvements |
| [Light Mode Strong Text Fix](./technical/ui/light-mode-strong-text-fix.md) | Fix invisible RichText::strong() labels in light mode |
| [Theme System](./technical/ui/theme-system.md) | Light/dark/System themes, ThemeManager, **user-configurable Ferrite accent** (headings, selection, chrome; links stay classic blue) |
| [Adaptive Toolbar](./technical/ui/adaptive-toolbar.md) | File-type aware toolbar, conditional buttons for Markdown vs JSON/YAML/TOML |
| [Navigation Buttons](./technical/ui/nav-buttons.md) | Document navigation overlay for quick jumping to top, middle, or bottom |
| [Frontmatter Panel](./technical/ui/frontmatter-panel.md) | Visual YAML frontmatter editor, form-based key-value editing, tag chips, bidirectional sync |
| [Header Spacing](./technical/ui/header-spacing.md) | Adjustable vertical spacing between headings (H1-H6) in rendered view |
| [Check for Updates](./technical/ui/check-for-updates.md) | Manual update checker via GitHub Releases API, security model, URL validation |

### Markdown & WYSIWYG

| Document | Description |
|----------|-------------|
| [Markdown Parser](./technical/markdown/markdown-parser.md) | Comrak integration, AST parsing, GFM support |
| [WYSIWYG Editor](./technical/markdown/wysiwyg-editor.md) | WYSIWYG markdown editing widget, source synchronization, theming |
| [WYSIWYG Interactions](./technical/markdown/wysiwyg-interactions.md) | WYSIWYG user interaction patterns and behaviors |
| [Editable Widgets](./technical/markdown/editable-widgets.md) | Standalone editable widgets for headings, paragraphs, lists |
| [Editable Code Blocks](./technical/markdown/editable-code-blocks.md) | Syntax-highlighted code blocks with edit mode, language selection |
| [Code block Run](./technical/markdown/code-block-run.md) | Run control in rendered/split preview: background worker, ANSI inline output (CRLF-safe on Windows), ✓/✗ exit, insert-as-fenced-block, **Stop** + hard timeout; § Known limitations + link to manual test file |
| [Code execution consent dialog](./technical/markdown/code-execution-consent-dialog.md) | First-run modal when clicking **Run** before consent; queues payload; Settings toggle skips modal |
| [Code block Run cancellation & timeout](./technical/markdown/code-block-cancellation.md) | `RunStatus::Cancelled`, atomic cancel token, Stop button, `Timed out after Ns` / `Stopped by user` labels, reader-thread shutdown |
| [Editable Links](./technical/markdown/editable-links.md) | Hover-based link editing with popup menu, autolink support |
| [Editable Tables](./technical/markdown/editable-tables.md) | GFM table editing, deferred commits, toolbar, markdown sync |
| [Table cell focus & navigation](./technical/markdown/table-cell-focus-navigation.md) | Empty-cell hit targets, Tab / Shift+Tab in-table (`lock_focus`, consume order) |
| [Click-to-Edit Formatting](./technical/markdown/click-to-edit-formatting.md) | Hybrid editing for formatted list items and paragraphs (superseded — see Rendered edit session: formatted blocks) |
| [Rendered edit session (overview)](./technical/markdown/rendered-edit-session.md) | Architecture hub: motivation, `source_epoch`, `BlockRef`, session API, commit policy, RS-1…RS-7 / TBLE matrix, design decisions |
| [Rendered edit session (Phase 0)](./technical/markdown/rendered-edit-session-phase0.md) | Formatted blur hotfix + `Tab::source_epoch`; foundation before full session coordinator |
| [Rendered edit session (core types)](./technical/markdown/rendered-edit-session-core.md) | `BlockRef`, `RenderedEditSession` state machine and tab-scoped egui storage |
| [Rendered edit session (headings)](./technical/markdown/rendered-edit-session-headings.md) | Headings wired to session: switch_to_ui, buffer commit, one-click cross-heading switch |
| [Rendered edit session (paragraphs & lists)](./technical/markdown/rendered-edit-session-paragraphs-lists.md) | Plain paragraphs and simple list items on session; epoch invalidation; cross-block switch with headings |
| [Rendered edit session (formatted blocks)](./technical/markdown/rendered-edit-session-formatted.md) | Formatted paragraphs and list items on session: click-to-edit, display→raw cursor mapping, Enter/Escape; replaces `FormattedItemEditState` and `formatted_exit_should_save` |
| [Rendered edit session (tables)](./technical/markdown/rendered-edit-session-tables.md) | `BlockRef::TableCell` activation + `signal_table_force_commit` one-shot signal: cross-block exit commits the table; intra-table Tab navigation preserves deferred commits |
| [Rendered edit session (split view)](./technical/markdown/rendered-edit-session-split-view.md) | `rendered_editor_id(tab.id)` shared by rendered-only and split preview; raw-pane epoch bumps invalidate session buffers (RS-6) |
| [Rendered edit session (undo)](./technical/markdown/rendered-edit-session-undo.md) | One logical undo step per block commit; session keystrokes stay off the undo stack until close/switch |
| [Rendered widget identity](./technical/markdown/rendered-widget-identity.md) | `ui.push_id(editor_id + source_epoch)` for stable TextEdit ids; `content_hash` for culling only |
| [Formatting Toolbar](./technical/markdown/formatting-toolbar.md) | Markdown formatting toolbar, keyboard shortcuts, selection handling |
| [Emphasis Rendering](./technical/markdown/emphasis-rendering.md) | Bold, italic, strikethrough rendering in WYSIWYG |
| [Table of Contents](./technical/markdown/table-of-contents.md) | TOC generation from headings, anchor links, update/insert modes |
| [Mermaid insert toolbar](./technical/markdown/mermaid-insert-toolbar.md) | Format toolbar combo: insert fenced Mermaid templates at cursor (Raw / Split) |
| [Mermaid syntax help](./technical/mermaid/mermaid-syntax-help.md) | About / Help (F1) tab: per-diagram descriptions and snippets aligned with Insert → Mermaid… |
| [List Editing Fixes](./technical/markdown/list-editing-fixes.md) | Frontmatter offset fix, edit buffer persistence, deferred commits, rendered-mode undo/redo |
| [List Editing Debug](./technical/markdown/list-editing-debug.md) | Debugging list editing issues and fixes |
| [Task List Checkbox](./technical/markdown/task-list-checkbox.md) | Interactive task list checkboxes in rendered view; click-to-toggle with source sync; scroll-stable via structure-preserving culling |
| [Table Editing Focus](./technical/markdown/table-editing-focus.md) | Fix cursor loss during table cell editing, deferred source updates |
| [Smart Paste](./technical/markdown/smart-paste.md) | URL detection, markdown link creation with selection, image markdown insertion |
| [Image Drag & Drop](./technical/markdown/image-drag-drop.md) | Drag images into editor, auto-save to assets/, insert markdown link |
| [CJK Paragraph Indentation](./technical/markdown/cjk-paragraph-indentation.md) | First-line paragraph indentation for Chinese (2em) and Japanese (1em) |
| [Block Element Alignment](./technical/markdown/block-element-alignment.md) | Consistent 4px left indent for tables, code blocks, blockquotes |
| [GitHub-Style Callouts](./technical/markdown/github-callouts.md) | GitHub-style callouts with color-coded rendering, collapse toggle |
| [Wikilinks](./technical/markdown/wikilinks.md) | `[[target]]` syntax, file resolution, click-to-navigate, broken link indicators |
| [Image Rendering](./technical/markdown/image-rendering.md) | Local image display in rendered/split view, path resolution, texture caching |
| [Setext Heading Detection](./technical/markdown/setext-heading-detection.md) | Single-dash false setext fix, backwards-scan underline detection |
| [Markdown AST Caching](./technical/markdown/markdown-ast-caching.md) | Blake3 content-hash AST cache to skip re-parsing unchanged markdown |
| [Rendered View Viewport Culling](./technical/markdown/rendered-view-viewport-culling.md) | show_viewport() two-phase culling with 500px overscan for large-document performance |
| [Block-Level Height Cache](./technical/markdown/block-level-height-cache.md) | Per-block blake3-keyed LRU height cache for off-screen block measurement skip |
| [Consecutive Fenced Blocks Fix](./technical/markdown/consecutive-fenced-blocks-fix.md) | issue #129 — horizontal `ScrollArea` `auto_shrink_y` fix so consecutive fenced blocks all stay visible |
| [Strict Line Breaks](./technical/markdown/strict-line-breaks.md) | Optional setting treating single newlines as hard `<br>` breaks |
| [Lazy Block Height Estimation](./technical/markdown/lazy-block-height-estimation.md) | Heuristic heights for unmeasured blocks, render budget cap, progressive refinement |
| [Paragraph Trailing Spaces](./technical/markdown/paragraph-trailing-spaces.md) | Fix for trailing spaces lost in plain paragraphs via persistent edit buffer |
| [Rendered Paragraph Block Spacing](./technical/markdown/rendered-paragraph-block-spacing.md) | Trailing space after block paragraphs and code blocks; viewport height alignment |
| [Table Inline Formatting](./technical/markdown/table-inline-formatting.md) | Preserve and render bold, italic, strikethrough, code in table cells (serialization + rich text display) |
| [Video embed parsing](./technical/markdown/video-embed-parsing.md) | `{{video URL}}` and bare YouTube paragraph syntax; `VideoEmbed` AST node, allowlist, round-trip `source_text` |

### Data Viewers

| Document | Description |
|----------|-------------|
| [CSV Viewer](./technical/viewers/csv-viewer.md) | CSV/TSV table viewer with scrolling, header highlighting, cell tooltips |
| **[CSV Lazy Parsing](./technical/viewers/csv-lazy-parsing.md)** | **Byte-offset row indexing for large CSVs, on-demand visible-row parsing** |
| [CSV Delimiter Detection](./technical/viewers/csv-delimiter-detection.md) | Auto-detect delimiter (comma/tab/semicolon/pipe), manual override |
| [CSV Header Detection](./technical/viewers/csv-header-detection.md) | Auto-detect header rows with heuristics, toggle UI, column alignment |
| [CSV Rainbow Columns](./technical/viewers/csv-rainbow-columns.md) | Subtle alternating column colors using Oklch, status bar toggle |
| [CSV Raw View Caching](./technical/viewers/csv-raw-view-caching.md) | Blake3 hash-guarded raw text cache to eliminate per-frame string allocation |
| [Image Viewer](./technical/viewers/image-viewer.md) | Dedicated image viewer tabs (PNG/JPEG/GIF/WebP/BMP) with zoom and metadata |
| [PDF Viewer](./technical/viewers/pdf-viewer.md) | Read-only PDF viewer tabs with hayro rendering, page navigation, zoom |
| [Print preview](./technical/viewers/print-preview.md) | Same `render_markdown_to_pdf` as Export PDF; temp file → `PdfViewer` tab; ephemeral session/temp cleanup |
| [Tree Viewer](./technical/viewers/tree-viewer.md) | JSON/YAML/TOML tree viewer with inline editing, expand/collapse, path copying |
| [Tree Viewer Caching](./technical/viewers/tree-viewer-caching.md) | Blake3-guarded parse cache and raw text buffer to avoid per-frame work |
| [Live Pipeline](./technical/viewers/live-pipeline.md) | JSON/YAML command piping through shell commands (jq, yq), recent history |
| [Document Export](./technical/viewers/document-export.md) | Themed HTML export (options dialog, Mermaid SVG, syntect blocks), clipboard HTML, PDF export pointer |
| [Themed HTML export](./technical/viewers/themed-html-export.md) | Implementation map: comrak adapter, Mermaid SVG, theme resolution, image/link post-process |
| **[PDF Export](./technical/viewers/pdf-export.md)** | **v0.3.x: Native-Rust PDF export via `krilla` (fonts, page size/margins, H1 page-break dialog, link annotations)** |

### File Operations & Workspaces

| Document | Description |
|----------|-------------|
| [File Dialogs](./technical/files/file-dialogs.md) | Native file dialogs with rfd, open/save operations |
| [Tab System](./technical/files/tab-system.md) | Tab data structure, tab bar UI, close buttons, unsaved changes dialog |
| [Recent Files](./technical/files/recent-files.md) | Recent files menu in status bar |
| [Workspace Folder Support](./technical/files/workspace-folder-support.md) | Folder workspace mode, file tree, quick switcher, search in files, file watching |
| [Workspace File Index](./technical/files/workspace-file-index.md) | Background full-tree index for Ctrl+P and Ctrl+Shift+F (independent of lazy file tree) |
| [Session Persistence](./technical/files/session-persistence.md) | Crash-safe session state, tab restoration, recovery dialog, lock file mechanism |
| [Auto-Save](./technical/files/auto-save.md) | Configurable auto-save with temp file backups, toolbar toggle, recovery dialog |
| [Git Integration](./technical/files/git-integration.md) | Branch display in status bar, file tree Git status badges, git2 integration |
| [Git Auto-Refresh](./technical/files/git-auto-refresh.md) | Automatic git status refresh on save, focus, and periodic intervals |

### Terminal Emulator

| Document | Description |
|----------|-------------|
| [Terminal Architecture](./technical/terminal/terminal-architecture.md) | Integrated terminal with PTY (portable-pty), VTE parsing, screen buffer, ANSI color |
| [Terminal UI](./technical/terminal/terminal-ui.md) | Terminal panel with tabs, split panes, floating windows, drag-and-drop |
| [Terminal Themes](./technical/terminal/terminal-themes.md) | Terminal color schemes (Solarized, Dracula, Monokai, Nord, etc.) |
| [Terminal Layout](./technical/terminal/terminal-layout.md) | Split pane layouts (horizontal/vertical), grid creation, layout save/load |
| [Terminal CJK Wide Chars](./technical/terminal/terminal-cjk-wide-chars.md) | Double-width CJK character rendering, cursor advancement, selection snapping |

### Productivity Hub

| Document | Description |
|----------|-------------|
| [Productivity Panel](./technical/productivity/productivity-panel.md) | Task management, Pomodoro timer, quick notes with workspace-scoped persistence |

### Async Workers

| Document | Description |
|----------|-------------|
| [Worker Infrastructure](./technical/workers/worker-infrastructure.md) | Background tokio runtime, channel-based UI communication, worker pattern |

### Platform-Specific

| Document | Description |
|----------|-------------|
| [eframe Window](./technical/platform/eframe-window.md) | Window lifecycle, dynamic titles, responsive layout, state persistence |
| **[eframe/egui 0.31 Upgrade](./technical/platform/eframe-egui-031-upgrade.md)** | **v0.3.0 GUI stack bump from 0.28 → 0.31.1 — breaking API migration patterns and validation** |
| [eframe/egui 0.34 Upgrade](./technical/platform/eframe-egui-034-upgrade.md) | **v0.3.0 GUI stack bump to 0.34.2 — viewport rects, Popup API, skrifa/HarfRust, MSRV 1.92** |
| **[v0.3.0 Cross-Platform Regression Matrix](./technical/platform/v0.3.0-regression-matrix.md)** | **Manual regression matrix for v0.3.0 (egui 0.31 + 0.34 delta, Task 89 §8)** |
| [Custom Title Bar](./technical/platform/custom-title-bar.md) | Windows-style custom title bar implementation |
| [Window Resize](./technical/platform/window-resize.md) | Custom resize handles for borderless windows, edge detection |
| [Windows Borderless Window](./technical/platform/windows-borderless-window.md) | Top edge resize fix, fullscreen toggle (F10), title bar button area exclusion |
| [Windows Borderless Transparent Fix](./technical/platform/windows-borderless-transparent-fix.md) | Fix rendering offset (black bars) on Intel GPUs via `with_transparent(true)` DWM workaround |
| [Windows Path Normalization](./technical/platform/windows-path-normalization.md) | Strip Windows `\\?\` prefix from canonicalized paths |
| [Linux Cursor Flicker Fix](./technical/platform/linux-cursor-flicker-fix.md) | Title bar exclusion zone to prevent cursor conflicts with window controls |
| **[Idle Mode Optimization](./technical/platform/idle-mode-optimization.md)** | **Tiered idle repaint system to reduce CPU usage on all platforms** |
| **[SignPath Code Signing](./technical/platform/signpath-code-signing.md)** | **Windows code signing via SignPath for OSS** |
| **[Single-Instance Protocol](./technical/platform/single-instance.md)** | **Lock file + TCP IPC to open files in existing window** |
| **[macOS .app Bundle CI](./technical/platform/macos-app-bundle-ci.md)** | **CI workflow for proper macOS .app bundle packaging** |
| [macOS Gatekeeper (GitHub Releases)](./technical/platform/macos-gatekeeper-github-releases.md) | Unsigned CI artifacts, doc map (#130), release checklist |
| [macOS Markdown file association](./technical/platform/macos-markdown-file-association.md) | UTI for .md files, Finder Open With / default app |
| [macOS Intel CPU Optimization](./technical/platform/macos-intel-cpu-optimization.md) | Idle repaint optimization to reduce CPU usage on Intel Macs |
| [Intel Mac Repaint Investigation](./technical/platform/intel-mac-continuous-repaint-investigation.md) | Investigation into continuous repaint issues on Intel Macs |
| [Intel Mac CPU Analysis](./technical/platform/intel-mac-cpu-issue-analysis.md) | Analysis of CPU usage issues on Intel Mac hardware |
| **[MSI Installer Features](./technical/platform/msi-installer-features.md)** | **Windows MSI feature tree: file associations, context menu, PATH, desktop shortcut** |
| **[Linux Portal Dialogs](./technical/platform/linux-portal-dialogs.md)** | **xdg-desktop-portal requirements for Hyprland, Sway, and minimal WMs** |
| [Linux Cinnamon Dialogs](./technical/platform/linux-cinnamon-dialogs.md) | X-Cinnamon desktop detection, xapp/gtk portal instructions, cancellation fix |
| [Flatpak File Dialog Portal](./technical/platform/flatpak-file-dialog-portal.md) | Open Folder/File/Save dialogs in Flatpak via xdg-desktop-portal |

### Distribution & Packaging

| Document | Description |
|----------|-------------|
| **[Flathub Maintenance](./flathub-maintenance.md)** | **How to maintain and update Ferrite on Flathub (release checklist, moderation)** |
| [Linux Package Distribution Plan](./linux-package-distribution-plan.md) | Plan for distributing Ferrite via Flathub, Snap, AUR, and native packages |
| [Nix Flake](../flake.nix) | Official Nix flake for reproducible builds, dev shells, NixOS/Home Manager |

### Mermaid Diagrams

| Document | Description |
|----------|-------------|
| [Mermaid Diagrams](./technical/mermaid/mermaid-diagrams.md) | MermaidJS code block detection, diagram type indicators, styled rendering |
| [Mermaid Text Measurement](./technical/mermaid/mermaid-text-measurement.md) | TextMeasurer trait, dynamic node sizing, egui font metrics integration |
| [Mermaid Modular Structure](./technical/mermaid/mermaid-modular-structure.md) | Modular directory layout for diagram types, TextMeasurer trait, shared utilities |
| [Mermaid Edge Parsing](./technical/mermaid/mermaid-edge-parsing.md) | Chained edge parsing fix, arrow pattern matching, label extraction |
| [Mermaid classDef Styling](./technical/mermaid/mermaid-classdef-styling.md) | Node styling with classDef/class directives, hex color parsing |
| [Mermaid YAML Frontmatter](./technical/mermaid/mermaid-frontmatter.md) | YAML frontmatter support for diagram titles, config parsing |
| [Mermaid Caching](./technical/mermaid/mermaid-caching.md) | AST and layout caching for flowcharts, blake3 hashing, LRU eviction |
| [Flowchart Layout Algorithm](./technical/mermaid/flowchart-layout-algorithm.md) | Sugiyama-style layered graph layout: cycle detection, crossing reduction, alone-on-layer branch-parent snap, `resolve_layer_overlaps` sibling-spacing safety net (v0.3.0) |
| [Flowchart Subgraphs](./technical/mermaid/flowchart-subgraphs.md) | Flowchart subgraph support, nested parsing, bounding box computation |
| [Flowchart Direction](./technical/mermaid/flowchart-direction.md) | Flow direction layout (LR/RL/TD/BT), axis transformation, edge anchoring |
| [Flowchart Branch Ordering](./technical/mermaid/flowchart-branch-ordering.md) | Decision node branch positioning, edge declaration order, barycenter algorithm |
| [Flowchart Subgraph Title](./technical/mermaid/flowchart-subgraph-title.md) | Subgraph title width expansion, preventing title truncation |
| [Flowchart Asymmetric Shape](./technical/mermaid/flowchart-asymmetric-shape.md) | Asymmetric (flag) shape rendering, text centering |
| [Flowchart shapes & style](./technical/mermaid/flowchart-shapes-and-style.md) | Trapezoid, double circle, `style nodeId`, `color:` in classDef, merge precedence |
| [Flowchart Viewport Clipping](./technical/mermaid/flowchart-viewport-clipping.md) | Viewport clipping fix, negative coordinate shifting |
| [Flowchart linkStyle](./technical/mermaid/flowchart-linkstyle.md) | Edge styling via linkStyle directive, stroke color/width customization |
| [Flowchart Crash Prevention](./technical/mermaid/flowchart-crash-prevention.md) | Infinite loop safety, panic handling, graceful degradation |
| [Subgraph Layer Clustering](./technical/mermaid/subgraph-layer-clustering.md) | Subgraph-aware layer assignment, consecutive layer clustering |
| [Subgraph Internal Layout](./technical/mermaid/subgraph-internal-layout.md) | Subgraph internal positioning, SubgraphLayoutEngine, bounding box computation |
| [Subgraph Edge Routing](./technical/mermaid/subgraph-edge-routing.md) | Edge routing through subgraph boundaries, orthogonal waypoints |
| [Flowchart edge obstacle routing](./technical/mermaid/flowchart-edge-obstacle-routing.md) | v0.3.0 FC-83a: forward-edge obstacle avoidance, painter sized from real node bounds, fixed-margin back-edge side channels, parallel back-edge lanes (`E→B` / `F→B`), inner back-edge top-corner up-first direct path |
| [Nested Subgraph Layout](./technical/mermaid/nested-subgraph-layout.md) | Nested subgraph margins, depth calculation, direction overrides |
| [Sequence Control Blocks](./technical/mermaid/sequence-control-blocks.md) | Sequence diagram loop/alt/opt/par blocks, nested parsing, block rendering |
| [Sequence Activations & Notes](./technical/mermaid/sequence-activations-notes.md) | Activation boxes, notes, +/- shorthand, state tracking |
| [State Composite Nested](./technical/mermaid/state-composite-nested.md) | State diagram composite and nested state support |
| [State pseudostates (fork/join/history)](./technical/mermaid/state-pseudostates-fork-join-history.md) | `<<fork>>` / `<<join>>` bars and `[H]` / `[H*]` history glyphs in native state diagrams |
| **[Flowchart Modular Refactor](./technical/mermaid/flowchart-modular-refactor.md)** | **Flowchart.rs split into 12 focused modules (types, parser, layout/, render/, utils)** |
| [Flowchart Refactor Plan](./technical/mermaid/flowchart-refactor-plan.md) | Original analysis and refactoring plan for flowchart.rs modularization |
| **[Mermaid Inline Validation](./technical/mermaid/mermaid-inline-validation.md)** | **Parse-time validation: warning header (line + hint), last-good fallback, raw-editor squiggles for broken `mermaid` blocks** |
| **[Mermaid Parity Matrix](./technical/mermaid/mermaid-parity-matrix.md)** | **Feature/status map vs Mermaid.js, GitHub issue cross-ref, repro catalog, pre-0.3.0 rendering backlog** |

### LSP Integration *(deferred to v0.2.9 — feature-gated behind `lsp` Cargo feature)*

| Document | Description |
|----------|-------------|
| [LSP Integration Plan](./lsp-integration-plan.md) | Planning: Language Server Protocol client (diagnostics, hover, go-to-def) |
| [LSP Module Infrastructure](./technical/lsp/lsp-module-infrastructure.md) | `src/lsp/` — LspManager, stdio transport, extension-to-server detection |
| [LSP Server Lifecycle](./technical/lsp/lsp-server-lifecycle.md) | Auto-detect/spawn servers, crash restart with backoff, clean shutdown |
| [LSP Windows — No Console](./technical/lsp/lsp-windows-no-console.md) | CREATE_NO_WINDOW on LSP Command spawn to prevent cmd.exe flash |
| [LSP Status & Overrides](./technical/lsp/lsp-status-and-overrides.md) | Status bar per-server state, lsp_server_overrides, Editor settings UI |
| [LSP On-Demand Startup](./technical/lsp/lsp-on-demand-startup.md) | Lazy server spawn on tab activation, idle shutdown, didClose on tab close |
| [LSP Inline Diagnostics](./technical/lsp/lsp-inline-diagnostics.md) | Inline squiggles (error/warning), hover tooltips, didOpen/didChange sync |

### Planning & Roadmap

| Document | Description |
|----------|-------------|
| **[Custom Editor Widget Plan](./technical/planning/custom-editor-widget-plan.md)** | **v0.3.0 planning: Replace egui TextEdit with custom FerriteEditor widget** |
| **[Memory Optimization Plan](./technical/planning/memory-optimization.md)** | **v0.2.6 planning: Reduce idle RAM from ~250MB to ~100-150MB** |
| [Custom Memory Allocator](./technical/planning/custom-memory-allocator.md) | Platform-specific allocators (mimalloc/jemalloc) for reduced fragmentation |
| [egui Memory Cleanup](./technical/planning/egui-memory-cleanup.md) | Clean up rendered editor temp data in egui memory on tab close |
| [Viewer State Cleanup](./technical/planning/viewer-state-cleanup.md) | Memory leak fix: cleanup viewer state HashMaps on tab close |
| [Dead Code Cleanup](./technical/planning/dead-code-cleanup.md) | Task 39 cleanup summary, removed code, module changes |
| **[app.rs Refactoring Plan](./technical/planning/app-rs-refactoring-plan.md)** | **Split 7,634-line app.rs into ~15 focused modules under src/app/** |
| **[Mermaid Crate Plan](./mermaid-crate-plan.md)** | **Extract Mermaid renderer as standalone pure-Rust crate** |
| **[Math Support Plan](./math-support-plan.md)** | **v0.4.0 planning: Native LaTeX/TeX math rendering (pure Rust)** |
| **[PDF Export Pipeline](./technical/planning/pdf-export-pipeline.md)** | **v0.3.x decision doc: native-Rust PDF export via `krilla` + `krilla-svg`, browser fallback retained** |

### Performance

| Document | Description |
|----------|-------------|
| **[Per-Frame Cache Elimination](./technical/performance/per-frame-cache-elimination.md)** | **content_version-based caching to eliminate 7 O(N) per-frame operations for large files** |
| [Background File Loading](./technical/performance/background-file-loading.md) | Background thread loading for 5MB+ files with progress bar, cancellation support |

### Core (Remaining)

| Document | Description |
|----------|-------------|
| [App State](./technical/app-state.md) | AppState, Tab, UiState structs, undo/redo, event handling |
| [View Mode Persistence](./technical/view-mode-persistence.md) | Per-tab view mode storage, session restoration, backward compatibility |
| [Document Statistics](./technical/document-statistics.md) | Statistics panel tab with word count, reading time, heading/link/image counts |
| [Text Statistics](./technical/text-statistics.md) | Word, character, line counting for status bar |
| [Sync Scrolling](./technical/sync-scrolling.md) | Split-view live sync (minimap **Sync** / **2-way**), per-pane scroll delivery, Ctrl+E mode-toggle preservation, content anchors |
| [Configurable Line Width](./technical/configurable-line-width.md) | MaxLineWidth setting (Off/80/100/120/Custom), text centering in all views |
| [Branding](./branding.md) | Icon design, asset generation, platform integration guidelines |

---

## Guides

| Guide | Description |
|-------|-------------|
| [macOS install & Gatekeeper](./install/macos.md) | Unsigned CI bundles, macOS 15.x / Sequoia, `xattr` quarantine removal, Open Anyway workarounds |
| [GitHub Release checklist](./github-release-checklist.md) | Pre-tag, GitHub Release, Flathub, and Nix steps; macOS Gatekeeper blurb (#130) |
| [Adding Languages](./adding-languages.md) | How to add new translations, translation portal setup, contributor workflow |
| [Translation Status Assessment](./translation-status-assessment.md) | List of user-facing strings not yet using i18n, for Weblate extraction |
| [v0.2.6 Test Suite](./v0.2.6-manual-test-suite.md) | Manual testing checklist for FerriteEditor release |
| [v0.2.8 Test Suite](./v0.2.8-manual-test-suite.md) | Manual testing checklist for v0.2.8 release |
| [v0.3.0 Test Suite](./v0.3.0-manual-test-suite.md) | Pre-merge manual checklist for v0.3.0 (Tasks 90–106, rendered session, session recovery) |
