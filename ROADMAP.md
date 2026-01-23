# Ferrite Roadmap

## Known Issues 🐛

### Blocked by egui TextEdit
These issues cannot be fixed without replacing egui's built-in text editor:
- [ ] **Multi-cursor incomplete** - Basic cursor rendering works, but text operations not implemented
- [ ] **Code folding incomplete** - Detection works, but text hiding not possible
- [ ] **Scroll sync imperfect** - Limited access to egui's internal scroll state
- [ ] **IME candidate box positioning** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Chinese/Japanese IME candidate window appears offset from cursor position; egui's IME support is limited
- [ ] **IME undo behavior** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Undoing during IME composition may delete an extra character; related to egui's text input handling

### Cursor Positioning Limitations
- [ ] **Click-to-edit cursor drift on mixed-format lines** - When clicking formatted text in rendered/split view, cursor may land 1-5 characters off on long lines with mixed formatting (bold + italic + links). This is due to font width differences between regular and bold text that cannot be perfectly measured without access to the actual render layout. Will be properly fixed with custom editor in v0.3.0.

---

## Planned Features 🚀

### v0.2.5 (Released) - Mermaid Update & Editor Polish

> **Status:** Released (2026-01-16)

#### Mermaid Improvements
- [x] **Modular refactor** - Split 7000+ line `mermaid.rs` into `src/markdown/mermaid/` directory with separate files per diagram type
- [x] **Edge parsing fixes** - Fix chained edge parsing (`A --> B --> C`), arrow pattern matching, label extraction
- [x] **Flowchart direction fix** - Respect LR/TB/RL/BT direction keywords in layout algorithm
- [x] **Node detection fixes** - Fix missing nodes and improve branching layout in complex flowcharts
- [x] **YAML frontmatter support** - Parse `---` metadata blocks with `title:`, `config:` etc. (MermaidJS v8.13+ syntax)
- [x] **Parallel edge operator (`&`)** - Support `A --> B & C & D` syntax for multiple edges from one source
- [x] **Rendering performance** - AST and layout caching with blake3 hashing for complex diagrams
- [x] **Semicolon & ampersand syntax** - Support Mermaid semicolon line terminators and `&` parallel edge syntax
- [x] **classDef/class styling** - Node styling via `classDef` and `class` directives
- [x] **linkStyle edge styling** - Edge customization via `linkStyle` directive
- [x] **Subgraph improvements** - Layer clustering, internal layout, edge routing, title expansion, nested margins
- [x] **Asymmetric shape rendering** - Flag/asymmetric node shape with proper text centering
- [x] **Viewport clipping fix** - Prevent diagram clipping with negative coordinate shifting
- [x] **Crash prevention** - Infinite loop safety, panic handling for malformed input

#### CSV Support ([#19](https://github.com/OlaProeis/Ferrite/issues/19))
- [x] **CSV/TSV viewer** - Native table view for CSV and TSV files with fixed-width column alignment
- [x] **Rainbow column coloring** - Alternating column colors for improved readability
- [x] **Delimiter detection** - Auto-detect comma, tab, semicolon, pipe separators
- [x] **Header row detection** - Intelligent detection and highlighting of header rows

#### Internationalization ([#18](https://github.com/OlaProeis/Ferrite/issues/18))
- [x] **i18n infrastructure** - YAML translation files in `locales/` directory
- [x] **String extraction** - UI strings moved to translation keys
- [x] **Weblate integration** - Community translations via [hosted.weblate.org/projects/ferrite](https://hosted.weblate.org/projects/ferrite/)

#### CJK Writing Conventions ([#20](https://github.com/OlaProeis/Ferrite/issues/20))
- [x] **Paragraph indentation setting** - New option in Settings: Off / Chinese (2 chars) / Japanese (1 char) / Custom
- [x] **Rendered view support** - Apply `text-indent` styling to paragraphs in preview mode

#### Split View Enhancements
- [x] **Dual editable panes** - Split view rendered pane is now fully editable, matching full Rendered mode behavior with undo/redo support

#### Bug Fixes & Polish
- [x] **Search highlight drift** - Fixed find/search highlight boxes drifting progressively from matched text (byte vs character position mismatch)
- [x] **Config.json persistence** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Fixed window state dirty flag, settings now persist correctly across restarts
- [x] **Zen mode rendered centering** - Center content in rendered/split view when Zen mode (F11) is active
- [x] **Git status auto-refresh** - Refresh git indicators on file save, window focus, periodically (every ~10 seconds), and on file system events
- [x] **Quick switcher mouse support** - Fixed mouse hover/click not working in quick switcher
- [x] **Table editing cursor loss** - Fix cursor losing focus after each keystroke when editing tables in rendered mode (deferred update model)
- [x] **Line width in rendered/split view** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Fixed line width setting respecting pane boundaries with proper centering behavior
- [x] **Windows top edge resize** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Window can now be resized from all edges including top
- [x] **macOS Intel CPU optimization** ([#24](https://github.com/OlaProeis/Ferrite/issues/24)) - Idle repaint scheduling to reduce CPU usage on Intel Macs

#### New Features
- [x] **Keyboard shortcut customization** - Users can rebind shortcuts via settings panel; stored in config.json
- [x] **Custom font selection** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Select preferred font for editor and UI; important for CJK regional glyph preferences
- [x] **Main menu UI redesign** - Modernized main menu with improved layout and visual design
- [x] **Windows fullscreen toggle** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Dedicated fullscreen button (F10) separate from Zen mode (F11)
- [x] **Session restore reliability** - Workspace folders and recent files now persist correctly with atomic file writes
- [x] **Recent files persistence** - Recent files list saves immediately on file open, pruning stale paths
- [x] **Recent folders** - Recent files menu now includes workspace folders
- [x] **Drag & drop images** - Drop images into editor → auto-save to `./assets/` → insert markdown link
- [x] **Table of Contents generation** - Insert/update `<!-- TOC -->` block with auto-generated heading links (Ctrl+Shift+U)
- [x] **Document statistics panel** - Tabbed info panel: Outline + Statistics (word count, reading time, heading/link/image counts)
- [x] **Snippets/abbreviations** - User-defined text expansions (`;date` → current date, `;time` → current time)

#### Semantic Minimap
- [x] **Header labels** - Display actual H1/H2/H3 text in minimap instead of unreadable scaled pixels
- [x] **Content type indicators** - Visual markers for code blocks, mermaid diagrams, tables, images
- [x] **Density visualization** - Show text density as subtle horizontal bars between headers
- [x] **Mode toggle** - Settings option to choose "Visual" or "Semantic" mode

#### Branding
New Ferrite logo and icon set.

- [x] **New logo design** - Ferrite crystal icon (orange geometric crystal shape)
- [x] **Windows icon** - Multi-size `.ico` file (16, 32, 48, 256px) embedded in executable
- [x] **macOS iconset** - `.iconset` folder for CI-generated `.icns`
- [x] **Linux icons** - PNG icons for `.deb` package (16-512px)
- [x] **Window icon** - Embedded 256px icon replaces default eframe "E" logo
- [x] **Icon generation script** - `assets/icons/generate_all_icons.py` for regenerating all sizes

---

### v0.2.5.1 (Released) - Memory, Encoding & Accuracy

> **Status:** Released (2026-01-17)

Point release focusing on memory optimization, multi-encoding support, cursor positioning improvements, scroll navigation accuracy, and UX polish.

#### Multi-Encoding File Support
- [x] **Encoding detection** - Auto-detect file encoding on open using `encoding_rs` + `chardetng` crates
- [x] **Common encodings** - Support Latin-1, Windows-1252, ISO-8859-x, Shift-JIS, EUC-KR, GBK, and other common encodings
- [x] **Status bar indicator** - Show detected encoding in status bar with click-to-change option
- [x] **Preserve on save** - Save files back in their original encoding (not forced UTF-8)

#### Cursor Positioning Improvements
- [x] **Galley-based click mapping** - Use egui's Galley for semi-accurate click-to-character index conversion
- [x] **Formatting marker mapping** - Map displayed text positions to raw markdown positions accounting for `**`, `*`, `` ` ``, `~~`, and `[links](url)`
- [x] **Text wrapping support** - Handle wrapped lines correctly by using actual text rect width for measurement
- [x] **Bold font measurement** - Use bold font for measurement when content starts with bold markers

#### Scroll Navigation Accuracy (Critical Fix)
- [x] **Unified scroll calculation** - Single function for all scroll-to-line operations ensuring consistent behavior across find, search-in-files, outline panel, and semantic minimap
- [x] **Fixed off-by-one errors** - Consistent 0-indexed vs 1-indexed line handling in `navigate_to_heading()` and related functions
- [x] **Fresh line height tracking** - Use actual rendered line height instead of stale/default 20.0 value
- [x] **Large file navigation** - Fixed scroll accuracy in files with 3000+ lines where targets around line 2000 could be 1000+ pixels off
- [x] **Semantic minimap highlight fix** - Fixed highlight offset when clicking outline/minimap items; uses byte offsets (matching search) instead of character offsets

> **Known Limitation:** Cursor positioning and scroll accuracy are best-effort within egui's constraints. Lines with mixed formatting may have slight drift on longer lines due to font width differences. Perfect positioning requires the custom editor widget planned for v0.3.0.

#### Internationalization
- [x] **Language selector** - Settings option to choose UI language
- [x] **Locale detection** - Auto-detect system language on first launch
- [x] **Simplified Chinese** - First community translation (thanks @sr79368142!)

#### Memory Optimization
> **Docs:** [Memory Optimization Plan](docs/technical/planning/memory-optimization.md)

Reduced ~250MB idle RAM usage to ~100-150MB:

- [x] **Fix memory leak** - Clean up `tree_viewer_states`, `csv_viewer_states`, `sync_scroll_states` when tabs close
- [x] **Custom allocator** - Add `mimalloc` (Windows) / `jemalloc` (Linux/macOS) to reduce heap fragmentation
- [x] **CJK font lazy loading** - Load CJK fonts on-demand when CJK characters detected, not at startup (saves 50-100MB for non-CJK users); granular per-language loading (Korean/Japanese/Chinese loaded independently)
- [x] **egui temp data cleanup** - Clean up stale `SyntaxHighlightCache` entries when tabs close

#### Intel Mac CPU Optimization ([#24](https://github.com/OlaProeis/Ferrite/issues/24))
> **Docs:** [CPU Issue Analysis](docs/technical/platform/intel-mac-cpu-issue-analysis.md) | [Repaint Investigation](docs/technical/platform/intel-mac-continuous-repaint-investigation.md)

Fixed 100% CPU usage on Intel Macs in Rendered mode:

- [x] **Log analysis tooling** - Created `scripts/analyze_log.py` to analyze verbose debug logs
- [x] **Removed verbose debug logging** - Eliminated `[LIST_ITEM_DEBUG]` statements generating ~2,200 log lines/second (48,850 in 22 seconds)
- [x] **Fixed continuous repaint cause** - Root cause identified: `Sense::hover()` on rendered editor's scroll area content was triggering continuous repaints (~60fps) whenever mouse moved over the area, bypassing the 100ms idle throttling. Changed to `Sense::focusable_noninteractive()` to allow proper throttling (~10fps when idle)

#### Bug Fixes & Polish
- [x] **Workspace view close button** - X button shifted left to prevent overlap with resize handles
- [x] **New file dirty flag** - Don't prompt to save new files that haven't been modified
- [x] **First-line indentation fix** - CJK paragraph indentation now only indents first line, not entire paragraph
- [x] **Session restore settings** - Added option to disable tab restoration on startup
- [x] **Linux close button** - Fixed hit-testing/overlay issue preventing close button clicks

---

### v0.2.5.2 (Released) - Editor Shortcuts, macOS, Linux & I18n

> **Status:** Released (2026-01-20)

Point release with new keyboard shortcuts, macOS improvements, Linux bug fixes, and internationalization cleanup.

#### New Features
- [x] **Delete Line shortcut** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Cmd/Ctrl+D deletes current line (configurable in settings) - thanks [@abcd-ca](https://github.com/abcd-ca)!
- [x] **Move Line Up/Down** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Alt+Up/Down swaps current line with adjacent lines also in split view - thanks [@abcd-ca](https://github.com/abcd-ca)!
- [x] **macOS file type associations** ([#30](https://github.com/OlaProeis/Ferrite/pull/30)) - Ferrite appears in Finder's "Open With" menu for .md, .json, .yaml, .toml, .txt files - thanks [@abcd-ca](https://github.com/abcd-ca)!

> **Note:** Opening files via "Open With" or dragging onto app icon not yet supported due to [winit#1751](https://github.com/rust-windowing/winit/issues/1751). Workaround: use `open -a Ferrite file.md` or File > Open.

#### Bug Fixes
- [x] **Ctrl+X cutting entire document** - Fixed egui bug where Ctrl+X with no selection would cut everything. Filter out `Event::Cut` when nothing is selected.
- [x] **Linux window drag stuck mouse** - Fixed critical bug where dragging the custom title bar on Linux caused the mouse to get "stuck" in drag mode. Bypassed egui's drag state machine using raw input detection for reliable window drag initiation.
- [x] **Split mode cursor position** ([#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Line operations now work correctly in Split view; rendered pane no longer overwrites cursor position - thanks [@abcd-ca](https://github.com/abcd-ca)!
- [x] **macOS modifier tooltips** ([#28](https://github.com/OlaProeis/Ferrite/pull/28), [#29](https://github.com/OlaProeis/Ferrite/pull/29)) - Tooltips now show "Cmd+E" on macOS instead of hardcoded "Ctrl+E" - thanks [@abcd-ca](https://github.com/abcd-ca)!
- [x] **Semantic minimap highlight accuracy** - Use byte offsets matching search behavior for correct highlight positioning

#### Internationalization
- [x] **I18n audit & cleanup** - Comprehensive audit of hardcoded strings, replacement with translation keys
- [x] **Orphaned key removal** - Removed ~200 unused translation keys from locale files
- [x] **Locale file sync** - All locale files now have consistent structure matching en.yaml
- [x] **New language support** - Added Estonian and Norwegian Bokmål via Weblate community translations

---

### v0.2.5.3 (In Progress) - Syntax Themes, Code Signing, Linux Performance & UI Polish

> **Status:** In Progress

Point release with Windows code signing, syntax theme selector, extended language support, Linux performance fixes, and UI improvements.

#### Code Signing
> **Docs:** [SignPath Code Signing](docs/technical/platform/signpath-code-signing.md)

- [x] **SignPath integration** - Windows artifacts (exe, MSI, portable zip) are now code signed via [SignPath.io](https://signpath.io/) free tier for open source
- [x] **CI/CD signing workflow** - Integrated signing into GitHub Actions release workflow with automatic artifact signing

#### UI Improvements
- [x] **View Mode Segmented Control** - Replaced single-letter toggle button (R/S/V) with a polished pill-shaped segmented control showing all three view modes (Raw, Split, Rendered) at once. Click directly on the desired mode with clear visual feedback for the active state. Adapts to file type (3 modes for markdown/CSV, 2 modes for JSON/YAML/TOML). Works in Zen mode.
- [x] **App logo in title bar** - Added Ferrite logo with transparent background to the title bar for better brand visibility

#### Syntax Highlighting
- [x] **Extended syntax support** - Added 100+ additional language syntaxes via `two-face` crate, including PowerShell (.ps1/.psm1/.psd1), TypeScript/TSX, Zig, Svelte, Vue, Terraform, Nix, and many more
- [x] **Syntax theme selector** - New dropdown in Appearance settings with 25+ syntax highlighting color themes (Dracula, Nord, Catppuccin variants, Gruvbox, Solarized, One Half, GitHub, VS Code Dark+, and more)

#### Performance
- [x] **Linux folder opening freeze** - Fixed critical 10+ second UI freeze when opening workspace folders on Linux (especially Fedora/KDE Plasma). Two root causes fixed:
  - **notify crate misconfiguration** - Was configured with `default-features = false, features = ["macos_kqueue"]` which disabled inotify on Linux, forcing fallback to slow polling-based file watching
  - **Synchronous recursive scanning** - Workspace initialization scanned entire directory tree on UI thread. Now uses lazy loading: only root is scanned initially, subdirectories load on-demand when expanded

#### Bug Fixes
- [x] **Line breaks in list items** ([#41](https://github.com/OlaProeis/Ferrite/issues/41)) - Fixed hard line breaks (`\` at end of line) within list items showing as a square box instead of rendering as a line break
- [x] **Git deleted file icon rendering** - Fixed git "deleted" status icon showing as a square box in file tree. Changed from unsupported Unicode character to ASCII minus.
- [x] **Blockquote/table overflow** - Added horizontal scrolling for tables and blockquotes when content exceeds container width. Wide content no longer breaks max line width for subsequent content. Code blocks and mermaid diagrams already have internal scroll handling.
- [x] **PowerShell file rendering collapse** - Fixed critical bug where PowerShell and other files without syntax definitions would collapse all content to a single line
- [x] **Alt-tab/taskbar visibility on Wayland** - Fixed Ferrite window not appearing in alt-tab switcher or taskbar on Linux desktop environments (KDE Plasma, GNOME) running Wayland. Added `app_id` to ViewportBuilder.
- [x] **Find/Replace replace icon** - Fixed the replace icon (↳) showing as a square box in the Find and Replace panel. Changed to universally-supported arrow (→).
- [x] **Tree viewer context menu icon** - Fixed the context menu button (⋯) in JSON/YAML/TOML tree viewer showing as a square. Changed to simple dots (...).
- [x] **Recent files menu position** - Fixed the recent files/folders popup menu appearing below and covering the filename button in the status bar. Menu now appears above the button.

---

### v0.2.6 (Planned) - Performance & Large Files

> **Status:** Planned

v0.2.6 focuses on **large file performance** (handling 80MB+ CSV files) and new features.

#### Check for Updates
> **Docs:** [Check for Updates PRD](docs/ai-workflow/prds/prd-v0.2.6-check-for-updates.md)

One-click update flow while maintaining Ferrite's offline-first philosophy:

- [ ] **Check for Updates button** - Settings panel button that checks GitHub and prompts to install if update found
- [ ] **Update prompt** - "v0.2.7 available. Update now?" with warning to save work
- [ ] **Download with progress** - Progress bar showing MB downloaded
- [ ] **Windows MSI** - Download → launch installer → app closes automatically
- [ ] **Portable/macOS/Linux** - Download to Downloads folder → open file manager → show instructions
- [ ] **Linux package detection** - Detect deb/rpm/AUR → show "update via package manager" message
- [ ] **Minimal dependency** - Uses lightweight `ureq` crate (~200KB)

> **Philosophy:** No automatic checking. Only goes online when user clicks the button. Single streamlined flow from check to install.

#### Idle Mode CPU Optimization ✅
> **Docs:** [Idle Mode Optimization](docs/technical/platform/idle-mode-optimization.md)

Fixed ~10% idle CPU usage down to <1% with tiered repaint scheduling:

- [x] **Tiered idle intervals** - Light idle (100ms/~10 FPS) and deep idle (500ms/~2 FPS) based on user interaction time
- [x] **User interaction tracking** - Detect keyboard, mouse, scroll events to determine idle state
- [x] **Scroll animation awareness** - Check sync scroll animations in `needs_continuous_repaint()`
- [x] **Window title optimization** - Only send viewport command when title changes
- [x] **Disable unnecessary repaints** - Set `repaint_on_widget_change = false`
- [x] **Animation time fix** - Remove conflicting animation_time override in ThemeManager

#### Memory Optimization (Deferred from v0.2.5.1)
- [ ] **Reduce undo history** - Lower `max_undo_size` from 100 to 50; add size-aware limits for large files
- [ ] **Syntect lazy loading** - Investigate loading syntax grammars on-demand instead of all 40+ at startup

#### Large File Performance ([#19](https://github.com/OlaProeis/Ferrite/issues/19) partial)
Critical performance improvements for handling large CSV/data files:

- [ ] **Large file detection** - Auto-detect files > 10MB on open, show warning toast
- [ ] **View-only mode for large files** - Disable Raw view editing for files > threshold (egui TextEdit can't handle 80MB)
- [ ] **Lazy CSV row parsing** - Parse rows on-demand using byte offset index instead of loading all rows into `Vec<Vec<String>>`
- [ ] **Row offset indexing** - First pass scans file to record byte offsets of each row start; parse only visible rows + buffer
- [ ] **LRU row cache** - Cache recently parsed rows (max ~10K rows) for smooth scrolling
- [ ] **Background CSV scanning** - Scan file in background thread with progress indicator; show first 1000 rows immediately
- [ ] **Disable expensive features** - Skip minimap, syntax highlighting, and undo history for large files
- [ ] **Memory optimization** - Don't store both raw content and parsed rows; use slices into original string where possible

#### Flowchart Refactoring
- [ ] **Modular refactor** - Split the 3500+ line `flowchart.rs` into smaller, maintainable modules (parser, layout, renderer, shapes, edges)
- [ ] **Code cleanup** - Improve code organization, reduce duplication, add documentation

#### Mermaid Improvements
Additional mermaid fixes and enhancements:

- [ ] **Testing & validation** - Comprehensive testing of all diagram types with edge cases
- [ ] **Bug fixes** - Address rendering issues discovered during v0.2.5 testing

#### Bug Fixes & Polish
- [ ] **Table overflow UX improvement** - Current horizontal scrollbar on wide tables is functional but not ideal. Consider: (1) cell word-wrap by default to naturally fit tables, (2) thin hover-visible scrollbar for truly wide tables, (3) overflow fade indicator with drag-to-pan
- [ ] **macOS Intel sync scrolling** ([#24](https://github.com/OlaProeis/Ferrite/issues/24)) - Bidirectional scroll sync between Raw/Rendered views on Intel Macs **Deferred**
- [ ] **macOS window controls** ([#24](https://github.com/OlaProeis/Ferrite/issues/24)) - Native traffic light style instead of Windows-style icons **Deferred**
- [ ] **Window controls redesign** - Redesign minimize/maximize/close icons for a more polished look
- [ ] **JSON rendered view Zen mode centering** - JSON tree viewer not centering content when Zen mode is active also applies to CSV mode
- [ ] **TOC navigation stability** - Investigate and fix crashes/accuracy issues when jumping via outline in large files
- [ ] **Light theme settings contrast** - Some settings panel options don't switch to dark foreground colors in light theme

#### Vim Mode
- [ ] **Vim keybindings** - Optional Vim-style modal editing (Normal/Insert/Visual modes) with common commands

#### Internationalization Polish
- [ ] **Expand i18n coverage** - Add translation keys for keyboard shortcuts panel, JSON structure panel, ribbon tooltips, and other untranslated strings
- [ ] **HTML export i18n** - Include CJK paragraph indentation in exported HTML

#### Portable Mode
- [ ] **Portable mode support** - Detect `ferrite.portable` marker file next to executable; store config and session data in local `data/` folder instead of system directories

#### Executable Code Blocks
Run code snippets directly in the rendered preview — inspired by Jupyter notebooks and Marco.

- [ ] **Run button on code blocks** - Add "▶ Run" button to fenced code blocks in rendered/split view
- [ ] **Shell/Bash execution** - Execute shell scripts via `std::process::Command`; display stdout/stderr below block
- [ ] **Python support** - Detect `python` or `python3` and run with system interpreter
- [ ] **Output persistence** - Option to keep output visible or clear on re-run
- [ ] **Timeout handling** - Kill long-running scripts after configurable timeout (default 30s)
- [ ] **Security warning** - First-run dialog warning about code execution risks; require opt-in in settings

> **Security Note:** Code execution is inherently risky. This feature will be opt-in and disabled by default. Users must explicitly enable it in settings.

#### Content Blocks / Callouts
Styled callout blocks for notes, warnings, tips — common in technical documentation (Obsidian, Notion, GitHub).

- [ ] **GitHub-style syntax** - Support `> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, `> [!CAUTION]`, `> [!IMPORTANT]`
- [ ] **Custom titles** - Support `> [!NOTE] Custom Title` syntax
- [ ] **Styled rendering** - Color-coded blocks with icons (ℹ️ 💡 ⚠️ 🔴 ❗) in rendered view
- [ ] **Collapsible variant** - `> [!NOTE]- Collapsed by default` syntax for expandable sections
- [ ] **Nesting support** - Allow content blocks inside other blocks and lists

#### Additional Format Support
Extend Ferrite's format support with common configuration and data files.

##### XML Tree Viewer
- [ ] **XML file support** - Open `.xml` files with syntax highlighting
- [ ] **Tree view** - Reuse JSON/YAML tree viewer for hierarchical XML display
- [ ] **Attribute display** - Show element attributes in tree nodes
- [ ] **Namespace handling** - Display namespace prefixes appropriately

##### Configuration Files
- [ ] **INI/CONF/CFG support** - Parse and display `.ini`, `.conf`, `.cfg` files with section headers
- [ ] **Properties files** - Java `.properties` file support with key-value tree view
- [ ] **ENV files** - `.env` file support with optional secret masking (hide values by default, click to reveal)
- [ ] **Syntax highlighting** - Appropriate highlighting for each config format

##### Log File Viewing
- [ ] **Log file detection** - Recognize `.log` files and common log patterns
- [ ] **Level highlighting** - Color-code ERROR (red), WARN (yellow), INFO (blue), DEBUG (gray)
- [ ] **Timestamp recognition** - Highlight ISO timestamps and common date formats
- [ ] **Large log handling** - Leverage lazy loading infrastructure for large log files

---

### v0.3.0 (Planned) - Mermaid Crate + Editor Improvements

> **Status:** Planning  
> **Docs:** [Mermaid Crate Plan](docs/mermaid-crate-plan.md) | [Custom Editor Plan](docs/technical/planning/custom-editor-widget-plan.md) | [Modular Refactor Plan](docs/refactor.md)

v0.3.0 focuses on extracting the Mermaid renderer as a standalone crate and continuing diagram improvements.

#### 1. Mermaid Crate Extraction
Extract Ferrite's native Mermaid renderer (~6000 lines) into a standalone pure-Rust crate.

- [ ] **Standalone crate** - Backend-agnostic architecture with SVG, PNG, and egui outputs
- [ ] **Public API** - `parse()`, `layout()`, `render()` pipeline
- [ ] **SVG export** - Generate valid SVG files from diagrams
- [ ] **PNG export** - Rasterize via resvg
- [ ] **WASM compatible** - SVG backend works in browsers

#### 2. Mermaid Diagram Improvements
Continue improving diagram rendering quality:

##### Deferred from v0.2.5
- [ ] **Diagram insertion toolbar** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Toolbar button to insert mermaid code blocks with template syntax
- [ ] **Syntax hints in Help** ([#4](https://github.com/OlaProeis/Ferrite/issues/4)) - Help panel documenting supported diagram types with syntax examples

##### Git Graph (Major Rewrite)
- [ ] **Horizontal timeline layout** - Left-to-right commit flow like Mermaid
- [ ] **Branch lanes** - Distinct horizontal lanes per branch with colored labels
- [ ] **Merge visualization** - Curved paths connecting branches
- [ ] **Tags and highlights** - Visual markers on commits

##### Flowchart
- [ ] **More node shapes** - Parallelogram, trapezoid, double-circle, etc.
- [ ] **style directive** - Per-node inline styling (complements existing classDef from v0.2.5)

##### State Diagram
- [ ] **Fork/join pseudostates** - Parallel regions
- [ ] **History states** - Shallow (H) and deep (H*) history

##### Manual Layout Support
Enable manual node positioning while maintaining mermaid.js compatibility — a key differentiator for mermaid-rs.

- [ ] **Comment-based position hints** - Parse `%% @pos <node_id> <x> <y>` directives (ignored by mermaid.js, respected by Ferrite)
- [ ] **Layout mode toggle** - Support `%% @ferrite-layout: manual` to enable manual positioning
- [ ] **Drag-to-reposition** - Drag nodes in rendered view → auto-update source with position comments
- [ ] **Export options** - "Export clean" strips layout hints for sharing pure Mermaid syntax
- [ ] **Fallback behavior** - Diagrams without position hints use auto-layout (Sugiyama, etc.)

> **Why this matters:** Mermaid is declarative — layout is computed, not specified. This prevents "visual thinking" workflows where users want to arrange diagrams as thought tools. By using `%%` comments (which mermaid.js ignores), we add manual positioning without breaking compatibility. Diagrams remain valid Mermaid syntax and render everywhere — just with different layouts.

#### 3. Custom Editor Widget (Stretch Goal)
Replace egui's `TextEdit` with a custom `FerriteEditor` widget to unblock advanced editing features.

- [ ] **FerriteEditor widget** - Custom text editor using egui drawing primitives
- [ ] **Rope-based buffer** - Efficient text storage via `ropey` crate
- [ ] **Full multi-cursor editing** - Text operations at all cursor positions
- [ ] **Code folding with text hiding** - Actually collapse regions visually

#### 4. Semantic Minimap Polish
- [ ] **Pixel-perfect scroll positioning** - With custom editor widget, use actual galley coordinates for perfect navigation centering (basic fix shipped in v0.2.5.1; v0.3.0 provides full solution via FerriteEditor)

#### 5. Markdown Enhancements
- [ ] **Wikilinks support** ([#1](https://github.com/OlaProeis/Ferrite/issues/1)) - `[[wikilinks]]` syntax with auto-completion
- [ ] **Backlinks panel** ([#1](https://github.com/OlaProeis/Ferrite/issues/1)) - Show documents linking to current file

#### 6. HTML Rendering (GitHub Parity)
Render embedded HTML in markdown preview, matching GitHub's supported subset.

##### Phase 1: Block Elements
- [ ] **`<div align="...">`** - Center/left/right alignment for content blocks
- [ ] **`<details><summary>`** - Collapsible sections using egui's CollapsingHeader
- [ ] **`<br>`** - Explicit line breaks

##### Phase 2: Inline Elements
- [ ] **`<kbd>`** - Keyboard key styling (monospace with border)
- [ ] **`<sup>` / `<sub>`** - Superscript and subscript text
- [ ] **`<img>` attributes** - Respect width/height on image tags

##### Phase 3: Advanced
- [ ] **Nested HTML** - HTML containing markdown containing HTML
- [ ] **`<table>` (HTML tables)** - Render HTML table syntax (separate from GFM pipe tables)

> **Note:** Only safe HTML elements supported — no `<script>`, `<style>`, `<iframe>`, or event handlers. This matches GitHub's security model.

#### 7. Platform & Distribution
Improve installation experience across all platforms.

##### Windows Installer
- [ ] **Inno Setup installer** - Professional `.exe` installer (like the Linux `.deb`)
- [ ] **File associations** - Register as handler for `.md`, `.json`, `.yaml`, `.toml` files
- [ ] **Context menu integration** - "Open with Ferrite" for files and folders (workspace mode)
- [ ] **Add to PATH option** - Run `ferrite` from any terminal
- [ ] **Start Menu & Desktop shortcuts** - Standard Windows integration
- [ ] **Clean uninstaller** - Remove all registry entries on uninstall
- [ ] **CI automation** - Build installer automatically in GitHub Actions release workflow

##### macOS
- [ ] **App signing & notarization** - Create proper `.app` bundle, sign with Developer ID, notarize with Apple

### v0.4.0 (Planned) - Math Support & Document Formats

> **Status:** Planning  
> **Docs:** [Math Support Plan](docs/math-support-plan.md)

Native LaTeX math rendering and read-only support for Office documents. Focus on the most impactful features first.

#### Math Rendering Engine
- [ ] **LaTeX parser** - Parse `$...$` (inline) and `$$...$$` (display) syntax
- [ ] **Layout engine** - TeX-style box model for fractions, subscripts, radicals
- [ ] **Math fonts** - Embedded glyph subset for consistent cross-platform rendering
- [ ] **egui integration** - Render math in preview and split views

#### Supported LaTeX (Target)
- [ ] **Fractions** - `\frac{a}{b}` with proper stacking
- [ ] **Subscripts/superscripts** - `x^2`, `x_i`, `x_i^2`
- [ ] **Greek letters** - `\alpha`, `\beta`, `\pi`, etc.
- [ ] **Operators** - `\sum`, `\int`, `\prod`, `\lim`
- [ ] **Roots** - `\sqrt{x}`, `\sqrt[n]{x}`
- [ ] **Delimiters** - Auto-scaling `\left( \right)`
- [ ] **Matrices** - `\begin{matrix}...\end{matrix}`
- [ ] **Font styles** - `\mathbf`, `\mathit`, `\mathrm`

#### WYSIWYG Features (Requires FerriteEditor from v0.3.0)
- [ ] **Inline math preview** - See rendered math while typing (Typora-style)
- [ ] **Click-to-edit** - Click rendered math to edit source
- [ ] **Symbol palette** - Quick access to common symbols

#### Office Document Support (Read-Only)
View Word and Excel documents without traditional page layouts — modern "page-less" document viewing.

> **Philosophy:** Word's page-based layout is a holdover from print. Ferrite displays Office documents as flowing, readable content — like a modern web reader.

##### DOCX Support (Word Documents)
- [ ] **DOCX file opening** - Open `.docx` files via File → Open and drag-drop
- [ ] **Page-less rendering** - Display Word content as continuous flowing text (no page breaks)
- [ ] **Text extraction** - Headings, paragraphs, lists, bold/italic formatting
- [ ] **Table rendering** - Display Word tables using existing table renderer
- [ ] **Image support** - Extract and display embedded images
- [ ] **Export to Markdown** - Convert DOCX → Markdown for editing (lossy conversion with warnings)

##### XLSX Support (Excel Spreadsheets)
- [ ] **XLSX file opening** - Open `.xlsx` files via File → Open and drag-drop
- [ ] **Sheet selector** - Tab bar or dropdown to switch between worksheets
- [ ] **Table rendering** - Reuse CSV viewer with rainbow columns, header detection
- [ ] **Cell formatting** - Preserve basic number/date formatting where possible
- [ ] **Large sheet handling** - Lazy row loading for sheets with 10K+ rows (reuse v0.2.6 lazy CSV infrastructure)

> **Technical Notes:**
> - DOCX/XLSX are open standards (ECMA-376/ISO 29500) — no licensing concerns
> - Rust crates: `calamine` (Excel), `docx-rs` or `quick-xml` (Word)
> - Read-only initially; editing would require significant additional work
> - Complex formatting (tracked changes, comments, macros) out of scope for v0.4.0

##### OpenDocument Support (LibreOffice)
- [ ] **ODT file opening** - Open `.odt` (Writer) files with same approach as DOCX
- [ ] **ODS file opening** - Open `.ods` (Calc) files with same approach as XLSX
- [ ] **Shared rendering** - Reuse DOCX/XLSX renderers (both are XML-in-ZIP formats)

---

### Future (v0.5.0+)

> **Note:** Features in this section are ideas under consideration. We haven't fully decided which to implement — some may be deferred, modified, or not implemented at all based on user feedback and development priorities.

#### Core Improvements
- [ ] **Memory-mapped file I/O** ([#19](https://github.com/OlaProeis/Ferrite/issues/19)) - Handle GB-scale CSV/JSON files efficiently without loading into RAM
- [ ] **TODO list editing UX** - Smart cursor behavior in task lists (respect line start position, don't jump past `- [ ]` syntax)
- [ ] **Spell checking** - Integrated spell check with custom dictionaries
- [ ] **Custom themes** - Import/export theme files
- [ ] **Virtual/ghost text** - AI completions, suggestions, etc.
- [ ] **Column/box selection** - Rectangular text selection

#### Additional Document Formats (Candidates)
These formats are under consideration based on user demand:

##### Jupyter Notebooks (.ipynb)
- [ ] **Notebook file opening** - Open `.ipynb` files (JSON-based format)
- [ ] **Cell rendering** - Display markdown cells rendered, code cells with syntax highlighting
- [ ] **Output display** - Show cell outputs (text, images, tables)
- [ ] **Read-only initially** - View notebooks; editing deferred

> Extremely popular in data science. JSON-based format is relatively straightforward to parse.

##### EPUB (E-Books)
- [ ] **EPUB file opening** - Open `.epub` files for distraction-free reading
- [ ] **Chapter navigation** - Outline panel shows table of contents
- [ ] **Page-less reading** - Continuous scroll through book content
- [ ] **Reading position** - Remember last position in book

> EPUB is HTML+CSS in a ZIP — fits Ferrite's "page-less document" vision.

##### LaTeX Source Files (.tex)
- [ ] **TeX file opening** - Open `.tex` files with syntax highlighting
- [ ] **Math preview** - Render `$...$` and `$$...$$` blocks inline (requires v0.4.0 Math Engine)
- [ ] **Section outline** - Extract `\section`, `\subsection` for outline panel
- [ ] **BibTeX support** - Basic `.bib` file viewing

#### Alternative Markup Languages ([#21](https://github.com/OlaProeis/Ferrite/issues/21))
Support for markup languages beyond Markdown. Implementation approach TBD (native parser vs plugin system).

- [ ] **reStructuredText** (.rst) - Python documentation standard; rendered preview like Markdown
- [ ] **Org-mode** (.org) - Emacs org-mode format; plain-text productivity (basic support only)
- [ ] **AsciiDoc** - Technical documentation format
- [ ] **Zim-Wiki** - Zim Desktop Wiki syntax
- [ ] **Format auto-detection** - Detect markup format from file extension or content

### Long-Term Vision

#### Plugin System
Extensibility architecture for custom functionality, inspired by Obsidian's plugin ecosystem.

- [ ] **Plugin API design** - Define extension points (commands, views, file handlers)
- [ ] **Scripting support** - Lua, WASM, or Rhai-based plugins
- [ ] **Community plugins** - Distribution and discovery mechanism

#### Headless Editor Library
Extract `FerriteEditor` as a standalone, framework-agnostic text editing library for the Rust ecosystem.

> **Context:** There's currently no general-purpose "headless" code editor library in Rust. Existing implementations (egui's TextEdit, Lapce/Floem, Zed/gpui) are tightly coupled to their UI frameworks. The v0.3.0 custom editor and modular architecture lay the groundwork for potential extraction.

**Prerequisites (from v0.3.0):**
- Custom `FerriteEditor` widget with rope-based buffer
- Modular architecture with clean separation of concerns
- Framework-agnostic core logic

**Extraction would involve:**
- [ ] Abstract rendering backend (trait-based: egui, wgpu, vello, SVG, etc.)
- [ ] Framework-agnostic input handling
- [ ] Standalone crate with minimal dependencies
- [ ] Integration with [Parley](https://github.com/linebender/parley) for advanced text layout/shaping (optional)

---

## Completed ✅

### v0.2.5.3 (Current Release) - Syntax Themes & UI Polish

See [CHANGELOG.md](CHANGELOG.md) for full release notes. Key highlights:
- **View Mode Segmented Control** - New pill-shaped segmented control replacing single-letter toggle
- **Extended syntax support** - 100+ additional language syntaxes via `two-face` crate
- **Syntax theme selector** - 25+ syntax highlighting themes (Dracula, Nord, Catppuccin, etc.)
- **Blockquote/code block overflow** - Horizontal scrolling for wide content
- **PowerShell rendering fix** - Fixed content collapsing to single line

### v0.2.5.2 - Editor Shortcuts, macOS, Linux & I18n

See [CHANGELOG.md](CHANGELOG.md) for full release notes. Key highlights:
- **Delete Line shortcut** - Cmd/Ctrl+D deletes current line
- **Move Line Up/Down** - Alt+↑/↓ swaps lines
- **macOS file type associations** - Ferrite appears in Finder's "Open With"
- **Linux window drag fix** - Fixed stuck mouse when dragging custom title bar
- **I18n cleanup** - Comprehensive audit, ~200 orphaned keys removed
- **New languages** - Estonian and Norwegian Bokmål

### v0.2.5.1 - Memory, Encoding & Polish

See [CHANGELOG.md](CHANGELOG.md) for full release notes. Key highlights:
- **Multi-encoding file support** - Auto-detect and preserve encodings (Latin-1, Shift-JIS, Windows-1252, etc.)
- **Memory optimization** - CJK lazy loading (250MB → 72MB), custom allocators, memory leak fixes, egui cleanup
- **Cursor positioning improvements** - Galley-based click mapping, formatting marker mapping, text wrapping support
- **Internationalization** - Language selector, locale detection, Simplified Chinese translation
- **Intel Mac CPU fix** - Fixed continuous repaint issue (60fps → 10fps when idle)
- **Bug fixes** - New file dirty flag, CJK first-line indentation, workspace close button, Linux close button, session restore settings

### v0.2.5 - Mermaid Update & Editor Polish

See [CHANGELOG.md](CHANGELOG.md) for full release notes. Key highlights:
- **Mermaid modular refactor** - Split 7000+ line file into maintainable modules
- **Mermaid improvements** - YAML frontmatter, parallel edges, classDef/linkStyle, subgraph improvements
- **CSV/TSV viewer** - Native table view with rainbow columns, delimiter detection, header detection
- **Semantic minimap** - Header labels, content type indicators, density visualization, mode toggle
- **i18n infrastructure** - String extraction, YAML translation files, Weblate integration
- **CJK paragraph indentation** - First-line indentation for Chinese/Japanese text
- **Custom font selection** - Select preferred fonts for editor and UI
- **Main menu UI redesign** - Modernized layout and visual design
- **Split view dual editing** - Both panes now fully editable with undo/redo support
- **Keyboard shortcut customization** - Rebind shortcuts via settings panel
- **Git status auto-refresh** - Automatic refresh on save, focus, timer, and file events
- **Drag & drop images** - Drop images to auto-save to ./assets/ and insert markdown link
- **Table of Contents generation** - Generate/update TOC with Ctrl+Shift+U
- **Document statistics** - Tabbed panel with word count, reading time, heading/link/image counts
- **Snippets** - Text expansions (`;date`, `;time`) with custom snippet support
- **Recent folders** - Recent files menu now includes workspace folders
- **Windows fullscreen toggle** - F10 for fullscreen (separate from F11 Zen mode)
- **Bug fixes** - Session persistence, table editing, quick switcher, config persistence, line width, window resize

### v0.2.3 - Polish & Editor Productivity

A focused release adding editor productivity features and platform improvements.

#### Editor Productivity
- [x] **Go to Line (Ctrl+G)** - Quick navigation to specific line number with modal dialog
- [x] **Duplicate Line (Ctrl+Shift+D)** - Duplicate current line or selection
- [x] **Move Line Up/Down (Alt+↑/↓)** - Rearrange lines without cut/paste
- [x] **Auto-close Brackets & Quotes** - Type `(` to get `()` with cursor in middle
- [x] **Smart Paste for Links** - Select text, paste URL → creates `[text](url)` markdown link

#### UX Improvements
- [x] **Configurable line width** ([#15](https://github.com/OlaProeis/Ferrite/issues/15)) - Option to limit text width for improved readability (Off/80/100/120/Custom)

#### Platform & Distribution
- [x] **Linux musl build** - Statically-linked musl binary for maximum Linux compatibility (no glibc dependency)

#### Bug Fixes
- [x] **Linux close button cursor flicker** - Fixed cursor rapidly switching between pointer/move/resize near window close button (title bar exclusion zone)

### v0.2.2 - Performance & Stability

A focused release addressing bugs reported after v0.2.1 launch, improving CLI usability, and adding quality-of-life features.

#### Bug Fixes
- [x] **UTF-8 crash in tree viewer** - Fix string slicing panic when displaying JSON/YAML strings containing multi-byte characters (Norwegian øæå, Chinese, emoji, etc.)
- [x] **Ubuntu 22.04 .deb compatibility** ([#6](https://github.com/OlaProeis/Ferrite/issues/6)) - Build on Ubuntu 22.04 for glibc 2.35 compatibility
- [x] **Undo/redo behavior** ([#5](https://github.com/OlaProeis/Ferrite/issues/5)) - Fixed scroll position reset, focus loss, double-press requirement, and cursor restoration on Ctrl+Z
- [x] **Misleading code folding UI** ([#12](https://github.com/OlaProeis/Ferrite/issues/12)) - Hide non-functional fold indicators by default; remove confusing "Raw View" button from Rendered JSON view
- [x] **CJK character rendering** ([#7](https://github.com/OlaProeis/Ferrite/issues/7)) - ✅ Multi-region CJK support (Korean, Chinese, Japanese) via system font fallback using `font-kit` (PR [#8](https://github.com/OlaProeis/Ferrite/pull/8) by [@SteelCrab](https://github.com/SteelCrab) 🙏)
- [x] **macOS Intel support** ([#16](https://github.com/OlaProeis/Ferrite/issues/16)) - Separate x86_64 build for Intel Macs via `macos-13` runner (PR [#2](https://github.com/OlaProeis/Ferrite/pull/2) fixed naming, Intel job added)

#### Performance Optimizations
- [x] **Large file performance** - Deferred syntax highlighting keeps typing responsive in 5000+ line files
- [x] **Syntax highlighting optimization** - Galley caching for instant scrolling, deferred re-highlighting while typing
- [x] **Scroll performance** - Instant syntax colors when scrolling/jumping via minimap

#### UX Improvements
- [x] **Default view mode setting** ([#3](https://github.com/OlaProeis/Ferrite/issues/3)) - Option to set default view mode (Raw/Rendered/Split) for new tabs

#### CLI Improvements
- [x] **Command-line file opening** ([#9](https://github.com/OlaProeis/Ferrite/issues/9)) - `ferrite file.md` opens file directly in editor
- [x] **Version/help flags** ([#10](https://github.com/OlaProeis/Ferrite/issues/10)) - `-V/--version` and `-h/--help` CLI support
- [x] **Configurable log level** ([#11](https://github.com/OlaProeis/Ferrite/issues/11)) - `log_level` setting in config.json with CLI override (`--log-level`)

### v0.2.1

#### Mermaid Diagram Enhancements
- [x] **Accurate text measurement** - Replace character-count estimation with egui font metrics
- [x] **Dynamic node sizing** - Nodes resize to fit their labels without clipping
- [x] **Text overflow handling** - Edge labels truncate with ellipsis when too long
- [x] **User Journey icons** - Fixed unsupported emoji rendering with text fallbacks
- [x] **Sequence control-flow blocks** - Support for `loop`, `alt`, `opt`, `par`, `critical`, `break` blocks with nesting
- [x] **Sequence activation boxes** - `activate`/`deactivate` markers and `+`/`-` shorthand on lifelines
- [x] **Sequence notes** - `Note left/right/over` syntax support with dog-ear rendering
- [x] **Flowchart branching layout** - Sugiyama-style layered graph with side-by-side branches
- [x] **Flowchart subgraphs** - Nested `subgraph`/`end` blocks with direction overrides
- [x] **Back-edge routing** - Cycle edges rendered with smooth bezier curves
- [x] **Smart edge exit points** - Decision node edges exit from different points to prevent crossing
- [x] **Composite/nested states** - `state Parent { ... }` syntax with recursive nesting
- [x] **Advanced state transitions** - Color-coded transitions and smart anchor points

### v0.2.0

#### Major Features
- [x] **Side-by-side split view** - Raw editor on left, rendered preview on right with resizable divider
- [x] **MermaidJS native rendering** - 11 diagram types rendered natively in Rust/egui (flowchart, sequence, pie, state, mindmap, class, ER, git graph, gantt, timeline, user journey)
- [x] **Editor minimap** - VS Code-style scaled preview with click-to-navigate and viewport indicator
- [x] **Code folding indicators** - Fold detection for headings, code blocks, lists; gutter indicators (▶/▼)
- [x] **Live Pipeline panel** - Pipe JSON/YAML content through shell commands with real-time output
- [x] **Zen Mode** - Distraction-free writing with centered text column
- [x] **Git integration** - Visual status indicators in file tree (modified, added, untracked, ignored)
- [x] **Auto-save** - Configurable delay, per-tab toggle, temp-file based safety
- [x] **Session persistence** - Restore open tabs, cursor position, scroll offset, view mode on restart
- [x] **Bracket matching** - Highlight matching brackets `()[]{}<>` and markdown emphasis `**` `__`
- [x] **Syntax highlighting** - Full-file syntax highlighting for source code files (40+ languages including Rust, Python, JavaScript, Go, C/C++, etc.)

#### Bug Fixes
- [x] **Rendered mode list editing** - Fixed item index mapping, structural key hashing, edit state consistency
- [x] **Light mode contrast** - Improved text/border visibility, WCAG AA compliant, added tab/editor separator
- [x] **Scroll synchronization** - Bidirectional sync between Raw/Rendered, mode switch preservation
- [x] **Search-in-Files navigation** - Click result scrolls to match with transient highlight
- [x] **Search panel viewport** - Fixed top/bottom clipping issues

#### UX Improvements
- [x] **Tab context menu** - Reorganized icons with logical grouping

### v0.1.0

#### Core Features
- [x] WYSIWYG Markdown editing
- [x] Multi-format support (Markdown, JSON, YAML, TOML)
- [x] Tree viewer for structured data
- [x] Workspace mode with file tree
- [x] Quick switcher (Ctrl+P)
- [x] Search in files (Ctrl+Shift+F)
- [x] Light and dark themes
- [x] Document outline panel
- [x] HTML export
- [x] Formatting toolbar
- [x] Custom borderless window
- [x] Multi-tab editing
- [x] Find and replace
- [x] Undo/redo per tab

---

## Contributing

Found a bug or have a feature request? Please [open an issue](https://github.com/OlaProeis/Ferrite/issues/new/choose) on GitHub!