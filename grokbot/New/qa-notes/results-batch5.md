# Ferrite v0.3.1 QA - Batch 5a (Tier 2 rows 30-35)

Tester: Grok Bot. Commit under test: 8e6ef508 (`git status --short` before run: only `?? test_md/zz_cjk_search.md`, left alone). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <file>`. Screenshots: `shots/`. Logs: `logs/`.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| T2-30 | PASS | Rendered: Ctrl+F `ferrite` shows 7 matches (log: found 7). Enter steps 1->7: viewport scrolls to each match (scroll offsets 0, 182, 407, 857, 1074, 1143 px in log); line 3 at top, Section 1 and 2 visible, match 4 and 5 and 6 and 7 brought into view. F3 at 7 wrapped to 1 of 7 and scrolled back to top; Shift+F3 wrapped 1 to 7 (scroll 1143) then 7 to 6 (1074). Ctrl+F reopened with `ferrite` pre-selected (highlighted); typing Section replaced it (field read Section, not ferriteSection). Restored query to ferrite. Split (S): raw pane (left) highlights matches and scrolls (line 69 match shown with Enter; top at line 3 after F3 wrap); preview pane (right) scrolls to the same matches (Section 5 / checklist shown at 7 of 7; top after wrap). Shift+F3 in Split wrapped to 7 of 7 and both panes scrolled. Ctrl+F in Split pre-selected query; typing Section replaced it. No crash, no hang; Ferrite closed via window X, exit 0 clean. Note: right after typing the screenshot briefly showed stale 'No matches' until next repaint; log confirms 7 matches. Also logged WARN epaint 'Failed to find replacement characters' at startup (font fallback, not visible in this row). | logs/ferrite-debug-20261008-225519-T2-30.log | shots/T2-30-rendered-enter1.png, shots/T2-30-rendered-match7.png, shots/T2-30-rendered-shift-f3-6of7.png, shots/T2-30-split-enter1.png, shots/T2-30-split-f3-wrap.png, shots/T2-30-split-shift-f3-wrap.png |
| T2-31 | PASS | Rendered (opened in Raw, switched to V): `&amp; &lt; &gt; &quot; &nbsp; &rarr; &larr; &reg; &copy; &mdash; &ndash; &hellip;` shows `& < > "` (nbsp as space) `→ ← ® © — – …`. Fixture line `&amp; &rarr; &reg;` shows `& → ®`. Numeric line `&#38; &#8594; &#174;` shows `& → ®`. `&foo;` shows literally as `&foo;`. `<span>tag</span>` is not executed: no span styling; rendered as small grey `«HTML»` chips around plain text `tag`. Exit code 0 clean (window closed). Note: startup WARN epaint 'Failed to find replacement characters' also logged (font fallback). | logs/ferrite-debug-20261008-225932-T2-31.log | shots/T2-31-rendered.png |
| T2-32 | FAIL | Raw (opened in Raw): line 9 `## Common single-codepoint emoji` has 7 emoji in the source (😀 😅 🤔 😴 🥳 😢 😡) but only 5 glyphs are drawn; the last two positions are blank (no glyph, no tofu box). Glyphs are monochrome (no colour) and the same in Rendered. Symbols line (13 emoji) draws all 13 glyphs in monochrome; the variation selector after the warning sign shows as an empty box. Skin-tone line draws 5 glyph groups. ZWJ line degrades to separate component glyphs (people, computer, flag/rainbow, rocket) with one empty box. Flags line shows the regional indicators as letters `N O U S J P D E`, not flags. Rendered (V): the same common line again shows 5 glyphs with blanks at the 4th and 7th positions (re-checked after about 1 min, identical). Toolbar and tab icons render normally, no tofu. No crash, no hang; window closed cleanly (exit 0). Log: 'Registered system emoji font fallback' (Noto Color Emoji) and 'Pre-warmed font atlas ... 9 emoji characters' at startup. | logs/ferrite-debug-20261008-230020-T2-32.log | shots/T2-32-raw.png, shots/T2-32-rendered.png, shots/T2-32-rendered-recheck.png |
| T2-33 | PASS | Rendered (V). Plain `![plain](assets/sample.png)`, `![dot-slash](./assets/sample.png)`, bare `![bare](sample.png)` and backslash `![backslash](assets\sample.png)` each render the sample image (orange diamond logo). Percent-encoded `assets/sample%2Epng` also renders the same image (extra, not required). `![remote placeholder](https://example.com/does-not-load.png)` shows a placeholder card labelled `remote placeholder` with `Web images not supported`, no image. Log has 0 lines matching ureq/http/example.com, so no network fetch. Missing `assets/definitely-not-here.png` shows a `missing` placeholder with `Image not found`; log WARN 'Failed to load image ... Image not found' (graceful, no crash, no retry storm). Scrolling smooth. Exit 0 clean (window closed). | logs/ferrite-debug-20261008-230432-T2-33.log | shots/T2-33-rendered-top.png, shots/T2-33-rendered-scrolled1.png, shots/T2-33-rendered-scrolled2.png |
| T2-34 | BLOCKED (drag-select) | Rendered (V). The row needs a mouse drag-select of the bash block (Part A, 4 lines starting `sudo mkdir -p /opt/stacks/notesnook`). Six drag attempts with the Computer drag tool did not produce a full selection: attempt 1 (plain drag) selected nothing; attempts 2 and 3 selected a partial range (line 1 partly, line 2 full, line 3 partly, or lines 1-4 up to `notesn`); attempts 4 and 6 (including a reverse drag) cleared the selection; attempt 5 partial again (line 1 from the second character, line 2 full, line 3 up to `sudo mkdir -p`). Ctrl+C with the partial selection (attempt 5) did not change the clipboard (it still held a leftover emoji), and the log shows no copy event for it. No crash, no hang; the window stayed responsive throughout. Supplementary, NOT the row's select path: the bash block's own Copy button (log: 'Code block copied to clipboard') put exactly 4 lines on the clipboard, read with `xclip -selection clipboard -o` while Ferrite was still running: `mkdir -p` intact, no inserted spaces, no trailing spaces, byte-identical to source (re-captured on a second launch, log logs/ferrite-debug-20261008-231936-T2-34-paste.log; see pasted output below). Note: after Ferrite quit, the clipboard was empty again (arboard WARN 'could not hand the clipboard contents over to the clipboard manager, request timed out'; no clipboard manager on this box). Rust block and inline-formatted paragraph not copied. Exit 0 clean. | logs/ferrite-debug-20261008-230533-T2-34.log | shots/T2-34-rendered-top.png, shots/T2-34-rendered-selected5.png, shots/T2-34-rendered-selected6.png, shots/T2-34-paste-after-copy.png |
| T2-35 | FAIL (hang) | Preview lock turned ON with the padlock. Split: Raw pane still editable with lock ON: Down moved the caret Ln 10 to Ln 11, typed Z appeared in the raw text (426 to 427 words, title gained `*`); Ctrl+Z restored it (426 words, no `*`). Rendered (V): typing Q changed nothing (426 words, no `*`). Arrow keys in Rendered: 3 Down presses with no click into the pane gave no visible scroll (before/after screenshots pixel-identical). Then a click on a blank area of the Rendered pane (not on text) and 1 Down: Ferrite froze. Log stopped advancing for more than 10 s, window stopped repainting, process still alive and using CPU. hang-dump.sh (logs/T2-35-relaunch-hang-bt.txt): main thread in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow, called from ferrite::markdown::editor::arrow_nav_sel_key <- remember_arrow_nav_sel <- show_rendered_editor. Killed with pkill -9 (exit 137). The first attempt on this row froze the same way on arrow keys in Rendered (logs/T2-35-hang-bt.txt). Persistence: after the first kill the relaunch loaded the crash-recovery state with no Session Recovery dialog (log: Found stale lock file, is_crash_recovery=true); the padlock was open when I checked it, and I turned it ON. After the second kill the lock was still ON on relaunch. Clean-quit check: lock ON, window closed with X (log: exited with code 0 clean exit). Relaunch of the same file: log shows is_crash_recovery=false, clean_shutdown=true, no dialog; padlock closed in Rendered view, so the lock is still ON (PASS for the persistence step). Toggling OFF showed the open padlock and the code-block Edit button; toggled back ON, closed padlock, Edit hidden. Fixture unchanged (git status clean; git checkout run anyway). Expected not met: arrow keys did not move or scroll the Rendered view and then froze the app. | logs/ferrite-debug-20261008-230821-T2-35.log (first freeze, exit 137); logs/ferrite-debug-20261008-231259-T2-35-relaunch-a.log (second freeze, exit 137); logs/ferrite-debug-20261008-231800-T2-35-clean-relaunch.log (clean relaunch, exit 0) | shots/T2-35-split-raw-typed-Z.png, shots/T2-35-rendered-lock-typed-Q.png, shots/T2-35-rendered-lock-on.png, shots/T2-35-fail.png, shots/T2-35-clean-relaunch-rendered.png, shots/T2-35-clean-relaunch-toggled-off.png |

## FAIL log excerpts

Each excerpt is `grep -v 'tracing::span' <log> | tail -n 30`.

### T2-32 (FAIL): logs/ferrite-debug-20261008-230020-T2-32.log
```
[2026-10-08T21:01:58Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:01:58Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:01:58Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:01:58Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:01:58Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:01:58Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:01:58Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T21:01:58Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T21:01:58Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T21:03:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T21:03:55Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T21:03:55Z INFO  ferrite::app] Application exiting
[2026-10-08T21:03:55Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T21:03:55Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=7, clean_shutdown=true
[2026-10-08T21:03:55Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T21:03:55Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T21:03:55Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 7 tabs, SingleFile)
[2026-10-08T21:03:55Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T21:03:55Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T21:03:55Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T21:03:55Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T21:03:55Z INFO  ferrite::state] Settings saved
[2026-10-08T21:03:55Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T21:03:55Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T21:03:55Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T21:03:55Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T21:03:55Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:03:55Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T21:03:55Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 23:03:55 CEST ===
```

### T2-35 (FAIL hang, first freeze): logs/ferrite-debug-20261008-230821-T2-35.log
```
[2026-10-08T21:09:29Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:29Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:30Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:30Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:30Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:30Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:31Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:31Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:31Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:31Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:31Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:31Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:32Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:32Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:32Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:32Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:32Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:32Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:09:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:09:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:09:33Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T21:09:33Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T21:09:33Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 23:12:42 CEST ===
```

### T2-35 (FAIL hang, second freeze): logs/ferrite-debug-20261008-231259-T2-35-relaunch-a.log
```
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:33Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:33Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:14:34Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:14:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T21:14:34Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 131 }) size=Physical(PhysicalSize { width: 566, height: 615 })
[2026-10-08T21:14:34Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T21:14:34Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 23:16:11 CEST ===
```

## Pasted output: T2-34 clipboard (Part A bash block, Copy button, read with `xclip -selection clipboard -o` while Ferrite was still running)

Source: `sed -n 12,15p test_md/test_copy_and_split_undo.md`. `cmp` against the xclip output: byte-identical (173 bytes, 4 lines, trailing newline, no inserted or trailing spaces). This is the Copy-button path only, not the drag-select path that the row asked for, so the row stays BLOCKED. The first capture from the earlier launch was not saved to a file, so this output is from the re-run.
```
sudo mkdir -p /opt/stacks/notesnook
sudo mkdir -p /srv/docker-data/notesnook/mongo
sudo mkdir -p /srv/docker-data/notesnook/minio
sudo mkdir -p /srv/backups-local/notesnook
```

## Final state

- Ferrite quit cleanly with the window X: last launch (logs/ferrite-debug-20261008-231936-T2-34-paste.log) reports `exited with code 0 clean exit`. `pgrep -a -x ferrite` is empty.
- `git -C /workspace/ferrite-qa/ferrite status --short`: `?? test_md/zz_cjk_search.md` (pre-existing, left alone). No code, commit or fixture changes from this batch.
- Rows reached: T2-30 through T2-35, all six. No rows skipped.

## Batch 5b (Tier 2 rows 37-40)

Tester: Grok Bot. Commit under test: 8e6ef50 (`git status --short` before run: only `?? test_md/zz_cjk_search.md`, left alone). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW>`. Screenshots: `shots/`. Logs: `logs/`. Second-terminal steps were run from the Shell tool (separate process, same DISPLAY). Raw view only (no Rendered/Split, no arrow keys in Rendered).

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| T2-37 | PASS (play-video sub-step BLOCKED) | Primary launched with test_emoji.md and test_tables.md; session restore brought back 9 earlier tabs (incl. test_emoji.md and Untitled) and test_tables.md was added, so window 1 had 10 tabs (Raw). Ctrl+Shift+N opened a 2nd OS window in the same process (one PID 488518; xdotool lists 2 Ferrite windows; log: 'Created document window 1'). First Ctrl+Shift+N press did not register (focus issue); the second did. Focus on window 2 (Untitled), then from a second shell `time $FER /tmp/lf-test.md`: returned in 0.088 s, rc 0. `lf-test.md` landed as a new tab in window 2 (the last-focused window), window 2 became active and titled `lf-test.md - Ferrite`, status bar 'Opened file from external request'; window 1 unchanged. Log: 'Received 1 path(s) from secondary instance' then 'Opened file from secondary instance: /tmp/lf-test.md'. Play-video sub-step: BLOCKED, not attempted (YouTube embed crash, see T2-36). Closed window 2 with its title-bar X: closed at once, no save prompt (log: 'Closed document window 1'). An extra empty Untitled window also appeared during the run (log 'Ribbon: New Window' at 21:23:35Z, then Ctrl+Shift+N 'Created document window 3'); I closed window 3 with its X so the quit state had 2 windows. Quit: Ctrl+Q has no effect in this build (no binding in src) and the File menu has no Quit item (locales/en.yaml has no quit label), so with 2 windows open I used Alt+F4 on the primary (root) window: log 'Closing root viewport', 'Application exiting', 'Session state saved ... (2 windows, 11 tabs)', exit code 0 clean. Relaunch with `run-ferrite.sh --tag T2-37-relaunch` (no file args): both windows restored with their tabs. Window A: 10 tabs in saved order (Untitled, vid-vimeo-only.md, ctrl-plain.md, crlf-video.md, test_find_arrow_nav.md, test_html_entities.md, test_emoji.md [active], test_local_images.md, test_copy_and_split_undo.md, test_tables.md). Window B: 1 Untitled tab. Log: 'Restored 11 of 11 tabs from session (2 windows)'. Observation only (not a FAIL): the log's pre-scan says 'Restoring 14 tab(s)' and lists 14 paths, including /tmp/lf-test.md (closed earlier in this run) and four video-embed docs (test_video_embeds.md, vid-1-min.md, yt-many.md, yt-live.md) that are not restored; the final count and the UI match the saved session. | logs/ferrite-debug-20261008-232204-T2-37.log (primary, quit 21:26:25Z); logs/ferrite-debug-20261008-232639-T2-37-relaunch.log (restore) | shots/T2-37-forward-window2.png (after forward, lf-test.md tab in window 2), shots/T2-37-pass.png (restored state) |
| T2-38 | PASS | Running primary (relaunch instance, PID 497368, window 1 active on test_emoji.md, Raw). `time $FER /tmp/lf-test.md` from a second shell: real 0.100 s, rc 0, no new window or process left running. `lf-test.md` opened as a new tab (last tab, active) in the existing window, which was raised and became active (`xdotool` active window: `lf-test.md - Ferrite`); status bar 'Opened file from external request'. Log: 'Received 1 path(s) from secondary instance', 'Opened file from secondary instance: /tmp/lf-test.md'. Then closed that tab with its x (log: 'Closed tab 25'). Then `cd /tmp && time $FER ./lf-test.md`: real 0.066 s, rc 0; the RELATIVE path opened as a tab (log resolves it to /tmp/lf-test.md; same 'Opened file from secondary instance'). No 'file not found'. Relative-path known gap: PASS. Forwarding process logs are empty by design (launcher note). | logs/ferrite-debug-20261008-232639-T2-37-relaunch.log (primary; forwarded processes log nothing) | shots/T2-38-pass.png (absolute forward, tab open), shots/T2-38-relative-pass.png (relative forward, tab reopened) |
| T2-39 | BLOCKED (Alt+Z sub-step); tab menu New / Close / Copy Path / Reveal and drag-reorder PASS | Window 2 had 3 tabs (Untitled, test_wrap_debug.md, test_horizontal_scroll.md) from forwarding two long-line fixtures. Right-click tab menu: New Tab (Ctrl+T), Close Tab (Ctrl+W), Copy Path, Reveal in Explorer. New Tab: PASS, new Untitled tab appended and active (log 'Created new tab 29 in window 2'). Close Tab on test_horizontal_scroll.md: PASS, closed with no prompt (log 'Closed tab 28'). Copy Path on test_wrap_debug.md: PASS, `xclip -selection clipboard -o` returned `/workspace/ferrite-qa/ferrite/test_md/test_wrap_debug.md` (exact, no trailing newline). Reveal in Explorer: PASS, thunar opened on `/workspace/ferrite-qa/ferrite/test_md/` (I closed that thunar afterwards). Drag reorder: PASS on the 3rd try (order Untitled, test_wrap_debug.md, Untitled became Untitled, Untitled, test_wrap_debug.md; log 'Reordered tabs: 2 <-> 1'); the first two Computer drags (one fast two-point, one seven-point) did not reorder and logged nothing. Alt+Z: BLOCKED after 4 attempts. Attempts 1-3 with the Raw editor focused: the key arrived as a plain 'z' (literal z typed into the unsaved test_wrap_debug.md buffer, removed again with BackSpace; file never saved, git status unchanged; Ctrl+Z did nothing) and no word-wrap change followed. Attempt 4 with focus on the tab strip: no text inserted, no visible change. No 'Word wrap' toggle in the log; config.json word_wrap still True. Ferrite did not crash or hang. Not a FAIL: I could not get Alt+Z to reach the shortcut handler through the Computer key tool, so the toggle is unverified. | logs/ferrite-debug-20261008-232639-T2-37-relaunch.log | shots/T2-39-contextmenu.png, shots/T2-39-newtab-pass.png, shots/T2-39-close-pass.png, shots/T2-39-copypath-after.png, shots/T2-39-reveal-pass.png, shots/T2-39-reorder-pass.png, shots/T2-39-altz-before.png, shots/T2-39-altz-attempt4.png |
| T2-40 | SKIPPED (platform: Windows title bar row) | Windows-only title bar row. Linux title bar is covered by TTL-6 (Batch 3). Not run. | — | — |

## Batch 5b notes

- Quit method: Ctrl+Q has no binding in this build (no match in `src/`), and the File menu has no Quit item (no quit label in `locales/en.yaml`). With two windows open I quit with Alt+F4 on the primary (root) window, which exits the app with the session saved for both windows (log: 'Closing root viewport (ViewportCommand::CancelClose was not sent)', 'Session state saved ... (2 windows, 11 tabs)', exit code 0). The T2-37 relaunch was started only after that.
- No FAIL in this batch, so no log excerpts are needed. No hang, no panic (no exit 134 or 137 in the T2-37 and T2-37-relaunch logs).
- Window-management note: the first click on a Ferrite title bar started a window move (the window shifted by about 350 px). I avoided title bars after that.
- Runs: T2-37 primary `logs/ferrite-debug-20261008-232204-T2-37.log` (exit 0, 21:26:25Z); T2-37-relaunch `logs/ferrite-debug-20261008-232639-T2-37-relaunch.log` (exit 0 at 23:34:24 CEST, used for T2-38 and T2-39).

## Final state (Batch 5b)

- Ferrite quit: exit code 0 clean (relaunch instance, 23:34:24 CEST). `pgrep -a -x ferrite`: empty.
- `git -C /workspace/ferrite-qa/ferrite status --short`: `?? test_md/zz_cjk_search.md` (pre-existing, left alone). No code, commit, or fixture changes. `test_wrap_debug.md` was briefly dirtied by stray 'z' keystrokes during the T2-39 Alt+Z attempts, removed before any save; git confirms the file is unchanged.
- Rows reached: T2-37, T2-38, T2-39, T2-40 (skipped). All four rows covered.
