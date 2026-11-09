# Ferrite v0.3.1 QA - Batch 6a (Tier 3 rows 41-47)

Tester: Grok Bot. Commit under test: 8e6ef508 (`0.3.1`). Pre-run `git status --short`: `?? test_md/zz_cjk_search.md` (left alone). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <file-or-folder>`. Screenshots: `shots/`. Logs: `logs/`.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| T3-41 | PARTIAL | Named fixture test_tables.md, Rendered: the only alignment table ("Table with Alignment") has Left/Center/Right headers but its separator is left-left-left (colon on the left only), so there is NO centred `:-:` or right `--:` column to verify, and no bold/code/link cells, so the inline-formatting check cannot be done on this file. EDIT STEP PASS: in Rendered clicked into the Center-column cell "Text" and typed Z; cell shows "TextZ", table layout intact, tab gained `*`. Raw (re-checked after a Rendered round trip) shows line 42 "Text / TextZ / Text" with separator row intact. Observation: the first Raw switch right after typing showed the old text and no `*` until a Rendered round trip; the edit did reach the buffer. Fixture discarded via Don't Save; git status clean afterwards. SUPPLEMENTARY (other repo fixtures, read-only): test_table_wrapping.md "Alignment with Long Text" Rendered: left column left, centre column centred, right column flush right (`:-:` and `--:` render correctly). task_50_table_inline_formatting.md Rendered: bold, italic, strikethrough, bold+italic, inline code, strike+code and plain links in cells render styled. FINDING: cell `[**bold link**](https://example.com)` renders as underlined link text with literal `**` asterisks visible (bold inside link not rendered). | logs/ferrite-debug-20261008-233605-T3-41.log (edit run, exit 0); supplementary logs/ferrite-debug-20261008-233832-T3-41.log, logs/ferrite-debug-20261008-233908-T3-41.log | shots/T3-41-partial-rendered-alignment.png, shots/T3-41-partial-raw-after-edit.png, shots/T3-41-supp-alignment-center-right-table_wrapping.png, shots/T3-41-supp-inline-format-task50.png |
| T3-42 | PASS | Rendered (V) CSV grid, test_data.csv: double-clicked row 1 city cell "New York", End, typed X (cell "New YorkX", tab marked `*`, 743 chars). Tab moved focus to the next cell (occupation "Software Engineer", outlined). Ctrl+Z reverted the edit to "New York", `*` cleared, 742 chars, status bar "Undo (0 remaining)". Fixture never saved (git status clean; line 2 still "New York"). Window closed without a prompt. | logs/ferrite-debug-20261008-233959-T3-42.log | shots/T3-42-pass.png |
| T3-43 | FAIL | Rendered, both files (no errors or crash; all mermaid blocks render, no error boxes). OVERLAPS found: (1) test_flowcharts.md "Subgraph Test (TD)": the Node 2 to Node 3 arrow runs through the "Inner Group" title text. (2) "Edge Routing Across Subgraph Boundaries (TD)": the Start to Step 1 arrow crosses the "Process A" title and the Step 2 to Step 3 arrow crosses the "Process B" title. (3) "Nested with Direction Override": the Input Stage box overlaps the Main Process container and hides its title, and Output Stage sits outside Main Process. (4) "Asymmetric Shape with Dash-Style Edge Labels": the "Two line / edge comment" label is clipped behind the Result One box and hides the Odd shape to Result One arrow; the "Simple label" text on the Odd shape to Result Two edge is not drawn; node "Flag" (A) and its dotted edge to Target are not drawn. OTHER MISMATCHES vs fixture Expected (not overlaps): "Nested Subgraph Test (Sibling Subgraphs)": `one --> two` shows as two extra boxes labelled "one" and "two" instead of linking the subgraphs; direction LR inside First and Second not applied (A/B stacked). "True Nested Subgraphs": Inner Container is beside, not inside, Outer Container. "Deeply Nested Subgraphs (3 Levels)": three sibling boxes touching, not nested. test_git_graphs.md: GG-01 and GG-02 and GG-03 render with no node or label overlap; lane and commit labels are ellipsised (Mainl..., Merg..., Featu..., cherr...). GG-02 feature lane forks from HF1 (source creates feature after HF1), while the fixture Expected text says both diverge from BASE. GG-03 dashed cherry-pick connector not clearly distinguishable at this zoom. Clean (no overlap seen): Coffee Machine TD, Chapter Flow LR, Simple Decision TD and LR, Edge Routing (LR), Complex Cross-Subgraph, Asymmetric Shape Test, All Node Shapes (fullscreen button overlaps the last Subroutine shape slightly), linkStyle Edge Styling / Default / Mixed / Invalid Index, YAML frontmatter x2, Empty frontmatter. | logs/ferrite-debug-20261008-234101-T3-43.log (excerpt in FAIL section below) | shots/T3-43-fail-subgraph-test-td-title-overlap.png, shots/T3-43-fail-edge-routing-td-title-overlap.png, shots/T3-43-fail-nested-direction-override-overlap.png, shots/T3-43-fail-dash-edge-label-overlap.png, shots/T3-43-gitgraph-gg01-02.png, shots/T3-43-gitgraph-gg03.png, shots/T3-43-flowchart-subgraph-td.png, shots/T3-43-flowchart-complex-cross.png |
| T3-44 | FAIL | Rendered test_github_html.md. PASS parts: multi-line `<div align="left/center/right">` blocks render correctly (center paragraph centred, right flush right, inline italic and code kept). `<details>` closed by default and `<details open>` expanded on load; clicking the summary expands and collapses it (verified on "Click to expand", expanded body shows nested list and code block; "Section A" back-to-back expands; rich summary "Bold summary" expands with its body). `<kbd>` renders boxed key caps (Ctrl, Ctrl+Shift+P chord, Ctrl+S in a sentence with bold, arrow glyphs). FAIL part: under "Adjacent divs" the two single-line blocks `<div align="center">First centered block.</div>` and `<div align="right">Second block, right-aligned.</div>` are NOT rendered; their text is missing and the document's first line "A# GitHub HTML Subset Test" appears in their place (twice, one indented). The repo docs (docs/technical/markdown/github-html-block-subset.md, single-line div form; test name test_html_single_line_div_align) say this form is supported, so this is a defect. Minor observation: `<strong>` inside a summary is shown as small grey tag chips and the summary text is not bold. No crash, no hang. | logs/ferrite-debug-20261008-234543-T3-44.log (excerpt in FAIL section below) | shots/T3-44-fail-adjacent-single-line-divs.png, shots/T3-44-pass-kbd.png, shots/T3-44-adjacent-divs-check.png |
| T3-45 | PASS | Rendered test_code_execution.md, section 2c (Python block `print("Hello from Python")`): Run button clicked; consent dialog "Allow running code from this document?" appeared (buttons Enable & run / Just enable / Cancel); chose "Enable & run". Output attached under the block: "Exit 0 · 21ms" with "Hello from Python", plus Insert as block / Copy / Dismiss. Then Raw: clicked end of line 4 (above the fence) and typed " EDITED" (tab marked `*`). Back in Rendered the output is still attached under the same 2c Python block (Exit 0, 21ms). Fixture was not saved (closed with Don't Save, then git checkout). No crash, no hang. | logs/ferrite-debug-20261008-234736-T3-45.log (exit code in log line at close) | shots/T3-45-pass-output-attached-after-edit.png |
| T3-46 | PASS | New untitled doc (Raw, Ctrl+N; Ferrite reused the empty Untitled tab). Typed `# Top heading`, three filler paragraphs with blank lines, then line 19 `## 1\.3\.1 Title` (literal backslashes kept in source, shown as typed) and line 20 body text. Opened the Outline panel with Ctrl+Shift+O (panel shows "Document / Outline": H1 "Top heading", H2 "1.3.1 Title"; the entry text has no backslashes). Caret moved to line 2; clicking the "1.3.1 Title" outline entry moved the caret to "Ln 19, Col 1" (status bar) and the toolbar showed H2. Did not save. Note: closing the modified Untitled doc with Alt+F4 gave NO Unsaved Changes prompt (log: CloseRequested then closing root viewport, exit 0); the untitled text went only into the crash-recovery session file (path=None) and nothing was written to disk or to the repo. The Don't Save step therefore could not be exercised as written. | logs/ferrite-debug-20261008-234912-T3-46.log (see exit line at close) | shots/T3-46-pass-outline-jump.png |
| T3-47 | PARTIAL | Launch 1 (workspace /workspace/ferrite-qa/fixtures): clicking sample.pdf in the file tree opened a PDF viewer tab "sample.pdf" showing "Ferrite QA PDF" (page 1/1); log "Opened PDF viewer". PASS. Clicking sample.zip: log "Opened in default application: .../sample.zip" plus "Delegated file tree selection to external app"; no Ferrite tab opened. The box's default zip handler is Chrome (xdg-mime: com.google.Chrome.desktop), which raised a Chrome window and saved the file to ~/Downloads/sample.zip (113 B) instead of showing it. The Ferrite toast ("Opened in default application", 2 s per source in state.rs) was NOT captured in any screenshot: Chrome came to the front over Ferrite on every zip click (3 clicks in total). Launch 2 (workspace /workspace/ferrite-qa/ferrite): assets > icons > icon-192x192.png opened an image viewer tab "icon-192x192.png" showing the 192x192 orange gem icon (log "Opened image viewer"). PASS. portable > FerriteMDPortable > FerriteMDPortable.exe: log "Opened in default application" plus "Delegated file tree selection to external app"; no crash and no text tab was opened (tabs were only Untitled*, sample.pdf, icon-192x192.png). Chrome downloaded it to ~/Downloads/FerriteMDPortable.exe (142 KB, Done; not run). The toast was again not captured because Chrome raised over Ferrite. Both launches exited with code 0 (Alt+F4). Environment notes: no Session Recovery dialog appeared; launch 2 restored the Untitled* recovery doc from T3-46 and a second window test_wrap_debug.md from the saved session (left alone). During the run Ferrite's window was found minimised (WM hidden state) after a re-click; it was restored with xdotool windowactivate, and no data was lost. Other agents' terminal windows are visible on this display; none were typed into. | logs/ferrite-debug-20261008-235054-T3-47.log (launch 1, exit 0 at 23:53:34); logs/ferrite-debug-20261008-235343-T3-47.log (launch 2, exit 0 at 23:55:10) | shots/T3-47-pass-pdf-viewer.png, shots/T3-47-pass-image-viewer.png |

## FAIL log excerpts (`grep -v 'tracing::span' <log> | tail -n 30`)

### T3-43 (`logs/ferrite-debug-20261008-234101-T3-43.log`)

```
[2026-10-08T21:45:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:45:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:45:35Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T21:45:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:45:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:45:35Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1060, height: 641 })
[2026-10-08T21:45:35Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T21:45:35Z INFO  ferrite::app] Application exiting
[2026-10-08T21:45:35Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T21:45:35Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=2, tabs=9, clean_shutdown=true
[2026-10-08T21:45:35Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T21:45:35Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T21:45:35Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (2 windows, 9 tabs, SingleFile)
[2026-10-08T21:45:35Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T21:45:35Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T21:45:35Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T21:45:35Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T21:45:35Z INFO  ferrite::state] Settings saved
[2026-10-08T21:45:35Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T21:45:35Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T21:45:35Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T21:45:35Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T21:45:35Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 23:45:35 CEST ===
```

### T3-44 (`logs/ferrite-debug-20261008-234543-T3-44.log`)

```
[2026-10-08T21:47:29Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:47:29Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:47:29Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T21:47:29Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:47:29Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:47:29Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1060, height: 641 })
[2026-10-08T21:47:29Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T21:47:29Z INFO  ferrite::app] Application exiting
[2026-10-08T21:47:29Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T21:47:29Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=2, tabs=10, clean_shutdown=true
[2026-10-08T21:47:29Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T21:47:29Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T21:47:29Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (2 windows, 10 tabs, SingleFile)
[2026-10-08T21:47:29Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T21:47:29Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T21:47:29Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T21:47:29Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T21:47:29Z INFO  ferrite::state] Settings saved
[2026-10-08T21:47:29Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T21:47:29Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T21:47:29Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T21:47:29Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T21:47:29Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 23:47:29 CEST ===
```

## Batch end

- Quit: Ferrite quit with Alt+F4 (last instance, log `logs/ferrite-debug-20261008-235343-T3-47.log`, exit code 0). `pgrep -a -x ferrite` afterwards: empty.
- `git -C /workspace/ferrite-qa/ferrite status --short` at the end: `?? test_md/zz_cjk_search.md` (pre-existing, left alone). No repo fixture changes remain.
- Files written by the OS handler (not by Ferrite) into `~/Downloads` during T3-47: `sample.zip`, `sample (1).zip`, `FerriteMDPortable.exe`. They are outside the repo.

## Batch 6b (Single instance SI-1..5)

Tester: Grok Bot. Binary: `ferrite 0.3.1` at `/workspace/ferrite-qa/ferrite/target/release/ferrite`, HEAD 8e6ef50. Pre-run `git status --short`: `?? test_md/zz_cjk_search.md` (pre-existing, left alone). Primary launched with `run-ferrite.sh --tag <ROW>`. Session restore reopened the previous tabs and a second window (test_wrap_debug.md); no Session Recovery dialog was seen. Display :2, WM focus was checked with xdotool. Screenshots: `shots/`. Logs: `logs/`. Forwarded (secondary) processes exit before logging starts, so their logs are empty by design; evidence for them is in the primary's log and in timing and rc.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| SI-1 | FAIL | Primary running (`run-ferrite.sh --tag SI-1 /tmp/lf-test.md`, one process, instance.pid = primary PID). `time $FER /tmp/lf-test.md` from a shell: real 0.095 s, rc=0, no 2nd process left. Tab part PASS: primary log says "Received 1 path(s) from secondary instance", "File already open, switching to window 0 tab 3"; no duplicate tab, status bar "Opened file from external request". Window-raise part FAIL: before the forward I activated the test_wrap_debug.md window (xdotool windowactivate) so it sat on top; after the forward the active window was still "test_wrap_debug.md - Ferrite" and the lf-test.md window (window 0) stayed behind it. Primary log shows only `winit::Window::request_user_attention; request_type=Some(Informational)` (x2), no raise. Reproduced once more: real 0.080 s, rc=0, same result. | logs/ferrite-debug-20261008-235629-SI-1.log (primary; secondary runs write no log) | shots/SI-1-fail.png |
| SI-2 | PASS | Closed the clean lf-test.md tab (Computer click on its X; the window had been raised by a title-bar click first). `cd /tmp && time $FER ./lf-test.md`: real 0.087 s, rc=0, no 2nd process left. Relative path resolved: primary log "Received 1 path(s) from secondary instance", then "Opened file: /tmp/lf-test.md (encoding: UTF-8, auto-save: false, view_mode: Raw)". No "file not found". Tab reopened with the fixture content (status bar /tmp/lf-test.md, 10 words, 62 chars, 8 lines). | logs/ferrite-debug-20261008-235629-SI-1.log (primary, lines around 3149; secondary writes no log) | shots/SI-2-pass.png |
| SI-3 | PASS | `run-ferrite.sh --tag SI-3 -- --new-instance &`: 2nd independent window ("ctrl-plain.md - Ferrite" title, with the Session Recovery dialog) and 2nd process: `pgrep -a -x ferrite` showed 553418 (primary, `/tmp/lf-test.md`) and 556397 (`--new-instance`). `ls -la ~/.config/ferrite/`: instance.lock, instance.port (35031) and instance.pid all still dated 23:56 (primary's). `cat instance.pid` = 553418 = primary PID (unchanged). Closed the new-instance window with Alt+F4: exit code 0, primary 553418 still running, instance.pid still 553418. SIDE FINDINGS (not part of the pass criteria, reported for triage): (a) the --new-instance process found the primary's session.lock, logged "Found stale lock file - previous session may have crashed", and showed a Session Recovery? dialog; I clicked Start Fresh as instructed, which logged "User discarded crash recovery" / "Cleared all session recovery data" (the primary's crash-recovery data was cleared). (b) On its exit it logged "Removed session lock file" (session.lock belonging to the primary is gone) and overwrote the shared session.json and config.json (SingleFile, 25 tabs) in ~/.config/ferrite. | logs/ferrite-debug-20261008-235629-SI-1.log (primary, still running); logs/ferrite-debug-20261008-235858-SI-3.log (new instance, exit 0 at 23:59:54) | shots/SI-3-pass.png (new window + recovery dialog), shots/SI-3-pass-closed.png (primary after close) |
| SI-4 | PASS | Settings (gear button; Ctrl+, did not open it) -> Files filter. Label exactly "Allow multiple Ferrite windows/instances" (tooltip: "each launch opens a new Ferrite window instead of forwarding files..."), unchecked by default. Toggled ON: config.json showed `allow_multiple_instances: true` only after a later save event (see note). `$FER /tmp/lf-test.md &`: no forwarding, 2 PIDs (553418 primary, 561273 new) and the new process's window showed "lf-test.md - Ferrite"; its Session Recovery? prompt was answered with Start Fresh. Alt+F4 on the 2nd window (PID 561273 only) -> 2nd process gone, primary 553418 alive. Toggled OFF and config.json showed `false`. `time $FER /tmp/lf-test.md`: real 0.119 s, rc=0, 1 process only; primary log "Received 1 path(s) from secondary instance" then "File already open, switching to window 0 tab 3" (forwarding restored). NOTE: the setting is written to config.json only when the app next calls save_settings_if_dirty (file open or close, shutdown); the periodic eframe save did not run (no "Saving application state" in the log), so after toggling, config.json stayed stale for 2+ min until I opened README.md (ON) and ROADMAP.md (OFF) from the file tree. Possible minor persistence latency, not a row failure. | logs/ferrite-debug-20261008-235629-SI-1.log (primary; config saves at 22:02:30 (ON) and 22:04:07 (OFF)); logs/si4-secondary-ON-stderr.out (2nd process stderr, WARN lines only); no log file for the 2nd process by design | shots/SI-4-setting-on.png (toggle ON, Files section), shots/SI-4-setting-persisted.png (after save, primary window) |
| SI-5 | PASS | `kill -9 $(cat instance.pid)` (primary 553418): exit code 137 (SIGKILL, expected for this row, not a panic). Stale files left behind: `ls ~/.config/ferrite/` showed instance.lock, instance.port (35031) and instance.pid (553418), plus session.lock removed. `run-ferrite.sh --tag SI-5 /tmp/lf-test.md &`: the new primary (PID 564348) started at once over the stale files, no hang; window present at my first check (about 12 s after launch; exact start time not measured); instance.pid updated to 564348 and instance.port to 44957 (new lock file timestamp 00:04:47). Session Recovery? prompt appeared (recovery from the killed run, recovery.session.tabs_unsaved), answered Start Fresh; window then showed lf-test.md active (status /tmp/lf-test.md). Forward check `time $FER /tmp/lf-test.md`: real 0.089 s, rc=0, 1 process only (564348), primary log "Received 1 path(s) from secondary instance" then "File already open, switching to window 0 tab 13". | logs/ferrite-debug-20261009-000447-SI-5.log (new primary); logs/ferrite-debug-20261008-235629-SI-1.log (killed primary, ends with the SIGKILL exit line); logs/launcher-SI-5.out | shots/SI-5-pass.png (Session Recovery prompt on the new primary, taken before Start Fresh) |

## FAIL log excerpts (`grep -v 'tracing::span' <log> | tail -n 30`)

### SI-1 (`logs/ferrite-debug-20261008-235629-SI-1.log`)

Note: the log's tail below is from the later kill -9 run of SI-5 (the primary was killed at 22:04:39Z, exit 137). The forward events that matter for SI-1 are at 21:57:13Z and 21:57:48Z; the excerpt after the tail covers them.

```
[2026-10-08T22:04:38Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T22:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T22:04:38Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T22:04:38Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T22:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T22:04:38Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T22:04:38Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:38Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T22:04:38Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T22:04:39Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T22:04:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T22:04:39Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T22:04:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:39Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T22:04:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T22:04:39Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T22:04:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T22:04:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T22:04:39Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 00:04:39 CEST ===
```

Forwarded-open events for SI-1 (non-span lines 2731-2784 of the same log, 21:57:12Z-21:57:48Z; the 2nd forward at 21:57:48Z is the retry):

```
[2026-10-08T21:57:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T21:57:12Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T21:57:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:12Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:57:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:57:13Z INFO  ferrite::app::file_ops] Received 1 path(s) from secondary instance
[2026-10-08T21:57:13Z INFO  ferrite::state] File already open, switching to window 0 tab 3
[2026-10-08T21:57:13Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /tmp/lf-test.md
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:57:13Z DEBUG winit::window] winit::Window::request_user_attention; request_type=Some(Informational)
[2026-10-08T21:57:13Z DEBUG winit::window] winit::Window::request_user_attention; request_type=Some(Informational)
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T21:57:13Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:13Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:57:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:57:48Z INFO  ferrite::app::file_ops] Received 1 path(s) from secondary instance
[2026-10-08T21:57:48Z INFO  ferrite::state] File already open, switching to window 0 tab 3
[2026-10-08T21:57:48Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /tmp/lf-test.md
[2026-10-08T21:57:48Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T21:57:48Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=2, tabs=7, clean_shutdown=false
[2026-10-08T21:57:48Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T21:57:48Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T21:57:48Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (2 windows, 7 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T21:57:48Z DEBUG ferrite::config::session] Saved recovery content for tab 25 (path=None, hash=None)
[2026-10-08T21:57:48Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T21:57:48Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T21:57:48Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:48Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
[2026-10-08T21:57:48Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'lf-test'
[2026-10-08T21:57:48Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:48Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T21:57:48Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T21:57:48Z DEBUG winit::window] winit::Window::request_user_attention; request_type=Some(Informational)
[2026-10-08T21:57:48Z DEBUG winit::window] winit::Window::request_user_attention; request_type=Some(Informational)
[2026-10-08T21:57:48Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'test_wrap_debug'
[2026-10-08T21:57:48Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T21:57:48Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=100.0, max_wrap_width=840.0, effective=100.0
```

## Batch end

- SI-1 FAIL (window not raised); SI-2, SI-3, SI-4, SI-5 PASS (see rows). No crash or hang in any row. The `--new-instance` run and the SI-5 kill -9 run exited 0 and 137 respectively, as expected.
- Quit: primary (SI-5 run, PID 564348) quit with Alt+F4, log `logs/ferrite-debug-20261009-000447-SI-5.log`, exit code 0 at 00:06:03 CEST. `pgrep -a -x ferrite` afterwards: empty.
- Setting "Allow multiple Ferrite windows/instances": OFF (config.json `allow_multiple_instances: false`, verified after quit).
- Files left in ~/.config/ferrite/ at the end: instance.* files were removed by the clean quit (see the SI-5 log); session.json and config.json were rewritten by the quit.
- `git -C /workspace/ferrite-qa/ferrite status --short` at the end: `?? test_md/zz_cjk_search.md` (pre-existing `?? test_md/zz_cjk_search.md`, untouched; no repo changes by this batch).
- Side findings for triage (not row failures): (1) an independent `--new-instance` process treats the running primary's session.lock as stale and shows the Session Recovery prompt; "Start Fresh" logs "Cleared all session recovery data". (2) On exit an independent instance removes the primary's session.lock and overwrites the shared session.json/config.json. (3) Toggling "Allow multiple Ferrite windows/instances" did not write config.json until another save event (file open/close).
