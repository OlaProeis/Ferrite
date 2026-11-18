# GitHub Issue Reply Drafts - v0.2.7 Release

This document contains draft replies for open issues tagged with the 0.2.7 milestone. The v0.2.7 release just shipped on March 11, 2026.

---

## Issue #1 - [Feature] Wikilinks and backrefs
**URL:** https://github.com/OlaProeis/Ferrite/issues/1
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @khimaros! Great news — **wikilinks and backlinks are now fully implemented in v0.2.7** which just released today!

## What's Implemented

### Wikilinks Support
- `[[target]]` and `[[target|display]]` syntax
- Relative path resolution (same-folder-first tie-breaker for ambiguous names)
- Spaces in filenames supported
- Click-to-navigate in rendered/split view
- Broken link indicators for missing targets

### Backlinks Panel
- New "Backlinks" tab in the side panel (next to Outline)
- Shows all files in the workspace that link to the current document
- Graph-based indexing for large workspaces (>50 files)
- Click any backlink to navigate to the referencing file
- Detects both `[[wikilinks]]` and standard `[markdown](links)` syntax

## How to Use
1. Create links with `[[File Name]]` or `[[File Name|Display Text]]`
2. Click the link in rendered/split view to navigate
3. Open the Backlinks tab to see what links to your current file

The backlinks panel is available via the side panel toggle strip on the right edge of the editor.

Thanks for the feature request — this was one of our most requested features! Let us know if you have any feedback on the implementation.
```

---

## Issue #36 - [Feature] Autoupdate
**URL:** https://github.com/OlaProeis/Ferrite/issues/36
**Status:** ✅ PARTIALLY COMPLETED (Manual Check) in v0.2.7

### Suggested Reply:

```markdown
Hi @GeorgTollefsen! Thanks for the feature request. **v0.2.7 (released today) includes a manual "Check for Updates" feature** as a first step toward full auto-update.

## What's Implemented

- **Settings → About → Check for Updates button**
- Queries GitHub Releases API when clicked
- Shows inline result:
  - "You're up to date" (current version)
  - "Update available" with download link (newer version)
  - Error message if check fails

## Design Philosophy

We opted for **manual trigger only** (not automatic background checking) to maintain our offline-first philosophy. No unexpected network calls, no background processes — you decide when to check.

## Full Auto-Update

True automatic updates (download + install) are planned for a future release but require:
- Code signing on all platforms (we have Windows via SignPath, working on macOS)
- Update server infrastructure
- Platform-specific update mechanisms

Would the current manual check satisfy your immediate needs? We'd love feedback on the workflow.
```

---

## Issue #76 - [Bug] Launch failure + Chinese rendering issue + Wrong config save path
**URL:** https://github.com/OlaProeis/Ferrite/issues/76
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @CodeCatMeow! Thank you for this detailed bug report. **All three issues have been addressed in v0.2.7** which released today.

## Issue 1: Launch Failure with Non-Auto CJK Preference
**Fixed.** The crash when setting CJK font preference to non-"Auto" was caused by missing graceful fallback when the specified font couldn't be loaded. The system now:
- Preloads the preferred CJK font at startup when set to explicit (non-Auto)
- Gracefully handles missing fonts (returns None instead of panicking)

## Issue 2: Chinese Rendering Failure on Startup  
**Fixed.** Chinese text now renders correctly on startup when:
- System locale is Chinese, OR
- CJK font preference is set to an explicit option

We preload the appropriate single CJK font (~15-20MB) at startup instead of loading all 4 fonts (~80MB). This maintains our memory optimization while ensuring correct rendering.

## Issue 3: Portable Version Config Path
**Clarified.** The portable version behavior is by design:
- If `portable/` folder exists next to `ferrite.exe` → config saved there
- Otherwise → config saved to user directory (standard behavior)

This allows the same binary to work as both portable and installed. Creating the `portable/` folder is the intended way to activate portable mode. We've updated the README to make this clearer.

## New in v0.2.7: FERRITE_DATA_DIR
For external launchers (like PortableApps.com), you can now set the `FERRITE_DATA_DIR` environment variable to explicitly redirect config storage:
```
FERRITE_DATA_DIR=Data\settings\ ferrite.exe
```

Please try v0.2.7 and let us know if these issues are resolved for you!
```

---

## Issue #82 - [Bug] Preview: Lines not auto-wrapping after first list item
**URL:** https://github.com/OlaProeis/Ferrite/issues/82
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @alexpGH! Thank you for the excellent bug report with screenshots. **This has been fixed in v0.2.7** which just released today.

## Root Cause

All list item `TextEdit` widgets were using `singleline` mode which fundamentally cannot wrap text — this is an egui limitation. When content exceeded the pane width, it extended beyond the visible area.

## The Fix

Changed list items to use `multiline` mode with:
- Custom `LayoutJob` layouter (matching paragraph wrapping pattern)
- `desired_rows(1)` to maintain single-row appearance
- Newline stripping to prevent Enter from inserting literal line breaks

## Bonus Fix

We also fixed a related issue where typing `- ` after a paragraph caused the paragraph to render as a heading. This was because comrak interpreted a single `-` + whitespace as a setext heading underline. Added `fix_false_setext_headings()` post-processing to convert these back to Paragraph + List(Item).

Both fixes are in v0.2.7 — please try it out and let us know if list wrapping works correctly for you now!
```

---

## Issue #85 - [Feature] 'Ctrl-Mouse Wheel' to zoom
**URL:** https://github.com/OlaProeis/Ferrite/issues/85
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @FrankFischer! **Ctrl+Mouse Wheel zoom is now implemented in v0.2.7** which just released today!

## What's Implemented

- Ctrl+Scroll Wheel now zooms in/out (same as Ctrl++/Ctrl+-)
- Added `ZoomIn`, `ZoomOut`, and `ResetZoom` as proper ShortcutCommand variants
- Keyboard shortcuts preserved (Ctrl++ / Ctrl+- / Ctrl+0)

The zoom level is applied globally via egui's `gui_zoom` system, affecting all UI elements consistently.

Thanks for the feature request — enjoy the new zoom workflow!
```

---

## Issue #91 - [Bug] Windows Chinese Input Bug
**URL:** https://github.com/OlaProeis/Ferrite/issues/91
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @JekYUlll! Thank you for reporting this IME issue. **This has been fixed in v0.2.7** which just released today.

## Root Cause

egui forwards raw `Key::Backspace` events alongside IME preedit updates. During IME composition, the editor was processing both:
1. The IME preedit update (correctly modifying the composition string)
2. The raw Backspace key (incorrectly deleting committed editor text)

## The Fix

We now suppress all `Key` and `Text` events while `ime_enabled` is true (active composition session). The editor only processes:
- IME preedit string updates
- IME commit events (final character insertion)

This affects Microsoft Pinyin, Xiaolanghao/Rime, and all other IME input methods on Windows, macOS, and Linux.

## Verification

Tested with your example scenario:
```
一二三四五六七八九
```
Inputting `shi` → pressing Backspace now correctly changes the IME preedit to `sh` without deleting `九`.

Please try v0.2.7 and let us know if the IME backspace behavior is resolved!
```

---

## Issue #94 - [Feature] Visible frontmatter editor
**URL:** https://github.com/OlaProeis/Ferrite/issues/94
**Status:** ✅ BASIC VERSION COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @xingwangzhe! **Basic frontmatter editing is now available in v0.2.7** which just released today!

## What's Implemented

### Visual Frontmatter Panel
- New "FM" tab in the outline panel (right side)
- Form-based YAML frontmatter editing as key-value pairs
- Bidirectional sync with source (edits in panel update source, edits in source update panel)

### Field Types Supported
- **String** — standard text input
- **Date** — date picker widget
- **List (Tags)** — chip-based UI for tag arrays

## What's NOT Yet Implemented (Future Releases)

Your feature requests for advanced features are noted:
- Project-wide tag autocomplete (from all files' frontmatter)
- SSG-specific field types (slug, permalink, id with random generation)
- Frontmatter templates for new files
- Support for Hexo/Hugo/Jekyll/Astro/Zola specific conventions

These are planned for future releases as we build out the frontmatter system.

## How to Use
1. Open any markdown file with YAML frontmatter (or add `---\n---` to create it)
2. Click the "FM" tab in the outline panel
3. Edit fields visually — changes sync to the markdown source automatically

Let us know if the current implementation helps your workflow and what additional features you'd prioritize!
```

---

## Issue #95 - todolist checkbox not working as expected
**URL:** https://github.com/OlaProeis/Ferrite/issues/95
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @ykfq! **Task list checkbox rendering has been fixed in v0.2.7** which just released today.

## What Was Fixed

- Task list checkboxes now render as proper UI elements instead of ASCII `[ ]` / `[x]`
- Bullet point markers are now suppressed for task list items (no double markers)
- Click-to-toggle in rendered view with source sync

## Comparison

| Before | After |
|--------|-------|
| `[ ]` / `[x]` ASCII text | ☐ / ☑ proper checkbox UI |
| Bullet + checkbox | Checkbox only |

## Interactive Toggle

In rendered/split view, clicking a checkbox will:
1. Toggle the checkbox state visually
2. Update the source markdown (`[ ]` ↔ `[x]`)

Try v0.2.7 and let us know if the task list rendering looks correct now!
```

---

## Issue #96 - [Bug] Open folder does not work unless a file from that folder is already open
**URL:** https://github.com/OlaProeis/Ferrite/issues/96
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @micahchoo! **This has been fixed in v0.2.7** which just released today.

## Root Cause

The Flatpak file dialog was failing when no "recent directory" was available (fresh install or first launch). The error occurred because the xdg-desktop-portal couldn't determine a sane default folder.

## The Fix

- All file dialogs now fall back to `$HOME` when no recent directory is available
- Added Flatpak environment detection
- User-friendly error messages if the portal still fails
- For Linux systems without proper portal setup (Hyprland, Sway, etc.), we now show a helpful error modal with distro-specific install instructions

## For Flatpak Users

If you're using the Flatpak version, ensure you have the file portal access:
```bash
flatpak override --user --filesystem=home com.ferritemd.Ferrite
```

Please try v0.2.7 and let us know if "Open Folder" works correctly now!
```

---

## Issue #97 - [Bug] export issue with hyprland
**URL:** https://github.com/OlaProeis/Ferrite/issues/97
**Status:** ✅ COMPLETED in v0.2.7

### Suggested Reply:

```markdown
Hi @superiums! **This has been addressed in v0.2.7** which just released today.

## Root Cause

Hyprland (and other minimal window managers like Sway, i3) often don't have the full xdg-desktop-portal infrastructure set up. The file dialog fails with:
```
pick_folder error No such file or directory (os error 2)
```

## The Fix

We've added comprehensive Linux file dialog error handling:

1. **Portal failure detection** — Detects when xdg-desktop-portal fails
2. **Helpful error modal** — Shows instead of silent failure
3. **Distro-specific instructions** — Commands for pacman, apt, dnf based on detected distro
4. **"Copy Install Command" button** — One-click to copy the correct install command

## For Hyprland Users

If you see the error dialog, you'll likely need to install the GTK portal:
```bash
# Arch/Artix
sudo pacman -S xdg-desktop-portal-gtk

# Or if you prefer the KDE portal
sudo pacman -S xdg-desktop-portal-kde
```

Also ensure the portal service is running:
```bash
systemctl --user enable --now xdg-desktop-portal
```

The error handling is much more informative now — please try v0.2.7 and let us know if the dialog appears and helps resolve the issue!
```

---

## Issue #98 - [Question] How to enable syntax highlighting?
**URL:** https://github.com/OlaProeis/Ferrite/issues/98
**Status:** ❓ CLARIFICATION NEEDED

### Suggested Reply:

```markdown
Hi @KaKi87! Thanks for the question — let me clarify how syntax highlighting works in Ferrite.

## Two Different Highlighting Systems

Ferrite has **two separate highlighting features** that work differently:

### 1. Editor Syntax Highlighting (the setting you found)
- **Settings → Editor → Syntax Highlighting**
- This highlights **code blocks** in the raw editor (the left pane in split view)
- Uses language-specific colors for programming languages in fenced code blocks (```python, ```rust, etc.)
- Does NOT highlight markdown formatting (bold, italic) in the editor

### 2. Rendered Markdown Styling
- The **right pane** (Rendered/Split view) shows formatted markdown
- Bold text (`**text**`) appears as bold
- This is NOT syntax highlighting — it's markdown rendering

## Why Your Example Shows Plain Text

In your example:
```markdown
Hello **World**!
```

- **In Raw view:** Shows as plain text with asterisks: `Hello **World**!`
- **In Rendered view:** Shows as: Hello **World**! (with "World" in bold)

The screenshots you're comparing against likely show the **Rendered** view, not the Raw editor.

## How to Switch Views

Click the view mode segment at the top:
- **Raw** — Plain text editing with code syntax highlighting
- **Split** — Side-by-side raw + rendered
- **Rendered** — Full preview with formatted text

Or use shortcuts:
- Ctrl+R — Raw
- Ctrl+S — Split  
- Ctrl+V — Rendered

Does this clarify the behavior? If you're still not seeing formatting in Rendered view, that would be a bug — let us know!
```

---

## Issue #99 - [Feature] `.dmg`/`.app`/`.pkg`/Brew for Mac
**URL:** https://github.com/OlaProeis/Ferrite/issues/99
**Status:** ✅ .app BUNDLE + HOMEBREW CASK COMPLETED in v0.2.7 — Reply posted 2026-03-13

### Suggested Reply:

```markdown
Hi @KaKi87! **macOS `.app` bundles and DMGs now ship in v0.2.7.**

### What's Available Now

- `ferrite-macos-arm64.dmg` — Apple Silicon (M1/M2/M3/M4)
- `ferrite-macos-x64.dmg` — Intel Macs

Both DMGs contain a proper `Ferrite.app` bundle with Info.plist, icons, and file type associations (.md, .json, .yaml, .toml, .txt).

### Homebrew Cask

We've also set up a Homebrew tap:

```bash
brew tap olaproeis/ferrite
brew install --cask ferrite
```

This automatically handles macOS Gatekeeper (strips quarantine attribute), so Ferrite launches with zero warnings.

### Not Yet Available
- Submission to the official `homebrew-cask` repository (requires broader adoption first)
- `.pkg` installer (may consider in future)

Let us know if the DMG or Homebrew install works for you!
```

---

## Issue #63 - [Bug] Crash on startup after changing CJK font settings
**URL:** https://github.com/OlaProeis/Ferrite/issues/63
**Status:** ✅ COMPLETED in v0.2.7 (was tagged 0.2.6.1)

### Suggested Reply:

```markdown
Hi @yadokariinthemarsh! **This crash has been fixed in v0.2.7** which just released today.

## Root Cause

The crash (ExceptionCode: c0000409 - STATUS_STACK_BUFFER_OVERRUN) occurred when:
1. CJK font preference was set to a non-"Auto" value (e.g., Japanese)
2. The specified font couldn't be loaded or wasn't available
3. The code panicked instead of gracefully handling the missing font

## The Fix

**v0.2.7 includes several CJK-related fixes that address this:**

1. **Graceful font loading** — Fonts now return `None` instead of panicking when unavailable
2. **Explicit preference preloading** — When CJK preference is set to non-Auto, the appropriate single font is preloaded at startup (~15-20MB instead of all 4 fonts ~80MB)
3. **Startup ordering fix** — Lock file creation is now done after session state loading, preventing false crash detection

## What You'll See Now

- **If the preferred font loads:** Chinese/Japanese renders correctly on startup
- **If the font is unavailable:** Falls back gracefully (may show tofu □ in settings until a CJK document opens, but won't crash)
- **The "tofu in settings UI" issue:** This is expected when no CJK documents are open — fonts load on-demand. Opening any CJK document will trigger font loading.

## To Update

1. Uninstall the old version (which will clear the problematic config)
2. Install v0.2.7 from: https://github.com/OlaProeis/Ferrite/releases/tag/v0.2.7
3. Your CJK preference should now work without crashes

Please try v0.2.7 and let us know if the crash is resolved!
```

---

## Additional Open Issues to Address (Not 0.2.7 tagged)

### Issue #93 - [Bug] [macOS] Release binary blocked by Gatekeeper
**Status:** ✅ FIXED in v0.2.7 — Reply posted 2026-03-13

### Suggested Reply:
```markdown
Hey @sfrankiel — thanks for the detailed report!

**Good news:** As of v0.2.7, the release now ships proper `.app` bundles inside DMGs instead of the raw binary you encountered in v0.2.6.1. This resolves the hard Gatekeeper block.

**However**, because Ferrite is not notarized (we don't have an Apple Developer Program license — $99/year), Gatekeeper will still show a warning dialog on first launch. Here are the ways to handle it:

### Option 1: Homebrew (Recommended — Zero Warnings)

We've set up a Homebrew tap with a Cask that automatically strips the quarantine attribute:

```bash
brew tap olaproeis/ferrite
brew install --cask ferrite
```

This installs `Ferrite.app` to `/Applications` with no Gatekeeper friction at all.

### Option 2: Manual Bypass After DMG Install

After dragging `Ferrite.app` to Applications from the DMG:

- **Right-click → Open** (may need to do this twice on macOS Sequoia)
- **Or:** System Settings → Privacy & Security → scroll down → click "Open Anyway" (appears after a blocked launch attempt)
- **Or terminal:** `xattr -cr /Applications/Ferrite.app`

### Why This Happens

Apple requires:
1. A paid Developer ID certificate ($99/year)
2. App notarization (Apple scans and approves the binary)

Without both, macOS shows warnings. The app is safe — you can [audit the source](https://github.com/OlaProeis/Ferrite) or build from source with `cargo build --release`.

We've also updated the [release notes](https://github.com/OlaProeis/Ferrite/releases/tag/v0.2.7) and README with clearer macOS instructions.
```

### Issue #72 - [Feature] Keep text selected after applying a format
**Status:** ✅ FIXED in v0.2.7

### Suggested Reply:
```markdown
Hi @Nicknag! **This has been fixed in v0.2.7** which just released today.

Format toolbar buttons (Bold, Italic, etc.) now preserve both:
1. **Selection** — The formatted text remains selected
2. **Focus** — The editor maintains focus

This allows chaining formatting operations (e.g., Bold → Italic) without reselecting text.

Thanks for the feature request — enjoy the improved formatting workflow!
```

---

## Summary Table

| Issue | Title | Status in v0.2.7 |
|-------|-------|------------------|
| #1 | Wikilinks and backrefs | ✅ Completed |
| #36 | Autoupdate | ✅ Manual check completed (auto-download planned) |
| #63 | CJK crash on startup | ✅ Completed (was 0.2.6.1) |
| #72 | Keep text selected after format | ✅ Completed |
| #76 | CJK launch/rendering issues | ✅ Completed |
| #82 | List item wrapping | ✅ Completed |
| #85 | Ctrl+Scroll zoom | ✅ Completed |
| #91 | Windows IME backspace | ✅ Completed |
| #93 | macOS Gatekeeper | ✅ Completed |
| #94 | Frontmatter editor | ✅ Basic version completed |
| #95 | Task list checkboxes | ✅ Completed |
| #96 | Open folder in Flatpak | ✅ Completed |
| #97 | Hyprland export issue | ✅ Completed |
| #98 | Syntax highlighting question | ❓ Clarification needed |
| #99 | macOS .app/dmg | ✅ .app + Homebrew Cask completed |

---

*Generated for Ferrite v0.2.7 release on March 11, 2026*
