# Ferrite v0.3.1 Linux — Rerun D Tier 1 remainder

Build tested: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (`9effd76`). Prior results are from `/workspace/ferrite-qa/qa-report-v0.3.1-linux.md` unless otherwise noted. Tier 1 rows already marked done by the request are omitted. T1-2 below is only the requested Ctrl+S substep recheck; it does not replace the prior whole-row FAIL.

## D2 configuration override check

`~/.config/ferrite/config.json` and `config.json.bak` both have `keyboard_shortcuts.bindings: {}`; no separate settings/keybindings file was present. Source `src/config/settings.rs` defaults remain Reload = Ctrl+Shift+R (line 896) and InsertToc = Ctrl+Shift+U (line 963). No override found, so config was not moved and no restore was needed.

| ROW-ID | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|
| T1-1 | PASS | Rendered click on “First paragraph.” then Escape; tab had no dirty `*`, no content change observed; clean exit. | PASS | `logs/ferrite-debug-20261109-120916-R2-T1-1.log` | `shots/R2-T1-1-pass.png` |
| T1-2 (Ctrl+S only) | PASS (scoped recheck) | After the dirty edit, Ctrl+S saved `/tmp/crlf-test.md`; the tab title lost `*`; CRLF remained 8/8 lines. The remaining T1-2 steps were not rerun. | FAIL (hang; Ctrl+S not reached) | `logs/ferrite-debug-20261109-121014-R2-T1-2.log`; `logs/R2-T1-2-eol-check.txt` | `shots/R2-T1-2-pass.png` |
| T1-3 | PASS | Raw edit/join produced `Second paragraph with more words.new line`; byte check: 117 bytes, CRLF 8, lone LF 0. | PASS | `logs/ferrite-debug-20261109-121014-R2-T1-2.log`; `logs/R2-T1-3-check.txt` | `shots/R2-T1-3-pass.png` |
| T1-4 | PASS | With the caret visibly in Raw text, Ctrl+Shift+D duplicated the heading and Ctrl+Shift+U inserted a 3-heading TOC; Ctrl+S saved it. EOL checker reported CRLF preserved (15/15, lone LF 0; its `file`/`xxd` subcommands were unavailable, while the Python byte check confirmed TOC lines are CRLF). Earlier no-response attempt lacked confirmed text focus. Alt+Space was intercepted by XFWM; MD reference had no TOC route. | PASS | `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; `logs/RERUN-D2-T1-4-log-tail.txt`; `logs/RERUN-D2-T1-4-eol-check.txt` | `shots/RERUN-D2-T1-4-pass.png`; initial attempt: `shots/RERUN-D2-T1-4-initial-unconfirmed-focus.png` |
| T1-5 | PASS | Clicked inside the clean Raw document, externally changed `First` to `FIRST`, then exercised Ctrl+Shift+R via xdotool and Computer; the clean tab showed `FIRST`. Log records the shortcut and the Reload-from-Disk ribbon route was also exercised (the app menu’s Reload from Disk command). | PASS | `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; `logs/RERUN-D2-T1-5-log-tail.txt` | — |
| T1-6 | PASS | Clicked and typed in Raw text to confirm focus, then appended `external edit` on disk. xdotool Ctrl+Shift+R opened Unsaved Changes; Cancel retained the local edit. After refocusing, Computer Ctrl+Shift+R reopened the dialog; Don’t Save loaded the external edit. | PASS | `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; `logs/RERUN-D2-T1-6-log-tail.txt` | `shots/RERUN-D2-T1-6-pass.png` |
| T1-8 | PASS | Focus-confirmed Ctrl+Shift+R opened the dialog. Closing `other-tab.md` by its tab X or middle-click closed the tab but dismissed the dialog; re-triggering then choosing Don’t Save loaded `ext2`. `other-tab.md` stayed unchanged on disk. Shortcut itself worked; dialog-dismissal caveat recorded. | PASS | `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; `logs/RERUN-D2-T1-8-log-tail.txt` | `shots/RERUN-D2-T1-8-dialog-dismissed.png` |
| T1-9 | PASS | Alt+F4 → Save saved both `save-a.md` and `save-b.md`; app exited cleanly (0), no dialog loop. | PASS | `logs/ferrite-debug-20261109-121419-R2-T1-9.log` | — |
| T1-10 | PASS | Alt+F4 → Don’t Save; relaunch showed no discarded edit/recovery banner; clean exit. | PASS | `logs/ferrite-debug-20261109-121537-R2-T1-10.log`; `logs/ferrite-debug-20261109-121616-R2-T1-10b.log` | `shots/R2-T1-10-pass.png` |
| T1-11 | PASS | Forced process kill was the test action (exit 137); relaunch offered recovery, restored the edit, and one Ctrl+Z returned to disk content; disk check showed the original content. | PASS | `logs/ferrite-debug-20261109-121644-R2-T1-11.log`; `logs/ferrite-debug-20261109-121718-R2-T1-11-relaunch.log`; `logs/R2-T1-11-disk-check.txt` | `shots/R2-T1-11-pass.png` |

original content
`. | PASS | `logs/ferrite-debug-20261109-121644-R2-T1-11.log`; `logs/ferrite-debug-20261109-121718-R2-T1-11-relaunch.log`; `logs/R2-T1-11-disk-check.txt` | `shots/R2-T1-11-pass.png` |
| T1-12 | PASS | Typed `§`, then `é` and `中` on separate lines in a new tab; no crash. | PASS | `logs/ferrite-debug-20261109-121758-R2-T1-12-13.log` | `shots/R2-T1-12-pass.png` |
| T1-13 | PASS | On `日本語テキスト ééé`, Ctrl+Shift+D, Alt+Up/Down, and delete completed without a crash; caret remained usable. Status-bar line count was stale during the sequence. | PASS | `logs/ferrite-debug-20261109-121758-R2-T1-12-13.log` | `shots/R2-T1-13-pass.png` |
| T1-16 | PASS | Raw Ctrl+F found whole-word `中文` (1/1). Used a zoomed crop and the Aa tooltip (`Match Case`) to identify Case sensitive; with Whole word and Case sensitive active, `é` returned 1/1 and the zoomed match was the standalone `é` in `éa é`. The initial edge click switched to the underlying `test_pos_hints.md`; returned to the fixture and completed the query. Ferrite later prompted about that unsaved tab edit; chose Don’t Save, and repo status remained clean. | PASS | `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; `logs/RERUN-D2-T1-16-log-tail.txt` | `shots/RERUN-D2-T1-16-pass.png`; `shots/RERUN-D2-T1-16-controls-zoom.png`; `shots/RERUN-D2-T1-16-match-zoom.png` |
| T1-16b | PASS | Double-clicked `hello` and pasted the URL; result was `[hello](https://example.com)`. Smart-paste log confirms link creation; no crash. | PASS | `logs/ferrite-debug-20261109-122517-R2-T1-16.log` | `shots/R2-T1-16b-pass.png` |
| T1-20 | FAIL | Selected Block 1 lines 7–9 and Ctrl+clicked line 33, then Tab. Buffer showed line 7 replaced by four spaces, lines 8–9 unchanged, and line 33 became `中文行    首`; this is not the expected per-target indent. One Ctrl+Z restored the fixture byte-for-byte. | BLOCKED (prior Alt+click swallowed by xfwm4; Ctrl+click only supplementary) | `logs/ferrite-debug-20261109-122703-R2-T1-20.log`; `logs/R2-T1-20-buffer-before-undo.txt`; fail tail below | `shots/R2-T1-20-fail.png` |

## D2 rerun log-tail files

Current-session event excerpts are saved separately for audit: `logs/RERUN-D2-T1-4-log-tail.txt`, `logs/RERUN-D2-T1-5-log-tail.txt`, `logs/RERUN-D2-T1-6-log-tail.txt`, `logs/RERUN-D2-T1-8-log-tail.txt`, and `logs/RERUN-D2-T1-16-log-tail.txt`. The full live-process log is `logs/ferrite-debug-20261109-123435-RERUN-D2-T1-4.log`; it spans the rerun rows because Ferrite stayed open between them.

## Previous R2 log tails for the superseded initial results

### T1-4 — `logs/ferrite-debug-20261109-121014-R2-T1-2.log`

```text
[2026-11-09T11:11:45Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+D (Duplicate Line)
[2026-11-09T11:11:58Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+S (Save)
[2026-11-09T11:11:58Z INFO  ferrite::state] Saved file: /tmp/crlf-test.md (encoding: UTF-8)
=== Ferrite exited with code 0 clean exit at 12:12:14 CET ===
```

No `Ctrl+Shift+U` or `TOC inserted` entry was present; screenshot shows no TOC in the document.

### T1-5 — `logs/ferrite-debug-20261109-121227-R2-T1-5-6-8.log`

```text
[2026-11-09T11:12:28Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/fixtures/reload-test.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-11-09T11:12:51Z DEBUG ferrite::app::central_panel] Content modified in raw editor
[2026-11-09T11:12:51Z DEBUG winit::window] winit::Window::set_title; title="reload-test.md* - Ferrite"
```

`Ctrl+Shift+R` / `Reloaded` event count in this log: 0.

### T1-6 — `logs/ferrite-debug-20261109-121227-R2-T1-5-6-8.log`

```text
[2026-11-09T11:12:51Z DEBUG ferrite::app::central_panel] Content modified in raw editor
[2026-11-09T11:12:51Z DEBUG winit::window] winit::Window::set_title; title="reload-test.md* - Ferrite"
[2026-11-09T11:12:52Z DEBUG ferrite::app::central_panel] Content modified in raw editor
```

`Ctrl+Shift+R` / Unsaved Changes / Reloaded event count in this log: 0.

### T1-8 — `logs/ferrite-debug-20261109-121227-R2-T1-5-6-8.log`

```text
[2026-11-09T11:13:59Z DEBUG eframe::native::epi_integration] Closing of root viewport canceled with ViewportCommand::CancelClose
[2026-11-09T11:14:06Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 12:14:06 CET ===
```

`Ctrl+Shift+R` / Reloaded event count in this log: 0; no reload dialog state was reached.

### T1-16 — `logs/ferrite-debug-20261109-122517-R2-T1-16.log`

```text
[2026-11-09T11:25:23Z DEBUG ferrite::app::find_replace] Find panel opened, replace_mode: false
[2026-11-09T11:25:28Z DEBUG ferrite::app::dialogs] Search executed (debounced), found 0 matches
[2026-11-09T11:25:53Z DEBUG ferrite::app::find_replace] Find panel opened, replace_mode: false
[2026-11-09T11:25:56Z DEBUG ferrite::app::dialogs] Search executed (debounced), found 1 matches
[2026-11-09T11:26:03Z DEBUG ferrite::app::dialogs] Search executed (debounced), found 0 matches
[2026-11-09T11:26:41Z DEBUG ferrite::app::input_handling] Smart paste: Created link [hello](https://example.com) at char 33..38
=== Ferrite exited with code 0 clean exit at 12:26:54 CET ===
```

### T1-20 — `logs/ferrite-debug-20261109-122703-R2-T1-20.log`

```text
[2026-11-09T11:29:19Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (6, 14) -> (6, 4)
[2026-11-09T11:29:19Z DEBUG ferrite::app::central_panel] Content modified in raw editor
[2026-11-09T11:29:41Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-11-09T11:29:41Z DEBUG ferrite::app::navigation] Undo performed, 0 entries remaining
=== Ferrite exited with code 0 clean exit at 12:31:23 CET ===
```

Pre-undo copied buffer evidence: `logs/R2-T1-20-buffer-before-undo.txt`. After one Ctrl+Z, the copied buffer matched `test_indent_selection.md` exactly.
