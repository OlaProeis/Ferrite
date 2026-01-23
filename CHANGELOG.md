# Changelog

All notable changes to Ferrite will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased - 0.2.5.3]

### Added

#### Code Signing
- **SignPath integration** - Windows artifacts (exe, MSI, portable zip) are now code signed via [SignPath.io](https://signpath.io/) free tier for open source. This helps prevent Windows Defender false positives and establishes trust with users.
- **CI/CD signing workflow** - Signing is integrated into GitHub Actions release workflow and runs automatically on tagged releases.

#### UI Improvements
- **View Mode Segmented Control** - Replaced single-letter toggle button (R/S/V) with a polished pill-shaped segmented control showing all three view modes at once. Users can now click directly on the mode they want (Raw, Split, Rendered) with clear visual feedback for the active mode. The control adapts to file type: 3 modes for markdown/CSV, 2 modes for JSON/YAML/TOML. Visible in both normal and Zen mode.
- **App logo in title bar** - Added Ferrite logo with transparent background to the title bar for better brand visibility.

#### Syntax Highlighting
- **Extended syntax support** - Added 100+ additional language syntaxes via `two-face` crate, including PowerShell (.ps1/.psm1/.psd1), TypeScript/TSX, Zig, Svelte, Vue, Terraform, Nix, and many more. Previously unsupported languages now get proper syntax highlighting instead of plain text.
- **Syntax theme selector** - New dropdown in Appearance settings to choose from 25+ syntax highlighting color themes including Dracula, Nord, Catppuccin (Mocha/Latte/Frappe/Macchiato), Gruvbox (light/dark), Solarized (light/dark), One Half, GitHub, VS Code Dark+, and more. Set to "Auto" to match the app theme.

### Fixed

#### Linux Desktop Integration
- **Alt-tab/taskbar visibility on Wayland** - Fixed Ferrite window not appearing in alt-tab switcher or taskbar on Linux desktop environments (KDE Plasma, GNOME) running Wayland. Added `app_id` to ViewportBuilder for proper window identification.

#### Icon Rendering
- **Find/Replace replace icon** - Fixed the replace icon (↳) showing as a square box in the Find and Replace panel. Changed to a universally-supported arrow character (→).
- **Tree viewer context menu icon** - Fixed the context menu button (⋯) in JSON/YAML/TOML tree viewer showing as a square. Changed to simple dots (...) for reliable rendering.
- **Font atlas pre-warming** - Added additional symbols (⇄⇅↳↵…⋯) to the font atlas pre-warm list to ensure they render correctly from startup.

#### UI Positioning
- **Recent files menu position** - Fixed the recent files/folders popup menu appearing below and covering the filename button in the status bar. Menu now appears above the button using proper anchor positioning.

#### Performance
- **Linux folder opening freeze** - Fixed critical 10+ second UI freeze when opening workspace folders on Linux (especially Fedora/KDE Plasma). Root causes:
  - **notify crate misconfiguration** - Was configured with `default-features = false, features = ["macos_kqueue"]` which disabled the inotify backend on Linux, forcing fallback to slow polling-based file watching that had to walk and stat entire directory trees.
  - **Synchronous recursive directory scanning** - `Workspace::new()` scanned the entire directory tree recursively on the main UI thread before showing anything. Now uses lazy loading: only the root directory is scanned initially, subdirectories are scanned on-demand when expanded.

#### Bug Fixes
- **Line breaks in list items** ([#41](https://github.com/OlaProeis/Ferrite/issues/41)) - Fixed hard line breaks (`\` at end of line) within list items showing as a square box instead of rendering as a proper line break.
- **Git deleted file icon rendering** - Fixed git "deleted" status icon showing as a square box in the file tree. The previous icon character (✕) was not supported by the embedded Inter font. Changed to standard ASCII minus character (-) for reliable cross-platform rendering.
- **Blockquote/table overflow** - Added horizontal scrolling for tables and blockquotes when content exceeds container width. Previously, wide content would expand the layout and break max line width for all subsequent content. Now wide tables scroll horizontally while the rest of the document respects the configured line width setting. Code blocks and mermaid diagrams already have internal horizontal scroll handling.
- **PowerShell file rendering collapse** - Fixed critical bug where PowerShell and other files without syntax definitions would collapse all content to a single line after initial render. Root cause: the fallback path for unsupported languages used `code.lines()` which strips newline characters. Fix uses `LinesWithEndings` to preserve newlines in plain text rendering.
- **View mode segment not clickable** - Fixed issue on Linux where clicking the view mode segment (R/S/V buttons) in the title bar would initiate window drag instead of switching modes. Increased the drag exclusion zone width to fully cover all title bar controls.
- **Inter font missing box-drawing characters** - Fixed box-drawing characters (─│┌┐└┘ etc.) rendering as squares when using Inter font. The embedded Inter font doesn't include Unicode box-drawing block (U+2500-U+257F). Added JetBrains Mono as fallback font for Inter to provide these characters.

## [0.2.5.2] - 2026-01-20

### Added

#### New Features
- **Delete Line shortcut** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Cmd/Ctrl+D deletes current line (configurable in settings) - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **Move Line Up/Down** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Alt+Up/Down swaps current line with adjacent line - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **macOS file type associations** ([#30](https://github.com/OlaProeis/Ferrite/pull/30)) - Ferrite appears in Finder's "Open With" menu for .md, .json, .yaml, .toml, .txt files - thanks [@abcd-ca](https://github.com/abcd-ca)!

#### Installation & Distribution
- **Windows portable build** - True portable mode (`ferrite-portable-windows-x64.zip`) with `portable` folder for self-contained operation. All settings stored next to executable - perfect for USB drives.
- **Windows MSI installer** - Proper Windows installer (`ferrite-windows-x64.msi`) with Start Menu shortcut, application icon, and clean uninstall support via Windows Settings. Built with WiX Toolset.
- **Linux RPM package** - Native package (`ferrite-editor.x86_64.rpm`) for Fedora, RHEL, CentOS, Rocky Linux, and other RPM-based distributions. Includes desktop entry and icon integration.

#### Internationalization
- **I18n audit & cleanup** - Comprehensive audit of hardcoded strings, replacement with translation keys
- **Orphaned key removal** - Removed ~200 unused translation keys from locale files
- **Locale file sync** - All locale files now have consistent structure matching en.yaml
- **New language support** - Added Estonian and Norwegian Bokmål via Weblate community translations

### Fixed

#### Bug Fixes
- **Ctrl+X cutting entire document** - Fixed egui bug where Ctrl+X with no text selection would cut the entire document. Now correctly does nothing when nothing is selected.
- **Linux window drag stuck mouse** - Fixed critical bug where dragging the custom title bar on Linux caused the mouse to get "stuck" in drag mode. Root cause: egui's widget-level drag tracking desynchronized with the window manager after `ViewportCommand::StartDrag`. Fix bypasses egui's drag state machine entirely, using raw input detection (`primary_pressed()`) for immediate, reliable window drag initiation.
- **Split mode cursor position** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Line operations now work correctly in Split view; rendered pane no longer overwrites cursor position - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **macOS modifier tooltips** ([#28](https://github.com/OlaProeis/Ferrite/pull/28), [#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Tooltips now show "Cmd+E" on macOS instead of hardcoded "Ctrl+E" - thanks [@abcd-ca](https://github.com/abcd-ca)!
- **Semantic minimap highlight accuracy** - Use byte offsets matching search behavior for correct highlight positioning

> **Note:** Opening files via "Open With" or dragging onto app icon not yet supported on macOS due to [winit#1751](https://github.com/rust-windowing/winit/issues/1751). Workaround: use `open -a Ferrite file.md` or File > Open.

## [0.2.5.1] - 2026-01-17

### Added

#### Idle Mode CPU Optimization
- **Tiered idle repaint system** - Implements intelligent repaint scheduling based on user interaction time:
  - Active (animations/dialogs): Continuous repaint at 60 FPS
  - Light idle (0-2 seconds): 100ms interval (~10 FPS)
  - Deep idle (2+ seconds): 500ms interval (~2 FPS)
- **User interaction tracking** - Detects keyboard, mouse, and scroll activity to determine idle state
- **Scroll animation awareness** - Continuous repaint during sync scroll animations

#### Multi-Encoding File Support
- **Automatic encoding detection** - Files are now automatically detected for encoding on open using `encoding_rs` and `chardetng` crates. No more garbled text when opening legacy files.
- **Common encoding support** - Full support for Latin-1, Windows-1252, ISO-8859-x, Shift-JIS, EUC-KR, GBK, and other common encodings beyond UTF-8.
- **Status bar indicator** - Current file encoding displayed in status bar with click-to-change dropdown menu for manual encoding selection.
- **Preserve encoding on save** - Files are saved back in their original encoding by default, not forced to UTF-8.

#### Memory Optimization
- **CJK font lazy loading** - CJK fonts (Korean, Japanese, Chinese) are now loaded on-demand when CJK characters are detected in documents, rather than at startup. This reduces idle memory usage from ~250MB to ~72MB for non-CJK users.
- **Granular per-language CJK loading** - Each CJK language font is loaded independently based on detected script (Hangul → Korean font, Hiragana/Katakana → Japanese font, Han characters → user's preferred Chinese variant). Opening a Korean document only loads Korean fonts (~30MB), not all CJK fonts (~180MB).
- **Custom memory allocators** - Platform-specific high-performance allocators: `mimalloc` on Windows, `jemalloc` on Linux/macOS. Reduces heap fragmentation and memory usage, especially for long-running sessions.
- **Viewer state cleanup** - Fixed memory leak by cleaning up `tree_viewer_states`, `csv_viewer_states`, and `sync_scroll_states` HashMap entries when tabs are closed.
- **egui temp data cleanup** - Stale `SyntaxHighlightCache` and per-tab temporary data cleared from egui memory on tab close.

#### Cursor Positioning Improvements
- **Galley-based click mapping** - Use egui's Galley for accurate click-to-character index conversion in rendered/split view click-to-edit.
- **Formatting marker mapping** - Map displayed text positions to raw markdown positions accounting for `**`, `*`, `` ` ``, `~~`, and `[links](url)` syntax.
- **Text wrapping support** - Handle wrapped lines correctly by using actual text rect width for measurement.
- **Bold font measurement** - Use bold font for measurement when content starts with bold markers for better accuracy.

#### Scroll Navigation Accuracy
- **Unified scroll calculation** - Single function for all scroll-to-line operations (find, search-in-files, outline, minimap) ensuring consistent positioning.
- **Fixed off-by-one errors** - Consistent 0-indexed vs 1-indexed line number handling across all navigation functions.
- **Fresh line height** - Ensure actual rendered line height is used instead of stale/default values when calculating scroll positions.
- **Large file accuracy** - Scroll navigation now works correctly in files with 3000+ lines; previously target lines could be hundreds of pixels off or completely out of view.
- **Semantic minimap highlight fix** - Fixed highlight offset when clicking items in semantic minimap/outline panel. The highlight now correctly marks the target line by using byte offsets (matching search behavior) instead of character offsets.

#### Settings & UX
- **Session restore option** - New setting to disable tab restoration on startup. When disabled, app starts with a single empty tab instead of restoring previous session.

### Fixed

#### CPU Usage Optimization
- **Fixed 10% idle CPU usage** - Application now uses <1% CPU when truly idle (previously ~10% even with no user interaction)
- **Window title optimization** - Only send viewport title command when title actually changes, avoiding per-frame overhead
- **Disabled repaint on widget change** - Set `options.repaint_on_widget_change = false` to prevent unnecessary repaints
- **Animation time optimization** - Removed conflicting animation_time override in ThemeManager

#### Intel Mac CPU Usage ([#24](https://github.com/OlaProeis/Ferrite/issues/24))
- **Removed verbose debug logging** - Eliminated `[LIST_ITEM_DEBUG]` statements in rendered mode that generated ~50,000 log lines per 22 seconds.
- **Fixed continuous repaint in Rendered mode** - Root cause: `Sense::hover()` on scroll area content caused ~60fps repaints bypassing idle throttling. Changed to `Sense::focusable_noninteractive()` for proper ~10fps idle throttling.

#### Bug Fixes
- **New file dirty flag** - New untitled files no longer prompt to save when closed if they haven't been modified.
- **CJK first-line indentation** - Paragraph indentation now correctly applies only to the first line, not the entire paragraph.
- **Workspace close button alignment** - X button to close workspace panel shifted left to prevent overlap with resize handles.
- **Linux close button** - Fixed issue where window close button couldn't be clicked due to hit-testing/overlay interference.

### Technical
- New `last_interaction_time` field tracks user activity for idle detection
- New `get_idle_repaint_interval()` returns appropriate interval based on idle duration
- New `had_user_input()` detects keyboard, mouse, and scroll events
- Enhanced `needs_continuous_repaint()` to check for scroll animations
- Explicit vsync and run_and_return settings in NativeOptions
- Added FPS diagnostic logging (debug builds only) for idle CPU optimization verification
- New `CjkScriptDetection` struct and `detect_cjk_scripts()` function for granular script identification
- Per-language `AtomicBool` flags track which CJK fonts have been loaded
- `CjkLoadSpec` determines fonts to load based on detected scripts and user preferences
- Platform-conditional allocator setup in `main.rs` with feature flags
- Documentation: [Idle Mode Optimization](docs/technical/platform/idle-mode-optimization.md)

### Changed

#### Antivirus False Positive Mitigation
- **Adjusted release build profile** - Changed `lto` from "fat" to "thin", `opt-level` from "z" to "3", disabled symbol stripping to reduce Windows Defender false positives.
- **Documentation** - Added "Antivirus False Positives" section to README explaining the issue and workarounds.

## [0.2.5] - 2026-01-16

### Added

#### Mermaid Improvements
- **Modular refactor** - Split 7000+ line `mermaid.rs` into `src/markdown/mermaid/` directory with separate files per diagram type
- **Edge parsing fixes** - Fix chained edge parsing (`A --> B --> C`), arrow pattern matching, label extraction
- **Flowchart direction fix** - Respect LR/TB/RL/BT direction keywords in layout algorithm
- **Node detection fixes** - Fix missing nodes and improve branching layout in complex flowcharts
- **YAML frontmatter support** - Parse `---` metadata blocks with `title:`, `config:` etc. (MermaidJS v8.13+ syntax)
- **Parallel edge operator** - Support `A --> B & C & D` syntax for multiple edges from one source
- **Rendering performance** - AST and layout caching with blake3 hashing for complex diagrams
- **classDef/class styling** - Node styling via `classDef` and `class` directives
- **linkStyle edge styling** - Edge customization via `linkStyle` directive
- **Subgraph improvements** - Layer clustering, internal layout, edge routing, title expansion, nested margins
- **Asymmetric shape rendering** - Flag/asymmetric node shape with proper text centering
- **Viewport clipping fix** - Prevent diagram clipping with negative coordinate shifting
- **Crash prevention** - Infinite loop safety, panic handling for malformed input

#### Split View Enhancements
- **Dual editable panes** - Split view rendered pane is now fully editable, matching full Rendered mode behavior
- Both panes edit the same content with changes syncing instantly
- Full undo/redo support for edits in either pane

#### Git Integration
- **Git status auto-refresh** - Automatic refresh of file tree git badges on file save, window focus, periodic interval (10 seconds), and file system events
- **Debounced refresh** - 500ms debounce prevents excessive git2 calls during rapid operations

#### CSV Support ([#19](https://github.com/OlaProeis/Ferrite/issues/19))
- **CSV/TSV viewer** - Native table view for CSV and TSV files with fixed-width column alignment
- **Rainbow column coloring** - Alternating column colors for improved readability
- **Delimiter detection** - Auto-detect comma, tab, semicolon, pipe separators
- **Header row detection** - Intelligent detection and highlighting of header rows

#### Internationalization ([#18](https://github.com/OlaProeis/Ferrite/issues/18))
- **i18n infrastructure** - YAML translation files in `locales/` directory with rust-i18n integration
- **Weblate integration** - Community translations via [hosted.weblate.org/projects/ferrite](https://hosted.weblate.org/projects/ferrite/)
- **String extraction** - UI strings moved to translation keys

#### CJK Writing Conventions ([#20](https://github.com/OlaProeis/Ferrite/issues/20))
- **Paragraph indentation** - First-line indentation setting for Chinese (2 chars), Japanese (1 char), or custom
- **Rendered view support** - Apply `text-indent` styling to paragraphs in preview mode

#### New Features
- **Keyboard shortcut customization** - Users can rebind shortcuts via settings panel; stored in config.json
- **Custom font selection** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Select preferred font for editor and UI; important for CJK regional glyph preferences
- **Main menu UI redesign** - Modernized main menu with improved layout and visual design
- **Windows fullscreen toggle** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Dedicated fullscreen button (F10) separate from Zen mode (F11)

#### Semantic Minimap
- **Header labels** - Display actual H1/H2/H3 text in minimap instead of unreadable scaled pixels
- **Content type indicators** - Visual markers for code blocks, mermaid diagrams, tables, images
- **Density visualization** - Text density shown as subtle horizontal bars between headers
- **Mode toggle** - Settings option to choose "Visual" (pixel) or "Semantic" (labels) mode

#### Editor Productivity
- **Drag & drop images** - Drop images into editor → auto-save to `./assets/` → insert markdown link
- **Table of Contents generation** - Insert/update `<!-- TOC -->` block with auto-generated heading links (Ctrl+Shift+U)
- **Document statistics panel** - Tabbed info panel with word count, reading time, heading/link/image counts
- **Snippets/abbreviations** - User-defined text expansions (`;date` → current date, `;time` → current time)
- **Recent folders** - Recent files menu now includes workspace folders

#### Branding
- **New Ferrite logo** - Orange geometric crystal icon
- **Platform icons** - Windows `.ico`, macOS `.iconset`, Linux PNGs (16-512px)
- **Window icon** - Embedded 256px icon replaces default eframe "E" logo

### Fixed

#### Bug Fixes
- **Search highlight drift** - Fixed find/search highlight boxes drifting progressively further from matched text; caused by byte vs character position mismatch in UTF-8 text
- **Config.json persistence** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Fixed window state dirty flag; settings now persist correctly across restarts
- **Session restore reliability** - Workspace folders and recent files now persist correctly across restarts with atomic file writes
- **Recent files persistence** - Recent files list now saves immediately on file open, pruning stale paths
- **Line width in rendered/split view** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Line width setting now respects pane boundaries with proper centering behavior
- **Quick switcher mouse support** - Fixed mouse hover/click not working (item flickering but not selecting)
- **Table editing cursor loss** - Table cells no longer lose focus after each keystroke in Rendered/Split modes; edits are buffered and committed when focus leaves (deferred update model)
- **Zen mode rendered centering** - Content now centers properly in rendered/split view when Zen mode (F11) is active
- **Windows top edge resize** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Window can now be resized from all edges including top
- **macOS Intel CPU optimization** ([#24](https://github.com/OlaProeis/Ferrite/issues/24)) - Idle repaint scheduling reduces CPU usage on Intel Macs

### Technical
- Split `mermaid.rs` into modular structure: `src/markdown/mermaid/` with `mod.rs`, `flowchart.rs`, `sequence.rs`, etc.
- Added `GitAutoRefresh` struct for managing refresh timing and focus tracking
- Added `had_focus_last_frame` and `content_modified` fields to `TableEditState` for focus tracking
- Added blake3 hashing for Mermaid diagram caching
- Added 11 unit tests for git auto-refresh logic
- Added comprehensive technical documentation in `docs/technical/`

### Deferred
- **Mermaid diagram toolbar** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Toolbar button to insert mermaid code blocks (deferred to v0.3.0)
- **Mermaid syntax hints** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Help panel with diagram type syntax examples (deferred to v0.3.0)
- **Simplified Chinese translation** - Waiting for community contributor (deferred)
- **Mermaid code cleanup** - Flowchart.rs modular refactor and documentation (deferred to v0.2.6)
- **Executable code blocks** - Run code snippets in preview (deferred to v0.2.6)
- **Content blocks/callouts** - GitHub-style `[!NOTE]` admonitions (deferred to v0.2.6)

## [0.2.3] - 2025-01-12

### Added

#### Editor Productivity
- **Go to Line (Ctrl+G)** - Quick navigation to specific line number with modal dialog and viewport centering
- **Duplicate Line (Ctrl+Shift+D)** - Duplicate current line or selection with proper char-to-byte index handling
- **Move Line Up/Down (Alt+↑/↓)** - Rearrange lines without cut/paste, cursor follows moved line
- **Auto-close Brackets & Quotes** - Type `(`, `[`, `{`, `"`, or `'` to get matching pair with cursor in middle; selection wrapping and skip-over behavior
- **Smart Paste for Links** - Select text then paste URL to create `[text](url)` markdown link; image URLs create `![](url)` syntax

#### UX Improvements
- **Configurable line width** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Limit text width for improved readability with presets (Off/80/100/120) or custom value; text centered in viewport

#### Platform & Distribution
- **macOS Intel cross-compilation** - CI now cross-compiles for Intel Macs from ARM64 runner

### Fixed

#### Bug Fixes
- **Task list rendering** - Task list items with inline formatting now render correctly; fixed checkbox alignment and replaced interactive checkboxes with non-interactive ASCII-style `[ ]`/`[x]` markers (interactive editing planned for v0.3.0)
- **macOS Intel support** ([#16](https://github.com/OlaProeis/Ferrite/issues/16)) - Fixed artifact naming for Intel Mac builds; separate x86_64 build via `macos-13` runner
- **Linux close button cursor flicker** - Fixed cursor rapidly switching between pointer/resize near window close button by adding title bar exclusion zone (35px) for north-edge resize detection and cursor caching

### Technical
- Added 7 new technical documentation files in `docs/technical/`
- Extended keyboard shortcut system with pre-render key consumption for move line operations

## [0.2.2] - 2025-01-11

### Added

#### CLI Features
- **Command-line file opening** ([#9](https://github.com/OlaProeis/Ferrite/issues/9)) - Open files directly: `ferrite file.md`, `ferrite file1.md file2.md`, or `ferrite ./folder/`
- **Version and help flags** ([#10](https://github.com/OlaProeis/Ferrite/issues/10)) - Support for `-V/--version` and `-h/--help` CLI arguments
- **Configurable log level** ([#11](https://github.com/OlaProeis/Ferrite/issues/11)) - New `log_level` setting in config.json with CLI override (`--log-level debug|info|warn|error|off`)

#### UX Improvements
- **Default view mode setting** ([#3](https://github.com/OlaProeis/Ferrite/issues/3)) - Choose default view mode (Raw/Rendered/Split) for new tabs in Settings > Appearance

### Fixed

#### Bug Fixes
- **CJK character rendering** ([#7](https://github.com/OlaProeis/Ferrite/issues/7)) - Multi-region CJK support (Korean, Chinese, Japanese) via system font fallback (PR [#8](https://github.com/OlaProeis/Ferrite/pull/8) by [@SteelCrab](https://github.com/SteelCrab) 🙏)
- **Undo/redo behavior** ([#5](https://github.com/OlaProeis/Ferrite/issues/5)) - Fixed scroll position reset, focus loss, double-press requirement, and cursor restoration
- **UTF-8 tree viewer crash** - Fixed string slicing panic when displaying JSON/YAML with multi-byte characters (Norwegian øæå, Chinese, emoji)
- **Misleading code folding UI** ([#12](https://github.com/OlaProeis/Ferrite/issues/12)) - Fold indicators now hidden by default (setting available for power users); removed confusing "Raw View" button from tree viewer toolbar

#### Performance
- **Large file editing** - Deferred syntax highlighting keeps typing responsive in 5000+ line files
- **Scroll performance** - Galley caching for instant syntax colors when scrolling via minimap

### Changed
- **Ubuntu 22.04 compatibility** ([#6](https://github.com/OlaProeis/Ferrite/issues/6)) - Release builds now target Ubuntu 22.04 for glibc 2.35 compatibility

### Documentation
- Added CLI reference documentation (`docs/cli.md`)
- Added technical docs for log level config, default view mode, and code folding UI changes

## [0.2.1] - 2025-01-10

### Added

#### Mermaid Diagram Enhancements
- **Sequence Diagram Control Blocks** - Full support for `loop`, `alt`, `opt`, `par`, `critical`, `break` blocks with proper nesting and colored labels
- **Sequence Activation Boxes** - `activate`/`deactivate` commands and `+`/`-` shorthand on messages for lifeline activation tracking
- **Sequence Notes** - `Note left/right/over` syntax with dog-ear corner rendering
- **Flowchart Subgraphs** - Nested `subgraph`/`end` blocks with semi-transparent backgrounds and direction overrides
- **Composite/Nested States** - State diagrams now support `state Parent { ... }` syntax with recursive nesting
- **Advanced State Transitions** - Color-coded transitions, smart anchor points, and cross-nesting-level edge routing

#### Layout Improvements
- **Flowchart Branching** - Sugiyama-style layered graph layout with proper side-by-side branch placement
- **Cycle Detection** - Back-edges rendered with smooth bezier curves instead of crossing lines
- **Smart Edge Routing** - Decision node edges exit from different points to prevent crossing
- **Edge Declaration Order** - Branch ordering now matches Mermaid's convention (later-declared edges go left)

### Fixed
- **Text Measurement** - Replaced character-count estimation with egui font metrics for accurate node sizing
- **Node Overflow** - Nodes dynamically resize to fit their labels without clipping
- **Edge Labels** - Long labels truncate with ellipsis instead of overflowing
- **User Journey Icons** - Fixed unsupported emoji rendering with text fallbacks

### Technical
- Extended `mermaid.rs` from ~4000 to ~6000+ lines
- Added technical documentation for all new features in `docs/technical/`

## [0.2.0] - 2025-01-09

### Added

#### Major Features
- **Split View** - Side-by-side raw editor and rendered preview with resizable divider and per-tab split ratio persistence
- **MermaidJS Native Rendering** - 11 diagram types rendered natively in Rust/egui (flowchart, sequence, pie, state, mindmap, class, ER, git graph, gantt, timeline, user journey)
- **Editor Minimap** - VS Code-style scaled preview with click-to-navigate, viewport indicator, and search highlights visible in minimap
- **Code Folding** - Fold detection for headings, code blocks, and lists with gutter indicators (▶/▼) and indentation-based folding for JSON/YAML
- **Live Pipeline Panel** - Pipe JSON/YAML content through shell commands with real-time output preview and command history
- **Zen Mode** - Distraction-free writing with centered text column and configurable column width
- **Git Integration** - Visual status indicators in file tree showing modified, added, untracked, and ignored files (using git2 library)
- **Auto-Save** - Configurable delay (default 15s), per-tab toggle, temp-file based for safety
- **Session Persistence** - Restore open tabs on restart with cursor position, scroll offset, view mode, and per-tab split ratio
- **Bracket Matching** - Highlight matching brackets `()[]{}<>` and markdown emphasis pairs `**` and `__` with theme-aware colors

### Fixed
- **Rendered Mode List Editing** - Fixed item index mapping issues, proper structural key hashing, and edit state consistency (Tasks 64-69)
- **Light Mode Contrast** - Improved text and border visibility with WCAG AA compliant contrast ratios, added separator between tabs and editor
- **Scroll Synchronization** - Bidirectional sync between Raw and Rendered modes with hybrid line-based/percentage approach and mode switch scroll preservation
- **Search-in-Files Navigation** - Click result now scrolls to match with transient highlight that auto-clears on scroll or edit
- **Search Panel Viewport** - Fixed top and bottom clipping issues with proper bounds calculation

### Changed
- **Tab Context Menu** - Reorganized icons with logical grouping for better visual clarity

### Technical
- Added ~4000 lines of Mermaid rendering code in `src/markdown/mermaid.rs`
- New modules: `src/vcs/` for git integration, `src/editor/minimap.rs`, `src/editor/folding.rs`, `src/editor/matching.rs`, `src/ui/pipeline.rs`, `src/config/session.rs`
- Comprehensive technical documentation for all major features in `docs/technical/`

### Deferred
- **Multi-cursor editing** (Task 72) - Deferred to v0.3.0, requires custom text editor implementation

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

- **0.2.5.3** - Windows code signing (SignPath), View Mode Segmented Control, app logo in title bar, extended syntax highlighting (100+ languages), syntax theme selector (25+ themes), list line break fix, table overflow fix, PowerShell rendering fix
- **0.2.5.2** - Delete Line shortcut, Move Line Up/Down, macOS file associations, Windows portable build, MSI installer, Linux RPM package, Linux window drag fix, I18n cleanup, new language support
- **0.2.5.1** - Multi-encoding support, memory optimization (250MB → 60-80MB), CPU optimization (10% → <1% idle), cursor positioning improvements, Intel Mac CPU fix, bug fixes
- **0.2.5** - Mermaid refactor, CSV viewer, semantic minimap, i18n, CJK indentation, custom fonts, snippets, TOC generation, drag-drop images, document statistics, main menu redesign, split view editing, bug fixes
- **0.2.3** - Editor productivity release (Go to Line, Duplicate Line, Move Line, Auto-close, Smart Paste, Line Width)
- **0.2.2** - Stability & CLI release (CJK fonts, undo/redo fixes, CLI arguments, default view mode)
- **0.2.1** - Mermaid diagram improvements (control blocks, subgraphs, nested states, improved layout)
- **0.2.0** - Major feature release (Split View, Mermaid, Minimap, Git integration, and more)
- **0.1.0** - Initial public release

[0.2.5.3]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5-hotfix.2...v0.2.5-hotfix.3
[0.2.5.2]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5-hotfix.1...v0.2.5-hotfix.2
[0.2.5.1]: https://github.com/OlaProeis/Ferrite/compare/v0.2.5...v0.2.5-hotfix.1
[0.2.5]: https://github.com/OlaProeis/Ferrite/compare/v0.2.3...v0.2.5
[0.2.3]: https://github.com/OlaProeis/Ferrite/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/OlaProeis/Ferrite/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/OlaProeis/Ferrite/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/OlaProeis/Ferrite/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/OlaProeis/Ferrite/releases/tag/v0.1.0
