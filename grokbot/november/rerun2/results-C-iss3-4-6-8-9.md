# Ferrite v0.3.1 rerun 2 (221b6ae+/HEAD 9effd76) - Batch C: ISS-3, ISS-4, ISS-6 (partial), ISS-8, ISS-9, T1-21 retry

**Requested rerun rows completed on the restored v0.3.1 release build (`9effd76`).** The earlier box-restart blockers were rerun. `~/.config/ferrite` was backed up to `rerun2/config-backup/ferrite/` before EXTRA-SET-9a/b and restored exactly after testing. The report below records each fresh result; the final process/config/repository state is recorded at the end.

Note: Ferrite repaints lazily under Xvfb/llvmpipe, so screenshots lagged one action behind. I jiggled the mouse before reading state. The status bar and the files on disk were the sources of truth. Search-in-Files rows used the workspace `/tmp/ferrite-rerun2/ws` (contents: `zz_cjk_search.md` with `搜索测试 中文 search` / `Istanbul İstanbul istanbul`), not the repo, to keep `git status` clean. The CJK query was pasted from the clipboard.

| ROW-ID | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|
| T1-4b | PASS | `/tmp/ferrite-rerun2/crlf-utf16.md` (BOM `fffe`, UTF-16LE, CRLF) opens as text in Ferrite. The status bar shows `UTF-16LE`, the content is readable and there was no "delegating to external app". I clicked in Raw, pressed Enter once and saved with Ctrl+S. The log says `Saved file: ... (encoding: UTF-16LE)`. `eol-check.sh ... utf16`: BOM `fffe`, 4 `0d00 0a00` pairs (was 3), lone `0a00` offsets `[]`, RESULT: PASS. The editor text renders faint grey (cosmetic, not investigated). | Prior `qa-report-v0.3.1-linux.md`: FAIL (ISS-3, file delegated to external app). | `logs/ferrite-debug-20261109-112724-R2-T1-4b.log`; `logs/R2-T1-4b-check.txt` | `shots/R2-T1-4b-pass.png` |
| RLD-7 (UTF-16 half) | PASS | After the save above I appended `X\r\n` (UTF-16LE) externally, then pressed Ctrl+Shift+R. The status bar showed `Reloaded: crlf-utf16.md`, `X` appeared on a new line and nothing was garbled. I typed `Y` on a blank line and saved. The file on disk is BOM `fffe`, 5 `0d00 0a00` pairs, no lone `0a00`, and decodes as `\r\nFirst paragraph.\r\nY\r\nSecond paragraph with more words.\r\nX\r\n`, so it is valid UTF-16LE. The Windows-1252 half was not re-run (it already passed). My first reload ran before the append because I issued the append and the key press together, so I reloaded again. | Prior `qa-report-v0.3.1-linux.md`: PARTIAL (UTF-16 half FAIL = ISS-3; cp1252 PASS). | `logs/ferrite-debug-20261109-112724-R2-T1-4b.log`; `logs/R2-RLD-7-utf16-check.txt` | `shots/R2-RLD-7-pass.png` |
| T1-15 | PASS | Workspace `ws`, Ctrl+Shift+F, `中文` (clipboard paste): "1 match in 1 file", `zz_cjk_search.md` listed. Clicking the result opens the file with the caret at **Ln 1, Col 6**, which is the start of `中文`. Log: `Opened file from search ... line 1, char offset 5`, `after set_cursor, cursor_after=(0,5)`. There was no crash and no jump to Ln 1, Col 1. | Prior `results-batch2.md`: FAIL (caret Ln 1, Col 1, ISS-4). | `logs/ferrite-debug-20261109-112856-R2-T1-15.log` | `shots/R2-T1-15-pass.png` |
| SIF-1 | PASS | Same steps and result as T1-15: one result, caret on `中文` (Col 6). | Prior `results-batch7.md`: FAIL (caret not on match). | `logs/ferrite-debug-20261109-112856-R2-T1-15.log` | `shots/R2-SIF-1-pass.png` |
| SIF-2 | PASS | Query `search` (Aa off): one result on the CJK line. After the click the status bar says **Ln 1, Col 9**, which is the start of `search` after the CJK characters. Log: `char offset 8`, `cursor_after=(0,8)`. | Prior `results-batch7.md`: FAIL (cursor_after=(0,0)). | `logs/ferrite-debug-20261109-112856-R2-T1-15.log` | `shots/R2-SIF-2-pass.png` |
| SIF-3 | PASS | Query `istanbul` (Aa off): "3 matches in 1 file". `Istanbul`, `İstanbul` (U+0130) and `istanbul` are all listed. Clicking each landed the caret on the matching word: `Istanbul` Ln 2, Col 1 (offset 15); `İstanbul` Ln 2, Col 10 (offset 24); `istanbul` Ln 2, Col 19 (offset 33). There was no drift after `İ`. | Prior `results-batch7.md`: FAIL (`İstanbul` not found, only 2 of 3). | `logs/ferrite-debug-20261109-112856-R2-T1-15.log` | `shots/R2-SIF-3-pass.png` |
| T1-19 | FAIL | Fresh-build recheck on `/tmp/ferrite-rerun2/test_copy_and_split_undo.md`: in Split, one `x` at the end of the preview Python block correctly appeared at line 43, but an unrelated blank line 49 became a duplicate `# What to check`. First Ctrl+Z removed only that stray heading edit and left `x`; a second Ctrl+Z removed `x`. I then completed 3 Ctrl+Y/Ctrl+Z cycles: each redo restored the stray heading and each undo removed it, while `x` remained after each cycle; SENTINEL lines stayed intact. | Prior `results-batch2.md`: FAIL (undo per-keystroke). | `logs/ferrite-debug-20261109-115009-R2-T1-19.log` | `shots/R2-T1-19-fail.png`, `shots/R2-T1-19-before-undo.png` |
| UND-1 | FAIL | Run separately on the same fixture. A preview code-block edit again created the unrelated duplicate heading; first Ctrl+Z undid only that stray edit, not the typed `x` (log confirms the undo action). This independently confirms the undo defect. | Prior `qa-report-v0.3.1-linux.md`: FAIL (via T1-19). | `logs/ferrite-debug-20261109-115152-R2-UND-1.log` | `shots/R2-UND-1-fail.png` |
| T1-22 | PASS | `test_mermaid_issue_165.md`: widest flowchart fit the pane; Fit/Native toggle appeared; popup opened and zoom in/out/reset and close worked. Wheel-scrolling with the pointer over the diagram showed no jitter, and no large gap or status-bar overpaint was seen. Native mode overflow had no visible scrollbar. | Prior `qa-report-v0.3.1-linux.md`: FAIL (popup never opened/hung). | `logs/ferrite-debug-20261109-115239-R2-T1-22.log` | `shots/R2-T1-22-pass.png` |
| MMD-1 | PARTIAL | Popup opened; zoom/reset/close worked. In Hand mode the diagram did not pan while it fit inside the viewport; after zooming beyond the viewport it did pan. Marked partial because Hand-mode panning was not usable at fit size. | Prior `qa-report-v0.3.1-linux.md`: PARTIAL (popup body click). | `logs/ferrite-debug-20261109-115441-R2-MMD-1.log` | `shots/R2-MMD-1-partial.png` |
| EXTRA-SET-9a | FAIL | With `~/.config/ferrite` backed up to `rerun2/config-backup/ferrite/`, replaced `config.json` with `{ not json` and launched. Ferrite started and renamed the invalid file to `config.json.corrupt-<timestamp>`, then created defaults; no user-visible toast appeared and the prior settings were not preserved. The app exited cleanly. | Prior `results-batch3.md`: no crash, but silent full config reset. | `logs/ferrite-debug-20261109-115756-R2-EXTRA-SET-9a.log` | `shots/R2-EXTRA-SET-9a-fail.png` |
| EXTRA-SET-9b | FAIL | Restored the good config from backup, set `font_size` to the wrong type (`"abc"`), then launched. Ferrite loaded without crashing and logged that only the invalid field was reset to default; the config remained valid and the other settings were retained. No user-visible toast appeared, so the stated ISS-9 feedback expectation was not met. | Prior `results-batch3.md`: one bad field silently discarded the whole config. | `logs/ferrite-debug-20261109-115843-R2-EXTRA-SET-9b.log` | `shots/R2-EXTRA-SET-9b-fail.png` |
| T1-21 (retry) | PASS | In Split, typed the ```` ```mermaid ```` opener one character at a time on the real keyboard. Then put the exact malformed lines `A}x{ --> B` and `subgraph a] [b` on the clipboard with `xclip` and pasted them. The opener remained ordered, the diagram rendered/updated, and Ferrite did not crash; the run exited cleanly. | Prior Batch B: PARTIAL (synthetic input dropped a CJK character and auto-close appended characters). | `logs/ferrite-debug-20261109-115953-R2-T1-21.log` | none (pass; screenshot not required) |

## FAIL log tails (fresh R2 runs)

### T1-19 — `logs/ferrite-debug-20261109-115009-R2-T1-19.log`

Undo/redo entries confirm three complete cycles after the first Ctrl+Z:
```text
[2026-11-09T10:50:53Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
[2026-11-09T10:51:01Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-11-09T10:51:06Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
[2026-11-09T10:51:11Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-11-09T10:51:17Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
[2026-11-09T10:51:22Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-11-09T10:51:28Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
```
Final log tail:
```text
[2026-11-09T10:51:39Z DEBUG ferrite::state] AppState shutdown complete
[2026-11-09T10:51:39Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:51:39Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:51:39Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:51:39Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:51:39Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:51:39Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:51:39 CET ===
```

### UND-1 — `logs/ferrite-debug-20261109-115152-R2-UND-1.log`

```text
[2026-11-09T10:52:31Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:52:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:52:31Z INFO  ferrite::state] Settings saved
[2026-11-09T10:52:31Z INFO  ferrite::state] AppState shutdown complete
[2026-11-09T10:52:31Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:52:31Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:52:31Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:52:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:52:31Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:52:31Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:52:31 CET ===
```

### EXTRA-SET-9a — `logs/ferrite-debug-20261109-115756-R2-EXTRA-SET-9a.log`

The startup log recorded `Config file not found ... creating defaults with system locale` after the invalid file had been renamed; this corroborates that the user's prior config was not loaded. Startup evidence:
```text
[2026-11-09T10:57:56Z DEBUG ferrite::config::persistence] Config file not found at /home/box/.config/ferrite/config.json, creating defaults with system locale
```
Final log tail:
```text
[2026-11-09T10:58:30Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:58:30Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:58:30Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:58:30Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:58:30 CET ===
```

### EXTRA-SET-9b — `logs/ferrite-debug-20261109-115843-R2-EXTRA-SET-9b.log`

```text
[2026-11-09T10:58:43Z WARN  ferrite::config::settings] Dropping invalid config field `font_size` (value: "abc"): invalid type: string "abc", expected f32
[2026-11-09T10:58:43Z WARN  ferrite::config::persistence] Config at /home/box/.config/ferrite/config.json recovered with 1 invalid field(s) reset to default
[2026-11-09T10:58:43Z INFO  ferrite::config::persistence] Configuration loaded successfully from /home/box/.config/ferrite/config.json
```
Final log tail:
```text
[2026-11-09T10:59:10Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-11-09T10:59:10Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-11-09T10:59:10Z INFO  ferrite::state] Settings saved
[2026-11-09T10:59:10Z DEBUG ferrite::state] AppState shutdown complete
[2026-11-09T10:59:10Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-11-09T10:59:10Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-11-09T10:59:10Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-11-09T10:59:10Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-11-09T10:59:10Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-11-09T10:59:10Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 11:59:10 CET ===
```

## Rows not reached
None of the requested rows remain blocked. T1-19, UND-1, T1-22, MMD-1, EXTRA-SET-9a/b and T1-21 (retry) all have fresh results above.

## End-of-run state
- Ferrite was closed cleanly after T1-21; `pgrep -a ferrite` returned no process.
- `~/.config/ferrite` matches the saved backup at `/workspace/ferrite-qa/rerun2/config-backup/ferrite/` exactly.
- `git -C /workspace/ferrite-qa/ferrite status --short` is empty (clean).
