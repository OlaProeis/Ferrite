# Ferrite v0.3.1 QA round 4 — Section C (SES-3a–3h)

Build under test: release binary at `1ad150bc140ef1665f7b5e1fa9edc1257690fb6c`. Required source markers were present: `Rendered arrow nav` (2), `take_locked_preview_arrow_presses` (3), `basis_curve_segments` (6), `close_document_window_now` (2). Initial Quick note workflow was ON. No code changes, commits, pushes, or issues were made.

| Row | Result | Observed | Log lines (quoted) | Evidence |
|---|---|---|---|---|
| SES-3a | PASS | Clean secondary closed immediately by Alt+F4; no dialog; primary remained open. | `Secondary window 1 closed`<br>`Closed document window 1`<br>`Ferrite exited with code 0 clean exit` | `shots/C-3a-before.png`, `shots/C-3a-secondary-focused.png`, `shots/C-3a-after.png` |
| SES-3b | PASS | Quick note ON. × on dirty secondary showed **Title:** `Unsaved Changes`; **body:** `Unsaved changes in 'Untitled'. Close this window anyway?`; **buttons:** `Save`, `Don't Save`, `Cancel`. The prohibited phrase was absent. Cancel left the window and `abc` open. | No close line on Cancel. 3b/3c shared a launch; subsequent 3c cleanup logged `Closed document window 1`. | `shots/C-3b-dialog.png`, `shots/C-3b-dialog-crop.png`, `shots/C-3b-cancelled3.png`; `logs/C-3b.log` |
| SES-3c | PASS | Alt+F4 showed **Title:** `Unsaved Changes`; **body:** `Unsaved changes in 'Untitled'. Close this window anyway?`; **buttons:** `Save`, `Don't Save`, `Cancel`. Choosing Don't Save closed the secondary; first window remained. | `Closed document window 1` | `shots/C-3c-dialog.png`, `shots/C-3c-after.png`; `logs/C-3c.log` |
| SES-3d | **FAIL** | Quick note ON. The same close prompt appeared. Repeated Save-button clicks did not open Save As; the prompt remained. No file was created at `files/SES-3d-abc.md`; the test note was discarded during cleanup. No Save As dialog was reached, so this was not classified as environment-blocked. | `Closed document window 1` (cleanup); process exited cleanly. No Save/Save As log line appeared. | `shots/C-3d-dialog.png`, `shots/C-3d-saveas-attempt3.png`, `shots/C-3d-after.png`; `logs/C-3d.log`; requested file absent |
| SES-3e | PASS | Quick note OFF (verified in Settings → Files). × and then Alt+F4 each showed **Title:** `Unsaved Changes`; **body:** `Unsaved changes in 'Untitled'. Close this window anyway?`; **buttons:** `Save`, `Don't Save`, `Cancel`. Cancel preserved `abc`; Don't Save closed the secondary. Prohibited phrase absent. | `Closed document window 1` | `shots/C-3e-dialog.png`, `shots/C-3e-cancelled.png`, `shots/C-3e-altf4-dialog.png`, `shots/C-3e-after.png`; `logs/C-3e.log` |
| SES-3f | PASS | Quick note ON, two windows. Closing the primary by × showed **Title:** `Unsaved Changes`; **body:** `Unsaved changes in 'Untitled'. Close this window anyway?`; **buttons:** `Save`, `Don't Save`, `Cancel`. Cancel kept both windows and the primary's `abc`. Prohibited phrase absent. | No close line on Cancel; cleanup later logged `Closed document window 0`. | `shots/C-3f-two-windows.png`, `shots/C-3f-dialog.png`, `shots/C-3f-cancelled.png`; `logs/C-3f.log` |
| SES-3g | PASS | Quick note ON. After closing all but one window, closing the last window produced no dialog and exited cleanly. Relaunch restored an Untitled note containing `abc`. No Session Recovery dialog appeared. A blank extra secondary was present at startup and was closed before the single-window test. | `Secondary window 1 closed`<br>`Closed document window 1`<br>`Ferrite exited with code 0 clean exit` (initial run and relaunch) | `shots/C-3g-typed.png`, `shots/C-3g-one-window.png`, `shots/C-3g-exit.png`, `shots/C-3g-restored.png`, `shots/C-3g-extra-window.png`; `logs/C-3g.log`, `logs/C-3g-relaunch.log` |
| SES-3h | PASS | Enabled Use system title bar in Settings → Appearance and restarted. Native × closed a clean secondary without a dialog. Native × on a dirty Untitled note showed **Title:** `Unsaved Changes`; **body:** `Unsaved changes in 'Untitled'. Close this window anyway?`; **buttons:** `Save`, `Don't Save`, `Cancel`. Cancel kept `abc`; prohibited phrase absent. Turned the option off and restarted; custom title bar was restored. | `Secondary window 1 closed`<br>`Closed document window 1`<br>Cleanup: `Closed document window 2` | `shots/C-3h-native-before.png`, `shots/C-3h-3a-secondary.png`, `shots/C-3h-3a-after.png`, `shots/C-3h-3b-dialog.png`, `shots/C-3h-3b-cancelled.png`, `shots/C-3h-titlebar-off-check.png`; `logs/C-3h.log` |

## Notes and cleanup

- Dialog body observed on 3b/3c/3e/3f/3h was exactly `Unsaved changes in 'Untitled'. Close this window anyway?`; no dialog contained `untitled note(s) are not listed`.
- No `Quick`-matching log entries were emitted; Quick note settings were checked in the UI and config.
- During 3d troubleshooting, pressing Return while the close dialog remained open inserted a newline; the note was discarded and no file persisted. The Save action itself never advanced to Save As.
- Config restoration: copied `config.json` and `session.json` from `config-backup-C/`; both `cmp` checks passed. Quick note workflow is back to its original ON state and system title bar to OFF.
- All Ferrite windows/processes were closed at completion; observed test launches exited cleanly (code 0). No crash occurred.

## SES-3d re-run with portal

**Result: PASS.** Quick note was ON in Settings → Files before testing. The primary-window control (new Untitled tab, `abc`, tab ×, Save) opened a GTK “Save File” chooser; Cancel closed the chooser and left the primary tab and its `abc` text in place.

For SES-3d, Ctrl+Shift+N opened a secondary window; after typing `abc`, clicking its × and choosing Save opened the GTK “Save File” chooser. Entering `/workspace/ferrite-qa/rerun4/files/SES-3d-abc.md` in Name and confirming saved the file. The secondary window closed automatically after saving; the primary window and its tabs remained. The saved file was read back and contains exactly `abc` (one line, 3 bytes).

Repeat of 3b: choosing Save opened the chooser; Cancel returned to the still-visible Unsaved Changes close prompt. The secondary window stayed open with `abc`; the close prompt did not vanish. For cleanup, Don't Save closed that secondary window.

Relevant log lines:
- `[2026-11-09T23:44:39Z DEBUG ferrite::app::file_ops] Save dialog cancelled` (primary control Cancel)
- `[2026-11-09T23:45:37Z INFO  ferrite::state] Saved file as: /workspace/ferrite-qa/rerun4/files/SES-3d-abc.md (encoding: utf-8)`
- `[2026-11-09T23:45:37Z INFO  ferrite::state] Closed document window 1`
- `[2026-11-09T23:46:03Z DEBUG ferrite::app::file_ops] Save dialog cancelled` (repeat Save As Cancel)
- `[2026-11-09T23:46:15Z INFO  ferrite::state] Closed document window 2` (cleanup)
- No `xdg_desktop_portal ERROR`, `pick_folder error`, or xdg-desktop-portal error signature appeared in this launch log. Ferrite exited with code 0 (clean exit); no crash.

Evidence: `shots/C-3d-portal-control-saveas.png`, `shots/C-3d-portal-saveas.png`, `shots/C-3d-portal-saved.png`, `shots/C-3d-portal-cancel-saveas.png`, `shots/C-3d-portal-primary-after.png`; full launch log: `logs/C-3d-portal.log`.

Cleanup: no Ferrite process remains. Restored `config.json` and `session.json` from `config-backup-3d`; both `cmp` checks passed. No code, commits, pushes, or issues were changed or created.
