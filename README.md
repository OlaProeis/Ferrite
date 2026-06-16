# Ferrite

**English** | [简体中文](README.zh-CN.md)

<div align="center">

[![Website](https://img.shields.io/badge/website-getferrite.dev-blue?style=flat-square)](https://getferrite.dev)
[![Latest Release](https://img.shields.io/github/v/release/OlaProeis/Ferrite?style=flat-square)](https://github.com/OlaProeis/Ferrite/releases)
[![License](https://img.shields.io/github/license/OlaProeis/Ferrite?style=flat-square)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/OlaProeis/Ferrite?style=flat-square)](https://github.com/OlaProeis/Ferrite/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/OlaProeis/Ferrite?style=flat-square)](https://github.com/OlaProeis/Ferrite/issues)
[![Translation Status](https://hosted.weblate.org/widget/ferrite/ferrite-ui/svg-badge.svg)](https://hosted.weblate.org/engage/ferrite/)

**[getferrite.dev](https://getferrite.dev)** — Official website with downloads, features, and documentation

</div>

A fast, lightweight text editor for Markdown, JSON, YAML, and TOML files. Built with Rust and egui for a native, responsive experience.

> ⚠️ **Platform Note:** Ferrite is developed and tested on **Windows** and **Linux**. macOS support is experimental. If you encounter issues, please [report them](https://github.com/OlaProeis/Ferrite/issues).

<details>
<summary><strong>🛡️ Code Signing & Antivirus</strong></summary>

Starting with v0.2.6.1, **all Windows releases are digitally signed** with a certificate from [SignPath Foundation](https://signpath.org). This means:

- **Windows SmartScreen** recognizes the publisher — no more "Unknown publisher" warnings
- **Antivirus false positives** are significantly reduced thanks to the trusted code signature
- **Integrity verification** — you can verify that the binary hasn't been tampered with

### Previous False Positives
Older unsigned builds may still trigger antivirus detections. This was caused by:
- **Live Pipeline feature**: Uses `cmd.exe /C` on Windows for shell commands, which ML-based detection can misidentify
- **Rust compilation patterns**: Rust binaries can trigger heuristic detections due to unique characteristics

We've addressed this through code signing, build profile adjustments (disabled symbol stripping, speed optimization), and reporting to Microsoft's Security Intelligence portal.

### If You're Still Affected
If Windows Defender quarantines Ferrite:
1. **Upgrade**: Download the latest signed release from [GitHub Releases](https://github.com/OlaProeis/Ferrite/releases)
2. **Verify the signature**: Right-click `ferrite.exe` → Properties → Digital Signatures → should show "SignPath Foundation"
3. **Check VirusTotal**: Upload the file to [VirusTotal](https://www.virustotal.com) — signed builds should show clean results

Ferrite does **NOT** access passwords, browser data, or make unsolicited network connections. The only network access is the manual "Check for Updates" button (Settings → About), which contacts the GitHub Releases API when you click it. The application only accesses files you explicitly open.

</details>

<details>
<summary><strong>🍎 macOS Installation & Gatekeeper</strong></summary>

**GitHub Release** `.dmg` / `.tar.gz` builds are packaged as `Ferrite.app` but are **not** signed with an Apple Developer ID and are **not** notarized (CI limitation). **Gatekeeper**, especially on **macOS 15.x (Sequoia / 15.6+)**, may block or warn on first launch—see [GitHub #130](https://github.com/OlaProeis/Ferrite/issues/130) and the workarounds below.

📖 **Full troubleshooting:** [docs/install/macos.md](docs/install/macos.md)

### Option 1: Homebrew (often smoothest)

```bash
brew tap olaproeis/ferrite
brew install --cask ferrite
```

Homebrew typically avoids quarantine friction compared to raw GitHub downloads.

### Option 2: DMG / tar.gz from GitHub Releases + workaround

1. Download from [Releases](https://github.com/OlaProeis/Ferrite/releases):
   - **Apple Silicon:** `ferrite-macos-arm64.dmg` or `.tar.gz`
   - **Intel:** `ferrite-macos-x64.dmg` or `.tar.gz`
2. Copy `Ferrite.app` to **Applications** (or another folder you prefer).
3. **First launch — pick one:**
   - **Terminal (most reliable):** `xattr -dr com.apple.quarantine /Applications/Ferrite.app` — use the real path if the app is not under `/Applications`.
   - **Finder:** Control-click `Ferrite.app` → **Open** → **Open** (may **not** work on every macOS 15.x configuration).
   - **System Settings → Privacy & Security:** after a blocked launch, use **Open Anyway** if shown.

### Why this happens

Apple expects apps distributed outside the Mac App Store to be signed with a **Developer ID** certificate and **notarized**. Ferrite is open source—you can [audit the code](https://github.com/OlaProeis/Ferrite) and [build from source](docs/building.md)—but paid enrollment and CI wiring are still pending for GitHub-hosted macOS builds.

</details>

## 🤖 AI-Assisted Development

This project is 100% AI-generated code. All Rust code, documentation, and configuration was written by Claude (Anthropic) via [Cursor](https://cursor.com) with MCP tools.

<details>
<summary><strong>About the AI workflow</strong></summary>

### My Role
- **Product direction** — Deciding what to build and why
- **Testing** — Running the app, finding bugs, verifying features
- **Review** — Reading generated code, understanding what it does
- **Orchestration** — Managing the AI workflow effectively

### The Workflow
1. **Idea refinement** — Discuss concepts with multiple AIs (Claude, Perplexity, Gemini Pro)
2. **PRD creation** — Generate requirements using [Task Master](https://github.com/task-master-ai/task-master)
3. **Task execution** — Claude Opus handles implementation (preferring larger tasks over many subtasks)
4. **Session handover** — Structured prompts maintain context between sessions
5. **Human review** — Every handover is reviewed; direction adjustments made as needed

📖 **Full details:** [AI Development Workflow](docs/ai-workflow/ai-development-workflow.md)

### Open Process
The actual prompts and documents used to build Ferrite are public:

| Document | Purpose |
|----------|---------|
| [`current-handover-prompt.md`](docs/current-handover-prompt.md) | Active session context |
| [`ai-workflow/`](docs/ai-workflow/) | Full workflow docs, PRDs, historical handovers |
| [`handover/`](docs/handover/) | Reusable handover templates |

This transparency is intentional — I want others to learn from (and improve upon) this approach.

</details>

## Screenshots

![Ferrite Demo](assets/screenshots/demo.gif)

| Raw Editor | Split View | Zen Mode |
|------------|------------|----------|
| ![Raw Editor](assets/screenshots/raw-dark.png) | ![Split View](assets/screenshots/split-dark.png) | ![Zen Mode](assets/screenshots/zen-dark.png) |

> ✨ **v0.3.0 (Latest):** **eframe / egui 0.34.2** platform refresh. **PDF + themed HTML export**. **Executable code blocks** (`▶ Run`). **Rendered edit session** (one-click WYSIWYG block switching). **Split-view scroll sync**. **User accent color**. **Mermaid** insert toolbar + validation + flowchart polish. **Phosphor icons**. **Session recovery** hardening. **Quick note workflow**. See [CHANGELOG.md](CHANGELOG.md) for full details.

> 🛠️ **Coming in v0.3.1:** Inline **YouTube video embeds**, **multi-window**, Mermaid second wave (git graph rewrite, manual layout), CSV cell editing, GitHub HTML subset, and preview lock. LSP deferred to v0.3.2+. See [ROADMAP.md](ROADMAP.md) and [prd-v0.3.1.md](docs/ai-workflow/prds/prd-v0.3.1.md).

> 📦 **v0.2.6 Highlights:** Custom Editor Engine with virtual scrolling (80MB file uses ~80MB RAM), Multi-Cursor Editing, Code Folding, IME/CJK input improvements.

## Features

### Core Editing
- **WYSIWYG Markdown Editing** - Edit markdown with live preview, **one-click block switching** between headings, paragraphs, lists, and table cells in rendered mode, click-to-edit formatting, and syntax highlighting
- **Executable Code Blocks** - Run shell or Python fenced blocks from rendered/split preview (`▶ Run`); inline ANSI output, timeout, and Stop (opt-in via Settings; first-run consent)
- **YouTube Video Embeds** *(v0.3.1)* - Use `{{video URL}}` on its own line for inline playback in the rendered preview (primary window); optional `width=640` / `height=360` params set player size (width-only keeps 16:9); bare YouTube links stay normal clickable links; other windows and untrusted hosts show a thumbnail that opens in your browser
- **Multi-Format Support** - Native support for Markdown, JSON, CSV, YAML, and TOML files
- **Multi-Encoding Support** - Auto-detect and preserve file encodings (UTF-8, Latin-1, Shift-JIS, Windows-1252, GBK, and more)
- **Tree Viewer** - Hierarchical view for JSON/YAML/TOML with inline editing, expand/collapse, and path copying
- **Find & Replace** - Search with regex support and match highlighting
- **Go to Line (Ctrl+G)** - Quick navigation to specific line number
- **Undo/Redo** - Full undo/redo support per tab

### View Modes
- **Split View** - Side-by-side raw editor and rendered preview with resizable divider; both panes are fully editable; optional **live scroll sync** (minimap **Sync** / **2-way** controls)
- **Zen Mode** - Distraction-free writing with centered text column

### Editor Features
- **Syntax Highlighting** - Full-file syntax highlighting for 100+ languages (Rust, Python, JavaScript, Go, TypeScript, PowerShell, and more)
- **Code Folding** - Fold/unfold regions with gutter indicators (▶/▼) for headings, code blocks, and lists; collapsed content is hidden
- **Semantic Minimap** - Navigation panel with clickable header labels, content type indicators, and text density bars (switchable to VS Code-style pixel view)
- **Multi-Cursor Editing** - Ctrl+Click to add multiple cursors; type, delete, and navigate at all positions simultaneously
- **Bracket Matching** - Highlight matching brackets `()[]{}<>` and emphasis pairs `**` `__`
- **Auto-close Brackets & Quotes** - Type `(`, `[`, `{`, `"`, or `'` to get matching pair; selection wrapping supported
- **Duplicate Line (Ctrl+Shift+D)** - Duplicate current line or selection
- **Move Line Up/Down (Alt+↑/↓)** - Rearrange lines without cut/paste
- **Smart Paste for Links** - Select text then paste URL to create `[text](url)` markdown link
- **Drag & Drop Images** - Drop images into editor to auto-save to `./assets/` and insert markdown link
- **Table of Contents** - Generate/update TOC from headings with `<!-- TOC -->` block (Ctrl+Shift+U)
- **Snippets** - Text expansions like `;date` → current date, `;time` → current time, plus custom snippets
- **Auto-Save** - Configurable auto-save with temp-file safety
- **Line Numbers** - Optional line number gutter
- **Configurable Line Width** - Limit text width for readability (80/100/120 or custom)
- **Custom Font Selection** - Choose preferred fonts for editor and UI; important for CJK regional glyph preferences
- **Keyboard Shortcut Customization** - Rebind shortcuts via settings panel

### MermaidJS Diagrams
Native rendering of 11 diagram types directly in the preview:
- Flowchart, Sequence, Pie, State, Mindmap
- Class, ER, Git Graph, Gantt, Timeline, User Journey
- **Insert templates** - Format toolbar → **Insert → Mermaid…** for starter snippets per diagram type
- **Inline validation** - Parse errors in preview with squiggles on broken `mermaid` blocks in the raw editor
- **Flowchart polish** - Improved edge routing, extra shapes, `style` / classDef support; state diagrams support fork/join and history nodes

> **v0.3.0 Mermaid update:** Insert toolbar, syntax help (F1), inline validation, flowchart edge-routing and layout fixes, and state-diagram pseudostates. Complex diagrams may still differ from mermaid.js. See [ROADMAP.md](ROADMAP.md) for planned parity work.

### CSV/TSV Viewer
- **Native Table View** - View CSV and TSV files in a formatted table with fixed-width column alignment
- **Rainbow Column Coloring** - Alternating column colors for improved readability
- **Delimiter Detection** - Auto-detect comma, tab, semicolon, and pipe separators
- **Header Row Detection** - Intelligent detection and highlighting of header rows

### Workspace Features
- **Workspace Mode** - Open folders with file tree, quick switcher (Ctrl+P), and search-in-files (Ctrl+Shift+F)
- **Workspace File Index** - Ctrl+P and Search in Files scan the **entire workspace** in the background (not only expanded folders in the tree)
- **Quick Note Workflow** - Pathless scratch tabs: quit without a save dialog by default; closing a tab still prompts; unsaved notes persist via session recovery (Settings → Files)
- **Git Integration** - Visual status indicators (modified, added, untracked, ignored) with auto-refresh on save, focus, and file changes
- **Session Persistence** - Restore open tabs, cursor positions, and scroll offsets on restart; hardened crash recovery with identity checks and disk-conflict banner

### Terminal Workspace
- **Integrated Terminal** - Multiple instances with shell selection (PowerShell, CMD, WSL, bash)
- **Tiling & Splitting** - Create complex 2D grids with horizontal and vertical splits
- **Smart Maximize** - Temporarily maximize any pane to focus on work (Ctrl+Shift+M)
- **Layout Persistence** - Save and load your favorite terminal arrangements to JSON files
- **Theming & Transparency** - Custom color schemes (Dracula, etc.) and background opacity
- **Drag-and-Drop Tabs** - Reorder terminals with visual feedback
- **AI-Ready** - Visual "breathing" indicator when terminal is waiting for input (perfect for AI agents)

### Additional Features
- **Light & Dark Themes** - Beautiful themes with runtime switching; **custom Ferrite accent color** (Settings / Welcome) for headings, selection, tabs, and chrome
- **Document Outline & Statistics** - Navigate with outline panel; tabbed statistics showing word count, reading time, heading/link/image counts
- **Export & Print** - Export to **PDF** or **themed HTML** (options dialog, Mermaid as SVG); **print preview** opens a temp PDF in the in-app viewer; copy as HTML
- **Formatting Toolbar** - Quick access to bold, italic, headings, lists, links, Mermaid insert, and more
- **Phosphor Icons** - Consistent icon font across ribbon, toolbar, panels, and preview controls
- **Live Pipeline** - Pipe JSON/YAML content through shell commands (for developers)
- **Custom Window** - Borderless window with custom title bar and resize handles
- **Recent Files & Folders** - Click filename in status bar to access recently opened files and workspace folders
- **CJK Paragraph Indentation** - First-line indentation options for Chinese (2 chars) and Japanese (1 char) writing conventions

## Installation

### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/OlaProeis/Ferrite/releases).

| Platform | Download | Notes |
|----------|----------|-------|
| **Windows** | `ferrite-windows-x64.msi` | Recommended - full installer with Start Menu |
| Windows | `ferrite-portable-windows-x64.zip` | Portable - extract anywhere, run from USB |
| **Linux (Debian/Ubuntu)** | `ferrite-editor_amd64.deb` | For Debian, Ubuntu, Mint, Pop!_OS |
| **Linux (Fedora/RHEL)** | `ferrite-editor.x86_64.rpm` | For Fedora, RHEL, CentOS, Rocky |
| Linux | `ferrite-linux-x64.tar.gz` | Universal - works on any distro |
| **macOS (Apple Silicon)** | `ferrite-macos-arm64.dmg` | Recommended - drag to Applications (M1/M2/M3) |
| macOS (Apple Silicon) | `ferrite-macos-arm64.tar.gz` | Alternative - contains Ferrite.app bundle |
| **macOS (Intel)** | `ferrite-macos-x64.dmg` | Recommended - drag to Applications |
| macOS (Intel) | `ferrite-macos-x64.tar.gz` | Alternative - contains Ferrite.app bundle |

<details>
<summary><strong>Windows Installation</strong></summary>

#### MSI Installer (Recommended)

Download `ferrite-windows-x64.msi` and run it. This will:
- Install Ferrite to `C:\Program Files\Ferrite`
- Add Start Menu shortcut with icon
- Enable easy uninstall via Windows Settings
- Store settings in `%APPDATA%\ferrite\`

#### Portable (ZIP)

Download `ferrite-portable-windows-x64.zip` and extract anywhere. The zip includes:
- `ferrite.exe` - the application
- `portable/` - folder for all your settings
- `README.txt` - quick start guide

**True portable mode:** All configuration, sessions, and data are stored in the `portable` folder next to the executable. Nothing is written to `%APPDATA%` or the Windows registry. Perfect for USB drives or trying Ferrite without installation.

</details>

<details>
<summary><strong>Linux Installation</strong></summary>

#### Debian/Ubuntu/Mint (.deb)

```bash
# Download the .deb file, then install with:
sudo apt install ./ferrite-editor_amd64.deb

# Or using dpkg:
sudo dpkg -i ferrite-editor_amd64.deb
```

#### Fedora/RHEL/CentOS (.rpm)

```bash
# Download the .rpm file, then install with:
sudo dnf install ./ferrite-editor.x86_64.rpm

# Or using rpm:
sudo rpm -i ferrite-editor.x86_64.rpm
```

Both .deb and .rpm packages will:
- Install Ferrite to `/usr/bin/ferrite`
- Add desktop entry (appears in your app menu)
- Register file associations for `.md`, `.json`, `.yaml`, `.toml` files
- Install icons for the system

#### Arch Linux (AUR)

[![Ferrite on AUR](https://img.shields.io/aur/version/ferrite?label=ferrite)](https://aur.archlinux.org/packages/ferrite/)
[![Ferrite-bin on AUR](https://img.shields.io/aur/version/ferrite-bin?label=ferrite-bin)](https://aur.archlinux.org/packages/ferrite-bin/)

Ferrite is available on the [AUR](https://wiki.archlinux.org/index.php/Arch_User_Repository):
- [Ferrite](https://aur.archlinux.org/packages/ferrite/) (release package)
- [Ferrite-bin](https://aur.archlinux.org/packages/ferrite-bin/) (binary package)

```console
# Release package
yay -Sy ferrite

# Binary package
yay -Sy ferrite-bin
```

#### Nix / NixOS (official flake)

```bash
# Run Ferrite directly from GitHub (no install)
nix run github:OlaProeis/Ferrite

# Drop into a shell that exposes ferrite in PATH
nix shell github:OlaProeis/Ferrite
```

From a local clone:

```bash
# Build package output
nix build .#ferrite
./result/bin/ferrite

# Enter development shell (Rust toolchain + system deps)
nix develop
```

#### Other Linux (tar.gz)

```bash
tar -xzf ferrite-linux-x64.tar.gz
./ferrite
```

#### Known Issues

**Keyboard not working on Ubuntu 24.04 LTS (GNOME/Wayland)**

Keyboard input does not work when Ferrite is launched under GNOME on Wayland (Ubuntu 24.04 LTS and similar).
As a workaround, launch Ferrite with `WAYLAND_DISPLAY` unset to force X11 mode:

```bash
env -u WAYLAND_DISPLAY ferrite
```

To make this permanent for your desktop launcher, create a user-level desktop entry override:

```bash
mkdir -p ~/.local/share/applications
cp /usr/share/applications/ferrite-editor.desktop ~/.local/share/applications/ferrite-editor.desktop
```

Then edit `~/.local/share/applications/ferrite-editor.desktop` and change the `Exec` line to:

```ini
Exec=env -u WAYLAND_DISPLAY ferrite %F
```

</details>

<details>
<summary><strong>macOS Installation</strong></summary>

#### DMG Installer (Recommended)

Download the `.dmg` for your Mac:
- **Apple Silicon** (M1/M2/M3/M4): `ferrite-macos-arm64.dmg`
- **Intel**: `ferrite-macos-x64.dmg`

Open the DMG and drag `Ferrite.app` to the **Applications** folder. Launch from Launchpad or Spotlight.

#### From tar.gz (Alternative)

```bash
# Apple Silicon
tar -xzf ferrite-macos-arm64.tar.gz
cp -R Ferrite.app /Applications/

# Intel
tar -xzf ferrite-macos-x64.tar.gz
cp -R Ferrite.app /Applications/
```

> **Gatekeeper (GitHub downloads):** Unsigned / non-notarized CI builds may be blocked on **macOS 15.x**. See **[docs/install/macos.md](docs/install/macos.md)** — quickest fix: `xattr -dr com.apple.quarantine /Applications/Ferrite.app` (adjust path). Tracked: [#130](https://github.com/OlaProeis/Ferrite/issues/130).

</details>

<details>
<summary><strong>Build from Source</strong></summary>

#### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **Platform-specific dependencies:**

**Nix users:** you can skip manual dependency installation and use `nix develop` from the repository root (see `flake.nix`).

**Windows:**
- Visual Studio Build Tools 2019+ with C++ workload

**Linux:**

```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libgtk-3-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora
sudo dnf install gcc pkg-config gtk3-devel libxcb-devel

# Arch
sudo pacman -S base-devel pkg-config gtk3 libxcb
```

**macOS:**

```bash
xcode-select --install
```

#### Build

```bash
# Clone the repository
git clone https://github.com/OlaProeis/Ferrite.git
cd Ferrite

# Build release version (optimized)
cargo build --release

# The binary will be at:
# Windows: target/release/ferrite.exe
# Linux/macOS: target/release/ferrite

# macOS: Create .app bundle (optional)
cargo install cargo-bundle
cargo bundle --release
# Bundle will be at: target/release/bundle/osx/Ferrite.app
```

> **macOS "Open With" Limitation:** The app bundle includes file type associations, so Ferrite appears in Finder's "Open With" menu. However, opening files this way (or by dragging files onto the app icon) is not yet supported due to [eframe/winit limitations](https://github.com/rust-windowing/winit/issues/1751). **Workaround:** Open files via Terminal: `open -a Ferrite path/to/file.md` or use File > Open within the app.

> **Development Builds:** Building from the `main` branch gives you the latest features before they're officially released. These builds are untested and may contain bugs. For stable versions, download from [GitHub Releases](https://github.com/OlaProeis/Ferrite/releases).

</details>

## Usage

```bash
# Open a file
ferrite path/to/file.md

# Open a folder as workspace
ferrite path/to/folder/
```

<details>
<summary><strong>More CLI options</strong></summary>

```bash
# Run from source
cargo run --release

# Or run the binary directly
./target/release/ferrite

# Open multiple files as tabs
./target/release/ferrite file1.md file2.md

# Show version
./target/release/ferrite --version

# Show help
./target/release/ferrite --help
```

See [docs/cli.md](docs/cli.md) for full CLI documentation.

</details>

### View Modes

Ferrite supports three view modes for Markdown files:

- **Raw** - Plain text editing with syntax highlighting
- **Rendered** - WYSIWYG editing with rendered markdown
- **Split** - Side-by-side raw editor and live preview

Toggle between modes using the toolbar buttons or keyboard shortcuts.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save file |
| `Ctrl+W` | Close tab |
| `Ctrl+P` | Quick file switcher |
| `Ctrl+F` | Find |
| `Ctrl+G` | Go to line |
| `Ctrl+,` | Open settings |

<details>
<summary><strong>All Keyboard Shortcuts</strong></summary>

### File Operations

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save file |
| `Ctrl+Shift+S` | Save as |
| `Ctrl+W` | Close tab |

### Navigation

| Shortcut | Action |
|----------|--------|
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+P` | Quick file switcher (workspace) |
| `Ctrl+Shift+F` | Search in files (workspace) |

### Editing

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find and replace |
| `Ctrl+G` | Go to line |
| `Ctrl+Shift+D` | Duplicate line |
| `Alt+↑` | Move line up |
| `Alt+↓` | Move line down |
| `Ctrl+B` | Bold |
| `Ctrl+I` | Italic |
| `Ctrl+K` | Insert link |

### View

| Shortcut | Action |
|----------|--------|
| `F11` | Toggle fullscreen |
| `Ctrl+,` | Open settings |
| `Ctrl+Shift+[` | Fold all |
| `Ctrl+Shift+]` | Unfold all |

### Terminal Workspace

Terminal shortcuts are **context-aware**; they work when the terminal panel is focused.

| Shortcut | Action |
|----------|--------|
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle through terminal tabs |
| `Ctrl+1-9` | Switch to specific terminal tab |
| `Ctrl+Arrow Keys` | Move focus between split panes |
| `Ctrl+Shift+M` | Toggle **Maximize Pane** (Zoom) |
| `Ctrl+L` | Clear terminal screen |
| `Ctrl+Shift+C` | Copy selection / screen |
| `Ctrl+Shift+V` | Paste to terminal |
| `Ctrl+W` / `Ctrl+F4` | Close focused pane (auto-collapses splits) |
| `Double-click Tab` | Rename terminal |

</details>

## Configuration

Access settings via `Ctrl+,` or the gear icon. Configure appearance, editor behavior, and file handling.

<details>
<summary><strong>Configuration details</strong></summary>

Settings are stored in platform-specific locations:

- **Windows:** `%APPDATA%\ferrite\`
- **Windows Portable:** `portable\` folder next to `ferrite.exe`
- **Linux:** `~/.config/ferrite/`
- **macOS:** `~/Library/Application Support/ferrite/`

**Portable Mode (Windows):** If a `portable` folder exists next to the executable, Ferrite automatically uses it for all configuration instead of `%APPDATA%`. This makes Ferrite fully self-contained - perfect for USB drives.

Workspace settings are stored in `.ferrite/` within the workspace folder.

### Settings Panel

- **Appearance:** Theme, font family, font size, default view mode
- **Editor:** Word wrap, line numbers, minimap, bracket matching, code folding, syntax highlighting, auto-close brackets, line width
- **Files:** Auto-save, recent files history

</details>

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned features and known issues.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Help Translate

Ferrite is being translated into multiple languages with help from the community.

[![Translation Status](https://hosted.weblate.org/widget/ferrite/ferrite-ui/multi-auto.svg)](https://hosted.weblate.org/engage/ferrite/)

**[Help translate Ferrite on Weblate](https://hosted.weblate.org/engage/ferrite/)** - no coding required!

<details>
<summary><strong>Quick Start for Contributors</strong></summary>

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/Ferrite.git
cd Ferrite

# Create a feature branch
git checkout -b feature/your-feature

# Make changes, then verify
cargo fmt
cargo clippy
cargo test
cargo build

# Optional (Nix users): validate flake outputs
nix flake check

# Commit and push
git commit -m "feat: your feature description"
git push origin feature/your-feature
```

</details>

## Tech Stack

Built with Rust 1.70+, egui/eframe for GUI, comrak for Markdown parsing, ropey for rope-based text editing, and syntect for syntax highlighting.

<details>
<summary><strong>Full tech stack</strong></summary>

| Component | Technology |
|-----------|------------|
| Language | Rust 1.70+ |
| GUI Framework | egui 0.28 + eframe 0.28 |
| Text Buffer | ropey 1.6 (rope data structure) |
| Markdown Parser | comrak 0.22 |
| Syntax Highlighting | syntect 5.1 + two-face 0.5 |
| Git Integration | git2 0.19 |
| Terminal PTY | portable-pty 0.8 |
| Terminal ANSI Parser | vte 0.13 |
| Encoding Detection | encoding_rs 0.8 + chardetng 0.1 |
| Internationalization | rust-i18n 3 + sys-locale 0.3 |
| CLI Parsing | clap 4 |
| File Dialogs | rfd 0.14 |
| Clipboard | arboard 3 |
| File Watching | notify 6 |
| Fuzzy Matching | fuzzy-matcher 0.3 |
| HTTP Client | ureq 2 (update checker) |
| Hashing | blake3 1.5 (Mermaid caching) |
| Date/Time | chrono 0.4 |
| CSV Parsing | csv 1.3 |
| Color Palette | palette 0.7 |
| Font Enumeration | font-kit 0.14 |
| Allocator (Windows) | mimalloc 0.1 |
| Allocator (Unix) | tikv-jemallocator 0.6 |

</details>

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

### Libraries
- [egui](https://github.com/emilk/egui) and eframe - Immediate mode GUI and native window integration
- [ropey](https://github.com/cessen/ropey) - Rope text buffer for large-file editing
- [comrak](https://github.com/kivikakk/comrak) - CommonMark + GFM compatible Markdown parser
- [syntect](https://github.com/trishume/syntect) and [two-face](https://github.com/CosmicHorrorDev/two-face) - Syntax highlighting and extra language definitions
- [harfrust](https://github.com/harfbuzz/harfrust) - OpenType text shaping for complex scripts
- [git2](https://github.com/rust-lang/git2-rs) - libgit2 bindings for Rust
- [portable-pty](https://github.com/wez/wezterm) and [vte](https://github.com/alacritty/vte) - Integrated terminal (PTY and ANSI parsing)
- [image](https://github.com/image-rs/image) - Raster image decoding (markdown preview and image viewer)
- [hayro](https://github.com/LaurenzV/hayro) - Pure Rust PDF rasterization (PDF viewer tabs)
- [rust-i18n](https://github.com/longbridge/rust-i18n) - Internationalization
- [Inter](https://rsms.me/inter/) and [JetBrains Mono](https://www.jetbrains.com/lp/mono/) fonts

### Development Tools
- [Claude](https://anthropic.com) (Anthropic) - AI assistant that wrote the code
- [Cursor](https://cursor.com) - AI-powered code editor
- [Task Master](https://github.com/eyaltoledano/claude-task-master) - AI task management for development workflows

### Contributors
- [@Star-sumi](https://github.com/Star-sumi) — Windows single-instance foreground activation ([#147](https://github.com/OlaProeis/Ferrite/issues/147), [PR #148](https://github.com/OlaProeis/Ferrite/pull/148)); tab strip context menu & path tooltip ([PR #150](https://github.com/OlaProeis/Ferrite/pull/150)); outline tree visual polish ([PR #151](https://github.com/OlaProeis/Ferrite/pull/151)); test build baseline fixes ([PR #152](https://github.com/OlaProeis/Ferrite/pull/152))
- [@moabtools](https://github.com/moabtools) — Ctrl+Home / Ctrl+End document navigation in Rendered view ([PR #137](https://github.com/OlaProeis/Ferrite/pull/137))
- [@liuxiaopai-ai](https://github.com/liuxiaopai-ai) — Nix/NixOS flake support for reproducible builds and dev shells ([PR #92](https://github.com/OlaProeis/Ferrite/pull/92))
- [@blizzard007dev](https://github.com/blizzard007dev) — Welcome page for first-launch configuration ([PR #80](https://github.com/OlaProeis/Ferrite/pull/80))
- [@wolverin0](https://github.com/wolverin0) — Integrated Terminal Workspace & Productivity Hub ([PR #74](https://github.com/OlaProeis/Ferrite/pull/74))
- [@abcd-ca](https://github.com/abcd-ca) — Delete Line, Move Line, macOS file associations ([PR #29](https://github.com/OlaProeis/Ferrite/pull/29), [#30](https://github.com/OlaProeis/Ferrite/pull/30))
- [@SteelCrab](https://github.com/SteelCrab) — CJK character rendering ([PR #8](https://github.com/OlaProeis/Ferrite/pull/8))

## Sponsors

<table>
  <tr>
    <td>
      <a href="https://signpath.io/?utm_source=foundation&utm_medium=github&utm_campaign=ferrite" target="_blank"><img src="https://signpath.org/assets/favicon-50x50.png" alt="SignPath" width="50" height="50" /></a>
    </td>
    <td>
      Free code signing on Windows provided by <a href="https://signpath.io/?utm_source=foundation&utm_medium=github&utm_campaign=ferrite">SignPath.io</a>, certificate by <a href="https://signpath.org/?utm_source=foundation&utm_medium=github&utm_campaign=ferrite">SignPath Foundation</a>
    </td>
  </tr>
  <tr>
    <td>
      <a href="https://weblate.org/" target="_blank"><img src="https://weblate.org/static/img/logo.svg" alt="Weblate" width="50" height="50" /></a>
    </td>
    <td>
      Hosted translations provided by <a href="https://weblate.org/">Weblate</a> on <a href="https://hosted.weblate.org/">Hosted Weblate</a> (gratis libre project plan)
    </td>
  </tr>
</table>

---

<sub>If you find Ferrite useful, consider [supporting its development](https://github.com/sponsors/OlaProeis).</sub>
