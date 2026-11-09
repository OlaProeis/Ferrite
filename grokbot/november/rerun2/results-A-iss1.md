# Ferrite v0.3.1 rerun 2 (221b6ae) - Batch A: ISS-1 rows

Note: env: missing libxkbcommon-x11, fixed, not a Ferrite defect. The previous T1-2 `FAIL (crash)` was startup environment failure, not a Ferrite defect.

| ROW-ID | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|
| T1-2 | PARTIAL | Rendered edit committed on click-away without hanging; Ctrl+S did not clear the dirty marker. Alt+F4 prompted for unsaved changes; chose Save, which wrote the edit. Disk check: all 8 line endings remained CRLF (zero lone LF); clean exit. | FAIL (hang; prior batch 2) | `ferrite-debug-20261109-104340-T1-2.log` | Computer capture after commit/save: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/bb45eef07cf4c004a0131aa69ad9be5cb2ecc798154f3016fb62ad7ef23fcc95.webp (`shot.sh` unavailable: `scrot` missing) |
| T1-7 | PASS | Rendered edit with two characters produced the Unsaved Changes dialog (Save/Don’t Save/Cancel). Cancel dismissed it and preserved the dirty edit; app remained responsive. Reopened the prompt, chose Save, then reload completed and the saved edit was present on the /tmp copy. | PASS (dialog; hang followed Cancel) | `ferrite-debug-20261109-104547-T1-7.log` | Computer capture of the dialog: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/599fe9f0f77d320c488a380bcf3cd19f2da159096f8f4d78177be47da41a59d8.webp (`shot.sh` unavailable: `scrot` missing) |
| T1-17 | FAIL | On the /tmp copy, clicked Section 1 and pressed ArrowDown repeatedly; navigation never reached the later multi-line paragraph. A diagnostic `X` typed after the arrow sequence landed in the heading (`## SectioXn 1`); Ctrl+Z restored it. App stayed responsive and did not hang; paragraph-integrity edit step not reached. | FAIL (hang; prior batch 2) | `ferrite-debug-20261109-104703-T1-17.log` | Computer capture of Raw diagnostic state: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/7a19bc9e2abf22c02419dc9e0fd7a0f976519623b0b0349a52aec00df7f8f993.webp (`shot.sh` unavailable: `scrot` missing) |
| T1-18 | FAIL | On the temporary copy, ArrowDowns from the preceding paragraph did not navigate into the blockquote; typed `x` landed in the preceding paragraph instead. Raw view confirmed the quote remained intact; Ctrl+Z restored the copy. No hang; app responsive. | FAIL (hang; prior batch 2) | `ferrite-debug-20261109-105042-T1-18.log` | Computer capture of Raw diagnostic state: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/f744570ece2594d06af169acc1e64001a53b6d898f24a3be952d0e0e116b28df.webp (`shot.sh` unavailable: `scrot` missing) |
| T1-18b | PASS | On a new /tmp document, edited heading, quoted paragraph, and callout body by appending `x` and clicking away after each. Raw view retained exact prefixes: `> ## Titlex`, `> quoted onex`, `> [!NOTE]`, `> callout bodyx`. Three click-away commits remained responsive; no hang. | FAIL (hang; prior batch 2) | `ferrite-debug-20261109-105213-T1-18b.log` | Computer capture of Raw verification: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/4f14d197255b6d060109209f470705f7444b62fa1286f9b6817efee571c343e5.webp (`shot.sh` unavailable: `scrot` missing) |
| T2-35 | FAIL | Split, preview lock ON (Edit controls hidden): Raw ArrowDown moved Ln 10 to Ln 11 and a temporary `Z` edit was reversible. Locked Rendered ArrowDown produced no visible navigation; typing `Q` left preview unchanged. No hang; app remained responsive. Restart/persistence check not reached. | FAIL (hang; prior batch 5) | `ferrite-debug-20261109-105458-T2-35.log` | Computer capture of locked preview state: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/07225fabd8f0f4f39d800c95e6790f3dd052c10ffdd5b903cc07a0f4473c9c02.webp (`shot.sh` unavailable: `scrot` missing) |
| ARR-1 | PASS | Rendered `test_find_arrow_nav.md`: click in Section 1 paragraph, ArrowDown twice; preview scrolled to the following paragraph/blocks and remained responsive. No hang or crash. | FAIL (hang; prior batch 9, two attempts) | `ferrite-debug-20261109-105645-ARR-1.log` | Computer capture after ArrowDown: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/c5855f139120ccf3873abac8292050fad413aaa6f7b3d5f1270d7d879a71a14f.webp (`shot.sh` unavailable: `scrot` missing) |
| ARR-2 | FAIL | Clicked the second visual row of Section 4’s wrapped paragraph and pressed ArrowDown repeatedly, including past the block. Ferrite remained responsive and the rendered page scrolled, but no visible caret/visual-row movement or updated cursor position was confirmed (status stayed at Ln 1, Col 1). No hang. | FAIL (hang; prior batch 9, two attempts) | `ferrite-debug-20261109-105726-ARR-2.log` | Computer capture after ArrowDown: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/6554e96d3b94c20397b8eae0a18f6c2d333cbf60447262683fd1aaa6503a676b.webp (`shot.sh` unavailable: `scrot` missing) |
| ARR-4 | FAIL | Split view: Raw ArrowDown moved the caret from Ln 10 to Ln 11. Focused the Rendered paragraph and pressed ArrowDown; no visible cursor navigation or preview scroll was confirmed, but the app remained responsive and raw focus did not appear to be stolen. No hang. | FAIL (hang; prior batch 9, two attempts) | `ferrite-debug-20261109-105807-ARR-4.log` | Computer capture after Rendered ArrowDown: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/4e98e1e8221b64255fd0ae7101e435fe51e992d5f99099565d51e712ec37b6be.webp (`shot.sh` unavailable: `scrot` missing) |
| CPY-3 | FAIL | Rendered inline-formatted paragraph was visibly selected with click then Shift-click and Ctrl+C was tried three times; `xclip -selection clipboard -o` and `primary` both remained empty, so no copied text was available to verify spacing. App responsive; no hang. | FAIL (hang; prior batch 9) | `ferrite-debug-20261109-105853-CPY-3.log` | Computer capture of selected paragraph after copy: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/8bca38cb7202d4ab5b0340feb619682425dcf2a5a08972c69aa47b7575c498d9.webp (`shot.sh` unavailable: `scrot` missing) |
| IND-9 | FAIL | Rendered list: clicked `list item one` with the preview unlocked and pressed Tab; no edit/caret or indentation appeared. Switching to Raw showed the three list lines unchanged; app stayed responsive, no hang. | FAIL (hang; prior batch 10) | `ferrite-debug-20261109-110047-IND-9.log` | Computer capture after Tab: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/ce890f1451d5ef328d992e3a88977ef7060618fae29bca9bbb1bf7b0dddf14cd.webp (`shot.sh` unavailable: `scrot` missing) |
| UX-9 | FAIL | In Rendered, plain click on the list-item file link switched it to source edit (Escape returned to preview; responsive). Ctrl-click first reported `Failed to open link: test_flowcharts.md` because the target was absent beside the /tmp fixture. After placing a temporary target copy alongside it, Ctrl-click opened a Chrome directory index of `/tmp/ferrite-rerun2/` rather than the linked document/file manager; no hang. | FAIL (hang; prior batch 12) | `ferrite-debug-20261109-110140-UX-9.log` | Computer capture of opened local directory index: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/97cf1c50f50aa65f341ba55ae20f8060294d53c71d8c6ebf85b1806fa55ea539.webp; plain-click edit: file:///cursor/stores/user/grok-bot/agents/ce8a3978-2cf3-438d-a569-1970300bd5d5/assets/0d6aae7473079134b92a5b4f523a013e11099002941b1d1d23df5da3d376fd45.webp (`shot.sh` unavailable: `scrot` missing) |

## FAIL log excerpts

Each excerpt is `grep -v 'tracing::span' <log> | tail -n 30`.

### T1-17 — `ferrite-debug-20261109-104703-T1-17.log`

```text
[2026-11-09T09:50:34Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:50:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T09:50:34Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T09:50:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T09:50:34Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T09:50:34Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1530, height: 963 })
[2026-11-09T09:50:34Z INFO  ferrite::app] Application exiting
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Deleted recovery content for tab 65
[2026-11-09T09:50:34Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=34, clean_shutdown=true
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 34 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T09:50:34Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T09:50:34Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T09:50:34Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:50:34Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:50:34Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:50:34Z INFO  ferrite::state] Settings saved
[2026-11-09T09:50:34Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:50:34Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T09:50:34Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T09:50:34Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T09:50:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:50:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:50:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:50:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:50:34Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T09:50:34Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 10:50:34 CET ===
```

### T1-18 — `ferrite-debug-20261109-105042-T1-18.log`

```text
[2026-11-09T09:51:55Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:51:55Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T09:51:55Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T09:51:55Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T09:51:55Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T09:51:55Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1530, height: 963 })
[2026-11-09T09:51:55Z INFO  ferrite::app] Application exiting
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Deleted recovery content for tab 66
[2026-11-09T09:51:55Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=34, clean_shutdown=true
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 34 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T09:51:55Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T09:51:55Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T09:51:55Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:51:55Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:51:55Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:51:55Z INFO  ferrite::state] Settings saved
[2026-11-09T09:51:55Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:51:55Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T09:51:55Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T09:51:55Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T09:51:55Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:51:55Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:51:55Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:51:55Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:51:55Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T09:51:55Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 10:51:55 CET ===
```

### T2-35 — `ferrite-debug-20261109-105458-T2-35.log`

```text
[2026-11-09T09:56:30Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:56:30Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T09:56:30Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T09:56:30Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=721.3, max_wrap_width=840.0, effective=721.3
[2026-11-09T09:56:30Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T09:56:30Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 761, height: 963 })
[2026-11-09T09:56:30Z INFO  ferrite::app] Application exiting
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Deleted recovery content for tab 67
[2026-11-09T09:56:30Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=35, clean_shutdown=true
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 35 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T09:56:30Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T09:56:30Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T09:56:30Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:56:30Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:56:30Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:56:30Z INFO  ferrite::state] Settings saved
[2026-11-09T09:56:30Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:56:30Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T09:56:30Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T09:56:30Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T09:56:30Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:56:30Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:56:30Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:56:30Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:56:30Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T09:56:30Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 10:56:30 CET ===
```

### ARR-2 — `ferrite-debug-20261109-105726-ARR-2.log`

```text
[2026-11-09T09:58:02Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-11-09T09:58:02Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:58:02Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:58:02Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:58:02Z INFO  ferrite::state] Settings saved
[2026-11-09T09:58:02Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:58:02Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T09:58:02Z INFO  ferrite::app] Application exiting
[2026-11-09T09:58:02Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T09:58:02Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=35, clean_shutdown=true
[2026-11-09T09:58:02Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T09:58:02Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T09:58:02Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 35 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T09:58:02Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T09:58:02Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T09:58:02Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:58:02Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:58:02Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:58:02Z INFO  ferrite::state] Settings saved
[2026-11-09T09:58:02Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:58:02Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T09:58:02Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T09:58:02Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T09:58:02Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:02Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:02Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:02Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:02Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T09:58:02Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 10:58:02 CET ===
```

### ARR-4 — `ferrite-debug-20261109-105807-ARR-4.log`

```text
[2026-11-09T09:58:48Z INFO  ferrite::state] Settings saved
[2026-11-09T09:58:48Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:58:48Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T09:58:48Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T09:58:48Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=721.3, max_wrap_width=840.0, effective=721.3
[2026-11-09T09:58:48Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T09:58:48Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 761, height: 963 })
[2026-11-09T09:58:48Z INFO  ferrite::app] Application exiting
[2026-11-09T09:58:48Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T09:58:48Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=35, clean_shutdown=true
[2026-11-09T09:58:48Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T09:58:48Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T09:58:48Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 35 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T09:58:48Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T09:58:48Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T09:58:48Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T09:58:48Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T09:58:48Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T09:58:48Z INFO  ferrite::state] Settings saved
[2026-11-09T09:58:48Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T09:58:48Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T09:58:48Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T09:58:48Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T09:58:48Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:48Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:48Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:48Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T09:58:48Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T09:58:48Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 10:58:48 CET ===
```

### CPY-3 — `ferrite-debug-20261109-105853-CPY-3.log`

```text
[2026-11-09T10:00:41Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:00:41Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:00:41Z INFO  ferrite::state] Settings saved
[2026-11-09T10:00:41Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:00:41Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T10:00:41Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 659, y: 661 }) size=Physical(PhysicalSize { width: 515, height: 34 })
[2026-11-09T10:00:41Z INFO  ferrite::app] Application exiting
[2026-11-09T10:00:41Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T10:00:41Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=36, clean_shutdown=true
[2026-11-09T10:00:41Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T10:00:41Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T10:00:41Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 36 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T10:00:41Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T10:00:41Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T10:00:41Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T10:00:41Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:00:41Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:00:41Z INFO  ferrite::state] Settings saved
[2026-11-09T10:00:41Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:00:41Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:00:41Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:00:41Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:00:41Z WARN  arboard::platform::linux::x11] Could not hand the clipboard contents over to the clipboard manager. The request timed out.
[2026-11-09T10:00:41Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:00:41Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:00:41Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:00:41Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:00:41Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:00:41Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:00:41 CET ===
```

### IND-9 — `ferrite-debug-20261109-110047-IND-9.log`

```text
[2026-11-09T10:01:34Z INFO  ferrite::state] Settings saved
[2026-11-09T10:01:34Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:01:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:01:34Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:01:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:01:34Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T10:01:34Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1530, height: 963 })
[2026-11-09T10:01:34Z INFO  ferrite::app] Application exiting
[2026-11-09T10:01:34Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T10:01:34Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=37, clean_shutdown=true
[2026-11-09T10:01:34Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T10:01:34Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T10:01:34Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 37 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T10:01:34Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T10:01:34Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T10:01:34Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T10:01:34Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:01:34Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:01:34Z INFO  ferrite::state] Settings saved
[2026-11-09T10:01:34Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:01:34Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:01:34Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:01:34Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:01:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:01:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:01:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:01:34Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:01:34Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:01:34Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:01:34 CET ===
```

### UX-9 — `ferrite-debug-20261109-110140-UX-9.log`

```text
[2026-11-09T10:03:08Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-11-09T10:03:08Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T10:03:08Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:03:08Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:03:08Z INFO  ferrite::state] Settings saved
[2026-11-09T10:03:08Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:03:08Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-11-09T10:03:08Z INFO  ferrite::app] Application exiting
[2026-11-09T10:03:08Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-11-09T10:03:08Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=true
[2026-11-09T10:03:08Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-11-09T10:03:08Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-11-09T10:03:08Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 38 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-11-09T10:03:08Z INFO  ferrite::app] Session state saved for next startup
[2026-11-09T10:03:08Z DEBUG ferrite::config::session] Removed session lock file
[2026-11-09T10:03:08Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-11-09T10:03:08Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:03:08Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:03:08Z INFO  ferrite::state] Settings saved
[2026-11-09T10:03:08Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:03:08Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:03:08Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:03:08Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:03:08Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:03:08Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:03:08Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:03:08Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:03:08Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:03:08Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:03:08 CET ===
```

## Final state

Ferrite was closed cleanly; `pgrep -a -x ferrite` returned no processes. All 12 requested rows were reached. `git -C /workspace/ferrite-qa/ferrite status --short` returned empty (clean repository). `shot.sh` could not capture images because `scrot` is missing; Computer captures are linked in the table.
