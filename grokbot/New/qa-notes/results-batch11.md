# Ferrite v0.3.1 QA - Batch 11 (Mermaid remainder)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (v0.3.1, HEAD 8e6ef508). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <files>`. Screenshots: `shots/`. Logs: `logs/`.
Fixtures: `test_mermaid_v031.md` (MMD-1, MMD-3, MMD-4), `test_mermaid_issue_83.md` (MMD-2, FC-83b; not one of the four listed fixtures, it holds FC-83b), `test_pos_hints.md` (MMD-6), `test_mermaid_issue_165.md` (MMD-9, MMD-10, MMD-11; MMD-13 uses its CJK-labelled FC-165a because no other fixture has CJK flowchart node labels).

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| MMD-1 | PARTIAL | Instance `--tag MMD-1` (test_mermaid_v031.md + test_mermaid_issue_83.md, Rendered view; no Session Recovery dialog appeared, tabs restored silently, no Start Fresh needed). Hover over the flowchart shows the expand icon top-right with tooltip "Open diagram in popup"; clicking it OPENS the popup (titled "Mermaid Diagram", toolbar Hand / Select / zoom % / zoom in / zoom out / reset / X). Zoom in 100->119%, zoom out 100->98%, reset back to 100% (all work). Wheel-up in Hand mode zooms (->229%); wheel in Select mode does not change zoom. X closes the popup. Colours: light popup on light app theme, consistent. **Plain click on diagram body: no popup. Ctrl+click on diagram body: no popup** (tried 4 times incl. after hovering the body first). The hover hint on the FC-83b diagram in test_mermaid_issue_83.md also says "Click diagram or Ctrl+Click to open popup", and a plain click on its hovered body did not open the popup either (1 try). **Hand-pan NOT verified (BLOCKED):** 7 Computer drag attempts (2-point, 8-point, 16 and 20-point paths, pre-hover, at 100% and 229%) left the diagram in place; the cursor showed the grab hand, so the tool's press/move/release did not register as a pan. Not a code verdict; select-mode drag also not run. | logs/ferrite-debug-20261009-035014-MMD-1.log (instance covers MMD-1, MMD-2, MMD-3, MMD-4; no panic, no hang) | shots/MMD-1-partial.png; shots/MMD-1-hover-button.png |
| MMD-2 | PASS | FC-83b in test_mermaid_issue_83.md (Rendered): node F shows `Car` (source `fa:fa-car Car`), no literal `fa:` text, no icon glyph drawn; other nodes Christmas / Go shopping / Let me think / Laptop / iPhone render normally. Also test_mermaid_v031.md section 2 shows `Loading`, `Repo`, `Done` (source `fa:fa-spinner`, `fab:fa-github`, `fa:fa-check`) with no literal prefix text. | logs/ferrite-debug-20261009-035014-MMD-1.log (same instance, no panic) | shots/MMD-2-pass.png |
| MMD-3 | FAIL | Fixture section 3 (`linkStyle 0,1,2 interpolate basis`, chain A-->B-->C-->D in LR, shot MMD-3-fail.png) draws straight horizontal segments. A collinear chain would look straight even if curved, so I ran a scratch control in /tmp/qa11 (not a fixture, not in the repo): control A (same 5-edge TD graph, no linkStyle) vs control B (identical plus `linkStyle 0,1,2,3,4 interpolate basis`). The two renders are pixel-identical (28 of 154560 px differ by >40 levels), both with square elbow corners, so the indexed form has no visible effect. Control C (`linkStyle default interpolate basis`, same graph, scratch file) DOES draw smooth curved edges, so the `default` form works and the indexed form the fixture uses does not. | logs/ferrite-debug-20261009-035014-MMD-1.log (main instance; scratch control files opened by forwarding into it; forwarded-instance logs ferrite-debug-20261009-035705-MMD-3-forward.log and ferrite-debug-20261009-035748-MMD-3-forward2.log are near-empty by design) | shots/MMD-3-fail.png (fixture), shots/MMD-3-control-AB-crop.png (A vs B, identical), shots/MMD-3-controlC.png (default form, curves) |
| MMD-4 | PASS | Fixture section 4 (`subgraph S1[This subgraph title is intentionally much wider than its nodes]`, shot MMD-4-pass.png). Measured the S1 box's olive border width from screenshots (physical px): inline 399 px in 3 static frames (1 scroll, 1 hover) and in 12 frames captured while scrolling down/up over the diagram; identical every frame (no flicker/oscillation). In the popup (opened via the expand button): 489 px at 100%, 537 px after one zoom-in and 586 px after a second zoom-in (2 frames each, identical), 489 px again after Reset. Title stays inside its box at 100%. Note outside the width criterion: the second subgraph `Short` (node one / two / three) overlaps the S1 box and its title `Short` is not visible at all in inline or popup view (only the S1 title shows); S1 and S2 also render as plain nodes inside the S1 box instead of as subgraph references. Flagged for triage, not scored under MMD-4. | logs/ferrite-debug-20261009-035014-MMD-1.log (same instance, no panic) | shots/MMD-4-pass.png (inline), shots/MMD-4-popup-reset.png (popup at 100%), shots/MMD-4-popup-z2.png (popup zoomed) |
| MMD-6 | FAIL | Fixture test_pos_hints.md, Rendered (shot MMD-6-fail.png). **Valid hints:** `Start` (A) sits at the pinned top-left position, `End` (C) at the pinned lower-right position (relative to the canvas margin), `Middle` (B) stays auto; hints are applied. The A-->B edge runs along Start's bottom border before entering Middle, a cosmetic blemish. **Invalid hint:** the amber header "Line 3: Unknown node id 'Ghost' in @pos hint" with the hint text appears, but the diagram itself does NOT render: the block shows the raw source (`flowchart TD` / `X[Only node]`) as plain monospace text, no node box. Scratch control in /tmp/qa11/mmd6-control.md (not a fixture): the same single node without the bad @pos line renders a box ("Only node"), and a two-node graph with the bad `%% @pos Ghost` line shows the same warning plus the raw source text instead of the graph. So the unknown-id warning itself triggers the source fallback. Contradicts "diagram still renders". | logs/ferrite-debug-20261009-035014-MMD-1.log (main instance; test_pos_hints.md opened 02:01:10Z via forwarding; forwarded-instance log logs/ferrite-debug-20261009-040110-MMD-6-forward.log is near-empty by design; control log logs/ferrite-debug-20261009-*-MMD-6-control-forward.log) | shots/MMD-6-fail.png (fixture), shots/MMD-6-control.png (scratch control) |
| MMD-9 | PASS | `test_mermaid_issue_165.md` FC-165a, `subgraph 业务客户端 PEP`. In the popup at 100% (Native mode) the title renders in full, left-aligned in its frame: "业务客户端 PEP", no truncation or ellipsis. Inline (Fit width) the title is also complete, just small. Side note for MMD-11: the neighbouring title 统一接入层 is partly hidden where its frame overlaps the 业务客户端 PEP frame. | logs/ferrite-debug-20261009-035014-MMD-1.log (main instance; test_mermaid_issue_165.md was opened into it, see the 02:08:01Z line; forward log logs/ferrite-debug-20261009-040236-MMD-9-11-13-forward.log is near-empty by design). No panic. | shots/MMD-13-firstpaint-fit.png (inline, title visible); shots/MMD-11-popup-native-100pct.png (popup 100%, full title) |
| MMD-10 | PARTIAL | FC-165b in `test_mermaid_issue_165.md`. Badges run 1 to 10 in source order: 1 to 7 are the pre-alt messages, 8 is the alt branch (返回认证失败), 9 and 10 are the else branch (推送标准化可信身份结构体, 进入策略授权决策流程). The alt label 认证失败 and else label 认证成功 are shown, and the participant aliases appear under the headers. Cosmetic: badges 5 and 6 sit on top of the message text, and the first message label runs into the participant header boxes. Control (scratch file /tmp/qa11/mmd10-autonumber2.md, not a fixture): `autonumber 10 10` gives badges 10, 20, 30 (correct). `autonumber off` as the last directive removes all badges, including the ones before it. The repo doc docs/technical/mermaid/sequence-autonumber-rendering.md says only the last top-level directive applies and mid-diagram `off` is out of scope, so "off stops numbering from that point" is not implemented. Verdict PARTIAL: the FC-165b numbering passes; the `off` case only works as a whole-diagram switch. | logs/ferrite-debug-20261009-035014-MMD-1.log (FC-165b in main instance). Control: logs/ferrite-debug-20261009-040706-MMD-10-control.log and logs/ferrite-debug-20261009-040632-MMD-10-control-forward.log (forwarded, near-empty by design). No panic. | shots/MMD-10-partial-fc165b-inline.png (FC-165b, 1-10); shots/MMD-10-partial-control-autonumber-10-10-and-off.png (control A 10/20/30, control B off = no badges) |
| MMD-11 | PARTIAL | FC-165a dense fan-out, checked in the popup at 100% and 68% (Native mode) and inline. No two node boxes overlap. The `&` fan-outs (A1 to A5 into B1/B2, D row into E row) spread into separate lanes with no stacking. No edge crossing a node body was seen; edges that run alongside subgraph borders look like frames, which makes the diagram harder to read. `<-->` edges show arrowheads at both ends: C2/C3 to the D row (checked at 100%, arrowheads on both C2 and C3 ends) and D1/D2/D4/D5 to the E row (checked at 68%, arrowheads on both the D and E ends). Reason for PARTIAL: the subgraph frames 业务客户端 PEP and 统一接入层 overlap. The 统一接入层 frame starts inside the 业务客户端 PEP frame, its title sits inside it, and the right border of the 业务 frame is hidden under it. This is not node-on-node overlap, but it breaks the FC-165a "no overlap" pass criterion. Also: the FC-165a popup did not open from the inline expand icon in Fit mode (3 tries: click, hover+click, Ctrl+click on the body), but it did open on the first expand-icon click after toggling to Native width. | logs/ferrite-debug-20261009-035014-MMD-1.log (main instance, no panic); forward log logs/ferrite-debug-20261009-040236-MMD-9-11-13-forward.log near-empty by design | shots/MMD-11-inline-fc165a.png (inline Fit); shots/MMD-11-popup-native-100pct.png (popup 100%); shots/MMD-11-popup-68pct.png (popup 68%) |
| MMD-13 | FAIL | Only fixture with CJK flowchart node labels: FC-165a in `test_mermaid_issue_165.md` (test_flowcharts.md has none; checked with rg). Measured on the popup 100% screenshot in physical pixels (box outer width vs. dark text ink width): A-row 堡垒机运维系统 196 vs 116 px (box 1.69x text), SDP零信任访问 192 vs 112 (1.71x), AI Agent命令代理 212 vs 130 (1.63x). So the A-row boxes are about 65 to 70 percent wider than the text, which is the defect the row describes. Other rows: REST API 415 vs 305 (1.36x), 多模型适配中枢 424 vs 329 (1.29x), OAuth2/OIDC 427 vs 409 (1.04x), Rust-Regorus 423 vs 405 (1.04x). So the sizing is inconsistent: some boxes hug, the A row and part of the B/C rows do not. Layout check: first paint (Fit width) and after scrolling away and back (wheel 25 clicks down, then outline jump back) are identical. Pixel diff over the diagram region: 26 of 748,000 pixels differ, all at the document scrollbar edge at the right. Layout is stable; box sizing is not. | logs/ferrite-debug-20261009-035014-MMD-1.log (main instance, no panic) | shots/MMD-13-firstpaint-fit.png (first paint); shots/MMD-13-scrolled-away.png (scrolled away); shots/MMD-13-after-scroll-back.png (after scroll back); shots/MMD-11-popup-native-100pct.png (popup 100%, used for box measurements) |

## Notes

- Instances: one Ferrite instance (log logs/ferrite-debug-20261009-035014-MMD-1.log) covered MMD-1 to MMD-4, MMD-6, MMD-9, MMD-10, MMD-11, MMD-13 (fixtures forwarded into it). Forwarded and control launches (logs/*-MMD-*-forward*.log, logs/*-MMD-10-control*.log) are near-empty by design, since the file opens in the running instance.
- Session Recovery: no dialog appeared in this instance. About 40 tabs from an earlier session were restored silently, so Start Fresh was not used.
- Scratch controls (not fixtures, nothing saved to the repo): /tmp/qa11/mmd10-autonumber.md, /tmp/qa11/mmd10-autonumber2.md (MMD-10); earlier /tmp/qa11/mmd3-*.md and /tmp/qa11/mmd6-control.md.
- Quit: Alt+F4, clean exit code 0 at 04:15:54 CEST; `pgrep -a -x ferrite` empty afterwards. No panic (exit 134) in any log.
- MMD-6 note: the 'Unknown node id' message is shown only in the UI (amber header); the log has no mermaid WARN line for it, so the grep below does not show it.
- MMD-1 Hand-pan and the FC-165a popup (Fit mode) are the only open UI questions; see the MMD-1 and MMD-11 rows.
- Fixture files were not edited or saved (no git checkout was needed).

## FAIL log excerpts (main instance log: logs/ferrite-debug-20261009-035014-MMD-1.log; snapshot taken after Ferrite quit, exit code 0 at 04:15:54 CEST, no panic)

Lines cut at 400 characters where longer (marked [cut]).

### MMD-3 (FAIL)

Last ~30 non-span log lines:
```
[2026-10-09T02:15:53Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:15:54Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T02:15:54Z INFO  ferrite::app] Application exiting
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saved recovery content for tab 80 (path=None, hash=None)
[2026-10-09T02:15:54Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=51, clean_shutdown=true
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 51 tabs, SingleFile)
[2026-10-09T02:15:54Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 04:15:54 CEST ===
```

Mermaid/warn lines (last 20, non-span):
```
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T01:50:14Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:50:14Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:55:10Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_83'
[2026-10-09T01:55:10Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_83.md - Ferrite"
[2026-10-09T01:56:37Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:56:37Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:58:25Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:58:25Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T02:02:36Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T02:02:36Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md
[2026-10-09T02:02:36Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:02:36Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T02:02:36Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
[2026-10-09T02:08:01Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:08:01Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
```

### MMD-6 (FAIL)

Last ~30 non-span log lines:
```
[2026-10-09T02:15:53Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:15:54Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T02:15:54Z INFO  ferrite::app] Application exiting
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saved recovery content for tab 80 (path=None, hash=None)
[2026-10-09T02:15:54Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=51, clean_shutdown=true
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 51 tabs, SingleFile)
[2026-10-09T02:15:54Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 04:15:54 CEST ===
```

Mermaid/warn lines (last 20, non-span):
```
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T01:50:14Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:50:14Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:55:10Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_83'
[2026-10-09T01:55:10Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_83.md - Ferrite"
[2026-10-09T01:56:37Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:56:37Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:58:25Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:58:25Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T02:02:36Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T02:02:36Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md
[2026-10-09T02:02:36Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:02:36Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T02:02:36Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
[2026-10-09T02:08:01Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:08:01Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
```

### MMD-13 (FAIL)

Last ~30 non-span log lines:
```
[2026-10-09T02:15:53Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:15:54Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T02:15:54Z INFO  ferrite::app] Application exiting
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saved recovery content for tab 80 (path=None, hash=None)
[2026-10-09T02:15:54Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=51, clean_shutdown=true
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 51 tabs, SingleFile)
[2026-10-09T02:15:54Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T02:15:54Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T02:15:54Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T02:15:54Z INFO  ferrite::state] Settings saved
[2026-10-09T02:15:54Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T02:15:54Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T02:15:54Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 04:15:54 CEST ===
```

Mermaid/warn lines (last 20, non-span):
```
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T01:50:14Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:50:14Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:55:10Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_83'
[2026-10-09T01:55:10Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_83.md - Ferrite"
[2026-10-09T01:56:37Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:56:37Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T01:58:25Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_v031'
[2026-10-09T01:58:25Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_v031.md - Ferrite"
[2026-10-09T02:02:36Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T02:02:36Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md
[2026-10-09T02:02:36Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:02:36Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T02:02:36Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
[2026-10-09T02:08:01Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:08:01Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
```

## git status (append, `git -C /workspace/ferrite-qa/ferrite status --short`)

```
```

(Empty output above = clean working tree; nothing was changed, committed or pushed.)
