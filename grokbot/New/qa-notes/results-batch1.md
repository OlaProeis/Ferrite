# Ferrite v0.3.1 QA - Batch 0 (setup) + Batch 1 results

Tester: Grok Bot (computerUse subagent). Launch log tags: session1 (main session), T1-10, T1-11.
Note: log timestamps are UTC; desktop clock is UTC+2.

## Batch 0 (setup)
- Step 1: pgrep -x ferrite was empty before launch.
- Step 2: moved ~/.config/ferrite -> ~/.config/ferrite.bak-1791483152 (existed before).
- Step 3: prep-fixtures.sh ran OK (exit 0).
- Step 4: launched run-ferrite.sh --tag session1 (log: ferrite-debug-20261008-201237-session1.log). Window appeared within ~3 s. F1 About shows "Version 0.3.1". Screenshot: T0-preflight-pass.png.

| Row | Result | Observed | Log | Screenshot |
|---|---|---|---|---|
| T0 setup | PASS | Setup steps 1-4 done; About = 0.3.1; window up in <3 s | ferrite-debug-20261008-201237-session1.log | T0-preflight-pass.png |

## Batch 1
| Row | Result | What I observed | Log | Screenshot |
|---|---|---|---|---|
| T1-1 | PASS | Rendered view: clicked 'First paragraph.', pressed Escape. Tab title 'crlf-test.md' has no '*' (not dirty). File on disk unchanged (md5 c2a7886a..., CRLF 8/8). Note: app froze ~6 s later during T1-2 attempts (see T1-2). | ferrite-debug-20261008-201237-session1.log | T1-1-pass.png |
| T1-2 | BLOCKED | Clicked into Rendered paragraph again; typing ' edited' (and single chars 'Z', 'z') produced no visible text and no dirty marker. Double-click on the word showed no selection. Then the whole UI stopped responding: clicks on the Untitled tab and the Raw view pill did nothing, no log output after 18:13:51Z, process alive (state S, main thread in futex_wait), no panic/exit. Killed with kill -9 => exit 137 (not a crash). Ctrl+S / eol-check not reached in this session. | ferrite-debug-20261008-201237-session1.log (exit 137) | T1-2-blocked-hang.png (frozen UI) |
| T1-2 (retry in fresh instance, tag T1-2-retry) | BLOCKED | Fresh launch with /tmp/crlf-test.md. Rendered -> click 'First paragraph.' -> typed ' edited': text DID insert at the click point (now 'First para editedgraph.'), but tab title showed no '*' and no commit. Clicked blank area to leave the block: UI froze again (log silent after 18:18:12Z, main thread futex_wait, workers idle, no panic). Probe click on view pill got no response. Killed -9 => exit 137. Ctrl+S / eol-check still not reached. Disk file unchanged (md5 c2a7886a..., CRLF 8/8). | ferrite-debug-20261008-201237-session1.log (exit 137); ferrite-debug-*-T1-2-retry.log (exit 137) | T1-2-blocked-hang.png, T1-2-blocked-hang-retry.png |
| T1-3 | PASS | Raw: caret end of 'Second paragraph with more words.', Enter, typed 'new line', Home, Backspace -> line reads 'Second paragraph with more words.new line' (joined, no stray CR mid-line, no doubled blank lines). Ctrl+S ok. eol-check crlf: 'RESULT: CRLF preserved (True)' (8/8 lines). cat -A: all lines end ^M$. Note: caret was mid-line, so the join is as expected. Launch exit 0 at close. | ferrite-debug-20261008-201938-T1-3.log | none (PASS, no shot required) |
| T1-4 | PASS (with observation) | Continued on the same doc (state after T1-3). Raw: Alt+Up moved the line up, Alt+Down moved it back (disk unchanged until save); Ctrl+Shift+D duplicated the line; Ctrl+Shift+U inserted TOC (status 'TOC inserted (2 headings)'). Caret was mid-line, so the TOC block was inserted between 'Second paragraph with more words.' and the 'new line' remainder, which moved below the TOC. Ctrl+S. eol-check crlf: 'RESULT: CRLF preserved (True)' 16/16; TOC lines end 0d0a. Observation: `file` reports 'exported SGML document' because of the <!-- TOC --> comment (not a line-ending issue). | ferrite-debug-20261008-201938-T1-3.log (session continued in that instance; logs/T1-4-check.txt) | none (PASS, no shot required) |
| T1-4b | FAIL | Could not open /tmp/crlf-utf16.md in Ferrite at all. Launched with the file (run-ferrite.sh --tag T1-4b) and via a forwarded open: Ferrite logged 'Opening file from CLI' then 'Delegating to external app (binary): /tmp/crlf-utf16.md' and the OS default app (LibreOffice Writer 25.2) opened the file instead; no Ferrite tab for it. Cause in code: src/state.rs is_binary_content() returns true for any 0x00 byte, and UTF-16 LE text is full of 0x00 bytes, so the file is routed to the external app before any encoding/BOM detection (FFFE BOM ignored). So Raw/Enter/Ctrl+S/eol round trip in Ferrite was NOT reachable. Fixture itself is valid: eol-check utf16 on the untouched file = PASS (8 pairs, no lone 0a00); md5 unchanged 83f42a03..., file not modified by me. | ferrite-debug-20261008-202232-T1-4b.log (instance pid 97949 still running at time of record; logs/T1-4b-check.txt is the checker on the UNMODIFIED fixture) | T1-4b-fail.png (Ferrite + terminal showing check output and the 'Delegating to external app (binary)' line) |
| T1-5 | PASS | Opened reload-test.md (Raw, clean). External `sed -i 's/First/FIRST/'`, then Ctrl+Shift+R: content updated to 'FIRST line of reload test.', no dialog, status bar 'Reloaded: reload-test.md'. | ferrite-debug-20261008-202232-T1-4b.log (same Ferrite instance, pid 97949) | none (PASS) |
| T1-6 | PASS | Typed 'dirty edit' (tab 'reload-test.md*'), external `printf 'external edit\n' >>`. Ctrl+Shift+R -> 'Unsaved Changes' dialog ('reload-test.md*' has unsaved changes. Reload from disk anyway? Save / Don't Save / Cancel). Cancel: edit kept (tab still dirty, 'dirty edit' visible). Ctrl+Shift+R again -> Don't Save: buffer = disk content incl. 'external edit' (verified cat -A), tab clean, status 'Reloaded'. | ferrite-debug-20261008-202232-T1-4b.log | T1-6-pass.png (dialog) |
| T1-7 | PASS (row expectation met) — but UI froze afterwards (separate defect, see below) | Clicked into 'Second paragraph.' in Rendered, typed 'ab' (rendered as 'Second paaragraph.'; tab dirty only after commit), did NOT click away, Ctrl+Shift+R -> 'Unsaved Changes' dialog appeared ('reload-test.md*' has unsaved changes. Reload from disk anyway?). Cancel click then had no effect: the UI froze (log stops 18:26:31Z after a periodic crash-recovery snapshot; main thread futex_wait; pill click ignored for 12+ s). Killed -9 => exit 137. | ferrite-debug-20261008-202232-T1-4b.log (exit 137) | T1-7-pass.png (dialog); T1-7-fail-hang-after-dialog.png (frozen) |
| T1-8 | PASS | Fresh instance (tag T1-8) with reload-test.md + other-tab.md. Session Recovery dialog appeared (left over from the killed T1-7 instance); chose 'Start Fresh' (see observation). Typed 'dirty' in reload-test (Raw, dirty), external printf 'ext2' appended, Ctrl+Shift+R -> Unsaved Changes dialog. With the dialog open, clicked the other-tab.md tab X: tab closed. Then Don't Save: reload-test reloaded and shows 'ext2', status 'Reloaded: reload-test.md'. other-tab.md on disk unchanged (md5 afaeb51ffa4e49014b9b2ae4c95692f5 before and after; content 'This tab must stay untouched.'). | ferrite-debug-20261008-202755-T1-8.log | none (PASS; T1-8-dialog-open.png saved for reference) |
| T1-9 | PASS | Opened save-a.md and save-b.md (forwarded into running instance, tag T1-8 instance). Edited both (' A-edit', ' B-edit'), Alt+F4 -> 'Unsaved Changes: You have unsaved changes. Exit anyway?' (Save / Don't Save / Cancel). Clicked Save: both files updated on disk (save-a: 'alpha A-edit', save-b: 'bravo B-edit'), app closed, no dialog loop. Launcher: 'Ferrite exited with code 0 clean exit'. | ferrite-debug-20261008-202755-T1-8.log (exit 0) | T1-9-exit-dialog.png (exit dialog, reference; FAIL-only shot not needed) |
| T1-10 | PASS | Launched with fixtures/discard-test.md (tag T1-10; no recovery dialog at launch). Typed ' DISCARD-ME' on line 3 (tab 'discard-test.md*', status 6 words/44 chars). Alt+F4 -> 'Unsaved Changes: You have unsaved changes. Exit anyway?' (Save / Don't Save / Cancel). Clicked Don't Save: app closed, launcher 'Ferrite exited with code 0 clean exit'. Disk: `cat -A` = '# Discard test$ / $ / original content$' (no DISCARD-ME). Relaunched run-ferrite.sh --tag T1-10b with same file: buffer shows 'original content' (33 chars, 4 lines), tab clean (no '*'), no 'Recovered content differs' banner, no autosave/recovery dialog; log: 'Loaded recovery content for 0 tabs', 'Session recovery status: is_crash_recovery=false'. Closed with Alt+F4 (no dialog, exit 0). recovery/ dir empty after both runs. Observation: first T1-10 launch logged WARN 'Ignoring stale recovery file for tab 1 (reload-test.md)' and pruned recovery/1.json (left over from an earlier instance); no user-visible dialog. | ferrite-debug-20261008-203037-T1-10.log (exit 0; launcher-T1-10.out); ferrite-debug-20261008-203128-T1-10b.log (exit 0; launcher-T1-10b.out) | none (FAIL-only shot rule; no FAIL) |
| T1-11 | PASS | Opened kill-test.md (tag T1-11, clean tab), clicked end of line 3, typed ' KILL-EDIT' (tab 'kill-test.md*'), waited ~5 s (recovery snapshot logged and recovery/12.json contained 'original content KILL-EDIT'), kill -9 => exit 137. Relaunch: 'Session Recovery?' dialog ('Ferrite detected that your previous session was not closed properly.' / 'recovery.session.tabs_unsaved' / 'Would you like to restore your previous session?' buttons: Restore Session / Start Fresh). Clicked Restore Session: kill-test.md shows 'original content KILL-EDIT', tab dirty, banner 'Recovered content differs from this file on disk.' (Keep Recovered / Reload from Disk), status 'Session restored'. One Ctrl+Z: buffer = disk content ('# Kill test / original content', 30 chars, status 'Undo (0 remaining)'); disk unchanged (cat -A). Closed with Alt+F4 (no dialog), exit 0. Observations: the banner stayed visible after undo; 'recovery.session.tabs_unsaved' is a raw key shown in the dialog; recovery/12.json was left behind after the clean exit (see Odd list). | ferrite-debug-20261008-203215-T1-11.log (kill exit 137; launcher-T1-11.out); ferrite-debug-20261008-203306-T1-11-relaunch.log (2nd kill exit 137); ferrite-debug-20261008-203405-T1-11-shot.log (3rd kill exit 137); logs/ferrite-debug-20261008-203534-T1-11-dialog.log (exit 0; launcher-T1-11-dialog.out) | T1-11-pass.png (recovery dialog, from the 4th launch); T1-11-observation-no-dialog-after-undo.png (3rd launch: no dialog, because the undone buffer matched disk) |

## Odd observations (not row results)
- Hangs: T1-2 (twice) and T1-7 (after Cancel in the reload dialog) froze the whole UI; process alive, main thread in futex_wait, no log output, needed kill -9 (exit 137). Not a panic/segfault.
- T1-4b: /tmp/crlf-utf16.md is delegated to an external app (is_binary_content treats 0x00 as binary) so it never opens in Ferrite.
- other-tab.md (closed during T1-8) shows up in session restore logs from the T1-10 launch on ("Restoring N tab(s)" lists it), and in the tab bar of the Session Recovery state before Restore; it disappears after Restore Session. Disk content untouched (md5 afaeb51ffa4e49014b9b2ae4c95692f5).
- T1-10 first launch logged WARN "Ignoring stale recovery file for tab 1 (reload-test.md)" and pruned recovery/1.json.
- Session Recovery dialog shows the raw string 'recovery.session.tabs_unsaved' (untranslated key).
- "Recovered content differs from this file on disk" banner stays visible after Ctrl+Z makes the buffer equal to disk.
- After a clean exit (T1-11 dialog run), ~/.config/ferrite/recovery/12.json was left behind holding the undone edit ('KILL-EDIT'); after T1-10 the recovery dir was empty.
- Every launch: WARN winit XSETTINGS, WARN XRandR '0mm in size', WARN epaint 'Failed to find replacement characters'.
- Deliberate extra kill -9s during T1-11 (3 launches) were needed to capture the dialog screenshot; the final relaunch was closed cleanly (exit 0).

## FAIL details

### T1-4b - UTF-16 LE CRLF round trip (FAIL)
Repro:
1. Run `/workspace/ferrite-qa/run-ferrite.sh --tag T1-4b /tmp/crlf-utf16.md` (or forward the file to a running instance with `ferrite /tmp/crlf-utf16.md`).
2. Observe: Ferrite logs "Opening file from CLI: /tmp/crlf-utf16.md" then "Delegating to external app (binary): /tmp/crlf-utf16.md"; the OS default app (LibreOffice Writer 25.2) opens the file instead. No Ferrite tab is created.
3. Cannot reach the remaining steps (Raw view, press Enter, Ctrl+S, round-trip check) inside Ferrite.
Expected: file opens in Ferrite (UTF-16 LE with BOM FFFE decoded as text), Raw view, one Enter, Ctrl+S, then `eol-check.sh /tmp/crlf-utf16.md utf16` shows only 0d00 0a00 pairs and no lone 0a00; text readable.
Actual: routed to external app as binary. Cause (source): src/state.rs is_binary_content() returns true for any 0x00 byte; UTF-16 LE text is full of 0x00 bytes; the FFFE BOM is never considered before that check.
Fixture check (untouched file, not modified): eol-check utf16 = PASS (BOM fffe, 8 pairs 0d00 0a00, lone 0a00 = []). Saved at logs/T1-4b-check.txt.
Log evidence (ferrite-debug-20261008-202232-T1-4b.log, lines 128-129):
[2026-10-08T18:22:33Z INFO  ferrite::app] Opening file from CLI: /tmp/crlf-utf16.md
[2026-10-08T18:22:33Z INFO  ferrite::state] Delegating to external app (binary): /tmp/crlf-utf16.md

Last 30 non-span lines of that log (end of that instance, after the later T1-5/T1-6/T1-7 work in the same pid; it was killed -9 at 20:27:38):
[2026-10-08T18:26:16Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=3, clean_shutdown=false
[2026-10-08T18:26:16Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:26:16Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:26:16Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 3 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:26:16Z DEBUG ferrite::config::session] Saved recovery content for tab 3 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(15334466962217878387))
[2026-10-08T18:26:16Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:26:16Z DEBUG winit::window] winit::Window::set_title; title="reload-test.md* - Ferrite"
[2026-10-08T18:26:16Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 180 }) size=Physical(PhysicalSize { width: 143, height: 34 })
[2026-10-08T18:26:21Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:26:21Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=3, clean_shutdown=false
[2026-10-08T18:26:21Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:26:21Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:26:21Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 3 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:26:21Z DEBUG ferrite::config::session] Saved recovery content for tab 3 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(15334466962217878387))
[2026-10-08T18:26:21Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:26:26Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:26:26Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=3, clean_shutdown=false
[2026-10-08T18:26:26Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:26:26Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:26:26Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 3 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:26:26Z DEBUG ferrite::config::session] Saved recovery content for tab 3 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(15334466962217878387))
[2026-10-08T18:26:26Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:26:31Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:26:31Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=3, clean_shutdown=false
[2026-10-08T18:26:31Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:26:31Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:26:31Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 3 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:26:31Z DEBUG ferrite::config::session] Saved recovery content for tab 3 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(15334466962217878387))
[2026-10-08T18:26:31Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 20:27:38 CEST ===
