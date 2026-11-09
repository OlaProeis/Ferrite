# Ferrite v0.3.1 QA - Batch 7 (Search-in-Files offsets, primary selection, image paste)

Tester: Grok Bot. Commit under test: 8e6ef50 (`0.3.1`). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> /workspace/ferrite-qa/ferrite` (launched from /workspace/ferrite-qa). Screenshots: `shots/`. Logs: `logs/`. Pre-run `git status --short`: `?? test_md/zz_cjk_search.md` (fixture, expected).

Fixture note: `test_md/zz_cjk_search.md` existed but had only line 1; line 2 (`Istanbul İstanbul istanbul`) was appended before SIF-1 so the file matches the brief (two lines).

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| SIF-1 | FAIL | Ctrl+Shift+F, query `中文` (Aa off): "60 matches in 26 files"; zz_cjk_search.md shows one hit (line 1 `搜索测试 中文 search`, 中文 highlighted in the result). Click on the hit opens the file and the log is correct: `Opened file from search: ...zz_cjk_search.md at line 1, char offset 5` (char offset 5 = 中, so CJK offset is right) and `Set transient highlight at 5..7`. BUT the caret does not move: `EditorWidget: after set_cursor, cursor_after=(0,0)` then `cursor sync changed: (0, 5) -> (0, 0)`. Status bar shows **Ln 1, Col 1** (expected Ln 1, Col 6), and no visible selection on 中文 in the fresh screenshot. Reproduced on the 2nd attempt (already-open tab, same log lines, Ln 1 Col 1) and a 3rd time at 23:32:04. No crash, no hang. Side note: Ctrl+Shift+F was dropped or mis-sent several times (Ctrl not registered; one keystroke typed a stray `F`), each stray edit was undone with Ctrl+Z; file never saved (42 chars, no `*`). | logs/ferrite-debug-20261009-012958-SIF-1.log (launcher exit: see end of batch) | shots/SIF-1-fail.png, shots/SIF-1-zoom.png, shots/SIF-1-retry2-zoom.png |
| SIF-2 | FAIL | Query `search` (Aa off), pasted via clipboard for the re-check (typed input dropped characters during the first run). Clicking the zz_cjk_search.md hit logs `Opened file from search: ...zz_cjk_search.md at line 1, char offset 8` (correct: `s` is char 8 after the CJK chars), but the caret does not land on `search`: `after set_cursor, cursor_after=(0,0)`, `cursor sync changed: (0, 8) -> (0, 0)`, status bar **Ln 1, Col 1** (expected Ln 1, Col 9). RETRACTED from the first run: the claim that the match length is 3 (`Set transient highlight at 8..11`) was made while the query box held a mistyped query; the 3-char highlight is not reproduced: with the clipboard-pasted `search` the results list highlights the whole word (e.g. `**Search**`, `search_pending`, 287 matches in 50 files). The zz_cjk_search.md hit is not in the 50-file-capped list on the re-check, so the editor-side length was not re-verified. No crash, no hang. | logs/ferrite-debug-20261009-013635-SIF-2.log | shots/SIF-2-fail.png, shots/SIF-2-zoom.png, shots/SIF-2-statusbar.png, shots/SIF-2-recheck-zoom.png |
| SIF-3 | FAIL | Query `istanbul` (Aa off), pasted via clipboard (typed input dropped leading chars). Case-insensitive search finds only 2 of the 3 variants on line 2: `Istanbul` (char 15) and `istanbul` (char 33) are listed; **`İstanbul` (U+0130, char 24) is NOT found** (zz file group shows `(2)`). Exact query `İstanbul` finds only the middle variant (`zz_cjk_search.md (1)`, highlight correct). Clicks: (1) `Istanbul` -> `Opened ... at line 2, char offset 15`, highlight 15..23, status **Ln 2, Col 1** = correct word start (PASS for this one). (2) `istanbul` -> `char offset 33`, highlight 33..41 (correct), but caret reset: `after set_cursor, cursor_after=(1,0)`, `cursor sync changed: (1, 18) -> (1, 0)`, status **Ln 2, Col 1** (expected Ln 2, Col 19). (3) `İstanbul` (exact query) -> `char offset 24`, highlight 24..32 (correct), caret reset `(1, 9) -> (1, 0)`, status **Ln 2, Col 1** (expected Col 10). Also: the first click ran on stale results from query `stanbul` (offset 16, caret Col 1 again) and was discarded. Earlier typed-query attempts were flaky (leading `i` dropped). No crash, no hang. | logs/ferrite-debug-20261009-014003-SIF-3.log | shots/SIF-3-query.png, shots/SIF-3-click1.png, shots/SIF-3-click2-fail.png, shots/SIF-3-Idotted-query.png, shots/SIF-3-click-Idotted-status.png |
| SIF-4 | PASS | Launched fresh (tag SIF-4, `run-ferrite.sh --tag SIF-4` with zz_cjk_search.md + fixtures/t1-multibyte.md; session restore reopened ~30 tabs, no Session Recovery dialog). Raw view. Ctrl+F, Whole word (W) ON, query `中文` (pasted via clipboard): Find shows **1 of 1**, highlight on 中文 in zz_cjk_search.md line 1, no crash. Then fixtures/t1-multibyte.md (line 7 is `éa é`, already present, no edit made): Aa (case sensitive) + W ON, query `é`: **1 of 1**, highlight on the standalone `é` (after the space) only; the `é` in `éa` is not matched. (Line 3 `ééé` gives no match with whole-word; consistent with whole-word semantics.) Doc not modified (67 chars / 10 lines before and after, no `*`). Side observation (UI, not a row failure): clicks on the Find panel's own toggles/field also reached the tab strip underneath (tab switched to test_git_graphs.md / lf-test.md); keyboard-only Ctrl+F/Ctrl+V input worked reliably. Earlier SIF-4 attempts in the SIF-3 instance (input routed to editor) are superseded by this fresh-instance run. | logs/ferrite-debug-20261009-015551-SIF-4.log | shots/SIF-4-pass-zz-find.png, shots/SIF-4-t1-zoom.png, shots/SIF-4-pass-t1-eacute-whole-case.png |
| PRI-1 | PASS | Fresh instance (tag PRI-1, zz_cjk_search.md, Raw view). Set PRIMARY to a sentinel first, then double-clicked the word `search` on line 1: `xclip -selection primary -o` after 1 s returned exactly `search` (sentinel replaced). Status bar Ln 1, Col 15 (caret at word end). | logs/ferrite-debug-20261009-015853-PRI-1.log | shots/PRI-1-pass.png |
| PRI-2 | PASS | `printf 'PRIMARY-TEXT' | xclip -selection primary`, middle-click on line 2 at the gap after `Istanbul İ` (Raw). `PRIMARY-TEXT` inserted at the click position (line 2 becomes `Istanbul İPRIMARY-TEXTstanbul istanbul`; 42 -> 54 chars; status Ln 2, Col 23). One Ctrl+Z removed all of it (`Undo (0 remaining)`, back to 42 chars, title without `*`). Nothing saved (file on disk unchanged, 55 bytes). Note: the debug log has no paste-specific line; evidence is screenshot + status bar. | logs/ferrite-debug-20261009-015853-PRI-1.log (same instance, PRI-2 ran after PRI-1) | shots/PRI-2-pass-inserted.png, shots/PRI-2-zoom.png |
| PRI-3 | FAIL | Middle-click on a tab did not close it. Tried 4 tabs in the PRI-1 instance: `ctrl-plain.md` (x2, the second attempt after hover), `lf-test.md`, `vid-1-min.md`, `test_emoji.md`; each stayed open after the middle-click (screenshots show the same tab strip), no crash/hang. Same middle-click input works in the editor (PRI-2), so the event path is live. No tab-close line in the log. Note: an older file `shots/PRI-3-pass.png` (01:53, from an earlier attempt in the SIF-3 instance) also shows the tab still open; it is NOT a pass and is not used as evidence. | logs/ferrite-debug-20261009-015853-PRI-1.log (same instance as PRI-1/PRI-2/PRI-4) | shots/PRI-3-fail-tab-still-open.png |
| PRI-4 | PASS | Settings (gear) -> Editor filter -> checkbox labelled **`Middle-click paste`** (tooltip: "Paste the Linux primary selection at the click position..."). Unchecked it, then repeated PRI-2 on zz line 2 with PRIMARY-TEXT set: nothing inserted (file stays 42 chars, no `*`). Checkbox turned back ON afterwards (verified checked). | logs/ferrite-debug-20261009-015853-PRI-1.log (same instance) | shots/PRI-4-setting-off.png, shots/PRI-4-setting-zoom.png, shots/PRI-4-pass-paste-off-nothing.png |
| IMG-X1 | FAIL | Untitled tab (Ctrl+N), `xclip -selection clipboard -t image/png -i .../test_md/assets/sample.png`, Ctrl+V twice: no toast at all (expected 'save the document first') and nothing inserted. Log: `ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` at 00:02:52Z and 00:03:06Z (same failure as T1-14), so the image-paste path is never reached. Good part: no new `assets/` in the launcher cwd (/workspace/ferrite-qa/assets does not exist) and ferrite/assets/ holds only pre-existing entries (20260130-084650-demo.gif, 20260302-164117-ferritelogo.jpg, dictionaries, fonts, icons, linux, macos, screenshots, web). Recorded by the parent from the operator's notes + log after the session ran out of steps; no screenshot. | logs/ferrite-debug-20261009-020219-IMG-X1.log | none |
| IMG-X2 | FAIL | Pre: `fixtures/assets/` = only `1791485625-clipboard.png` (6190 B, Oct 8 20:53). Doc `fixtures/t1-multibyte.md` opened from the command line (Raw, clean, md5 241dbf8f before and after; 67 chars / 10 lines unchanged, nothing saved). Clipboard set with `xclip -selection clipboard -t image/png -i .../test_md/assets/sample.png`; `xclip -t TARGETS` = `image/png` only. Caret at end (Ln 10, Col 1). Attempt 1: Ctrl+V twice quickly (one key action). Attempt 2: single Ctrl+V (re-checked clipboard first). Every paste fails at the egui_winit layer: `arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` (00:04:57Z x2, 00:05:16Z x1). Image path never reached: no new PNG in `fixtures/assets/` (ls after identical to before; nothing overwritten), no markdown image ref inserted, no toast on screen. `ferrite/assets/` and `/workspace/ferrite-qa/assets` also unchanged. Same failure as IMG-X1 / T1-14. No crash, no hang; clean quit (exit 0). Not a pass; the second-paste collision case (timestamp file name collision) could not be exercised. | logs/ferrite-debug-20261009-020435-IMG-X2.log | shots/IMG-X2-fail.png |

### IMG-X2 FAIL - log evidence

`grep -i -E 'arboard|clipboard|paste|image' logs/ferrite-debug-20261009-020435-IMG-X2.log | grep -v tracing::span | tail` (last 8 shown):

```
[2026-10-09T00:04:35Z DEBUG egui_winit::clipboard] Cannot init smithay clipboard without a Wayland display handle
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored tab: /workspace/ferrite-qa/ferrite/test_md/test_local_images.md (encoding: UTF-8)
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored image viewer tab: /workspace/ferrite-qa/ferrite/assets/icons/icon-192x192.png
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored tab 39 from session: test_local_images.md
[2026-10-09T00:04:57Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-09T00:04:57Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-09T00:05:16Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
```

Last 30 non-span lines (`grep -v 'tracing::span' ... | tail -n 30`):

```
[2026-10-09T00:05:36Z INFO  ferrite::state] Settings saved
[2026-10-09T00:05:36Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:05:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T00:05:36Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T00:05:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T00:05:36Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:05:36Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1250, height: 963 })
[2026-10-09T00:05:36Z INFO  ferrite::app] Application exiting
[2026-10-09T00:05:36Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:05:36Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=29, clean_shutdown=true
[2026-10-09T00:05:36Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:05:36Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:05:36Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 29 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:05:36Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:05:36Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:05:36Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (2 recent files, 1 expanded paths)
[2026-10-09T00:05:36Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:05:36Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:05:36Z INFO  ferrite::state] Settings saved
[2026-10-09T00:05:36Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:05:36Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:05:36Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:05:36Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:05:36Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:05:36Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:05:36Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:05:36Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:05:36Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:05:36Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:05:36 CEST ===
```

`git status --short after batch 7:` (empty output = clean tree)

```
```
