# External File Open Fallback (#142)

## Overview

When a user opens a file Ferrite cannot edit as text (binary data, Office documents, archives), the app delegates to the OS default application via `open::that` and shows a brief toast. Blocking error dialogs are never used for this path.

In-app types are unchanged: markdown, JSON/YAML/TOML, CSV/TSV, images, PDF, and text/code that passes binary detection still open in Ferrite tabs.

## Classification

`OpenResult` in `src/state.rs`:

| Variant | Meaning |
|---------|---------|
| `OpenedTab(usize)` | File opened or focused in an in-app tab (strip index) |
| `OpenedExternal` | File should be opened by the OS default app |
| `Failed(io::Error)` | Read or I/O failure |

Delegation triggers when either:

1. **Extension denylist** — `is_external_open_extension()` matches before reading bytes (`docx`, `xlsx`, `pptx`, archives, executables, etc.).
2. **Binary heuristics** — `is_binary_content()` on file bytes (null bytes or high ratio of non-printable control characters).

Images and PDFs are intercepted before binary detection and open in dedicated viewer tabs.

## Launch and toasts

`complete_external_file_open()` in `state.rs` calls `open::that(path)` and shows:

- Success: `notification.opened_in_default_app` (2 s)
- Failure: `notification.opened_external_failed` (4 s)

`FerriteApp::finalize_open_result()` in `src/app/file_ops.rs` resolves `OpenedExternal` by calling that helper. Callers should use `open_file_smart_in_window()` (which finalizes internally) rather than raw `open_file_with_focus()` unless they finalize themselves.

## Entry points

The same policy applies wherever files are opened from the UI:

| Source | Handler |
|--------|---------|
| File tree click | `app/mod.rs` |
| Quick switcher (Ctrl+P) | `central_panel.rs` |
| Search-in-files navigation | `file_ops.rs` → `handle_search_navigation` |
| Wikilink click | `file_ops.rs` → `navigate_wikilink` |
| Drag-and-drop | `file_ops.rs` → `handle_dropped_files` |
| File dialog / CLI / secondary instance | `file_ops.rs` |
| Recent files popup | `status_bar.rs` |
| Backlinks panel | `app/mod.rs` |

Hard failures use `show_toast` with `error.open_file_failed`, not blocking `show_error`.

## Large files (background load)

Files ≥ 5 MB use a loading tab while a background thread reads the file. If the loaded bytes classify as external (`should_open_externally`), the loader sends `FileLoadMsg::OpenExternal { tab_id, path }`. `poll_file_load_messages` closes the loading tab and runs `complete_external_file_open`.

## Out of scope

- Context menu **Open with system default** (Tier C)
- Opening non-markdown wikilink targets that resolve to denylisted paths still delegate externally when opened

## Tests

Unit tests in `state.rs`:

- `test_external_open_extension_denylist`
- `test_should_open_externally_classification`
- `test_open_file_with_focus_binary_returns_external`
- `test_open_file_with_focus_denylisted_extension_returns_external`

Manual: on Windows and one Unix OS, `.docx`/`.exe`/`.zip` in the file tree open externally with a toast; `.md`/`.json`/`.png`/`.rs` open in Ferrite; missing associated app shows error toast.
