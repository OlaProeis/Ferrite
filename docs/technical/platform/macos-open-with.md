# macOS Finder Open With path reception (#154)

Best-effort reception and tab routing for file paths when the user opens a document via Finder double-click or **Open With** Ferrite.

## Problem

On macOS, Finder does not pass the file path in `argv`. AppKit delivers a `kAEOpenDocuments` Apple Event, normally forwarded to `application:openURLs:` on the app delegate. Without a handler, the app launches empty and Finder may show “does not support this file type.”

## Stack constraint

Ferrite uses **eframe 0.34 / winit 0.30**. That winit release owns `NSApplicationDelegate` and does **not** expose `Event::Opened` or a safe public delegate hook. Setting a custom delegate before the event loop **crashes** startup.

Upstream: [winit#1751](https://github.com/rust-windowing/winit/issues/1751), [winit#3758](https://github.com/rust-windowing/winit/pull/3758) (user delegates from winit 0.31+). No notarization required for this path.

## Path reception (`src/platform/macos.rs`)

No custom Objective-C class and no extra crates — C FFI into CoreServices, CoreFoundation, and libobjc:

| Phase | Mechanism |
|-------|-----------|
| **Cold launch** | Observe `NSApplicationWillFinishLaunchingNotification`; `class_addMethod` injects `application:openURLs:` onto winit’s existing delegate class so AppKit’s own open-documents handler delivers URLs and replies success. |
| **Warm open** | After eframe creation, install a Carbon `kAEOpenDocuments` (`aevt`/`odoc`) handler that replaces AppKit’s dispatch entry. |

Paths are queued in a process-wide `Mutex<Vec<PathBuf>>`. Delivery is logged at info level. `bind_egui_context` stashes `egui::Context` and calls `request_repaint` on delivery.

### Public API

| Function | Role |
|----------|------|
| `init_app_delegate()` | Early, safe: register WillFinishLaunching observer only (called from `main` before the event loop). |
| `get_open_file_paths()` / `take_opened_files()` | Drain the queue. |
| `bind_egui_context(ctx)` | Stash `egui::Context` for warm-open `request_repaint` + install AE handler (eframe `CreationContext`). |

Non-macOS stubs in `src/platform/mod.rs` return empty / no-op.

## Tab routing

| Scenario | Wiring |
|----------|--------|
| **Cold launch** | `main` → early drain (usually empty) → eframe `CreationContext` drains again → merge into `initial_paths` → `open_initial_paths` (same as CLI). |
| **Warm open (app running)** | Carbon AE / injected `openURLs:` → queue → update loop `handle_macos_open_paths` → `open_os_paths_in_focused_window` (files → tabs, dirs → workspace, focus + toasts). |
| **Secondary process** | macOS may spawn a second Ferrite; single-instance TCP forwards paths → `handle_instance_paths` (unchanged). |

Shared opening logic lives in `open_os_paths_in_focused_window` (`src/app/file_ops.rs`), also used by single-instance IPC.

CLI `ferrite /path/to/file.md` is unchanged.

## Limitations / escape hatch

- **winit 0.30 workaround** — Open With relies on delegate-method injection and a Carbon AE handler rather than a first-class winit API. Edge cases may still fail (e.g. no delegate at WillFinishLaunching).
- **If Open With fails** — Use CLI or Terminal:
  ```bash
  open -a Ferrite /path/to/file.md
  # or
  /Applications/Ferrite.app/Contents/MacOS/ferrite /path/to/file.md
  ```
- **AppleScript** (optional):
  ```applescript
  tell application "Ferrite" to open POSIX file "/path/to/file.md"
  ```
- File-type association / bundle document types: [`macos-markdown-file-association.md`](macos-markdown-file-association.md). Gatekeeper/notarization: [#130](https://github.com/OlaProeis/Ferrite/issues/130), out of scope.

User-facing install notes: [`docs/install/macos.md`](../../install/macos.md) § Opening files.

## Tests

macOS-only unit tests in `src/platform/macos.rs`:

- `file_url_to_path` / percent-decode (spaces, Unicode, `localhost`, non-`file://` rejected)
- Queue drain via `take_opened_files`

CI on Windows/Linux exercises the stubs only. Manual: Open With / double-click (cold and warm) and confirm log `macOS Open With / Apple Event paths received` plus tab open.

## Related

- Issue [#154](https://github.com/OlaProeis/Ferrite/issues/154)
- [`macos-markdown-file-association.md`](macos-markdown-file-association.md)
- [`single-instance.md`](single-instance.md)
- [`multi-window-file-routing.md`](multi-window-file-routing.md)
- [`docs/install/macos.md`](../../install/macos.md)
