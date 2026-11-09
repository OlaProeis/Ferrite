# Ferrite v0.3.1 QA - Batch 9a (data integrity remainder)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite`. Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <files>`. Screenshots: `shots/`. Logs: `logs/`.
Rows: STAT-3, EOL-3, EOL-5, EOL-9, SES-1, SES-3, SES-4, SES-5, SES-6.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| STAT-3 | FAIL | New Untitled doc (Raw). Typed `- item` + Enter: cursor went to a new empty line 2 (Ln 2, Col 1), no `- ` marker continued. Same for `* item` + Enter and `1. item` + Enter (Ln 3/Ln 4 Col 1, no marker). List detection/continuation on Enter does NOT happen in Raw. Status-bar stats were sane throughout: after `- item`/`* item`/`1. item` = "6 words, 21 chars, 3 lines"; after the final Enter = "6 words, 22 chars, 4 lines" (matches content). Code note: `split_list_item` (src/markdown/ast_ops.rs) is only called from the Rendered/markdown editor (src/markdown/editor.rs); the Raw FerriteEditor has no list-continuation logic. Doc left unsaved (Untitled*). Tool note: my first attempt sent type+Enter in one parallel block and keys were reordered; the reported result is from the sequential re-run. | logs/ferrite-debug-20261009-023201-STAT-3.log (no list/continuation log lines; no panic) | shots/STAT-3-fail.png |
| EOL-3 | PASS | Opened /tmp/crlf-test.md (launched with the file), switched to Split with Ctrl+E. Clicked at the end of `First paragraph.` in the preview pane, typed ` x`. The preview showed `First paragraph. x` but the Raw pane, status bar and tab dirty marker did not update until Ctrl+S (then the Raw pane showed `First paragraph. x`, status "Saved /tmp/crlf-test.md", 17 words / 104 chars). Checker: `file` = "ASCII text, with CRLF line terminators"; 8/8 lines end in CR; RESULT: CRLF preserved (True). `cat -A` shows every line `^M$`. Note: the preview edit did not update the Raw pane, status bar or dirty marker until Ctrl+S (cause not determined; not a line-ending issue). | logs/ferrite-debug-20261009-023554-EOL-3.log; checker logs/EOL-3-check.txt | shots/EOL-3-pass.png |
| EOL-5 | PASS | Opened /tmp/lf-test.md (session had it in Split; Ctrl+E once moved to Rendered-only). Clicked at end of `First paragraph.` in Rendered, typed ` x`. Rendered text showed `First paragraph. x` but the status bar/dirty marker only updated after Ctrl+S ("Saved /tmp/lf-test.md", 11 words / 64 chars). No hang, no crash. Checker: `file` = "ASCII text"; lines containing CR = 0; RESULT: LF-only (True). `cat -A`: every line ends `$` with no `^M`. | logs/ferrite-debug-20261009-023739-EOL-5.log; checker logs/EOL-5-check.txt | shots/EOL-5-pass.png |
| EOL-9 | FAIL | Pre-check: `file` = "ASCII text, with CRLF, LF line terminators", 4 of 5 lines CRLF (logs/EOL-9-check-pre.txt). Opened /tmp/mixed-eol.md (it opened in Raw). Clicked at the end of `end` (line 5), typed ` x`, Ctrl+S -> status "Saved /tmp/mixed-eol.md". No crash, no hang. After save: `file` still says "ASCII text, with CRLF, LF line terminators"; checker RESULT: NOT all CRLF (False), 4 CRLF / 5 lines. `cat -A`: `# Mixed^M$`, `^M$`, `line crlf^M$`, `line lf$` (the original LF line was left as LF), `end x^M$`. So the save did not normalise to one consistent ending (expected: majority CRLF applied to all lines); each line kept its own ending. | logs/ferrite-debug-20261009-023851-EOL-9.log ; checker logs/EOL-9-check.txt | shots/EOL-9-fail.png |
| SES-1 | FAIL | Launched with /tmp/ses1-a.md and /tmp/ses1-b.md (copies of lf-test.md) plus one untitled tab (Ctrl+N). Typed ` A1` into ses1-a (Second paragraph.), ` B1` into ses1-b, and `untitled SES-1 text` into Untitled (3 dirty tabs, 19 chars in Untitled). Alt+F4 showed a single dialog "Unsaved Changes - You have unsaved changes. Exit anyway?" with buttons Save / Don't Save / Cancel (no per-tab list, no Save As). Clicked Save. Ferrite exited with code 0 with NO Save As dialog for the untitled tab. Log: "Saved file: /tmp/ses1-a.md", "Saved file: /tmp/ses1-b.md", "Exit confirmed", then "Saved recovery content for tab 69 (path=None)" for the untitled tab (also for 5 older Untitled tabs), session saved with 39 tabs. Disk check: /tmp/ses1-a.md ends `Second paragraph. A1`, /tmp/ses1-b.md ends `Second paragraph. B1` (both saved, LF, 67 bytes). /workspace/ferrite-qa/fixtures/untitled-save.md was NOT created. Untitled content was kept only as session/crash recovery (path=None), not written to a file. Expected per row: Save As dialog for the untitled tab, saved to fixtures/untitled-save.md, then exit. | logs/ferrite-debug-20261009-024031-SES-1.log (exit code 0; no panic) | shots/SES-1-fail-dialog.png (exit dialog); shots/SES-1-fail.png (desktop after exit) |
| SES-3 | PASS | Window 1 was the restored session (Ferrite relaunched with no Session Recovery prompt). Ctrl+Shift+N only created window 2 after I clicked into the editor first (first press was lost). The native Open dialog does not work here (log: rfd xdg_desktop_portal "pick_files error No such file or directory"), so instead I created /tmp/ses3-w2.md (3 lines) and forwarded it with `$FER /tmp/ses3-w2.md`: the running instance opened it in the last-focused window (window 2). NOTE: deviation from the row, which said /tmp/ses1-a.md; ses1-a.md was already open in window 1 and the file was a fresh file so the dirty tab was unambiguous. Modified it (added ` x` to line 3; tab showed ses3-w2.md*). Clicked window 2's own close button -> dialog "Unsaved Changes - This window has unsaved changes. Close anyway?" (Save / Don't Save / Cancel). Clicked Save -> log "Saved file: /tmp/ses3-w2.md", "Closed document window 1" (internal index), window 2 viewport removed. Window 1 stayed open (session windows=1, tabs=39). Disk: /tmp/ses3-w2.md = `Window two paragraph. x`. | logs/ferrite-debug-20261009-024315-SES-3.log | shots/SES-3-pass-dialog.png (close dialog); shots/SES-3-pass-after.png (after close, window 1 remains) |
| SES-4 | PASS | (a) Tab close: ses1-b.md was restored as a clean tab, I typed ` Q` at the end of line 7 (tab showed ses1-b.md*), clicked the tab's x -> dialog "'ses1-b.md*' has unsaved changes. Close anyway?" (Save / Don't Save / Cancel) -> Don't Save -> tab closed. Disk: /tmp/ses1-b.md still ends `Second paragraph. B1` (no Q). Log: "Deleted recovery content for tab 70", "Closed tab 70". (b) Window close: created window 2 (Ctrl+Shift+N after clicking into the editor), forwarded /tmp/ses4-other.md into it with `$FER /tmp/ses4-other.md` (no native dialog available), typed text (tab ses4-other.md*), clicked window 2's own close button -> dialog "Unsaved Changes - This window has unsaved changes. Close anyway?" -> Don't Save -> window closed, window 1 stayed. Log: "Closed tab 74", "Closed tab 75", "Closed document window 2", "Deleted recovery content for tab 75". Disk: /tmp/ses4-other.md unchanged (`# SES-4 other doc / Other paragraph.`). (c) `find ~/.config/ferrite -newer /tmp/lf-test.md -type f` lists: config.json, instance.lock/.pid/.port, session.json, session.lock, session.recovery.json and recovery/{54,55,57,58,59,60,61,62,63,64,65,66,69,71}.json. Recovery files for the discarded docs are gone: 70 (ses1-b), 75 (ses4-other), and 74/72/73 (empty/saved window tabs) are absent. Remaining recovery files belong to the still-open tabs of window 1 (untitled/scratch, path=None) and were not mapped one by one. | logs/ferrite-debug-20261009-024315-SES-3.log (same process as SES-4; no panic) | shots/SES-4-pass-tab-dialog.png; shots/SES-4-pass-window-dialog.png; shots/SES-4-pass-after.png |
| SES-5 | PASS | Ctrl+N -> new Untitled tab; pasted `quick note SES-5` (clipboard via xclip + Ctrl+V); status "3 words, 16 chars, 1 lines", tab Untitled*. Alt+F4 -> NO "Unsaved Changes" prompt appeared; Ferrite exited straight away (log: "Received WindowEvent::CloseRequested for viewport FFFF", "Application exiting", "Saved recovery content for tab 76 (path=None)"). So there was no dialog to answer; the option that keeps the text was not needed. Relaunch with `run-ferrite.sh --tag SES-5` restored the session with no Session Recovery prompt. The Untitled tab came back with `quick note SES-5` (16 chars, 3 words, 1 line, tab still dirty Untitled*). Log: "Using recovered content for tab 76 (Untitled)", "Restored tab 76 from session: Untitled". Second quit after that relaunch (Alt+F4) also gave no prompt. Observation for the lead: the exit prompt appeared in SES-1 when dirty FILE-backed tabs existed; with only untitled dirty tabs the app quits silently and relies on recovery. | logs/ferrite-debug-20261009-024315-SES-3.log (quit #1); logs/ferrite-debug-20261009-024826-SES-5.log (relaunch and quit #2) | shots/SES-5-pass-restored.png (restored content after relaunch) |
| SES-6 | PARTIAL | Relaunched with `run-ferrite.sh --tag SES-6` (no recovery prompt; the restored session still had the SES-5 quick note, `quick note SES-5`, 16 chars). Window 1 showed restored tab ses1-a.md; I clicked into it, End, typed ` W1`. Ferrite then showed only ` W1` (status "1 words, 4 chars, 2 lines"); recovery for that tab held ` W1` only, while /tmp/ses1-a.md on disk was untouched (`Second paragraph. A1`). This was an unexpected loss of the restored in-memory text, see note below. Window 2: created with Ctrl+Shift+N and forwarded /tmp/ses1-b.md into it (native dialog unavailable, see SES-3), typed ` W2` (it landed in the first paragraph, not at the end; tab ses1-b.md*). Clicking window 2's own close button showed "Unsaved Changes - This window has unsaved changes. Close anyway?" (Save / Don't Save / Cancel). The dialog does NOT name a file; the window title was "ses1-b.md* - Ferrite". Don't Save -> window 2 closed, window 1 stayed. Window 1 (ses1-a.md*) closed with Alt+F4 -> "Unsaved Changes - You have unsaved changes. Exit anyway?" (also no file name). Don't Save -> "Exit confirmed", exit code 0. Disk after: /tmp/ses1-a.md ends `Second paragraph. A1`, /tmp/ses1-b.md ends `Second paragraph. B1` (both unchanged). Expectation "dialog names the correct tab/file" NOT met for window close (the single-tab close dialog from SES-1/SES-4 does name the file: "'ses1-b.md*' has unsaved changes"). | logs/ferrite-debug-20261009-024942-SES-6.log (exit code 0, no panic) | shots/SES-6-window2-dialog.png; shots/SES-6-window1-dialog.png |

## Last ~30 non-span log lines for each FAIL

Command used per log: `grep -v 'tracing::span' <log> | tail -n 30`.

### STAT-3 (logs/ferrite-debug-20261009-023201-STAT-3.log)

```text
[2026-10-09T00:35:40Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:35:40Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1250, height: 963 })
[2026-10-09T00:35:40Z INFO  ferrite::app] Application exiting
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saved recovery content for tab 57 (path=None, hash=None)
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saved recovery content for tab 58 (path=None, hash=None)
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saved recovery content for tab 60 (path=None, hash=None)
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saved recovery content for tab 61 (path=None, hash=None)
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saved recovery content for tab 62 (path=None, hash=None)
[2026-10-09T00:35:40Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=34, clean_shutdown=true
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 34 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:35:40Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:35:40Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:35:40Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T00:35:40Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:35:40Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:35:40Z INFO  ferrite::state] Settings saved
[2026-10-09T00:35:40Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:35:40Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:35:40Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:35:40Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:35:40Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:35:40Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:35:40Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:35:40Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:35:40Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:35:40Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:35:41 CEST ===
```

### EOL-9 (logs/ferrite-debug-20261009-023851-EOL-9.log)

```text
[2026-10-09T00:40:13Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:40:13Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1250, height: 963 })
[2026-10-09T00:40:13Z INFO  ferrite::app] Application exiting
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saved recovery content for tab 58 (path=None, hash=None)
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saved recovery content for tab 59 (path=None, hash=None)
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saved recovery content for tab 61 (path=None, hash=None)
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saved recovery content for tab 62 (path=None, hash=None)
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saved recovery content for tab 63 (path=None, hash=None)
[2026-10-09T00:40:13Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=36, clean_shutdown=true
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 36 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:40:13Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:40:13Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:40:13Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T00:40:13Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:40:13Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:40:13Z INFO  ferrite::state] Settings saved
[2026-10-09T00:40:13Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:40:13Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:40:13Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:40:13Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:40:13Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:40:13Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:40:13Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:40:13Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:40:13Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:40:13Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:40:13 CEST ===
```

### SES-1 (logs/ferrite-debug-20261009-024031-SES-1.log)

```text
[2026-10-09T00:42:17Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T00:42:17Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T00:42:17Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T00:42:17Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:42:17Z INFO  ferrite::app] Application exiting
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 59 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 60 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 62 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 63 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 64 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saved recovery content for tab 69 (path=None, hash=None)
[2026-10-09T00:42:17Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=true
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 39 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:42:17Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:42:17Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T00:42:17Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:42:17Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:42:17Z INFO  ferrite::state] Settings saved
[2026-10-09T00:42:17Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:42:17Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:42:17Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:42:17Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:42:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:42:17Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:42:17Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:42:17 CEST ===
```

## Notes

- SES-6 finding (not a listed row, found while testing SES-6): after a restart, the restored tab `ses1-a.md` showed its full text, but after clicking, End and typing ` W1` the whole buffer became just ` W1`. The file on disk was not changed, but the unsaved in-memory text was lost. Reproduce with: restore `/tmp/ses1-a.md` in a session, edit it, look at the editor content. Evidence: shots/SES-6-window1-dialog.png and logs/ferrite-debug-20261009-024942-SES-6.log.
- Dialog wording: the window-close and app-exit confirmations (SES-1, SES-3, SES-6) do not name the file; the single-tab close dialog in SES-4 does (`'ses1-b.md*' has unsaved changes`).
- EOL-3/EOL-5: preview edits did not update the Raw pane, the status bar or the dirty marker until Ctrl+S; the cause was not determined (not a line-ending problem).

## Final state

- Ferrite quit: SES-6 exit was a clean exit, code 0 (`Exit confirmed`, `Application exiting`). `pgrep -a -x ferrite` returned nothing (exit 1).
- `git -C /workspace/ferrite-qa/ferrite status --short` (exit 0) printed no output, so the working tree is clean; no code was changed and nothing was committed.

```text
$ git -C /workspace/ferrite-qa/ferrite status --short
(no output: clean working tree)
```

## Batch 9b (rendered fidelity remainder)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite`. Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <files>`. Screenshots: `shots/`. Logs: `logs/`.
Rows: ENT-6, FIND-5, ARR-1, ARR-2, ARR-4, ARR-5, ARR-6b, CPY-2, CPY-3, UND-3.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| ENT-6 | PASS | Launched with test_html_entities.md (no Session Recovery prompt; session restore opened the tabs directly). Switched to Split via the R/S/V view selector (S button, no click in the preview). Preview: `&amp; &rarr; &reg;` shows `& → ®`; `&#38; &#8594; &#174;` shows `& → ®` (the row's exact string `&amp; &rarr; &reg; &#8594;` is not in the fixture, so the closest fixture lines were checked); `&foo;` shows literally; `<span>tag</span>` is not executed (no span styling, grey `«HTML»` chips around plain `tag`). Same as the T2-31 Rendered result. Window still responsive at the end, no panic. | logs/ferrite-debug-20261009-025410-ENT-6.log | shots/ENT-6-pass.png |
| FIND-5 | PASS | Raw view (switched with the R selector, clicked into Section 2 text). Ctrl+F `ferrite` (7 matches, matching grep count of 7 incl. the checklist text). Counter: Enter -> 2 of 7, Enter -> 3 of 7 (line 21), F3 -> 4 (line 34), F3 -> 5 (line 54), F3 -> 6 (line 65, off-screen, viewport scrolled). Shift+F3 back: 5, 4, 3, 2, 1 (line 3, top), then Shift+F3 from 1 wraps to 7 of 7 (line 69, viewport scrolled). Each step highlighted the expected match. Note (input artifact, not app): on 4 of the Shift+F3 presses xdotool dropped Shift and Ferrite logged "F3 (Find Next)" instead; the log, not the screenshot counter, was used to confirm each step. Typing `ferrite` showed "No matches" for a moment until the debounced search ran (log: "Search executed (debounced), found 7 matches"). No crash, no hang. | logs/ferrite-debug-20261009-025519-FIND-5.log | shots/FIND-5-pass.png |
| ARR-1 | FAIL (hang) | Rendered view of test_find_arrow_nav.md. Attempt 1: clicked into `The ferrite crystal sat on the shelf.` (Section 1), the window repainted normally. Pressed Down: no caret move and no repaint; Ferrite froze (0% CPU, window stopped repainting for 30+ s). Backtrace (logs/ARR-1-hang-bt.txt): Thread 1 in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow` <- `ferrite::markdown::editor::arrow_nav_sel_key` <- `remember_arrow_nav_sel`, i.e. the known arrow_nav_sel_key deadlock. Killed with kill -9 (exit 137). Attempt 2 (relaunch; the kill made the next start show a Session Recovery dialog, answered Start Fresh): same click into the same paragraph, then Down: same freeze, same backtrace (logs/ARR-1b-hang-bt.txt). Exact keystroke that froze it: Down (ArrowDown) in the Rendered paragraph. Up was not tried (2-attempt cap). | attempt 1 logs/ferrite-debug-20261009-025846-ARR-1.log; attempt 2 logs/ferrite-debug-20261009-030021-ARR-1b.log | shots/ARR-1-fail.png (attempt 1), shots/ARR-1b-fail.png (attempt 2) |
| ARR-2 | FAIL (hang) | Rendered view, Section 4 multi-line paragraph (`Multi-line paragraph for caret-boundary testing...`, 4 visual rows). Clicked into the 2nd visual row. Down #1 moved the caret to row 3 (`three so the caret...`, seen in the screenshot), Down #2 to row 4 (`block at the first or last row.`, seen in the screenshot): the in-block visual-row movement works. Down #3 (from the last visual row, i.e. leaving the block) froze Ferrite: no repaint, 0% CPU, no new log lines. Backtrace (logs/ARR-2-hang-bt.txt): main thread in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow` <- `ferrite::markdown::editor::arrow_nav_sel_key` <- `remember_arrow_nav_sel`, the same deadlock as ARR-1. Attempt 2 (relaunch; Session Recovery answered Start Fresh) repeated the sequence: click into row 2, Down, Down (both moved within the block per the IME-cursor log lines), Down #3 froze; backtrace (logs/ARR-2b-hang-bt.txt) identical. Exact keystroke that froze it: the third ArrowDown that leaves the multi-line block to the next block. Up not tried (2-attempt cap). Pattern across ARR-1/ARR-2: Down that changes block freezes; Down within a block works. | attempt 1 logs/ferrite-debug-20261009-030131-ARR-2.log; attempt 2 logs/ferrite-debug-20261009-030412-ARR-2b.log | shots/ARR-2-fail.png (attempt 1), shots/ARR-2b-fail.png (attempt 2) |
| ARR-4 | FAIL (hang) | Split view of test_find_arrow_nav.md. Raw pane arrows work and move the raw caret only: status bar Ln 10 -> Ln 11 (Down) -> Ln 10 (Up) in attempt 1, Ln 10 -> Ln 11 (Down) in attempt 2; the preview did not change. Focus does not jump: after clicking into a preview paragraph the status bar stayed on the raw caret line (Ln 10 / Ln 11) and the raw pane did not move. Preview arrows FREEZE Ferrite: attempt 1 clicked the preview paragraph `The ferrite crystal sat on the shelf.` and pressed Down: no repaint, 0% CPU; backtrace logs/ARR-4-hang-bt.txt: main thread `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow` <- `ferrite::markdown::editor::arrow_nav_sel_key` <- `remember_arrow_nav_sel` (the known deadlock). Attempt 2 (relaunch, Session Recovery answered Start Fresh): raw Down worked, clicked the preview paragraph `Paragraph without the keyword.`, pressed Up: same freeze, same backtrace (logs/ARR-4b-hang-bt.txt). Exact keystrokes that froze it: ArrowDown and ArrowUp with focus in the preview pane. Killed with kill -9 (exit 137). | attempt 1 logs/ferrite-debug-20261009-030543-ARR-4.log; attempt 2 logs/ferrite-debug-20261009-030712-ARR-4b.log | shots/ARR-4-fail.png (attempt 1), shots/ARR-4b-fail.png (attempt 2) |
| ARR-5 | PASS | Raw-only view (R selector) of test_find_arrow_nav.md, no preview. Clicked on line 10 (`The ferrite crystal...`, status Ln 10, Col 29). Down -> Ln 11, Col 1; Right on the empty line -> Ln 12, Col 1; Left -> Ln 11, Col 1; Up -> Ln 10, Col 1. Every arrow moved the raw caret one step as expected (status bar checked each time), no freeze, no repaint problem. | logs/ferrite-debug-20261009-030834-ARR-5.log | shots/ARR-5-pass.png |
| ARR-6b | PASS | New doc (Ctrl+N, Untitled tab; no Session Recovery prompt for this launch), Raw: pasted `## #1 issue` + Enter + `text` via xclip (paste intact). Switched to Rendered, clicked into the heading, End, typed ` x` (typed input fine). Rendered heading showed `#1 issue x` (still a heading, no backslash escape). Switched back to Raw via the R selector: Raw showed line 1 `## #1 issue x`, line 2 `text`; toolbar showed H2, status bar 18 chars / 5 words. Heading not escaped to `\#1` and level unchanged. Observation: the status bar (16 chars) and the dirty marker did not update while in Rendered; the Raw pane showed the edit as soon as Raw was selected (no Ctrl+S needed). Doc left unsaved (Untitled*), nothing saved to disk. | logs/ferrite-debug-20261009-030935-ARR-6b.log | shots/ARR-6b-pass.png |
| CPY-2 | FAIL | Part A of test_copy_and_split_undo.md in Rendered (file opened straight into Rendered, no Session Recovery prompt). Selection path (mouse drag / double-click; Ctrl+C): the copied text had inserted spaces at token boundaries. Clipboard after selecting `tems.iter().m` of line 21 (`tems.iter().m` in the fixture): `tems. iter(). m` (spaces after `.` and `)`). A longer drag selection of the same rust line copied as `let x: Vec< String> = items. iter(). map(| s| s.` (spaces after `<`, `.`, and `|`; the fixture has none). Clipboard evidence saved in logs/CPY-2-selection-clipboard.txt. Fallback used for the byte check: the code block's **Copy** button (noted as fallback): clipboard matched fixture line 21 byte-for-byte (`cmp` identical, `let x: Vec<String> = items.iter().map(|s| s.to_string()).collect();`), so the spacing bug is in the selection-copy path, not the fixture. Caveat: I could not get a clean full-line mouse selection with the input tool (drags sometimes registered only partially), so the full-line selection copy itself was not captured; the two partial selections both show the spacing. | logs/ferrite-debug-20261009-031127-CPY-2.log (tail in section below); clipboard evidence logs/CPY-2-selection-clipboard.txt | shots/CPY-2-fail.png |
| CPY-3 | FAIL (hang) | Same doc, Rendered (opened straight into Rendered). Tried about 4 ways to select the inline-formatted paragraph (`And this inline-formatted paragraph ... normal **bold** *italic* `code` end.`) with the mouse: (1) slow drag from the right margin to the start of the paragraph: the page scrolled instead of selecting; (2) slow drag from the end of line 2 to the start of line 1: page scrolled, no selection; (3) double-click on `bold`: the paragraph switched into its raw edit view (`**bold**` markers shown), Ctrl+A did not select the rendered text, and Escape / a click in the margin did not return it to the rendered view; (4) slow drag across line 2 (`spaces ... end.`): no rendered selection, the paragraph stayed in the raw view. No text was changed (title stayed clean, no `*`). No Copy button exists for prose paragraphs, so no clipboard text was produced (nothing recorded, nothing invented). HANG: after attempt (4) the window stopped repainting (0% CPU, last log line 01:16:42Z, about 70 s of silence); the window manager then showed `Warning: This window might be busy and is not responding` when Alt+F4 was sent. Trigger: the last input before the freeze was the mouse drag-select across the paragraph line in Rendered view (the paragraph was already in raw edit view from the double-click on `bold`). Backtrace (logs/CPY-3-hang-bt.txt): main thread `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow` <- `ferrite::markdown::editor::arrow_nav_sel_key` <- `remember_arrow_nav_sel` <- `MarkdownEditor::show_rendered_editor`: the same known arrow_nav_sel_key deadlock as ARR-1/2/4. Killed with pkill -9 (no clean exit). Also observed: no Session Recovery prompt on this CPY-3 launch; a restored dirty `Untitled*` tab was already in the tab bar (carried over from the earlier session, not made by this row). | logs/ferrite-debug-20261009-031538-CPY-3.log (tail in section below; backtrace logs/CPY-3-hang-bt.txt) | shots/CPY-3-fail.png (hang evidence; earlier pre-hang state shots/CPY-3-blocked.png) |
| UND-3 | FAIL | Launch: Session Recovery dialog after the CPY-3 hang kill, answered Start Fresh. test_copy_and_split_undo.md Part B repeated in Rendered (non-split, view V). Pass 1: clicked **Edit** on the Python block, Ctrl+End, Enter, typed `UND3 added three words`: the Rendered code-block editor kept only `U` on the new line (typed characters dropped; the block dropped out of edit mode after the `U`). Ctrl+Z #1 removed the `U`; Ctrl+Z #2 removed the new empty line, and the doc returned clean (title without `*`, 2012 chars / 57 lines, `Undo (0 remaining)`), SENTINEL-LINE-1/2 intact. Pass 2: Edit again, End, Enter, typed `alpha beta gamma`: the editor showed garbled text (`return valuea b` / `eta gamma`; some keystrokes landed in the wrong place). Clicked Done (block committed). Ctrl+Z #1 removed ONE character (`eta gamma` -> `eta gamm`, status `Undo (12 remaining)`), Ctrl+Z #2 removed one more (`eta gam`, `Undo (11 remaining)`). Log confirms one entry per keystroke: `Undo performed, 12 entries remaining` then `11 entries remaining`. So Rendered undo is per-keystroke, not per word/edit group, and one Ctrl+Z did not revert the appended line as a unit (expected "exactly the appended line is removed"). Doc left dirty (`*`, 11 undo entries) and NOT saved; the file on disk was never written (fixture untouched). | logs/ferrite-debug-20261009-031831-UND-3.log (tail in section below) | shots/UND-3-fail.png |

### Batch 9b: last ~30 non-span log lines for each FAIL

Command used per log: `grep -v 'tracing::span' <log> | tail -n 30`.

**ARR-1 (attempt 1)** `logs/ferrite-debug-20261009-025846-ARR-1.log`

```text
[2026-10-09T00:59:12Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 39 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 63 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 64 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 66 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 68 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 72 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::config::session] Saved recovery content for tab 73 (path=None, hash=None)
[2026-10-09T00:59:12Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T00:59:14Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T00:59:14Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T00:59:14Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 367 }) size=Physical(PhysicalSize { width: 227, height: 17 })
[2026-10-09T00:59:17Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 39 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 63 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 64 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 66 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 68 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 72 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::config::session] Saved recovery content for tab 73 (path=None, hash=None)
[2026-10-09T00:59:17Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:00:07 CEST ===
```

**ARR-1b (attempt 2)** `logs/ferrite-debug-20261009-030021-ARR-1b.log`

```text
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:00:43Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:00:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:00:44Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-09T01:00:44Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-09T01:00:50Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:00:50Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T01:00:50Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 341 }) size=Physical(PhysicalSize { width: 227, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:01:13 CEST ===
```

**ARR-2 (attempt 1)** `logs/ferrite-debug-20261009-030131-ARR-2.log`

```text
[2026-10-09T01:01:44Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:44Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:44Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:44Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:44Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:44Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:44Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:45Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:45Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:45Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:01:45Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:01:45Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:01:45Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-09T01:01:45Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-09T01:01:53Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:01:53Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T01:01:53Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:02:00Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:02:00Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:02:19Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:03:00Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:03:00Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:04:10 CEST ===
```

**ARR-2b (attempt 2)** `logs/ferrite-debug-20261009-030412-ARR-2b.log`

```text
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:04:38Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T01:04:39Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-09T01:04:39Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-09T01:04:46Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:04:46Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T01:04:46Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:04:53Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:04:53Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:05:02Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
[2026-10-09T01:05:02Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 394, y: 1107 }) size=Physical(PhysicalSize { width: 515, height: 68 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:05:26 CEST ===
```

**ARR-4 (attempt 1)** `logs/ferrite-debug-20261009-030543-ARR-4.log`

```text
[2026-10-09T01:06:40Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:40Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:40Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:41Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:06:42Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:06:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=581.3, max_wrap_width=840.0, effective=581.3
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:07:10 CEST ===
```

**ARR-4b (attempt 2)** `logs/ferrite-debug-20261009-030712-ARR-4b.log`

```text
[2026-10-09T01:08:03Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:03Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:04Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:05Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:08:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:08:06Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:08:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:08:26 CEST ===
```

**CPY-2** `logs/ferrite-debug-20261009-031127-CPY-2.log`

```text
[2026-10-09T01:15:34Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T01:15:34Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T01:15:34Z INFO  ferrite::state] Settings saved
[2026-10-09T01:15:34Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T01:15:34Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T01:15:34Z INFO  ferrite::app] Application exiting
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T01:15:34Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=33, clean_shutdown=true
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 33 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T01:15:34Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T01:15:34Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T01:15:34Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T01:15:34Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T01:15:34Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T01:15:34Z INFO  ferrite::state] Settings saved
[2026-10-09T01:15:34Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T01:15:34Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T01:15:34Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T01:15:34Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T01:15:34Z WARN  arboard::platform::linux::x11] Could not hand the clipboard contents over to the clipboard manager. The request timed out.
[2026-10-09T01:15:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T01:15:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T01:15:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T01:15:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T01:15:34Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T01:15:34Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 03:15:34 CEST ===
```

**CPY-3** `logs/ferrite-debug-20261009-031538-CPY-3.log`

```text
[2026-10-09T01:16:26Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T01:16:26Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T01:16:32Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T01:16:32Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=33, clean_shutdown=false
[2026-10-09T01:16:32Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T01:16:32Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T01:16:32Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 33 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T01:16:32Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T01:16:32Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 599 }) size=Physical(PhysicalSize { width: 515, height: 34 })
[2026-10-09T01:16:34Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 599 }) size=Physical(PhysicalSize { width: 515, height: 34 })
[2026-10-09T01:16:37Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T01:16:37Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=33, clean_shutdown=false
[2026-10-09T01:16:37Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T01:16:37Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T01:16:37Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 33 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T01:16:37Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T01:16:37Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T01:16:42Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T01:16:42Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=33, clean_shutdown=false
[2026-10-09T01:16:42Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T01:16:42Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T01:16:42Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 33 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T01:16:42Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T01:16:42Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:18:15 CEST ===
```

**UND-3** `logs/ferrite-debug-20261009-031831-UND-3.log`

```text
[2026-10-09T01:21:39Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=SingleFile, windows=1, tabs=35, clean_shutdown=false
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 35 tabs, SingleFile)
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Saved recovery content for tab 11 (path=Some("/workspace/ferrite-qa/ferrite/test_md/test_copy_and_split_undo.md"), hash=Some(17641245912791671375))
[2026-10-09T01:21:39Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T01:21:39Z DEBUG ferrite::state] Exit confirmed
[2026-10-09T01:21:39Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T01:21:39Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T01:21:39Z INFO  ferrite::app] Application exiting
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Deleted recovery content for tab 11
[2026-10-09T01:21:39Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=35, clean_shutdown=true
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 35 tabs, SingleFile)
[2026-10-09T01:21:39Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T01:21:39Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T01:21:39Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T01:21:39Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T01:21:39Z INFO  ferrite::state] Settings saved
[2026-10-09T01:21:39Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T01:21:39Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T01:21:39Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T01:21:39Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T01:21:39Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T01:21:39Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T01:21:39Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 03:21:39 CEST ===
```


### Batch 9b final state

- Ferrite quit: UND-3 session closed with code 0 (Don't Save chosen at the Unsaved Changes prompt; nothing written to disk). `pgrep -a -x ferrite` returned nothing (exit 1).
- `git -C /workspace/ferrite-qa/ferrite status --short` (exit 0):

```text
$ git -C /workspace/ferrite-qa/ferrite status --short
(no output: clean working tree)
```

